//! T0.56 (master build order; T0-004 packet, step 8): the deterministic
//! causal recorder record — an internal causal trace whose trace/span ids
//! are DERIVED from (run, scenario, causation sequence), never randomness,
//! so two identical runs produce identical records (optional OTel export
//! maps these onto OTel ids; it never sources them).
//!
//! Determinism story (Ben's law): every id here is a pure function of
//! stable identity; there is no wall-clock, no RNG, no
//! iteration-order-dependent field.

use crate::state_hash::{DomainHash, RecorderSchemaRef};
use serde::{Deserialize, Serialize};

/// A deterministic span/trace/causation id — derived, never random.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CausalId(pub u64);

/// Derive a span id from the parent context — a pure fold over
/// (run, scenario, tick, phase ordinal, producer-local sequence). The same
/// context always yields the same id.
pub fn derive_span_id(
    run_id: u64,
    scenario_id: u64,
    tick: u64,
    phase_ordinal: u16,
    sequence: u32,
) -> CausalId {
    // fxhash-style deterministic mix (no ambient state).
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for part in [
        run_id,
        scenario_id,
        tick,
        u64::from(phase_ordinal),
        u64::from(sequence),
    ] {
        h ^= part;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    CausalId(h)
}

/// The terminal outcome a causal record closes with — every admitted
/// command/async request reaches exactly one (a missing outcome is itself
/// an invariant failure, per the packet).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalOutcome {
    Ok,
    Rejected,
    Superseded,
    Canceled,
    RetryableFailure,
    PermanentFailure,
    Compensated,
    OwnerGone,
}

/// T0.56: one causal record. Pre/post state hashes bound the effect;
/// causation/correlation relate records without asserting one total
/// chronology (the T0.31 principle at recorder scope).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CausalRecord {
    pub schema: RecorderSchemaRef,
    pub run_id: u64,
    pub trace_id: CausalId,
    pub span_id: CausalId,
    pub parent_span: Option<CausalId>,
    /// Cross-record LINKS (batched / scatter-gather causation) — links, not
    /// fake parenthood.
    pub links: Vec<CausalId>,
    pub tick: u64,
    pub phase_ordinal: u16,
    pub sequence: u32,
    /// Event-kind id — never reused (an append-only registry, per T0.58).
    pub kind: u32,
    pub causation_id: Option<CausalId>,
    pub correlation_id: Option<CausalId>,
    pub actor: Option<u64>,
    pub target: Option<u64>,
    pub pre_hash: Option<DomainHash>,
    pub post_hash: Option<DomainHash>,
    pub outcome: CausalOutcome,
}

/// T0.60 (T0-004 packet, step 8 family): the span hierarchy an
/// instrumented run nests spans into — scenario ⊃ outer_tick ⊃ phase ⊃
/// command/transaction ⊃ leaf (job-leg / terrain edit / transfer / step).
/// Batched or scatter-gather causation uses LINKS on [`CausalRecord`], not
/// fake parenthood — so this is a strict DEPTH ladder, deepening only.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SpanKind {
    Scenario,
    OuterTick,
    Phase,
    Command,
    Leaf,
}

impl SpanKind {
    /// Nesting depth (Scenario = 0). A child span's kind must be strictly
    /// deeper than its parent's — the instrumentation contract, checkable
    /// where a parent link is real parenthood (not a batched LINK).
    pub fn depth(self) -> u8 {
        match self {
            SpanKind::Scenario => 0,
            SpanKind::OuterTick => 1,
            SpanKind::Phase => 2,
            SpanKind::Command => 3,
            SpanKind::Leaf => 4,
        }
    }

    /// Whether `self` may be the direct parent of `child` (strictly one or
    /// more levels shallower — the ladder only deepens).
    pub fn may_parent(self, child: SpanKind) -> bool {
        self.depth() < child.depth()
    }
}

#[cfg(test)]
mod t0_60_tests {
    use super::SpanKind;

    #[test]
    fn t0_60_span_ladder_only_deepens() {
        assert!(SpanKind::Phase.may_parent(SpanKind::Command));
        assert!(SpanKind::Command.may_parent(SpanKind::Leaf));
        // Same level or shallower child is not real parenthood (use LINKS).
        assert!(!SpanKind::Command.may_parent(SpanKind::Command));
        assert!(!SpanKind::Leaf.may_parent(SpanKind::Phase));
    }
}

#[cfg(test)]
mod t0_56_tests {
    use super::*;

    #[test]
    fn t0_56_span_ids_are_deterministic_and_context_sensitive() {
        // Same context → same id (two "runs" reproduce).
        let a = derive_span_id(1, 2, 100, 3, 7);
        let b = derive_span_id(1, 2, 100, 3, 7);
        assert_eq!(a, b, "span ids must be a pure function of context");
        // Any context field change → different id (no collisions on the
        // dimensions the packet keys on).
        assert_ne!(a, derive_span_id(9, 2, 100, 3, 7));
        assert_ne!(a, derive_span_id(1, 9, 100, 3, 7));
        assert_ne!(a, derive_span_id(1, 2, 999, 3, 7));
        assert_ne!(a, derive_span_id(1, 2, 100, 9, 7));
        assert_ne!(a, derive_span_id(1, 2, 100, 3, 9));
    }
}
