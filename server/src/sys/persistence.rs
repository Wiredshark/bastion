use crate::{persistence::character_updater, sys::SysScheduler};
use crate::Tick;
use common::{
    comp::{
        ActiveAbilities, Alignment, Body, Inventory, MapMarker, Presence, PresenceKind, SkillSet,
        Stats, Waypoint,
        pet::{Pet, is_tameable},
    },
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Join, LendJoin, Read, ReadExpect, ReadStorage, Write, WriteExpect};
use tracing::error;

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadStorage<'a, Alignment>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, SkillSet>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Waypoint>,
        ReadStorage<'a, MapMarker>,
        ReadStorage<'a, Pet>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, ActiveAbilities>,
        WriteExpect<'a, character_updater::CharacterUpdater>,
        Write<'a, SysScheduler<Self>>,
        Read<'a, Tick>,
        ReadExpect<'a, common_state::ExecutionMode>,
    );

    const NAME: &'static str = "persistence";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            alignments,
            bodies,
            presences,
            player_skill_set,
            player_inventories,
            uids,
            player_waypoints,
            map_markers,
            pets,
            stats,
            active_abilities,
            mut updater,
            mut scheduler,
            tick,
            execution_mode,
        ): Self::SystemData,
    ) {
        // T0.1: tick-cadenced in deterministic mode (wall cadence would
        // fire on machine-dependent ticks), wall-cadenced live.
        // DET-CLK-010/011 (v5 deep-pass): tick cadence is now authoritative
        // in LIVE too — persistence is authoritative work, so which sim tick
        // it fires on must be a pure function of the tick count, not host
        // speed. Composes with DET-CLK-006's fixed step (sim time = ticks).
        // Intended consequence: under server overload (time dilation) the
        // wall interval between snapshots stretches with sim time — the
        // deterministic-correct semantics (snapshot cadence tracks sim
        // evolution, not wall). The wall path in SysScheduler remains only
        // for future diagnostic/keepalive consumers.
        let _ = execution_mode; // cadence no longer branches on mode
        if scheduler.should_run_at(tick.0, true) {
            updater.batch_update(
                (
                    &presences,
                    &player_skill_set,
                    &player_inventories,
                    &uids,
                    player_waypoints.maybe(),
                    &active_abilities,
                    map_markers.maybe(),
                )
                    .join()
                    .filter_map(
                        |(
                            presence,
                            skill_set,
                            inventory,
                            player_uid,
                            waypoint,
                            active_abilities,
                            map_marker,
                        )| match presence.kind {
                            PresenceKind::LoadingCharacter(_char_id) => {
                                error!(
                                    "Unexpected state when persisting characters! Some of the \
                                     components required above should only be present after a \
                                     character is loaded!"
                                );
                                None
                            },
                            PresenceKind::Character(id) => {
                                // DET-PER-009 (v5 deep-pass, Critical): collect
                                // the owner's pets in canonical Uid order. The
                                // ECS join yields them in storage order, and
                                // that order is what update_pets zips against a
                                // freshly-allocated id range (PER-024) — so pet
                                // persistent identity rode ECS iteration order.
                                // Key on the pet's own Uid (stable), sort, drop
                                // the key.
                                let mut pets = (&alignments, &bodies, &stats, &pets, &uids)
                                    .join()
                                    .filter_map(|(alignment, body, stats, pet, pet_uid)| {
                                        match alignment {
                                            // Don't try to persist non-tameable pets (likely spawned
                                            // using /spawn) since there isn't any code to handle
                                            // persisting them
                                            Alignment::Owned(pet_owner)
                                                if pet_owner == player_uid && is_tameable(body) =>
                                            {
                                                Some((*pet_uid, ((*pet).clone(), *body, stats.clone())))
                                            },
                                            _ => None,
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                pets.sort_unstable_by_key(|(pet_uid, _)| pet_uid.0);
                                let pets =
                                    pets.into_iter().map(|(_, pet)| pet).collect::<Vec<_>>();

                                Some((
                                    id,
                                    skill_set.clone(),
                                    inventory.clone(),
                                    pets,
                                    waypoint.cloned(),
                                    active_abilities.clone(),
                                    map_marker.cloned(),
                                ))
                            },
                            // bastion (ROW-COLONY-PRESENCE): unreachable in
                            // practice (this join also requires
                            // `player_skill_set`/`player_inventories`, which
                            // a colony presence never has), kept exhaustive.
                            PresenceKind::Spectator
                            | PresenceKind::Possessor
                            | PresenceKind::Colony => None,
                        },
                    ),
            );
        }
    }
}
