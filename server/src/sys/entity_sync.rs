use super::sentinel::{DeletedEntities, TrackedStorages, UpdateTrackers};
use crate::{
    EditableSettings, Tick,
    client::Client,
    presence::RegionSubscription,
    semantic_net::{
        order::{SemanticPayloadRankV1, SemanticProducerV1, phase_rank},
        outbox::{CanonicalSubjectKeyV1, ServerSemanticOutboxV1},
    },
};
use common::{
    calendar::Calendar,
    comp::{
        Collider, ForceUpdate, InventoryUpdateBuffer, Last, Ori, Player, Pos, Presence, Vel,
        presence::SpectatingEntity,
    },
    event::EventBus,
    link::Is,
    mounting::Rider,
    outcome::Outcome,
    region::{RegionEvent, RegionMap},
    resources::{PlayerPhysicsSettings, Time, TimeOfDay, TimeScale},
    terrain::TerrainChunkSize,
    uid::Uid,
    vol::RectVolSize,
};
use common_base::dev_panic;
use common_ecs::{Job, Origin, Phase, System};
use common_net::{
    msg::ServerGeneral,
    sync::CompSyncPackage,
};
use itertools::Either;
use specs::{Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteStorage};
use vek::*;

/// `APEX-T3.3.13`/`.14`: this file's own `local_ordinal` conventions,
/// named and shared between the real call sites below and their test
/// coverage in `semantic_intents`/`semantic_intents_parallel` --
/// closing Opus's T3.3.13 pre-merge advisory (the conventions were
/// previously mirrored as separate literals in the test fixtures, so a
/// drive-by edit of a real site's ordinal failed no test). Every
/// constant here is a plain `local_ordinal`; the crucial invariant is
/// only ever "two intents that could share (recipient, stream,
/// payload_rank, subject) must not share an ordinal" -- see each
/// constant's own doc for which sibling it's disambiguating against.
pub(crate) mod ordinals {
    /// `Create`/`Delete` entity events: subject is `for_uid(entity)`,
    /// unique per (client, region, tick) by construction (an entity has
    /// exactly one Entered/Left event per actual transition) -- no
    /// sibling to disambiguate against, so a constant `0` is safe.
    pub(crate) const CREATE_OR_DELETE_ENTITY: u32 = 0;
    /// The paired-block `EntitySync` batch (subject: `for_region`). No
    /// same-payload_rank sibling exists, so a constant `0` is safe --
    /// named separately from [`PAIRED_COMPSYNC`] purely for readability
    /// even though the values happen to match.
    pub(crate) const PAIRED_ENTITYSYNC: u32 = 0;
    /// The paired-block `EntitySync`+`CompSync` batch (subject:
    /// `for_region`). Disambiguated against [`THROTTLE_COMPSYNC`] below,
    /// its only same-(recipient,stream,payload_rank,subject) sibling.
    pub(crate) const PAIRED_COMPSYNC: u32 = 0;
    /// The physics-throttle `CompSync` batch (subject: `for_region`,
    /// same region as [`PAIRED_COMPSYNC`] for the same client) --
    /// disambiguated against it.
    pub(crate) const THROTTLE_COMPSYNC: u32 = 1;
    /// Own-entity `CompSync` (subject: `for_uid(client's own entity)`).
    /// Disambiguated against [`SPECTATOR_COMPSYNC`] below: a spectating
    /// client's own entity could in principle appear in both this loop
    /// and the spectator loop in the same tick, and both would share
    /// the same recipient/subject/payload_rank without this split.
    pub(crate) const OWN_ENTITY_COMPSYNC: u32 = 0;
    /// Spectator `CompSync` (subject: `for_uid(spectating client's own
    /// entity)`, NOT the spectated target -- the recipient is what the
    /// subject key names). Disambiguated against [`OWN_ENTITY_COMPSYNC`].
    pub(crate) const SPECTATOR_COMPSYNC: u32 = 1;
    /// `T3.3.14a`. `InventoryUpdate` (subject: `for_uid(client's own
    /// entity)`): at most one such message per (client, tick) -- no
    /// sibling to disambiguate against.
    pub(crate) const INVENTORY_UPDATE: u32 = 0;
    /// `T3.3.14a`. `Outcomes` (subject: `for_singleton("outcomes")`,
    /// shared across every recipient -- safe because the total-sort
    /// collision rule is scoped per-recipient, and each client's own
    /// intent carries its own `recipient`): no sibling.
    pub(crate) const OUTCOMES: u32 = 0;
    /// `T3.3.14a`. `TimeOfDay` (subject: `for_singleton("time_of_day")`,
    /// same per-recipient reasoning as [`OUTCOMES`]): no sibling.
    pub(crate) const TIME_OF_DAY: u32 = 0;
}

/// `APEX-T3.3.13`: builds one `SemanticSendIntentV1` for `payload` and
/// enqueues it, iff `recipient_binding` is `Some` (the client has a live
/// V1 attachment) -- Legacy clients (the ONLY kind possible today,
/// `T3.3.05`'s negotiation always resolves `Legacy`) fall through
/// untouched and the caller keeps using its existing direct-send call
/// for them. Returns `true` iff it enqueued (so the caller knows
/// whether it still needs the Legacy fallback for this client).
///
/// Deliberately takes `Option<ActiveSessionBindingV1>`, not `&Client`:
/// this crate has no lightweight way to construct a live `Client`
/// (`Client::new` needs a real `network::Participant` + 6 real
/// `Stream`s) for a unit test, matching the exact reason `T3.3.10`'s
/// `send_semantic_v1` was never directly unit-tested either -- pulling
/// "extract the binding from a client" out of "build+enqueue the
/// intent" keeps the second half (all the actual logic worth testing)
/// fully pure and testable without one. Call sites pass
/// `client.semantic_send_state().map(|s| s.binding())`.
///
/// `phase_rank`/`producer_rank` are fixed to this system's own values
/// (`Phase::Create`, `SemanticProducerV1::EntitySync`) -- every call
/// site in this file shares them; only `payload_rank`/`subject`/
/// `local_ordinal` vary per call site. `T3.3.14a`: now a thin wrapper
/// over [`ServerSemanticOutboxV1::try_enqueue_if_v1`], the shared
/// primitive `subscription.rs` (and onward, the rest of the
/// "replication family") also calls -- this file's own signature is
/// unchanged so none of its existing call sites or tests needed to move.
fn try_enqueue_entity_sync_intent(
    outbox: &ServerSemanticOutboxV1,
    recipient_binding: Option<common_net::msg::envelope::ActiveSessionBindingV1>,
    payload: ServerGeneral,
    source_tick: u64,
    payload_rank: SemanticPayloadRankV1,
    subject: CanonicalSubjectKeyV1,
    local_ordinal: u32,
) -> bool {
    outbox.try_enqueue_if_v1(
        recipient_binding,
        payload,
        source_tick,
        phase_rank(Phase::Create),
        SemanticProducerV1::EntitySync.producer_rank(),
        payload_rank.payload_rank(),
        subject,
        local_ordinal,
    )
}

/// This system will send physics updates to the client
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        Read<'a, PlayerPhysicsSettings>,
        TrackedStorages<'a>,
        ReadExpect<'a, TimeOfDay>,
        ReadExpect<'a, Time>,
        ReadExpect<'a, Calendar>,
        ReadExpect<'a, TimeScale>,
        ReadExpect<'a, RegionMap>,
        ReadExpect<'a, UpdateTrackers>,
        Write<'a, DeletedEntities>,
        Read<'a, EventBus<Outcome>>,
        ReadExpect<'a, EditableSettings>,
        ReadExpect<'a, ServerSemanticOutboxV1>,
        (
            ReadStorage<'a, Pos>,
            ReadStorage<'a, Vel>,
            ReadStorage<'a, Ori>,
            ReadStorage<'a, RegionSubscription>,
            ReadStorage<'a, Player>,
            ReadStorage<'a, Presence>,
            ReadStorage<'a, SpectatingEntity>,
            ReadStorage<'a, Client>,
            WriteStorage<'a, Last<Pos>>,
            WriteStorage<'a, Last<Vel>>,
            WriteStorage<'a, Last<Ori>>,
            WriteStorage<'a, ForceUpdate>,
            WriteStorage<'a, InventoryUpdateBuffer>,
        ),
    );

    const NAME: &'static str = "entity_sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (
            entities,
            tick,
            player_physics_settings,
            tracked_storages,
            time_of_day,
            time,
            calendar,
            time_scale,
            region_map,
            trackers,
            mut deleted_entities,
            outcomes,
            editable_settings,
            semantic_outbox,
            (
                positions,
                velocities,
                orientations,
                subscriptions,
                players,
                presences,
                spectating_entities,
                clients,
                mut last_pos,
                mut last_vel,
                mut last_ori,
                mut force_updates,
                mut inventory_update_buffers,
            ),
        ): Self::SystemData,
    ) {
        let tick = tick.0;

        // Storages already provided in `TrackedStorages` that we need to use
        // for other things besides change detection.
        let uids = &tracked_storages.uid;
        let colliders = &tracked_storages.collider;
        let inventories = &tracked_storages.inventory;
        let is_rider = &tracked_storages.is_rider;

        // To send entity updates
        // 1. Iterate through regions
        // 2. Iterate through region subscribers (ie clients)
        //     - Collect a list of entity ids for clients who are subscribed to this
        //       region (hash calc to check each)
        // 3. Iterate through events from that region
        //     - For each entity entered event, iterate through the client list and
        //       check if they are subscribed to the source (hash calc per subscribed
        //       client per entity event), if not subscribed to the source send a entity
        //       creation message to that client
        //     - For each entity left event, iterate through the client list and check
        //       if they are subscribed to the destination (hash calc per subscribed
        //       client per entity event)
        // 4. Iterate through entities in that region
        // 5. Inform clients of the component changes for that entity
        //     - Throttle update rate base on distance to each client

        // Sync physics and other components
        // via iterating through regions (in parallel)

        // Pre-collect regions paired with deleted entity list so we can iterate over
        // them in parallel below
        let regions_and_deleted_entities = region_map
            .iter()
            .map(|(key, region)| (key, region, deleted_entities.take_deleted_in_region(key)))
            .collect::<Vec<_>>();

        use rayon::iter::{IntoParallelIterator, ParallelIterator};
        job.cpu_stats.measure(common_ecs::ParMode::Rayon);
        common_base::prof_span!(guard, "regions");
        regions_and_deleted_entities.into_par_iter().for_each_init(
            || {
                common_base::prof_span!(guard, "entity sync rayon job");
                guard
            },
            |_guard, (key, region, deleted_entities_in_region)| {
                // Assemble subscriber list for this region by iterating through clients and
                // checking if they are subscribed to this region
                let mut subscribers = (
                    &clients,
                    &entities,
                    presences.maybe(),
                    &subscriptions,
                    &positions,
                )
                    .join()
                    .filter_map(|(client, entity, presence, subscription, pos)| {
                        if presence.is_some() && subscription.regions.contains(&key) {
                            Some((client, &subscription.regions, entity, *pos))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                for event in region.events() {
                    match event {
                        RegionEvent::Entered(id, maybe_key) => {
                            // Don't process newly created entities here (redundant network
                            // messages)
                            if trackers.uid.inserted().contains(*id) {
                                continue;
                            }
                            let entity = entities.entity(*id);
                            if let Some(pkg) = positions
                                .get(entity)
                                .map(|pos| (pos, velocities.get(entity), orientations.get(entity)))
                                .and_then(|(pos, vel, ori)| {
                                    tracked_storages.create_entity_package(
                                        entity,
                                        Some(*pos),
                                        vel.copied(),
                                        ori.copied(),
                                    )
                                })
                            {
                                let create_msg = ServerGeneral::CreateEntity(pkg);
                                // APEX-T3.3.13: subject = the CREATED entity's own uid --
                                // unique per (client, region, tick) by construction (an
                                // entity has exactly one Entered event per actual creation
                                // in this region), so local_ordinal stays a constant 0.
                                let create_subject =
                                    uids.get(entity).map(|&uid| CanonicalSubjectKeyV1::for_uid(uid));
                                for (client, regions, client_entity, _) in &mut subscribers {
                                    if maybe_key
                                    .as_ref()
                                    .map(|key| !regions.contains(key))
                                    .unwrap_or(true)
                                    // Client doesn't need to know about itself
                                    && *client_entity != entity
                                    {
                                        let enqueued = create_subject.clone().is_some_and(|subject| {
                                            try_enqueue_entity_sync_intent(
                                                &semantic_outbox,
                                                client.semantic_send_state().map(|s| s.binding()),
                                                create_msg.clone(),
                                                tick,
                                                SemanticPayloadRankV1::Create,
                                                subject,
                                                ordinals::CREATE_OR_DELETE_ENTITY,
                                            )
                                        });
                                        if !enqueued {
                                            client.send_fallible(create_msg.clone());
                                        }
                                    }
                                }
                            }
                        },
                        RegionEvent::Left(id, maybe_key) => {
                            // Lookup UID for entity
                            if let Some(&uid) = uids.get(entities.entity(*id)) {
                                for (client, regions, _, _) in &mut subscribers {
                                    if maybe_key
                                        .as_ref()
                                        .map(|key| !regions.contains(key))
                                        .unwrap_or(true)
                                    {
                                        // TODO: I suspect it would be more efficient (in terms of
                                        // bandwidth) to batch messages like this (same in
                                        // subscription.rs).
                                        // APEX-T3.3.13: subject = the DELETED entity's own
                                        // uid; local_ordinal constant 0, same reasoning as
                                        // the Create case above.
                                        let enqueued = try_enqueue_entity_sync_intent(
                                            &semantic_outbox,
                                            client.semantic_send_state().map(|s| s.binding()),
                                            ServerGeneral::DeleteEntity(uid),
                                            tick,
                                            SemanticPayloadRankV1::Delete,
                                            CanonicalSubjectKeyV1::for_uid(uid),
                                            ordinals::CREATE_OR_DELETE_ENTITY,
                                        );
                                        if !enqueued {
                                            client.send_fallible(ServerGeneral::DeleteEntity(uid));
                                        }
                                    }
                                }
                            }
                        },
                    }
                }

                // Sync tracked components
                // Get deleted entities in this region from DeletedEntities
                let (mut entity_sync_package, mut comp_sync_package) = trackers
                    .create_sync_packages(
                        &tracked_storages,
                        region.entities(),
                        deleted_entities_in_region,
                    );
                // DET-NET-011/012 (v6, stage 1): stamp the server sim tick —
                // the client can align packages across streams by tick.
                entity_sync_package.sync_tick = tick;
                comp_sync_package.sync_tick = tick;
                // APEX-T3.3.13: V1-attached clients (none reachable live today,
                // T3.3.05) never reach the lazy prepare/send_prepared machinery
                // below at all -- they get their own owned EntitySync/CompSync
                // ServerGeneral values enqueued as intents instead. Cloning
                // entity_sync_package/comp_sync_package here (once, not per
                // client) is what lets the ORIGINAL values still feed the
                // untouched Legacy lazy-prepare path exactly as before.
                let entity_sync_package_for_v1 = entity_sync_package.clone();
                let comp_sync_package_for_v1 = comp_sync_package.clone();
                let region_subject = CanonicalSubjectKeyV1::for_region(key);
                // We lazily initialize the the synchronization messages in case there are no
                // clients.
                let mut entity_comp_sync = Either::Left((entity_sync_package, comp_sync_package));
                for (client, _, client_entity, _) in &mut subscribers {
                    let recipient_binding = client.semantic_send_state().map(|s| s.binding());
                    if try_enqueue_entity_sync_intent(
                        &semantic_outbox,
                        recipient_binding,
                        ServerGeneral::EntitySync(entity_sync_package_for_v1.clone()),
                        tick,
                        SemanticPayloadRankV1::EntitySync,
                        region_subject.clone(),
                        ordinals::PAIRED_ENTITYSYNC,
                    ) {
                        try_enqueue_entity_sync_intent(
                            &semantic_outbox,
                            recipient_binding,
                            ServerGeneral::CompSync(
                                comp_sync_package_for_v1.clone(),
                                force_updates.get(*client_entity).map_or(0, |f| f.counter()),
                            ),
                            tick,
                            SemanticPayloadRankV1::CompSync,
                            region_subject.clone(),
                            ordinals::PAIRED_COMPSYNC,
                        );
                        continue;
                    }

                    let msg = entity_comp_sync.right_or_else(
                        |(entity_sync_package, comp_sync_package)| {
                            (
                                client.prepare(ServerGeneral::EntitySync(entity_sync_package)),
                                client.prepare(ServerGeneral::CompSync(
                                    comp_sync_package,
                                    force_updates.get(*client_entity).map_or(0, |f| f.counter()),
                                )),
                            )
                        },
                    );
                    // We don't care much about stream errors here since they could just represent
                    // network disconnection, which is handled elsewhere.
                    let _ = client.send_prepared(&msg.0);
                    let _ = client.send_prepared(&msg.1);
                    entity_comp_sync = Either::Right(msg);
                }

                for (client, _, client_entity, client_pos) in &mut subscribers {
                    let mut comp_sync_package = CompSyncPackage::new();
                    // DET-NET-012 (v6, stage 1): tick stamp.
                    comp_sync_package.sync_tick = tick;

                    for (_, entity, &uid, (&pos, last_pos), vel, ori, collider) in (
                        region.entities(),
                        &entities,
                        uids,
                        (&positions, last_pos.mask().maybe()),
                        (&velocities, last_vel.mask().maybe()).maybe(),
                        (&orientations, last_vel.mask().maybe()).maybe(),
                        colliders.maybe(),
                    )
                        .join()
                    {
                        // Decide how regularly to send physics updates.
                        let send_now = if client_entity == &entity {
                            should_sync_client_physics(
                                entity,
                                &player_physics_settings,
                                &players,
                                &force_updates,
                                is_rider,
                                &editable_settings,
                            )
                        } else if matches!(collider, Some(Collider::Voxel { .. })) {
                            // Things with a voxel collider (airships, etc.) need to have very
                            // stable physics so we always send updated
                            // for these where we can.
                            true
                        } else {
                            // Throttle update rates for all other entities based on distance to
                            // client
                            let distance_sq = client_pos.0.distance_squared(pos.0);
                            let id_staggered_tick = tick + entity.id() as u64;

                            // More entities farther away so checks start there
                            if distance_sq > 500.0f32.powi(2) {
                                id_staggered_tick.is_multiple_of(32)
                            } else if distance_sq > 300.0f32.powi(2) {
                                id_staggered_tick.is_multiple_of(16)
                            } else if distance_sq > 200.0f32.powi(2) {
                                id_staggered_tick.is_multiple_of(8)
                            } else if distance_sq > 120.0f32.powi(2) {
                                id_staggered_tick.is_multiple_of(6)
                            } else if distance_sq > 64.0f32.powi(2) {
                                id_staggered_tick.is_multiple_of(3)
                            } else if distance_sq > 24.0f32.powi(2) {
                                id_staggered_tick.is_multiple_of(2)
                            } else {
                                true
                            }
                        };

                        add_physics_components(
                            send_now,
                            &mut comp_sync_package,
                            uid,
                            pos,
                            last_pos,
                            ori,
                            vel,
                        );
                    }

                    // TODO: force update counter only needs to be sent once per frame (and only if
                    // it changed, although it might not be worth having a separate message for
                    // optionally sending it since individual messages may have a bandwidth
                    // overhead), however, here we send it potentially 2 times per subscribed
                    // region by including it in the `CompSync` message.
                    let comp_sync_msg = ServerGeneral::CompSync(
                        comp_sync_package,
                        force_updates.get(*client_entity).map_or(0, |f| f.counter()),
                    );
                    // APEX-T3.3.13: same region subject as the paired block above,
                    // local_ordinal 1 (not 0) -- this is a SECOND, distinct CompSync
                    // send for the same (recipient, region, tick), and the packet's
                    // own rule ("two intents with the same recipient, stream, and
                    // order key are a terminal producer bug") means it needs its own
                    // slot in the order key, not a collision with the paired block's
                    // CompSync at local_ordinal 0.
                    if !try_enqueue_entity_sync_intent(
                        &semantic_outbox,
                        client.semantic_send_state().map(|s| s.binding()),
                        comp_sync_msg.clone(),
                        tick,
                        SemanticPayloadRankV1::CompSync,
                        CanonicalSubjectKeyV1::for_region(key),
                        ordinals::THROTTLE_COMPSYNC,
                    ) {
                        client.send_fallible(comp_sync_msg);
                    }
                }
            },
        );
        drop(guard);
        job.cpu_stats.measure(common_ecs::ParMode::Single);

        // Sync components that are only synced for the client's own entity.
        for (entity, client, &uid, (maybe_pos, last_pos), vel, ori) in (
            &entities,
            &clients,
            uids,
            (positions.maybe(), last_pos.mask().maybe()),
            (&velocities, last_vel.mask().maybe()).maybe(),
            (&orientations, last_vel.mask().maybe()).maybe(),
        )
            .join()
        {
            // Include additional components for clients that aren't in a region (e.g. due
            // to having no position or have sync_me as `false`) since those
            // won't be synced above.
            let include_all_comps = region_map.in_region_map(entity);

            let mut comp_sync_package = trackers.create_sync_from_client_package(
                &tracked_storages,
                entity,
                include_all_comps,
            );

            if include_all_comps && let Some(&pos) = maybe_pos {
                let send_now = should_sync_client_physics(
                    entity,
                    &player_physics_settings,
                    &players,
                    &force_updates,
                    is_rider,
                    &editable_settings,
                );
                add_physics_components(
                    send_now,
                    &mut comp_sync_package,
                    uid,
                    pos,
                    last_pos,
                    ori,
                    vel,
                );
            }

            // DET-NET-012 (v6, stage 1): tick stamp.
            comp_sync_package.sync_tick = tick;
            if !comp_sync_package.is_empty() {
                // APEX-T3.3.14: the (c) sequential own-entity CompSync
                // site Fable's T3.3.13 scope ruling deferred here.
                // Subject = the receiving client's own entity uid
                // (already in scope from the join above).
                let msg = ServerGeneral::CompSync(comp_sync_package, force_updates.get(entity).map_or(0, |f| f.counter()));
                if !try_enqueue_entity_sync_intent(
                    &semantic_outbox,
                    client.semantic_send_state().map(|s| s.binding()),
                    msg.clone(),
                    tick,
                    SemanticPayloadRankV1::CompSync,
                    CanonicalSubjectKeyV1::for_uid(uid),
                    ordinals::OWN_ENTITY_COMPSYNC,
                ) {
                    client.send_fallible(msg);
                }
            }
        }

        for (entity, client, spectating_entity) in
            (&entities, &clients, &spectating_entities).join()
        {
            // TODO: If the spectated entity is out of range while a change occurs it will
            // cause the client to log errors, and those changes will be missed
            // by the spectating entity.
            //
            // Additionally, when we stop spectating we don't delete the components that are
            // synced for spectators. Leaving stale components on the client.
            let mut comp_sync_package = trackers.create_sync_from_spectated_entity_package(
                &tracked_storages,
                entity,
                spectating_entity.0,
            );

            // DET-NET-012 (v6, stage 1): tick stamp.
            comp_sync_package.sync_tick = tick;
            if !comp_sync_package.is_empty() {
                // APEX-T3.3.14: the (d) sequential spectator CompSync
                // site Fable's T3.3.13 scope ruling deferred here.
                // Subject = the SPECTATING client's own entity uid (the
                // recipient this message is addressed to), not the
                // spectated target -- distinct local_ordinal from (c)
                // since the same client could in principle appear in
                // both loops in one tick.
                let msg = ServerGeneral::CompSync(comp_sync_package, force_updates.get(entity).map_or(0, |f| f.counter()));
                let enqueued = uids.get(entity).is_some_and(|&uid| {
                    try_enqueue_entity_sync_intent(
                        &semantic_outbox,
                        client.semantic_send_state().map(|s| s.binding()),
                        msg.clone(),
                        tick,
                        SemanticPayloadRankV1::CompSync,
                        CanonicalSubjectKeyV1::for_uid(uid),
                        ordinals::SPECTATOR_COMPSYNC,
                    )
                });
                if !enqueued {
                    client.send_fallible(msg);
                }
            }
        }

        // Update the last physics components for each entity

        (
            &entities,
            &positions,
            velocities.maybe(),
            orientations.maybe(),
            last_pos.entries(),
            last_vel.entries(),
            last_ori.entries(),
        )
            .lend_join()
            .for_each(|(_, &pos, vel, ori, last_pos, last_vel, last_ori)| {
                last_pos.replace(Last(pos));
                vel.and_then(|&v| last_vel.replace(Last(v)));
                ori.and_then(|&o| last_ori.replace(Last(o)));
            });

        // Handle entity deletion in regions that don't exist in RegionMap
        // (theoretically none)
        for (region_key, deleted) in deleted_entities.take_remaining_deleted() {
            for client in (presences.maybe(), &subscriptions, &clients)
                .join()
                .filter_map(|(presence, subscription, client)| {
                    if presence.is_some() && subscription.regions.contains(&region_key) {
                        Some(client)
                    } else {
                        None
                    }
                })
            {
                for uid in &deleted {
                    client.send_fallible(ServerGeneral::DeleteEntity(*uid));
                }
            }
        }

        let mut entities_to_remove_buf = Vec::new();

        // Sync inventories
        for (entity, buf, inventory, client) in (
            &entities,
            &mut inventory_update_buffers,
            inventories.maybe(),
            clients.maybe(),
        )
            .join()
        {
            let Some(inventory) = inventory else {
                dev_panic!(format!(
                    "Entity without Inventory has InventoryUpdateBuffer component. This is a bug. \
                     entity={:?}",
                    entity
                ));
                entities_to_remove_buf.push(entity);
                continue;
            };
            let Some(client) = client else {
                dev_panic!(format!(
                    "Entity without Client has InventoryUpdateBuffer component. This is a bug. \
                     entity={:?}",
                    entity
                ));
                entities_to_remove_buf.push(entity);
                continue;
            };

            let events = buf.take_events();
            if !events.is_empty() {
                // APEX-T3.3.14a: subject = the receiving client's own
                // entity uid -- an inventory update is inherently
                // "sync this entity's own inventory to itself", so
                // there is at most one such message per (client, tick)
                // and no sibling to disambiguate against.
                let msg = ServerGeneral::InventoryUpdate(inventory.clone(), events);
                let enqueued = uids.get(entity).is_some_and(|&uid| {
                    try_enqueue_entity_sync_intent(
                        &semantic_outbox,
                        client.semantic_send_state().map(|s| s.binding()),
                        msg.clone(),
                        tick,
                        SemanticPayloadRankV1::InventoryUpdate,
                        CanonicalSubjectKeyV1::for_uid(uid),
                        ordinals::INVENTORY_UPDATE,
                    )
                });
                if !enqueued {
                    client.send_fallible(msg);
                }
            }
        }

        // In optimized builds, remove InventoryUpdateBuffer component from entities
        // that shouldn't have it (builds with debug assertions will panic)
        for entity in entities_to_remove_buf {
            inventory_update_buffers.remove(entity);
        }

        // Consume/clear the current outcomes and convert them to a vec
        let outcomes = outcomes.recv_all().collect::<Vec<_>>();

        // Sync outcomes
        for (presence, pos, client) in (presences.maybe(), positions.maybe(), &clients).join() {
            let is_near = |o_pos: Vec3<f32>| {
                pos.zip_with(presence, |pos, presence| {
                    pos.0.xy().distance_squared(o_pos.xy())
                        < (presence.entity_view_distance.current() as f32
                            * TerrainChunkSize::RECT_SIZE.x as f32)
                            .powi(2)
                })
            };

            let outcomes = outcomes
                .iter()
                .filter(|o| o.get_pos().and_then(is_near).unwrap_or(true))
                .cloned()
                .collect::<Vec<_>>();

            if !outcomes.is_empty() {
                // APEX-T3.3.14a: subject = a fixed singleton label, not
                // per-entity -- outcomes are miscellaneous world events,
                // not owned by any one entity. Safe to share across every
                // recipient in the same tick: the "no duplicate order
                // key" rule is scoped per-recipient (each client's own
                // intent carries its own `recipient`), so identical
                // subject/ordinal for DIFFERENT clients never collides.
                let msg = ServerGeneral::Outcomes(outcomes);
                if !try_enqueue_entity_sync_intent(
                    &semantic_outbox,
                    client.semantic_send_state().map(|s| s.binding()),
                    msg.clone(),
                    tick,
                    SemanticPayloadRankV1::Outcomes,
                    CanonicalSubjectKeyV1::for_singleton("outcomes"),
                    ordinals::OUTCOMES,
                ) {
                    client.send_fallible(msg);
                }
            }
        }

        // Remove all force flags.
        for force_update in (&mut force_updates).join() {
            force_update.clear();
        }

        // Sync resources
        // TODO: doesn't really belong in this system (rename system or create another
        // system?)
        const TOD_SYNC_FREQ: u64 = 100;
        if tick % TOD_SYNC_FREQ == 0 {
            // APEX-T3.3.14a: TimeOfDay is identical for every recipient
            // (no per-client field, unlike CompSync's force-update
            // counter), so building the owned ServerGeneral value once
            // and cloning it per V1 client is exact-content-equivalent
            // to Legacy's own prepare/send_prepared byte-sharing --
            // same content reaches everyone either way, only WHICH
            // clients still use the lazy-prepared-bytes optimization
            // changes (V1 clients no longer do, matching every other
            // migrated site in this file).
            let tod_msg = ServerGeneral::TimeOfDay(*time_of_day, (*calendar).clone(), *time, *time_scale);
            let tod_subject = CanonicalSubjectKeyV1::for_singleton("time_of_day");
            let mut tod_lazymsg = None;
            for client in (&clients).join() {
                if try_enqueue_entity_sync_intent(
                    &semantic_outbox,
                    client.semantic_send_state().map(|s| s.binding()),
                    tod_msg.clone(),
                    tick,
                    SemanticPayloadRankV1::TimeOfDay,
                    tod_subject.clone(),
                    ordinals::TIME_OF_DAY,
                ) {
                    continue;
                }
                let msg = tod_lazymsg.unwrap_or_else(|| client.prepare(tod_msg.clone()));
                // We don't care much about stream errors here since they could just represent
                // network disconnection, which is handled elsewhere.
                let _ = client.send_prepared(&msg);
                tod_lazymsg = Some(msg);
            }
        }
    }
}

/// Determines whether a client should receive an update about its own physics
/// components.
fn should_sync_client_physics(
    entity: specs::Entity,
    player_physics_settings: &PlayerPhysicsSettings,
    players: &ReadStorage<'_, Player>,
    force_updates: &WriteStorage<'_, ForceUpdate>,
    is_rider: &ReadStorage<'_, Is<Rider>>,
    editable_settings: &EditableSettings,
) -> bool {
    let server_authoritative_physics = players.get(entity).is_none_or(|player| {
        player_physics_settings
            .settings
            .get(&player.uuid())
            .is_some_and(|settings| settings.server_authoritative_physics_optin())
            || editable_settings
                .server_physics_force_list
                .contains_key(&player.uuid())
    });
    // Don't send client physics updates about itself unless force update is
    // set or the client is subject to
    // server-authoritative physics
    force_updates.get(entity).is_some_and(|f| f.is_forced())
        || server_authoritative_physics
        || is_rider.contains(entity)
}

/// Adds physics components if `send_now` is true or `Option<Last<T>>` is
/// `None`.
///
/// If `Last<T>` isn't present, this is recorded as an insertion rather than a
/// modification.
fn add_physics_components(
    send_now: bool,
    comp_sync_package: &mut CompSyncPackage<common_net::msg::EcsCompPacket>,
    uid: Uid,
    pos: Pos,
    last_pos: Option<u32>,
    ori: Option<(&Ori, Option<u32>)>,
    vel: Option<(&Vel, Option<u32>)>,
) {
    if last_pos.is_none() {
        comp_sync_package.comp_inserted(uid, pos);
    } else if send_now {
        comp_sync_package.comp_modified(uid, pos);
    }

    if let Some((v, last_vel)) = vel {
        if last_vel.is_none() {
            comp_sync_package.comp_inserted(uid, *v);
        } else if send_now {
            comp_sync_package.comp_modified(uid, *v);
        }
    }

    if let Some((o, last_ori)) = ori {
        if last_ori.is_none() {
            comp_sync_package.comp_inserted(uid, *o);
        } else if send_now {
            comp_sync_package.comp_modified(uid, *o);
        }
    }
}

/// `APEX-T3.3.13` tests: `try_enqueue_entity_sync_intent`'s own
/// direct unit coverage. It's a pure function of its explicit
/// arguments (no `&Client`, see its own doc comment for why), so it
/// gets the same direct-unit-test treatment T3.3.08/10's pure
/// validation functions did -- no live `Client`/ECS harness needed.
#[cfg(test)]
mod semantic_intents {
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::envelope::ActiveSessionBindingV1;

    use super::*;

    fn binding(seed: u8) -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([seed; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([seed.wrapping_add(1); 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    #[test]
    fn no_binding_does_not_enqueue() {
        let outbox = ServerSemanticOutboxV1::new();
        let enqueued = try_enqueue_entity_sync_intent(
            &outbox,
            None,
            ServerGeneral::UpdateRecipes,
            1,
            SemanticPayloadRankV1::Create,
            CanonicalSubjectKeyV1::for_singleton("x"),
            ordinals::CREATE_OR_DELETE_ENTITY,
        );
        assert!(!enqueued);
        assert!(outbox.take_pending().is_empty());
    }

    #[test]
    fn some_binding_enqueues_with_expected_order_key() {
        let outbox = ServerSemanticOutboxV1::new();
        let b = binding(1);
        let enqueued = try_enqueue_entity_sync_intent(
            &outbox,
            Some(b),
            ServerGeneral::DeleteEntity(Uid(std::num::NonZeroU64::new(7).unwrap())),
            42,
            SemanticPayloadRankV1::Delete,
            CanonicalSubjectKeyV1::for_uid(Uid(std::num::NonZeroU64::new(7).unwrap())),
            ordinals::CREATE_OR_DELETE_ENTITY,
        );
        assert!(enqueued);
        let pending = outbox.take_pending();
        assert_eq!(pending.len(), 1);
        let intent = &pending[0];
        assert_eq!(intent.recipient, b);
        assert_eq!(intent.semantic_stream, common_net::msg::envelope::SemanticStreamIdV1::General);
        assert_eq!(intent.order_key.source_tick, 42);
        assert_eq!(intent.order_key.phase_rank, phase_rank(Phase::Create));
        assert_eq!(intent.order_key.producer_rank, SemanticProducerV1::EntitySync.producer_rank());
        assert_eq!(intent.order_key.payload_rank, SemanticPayloadRankV1::Delete.payload_rank());
        assert_eq!(intent.order_key.local_ordinal, ordinals::CREATE_OR_DELETE_ENTITY);
    }

    /// Create and Delete both use local_ordinal 0 -- proves that's safe
    /// because their DIFFERENT payload_rank already keeps the order keys
    /// distinct even for the same subject (same entity created-then-
    /// deleted would share a uid, hypothetically).
    #[test]
    fn create_and_delete_for_the_same_uid_do_not_collide() {
        let outbox = ServerSemanticOutboxV1::new();
        let b = binding(1);
        let uid = Uid(std::num::NonZeroU64::new(3).unwrap());
        try_enqueue_entity_sync_intent(
            &outbox,
            Some(b),
            ServerGeneral::DeleteEntity(uid),
            1,
            SemanticPayloadRankV1::Create,
            CanonicalSubjectKeyV1::for_uid(uid),
            ordinals::CREATE_OR_DELETE_ENTITY,
        );
        try_enqueue_entity_sync_intent(
            &outbox,
            Some(b),
            ServerGeneral::DeleteEntity(uid),
            1,
            SemanticPayloadRankV1::Delete,
            CanonicalSubjectKeyV1::for_uid(uid),
            ordinals::CREATE_OR_DELETE_ENTITY,
        );
        let mut pending = outbox.take_pending();
        pending.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        assert_ne!(pending[0].order_key, pending[1].order_key, "must not collide despite identical subject");
    }

    /// The paired block's CompSync (local_ordinal 0) and the physics-
    /// throttle block's CompSync (local_ordinal 1) target the same
    /// recipient/stream/payload_rank/subject in the real code -- proves
    /// the local_ordinal split is what keeps them from colliding.
    #[test]
    fn paired_and_throttle_compsync_do_not_collide() {
        let outbox = ServerSemanticOutboxV1::new();
        let b = binding(1);
        let subject = CanonicalSubjectKeyV1::for_region(Vec2::new(0, 0));
        for local_ordinal in [ordinals::PAIRED_COMPSYNC, ordinals::THROTTLE_COMPSYNC] {
            try_enqueue_entity_sync_intent(
                &outbox,
                Some(b),
                ServerGeneral::CompSync(CompSyncPackage::new(), 0),
                1,
                SemanticPayloadRankV1::CompSync,
                subject.clone(),
                local_ordinal,
            );
        }
        let mut pending = outbox.take_pending();
        pending.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        assert_ne!(pending[0].order_key, pending[1].order_key);
    }

    /// `APEX-T3.3.14`: the (c)/(d) sequential own-entity and spectator
    /// CompSync sites both key off the RECEIVING client's own entity
    /// uid, and a spectating client's own entity could in principle
    /// appear in both loops in the same tick -- proves the ordinal split
    /// is what keeps them from colliding even with an identical subject.
    #[test]
    fn own_entity_and_spectator_compsync_do_not_collide() {
        let outbox = ServerSemanticOutboxV1::new();
        let b = binding(1);
        let subject = CanonicalSubjectKeyV1::for_uid(Uid(std::num::NonZeroU64::new(11).unwrap()));
        for local_ordinal in [ordinals::OWN_ENTITY_COMPSYNC, ordinals::SPECTATOR_COMPSYNC] {
            try_enqueue_entity_sync_intent(
                &outbox,
                Some(b),
                ServerGeneral::CompSync(CompSyncPackage::new(), 0),
                1,
                SemanticPayloadRankV1::CompSync,
                subject.clone(),
                local_ordinal,
            );
        }
        let mut pending = outbox.take_pending();
        pending.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        assert_ne!(pending[0].order_key, pending[1].order_key);
    }
}

/// `APEX-T3.3.13` requirement 4 (Fable's elevated-gate ruling): prove
/// entity_sync's intent construction produces byte-identical, order-
/// independent results regardless of how many rayon workers process the
/// regions, or what order the regions are visited in. Exercises the
/// EXACT SAME `try_enqueue_entity_sync_intent` the real system calls
/// (not a re-implementation) -- driven by synthetic per-region fixture
/// data instead of a live `specs::World`, for the same reason
/// `semantic_intents` above skips a `Client` (no lightweight way to
/// build one here). This is a faithful test of the real concurrency
/// pattern entity_sync now uses (rayon `par_iter` over regions, each
/// producing intents into one shared `Mutex`-backed outbox) even though
/// it doesn't invoke `Sys::run` itself.
#[cfg(test)]
mod semantic_intents_parallel {
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};
    use common_net::msg::envelope::{
        ActiveSessionBindingV1, SemanticPayloadEncodingV1, SemanticRouteV1, encode_payload_v1,
        net_envelope_profile_root_v1, payload_digest_v1,
    };
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    use super::*;

    fn fixture_binding() -> ActiveSessionBindingV1 {
        ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([9; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([10; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        }
    }

    /// One "region" of fixture work: a create + a delete + a paired-
    /// style CompSync + a throttle-style CompSync, exactly the four
    /// call-site shapes the real per-region closure produces, using the
    /// SAME subject/payload_rank/local_ordinal conventions as the real
    /// code.
    fn produce_region_intents(outbox: &ServerSemanticOutboxV1, binding: ActiveSessionBindingV1, region_key: Vec2<i32>, tick: u64) {
        let region_ord = (region_key.x.unsigned_abs() as u64) * 1000 + region_key.y.unsigned_abs() as u64;
        let created_uid = Uid(std::num::NonZeroU64::new(region_ord * 2 + 1).unwrap());
        let deleted_uid = Uid(std::num::NonZeroU64::new(region_ord * 2 + 2).unwrap());
        let region_subject = CanonicalSubjectKeyV1::for_region(region_key);

        try_enqueue_entity_sync_intent(
            outbox,
            Some(binding),
            ServerGeneral::DeleteEntity(created_uid), // stand-in payload; only the header fields matter for the tape
            tick,
            SemanticPayloadRankV1::Create,
            CanonicalSubjectKeyV1::for_uid(created_uid),
            ordinals::CREATE_OR_DELETE_ENTITY,
        );
        try_enqueue_entity_sync_intent(
            outbox,
            Some(binding),
            ServerGeneral::DeleteEntity(deleted_uid),
            tick,
            SemanticPayloadRankV1::Delete,
            CanonicalSubjectKeyV1::for_uid(deleted_uid),
            ordinals::CREATE_OR_DELETE_ENTITY,
        );
        try_enqueue_entity_sync_intent(
            outbox,
            Some(binding),
            ServerGeneral::CompSync(CompSyncPackage::new(), 0),
            tick,
            SemanticPayloadRankV1::CompSync,
            region_subject.clone(),
            ordinals::PAIRED_COMPSYNC,
        );
        try_enqueue_entity_sync_intent(
            outbox,
            Some(binding),
            ServerGeneral::CompSync(CompSyncPackage::new(), 0),
            tick,
            SemanticPayloadRankV1::CompSync,
            region_subject,
            ordinals::THROTTLE_COMPSYNC,
        );
    }

    type TapeEntry = (Vec<u8>, u64, u16, u16, u16, u32, [u8; 32]);

    fn tape_of(outbox: &ServerSemanticOutboxV1) -> Vec<TapeEntry> {
        let mut pending = outbox.take_pending();
        pending.sort_by(|a, b| a.total_sort_key().cmp(&b.total_sort_key()));
        pending
            .into_iter()
            .map(|intent| {
                let payload_bytes = encode_payload_v1(&*intent.payload);
                let digest = payload_digest_v1(
                    net_envelope_profile_root_v1(),
                    intent.payload.payload_schema(),
                    SemanticPayloadEncodingV1::Bincode2LegacySerde,
                    &payload_bytes,
                );
                (
                    intent.order_key.subject.as_bytes().to_vec(),
                    intent.order_key.source_tick,
                    intent.order_key.phase_rank,
                    intent.order_key.producer_rank,
                    intent.order_key.payload_rank,
                    intent.order_key.local_ordinal,
                    *digest.as_array(),
                )
            })
            .collect()
    }

    fn run_with_pool_size(regions: &[Vec2<i32>], workers: usize) -> Vec<TapeEntry> {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(workers).build().unwrap();
        let outbox = ServerSemanticOutboxV1::new();
        let binding = fixture_binding();
        pool.install(|| {
            regions.to_vec().into_par_iter().for_each(|region_key| {
                produce_region_intents(&outbox, binding, region_key, 7);
            });
        });
        tape_of(&outbox)
    }

    fn fixture_regions() -> Vec<Vec2<i32>> { (0..12).map(|i| Vec2::new(i, -i)).collect() }

    #[test]
    fn byte_identical_tape_across_worker_counts() {
        let regions = fixture_regions();
        let tape_1 = run_with_pool_size(&regions, 1);
        let tape_2 = run_with_pool_size(&regions, 2);
        let tape_8 = run_with_pool_size(&regions, 8);
        assert_eq!(tape_1, tape_2);
        assert_eq!(tape_1, tape_8);
    }

    #[test]
    fn byte_identical_tape_under_region_permutation() {
        let mut regions = fixture_regions();
        let forward = run_with_pool_size(&regions, 8);
        regions.reverse();
        let reversed = run_with_pool_size(&regions, 8);
        regions.rotate_left(5);
        let rotated = run_with_pool_size(&regions, 8);
        assert_eq!(forward, reversed);
        assert_eq!(forward, rotated);
    }

    /// Non-vacuity (Fable's elevated-gate requirement 4): the harness
    /// technique above must be able to FAIL, not just always pass.
    /// Deliberately deterministic (not a real thread race, which turned
    /// out not to reliably manifest with this fixture's cheap per-item
    /// work -- observed directly: an earlier `AtomicU32`-race version of
    /// this test occasionally produced IDENTICAL tapes across worker
    /// counts by luck, exactly the kind of flaky non-proof this
    /// campaign's own culture rejects). Instead this directly compares
    /// two DIFFERENT, both individually-deterministic assignments of
    /// `local_ordinal` -- one mirroring the real per-call-site-constant
    /// scheme, one mirroring a plausible-but-wrong "arrival index"
    /// scheme -- proving the tape/comparison TECHNIQUE itself can tell
    /// two genuinely different constructions apart, which is the actual
    /// property non-vacuity needs: not "did a race happen to fire this
    /// run" but "can the check discriminate a real divergence at all."
    #[test]
    fn falsifier_arrival_index_local_ordinal_diverges_from_the_real_construction() {
        fn produce_region_intent_broken(outbox: &ServerSemanticOutboxV1, binding: ActiveSessionBindingV1, region_key: Vec2<i32>, tick: u64, arrival_index: u32) {
            // Deliberately broken: local_ordinal is the region's ARRIVAL
            // INDEX in whatever order it happened to be processed,
            // instead of the real code's fixed per-call-site constant
            // (0 for the paired block's CompSync, 1 for the throttle
            // block's) -- a realistic "leaking knob" bug class
            // (accidentally keying off iteration/arrival order instead
            // of call-site identity).
            try_enqueue_entity_sync_intent(
                outbox,
                Some(binding),
                ServerGeneral::CompSync(CompSyncPackage::new(), 0),
                tick,
                SemanticPayloadRankV1::CompSync,
                CanonicalSubjectKeyV1::for_region(region_key),
                arrival_index,
            );
        }

        fn run_broken(regions: &[Vec2<i32>]) -> Vec<TapeEntry> {
            let outbox = ServerSemanticOutboxV1::new();
            let binding = fixture_binding();
            for (i, &region_key) in regions.iter().enumerate() {
                produce_region_intent_broken(&outbox, binding, region_key, 7, i as u32);
            }
            tape_of(&outbox)
        }

        let regions = fixture_regions();
        // The real construction's single-CompSync-per-region tape (same
        // shape as run_with_pool_size's fixture, but with only ONE
        // CompSync per region so the ordinals line up 1:1 against the
        // broken variant's ONE-per-region arrival index for a fair,
        // directly comparable pair).
        fn run_real_single_compsync(regions: &[Vec2<i32>]) -> Vec<TapeEntry> {
            let outbox = ServerSemanticOutboxV1::new();
            let binding = fixture_binding();
            for &region_key in regions {
                try_enqueue_entity_sync_intent(
                    &outbox,
                    Some(binding),
                    ServerGeneral::CompSync(CompSyncPackage::new(), 0),
                    7,
                    SemanticPayloadRankV1::CompSync,
                    CanonicalSubjectKeyV1::for_region(region_key),
                    0, // the real per-call-site constant
                );
            }
            tape_of(&outbox)
        }

        let real_tape = run_real_single_compsync(&regions);
        let broken_tape = run_broken(&regions);
        assert_ne!(
            real_tape, broken_tape,
            "arrival-index-derived local_ordinal must diverge from the real fixed-constant construction -- proves the tape comparison technique is not vacuously green"
        );
    }
}
