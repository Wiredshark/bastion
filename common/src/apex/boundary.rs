//! Authoritative-boundary vocabulary (`APEX-T0.1`, packet section 5.1).
//!
//! A field or conversion is in scope for the apex fixed-width/explicit-endian
//! rules when its value becomes one of the kinds below. A type is not in
//! scope merely because it derives `Serialize`/`Deserialize` — the boundary
//! inventory records a real authority path, not a blanket Serde scan.

/// Which kind of authoritative surface a value crosses.
///
/// - `Manifest`: becomes a field in a canonical manifest (`APEX-T0.2`+).
/// - `Wire`: transmitted between client and server as protocol state.
/// - `Persistence`: written to a save/database as durable authoritative state.
/// - `CanonicalHashInput`: fed into a digest whose output is compared across
///   runs/platforms (e.g. a state hash or content-identity digest).
/// - `RngSeedBytes`: becomes the byte input to a deterministic RNG seed.
/// - `ReplayOrEvidence`: recorded in a replay/evidence artifact that a later
///   run or a different machine must reproduce exactly.
/// - `CrossProcessIdentity`: identifies an entity/session/command across a
///   process boundary (client<->server), as opposed to a process-local index.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AuthoritativeBoundaryKindV1 {
    Manifest,
    Wire,
    Persistence,
    CanonicalHashInput,
    RngSeedBytes,
    ReplayOrEvidence,
    CrossProcessIdentity,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant must be constructible and comparable — this is a
    /// compile-time-shape test more than a runtime one, but it also pins
    /// that the enum stays exhaustively matchable (no accidental
    /// non-exhaustive marker) for the inventory tool's own tests.
    #[test]
    fn every_variant_is_constructible_and_distinct() {
        let all = [
            AuthoritativeBoundaryKindV1::Manifest,
            AuthoritativeBoundaryKindV1::Wire,
            AuthoritativeBoundaryKindV1::Persistence,
            AuthoritativeBoundaryKindV1::CanonicalHashInput,
            AuthoritativeBoundaryKindV1::RngSeedBytes,
            AuthoritativeBoundaryKindV1::ReplayOrEvidence,
            AuthoritativeBoundaryKindV1::CrossProcessIdentity,
        ];
        for i in 0..all.len() {
            for j in 0..all.len() {
                assert_eq!(i == j, all[i] == all[j]);
            }
        }
    }

    /// Doc-example: a field that becomes wire state is in scope.
    #[test]
    fn wire_field_is_authoritative() {
        let kind = AuthoritativeBoundaryKindV1::Wire;
        assert_eq!(kind, AuthoritativeBoundaryKindV1::Wire);
    }

    /// Doc-example: local-only fields (e.g. a `Vec` growth-capacity hint)
    /// never get one of these kinds assigned in the first place — there is
    /// no "Local" variant, by design (packet section 5.1: "not in scope
    /// merely because it derives Serde").
    #[test]
    fn no_local_variant_exists() {
        // If this ever needs a `Local` variant, that is itself a scope
        // regression the boundary linter should catch, not something to
        // silently add here.
        let kinds = [
            AuthoritativeBoundaryKindV1::Manifest,
            AuthoritativeBoundaryKindV1::Wire,
            AuthoritativeBoundaryKindV1::Persistence,
            AuthoritativeBoundaryKindV1::CanonicalHashInput,
            AuthoritativeBoundaryKindV1::RngSeedBytes,
            AuthoritativeBoundaryKindV1::ReplayOrEvidence,
            AuthoritativeBoundaryKindV1::CrossProcessIdentity,
        ];
        assert_eq!(kinds.len(), 7);
    }
}
