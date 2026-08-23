//! bastion (B7-0, row 44): the server-side half of the mood pipeline —
//! the THOUGHT TABLE and the chronicle query that turns remembered
//! events into the formula's summed thought term.
//!
//! Lives server-side by dependency direction: the table keys on
//! [`rtsim::data::ChronicleKind`], which `common` cannot see — so
//! `common`'s [`common::comp::bastion::mood_formula`] takes the summed
//! term as a plain input, and this module owns producing it (the server
//! sees both crates). v1 ships the seed set the design names (a `Death`
//! thought; `CaveIn` pending the fork ruling); more thoughts are PURE
//! DATA added to the RON as their emitter events land (HIST-1) — the
//! formula never reshapes.

use common::assets::{self, AssetExt, BoxedError, FileAsset, load_ron};
use serde::Deserialize;
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

/// Thought tuning: [`rtsim::data::ChronicleKind`] → (signed magnitude,
/// lifetime in game-seconds). An event only weighs on a colonist whose
/// [`common::rtsim::Actor`] appears in the entry's `actors`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ThoughtTable {
    pub thoughts: HashMap<rtsim::data::ChronicleKind, (f32, f64)>,
}

impl FileAsset for ThoughtTable {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> { load_ron(&bytes) }
}

impl ThoughtTable {
    /// The loaded table (hot-reloadable); an EMPTY table on a
    /// missing/broken asset — graceful: no thoughts weigh, needs still
    /// drive mood, nothing panics.
    pub fn current() -> Self {
        Self::load("common.bastion_thoughts")
            .map(|h| h.read().clone())
            .unwrap_or_default()
    }
}

/// bastion (B-AG3 slice 1): [`rtsim::data::ChronicleKind`] → the
/// `(Value, affinity)` row the care multiplier reads — how much holding
/// each value changes caring about that kind of event. Same server-side
/// home as [`ThoughtTable`] (keys on `ChronicleKind`, invisible to
/// `common`); same graceful-empty semantics (a kind with no row = care
/// 1.0 = the pre-B-AG3 weighting). PURE DATA — rows join as thought
/// emitters land, the math never reshapes.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ValueAffinityTable {
    pub affinities: HashMap<rtsim::data::ChronicleKind, Vec<(common::bastion::Value, f32)>>,
}

impl FileAsset for ValueAffinityTable {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> { load_ron(&bytes) }
}

impl ValueAffinityTable {
    /// The loaded table (hot-reloadable); EMPTY on missing/broken —
    /// every care factor is 1.0 and mood behaves exactly as before.
    pub fn current() -> Self {
        Self::load("common.bastion_value_affinities")
            .map(|h| h.read().clone())
            .unwrap_or_default()
    }
}

/// The summed thought term for one colonist: every chronicle entry whose
/// `actors` names them and whose kind the table maps, decayed by
/// [`common::comp::bastion::thought_decay`] (pure `(deposit, now)` — no
/// per-tick state), and — B-AG3 slice 1 — scaled by THIS colonist's
/// [`common::comp::bastion::care_factor`] (their ±50 value weights ×
/// the kind's affinity row, with the Neurotic negative amp). Empty
/// values/rows = care 1.0 = the B7-0 sum bit-for-bit (non-neurotic).
/// Order-free (addition commutes) — the determinism house invariant,
/// same as the formula it feeds.
/// ★ SAME-KIND THOUGHTS STACK WITH DIMINISHING RETURNS, CAPPED (the
/// nap-bliss engine, narrator sweep 2026-08-23): a colonist who napped all
/// evening carried 25→51 SleptInBed thoughts (+0.08 × one-day life each) —
/// an unbounded +2.0 of bliss that clamped mood to 1.0000 THROUGH hunger
/// 0.0000. Four colonists starved beaming beside a full pantry because the
/// pile out-shouted every need penalty; the same mechanism runs the 1.0→0.0
/// cliff in reverse with stacked fear. RimWorld's answer, adopted: the n-th
/// same-kind thought counts at STACK_FACTOR^(n-1), and beyond MAX_STACK it
/// counts ZERO. Max same-kind contribution = mag × (1 + 0.75 + 0.5625) ≈
/// 2.31 × mag — a good night's sleep still warms you (+0.18), it just can't
/// anaesthetise starvation (−0.25). Applied identically in `thought_sum`
/// and `thought_contributions` via this one function, so the diagnostic can
/// never disagree with the number that ships.
pub const THOUGHT_MAX_STACK: u32 = 3;
pub const THOUGHT_STACK_FACTOR: f32 = 0.75;

pub fn stack_multiplier(occurrence_index: u32) -> f32 {
    // occurrence_index is 0-based: first of a kind = 1.0, second = 0.75,
    // third = 0.5625, fourth and beyond = 0 (not merely small — a cap that
    // only shrinks still grows without bound over a long evening).
    if occurrence_index >= THOUGHT_MAX_STACK {
        0.0
    } else {
        THOUGHT_STACK_FACTOR.powi(occurrence_index as i32)
    }
}

pub fn thought_sum(
    chronicle: &rtsim::data::Chronicle,
    table: &ThoughtTable,
    affinity_table: &ValueAffinityTable,
    actor: common::rtsim::Actor,
    now: f64,
    values: &BTreeMap<common::bastion::Value, i8>,
    neurotic: bool,
) -> f32 {
    // T0.40 (T0-003): the event sequence is already stable (chronicle
    // append order); the ACCUMULATION is f64 with Neumaier compensation so
    // long chronicles cannot drift mood through f32 rounding.
    let mut sum = 0.0f64;
    let mut compensation = 0.0f64;
    // Per-kind occurrence counter for the stacking cap — BTreeMap and
    // chronicle append order, so which occurrences survive the cap is
    // deterministic (DET-MOOD-003's discipline).
    let mut stacks: std::collections::BTreeMap<rtsim::data::ChronicleKind, u32> =
        std::collections::BTreeMap::new();
    for term in chronicle
        .events()
        .filter(|e| e.actors.contains(&actor))
        .filter_map(|e| {
            table.thoughts.get(&e.kind).map(|(mag, life)| {
                let occ = stacks.entry(e.kind).or_insert(0);
                let mult = stack_multiplier(*occ);
                *occ += 1;
                let care = common::comp::bastion::care_factor(
                    values,
                    affinity_table
                        .affinities
                        .get(&e.kind)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    neurotic,
                    *mag,
                );
                mult * care
                    * common::comp::bastion::thought_decay(*mag, e.at_tod.0, now, *life)
            })
        })
    {
        let term = f64::from(term);
        let tentative = sum + term;
        compensation += if sum.abs() >= term.abs() {
            (sum - tentative) + term
        } else {
            (term - tentative) + sum
        };
        sum = tentative;
    }
    (sum + compensation) as f32
}

/// engine-list T3.54: [`thought_sum`]'s per-thought breakdown — same
/// filter/care/decay per term, kept individual instead of folded into one
/// compensated sum. Diagnostic only: callers must still call
/// [`thought_sum`] for the number that actually drives
/// [`common::comp::bastion::mood_formula`] (this function's own
/// unweighted `.sum()` is not compensated and must never stand in for
/// it). `thought_id` is [`rtsim::data::ChronicleKind`]'s declaration-order
/// discriminant — stable as long as the enum's declared order doesn't
/// change, the same invariant `Ord` on the kind already relies on
/// (DET-MOOD-003).
pub fn thought_contributions(
    chronicle: &rtsim::data::Chronicle,
    table: &ThoughtTable,
    affinity_table: &ValueAffinityTable,
    actor: common::rtsim::Actor,
    now: f64,
    values: &BTreeMap<common::bastion::Value, i8>,
    neurotic: bool,
) -> Vec<common::comp::bastion::ThoughtContributionV1> {
    // The SAME stacking cap as `thought_sum`, same order — a diagnostic
    // that showed uncapped rows beside a capped total would re-open the
    // exact confusion the narrator hit (thoughts=51 next to a number they
    // cannot reach).
    let mut stacks: std::collections::BTreeMap<rtsim::data::ChronicleKind, u32> =
        std::collections::BTreeMap::new();
    chronicle
        .events()
        .filter(|e| e.actors.contains(&actor))
        .filter_map(|e| {
            table.thoughts.get(&e.kind).map(|(mag, life)| {
                let occ = stacks.entry(e.kind).or_insert(0);
                let mult = stack_multiplier(*occ);
                *occ += 1;
                let care = common::comp::bastion::care_factor(
                    values,
                    affinity_table
                        .affinities
                        .get(&e.kind)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    neurotic,
                    *mag,
                );
                let contribution = mult
                    * care
                    * common::comp::bastion::thought_decay(*mag, e.at_tod.0, now, *life);
                common::comp::bastion::ThoughtContributionV1 {
                    source_event_id: e.seq,
                    thought_id: e.kind as u32,
                    base_magnitude: *mag,
                    care_multiplier: care,
                    contribution,
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod stack_cap_tests {
    use super::*;
    use rtsim::data::{Chronicle, ChronicleKind, Importance, Scope};

    /// ★ THE NAP-BLISS ENGINE, pinned shut both ways (narrator sweep,
    /// 2026-08-23: 51 stacked SleptInBed thoughts held mood at 1.0000
    /// through hunger 0.0000). Direction one: an evening of naps cannot
    /// out-shout an empty stomach. Direction two: the cap must not delete
    /// feeling — one good sleep still warms, and DISTINCT kinds all count
    /// in full (diversity is not stacking).
    #[test]
    fn an_evening_of_naps_cannot_anaesthetise_starvation() {
        // The multiplier ladder itself.
        assert_eq!(stack_multiplier(0), 1.0);
        assert_eq!(stack_multiplier(1), 0.75);
        assert_eq!(stack_multiplier(2), 0.5625);
        assert_eq!(stack_multiplier(3), 0.0, "the 4th same-kind thought counts ZERO");
        assert_eq!(stack_multiplier(50), 0.0, "the 51st too — a cap, not a taper");

        // The narrator's case end-to-end: 51 fresh SleptInBed events.
        let mut chron = Chronicle::default();
        let actor = common::rtsim::Actor::Npc(Default::default());
        let now = common::resources::TimeOfDay(1000.0);
        for _ in 0..51 {
            chron.record(
                now,
                ChronicleKind::SleptInBed,
                vec![actor],
                None,
                None,
                Importance::Routine,
                Scope::World,
                None,
            );
        }
        let table = ThoughtTable {
            thoughts: [(ChronicleKind::SleptInBed, (0.08f32, 86_400.0f64))]
                .into_iter()
                .collect(),
        };
        let affinities = ValueAffinityTable { affinities: Default::default() };
        let values = Default::default();
        let sum = thought_sum(&chron, &table, &affinities, actor, now.0, &values, false);

        let uncapped = 51.0 * 0.08;
        let cap = 0.08 * (1.0 + 0.75 + 0.5625);
        assert!(
            (sum - cap).abs() < 1e-4,
            "51 naps must sum to the 3-stack cap ({cap:.4}), not {uncapped:.2}: got {sum:.4}"
        );
        // The consequence that matters: at hunger 0 the capped bliss cannot
        // reach a clamped 1.0 — the starving napper LOOKS unhappy again.
        let cfg = common::bastion::MoodConfig::default();
        let needs = common::comp::bastion::Needs {
            hunger: 0.0,
            rest: 1.0,
            recreation: 1.0,
        };
        let mood = common::comp::bastion::mood_formula(&cfg, &needs, sum);
        assert!(
            mood < 0.99,
            "a starving colonist must not read blissful off naps alone: mood={mood:.4}"
        );
        // Direction two: one sleep still counts in full…
        assert!(stack_multiplier(0) == 1.0);
        // …and the diagnostic agrees with the shipped sum row-for-row.
        let rows = thought_contributions(&chron, &table, &affinities, actor, now.0, &values, false);
        let row_sum: f32 = rows.iter().map(|r| r.contribution).sum();
        assert!(
            (row_sum - sum).abs() < 1e-3,
            "thought_contributions must carry the SAME cap as thought_sum:              rows={row_sum:.4} vs sum={sum:.4}"
        );
        // Distinct kinds are not stacking: a second KIND enters at full
        // multiplier even after 51 naps.
        chron.record(
            now,
            ChronicleKind::Death,
            vec![actor],
            None,
            None,
            Importance::Notable,
            Scope::World,
            None,
        );
        let table2 = ThoughtTable {
            thoughts: [
                (ChronicleKind::SleptInBed, (0.08f32, 86_400.0f64)),
                (ChronicleKind::Death, (-0.15f32, 172_800.0f64)),
            ]
            .into_iter()
            .collect(),
        };
        let sum2 = thought_sum(&chron, &table2, &affinities, actor, now.0, &values, false);
        assert!(
            (sum2 - (cap - 0.15)).abs() < 1e-3,
            "a DIFFERENT kind counts in full alongside a capped stack: got {sum2:.4}"
        );
    }
}
