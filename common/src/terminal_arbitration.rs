//! `T0.86` (E4 row 2, Fable-ruled 2026-07-28): deterministic arbitration of
//! terminal transitions. Replaces "CPU arrival order = semantic truth"
//! (confirmed live in `rtsim::data::quest::Quest::resolve`'s raw
//! `compare_exchange(0, 2|1, Relaxed)`: whichever caller's CAS lands first
//! decides success vs timeout vs cancel, which decides whether the quest
//! deposit comes back) with a named commit phase: collect every competing
//! proposal for an aggregate, discard stale ones, stable-sort by an
//! EXPLICIT domain policy, and commit exactly one outcome.
//!
//! Sits ON [`crate::command_protocol`] (T1.10) rather than inventing a new
//! state-machine type family: a committed intent is expected to progress
//! through the SAME `CommandStatus`/`may_transition_to` lifecycle every
//! other command already uses. `T1.22` (saga machinery) is confirmed NOT
//! built anywhere in the workspace -- this module is scoped to the
//! arbitration MECHANISM (a pure, generic commit-phase function) and does
//! not attempt saga coordination, compensation, or multi-step recovery.
//!
//! Determinism story (Ben's law): pure function of its inputs, no RNG, no
//! wall-clock (`effective_tick` is caller-supplied SIM time). The caller
//! is responsible for presenting `intents` in an ALREADY-canonical order
//! (e.g. sorted by `(stable_producer, producer_sequence)`) before calling
//! [`commit_terminal_intents`] -- the stable sort inside relies on that
//! order for its own tie-break, so a non-canonical input order would leak
//! back in as non-determinism.

use crate::command_protocol::IdempotencyKey;
use core::cmp::Ordering;

/// One competing proposal to terminally transition an aggregate at a given
/// optimistic-concurrency generation. `effective_tick` is SIM time, never
/// wall-clock.
#[derive(Clone, Debug)]
pub struct TerminalIntent<O> {
    /// The optimistic-concurrency version this intent observed the
    /// aggregate to be at. Compared against the aggregate's CURRENT
    /// version at commit time; a mismatch means the intent is stale
    /// (decided based on state that has since moved on) and is discarded
    /// rather than committed, regardless of policy rank.
    pub observed_version: u64,
    pub outcome: O,
    pub reason: &'static str,
    pub effective_tick: u64,
    pub causation: IdempotencyKey,
    pub stable_producer: u64,
    pub producer_sequence: u64,
}

/// A domain's arbitration policy over its own outcome type.
pub trait TerminalPolicy<O> {
    /// A TOTAL order (reflexive/antisymmetric/transitive) ranking two
    /// FRESH (non-stale) intents' priority -- higher wins. Returning
    /// `Ordering::Equal` for two intents that are NOT
    /// [`Self::is_duplicate`] is a genuine equal-priority contradiction:
    /// [`commit_terminal_intents`] reports it as [`TerminalReceipt::Conflict`],
    /// never picks one arbitrarily.
    fn compare(&self, a: &TerminalIntent<O>, b: &TerminalIntent<O>) -> Ordering;

    /// Whether two intents represent the SAME logical request re-arriving
    /// (idempotent duplicate) rather than two genuinely competing
    /// outcomes.
    fn is_duplicate(&self, a: &O, b: &O) -> bool;
}

/// The result for one submitted intent after a commit phase runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalReceipt<O: PartialEq> {
    /// This intent's outcome is now the aggregate's authoritative terminal
    /// state.
    Committed,
    /// This intent duplicated the one that committed (same logical
    /// request) -- idempotent: treat it as if it had committed too, but
    /// no new side effects should fire for it.
    DuplicateOfCommitted,
    /// A higher-ranked intent committed instead.
    Lost { winner: O },
    /// This intent observed a version of the aggregate that has since
    /// moved on; discarded without ranking.
    Stale { observed_version: u64, current_version: u64 },
    /// Two or more intents tied under the domain policy and were NOT
    /// duplicates of each other -- a genuine, typed, deterministic
    /// contradiction. Nothing commits from this group; the caller decides
    /// how to surface/retry it. Never resolved by picking one arbitrarily.
    Conflict { contenders: Vec<O> },
}

/// The named commit phase (`T0.86`): discard stale intents (observed
/// version != `current_version`), stable-sort the rest by the domain
/// policy (descending), and commit exactly one -- UNLESS the top-ranked
/// group is a genuine equal-priority contradiction, in which case nothing
/// commits and every contender in that group gets
/// [`TerminalReceipt::Conflict`]. Returns one receipt per input intent, in
/// the SAME order as `intents`.
pub fn commit_terminal_intents<O: Clone + PartialEq, P: TerminalPolicy<O>>(
    current_version: u64,
    intents: &[TerminalIntent<O>],
    policy: &P,
) -> Vec<TerminalReceipt<O>> {
    if intents.is_empty() {
        return Vec::new();
    }

    // Stale intents are discarded without ranking, regardless of policy.
    let mut fresh_idx: Vec<usize> = (0..intents.len())
        .filter(|&i| intents[i].observed_version == current_version)
        .collect();

    // Stable sort (descending rank) -- ties preserve the CALLER's input
    // order, which must already be canonical (see module doc).
    fresh_idx.sort_by(|&a, &b| policy.compare(&intents[b], &intents[a]));

    let mut receipts = vec![None; intents.len()];
    for i in 0..intents.len() {
        if intents[i].observed_version != current_version {
            receipts[i] = Some(TerminalReceipt::Stale {
                observed_version: intents[i].observed_version,
                current_version,
            });
        }
    }

    if let Some(&top) = fresh_idx.first() {
        // Everyone ranked exactly equal to (and not a duplicate of) the
        // top-ranked intent forms the contention group.
        let tied: Vec<usize> = fresh_idx
            .iter()
            .copied()
            .filter(|&i| {
                i == top
                    || (policy.compare(&intents[i], &intents[top]) == Ordering::Equal
                        && !policy.is_duplicate(&intents[i].outcome, &intents[top].outcome))
            })
            .collect();

        if tied.len() > 1 {
            let contenders: Vec<O> = tied.iter().map(|&i| intents[i].outcome.clone()).collect();
            for &i in &tied {
                receipts[i] = Some(TerminalReceipt::Conflict {
                    contenders: contenders.clone(),
                });
            }
            for &i in &fresh_idx {
                if receipts[i].is_none() {
                    receipts[i] = Some(TerminalReceipt::Lost {
                        winner: intents[top].outcome.clone(),
                    });
                }
            }
        } else {
            for &i in &fresh_idx {
                receipts[i] = Some(if i == top {
                    TerminalReceipt::Committed
                } else if policy.is_duplicate(&intents[i].outcome, &intents[top].outcome) {
                    TerminalReceipt::DuplicateOfCommitted
                } else {
                    TerminalReceipt::Lost {
                        winner: intents[top].outcome.clone(),
                    }
                });
            }
        }
    }

    receipts.into_iter().map(|r| r.expect("every intent index is covered by exactly one branch above")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Outcome {
        A,
        B,
        C,
        D,
    }

    struct RankPolicy;
    impl TerminalPolicy<Outcome> for RankPolicy {
        fn compare(&self, a: &TerminalIntent<Outcome>, b: &TerminalIntent<Outcome>) -> Ordering {
            fn rank(o: &Outcome) -> u8 {
                match o {
                    Outcome::A => 2,
                    Outcome::B => 1,
                    Outcome::C => 1,
                    Outcome::D => 0,
                }
            }
            rank(&a.outcome).cmp(&rank(&b.outcome))
        }

        fn is_duplicate(&self, a: &Outcome, b: &Outcome) -> bool { a == b }
    }

    fn intent(outcome: Outcome, observed_version: u64, seq: u64) -> TerminalIntent<Outcome> {
        TerminalIntent {
            observed_version,
            outcome,
            reason: "test",
            effective_tick: 0,
            causation: IdempotencyKey(seq),
            stable_producer: 0,
            producer_sequence: seq,
        }
    }

    #[test]
    fn empty_input_is_empty_output() {
        let receipts = commit_terminal_intents(0, &[], &RankPolicy);
        assert!(receipts.is_empty());
    }

    #[test]
    fn single_fresh_intent_commits() {
        let intents = [intent(Outcome::A, 0, 0)];
        let receipts = commit_terminal_intents(0, &intents, &RankPolicy);
        assert_eq!(receipts, vec![TerminalReceipt::Committed]);
    }

    /// The defect this row exists to fix: CPU arrival order must not
    /// decide the winner among genuinely different-ranked outcomes --
    /// the higher-ranked one wins regardless of submission order.
    #[test]
    fn higher_ranked_outcome_wins_regardless_of_submission_order() {
        let forward = [intent(Outcome::B, 0, 0), intent(Outcome::A, 0, 1)];
        let backward = [intent(Outcome::A, 0, 1), intent(Outcome::B, 0, 0)];

        let r1 = commit_terminal_intents(0, &forward, &RankPolicy);
        assert_eq!(r1[0], TerminalReceipt::Lost { winner: Outcome::A });
        assert_eq!(r1[1], TerminalReceipt::Committed);

        let r2 = commit_terminal_intents(0, &backward, &RankPolicy);
        assert_eq!(r2[0], TerminalReceipt::Committed);
        assert_eq!(r2[1], TerminalReceipt::Lost { winner: Outcome::A });
    }

    #[test]
    fn stale_intent_is_discarded_without_ranking() {
        let intents = [intent(Outcome::A, 5, 0), intent(Outcome::B, 0, 1)];
        let receipts = commit_terminal_intents(0, &intents, &RankPolicy);
        assert_eq!(
            receipts[0],
            TerminalReceipt::Stale {
                observed_version: 5,
                current_version: 0
            }
        );
        assert_eq!(receipts[1], TerminalReceipt::Committed);
    }

    #[test]
    fn exact_duplicate_of_the_winner_is_idempotent_not_a_conflict() {
        let intents = [intent(Outcome::A, 0, 0), intent(Outcome::A, 0, 1)];
        let receipts = commit_terminal_intents(0, &intents, &RankPolicy);
        assert_eq!(receipts[0], TerminalReceipt::Committed);
        assert_eq!(receipts[1], TerminalReceipt::DuplicateOfCommitted);
    }

    /// Equal-priority, non-duplicate outcomes: a genuine contradiction,
    /// reported as such -- never resolved by picking one arbitrarily.
    #[test]
    fn equal_priority_non_duplicate_outcomes_are_a_typed_conflict() {
        let intents = [intent(Outcome::B, 0, 0), intent(Outcome::C, 0, 1)];
        let receipts = commit_terminal_intents(0, &intents, &RankPolicy);
        let expected = TerminalReceipt::Conflict {
            contenders: vec![Outcome::B, Outcome::C],
        };
        assert_eq!(receipts[0], expected);
        assert_eq!(receipts[1], expected);
    }

    /// A conflict AT the top rank still lets a genuinely LOWER-ranked
    /// intent lose cleanly (not swept into the conflict) -- B and C tie
    /// for the highest rank present; D is unambiguously lower.
    #[test]
    fn lower_ranked_intent_loses_cleanly_even_when_the_top_rank_conflicts() {
        let intents = [
            intent(Outcome::B, 0, 0),
            intent(Outcome::C, 0, 1),
            intent(Outcome::D, 0, 2),
        ];
        let receipts = commit_terminal_intents(0, &intents, &RankPolicy);
        let conflict = TerminalReceipt::Conflict {
            contenders: vec![Outcome::B, Outcome::C],
        };
        assert_eq!(receipts[0], conflict);
        assert_eq!(receipts[1], conflict);
        assert_eq!(
            receipts[2],
            TerminalReceipt::Lost {
                winner: Outcome::B
            },
            "D loses to WHICHEVER of the tied contenders sorted first (B, by stable-sort \
             input order) -- it's a clean loss, not swept into the conflict"
        );
    }
}
