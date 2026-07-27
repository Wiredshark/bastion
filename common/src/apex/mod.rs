//! Shared foundation for the APEX determinism program (`APEX-T0.*`).
//!
//! Determinism story: everything under `apex` is boundary machinery, not
//! game logic. It exists so that later manifest/wire/hash/RNG-seed schemas
//! cannot accidentally encode a native-width integer or native-endian byte
//! order into authoritative state. See `readme/apex/` for the program's
//! packets; this module implements `APEX-T0.1`.

pub mod boundary;
pub mod digest;
pub mod identity;
pub mod manifest;
pub mod scalar;
pub mod source_closure;

pub use boundary::AuthoritativeBoundaryKindV1;
pub use scalar::{
    BoundaryScalarError, CanonicalByteLength, CanonicalCount, CanonicalOrdinal, CanonicalSequence,
    FixedWidthScalar, ProtocolVersion, SchemaVersion,
};

// NOTE (scope decision, recorded honestly rather than silently dropped):
// the T0.1 packet's section 7.6/7.7 "legacy remediation types" (a
// u64-backed TradeId/OverflowSlotId replacing the live usize-backed types
// in common::trade / common::comp::inventory::slot, and a little-endian
// WorldSeedExpansionVersion replacing world/src/util/seed_expan.rs's
// to_ne_bytes call) are cross-cutting live-code migrations touching
// server/client/network call sites this pass did not attempt to compile
// and verify end-to-end. They are tracked as explicit blocking-migration
// rows in readme/apex/APEX-BOUNDARY-INVENTORY-SEED-v1.csv with
// status=LEGACY_VIOLATION rather than silently marked closed. This module
// provides the foundation (sealed trait + macro + protocol-neutral
// scalars) that migration depends on; the migration itself is deferred.
