//! Opaque UUIDv4 lifecycle identities (`APEX-T0.4`, packet sections
//! 7.2, 7.5, 7.6).
//!
//! Determinism story: these are collection-orderable *opaque* identities,
//! not causal/creation-time identities. Ordering is unsigned lexicographic
//! comparison of the raw 16 UUID octets — a deterministic, platform-
//! independent tiebreaker for sorting/storage, never a substitute for an
//! explicit epoch/generation counter and never interpreted as "earlier" or
//! "later" in real time.

use uuid::Uuid;

use super::error::{IdentityDecodeErrorV1, IdentityGenerationErrorV1};

/// Supplies raw random octets only — never a preformatted UUID. Every
/// opaque identity constructor routes through `uuid::Builder::
/// from_random_bytes`, which is the single owner of UUIDv4 version/variant
/// bit-layout (packet section 7.5's "pre_masked_entropy_contract_forbidden"
/// negative canary: a source that tries to hand back an already-versioned
/// UUID is not the contract).
pub trait IdRandomBytesSourceV1 {
    fn fill_random_bytes(&mut self, out: &mut [u8; 16]) -> Result<(), IdentityGenerationErrorV1>;
}

/// Production entropy source: OS-backed CSPRNG via `rand`.
pub struct OsRandomBytesSourceV1;

impl IdRandomBytesSourceV1 for OsRandomBytesSourceV1 {
    fn fill_random_bytes(&mut self, out: &mut [u8; 16]) -> Result<(), IdentityGenerationErrorV1> {
        use rand::Rng;
        rand::rng().fill_bytes(out);
        Ok(())
    }
}

/// Test-only fixed-byte source (all-zero, all-one, and arbitrary patterns
/// used by the golden-vector corpus).
pub struct FixedRandomBytesSourceV1(pub [u8; 16]);

impl IdRandomBytesSourceV1 for FixedRandomBytesSourceV1 {
    fn fill_random_bytes(&mut self, out: &mut [u8; 16]) -> Result<(), IdentityGenerationErrorV1> {
        *out = self.0;
        Ok(())
    }
}

fn generate_uuid_v4(source: &mut impl IdRandomBytesSourceV1) -> Result<Uuid, IdentityGenerationErrorV1> {
    let mut random = [0u8; 16];
    source.fill_random_bytes(&mut random)?;
    Ok(uuid::Builder::from_random_bytes(random).into_uuid())
}

fn validate_uuid_v4(uuid: &Uuid) -> Result<(), IdentityDecodeErrorV1> {
    if uuid.is_nil() {
        return Err(IdentityDecodeErrorV1::NilUuid);
    }
    if uuid.get_version_num() != 4 {
        return Err(IdentityDecodeErrorV1::WrongUuidVersion { actual: Some(uuid.get_version_num() as u8) });
    }
    if uuid.get_variant() != uuid::Variant::RFC4122 {
        return Err(IdentityDecodeErrorV1::WrongUuidVariant);
    }
    Ok(())
}

/// Unsigned lexicographic order over the raw 16 octets. Manual, not
/// derived: derived `Ord` would silently become a canonical-order contract
/// tied to `uuid::Uuid`'s own internal representation, which this V1 must
/// not depend on (packet section 7.2's revalidation correction).
fn byte_order(a: &Uuid, b: &Uuid) -> core::cmp::Ordering { a.as_bytes().cmp(b.as_bytes()) }

macro_rules! opaque_lifecycle_id {
    ($(#[$meta:meta])* $name:ident, $text_prefix:literal) => {
        $(#[$meta])*
        // Serde added at APEX-T3.1 (see readme/apex/
        // APEX-T3.1-T0.4-ABI-REVALIDATION.md): T0.4 deliberately omitted
        // it ("live wire migration belongs to owning rows"); T3.1 is that
        // owning row -- these IDs now cross the existing bincode-legacy
        // wire protocol (ServerInfo/ClientRegister/GameSync), not just
        // BastionManifestEncodingV1. Manual impl below, NOT #[derive] --
        // deriving would inherit uuid::Uuid's own Serde impl, which
        // length-prefixes the bytes under bincode (24 bytes on the wire,
        // not the compact 16 the packet's acceptance gate asks for) and
        // skips version/variant revalidation on decode.
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq, Hash)]
        pub struct $name(Uuid);

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.as_bytes().serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let bytes: [u8; 16] = serde::Deserialize::deserialize(deserializer)?;
                Self::from_uuid_v4(Uuid::from_bytes(bytes))
                    .map_err(|_| serde::de::Error::custom(concat!("invalid ", stringify!($name), ": not a valid UUIDv4")))
            }
        }

        impl $name {
            pub fn generate(source: &mut impl IdRandomBytesSourceV1) -> Result<Self, IdentityGenerationErrorV1> {
                let uuid = generate_uuid_v4(source)?;
                Self::from_uuid_v4(uuid).map_err(|_| IdentityGenerationErrorV1::GeneratedInvariantViolation)
            }

            pub fn from_uuid_v4(uuid: Uuid) -> Result<Self, IdentityDecodeErrorV1> {
                validate_uuid_v4(&uuid)?;
                Ok(Self(uuid))
            }

            pub fn as_uuid(&self) -> &Uuid { &self.0 }

            /// Parses `"<prefix>/<hyphenated-lowercase-uuid>"`.
            pub fn from_text_v1(s: &str) -> Result<Self, IdentityDecodeErrorV1> {
                let rest = s.strip_prefix(concat!($text_prefix, "/")).ok_or(IdentityDecodeErrorV1::WrongTextPrefix)?;
                let uuid = Uuid::parse_str(rest).map_err(|_| IdentityDecodeErrorV1::InvalidText)?;
                Self::from_uuid_v4(uuid)
            }

            pub fn to_text_v1(&self) -> String { format!(concat!($text_prefix, "/{}"), self.0.hyphenated()) }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.to_text_v1())
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering { byte_order(&self.0, &other.0) }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(other)) }
        }
    };
}

opaque_lifecycle_id!(
    /// Identifies one server process lifetime. New on every server boot;
    /// invalidates prior sessions/replication state (owned/issued by `T3.1`).
    ServerBootId,
    "boot"
);
opaque_lifecycle_id!(
    /// Identifies one authenticated application attachment (owned/issued by `T3.2`).
    SessionId,
    "session"
);
opaque_lifecycle_id!(
    /// Identifies one discrete, deduplicatable client command (owned/issued by `T3.5`).
    CommandId,
    "command"
);
opaque_lifecycle_id!(
    /// Identifies one save/world lineage branch (owned/issued by `T4`).
    UniverseBranchId,
    "branch"
);

/// Generic evidence-envelope tag for contexts where the static Rust type
/// is unavailable (registry/evidence records). Never used in gameplay
/// APIs — those always use the concrete typed identity. `APEX-T0.4.6`
/// (completing T0.4's own "tagged canonical encodings" contract, Opus 5's
/// boundary-review finding): also the embedded type discriminant in each
/// opaque identity's canonical `BastionManifestEncodingV1` form, closing
/// the cross-type substitution hole a bare 16-byte bytestring left open
/// (a `ServerBootId`'s encoded bytes could not otherwise be distinguished
/// from a `SessionId`'s at decode time, since both are valid UUIDv4 byte
/// strings).
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityKindV1 {
    ServerBoot = 1,
    Session = 2,
    Command = 3,
    UniverseBranch = 4,
}

impl IdentityKindV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    pub const ALL: [IdentityKindV1; 4] = [Self::ServerBoot, Self::Session, Self::Command, Self::UniverseBranch];

    pub fn try_from_u16(raw: u16) -> Option<Self> { Self::ALL.into_iter().find(|k| k.as_u16() == raw) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_kind_ids_are_unique_and_round_trip() {
        use std::collections::HashSet;
        let ids: HashSet<u16> = IdentityKindV1::ALL.iter().map(|k| k.as_u16()).collect();
        assert_eq!(ids.len(), IdentityKindV1::ALL.len());
        assert_eq!(IdentityKindV1::ServerBoot.as_u16(), 1);
        assert_eq!(IdentityKindV1::Session.as_u16(), 2);
        assert_eq!(IdentityKindV1::Command.as_u16(), 3);
        assert_eq!(IdentityKindV1::UniverseBranch.as_u16(), 4);
        for k in IdentityKindV1::ALL {
            assert_eq!(IdentityKindV1::try_from_u16(k.as_u16()), Some(k));
        }
        assert_eq!(IdentityKindV1::try_from_u16(0), None);
        assert_eq!(IdentityKindV1::try_from_u16(5), None);
    }

    fn hex_to_16(s: &str) -> [u8; 16] {
        let v: Vec<u8> = (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    #[test]
    fn from_random_bytes_all_zero() {
        let mut source = FixedRandomBytesSourceV1([0u8; 16]);
        let id = ServerBootId::generate(&mut source).unwrap();
        assert_eq!(id.as_uuid().hyphenated().to_string(), "00000000-0000-4000-8000-000000000000");
    }

    #[test]
    fn from_random_bytes_all_one() {
        let mut source = FixedRandomBytesSourceV1([0xFFu8; 16]);
        let id = ServerBootId::generate(&mut source).unwrap();
        assert_eq!(id.as_uuid().hyphenated().to_string(), "ffffffff-ffff-4fff-bfff-ffffffffffff");
    }

    #[test]
    fn version_and_variant_bits_are_overwritten() {
        let random = hex_to_16("919108f752d11320dbacf847db4148a8");
        let mut source = FixedRandomBytesSourceV1(random);
        let id = ServerBootId::generate(&mut source).unwrap();
        assert_eq!(id.as_uuid().hyphenated().to_string(), "919108f7-52d1-4320-9bac-f847db4148a8");
    }

    /// Diagnostic, not an assertion of desired behavior: confirms exactly
    /// which bincode primitive a raw `[u8; 16]` array uses under legacy
    /// config, to compare against `Uuid`'s own Serde impl below.
    #[test]
    fn diagnostic_raw_fixed_array_bincode_length() {
        let arr: [u8; 16] = [0u8; 16];
        let bytes = bincode::serde::encode_to_vec(arr, bincode::config::legacy()).unwrap();
        println!("raw [u8;16] bincode length: {}", bytes.len());
        assert_eq!(bytes.len(), 16, "a fixed-size array has no length prefix under bincode legacy");
    }

    /// The manual Serde impl (not derived -- see the macro's doc comment
    /// for why) produces exactly the compact 16-byte wire form the T3.1
    /// packet's acceptance gate asks for ("client receives full 16-byte
    /// ID"), confirmed against the real byte content, not just a length.
    #[test]
    fn bincode_legacy_round_trip_is_exactly_sixteen_raw_bytes() {
        let random = hex_to_16("919108f752d11320dbacf847db4148a8");
        let mut source = FixedRandomBytesSourceV1(random);
        let id = ServerBootId::generate(&mut source).unwrap();

        let bytes = bincode::serde::encode_to_vec(id, bincode::config::legacy()).unwrap();
        assert_eq!(bytes.len(), 16, "expected exactly the 16 raw UUID bytes, got {} bytes: {bytes:02x?}", bytes.len());
        assert_eq!(&bytes, id.as_uuid().as_bytes());

        let (decoded, consumed): (ServerBootId, usize) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::legacy()).unwrap();
        assert_eq!(consumed, 16);
        assert_eq!(decoded, id);
    }

    /// Wire deserialization revalidates version/variant bits -- a peer
    /// cannot smuggle a nil UUID or a non-v4 UUID onto the wire and have
    /// it silently accepted as a valid opaque ID.
    #[test]
    fn bincode_deserialize_rejects_invalid_uuid_bytes() {
        let nil_bytes = [0u8; 16];
        let wire = bincode::serde::encode_to_vec(nil_bytes, bincode::config::legacy()).unwrap();
        let result: Result<(ServerBootId, usize), _> = bincode::serde::decode_from_slice(&wire, bincode::config::legacy());
        assert!(result.is_err(), "nil UUID must not decode as a valid ServerBootId");
    }

    #[test]
    fn canonical_ordering_is_unsigned_lexicographic() {
        let low = ServerBootId::from_text_v1("boot/00000000-0000-4000-8000-000000000000").unwrap();
        let mid = ServerBootId::from_text_v1("boot/919108f7-52d1-4320-9bac-f847db4148a8").unwrap();
        let high = ServerBootId::from_text_v1("boot/ffffffff-ffff-4fff-bfff-ffffffffffff").unwrap();
        let mut v = vec![high, mid, low];
        v.sort();
        assert_eq!(v, vec![low, mid, high]);
    }

    #[test]
    fn typed_text_round_trip() {
        assert_eq!(
            ServerBootId::from_text_v1("boot/919108f7-52d1-4320-9bac-f847db4148a8").unwrap().to_text_v1(),
            "boot/919108f7-52d1-4320-9bac-f847db4148a8"
        );
        assert_eq!(
            SessionId::from_text_v1("session/919108f7-52d1-4320-9bac-f847db4148a8").unwrap().to_text_v1(),
            "session/919108f7-52d1-4320-9bac-f847db4148a8"
        );
        assert_eq!(
            CommandId::from_text_v1("command/919108f7-52d1-4320-9bac-f847db4148a8").unwrap().to_text_v1(),
            "command/919108f7-52d1-4320-9bac-f847db4148a8"
        );
        assert_eq!(
            UniverseBranchId::from_text_v1("branch/919108f7-52d1-4320-9bac-f847db4148a8").unwrap().to_text_v1(),
            "branch/919108f7-52d1-4320-9bac-f847db4148a8"
        );
    }

    #[test]
    fn wrong_text_prefix_is_rejected() {
        let err = ServerBootId::from_text_v1("session/919108f7-52d1-4320-9bac-f847db4148a8").unwrap_err();
        assert_eq!(err, IdentityDecodeErrorV1::WrongTextPrefix);
    }

    #[test]
    fn nil_uuid_is_rejected() {
        let nil = Uuid::nil();
        let err = ServerBootId::from_uuid_v4(nil).unwrap_err();
        assert_eq!(err, IdentityDecodeErrorV1::NilUuid);
        assert_eq!(err.terminal_class(), "INVALID_UUID_VERSION_VARIANT");
    }

    #[test]
    fn uuid_v7_is_rejected() {
        // A UUIDv7 has version nibble 7; construct one directly (timestamp
        // content is irrelevant -- only the version/variant bits matter here).
        let mut bytes = [0u8; 16];
        bytes[6] = 0x70; // version nibble 7 in the high nibble of byte 6
        bytes[8] = 0x80; // RFC4122 variant
        let v7 = Uuid::from_bytes(bytes);
        assert_eq!(v7.get_version_num(), 7);
        let err = ServerBootId::from_uuid_v4(v7).unwrap_err();
        assert!(matches!(err, IdentityDecodeErrorV1::WrongUuidVersion { .. }));
        assert_eq!(err.terminal_class(), "INVALID_UUID_VERSION_VARIANT");
    }

    #[test]
    fn distinct_typed_ids_are_not_mutually_constructible() {
        // ServerBootId and SessionId are distinct nominal types generated by
        // the same macro -- there is no From<ServerBootId> for SessionId
        // anywhere in this module, so mixing them up is a compile error,
        // not a runtime check. This test documents that structurally.
        let boot = ServerBootId::from_text_v1("boot/919108f7-52d1-4320-9bac-f847db4148a8").unwrap();
        let session = SessionId::from_text_v1("session/919108f7-52d1-4320-9bac-f847db4148a8").unwrap();
        assert_eq!(boot.as_uuid(), session.as_uuid()); // same underlying UUID bytes...
        // ...but boot and session are different Rust types, so `boot ==
        // session` would not even compile. That's the actual guarantee.
    }
}
