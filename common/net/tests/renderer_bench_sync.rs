//! renderer-bench W1 (interface `synced_entity_id`): the stable semantic id
//! is a registered synced component and travels the NORMAL replication path
//! (W0 invariant — no bespoke channel). This test binds the registration:
//! if `RendererBenchEntityId` falls out of the synced-components x-macro or
//! loses its `NetSync` impl, this file stops compiling — the test IS the
//! registration's compile-time witness, plus a serde round-trip for the
//! payload itself.

use common::comp::bastion::RendererBenchEntityId;
use veloren_common_net::sync::{NetSync, SyncFrom};

/// Compile-time: the component implements NetSync with the expected policy.
/// (A const read is a use the compiler cannot dead-strip silently.)
const _: () = {
    // Exhaustive match, no wildcard: every variant named per contract style.
    match <RendererBenchEntityId as NetSync>::SYNC_FROM {
        SyncFrom::AnyEntity => {},
        SyncFrom::ClientSpectatorEntity => {},
        SyncFrom::ClientEntity => {},
    }
};

#[test]
fn sync_policy_is_any_entity() {
    assert!(matches!(
        <RendererBenchEntityId as NetSync>::SYNC_FROM,
        SyncFrom::AnyEntity
    ));
}

#[test]
fn payload_round_trips_and_is_stable() {
    let id = RendererBenchEntityId(42);
    let ser = bincode::serialize(&id).expect("serializes");
    let de: RendererBenchEntityId = bincode::deserialize(&ser).expect("deserializes");
    assert_eq!(de, id);
    // The wire payload is exactly the u32 (no hidden fields drifting in).
    assert_eq!(ser.len(), 4, "RendererBenchEntityId must stay a bare u32");
}
