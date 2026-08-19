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
    for term in chronicle
        .events()
        .filter(|e| e.actors.contains(&actor))
        .filter_map(|e| {
            table.thoughts.get(&e.kind).map(|(mag, life)| {
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
                care * common::comp::bastion::thought_decay(*mag, e.at_tod.0, now, *life)
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
    chronicle
        .events()
        .filter(|e| e.actors.contains(&actor))
        .filter_map(|e| {
            table.thoughts.get(&e.kind).map(|(mag, life)| {
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
                let contribution =
                    care * common::comp::bastion::thought_decay(*mag, e.at_tod.0, now, *life);
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
