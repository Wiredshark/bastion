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
pub mod bastion_mood;
// Gated exactly as it was in bastion-server, so moving it does not
// change which builds compile it.
#[cfg(not(feature = "worldgen"))]
pub mod test_world;
