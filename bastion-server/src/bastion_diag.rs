//! Diagnostic-only wall-clock access, DELIBERATELY outside the T0.2
//! labor-path ban list (`t0_2_labor_paths_declare_sim_clock_only`).
//!
//! The ban exists because labor DURATIONS must be sim-clock only — a
//! wall-clock read that feeds a job timer breaks determinism and pause
//! semantics (the LootOwner/ENGOPT6 class). Profiling spans are the one
//! legitimate wall-clock consumer: they measure how long the SERVER took,
//! never how long the WORK takes, and their values flow only into log
//! lines. Nothing returned from this module may feed sim state; a caller
//! that does so is reintroducing the exact bug the ban names.
pub fn span_start() -> std::time::Instant {
    std::time::Instant::now()
}
