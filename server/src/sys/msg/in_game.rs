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
    terrain::{SpriteKind, TerrainGrid},
    uid::IdMaps,
    vol::ReadVol,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ClientGeneral, ServerGeneral};
use common_state::{AreasContainer, BlockChange, BuildArea};
use core::mem;
use rayon::prelude::*;
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteStorage};
use std::{borrow::Cow, time::Instant};
use tracing::{debug, trace, warn};
use vek::*;

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
struct RareWrites<'a, 'b> {
    block_changes: &'b mut BlockChange,
    _terrain_persistence: &'b mut TerrainPersistenceData<'a>,
}

event_emitters! {
    struct Events[Emitters] {
        exit_ingame: event::ExitIngameEvent,
        request_site_info: event::RequestSiteInfoEvent,
        update_map_marker: event::UpdateMapMarkerEvent,
        client_disconnect: event::ClientDisconnectEvent,
        set_battle_mode: event::SetBattleModeEvent,
        // bastion (B3): god-anchor invulnerability buff add/remove
        buff: event::BuffEvent,
    }
}

impl Sys {
    #[expect(clippy::too_many_arguments)]
    fn handle_client_in_game_msg(
        emitters: &mut Emitters,
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
        rare_writes: &parking_lot::Mutex<RareWrites<'_, '_>>,
        position: Option<&mut Pos>,
        spectating_entity: &mut Option<Option<common::uid::Uid>>,
        bastion_anchor: &mut Option<bool>,
        bastion_spawn: &mut Option<(Vec3<f32>, u8)>,
        // bastion (B4): deferred designation ops — Some(kind) = place,
        // None = cancel (the job board can't be touched in the parallel join).
        // B5.6b-2: the third field is Some(extent) for surface-relative
        // placements (region = footprint XY + paint-plane hint in max.z);
        // None keeps the legacy literal-region path. Always None for cancel.
        bastion_designations: &mut Vec<(
            common::bastion::Region,
            Option<common::bastion::DesignationKind>,
            Option<common::bastion::ZExtent>,
        )>,
        controller: Option<&mut Controller>,
        settings: &Read<'_, Settings>,
        build_areas: &Read<'_, AreasContainer<BuildArea>>,
        player_physics_setting: Option<&mut PlayerPhysicsSetting>,
        server_physics_forced: bool,
        maybe_admin: &Option<&Admin>,
        time_for_vd_changes: Instant,
        msg: ClientGeneral,
        player_physics: &mut Option<(Pos, Vel, Ori)>,
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
                emitters.emit(event::ExitIngameEvent { entity });
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
                force_counter,
            } => {
                if presence.kind.controlling_char()
                    && force_update
                        .is_none_or(|force_update| force_update.counter() == force_counter)
                    && healths.get(entity).is_none_or(|h| !h.is_dead)
                    && is_rider.get(entity).is_none()
                    && is_volume_rider.get(entity).is_none()
                    && !server_physics_forced
                    && player_physics_setting
                        .as_ref()
                        .is_none_or(|s| !s.server_authoritative_physics_optin())
                {
                    *player_physics = Some((pos, vel, ori));
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
                            // Take the rare writes lock as briefly as possible.
                            let mut guard = rare_writes.lock();
                            let _was_set = guard.block_changes.try_set(pos, new_block).is_some();
                            #[cfg(feature = "persistent_world")]
                            if _was_set
                                && let Some(terrain_persistence) =
                                    guard._terrain_persistence.as_mut()
                            {
                                terrain_persistence.set_block(pos, new_block);
                            }
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
                            // Take the rare writes lock as briefly as possible.
                            let mut guard = rare_writes.lock();
                            let _was_set = guard.block_changes.try_set(pos, new_block).is_some();
                            #[cfg(feature = "persistent_world")]
                            if _was_set
                                && let Some(terrain_persistence) =
                                    guard._terrain_persistence.as_mut()
                            {
                                terrain_persistence.set_block(pos, new_block);
                            }
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
                emitters.emit(event::RequestSiteInfoEvent { entity, id });
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
                emitters.emit(event::UpdateMapMarkerEvent { entity, update });
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
                                    crate::bastion_jobs::column_flat_surface_z(
                                        terrain, x, y, floor,
                                    )
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
                                    max_surface =
                                        Some(max_surface.map_or(s, |m: i32| m.max(s)));
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
                                let nominal = ((max_crest_for(clamped) - clamped)
                                    .max(0) as i64
                                    + 1)
                                    + extent.up as i64
                                    + 8;
                                if footprint * nominal > 0
                                    && footprint * nominal
                                        <= common::bastion::MAX_DESIGNATION_VOLUME
                                {
                                    resolved =
                                        crate::bastion_jobs::resolve_surface_bounds(
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
                        client.send(ServerGeneral::BastionDesignation {
                            region: bounds,
                            kind,
                            z_extent: Some(extent),
                        })?;
                        bastion_designations.push((region, Some(kind), Some(extent)));
                    } else {
                        client.send(ServerGeneral::server_msg(
                            common::comp::ChatType::CommandError,
                            common::comp::Content::Plain(format!(
                                "Designation rejected: volume {} outside 1..={} or no \
                                 terrain surface under the footprint",
                                volume,
                                common::bastion::MAX_DESIGNATION_VOLUME
                            )),
                        ))?;
                    }
                } else {
                    let volume = region.volume();
                    if volume > 0 && volume <= common::bastion::MAX_DESIGNATION_VOLUME {
                        client.send(ServerGeneral::BastionDesignation {
                            region,
                            kind,
                            z_extent: None,
                        })?;
                        // bastion (B4): job generation happens post-loop.
                        bastion_designations.push((region, Some(kind), None));
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
                bastion_designations.push((region, None, None));
                // bastion (B5.5): echo the removal so the client subtracts
                // it from its overlay rects (mirrors the place echo above).
                client.send(ServerGeneral::BastionDesignationRemoved { region })?;
                client.send(ServerGeneral::server_msg(
                    common::comp::ChatType::CommandInfo,
                    common::comp::Content::Plain("Designations cancelled.".to_string()),
                ))?;
            },
            ClientGeneral::BastionSpawnColony { pos, count } => {
                // bastion (B3): validated here, spawned post-loop (rtsim
                // resource can't be touched inside the parallel join).
                if presence.bastion_terrain_anchor.is_some()
                    && pos.map(|e| e.is_finite()).reduce_and()
                    && (1..=16).contains(&count)
                {
                    *bastion_spawn = Some((pos, count));
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
                emitters.emit(event::SetBattleModeEvent {
                    entity,
                    battle_mode,
                });
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
            | ClientGeneral::RequestPlugins(_) => {
                debug!("Kicking possibly misbehaving client due to invalid client in game request");
                emitters.emit(event::ClientDisconnectEvent(
                    entity,
                    common::comp::DisconnectReason::NetworkError,
                ));
            },
        }
        Ok(())
    }
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
        ),
        (
            Read<'a, IdMaps>,
            Read<'a, DeltaTime>,
            Read<'a, Settings>,
            Read<'a, AreasContainer<BuildArea>>,
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
            specs::WriteExpect<'a, crate::rtsim::RtSim>,
            Write<'a, crate::bastion_jobs::JobBoard>,
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
            (terrain, slow_jobs, editable_settings),
            (id_maps, dt, settings, build_areas),
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
            (mut god_anchors, time, mut rtsim, mut job_board),
        ): Self::SystemData,
    ) {
        let time_for_vd_changes = Instant::now();

        // NOTE: stdlib mutex is more than good enough on Linux and (probably) Windows,
        // but not Mac.
        let rare_writes = parking_lot::Mutex::new(RareWrites {
            block_changes: &mut block_changes,
            _terrain_persistence: &mut terrain_persistence,
        });

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
                || events.get_emitters(),
                |emitters, (
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
                    let mut spectating_entity = None;
                    let mut bastion_anchor = None;
                    let mut bastion_spawn = None;
                    let mut bastion_designations = Vec::new();
                    let _ = super::try_recv_all(client, 2, |client, msg| {
                        Self::handle_client_in_game_msg(
                            emitters,
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
                            &rare_writes,
                            pos.as_deref_mut(),
                            &mut spectating_entity,
                            &mut bastion_anchor,
                            &mut bastion_spawn,
                            &mut bastion_designations,
                            controller.as_deref_mut(),
                            &settings,
                            &build_areas,
                            new_player_physics_setting.as_mut(),
                            is_server_physics_forced,
                            &maybe_admin,
                            time_for_vd_changes,
                            msg,
                            &mut player_physics,
                        )
                    });

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
                    (
                        skill_set_update,
                        spectating_entity_update,
                        physics_update,
                        bastion_anchor_update,
                        bastion_spawn,
                        bastion_designations,
                    )
                },
            )
            // NOTE: Would be nice to combine this with the map_init somehow, but I'm not sure if
            // that's possible.
            .filter(|(x, y, z, w, v, d)| {
                x.is_some() || y.is_some() || z.is_some() || w.is_some() || v.is_some()
                    || !d.is_empty()
            })
            // NOTE: I feel like we shouldn't actually need to allocate here, but hopefully this
            // doesn't turn out to be important as there shouldn't be that many connected clients.
            // The reason we can't just use unzip is that the two sides might be different lengths.
            .collect::<Vec<_>>();
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
                skill_set_update,
                spectating_entity_update,
                physics_update,
                bastion_anchor_update,
                bastion_spawn_update,
                bastion_designation_updates,
            )| {
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
                if let &mut Some((pos, count)) = bastion_spawn_update {
                    // bastion (B3): spawn the starting band (validated above).
                    rtsim.bastion_spawn_colony(pos, count);
                }
                // bastion (B4): apply deferred designation ops to the board.
                // B5.6b-2: surface-relative placements recompute the same
                // per-column surfaces the handler's echo bounds came from
                // (terrain can't change between the two — block edits land
                // post-tick), so the echoed rect bounds every job created.
                for (region, op, extent) in bastion_designation_updates.drain(..) {
                    match (op, extent) {
                        (Some(kind), Some(extent)) => {
                            job_board.place_designation_surface(
                                &terrain,
                                region.min.xy(),
                                region.max.xy(),
                                region.max.z,
                                extent,
                                kind,
                            );
                        },
                        (Some(kind), None) => {
                            job_board.place_designation(&terrain, region, kind);
                        },
                        (None, _) => {
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
                            {
                                let mut guard = rare_writes.lock();
                                for x in region.min.x..=region.max.x {
                                    for y in region.min.y..=region.max.y {
                                        for z in region.min.z..=region.max.z {
                                            let p = vek::Vec3::new(x, y, z);
                                            if let Ok(b) = terrain.get(p)
                                                && b.get_sprite() == Some(SpriteKind::Ladder)
                                            {
                                                let vacant = b.into_vacant();
                                                if guard
                                                    .block_changes
                                                    .try_set(p, vacant)
                                                    .is_some()
                                                {
                                                    removed_any = true;
                                                }
                                            }
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
        // Finally, drop the deferred updates in another thread.
        slow_jobs.spawn("CHUNK_DROP", move || {
            drop(deferred_updates);
        });
    }
}
