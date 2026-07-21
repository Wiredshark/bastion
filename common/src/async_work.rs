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

/// T0.51 step 4 (T0-004 packet): one completed unit of work as the worker
/// reports it — carried verbatim into the owner-phase merge; nothing here
/// touches live state.
#[derive(Clone, Debug)]
pub struct CompletedWork<O, E> {
    pub request_id: AsyncRequestId,
    pub owner: AsyncOwnerKey,
    pub generation: AsyncGeneration,
    pub terminal: AsyncTerminal<O, E>,
}

/// Why a completed unit was rejected at the merge.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompletionRejection {
    /// Generation/key validation failed — the owner moved on (the packet's
    /// Superseded path at acceptance time).
    Stale,
    /// This request id already reached a terminal — exactly-once violated
    /// by a duplicate report; the duplicate is refused.
    DuplicateTerminal,
}

/// T0.51 step 4: the owner-phase completion buffer — workers push in
/// completion order (meaningless), the owner phase drains at its named
/// point: SORT BY SEMANTIC KEY (owner, purpose, request id — never
/// arrival), enforce terminal-uniqueness, validate via the acceptance
/// predicate, and hand accepted outputs onward (the caller's command
/// journal). Completion arrival order is never semantic authority.
#[derive(Clone, Debug, Default)]
pub struct AsyncCompletionBuffer<O, E> {
    completed: Vec<CompletedWork<O, E>>,
    /// Terminal-uniqueness ledger (ids are monotonic; prune below the
    /// caller's outstanding watermark to bound memory).
    terminalized: std::collections::BTreeSet<AsyncRequestId>,
}

impl<O, E> AsyncCompletionBuffer<O, E> {
    /// Worker-side push — any thread order; order is discarded at drain.
    pub fn push(&mut self, work: CompletedWork<O, E>) { self.completed.push(work); }

    /// The named owner-phase drain. `current` reports the owner's live
    /// (generation, already_terminal) — `None` = owner gone (stale).
    /// Returns (accepted in semantic order, rejections in the same order).
    #[expect(clippy::type_complexity, reason = "the merge's two result lanes")]
    pub fn drain_at_owner_phase(
        &mut self,
        mut current: impl FnMut(AsyncOwnerKey) -> Option<(AsyncGeneration, bool)>,
    ) -> (
        Vec<(AsyncOwnerKey, AsyncRequestId, AsyncTerminal<O, E>)>,
        Vec<(AsyncRequestId, CompletionRejection)>,
    ) {
        let mut batch = core::mem::take(&mut self.completed);
        // THE SEMANTIC MERGE KEY — never completion arrival.
        batch.sort_by_key(|work| (work.owner, work.request_id));
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for work in batch {
            if self.terminalized.contains(&work.request_id) {
                rejected.push((work.request_id, CompletionRejection::DuplicateTerminal));
                continue;
            }
            self.terminalized.insert(work.request_id);
            let live = current(work.owner);
            let ok = live.is_some_and(|(generation, already_terminal)| {
                accepts(
                    work.owner,
                    generation,
                    work.owner,
                    work.generation,
                    already_terminal,
                )
            });
            if ok {
                accepted.push((work.owner, work.request_id, work.terminal));
            } else {
                rejected.push((work.request_id, CompletionRejection::Stale));
            }
        }
        (accepted, rejected)
    }

    /// Prune the uniqueness ledger below the caller's minimum outstanding
    /// request id (everything below can never legitimately complete again).
    pub fn prune_terminalized_below(&mut self, watermark: AsyncRequestId) {
        self.terminalized = self.terminalized.split_off(&watermark);
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

    #[test]
    fn t0_51_owner_phase_merge_is_semantic_order_validated_exactly_once() {
        let mut buffer: AsyncCompletionBuffer<u32, ()> = AsyncCompletionBuffer::default();
        let completed = |id: u64, owner: u64, input_generation: u64| CompletedWork {
            request_id: AsyncRequestId(id),
            owner: key(owner, 0),
            generation: generation(input_generation),
            terminal: AsyncTerminal::Succeeded(id as u32),
        };
        // Completion ARRIVAL order deliberately scrambled + one stale + one
        // duplicate.
        buffer.push(completed(9, 2, 0));
        buffer.push(completed(3, 1, 0));
        buffer.push(completed(5, 1, 7)); // stale: owner is at input_generation 0
        buffer.push(completed(3, 1, 0)); // duplicate terminal
        let (accepted, rejected) =
            buffer.drain_at_owner_phase(|_| Some((generation(0), false)));
        let ids: Vec<u64> = accepted.iter().map(|(_, id, _)| id.0).collect();
        // Semantic order: owner 1 before owner 2; ids ascending within.
        assert_eq!(ids, vec![3, 9]);
        assert_eq!(rejected, vec![
            (AsyncRequestId(3), CompletionRejection::DuplicateTerminal),
            (AsyncRequestId(5), CompletionRejection::Stale),
        ]);
        // Owner-gone results are stale, never accepted.
        buffer.push(completed(11, 3, 0));
        let (accepted, rejected) = buffer.drain_at_owner_phase(|_| None);
        assert!(accepted.is_empty());
        assert_eq!(rejected, vec![(AsyncRequestId(11), CompletionRejection::Stale)]);
        // Pruning bounds the ledger without forgetting live ids.
        buffer.prune_terminalized_below(AsyncRequestId(10));
        buffer.push(completed(9, 2, 0));
        let (accepted, _) = buffer.drain_at_owner_phase(|_| Some((generation(0), false)));
        // Id 9 was pruned below the watermark — the caller guarantees no
        // legitimate re-completion below it, so this duplicate slips the
        // ledger BY DESIGN (documented contract: watermark = min
        // outstanding).
        assert_eq!(accepted.len(), 1);
    }
}
