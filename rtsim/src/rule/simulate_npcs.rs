use crate::{
    RtState, Rule, RuleError,
    data::{
        Sentiment,
        npc::SimulationMode,
        quest::{QuestTerminalOutcome, QuestTerminalPolicy},
    },
    event::{EventCtx, OnHealthChange, OnHelped, OnMountVolume, OnTick},
};
use common::{
    comp::{self, Body, agent::FlightMode},
    command_protocol::IdempotencyKey,
    mounting::{Volume, VolumePos},
    rtsim::{Actor, NpcAction, NpcActivity, NpcId, NpcInput, QuestId},
    terminal_arbitration::{TerminalIntent, TerminalReceipt, commit_terminal_intents},
    terrain::{CoordinateConversions, TerrainChunkSize},
    vol::RectVolSize,
};
use hashbrown::HashMap;
use slotmap::{Key, SecondaryMap};
use vek::{Clamp, Vec2};

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind(on_helped);
        rtstate.bind(on_health_changed);
        rtstate.bind(on_mount_volume);
        rtstate.bind(on_tick);

        Ok(Self)
    }
}

fn on_mount_volume(ctx: EventCtx<SimulateNpcs, OnMountVolume>) {
    let data = &mut *ctx.state.data_mut();

    // TODO: Add actor to riders.
    if let VolumePos {
        kind: Volume::Entity(vehicle),
        ..
    } = ctx.event.pos
        && let Some(link) = data.npcs.mounts.get_steerer_link(vehicle)
        && let Actor::Npc(driver) = link.rider
        && let Some(driver) = data.npcs.get_mut(driver)
    {
        driver.controller.actions.push(NpcAction::Say(
            Some(ctx.event.actor),
            comp::Content::localized("npc-speech-welcome-aboard"),
        ))
    }
}

fn on_health_changed(ctx: EventCtx<SimulateNpcs, OnHealthChange>) {
    let data = &mut *ctx.state.data_mut();

    if let Some(cause) = ctx.event.cause
        && let Actor::Npc(npc) = ctx.event.actor
        && let Some(npc) = data.npcs.get_mut(npc)
    {
        if ctx.event.change < 0.0 {
            npc.sentiments
                .toward_mut(cause)
                .change_by(-0.1, Sentiment::ENEMY);
        } else if ctx.event.change > 0.0 {
            npc.sentiments
                .toward_mut(cause)
                .change_by(0.05, Sentiment::POSITIVE);
        }
    }
}

fn on_helped(ctx: EventCtx<SimulateNpcs, OnHelped>) {
    let data = &mut *ctx.state.data_mut();

    if let Some(saver) = ctx.event.saver
        && let Actor::Npc(npc) = ctx.event.actor
        && let Some(npc) = data.npcs.get_mut(npc)
    {
        npc.controller.actions.push(NpcAction::Say(
            Some(ctx.event.actor),
            comp::Content::localized("npc-speech-thank_you"),
        ));
        npc.sentiments
            .toward_mut(saver)
            .change_by(0.3, Sentiment::FRIEND);
    }
}

fn on_tick(ctx: EventCtx<SimulateNpcs, OnTick>) {
    let data = &mut *ctx.state.data_mut();

    // Maintain links
    let ids = data.npcs.mounts.ids().collect::<Vec<_>>();
    let mut mount_activity = SecondaryMap::new();
    for link_id in ids {
        if let Some(link) = data.npcs.mounts.get(link_id) {
            if let Some(mount) = data
                .npcs
                .npcs
                .get(link.mount)
                .filter(|mount| !mount.is_dead())
            {
                let wpos = mount.wpos;
                if let Actor::Npc(rider) = link.rider {
                    if let Some(rider) = data
                        .npcs
                        .npcs
                        .get_mut(rider)
                        .filter(|rider| !rider.is_dead())
                    {
                        rider.wpos = wpos;
                        mount_activity.insert(link.mount, rider.controller.activity);
                    } else {
                        data.npcs.mounts.dismount(link.rider)
                    }
                }
            } else {
                data.npcs.mounts.remove_mount(link.mount)
            }
        }
    }

    let mut npc_inputs = Vec::new();

    // `T0.86`/`E5-A`: collected across ALL npcs before any of them are
    // arbitrated -- mirrors `quests_to_create`'s own buffer-then-commit
    // shape (drained per-npc below, but a quest's competing intents can
    // come from MULTIPLE different npcs' controllers, so committing needs
    // to see the whole tick's set before picking a winner).
    let mut quest_intents: HashMap<QuestId, Vec<(NpcId, TerminalIntent<QuestTerminalOutcome>)>> =
        HashMap::new();

    for (npc_id, npc) in data.npcs.npcs.iter_mut().filter(|(_, npc)| !npc.is_dead()) {
        npc.controller.actions.retain(|action| match action {
            // NPC-to-NPC messages never leave rtsim
            NpcAction::Msg { to, msg } => {
                if let Actor::Npc(to) = to {
                    let from: Actor = npc_id.into();
                    npc_inputs.push((*to, from, NpcInput::Msg {
                        from,
                        msg: msg.clone(),
                    }));
                } else {
                    // TODO: Send to players?
                }
                false
            },
            // All other cases are handled by the game when loaded
            NpcAction::Say(_, _) | NpcAction::Attack(_) | NpcAction::Dialogue(_, _) => {
                matches!(npc.mode, SimulationMode::Loaded)
            },
        });

        if matches!(npc.mode, SimulationMode::Simulated) {
            let activity = if data.npcs.mounts.get_mount_link(npc_id).is_some() {
                // We are riding, nothing to do.
                continue;
            } else if let Some(activity) = mount_activity.get(npc_id) {
                *activity
            } else {
                npc.controller.activity
            };

            match activity {
                // Move NPCs if they have a target destination
                Some(NpcActivity::Goto(target, speed_factor)) => {
                    let diff = target - npc.wpos;
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        let offset = diff
                            * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                / dist2.sqrt())
                            .min(1.0);
                        let new_wpos = npc.wpos + offset;

                        let is_valid = match npc.body {
                            // Don't move water bound bodies outside of water.
                            Body::Ship(comp::ship::Body::SailBoat | comp::ship::Body::Galleon)
                            | Body::FishMedium(_)
                            | Body::FishSmall(_) => {
                                let chunk_pos = new_wpos.xy().as_().wpos_to_cpos();
                                ctx.world
                                    .sim()
                                    .get(chunk_pos)
                                    .is_none_or(|f| f.river.river_kind.is_some())
                            },
                            Body::Ship(comp::ship::Body::DefaultAirship) => false,
                            _ => true,
                        };

                        if is_valid {
                            npc.wpos = new_wpos;
                        }

                        npc.dir = (target.xy() - npc.wpos.xy())
                            .try_normalized()
                            .unwrap_or(npc.dir);
                    }
                },
                // Move Flying NPCs like airships if they have a target destination
                Some(NpcActivity::GotoFlying(target, speed_factor, height, dir, mode)) => {
                    let diff = target - npc.wpos;
                    let dist2 = diff.magnitude_squared();

                    if dist2 > 0.5f32.powi(2) {
                        match npc.body {
                            Body::Ship(comp::ship::Body::DefaultAirship) => {
                                // RTSim NPCs don't interract with terrain, and their position is
                                // independent of ground level.
                                // While movement is simulated, airships will happily stay at ground
                                // level or fly through mountains.
                                // The code at the end of this block "Make sure NPCs remain in a
                                // valid location" just forces
                                // airships to be at least above ground (on the ground actually).
                                // The reason is that when docking, airships need to descend much
                                // closer to the terrain
                                // than when cruising between sites, so airships cannot be forced to
                                // stay at a fixed height above
                                // terrain (i.e. flying_height()). Instead, when mode is
                                // FlightMode::FlyThrough, set the airship altitude directly to
                                // terrain height + height (if Some)
                                // or terrain height + default height (npc.body.flying_height()).
                                // When mode is FlightMode::Braking, the airship is allowed to
                                // descend below flying height
                                // because it is near or at the dock. In this mode, if height is
                                // Some, set the airship altitude to
                                // the maximum of target.z or terrain height + height. If height is
                                // None, set the airship altitude to
                                // target.z. By forcing the airship altitude to be at a specific
                                // value, when the airship is
                                // suddenly in a loaded chunk it will not be below or at the ground
                                // and will not get stuck.

                                // Move in x,y
                                let diffxy = target.xy() - npc.wpos.xy();
                                let distxy2 = diffxy.magnitude_squared();
                                if distxy2 > 0.5f32.powi(2) {
                                    let offsetxy = diffxy
                                        * (npc.body.max_speed_approx()
                                            * speed_factor
                                            * ctx.event.dt
                                            / distxy2.sqrt());
                                    npc.wpos.x += offsetxy.x;
                                    npc.wpos.y += offsetxy.y;
                                }
                                // The diff is not computed for z like x,y. Rather, the altitude is
                                // set directly so that when the
                                // simulated ship is suddenly in a loaded chunk it will not be below
                                // or at the ground level and risk getting stuck.
                                let base_height =
                                    if mode == FlightMode::FlyThrough || height.is_some() {
                                        ctx.world.sim().get_surface_alt_approx(npc.wpos.xy().as_())
                                    } else {
                                        0.0
                                    };
                                let ship_z = match mode {
                                    FlightMode::FlyThrough => {
                                        base_height + height.unwrap_or(npc.body.flying_height())
                                    },
                                    FlightMode::Braking(_) => {
                                        (base_height + height.unwrap_or(0.0)).max(target.z)
                                    },
                                };
                                npc.wpos.z = ship_z;
                            },
                            _ => {
                                let offset = diff
                                    * (npc.body.max_speed_approx() * speed_factor * ctx.event.dt
                                        / dist2.sqrt())
                                    .min(1.0);
                                let new_wpos = npc.wpos + offset;

                                let is_valid = match npc.body {
                                    // Don't move water bound bodies outside of water.
                                    Body::Ship(
                                        comp::ship::Body::SailBoat | comp::ship::Body::Galleon,
                                    )
                                    | Body::FishMedium(_)
                                    | Body::FishSmall(_) => {
                                        let chunk_pos = new_wpos.xy().as_().wpos_to_cpos();
                                        ctx.world
                                            .sim()
                                            .get(chunk_pos)
                                            .is_none_or(|f| f.river.river_kind.is_some())
                                    },
                                    _ => true,
                                };

                                if is_valid {
                                    npc.wpos = new_wpos;
                                }
                            },
                        }

                        if let Some(dir_override) = dir {
                            npc.dir = dir_override.xy().try_normalized().unwrap_or(npc.dir);
                        } else {
                            npc.dir = (target.xy() - npc.wpos.xy())
                                .try_normalized()
                                .unwrap_or(npc.dir);
                        }
                    }
                },
                Some(
                    NpcActivity::Gather(_)
                    | NpcActivity::HuntAnimals
                    | NpcActivity::Dance(_)
                    | NpcActivity::Cheer(_)
                    | NpcActivity::Sit(..)
                    | NpcActivity::Talk(..),
                ) => {
                    // TODO: Maybe they should walk around randomly
                    // when gathering resources?
                },
                None => {},
            }

            // Make sure NPCs remain in a valid location
            let clamped_wpos = npc.wpos.xy().clamped(
                Vec2::zero(),
                (ctx.world.sim().get_size() * TerrainChunkSize::RECT_SIZE).as_(),
            );
            match npc.body {
                // Don't force air ships to be at flying_height, else they can't land at docks.
                Body::Ship(comp::ship::Body::DefaultAirship | comp::ship::Body::AirBalloon) => {
                    npc.wpos = clamped_wpos.with_z(
                        ctx.world
                            .sim()
                            .get_surface_alt_approx(clamped_wpos.as_())
                            .max(npc.wpos.z),
                    );
                },
                _ => {
                    npc.wpos = clamped_wpos.with_z(
                        ctx.world.sim().get_surface_alt_approx(clamped_wpos.as_())
                            + npc.body.flying_height(),
                    );
                },
            }
        }

        // Move home if required
        if let Some(new_home) = npc.controller.new_home.take() {
            // Remove the NPC from their old home population
            if let Some(old_home) = npc.home
                && let Some(old_home) = data.sites.get_mut(old_home)
            {
                old_home.population.remove(&npc_id);
            }
            // Add the NPC to their new home population
            if let Some(new_home) = new_home
                && let Some(new_home) = data.sites.get_mut(new_home)
            {
                new_home.population.insert(npc_id);
            }
            npc.home = new_home;
        }

        // Create registered quests
        for (id, quest) in core::mem::take(&mut npc.controller.quests_to_create) {
            data.quests.create(id, quest);
        }

        // T0.86/E5-A: collect this npc's quest terminal intents --
        // arbitrated together with every other npc's, below, after this
        // whole per-npc loop finishes (a quest's competing intents can
        // come from different npcs, so they can't be committed one npc at
        // a time the way quests_to_create is).
        for (i, (quest_id, outcome)) in
            core::mem::take(&mut npc.controller.quest_terminal_intents)
                .into_iter()
                .enumerate()
        {
            quest_intents.entry(quest_id).or_default().push((
                npc_id,
                TerminalIntent {
                    observed_version: 0,
                    outcome,
                    reason: "quest terminal intent",
                    effective_tick: 0,
                    causation: IdempotencyKey(0),
                    stable_producer: npc_id.data().as_ffi(),
                    producer_sequence: i as u64,
                },
            ));
        }

        // Set job status
        npc.job = npc.controller.job.clone();
    }

    // T0.86/E5-A: the named commit phase -- arbitrate each quest's
    // competing intents (if any) and commit exactly one via
    // `Quest::resolve`, now genuinely race-free since this whole phase is
    // serial (NPC AI's own parallel tick, where these intents were
    // submitted, has already finished by this point). The winning
    // submitter's own `Controller` gets the receipt (deposit included) to
    // claim on a later tick; everyone else just observes
    // `Quest::resolution()` go `Some` without a personal receipt --
    // see `npc_ai::quest::poll_quest_terminal`'s doc for why that's
    // sufficient (duplicates/losers get no new side effects either way).
    for (quest_id, winner_npc, outcome) in decide_quest_terminal_commits(&quest_intents, |id| {
        data.quests.get(id).is_some_and(|q| q.resolution().is_none())
    }) {
        let Some(quest) = data.quests.get(quest_id) else {
            continue;
        };
        let success = outcome.as_success_bool();
        if let Some(resolved) = quest.resolve(quest.arbiter, success)
            && let Some(winner) = data.npcs.npcs.get_mut(winner_npc)
        {
            winner
                .controller
                .quest_terminal_receipts
                .push((quest_id, success, resolved.deposit));
        }
    }

    // DET-ESIM-015 (v8 rtsim-economy, High): deliver NPC-to-NPC messages to each
    // recipient inbox in canonical (recipient, sender) order. npc_inputs was
    // built by iterating the npc slotmap, so the inbox chronology rode that
    // (arbitrary, though stable) iteration order — the determinism law forbids
    // authoritative outcomes depending on iteration order even when stable.
    // Stable sort keeps multiple messages from one sender in send order.
    let npc_inputs = canonical_npc_input_order(npc_inputs);
    for (npc_id, _from, input) in npc_inputs {
        if let Some(npc) = data.npcs.get_mut(npc_id) {
            npc.inbox.push_back(input);
        }
    }
}

/// `T0.86`/`E5-A`: pure decision half of the quest-terminal commit phase
/// (`on_tick`'s own I/O -- looking up `Quest::resolve`, writing the
/// winner's receipt -- stays there; this only DECIDES who wins). Pure so
/// it's directly unit-testable without a live `Data`/`Npcs` fixture: takes
/// this tick's collected intents plus an `is_unresolved` predicate the
/// caller supplies, returns `(quest_id, winning_npc, winning_outcome)` for
/// every quest that actually committed this tick.
fn decide_quest_terminal_commits(
    quest_intents: &HashMap<QuestId, Vec<(NpcId, TerminalIntent<QuestTerminalOutcome>)>>,
    is_unresolved: impl Fn(QuestId) -> bool,
) -> Vec<(QuestId, NpcId, QuestTerminalOutcome)> {
    let mut commits = Vec::new();
    for (&quest_id, npc_intents) in quest_intents {
        if !is_unresolved(quest_id) {
            // Already resolved (e.g. by a prior tick's commit); nothing to
            // arbitrate.
            continue;
        }
        let intents = npc_intents
            .iter()
            .map(|(_, intent)| intent.clone())
            .collect::<Vec<_>>();
        let receipts = commit_terminal_intents(0, &intents, &QuestTerminalPolicy);
        let Some(winner_idx) = receipts
            .iter()
            .position(|r| matches!(r, TerminalReceipt::Committed))
        else {
            // Every intent was stale (shouldn't happen -- quests aren't
            // versioned yet, so observed_version always matches) or the
            // top rank was a genuine conflict; nothing commits this tick.
            continue;
        };
        let (winner_npc, winner_intent) = &npc_intents[winner_idx];
        commits.push((quest_id, *winner_npc, winner_intent.outcome));
    }
    // Cross-review (Opus F2, CONFIRMED): quest_intents is a HashMap, so the
    // iteration above runs in run-varying order. An NPC winning TWO quests
    // in the same tick would get its receipts pushed in that varying order
    // -- the same DET-ESIM-015 law npc_inputs' canonical_npc_input_order
    // already enforces a few lines below, missed here. Sort by the stable
    // QuestId key before returning.
    commits.sort_by_key(|(quest_id, _, _)| quest_id.0);
    commits
}

#[cfg(test)]
mod decide_quest_terminal_commits_tests {
    use super::*;
    use slotmap::KeyData;

    fn npc(raw: u64) -> NpcId { NpcId::from(KeyData::from_ffi(raw)) }

    fn intent(outcome: QuestTerminalOutcome, seq: u64) -> TerminalIntent<QuestTerminalOutcome> {
        TerminalIntent {
            observed_version: 0,
            outcome,
            reason: "test",
            effective_tick: 0,
            causation: IdempotencyKey(seq),
            stable_producer: seq,
            producer_sequence: seq,
        }
    }

    /// The required non-vacuity case (Fable-ruled): two DIFFERENT npcs
    /// submit competing intents for the SAME quest in the SAME tick --
    /// the domain policy picks the winner, not which npc happened to be
    /// collected first. Also proves the LOSER gets no commit entry at all
    /// (its own poll must fall back to `resolution().is_some()`, per
    /// `poll_quest_terminal`'s doc).
    #[test]
    fn competing_intents_from_two_different_npcs_resolve_by_policy_not_arrival_order() {
        use QuestTerminalOutcome::*;
        let quest_id = QuestId(1);
        let mut quest_intents = HashMap::new();
        quest_intents.insert(quest_id, vec![
            (npc(1), intent(TimedOut, 0)),
            (npc(2), intent(CompletedPreDeadline, 1)),
        ]);

        let commits = decide_quest_terminal_commits(&quest_intents, |_| true);
        assert_eq!(commits, vec![(quest_id, npc(2), CompletedPreDeadline)]);

        // Order-independence: swapping which npc submitted first doesn't
        // change the winner.
        let mut reversed = HashMap::new();
        reversed.insert(quest_id, vec![
            (npc(2), intent(CompletedPreDeadline, 0)),
            (npc(1), intent(TimedOut, 1)),
        ]);
        let commits_reversed = decide_quest_terminal_commits(&reversed, |_| true);
        assert_eq!(commits_reversed, vec![(quest_id, npc(2), CompletedPreDeadline)]);
    }

    /// Cross-review (Opus F2, CONFIRMED): quest_intents is a HashMap, so
    /// commits used to come out in run-varying order -- an NPC winning
    /// several quests in one tick would have its receipts pushed in that
    /// varying order. Multiple quests (HashMap iteration order is not
    /// insertion order in Rust, so this doesn't merely restate insertion
    /// order) must always come out sorted by QuestId, regardless of what
    /// order the map happens to iterate in.
    #[test]
    fn multiple_quest_commits_are_returned_in_stable_quest_id_order() {
        use QuestTerminalOutcome::*;
        let mut quest_intents = HashMap::new();
        // Deliberately inserted out of QuestId order.
        for id in [5u64, 1, 3, 2, 4] {
            quest_intents.insert(QuestId(id), vec![(npc(id), intent(CompletedPreDeadline, id))]);
        }

        let commits = decide_quest_terminal_commits(&quest_intents, |_| true);
        let ids: Vec<u64> = commits.iter().map(|(q, _, _)| q.0).collect();
        assert_eq!(
            ids,
            vec![1, 2, 3, 4, 5],
            "commits must be sorted by QuestId regardless of HashMap iteration order"
        );
    }

    #[test]
    fn no_intents_for_a_quest_is_no_commit() {
        let commits = decide_quest_terminal_commits(&HashMap::new(), |_| true);
        assert!(commits.is_empty());
    }

    /// Already-resolved quests (per the caller's `is_unresolved`
    /// predicate) are skipped entirely, even with pending intents --
    /// prevents re-arbitrating/re-committing a quest a prior tick already
    /// settled.
    #[test]
    fn already_resolved_quest_is_skipped_even_with_pending_intents() {
        let quest_id = QuestId(1);
        let mut quest_intents = HashMap::new();
        quest_intents.insert(quest_id, vec![(npc(1), intent(QuestTerminalOutcome::TimedOut, 0))]);

        let commits = decide_quest_terminal_commits(&quest_intents, |_| false);
        assert!(commits.is_empty());
    }

    /// Two quests in the same tick are arbitrated independently.
    #[test]
    fn multiple_quests_in_one_tick_are_arbitrated_independently() {
        use QuestTerminalOutcome::*;
        let q1 = QuestId(1);
        let q2 = QuestId(2);
        let mut quest_intents = HashMap::new();
        quest_intents.insert(q1, vec![(npc(1), intent(TimedOut, 0))]);
        quest_intents.insert(q2, vec![(npc(2), intent(CompletedPreDeadline, 0))]);

        let mut commits = decide_quest_terminal_commits(&quest_intents, |_| true);
        commits.sort_by_key(|(id, ..)| id.0);
        assert_eq!(commits, vec![
            (q1, npc(1), TimedOut),
            (q2, npc(2), CompletedPreDeadline),
        ]);
    }
}

/// DET-ESIM-015: deliver NPC-to-NPC messages to each recipient inbox in a
/// canonical (recipient, sender) order. The inputs are built by iterating the
/// npc slotmap, so without this the inbox chronology rode that stable-but-
/// arbitrary iteration order (the determinism law forbids authoritative outcomes
/// depending on iteration order even when stable). STABLE sort so multiple
/// messages from one sender keep their send order. Generic over the payload.
pub fn canonical_npc_input_order<I>(
    mut npc_inputs: Vec<(common::rtsim::NpcId, common::rtsim::Actor, I)>,
) -> Vec<(common::rtsim::NpcId, common::rtsim::Actor, I)> {
    npc_inputs.sort_by_key(|(to, from, _)| (*to, *from));
    npc_inputs
}

#[cfg(test)]
mod det_esim_015_tests {
    use super::*;
    use common::rtsim::{Actor, NpcId};

    #[test]
    fn canonical_npc_input_order_is_iteration_order_independent_and_stable() {
        // Ordered NpcIds from a fresh slotmap: a < b < c.
        let mut sm: slotmap::DenseSlotMap<NpcId, ()> = slotmap::DenseSlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        // Senders are Actors (npc_id.into()); Actor::Npc(a) < Actor::Npc(c).
        let (na, nb, nc) = (Actor::Npc(a), Actor::Npc(b), Actor::Npc(c));

        // (to, from, tag). Two messages to b from na (tags 1,2) must keep send
        // order (stable). The same set in two different iteration orders.
        let set1 = vec![
            (c, na, 10u32),
            (b, na, 1u32),
            (b, nc, 3u32),
            (b, na, 2u32),
            (a, nb, 5u32),
        ];
        let set2 = vec![
            (a, nb, 5u32),
            (b, na, 1u32),
            (b, na, 2u32),
            (b, nc, 3u32),
            (c, na, 10u32),
        ];
        let r1 = canonical_npc_input_order(set1);
        let r2 = canonical_npc_input_order(set2);

        // Canonical (recipient, sender) order, stable within (b,na): 1 then 2.
        let expected = vec![
            (a, nb, 5u32),
            (b, na, 1u32),
            (b, na, 2u32),
            (b, nc, 3u32),
            (c, na, 10u32),
        ];
        assert_eq!(
            r1, expected,
            "npc inputs not in canonical (recipient, sender) order or unstable (DET-ESIM-015)"
        );
        // Iteration-order-independent.
        assert_eq!(
            r1, r2,
            "npc input delivery order depends on iteration order — DET-ESIM-015 regressed"
        );
    }
}
