use std::collections::HashMap;

use common::{
    CachedSpatialGrid, comp,
    event::{DeleteEvent, EventBus},
    resources::ProgramTime,
};
use common_ecs::{Origin, Phase, System};
use specs::{Entities, Entity, Join, LendJoin, Read, ReadStorage, WriteStorage};

const MAX_ITEM_MERGE_DIST: f32 = 2.0;
const CHECKS_PER_SECOND: f64 = 10.0; // Start by checking an item 10 times every second

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, comp::PickupItem>,
        ReadStorage<'a, comp::Pos>,
        ReadStorage<'a, comp::LootOwner>,
        ReadStorage<'a, comp::bastion::BastionPile>,
        Read<'a, CachedSpatialGrid>,
        Read<'a, ProgramTime>,
        Read<'a, EventBus<DeleteEvent>>,
        ReadStorage<'a, common::uid::Uid>,
        Read<'a, crate::Tick>,
    );

    const NAME: &'static str = "item";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            entities,
            mut items,
            positions,
            loot_owners,
            bastion_piles,
            spatial_grid,
            program_time,
            delete_bus,
            uids,
            tick,
        ): Self::SystemData,
    ) {
        // ENGOPT6 hunt round 5: the full merge trail — round 4 proved the
        // divergent item deletion is THIS system's comp-take (uid 2's own
        // pickup attempts were out-of-range every tick in both runs), with
        // the check-due schedules byte-equal until the flip tick: one run's
        // check found a merge partner, the other's found none. The partner's
        // own state history was invisible to the recorder — so record every
        // check fire (partner count), every performed merge, and every
        // backoff update.
        let recorder_on = bastion_server::bastion_flight_recorder::enabled();
        let record_merge_event = |uid: u64, note: String| {
            bastion_server::bastion_flight_recorder::record_writer(
                bastion_server::bastion_flight_recorder::WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: tick.0,
                    uid,
                    observation_sequence: 320,
                    snapshot_stage: "item-merge-trail".into(),
                    dispatcher_dependency_proven: false,
                    writer: "sys_item_merge".into(),
                    move_dir: [0.0; 2],
                    move_z: 0.0,
                    target: None,
                    note,
                },
            );
        };
        let trace_b55_merges = std::env::var_os("BASTION_B55_TRACE_MERGES").is_some();
        // Contains items that have been checked for merge, or that were merged into
        // another one
        let mut merged = HashMap::new();
        // Contains merges that will be performed (from, into)
        let mut merges = Vec::new();
        // Delete events are emitted when this is dropped
        let mut delete_emitter = delete_bus.emitter();

        for (entity, item, pos, loot_owner) in
            (&entities, &items, &positions, loot_owners.maybe()).join()
        {
            // Do not process items that are already being merged
            if merged.contains_key(&entity) {
                continue;
            }

            // For items that merge, exponentially back off the frequency of the merge check
            if !item.should_merge || program_time.0 < item.next_merge_check().0 {
                continue;
            }

            // We do not want to allow merging this item if it isn't already being
            // merged into another
            merged.insert(entity, true);

            let partners_before = merges.len();
            for (source_entity, _) in get_nearby_mergeable_items(
                item,
                pos,
                loot_owner,
                bastion_piles.contains(entity),
                (
                    &entities,
                    &items,
                    &positions,
                    &loot_owners,
                    &bastion_piles,
                    &spatial_grid,
                ),
            ) {
                // Prevent merging an item multiple times, we cannot
                // do this in the above filter since we mutate `merged` below
                if merged.contains_key(&source_entity) {
                    continue;
                }

                // Do not merge items multiple times
                merged.insert(source_entity, false);
                // Defer the merge
                merges.push((source_entity, entity));
            }
            if recorder_on {
                record_merge_event(
                    uids.get(entity).map(|u| u.0.get()).unwrap_or(0),
                    format!(
                        "check-fire; pt={}; due={}; partners={}; amount={}",
                        program_time.0,
                        item.next_merge_check().0,
                        merges.len() - partners_before,
                        item.amount(),
                    ),
                );
            }
        }

        for (source, target) in merges {
            let source_persistent = bastion_piles.contains(source);
            let target_persistent = bastion_piles.contains(target);
            let source_pos = positions.get(source).map(|pos| pos.0);
            let target_pos = positions.get(target).map(|pos| pos.0);
            let source_item = items
                .remove(source)
                .expect("We know this entity must have an item.");
            let source_amount = source_item.amount();
            let source_created = source_item.created().0;
            let mut target_item = items
                .get_mut(target)
                .expect("We know this entity must have an item.");
            let target_before = target_item.amount();

            if let Err(item) = target_item.try_merge(source_item) {
                // We re-insert the item, should be unreachable since we already checked whether
                // the items were mergeable in the above loop
                items
                    .insert(source, item)
                    .expect("PickupItem was removed from this entity earlier");
            } else {
                let target_after = target_item.amount();
                if trace_b55_merges && (source_persistent || target_persistent) {
                    tracing::warn!(
                        source = source.id(),
                        target = target.id(),
                        source_amount,
                        target_before,
                        target_after,
                        conserved =
                            target_after as u64 == source_amount as u64 + target_before as u64,
                        source_persistent,
                        target_persistent,
                        source_created,
                        now = program_time.0,
                        ?source_pos,
                        ?target_pos,
                        "B5.5 persistent pile deletion attributed to item merge"
                    );
                }
                // If the merging was successfull, we remove the old item entity from the ECS
                if recorder_on {
                    record_merge_event(
                        uids.get(source).map(|u| u.0.get()).unwrap_or(0),
                        format!(
                            "merged-into; target={}; src_amount={source_amount};                              target_after={target_after}; pt={}",
                            uids.get(target).map(|u| u.0.get()).unwrap_or(0),
                            program_time.0,
                        ),
                    );
                }
                delete_emitter.emit(DeleteEvent(source));
            }
        }

        for updated in merged
            .into_iter()
            .filter_map(|(entity, is_merge_parent)| is_merge_parent.then_some(entity))
        {
            if let Some(mut item) = items.get_mut(updated) {
                item.next_merge_check_mut().0 +=
                    (program_time.0 - item.created().0).max(1.0 / CHECKS_PER_SECOND);
                if recorder_on {
                    record_merge_event(
                        uids.get(updated).map(|u| u.0.get()).unwrap_or(0),
                        format!("backoff; next={}; created={}", item.next_merge_check().0, item.created().0),
                    );
                }
            }
        }
    }
}

pub fn get_nearby_mergeable_items<'a>(
    item: &'a comp::PickupItem,
    pos: &'a comp::Pos,
    loot_owner: Option<&'a comp::LootOwner>,
    // bastion (B5.5): whether the item being merged is a persistent colonist
    // pile. Merges must stay WITHIN a persistence class — a persistent pile
    // merging into a timed vanilla drop would inherit its despawn timer
    // (silent item loss), and the reverse would grant vanilla loot
    // immortality.
    is_persistent_pile: bool,
    (entities, items, positions, loot_owners, bastion_piles, spatial_grid): (
        &'a Entities<'a>,
        // We do not actually need write access here, but currently all callers of this function
        // have a WriteStorage<Item> in scope which we cannot *downcast* into a ReadStorage
        &'a WriteStorage<'a, comp::PickupItem>,
        &'a ReadStorage<'a, comp::Pos>,
        &'a ReadStorage<'a, comp::LootOwner>,
        &'a ReadStorage<'a, comp::bastion::BastionPile>,
        &'a CachedSpatialGrid,
    ),
) -> impl Iterator<Item = (Entity, f32)> + 'a {
    // Get nearby items
    spatial_grid
        .0
        .in_circle_aabr(pos.0.xy(), MAX_ITEM_MERGE_DIST)
        // Filter out any unrelated entities
        .flat_map(move |entity| {
            (entities, items, positions, loot_owners.maybe())
                .lend_join()
                .get(entity, entities)
                .and_then(|(entity, item, other_position, loot_owner)| {
                    let distance_sqrd = other_position.0.distance_squared(pos.0);
                    if distance_sqrd < MAX_ITEM_MERGE_DIST.powi(2) {
                        Some((entity, item, distance_sqrd, loot_owner))
                    } else {
                        None
                    }
                })
        })
        // Filter by "mergeability"
        .filter_map(move |(entity, other_item, distance, other_loot_owner)| {
            (other_loot_owner.map(|owner| owner.owner()) == loot_owner.map(|owner| owner.owner())
                && bastion_piles.contains(entity) == is_persistent_pile
                && item.can_merge(other_item)).then_some((entity, distance))
        })
}
