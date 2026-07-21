//! T0.50 (master build order; T0-004 packet, step 1): the async ownership
//! substrate — the identity and generation types every async
//! request/result flow validates against.
//!
//! THE RULES (packet non-negotiables this module encodes):
//! - Acceptance = owner key matches AND generation matches AND the request
//!   is not already terminal. Nothing else — in particular, worker
//!   completion order is never semantic authority.
//! - CANCELLATION IS EFFICIENCY-ONLY; the GENERATION is the correctness
//!   barrier. A canceled-too-late result is rejected by generation
//!   validation, not by having "won" a cancellation race.
//! - Request ids are NEVER reused (monotonic allocator).
//! - The incarnation bumps on unload/reload, promotion-recreation, and
//!   destruction; the input/options generations bump on authoritative
//!   mutation of the respective request-shaping state.

use serde::{Deserialize, Serialize};

/// Which async purpose a request belongs to (path search, chunk gen, DB
/// write, ...). A plain domain ordinal: purposes are declared per consumer
/// as constants; two purposes never share a queue identity.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AsyncPurpose(pub u16);

/// The stable owner identity of an async request: WHO asked, in which
/// lifetime, for what purpose.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AsyncOwnerKey {
    /// Stable owner identity projection (uid bits, chunk key bits, ...) —
    /// never an ECS entity id (those recycle).
    pub stable_owner: u64,
    /// Lifetime ordinal: bumped on unload/reload, promotion-recreation,
    /// destruction. A result addressed to a previous incarnation is stale
    /// by construction.
    pub incarnation: u64,
    pub purpose: AsyncPurpose,
}

/// The generation stamp a request carries and a result must match to be
/// accepted.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AsyncGeneration {
    /// The owner's epoch at request time (coarse owner-state version).
    pub owner_epoch: u64,
    /// Bumped when the request's INPUT state mutates authoritatively.
    pub input_generation: u64,
    /// Bumped when the request-shaping OPTIONS mutate authoritatively.
    pub options_generation: u64,
    /// Optional finer content version for consumers that track one.
    pub content_generation: Option<u64>,
}

/// A never-reused request identity.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AsyncRequestId(pub u64);

/// The monotonic request-id allocator (per service instance).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AsyncRequestIdAllocator {
    next: u64,
}

impl AsyncRequestIdAllocator {
    pub fn allocate(&mut self) -> AsyncRequestId {
        let id = AsyncRequestId(self.next);
        self.next += 1;
        id
    }
}

/// THE acceptance predicate — the only path by which an async result may
/// enter authoritative state.
pub fn accepts(
    current_key: AsyncOwnerKey,
    current_generation: AsyncGeneration,
    result_key: AsyncOwnerKey,
    result_generation: AsyncGeneration,
    already_terminal: bool,
) -> bool {
    current_key == result_key && current_generation == result_generation && !already_terminal
}

#[cfg(test)]
mod t0_50_tests {
    use super::*;

    fn key(owner: u64, incarnation: u64) -> AsyncOwnerKey {
        AsyncOwnerKey {
            stable_owner: owner,
            incarnation,
            purpose: AsyncPurpose(1),
        }
    }

    fn generation(input: u64) -> AsyncGeneration {
        AsyncGeneration {
            owner_epoch: 0,
            input_generation: input,
            options_generation: 0,
            content_generation: None,
        }
    }

    #[test]
    fn t0_50_acceptance_truth_table() {
        // Accept: same key, same generation, not terminal.
        assert!(accepts(key(7, 0), generation(3), key(7, 0), generation(3), false));
        // Reject: incarnation bumped (owner reloaded) — stale by construction.
        assert!(!accepts(key(7, 1), generation(3), key(7, 0), generation(3), false));
        // Reject: input generation moved (authoritative mutation since).
        assert!(!accepts(key(7, 0), generation(4), key(7, 0), generation(3), false));
        // Reject: already terminal — exactly-one-terminal, always.
        assert!(!accepts(key(7, 0), generation(3), key(7, 0), generation(3), true));
        // Reject: different purpose is a different queue identity.
        let mut other = key(7, 0);
        other.purpose = AsyncPurpose(2);
        assert!(!accepts(other, generation(3), key(7, 0), generation(3), false));
    }

    #[test]
    fn t0_50_request_ids_never_reuse() {
        let mut alloc = AsyncRequestIdAllocator::default();
        let a = alloc.allocate();
        let b = alloc.allocate();
        assert!(a < b);
        assert_ne!(a, b);
    }
}
