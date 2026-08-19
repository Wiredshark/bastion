//! bastion-server: the bastion server-side systems/logic leaf crate.
//!
//! Pure structural extraction of the 12 `bastion_*` modules from
//! `veloren-server` (see `readme/CRATE-SPLIT-BASTION-SERVER-PACKET.md`) so a
//! job-logic edit recompiles this leaf + a server relink instead of all of
//! `veloren-server`. Same pattern as `common`/`common-ecs`/`common-state`:
//! nothing here is new code — behavior is byte-identical by the split's
//! acceptance bar.
//!
//! `veloren-server` re-exports every moved item at its old path (`Tick`,
//! `presence::RepositionToFreeSpace`, the `bastion_*` modules, `test_world`),
//! so existing `crate::…` references in the server and `server::…` references
//! in the harness compile unchanged.

use serde::{Deserialize, Serialize};
use specs::{Component, VecStorage};

pub mod bastion_actions;
// (bastion_arena stayed in veloren-server: it is an `impl Server` integration
// shim — an inherent impl on the server type cannot live in a leaf crate.)
pub mod bastion_chop;
pub mod bastion_entity_event_log;
pub mod bastion_flat_arena;
pub mod bastion_founding_preset;
pub mod bastion_jobs;
pub mod bastion_path;
pub mod bastion_piles;
pub mod bastion_traversal_tooling;
// ★ SPEED ROW: these moved to `bastion-core` so `veloren-server` can depend
// on that crate instead of this one. Re-exported so every existing
// `bastion_server::…` path keeps working unchanged.
// ★ test_world keeps its ORIGINAL cfg gate — removing `pub mod test_world;`
// orphaned the `#[cfg(not(feature = "worldgen"))]` that guarded it, which
// would have silently compiled it into worldgen builds for the first time.
pub use bastion_core::{RepositionToFreeSpace, SIM_TPS, Tick};
pub use bastion_core::bastion_mood;
pub use bastion_core::bastion_traversal;
pub use bastion_core::bastion_flight_recorder;
#[cfg(feature = "worldgen")]
pub use bastion_core::bastion_assets;
#[cfg(not(feature = "worldgen"))]
pub use bastion_core::test_world;




