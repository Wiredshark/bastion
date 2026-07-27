use super::sentinel::{DeletedEntities, TrackedStorages};
use crate::{
    Tick,
    client::Client,
    presence::{self, RegionSubscription},
    semantic_net::{
        order::{SemanticPayloadRankV1, SemanticProducerV1, phase_rank},
        outbox::{CanonicalSubjectKeyV1, ServerSemanticOutboxV1},
    },
};
use common::{
    comp::{Ori, Pos, Presence, Vel},
    region::{RegionEvent, RegionMap, region_in_vd, regions_in_vd},
    terrain::{CoordinateConversions, TerrainChunkSize},
    uid::Uid,
    vol::RectVolSize,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{ServerGeneral, envelope::ActiveSessionBindingV1};
use specs::{
    Entities, Join, LendJoin, Read, ReadExpect, ReadStorage, SystemData, World, WorldExt,
    WriteStorage,
};
use tracing::{debug, error};
use vek::*;

/// `APEX-T3.3.14a`: `subscription.rs`'s own thin wrapper over
/// `ServerSemanticOutboxV1::try_enqueue_if_v1`, mirroring
/// `entity_sync.rs`'s -- Fable's sequencing ruling groups this file
/// into the same "replication family" as `entity_sync.rs`, so it
/// shares that file's `SemanticProducerV1::EntitySync` producer and
/// `Phase::Create` phase rank rather than inventing a new producer for
/// what is, semantically, the same replication work living in a
/// second file.
/// `T3.3.14a`: every `CreateEntity`/`DeleteEntity` subject in this file
/// is `for_uid(the created/deleted entity)`, unique per (client, tick)
/// by construction (same reasoning as `entity_sync.rs`'s own
/// `ordinals::CREATE_OR_DELETE_ENTITY`) -- no sibling to disambiguate
/// against, so a constant `0` is safe.
const CREATE_OR_DELETE_ORDINAL: u32 = 0;

fn try_enqueue_subscription_intent(
    outbox: &ServerSemanticOutboxV1,
    recipient_binding: Option<ActiveSessionBindingV1>,
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

/// This system will update region subscriptions based on client positions
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, RegionMap>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
        WriteStorage<'a, RegionSubscription>,
        Read<'a, DeletedEntities>,
        TrackedStorages<'a>,
        Read<'a, Tick>,
        ReadExpect<'a, ServerSemanticOutboxV1>,
    );

    const NAME: &'static str = "subscription";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            region_map,
            uids,
            positions,
            velocities,
            orientations,
            presences,
            clients,
            mut subscriptions,
            deleted_entities,
            tracked_comps,
            tick,
            semantic_outbox,
        ): Self::SystemData,
    ) {
        let tick = tick.0;
        // To update subscriptions
        // 1. Iterate through clients
        // 2. Calculate current chunk position
        // 3. If chunk is different (use fuzziness) or the client view distance has
        //    changed continue, otherwise return
        // 4. Iterate through subscribed regions
        // 5. Check if region is still in range (use fuzziness)
        // 6. If not in range
        //     - remove from hashset
        //     - inform client of which entities to remove
        // 7. Determine list of regions that are in range and iterate through it
        //    - check if in hashset (hash calc) if not add it
        let mut regions_to_remove = Vec::new();
        for (subscription, pos, presence, client_entity, client) in (
            &mut subscriptions,
            &positions,
            &presences,
            &entities,
            &clients,
        )
            .join()
        {
            let vd = presence.entity_view_distance.current();
            // Calculate current chunk
            let chunk = (Vec2::<f32>::from(pos.0)).as_::<i32>().wpos_to_cpos();
            // Only update regions when moving to a new chunk or if view distance has
            // changed.
            //
            // Uses a fuzzy border to prevent rapid triggering when moving along chunk
            // boundaries.
            if chunk != subscription.fuzzy_chunk
                && (subscription
                    .fuzzy_chunk
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        (e as f32 + 0.5) * sz as f32
                    })
                    - Vec2::from(pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                    e.abs() > (sz / 2 + presence::CHUNK_FUZZ) as f32
                })
                .reduce_or()
                || subscription.last_entity_view_distance != vd
            {
                // Update the view distance
                subscription.last_entity_view_distance = vd;
                // Update current chunk
                subscription.fuzzy_chunk = Vec2::<f32>::from(pos.0).as_::<i32>().wpos_to_cpos();
                // Use the largest side length as our chunk size
                let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
                // Iterate through currently subscribed regions
                for key in &subscription.regions {
                    // Check if the region is not within range anymore
                    if !region_in_vd(
                        *key,
                        pos.0,
                        (vd as f32 * chunk_size)
                            + (presence::CHUNK_FUZZ as f32
                                + presence::REGION_FUZZ as f32
                                + chunk_size)
                                * 2.0f32.sqrt(),
                    ) {
                        // Add to the list of regions to remove
                        regions_to_remove.push(*key);
                    }
                }

                // Iterate through regions to remove
                for key in regions_to_remove.drain(..) {
                    // Remove region from this client's set of subscribed regions
                    subscription.regions.remove(&key);
                    // Tell the client to delete the entities in that region if it exists in the
                    // RegionMap
                    if let Some(region) = region_map.get(key) {
                        // Process entity left events since they won't be processed during entity
                        // sync because this region is no longer subscribed to
                        // TODO: consider changing system ordering??
                        for event in region.events() {
                            match event {
                                RegionEvent::Entered(_, _) => {
                                    // These don't need to be processed because
                                    // this region is being thrown out anyway
                                },
                                RegionEvent::Left(id, maybe_key) => {
                                    // Lookup UID for entity
                                    // Doesn't overlap with entity deletion in sync packages
                                    // because the uid would not be available if the entity was
                                    // deleted
                                    if let Some(&uid) = uids.get(entities.entity(*id))
                                        && !maybe_key
                                            .as_ref()
                                            // Don't need to check that this isn't also in the
                                            // regions to remove since the entity will be removed 
                                            // when we get to that one.
                                            .map(|key| subscription.regions.contains(key))
                                            .unwrap_or(false)
                                    {
                                        let msg = ServerGeneral::DeleteEntity(uid);
                                        if !try_enqueue_subscription_intent(
                                            &semantic_outbox,
                                            client.semantic_send_state().map(|s| s.binding()),
                                            msg.clone(),
                                            tick,
                                            SemanticPayloadRankV1::Delete,
                                            CanonicalSubjectKeyV1::for_uid(uid),
                                            CREATE_OR_DELETE_ORDINAL,
                                        ) {
                                            client.send_fallible(msg);
                                        }
                                    }
                                },
                            }
                        }
                        // Tell client to delete entities in the region
                        for (&uid, _) in (&uids, region.entities()).join() {
                            let msg = ServerGeneral::DeleteEntity(uid);
                            if !try_enqueue_subscription_intent(
                                &semantic_outbox,
                                client.semantic_send_state().map(|s| s.binding()),
                                msg.clone(),
                                tick,
                                SemanticPayloadRankV1::Delete,
                                CanonicalSubjectKeyV1::for_uid(uid),
                                CREATE_OR_DELETE_ORDINAL,
                            ) {
                                client.send_fallible(msg);
                            }
                        }
                    }
                    // Send deleted entities since they won't be processed for this client
                    // in entity sync
                    for &uid in deleted_entities.get_deleted_in_region(key).iter() {
                        let msg = ServerGeneral::DeleteEntity(uid);
                        if !try_enqueue_subscription_intent(
                            &semantic_outbox,
                            client.semantic_send_state().map(|s| s.binding()),
                            msg.clone(),
                            tick,
                            SemanticPayloadRankV1::Delete,
                            CanonicalSubjectKeyV1::for_uid(uid),
                            CREATE_OR_DELETE_ORDINAL,
                        ) {
                            client.send_fallible(msg);
                        }
                    }
                }

                for key in regions_in_vd(
                    pos.0,
                    (vd as f32 * chunk_size)
                        + (presence::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
                ) {
                    // Send client initial info about the entities in this region if it was not
                    // already within the set of subscribed regions
                    if subscription.regions.insert(key)
                        && let Some(region) = region_map.get(key)
                    {
                        (
                                &positions,
                                velocities.maybe(),
                                orientations.maybe(),
                                region.entities(),
                                &entities,
                            )
                                .join()
                                .filter(|(_, _, _, _, e)| *e != client_entity)
                                .filter_map(|(pos, vel, ori, _, entity)| {
                                    tracked_comps.create_entity_package(
                                        entity,
                                        Some(*pos),
                                        vel.copied(),
                                        ori.copied(),
                                    ).zip(uids.get(entity).copied())
                                })
                                // TODO: batch this into a single message
                                .for_each(|(msg, uid)| {
                                    // Send message to create entity and tracked components and
                                    // physics components
                                    let msg = ServerGeneral::CreateEntity(msg);
                                    if !try_enqueue_subscription_intent(
                                        &semantic_outbox,
                                        client.semantic_send_state().map(|s| s.binding()),
                                        msg.clone(),
                                        tick,
                                        SemanticPayloadRankV1::Create,
                                        CanonicalSubjectKeyV1::for_uid(uid),
                                        CREATE_OR_DELETE_ORDINAL,
                                    ) {
                                        client.send_fallible(msg);
                                    }
                                })
                    }
                }
            }
        }
    }
}

/// Initialize region subscription
pub fn initialize_region_subscription(world: &World, entity: specs::Entity) {
    if let (Some(client_pos), Some(presence), Some(client)) = (
        world.read_storage::<Pos>().get(entity),
        world.read_storage::<Presence>().get(entity),
        world.write_storage::<Client>().get(entity),
    ) {
        let fuzzy_chunk = (Vec2::<f32>::from(client_pos.0))
            .as_::<i32>()
            .wpos_to_cpos();
        let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
        let regions = regions_in_vd(
            client_pos.0,
            (presence.entity_view_distance.current() as f32 * chunk_size)
                + (presence::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
        );

        let region_map = world.read_resource::<RegionMap>();
        let tracked_comps = TrackedStorages::fetch(world);
        let uids = world.read_storage::<Uid>();
        let semantic_outbox = world.read_resource::<ServerSemanticOutboxV1>();
        let tick = world.read_resource::<Tick>().0;
        for key in &regions {
            if let Some(region) = region_map.get(*key) {
                (
                    &world.read_storage::<Pos>(), // We assume all these entities have a position
                    world.read_storage::<Vel>().maybe(),
                    world.read_storage::<Ori>().maybe(),
                    region.entities(),
                    &world.entities(),
                )
                .join()
                // Don't send client its own components because we do that below
                .filter(|t| t.4 != entity)
                .filter_map(|(pos, vel, ori, _, entity)|
                    tracked_comps.create_entity_package(
                        entity,
                        Some(*pos),
                        vel.copied(),
                        ori.copied(),
                    ).zip(uids.get(entity).copied())
                )
                .for_each(|(msg, uid)| {
                    // Send message to create entity and tracked components and physics components
                    let msg = ServerGeneral::CreateEntity(msg);
                    if !try_enqueue_subscription_intent(
                        &semantic_outbox,
                        client.semantic_send_state().map(|s| s.binding()),
                        msg.clone(),
                        tick,
                        SemanticPayloadRankV1::Create,
                        CanonicalSubjectKeyV1::for_uid(uid),
                        CREATE_OR_DELETE_ORDINAL,
                    ) {
                        client.send_fallible(msg);
                    }
                });
            }
        }
        // If client position was modified it might not be updated in the region system
        // so we send its components here
        if let Some(pkg) = tracked_comps.create_entity_package(
            entity,
            Some(*client_pos),
            world.read_storage().get(entity).copied(),
            world.read_storage().get(entity).copied(),
        ) {
            let msg = ServerGeneral::CreateEntity(pkg);
            let enqueued = uids.get(entity).copied().is_some_and(|uid| {
                try_enqueue_subscription_intent(
                    &semantic_outbox,
                    client.semantic_send_state().map(|s| s.binding()),
                    msg.clone(),
                    tick,
                    SemanticPayloadRankV1::Create,
                    CanonicalSubjectKeyV1::for_uid(uid),
                    CREATE_OR_DELETE_ORDINAL,
                )
            });
            if !enqueued {
                client.send_fallible(msg);
            }
        }

        if let Err(e) = world.write_storage().insert(entity, RegionSubscription {
            fuzzy_chunk,
            last_entity_view_distance: presence.entity_view_distance.current(),
            regions,
        }) {
            error!(?e, "Failed to insert region subscription component");
        }
    } else {
        debug!(
            ?entity,
            "Failed to initialize region subscription. Couldn't retrieve all the neccesary \
             components on the provided entity"
        );
    }
}

/// `APEX-T3.3.14a`, middle-tier discipline (Fable's sequencing ruling):
/// `try_enqueue_subscription_intent` is a thin wrapper reusing the
/// EXACT mechanism `entity_sync.rs`'s own extensively-tested
/// `semantic_intents` suite already proves (same
/// `ServerSemanticOutboxV1::try_enqueue_if_v1` underneath) -- this file
/// only needs to confirm its own wiring (producer/phase constants,
/// no-binding fallthrough), not re-derive that suite's own coverage.
#[cfg(test)]
mod semantic_intents {
    use common::apex::identity::{ConnectionEpoch, FixedRandomBytesSourceV1, ServerBootId, SessionId};

    use super::*;

    #[test]
    fn no_binding_does_not_enqueue() {
        let outbox = ServerSemanticOutboxV1::new();
        let enqueued = try_enqueue_subscription_intent(
            &outbox,
            None,
            ServerGeneral::DeleteEntity(Uid(std::num::NonZeroU64::new(1).unwrap())),
            1,
            SemanticPayloadRankV1::Delete,
            CanonicalSubjectKeyV1::for_uid(Uid(std::num::NonZeroU64::new(1).unwrap())),
            CREATE_OR_DELETE_ORDINAL,
        );
        assert!(!enqueued);
        assert!(outbox.take_pending().is_empty());
    }

    #[test]
    fn some_binding_enqueues_with_this_files_own_producer_and_phase() {
        let outbox = ServerSemanticOutboxV1::new();
        let b = ActiveSessionBindingV1 {
            server_boot_id: ServerBootId::generate(&mut FixedRandomBytesSourceV1([1; 16])).unwrap(),
            session_id: SessionId::generate(&mut FixedRandomBytesSourceV1([2; 16])).unwrap(),
            epoch: ConnectionEpoch::new(1).unwrap(),
        };
        let uid = Uid(std::num::NonZeroU64::new(9).unwrap());
        let enqueued = try_enqueue_subscription_intent(
            &outbox,
            Some(b),
            ServerGeneral::CreateEntity(common_net::sync::EntityPackage { uid, comps: vec![] }),
            7,
            SemanticPayloadRankV1::Create,
            CanonicalSubjectKeyV1::for_uid(uid),
            CREATE_OR_DELETE_ORDINAL,
        );
        assert!(enqueued);
        let pending = outbox.take_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].recipient, b);
        assert_eq!(pending[0].order_key.phase_rank, phase_rank(Phase::Create));
        assert_eq!(pending[0].order_key.producer_rank, SemanticProducerV1::EntitySync.producer_rank());
        assert_eq!(pending[0].order_key.payload_rank, SemanticPayloadRankV1::Create.payload_rank());
    }
}
