//! `T3.35+T3.39` (engine-list, E3, Fable-ruled 2026-07-27, merged row):
//! threat ranking — replaces distance-only/first-nearby target selection
//! (rtsim's `check_for_enemies`, server-agent's `choose_target`/
//! `target_if_attacked`, currently their own separate ad-hoc pickers) with
//! one shared, class-first, explicitly-weighted, deterministically
//! tie-broken policy.
//!
//! Scope decision, disclosed rather than silently narrowed: this is the
//! pure POLICY only (same discipline as [T3.27's
//! `rtsim::ai::action_policy`](../../rtsim/src/ai/action_policy.rs) — read
//! that module's own scope note for the sibling precedent). Wiring it
//! into `check_for_enemies`/`choose_target`/`target_if_attacked` is E3-W
//! (Fable-ruled, sequenced after this row so the storage migration for
//! T3.27 and this policy happens once, not twice). Lives in `common`
//! rather than `rtsim` because the row merges a threat-selection call site
//! in `rtsim` with a threat-arbitration call site in `server-agent` — one
//! shared crate both already depend on.

use core::cmp::Ordering;

/// Ruling #2's three named classes, `AttackingMe` highest. `capability`/
/// `recency` are meaningless once something is already attacking (the
/// engagement itself is the signal) — the class alone settles those two
/// cases; the fixed-weight score only discriminates WITHIN
/// `HostileNearby`, where nothing has engaged yet.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ThreatClassV1 {
    HostileNearby,
    AttackingAlly,
    AttackingMe,
}

/// Fixed weights, named per ruling #2 ("Weights are named consts, tunable
/// later") — placeholder magnitudes, not yet tuned against real combat
/// data. Proximity is the ONLY negatively-weighted term (closer = more
/// threatening = LOWER raw distance but HIGHER score contribution, so the
/// weight itself stays positive and [`ThreatCandidateV1::proximity_term`]
/// negates the distance).
pub const PROXIMITY_WEIGHT: f32 = 1.0;
pub const CAPABILITY_WEIGHT: f32 = 1.0;
pub const RECENCY_WEIGHT: f32 = 1.0;

/// One threat candidate. `distance`/`capability_vs_me`/`recency` are
/// caller-normalized inputs (this module does not define HOW distance is
/// measured, how capability is compared, or how recency decays — those
/// are the live call sites' own concerns; T3.35's cited mechanism names
/// `Sentiments`/nearby-grid, T3.39 names `choose_target`'s existing
/// distance search, neither of which this pure module can see).
/// `capability_vs_me` and `recency` are expected pre-normalized to
/// comparable ranges (higher = more threatening in both); `tiebreak` is
/// the target's own stable identity (e.g. `Uid`) — the last-resort
/// determinism key when `(class, score)` ties exactly.
#[derive(Copy, Clone, Debug)]
pub struct ThreatCandidateV1<T> {
    pub class: ThreatClassV1,
    pub distance: f32,
    pub capability_vs_me: f32,
    pub recency: f32,
    pub tiebreak: T,
}

impl<T> ThreatCandidateV1<T> {
    fn proximity_term(&self) -> f32 { -self.distance * PROXIMITY_WEIGHT }

    fn score(&self) -> f32 {
        self.proximity_term() + self.capability_vs_me * CAPABILITY_WEIGHT + self.recency * RECENCY_WEIGHT
    }
}

/// Total order over candidates: class first, then the fixed-weight
/// in-class score, then `tiebreak`. `NaN` scores sort as the WORST
/// possible score in their class (never silently "highest" via `f32`'s
/// partial order) — a malformed input must lose, not win.
pub fn compare<T: Ord>(a: &ThreatCandidateV1<T>, b: &ThreatCandidateV1<T>) -> Ordering {
    a.class.cmp(&b.class).then_with(|| cmp_score(a.score(), b.score())).then_with(|| a.tiebreak.cmp(&b.tiebreak))
}

fn cmp_score(a: f32, b: f32) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.partial_cmp(&b).expect("neither operand is NaN"),
    }
}

/// Picks the highest-ranked threat — the canonical entry point E3-W's
/// eventual wiring calls. Ties resolve via [`compare`], so two runs over
/// the same candidate set (any order) always agree.
pub fn arbitrate<T: Ord + Copy>(candidates: &[ThreatCandidateV1<T>]) -> Option<usize> {
    candidates.iter().enumerate().max_by(|(_, a), (_, b)| compare(a, b)).map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(class: ThreatClassV1, distance: f32, capability: f32, recency: f32, tiebreak: u32) -> ThreatCandidateV1<u32> {
        ThreatCandidateV1 { class, distance, capability_vs_me: capability, recency, tiebreak }
    }

    /// Ruling #2's own order: attacking-me > attacking-ally > hostile-
    /// nearby, regardless of score — even a maximally-scored hostile-
    /// nearby candidate loses to a minimally-scored attacking-me one.
    #[test]
    fn class_order_is_attacking_me_ally_then_hostile() {
        let hostile = cand(ThreatClassV1::HostileNearby, 0.0, 1000.0, 1000.0, 0);
        let ally = cand(ThreatClassV1::AttackingAlly, 1000.0, 0.0, 0.0, 0);
        let me = cand(ThreatClassV1::AttackingMe, 1000.0, 0.0, 0.0, 0);
        assert_eq!(arbitrate(&[hostile, ally, me]), Some(2));
        assert_eq!(arbitrate(&[me, ally, hostile]), Some(0), "order in the slice must not matter");
        assert_eq!(compare(&ally, &hostile), Ordering::Greater);
        assert_eq!(compare(&me, &ally), Ordering::Greater);
    }

    /// Within `HostileNearby`, closer wins (proximity dominates when
    /// capability/recency are equal) — not first-in-grid-order.
    #[test]
    fn closer_hostile_wins_within_class() {
        let far = cand(ThreatClassV1::HostileNearby, 20.0, 0.0, 0.0, 0);
        let near = cand(ThreatClassV1::HostileNearby, 2.0, 0.0, 0.0, 1);
        assert_eq!(arbitrate(&[far, near]), Some(1));
        assert_eq!(arbitrate(&[near, far]), Some(0));
    }

    /// A more capable OR more recently-hostile candidate can outweigh a
    /// closer one — the score is a genuine weighted combination, not
    /// proximity-only (the row's own complaint about the current picker).
    #[test]
    fn capability_and_recency_can_outweigh_pure_proximity() {
        let close_weak = cand(ThreatClassV1::HostileNearby, 1.0, 0.0, 0.0, 0);
        let far_capable = cand(ThreatClassV1::HostileNearby, 5.0, 10.0, 0.0, 1);
        assert_eq!(arbitrate(&[close_weak, far_capable]), Some(1));

        let far_recent = cand(ThreatClassV1::HostileNearby, 5.0, 0.0, 10.0, 1);
        assert_eq!(arbitrate(&[close_weak, far_recent]), Some(1));
    }

    /// Exact score ties resolve by `tiebreak`, order-independently.
    #[test]
    fn exact_ties_resolve_by_tiebreak_order_independently() {
        let a = cand(ThreatClassV1::HostileNearby, 3.0, 1.0, 1.0, 7);
        let b = cand(ThreatClassV1::HostileNearby, 3.0, 1.0, 1.0, 3);
        assert_eq!(arbitrate(&[a, b]), Some(0), "higher tiebreak (7 > 3) wins");
        assert_eq!(arbitrate(&[b, a]), Some(1), "same winner regardless of slice order");
    }

    /// A NaN score (e.g. from a malformed capability/recency input) never
    /// wins by virtue of `f32`'s partial-order quirks.
    #[test]
    fn nan_score_never_wins() {
        let broken = cand(ThreatClassV1::HostileNearby, f32::NAN, 0.0, 0.0, 99);
        let normal = cand(ThreatClassV1::HostileNearby, 1000.0, 0.0, 0.0, 0);
        assert_eq!(arbitrate(&[broken, normal]), Some(1));
        assert_eq!(arbitrate(&[normal, broken]), Some(0));
    }
}
