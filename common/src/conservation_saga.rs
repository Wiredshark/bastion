//! T1.5 (master build order; T1-001 packet, step 6): typed conservation
//! sagas — reserve / commit / compensate for multi-owner resource
//! transfers (inventory ↔ inventory, trade, destruction drops, Bastion job
//! completion).
//!
//! ORCHESTRATED, not choreographed: one durable coordinator owns the step
//! ORDER and the compensation. The coordinator is RECOVERABLE — its state
//! (which steps committed) is serializable, so a restart resumes or
//! compensates exactly once. Every step is IDEMPOTENT (re-running a
//! committed step is a no-op). Compensation restores ownership or creates
//! an explicit recoverable escrow — it NEVER fabricates or deletes stock,
//! so total quantity is conserved across any commit-then-compensate path.
//!
//! Determinism story (Ben's law): steps execute in a fixed order, progress
//! is a bitset, compensation runs completed steps in REVERSE order; no RNG,
//! no wall-clock (the deadline is a sim tick).

use serde::{Deserialize, Serialize};

/// A saga id (the durable coordinator's identity).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SagaId(pub u64);

/// A reservation held for the duration of the saga (an escrow ticket).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReservationId(pub u64);

/// The coordinator's lifecycle state.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaState {
    Running,
    Completed,
    Compensating,
    Compensated,
    Failed,
}

/// T1.5: the durable, recoverable saga coordinator. `completed_steps` is a
/// bitset (step i committed ⇔ bit i set); restart reads it to resume or
/// compensate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConservationSaga {
    pub id: SagaId,
    pub state: SagaState,
    pub reservations: Vec<ReservationId>,
    pub step_count: u32,
    /// Bit i set ⇔ step i has committed. Idempotency: a set bit skips.
    pub completed_steps: u64,
    pub deadline_tick: u64,
}

impl ConservationSaga {
    pub fn new(id: SagaId, step_count: u32, deadline_tick: u64) -> Self {
        assert!(step_count <= 64, "saga step bitset is u64");
        Self {
            id,
            state: SagaState::Running,
            reservations: Vec::new(),
            step_count,
            completed_steps: 0,
            deadline_tick,
        }
    }

    fn is_done(&self, step: u32) -> bool { self.completed_steps & (1 << step) != 0 }

    fn mark_done(&mut self, step: u32) { self.completed_steps |= 1 << step; }

    fn mark_undone(&mut self, step: u32) { self.completed_steps &= !(1 << step); }

    /// Drive the saga forward: apply each not-yet-committed step in order
    /// (idempotent — a committed step is skipped). `apply_step(i)` returns
    /// Ok on commit, Err to trigger compensation. On any step failure the
    /// coordinator compensates every ALREADY-committed step in REVERSE
    /// order and ends `Compensated`; full success ends `Completed`.
    pub fn drive<E>(
        &mut self,
        mut apply_step: impl FnMut(u32) -> Result<(), E>,
        mut compensate_step: impl FnMut(u32),
    ) -> SagaState {
        if self.state != SagaState::Running {
            return self.state;
        }
        for step in 0..self.step_count {
            if self.is_done(step) {
                continue; // idempotent skip
            }
            match apply_step(step) {
                Ok(()) => self.mark_done(step),
                Err(_) => {
                    self.state = SagaState::Compensating;
                    // Compensate committed steps in REVERSE order — restore
                    // ownership; conservation holds (nothing fabricated or
                    // deleted, only moved back).
                    for done in (0..self.step_count).rev() {
                        if self.is_done(done) {
                            compensate_step(done);
                            self.mark_undone(done);
                        }
                    }
                    self.state = SagaState::Compensated;
                    return self.state;
                },
            }
        }
        self.state = SagaState::Completed;
        self.state
    }
}

#[cfg(test)]
mod t1_5_tests {
    use super::*;

    /// A toy conservation model: a fixed total moves across three accounts
    /// through saga steps; commit-then-compensate must preserve the total.
    #[test]
    fn t1_5_full_success_completes_and_conserves() {
        let mut accounts = [10i64, 0, 0];
        let total: i64 = accounts.iter().sum();
        let mut saga = ConservationSaga::new(SagaId(1), 2, 1000);
        let state = saga.drive::<()>(
            |step| {
                match step {
                    0 => {
                        accounts[0] -= 4;
                        accounts[1] += 4;
                    },
                    1 => {
                        accounts[1] -= 4;
                        accounts[2] += 4;
                    },
                    _ => unreachable!(),
                }
                Ok(())
            },
            |_| unreachable!("no compensation on success"),
        );
        assert_eq!(state, SagaState::Completed);
        assert_eq!(accounts.iter().sum::<i64>(), total, "quantity conserved");
        assert_eq!(accounts, [6, 0, 4]);
    }

    #[test]
    fn t1_5_step_failure_compensates_in_reverse_and_conserves() {
        use std::cell::RefCell;
        let accounts = RefCell::new([10i64, 0, 0]);
        let total: i64 = accounts.borrow().iter().sum();
        let mut saga = ConservationSaga::new(SagaId(2), 3, 1000);
        let state = saga.drive(
            |step| {
                let mut a = accounts.borrow_mut();
                match step {
                    0 => {
                        a[0] -= 4;
                        a[1] += 4;
                        Ok(())
                    },
                    1 => {
                        a[1] -= 4;
                        a[2] += 4;
                        Ok(())
                    },
                    2 => Err(()), // fails — triggers compensation
                    _ => unreachable!(),
                }
            },
            |step| {
                let mut a = accounts.borrow_mut();
                match step {
                    // Compensation RESTORES (reverse of the committed step)
                    // — never fabricates or deletes, only moves back.
                    1 => {
                        a[2] -= 4;
                        a[1] += 4;
                    },
                    0 => {
                        a[1] -= 4;
                        a[0] += 4;
                    },
                    _ => unreachable!(),
                }
            },
        );
        assert_eq!(state, SagaState::Compensated);
        // Fully restored: total conserved AND back to the start.
        assert_eq!(accounts.borrow().iter().sum::<i64>(), total);
        assert_eq!(*accounts.borrow(), [10, 0, 0]);
        assert_eq!(saga.completed_steps, 0, "all steps compensated");
    }

    #[test]
    fn t1_5_idempotent_resume_skips_committed_steps() {
        let mut calls = Vec::new();
        let mut saga = ConservationSaga::new(SagaId(3), 3, 1000);
        // Simulate a crash after step 0: mark it done, resume.
        saga.completed_steps = 0b001;
        saga.drive::<()>(
            |step| {
                calls.push(step);
                Ok(())
            },
            |_| {},
        );
        // Step 0 was already committed — resume applies only 1 and 2.
        assert_eq!(calls, vec![1, 2]);
        assert_eq!(saga.state, SagaState::Completed);
    }
}
