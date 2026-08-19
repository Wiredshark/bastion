//! Shared bastion surface consumed by `veloren-server`.
//!
//! ★ WHY THIS CRATE EXISTS (speed row, 2026-08-19). `veloren-server` used to
//! depend on `bastion-server`, so a one-line edit to the 23k-line job logic
//! forced a 9.95 s `veloren-server` rebuild — measured at 48% of a warm
//! iteration, and unavoidable by any rearrangement BELOW the dependent.
//! Everything `veloren-server` needs lives here instead; the job logic sits
//! ABOVE it and no longer cascades.

// `bastion_assets` keeps its ORIGINAL `worldgen` gate — the same class of
// silent semantic change that test_world nearly suffered.
#[cfg(feature = "worldgen")]
pub mod bastion_assets;
pub mod bastion_flight_recorder;
pub mod bastion_jobs_core;
pub mod bastion_traversal;
pub mod bastion_mood;
// Gated exactly as it was in bastion-server, so moving it does not
// change which builds compile it.
#[cfg(not(feature = "worldgen"))]
pub mod test_world;

use serde::{Deserialize, Serialize};
use specs::{Component, VecStorage};

// Tick count used for throttling network updates
// Note this doesn't account for dt (so update rate changes with tick rate)
// (moved from veloren-server lib.rs in the crate-split; the field is `pub`
// now that its server-side users live in a different crate)
#[derive(Copy, Clone, Default)]
pub struct Tick(pub u64);

/// T0.3 (master build order; ledger #39): THE declared simulation clock —
/// the fixed-step cadence (ticks per simulated second) every
/// tick-denominated budget derives from. The server loop targets 30 tps
/// (`Settings` tick rate; the headless harness runs the same fixed step
/// uncapped), and before this constant existed the 30 was scattered as
/// magic through mount/exit/stability/energy-wait/teleport budgets — a
/// cadence change would have skewed every budget silently and
/// independently.
pub const SIM_TPS: u64 = 30;

// (moved from veloren-server presence.rs in the crate-split; re-exported there)
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepositionToFreeSpace {
    pub needs_ground: bool,
    pub modify_waypoints: bool,
}

impl Component for RepositionToFreeSpace {
    type Storage = VecStorage<Self>;
}
