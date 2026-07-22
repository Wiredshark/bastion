//! BUILD-007A10.1 — shared canonical protocol foundation.
//!
//! Deterministic CBOR (RFC 8949 Core Deterministic) with the stricter Bastion
//! profile: preferred/shortest integer and length encodings, definite lengths
//! only, nonnegative-integer map keys sorted by encoded bytes, floats forbidden,
//! duplicate keys / indefinite lengths / trailing bytes / nonpreferred encodings
//! rejected on decode. A decoder re-encodes and byte-compares before returning
//! [`ValidatedCanonicalBytesV1`]; only validated-canonical bytes receive an
//! admitted semantic hash. The frozen vectors in the design (§4.6) are the
//! golden truth — this encoder reproduces them byte-for-byte.

use crate::domain_hash;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The canonical value subset R0D semantic manifests use. Floats are absent by
/// construction (prohibited in canonical payloads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CborValue {
    Uint(u64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<CborValue>),
    /// Map with nonnegative integer keys. Encoded in ascending key order, which
    /// for uint keys equals canonical encoded-byte order.
    IntMap(BTreeMap<u64, CborValue>),
}

/// Major type in the top 3 bits, argument via the preferred (shortest) encoding.
/// CBOR uses big-endian for multi-byte arguments.
fn encode_head(out: &mut Vec<u8>, major: u8, arg: u64) {
    let m = major << 5;
    if arg < 24 {
        out.push(m | (arg as u8));
    } else if arg <= u64::from(u8::MAX) {
        out.push(m | 24);
        out.push(arg as u8);
    } else if arg <= u64::from(u16::MAX) {
        out.push(m | 25);
        out.extend_from_slice(&(arg as u16).to_be_bytes());
    } else if arg <= u64::from(u32::MAX) {
        out.push(m | 26);
        out.extend_from_slice(&(arg as u32).to_be_bytes());
    } else {
        out.push(m | 27);
        out.extend_from_slice(&arg.to_be_bytes());
    }
}

impl CborValue {
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            CborValue::Uint(n) => encode_head(out, 0, *n),
            CborValue::Bytes(b) => {
                encode_head(out, 2, b.len() as u64);
                out.extend_from_slice(b);
            },
            CborValue::Text(s) => {
                encode_head(out, 3, s.len() as u64);
                out.extend_from_slice(s.as_bytes());
            },
            CborValue::Array(a) => {
                encode_head(out, 4, a.len() as u64);
                for v in a {
                    v.encode(out);
                }
            },
            CborValue::IntMap(m) => {
                encode_head(out, 5, m.len() as u64);
                for (k, v) in m {
                    encode_head(out, 0, *k);
                    v.encode(out);
                }
            },
        }
    }

    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode(&mut out);
        out
    }
}

/// Convenience: build an integer-keyed map from `(tag, value)` pairs.
#[must_use]
pub fn int_map(pairs: Vec<(u64, CborValue)>) -> CborValue {
    CborValue::IntMap(pairs.into_iter().collect())
}

/// The outer envelope (§4.2). Frames the canonical payload with magic, version,
/// length, and the payload digest.
pub struct CanonicalEnvelopeV1 {
    pub payload: Vec<u8>,
}

impl CanonicalEnvelopeV1 {
    pub const MAGIC: [u8; 8] = *b"BSTR0D1\0";
    pub const VERSION: u16 = 1;

    #[must_use]
    pub fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }

    /// SHA-256 of the canonical payload (the `payload_sha256` field).
    #[must_use]
    pub fn payload_sha256(&self) -> [u8; 32] {
        Sha256::digest(&self.payload).into()
    }

    /// The full envelope bytes: magic || version_le || payload_len_le ||
    /// payload || payload_sha256.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(8 + 2 + 4 + self.payload.len() + 32);
        b.extend_from_slice(&Self::MAGIC);
        b.extend_from_slice(&Self::VERSION.to_le_bytes());
        b.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        b.extend_from_slice(&self.payload);
        b.extend_from_slice(&self.payload_sha256());
        b
    }

    /// SHA-256 of the complete envelope.
    #[must_use]
    pub fn envelope_sha256(&self) -> [u8; 32] {
        Sha256::digest(self.to_bytes()).into()
    }
}

/// Canonical bytes that survived decode + re-encode + byte-compare — the only
/// bytes eligible for an admitted semantic hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedCanonicalBytesV1(Vec<u8>);

/// Decode failures that must each reject rather than best-effort continue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalDecodeError {
    NonPreferredEncoding,
    IndefiniteLength,
    DuplicateMapKey,
    UnsortedMapKey,
    NonIntegerMapKey,
    FloatForbidden,
    TrailingBytes,
    Truncated,
    OversizedLength,
    UnsupportedMajor,
}

impl ValidatedCanonicalBytesV1 {
    /// Decode `bytes`, then re-encode and byte-compare. Any deviation from the
    /// canonical profile (nonpreferred int, indefinite length, unsorted /
    /// duplicate keys, floats, trailing bytes, truncation) rejects.
    pub fn validate(bytes: &[u8]) -> Result<Self, CanonicalDecodeError> {
        let mut dec = Decoder {
            b: bytes,
            pos: 0,
        };
        let v = dec.value()?;
        if dec.pos != bytes.len() {
            return Err(CanonicalDecodeError::TrailingBytes);
        }
        // Re-encode and byte-compare — the canonical-form gate.
        if v.to_bytes() != bytes {
            return Err(CanonicalDecodeError::NonPreferredEncoding);
        }
        Ok(Self(bytes.to_vec()))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The admitted semantic hash of these canonical bytes.
    #[must_use]
    pub fn semantic_hash(&self, domain: &str) -> [u8; 32] {
        domain_hash(domain, 1, 0, &self.0)
    }
}

/// A minimal canonical-profile decoder used only to validate (decode +
/// re-encode). It enforces preferred encoding, definite lengths, and sorted
/// unique integer map keys; everything else rejects.
struct Decoder<'a> {
    b: &'a [u8],
    pos: usize,
}

impl Decoder<'_> {
    fn byte(&mut self) -> Result<u8, CanonicalDecodeError> {
        let x = *self.b.get(self.pos).ok_or(CanonicalDecodeError::Truncated)?;
        self.pos += 1;
        Ok(x)
    }

    fn take(&mut self, n: usize) -> Result<&[u8], CanonicalDecodeError> {
        let end = self.pos.checked_add(n).ok_or(CanonicalDecodeError::OversizedLength)?;
        let s = self.b.get(self.pos..end).ok_or(CanonicalDecodeError::Truncated)?;
        self.pos = end;
        Ok(s)
    }

    /// Read a definite-length argument in preferred form.
    fn arg(&mut self, low: u8) -> Result<u64, CanonicalDecodeError> {
        match low {
            0..=23 => Ok(u64::from(low)),
            24 => {
                let v = u64::from(self.byte()?);
                if v < 24 {
                    return Err(CanonicalDecodeError::NonPreferredEncoding);
                }
                Ok(v)
            },
            25 => {
                let a = self.take(2)?;
                let v = u64::from(u16::from_be_bytes([a[0], a[1]]));
                if v <= u64::from(u8::MAX) {
                    return Err(CanonicalDecodeError::NonPreferredEncoding);
                }
                Ok(v)
            },
            26 => {
                let a = self.take(4)?;
                let v = u64::from(u32::from_be_bytes([a[0], a[1], a[2], a[3]]));
                if v <= u64::from(u16::MAX) {
                    return Err(CanonicalDecodeError::NonPreferredEncoding);
                }
                Ok(v)
            },
            27 => {
                let a = self.take(8)?;
                let v = u64::from_be_bytes(a.try_into().unwrap());
                if v <= u64::from(u32::MAX) {
                    return Err(CanonicalDecodeError::NonPreferredEncoding);
                }
                Ok(v)
            },
            28..=30 => Err(CanonicalDecodeError::OversizedLength),
            _ => Err(CanonicalDecodeError::IndefiniteLength), // 31 = indefinite
        }
    }

    fn value(&mut self) -> Result<CborValue, CanonicalDecodeError> {
        let ib = self.byte()?;
        let major = ib >> 5;
        let low = ib & 0x1f;
        match major {
            0 => Ok(CborValue::Uint(self.arg(low)?)),
            2 => {
                let n = self.arg(low)? as usize;
                Ok(CborValue::Bytes(self.take(n)?.to_vec()))
            },
            3 => {
                let n = self.arg(low)? as usize;
                let s = std::str::from_utf8(self.take(n)?)
                    .map_err(|_| CanonicalDecodeError::UnsupportedMajor)?;
                Ok(CborValue::Text(s.to_string()))
            },
            4 => {
                let n = self.arg(low)?;
                let mut a = Vec::new();
                for _ in 0..n {
                    a.push(self.value()?);
                }
                Ok(CborValue::Array(a))
            },
            5 => {
                let n = self.arg(low)?;
                let mut m = BTreeMap::new();
                let mut last: Option<u64> = None;
                for _ in 0..n {
                    // keys must be nonnegative integers (major 0), strictly ascending
                    let kib = self.byte()?;
                    if kib >> 5 != 0 {
                        return Err(CanonicalDecodeError::NonIntegerMapKey);
                    }
                    let k = self.arg(kib & 0x1f)?;
                    if let Some(l) = last {
                        if k <= l {
                            return Err(if k == l {
                                CanonicalDecodeError::DuplicateMapKey
                            } else {
                                CanonicalDecodeError::UnsortedMapKey
                            });
                        }
                    }
                    last = Some(k);
                    let v = self.value()?;
                    m.insert(k, v);
                }
                Ok(CborValue::IntMap(m))
            },
            7 => Err(CanonicalDecodeError::FloatForbidden), // simple/float/break
            _ => Err(CanonicalDecodeError::UnsupportedMajor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hex32, hex_bytes};

    /// The frozen RendererBenchManifestV1 empty vector (§4.6).
    fn empty_manifest() -> CborValue {
        int_map(vec![
            (0, CborValue::Uint(1)),                             // schema_version
            (1, CborValue::Text("r0d-empty-v1".to_string())),   // scenario_id
            (2, CborValue::Uint(60)),                            // simulation_tps
            (3, CborValue::Array(vec![])),                       // fixtures
            (4, CborValue::Array(vec![])),                       // cameras
            (5, CborValue::Array(vec![])),                       // expected_assets
            (6, CborValue::Array(vec![])),                       // expected_entities
            (7, CborValue::Bytes(vec![0u8; 32])),                // seed_root
        ])
    }

    /// The frozen stable-entity primitive vector (§4.6).
    fn entity_vector() -> CborValue {
        int_map(vec![
            (0, CborValue::Uint(1)),
            (1, CborValue::Uint(1)),
            (2, CborValue::Bytes(vec![0x11u8; 32])),
            (3, CborValue::Text("fixture/humanoid/0001".to_string())),
        ])
    }

    #[test]
    fn frozen_empty_manifest_payload_bytes() {
        let got = hex_bytes(&empty_manifest().to_bytes());
        assert_eq!(
            got,
            "a80001016c7230642d656d7074792d763102183c03800480058006800758200000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn frozen_empty_manifest_payload_and_envelope_sha256() {
        let env = CanonicalEnvelopeV1::new(empty_manifest().to_bytes());
        assert_eq!(
            hex32(&env.payload_sha256()),
            "a10d1bbe7d858d697d5ec07ba8f83adf259408a8d17155d6f89fac22ac08adb3"
        );
        assert_eq!(
            hex32(&env.envelope_sha256()),
            "43f193ecc6ae8b8fdb4305e2353d128d5f4ac7191c232587058e44e020e236b3"
        );
    }

    #[test]
    fn frozen_entity_vector_bytes_and_sha256() {
        let bytes = entity_vector().to_bytes();
        assert_eq!(
            hex_bytes(&bytes),
            "a40001010102582011111111111111111111111111111111111111111111111111111111111111110375666978747572652f68756d616e6f69642f30303031"
        );
        assert_eq!(
            hex32(&Sha256::digest(&bytes).into()),
            "9bac9fb88f770e3d68dad56a6b6ae6d9b22442575b9d913b6f44fcc5fc164251"
        );
    }

    #[test]
    fn round_trip_validates_canonical() {
        let bytes = empty_manifest().to_bytes();
        let v = ValidatedCanonicalBytesV1::validate(&bytes).expect("canonical");
        assert_eq!(v.as_bytes(), bytes.as_slice());
    }

    #[test]
    fn nonpreferred_integer_rejected() {
        // 0x18 0x00 encodes uint 0 in nonpreferred long form.
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0x18, 0x00]),
            Err(CanonicalDecodeError::NonPreferredEncoding)
        );
    }

    #[test]
    fn indefinite_length_rejected() {
        // 0x9f = indefinite-length array.
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0x9f, 0xff]),
            Err(CanonicalDecodeError::IndefiniteLength)
        );
    }

    #[test]
    fn duplicate_and_unsorted_map_keys_rejected() {
        // map(2) with keys 1,1 (duplicate)
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0xa2, 0x01, 0x00, 0x01, 0x00]),
            Err(CanonicalDecodeError::DuplicateMapKey)
        );
        // map(2) with keys 2,1 (unsorted)
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0xa2, 0x02, 0x00, 0x01, 0x00]),
            Err(CanonicalDecodeError::UnsortedMapKey)
        );
    }

    #[test]
    fn float_and_trailing_and_truncation_rejected() {
        // major 7 (float/simple)
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0xf9, 0x00, 0x00]),
            Err(CanonicalDecodeError::FloatForbidden)
        );
        // trailing byte after a complete uint
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0x01, 0x00]),
            Err(CanonicalDecodeError::TrailingBytes)
        );
        // truncated byte string (says 4 bytes, has 1)
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&[0x44, 0x11]),
            Err(CanonicalDecodeError::Truncated)
        );
    }
}
