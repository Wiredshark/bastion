//! T0.68: the standing total-order selection gate.
//!
//! `world/src/civ/mod.rs:1124` picks the chunk closest to a biome's center
//! via `min_by_key(|&b| center.distance_squared(...))` with no tiebreak at
//! all -- on an exact tie, the winner is whichever chunk `Iterator::
//! min_by_key` happened to visit first, which is insertion order into
//! `biome.1`, an accident of the flood-fill that built it, not a decision
//! anyone reviewed. `world/src/civ/mod.rs:712` (DET-SITE-003) already does
//! this right: `min_by_key(|(id, s)| (distance_squared, *id))` -- the site
//! id is an explicit, reviewable second field, not a hope that the
//! iterator order happens to be stable.
//!
//! This module is the CONVENTION line 712 already follows, given a name
//! and a type so line 1124's class of bug stops recurring by omission
//! rather than by every author re-deriving DET-SITE-003's reasoning from
//! scratch. `DecisionKeyV1` is not a new selection algorithm -- it is
//! exactly the tuple every author who got this right already reached for
//! (line 712, `common/src/comp/health.rs:219`, `world/src/site/mod.rs:598`),
//! made into a name that shows up in a signature instead of a comment.
//!
//! **Convention:** an authoritative selection (`min_by_key`/`max_by_key`
//! over candidates) ends in a `DecisionKeyV1`, never a bare score.
//! Insertion/iteration order is NEVER an implicit tiebreak. A keyed-random
//! tiebreak is allowed only where the randomness is deliberately designed
//! and domain-separated (a real tiebreak-by-coinflip, not an accident) --
//! that case still fills `local_sequence` with the derived value, it does
//! not skip the type.

/// One selection's total-order key. Every field participates in `Ord`, in
/// declared order -- `score_class` breaks ties between candidates of
/// different KINDS before their scores are even compared (mirroring
/// `NumericReachV1`'s ordering in `numeric_surface.rs`); `canonical_score`
/// is the comparison that actually decides most selections; the two ids
/// and `local_sequence` are what decide an exact tie, in that order, so a
/// tie is never silently resolved by whichever candidate the iterator
/// visited first.
///
/// `C`/`S`/`I` must each be `Ord` themselves -- for a floating-point
/// score, that means passing a canonicalized representation (see
/// [`canonical_nonnegative_f32`]), never the raw `f32`/`f64`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionKeyV1<C, S, I> {
    pub score_class: C,
    pub canonical_score: S,
    pub subject_id: I,
    pub target_id: I,
    pub local_sequence: u64,
}

impl<C, S, I> DecisionKeyV1<C, S, I> {
    /// The common case: one score class (nothing to rank ahead of
    /// anything else), no meaningful subject/sequence -- just "closest
    /// wins, id breaks a tie". `score_class`/`subject_id`/`local_sequence`
    /// are filled with a caller-chosen constant so every key in the
    /// comparison set agrees and only `canonical_score`/`target_id`
    /// actually discriminate.
    pub fn nearest(score_class: C, canonical_score: S, target_id: I, subject_id: I) -> Self {
        Self { score_class, canonical_score, subject_id, target_id, local_sequence: 0 }
    }
}

/// Canonicalizes a non-negative `f32` (a distance, a magnitude -- never a
/// signed quantity) into a value whose `Ord` matches the float's natural
/// order. `f32::to_bits()` gives a totally-ordered `u32` ONLY for
/// non-negative, non-NaN floats: IEEE 754's sign bit means negative
/// values' bit patterns sort in REVERSE, and NaN has no natural order at
/// all. Both are debug-asserted against here rather than silently
/// mis-ordered.
pub fn canonical_nonnegative_f32(x: f32) -> u32 {
    debug_assert!(x.is_finite() && x >= 0.0, "canonical_nonnegative_f32 requires a finite, non-negative input, got {x}");
    x.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_falls_through_score_class_then_score_then_ids_then_sequence() {
        let a = DecisionKeyV1 { score_class: 0u8, canonical_score: 5u32, subject_id: 1u64, target_id: 1u64, local_sequence: 0 };
        let b = DecisionKeyV1 { score_class: 0u8, canonical_score: 5u32, subject_id: 1u64, target_id: 2u64, local_sequence: 0 };
        // Equal score_class and score: target_id decides.
        assert!(a < b);

        let c = DecisionKeyV1 { score_class: 1u8, canonical_score: 0u32, subject_id: 0u64, target_id: 0u64, local_sequence: 0 };
        // score_class dominates canonical_score entirely, even though c's
        // score is smaller -- a different CLASS of candidate, not a
        // better-scored one of the same class.
        assert!(a < c);
    }

    #[test]
    fn permuting_equal_scoring_candidates_does_not_change_the_winner() {
        let candidates = [3u64, 7u64, 1u64, 9u64];
        let key = |&id: &u64| DecisionKeyV1::nearest(0u8, 5u32, id, 0u64);

        let winner_forward = candidates.iter().min_by_key(|c| key(c)).copied();
        let winner_reversed =
            candidates.iter().rev().min_by_key(|c| key(c)).copied();
        assert_eq!(winner_forward, winner_reversed);
        assert_eq!(winner_forward, Some(1), "the lowest target_id must win an exact score tie");
    }

    #[test]
    fn canonical_nonnegative_f32_preserves_order() {
        let mut xs = [3.5f32, 0.0, 100.25, 1.0];
        let mut by_bits = xs;
        by_bits.sort_by_key(|&x| canonical_nonnegative_f32(x));
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(xs, by_bits);
    }
}
