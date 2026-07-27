//! `APEX-T3.2`: application session state machine -- `SessionRecordV1`,
//! deterministic sorted-commit admission, and capacity/retention policy.
//! Spec: `readme/apex/APEX-T3.2-FLEET-AUTHORED-SPEC-v1.md`.
//!
//! Determinism story (spec section 2): registry mutation happens only
//! inside [`SessionRegistry::admit_sorted`]'s single sequential pass,
//! never from a parallel/async context. The pass sorts on `(principal
//! bytes, descending attempt_seq)` -- a key fixed at receipt time,
//! before any real authentication race begins -- so the real,
//! non-reproducible completion-order race has no vote in commit order.

use authc::Uuid;
use common::apex::identity::{ConnectionEpoch, CounterAdvanceErrorV1, IdRandomBytesSourceV1, SessionId};
use common_net::msg::{ClientType, SessionRequestV1};
use common_net::msg::server::{RegisterError, SessionAdmissionV1, SessionBindingV1};
use hashbrown::HashMap;
use std::time::{Duration, Instant};

/// Zero-valid, checked, monotonic per-process counter allocated at
/// message-receipt time (single-threaded, before the awaited auth race
/// begins -- spec section 2.2 item 1). Not part of any canonical digest;
/// pure admission-ordering machinery.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SessionAttemptSeqV1(u64);

impl SessionAttemptSeqV1 {
    pub const INITIAL: Self = Self(0);

    pub const fn get(self) -> u64 { self.0 }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SessionStateV1 {
    Active,
    Detached,
}

#[derive(Clone, Debug)]
pub struct SessionRecordV1 {
    pub session_id: SessionId,
    pub principal: Uuid,
    pub client_type: ClientType,
    pub epoch: ConnectionEpoch,
    pub state: SessionStateV1,
    /// `Some` only while `Detached` -- the retention deadline. `None`
    /// while `Active` (an active session has no expiry; only a detached
    /// one does).
    pub expires_at: Option<Instant>,
}

impl SessionRecordV1 {
    fn binding(&self) -> SessionBindingV1 { SessionBindingV1 { session_id: self.session_id, epoch: self.epoch } }
}

/// One authenticated registration intent, ready for the sorted commit
/// pass. Built by the caller from a completed `LoginProvider` result;
/// carries everything the pass needs and nothing it must look up
/// elsewhere mid-pass.
#[derive(Clone, Debug)]
pub struct AuthenticatedIntentV1 {
    pub principal: Uuid,
    pub client_type: ClientType,
    pub attempt_seq: SessionAttemptSeqV1,
    pub request: SessionRequestV1,
    /// Capacity-exempt (admin role) -- decided by the caller from the
    /// same `admin` lookup the existing login flow already performs;
    /// this module does not know about roles.
    pub capacity_exempt: bool,
}

/// How long a detached session's slot is retained before it expires
/// (spec section 4 policy 4).
pub const DETACHED_RETENTION_GRACE: Duration = Duration::from_secs(60);

/// Max detached records retained across the whole registry, independent
/// of `max_active` (spec section 4 policy 4, canaries SES-095-098).
pub const DEFAULT_DETACHED_RETENTION_CAP: usize = 64;

#[derive(Default)]
pub struct SessionRegistry {
    records: HashMap<SessionId, SessionRecordV1>,
    /// Exactly one current record per principal -- a principal transitions
    /// between `Active`/`Detached` on the same record, never holds two.
    by_principal: HashMap<Uuid, SessionId>,
    next_attempt_seq: SessionAttemptSeqV1,
}

impl SessionRegistry {
    pub fn new() -> Self { Self::default() }

    /// Allocates the next `SessionAttemptSeqV1`. Call exactly once per
    /// incoming `ClientRegister`, at receipt time, in the single-threaded
    /// message-drain phase -- never from the parallel auth-completion
    /// phase (spec section 2.2 item 1, canary SES-051/052).
    pub fn allocate_attempt_seq(&mut self) -> Result<SessionAttemptSeqV1, CounterAdvanceErrorV1> {
        let issued = self.next_attempt_seq;
        self.next_attempt_seq = SessionAttemptSeqV1(self.next_attempt_seq.0.checked_add(1).ok_or(CounterAdvanceErrorV1::Exhausted)?);
        Ok(issued)
    }

    pub fn record(&self, id: SessionId) -> Option<&SessionRecordV1> { self.records.get(&id) }

    /// The current session bound to `principal`, if any -- the lookup a
    /// disconnect handler needs to translate "this player's `Uuid`" into
    /// "which `SessionId` to detach/close" (spec section 4 policy 4).
    pub fn session_for_principal(&self, principal: Uuid) -> Option<SessionId> { self.by_principal.get(&principal).copied() }

    fn active_count(&self) -> usize { self.records.values().filter(|r| matches!(r.state, SessionStateV1::Active)).count() }

    /// Removes every detached record whose `expires_at` is at or before
    /// `phase_now` (boundary-inclusive expiry -- canary SES-094). Runs once
    /// per sorted commit pass, AFTER that pass's admissions are decided
    /// (spec section 2.2 item 6, canary SES-064) -- never per-intent, and
    /// never before, since a resume attempt THIS pass against a record
    /// that is expired-but-still-present must observe `SessionExpired`
    /// (SES-030), not have the record vanish out from under it into
    /// `UnknownSession`. Reclaiming capacity/retention slots for expired
    /// detached records is therefore a next-pass concern, not this one's.
    pub fn purge_expired(&mut self, phase_now: Instant) {
        let expired: Vec<SessionId> = self
            .records
            .iter()
            .filter(|(_, r)| matches!(r.state, SessionStateV1::Detached) && r.expires_at.is_some_and(|e| e <= phase_now))
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            if let Some(r) = self.records.remove(&id) {
                self.by_principal.remove(&r.principal);
            }
        }
    }

    /// Sorts `intents` canonically (principal bytes, then descending
    /// `attempt_seq`; SES-053/060) and commits them one at a time in that
    /// order -- the only path that ever mutates this registry (spec
    /// section 2.2 item 3, canaries SES-024/065). Returns one outcome per
    /// intent, in the caller's original order (not commit order), keyed
    /// by whatever correlation value the caller attached.
    pub fn admit_sorted<K: Clone>(
        &mut self,
        mut intents: Vec<(K, AuthenticatedIntentV1)>,
        max_active: usize,
        phase_now: Instant,
        detached_retention_cap: usize,
        random_source: &mut impl IdRandomBytesSourceV1,
    ) -> Vec<(K, Result<SessionAdmissionV1, RegisterError>)> {
        // Stable sort preserves original relative order for any
        // (principal, attempt_seq) tie, but attempt_seq collisions within
        // one principal are themselves an allocation-invariant violation
        // (BLOCK-AMBIGUOUS-ATTEMPT) the caller must catch before this
        // call; this pass does not silently break such a tie. Descending
        // attempt_seq per principal means the FIRST same-principal intent
        // processed is always the one with the largest attempt_seq --
        // `admitted_this_pass` below relies on exactly that ordering.
        intents.sort_by(|a, b| a.1.principal.as_bytes().cmp(b.1.principal.as_bytes()).then(b.1.attempt_seq.cmp(&a.1.attempt_seq)));

        let mut active_count = self.active_count();
        // Which principals this SAME pass has already committed an
        // admission for. Because of the sort above, the first entry seen
        // per principal is that principal's largest attempt_seq this
        // pass; any subsequent same-principal entry is, by construction,
        // an older attempt that lost the race (SES-054/055) -- it must
        // never be allowed to re-admit/replace what the newer one already
        // committed (that would let the SMALLER attempt_seq win, backwards
        // from "larger captured attempt wins").
        let mut admitted_this_pass: hashbrown::HashSet<Uuid> = hashbrown::HashSet::new();
        let outcomes = intents
            .into_iter()
            .map(|(key, intent)| {
                let outcome = if admitted_this_pass.contains(&intent.principal) {
                    Err(RegisterError::OlderAttemptSuperseded)
                } else {
                    admitted_this_pass.insert(intent.principal);
                    self.admit_one(intent, &mut active_count, max_active, phase_now, detached_retention_cap, random_source)
                };
                (key, outcome)
            })
            .collect();

        // Reclaim capacity/retention slots for the NEXT pass only after
        // this pass's own resume attempts had a chance to observe an
        // expired-but-still-present record as `SessionExpired` (see the
        // doc comment on `purge_expired`).
        self.purge_expired(phase_now);

        outcomes
    }

    fn admit_one(
        &mut self,
        intent: AuthenticatedIntentV1,
        active_count: &mut usize,
        max_active: usize,
        phase_now: Instant,
        detached_retention_cap: usize,
        random_source: &mut impl IdRandomBytesSourceV1,
    ) -> Result<SessionAdmissionV1, RegisterError> {
        match intent.request {
            SessionRequestV1::New => self.admit_new(intent, active_count, max_active, random_source),
            SessionRequestV1::Resume { locator, expected_epoch } => {
                self.admit_resume(intent, locator, expected_epoch, active_count, max_active, phase_now, detached_retention_cap)
            },
        }
    }

    fn admit_new(
        &mut self,
        intent: AuthenticatedIntentV1,
        active_count: &mut usize,
        max_active: usize,
        random_source: &mut impl IdRandomBytesSourceV1,
    ) -> Result<SessionAdmissionV1, RegisterError> {
        // Same-principal replacement: capacity delta 0 by construction --
        // resolved BEFORE the capacity check, never counted as +1 and
        // corrected after (spec section 4 policy 1/§9, canaries
        // SES-019/070/073/074; this is the exact ordering fix for the
        // live register.rs double-count edge documented in the spec).
        if let Some(&existing_id) = self.by_principal.get(&intent.principal) {
            let existing = self.records.get(&existing_id).expect("by_principal is always in sync with records");
            let was_active = matches!(existing.state, SessionStateV1::Active);
            let epoch = existing.epoch.checked_next().map_err(|_| RegisterError::ConnectionEpochExhausted)?;
            let record = self
                .records
                .get_mut(&existing_id)
                .expect("by_principal is always in sync with records");
            record.client_type = intent.client_type;
            record.epoch = epoch;
            record.state = SessionStateV1::Active;
            record.expires_at = None;
            let binding = record.binding();
            if !was_active {
                *active_count += 1;
            }
            return Ok(SessionAdmissionV1::Replaced { binding });
        }

        if !intent.capacity_exempt && *active_count >= max_active {
            return Err(RegisterError::TooManyPlayers);
        }

        let session_id = SessionId::generate(random_source).map_err(|_| RegisterError::ConnectionEpochExhausted)?;
        let record = SessionRecordV1 {
            session_id,
            principal: intent.principal,
            client_type: intent.client_type,
            epoch: ConnectionEpoch::FIRST,
            state: SessionStateV1::Active,
            expires_at: None,
        };
        let binding = record.binding();
        self.records.insert(session_id, record);
        self.by_principal.insert(intent.principal, session_id);
        *active_count += 1;
        Ok(SessionAdmissionV1::Created { binding })
    }

    fn admit_resume(
        &mut self,
        intent: AuthenticatedIntentV1,
        locator: SessionId,
        expected_epoch: ConnectionEpoch,
        active_count: &mut usize,
        max_active: usize,
        phase_now: Instant,
        detached_retention_cap: usize,
    ) -> Result<SessionAdmissionV1, RegisterError> {
        let record = self.records.get(&locator).ok_or(RegisterError::UnknownSession)?;
        if record.principal != intent.principal {
            return Err(RegisterError::SessionPrincipalMismatch);
        }
        if record.client_type != intent.client_type {
            return Err(RegisterError::SessionClientTypeMismatch { session: record.client_type, requested: intent.client_type });
        }
        match record.state {
            SessionStateV1::Detached => {
                // Boundary-inclusive expiry: exactly `expires_at` counts
                // as expired (SES-030).
                if record.expires_at.is_some_and(|e| e <= phase_now) {
                    return Err(RegisterError::SessionExpired);
                }
            },
            SessionStateV1::Active => {},
        }
        let current_epoch = record.epoch;
        if expected_epoch != current_epoch {
            return Err(RegisterError::ConnectionEpochMismatch { current: current_epoch, expected: expected_epoch });
        }

        let was_active = matches!(record.state, SessionStateV1::Active);
        // Capacity: a detached-resume competing for capacity is delta +1
        // (it wasn't counted while detached); an already-active
        // same-session resume is delta 0 (SES-047/048/071/072). Rejecting
        // a detached resume at capacity must leave the detached record
        // intact, resumable until its own expiry (SES-048/084) -- this
        // function returns an error without mutating the record.
        if !was_active && !intent.capacity_exempt && *active_count >= max_active {
            return Err(RegisterError::TooManyPlayers);
        }
        let _ = detached_retention_cap; // retention-cap enforcement lives in detach(), not resume

        let new_epoch = current_epoch.checked_next().map_err(|_| RegisterError::ConnectionEpochExhausted)?;
        let record = self.records.get_mut(&locator).expect("looked up above");
        record.epoch = new_epoch;
        record.state = SessionStateV1::Active;
        record.expires_at = None;
        let binding = record.binding();
        if !was_active {
            *active_count += 1;
        }
        Ok(SessionAdmissionV1::Resumed { binding })
    }

    /// Transitions an active session to `Detached` (NetworkError/timeout,
    /// spec section 4 policy 4, SES-085/086) with `expires_at = phase_now
    /// + grace`. Enforces the detached-retention cap deterministically:
    /// ties break on `expires_at` (greatest survives), then canonical
    /// `SessionId` byte order (SES-099); `HashMap`-iteration-order
    /// eviction is never used (SES-100). Returns `false` (no-op, no
    /// record retained) if the cap is zero or the incoming candidate loses
    /// every tie-break (SES-095/097).
    pub fn detach(&mut self, session_id: SessionId, phase_now: Instant, grace: Duration, detached_retention_cap: usize) -> bool {
        let Some(record) = self.records.get_mut(&session_id) else { return false };
        if !matches!(record.state, SessionStateV1::Active) {
            return false;
        }
        record.state = SessionStateV1::Detached;
        record.expires_at = Some(phase_now + grace);

        if detached_retention_cap == 0 {
            if let Some(r) = self.records.remove(&session_id) {
                self.by_principal.remove(&r.principal);
            }
            return false;
        }

        let mut detached: Vec<(Instant, SessionId)> = self
            .records
            .iter()
            .filter(|(_, r)| matches!(r.state, SessionStateV1::Detached))
            .map(|(id, r)| (r.expires_at.expect("detached records always carry expires_at"), *id))
            .collect();
        // Greatest expires_at first; ties broken by greatest canonical
        // SessionId byte order first (SES-099) -- `take(cap)` below keeps
        // whichever sorts first, so both keys must sort descending.
        detached.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

        let retained: Vec<SessionId> = detached.iter().take(detached_retention_cap).map(|(_, id)| *id).collect();
        let survived = retained.contains(&session_id);
        for (_, id) in detached.into_iter().skip(detached_retention_cap) {
            if let Some(r) = self.records.remove(&id) {
                self.by_principal.remove(&r.principal);
            }
        }
        survived
    }

    /// Removes a session outright (client-requested disconnect, kick,
    /// replacement-supersedes-old, invalid client type -- spec section 4
    /// policy 4, SES-087-090). Idempotent: closing an already-absent
    /// session is a no-op, not an error (matches SES-102/103's
    /// exactly-once transition requirement at the call site).
    pub fn close(&mut self, session_id: SessionId) {
        if let Some(r) = self.records.remove(&session_id) {
            self.by_principal.remove(&r.principal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apex::identity::FixedRandomBytesSourceV1;

    fn principal(seed: u8) -> Uuid { Uuid::from_bytes([seed; 16]) }

    fn intent(principal: Uuid, attempt_seq: SessionAttemptSeqV1, request: SessionRequestV1) -> AuthenticatedIntentV1 {
        AuthenticatedIntentV1 { principal, client_type: ClientType::Game, attempt_seq, request, capacity_exempt: false }
    }

    fn source(seed: u8) -> FixedRandomBytesSourceV1 { FixedRandomBytesSourceV1([seed; 16]) }

    fn now() -> Instant { Instant::now() }

    #[test]
    fn fresh_new_below_capacity_creates() {
        let mut reg = SessionRegistry::new();
        let seq = reg.allocate_attempt_seq().unwrap();
        let mut src = source(1);
        let out = reg.admit_sorted(vec![((), intent(principal(1), seq, SessionRequestV1::New))], 10, now(), 64, &mut src);
        assert!(matches!(out[0].1, Ok(SessionAdmissionV1::Created { .. })));
    }

    /// SES-018/069: non-admin New at capacity is rejected.
    #[test]
    fn new_at_capacity_is_rejected() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 1, now(), 64, &mut src);
        let seq1 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(vec![((), intent(principal(2), seq1, SessionRequestV1::New))], 1, now(), 64, &mut src);
        assert_eq!(out[0].1, Err(RegisterError::TooManyPlayers));
    }

    /// SES-019/070/073/074: same-principal New replaces the existing
    /// active session with capacity delta 0 -- never double-counted, even
    /// at exactly `max_active`.
    #[test]
    fn same_principal_new_at_capacity_replaces_with_delta_zero() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        let first = reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 1, now(), 64, &mut src);
        let first_binding = match &first[0].1 {
            Ok(SessionAdmissionV1::Created { binding }) => *binding,
            other => panic!("expected Created, got {other:?}"),
        };
        let seq1 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(vec![((), intent(principal(1), seq1, SessionRequestV1::New))], 1, now(), 64, &mut src);
        let replaced_binding = match &out[0].1 {
            Ok(SessionAdmissionV1::Replaced { binding }) => *binding,
            other => panic!("expected Replaced (delta 0, still fits at capacity 1), got {other:?}"),
        };
        assert_eq!(replaced_binding.session_id, first_binding.session_id, "replacement reuses the same SessionId");
        assert_eq!(replaced_binding.epoch, first_binding.epoch.checked_next().unwrap());
    }

    /// SES-017/049/078/079: admin/moderator-exempt New at capacity still succeeds.
    #[test]
    fn capacity_exempt_new_succeeds_at_capacity() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 1, now(), 64, &mut src);
        let seq1 = reg.allocate_attempt_seq().unwrap();
        let mut exempt = intent(principal(2), seq1, SessionRequestV1::New);
        exempt.capacity_exempt = true;
        let out = reg.admit_sorted(vec![((), exempt)], 1, now(), 64, &mut src);
        assert!(matches!(out[0].1, Ok(SessionAdmissionV1::Created { .. })));
    }

    /// SES-053/058/059/060: commit order is (principal bytes, descending
    /// attempt_seq) -- never insertion order. Two different principals
    /// contend for the single remaining slot; the canonically-earlier
    /// principal wins regardless of which was pushed into the intents
    /// vector last.
    #[test]
    fn commit_order_is_canonical_not_insertion_order() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq_a = reg.allocate_attempt_seq().unwrap();
        let seq_b = reg.allocate_attempt_seq().unwrap();
        // principal(2) > principal(1) in byte order; push principal(2) FIRST
        // to prove insertion order is not what decides the winner.
        let intents = vec![
            ("b", intent(principal(2), seq_b, SessionRequestV1::New)),
            ("a", intent(principal(1), seq_a, SessionRequestV1::New)),
        ];
        let out = reg.admit_sorted(intents, 1, now(), 64, &mut src);
        let a_result = out.iter().find(|(k, _)| *k == "a").unwrap();
        let b_result = out.iter().find(|(k, _)| *k == "b").unwrap();
        assert!(matches!(a_result.1, Ok(SessionAdmissionV1::Created { .. })), "canonically-earlier principal must win the slot");
        assert_eq!(b_result.1, Err(RegisterError::TooManyPlayers));
    }

    /// SES-054/055: same-principal race, larger captured attempt_seq wins
    /// regardless of vector order (standing in for "regardless of which
    /// one's auth finished first" -- attempt_seq is the pre-fixed proxy
    /// for that real-world race, per spec section 2.2). "Wins" means: the
    /// larger-attempt_seq entry is the one that actually becomes the
    /// admitted session (Created); the smaller one is `OlderAttemptSuperseded`,
    /// never allowed to silently replace what the newer one just
    /// committed (that would let the SMALLER attempt_seq win, which is
    /// backwards -- a real bug an earlier draft of this test had wrong).
    #[test]
    fn same_principal_race_larger_attempt_seq_wins_regardless_of_order() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq_low = reg.allocate_attempt_seq().unwrap();
        let seq_high = reg.allocate_attempt_seq().unwrap();
        // Put the LOWER seq first in the vector; canonical sort must still
        // process the higher one as this principal's winning entry.
        let intents = vec![("low", intent(principal(1), seq_low, SessionRequestV1::New)), ("high", intent(principal(1), seq_high, SessionRequestV1::New))];
        let out = reg.admit_sorted(intents, 10, now(), 64, &mut src);
        match out.iter().find(|(k, _)| *k == "high").unwrap().1 {
            Ok(SessionAdmissionV1::Created { .. }) => {},
            ref other => panic!("expected the larger attempt_seq to win as Created, got {other:?}"),
        }
        assert_eq!(out.iter().find(|(k, _)| *k == "low").unwrap().1, Err(RegisterError::OlderAttemptSuperseded));
    }

    /// SES-027/028/032/033/034/035-038: every Resume mismatch class is
    /// distinct and typed.
    #[test]
    fn resume_mismatch_classes_are_distinct() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        let created = reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 10, now(), 64, &mut src);
        let binding = match &created[0].1 {
            Ok(SessionAdmissionV1::Created { binding }) => *binding,
            other => panic!("{other:?}"),
        };

        // UNKNOWN-SESSION: locator never existed.
        let bogus = SessionId::generate(&mut source(99)).unwrap();
        let seq1 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(
            vec![((), intent(principal(1), seq1, SessionRequestV1::Resume { locator: bogus, expected_epoch: ConnectionEpoch::FIRST }))],
            10,
            now(),
            64,
            &mut src,
        );
        assert_eq!(out[0].1, Err(RegisterError::UnknownSession));

        // SESSION-PRINCIPAL-MISMATCH: right locator, wrong principal.
        let seq2 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(
            vec![((), intent(principal(2), seq2, SessionRequestV1::Resume { locator: binding.session_id, expected_epoch: binding.epoch }))],
            10,
            now(),
            64,
            &mut src,
        );
        assert_eq!(out[0].1, Err(RegisterError::SessionPrincipalMismatch));

        // STALE-CONNECTION-EPOCH / FUTURE-CONNECTION-EPOCH.
        let stale = ConnectionEpoch::new(binding.epoch.get()).unwrap(); // same value once the session already advanced below
        let seq3 = reg.allocate_attempt_seq().unwrap();
        // First a legitimate resume to advance the epoch, so `binding.epoch` becomes stale.
        reg.admit_sorted(
            vec![((), intent(principal(1), seq3, SessionRequestV1::Resume { locator: binding.session_id, expected_epoch: binding.epoch }))],
            10,
            now(),
            64,
            &mut src,
        );
        let seq4 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(
            vec![((), intent(principal(1), seq4, SessionRequestV1::Resume { locator: binding.session_id, expected_epoch: stale }))],
            10,
            now(),
            64,
            &mut src,
        );
        match out[0].1 {
            Err(RegisterError::ConnectionEpochMismatch { .. }) => {},
            ref other => panic!("expected ConnectionEpochMismatch, got {other:?}"),
        }
    }

    /// SES-030: resume at exactly `expires_at` is expired (boundary-inclusive).
    #[test]
    fn detached_resume_at_exact_expiry_boundary_is_expired() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        let created = reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 10, now(), 64, &mut src);
        let binding = match &created[0].1 {
            Ok(SessionAdmissionV1::Created { binding }) => *binding,
            other => panic!("{other:?}"),
        };
        let detach_time = now();
        reg.detach(binding.session_id, detach_time, Duration::from_secs(10), 64);
        let expiry_boundary = detach_time + Duration::from_secs(10);
        let seq1 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(
            vec![((), intent(principal(1), seq1, SessionRequestV1::Resume { locator: binding.session_id, expected_epoch: binding.epoch }))],
            10,
            expiry_boundary,
            64,
            &mut src,
        );
        assert_eq!(out[0].1, Err(RegisterError::SessionExpired));
    }

    /// SES-048/084: detached resume rejected at capacity leaves the
    /// detached record intact and resumable until its own expiry.
    #[test]
    fn rejected_detached_resume_at_capacity_preserves_the_record() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        let created = reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 1, now(), 64, &mut src);
        let binding = match &created[0].1 {
            Ok(SessionAdmissionV1::Created { binding }) => *binding,
            other => panic!("{other:?}"),
        };
        reg.detach(binding.session_id, now(), Duration::from_secs(60), 64);
        // Fill the single slot with a different principal. Must use a
        // distinct fake random source: `FixedRandomBytesSourceV1` always
        // returns the same bytes, so reusing `src` here would generate the
        // IDENTICAL `SessionId` as principal(1)'s and silently overwrite
        // that record in the map instead of adding a second one.
        let seq1 = reg.allocate_attempt_seq().unwrap();
        let mut src2 = source(2);
        reg.admit_sorted(vec![((), intent(principal(2), seq1, SessionRequestV1::New))], 1, now(), 64, &mut src2);
        // Now principal(1)'s detached resume is rejected for capacity...
        let seq2 = reg.allocate_attempt_seq().unwrap();
        let out = reg.admit_sorted(
            vec![((), intent(principal(1), seq2, SessionRequestV1::Resume { locator: binding.session_id, expected_epoch: binding.epoch }))],
            1,
            now(),
            64,
            &mut src,
        );
        assert_eq!(out[0].1, Err(RegisterError::TooManyPlayers));
        // ...but the record must still exist, unexpired, resumable later.
        assert!(reg.record(binding.session_id).is_some(), "rejected-for-capacity resume must not destroy the detached record");
    }

    /// SES-099/100: detached retention keeps the greatest `expires_at`,
    /// tie-broken by canonical SessionId bytes -- never HashMap order.
    #[test]
    fn detached_retention_keeps_greatest_expiry_tie_broken_by_session_id() {
        let mut reg = SessionRegistry::new();
        let mut bindings = Vec::new();
        for p in 1..=3u8 {
            // Distinct source per principal: a shared `FixedRandomBytesSourceV1`
            // would generate the same `SessionId` for all three and collapse
            // them into one record instead of three.
            let mut src = source(p);
            let seq = reg.allocate_attempt_seq().unwrap();
            let created = reg.admit_sorted(vec![((), intent(principal(p), seq, SessionRequestV1::New))], 10, now(), 64, &mut src);
            let binding = match &created[0].1 {
                Ok(SessionAdmissionV1::Created { binding }) => *binding,
                other => panic!("{other:?}"),
            };
            bindings.push(binding);
        }
        let t = now();
        // Two share the exact same expires_at (via identical detach time + grace);
        // cap = 1 forces exactly one survivor, decided by SessionId tie-break.
        for b in &bindings {
            reg.detach(b.session_id, t, Duration::from_secs(10), 1);
        }
        let survivors: Vec<SessionId> = bindings.iter().filter(|b| reg.record(b.session_id).is_some()).map(|b| b.session_id).collect();
        assert_eq!(survivors.len(), 1, "cap=1 must retain exactly one detached record");
        let expected_survivor = bindings.iter().map(|b| b.session_id).max().unwrap();
        assert_eq!(survivors[0], expected_survivor, "tie-break must select by canonical SessionId byte order, not insertion/hash order");
    }

    /// SES-105: the registry is memory-only -- a fresh instance starts
    /// with no records, regardless of anything a prior instance held
    /// (there is no persistence path to even test the negative against).
    #[test]
    fn fresh_registry_is_empty() {
        let reg = SessionRegistry::new();
        assert_eq!(reg.active_count(), 0);
    }

    /// SES-102/103: a stale disconnect (old binding) is a no-op; the
    /// current binding transitions exactly once.
    #[test]
    fn close_by_stale_binding_is_a_no_op_current_binding_closes_once() {
        let mut reg = SessionRegistry::new();
        let mut src = source(1);
        let seq0 = reg.allocate_attempt_seq().unwrap();
        let created = reg.admit_sorted(vec![((), intent(principal(1), seq0, SessionRequestV1::New))], 10, now(), 64, &mut src);
        let binding = match &created[0].1 {
            Ok(SessionAdmissionV1::Created { binding }) => *binding,
            other => panic!("{other:?}"),
        };
        let stale_id = SessionId::generate(&mut source(200)).unwrap();
        reg.close(stale_id); // no-op: never existed
        assert!(reg.record(binding.session_id).is_some());
        reg.close(binding.session_id);
        assert!(reg.record(binding.session_id).is_none());
        reg.close(binding.session_id); // idempotent: already gone
        assert!(reg.record(binding.session_id).is_none());
    }
}
