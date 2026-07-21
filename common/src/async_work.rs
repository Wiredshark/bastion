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

/// T0.51 (packet step 2): the work class — scheduling and terminal
/// semantics differ per class (a PureCompute result can always be
/// recomputed; an ExternalTransaction that committed must surface its
/// watermark even when superseded locally).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AsyncWorkClass {
    PureCompute,
    ExternalRead,
    ExternalTransaction,
}

/// T0.51: the shared request envelope every async service speaks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AsyncWorkRequest<I> {
    pub request_id: AsyncRequestId,
    pub owner: AsyncOwnerKey,
    pub generation: AsyncGeneration,
    pub class: AsyncWorkClass,
    /// Higher = sooner within a class; ties break on request_id (stable).
    pub priority: u16,
    /// Semantic cost units (never microseconds) for admission budgeting.
    pub estimated_cost: u32,
    pub estimated_bytes: u32,
    /// Requests sharing a live coalesce key may be merged by admission —
    /// the packet's coalescing hook; `None` = never coalesce.
    pub coalesce_key: Option<u64>,
    /// Tick after which the service should not START this work (already-
    /// running work finishes and is generation-validated as usual).
    pub deadline_tick: Option<u64>,
    /// EFFICIENCY-ONLY early-out flag (see module docs: the generation is
    /// the correctness barrier, never this).
    pub cancel: bool,
    pub input: I,
}

/// T0.51: the exhaustive terminal outcome — every admitted request reaches
/// EXACTLY ONE of these (a missing terminal is itself an invariant
/// failure, per the packet).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncTerminal<O, E> {
    Succeeded(O),
    /// The owner's generation moved while the work ran — the result is
    /// discarded by the acceptance predicate; recorded, not silently lost.
    Superseded,
    CanceledBeforeStart,
    CanceledDuringCompute,
    /// Failed in a way the owner may retry (with a NEW request id).
    Retryable(E),
    PermanentFailure(E),
    /// An external transaction COMMITTED even though the local owner moved
    /// on — the watermark + output must surface so reconciliation can run
    /// (never pretend an external commit didn't happen).
    CommittedExternal { watermark: u64, output: O },
}

impl<O, E> AsyncTerminal<O, E> {
    /// Whether this terminal carries authoritative output the owner-phase
    /// merge should offer to the acceptance predicate.
    pub fn carries_output(&self) -> bool {
        matches!(
            self,
            AsyncTerminal::Succeeded(_) | AsyncTerminal::CommittedExternal { .. }
        )
    }
}

/// T0.51 step 3 (T0-004 packet): the bounded admission queue — pure data
/// structure (services embed it; no threads here). Admission enforces the
/// bound, coalesces on live coalesce keys, and orders by
/// (class, priority desc, request_id) — a stable total order, never
/// arrival time. Cancellation flags efficiency-only; shutdown drains
/// everything to CanceledBeforeStart terminals so EVERY admitted request
/// still reaches exactly one terminal.
#[derive(Clone, Debug)]
pub struct AsyncWorkQueue<I> {
    pending: Vec<AsyncWorkRequest<I>>,
    capacity: usize,
}

/// Why admission refused a request.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AdmissionRefusal {
    /// The queue is at capacity — the caller backpressures (correctness
    /// work is delayed, never silently dropped, per the packet).
    AtCapacity,
    /// Coalesced into an existing live request (its id is returned) — the
    /// caller treats that request as its own.
    Coalesced(AsyncRequestId),
}

impl<I> AsyncWorkQueue<I> {
    pub fn new(capacity: usize) -> Self {
        Self {
            pending: Vec::new(),
            capacity,
        }
    }

    pub fn len(&self) -> usize { self.pending.len() }

    pub fn is_empty(&self) -> bool { self.pending.is_empty() }

    /// Admit a request: coalesce first (a live request with the same
    /// coalesce key absorbs this one), then bound-check.
    pub fn admit(&mut self, request: AsyncWorkRequest<I>) -> Result<(), AdmissionRefusal> {
        if let Some(key) = request.coalesce_key
            && let Some(existing) = self
                .pending
                .iter()
                .find(|p| p.coalesce_key == Some(key) && !p.cancel)
        {
            return Err(AdmissionRefusal::Coalesced(existing.request_id));
        }
        if self.pending.len() >= self.capacity {
            return Err(AdmissionRefusal::AtCapacity);
        }
        self.pending.push(request);
        Ok(())
    }

    /// Flag a pending request canceled (efficiency-only — see module docs).
    pub fn cancel(&mut self, id: AsyncRequestId) {
        if let Some(request) = self.pending.iter_mut().find(|p| p.request_id == id) {
            request.cancel = true;
        }
    }

    /// Pop the next request to run: stable total order
    /// (class, priority desc, request_id) — never arrival order. Canceled
    /// entries pop first as immediate CanceledBeforeStart terminals via
    /// `Err`.
    pub fn pop_next(&mut self, now_tick: u64) -> Option<Result<AsyncWorkRequest<I>, AsyncRequestId>> {
        // Deadline-expired or canceled entries terminate without starting.
        if let Some(position) = self.pending.iter().position(|p| {
            p.cancel || p.deadline_tick.is_some_and(|deadline| now_tick > deadline)
        }) {
            return Some(Err(self.pending.remove(position).request_id));
        }
        let best = self
            .pending
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| {
                (
                    match p.class {
                        AsyncWorkClass::ExternalTransaction => 0u8,
                        AsyncWorkClass::ExternalRead => 1,
                        AsyncWorkClass::PureCompute => 2,
                    },
                    u16::MAX - p.priority,
                    p.request_id,
                )
            })
            .map(|(index, _)| index)?;
        Some(Ok(self.pending.remove(best)))
    }

    /// Shutdown: drain everything — every admitted request still reaches
    /// exactly one terminal (CanceledBeforeStart, reported to the caller).
    pub fn drain_for_shutdown(&mut self) -> Vec<AsyncRequestId> {
        self.pending.drain(..).map(|p| p.request_id).collect()
    }
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

    fn request(id: u64, class: AsyncWorkClass, priority: u16) -> AsyncWorkRequest<u32> {
        AsyncWorkRequest {
            request_id: AsyncRequestId(id),
            owner: key(1, 0),
            generation: generation(0),
            class,
            priority,
            estimated_cost: 1,
            estimated_bytes: 0,
            coalesce_key: None,
            deadline_tick: None,
            cancel: false,
            input: 0,
        }
    }

    #[test]
    fn t0_51_queue_orders_by_class_priority_id_never_arrival() {
        let mut queue = AsyncWorkQueue::new(8);
        // Arrival order deliberately scrambled.
        queue.admit(request(5, AsyncWorkClass::PureCompute, 0)).unwrap();
        queue.admit(request(3, AsyncWorkClass::PureCompute, 9)).unwrap();
        queue
            .admit(request(9, AsyncWorkClass::ExternalTransaction, 0))
            .unwrap();
        queue.admit(request(1, AsyncWorkClass::PureCompute, 9)).unwrap();

        let order: Vec<u64> = core::iter::from_fn(|| queue.pop_next(0))
            .map(|r| r.unwrap().request_id.0)
            .collect();
        // Transactions first, then priority desc, then id.
        assert_eq!(order, vec![9, 1, 3, 5]);
    }

    #[test]
    fn t0_51_queue_bounds_coalesces_cancels_and_drains() {
        let mut queue = AsyncWorkQueue::new(2);
        let mut a = request(1, AsyncWorkClass::PureCompute, 0);
        a.coalesce_key = Some(42);
        queue.admit(a).unwrap();
        // Coalesce: same live key absorbs.
        let mut b = request(2, AsyncWorkClass::PureCompute, 0);
        b.coalesce_key = Some(42);
        assert_eq!(
            queue.admit(b),
            Err(AdmissionRefusal::Coalesced(AsyncRequestId(1)))
        );
        // Bound: backpressure, never drop.
        queue.admit(request(3, AsyncWorkClass::PureCompute, 0)).unwrap();
        assert_eq!(
            queue.admit(request(4, AsyncWorkClass::PureCompute, 0)),
            Err(AdmissionRefusal::AtCapacity)
        );
        // Cancel pops first as a terminal-without-starting.
        queue.cancel(AsyncRequestId(3));
        assert!(matches!(queue.pop_next(0), Some(Err(AsyncRequestId(3)))));
        // Deadline expiry likewise.
        let mut d = request(7, AsyncWorkClass::PureCompute, 0);
        d.deadline_tick = Some(10);
        queue.admit(d).unwrap();
        assert!(matches!(queue.pop_next(11), Some(Err(AsyncRequestId(7)))));
        // Shutdown drains the remainder to terminals.
        assert_eq!(queue.drain_for_shutdown(), vec![AsyncRequestId(1)]);
        assert!(queue.is_empty());
    }
}
