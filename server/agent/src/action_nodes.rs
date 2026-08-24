use crate::{
    consts::{
        AVG_FOLLOW_DIST, DEFAULT_ATTACK_RANGE, IDLE_HEALING_ITEM_THRESHOLD, MAX_PATROL_DIST,
        SEPARATION_BIAS, SEPARATION_DIST, STD_AWARENESS_DECAY_RATE,
    },
    data::{AgentData, AgentEmitters, AttackData, Path, ReadData, Tactic, TargetData},
    util::{
        are_our_owners_hostile, entities_have_line_of_sight, get_attacker, get_entity_by_id,
        is_dead_or_invulnerable, is_dressed_as_cultist, is_dressed_as_pirate, is_dressed_as_witch,
        is_invulnerable, is_steering, is_village_guard, is_villager,
    },
};
use common::{
    combat::perception_dist_multiplier_from_stealth,
    comp::{
        self, Agent, Alignment, Body, CharacterState, Content, ControlAction, ControlEvent,
        Controller, HealthChange, InputKind, InventoryAction, Pos, PresenceKind, Scale,
        UnresolvedChatMsg, UtteranceKind,
        ability::BASE_ABILITY_LIMIT,
        agent::{FlightMode, PidControllers, Sound, SoundKind, Target},
        biped_large, body,
        inventory::slot::EquipSlot,
        item::{
            ConsumableKind, Effects, Item, ItemDesc, ItemKind,
            tool::{AbilitySpec, ToolKind},
        },
        projectile::{ProjectileConstructorKind, aim_projectile},
    },
    consts::MAX_MOUNT_RANGE,
    effect::{BuffEffect, Effect},
    event::{ChatEvent, EmitExt, SoundEvent},
    interaction::InteractionKind,
    match_some,
    mounting::VolumePos,
    path::TraversalConfig,
    rtsim::NpcActivity,
    states::basic_beam,
    terrain::Block,
    threat_policy::{ThreatCandidateV1, ThreatClassV1, arbitrate},
    time::DayPeriod,
    uid::Uid,
    util::Dir,
    vol::ReadVol,
};
use itertools::Itertools;
use rand::{Rng, RngExt, rng};
use specs::Entity as EcsEntity;
use vek::*;

#[cfg(feature = "use-dyn-lib")]
use {crate::LIB, std::ffi::CStr};

fn bastion_goto_writer_diag(uid: u64) -> bool {
    std::env::var("BASTION_GOTO_WRITER_DIAG_UID")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        == Some(uid)
}

fn effect_healing_value(effect: &Effect) -> (f32, f32) {
    let mut value = 0.0;
    let mut heal_reduction = 0.0;
    match effect {
        Effect::Health(HealthChange { amount, .. }) => value += *amount,
        Effect::Buff(BuffEffect { kind, data, .. }) => {
            if let Some(duration) = data.duration {
                // Evaluation only (aggregating projected healing value, not
                // applying a buff), so the instance id this derives is
                // discarded -- any Time value is fine.
                for effect in kind.effects(data, None, common::resources::Time::default()) {
                    match effect {
                        comp::BuffEffect::HealthChangeOverTime { rate, kind, .. } => {
                            value += match kind {
                                comp::ModifierKind::Additive => rate * duration.0 as f32,
                                comp::ModifierKind::Multiplicative => {
                                    (1.0 + rate).powf(duration.0 as f32)
                                },
                            };
                        },
                        comp::BuffEffect::ItemEffectReduction(amount) => {
                            heal_reduction = heal_reduction + amount - heal_reduction * amount;
                        },
                        _ => {},
                    }
                }
                value += data.strength * data.duration.map_or(0.0, |d| d.0 as f32);
            }
        },
        _ => {},
    }
    (value, heal_reduction)
}

fn healing_value(item: &Item, relaxed: bool, heal_multiplier: f32) -> i32 {
    let mut value = 0.0;
    let mut heal_multiplier_value = 1.0;
    if let ItemKind::Consumable { kind, effects, .. } = &*item.kind()
        && (matches!(kind, ConsumableKind::Drink)
            || (relaxed && matches!(kind, ConsumableKind::Food)))
    {
        match effects {
            Effects::Any(effects) => {
                for effect in effects.iter() {
                    let (add, red) = effect_healing_value(effect);
                    value += add / effects.len() as f32;
                    heal_multiplier_value *= 1.0 - red / effects.len() as f32;
                }
            },
            Effects::All(_) | Effects::One(_) => {
                for effect in effects.effects() {
                    let (add, red) = effect_healing_value(effect);
                    value += add;
                    heal_multiplier_value *= 1.0 - red;
                }
            },
        }
    }
    if heal_multiplier_value < 1.0 && (heal_multiplier < 1.0 || relaxed) {
        value *= 0.1;
    }
    value as i32
}

/// Return the exact inventory slot selected by the production healing rule.
/// Exposed so deterministic fixtures can assert the behavioral choice rather
/// than inferring it from a later trajectory split.
pub fn select_healing_item(
    inventory: &comp::Inventory,
    relaxed: bool,
    heal_multiplier: f32,
) -> Option<comp::inventory::slot::InvSlotId> {
    inventory
        .slots_with_id()
        .filter_map(|(id, slot)| match slot {
            Some(item) if healing_value(item, relaxed, heal_multiplier) > 0 => Some((id, item)),
            _ => None,
        })
        .max_by_key(|(_, item)| {
            if relaxed {
                -healing_value(item, relaxed, heal_multiplier)
            } else {
                healing_value(item, relaxed, heal_multiplier)
            }
        })
        .map(|(id, _)| id)
}

/// bastion ENGINE-OPT-3 (ledger #160): the loot-pickup ATTEMPT decision,
/// extracted pure so its truth table is unit-pinned (the release-decision
/// discipline). `authorized` = `LootOwner::can_pickup` (the SAME authority
/// the commit-side gate in `inventory_manip` revalidates — choose-time here
/// is advisory; the commit is the security boundary). `is_soft` = the
/// owner's no-pickup WISH, respected unless we are hostile to them.
///
/// Non-humanoids need no special arm: `can_pickup` already encodes
/// "non-humanoids ignore ownership" by design, and `wants_pickup` already
/// restricts them to consumables.
///
/// HISTORY (why this exists): the old inline predicate carried TWO
/// inversions — an outer `!` around the entire authorization conjunction
/// (humanoids attempted pickup precisely when unauthorized, refused their
/// own drops; the commit gate then rejected the unauthorized attempts, so
/// the observable damage was refused-entitled-loot + attempt-spam, not
/// theft) and a hostility polarity in the soft-wish term that contradicted
/// its own comment. The paired test module pins both the intended table
/// and the old predicate's flipped rows.
pub fn loot_attempt_decision(
    wants_pickup: bool,
    authorized: bool,
    is_soft: bool,
    hostile_to_owner: bool,
) -> bool {
    wants_pickup && authorized && (!is_soft || hostile_to_owner)
}

/// T0.7 (master build order; ledger #166): tick-rate-invariant probability
/// gate. `rate_per_second` is the hazard — the chance of the event per
/// simulated second — compounded down to THIS tick's dt (Poisson/Gillespie
/// prior art; mirrors rtsim's `discrete_chance`). The named rates below are
/// the EXACT inverses of the old raw per-tick constants at 30 tps
/// (`rate = 1 − (1 − p_tick)^SIM_TPS`), so today's per-tick probability is
/// reproduced while cadence changes no longer distort AI behavior rates.
/// Per-DECISION draws (one-shot choices like `can_sense_directly_near`'s
/// jitter or the jump-vs-roll pick) are deliberately NOT hazards and keep
/// raw draws. `unstuck_if`'s OUTER gate ("attempt an unstuck action this
/// tick") WAS this per-tick debt (E7 Stage 3, T0.79: fixed, routed through
/// `hazard_chance` below via `UNSTUCK_ATTEMPT_RATE`); its INNER jump-vs-
/// roll pick is a one-shot decision GIVEN the outer gate already fired,
/// not a recurring hazard, and correctly stays a flat draw -- scaling a
/// discrete either/or choice by dt would be wrong, not merely unconverted.
/// E7 Stage 3 (T0.79): the chance computation, pulled out of `hazard` so
/// `unstuck_if` (whose gate historically drew `self.helper_random_bool`
/// via the Option-gated deterministic/live rng resolution, not a raw
/// `rng: &mut impl Rng` param) can reuse the SAME formula without also
/// having to restructure its rng source.
fn hazard_chance(dt: f32, rate_per_second: f64) -> f64 {
    let survival = (1.0 - rate_per_second.clamp(0.0, 1.0)).max(0.0);
    (1.0 - survival.powf(f64::from(dt))).clamp(0.0, 1.0)
}

fn hazard(rng: &mut impl Rng, dt: f32, rate_per_second: f64) -> bool {
    rng.random_bool(hazard_chance(dt, rate_per_second))
}

/// 0.05 per tick @30tps: attempt an unstuck action (jump/roll) this tick.
const UNSTUCK_ATTEMPT_RATE: f64 = 0.785_361_236_057_062_8;
/// 0.1 per tick @30tps: put away a wielded weapon while idling.
const UNWIELD_IDLE_RATE: f64 = 0.957_608_841_724_783_8;
/// 0.1 per tick @30tps: re-pick a hunting target.
const HUNT_RETARGET_RATE: f64 = 0.957_608_841_724_783_8;
/// 0.001 per tick @30tps: toggle the equipped lantern with day/night.
const LANTERN_TOGGLE_RATE: f64 = 0.029_569_032_736_914_247;
/// 0.0001 per tick @30tps: a riding pet hops off its owner's shoulders.
const PET_DISMOUNT_RATE: f64 = 0.002_995_654_057_260_544;
/// 0.01 per tick @30tps: a nearby pet jumps onto its owner.
const PET_MOUNT_RATE: f64 = 0.260_299_626_611_719_8;
/// 0.0015 per tick @30tps: idle calm utterance.
const IDLE_UTTERANCE_RATE: f64 = 0.044_034_814_837_612_07;
/// 0.0035 per tick @30tps: idle sit-down.
const IDLE_SIT_RATE: f64 = 0.099_841_283_805_460_65;

/// E7 Stage 3 (T0.79): `unstuck_if`'s outer gate, converted per Fable's
/// ruling -- same exact-inversion-preserves-today's-behavior discipline
/// as the sentiment decay fix (T0.79 Stage 2). `unstuck_if` runs every
/// tick directly (no analogous NPC_SENTIMENT_TICK_SKIP), so the
/// calibration reference is dt = 1 TICK (1/30s @ SIM_TPS), not 1 second --
/// `hazard_chance` at that dt must reproduce the original raw 0.05
/// per-tick probability exactly.
/// bastion (2026-08-22): whether an unstuck attempt may use the JUMP.
///
/// Pure so the rule can be pinned without an ECS, and because the rule is the
/// whole content of the fix: `handle_wallrun` requires only `on_wall &&
/// !on_ground` — no intent — so the only way to stop a colonist wallrunning is
/// to stop it being airborne beside a wall, and the jump is the only way a
/// colonist puts itself in the air.
///
/// `in_climb` is deliberately still honoured for NON-colonists: the vanilla
/// behaviour made the jump unconditional while climbing (that is how a player
/// or wild NPC gets off a wall), and this fix must not change what anything
/// other than a colonist does.
pub(crate) fn unstuck_should_jump(discourage_climb: bool, in_climb: bool, coin: bool) -> bool {
    !discourage_climb && (in_climb || coin)
}

/// bastion (2026-08-22): the wall-detach steer, pinned in BOTH directions.
///
/// Measured cause: 23 of 26 ultimate-fail-safe strandings had `on_ground=false`
/// with `character_state` Wallrun x15 / Climb x5 — colonists pinned to the
/// vertical faces of buildings, held there by a steer that points through the
/// wall at their target.
/// bastion (2026-08-22): a colonist must never jump, and everything else must
/// still be able to.
#[cfg(test)]
mod unstuck_jump_tests {
    use super::unstuck_should_jump;

    /// ★ THE CLOSURE MATTERS, NOT THE COMMON CASE. The previous guard withheld
    /// the jump only when `on_wall` was already true, which is one tick too
    /// late: the colonist jumps from clear ground, drifts into the wall while
    /// airborne, and acquires the contact afterwards. So the rule has to hold
    /// for EVERY combination of the other inputs, including the one the old
    /// code special-cased into an UNCONDITIONAL jump (`in_climb`).
    #[test]
    fn a_colonist_never_jumps_under_any_combination() {
        for in_climb in [false, true] {
            for coin in [false, true] {
                assert!(
                    !unstuck_should_jump(true, in_climb, coin),
                    "a colonist jumped (in_climb={in_climb}, coin={coin}) — \
                     `handle_wallrun` needs only `on_wall && !on_ground` and no \
                     intent, so ANY airborne moment beside a wall strands them"
                );
            }
        }
    }

    /// The other direction, and the reason a one-sided test would be worthless:
    /// players and wild NPCs must keep the vanilla behaviour exactly. A guard
    /// that refuses everything also stops the bug reproducing.
    #[test]
    fn everything_that_is_not_a_colonist_still_jumps_exactly_as_before() {
        assert!(
            unstuck_should_jump(false, true, false),
            "a climbing non-colonist must still jump unconditionally — that is \
             how a player gets off a wall, and this fix must not touch it"
        );
        assert!(
            unstuck_should_jump(false, false, true),
            "a non-colonist must still jump on the coin-flip arm"
        );
        assert!(
            !unstuck_should_jump(false, false, false),
            "and must still ROLL when the coin says roll — the arm was not \
             removed, only the colonist path diverted into it"
        );
    }
}

#[cfg(test)]
mod wall_detach_tests {
    use vek::Vec3;

    // The function under test is an associated fn on the agent data struct;
    // reach it through the same path production code uses.
    type A<'a> = super::AgentData<'a>;

    /// ★ THE SIGN IS THE WHOLE TEST. `PhysicsState::on_wall` accumulates the
    /// probe direction `dirs[dir]` where `pos + dir * 0.01` collided, so it
    /// points **TOWARD** the wall. If this function ever returns the
    /// unnegated vector it will steer colonists HARDER INTO the surface they
    /// are stuck on — and the logs would show unchanged wallrun counts, which
    /// reads as "the fix did nothing" rather than as a reversed sign. That is
    /// the failure this assertion exists to make impossible.
    #[test]
    fn detach_steers_away_from_the_wall_not_into_it() {
        // Wall to the +x side: `on_wall` points +x.
        let toward_wall = Some(Vec3::new(1.0, 0.0, 0.0));
        let away = A::wall_detach_dir(toward_wall, true).expect("pinned: must produce a steer");
        assert!(
            away.x < 0.0,
            "detach steered x={} — TOWARD the wall. `on_wall` points at the \
             wall, so the escape direction is its NEGATION.",
            away.x
        );
        assert!(
            (away.magnitude() - 1.0).abs() < 1e-5,
            "detach bearing must be normalized; got magnitude {}",
            away.magnitude()
        );
        assert_eq!(away.z, 0.0, "the push-off is horizontal; gravity does the rest");
    }

    /// The other direction, which a one-sided test would miss: a colonist
    /// merely WALKING ALONGSIDE a building touches a wall and must keep its
    /// own bearing. An override that fired here would stop colonists walking
    /// past houses at all — a fix that refuses everything also stops the bug
    /// reproducing.
    #[test]
    fn a_colonist_not_pinned_keeps_its_own_bearing() {
        assert_eq!(
            A::wall_detach_dir(Some(Vec3::new(1.0, 0.0, 0.0)), false),
            None,
            "not pinned (on the ground, not climbing) must NOT override the steer"
        );
        assert_eq!(
            A::wall_detach_dir(None, true),
            None,
            "pinned but touching no wall has no escape direction to give"
        );
    }

    /// A degenerate `on_wall` — opposed probe directions cancelling, or a
    /// purely vertical contact — has no horizontal escape. Normalizing a zero
    /// vector yields NaN, and a NaN steer is far worse than no steer.
    #[test]
    fn a_degenerate_contact_yields_no_steer_rather_than_nan() {
        assert_eq!(
            A::wall_detach_dir(Some(Vec3::new(0.0, 0.0, 1.0)), true),
            None,
            "a contact with no horizontal component must return None, never a \
             normalized zero (NaN)"
        );
    }
}

#[cfg(test)]
mod unstuck_if_hazard_conversion_tests {
    use super::{UNSTUCK_ATTEMPT_RATE, hazard_chance};

    const SIM_TPS: f32 = 30.0;

    /// T0.32-style exact-to-1-ulp equivalence pin: at dt = one tick (the
    /// only cadence unstuck_if has ever actually run at), the converted
    /// hazard must reproduce the original raw per-tick constant (0.05)
    /// exactly -- proof, not assertion, that the conversion changes zero
    /// observable behavior today.
    #[test]
    fn hazard_chance_matches_pre_fix_probability_at_one_tick() {
        let dt = 1.0 / SIM_TPS;
        let chance = hazard_chance(dt, UNSTUCK_ATTEMPT_RATE);
        // Tolerance is f32-dt/f64-powf precision scale (UNWIELD_IDLE_RATE
        // and friends use the identical inversion at the identical 30tps
        // reference and are only pinned to their literal's own precision
        // too), not a loose approximation.
        assert!(
            (chance - 0.05).abs() <= 1e-7,
            "hazard_chance at dt=1 tick must reproduce the original raw 0.05 per-tick \
             probability: got {chance}"
        );
    }

    /// The property the raw per-tick constant never had: checking twice
    /// as often (half the dt) should NOT roughly double the per-tick
    /// probability -- a correct hazard's checks-per-second * chance-per-
    /// check stays close to constant across a cadence sweep near 1 tick.
    /// Swept narrowly (half-tick to double-tick, not the wider sweep
    /// sentiment.rs used): UNSTUCK_ATTEMPT_RATE (0.785/s) is a much larger
    /// rate than sentiment's, so discrete_chance's compounding curve is
    /// measurably non-linear even a few ticks out -- that is the actual
    /// math of a large-rate hazard, not a bug, and out of scope for this
    /// pin the same way sentiment's saturation regime was.
    #[test]
    fn hazard_chance_is_cadence_invariant_near_one_tick() {
        let decays_per_second = |dt: f64| -> f64 {
            (1.0 / dt) * hazard_chance(dt as f32, UNSTUCK_ATTEMPT_RATE)
        };
        let baseline = decays_per_second(1.0 / SIM_TPS as f64);
        for dt in [1.0 / 32.0, 1.0 / 30.0, 1.0 / 28.0] {
            let observed = decays_per_second(dt);
            let ratio = observed / baseline;
            assert!(
                (ratio - 1.0).abs() < 0.01,
                "expected decays-per-real-second to stay near the 1-tick baseline \
                 ({baseline}) across a narrow cadence sweep, but dt={dt} gave {observed} \
                 (ratio \
                 {ratio})"
            );
        }
    }

    /// Sanity: the gate is a genuine per-tick probability, not saturated
    /// to 0 or 1 at the cadence it actually runs at.
    #[test]
    fn hazard_chance_is_a_genuine_probability_at_one_tick() {
        let chance = hazard_chance(1.0 / SIM_TPS, UNSTUCK_ATTEMPT_RATE);
        assert!(chance > 0.0 && chance < 1.0, "chance={chance} is not a genuine probability");
    }
}

impl AgentData<'_> {
    ////////////////////////////////////////
    // Action Nodes
    ////////////////////////////////////////
    pub fn glider_equip(&self, controller: &mut Controller, read_data: &ReadData) {
        self.dismount(controller, read_data);
        controller.push_action(ControlAction::GlideWield);
    }

    // TODO: add the ability to follow the target?
    pub fn glider_flight(&self, controller: &mut Controller, _read_data: &ReadData) {
        let Some(fluid) = self.physics_state.in_fluid else {
            return;
        };

        let vel = self.vel;

        let comp::Vel(rel_flow) = fluid.relative_flow(vel);

        let is_wind_downwards = rel_flow.z.is_sign_negative();

        let look_dir = if is_wind_downwards {
            Vec3::from(-rel_flow.xy())
        } else {
            -rel_flow
        };

        controller.inputs.look_dir = Dir::from_unnormalized(look_dir).unwrap_or_else(Dir::forward);
    }

    pub fn fly_upward(&self, controller: &mut Controller, read_data: &ReadData) {
        self.dismount(controller, read_data);

        controller.push_basic_input(InputKind::Fly);
        controller.inputs.move_z = 1.0;
    }

    /// Directs the entity to path and move toward the target
    /// If path is not Full, the entity will path to a location 50 units along
    /// the vector between the entity and the target. The speed multiplier
    /// multiplies the movement speed by a value less than 1.0.
    /// A `None` value implies a multiplier of 1.0.
    /// Returns `false` if the pathfinding algorithm fails to return a path
    pub fn path_toward_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_pos: Vec3<f32>,
        read_data: &ReadData,
        path: Path,
        speed_multiplier: Option<f32>,
    ) -> Option<Vec3<f32>> {
        self.dismount_uncontrollable(controller, read_data);

        let pos_difference = tgt_pos - self.pos.0;
        let pathing_pos = match path {
            Path::Separate => {
                let mut sep_vec: Vec3<f32> = Vec3::zero();

                for entity in read_data
                    .cached_spatial_grid
                    .0
                    .in_circle_aabr(self.pos.0.xy(), SEPARATION_DIST)
                {
                    if let (Some(alignment), Some(other_alignment)) =
                        (self.alignment, read_data.alignments.get(entity))
                        && Alignment::passive_towards(*alignment, *other_alignment)
                        && let (Some(pos), Some(body), Some(other_body)) = (
                            read_data.positions.get(entity),
                            self.body,
                            read_data.bodies.get(entity),
                        )
                    {
                        let dist_xy = self.pos.0.xy().distance(pos.0.xy());
                        let spacing = body.spacing_radius() + other_body.spacing_radius();
                        if dist_xy < spacing {
                            let pos_diff = self.pos.0.xy() - pos.0.xy();
                            sep_vec += pos_diff.try_normalized().unwrap_or_else(Vec2::zero)
                                * ((spacing - dist_xy) / spacing);
                        }
                    }
                }

                tgt_pos + sep_vec * SEPARATION_BIAS + pos_difference * (1.0 - SEPARATION_BIAS)
            },
            Path::AtTarget => tgt_pos,
        };
        let speed_multiplier = speed_multiplier.unwrap_or(1.0).min(1.0);

        let in_loaded_chunk = |pos: Vec3<f32>| {
            read_data
                .terrain
                .contains_key(read_data.terrain.pos_key(pos.map(|e| e.floor() as i32)))
        };

        // If current position lies inside a loaded chunk, we need to plan routes using
        // voxel info. If target happens to be in an unloaded chunk,
        // we need to make our way to the current chunk border, and
        // then reroute if needed.
        let is_target_loaded = in_loaded_chunk(pathing_pos);

        let writer_diag = bastion_goto_writer_diag(self.uid.0.get());
        let route_before = writer_diag.then(|| {
            let route = agent.chaser.get_route();
            (
                agent.chaser.last_target(),
                agent.chaser.route_target(),
                agent.chaser.route_is_complete(),
                route.map(|route| route.next_idx()),
                route.and_then(|route| route.get_path().end().copied()),
                route.map(|route| route.get_path().len()),
                agent.chaser.state(),
            )
        });
        let controller_before =
            writer_diag.then_some((controller.inputs.move_dir, controller.inputs.move_z));
        let ordinary_node_tolerance = self.traversal_config.node_tolerance;
        let route_endpoint_tolerance = agent.rtsim_controller.path_endpoint_tolerance(pathing_pos);
        let configured_node_tolerance = route_endpoint_tolerance
            .map_or(ordinary_node_tolerance, |tolerance| {
                ordinary_node_tolerance.min(tolerance)
            });
        let chase_result = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            pathing_pos,
            TraversalConfig {
                node_tolerance: configured_node_tolerance,
                min_tgt_dist: 0.25,
                is_target_loaded,
                ..self.traversal_config.clone()
            },
            &read_data.time,
        );
        if writer_diag {
            let route = agent.chaser.get_route();
            let target_delta = pathing_pos - self.pos.0;
            tracing::info!(
                uid = self.uid.0.get(),
                now = read_data.time.0,
                position = ?self.pos.0,
                velocity = ?self.vel.0,
                orientation = ?self.ori.look_vec(),
                requested_target = ?tgt_pos,
                pathing_target = ?pathing_pos,
                target_delta = ?target_delta,
                ordinary_node_tolerance,
                ?route_endpoint_tolerance,
                configured_node_tolerance,
                ?route_before,
                controller_before = ?controller_before,
                chase_result = ?chase_result,
                target_bearing_dot = chase_result
                    .as_ref()
                    .map(|(bearing, _, _)| bearing.dot(target_delta)),
                chaser_last_target_after = ?agent.chaser.last_target(),
                chaser_route_target_after = ?agent.chaser.route_target(),
                chaser_route_complete_after = ?agent.chaser.route_is_complete(),
                chaser_route_next_after = ?route.map(|route| route.next_idx()),
                chaser_route_end_after = ?route.and_then(|route| route.get_path().end().copied()),
                chaser_route_len_after = ?route.map(|route| route.get_path().len()),
                chaser_state_after = ?agent.chaser.state(),
                writer = "agent_chaser",
                explicit_agent_to_bastion_jobs_dependency = false,
                "bastion: goto chaser writer result"
            );
        }
        if let Some((bearing, speed, stuck)) = chase_result {
            let is_colonist = read_data.colonists.contains(*self.entity);
            self.unstuck_if(stuck, read_data.dt.0, controller, is_colonist);
            // ★ COME OFF THE WALL BEFORE PURSUING THE TARGET. While pinned to a
            // wall the target bearing points THROUGH it, and steering that way
            // is what keeps `on_wall` true and the wallrun alive. Overriding
            // the bearing (rather than merely zeroing it) makes the detach an
            // ACTION with a direction, so the body separates instead of hanging
            // in contact at zero horizontal speed.
            let bearing = self.colonist_wall_detach(is_colonist).unwrap_or(bearing);
            self.traverse(controller, bearing, speed * speed_multiplier);
            if writer_diag {
                tracing::info!(
                    uid = self.uid.0.get(),
                    now = read_data.time.0,
                    position = ?self.pos.0,
                    requested_target = ?tgt_pos,
                    ?bearing,
                    speed,
                    stuck,
                    controller_move_dir = ?controller.inputs.move_dir,
                    controller_move_z = controller.inputs.move_z,
                    writer = "agent_traverse",
                    "bastion: goto controller write"
                );
            }
            Some(bearing)
        } else {
            if writer_diag {
                tracing::info!(
                    uid = self.uid.0.get(),
                    now = read_data.time.0,
                    position = ?self.pos.0,
                    requested_target = ?tgt_pos,
                    controller_move_dir = ?controller.inputs.move_dir,
                    controller_move_z = controller.inputs.move_z,
                    writer = "agent_chaser_none",
                    "bastion: goto produced no controller write"
                );
            }
            None
        }
    }

    fn traverse(&self, controller: &mut Controller, bearing: Vec3<f32>, speed: f32) {
        controller.inputs.move_dir =
            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;

        // Only jump if we are grounded and can't blockhop or if we can fly
        let jump_condition = (self.physics_state.on_ground.is_some() && bearing.z > 1.5)
            || self.traversal_config.can_fly;
        // BEARING-TRACE (2026-08-08, 20-vs-23 matched-pair read, Opus-
        // directed): gated to a single uid via BASTION_BEARING_TRACE_UID
        // so it never fires corpus-wide. bearing.z is the jump_if
        // predicate's other conjunct -- min_distance_to_target and
        // on_ground alone can't distinguish "never qualifies" from
        // "qualifies but rarely grounded."
        if std::env::var("BASTION_BEARING_TRACE_UID")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            == Some(self.uid.0.get())
        {
            tracing::info!(
                uid = self.uid.0.get(),
                bearing_z = bearing.z,
                on_ground = self.physics_state.on_ground.is_some(),
                jump_condition,
                "BEARING-TRACE"
            );
        }
        self.jump_if(jump_condition, controller);
        controller.inputs.move_z = bearing.z;
    }

    /// `dt` (E7 Stage 3, T0.79): the OUTER gate ("attempt an unstuck
    /// action this tick") is a per-time hazard, routed through the same
    /// `hazard_chance` the other T0.7 gates use -- was a raw per-tick
    /// 0.05 that silently sped up or slowed down with tick-rate changes.
    /// The INNER jump-vs-roll pick stays a flat one-shot draw (see the
    /// module doc comment): it fires once GIVEN the outer gate already
    /// fired, not on every tick, so it isn't a hazard to begin with.
    /// bastion (2026-08-22): the horizontal direction a colonist must steer to
    /// COME OFF a wall, or `None` if it is not pinned to one.
    ///
    /// Measured cause: of 26 ultimate-fail-safe strandings across a paired A/B,
    /// **23 had `on_ground=false`** and `character_state` was **Wallrun x15,
    /// Climb x5**. Stranded colonists are not on rooftops by accident — they
    /// are on the VERTICAL FACE of buildings, and they stay there because the
    /// travel steer keeps pointing at a target on the far side of the wall.
    /// Every tick the agent pushes INTO the wall, which is exactly the
    /// condition `handle_wallrun` fires on (`on_wall && !on_ground`), so the
    /// wallrun renews itself for as long as the colonist wants to go that way.
    ///
    /// Withholding the unstuck JUMP (the previous fix) stops the colonist
    /// climbing HIGHER; it does not bring it down, because nothing was ever
    /// steering it off. This does.
    ///
    /// ★ SIGN, READ FROM THE PRODUCER, NOT ASSUMED. `PhysicsState::on_wall`
    /// accumulates `dirs[dir]` for each direction whose probe collides, and the
    /// probe is `pos + dir * 0.01` — so the vector points **TOWARD the wall**.
    /// Detaching is its NEGATION. Steering along `on_wall` unnegated would
    /// drive the colonist harder into the surface, which would read in the logs
    /// as "the fix did nothing" rather than as a reversed sign.
    ///
    /// Z is dropped: this is a horizontal push-off, and gravity is what returns
    /// them to the ground. Pure so both the sign and the not-pinned case are
    /// unit-testable without an ECS.
    pub(crate) fn wall_detach_dir(
        on_wall: Option<Vec3<f32>>,
        pinned: bool,
    ) -> Option<Vec3<f32>> {
        if !pinned {
            return None;
        }
        let toward_wall = on_wall?;
        let away = Vec3::new(-toward_wall.x, -toward_wall.y, 0.0);
        // A purely vertical `on_wall` (or a degenerate accumulation of opposed
        // directions cancelling out) carries no horizontal escape; returning a
        // normalized zero here would be a NaN steer.
        if away.magnitude_squared() <= f32::EPSILON {
            return None;
        }
        Some(away.normalized())
    }

    /// The live-state wrapper: a colonist counts as PINNED when it is touching
    /// a wall and either airborne or already in a climb/wallrun state.
    ///
    /// A colonist walking normally along a wall is `on_ground` and in neither
    /// state, so it is NOT pinned and its steer is untouched — the override
    /// must not fire on someone merely walking beside a building.
    fn colonist_wall_detach(&self, is_colonist: bool) -> Option<Vec3<f32>> {
        if !is_colonist {
            return None;
        }
        let pinned = self.physics_state.on_ground.is_none()
            || matches!(
                self.char_state,
                CharacterState::Climb(_) | CharacterState::Wallrun(_)
            );
        Self::wall_detach_dir(self.physics_state.on_wall, pinned)
    }

    pub fn unstuck_if(
        &self,
        condition: bool,
        dt: f32,
        controller: &mut Controller,
        // COLONISTS DO NOT CLIMB OUT OF TROUBLE (Ben, 2026-08-21: "climbing
        // and falling should be actively discouraged -- a colonist scaling a
        // house wall to reach a crate is a bug even when it works").
        discourage_climb: bool,
    ) {
        if condition && self.helper_random_bool(hazard_chance(dt, UNSTUCK_ATTEMPT_RATE)) {
            // THE UNSTUCK ACTION WAS SUSTAINING THE STRANDING IT EXISTS TO
            // FIX. Measured on a live town: a colonist walking to a Cook job
            // ended up character_state=Wallrun on_wall=true on_ground=false,
            // hung there sixty seconds, and was teleported by the ultimate
            // fail-safe with terminal_cause=
            // "below_grade_watch_without_egress_verdict" and every egress
            // counter at ZERO -- the rescue ladder never produced a verdict,
            // because it is built for pits, not for someone stuck up a wall.
            //
            // The branch below is why they stayed: being in Climb made the
            // jump UNCONDITIONAL, and handle_wallrun fires on
            // `on_wall && !on_ground`, so every jump against a wall renews
            // exactly the condition that put them there.
            //
            // ★ A COLONIST NEVER JUMPS (2026-08-22). The previous version of
            // this guard withheld the jump only once `on_wall` was ALREADY
            // true, and that is one tick too late to matter: a colonist
            // standing on clear ground has `on_wall = None`, jumps, drifts into
            // the wall WHILE AIRBORNE, and only then acquires the contact. The
            // guard could never see the jump that caused the problem.
            //
            // Measured on the clean leg, 34 ultimate-fail-safe strandings:
            //     character_state  Wallrun x27, Idle x7
            //     on_ground=false  27 of 34
            //     feet z=408       GROUND LEVEL -- they did not fall from
            //                      anywhere, so they left the ground upward
            //     vertical velocity  median +1.001, max +2.500, only 3 of 34
            //                        falling
            //     horizontal speed   median 0.004 -- going straight up,
            //                        not along
            //
            // `handle_wallrun` needs `on_wall && !on_ground` and NO INTENT AT
            // ALL -- unlike `handle_climb`, which requires `move_dir` pointing
            // into the wall and which the wall-detach steer already defeats
            // (Climb fell 4 -> 0 when that landed). So steering away cannot
            // prevent a wallrun; only never being airborne beside a wall can.
            //
            // The jump is the ONLY way a colonist puts itself in the air.
            // Removing it removes the entry condition rather than fighting the
            // state afterwards, and it is what Ben asked for in plain words:
            // climbing and falling are to be actively discouraged, and a
            // person who cannot reach a crate walks around the building.
            //
            // Roll stays: it is a GROUND action, it is the actual unstick for
            // the case this function exists to serve (a body wedged on
            // geometry), and it cannot start a wallrun.
            let in_climb = matches!(self.char_state, CharacterState::Climb(_));
            if unstuck_should_jump(discourage_climb, in_climb, self.helper_random_bool(0.5)) {
                controller.push_basic_input(InputKind::Jump);
            } else {
                // Cancel any jump another node queued this tick, not merely
                // decline to add one -- `push_basic_input` is not the only
                // writer, and a jump that arrives from elsewhere strands the
                // colonist exactly the same way.
                if controller.queued_inputs.contains_key(&InputKind::Jump) {
                    controller.push_cancel_input(InputKind::Jump);
                }
                controller.push_basic_input(InputKind::Roll);
            }
        } else {
            if controller.queued_inputs.contains_key(&InputKind::Jump) {
                controller.push_cancel_input(InputKind::Jump);
            }
            if controller.queued_inputs.contains_key(&InputKind::Roll) {
                controller.push_cancel_input(InputKind::Roll);
            }
        }
    }

    pub fn jump_if(&self, condition: bool, controller: &mut Controller) {
        if condition {
            controller.push_basic_input(InputKind::Jump);
        } else if controller.queued_inputs.contains_key(&InputKind::Jump) {
            controller.push_cancel_input(InputKind::Jump)
        }
    }

    pub fn idle(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        _emitters: &mut AgentEmitters,
        rng: &mut impl RngExt,
    ) {
        enum ActionTimers {
            TimerIdle = 0,
        }

        agent
            .awareness
            .change_by(STD_AWARENESS_DECAY_RATE * read_data.dt.0);

        // Light lanterns at night
        // TODO Add a method to turn on NPC lanterns underground
        let lantern_equipped = self
            .inventory
            .equipped(EquipSlot::Lantern)
            .as_ref()
            .is_some_and(|item| matches!(&*item.kind(), comp::item::ItemKind::Lantern(_)));
        let lantern_turned_on = self.light_emitter.is_some();
        let day_period = DayPeriod::from(read_data.time_of_day.0);
        // Only emit event for agents that have a lantern equipped
        if lantern_equipped && hazard(rng, read_data.dt.0, LANTERN_TOGGLE_RATE) {
            if day_period.is_dark() && !lantern_turned_on {
                // Agents with turned off lanterns turn them on randomly once it's
                // nighttime and keep them on.
                // Only emit event for agents that sill need to
                // turn on their lantern.
                controller.push_event(ControlEvent::EnableLantern)
            } else if lantern_turned_on && day_period.is_light() {
                // agents with turned on lanterns turn them off randomly once it's
                // daytime and keep them off.
                controller.push_event(ControlEvent::DisableLantern)
            }
        };

        if let Some(body) = self.body {
            let attempt_heal = if matches!(body, Body::Humanoid(_)) {
                self.damage < IDLE_HEALING_ITEM_THRESHOLD
            } else {
                true
            };
            if attempt_heal && self.heal_self(agent, controller, true) {
                agent.behavior_state.timers[ActionTimers::TimerIdle as usize] = 0.01;
                return;
            }
        } else {
            agent.behavior_state.timers[ActionTimers::TimerIdle as usize] = 0.01;
            return;
        }

        agent.behavior_state.timers[ActionTimers::TimerIdle as usize] = 0.0;

        'activity: {
            match agent.rtsim_controller.activity {
                Some(NpcActivity::Goto(travel_to, speed_factor)) => {
                    self.dismount_uncontrollable(controller, read_data);
                    // DECISIONS #94 (Option A): a `Goto` issued to a
                    // sitting NPC is silently inert -- `move_dir` cannot
                    // move a seated character, and the ONLY other
                    // `ControlAction::Stand` in this whole tree lives
                    // under `NpcActivity::Talk`. Idle wandering's own
                    // `IDLE_SIT_RATE` roll can seat any agent (including
                    // one under a bastion `Goto`, since the ActiveJob
                    // gate at `rtsim/tick.rs` protects `activity` from
                    // being overwritten, not the separate `actions`
                    // channel `Sit` arrives on) with nothing downstream
                    // to stand it back up. Fixed at the class, not the
                    // caller: every `Goto` consumer, bastion or vanilla,
                    // now stands before moving.
                    controller.push_action(ControlAction::Stand);
                    // #94 acceptance instrument (Opus, Fable co-signed):
                    // the defect this fixes is RNG-gated (IDLE_SIT_RATE),
                    // so "zero ULTIMATE FAIL-SAFE teleports" alone can't
                    // distinguish "fixed" from "never exercised" on any
                    // one run. Witness the fix's own precondition --
                    // ungated, permanent, self-accounting: if a future
                    // refactor breaks the Stand push above, this count
                    // silently drops to zero while fail-safes climb,
                    // which is diagnostic on its own.
                    if matches!(self.char_state, CharacterState::Sit) {
                        tracing::info!(
                            uid = self.uid.0.get(),
                            "bastion GOTO-STAND-RESCUE"
                        );
                    }

                    agent.bearing = Vec2::zero();

                    // If it has an rtsim destination and can fly, then it should.
                    // If it is flying and bumps something above it, then it should move down.
                    if self.traversal_config.can_fly
                        && !read_data
                            .terrain
                            .ray(self.pos.0, self.pos.0 + (Vec3::unit_z() * 3.0))
                            .until(Block::is_solid)
                            .cast()
                            .1
                            .map_or(true, |b| b.is_some())
                    {
                        controller.push_basic_input(InputKind::Fly);
                    } else {
                        controller.push_cancel_input(InputKind::Fly)
                    }

                    if let Some(bearing) = self.path_toward_target(
                        agent,
                        controller,
                        travel_to,
                        read_data,
                        Path::AtTarget,
                        Some(speed_factor),
                    ) {
                        let height_offset = bearing.z
                            + if self.traversal_config.can_fly {
                                // NOTE: costs 4 us (imbris)
                                let obstacle_ahead = read_data
                                    .terrain
                                    .ray(
                                        self.pos.0 + Vec3::unit_z(),
                                        self.pos.0
                                            + bearing.try_normalized().unwrap_or_else(Vec3::unit_y)
                                                * 80.0
                                            + Vec3::unit_z(),
                                    )
                                    .until(Block::is_solid)
                                    .cast()
                                    .1
                                    .map_or(true, |b| b.is_some());

                                let mut ground_too_close = self
                                    .body
                                    .map(|body| {
                                        #[cfg(feature = "worldgen")]
                                        let height_approx = self.pos.0.z
                                            - read_data
                                                .world
                                                .sim()
                                                .get_alt_approx(
                                                    self.pos.0.xy().map(|x: f32| x as i32),
                                                )
                                                .unwrap_or(0.0);
                                        #[cfg(not(feature = "worldgen"))]
                                        let height_approx = self.pos.0.z;

                                        height_approx < body.flying_height()
                                    })
                                    .unwrap_or(false);

                                const NUM_RAYS: usize = 5;

                                // NOTE: costs 15-20 us (imbris)
                                for i in 0..=NUM_RAYS {
                                    let magnitude = self.body.map_or(20.0, |b| b.flying_height());
                                    // Lerp between a line straight ahead and straight down to
                                    // detect a
                                    // wedge of obstacles we might fly into (inclusive so that both
                                    // vectors are sampled)
                                    if let Some(dir) = Lerp::lerp(
                                        -Vec3::unit_z(),
                                        Vec3::new(bearing.x, bearing.y, 0.0),
                                        i as f32 / NUM_RAYS as f32,
                                    )
                                    .try_normalized()
                                    {
                                        ground_too_close |= read_data
                                            .terrain
                                            .ray(self.pos.0, self.pos.0 + magnitude * dir)
                                            .until(|b: &Block| b.is_solid() || b.is_liquid())
                                            .cast()
                                            .1
                                            .is_ok_and(|b| b.is_some())
                                    }
                                }

                                if obstacle_ahead || ground_too_close {
                                    5.0 //fly up when approaching obstacles
                                } else {
                                    -2.0
                                } //flying things should slowly come down from the stratosphere
                            } else {
                                0.05 //normal land traveller offset
                            };

                        if let Some(mpid) = agent.multi_pid_controllers.as_mut() {
                            if let Some(z_controller) = mpid.z_controller.as_mut() {
                                z_controller.sp = self.pos.0.z + height_offset;
                                controller.inputs.move_z = z_controller.calc_err();
                                // when changing setpoints, limit PID windup
                                z_controller.limit_integral_windup(|z| *z = z.clamp(-10.0, 10.0));
                            } else {
                                controller.inputs.move_z = 0.0;
                            }
                        } else {
                            controller.inputs.move_z = height_offset;
                        }
                    }

                    // Put away weapon
                    if hazard(rng, read_data.dt.0, UNWIELD_IDLE_RATE)
                        && matches!(
                            read_data.char_states.get(*self.entity),
                            Some(CharacterState::Wielding(_))
                        )
                    {
                        controller.push_action(ControlAction::Unwield);
                    }
                    break 'activity; // Don't fall through to idle wandering
                },

                Some(NpcActivity::GotoFlying(
                    travel_to,
                    speed_factor,
                    height_offset,
                    direction_override,
                    flight_mode,
                )) => {
                    self.dismount_uncontrollable(controller, read_data);

                    if self.traversal_config.vectored_propulsion {
                        // This is the action for Airships.

                        // Note - when the Agent code is run, the entity will be the captain that is
                        // mounted on the ship and the movement calculations
                        // must be done relative to the captain's position
                        // which is offset from the ship's position and apparently scaled.
                        // When the State system runs to apply the movement accel and velocity, the
                        // ship entity will be the subject entity.

                        // entities that have vectored propulsion should always be flying
                        // and do not depend on forward movement or displacement to move.
                        // E.g., Airships.
                        controller.push_basic_input(InputKind::Fly);

                        // These entities can either:
                        // - Move in any direction, following the terrain
                        // - Move essentially vertically, as in
                        //   - Hover in place (station-keeping), like at a dock
                        //   - Move straight up or down, as when taking off or landing

                        // If there is lateral movement, then the entity's direction should be
                        // aligned with that movement direction. If there is
                        // no or minimal lateral movement, then the entity
                        // is either hovering or moving vertically, and the entity's direction
                        // should not change. This is indicated by the direction_override parameter.

                        // If a direction override is provided, attempt to orient the entity in that
                        // direction.
                        if let Some(direction) = direction_override {
                            controller.inputs.look_dir = direction;
                        } else {
                            // else orient the entity in the direction of travel, but keep it level
                            controller.inputs.look_dir =
                                Dir::from_unnormalized((travel_to - self.pos.0).xy().with_z(0.0))
                                    .unwrap_or_default();
                        }

                        // the look_dir will be used as the orientation override. Orientation
                        // override is always enabled for airships, so this
                        // code must set controller.inputs.look_dir for
                        // all cases (vertical or lateral movement).

                        // When pid_mode is PureZ, only the z component of movement is is adjusted
                        // by the PID controller.

                        // If the PID controller is not set or the mode or gain has changed, create
                        // a new one. PidControllers is a wrapper around one
                        // or more PID controllers. Each controller acts on
                        // one axis of movement. There are three controllers for FixedDirection mode
                        // and one for PureZ mode.
                        if agent
                            .multi_pid_controllers
                            .as_ref()
                            .is_some_and(|mpid| mpid.mode != flight_mode)
                        {
                            agent.multi_pid_controllers = None;
                        }
                        let mpid = agent.multi_pid_controllers.get_or_insert_with(|| {
                            PidControllers::<16>::new_multi_pid_controllers(flight_mode, travel_to)
                        });
                        let sample_time = read_data.time.0;

                        #[allow(unused_variables)]
                        let terrain_alt_with_lookahead = |dist: f32| -> f32 {
                            // look ahead some blocks to sample the terrain altitude
                            #[cfg(feature = "worldgen")]
                            let terrain_alt = read_data
                                .world
                                .sim()
                                .get_alt_approx(
                                    (self.pos.0.xy()
                                        + controller.inputs.look_dir.to_vec().xy() * dist)
                                        .map(|x: f32| x as i32),
                                )
                                .unwrap_or(0.0);
                            #[cfg(not(feature = "worldgen"))]
                            let terrain_alt = 0.0;
                            terrain_alt
                        };

                        if flight_mode == FlightMode::FlyThrough {
                            let travel_vec = travel_to - self.pos.0;
                            let bearing =
                                travel_vec.xy().try_normalized().unwrap_or_else(Vec2::zero);
                            controller.inputs.move_dir = bearing * speed_factor;
                            let terrain_alt = terrain_alt_with_lookahead(32.0);
                            let height = height_offset.unwrap_or(100.0);
                            if let Some(z_controller) = mpid.z_controller.as_mut() {
                                z_controller.sp = terrain_alt + height;
                            }
                            mpid.add_measurement(sample_time, self.pos.0);
                            // check if getting close to terrain
                            if terrain_alt >= self.pos.0.z - 32.0 {
                                // It's likely the airship will hit an upslope. Maximize the climb
                                // rate.
                                controller.inputs.move_z = 1.0 * speed_factor;
                                // try to stop forward movement
                                controller.inputs.move_dir =
                                    self.vel.0.xy().try_normalized().unwrap_or_else(Vec2::zero)
                                        * -1.0
                                        * speed_factor;
                            } else {
                                controller.inputs.move_z =
                                    mpid.calc_err_z().unwrap_or(0.0).min(1.0) * speed_factor;
                            }
                            // PID controllers that change the setpoint suffer from "windup", where
                            // the integral term accumulates error.
                            // There are several ways to compensate for this. One way is to limit
                            // the integral term to a range.
                            mpid.limit_windup_z(|z| *z = z.clamp(-20.0, 20.0));
                        } else {
                            // When doing step-wise movement, the target waypoint changes. Make sure
                            // the PID controller setpoints keep up with
                            // the changes.
                            if let Some(x_controller) = mpid.x_controller.as_mut() {
                                x_controller.sp = travel_to.x;
                            }
                            if let Some(y_controller) = mpid.y_controller.as_mut() {
                                y_controller.sp = travel_to.y;
                            }

                            // If terrain following, get the terrain altitude at the current
                            // position. Set the z setpoint to the max
                            // of terrain alt + height offset or the
                            // target z.
                            let z_setpoint = if let Some(height) = height_offset {
                                let clearance_alt = terrain_alt_with_lookahead(16.0) + height;
                                clearance_alt.max(travel_to.z)
                            } else {
                                travel_to.z
                            };
                            if let Some(z_controller) = mpid.z_controller.as_mut() {
                                z_controller.sp = z_setpoint;
                            }

                            mpid.add_measurement(sample_time, self.pos.0);
                            controller.inputs.move_dir.x =
                                mpid.calc_err_x().unwrap_or(0.0).min(1.0) * speed_factor;
                            controller.inputs.move_dir.y =
                                mpid.calc_err_y().unwrap_or(0.0).min(1.0) * speed_factor;
                            controller.inputs.move_z =
                                mpid.calc_err_z().unwrap_or(0.0).min(1.0) * speed_factor;

                            // Limit the integral term to a range to prevent windup.
                            mpid.limit_windup_x(|x| *x = x.clamp(-1.0, 1.0));
                            mpid.limit_windup_y(|y| *y = y.clamp(-1.0, 1.0));
                            mpid.limit_windup_z(|z| *z = z.clamp(-1.0, 1.0));
                        }
                    }
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Gather(_resources)) => {
                    // bastion (B-AG1): real gathering is row 38's block
                    // (GATHER — sprite search + collect). Until it lands,
                    // DEGRADE GRACEFULLY: fall through to idle wandering.
                    // The previous stub pushed Dance and broke out — a
                    // promoted gatherer danced in place indefinitely, the
                    // exact stuck-looking NPC the promote-handoff exists to
                    // prevent. Wandering reads as alive AND keeps the NPC
                    // moving through re-plans (rtsim re-decides on its own
                    // cadence).
                },
                Some(NpcActivity::Dance(dir)) => {
                    // Look at targets specified by rtsim
                    if let Some(look_dir) = dir {
                        controller.inputs.look_dir = look_dir;
                        if self.ori.look_dir().dot(look_dir.to_vec()) < 0.95 {
                            controller.inputs.move_dir = look_dir.to_vec().xy() * 0.01;
                            break 'activity;
                        } else {
                            controller.inputs.move_dir = Vec2::zero();
                        }
                    }
                    controller.push_action(ControlAction::Dance);
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Cheer(dir)) => {
                    if let Some(look_dir) = dir {
                        controller.inputs.look_dir = look_dir;
                        if self.ori.look_dir().dot(look_dir.to_vec()) < 0.95 {
                            controller.inputs.move_dir = look_dir.to_vec().xy() * 0.01;
                            break 'activity;
                        } else {
                            controller.inputs.move_dir = Vec2::zero();
                        }
                    }
                    controller.push_action(ControlAction::Talk(None));
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Sit(dir, pos)) => {
                    if let Some(pos) =
                        pos.filter(|p| read_data.terrain.get(*p).is_ok_and(|b| b.is_mountable()))
                    {
                        if !read_data.is_volume_riders.contains(*self.entity) {
                            controller
                                .push_event(ControlEvent::MountVolume(VolumePos::terrain(pos)));
                        }
                    } else {
                        if let Some(look_dir) = dir {
                            controller.inputs.look_dir = look_dir;
                            if self.ori.look_dir().dot(look_dir.to_vec()) < 0.95 {
                                controller.inputs.move_dir = look_dir.to_vec().xy() * 0.01;
                                break 'activity;
                            } else {
                                controller.inputs.move_dir = Vec2::zero();
                            }
                        }
                        controller.push_action(ControlAction::Sit);
                    }
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::HuntAnimals) => {
                    if hazard(rng, read_data.dt.0, HUNT_RETARGET_RATE) {
                        self.choose_target(
                            agent,
                            controller,
                            read_data,
                            AgentData::is_hunting_animal,
                        );
                    }
                },
                Some(NpcActivity::Talk(target)) => {
                    if agent.target.is_none()
                        && let Some(target) = read_data.id_maps.actor_entity(target)
                        && let Some(target_uid) = read_data.uids.get(target)
                    {
                        // We're always aware of someone we're talking to
                        controller.push_action(ControlAction::Stand);
                        self.look_toward(controller, read_data, target);
                        controller.push_action(ControlAction::Talk(Some(*target_uid)));
                        break 'activity;
                    }
                },
                None => {},
            }

            let owner_uid = self
                .alignment
                .and_then(|alignment| match_some!(alignment, Alignment::Owned(uid) => uid));

            let owner = owner_uid.and_then(|owner_uid| get_entity_by_id(*owner_uid, read_data));

            let is_being_pet = read_data
                .interactors
                .get(*self.entity)
                .and_then(|interactors| interactors.get(*owner_uid?))
                .is_some_and(|interaction| matches!(interaction.kind, InteractionKind::Pet));

            let is_in_range = owner
                .and_then(|owner| read_data.positions.get(owner))
                .is_some_and(|pos| pos.0.distance_squared(self.pos.0) < MAX_MOUNT_RANGE.powi(2));

            // Idle NPCs should try to jump on the shoulders of their owner, sometimes.
            if read_data.is_riders.contains(*self.entity) {
                if hazard(rng, read_data.dt.0, PET_DISMOUNT_RATE) {
                    self.dismount_uncontrollable(controller, read_data);
                } else {
                    break 'activity;
                }
            } else if let Some(owner_uid) = owner_uid
                && is_in_range
                && !is_being_pet
                && hazard(rng, read_data.dt.0, PET_MOUNT_RATE)
            {
                controller.push_event(ControlEvent::Mount(*owner_uid));
                break 'activity;
            }

            // Bats should fly
            // Use a proportional controller as the bouncing effect mimics bat flight
            if self.traversal_config.can_fly
                && self
                    .inventory
                    .equipped(EquipSlot::ActiveMainhand)
                    .as_ref()
                    .is_some_and(|item| {
                        item.ability_spec().is_some_and(|a_s| match &*a_s {
                            AbilitySpec::Custom(spec) => {
                                matches!(
                                    spec.as_str(),
                                    "Simple Flying Melee"
                                        | "Bloodmoon Bat"
                                        | "Vampire Bat"
                                        | "Flame Wyvern"
                                        | "Frost Wyvern"
                                        | "Cloud Wyvern"
                                        | "Sea Wyvern"
                                        | "Weald Wyvern"
                                )
                            },
                            _ => false,
                        })
                    })
            {
                // Bats don't like the ground, so make sure they are always flying
                controller.push_basic_input(InputKind::Fly);
                // Use a proportional controller with a coefficient of 1.0 to
                // maintain altitude
                let alt = read_data
                    .terrain
                    .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 7.0))
                    .until(Block::is_solid)
                    .cast()
                    .0;
                let set_point = 5.0;
                let error = set_point - alt;
                controller.inputs.move_z = error;
                // If on the ground, jump
                if self.physics_state.on_ground.is_some() {
                    controller.push_basic_input(InputKind::Jump);
                }
            }

            let diff = Vec2::new(rng.random::<f32>() - 0.5, rng.random::<f32>() - 0.5);
            agent.bearing += (diff * 0.1 - agent.bearing * 0.01)
                * agent.psyche.idle_wander_factor.max(0.0).sqrt()
                * agent.psyche.aggro_range_multiplier.max(0.0).sqrt();
            if let Some(patrol_origin) = agent.patrol_origin
                // Use owner as patrol origin otherwise
                .or_else(|| if let Some(Alignment::Owned(owner_uid)) = self.alignment
                    && let Some(owner) = get_entity_by_id(*owner_uid, read_data)
                    && let Some(pos) = read_data.positions.get(owner)
                {
                    Some(pos.0)
                } else {
                    None
                })
            {
                agent.bearing += ((patrol_origin.xy() - self.pos.0.xy())
                    / (0.01 + MAX_PATROL_DIST * agent.psyche.idle_wander_factor))
                    * 0.015
                    * agent.psyche.idle_wander_factor;
            }

            // bastion (ZONE-0): the ACTIVITY-ZONE soft magnet — a nearby
            // Meeting zone pulls a COLONIST's idle wander toward its
            // center, using EXACTLY the patrol-origin mechanism above (a
            // weak bearing BIAS on the random walk, never a Goto). Needs
            // always win BY CONSTRUCTION: hunger/threat/work branch off
            // long before this idle fall-through executes, and the bias
            // cannot hold anyone (a stronger drive simply never runs this
            // code). Vanilla NPCs keep pure wander (colonist-gated).
            if read_data.colonists.contains(*self.entity)
                && let Some(center) = read_data
                    .activity_zones
                    .0
                    .iter()
                    .map(|(_, r)| {
                        Vec2::new(
                            (r.min.x + r.max.x) as f32 / 2.0,
                            (r.min.y + r.max.y) as f32 / 2.0,
                        )
                    })
                    .filter(|c| {
                        c.distance_squared(self.pos.0.xy())
                            < common::bastion::ZONE_MAGNET_RANGE.powi(2)
                    })
                    .min_by(|a, b| {
                        a.distance_squared(self.pos.0.xy())
                            .partial_cmp(&b.distance_squared(self.pos.0.xy()))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            {
                agent.bearing += (center - self.pos.0.xy())
                    / (0.01 + common::bastion::ZONE_MAGNET_RANGE)
                    * common::bastion::ZONE_MAGNET_WEIGHT;
            }

            // Stop if we're too close to a wall
            // or about to walk off a cliff
            // NOTE: costs 1 us (imbris) <- before cliff raycast added
            agent.bearing *= 0.1
                + if read_data
                    .terrain
                    .ray(
                        self.pos.0 + Vec3::unit_z(),
                        self.pos.0
                            + Vec3::from(agent.bearing)
                                .try_normalized()
                                .unwrap_or_else(Vec3::unit_y)
                                * 5.0
                            + Vec3::unit_z(),
                    )
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_none())
                    && read_data
                        .terrain
                        .ray(
                            self.pos.0
                                + Vec3::from(agent.bearing)
                                    .try_normalized()
                                    .unwrap_or_else(Vec3::unit_y),
                            self.pos.0
                                + Vec3::from(agent.bearing)
                                    .try_normalized()
                                    .unwrap_or_else(Vec3::unit_y)
                                - Vec3::unit_z() * 4.0,
                        )
                        .until(Block::is_solid)
                        .cast()
                        .0
                        < 3.0
                {
                    0.9
                } else {
                    0.0
                };

            if agent.bearing.magnitude_squared() > 0.5f32.powi(2) {
                controller.inputs.move_dir = agent.bearing;
            }

            // Put away weapon
            if hazard(rng, read_data.dt.0, UNWIELD_IDLE_RATE)
                && matches!(
                    read_data.char_states.get(*self.entity),
                    Some(CharacterState::Wielding(_))
                )
            {
                controller.push_action(ControlAction::Unwield);
            }

            if hazard(rng, read_data.dt.0, IDLE_UTTERANCE_RATE) {
                controller.push_utterance(UtteranceKind::Calm);
            }

            // Sit
            // COLONISTS DO NOT IDLE-SIT (2026-08-21). A live town measured
            // mean_stuck = 3.04 of 8 -- 38% holding a job and not moving --
            // alongside 261 GOTO-STAND-RESCUE emits. That emit is NOT a failed
            // rescue, whatever the improvement list said: it is the precondition
            // witness for #94, and it fires ONLY when a colonist under a Goto is
            // found SITTING. 261 firings = 261 times someone sat down mid-errand.
            //
            // #94 fixed the symptom at the consumer (every Goto pushes Stand
            // first), but the sit still happens, and a colonist repeatedly seated
            // and stood reads as speed < 0.2 -- EXACTLY how the census defines
            // `stuck`. Standing them up afterwards cannot make them look
            // purposeful; it can only stop them being permanently seated.
            //
            // A colonist's rest is MODELLED: a RestAt self-job in an owned bed.
            // Vanilla idle-sitting is villager flavour that fights that system.
            // Vanilla NPCs keep it -- they have no job to interrupt.
            if !read_data.colonists.contains(*self.entity)
                && hazard(rng, read_data.dt.0, IDLE_SIT_RATE)
            {
                controller.push_action(ControlAction::Sit);
            }
        }
    }

    pub fn follow(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        tgt_pos: &Pos,
    ) {
        self.dismount_uncontrollable(controller, read_data);

        if let Some((bearing, speed, stuck)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            tgt_pos.0,
            TraversalConfig {
                min_tgt_dist: AVG_FOLLOW_DIST,
                ..self.traversal_config.clone()
            },
            &read_data.time,
        ) {
            self.unstuck_if(stuck, read_data.dt.0, controller, read_data.colonists.contains(*self.entity));
            let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
            self.traverse(
                controller,
                bearing,
                speed.min(0.2 + (dist_sqrd - AVG_FOLLOW_DIST.powi(2)) / 8.0),
            );
        }
    }

    pub fn look_toward(
        &self,
        controller: &mut Controller,
        read_data: &ReadData,
        target: EcsEntity,
    ) -> bool {
        if let Some(tgt_pos) = read_data.positions.get(target)
            && !is_steering(*self.entity, read_data)
            && let Some(dir) = Dir::look_toward(
                self.pos,
                self.body,
                Some(&comp::Scale(self.scale)),
                tgt_pos,
                read_data.bodies.get(target),
                read_data.scales.get(target),
            )
        {
            controller.inputs.look_dir = dir;
            true
        } else {
            false
        }
    }

    pub fn flee(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        tgt_pos: &Pos,
    ) {
        // Proportion of full speed
        const MAX_FLEE_SPEED: f32 = 0.65;

        self.dismount_uncontrollable(controller, read_data);

        if let Some(body) = self.body
            && body.can_strafe()
            && !self.is_gliding
        {
            controller.push_action(ControlAction::Unwield);
        }

        if let Some((bearing, speed, stuck)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            // Away from the target (ironically)
            self.pos.0
                + (self.pos.0 - tgt_pos.0)
                    .try_normalized()
                    .unwrap_or_else(Vec3::unit_y)
                    * 50.0,
            TraversalConfig {
                min_tgt_dist: 1.25,
                ..self.traversal_config.clone()
            },
            &read_data.time,
        ) {
            let is_colonist = read_data.colonists.contains(*self.entity);
            self.unstuck_if(stuck, read_data.dt.0, controller, is_colonist);
            // Same override on the FLEE path: a colonist fleeing into a wall
            // wallruns exactly as one walking to a job does, and a body pinned
            // mid-flee is the worst case of all -- it is being chased.
            let bearing = self.colonist_wall_detach(is_colonist).unwrap_or(bearing);
            self.traverse(controller, bearing, speed.min(MAX_FLEE_SPEED));
        }
    }

    /// Attempt to consume a healing item, and return whether any healing items
    /// were queued. Callers should use this to implement a delay so that
    /// the healing isn't interrupted. If `relaxed` is `true`, we allow eating
    /// food and prioritise healing.
    pub fn heal_self(
        &self,
        _agent: &mut Agent,
        controller: &mut Controller,
        relaxed: bool,
    ) -> bool {
        // If we already have a healing buff active, don't start another one.
        if self.buffs.is_some_and(|buffs| {
            buffs.iter_active().flatten().any(|buff| {
                // We don't care about seeing the optional combat requirements that can be
                // tacked onto buff effects, so we'll just pass in None to this
                // Inspection only (checking effect shape, not applying a
                // buff), so the instance id this derives is discarded --
                // any Time value is fine.
                buff.kind
                    .effects(&buff.data, None, common::resources::Time::default())
                    .iter()
                    .any(|effect| {
                    if let comp::BuffEffect::HealthChangeOverTime { rate, .. } = effect
                        && *rate > 0.0
                    {
                        true
                    } else {
                        false
                    }
                })
            })
        }) {
            return false;
        }

        // Wait for potion sickness to wear off if potions are less than 50% effective.
        let heal_multiplier = self.stats.map_or(1.0, |s| s.item_effect_reduction);
        if heal_multiplier < 0.5 {
            return false;
        }
        if let Some(id) = select_healing_item(self.inventory, relaxed, heal_multiplier) {
            use comp::inventory::slot::Slot;
            controller.push_action(ControlAction::InventoryAction(InventoryAction::Use(
                Slot::Inventory(id),
            )));
            true
        } else {
            false
        }
    }

    pub fn choose_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        is_enemy: fn(&Self, EcsEntity, &ReadData) -> bool,
    ) {
        enum ActionStateTimers {
            TimerChooseTarget = 0,
        }
        agent.behavior_state.timers[ActionStateTimers::TimerChooseTarget as usize] = 0.0;
        let mut aggro_on = false;

        // Search the area.
        // TODO: choose target by more than just distance
        let common::CachedSpatialGrid(grid) = self.cached_spatial_grid;

        // DET-AIT-001 (v8 npc-combat-targeting, Critical): the spatial grid
        // yields candidates in cell-traversal order. That order then drives
        // tie-breaking in target selection AND the call order of the
        // per-candidate helper RNG (`can_sense_directly_near`) — both
        // authoritative. Sort the candidates by Uid so target choice is a pure
        // function of the candidate SET, not grid traversal order (this also
        // makes the shared helper-RNG cursor advance in a canonical order,
        // addressing the ordering half of DET-AIT-002).
        let mut entities_nearby = grid
            .in_circle_aabr(self.pos.0.xy(), agent.psyche.search_dist())
            .collect_vec();
        entities_nearby.sort_unstable_by_key(|e| read_data.uids.get(*e).map(|u| u.0));

        let get_pos = |entity| read_data.positions.get(entity);
        // T3.35+T3.39 (E3-WT): the bool is now `Option<ThreatClassV1>` --
        // `None` for a non-combat interaction target (item pickup /
        // downed-ally-save, unchanged), `Some(class)` for a combat
        // candidate, class-tagged so the threat_policy wiring below can
        // rank AttackingAlly (defending) above HostileNearby (merely
        // hostile), which the old bare bool couldn't distinguish.
        let get_enemy = |(entity, attack_target): (EcsEntity, bool)| {
            if attack_target {
                if is_enemy(self, entity, read_data) {
                    Some((entity, Some(ThreatClassV1::HostileNearby)))
                } else if self.should_defend(entity, read_data) {
                    if let Some(attacker) = get_attacker(entity, read_data) {
                        if !self.passive_towards(attacker, read_data) {
                            // aggro_on: attack immediately, do not warn/menace.
                            aggro_on = true;
                            Some((attacker, Some(ThreatClassV1::AttackingAlly)))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                Some((entity, None))
            }
        };
        // DECISIONS #97 (Ben, ROW-ITEM6-PROVISIONING-PACKET): ambient
        // rtsim NPCs do not loot ANY world item drop this era. A plain
        // function (not inlined as `if false`) so the detailed
        // wants-pickup/loot-owner logic it guards below stays reachable
        // to the compiler and isn't lint-flagged as dead code. Flipping
        // this to `true` (or wiring it to a real per-NPC trait later)
        // is the future thievery feature's first line.
        fn ambient_item_looting_enabled() -> bool { false }

        let is_valid_target = |entity: EcsEntity| match read_data.bodies.get(entity) {
            Some(Body::Item(item)) => {
                // Bastion colonists must not opportunistically auto-loot
                // their own work drops via this vanilla wander-and-grab
                // behavior — a mined/chopped item is meant to sit on the
                // ground until a deliberate (future) hauling system
                // collects it. Gated on the `Colonist` component itself
                // rather than `ActiveJob`: the job (and its `ActiveJob`)
                // is already released the same tick the item drops, so an
                // `ActiveJob`-only gate would miss the very colonist who
                // just finished the job.
                if read_data.colonists.contains(*self.entity) {
                    return None;
                }
                // DECISIONS #97 (Ben, ROW-ITEM6-PROVISIONING-PACKET):
                // ambient rtsim NPCs do not loot ANY world item drops
                // this era -- distinct from the colonist exclusion
                // above (which is about a colonist's OWN work drops
                // specifically). Thievery becomes a designed feature
                // later; lifting this gate is its first line, which is
                // why the detailed wants-pickup/loot-owner logic below
                // stays intact rather than being deleted.
                if !ambient_item_looting_enabled() {
                    return None;
                }
                if !matches!(item, body::item::Body::Thrown(_)) {
                    let is_humanoid = matches!(self.body, Some(Body::Humanoid(_)));
                    let avoids_item_drops = matches!(
                        self.body,
                        Some(Body::BipedLarge(biped_large::Body {
                            species: biped_large::Species::Gigasfrost
                                | biped_large::Species::Gigasfire,
                            ..
                        }))
                    );
                    // If the agent is humanoid, it will pick up all kinds of item drops. If the
                    // agent isn't humanoid, it will pick up only consumable item drops.
                    let wants_pickup = !avoids_item_drops
                        && (is_humanoid || matches!(item, body::item::Body::Consumable));

                    // The agent will attempt to pickup the item if it wants to pick it up and
                    // is allowed to (bastion ENGINE-OPT-3, ledger #160: the old inline
                    // predicate wrapped this whole authorization in `!` — humanoids
                    // attempted pickup exactly when can_pickup was FALSE and refused
                    // their own/group/soft drops — and its soft-wish term additionally
                    // inverted hostility against its own comment. Decision extracted
                    // pure; truth table pinned in the tests below.)
                    let attempt_pickup = read_data
                        .loot_owners
                        .get(entity)
                        .map_or(wants_pickup, |loot_owner| {
                            let authorized = loot_owner.can_pickup(
                                *self.uid,
                                read_data.groups.get(entity),
                                self.alignment,
                                self.body,
                                None,
                            );
                            // If we are hostile towards the owner, ignore their wish
                            // to not pick up the loot (soft ownership). An
                            // unresolvable owner (offline) counts as not-hostile, so
                            // the wish is respected — the conservative default the
                            // old code also took.
                            let hostile_to_owner = loot_owner
                                .uid()
                                .and_then(|uid| read_data.id_maps.uid_entity(uid))
                                .is_some_and(|owner| is_enemy(self, owner, read_data));
                            loot_attempt_decision(
                                wants_pickup,
                                authorized,
                                loot_owner.is_soft(),
                                hostile_to_owner,
                            )
                        });

                    if attempt_pickup {
                        Some((entity, false))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            _ => {
                if read_data
                    .healths
                    .get(entity)
                    .is_some_and(|health| !health.is_dead && !is_invulnerable(entity, read_data))
                {
                    let needs_saving = comp::is_downed(
                        read_data.healths.get(entity),
                        read_data.char_states.get(entity),
                    );

                    let wants_to_save = match (self.alignment, read_data.alignments.get(entity)) {
                        // Npcs generally do want to save players. Could have extra checks for
                        // sentiment in the future.
                        (Some(Alignment::Npc), _) if read_data.presences.get(entity).is_some_and(|presence| matches!(presence.kind, PresenceKind::Character(_))) => true,
                        (Some(Alignment::Npc), Some(Alignment::Npc)) => true,
                        (Some(Alignment::Enemy), Some(Alignment::Enemy)) => true,
                        _ => false,
                    } && agent.allowed_to_speak()
                        // Check that anyone else isn't already saving them.
                        && read_data
                            .interactors
                            .get(entity).is_none_or(|interactors| {
                                !interactors.has_interaction(InteractionKind::HelpDowned)
                            }) && self.char_state.can_interact();

                    // TODO: Make targets that need saving have less priority as a target.
                    Some((entity, !(needs_saving && wants_to_save)))
                } else {
                    None
                }
            },
        };

        let is_detected = |entity: &EcsEntity, e_pos: &Pos, e_scale: Option<&Scale>| {
            self.detects_other(agent, controller, entity, e_pos, e_scale, read_data)
        };

        // Candidates without a Uid (no networked identity) can't carry a
        // stable threat_policy tiebreak and are dropped here -- in practice
        // every attackable/interactable entity is networked, so this is not
        // expected to change real selections.
        let candidates = entities_nearby
            .iter()
            .filter_map(|e| is_valid_target(*e))
            .filter_map(get_enemy)
            .filter_map(|(entity, class)| {
                let pos = get_pos(entity)?;
                let uid = read_data.uids.get(entity)?;
                Some((entity, *uid, pos, class))
            })
            .filter(|(entity, _, e_pos, _)| is_detected(entity, e_pos, read_data.scales.get(*entity)))
            .collect_vec();

        let target = pick_target_candidate(
            self.pos.0,
            candidates.iter().map(|&(_, uid, e_pos, class)| (uid, e_pos.0, class)),
        )
        .and_then(|(uid, hostile)| {
            candidates
                .iter()
                .find(|&&(_, cand_uid, _, _)| cand_uid == uid)
                .map(|&(entity, ..)| (entity, hostile))
        });

        if agent.target.is_none() && target.is_some() {
            if aggro_on {
                controller.push_utterance(UtteranceKind::Angry);
            } else {
                controller.push_utterance(UtteranceKind::Surprised);
            }
        }
        if agent.psyche.should_stop_pursuing || target.is_some() {
            agent.target = target.map(|(entity, attack_target)| Target {
                target: entity,
                hostile: attack_target,
                selected_at: read_data.time.0,
                aggro_on,
                last_known_pos: get_pos(entity).map(|pos| pos.0),
            })
        }
    }

    pub fn attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl RngExt,
    ) {
        #[cfg(any(feature = "be-dyn-lib", feature = "use-dyn-lib"))]
        let _rng = rng;

        #[cfg(not(feature = "use-dyn-lib"))]
        {
            #[cfg(not(feature = "be-dyn-lib"))]
            self.attack_inner(agent, controller, tgt_data, read_data, rng);
            #[cfg(feature = "be-dyn-lib")]
            self.attack_inner(agent, controller, tgt_data, read_data);
        }
        #[cfg(feature = "use-dyn-lib")]
        {
            let lock = LIB.lock().unwrap();
            let lib = &lock.as_ref().unwrap().lib;
            const ATTACK_FN: &[u8] = b"attack_inner\0";

            let attack_fn: common_dynlib::Symbol<
                fn(&Self, &mut Agent, &mut Controller, &TargetData, &ReadData),
            > = unsafe { lib.get(ATTACK_FN) }.unwrap_or_else(|e| {
                panic!(
                    "Trying to use: {} but had error: {:?}",
                    CStr::from_bytes_with_nul(ATTACK_FN)
                        .map(CStr::to_str)
                        .unwrap()
                        .unwrap(),
                    e
                )
            });
            attack_fn(self, agent, controller, tgt_data, read_data);
        }
    }

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "attack_inner"))]
    pub fn attack_inner(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        #[cfg(not(feature = "be-dyn-lib"))] rng: &mut impl RngExt,
    ) {
        #[cfg(feature = "be-dyn-lib")]
        let rng = &mut rng();

        self.dismount_uncontrollable(controller, read_data);

        let tool_tactic = |tool_kind| match tool_kind {
            ToolKind::Bow => Tactic::Bow,
            ToolKind::Staff => Tactic::Staff,
            ToolKind::Sceptre => Tactic::Sceptre,
            ToolKind::Hammer => Tactic::Hammer,
            ToolKind::Sword | ToolKind::Blowgun => Tactic::Sword,
            ToolKind::Axe => Tactic::Axe,
            _ => Tactic::SimpleMelee,
        };

        let tactic = self
            .inventory
            .equipped(EquipSlot::ActiveMainhand)
            .as_ref()
            .map(|item| {
                if let Some(ability_spec) = item.ability_spec() {
                    match &*ability_spec {
                        AbilitySpec::Custom(spec) => match spec.as_str() {
                            "Oni" | "Sword Simple" | "BipedLargeCultistSword" => {
                                Tactic::SwordSimple
                            },
                            "Staff Simple" | "BipedLargeCultistStaff" | "Ogre Staff" => {
                                Tactic::Staff
                            },
                            "BipedLargeCultistHammer" => Tactic::Hammer,
                            "Simple Flying Melee" => Tactic::SimpleFlyingMelee,
                            "Bow Simple" | "BipedLargeCultistBow" => Tactic::Bow,
                            "Stone Golem" | "Coral Golem" => Tactic::StoneGolem,
                            "Iron Golem" => Tactic::IronGolem,
                            "Quad Med Quick" => Tactic::CircleCharge {
                                radius: 5,
                                circle_time: 2,
                            },
                            "Quad Med Jump" | "Darkhound" => Tactic::QuadMedJump,
                            "Quad Med Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Quad Med Basic" => Tactic::QuadMedBasic,
                            "Quad Med Hoof" => Tactic::QuadMedHoof,
                            "ClaySteed" => Tactic::ClaySteed,
                            "Elephant" => Tactic::Elephant,
                            "Rocksnapper" => Tactic::Rocksnapper,
                            "Roshwalr" => Tactic::Roshwalr,
                            "Asp" | "Maneater" => Tactic::QuadLowRanged,
                            "Quad Low Breathe" | "Quad Low Beam" | "Basilisk" => {
                                Tactic::QuadLowBeam
                            },
                            "Organ" => Tactic::OrganAura,
                            "Quad Low Tail" | "Husk Brute" => Tactic::TailSlap,
                            "Quad Low Quick" => Tactic::QuadLowQuick,
                            "Quad Low Basic" => Tactic::QuadLowBasic,
                            "Theropod Basic" | "Theropod Bird" | "Theropod Small" => {
                                Tactic::Theropod
                            },
                            // Arthropods
                            "Antlion" => Tactic::ArthropodMelee,
                            "Tarantula" | "Horn Beetle" => Tactic::ArthropodAmbush,
                            "Weevil" | "Black Widow" | "Crawler" => Tactic::ArthropodRanged,
                            "Theropod Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Turret" => Tactic::RadialTurret,
                            "Flamethrower" => Tactic::RadialTurret,
                            "Haniwa Sentry" => Tactic::RotatingTurret,
                            "Bird Large Breathe" => Tactic::BirdLargeBreathe,
                            "Bird Large Fire" => Tactic::BirdLargeFire,
                            "Bird Large Basic" => Tactic::BirdLargeBasic,
                            "Flame Wyvern" | "Frost Wyvern" | "Cloud Wyvern" | "Sea Wyvern"
                            | "Weald Wyvern" => Tactic::Wyvern,
                            "Bird Medium Basic" => Tactic::BirdMediumBasic,
                            "Bushly" | "Cactid" | "Irrwurz" | "Driggle" | "Mossy Snail"
                            | "Strigoi Claws" | "Harlequin" => Tactic::SimpleDouble,
                            "Clay Golem" => Tactic::ClayGolem,
                            "Ancient Effigy" => Tactic::AncientEffigy,
                            "TerracottaStatue" | "Mogwai" => Tactic::TerracottaStatue,
                            "TerracottaBesieger" => Tactic::Bow,
                            "TerracottaDemolisher" => Tactic::SimpleDouble,
                            "TerracottaPunisher" => Tactic::SimpleMelee,
                            "TerracottaPursuer" => Tactic::SwordSimple,
                            "Cursekeeper" => Tactic::Cursekeeper,
                            "CursekeeperFake" => Tactic::CursekeeperFake,
                            "ShamanicSpirit" => Tactic::ShamanicSpirit,
                            "Jiangshi" => Tactic::Jiangshi,
                            "Mindflayer" => Tactic::Mindflayer,
                            "Flamekeeper" => Tactic::Flamekeeper,
                            "Forgemaster" => Tactic::Forgemaster,
                            "Minotaur" => Tactic::Minotaur,
                            "Cyclops" => Tactic::Cyclops,
                            "Dullahan" => Tactic::Dullahan,
                            "Grave Warden" => Tactic::GraveWarden,
                            "Tidal Warrior" => Tactic::TidalWarrior,
                            "Karkatha" => Tactic::Karkatha,
                            "Tidal Totem"
                            | "Tornado"
                            | "Gnarling Totem Red"
                            | "Gnarling Totem Green"
                            | "Gnarling Totem White" => Tactic::RadialTurret,
                            "FieryTornado" => Tactic::FieryTornado,
                            "Yeti" => Tactic::Yeti,
                            "Harvester" => Tactic::Harvester,
                            "Cardinal" => Tactic::Cardinal,
                            "Sea Bishop" => Tactic::SeaBishop,
                            "Dagon" => Tactic::Dagon,
                            "Snaretongue" => Tactic::Snaretongue,
                            "Dagonite" => Tactic::ArthropodAmbush,
                            "Gnarling Dagger" => Tactic::SimpleBackstab,
                            "Gnarling Blowgun" => Tactic::ElevatedRanged,
                            "Deadwood" => Tactic::Deadwood,
                            "Mandragora" => Tactic::Mandragora,
                            "Wood Golem" => Tactic::WoodGolem,
                            "Gnarling Chieftain" => Tactic::GnarlingChieftain,
                            "Frost Gigas" => Tactic::FrostGigas,
                            "Boreal Hammer" => Tactic::BorealHammer,
                            "Boreal Bow" => Tactic::BorealBow,
                            "Fire Gigas" => Tactic::FireGigas,
                            "Ashen Axe" => Tactic::AshenAxe,
                            "Ashen Staff" => Tactic::AshenStaff,
                            "Adlet Hunter" => Tactic::AdletHunter,
                            "Adlet Icepicker" => Tactic::AdletIcepicker,
                            "Adlet Tracker" => Tactic::AdletTracker,
                            "Hydra" => Tactic::Hydra,
                            "Ice Drake" => Tactic::IceDrake,
                            "Frostfang" => Tactic::RandomAbilities {
                                primary: 1,
                                secondary: 3,
                                abilities: [0; BASE_ABILITY_LIMIT],
                            },
                            "Tursus Claws" => Tactic::RandomAbilities {
                                primary: 2,
                                secondary: 1,
                                abilities: [4, 0, 0, 0, 0],
                            },
                            "Adlet Elder" => Tactic::AdletElder,
                            "Haniwa Soldier" => Tactic::HaniwaSoldier,
                            "Haniwa Guard" => Tactic::HaniwaGuard,
                            "Haniwa Archer" => Tactic::HaniwaArcher,
                            "Bloodmoon Bat" => Tactic::BloodmoonBat,
                            "Vampire Bat" => Tactic::VampireBat,
                            "Bloodmoon Heiress" => Tactic::BloodmoonHeiress,

                            _ => Tactic::SimpleMelee,
                        },
                        AbilitySpec::Tool(tool_kind) => tool_tactic(*tool_kind),
                    }
                } else if let ItemKind::Tool(tool) = &*item.kind() {
                    tool_tactic(tool.kind)
                } else {
                    Tactic::SimpleMelee
                }
            })
            .unwrap_or(Tactic::SimpleMelee);

        // Wield the weapon as running towards the target
        controller.push_action(ControlAction::Wield);

        // Information for attack checks
        // 'min_attack_dist' uses DEFAULT_ATTACK_RANGE, while 'body_dist' does not
        let self_radius = self.body.map_or(0.5, |b| b.max_radius()) * self.scale;
        let self_attack_range =
            (self.body.map_or(0.5, |b| b.front_radius()) + DEFAULT_ATTACK_RANGE) * self.scale;
        let tgt_radius =
            tgt_data.body.map_or(0.5, |b| b.max_radius()) * tgt_data.scale.map_or(1.0, |s| s.0);
        let min_attack_dist = self_attack_range + tgt_radius;
        let body_dist = self_radius + tgt_radius;
        let dist_sqrd = self.pos.0.distance_squared(tgt_data.pos.0);
        let angle = self
            .ori
            .look_vec()
            .angle_between(tgt_data.pos.0 - self.pos.0)
            .to_degrees();
        let angle_xy = self
            .ori
            .look_vec()
            .xy()
            .angle_between((tgt_data.pos.0 - self.pos.0).xy())
            .to_degrees();

        let eye_offset = self.body.map_or(0.0, |b| b.eye_height(self.scale));

        let tgt_eye_height = tgt_data
            .body
            .map_or(0.0, |b| b.eye_height(tgt_data.scale.map_or(1.0, |s| s.0)));
        let tgt_eye_offset = tgt_eye_height +
                   // Special case for jumping attacks to jump at the body
                   // of the target and not the ground around the target
                   // For the ranged it is to shoot at the feet and not
                   // the head to get splash damage
                   if tactic == Tactic::QuadMedJump {
                       1.0
                   } else if matches!(tactic, Tactic::QuadLowRanged) {
                       -1.0
                   } else {
                       0.0
                   };

        // FIXME:
        // 1) Retrieve actual projectile speed!
        // We have to assume projectiles are faster than base speed because there are
        // skills that increase it, and in most cases this will cause agents to
        // overshoot
        //
        // 2) We use eye_offset-s which isn't actually ideal.
        // Some attacks (beam for example) may use different offsets,
        // we should probably use offsets from corresponding states.
        //
        // 3) Should we even have this big switch?
        // Not all attacks may want their direction overwritten.
        // And this is quite hard to debug when you don't see it in actual
        // attack handler.
        if let Some(dir) = match self.char_state {
            CharacterState::ChargedRanged(c) if dist_sqrd > 0.0 => {
                let charge_factor =
                    c.timer.as_secs_f32() / c.static_data.charge_duration.as_secs_f32();
                let projectile_speed = c.static_data.initial_projectile_speed
                    + charge_factor * c.static_data.scaled_projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec(), self.scale)
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                    false,
                )
            },
            CharacterState::BasicRanged(c) => {
                let offset_z = match c.static_data.projectile.kind {
                    // Aim explosives and hazards at feet instead of eyes for splash damage
                    ProjectileConstructorKind::Explosive { .. }
                    | ProjectileConstructorKind::ExplosiveHazard { .. }
                    | ProjectileConstructorKind::Hazard { .. } => 0.0,
                    _ => tgt_eye_offset,
                };
                let projectile_speed = c.static_data.projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec(), self.scale)
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + offset_z,
                    ),
                    false,
                )
                //Correct for ability's vertical offset if present.
                //NOTE: Consider computing before controller.inputs.look_dir = dir,
                //If vertical offset is added to other abilities.
                .map(|dir| {
                    if c.static_data.vertical_angle_offset != 0.0 {
                        let cross_z = vek::Vec3::unit_z().cross(*dir).normalized();
                        Dir::from_unnormalized(
                            vek::Quaternion::rotation_3d(c.static_data.vertical_angle_offset, cross_z)
                                * *dir,
                        )
                        .unwrap_or(dir)
                    } else {
                        dir
                    }
                })
            },
            CharacterState::RapidRanged(c) => {
                let offset_z = match c.static_data.projectile.kind {
                    // Aim explosives and hazards at feet instead of eyes for splash damage
                    ProjectileConstructorKind::Explosive { .. }
                    | ProjectileConstructorKind::ExplosiveHazard { .. }
                    | ProjectileConstructorKind::Hazard { .. } => 0.0,
                    _ => tgt_eye_offset,
                };
                let projectile_speed = c.static_data.projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec(), self.scale)
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + offset_z,
                    ),
                    false,
                )
            },
            CharacterState::LeapMelee(_)
                if matches!(tactic, Tactic::Hammer | Tactic::BorealHammer | Tactic::Axe) =>
            {
                let direction_weight = match tactic {
                    Tactic::Hammer | Tactic::BorealHammer => 0.1,
                    Tactic::Axe => 0.3,
                    _ => unreachable!("Direction weight called on incorrect tactic."),
                };

                let tgt_pos = tgt_data.pos.0;
                let self_pos = self.pos.0;

                let delta_x = (tgt_pos.x - self_pos.x) * direction_weight;
                let delta_y = (tgt_pos.y - self_pos.y) * direction_weight;

                Dir::from_unnormalized(Vec3::new(delta_x, delta_y, -1.0))
            },
            CharacterState::BasicBeam(_) => {
                let aim_from = self.body.map_or(self.pos.0, |body| {
                    self.pos.0
                        + basic_beam::beam_offsets(
                            body,
                            controller.inputs.look_dir,
                            self.ori.look_vec(),
                            // Try to match animation by getting some context
                            self.vel.0 - self.physics_state.ground_vel,
                            self.physics_state.on_ground,
                        )
                });
                let aim_to = Vec3::new(
                    tgt_data.pos.0.x,
                    tgt_data.pos.0.y,
                    tgt_data.pos.0.z + tgt_eye_offset,
                );
                Dir::from_unnormalized(aim_to - aim_from)
            },
            _ => {
                let aim_from = Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset);
                let aim_to = Vec3::new(
                    tgt_data.pos.0.x,
                    tgt_data.pos.0.y,
                    tgt_data.pos.0.z + tgt_eye_offset,
                );
                Dir::from_unnormalized(aim_to - aim_from)
            },
        } {
            controller.inputs.look_dir = dir;
        }

        let attack_data = AttackData {
            body_dist,
            min_attack_dist,
            dist_sqrd,
            angle,
            angle_xy,
        };

        // Match on tactic. Each tactic has different controls depending on the distance
        // from the agent to the target.
        match tactic {
            Tactic::SimpleFlyingMelee => self.handle_simple_flying_melee(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::SimpleMelee => {
                self.handle_simple_melee(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Axe => {
                self.handle_axe_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Hammer => {
                self.handle_hammer_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Sword => {
                self.handle_sword_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Bow => {
                self.handle_bow_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Staff => {
                self.handle_staff_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Sceptre => self.handle_sceptre_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::StoneGolem => {
                self.handle_stone_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::IronGolem => {
                self.handle_iron_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::CircleCharge {
                radius,
                circle_time,
            } => self.handle_circle_charge_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                radius,
                circle_time,
                rng,
            ),
            Tactic::QuadLowRanged => self.handle_quadlow_ranged_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::TailSlap => {
                self.handle_tail_slap_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::QuadLowQuick => self.handle_quadlow_quick_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadLowBasic => self.handle_quadlow_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadMedJump => self.handle_quadmed_jump_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadMedBasic => self.handle_quadmed_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadMedHoof => self.handle_quadmed_hoof_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadLowBeam => self.handle_quadlow_beam_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Elephant => self.handle_elephant_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Rocksnapper => {
                self.handle_rocksnapper_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Roshwalr => {
                self.handle_roshwalr_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::OrganAura => {
                self.handle_organ_aura_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Theropod => {
                self.handle_theropod_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ArthropodMelee => self.handle_arthropod_melee_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::ArthropodAmbush => self.handle_arthropod_ambush_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::ArthropodRanged => self.handle_arthropod_ranged_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Turret => {
                self.handle_turret_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::FixedTurret => self.handle_fixed_turret_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::RotatingTurret => {
                self.handle_rotating_turret_attack(agent, controller, tgt_data, read_data)
            },
            Tactic::Mindflayer => self.handle_mindflayer_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Flamekeeper => {
                self.handle_flamekeeper_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Forgemaster => {
                self.handle_forgemaster_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::BirdLargeFire => self.handle_birdlarge_fire_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            // Mostly identical to BirdLargeFire but tweaked for flamethrower instead of shockwave
            Tactic::BirdLargeBreathe => self.handle_birdlarge_breathe_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BirdLargeBasic => self.handle_birdlarge_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Wyvern => {
                self.handle_wyvern_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::BirdMediumBasic => {
                self.handle_simple_melee(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::SimpleDouble => self.handle_simple_double_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Jiangshi => {
                self.handle_jiangshi_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ClayGolem => {
                self.handle_clay_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ClaySteed => {
                self.handle_clay_steed_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::AncientEffigy => self.handle_ancient_effigy_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::TerracottaStatue => {
                self.handle_terracotta_statue_attack(agent, controller, &attack_data, read_data)
            },
            Tactic::Minotaur => {
                self.handle_minotaur_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Cyclops => {
                self.handle_cyclops_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Dullahan => {
                self.handle_dullahan_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::GraveWarden => self.handle_grave_warden_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::TidalWarrior => self.handle_tidal_warrior_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Karkatha => self.handle_karkatha_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::RadialTurret => self.handle_radial_turret_attack(controller),
            Tactic::FieryTornado => self.handle_fiery_tornado_attack(agent, controller),
            Tactic::Yeti => {
                self.handle_yeti_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Harvester => self.handle_harvester_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Cardinal => self.handle_cardinal_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::SeaBishop => self.handle_sea_bishop_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Cursekeeper => self.handle_cursekeeper_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::CursekeeperFake => {
                self.handle_cursekeeper_fake_attack(controller, &attack_data)
            },
            Tactic::ShamanicSpirit => self.handle_shamanic_spirit_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Dagon => {
                self.handle_dagon_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Snaretongue => {
                self.handle_snaretongue_attack(agent, controller, &attack_data, read_data)
            },
            Tactic::SimpleBackstab => {
                self.handle_simple_backstab(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ElevatedRanged => {
                self.handle_elevated_ranged(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Deadwood => {
                self.handle_deadwood(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Mandragora => {
                self.handle_mandragora(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::WoodGolem => {
                self.handle_wood_golem(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::GnarlingChieftain => self.handle_gnarling_chieftain(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::FrostGigas => self.handle_frostgigas_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BorealHammer => self.handle_boreal_hammer_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BorealBow => self.handle_boreal_bow_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::FireGigas => self.handle_firegigas_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::AshenAxe => self.handle_ashen_axe_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::AshenStaff => self.handle_ashen_staff_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::SwordSimple => self.handle_sword_simple_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::AdletHunter => {
                self.handle_adlet_hunter(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::AdletIcepicker => {
                self.handle_adlet_icepicker(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::AdletTracker => {
                self.handle_adlet_tracker(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::IceDrake => {
                self.handle_icedrake(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Hydra => {
                self.handle_hydra(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::BloodmoonBat => self.handle_bloodmoon_bat_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::VampireBat => self.handle_vampire_bat_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BloodmoonHeiress => self.handle_bloodmoon_heiress_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::RandomAbilities {
                primary,
                secondary,
                abilities,
            } => self.handle_random_abilities(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
                primary,
                secondary,
                abilities,
            ),
            Tactic::AdletElder => {
                self.handle_adlet_elder(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::HaniwaSoldier => {
                self.handle_haniwa_soldier(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::HaniwaGuard => {
                self.handle_haniwa_guard(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::HaniwaArcher => {
                self.handle_haniwa_archer(agent, controller, &attack_data, tgt_data, read_data)
            },
        }
    }

    pub fn handle_sounds_heard(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        emitters: &mut AgentEmitters,
        rng: &mut impl RngExt,
    ) {
        agent.forget_old_sounds(read_data.time.0);

        if is_invulnerable(*self.entity, read_data) || is_steering(*self.entity, read_data) {
            self.idle(agent, controller, read_data, emitters, rng);
            return;
        }

        if let Some(sound) = agent.sounds_heard.last() {
            let sound_pos = Pos(sound.pos);
            let dist_sqrd = self.pos.0.distance_squared(sound_pos.0);
            // NOTE: There is an implicit distance requirement given that sound volume
            // dissipates as it travels, but we will not want to flee if a sound is super
            // loud but heard from a great distance, regardless of how loud it was.
            // `is_close` is this limiter.
            let is_close = dist_sqrd < 35.0_f32.powi(2);

            let sound_was_loud = sound.vol >= 10.0;
            let sound_was_threatening = sound_was_loud
                || matches!(sound.kind, SoundKind::Utterance(UtteranceKind::Scream, _));

            let has_enemy_alignment = matches!(self.alignment, Some(Alignment::Enemy));
            let follows_threatening_sounds =
                has_enemy_alignment || is_village_guard(*self.entity, read_data);

            if sound_was_threatening && is_close {
                if !self.below_flee_health(agent) && follows_threatening_sounds {
                    self.follow(agent, controller, read_data, &sound_pos);
                } else if self.below_flee_health(agent) || !follows_threatening_sounds {
                    self.flee(agent, controller, read_data, &sound_pos);
                } else {
                    self.idle(agent, controller, read_data, emitters, rng);
                }
            } else {
                self.idle(agent, controller, read_data, emitters, rng);
            }
        } else {
            self.idle(agent, controller, read_data, emitters, rng);
        }
    }

    pub fn attack_target_attacker(
        &self,
        agent: &mut Agent,
        read_data: &ReadData,
        controller: &mut Controller,
        emitters: &mut AgentEmitters,
        rng: &mut impl RngExt,
    ) {
        if let Some(Target { target, .. }) = agent.target
            && let Some(tgt_health) = read_data.healths.get(target)
            && let Some(by) = tgt_health.last_change.damage_by()
            && let Some(attacker) = get_entity_by_id(by.uid(), read_data)
        {
            if agent.target.is_none() {
                controller.push_utterance(UtteranceKind::Angry);
            }

            let attacker_pos = read_data.positions.get(attacker).map(|pos| pos.0);
            agent.target = Some(Target::new(
                attacker,
                true,
                read_data.time.0,
                true,
                attacker_pos,
            ));

            if let Some(tgt_pos) = read_data.positions.get(attacker) {
                if is_dead_or_invulnerable(attacker, read_data) {
                    agent.target = Some(Target::new(
                        target,
                        false,
                        read_data.time.0,
                        false,
                        Some(tgt_pos.0),
                    ));

                    self.idle(agent, controller, read_data, emitters, rng);
                } else {
                    let target_data = TargetData::new(tgt_pos, target, read_data);
                    // TODO: Reimplement this in rtsim
                    // if let Some(tgt_name) =
                    //     read_data.stats.get(target).map(|stats| stats.name.clone())
                    // {
                    //     agent.add_fight_to_memory(&tgt_name, read_data.time.0)
                    // }
                    self.attack(agent, controller, &target_data, read_data, rng);
                }
            }
        }
    }

    // TODO: Pass a localisation key instead of `Content` to avoid allocating if
    // we're not permitted to speak.
    pub fn chat_npc_if_allowed_to_speak(
        &self,
        msg: Content,
        agent: &Agent,
        emitters: &mut AgentEmitters,
    ) -> bool {
        if agent.allowed_to_speak() {
            self.chat_npc(msg, emitters);
            true
        } else {
            false
        }
    }

    pub fn chat_npc(&self, content: Content, emitters: &mut AgentEmitters) {
        emitters.emit(ChatEvent {
            msg: UnresolvedChatMsg::npc(*self.uid, content),
            from_client: false,
        });
    }

    fn emit_scream(&self, time: f64, emitters: &mut AgentEmitters) {
        if let Some(body) = self.body {
            emitters.emit(SoundEvent {
                sound: Sound::new(
                    SoundKind::Utterance(UtteranceKind::Scream, *body),
                    self.pos.0,
                    13.0,
                    time,
                ),
            });
        }
    }

    pub fn cry_out(&self, agent: &Agent, emitters: &mut AgentEmitters, read_data: &ReadData) {
        let has_enemy_alignment = matches!(self.alignment, Some(Alignment::Enemy));
        let is_below_flee_health = self.below_flee_health(agent);

        if has_enemy_alignment && is_below_flee_health {
            self.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-cultist_low_health_fleeing"),
                agent,
                emitters,
            );
        } else if is_villager(self.alignment) {
            self.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-villager_under_attack"),
                agent,
                emitters,
            );
            self.emit_scream(read_data.time.0, emitters);
        }
    }

    pub fn exclaim_relief_about_enemy_dead(&self, agent: &Agent, emitters: &mut AgentEmitters) {
        if is_villager(self.alignment) {
            self.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-villager_enemy_killed"),
                agent,
                emitters,
            );
        }
    }

    pub fn below_flee_health(&self, agent: &Agent) -> bool {
        self.damage.min(1.0) < agent.psyche.flee_health
    }

    /// `T3.35+T3.39` (E3-WT, Fable-ruled 2026-07-27): delegates to the
    /// shared [`common::threat_policy`] instead of its own ad-hoc fuzzy-
    /// distance comparison (which had gone dead: it compared
    /// `target_pos.distance(entity_pos)` against itself scaled by 0.8,
    /// never `entity`'s own distance, so the "closer entity" branch could
    /// never fire once `target.aggro_on` was set — disclosed old-vs-new
    /// delta, not silently carried forward). `entity` (the newly-detected
    /// attacker) is unconditionally `AttackingMe` — this comparator is
    /// only ever called from `target_if_attacked`, where `entity` just
    /// dealt us real damage, which is definitionally the `AttackingMe`
    /// signal itself (no separate alignment-hostility gate needed here,
    /// unlike the old code's redundant check). `target` gets `AttackingMe`
    /// too when `aggro_on` (already engaged, so score/proximity decides)
    /// or `HostileNearby` otherwise (so `entity` always wins by class,
    /// matching the old `!target.aggro_on` unconditional-switch branch).
    /// `capability_vs_me`/`recency` are 0.0 for both sides: no such signal
    /// existed in the old comparator either, so this is parity, not a
    /// regression.
    pub fn is_more_dangerous_than_target(
        &self,
        entity: EcsEntity,
        target: Target,
        read_data: &ReadData,
    ) -> bool {
        let Some(entity_pos) = read_data.positions.get(entity) else {
            return false;
        };
        let Some(target_pos) = read_data.positions.get(target.target) else {
            return true;
        };
        let (Some(entity_uid), Some(target_uid)) = (
            read_data.uids.get(entity),
            read_data.uids.get(target.target),
        ) else {
            // No stable tiebreak identity available; fall back to the old
            // unconditional-switch-unless-engaged rule.
            return !target.aggro_on;
        };

        threat_switch_decision(
            self.pos.0,
            entity_pos.0,
            *entity_uid,
            target_pos.0,
            *target_uid,
            target.aggro_on,
        )
    }

    pub fn is_enemy(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        let other_alignment = read_data.alignments.get(entity);

        (entity != *self.entity)
            && !self.passive_towards(entity, read_data)
            && (are_our_owners_hostile(self.alignment, other_alignment, read_data)
                || (is_villager(self.alignment) && is_dressed_as_cultist(entity, read_data)
                    || (is_villager(self.alignment) && is_dressed_as_witch(entity, read_data))
                    || (is_villager(self.alignment) && is_dressed_as_pirate(entity, read_data))))
    }

    pub fn is_hunting_animal(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        (entity != *self.entity)
            && !self.friendly_towards(entity, read_data)
            && matches!(read_data.bodies.get(entity), Some(Body::QuadrupedSmall(_)))
    }

    fn should_defend(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        let entity_alignment = read_data.alignments.get(entity);

        let we_are_friendly = entity_alignment.is_some_and(|entity_alignment| {
            self.alignment
                .is_some_and(|alignment| !alignment.hostile_towards(*entity_alignment))
        });
        let we_share_species = read_data.bodies.get(entity).is_some_and(|entity_body| {
            self.body.is_some_and(|body| {
                entity_body.is_same_species_as(body)
                    || (entity_body.is_humanoid() && body.is_humanoid())
            })
        });
        let self_owns_entity =
            matches!(entity_alignment, Some(Alignment::Owned(ouid)) if *self.uid == *ouid);

        (we_are_friendly && we_share_species)
            || (is_village_guard(*self.entity, read_data) && is_villager(entity_alignment))
            || self_owns_entity
    }

    fn passive_towards(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        if let (Some(self_alignment), Some(other_alignment)) =
            (self.alignment, read_data.alignments.get(entity))
        {
            self_alignment.passive_towards(*other_alignment)
        } else {
            false
        }
    }

    fn friendly_towards(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        if let (Some(self_alignment), Some(other_alignment)) =
            (self.alignment, read_data.alignments.get(entity))
        {
            self_alignment.friendly_towards(*other_alignment)
        } else {
            false
        }
    }

    pub fn can_see_entity(
        &self,
        agent: &Agent,
        controller: &Controller,
        other: EcsEntity,
        other_pos: &Pos,
        other_scale: Option<&Scale>,
        read_data: &ReadData,
    ) -> bool {
        let other_stealth_multiplier = {
            let other_inventory = read_data.inventories.get(other);
            let other_char_state = read_data.char_states.get(other);

            perception_dist_multiplier_from_stealth(other_inventory, other_char_state, self.msm)
        };

        let within_sight_dist = {
            let sight_dist = agent.psyche.sight_dist * other_stealth_multiplier;
            let dist_sqrd = other_pos.0.distance_squared(self.pos.0);

            dist_sqrd < sight_dist.powi(2)
        };

        let within_fov = (other_pos.0 - self.pos.0)
            .try_normalized()
            .is_some_and(|v| v.dot(*controller.inputs.look_dir) > 0.15);

        let other_body = read_data.bodies.get(other);

        (within_sight_dist)
            && within_fov
            && entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                other_pos,
                other_body,
                other_scale,
                read_data,
            )
    }

    pub fn detects_other(
        &self,
        agent: &Agent,
        controller: &Controller,
        other: &EcsEntity,
        other_pos: &Pos,
        other_scale: Option<&Scale>,
        read_data: &ReadData,
    ) -> bool {
        self.can_sense_directly_near(other_pos, *other, read_data)
            || self.can_see_entity(agent, controller, *other, other_pos, other_scale, read_data)
    }

    pub fn can_sense_directly_near(
        &self,
        e_pos: &Pos,
        other: EcsEntity,
        read_data: &ReadData,
    ) -> bool {
        e_pos.0.distance_squared(self.pos.0) < 5_f32.powi(2) && self.senses_directly(other, read_data)
    }

    /// DET-AIT-002: the 0.3 "senses a nearby entity directly" gate, as a
    /// STATELESS keyed decision instead of a shared-cursor RNG draw. The old
    /// `helper_random_bool(0.3)` advanced a per-agent RNG cursor, so the
    /// outcome for THIS (observer, candidate) pair depended on how many other
    /// detection checks were evaluated first this tick — an order-coupling
    /// across unrelated candidates in an authoritative code path. Derive the
    /// gate purely from certified inputs (tick, observer uid, candidate uid)
    /// so each pair's decision is independent of evaluation order and
    /// idempotent within a tick. ARCH-003's `helper_rng` stream is retained
    /// for its other (non-authoritative) users.
    fn senses_directly(&self, other: EcsEntity, read_data: &ReadData) -> bool {
        let candidate = read_data.uids.get(other).map_or(0, |u| u.0.get());
        let mut h = common::state_hash::DomainHasher::new(
            "bastion/domain/agent-sense-directly/v1/sha256",
        );
        h.field(&read_data.time.0.to_bits().to_le_bytes());
        h.field(&self.uid.0.get().to_le_bytes());
        h.field(&candidate.to_le_bytes());
        let draw = u64::from_le_bytes(h.finish().0[..8].try_into().expect("sha256 >= 8 bytes"));
        // 0.3 probability gate over the uniform u64 draw:
        //   floor(0.3 * 2^64) = 5_534_023_222_112_865_484
        draw < 5_534_023_222_112_865_484
    }

    /// Draw from the deterministic per-agent helper stream in harness mode,
    /// otherwise preserve the existing live OS entropy.
    fn helper_random_bool(&self, probability: f64) -> bool {
        self.helper_rng.borrow_mut().as_mut().map_or_else(
            || rng().random_bool(probability),
            |rng| rng.random_bool(probability),
        )
    }

    pub fn menacing(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        target: EcsEntity,
        tgt_data: &TargetData,
        read_data: &ReadData,
        emitters: &mut AgentEmitters,
        remembers_fight_with_target: bool,
    ) {
        let max_move = 0.5;
        let move_dir = controller.inputs.move_dir;
        let move_dir_mag = move_dir.magnitude();
        let mut chat = |agent: &mut Agent, content: Content| {
            self.chat_npc_if_allowed_to_speak(content, agent, emitters);
        };
        let mut chat_villager_remembers_fighting = |agent: &mut Agent| {
            let tgt_name = read_data.stats.get(target).map(|stats| stats.name.clone());

            // TODO: Localise
            // Is this thing even used??
            if let Some(tgt_name) = tgt_name.as_ref().and_then(|name| name.as_plain()) {
                chat(
                    agent,
                    Content::localized_with_args("npc-speech-remembers-fight", [(
                        "name", tgt_name,
                    )]),
                )
            } else {
                chat(
                    agent,
                    Content::localized("npc-speech-remembers-fight-no-name"),
                );
            }
        };

        self.look_toward(controller, read_data, target);
        controller.push_action(ControlAction::Wield);

        if move_dir_mag > max_move {
            controller.inputs.move_dir = max_move * move_dir / move_dir_mag;
        }

        match agent
            .timer
            .timeout_elapsed(read_data.time.0, comp::agent::TimerAction::Warn, 5.0)
        {
            Some(true) | None => {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::AtTarget,
                    Some(0.4),
                );
            },
            Some(false) => {
                agent
                    .timer
                    .start(read_data.time.0, comp::agent::TimerAction::Warn);
                controller.push_utterance(UtteranceKind::Angry);
                if is_villager(self.alignment) {
                    if remembers_fight_with_target {
                        chat_villager_remembers_fighting(agent);
                    } else if is_dressed_as_cultist(target, read_data) {
                        chat(
                            agent,
                            Content::localized("npc-speech-villager_cultist_alarm"),
                        );
                    } else if is_dressed_as_witch(target, read_data) {
                        chat(agent, Content::localized("npc-speech-villager_witch_alarm"));
                    } else if is_dressed_as_pirate(target, read_data) {
                        chat(
                            agent,
                            Content::localized("npc-speech-villager_pirate_alarm"),
                        );
                    } else {
                        chat(agent, Content::localized("npc-speech-menacing"));
                    }
                } else {
                    chat(agent, Content::localized("npc-speech-menacing"));
                }
            },
        }
    }

    /// Dismount if riding something the agent can't control.
    pub fn dismount_uncontrollable(&self, controller: &mut Controller, read_data: &ReadData) {
        if read_data.is_riders.get(*self.entity).is_some_and(|mount| {
            read_data
                .id_maps
                .uid_entity(mount.mount)
                .and_then(|e| read_data.bodies.get(e))
                .is_none_or(|b| b.has_free_will())
        }) || read_data
            .is_volume_riders
            .get(*self.entity)
            .is_some_and(|r| !r.is_steering_entity())
        {
            controller.push_event(ControlEvent::Unmount);
        }
    }

    /// Dismount if riding something.
    ///
    /// Currently there's an exception for if the agent is steering a volume
    /// entity.
    pub fn dismount(&self, controller: &mut Controller, read_data: &ReadData) {
        if read_data.is_riders.contains(*self.entity)
            || read_data
                .is_volume_riders
                .get(*self.entity)
                .is_some_and(|r| !r.is_steering_entity())
        {
            controller.push_event(ControlEvent::Unmount);
        }
    }
}

/// `T3.35+T3.39` (E3-WT): pure core of
/// [`AgentData::is_more_dangerous_than_target`], extracted so it's
/// unit-testable without constructing an ECS `ReadData` (mirrors the
/// `loot_attempt_decision` pattern below). `self_pos` is the deciding
/// agent's own position; `entity`/`target` are the two candidates.
fn threat_switch_decision(
    self_pos: Vec3<f32>,
    entity_pos: Vec3<f32>,
    entity_uid: Uid,
    target_pos: Vec3<f32>,
    target_uid: Uid,
    target_aggro_on: bool,
) -> bool {
    let entity_candidate = ThreatCandidateV1 {
        class: ThreatClassV1::AttackingMe,
        distance: entity_pos.distance(self_pos),
        capability_vs_me: 0.0,
        recency: 0.0,
        tiebreak: entity_uid,
    };
    let target_candidate = ThreatCandidateV1 {
        class: if target_aggro_on {
            ThreatClassV1::AttackingMe
        } else {
            ThreatClassV1::HostileNearby
        },
        distance: target_pos.distance(self_pos),
        capability_vs_me: 0.0,
        recency: 0.0,
        tiebreak: target_uid,
    };

    arbitrate(&[target_candidate, entity_candidate]) == Some(1)
}

/// `T3.35+T3.39` (E3-WT): pure core of
/// [`AgentData::choose_target`]'s candidate selection, generic over
/// whichever `Ord + Copy` identity the caller has on hand (real call site
/// uses `Uid`; tests use plain integers) so it's unit-testable without an
/// ECS `ReadData`. `class: None` is a non-combat interaction target (item
/// pickup / a downed ally to save) — those keep first priority, tie-broken
/// by plain distance, unchanged from the pre-E3-WT behavior (a help-vs-
/// fight priority, not a threat ranking). Only when no non-combat
/// candidate exists does the combat bucket (`class: Some(_)`) get ranked
/// via `threat_policy::arbitrate`.
fn pick_target_candidate<T: Ord + Copy>(
    self_pos: Vec3<f32>,
    candidates: impl IntoIterator<Item = (T, Vec3<f32>, Option<ThreatClassV1>)>,
) -> Option<(T, bool)> {
    let candidates = candidates.into_iter().collect_vec();

    if let Some(&(id, ..)) = candidates
        .iter()
        .filter(|(_, _, class)| class.is_none())
        .min_by_key(|(_, pos, _)| (pos.distance_squared(self_pos) * 100.0) as i32)
    {
        return Some((id, false));
    }

    let threat_candidates = candidates
        .iter()
        .filter_map(|&(id, pos, class)| {
            class.map(|class| ThreatCandidateV1 {
                class,
                distance: pos.distance(self_pos),
                capability_vs_me: 0.0,
                recency: 0.0,
                tiebreak: id,
            })
        })
        .collect_vec();
    arbitrate(&threat_candidates).map(|i| (threat_candidates[i].tiebreak, true))
}

// T3.35+T3.39 (E3-WT): non-vacuity for pick_target_candidate's bucketing +
// threat_policy wiring.
#[cfg(test)]
mod pick_target_candidate_tests {
    use super::{ThreatClassV1, pick_target_candidate};
    use vek::Vec3;

    fn pos(x: f32) -> Vec3<f32> { Vec3::new(x, 0.0, 0.0) }

    #[test]
    fn no_candidates_is_none() {
        let candidates: Vec<(u32, Vec3<f32>, Option<ThreatClassV1>)> = Vec::new();
        assert_eq!(pick_target_candidate(Vec3::zero(), candidates), None);
    }

    /// A non-combat candidate is preferred over ANY combat candidate,
    /// however close the combat one is -- the tuple-ordering behavior of
    /// the old bare-bool `min_by_key` (`false < true` dominates the
    /// distance term), now expressed as an explicit two-phase pick.
    #[test]
    fn non_combat_wins_over_a_much_closer_combat_candidate() {
        let far_pickup = (1u32, pos(20.0), None);
        let close_enemy = (2u32, pos(1.0), Some(ThreatClassV1::HostileNearby));
        assert_eq!(
            pick_target_candidate(Vec3::zero(), [far_pickup, close_enemy]),
            Some((1, false))
        );
    }

    /// Among non-combat candidates, plain distance still decides (parity
    /// with the pre-row behavior).
    #[test]
    fn non_combat_bucket_picks_the_closer_one() {
        let far = (1u32, pos(20.0), None);
        let near = (2u32, pos(1.0), None);
        assert_eq!(pick_target_candidate(Vec3::zero(), [far, near]), Some((2, false)));
    }

    /// No non-combat candidate: `AttackingAlly` outranks `HostileNearby`
    /// regardless of distance -- the class distinction the old bare bool
    /// couldn't express (both were the same `true`).
    #[test]
    fn attacking_ally_outranks_a_much_closer_hostile_nearby() {
        let close_hostile = (1u32, pos(1.0), Some(ThreatClassV1::HostileNearby));
        let far_defends_ally = (2u32, pos(20.0), Some(ThreatClassV1::AttackingAlly));
        assert_eq!(
            pick_target_candidate(Vec3::zero(), [close_hostile, far_defends_ally]),
            Some((2, true))
        );
    }

    /// Within the same combat class, real proximity decides (previously
    /// this call site had none at all -- pure distance-squared min, which
    /// is preserved here as the in-class score).
    #[test]
    fn same_class_combat_candidates_rank_by_proximity() {
        let far = (1u32, pos(20.0), Some(ThreatClassV1::HostileNearby));
        let near = (2u32, pos(1.0), Some(ThreatClassV1::HostileNearby));
        assert_eq!(pick_target_candidate(Vec3::zero(), [far, near]), Some((2, true)));
    }
}

// T3.35+T3.39 (E3-WT): non-vacuity for the threat_policy wiring — proves
// the class ordering actually drives the switch decision, not just that
// it compiles.
#[cfg(test)]
mod threat_switch_decision_tests {
    use super::threat_switch_decision;
    use common::uid::Uid;
    use std::num::NonZeroU64;
    use vek::Vec3;

    fn uid(n: u64) -> Uid { Uid(NonZeroU64::new(n).unwrap()) }

    /// Not yet aggro'd: the attacker (always `AttackingMe`) must win by
    /// class alone even when it is much farther away than the current
    /// (merely `HostileNearby`) target — the old code's unconditional
    /// `!target.aggro_on` branch, now expressed as class precedence.
    #[test]
    fn unengaged_target_always_loses_to_a_farther_attacker() {
        let self_pos = Vec3::new(0.0, 0.0, 0.0);
        let far_attacker = Vec3::new(50.0, 0.0, 0.0);
        let near_target = Vec3::new(1.0, 0.0, 0.0);
        assert!(threat_switch_decision(
            self_pos,
            far_attacker,
            uid(1),
            near_target,
            uid(2),
            false,
        ));
    }

    /// Already engaged (`aggro_on`): both candidates are `AttackingMe`, so
    /// the fixed-weight score (pure proximity here) decides — closer
    /// attacker wins. This is the dead-fuzzy-comparison bug's fix: the old
    /// comparator could never switch in this branch at all.
    #[test]
    fn engaged_target_loses_to_a_closer_attacker() {
        let self_pos = Vec3::new(0.0, 0.0, 0.0);
        let close_attacker = Vec3::new(1.0, 0.0, 0.0);
        let far_target = Vec3::new(10.0, 0.0, 0.0);
        assert!(threat_switch_decision(
            self_pos,
            close_attacker,
            uid(1),
            far_target,
            uid(2),
            true,
        ));
    }

    /// Already engaged, attacker is farther than the current target: keep
    /// the current target (class tie, score favors the incumbent).
    #[test]
    fn engaged_target_keeps_a_closer_incumbent_over_a_farther_attacker() {
        let self_pos = Vec3::new(0.0, 0.0, 0.0);
        let far_attacker = Vec3::new(10.0, 0.0, 0.0);
        let close_target = Vec3::new(1.0, 0.0, 0.0);
        assert!(!threat_switch_decision(
            self_pos,
            far_attacker,
            uid(1),
            close_target,
            uid(2),
            true,
        ));
    }

    /// Exact distance tie while engaged: resolved by `Uid`'s own order
    /// (higher wins, per `threat_policy::compare`'s tiebreak), not by
    /// argument position — order-independence pinned directly.
    #[test]
    fn engaged_exact_tie_resolves_by_uid_order_independently_of_argument_position() {
        let self_pos = Vec3::new(0.0, 0.0, 0.0);
        let pos_a = Vec3::new(5.0, 0.0, 0.0);
        let pos_b = Vec3::new(5.0, 0.0, 0.0);
        // Higher uid (7) as the attacker: attacker wins.
        assert!(threat_switch_decision(self_pos, pos_a, uid(7), pos_b, uid(3), true));
        // Higher uid (7) as the target: attacker (lower uid) loses.
        assert!(!threat_switch_decision(self_pos, pos_a, uid(3), pos_b, uid(7), true));
    }
}

// bastion ENGINE-OPT-3 (ledger #160) truth-table pins.
#[cfg(test)]
mod loot_auth_tests {
    use super::loot_attempt_decision;

    /// The pre-fix inline predicate, transcribed VERBATIM in shape (outer
    /// negation + inverted hostility) — the falsifier's reference. Kept so
    /// the flipped rows below remain executable documentation of the bug.
    fn old_inverted_decision(
        wants_pickup: bool,
        is_humanoid: bool,
        authorized: bool,
        is_soft: bool,
        hostile_to_owner: bool,
    ) -> bool {
        wants_pickup && !(is_humanoid && authorized && (!is_soft || !hostile_to_owner))
    }

    #[test]
    fn item_160_intended_truth_table() {
        // wants nothing -> never.
        assert!(!loot_attempt_decision(false, true, false, false));
        // entitled (own/group drop, hard ownership) -> attempt.
        assert!(loot_attempt_decision(true, true, false, false));
        // foreign hard-owned, unauthorized -> no attempt.
        assert!(!loot_attempt_decision(true, false, false, false));
        // soft wish, not hostile -> respect the wish.
        assert!(!loot_attempt_decision(true, true, true, false));
        // soft wish, hostile to the owner -> ignore the wish, attempt.
        assert!(loot_attempt_decision(true, true, true, true));
    }

    #[test]
    fn item_160_falsifier_old_predicate_flips_the_load_bearing_rows() {
        // ROW 1 — entitled humanoid (owns the drop, hard): intended ATTEMPT,
        // old REFUSED (the refused-entitled-loot half of the bug).
        assert!(loot_attempt_decision(true, true, false, false));
        assert!(!old_inverted_decision(true, true, true, false, false));
        // ROW 2 — foreign hard-owned, unauthorized humanoid: intended NO,
        // old ATTEMPTED (the attempt-spam half; commit gate then rejected).
        assert!(!loot_attempt_decision(true, false, false, false));
        assert!(old_inverted_decision(true, true, false, false, false));
        // ROW 3 — soft + hostile: intended ATTEMPT (ignore an enemy's
        // wish) — and the OLD predicate ALSO attempted here: its two
        // inversions CANCELLED on this branch (the outer `!` undoing the
        // flipped hostility), which is precisely why the bug survived
        // review — some branches accidentally behaved correctly.
        assert!(loot_attempt_decision(true, true, true, true));
        assert!(old_inverted_decision(true, true, true, true, true));
        // ROW 3b — soft + NOT hostile: both respect the wish (cancelled
        // branch too; documented for completeness).
        assert!(!loot_attempt_decision(true, true, true, false));
        assert!(!old_inverted_decision(true, true, true, true, false));
        // ROW 4 — non-humanoid on owned loot: can_pickup's design says
        // authorized=true (non-humanoids ignore ownership); intended follows
        // the same single path (attempt unless soft-and-not-hostile) — the
        // old predicate's `is_humanoid` guard made non-humanoids attempt
        // UNCONDITIONALLY, even against a soft wish they weren't hostile to.
        assert!(!loot_attempt_decision(true, true, true, false));
        assert!(old_inverted_decision(true, false, true, true, false));
    }
}

// T0.7: the named rates are EXACT inverses of the old per-tick constants at
// 30 tps, and the hazard form is dt-invariant (two half-ticks compound to
// one full tick) — pinned so a rate edit can't silently change today's
// behavior and a cadence change can't distort it.
#[cfg(test)]
mod t0_7_tests {
    #[test]
    fn t0_7_rates_reproduce_old_per_tick_probabilities_at_30_tps() {
        for (rate, old_p) in [
            (super::UNWIELD_IDLE_RATE, 0.1),
            (super::HUNT_RETARGET_RATE, 0.1),
            (super::LANTERN_TOGGLE_RATE, 0.001),
            (super::PET_DISMOUNT_RATE, 0.0001),
            (super::PET_MOUNT_RATE, 0.01),
            (super::IDLE_UTTERANCE_RATE, 0.0015),
            (super::IDLE_SIT_RATE, 0.0035),
        ] {
            let p = 1.0 - (1.0 - rate).powf(1.0 / 30.0);
            assert!(
                (p - old_p).abs() < 1e-12,
                "rate {rate} reproduces {p}, expected {old_p}"
            );
        }
        let half = 1.0 - (1.0 - super::UNWIELD_IDLE_RATE).powf(0.5 / 30.0);
        let compound = 1.0 - (1.0 - half) * (1.0 - half);
        let full = 1.0 - (1.0 - super::UNWIELD_IDLE_RATE).powf(1.0 / 30.0);
        assert!((compound - full).abs() < 1e-12, "hazard must compound");
    }
}
