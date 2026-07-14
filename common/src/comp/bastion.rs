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

/// bastion (AUTON-0, row 48): the arbiter's drive — WHAT a colonist's
/// autonomy layer has decided it is doing. Utility-AI shape (The Sims/
/// RimWorld prior art per the packet): score → pick max → COMMIT.
/// Self-jobs (RestAt/EatFrom/Despond) are deliberately NOT a variant —
/// they are an exempt occupancy the arbiter steps around (GUARD 6: B7
/// keeps sole authority for that colonist until the self-job completes;
/// the full unification is AUTON-2's job). Work carries no JobId — the
/// ActiveJob comp IS the work handle (no dual source of truth).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drive {
    Work,
    Flee,
    Idle,
}

/// bastion (AUTON-0): the per-colonist arbiter state — the current
/// drive, the same-tier commitment deadline (anti-thrash hysteresis;
/// higher-tier Flee preemption ignores it per-tick), and the last
/// scored urgencies (work, flee, idle) as REPORTED telemetry.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arbiter {
    pub current: Drive,
    pub committed_until: f64,
    pub last_scores: (f32, f32, f32),
}

impl Default for Arbiter {
    fn default() -> Self {
        Self {
            current: Drive::Idle,
            committed_until: 0.0,
            last_scores: (0.0, 0.0, 0.0),
        }
    }
}

impl Component for Arbiter {
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
/// bastion (FOCUS-0-DERIVE, row 43.1): the derived per-colonist NEED
/// WEIGHT — how much THIS colonist's mind makes them care about each
/// personal [`crate::bastion::Need`], from their rolled [`crate::bastion::Value`]
/// weights + vanilla Big-Five traits. Baseline 1.0; a value-mapped need
/// scales 1 + weight/50 (so ±50 spans 0..2); `Socialize` reads the
/// boolean trait API at 3 levels (Extroverted/Sociable 1.5, Introverted
/// 0.5, else 1.0 — the architect's no-vanilla-getter ruling);
/// `Drink`/`AdmireArt`/`Learn` have no clean correlate and STAY 1.0
/// (the design's degrade-gracefully law — no forced weak mapping, no
/// invented Value). Clamped 0..=2. PURE — a FOCUS-1 scorer eventually
/// consumes this; nothing does yet (this block produces + proves only).
pub fn derive_need_weight(
    need: crate::bastion::Need,
    personality: &crate::rtsim::Personality,
    values: &std::collections::HashMap<crate::bastion::Value, i8>,
) -> f32 {
    use crate::bastion::{Need, Value};
    use crate::rtsim::PersonalityTrait;
    let from_value = |v: Value| -> f32 {
        1.0 + values.get(&v).copied().map_or(0.0, |w| f32::from(w) / 50.0)
    };
    let w = match need {
        // The near-1:1 vocabulary correspondences (the mapping is the
        // enums' own design — Pray↔Piety, Family↔Kin, Craft↔Craft,
        // SeeAnimals↔Nature, Acquire↔Wealth, Fight↔Glory).
        Need::Pray => from_value(Value::Piety),
        Need::Family => from_value(Value::Kin),
        Need::Craft => from_value(Value::Craft),
        Need::SeeAnimals => from_value(Value::Nature),
        Need::Acquire => from_value(Value::Wealth),
        Need::Fight => from_value(Value::Glory),
        // Temperament-derived: the boolean-trait 3-level.
        Need::Socialize => {
            if personality.is(PersonalityTrait::Extroverted)
                || personality.is(PersonalityTrait::Sociable)
            {
                1.5
            } else if personality.is(PersonalityTrait::Introverted) {
                0.5
            } else {
                1.0
            }
        },
        // No clean correlate — baseline (degrade gracefully; never
        // force a weak mapping).
        Need::Drink | Need::AdmireArt | Need::Learn => 1.0,
    };
    w.clamp(0.0, 2.0)
}

/// bastion (AUTON-2, row 50): the preempt-threshold SAFETY FLOOR — even
/// the hardiest possible colonist keeps a live preempt-to-eat edge above
/// zero (Opus's hard guard: the stagger WIDENS the recoverable band, it
/// never disables B7-2's backstop). At hunger decay 0.0004/s a 0.05
/// threshold still leaves ~2 sim-minutes of margin before empty.
pub const INTERRUPT_FLOOR: f32 = 0.05;

/// bastion (AUTON-2, row 50): the TRAIT-STAGGER — one colonist's
/// EFFECTIVE preempt threshold for a need (the per-colonist form of
/// `NeedTuning.interrupt`, the [`care_factor`] modulation pattern).
/// Dutiful/hardy colonists (Craft/Tradition-valuing, Conscientious)
/// tolerate a DEEPER deficit before abandoning work (lower threshold);
/// anxious ones (Neurotic, anti-valuing) preempt EARLIER (higher). The
/// spread is the death-spiral defense: a shortage never yanks the whole
/// crew off the farm at once. Hardiness h ∈ [−1.5, +1.5] (values ±0.5
/// each + Conscientious +0.5 / Neurotic −0.5); eff = base·(1 − 0.4·h),
/// clamped to [`INTERRUPT_FLOOR`-floored, base×1.5]. The `.min(base)`
/// on the floor keeps a base of 0.0 (recreation: never-preempts) at
/// exactly 0.0 — the stagger cannot INVENT a preempt class — and keeps
/// the clamp well-formed if a RON retunes base below the floor. The
/// ceiling base×1.5 (0.3 at the 0.2 default) stays under the 0.5
/// comfort band: nobody preempts while comfortable. PURE + RNG-free
/// (field reads only — the determinism house invariant).
pub fn stagger_interrupt(
    base: f32,
    values: &std::collections::HashMap<crate::bastion::Value, i8>,
    conscientious: bool,
    neurotic: bool,
) -> f32 {
    use crate::bastion::Value;
    let mut h = 0.0f32;
    for v in [Value::Craft, Value::Tradition] {
        if let Some(w) = values.get(&v) {
            h += f32::from(*w) / 100.0;
        }
    }
    if conscientious {
        h += 0.5;
    }
    if neurotic {
        h -= 0.5;
    }
    (base * (1.0 - 0.4 * h)).clamp(INTERRUPT_FLOOR.min(base), base * 1.5)
}

/// bastion (AUTON-3, row 51): the DRIVE-ORDER guard — Flee's modulated
/// urgency can never sink below this, and the floor sits strictly above
/// Work's modulated CEILING (0.6), so the AUTON-0 safety ordering
/// (Flee > Work > Idle) survives EVERY possible trait roll. Baked in now
/// so B8's live threats inherit a correct invariant, not a landmine.
pub const FLEE_URGENCY_FLOOR: f32 = 0.8;

/// bastion (AUTON-3, row 51): TRAIT-MODULATED drive urgencies — the E2
/// legibility mechanism: two colonists in the SAME state score their
/// drives differently because of WHO THEY ARE. Distinct from AUTON-2's
/// threshold-stagger (when a need becomes urgent); this shapes which
/// drive WINS the arbiter's pick. One value + one personality pair per
/// axis, mirror-simple (the spec's own examples):
/// - WORK  × (1 + 0.4·g), g = Wealth/100 ∈ [−0.5, 0.5] → [0.4, 0.6]
///   (the greedy work harder; ceiling 0.6 < the Flee floor).
/// - FLEE  × (1 − 0.2·b), b = Glory/100 + 0.25·Adventurous −
///   0.25·Worried ∈ [−0.75, 0.75] → [0.85, 1.15], then `.max(floor)`
///   (glory-seekers stand their ground longer — but NEVER below the
///   order guard; bravest possible = 0.85 > 0.8 floor > 0.6 ceiling).
/// - IDLE  × (1 + 0.4·s), s = Kin/100 + 0.25·(Sociable∨Extroverted) −
///   0.25·Introverted ∈ [−0.75, 0.75] → [0.07, 0.13] (the social idle
///   richer; ceiling 0.13 < Work's floor 0.4 — work-when-available
///   still always wins, the AUTON-0 liveness contract).
/// Returns (work, flee, idle) — the `Arbiter.last_scores` order. PURE +
/// RNG-free (field reads only — the determinism house invariant).
#[expect(clippy::too_many_arguments)]
pub fn modulated_urgencies(
    base: (f32, f32, f32),
    values: &std::collections::HashMap<crate::bastion::Value, i8>,
    adventurous: bool,
    worried: bool,
    sociable: bool,
    introverted: bool,
) -> (f32, f32, f32) {
    use crate::bastion::Value;
    let vw = |v: Value| -> f32 {
        values.get(&v).copied().map_or(0.0, |w| f32::from(w) / 100.0)
    };
    let g = vw(Value::Wealth);
    let b = vw(Value::Glory)
        + if adventurous { 0.25 } else { 0.0 }
        - if worried { 0.25 } else { 0.0 };
    let s = vw(Value::Kin)
        + if sociable { 0.25 } else { 0.0 }
        - if introverted { 0.25 } else { 0.0 };
    (
        base.0 * (1.0 + 0.4 * g),
        (base.1 * (1.0 - 0.2 * b)).max(FLEE_URGENCY_FLOOR.min(base.1)),
        base.2 * (1.0 + 0.4 * s),
    )
}

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

    /// AUTON-3: the drive-order guard pinned. THE FLEE-FLOOR ASSERT
    /// (the packet's load-bearing guard, unit form): the BRAVEST
    /// possible roll (Glory +50, Adventurous, not Worried → b = 0.75)
    /// still scores Flee at 0.85 — above the 0.8 floor and strictly
    /// above the GREEDIEST possible Work ceiling (Wealth +50 → 0.6):
    /// the AUTON-0 ordering survives every trait combination. Plus:
    /// identity (no traits = bases exactly), the zero-preservation
    /// guard (a no-signal flee base of 0.0 stays 0.0 — modulation can
    /// not invent a flee), and Idle's ceiling (most-social 0.13) under
    /// Work's floor (least-greedy 0.4) — work-when-available always
    /// wins.
    #[test]
    fn auton3_drive_order_guard() {
        use crate::bastion::Value;
        use std::collections::HashMap;
        let base = (0.5f32, 1.0f32, 0.1f32);
        let none = HashMap::new();
        // Identity.
        assert_eq!(
            modulated_urgencies(base, &none, false, false, false, false),
            base
        );
        // Bravest Flee vs greediest Work — the order guard, exact.
        let mut brave = HashMap::new();
        brave.insert(Value::Glory, 50i8);
        let (_, flee_min, _) =
            modulated_urgencies(base, &brave, true, false, false, false);
        let mut greedy = HashMap::new();
        greedy.insert(Value::Wealth, 50i8);
        let (work_max, _, _) =
            modulated_urgencies(base, &greedy, false, false, false, false);
        assert!((flee_min - 0.85).abs() < 1e-6);
        assert!((work_max - 0.6).abs() < 1e-6);
        assert!(flee_min > work_max);
        assert!(flee_min >= FLEE_URGENCY_FLOOR);
        // Zero-preservation: no flee signal (base 0.0) stays 0.0 even
        // for the most fearful roll (modulation cannot INVENT a flee).
        let mut fearful = HashMap::new();
        fearful.insert(Value::Glory, -50i8);
        let (_, f0, _) = modulated_urgencies(
            (0.5, 0.0, 0.1),
            &fearful,
            false,
            true,
            false,
            false,
        );
        assert_eq!(f0, 0.0);
        // Idle ceiling < Work floor: the liveness contract.
        let mut social = HashMap::new();
        social.insert(Value::Kin, 50i8);
        let (_, _, idle_max) =
            modulated_urgencies(base, &social, false, false, true, false);
        let mut lazy_poor = HashMap::new();
        lazy_poor.insert(Value::Wealth, -50i8);
        let (work_min, _, _) = modulated_urgencies(
            base, &lazy_poor, false, false, false, false,
        );
        assert!((idle_max - 0.13).abs() < 1e-6);
        assert!((work_min - 0.4).abs() < 1e-6);
        assert!(idle_max < work_min);
    }

    /// AUTON-2: the trait-stagger pinned. THE OPUS FLOOR ASSERT (unit
    /// form): the hardiest POSSIBLE colonist (both values +50,
    /// Conscientious, not Neurotic → h = 1.5) still holds a strictly
    /// positive threshold at/above the floor — the preempt-to-eat
    /// backstop survives maximal hardiness. Plus: identity (no traits =
    /// base exactly), monotonicity (hardier ⇒ never higher), the
    /// anxious ceiling (< comfort), and recreation's 0.0 stays 0.0 (the
    /// stagger cannot invent a preempt class).
    #[test]
    fn auton2_stagger_interrupt_floor_and_shape() {
        use crate::bastion::Value;
        use std::collections::HashMap;
        let base = 0.2f32;
        let mut hardiest = HashMap::new();
        hardiest.insert(Value::Craft, 50i8);
        hardiest.insert(Value::Tradition, 50i8);
        let floor_case = stagger_interrupt(base, &hardiest, true, false);
        assert!(floor_case >= INTERRUPT_FLOOR);
        assert!(floor_case > 0.0);
        // h = 1.5 → 0.2 × (1 − 0.6) = 0.08 exactly.
        assert!((floor_case - 0.08).abs() < 1e-6);
        // Identity: empty values, no traits → base bit-for-bit.
        let none = HashMap::new();
        assert_eq!(stagger_interrupt(base, &none, false, false), base);
        // Monotone: each hardiness step never RAISES the threshold.
        let mut mid = HashMap::new();
        mid.insert(Value::Craft, 50i8);
        let steps = [
            stagger_interrupt(base, &none, false, true), // anxious
            stagger_interrupt(base, &none, false, false),
            stagger_interrupt(base, &mid, false, false),
            stagger_interrupt(base, &hardiest, false, false),
            stagger_interrupt(base, &hardiest, true, false),
        ];
        for w in steps.windows(2) {
            assert!(w[1] <= w[0]);
        }
        // The anxious ceiling: h = −1.5 → 0.2×1.6 = 0.32, clamped to
        // base×1.5 = 0.3 — still under the 0.5 comfort band.
        let mut anti = HashMap::new();
        anti.insert(Value::Craft, -50i8);
        anti.insert(Value::Tradition, -50i8);
        let anxious = stagger_interrupt(base, &anti, false, true);
        assert!((anxious - 0.3).abs() < 1e-6);
        assert!(anxious < 0.5);
        // Recreation's never-preempt base survives every temperament.
        assert_eq!(stagger_interrupt(0.0, &hardiest, true, false), 0.0);
        assert_eq!(stagger_interrupt(0.0, &anti, false, true), 0.0);
    }

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

    /// FOCUS-0-DERIVE (43.1): the derivation pinned — value-mapped needs
    /// scale 1 + weight/50 exactly (±50 spans 0..2); unmapped needs sit
    /// at baseline regardless of values; Socialize's 3-level trait gate
    /// is consistent with the public `.is()` API over a seeded
    /// personality sample, and both extremes occur in the sample.
    #[test]
    fn bastion_derive_need_weight_exact() {
        use crate::bastion::{Need, Value};
        use crate::rtsim::{Personality, PersonalityTrait};
        use rand::SeedableRng;
        use std::collections::HashMap;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xF0C0_5D34);
        let neutral = Personality::random(&mut rng);
        // Value arms: exact linear map, empty = baseline.
        let empty: HashMap<Value, i8> = HashMap::new();
        assert_eq!(derive_need_weight(Need::Pray, &neutral, &empty), 1.0);
        let mut v = HashMap::new();
        v.insert(Value::Piety, 50i8);
        v.insert(Value::Kin, -50);
        v.insert(Value::Wealth, 25);
        assert_eq!(derive_need_weight(Need::Pray, &neutral, &v), 2.0);
        assert_eq!(derive_need_weight(Need::Family, &neutral, &v), 0.0);
        assert!((derive_need_weight(Need::Acquire, &neutral, &v) - 1.5).abs() < 1e-6);
        // Unmapped needs: baseline even with a loud value map.
        assert_eq!(derive_need_weight(Need::Drink, &neutral, &v), 1.0);
        assert_eq!(derive_need_weight(Need::AdmireArt, &neutral, &v), 1.0);
        assert_eq!(derive_need_weight(Need::Learn, &neutral, &v), 1.0);
        // Socialize: 3-level, consistent with the public trait API; a
        // 400-draw seeded sample contains both extremes.
        let (mut saw_high, mut saw_low) = (false, false);
        for _ in 0..400 {
            let p = Personality::random(&mut rng);
            let w = derive_need_weight(Need::Socialize, &p, &empty);
            let expect = if p.is(PersonalityTrait::Extroverted)
                || p.is(PersonalityTrait::Sociable)
            {
                1.5
            } else if p.is(PersonalityTrait::Introverted) {
                0.5
            } else {
                1.0
            };
            assert_eq!(w, expect);
            saw_high |= w == 1.5;
            saw_low |= w == 0.5;
        }
        assert!(saw_high && saw_low, "seeded sample must span the gate");
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
