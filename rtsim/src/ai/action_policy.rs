//! `T3.27` (engine-list, E3, Fable-ruled 2026-07-27): explicit action
//! arbitration — replaces "sticky numeric first-wins priorities" (the
//! existing [`super::Consider::action`]'s tie-break: among candidates at
//! the same or higher tier, whichever `.important()`/`.casual()` call
//! happened to execute FIRST in the closure body wins, an accident of
//! source order, never a designed policy) with a named class order,
//! explicit in-class scores, deliberate hysteresis for the running
//! action, and a total, deterministic tie-break.
//!
//! Scope decision, disclosed rather than silently narrowed: this module
//! is the pure POLICY — the comparison the row's `ActionPolicyV1` names
//! (`candidate key (policy_rank, score_1_q,...,score_n_q, action_kind,
//! target_id, proposal_id)`, ranked class-first). Wiring it into
//! [`super::Consider`]/[`super::Tree`]'s live storage (which currently
//! carries a bare `u32` priority, shared by every existing `.urgent()`/
//! `.important()`/`.casual()` call site across `villager`/`humanoid`) is
//! left as a follow-up — that is a storage-type migration touching every
//! existing `choose()` caller, not a policy question, and deserves its
//! own verification pass rather than landing blind alongside the policy
//! itself.

use core::cmp::Ordering;

/// Ruling #1's three named classes, Survival highest. Reuses the spirit of
/// the existing `PRIORITY_URGENT`/`PRIORITY_IMPORTANT`/`PRIORITY_CASUAL`
/// tiers (`Survival` ~ urgent, `AssignedJob` ~ important, `Social` ~
/// casual) — a rename + an explicit total order, not a new tier count.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ActionClassV1 {
    Social,
    AssignedJob,
    Survival,
}

/// Ruling #1's stickiness bonus: the currently-running action's effective
/// score is `score + HYSTERESIS_BONUS` when compared against a fresh
/// candidate in the SAME class — a fresh candidate must beat the
/// incumbent by MORE than this margin to preempt it. Applies within a
/// class only: a higher-class candidate always preempts regardless of
/// hysteresis (ruling #1 does not exempt class-crossing preemption from
/// the "survival > assigned-job > social" order).
pub const HYSTERESIS_BONUS: f32 = 0.15;

/// One arbitration candidate. `tiebreak` is the last-resort determinism
/// key when `(class, effective_score)` ties exactly — generic over
/// whichever `Ord` identity the caller has on hand (a target's `Uid`, an
/// `NpcId`, ...); this module does not mandate which.
#[derive(Copy, Clone, Debug)]
pub struct ActionCandidateV1<T> {
    pub class: ActionClassV1,
    pub score: f32,
    pub tiebreak: T,
    pub is_current: bool,
}

impl<T> ActionCandidateV1<T> {
    fn effective_score(&self) -> f32 {
        if self.is_current { self.score + HYSTERESIS_BONUS } else { self.score }
    }
}

/// Total order over candidates: class first, then effective score
/// (hysteresis-adjusted for the incumbent), then `tiebreak`. `NaN` scores
/// sort as the WORST possible score in their class (never silently
/// treated as "highest" via `f32`'s partial order) — a policy bug that
/// somehow produces NaN must lose, not win.
pub fn compare<T: Ord>(a: &ActionCandidateV1<T>, b: &ActionCandidateV1<T>) -> Ordering {
    a.class
        .cmp(&b.class)
        .then_with(|| cmp_score(a.effective_score(), b.effective_score()))
        .then_with(|| a.tiebreak.cmp(&b.tiebreak))
}

fn cmp_score(a: f32, b: f32) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => a.partial_cmp(&b).expect("neither operand is NaN"),
    }
}

/// Picks the winner of one arbitration pass — the canonical entry point
/// `Consider`'s eventual wiring calls. Ties resolve via [`compare`], so
/// two runs over the same candidate set (any order) always agree.
pub fn arbitrate<T: Ord + Copy>(candidates: &[ActionCandidateV1<T>]) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| compare(a, b))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(class: ActionClassV1, score: f32, tiebreak: u32, is_current: bool) -> ActionCandidateV1<u32> {
        ActionCandidateV1 { class, score, tiebreak, is_current }
    }

    /// Class always dominates score — even a maximal Social score loses
    /// to a minimal Survival one.
    #[test]
    fn class_dominates_score() {
        let social = cand(ActionClassV1::Social, 1000.0, 0, false);
        let survival = cand(ActionClassV1::Survival, -1000.0, 0, false);
        assert_eq!(compare(&survival, &social), Ordering::Greater);
        assert_eq!(arbitrate(&[social, survival]), Some(1));
    }

    /// Within a class, the higher explicit score wins — NOT declaration
    /// order (the row's own complaint about the old system).
    #[test]
    fn in_class_score_beats_declaration_order() {
        let first = cand(ActionClassV1::AssignedJob, 0.2, 0, false);
        let second = cand(ActionClassV1::AssignedJob, 0.9, 0, false);
        // `second` is declared AFTER `first` but scores higher — it must win.
        assert_eq!(arbitrate(&[first, second]), Some(1));
        // Order in the slice must not matter.
        assert_eq!(arbitrate(&[second, first]), Some(0));
    }

    /// Hysteresis: the incumbent survives a marginally-better challenger,
    /// but a challenger that clears the bonus margin still wins.
    #[test]
    fn hysteresis_protects_the_incumbent_within_margin() {
        let incumbent = cand(ActionClassV1::Survival, 0.5, 0, true);
        let weak_challenger = cand(ActionClassV1::Survival, 0.5 + HYSTERESIS_BONUS * 0.5, 1, false);
        assert_eq!(arbitrate(&[incumbent, weak_challenger]), Some(0), "incumbent keeps a within-margin lead");

        let strong_challenger = cand(ActionClassV1::Survival, 0.5 + HYSTERESIS_BONUS * 2.0, 1, false);
        assert_eq!(arbitrate(&[incumbent, strong_challenger]), Some(1), "a clearing challenger still preempts");
    }

    /// A higher-class challenger preempts the incumbent regardless of
    /// hysteresis — stickiness never overrides the class order.
    #[test]
    fn hysteresis_never_crosses_class_boundaries() {
        let incumbent = cand(ActionClassV1::Social, 1000.0, 0, true);
        let threat = cand(ActionClassV1::Survival, -1000.0, 1, false);
        assert_eq!(arbitrate(&[incumbent, threat]), Some(1));
    }

    /// Exact (class, effective_score) ties resolve by `tiebreak`, and the
    /// result does not depend on input order — the determinism-by-
    /// construction invariant this whole policy exists to guarantee.
    #[test]
    fn exact_ties_resolve_by_tiebreak_order_independently() {
        let a = cand(ActionClassV1::AssignedJob, 0.4, 7, false);
        let b = cand(ActionClassV1::AssignedJob, 0.4, 3, false);
        assert_eq!(arbitrate(&[a, b]), Some(0), "higher tiebreak (7 > 3) wins");
        assert_eq!(arbitrate(&[b, a]), Some(1), "same winner regardless of slice order");
    }

    /// A NaN score never wins by virtue of `f32`'s partial order quirks —
    /// it is treated as strictly worse than any real score in its class.
    #[test]
    fn nan_score_never_wins() {
        let broken = cand(ActionClassV1::Survival, f32::NAN, 99, false);
        let normal = cand(ActionClassV1::Survival, -1000.0, 0, false);
        assert_eq!(arbitrate(&[broken, normal]), Some(1));
        assert_eq!(arbitrate(&[normal, broken]), Some(0));
    }
}
