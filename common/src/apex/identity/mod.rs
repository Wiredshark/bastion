//! Authoritative lifecycle identity foundations (`APEX-T0.4`).
//!
//! Two families, both **foundations only** — no live issuance, transport
//! integration, or subsystem wiring happens here (that's `T3`/`T4`'s job;
//! see each type's doc comment for its owning row):
//!
//! - Opaque UUIDv4 identities: [`ServerBootId`], [`SessionId`],
//!   [`CommandId`], [`UniverseBranchId`].
//! - Scoped monotonic counters: [`ConnectionEpoch`], [`PhysicsGeneration`],
//!   [`SnapshotEpoch`], [`SaveEpoch`].
//!
//! Determinism story: an opaque identity's *value* is randomly generated
//! (by design — it identifies an instance, not a derivable quantity) but
//! its *representation* is fully pinned: exactly 16 bytes, UUIDv4
//! version/variant bits owned by a single constructor path
//! (`uuid::Builder::from_random_bytes`), and a manual byte-lexicographic
//! `Ord` that never depends on `uuid::Uuid`'s own internal comparison
//! behavior. Counters are checked `u64`s that error rather than wrap.

mod codec;
mod counter;
mod error;
mod opaque;

pub use counter::{ConnectionEpoch, PhysicsGeneration, SaveEpoch, SnapshotEpoch};
pub use error::{CounterAdvanceErrorV1, IdentityDecodeErrorV1, IdentityGenerationErrorV1};
pub use opaque::{
    CommandId, FixedRandomBytesSourceV1, IdRandomBytesSourceV1, IdentityKindV1, OsRandomBytesSourceV1, ServerBootId,
    SessionId, UniverseBranchId,
};

#[cfg(test)]
mod golden_vector_tests {
    use super::*;
    use uuid::Uuid;

    fn hex_to_16(s: &str) -> [u8; 16] {
        let v: Vec<u8> = (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    /// `uuid_v4_from_random_bytes` vectors, exercised end to end through
    /// the public generate() API (not a reimplementation).
    #[test]
    fn uuid_v4_from_random_bytes_vectors() {
        let cases: [([u8; 16], &str); 3] = [
            ([0u8; 16], "00000000-0000-4000-8000-000000000000"),
            (hex_to_16("919108f752d11320dbacf847db4148a8"), "919108f7-52d1-4320-9bac-f847db4148a8"),
            ([0xFFu8; 16], "ffffffff-ffff-4fff-bfff-ffffffffffff"),
        ];
        for (random_bytes, expected) in cases {
            let mut source = FixedRandomBytesSourceV1(random_bytes);
            let id = ServerBootId::generate(&mut source).unwrap();
            assert_eq!(id.as_uuid().hyphenated().to_string(), expected, "random_bytes={random_bytes:02x?}");
        }
    }

    /// `negative_vectors`: `uuid_v7_rejected` and `nil_uuid_rejected`.
    #[test]
    fn negative_vectors_uuid_class() {
        let nil = Uuid::nil();
        assert_eq!(ServerBootId::from_uuid_v4(nil).unwrap_err().terminal_class(), "INVALID_UUID_VERSION_VARIANT");

        let mut v7_bytes = [0u8; 16];
        v7_bytes[6] = 0x70;
        v7_bytes[8] = 0x80;
        let v7 = Uuid::from_bytes(v7_bytes);
        assert_eq!(ServerBootId::from_uuid_v4(v7).unwrap_err().terminal_class(), "INVALID_UUID_VERSION_VARIANT");
    }

    /// `negative_vectors`: `connection_epoch_zero` and `counter_wrap`.
    #[test]
    fn negative_vectors_counter_class() {
        assert_eq!(ConnectionEpoch::new(0).unwrap_err().terminal_class(), "ZERO_RESERVED");
        let max = ConnectionEpoch::new(u64::MAX).unwrap();
        assert_eq!(max.checked_next().unwrap_err().terminal_class(), "COUNTER_EXHAUSTED");
    }

    /// `negative_vectors`: `wrong_text_prefix`.
    #[test]
    fn negative_vector_wrong_text_prefix() {
        let err = ServerBootId::from_text_v1("session/919108f7-52d1-4320-9bac-f847db4148a8").unwrap_err();
        assert_eq!(err.terminal_class(), "WRONG_TYPE_PREFIX");
    }
}
