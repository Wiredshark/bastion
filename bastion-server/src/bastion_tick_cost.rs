//! bastion (ITEM 39): how long a tick takes in WALL time.
//!
//! ## Why this is its own module
//!
//! `bastion_jobs.rs` is on the T0.2 banned list: no `Instant::now`, no
//! `SystemTime::now`, because **labor durations are sim-clock only**. A mining
//! job that took "3 real seconds" would run at a different rate on a loaded
//! host than a quiet one, and every A/B arm in this project would stop being
//! comparable.
//!
//! Item 39's profiler tripped that guard, correctly. But "how many microseconds
//! did this tick cost" is a genuinely WALL question — sim time cannot answer it,
//! because sim time is a fixed step by construction and would report the same
//! number on a machine ten times slower. The standing law for exactly this case
//! is *answer wall questions elsewhere*, so the timer lives here rather than
//! being granted an exemption inside the labor path. Moving it is not evading
//! the guard: the guard protects labor durations, and this value is not one.
//!
//! ## The contract this module exists to keep
//!
//! **The measurement may be LOGGED and never READ BY GAMEPLAY.** No decision,
//! generator, gate, threshold or score may consume it. The moment something
//! branches on it, tick behaviour depends on host speed and determinism is
//! gone — silently, because a wall-coupled colony still runs and still looks
//! right, and only a same-seed comparison would ever show it.
//!
//! `tick_cost_has_no_gameplay_consumer` in `bastion_traversal_tooling` pins
//! that, because a comment cannot enforce it.

/// A started wall-clock measurement. Opaque on purpose — it hands out a
/// duration and nothing else, so there is no clock here for gameplay to read.
pub struct TickTimer(std::time::Instant);

/// Begin timing a tick.
pub fn start() -> TickTimer { TickTimer(std::time::Instant::now()) }

impl TickTimer {
    /// Microseconds since [`start`]. Diagnostics only — see the module contract.
    pub fn elapsed_us(&self) -> u64 { self.0.elapsed().as_micros() as u64 }
}
