#[cfg(feature = "persistent_world")]
use crate::TerrainPersistence;
use crate::{EditableSettings, Settings, client::Client};
use common::{
    comp::{
        Admin, AdminRole, Body, CanBuild, ControlEvent, Controller, ForceUpdate, Health, Ori,
        Player, Pos, Presence, PresenceKind, Scale, SkillSet, SpectatingEntity, Vel,
    },
    event::{self, EmitExt},
    event_emitters,
    link::Is,
    mounting::{Rider, VolumeRider},
    resources::{DeltaTime, PlayerPhysicsSetting, PlayerPhysicsSettings},
    slowjob::SlowJobPool,
    terrain::{Block, BlockKind, SpriteKind, TerrainGrid},
    uid::IdMaps,
    vol::ReadVol,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral, envelope::{SemanticIngressMetricsV1, SemanticStreamIdV1}};
use common_state::{AreasContainer, BlockChange, BuildArea};
use core::mem;
use rayon::prelude::*;
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteStorage};
use std::{borrow::Cow, sync::Arc, time::Instant};
use tracing::{debug, trace, warn};
use vek::*;
// bastion (CHOP redesign, FR10): the tree ORACLE — Chop tree-detection runs in
// this handler (World stays out of the terrain-only bastion_jobs system). The
// non-worldgen stub World has no oracle, so detection is cfg-gated (degrades
// to no-trees there).
#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World, util::Sampler};

#[cfg(feature = "persistent_world")]
pub type TerrainPersistenceData<'a> = Option<Write<'a, TerrainPersistence>>;
#[cfg(not(feature = "persistent_world"))]
pub type TerrainPersistenceData<'a> = core::marker::PhantomData<&'a mut ()>;

// NOTE: These writes are considered "rare", meaning (currently) that they are
// admin-gated features that players shouldn't normally access, and which we're
// not that concerned about the performance of when two players try to use them
// at once.
//
// In such cases, we're okay putting them behind a mutex and penalizing the
// system if they're actually used concurrently by lots of users.  Please do not
// put less rare writes here, unless you want to serialize the system!
// DET-ECS-015 (v5 deep-pass): RareWrites + its mutex are GONE - terrain
// writes are now deferred per-client records applied in the sorted commit
// phase (gather-sort-commit, same shape as DET-ECS-014), so lock-acquisition
// order can never choose a same-cell winner.

event_emitters! {
    struct Events[Emitters] {
        exit_ingame: event::ExitIngameEvent,
        request_site_info: event::RequestSiteInfoEvent,
        update_map_marker: event::UpdateMapMarkerEvent,
        client_disconnect: event::ClientDisconnectEvent,
        set_battle_mode: event::SetBattleModeEvent,
        // bastion (B3): god-anchor invulnerability buff add/remove
        buff: event::BuffEvent,
        // bastion (#105, DECISIONS-FOR-BEN: FOUNDING SEED STOCK): the LIVE
        // BastionSpawnColony path -- Server::bastion_spawn_colony's own
        // seed-stock call never reaches this system; live colony founding
        // calls `rtsim.bastion_spawn_colony` directly (see the call site
        // below), so the founding drop has to be emitted from here too,
        // not just the Server-level wrapper the harness goes through.
        create_item_drop: event::CreateItemDropEvent,
        // bastion (ROW-COLONY-PRESENCE, DECISIONS #106): same live-path
        // reasoning as `create_item_drop` right above -- the Server-level
        // founding wrapper (`bastion_found_colony_presence`) never runs
        // for a live client founding, which calls `rtsim.
        // bastion_spawn_colony` directly from inside this system.
        create_colony_presence: event::CreateColonyPresenceEvent,
    }
}

// DET-EVT-005 (v5 deep-pass): events produced inside the parallel per-client
// handler are BUFFERED here and emitted in the sorted commit phase — their
// cross-client bus order was worker-completion order (the T0.29 stable merge
// ties on (epoch, producer-site, worker-local seq) for one par site).
enum DeferredInGameEvent {
    ExitIngame(event::ExitIngameEvent),
    SiteInfo(event::RequestSiteInfoEvent),
    MapMarker(event::UpdateMapMarkerEvent),
    BattleMode(event::SetBattleModeEvent),
    Disconnect(event::ClientDisconnectEvent),
}

impl Sys {
    #[expect(clippy::too_many_arguments)]
    fn handle_client_in_game_msg(
        deferred_events: &mut Vec<DeferredInGameEvent>,
        entity: specs::Entity,
        client: &Client,
        maybe_presence: &mut Option<&mut Presence>,
        terrain: &ReadExpect<'_, TerrainGrid>,
        can_build: &ReadStorage<'_, CanBuild>,
        is_rider: &ReadStorage<'_, Is<Rider>>,
        is_volume_rider: &ReadStorage<'_, Is<VolumeRider>>,
        force_update: Option<&&mut ForceUpdate>,
        skill_set: &mut Option<Cow<'_, SkillSet>>,
        healths: &ReadStorage<'_, Health>,
        terrain_writes: &mut Vec<(Vec3<i32>, Block)>,
        position: Option<&mut Pos>,
        spectating_entity: &mut Option<Option<common::uid::Uid>>,
        bastion_anchor: &mut Option<bool>,
        // FOUNDING PRESET v1: carries the REQUESTER as well as the site.
        // The founding's outcome is no longer unconditional — a refusal
        // (colony_exists / terrain) has to reach the player who asked, and
        // the post-loop is where the board, terrain and rtsim can all be
        // consulted at once, so the entity travels with the request.
        bastion_spawn: &mut Option<(specs::Entity, Vec3<f32>, u8)>,
        // bastion (B4): deferred designation ops — Some(kind) = place,
        // None = cancel (the job board can't be touched in the parallel join).
        // B5.6b-2: the third field is Some(extent) for surface-relative
        // placements (region = footprint XY + paint-plane hint in max.z);
        // None keeps the legacy literal-region path. Always None for cancel.
        // CHOP-FELLING (row 51.6, refining FR10): the fourth field carries a
        // resolved (base, FELL-SET) for the Area2D Chop path — one base-cut
        // job per tree, the whole set fells on completion; None for every
        // volume/legacy/cancel op.
        bastion_designations: &mut Vec<(
            common::bastion::Region,
            Option<common::bastion::DesignationKind>,
            Option<common::bastion::ZExtent>,
            Option<(Vec3<i32>, Vec<Vec3<i32>>)>,
        )>,
        // bastion (UI-4 → UI-5): inspector requests — targets (an entity uid
        // OR a world cell) gathered here, resolved + answered in the post-join
        // drain (the bastion_spawn deferral pattern; the payload sources —
        // JobBoard, item entities, terrain — can't be read in-join).
        bastion_inspects: &mut Vec<common::comp::bastion::BastionInspectTarget>,
        world: &Arc<World>,
        index: &IndexOwned,
        controller: Option<&mut Controller>,
        settings: &Read<'_, Settings>,
        build_areas: &Read<'_, AreasContainer<BuildArea>>,
        player_physics_setting: Option<&mut PlayerPhysicsSetting>,
        server_physics_forced: bool,
        maybe_admin: &Option<&Admin>,
        time_for_vd_changes: Instant,
        msg: ClientGeneral,
        player_physics: &mut Option<(Pos, Vel, Ori)>,
        // APEX-T5.1: (seen, admitted) client physics reports this tick.
        // A tally rather than a cohort: the handler must not learn what
        // cohort anyone is in, or the control's path could start to differ.
        physics_reports: &mut (u64, u64),
    ) -> Result<(), crate::error::Error> {
        let presence = match maybe_presence.as_deref_mut() {
            Some(g) => g,
            None => {
                debug!(?entity, "client is not in_game, ignoring msg");
                trace!(?msg, "ignored msg content");
                return Ok(());
            },
        };
        match msg {
            // Go back to registered state (char selection screen)
            ClientGeneral::ExitInGame => {
                deferred_events.push(DeferredInGameEvent::ExitIngame(event::ExitIngameEvent { entity }));
                client.send(ServerGeneral::ExitInGameSuccess)?;
                *maybe_presence = None;
            },
            ClientGeneral::SetViewDistance(view_distances) => {
                let clamped_vds = view_distances.clamp(settings.max_view_distance);

                presence
                    .terrain_view_distance
                    .set_target(clamped_vds.terrain, time_for_vd_changes);
                presence
                    .entity_view_distance
                    .set_target(clamped_vds.entity, time_for_vd_changes);

                // Correct client if its requested VD is too high.
                if view_distances.terrain != clamped_vds.terrain {
                    client.send(ServerGeneral::SetViewDistance(clamped_vds.terrain))?;
                }
            },
            ClientGeneral::ControllerInputs(inputs) => {
                if presence.kind.controlling_char()
                    && let Some(controller) = controller
                {
                    controller.inputs.update_with_new(*inputs);
                }
            },
            ClientGeneral::ControlEvent(event) => {
                if presence.kind.controlling_char()
                    && let Some(controller) = controller
                {
                    // Skip respawn if client entity is alive
                    let skip_respawn = matches!(event, ControlEvent::Respawn)
                        && healths.get(entity).is_none_or(|h| !h.is_dead);

                    if !skip_respawn {
                        controller.push_event(event);
                    }
                }
            },
            ClientGeneral::ControlAction(event) => {
                if presence.kind.controlling_char()
                    && let Some(controller) = controller
                {
                    controller.push_action(event);
                }
            },
            ClientGeneral::PlayerPhysics {
                pos,
                vel,
                ori,
                physics_generation,
                // APEX-T5.2: named so the destructure fails if the field is
                // removed, but deliberately unused HERE — admitting a
                // report is T3.6's generation decision, and mixing the
                // weather snapshot into it would make one gate two rules.
                weather_snapshot: _,
            } => {
                // APEX-T3.6 step 2/3: the report is admitted through the
                // typed generation gate, which distinguishes a STALE
                // report (computed before a correction the client had not
                // seen) from a FORGED one (a generation the server never
                // issued). The old bare equality could express neither.
                let generation_eligible = force_update.is_none_or(|force_update| {
                    matches!(
                        common::apex::physics_generation::PhysicsCorrectionStateV1::from_legacy_counter_v1(
                            force_update.counter(),
                        )
                        .admit_report_v1(common::apex::physics_generation::PhysicsStampV1 {
                            generation: physics_generation,
                        }),
                        common::apex::physics_generation::PhysicsAdmitV1::Eligible
                    )
                });
                physics_reports.0 += 1;
                if presence.kind.controlling_char()
                    && generation_eligible
                    && healths.get(entity).is_none_or(|h| !h.is_dead)
                    && is_rider.get(entity).is_none()
                    && is_volume_rider.get(entity).is_none()
                    && !server_physics_forced
                    && player_physics_setting
                        .as_ref()
                        .is_none_or(|s| !s.server_authoritative_physics_optin())
                {
                    *player_physics = Some((pos, vel, ori));
                    physics_reports.1 += 1;
                }
            },
            ClientGeneral::BreakBlock(pos) => {
                if let Some(comp_can_build) = can_build.get(entity)
                    && comp_can_build.enabled
                {
                    for area in comp_can_build.build_areas.iter() {
                        if let Some(old_block) = build_areas
                                .areas()
                                .get(*area)
                                // TODO: Make this an exclusive check on the upper bound of the AABB
                                // Vek defaults to inclusive which is not optimal
                                .filter(|aabb| aabb.contains_point(pos))
                                .and_then(|_| terrain.get(pos).ok())
                        {
                            let new_block = old_block.into_vacant();
                            // DET-ECS-015: deferred - applied in the sorted
                            // commit phase (first writer in canonical client
                            // order wins a contested cell).
                            terrain_writes.push((pos, new_block));
                        }
                    }
                }
            },
            ClientGeneral::PlaceBlock(pos, new_block) => {
                if let Some(comp_can_build) = can_build.get(entity)
                    && comp_can_build.enabled
                {
                    for area in comp_can_build.build_areas.iter() {
                        if build_areas
                                .areas()
                                .get(*area)
                                // TODO: Make this an exclusive check on the upper bound of the AABB
                                // Vek defaults to inclusive which is not optimal
                                .filter(|aabb| aabb.contains_point(pos))
                                .is_some()
                        {
                            // DET-ECS-015: deferred (see BreakBlock above).
                            terrain_writes.push((pos, new_block));
                        }
                    }
                }
            },
            ClientGeneral::UnlockSkill(skill) => {
                // FIXME: How do we want to handle the error?  Probably not by swallowing it.
                let _ = skill_set
                    .as_mut()
                    .map(|skill_set| {
                        SkillSet::unlock_skill_cow(skill_set, skill, |skill_set| skill_set.to_mut())
                    })
                    .transpose();
            },
            ClientGeneral::RequestSiteInfo(id) => {
                deferred_events.push(DeferredInGameEvent::SiteInfo(event::RequestSiteInfoEvent { entity, id }));
            },
            ClientGeneral::RequestPlayerPhysics {
                server_authoritative,
            } => {
                if let Some(setting) = player_physics_setting {
                    setting.client_optin = server_authoritative;
                }
            },
            ClientGeneral::RequestLossyTerrainCompression {
                lossy_terrain_compression,
            } => {
                presence.lossy_terrain_compression = lossy_terrain_compression;
            },
            ClientGeneral::UpdateMapMarker(update) => {
                deferred_events.push(DeferredInGameEvent::MapMarker(event::UpdateMapMarkerEvent { entity, update }));
            },
            ClientGeneral::SpectatePosition(pos) => {
                if let Some(admin) = maybe_admin
                    && admin.0 >= AdminRole::Moderator
                    && presence.kind == PresenceKind::Spectator
                    && let Some(position) = position
                {
                    position.0 = pos;
                }
            },
            ClientGeneral::SpectateEntity(uid) => {
                if let Some(admin) = maybe_admin
                    && admin.0 >= AdminRole::Moderator
                {
                    *spectating_entity = Some(uid);
                }
            },
            ClientGeneral::BastionCameraAnchor(anchor) => {
                // bastion (B1.6): god-camera terrain anchor — widens
                // terrain-request validation (see sys/msg/terrain.rs); never
                // moves the entity (unlike SpectatePosition above).
                // bastion (B3, §4 directive): entering/leaving god mode also
                // toggles the inert + invulnerable anchor state (marker +
                // permanent Invulnerability buff, applied post-loop in run()).
                let was_god = presence.bastion_terrain_anchor.is_some();
                let now_god = anchor.is_some();
                if was_god != now_god {
                    *bastion_anchor = Some(now_god);
                }
                presence.bastion_terrain_anchor = anchor;
            },
            // bastion (B2a): overseer interaction-surface stubs. The server
            // VALIDATES and ECHOES; behavior arrives with B4 (designations →
            // jobs), B13 (influence), B3/B2b (context verbs on entities).
            ClientGeneral::BastionPlaceDesignation {
                region,
                kind,
                z_extent,
            } => {
                let region = region.normalized();
                if let Some(extent) = z_extent {
                    // bastion (B5.6b-2): surface-relative path. `region`'s XY
                    // is the footprint, `max.z` the paint-plane hint; the
                    // volume is footprint × extent, resolved per column. The
                    // echo must carry the EXACT resolved bounds (they bound
                    // every generated job) or 3D cancel/erase on the echoed
                    // rect would miss jobs and orphan them — so resolve the
                    // bounds HERE (terrain is readable in this loop) and let
                    // the deferred board op recompute the same surfaces.
                    let footprint = (region.max.x - region.min.x + 1) as i64
                        * (region.max.y - region.min.y + 1) as i64;
                    // B5.6b-2.1 + flatten-hill (Ben live-bug #4): a flat-floor
                    // dig removes each column from the shared floor up to its
                    // TRUE crest, not to the paint plane — so measure the
                    // TALLEST crest over the footprint for an HONEST volume
                    // cap. The old paint-plane estimate under-counted a hill
                    // painted from its base (region.max.z sits near the base),
                    // which — once the surface resolution reaches the real
                    // crest — would silently over-generate jobs. Bounded by
                    // FLAT_SURFACE_SCAN_MAX inside column_flat_surface_z.
                    let max_crest_for = |floor: i32| -> i32 {
                        let mut m = floor;
                        for y in region.min.y..=region.max.y {
                            for x in region.min.x..=region.max.x {
                                if let Some(s) =
                                    crate::bastion_jobs::column_flat_surface_z(terrain, x, y, floor)
                                {
                                    m = m.max(s);
                                }
                            }
                        }
                        m
                    };
                    let nominal_levels = match extent.floor_z {
                        Some(floor) => {
                            ((max_crest_for(floor) - floor).max(0) as i64 + 1)
                                + extent.up as i64
                                + 8
                        },
                        None => extent.levels() as i64,
                    };
                    let volume = footprint * nominal_levels;
                    let mut extent = extent;
                    let mut resolved = (volume > 0
                        && volume <= common::bastion::MAX_DESIGNATION_VOLUME)
                        .then(|| {
                            crate::bastion_jobs::resolve_surface_bounds(
                                terrain,
                                region.min.xy(),
                                region.max.xy(),
                                region.max.z,
                                extent,
                            )
                        })
                        .flatten();
                    // B-LIVE1 (Ben's flat-mine drag false-reject): a flat
                    // floor derived from a camera pick plane ABOVE the
                    // ground lands above every column's surface — zero
                    // columns resolve and a perfectly valid drag rejected
                    // with "no terrain surface". Rare-path fallback:
                    // reinterpret the floor relative to the footprint's
                    // HIGHEST surface (the same "N deep from the ground I
                    // clicked" intent the client derives), re-gate the
                    // volume, re-resolve. The adjusted extent flows to the
                    // echo AND job gen together (echo-bounds invariant).
                    if resolved.is_none()
                        && let Some(orig_floor) = extent.floor_z
                    {
                        let mut max_surface: Option<i32> = None;
                        for y in region.min.y..=region.max.y {
                            for x in region.min.x..=region.max.x {
                                if let Some(s) = crate::bastion_jobs::column_surface_z(
                                    terrain,
                                    x,
                                    y,
                                    region.max.z,
                                ) {
                                    max_surface = Some(max_surface.map_or(s, |m: i32| m.max(s)));
                                }
                            }
                        }
                        if let Some(ms) = max_surface {
                            let clamped = ms - extent.down as i32;
                            if clamped < orig_floor {
                                extent.floor_z = Some(clamped);
                                // True-crest volume (as above) — the clamped
                                // floor now sits under the surfaces, so the
                                // dig reaches each column's real top.
                                let nominal = ((max_crest_for(clamped) - clamped).max(0) as i64
                                    + 1)
                                    + extent.up as i64
                                    + 8;
                                if footprint * nominal > 0
                                    && footprint * nominal
                                        <= common::bastion::MAX_DESIGNATION_VOLUME
                                {
                                    resolved = crate::bastion_jobs::resolve_surface_bounds(
                                        terrain,
                                        region.min.xy(),
                                        region.max.xy(),
                                        region.max.z,
                                        extent,
                                    );
                                }
                            }
                        }
                    }
                    if let Some(bounds) = resolved {
                        client.send_bastion_designation(bounds, kind, Some(extent))?;
                        bastion_designations.push((region, Some(kind), Some(extent), None));
                    } else {
                        client.send(ServerGeneral::server_msg(
                            common::comp::ChatType::CommandError,
                            common::comp::Content::Plain(format!(
                                "Designation rejected: volume {} outside 1..={} or no terrain \
                                 surface under the footprint",
                                volume,
                                common::bastion::MAX_DESIGNATION_VOLUME
                            )),
                        ))?;
                    }
                } else if kind == common::bastion::DesignationKind::Chop {
                    // ── CHOP redesign (FR10): the first Area2D kind ────────
                    // The paint is a PURE XY footprint; the fell-set = every
                    // whole tree ROOTED in it, resolved here via the World
                    // oracle (get_area_trees candidates → tree_valid_at
                    // confirm → bounded Wood+Leaves flood-fill). Each tree
                    // echoes as its OWN designation (region = the tree's
                    // tight AABB) — the client renders per-tree outline boxes
                    // and cancel-through-the-echo reaches exactly that tree,
                    // all with ZERO message-schema change.
                    let area = (region.max.x - region.min.x + 1) as i64
                        * (region.max.y - region.min.y + 1) as i64;
                    if area <= 0 || area > 64 * 64 {
                        client.send(ServerGeneral::server_msg(
                            common::comp::ChatType::CommandError,
                            common::comp::Content::Plain(format!(
                                "Chop area {} outside 1..={} tiles",
                                area,
                                64 * 64
                            )),
                        ))?;
                    } else {
                        // The SHARED detection (bastion_chop::detect_trees) —
                        // the harness hook calls the same fn (B17: the tested
                        // path IS the shipping path).
                        let trees = crate::bastion_chop::detect_trees(
                            world,
                            index,
                            terrain,
                            region.min.xy(),
                            region.max.xy(),
                        );
                        // FOUNDING PRESET F8-C2 (2026-08-12): the chop path had NO
                        // server-side witness for EITHER outcome. It echoes per-tree
                        // BastionDesignation messages to the CLIENT and never emits the
                        // shared `designation placed` line, so `designation placed
                        // kind=Chop` has ZERO occurrences whether chop works or fails --
                        // proven by running both (arena: refused; real worldgen: 6 trees
                        // designated). Any scored run reading the SERVER log is blind to
                        // chop entirely, which is the name-the-line law broken on the
                        // server side.
                        //
                        // Both arms emit, by name, so an absence in the log now means
                        // "the message never arrived" rather than "chop is unwitnessed".
                        // PER-TREE CELL COUNTS, not merely a tree count: a bar
                        // reading `trees=N` alone would pass on N EMPTY
                        // resolutions. These come from `tree_fell_set` reading
                        // real blocks, so they are what make the count
                        // non-vacuous — and on the arena they should equal the
                        // trunk heights in `RESOURCED_TREES`.
                        tracing::info!(
                            ?region,
                            trees = trees.len(),
                            cells = ?trees.iter().map(|(_, _, c)| c.len()).collect::<Vec<_>>(),
                            "bastion: chop designation resolved"
                        );
                        if trees.is_empty() {
                            tracing::info!(
                                ?region,
                                reason = "no_trees_rooted",
                                "bastion: chop designation refused"
                            );
                            client.send(ServerGeneral::server_msg(
                                common::comp::ChatType::CommandInfo,
                                common::comp::Content::Plain(
                                    "No trees rooted in the marked area.".into(),
                                ),
                            ))?;
                        }
                        for (aabb, base, cells) in trees {
                            client.send_bastion_designation(aabb, kind, None)?;
                            bastion_designations.push((
                                aabb,
                                Some(kind),
                                None,
                                Some((base, cells)),
                            ));
                        }
                    }
                } else {
                    let volume = region.volume();
                    if volume > 0 && volume <= common::bastion::MAX_DESIGNATION_VOLUME {
                        client.send_bastion_designation(region, kind, None)?;
                        // bastion (B4): job generation happens post-loop.
                        bastion_designations.push((region, Some(kind), None, None));
                    } else {
                        client.send(ServerGeneral::server_msg(
                            common::comp::ChatType::CommandError,
                            common::comp::Content::Plain(format!(
                                "Designation rejected: volume {} outside 1..={}",
                                volume,
                                common::bastion::MAX_DESIGNATION_VOLUME
                            )),
                        ))?;
                    }
                }
            },
            ClientGeneral::BastionApplyInfluence { target, kind } => {
                if target.map(|e| e.is_finite()).reduce_and() {
                    client.send(ServerGeneral::server_msg(
                        common::comp::ChatType::CommandInfo,
                        common::comp::Content::Plain(format!(
                            "[bastion stub] influence {} at ({:.0}, {:.0}, {:.0})",
                            kind.label(),
                            target.x,
                            target.y,
                            target.z
                        )),
                    ))?;
                }
            },
            ClientGeneral::BastionCancelDesignation { region } => {
                // bastion (B4): jobs removed + claims released post-loop.
                let region = region.normalized();
                bastion_designations.push((region, None, None, None));
                // bastion (B5.5): echo the removal so the client subtracts
                // it from its overlay rects (mirrors the place echo above).
                client.send_bastion_designation_removed(region)?;
                client.send(ServerGeneral::server_msg(
                    common::comp::ChatType::CommandInfo,
                    common::comp::Content::Plain("Designations cancelled.".to_string()),
                ))?;
            },
            ClientGeneral::BastionInspect { target } => {
                // bastion (UI-4): deferred to the post-join drain — the
                // reply needs storages the par_join must not touch.
                bastion_inspects.push(target);
            },
            ClientGeneral::BastionSpawnColony { pos, count } => {
                // bastion (B3): validated here, spawned post-loop (rtsim
                // resource can't be touched inside the parallel join).
                if presence.bastion_terrain_anchor.is_some()
                    && pos.map(|e| e.is_finite()).reduce_and()
                    && (1..=16).contains(&count)
                {
                    *bastion_spawn = Some((entity, pos, count));
                    // FOUNDING PRESET v1: this is now an ACKNOWLEDGEMENT,
                    // not the outcome — the founding can still refuse
                    // (colony_exists / terrain) in the post-loop, where the
                    // board, terrain and rtsim are all readable. The
                    // refusal carries its own player-visible message from
                    // there.
                    client.send(ServerGeneral::server_msg(
                        common::comp::ChatType::CommandInfo,
                        common::comp::Content::Plain(format!(
                            "Founding colony: {count} settlers arriving."
                        )),
                    ))?;
                } else {
                    client.send(ServerGeneral::server_msg(
                        common::comp::ChatType::CommandError,
                        common::comp::Content::Plain(
                            "Colony spawn rejected (need god mode; count 1..=16)".to_string(),
                        ),
                    ))?;
                }
            },
            ClientGeneral::BastionContextAction { target, verb } => {
                let target_desc = match target {
                    common::bastion::ContextTarget::Entity(uid) => format!("entity {uid}"),
                    common::bastion::ContextTarget::Block(pos) => {
                        format!("block ({}, {}, {})", pos.x, pos.y, pos.z)
                    },
                };
                client.send(ServerGeneral::server_msg(
                    common::comp::ChatType::CommandInfo,
                    common::comp::Content::Plain(format!(
                        "[bastion stub] {} on {}",
                        verb.label(),
                        target_desc
                    )),
                ))?;
            },
            ClientGeneral::SetBattleMode(battle_mode) => {
                deferred_events.push(DeferredInGameEvent::BattleMode(
                    event::SetBattleModeEvent { entity, battle_mode },
                ));
            },
            ClientGeneral::RequestCharacterList
            | ClientGeneral::CreateCharacter { .. }
            | ClientGeneral::EditCharacter { .. }
            | ClientGeneral::DeleteCharacter(_)
            | ClientGeneral::Character(_, _)
            | ClientGeneral::Spectate(_)
            | ClientGeneral::TerrainChunkRequest { .. }
            | ClientGeneral::LodZoneRequest { .. }
            | ClientGeneral::ChatMsg(_)
            | ClientGeneral::Command(..)
            | ClientGeneral::Terminate
            | ClientGeneral::RequestPlugins(_)
            | ClientGeneral::RequestPluginArtifacts(_)
            // T3.4.19: the ack belongs to the General stream; arriving
            // here it is as wrong as any other misrouted message.
            | ClientGeneral::CheckpointCommitAck(_) => {
                debug!("Kicking possibly misbehaving client due to invalid client in game request");
                deferred_events.push(DeferredInGameEvent::Disconnect(
                    event::ClientDisconnectEvent(
                        entity,
                        common::comp::DisconnectReason::NetworkError,
                    ),
                ));
            },
        }
        Ok(())
    }
}

/// bastion (UI-5, row 62.2): resolve a world CELL to whatever Bastion-tracked
/// object sits there, for the Universal Debug Inspector. Priority: an active
/// job at the exact cell (most actionable) → a stockpile zone (with contents,
/// the 51.64 legibility fix) → a farm plot (with crop growth) → a registered
/// fell-set → nothing. READ-ONLY; runs in the post-join drain at request
/// cadence (~1Hz per open panel), so the O(items) contents sum is affordable.
#[allow(clippy::too_many_arguments)]
fn resolve_cell_inspect(
    cell: Vec3<i32>,
    board: &crate::bastion_jobs::JobBoard,
    terrain: &TerrainGrid,
    pickup_items: &ReadStorage<'_, common::comp::PickupItem>,
    positions: &WriteStorage<'_, common::comp::Pos>,
    id_maps: &IdMaps,
    colonists: &ReadStorage<'_, common::comp::Colonist>,
) -> Option<common::comp::bastion::BastionInspectKind> {
    use common::{
        comp::bastion::{
            BastionFarmInspect, BastionFellSetInspect, BastionInspectKind, BastionJobInspect,
            BastionStockpileInspect,
        },
        vol::ReadVol,
    };
    use specs::Join;

    // 1. An active job in the clicked XY column — the most actionable target.
    // Matched by XY (a top-down overseer click) with the nearest z inside a
    // generous window, so click-projection z-imprecision still lands the job.
    if let Some((id, job)) = board
        .jobs
        .iter()
        .filter(|(_, j)| {
            j.pos.x == cell.x && j.pos.y == cell.y && (j.pos.z - cell.z).abs() <= 6
        })
        .min_by_key(|(_, j)| (j.pos.z - cell.z).abs())
    {
        let claimant = job.claimed_by.and_then(|uid| {
            id_maps
                .uid_entity(uid)
                .and_then(|e| colonists.get(e))
                .map(|c| c.0.name.clone())
        });
        return Some(BastionInspectKind::Job(BastionJobInspect {
            work: job.work,
            pos: job.pos,
            progress: job.progress,
            claimant,
            unreachable: job.unreachable,
            needs_materials: job.needs_materials,
            is_access: job.is_access,
            stuck_strikes: job.stuck_strikes,
            blocked_by: board.blocked_by(job.pos),
            benched_since_tick: board.benched_since.get(id).copied(),
            // ROW B′: a direct field read off `job`, no lookup --
            // cheaper than Row B's HashMap probe it replaces.
            benched_until_tick: job.benched_until_tick,
        }));
    }

    // 2. A stockpile zone → its contents (the 51.64 legibility fix: a painted
    // stockpile finally shows WHAT it holds, grouped by item, most first).
    if let Some(zid) = board.stockpile_at(cell)
        && let Some((_, region)) = board.stockpiles.iter().find(|(id, _)| *id == zid)
    {
        let mut tally: Vec<(String, u32)> = Vec::new();
        let mut total = 0u32;
        for (item, pos) in (pickup_items, positions).join() {
            let ip = pos.0.map(|e| e.floor() as i32);
            if !region.contains_point_xy(ip) {
                continue;
            }
            let amount = item.amount() as u32;
            total += amount;
            let def = item
                .item()
                .item_definition_id()
                .itemdef_id()
                .unwrap_or("?")
                .to_string();
            if let Some(slot) = tally.iter_mut().find(|(d, _)| *d == def) {
                slot.1 += amount;
            } else {
                tally.push((def, amount));
            }
        }
        tally.sort_by(|a, b| b.1.cmp(&a.1));
        return Some(BastionInspectKind::Stockpile(BastionStockpileInspect {
            contents: tally,
            total,
        }));
    }

    // 3. A farm plot → the sampled cell's crop growth stage (flat plot: XY area).
    if let Some((_, region)) = board.farms.iter().find(|(_, r)| r.contains_point_xy(cell)) {
        let growth = terrain.get(cell).ok().and_then(|b| {
            b.get_attr::<common::terrain::sprite::Growth>()
                .ok()
                .map(|g| g.0)
        });
        let cells = ((region.max.x - region.min.x + 1).max(0)
            * (region.max.y - region.min.y + 1).max(0)) as u32;
        return Some(BastionInspectKind::Farm(BastionFarmInspect {
            growth,
            cells,
        }));
    }

    // 4. A registered fell-set (a tree queued / mid-timber).
    if let Some(fell) = board
        .chop_fell_sets
        .values()
        .find(|cf| cf.cells.iter().any(|c| c.x == cell.x && c.y == cell.y))
    {
        return Some(BastionInspectKind::FellSet(BastionFellSetInspect {
            remaining: fell.cells.len() as u32,
            total: fell.wood_count,
        }));
    }

    None
}

/// This system will handle new messages from clients
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Events<'a>,
        (
            ReadExpect<'a, TerrainGrid>,
            ReadExpect<'a, SlowJobPool>,
            ReadExpect<'a, EditableSettings>,
            ReadExpect<'a, SemanticIngressMetricsV1>,
            // APEX-T5.1
            ReadExpect<'a, crate::physics_cohort::PhysicsCohortRegistryV1>,
            ReadExpect<'a, crate::physics_cohort::PhysicsCohortMetricsV1>,
        ),
        (
            Read<'a, IdMaps>,
            Read<'a, DeltaTime>,
            Read<'a, Settings>,
            Read<'a, AreasContainer<BuildArea>>,
            // COLONY-TICK row: stamps `BastionColonyInspect::tick`. This is
            // `crate::Tick` (re-exported from bastion-server), the SAME
            // resource the `bastion F3-BRANCH` emit stamps -- so a colony
            // sample and a branch transition can be placed on one timeline.
            // A parallel clock would align plausibly and wrongly.
            Read<'a, crate::Tick>,
            // ITEM 32: the favor pool, so the colony panel can show the
            // player the resource their god-powers spend.
            Read<'a, crate::bastion_jobs::DivineFavor>,
        ),
        ReadStorage<'a, CanBuild>,
        WriteStorage<'a, ForceUpdate>,
        ReadStorage<'a, Is<Rider>>,
        ReadStorage<'a, Is<VolumeRider>>,
        WriteStorage<'a, SkillSet>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Scale>,
        Write<'a, BlockChange>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Presence>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, SpectatingEntity>,
        Write<'a, PlayerPhysicsSettings>,
        TerrainPersistenceData<'a>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Admin>,
        // bastion (B3): god-anchor marker + buff timing + colony spawning;
        // (B4) the job board for designation ops.
        (
            WriteStorage<'a, common::comp::BastionGodAnchor>,
            Read<'a, common::resources::Time>,
            // #105: PickupItem::new needs this for the founding seed drop.
            ReadExpect<'a, common::resources::ProgramTime>,
            specs::WriteExpect<'a, crate::rtsim::RtSim>,
            Write<'a, crate::bastion_jobs::JobBoard>,
            // CHOP redesign (FR10): the tree oracle — read-only Arc, safe
            // inside the parallel client join (B10: no shared mutable state).
            ReadExpect<'a, Arc<World>>,
            ReadExpect<'a, IndexOwned>,
            // bastion (UI-4, row 62): the inspector's payload sources —
            // READ-ONLY, touched only in the post-join drain at request
            // cadence (~1Hz per open panel), never inside the par_join.
            ReadStorage<'a, common::comp::Colonist>,
            ReadStorage<'a, common::comp::bastion::Needs>,
            ReadStorage<'a, common::comp::bastion::Mood>,
            ReadStorage<'a, common::comp::bastion::Arbiter>,
            ReadStorage<'a, common::rtsim::RtSimEntity>,
            // UI-5 (row 62.2): dropped-item entities — a stockpile cell's
            // contents are summed from these in the same post-join drain.
            ReadStorage<'a, common::comp::PickupItem>,
            ReadStorage<'a, common::uid::Uid>,
            // STATUS-SURFACE: energy meter + the tick for status-stamp TTL.
            ReadStorage<'a, common::comp::Energy>,
            specs::Read<'a, crate::Tick>,
            // engine-list T3.58: the colonist's current job assignment,
            // for the InspectorOwnershipV1 evidence key.
            ReadStorage<'a, common::comp::bastion::ActiveJob>,
            // ITEM 13 (health branch): the inspector could report every need a
            // colonist has EXCEPT the one that kills them. `Health` was already
            // in this system's outer SystemData, but the payload is built inside
            // a closure over the bastion sub-tuple, so it is taken here under the
            // same `insp_` convention as its siblings rather than reached for
            // across the borrow.
            ReadStorage<'a, Health>,
        ),
    );

    const NAME: &'static str = "msg::in_game";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            events,
            (terrain, slow_jobs, editable_settings, semantic_metrics, cohort_registry, cohort_metrics),
            (id_maps, dt, settings, build_areas, bastion_tick, insp_favor),
            can_build,
            mut force_updates,
            is_rider,
            is_volume_rider,
            mut skill_sets,
            healths,
            bodies,
            scales,
            mut block_changes,
            mut positions,
            mut velocities,
            mut orientations,
            mut presences,
            mut clients,
            mut controllers,
            mut spectating_entities,
            mut player_physics_settings_,
            mut terrain_persistence,
            players,
            admins,
            (
                mut god_anchors,
                time,
                program_time,
                mut rtsim,
                mut job_board,
                world,
                index,
                insp_colonists,
                insp_needs,
                insp_moods,
                insp_arbiters,
                insp_rtsim_entities,
                insp_pickup_items,
                insp_uids,
                insp_energies,
                insp_tick,
                insp_active_jobs,
                insp_healths,
            ),
        ): Self::SystemData,
    ) {
        let time_for_vd_changes = Instant::now();


        let player_physics_settings = &*player_physics_settings_;
        let mut deferred_updates = (
            &entities,
            &mut clients,
            (&mut presences).maybe(),
            players.maybe(),
            admins.maybe(),
            (&skill_sets).maybe(),
            (&mut positions).maybe(),
            (&mut velocities).maybe(),
            (&mut orientations).maybe(),
            (&mut controllers).maybe(),
            (&mut force_updates).maybe(),
        )
            .join()
            // NOTE: Required because Specs has very poor work splitting for sparse joins.
            .par_bridge()
            .map_init(
                // DET-EVT-005: the parallel section emits NOTHING — all
                // events buffer per client and emit at the sorted commit.
                || (),
                |(), (
                    entity,
                    client,
                    mut maybe_presence,
                    maybe_player,
                    maybe_admin,
                    skill_set,
                    ref mut pos,
                    ref mut vel,
                    ref mut ori,
                    ref mut controller,
                    ref mut force_update,
                )| {
                    let old_player_physics_setting = maybe_player.map(|p| {
                        player_physics_settings
                            .settings
                            .get(&p.uuid())
                            .copied()
                            .unwrap_or_default()
                    });
                    let mut new_player_physics_setting = old_player_physics_setting;
                    let is_server_physics_forced = maybe_player.is_none_or(|p| editable_settings.server_physics_force_list.contains_key(&p.uuid()));
                    // If an `ExitInGame` message is received this is set to `None` allowing further
                    // ingame messages to be ignored.
                    let mut clearable_maybe_presence = maybe_presence.as_deref_mut();
                    let mut skill_set = skill_set.map(Cow::Borrowed);
                    let mut player_physics = None;
                    let mut physics_reports = (0u64, 0u64);
                    let mut spectating_entity = None;
                    let mut bastion_anchor = None;
                    let mut bastion_spawn = None;
                    let mut bastion_designations = Vec::new();
                    let mut bastion_inspects = Vec::new();
                    let mut terrain_writes = Vec::new();
                    let mut deferred_events = Vec::new();
                    let _ = super::try_recv_all_dispatch(client, 2, SemanticStreamIdV1::InGame, &semantic_metrics, |client, msg| {
                        Self::handle_client_in_game_msg(
                            &mut deferred_events,
                            entity,
                            client,
                            &mut clearable_maybe_presence,
                            &terrain,
                            &can_build,
                            &is_rider,
                            &is_volume_rider,
                            force_update.as_ref(),
                            &mut skill_set,
                            &healths,
                            &mut terrain_writes,
                            pos.as_deref_mut(),
                            &mut spectating_entity,
                            &mut bastion_anchor,
                            &mut bastion_spawn,
                            &mut bastion_designations,
                            &mut bastion_inspects,
                            &*world,
                            &*index,
                            controller.as_deref_mut(),
                            &settings,
                            &build_areas,
                            new_player_physics_setting.as_mut(),
                            is_server_physics_forced,
                            &maybe_admin,
                            time_for_vd_changes,
                            msg,
                            &mut player_physics,
                            &mut physics_reports,
                        )
                    });

                    // APEX-T5.1: attribute this tick's reports to the player's
                    // cohort. Assignment reads the OPT-IN ONLY -- the force
                    // list is a moderation tool and is deliberately not an
                    // input here (see physics_cohort's module doc). Nothing
                    // below branches on the result.
                    if physics_reports.0 > 0
                        && let Some(player) = maybe_player
                    {
                        let cohort_lookup = cohort_registry.lookup_v1(
                            player.uuid(),
                            crate::physics_cohort::CohortInputsV1 {
                                opted_in: new_player_physics_setting
                                    .is_some_and(|s| s.server_authoritative_physics_optin()),
                            },
                        );
                        cohort_metrics.record_reports_v1(
                            cohort_lookup,
                            physics_reports.1,
                            physics_reports.0 - physics_reports.1,
                        );
                    }

                    if let Some((new_pos, new_vel, new_ori)) = player_physics
                        && let Some(old_pos) = pos.as_deref_mut()
                        && let Some(old_vel) = vel.as_deref_mut()
                        && let Some(old_ori) = ori.as_deref_mut()
                    {
                        enum Rejection {
                            TooFar { old: Vec3<f32>, new: Vec3<f32> },
                            TooFast { vel: Vec3<f32> },
                            InsideTerrain,
                        }

                        let rejection = if maybe_admin.is_some() {
                            None
                        } else {
                            // Reminder: review these frequently to ensure they're reasonable
                            const MAX_H_VELOCITY: f32 = 75.0;
                            const MAX_V_VELOCITY: std::ops::Range<f32> = -100.0..80.0;

                            'rejection: {
                                let is_velocity_ok = new_vel.0.xy().magnitude_squared() < MAX_H_VELOCITY.powi(2)
                                    && MAX_V_VELOCITY.contains(&new_vel.0.z);

                                if !is_velocity_ok {
                                    break 'rejection Some(Rejection::TooFast { vel: new_vel.0 });
                                }

                                // How far the player is permitted to stray from the correct position (perhaps due to
                                // latency problems).
                                const POSITION_THRESHOLD: f32 = 16.0;

                                // The position can either be sensible with respect to either the old or the new
                                // velocity such that we don't punish for edge cases after a sudden change
                                let is_position_ok = [old_vel.0, new_vel.0]
                                    .into_iter()
                                    .any(|ref_vel| {
                                        let rpos = new_pos.0 - old_pos.0;
                                        // Determine whether the change in position is broadly consistent with both
                                        // the magnitude and direction of the velocity, with appropriate thresholds.
                                        LineSegment3 {
                                            start: Vec3::zero(),
                                            end: ref_vel * dt.0,
                                        }
                                            .projected_point(rpos)
                                            // + 1.5 accounts for minor changes in position without corresponding
                                            // velocity like block hopping/snapping
                                            .distance_squared(rpos) < (rpos.magnitude() * 0.5 + 1.5 + POSITION_THRESHOLD).powi(2)
                                    });

                                if !is_position_ok {
                                    break 'rejection Some(Rejection::TooFar { old: old_pos.0, new: new_pos.0 });
                                }

                                // Checks that are only relevant if the position changed
                                if new_pos.0 != old_pos.0 {
                                    // Reject updates that would move the entity into terrain
                                    let scale = scales.get(entity).map_or(1.0, |s| s.0);
                                    let min_z = new_pos.0.z as i32;
                                    let height = bodies.get(entity).map_or(0.0, |b| b.height()) * scale;
                                    let head_pos_z = (new_pos.0.z + height) as i32;

                                    if !(min_z..=head_pos_z).any(|z| {
                                        let pos = new_pos.0.as_().with_z(z);

                                        terrain
                                            .get(pos)
                                            .is_ok_and(|block| block.is_fluid())
                                    }) {
                                        break 'rejection Some(Rejection::InsideTerrain);
                                    }
                                }

                                None
                            }
                        };

                        if let Some(rejection) = rejection {
                            // TODO: Log when false positives aren't generated often
                            let alias = maybe_player.map(|p| &p.alias);
                            match rejection {
                                Rejection::TooFar { old, new } => warn!("Rejected physics for player {alias:?} (new position {new:?} is too far from old position {old:?})"),
                                Rejection::TooFast { vel } => warn!("Rejected physics for player {alias:?} (new velocity {vel:?} is too fast)"),
                                Rejection::InsideTerrain => warn!("Rejected physics for player {alias:?}: Inside terrain."),
                            }

                            /*
                            // Perhaps this is overzealous?
                            if let Some(mut setting) = new_player_physics_setting.as_mut() {
                                setting.server_force = true;
                                warn!("Switching player {alias:?} to server-side physics");
                            }
                            */

                            // Reject the change and force the server's view of the physics state
                            force_update.as_mut().map(|fu| fu.update());
                        } else {
                            *old_pos = new_pos;
                            *old_vel = new_vel;
                            *old_ori = new_ori;
                        }
                    }

                    // Ensure deferred view distance changes are applied (if the
                    // requsite time has elapsed).
                    if let Some(presence) = maybe_presence {
                        presence.terrain_view_distance.update(time_for_vd_changes);
                        presence.entity_view_distance.update(time_for_vd_changes);
                    }

                    // Return the possibly modified skill set, and possibly modified server physics
                    // settings.
                    let skill_set_update = skill_set.and_then(|skill_set| match skill_set {
                        Cow::Borrowed(_) => None,
                        Cow::Owned(skill_set) => Some((entity, skill_set)),
                    });
                    // NOTE: Since we pass Option<&mut _> rather than &mut Option<_> to
                    // handle_client_in_game_msg, and the new player was initialized to the same
                    // value as the old setting , we know that either both the new and old setting
                    // are Some, or they are both None.
                    let physics_update = maybe_player.map(|p| p.uuid())
                        .zip(new_player_physics_setting
                             .filter(|_| old_player_physics_setting != new_player_physics_setting));
                     let spectating_entity_update = spectating_entity.map(|e| (entity, e));
                    let bastion_anchor_update = bastion_anchor.map(|on| (entity, on));
                    let bastion_inspects_update = (!bastion_inspects.is_empty())
                        .then_some((entity, bastion_inspects));
                    (
                        // DET-ECS-014 (v5 deep-pass): stable sort key — the
                        // collected order below is Rayon completion order.
                        entity.id(),
                        skill_set_update,
                        spectating_entity_update,
                        physics_update,
                        bastion_anchor_update,
                        bastion_spawn,
                        bastion_designations,
                        bastion_inspects_update,
                        terrain_writes,
                        deferred_events,
                    )
                },
            )
            // NOTE: Would be nice to combine this with the map_init somehow, but I'm not sure if
            // that's possible.
            .filter(|(_, x, y, z, w, v, d, i, tw, ev)| {
                x.is_some() || y.is_some() || z.is_some() || w.is_some() || v.is_some()
                    || !d.is_empty()
                    || i.is_some()
                    || !tw.is_empty()
                    || !ev.is_empty()
            })
            // NOTE: I feel like we shouldn't actually need to allocate here, but hopefully this
            // doesn't turn out to be important as there shouldn't be that many connected clients.
            // The reason we can't just use unzip is that the two sides might be different lengths.
            .collect::<Vec<_>>();
        // DET-ECS-014 (v5 deep-pass): GATHER-SORT-COMMIT. par_bridge
        // explicitly does not preserve order, so the collected per-client
        // updates (and the deferred event/designation apply order below)
        // arrived in worker-completion order. Sort by the stable per-client
        // entity id before applying — the apply order is now a pure function
        // of the client set, independent of worker timing.
        deferred_updates.sort_unstable_by_key(|(entity_id, ..)| *entity_id);
        let player_physics_settings = &mut *player_physics_settings_;
        // Deferred updates to skillsets and player physics.
        //
        // NOTE: It is an invariant that there is at most one client entry per player
        // uuid; since we joined on clients, it follows that there's just one update
        // per uuid, so the physics update is sound and doesn't depend on evaluation
        // order, even though we're not updating directly by entity or uid (note that
        // for a given entity, we process messages serially).
        let mut post_emitters = events.get_emitters();
        deferred_updates.iter_mut().for_each(
            |(
                _entity_id,
                skill_set_update,
                spectating_entity_update,
                physics_update,
                bastion_anchor_update,
                bastion_spawn_update,
                bastion_designation_updates,
                bastion_inspects_update,
                terrain_writes,
                deferred_events,
            )| {
                // DET-EVT-005: emit the buffered events in the SORTED commit
                // order — bus chronology is now a pure function of the
                // client set, not worker timing.
                for ev in deferred_events.drain(..) {
                    match ev {
                        DeferredInGameEvent::ExitIngame(e) => post_emitters.emit(e),
                        DeferredInGameEvent::SiteInfo(e) => post_emitters.emit(e),
                        DeferredInGameEvent::MapMarker(e) => post_emitters.emit(e),
                        DeferredInGameEvent::BattleMode(e) => post_emitters.emit(e),
                        DeferredInGameEvent::Disconnect(e) => post_emitters.emit(e),
                    }
                }
                // DET-ECS-015: apply the deferred terrain writes in the
                // SORTED commit order - same-cell conflicts resolve by the
                // canonical client order (first writer wins via try_set),
                // never by mutex-acquisition timing.
                for (pos, block) in terrain_writes.drain(..) {
                    let _was_set = block_changes.try_set(pos, block).is_some();
                    #[cfg(feature = "persistent_world")]
                    if _was_set
                        && let Some(terrain_persistence) = terrain_persistence.as_mut()
                    {
                        terrain_persistence.set_block(pos, block);
                    }
                }
                if let Some((entity, new_skill_set)) = skill_set_update {
                    // We know this exists, because we already iterated over it with the skillset
                    // lock taken, so we can ignore the error.
                    //
                    // Note that we replace rather than just updating.  This is in order to avoid
                    // dropping here; we'll drop later on a background thread, in case skillsets are
                    // slow to drop.
                    skill_sets
                        .get_mut(*entity)
                        .map(|mut old_skill_set| mem::swap(&mut *old_skill_set, new_skill_set));
                }
                if let &mut Some((entity, spectating_uid)) = spectating_entity_update {
                    if let Some(uid) = spectating_uid
                        && let Some(spectated_entity) = id_maps.uid_entity(uid)
                    {
                        // We know this exists, so can ignore the error.
                        let _ =
                            spectating_entities.insert(entity, SpectatingEntity(spectated_entity));
                    } else {
                        spectating_entities.remove(entity);
                    }
                }
                if let &mut Some((uuid, player_physics_setting)) = physics_update {
                    // We don't necessarily know this exists, but that's fine, because dropping
                    // player physics is a no op.
                    player_physics_settings
                        .settings
                        .insert(uuid, player_physics_setting);
                }
                if let &mut Some((entity, god_on)) = bastion_anchor_update {
                    // bastion (B3, §4 standing directive): while the god
                    // camera is anchored the avatar is an inert, invulnerable
                    // anchor — marker for the world-ignores filters, plus a
                    // permanent vanilla Invulnerability buff (100% damage
                    // reduction; agents also drop invulnerable targets).
                    use common::comp::{
                        Buff, BuffChange, BuffData, BuffKind, BuffSource, buff::DestInfo,
                    };
                    if god_on {
                        let _ = god_anchors.insert(entity, common::comp::BastionGodAnchor);
                        post_emitters.emit(event::BuffEvent {
                            entity,
                            buff_change: BuffChange::Add(Buff::new(
                                BuffKind::Invulnerability,
                                BuffData::new(1.0, None),
                                vec![],
                                BuffSource::Command,
                                *time,
                                DestInfo {
                                    stats: None,
                                    mass: None,
                                },
                                None,
                            )),
                        });
                    } else {
                        god_anchors.remove(entity);
                        post_emitters.emit(event::BuffEvent {
                            entity,
                            buff_change: BuffChange::RemoveByKind(BuffKind::Invulnerability),
                        });
                    }
                }
                if let &mut Some((requester, pos, count)) = bastion_spawn_update {
                    // ─── FOUNDING PRESET v1 (ITEM-FOUNDING-PRESET-PACKET.md)
                    //
                    // The player path gets the script path's kit. Order
                    // matters and is ruled: BOUNDARY, then SITE, then
                    // placement — a refused founding must mutate NOTHING
                    // (no half-colony, no orphan seeds), so both refusals
                    // are decided before the first designation is placed.
                    use crate::bastion_founding_preset as preset;

                    let origin_xy = preset::origin_xy(pos);
                    // B1: the datum is DERIVED. `pos.z` is only the hint
                    // that centres the resolver's window.
                    let datum = preset::resolve_datum(&terrain, origin_xy, pos.z.floor() as i32);

                    // §4, THE ONE-COLONY BOUNDARY. Reads rtsim colonist
                    // records — the persistent half (see
                    // `RtSim::bastion_colony_exists`). Deliberately
                    // FIRST: "your colony already lives here" is true
                    // regardless of what the ground looks like.
                    let refusal = if rtsim.bastion_colony_exists() {
                        Some((preset::FoundingRefusal::ColonyExists, None))
                    } else {
                        {
                            // F2: THE EMIT COVERS BOTH ARMS.
                            //
                            // This block used to live inside `Some(datum_z)`,
                            // so an origin whose own column resolved no
                            // surface was refused with NO relief line at all —
                            // 16 of 24 search attempts were invisible to the
                            // instrument built to make absence visible. An
                            // absent line and a zero line render identically.
                            //
                            // When the datum is unresolved we survey against
                            // the HINT — the same window `column_surface_z`
                            // was already given — and say so in
                            // `datum_resolved`, rather than fabricating a
                            // datum that would read as resolved.
                            let datum_resolved = datum.is_some();
                            let survey_z = datum.unwrap_or(pos.z.floor() as i32);
                            let origin = Vec3::new(origin_xy.x, origin_xy.y, survey_z);
                            // ONE PRODUCER, TWO CONSUMERS (worldgen row
                            // §5): the verdict below and the relief emit
                            // read the SAME measurement. A second
                            // function recomputing relief beside the
                            // real one is the F8 defect.
                            let relief = preset::survey_site(&terrain, origin);
                            // WITHOUT THIS EMIT the refusal carries no
                            // number: `reason="terrain"` cannot separate
                            // a 2-block slope from a 90-block lakebed,
                            // nor deviation from absence. `branch` is
                            // what makes the water prediction testable.
                            tracing::info!(
                                ?origin,
                                datum = relief.datum,
                                datum_resolved,
                                columns = relief.columns,
                                resolved = relief.resolved,
                                min_dev = ?relief.min_dev,
                                max_dev = ?relief.max_dev,
                                worst = ?relief.worst,
                                submerged = relief.submerged,
                                branch = relief.branch().name(),
                                "bastion: founding site relief"
                            );
                            if datum_resolved {
                                // §3.2 TERRAIN VALIDATION over every plot
                                // column, not just the centre.
                                match relief.verdict() {
                                    Ok(()) => None,
                                    Err((refusal, column)) => Some((refusal, Some(column))),
                                }
                            } else {
                                // No resolvable surface at F itself (open
                                // water, void, unloaded chunk) is the same
                                // refusal by the same name. UNCHANGED — this
                                // row adds a witness, it does not move a
                                // decision.
                                Some((preset::FoundingRefusal::Terrain, Some(origin_xy)))
                            }
                        }
                    };

                    if let Some((refusal, column)) = refusal {
                        // REFUSAL IS A FIRST-CLASS OUTCOME (§3.2): its own
                        // emit, and a player-visible message that names the
                        // reason. Refusal-needs-refusal-aware-consumers —
                        // the UI shows it, the log carries it, and A4/A5
                        // read the `reason=` field by name.
                        tracing::info!(
                            reason = refusal.reason(),
                            ?pos,
                            ?column,
                            "bastion: founding refused"
                        );
                        if let Some(client) = clients.get(requester) {
                            let _ = client.send(ServerGeneral::server_msg(
                                common::comp::ChatType::CommandError,
                                common::comp::Content::Plain(refusal.player_message().to_string()),
                            ));
                        }
                        // Nothing in the `else` runs: no colonists, no
                        // stock, no presence. A refused founding leaves the
                        // world exactly as it found it. (An `else`, not an
                        // early return — the inspector drain below still
                        // owes this client its answers this tick.)
                    } else {
                    let origin = Vec3::new(
                        origin_xy.x,
                        origin_xy.y,
                        datum.expect("a refusal was returned above when the datum is unresolvable"),
                    );

                    // THE PRESET (§1): placed from the plot template, in
                    // template order, through the SAME `place_designation`
                    // the painted path uses — one placement authority, not
                    // a founding-only copy of it.
                    let (placed_roles, placed_jobs) =
                        preset::place_preset(&mut job_board, &terrain, origin);

                    // bastion (B3): spawn the starting band (validated above).
                    rtsim.bastion_spawn_colony(pos, count);
                    // #105 (DECISIONS-FOR-BEN, FOUNDING SEED STOCK): a
                    // persistent loose drop, same mechanism as a player's
                    // own `/dropall true` (item 6's own instrument) --
                    // eligible for the B6 haul-to-stockpile pipeline the
                    // moment a stockpile is designated nearby. Live-path
                    // twin of `Server::bastion_found_colony_seed_stock`;
                    // this is the entry point that actually fires for a
                    // real in-game founding (caught live-first: the
                    // Server-level wrapper alone tested green against the
                    // harness while staying inert here -- the acceptance
                    // run's honest 0-sown result is what caught it).
                    //
                    // NOT a double-fire with the Server-level twin: a
                    // founding reaches exactly one of the two entry points,
                    // never both. This one fires ONLY from the live
                    // `ClientGeneral::BastionSpawnColony` message, handled
                    // entirely inside this system -- it never calls
                    // `Server::bastion_spawn_colony`/`_seeded`. Every other
                    // caller of THOSE (the harness's ~60 scenario sites,
                    // `bastion_arena.rs`'s "fixture" staging spawn, any
                    // determinism-capture code) is a direct Rust call that
                    // never routes through this client-message system. See
                    // the Server-level twin's own doc for the full
                    // caller enumeration.
                    if let Ok(mut item) =
                        common::comp::Item::new_from_asset(crate::bastion_jobs::FARM_SEED_ITEM)
                    {
                        let _ = item.set_amount(crate::bastion_jobs::FOUNDING_SEED_STOCK);
                        // FOUNDING PRESET F-2 (2026-08-12): the founding stock had NO
                        // witness line. A5/A3 both read "founded WITH stock", and with
                        // no emit that premise is UNREAD rather than true -- the same
                        // shape as every unwitnessed outcome this arc has paid for.
                        //
                        // `amount` is read BACK OFF THE ITEM, never echoed from
                        // FOUNDING_SEED_STOCK: `set_amount`'s Result is discarded above,
                        // so the constant is the INTENT and only the item carries the
                        // EFFECT. Reporting the constant here would be the F8 defect in
                        // miniature -- announcing what we meant to do.
                        //
                        // Inside the `Ok` arm on purpose: if the asset fails to load
                        // there is no drop, and there must be no line claiming one.
                        let dropped_amount = item.amount();
                        post_emitters.emit(event::CreateItemDropEvent {
                            pos: common::comp::Pos(pos),
                            vel: common::comp::Vel(Vec3::zero()),
                            ori: common::comp::Ori::default(),
                            item: common::comp::PickupItem::new(item, *program_time, true),
                            loot_owner: None,
                            persistent: true,
                        });
                        tracing::info!(
                            item = crate::bastion_jobs::FARM_SEED_ITEM,
                            amount = dropped_amount,
                            pos = ?pos,
                            "bastion: founding stock dropped"
                        );
                    }
                    // bastion (ROW-COLONY-PRESENCE, DECISIONS #106): the
                    // live-path twin of `Server::bastion_found_colony_
                    // presence` above -- same disjoint-producer shape as
                    // the seed-stock drop right above it.
                    //
                    // FOUNDING PRESET v1: this IS the promotion half of
                    // "spawn + promote" — the server-owned colony presence
                    // is what keeps the band in `SimulationMode::Loaded`
                    // with no client connected.
                    post_emitters.emit(event::CreateColonyPresenceEvent {
                        pos: common::comp::Pos(pos),
                    });

                    // THE LIVE WITNESS (§3.4, name-the-line law): every
                    // §5 claim reads from this emit. `elements` carries
                    // the ROLES placed, so A1's planted failure (a
                    // PARTIAL preset — the farm dropped) is visible in
                    // the witness itself rather than inferred from a
                    // count; `complete` is the bar's own boolean.
                    tracing::info!(
                        preset = preset::PRESET_VERSION,
                        ?pos,
                        datum = origin.z,
                        colonists = count,
                        elements = %preset::roles_summary(&placed_roles),
                        complete = preset::preset_is_complete(&placed_roles),
                        jobs = placed_jobs,
                        // The claim mask's own size AFTER placement — the
                        // "registered rev" §5 A1 reads. Named for what it
                        // actually is (a region count, append-on-place /
                        // subtract-on-cancel), not dressed up as a
                        // monotonic revision it isn't.
                        designated_regions = job_board.designated.len(),
                        "bastion: colony founded"
                    );
                    }
                }
                // bastion (UI-4): answer inspector requests — read-only
                // payload assembly at request cadence (~1Hz per open
                // panel; the rtsim guard is acquired per REQUEST, never
                // per tick — the AUTON-3 cadence lesson). A non-colonist
                // (or stale) target answers `payload: None`, which the
                // client renders as nothing — the packet's no-crash
                // invariant for non-colonist picks.
                if let Some((requester, targets)) = bastion_inspects_update {
                    if let Some(client) = clients.get(*requester) {
                        for target in targets.drain(..) {
                            use common::comp::bastion::{BastionInspectKind, BastionInspectTarget};
                            let payload = match target {
                                // UI-5: a world cell → whatever Bastion object
                                // sits there (job / stockpile / farm / fell-set).
                                BastionInspectTarget::Cell(cell) => resolve_cell_inspect(
                                    cell,
                                    &job_board,
                                    &terrain,
                                    &insp_pickup_items,
                                    &positions,
                                    &id_maps,
                                    &insp_colonists,
                                ),
                                // UI-4: an entity → the colonist inner state.
                                BastionInspectTarget::Entity(uid) => id_maps
                                    .uid_entity(uid)
                                    .and_then(|e| {
                                        let colonist = insp_colonists.get(e)?;
                                        let needs = insp_needs.get(e)?;
                                        let (p4, consc, neur, trait_list) = insp_rtsim_entities
                                            .get(e)
                                            .and_then(|re| {
                                                let data = rtsim.state().data();
                                                data.npcs.get(*re).map(|npc| {
                                                    use common::rtsim::PersonalityTrait as PT;
                                                    (
                                                        (
                                                            npc.personality.is(PT::Adventurous),
                                                            npc.personality.is(PT::Worried),
                                                            npc.personality.is(PT::Sociable)
                                                                || npc
                                                                    .personality
                                                                    .is(PT::Extroverted),
                                                            npc.personality.is(PT::Introverted),
                                                        ),
                                                        npc.personality.is(PT::Conscientious),
                                                        npc.personality.is(PT::Neurotic),
                                                        // ITEM 21: every satisfied
                                                        // trait, same source.
                                                        // PersonalityTrait has
                                                        // no Debug (same wall
                                                        // the pin hit):
                                                        // parallel labels.
                                                        [
                                                            (PT::Open, "Open"),
                                                            (PT::Adventurous, "Adventurous"),
                                                            (PT::Closed, "Closed"),
                                                            (PT::Conscientious, "Conscientious"),
                                                            (PT::Busybody, "Busybody"),
                                                            (PT::Unconscientious, "Unconscientious"),
                                                            (PT::Extroverted, "Extroverted"),
                                                            (PT::Introverted, "Introverted"),
                                                            (PT::Agreeable, "Agreeable"),
                                                            (PT::Sociable, "Sociable"),
                                                            (PT::Disagreeable, "Disagreeable"),
                                                            (PT::Neurotic, "Neurotic"),
                                                            (PT::Seeker, "Seeker"),
                                                            (PT::Worried, "Worried"),
                                                            (PT::SadLoner, "SadLoner"),
                                                            (PT::Stable, "Stable"),
                                                        ]
                                                        .into_iter()
                                                        .filter(|(t, _)| {
                                                            npc.personality.is(*t)
                                                        })
                                                        .map(|(_, l)| l.to_string())
                                                        .collect::<Vec<_>>(),
                                                    )
                                                })
                                            })
                                            .unwrap_or((
                                                (false, false, false, false),
                                                false,
                                                false,
                                                Vec::new(),
                                            ));
                                        let arb = insp_arbiters.get(e);
                                        // engine-list T3.54: mood
                                        // explainability — same tables +
                                        // Actor the B7-0 mood tick reads,
                                        // assembled fresh at request
                                        // cadence (never cached; the
                                        // inspector's own no-drift rule).
                                        let mood_explanation = insp_rtsim_entities.get(e).map(|re| {
                                            let mood_cfg = common::bastion::MoodConfig::current();
                                            let table = bastion_server::bastion_mood::ThoughtTable::current();
                                            let affinities =
                                                bastion_server::bastion_mood::ValueAffinityTable::current();
                                            let data = rtsim.state().data();
                                            let actor = common::rtsim::Actor::Npc(*re);
                                            let thoughts = bastion_server::bastion_mood::thought_contributions(
                                                &data.chronicle,
                                                &table,
                                                &affinities,
                                                actor,
                                                data.time_of_day.0,
                                                &colonist.0.values,
                                                neur,
                                            );
                                            let thought_sum = bastion_server::bastion_mood::thought_sum(
                                                &data.chronicle,
                                                &table,
                                                &affinities,
                                                actor,
                                                data.time_of_day.0,
                                                &colonist.0.values,
                                                neur,
                                            );
                                            common::comp::bastion::MoodExplanationV1::build(
                                                insp_tick.0,
                                                actor,
                                                &mood_cfg,
                                                needs,
                                                thought_sum,
                                                thoughts,
                                            )
                                        });
                                        // engine-list T3.58: job ownership +
                                        // Drive telemetry — same ActiveJob
                                        // lookup the FailsafeTeleportEvent
                                        // diagnostic uses.
                                        let active_job = insp_active_jobs.get(e);
                                        let looked_up_job =
                                            active_job.and_then(|a| job_board.jobs.get(&a.job));
                                        let ownership = Some(
                                            common::comp::bastion::InspectorOwnershipV1::build(
                                                insp_tick.0,
                                                uid,
                                                active_job,
                                                looked_up_job.map(|j| &j.kind),
                                                looked_up_job.and_then(|j| j.claimed_by),
                                                arb,
                                            ),
                                        );
                                        Some(common::comp::bastion::BastionInspectPayload {
                                            name: colonist.0.name.clone(),
                                            hunger: needs.hunger,
                                            rest: needs.rest,
                                            recreation: needs.recreation,
                                            mood: insp_moods.get(e).map_or(0.0, |m| m.0),
                                            personality4: p4,
                                            conscientious: consc,
                                            neurotic: neur,
                                            drive: arb
                                                .map_or(common::comp::bastion::Drive::Idle, |a| {
                                                    a.current
                                                }),
                                            last_scores: arb
                                                .map_or((0.0, 0.0, 0.0), |a| a.last_scores),
                                            // CHOP-PROGRESS-INDICATOR (row 51.61):
                                            // current work job + progress, ridden
                                            // to the inspector from the Arbiter.
                                            activity: arb.and_then(|a| a.activity),
                                            // STATUS-SURFACE: energy meter +
                                            // status via the ONE read-only
                                            // accessor (same fn as the
                                            // harness probe — cannot drift).
                                            energy: insp_energies
                                                .get(e)
                                                .map_or(0.0, |en| en.fraction()),
                                            // ITEM 13 (health branch): `.fraction()`
                                            // to match `energy`'s unit, but kept
                                            // inside the Option so a missing
                                            // component cannot read as death.
                                            health: insp_healths
                                                .get(e)
                                                .map(|h| h.fraction()),
                                            status: crate::bastion_jobs::colonist_status(
                                                &job_board,
                                                uid,
                                                insp_tick.0,
                                            ),
                                            mood_explanation,
                                            ownership,
                                            // ITEM 17 (VISIBLE): filled from
                                            // level_for -- the same source the
                                            // claim gate and work rate read.
                                            skills: {
                                                use common::bastion::WorkType as W;
                                                [W::Mine, W::Chop, W::Build, W::Haul, W::Cook, W::Farm, W::Guard]
                                                    .into_iter()
                                                    .map(|w| {
                                                        (
                                                            w.label().to_string(),
                                                            colonist.0.skills.level_for(w),
                                                        )
                                                    })
                                                    .collect()
                                            },
                                            traits: trait_list,
                                            desires: {
                                                use common::bastion::WorkType as W;
                                                [W::Mine, W::Chop, W::Build, W::Haul, W::Cook, W::Farm, W::Guard]
                                                    .into_iter()
                                                    .map(|w| {
                                                        (
                                                            w.label().to_string(),
                                                            colonist.0.desires.get(w),
                                                        )
                                                    })
                                                    .collect()
                                            },
                                            guard_bravery: colonist.0.guard_bravery,
                                            // ITEM 22: same-source fill from
                                            // Sentiments::iter_held — the
                                            // record change_by writes. Npc
                                            // targets resolve to colonist
                                            // uids so pairs are namable in
                                            // the driver log.
                                            sentiments: insp_rtsim_entities
                                                .get(e)
                                                .map(|re| {
                                                    let data = rtsim.state().data();
                                                    data.npcs
                                                        .get(*re)
                                                        .map(|npc| {
                                                            use ::rtsim::data::sentiment::Target;
                                                            npc.sentiments
                                                                .iter_held()
                                                                .map(|(t, v)| {
                                                                    let label = match t {
                                                                        Target::Npc(id) => id_maps
                                                                            .rtsim_entity(id)
                                                                            .and_then(|te| {
                                                                                insp_uids.get(te)
                                                                            })
                                                                            .map(|u| {
                                                                                format!(
                                                                                    "uid:{}",
                                                                                    u.0.get()
                                                                                )
                                                                            })
                                                                            .unwrap_or_else(|| {
                                                                                format!(
                                                                                    "npc:{:?}",
                                                                                    id
                                                                                )
                                                                            }),
                                                                        Target::Character(c) => {
                                                                            format!("char:{:?}", c)
                                                                        },
                                                                        Target::Faction(f) => {
                                                                            format!("faction:{:?}", f)
                                                                        },
                                                                    };
                                                                    (label, v)
                                                                })
                                                                .collect()
                                                        })
                                                        .unwrap_or_default()
                                                })
                                                .unwrap_or_default(),
                                        })
                                    })
                                    .map(BastionInspectKind::Colonist),
                                // ARC 2 item 10: the colony as a whole. Every
                                // field is a count of something already
                                // tracked -- and `food_stock` calls
                                // `colony_food_stock`, the SAME producer the
                                // colony-terminal check reads, so the
                                // dashboard cannot report a healthier colony
                                // than the one the death check sees.
                                BastionInspectTarget::Colony => {
                                    use specs::Join;
                                    let colonists = (&insp_colonists).join().count() as u32;
                                    let food_stock = crate::bastion_jobs::colony_food_stock(
                                        (&insp_pickup_items, &positions).join(),
                                        &job_board,
                                    );
                                    let jobs_total = job_board.jobs.len() as u32;
                                    let jobs_claimed = job_board
                                        .jobs
                                        .values()
                                        .filter(|j| j.claimed_by.is_some())
                                        .count()
                                        as u32;
                                    // ★ F19 (found by an adversarial play
                                    // session, 2026-08-21): a refusal reason
                                    // with no dashboard category is INVISIBLE
                                    // BY CONSTRUCTION. The session watched 8
                                    // colonists stand idle beside 4 jobs while
                                    // the player-facing counters read
                                    // `jobs_unreachable=0 blocked_materials=0`
                                    // -- every one of 32 considerations had
                                    // been refused on AFFORDANCE (no cell a
                                    // colonist can stand in to work the job),
                                    // and the dashboard had no bucket for it.
                                    // The player sees four healthy-looking
                                    // jobs, zero claimed, eight idle people,
                                    // and no explanation anywhere.
                                    //
                                    // Same predicate the claim gate uses, for
                                    // the same reason blocked_materials does:
                                    // the dashboard must not disagree with the
                                    // selector about why work is stuck.
                                    let jobs_blocked_stance = job_board
                                        .jobs
                                        .values()
                                        .filter(|j| {
                                            j.claimed_by.is_none()
                                                && crate::bastion_jobs::job_stance_missing(
                                                    &terrain, j,
                                                )
                                        })
                                        .count() as u32;
                                    let jobs_unreachable = job_board
                                        .jobs
                                        .values()
                                        .filter(|j| j.unreachable)
                                        .count()
                                        as u32;
                                    // BLOCKED-MATERIALS row: per JOB, never per
                                    // (job, colonist) pair. `needs_materials`
                                    // is the colony-wide "nobody carries it"
                                    // flag the board already maintains;
                                    // `stockpile_has_material` is the SAME
                                    // fetch-leg rule the claim gate uses, so
                                    // the dashboard cannot disagree with the
                                    // selector about what is blocked.
                                    let jobs_blocked_materials = job_board
                                        .jobs
                                        .values()
                                        .filter(|j| {
                                            j.needs_materials
                                                && !matches!(
                                                    j.kind,
                                                    common::bastion::JobKind::Haul { .. }
                                                )
                                                && j.required_item.is_some_and(|req| {
                                                    !crate::bastion_jobs::stockpile_has_material(
                                                        req,
                                                        (&insp_pickup_items, &positions, &insp_uids)
                                                            .join(),
                                                        &job_board,
                                                    )
                                                })
                                        })
                                        .count()
                                        as u32;
                                    Some(BastionInspectKind::Colony(
                                        common::comp::bastion::BastionColonyInspect {
                                            colonists,
                                            food_stock,
                                            jobs_total,
                                            jobs_claimed,
                                            jobs_blocked_stance,
                                            jobs_unreachable,
                                            designations: job_board.designated_regions().count() as u32,
                                            jobs_blocked_materials,
                                            // ITEM 32: the pool the player
                                            // spends on god-powers, from the
                                            // same resource the cast gate
                                            // reads — one number, not a copy.
                                            favor: insp_favor.0,
                                            tick: bastion_tick.0,
                                        },
                                    ))
                                },
                                // ARC 2 item 12: the chronicle — the entity
                                // log's player view. READ-ONLY: only
                                // events_for/truncated are called. `enabled`
                                // and `truncated` ride the payload because an
                                // empty list, a disabled log and an overflowed
                                // ring are three different states, and the UI
                                // can only distinguish states the payload
                                // carries (item 12 prereg, bar 2).
                                BastionInspectTarget::Chronicle(uid) => {
                                    use bastion_server::bastion_entity_event_log as ev;
                                    // ★ THE STORY SURFACE IS NOT THE CENSUS
                                    // (improvement-list row 20: "the chronicle
                                    // is 93% job-release spam"). Released
                                    // rows stay IN the ring (Measure 0's
                                    // producer, the release census, every
                                    // debugging consumer) — the PLAYER VIEW
                                    // filters them so a life reads as a life:
                                    // sleeps, meals, wounds, rescues, fear.
                                    // BASTION_CHRONICLE_RAW=1 restores the
                                    // unfiltered feed for instrument work.
                                    // Deliberate scope: a VIEW filter, not a
                                    // recorder change — EXCLUSION here is a
                                    // presentation choice, and the census
                                    // this class belongs to lives in the
                                    // release histogram, not this list.
                                    let raw = std::env::var_os(
                                        "BASTION_CHRONICLE_RAW",
                                    )
                                    .is_some();
                                    let events = ev::events_for(uid)
                                        .into_iter()
                                        .filter(|e| {
                                            raw || !matches!(
                                                e.kind,
                                                ev::EventKind::Colonist(
                                                    ev::ColonistEventKind::Released { .. }
                                                )
                                            )
                                        })
                                        .map(|e| {
                                            common::comp::bastion::BastionChronicleRow {
                                                tick: e.tick,
                                                kind: format!("{:?}", e.kind),
                                                actor: e.actor.map(|a| a.0.get()),
                                            }
                                        })
                                        .collect();
                                    Some(BastionInspectKind::Chronicle(
                                        common::comp::bastion::BastionChronicleInspect {
                                            enabled: ev::enabled(),
                                            truncated: ev::truncated(uid),
                                            events,
                                        },
                                    ))
                                },
                            };
                            let _ =
                                client.send(ServerGeneral::BastionInspectInfo { target, payload });
                        }
                    }
                }
                // bastion (B4): apply deferred designation ops to the board.
                // B5.6b-2: surface-relative placements recompute the same
                // per-column surfaces the handler's echo bounds came from
                // (terrain can't change between the two — block edits land
                // post-tick), so the echoed rect bounds every job created.
                for (region, op, extent, chop_cells) in bastion_designation_updates.drain(..) {
                    match (op, extent, chop_cells) {
                        (Some(kind), Some(extent), _) => {
                            // ★ PLAYER PAINT IS A MANDATE (Ben: 513 painted
                            // mine jobs, 44 game-days, zero claims — painted
                            // work sat in a priority caste that never wins
                            // against the colony's own in-lane jobs). This
                            // drain is the PLAYER's path; adoption and
                            // founding call the same placement fns and stay
                            // unstamped.
                            let created = job_board.place_designation_surface(
                                &terrain,
                                region.min.xy(),
                                region.max.xy(),
                                region.max.z,
                                extent,
                                kind,
                            );
                            let n = created.len();
                            for id in created {
                                if let Some(j) = job_board.jobs.get_mut(&id) {
                                    j.player_ordered = true;
                                }
                            }
                            if n > 0 {
                                // off_hours resolved by the drain (next tick,
                                // same hour) — this system has no schedule
                                // vocabulary and should not grow one.
                                job_board.pending_paint_notices.push((n, false));
                            }
                        },
                        // CHOP-FELLING (row 51.6): a resolved (base, fell-set)
                        // — one base-cut job per tree; the whole set fells on
                        // completion.
                        (
                            Some(common::bastion::DesignationKind::Chop),
                            None,
                            Some((base, cells)),
                        ) => {
                            let created = job_board.place_chop_fell(&terrain, base, &cells);
                            for id in created {
                                if let Some(j) = job_board.jobs.get_mut(&id) {
                                    j.player_ordered = true;
                                }
                            }
                        },
                        (Some(kind), None, _) => {
                            let created = job_board.place_designation(&terrain, region, kind);
                            for id in created {
                                if let Some(j) = job_board.jobs.get_mut(&id) {
                                    j.player_ordered = true;
                                }
                            }
                        },
                        (None, _, _) => {
                            job_board.cancel_region(region);
                            // B6-hotfix (Ben live-test: "a way to delete
                            // ladders"): Erase ALSO removes built ladders
                            // in-region. LADDERS ONLY — a targeted cleanup
                            // god-action that can't nuke a wall. Set each
                            // Ladder sprite block to its vacant (air) form
                            // via BlockChange (the same path a build job
                            // uses, in reverse), then drop the access
                            // anchor for any emptied column so staged
                            // routing doesn't point at a ghost link.
                            let mut removed_any = false;
                            // DET-ECS-015: deferred writes; the removal
                            // decision derives from the READ (ladder cells
                            // present), no longer from winning a same-tick
                            // try_set race - a vanishing-edge semantic shift
                            // (contested-cell case) disclosed in the commit.
                            for x in region.min.x..=region.max.x {
                                for y in region.min.y..=region.max.y {
                                    for z in region.min.z..=region.max.z {
                                        let p = vek::Vec3::new(x, y, z);
                                        if let Ok(b) = terrain.get(p)
                                            && b.get_sprite() == Some(SpriteKind::Ladder)
                                        {
                                            terrain_writes.push((p, b.into_vacant()));
                                            removed_any = true;
                                        }
                                    }
                                }
                            }
                            if removed_any {
                                job_board.drop_access_anchors_in(region);
                            }
                        },
                    }
                }
            },
        );
        // ── bastion: RECONCILE the designation overlay ────────────────────
        // ★ FOUND BY A PLAY SESSION (2026-08-21), on the adopt-town arm.
        // `inspect_colony` said `designations=5` and `list_designations` said
        // `[]`, on a fresh connection, twice. Both numbers were right: the
        // first is `JobBoard::designated`, the second is the client's mirror,
        // and NOTHING connected them. Designation sync was ECHO-ONLY — the
        // sends live inside the place/cancel handlers above, so a client
        // heard about designations IT placed and nothing else. Every other
        // producer was invisible by construction: ADOPT-A-TOWN's deferred
        // surface drain (which places as chunks load — i.e. AFTER the player
        // has joined, so a join-time snapshot would have missed it too),
        // colony-persistence restore, AUTON-1 build plans, another player's
        // paint.
        //
        // The cost in play: adopting a town MEANS inheriting its beds,
        // fields and barn, and the player could not ask what they had
        // inherited, see it on the map or minimap, or right-click a zone to
        // cancel it — every one of those reads this same mirror. The zones
        // were only locatable by reading the SERVER's own log.
        //
        // Server truth wins, always, and the comparison is the whole set:
        // no revision counter to keep honest across `designated`'s many
        // mutators (place, cancel's `retain`, the adopt drain, restore), no
        // hash to collide. `Region` and `DesignationKind` are both `Eq`, so
        // the steady state — mirror equals board — costs one slice compare
        // per client per tick and sends nothing at all.
        //
        // Divergence resyncs as CLEAR + REFILL rather than a computed delta:
        // one removal covering the bounding box of everything we have sent
        // (a region that contains a rect subtracts it to nothing — see
        // `Region::subtract`), then the board's set. A delta would have to
        // re-derive the client's AABB-subtraction state, which is a second
        // implementation of a thing that must agree with the first forever.
        {
            let truth = job_board.designated.as_slice();
            for (client, _presence) in (&clients, &presences).join() {
                // Scoped so the guard is released before the sends below —
                // the send helpers take the same lock to keep the mirror in
                // step, and a re-entrant lock is a deadlock, not an error.
                let bounds = {
                    let mirror = client.bastion_designations_mirror();
                    if mirror.as_slice() == truth {
                        continue;
                    }
                    common::bastion::Region::bounding(mirror.iter().map(|(r, _)| *r))
                };
                if let Some(bounds) = bounds {
                    // Subtracts the mirror to nothing on both sides — the
                    // helper applies the client's own `Region::subtract`,
                    // and `Region::bounding`'s own test pins that a set's
                    // bounding box clears every member of that set.
                    // NOT force-cleared here: if the send fails the mirror
                    // must keep describing what the client still holds, or
                    // the refill below would double it up. A failed clear
                    // leaves mirror = old + truth, which the next tick sees
                    // as divergent and clears for real.
                    let _ = client.send_bastion_designation_removed(bounds);
                }
                for (region, kind) in truth {
                    // `z_extent: None` — the resolved bounds ARE the volume
                    // (every `designated` entry is stored resolved), and no
                    // consumer reads the extent: voxygen's overlay counts
                    // levels from `region.max.z - region.min.z`, and the
                    // map, minimap and radial-cancel paths destructure it
                    // away entirely.
                    let _ = client.send_bastion_designation(*region, *kind, None);
                }
            }
        }
        // Finally, drop the deferred updates in another thread.
        slow_jobs.spawn("CHUNK_DROP", move || {
            drop(deferred_updates);
        });
    }
}

/// `T3.3.09`: see the identical rationale in `general.rs`'s own
/// `mod semantic` -- the validation matrix is proven once, system-
/// agnostically, in `T3.3.08`; this test only guards against a
/// copy-paste stream-ID mismatch at this file's own dispatch call site.
#[cfg(test)]
mod semantic {
    use common_net::msg::{ClientGeneral, envelope::{SemanticRouteV1, SemanticStreamIdV1}};

    #[test]
    fn dispatch_stream_matches_handled_in_game_messages() {
        assert_eq!(ClientGeneral::ExitInGame.semantic_stream(), SemanticStreamIdV1::InGame);
        assert_eq!(ClientGeneral::SpectateEntity(None).semantic_stream(), SemanticStreamIdV1::InGame);
    }
}
