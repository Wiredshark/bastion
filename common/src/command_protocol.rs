//! T1.3 + T1.10 (master build order; T1-001 packet, steps 2-3): the command
//! admission receipt and the full command-status lifecycle.
//!
//! A thin protocol layer over the EXISTING authorities — not a parallel
//! engine. Every admitted command gains a [`CommandReceipt`] with
//! correlation + idempotency identity (a duplicate request returns the
//! EXISTING receipt, never a second workflow) and progresses through
//! exactly one immutable terminal [`CommandStatus`]. HUD and harness read
//! the SAME server status; a client never infers completion from animation
//! or an entity disappearing.
//!
//! Reuses the T0-004 async substrate concepts: `Accepted` = entered the
//! workflow (not completion), the acceptance predicate's generation barrier
//! governs whether a result may commit, and the terminal-uniqueness rule
//! is the same exactly-one-terminal law.
//!
//! Determinism story (Ben's law): idempotency-keyed BTreeMap dedup +
//! monotonic id allocation + a centralized legal-transition table; no RNG,
//! no wall-clock, no iteration-order dependence.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A never-reused command identity.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommandId(pub u64);

/// Correlates a command with the request that issued it (client turn, ...).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CorrelationId(pub u64);

/// A stable hash of the command intent — duplicate submissions share it and
/// resolve to the same receipt (idempotency).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IdempotencyKey(pub u64);

/// T1.3: the admission decision — `Accepted` means ENTERED THE WORKFLOW,
/// not completion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionStatus {
    Accepted,
    Rejected(String),
    Deferred(String),
    Expired,
}

/// T1.3: the receipt every admitted command returns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub command_id: CommandId,
    pub correlation_id: CorrelationId,
    pub idempotency_key: IdempotencyKey,
    pub admission: AdmissionStatus,
}

/// T1.10: the full command-status lifecycle. Terminal states
/// (`Committed`, `Compensated`, `Rejected`, `Expired`, `Failed`) are
/// immutable once reached.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandStatus {
    Accepted,
    Deferred,
    Executing,
    Committed,
    Compensating,
    Compensated,
    Rejected(String),
    Expired,
    Failed(String),
}

impl CommandStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CommandStatus::Committed
                | CommandStatus::Compensated
                | CommandStatus::Rejected(_)
                | CommandStatus::Expired
                | CommandStatus::Failed(_)
        )
    }

    /// The centralized legal-transition table. Terminal → anything is
    /// illegal (immutability); the happy path is
    /// Accepted → Executing → Committed, with Deferred, compensation, and
    /// failure/expiry branches.
    pub fn may_transition_to(&self, next: &CommandStatus) -> bool {
        use CommandStatus::*;
        if self.is_terminal() {
            return false;
        }
        match (self, next) {
            (Accepted, Deferred | Executing | Rejected(_) | Expired) => true,
            (Deferred, Executing | Rejected(_) | Expired) => true,
            (Executing, Committed | Compensating | Failed(_) | Expired) => true,
            (Compensating, Compensated | Failed(_)) => true,
            _ => false,
        }
    }
}

/// T1.3: the admission ledger — idempotency-keyed dedup + monotonic command
/// ids. Embedded per command domain; pure state (serialize/hash it and a
/// replay reproduces it).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AdmissionLedger {
    receipts: BTreeMap<IdempotencyKey, CommandReceipt>,
    next_command_id: u64,
}

impl AdmissionLedger {
    /// Admit a command: a DUPLICATE idempotency key returns the EXISTING
    /// receipt (no second workflow); otherwise allocate a fresh command id,
    /// record the decision, and return the new receipt.
    pub fn admit(
        &mut self,
        idempotency_key: IdempotencyKey,
        correlation_id: CorrelationId,
        decision: AdmissionStatus,
    ) -> CommandReceipt {
        if let Some(existing) = self.receipts.get(&idempotency_key) {
            return existing.clone();
        }
        let command_id = CommandId(self.next_command_id);
        self.next_command_id += 1;
        let receipt = CommandReceipt {
            command_id,
            correlation_id,
            idempotency_key,
            admission: decision,
        };
        self.receipts.insert(idempotency_key, receipt.clone());
        receipt
    }

    pub fn receipt(&self, idempotency_key: IdempotencyKey) -> Option<&CommandReceipt> {
        self.receipts.get(&idempotency_key)
    }
}

#[cfg(test)]
mod t1_3_tests {
    use super::*;

    #[test]
    fn t1_3_duplicate_requests_return_existing_receipt() {
        let mut ledger = AdmissionLedger::default();
        let first = ledger.admit(
            IdempotencyKey(42),
            CorrelationId(1),
            AdmissionStatus::Accepted,
        );
        // Duplicate key: same receipt (same command id), NOT a second
        // workflow — even if a different decision is offered.
        let dup = ledger.admit(
            IdempotencyKey(42),
            CorrelationId(2),
            AdmissionStatus::Rejected("late".to_string()),
        );
        assert_eq!(first, dup);
        assert_eq!(dup.command_id, CommandId(0));
        assert_eq!(dup.correlation_id, CorrelationId(1));
        // A fresh key allocates a new command id.
        let other = ledger.admit(
            IdempotencyKey(43),
            CorrelationId(3),
            AdmissionStatus::Accepted,
        );
        assert_eq!(other.command_id, CommandId(1));
    }

    #[test]
    fn t1_10_status_transitions_are_legal_and_terminals_immutable() {
        use CommandStatus::*;
        // Happy path.
        assert!(Accepted.may_transition_to(&Executing));
        assert!(Executing.may_transition_to(&Committed));
        // Compensation branch.
        assert!(Executing.may_transition_to(&Compensating));
        assert!(Compensating.may_transition_to(&Compensated));
        // Illegal: skip Executing.
        assert!(!Accepted.may_transition_to(&Committed));
        // Terminal is immutable — Committed goes nowhere.
        assert!(Committed.is_terminal());
        assert!(!Committed.may_transition_to(&Executing));
        assert!(!Compensated.may_transition_to(&Committed));
        // Rejected/Expired/Failed are terminal.
        assert!(Rejected("x".to_string()).is_terminal());
        assert!(Expired.is_terminal());
        assert!(Failed("y".to_string()).is_terminal());
    }
}
