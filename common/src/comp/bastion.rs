//! bastion (Project Bastion): ECS marker components for the overseer
//! interaction surface (B2a).

use serde::{Deserialize, Serialize};
use specs::{Component, NullStorage};

/// Marks the entity currently selected by the overseer (client-side; at most
/// a handful at once). Drives the inspection HUD and feeds the B1.6 cutaway
/// targets, replacing that block's focus+debug-marker stubs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionSelected;

impl Component for BastionSelected {
    type Storage = NullStorage<Self>;
}

/// A colony member (B3): the ECS mirror of the rtsim-side
/// [`crate::bastion::BastionColonist`], attached when the NPC promotes to a
/// loaded entity. Synced to clients (overhead markers, box-select, roster).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colonist(pub crate::bastion::BastionColonist);

impl Component for Colonist {
    // Synced to clients → needs change-tracked storage.
    type Storage = specs::DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

/// Ownership tag: this entity belongs to THE player colony. Server-side only;
/// B2b's God-mode target restriction reads it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerColony;

impl Component for PlayerColony {
    type Storage = NullStorage<Self>;
}

/// Need clocks, 1.0 = fully satisfied, 0.0 = starved/exhausted/miserable.
/// Attached in B3; decay + satisfaction behavior land in B7.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub rest: f32,
    pub recreation: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 1.0,
            rest: 1.0,
            recreation: 1.0,
        }
    }
}

impl Component for Needs {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Mood aggregate, 0.0 (breakdown) ..= 1.0 (content). B7 feeds it.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mood(pub f32);

impl Default for Mood {
    fn default() -> Self { Self(0.6) }
}

impl Component for Mood {
    type Storage = specs::DenseVecStorage<Self>;
}

/// bastion (B7-0, row 44): decay all three meters by `dt` game-seconds,
/// saturating at 0.0. Pure; the caller owns the cadence (per-tick,
/// dt-scaled — decay is rate × time, cadence-independent).
pub fn decay_needs(needs: &mut Needs, dt: f32, cfg: &crate::bastion::MoodConfig) {
    needs.hunger = (needs.hunger - cfg.hunger.decay_per_sec * dt).max(0.0);
    needs.rest = (needs.rest - cfg.rest.decay_per_sec * dt).max(0.0);
    needs.recreation =
        (needs.recreation - cfg.recreation.decay_per_sec * dt).max(0.0);
}

/// bastion (B7-0): a need's penalty basis — nonzero only BELOW the
/// comfort band, so a topped-up colonist is unperturbed and a starving
/// one is heavily penalized. Continuous (mood tracks pressure smoothly).
pub fn shortfall(value: f32, comfort: f32) -> f32 {
    (comfort - value).max(0.0)
}

/// bastion (B7-0): a thought's decayed contribution — linear to zero
/// over its lifetime, a PURE function of `(deposit_time, now)` (no
/// per-tick state, no drift; the determinism house invariant).
pub fn thought_decay(magnitude: f32, deposit: f64, now: f64, lifetime: f64) -> f32 {
    if lifetime <= 0.0 {
        return 0.0;
    }
    let age = (now - deposit).max(0.0);
    if age >= lifetime {
        0.0
    } else {
        magnitude * (1.0 - age / lifetime) as f32
    }
}

/// bastion (B7-0): THE mood formula (design §3 — RimWorld's base+Σ,
/// named prior art): `clamp01(base + Σ w_need·shortfall(need) +
/// thought_sum)`. Order-free (addition commutes); RECOMPUTED each
/// cadence, never integrated across ticks (no float accumulation). The
/// thought term arrives summed (the server owns the chronicle query —
/// the kind table keys on rtsim's `ChronicleKind`, which common cannot
/// see; the formula is layering-agnostic by taking the sum).
pub fn mood_formula(
    cfg: &crate::bastion::MoodConfig,
    needs: &Needs,
    thought_sum: f32,
) -> f32 {
    (cfg.mood_base
        + cfg.hunger.weight * shortfall(needs.hunger, cfg.hunger.comfort)
        + cfg.rest.weight * shortfall(needs.rest, cfg.rest.comfort)
        + cfg.recreation.weight
            * shortfall(needs.recreation, cfg.recreation.comfort)
        + thought_sum)
        .clamp(0.0, 1.0)
}

/// bastion (B-AG3 slice 1): the care multiplier is CLAMPED — a stack of
/// scorned values can mute a thought to a quarter, never erase it; a
/// stack of held values can quadruple it, never explode it.
pub const CARE_MIN: f32 = 0.25;
pub const CARE_MAX: f32 = 4.0;
/// bastion (B-AG3 slice 1): a Neurotic colonist (vanilla Big-Five trait,
/// public `Personality::is` API) feels NEGATIVE thoughts half again as
/// hard — the one temperament term this slice consumes (DF/RimWorld's
/// standard neuroticism→bad-thought amplification).
pub const NEUROTIC_NEGATIVE_AMP: f32 = 1.5;

/// bastion (B-AG3 slice 1): how much THIS colonist cares about one
/// thought — the personalized multiplier on the thought's table weight.
/// `values` is the colonist's ±50 weight map; `affinities` is the
/// thought-kind's `(Value, affinity)` row (the ChronicleKind→Value table
/// lives server-side — this is the pure math, layering-agnostic exactly
/// like [`mood_formula`]'s summed thought term). Empty values OR an
/// empty affinity row → exactly 1.0 (+ the neurotic amp if applicable):
/// the pre-B-AG3 formula for unvalued colonists, bit-for-bit when
/// non-neurotic. PURE — no state, no rng; two colonists differing only
/// in `values` produce different multipliers from the SAME thought (the
/// slice's whole point).
pub fn care_factor(
    values: &std::collections::HashMap<crate::bastion::Value, i8>,
    affinities: &[(crate::bastion::Value, f32)],
    neurotic: bool,
    base_weight: f32,
) -> f32 {
    let mut care = 1.0f32;
    for (value, affinity) in affinities {
        if let Some(w) = values.get(value) {
            care += (f32::from(*w) / 50.0) * affinity;
        }
    }
    let care = care.clamp(CARE_MIN, CARE_MAX);
    // The amp applies AFTER the clamp (a maxed-care neurotic feels a bad
    // thought at 6.0×, bounded) and only to negative thoughts — good
    // news is not amplified by anxiety.
    if neurotic && base_weight < 0.0 {
        care * NEUROTIC_NEGATIVE_AMP
    } else {
        care
    }
}

#[cfg(test)]
mod bastion_b70_tests {
    use super::*;

    /// B7-0's formula pinned: topped-up == base exactly; the fully
    /// starved case matches the hand-computed value; decay arithmetic is
    /// exact and saturates; thought decay is linear-pure; clamp holds.
    #[test]
    fn bastion_mood_formula_exact() {
        let cfg = crate::bastion::MoodConfig::default();
        let full = Needs::default();
        assert_eq!(mood_formula(&cfg, &full, 0.0), cfg.mood_base);
        // Fully starved: clamp01(0.6 − 0.5·0.5 − 0.4·0.5 − 0.15·0.4)
        // = clamp01(0.6 − 0.25 − 0.2 − 0.06) = 0.09.
        let starved = Needs {
            hunger: 0.0,
            rest: 0.0,
            recreation: 0.0,
        };
        assert!((mood_formula(&cfg, &starved, 0.0) - 0.09).abs() < 1e-6);
        // A big negative thought clamps at 0, a big positive at 1.
        assert_eq!(mood_formula(&cfg, &starved, -5.0), 0.0);
        assert_eq!(mood_formula(&cfg, &full, 5.0), 1.0);
        // Decay: exact rate × time, saturating at 0.
        let mut n = Needs::default();
        decay_needs(&mut n, 100.0, &cfg);
        assert!((n.hunger - (1.0 - 0.04)).abs() < 1e-6);
        assert!((n.rest - (1.0 - 0.03)).abs() < 1e-6);
        assert!((n.recreation - (1.0 - 0.02)).abs() < 1e-6);
        decay_needs(&mut n, 1.0e9, &cfg);
        assert_eq!((n.hunger, n.rest, n.recreation), (0.0, 0.0, 0.0));
        // Thought decay: full at age 0, half at half-life, zero past.
        assert!((thought_decay(-0.15, 0.0, 0.0, 100.0) + 0.15).abs() < 1e-6);
        assert!((thought_decay(-0.15, 0.0, 50.0, 100.0) + 0.075).abs() < 1e-6);
        assert_eq!(thought_decay(-0.15, 0.0, 100.0, 100.0), 0.0);
        assert_eq!(thought_decay(-0.15, 0.0, 500.0, 100.0), 0.0);
    }

    /// B-AG3 slice 1: the care multiplier pinned — identity for the
    /// unvalued; DIVERGENT for two colonists with different value maps on
    /// the SAME affinity row (the block's done-when in pure form); exact
    /// arithmetic at the ±50 scale; clamped both ways; the neurotic amp
    /// hits negative thoughts only, after the clamp.
    #[test]
    fn bastion_care_factor_exact() {
        use crate::bastion::Value;
        use std::collections::HashMap;
        let empty: HashMap<Value, i8> = HashMap::new();
        let row = [(Value::Kin, 0.6f32), (Value::Glory, -0.4)];
        // Identity: no values, or no affinity row -> exactly 1.0.
        assert_eq!(care_factor(&empty, &row, false, -0.15), 1.0);
        let mut kin = HashMap::new();
        kin.insert(Value::Kin, 50i8);
        assert_eq!(care_factor(&kin, &[], false, -0.15), 1.0);
        // DIVERGENCE: same row, two different value maps.
        let mut glory = HashMap::new();
        glory.insert(Value::Glory, 50i8);
        let care_kin = care_factor(&kin, &row, false, -0.15);
        let care_glory = care_factor(&glory, &row, false, -0.15);
        assert!((care_kin - 1.6).abs() < 1e-6); // 1 + (50/50)·0.6
        assert!((care_glory - 0.6).abs() < 1e-6); // 1 + (50/50)·(−0.4)
        assert!(care_kin > care_glory);
        // Scorn: a negative weight flips the affinity's direction.
        let mut scorns_kin = HashMap::new();
        scorns_kin.insert(Value::Kin, -50i8);
        assert!((care_factor(&scorns_kin, &row, false, -0.15) - 0.4).abs() < 1e-6);
        // Clamps: stacked scorn floors at CARE_MIN, stacked zeal caps at
        // CARE_MAX.
        let big_row = [(Value::Kin, 5.0f32)];
        assert_eq!(care_factor(&kin, &big_row, false, -0.15), CARE_MAX);
        let neg_row = [(Value::Kin, -5.0f32)];
        assert_eq!(care_factor(&kin, &neg_row, false, -0.15), CARE_MIN);
        // Neurotic: ×1.5 on NEGATIVE thoughts only, applied post-clamp.
        assert!(
            (care_factor(&kin, &row, true, -0.15) - 1.6 * NEUROTIC_NEGATIVE_AMP)
                .abs()
                < 1e-6
        );
        assert!((care_factor(&kin, &row, true, 0.15) - 1.6).abs() < 1e-6);
        assert_eq!(
            care_factor(&kin, &big_row, true, -0.15),
            CARE_MAX * NEUROTIC_NEGATIVE_AMP
        );
    }
}

/// The colonist's current job assignment (B4). Server-side only; the job
/// system owns the colonist's rtsim-controller activity while this exists.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveJob {
    pub job: crate::bastion::JobId,
    pub state: ActiveJobState,
    /// Travel watchdog: best distance-to-target achieved so far + time since
    /// it last improved. Displacement alone is useless — an agent pacing
    /// around an unreachable target moves plenty without progressing.
    pub best_dist: f32,
    pub stuck_time: f32,
    /// bastion (B-LIVE3, reviewer R3 fix-1 — stuck-time HYSTERESIS): the
    /// distance at the last stuck_time ZERO. The accumulator only resets
    /// on ≥1 block of NET progress since then, so sub-block jitter (magnet
    /// nudges, hover bobbing, physics wobble — all ≥ the 0.5 EPSILON)
    /// can't starve the watchdog forever; real walking (2+ blocks/s)
    /// resets comfortably. Without this, a hovering colonist generated
    /// ZERO timeouts → zero churn → no net ever fired.
    #[serde(default)]
    pub reset_dist: f32,
    /// bastion (B6 SOFT-0): this stall already got its soft-collision
    /// GRACE WINDOW (SOFT-COLLISION-design §0 trigger a). The watchdog
    /// grants soft-pass ONCE per assignment before degrading to the
    /// carve/unreachable pipeline — most chokepoint deadlocks clear in
    /// the grace; a still-stuck soft colonist is genuinely blocked.
    #[serde(default)]
    pub soft_granted: bool,
    /// bastion (B15 / reviewer FR12): the committed work-STANCE — the feet-cell
    /// OFFSET from `job.pos` where the colonist stands to work the block.
    /// (0,0,1) = ON-TOP (stand on the block; the default, = the pre-B15
    /// `job.pos + (0.5,0.5,1.0)` arrive-target). A cardinal `(±1,0,0)`/`(0,±1,0)`
    /// = an ADJACENT-ground stance (stand beside + mine sideways — the fix for
    /// hillside `+1`-arrival-gap cells whose on-top stance is a 1-wide slot the
    /// capsule can't occupy). PINNED at claim by the once-per-cycle
    /// standability pass, NOT re-picked each tick (avoids re-introducing the R3
    /// steer oscillation). Server-only; the serde default is inert (never
    /// deserialized — every insert sets it explicitly).
    #[serde(default)]
    pub stance: vek::Vec3<i32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveJobState {
    /// Walking to the job site.
    Traveling,
    /// At the site, ready to work (B5 hooks here).
    Arrived,
    /// bastion (B6, reviewer R3 fix-2): queued at a single-file vertical
    /// link — another colonist is closer to the staged access anchor, so
    /// this one WAITS ITS TURN. The watchdog skips Waiting entirely (no
    /// stall accrual, no unreachable, no strikes, no churn — queue-waiting
    /// is not stuckness); promotion back to Traveling happens every
    /// arbitration pass, which re-evaluates the queue order. Emergent
    /// single-file: nearest climbs, the rest hold.
    Waiting,
}

impl Component for ActiveJob {
    type Storage = specs::DenseVecStorage<Self>;
}

/// The god-mode anchor marker (§4 standing directive): while the overseer is
/// active, the player's avatar entity carries this — the world must ignore it
/// (no targeting/aggro/greeting/pushback) and it must be invulnerable (the
/// server also applies a permanent `Invulnerability` buff). Removed on F9 /
/// anchor clear; mortality applies only under Embody (B12).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionGodAnchor;

impl Component for BastionGodAnchor {
    type Storage = NullStorage<Self>;
}

/// bastion (B-ASSET1): a direct movement order for test fixtures — the
/// colonist walks to `target` through the vanilla agent (the same
/// `NpcActivity::Goto` mechanism job travel uses) with the same 3D-arrival +
/// progress-watchdog semantics. Server-side only; inert unless inserted
/// (harness `--asset-test` and `--asset-arena` fixtures). Mutually exclusive
/// with [`ActiveJob`] by convention (the hook that inserts it refuses
/// job-holding colonists).
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BastionTestGoto {
    pub target: vek::Vec3<f32>,
    /// Travel watchdog (same scheme as [`ActiveJob`]): best distance achieved
    /// so far + time since it last improved.
    pub best_dist: f32,
    pub stuck_time: f32,
    /// Sim seconds spent on this order (arrival-budget accounting).
    pub elapsed: f32,
    pub arrived: bool,
    /// The watchdog gave up: no progress within the stuck timeout.
    pub stuck: bool,
}

impl BastionTestGoto {
    pub fn new(target: vek::Vec3<f32>) -> Self {
        Self {
            target,
            best_dist: f32::INFINITY,
            stuck_time: 0.0,
            elapsed: 0.0,
            arrived: false,
            stuck: false,
        }
    }
}

impl Component for BastionTestGoto {
    type Storage = specs::DenseVecStorage<Self>;
}

/// A persistent colonist-produced item pile (B5.5). Entities carrying this:
/// never get a despawn timer (colonist output is a player resource — item
/// loss is an invariant violation), aggregate freely with each other via the
/// vanilla merge machinery, and NEVER merge across class with timed vanilla
/// drops (a pile merging into a timed drop would inherit its despawn — a
/// silent-loss path). Server-side only.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BastionPile;

impl Component for BastionPile {
    type Storage = NullStorage<Self>;
}
