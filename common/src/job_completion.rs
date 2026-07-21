//! T1.14 (Bastion conservation cluster): the job-completion plan — a
//! completion's authoritative effects staged as ONE validated batch so the
//! completion is all-or-nothing at the conservation level: it can never
//! clear a loot-bearing block without creating its item, nor create a drop
//! with no block removed. Built in the spirit of the T1.2
//! [`crate::effect_journal`] two-phase discipline (validate BEFORE any
//! authoritative mutation): the pairing invariant is checked at
//! [`JobCompletionPlan::validate`], and only a validated plan is applied.
//!
//! Determinism story (Ben's law): validation is a pure count/parity check
//! over the staged effects — no RNG, no wall-clock, no iteration-order
//! dependence. The commit applies staged ops in the order they were staged.

use crate::command_protocol::CommandId;
use serde::{Deserialize, Serialize};
use vek::Vec3;

/// A completion effect that removes a block. `yields` marks whether this
/// clear must produce a paired item (a mined/felled loot block) versus an
/// inert structural clear (a severed support cell that drops nothing).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionClear {
    pub cell: Vec3<i32>,
    pub yields: bool,
}

/// A completion effect that creates a world item drop at `pos`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionDrop {
    pub pos: Vec3<i32>,
}

/// Why a completion plan failed validation (before any authoritative write).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionError {
    /// The number of loot-yielding clears does not equal the number of item
    /// drops — a block would vanish with no item (loss) or an item would
    /// appear with no block removed (dupe).
    YieldDropImbalance { yielding_clears: usize, drops: usize },
}

/// T1.14: the staged, validate-before-commit batch of one job completion's
/// authoritative effects. Callers stage every clear / drop / inventory
/// delta, [`validate`](Self::validate) once (the conservation gate), then
/// apply — never applying a partial completion.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JobCompletionPlan {
    pub command: Option<CommandId>,
    pub clears: Vec<CompletionClear>,
    pub drops: Vec<CompletionDrop>,
    /// Signed inventory deltas `(owner_uid, amount)` — deposits (+) and
    /// consumes (−). Kept for the batch's completeness; net double-entry
    /// accounting rides T1.31.
    pub inventory_deltas: Vec<(u64, i64)>,
}

impl JobCompletionPlan {
    pub fn new(command: Option<CommandId>) -> Self {
        Self {
            command,
            clears: Vec::new(),
            drops: Vec::new(),
            inventory_deltas: Vec::new(),
        }
    }

    /// Stage a block clear that yields a paired item drop.
    pub fn yield_clear(&mut self, cell: Vec3<i32>, drop_pos: Vec3<i32>) -> &mut Self {
        self.clears.push(CompletionClear { cell, yields: true });
        self.drops.push(CompletionDrop { pos: drop_pos });
        self
    }

    /// Stage an inert (non-yielding) block clear — no paired drop.
    pub fn inert_clear(&mut self, cell: Vec3<i32>) -> &mut Self {
        self.clears.push(CompletionClear { cell, yields: false });
        self
    }

    /// Stage a signed inventory delta.
    pub fn inventory_delta(&mut self, owner_uid: u64, amount: i64) -> &mut Self {
        self.inventory_deltas.push((owner_uid, amount));
        self
    }

    /// The conservation gate (T1.14): every loot-yielding clear must be
    /// paired with exactly one item drop. Call BEFORE applying any effect;
    /// an imbalanced plan is a completion bug and must be rejected whole,
    /// never applied partially.
    pub fn validate(&self) -> Result<(), CompletionError> {
        let yielding_clears = self.clears.iter().filter(|c| c.yields).count();
        let drops = self.drops.len();
        if yielding_clears != drops {
            return Err(CompletionError::YieldDropImbalance {
                yielding_clears,
                drops,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod t1_14_tests {
    use super::*;

    fn cell(x: i32) -> Vec3<i32> { Vec3::new(x, 0, 0) }

    #[test]
    fn t1_14_balanced_completion_validates() {
        let mut plan = JobCompletionPlan::new(Some(CommandId(1)));
        plan.yield_clear(cell(0), cell(0))
            .yield_clear(cell(1), cell(1))
            .inert_clear(cell(2)) // a severed support cell — no drop
            .inventory_delta(42, -1);
        assert_eq!(plan.validate(), Ok(()));
        // Two yielding clears, two drops; the inert clear is not counted.
        assert_eq!(plan.clears.iter().filter(|c| c.yields).count(), 2);
        assert_eq!(plan.drops.len(), 2);
    }

    #[test]
    fn t1_14_clear_without_drop_is_a_conservation_leak() {
        let mut plan = JobCompletionPlan::new(None);
        plan.clears.push(CompletionClear {
            cell: cell(0),
            yields: true,
        }); // a yielding clear with NO paired drop staged
        assert_eq!(plan.validate(), Err(CompletionError::YieldDropImbalance {
            yielding_clears: 1,
            drops: 0,
        }));
    }

    #[test]
    fn t1_14_drop_without_clear_is_a_dupe() {
        let mut plan = JobCompletionPlan::new(None);
        plan.drops.push(CompletionDrop { pos: cell(0) }); // orphan drop
        assert_eq!(plan.validate(), Err(CompletionError::YieldDropImbalance {
            yielding_clears: 0,
            drops: 1,
        }));
    }
}
