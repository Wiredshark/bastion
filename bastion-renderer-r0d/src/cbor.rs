//! Bounded canonical CBOR subset and deterministic envelope for renderer
//! protocols.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const MAX_CANONICAL_INPUT_BYTES_V1: usize = 1_048_576;
pub const MAX_CANONICAL_BYTE_STRING_BYTES_V1: usize = 262_144;
pub const MAX_CANONICAL_TEXT_BYTES_V1: usize = 65_536;
pub const MAX_CANONICAL_COLLECTION_ITEMS_V1: usize = 4_096;
pub const MAX_CANONICAL_DEPTH_V1: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalValueV1 {
    Uint(u64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Self>),
    IntMap(BTreeMap<u64, Self>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalErrorV1 {
    InputTooLarge { actual: usize, maximum: usize },
    ByteStringTooLarge { actual: u64, maximum: usize },
    TextTooLarge { actual: u64, maximum: usize },
    CollectionTooLarge { actual: u64, maximum: usize },
    DepthExceeded { maximum: usize },
    NonPreferredEncoding,
    IndefiniteValue,
    DuplicateMapKey(u64),
    UnsortedMapKey { previous: u64, actual: u64 },
    NonIntegerMapKey,
    InvalidUtf8,
    FloatForbidden,
    UnsupportedMajor(u8),
    UnsupportedAdditionalInfo(u8),
    Truncated,
    TrailingBytes(usize),
    LengthOverflow,
    InvalidEnvelopeMagic,
    UnsupportedEnvelopeVersion(u16),
    EnvelopeDigestMismatch,
}

pub fn try_int_map(
    pairs: impl IntoIterator<Item = (u64, CanonicalValueV1)>,
) -> Result<CanonicalValueV1, CanonicalErrorV1> {
    let mut map = BTreeMap::new();
    for (key, value) in pairs {
        if map.insert(key, value).is_some() {
            return Err(CanonicalErrorV1::DuplicateMapKey(key));
        }
        if map.len() > MAX_CANONICAL_COLLECTION_ITEMS_V1 {
            return Err(CanonicalErrorV1::CollectionTooLarge {
                actual: usize_to_u64(map.len())?,
                maximum: MAX_CANONICAL_COLLECTION_ITEMS_V1,
            });
        }
    }
    Ok(CanonicalValueV1::IntMap(map))
}

impl CanonicalValueV1 {
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CanonicalErrorV1> {
        self.validate_shape(0)?;
        let mut output = Vec::new();
        self.encode(&mut output)?;
        if output.len() > MAX_CANONICAL_INPUT_BYTES_V1 {
            return Err(CanonicalErrorV1::InputTooLarge {
                actual: output.len(),
                maximum: MAX_CANONICAL_INPUT_BYTES_V1,
            });
        }
        Ok(output)
    }

    fn validate_shape(&self, depth: usize) -> Result<(), CanonicalErrorV1> {
        if depth > MAX_CANONICAL_DEPTH_V1 {
            return Err(CanonicalErrorV1::DepthExceeded {
                maximum: MAX_CANONICAL_DEPTH_V1,
            });
        }
        match self {
            Self::Uint(_) => Ok(()),
            Self::Bytes(bytes) => {
                if bytes.len() > MAX_CANONICAL_BYTE_STRING_BYTES_V1 {
                    return Err(CanonicalErrorV1::ByteStringTooLarge {
                        actual: usize_to_u64(bytes.len())?,
                        maximum: MAX_CANONICAL_BYTE_STRING_BYTES_V1,
                    });
                }
                Ok(())
            },
            Self::Text(text) => {
                if text.len() > MAX_CANONICAL_TEXT_BYTES_V1 {
                    return Err(CanonicalErrorV1::TextTooLarge {
                        actual: usize_to_u64(text.len())?,
                        maximum: MAX_CANONICAL_TEXT_BYTES_V1,
                    });
                }
                Ok(())
            },
            Self::Array(values) => {
                if values.len() > MAX_CANONICAL_COLLECTION_ITEMS_V1 {
                    return Err(CanonicalErrorV1::CollectionTooLarge {
                        actual: usize_to_u64(values.len())?,
                        maximum: MAX_CANONICAL_COLLECTION_ITEMS_V1,
                    });
                }
                for value in values {
                    value.validate_shape(depth + 1)?;
                }
                Ok(())
            },
            Self::IntMap(values) => {
                if values.len() > MAX_CANONICAL_COLLECTION_ITEMS_V1 {
                    return Err(CanonicalErrorV1::CollectionTooLarge {
                        actual: usize_to_u64(values.len())?,
                        maximum: MAX_CANONICAL_COLLECTION_ITEMS_V1,
                    });
                }
                for value in values.values() {
                    value.validate_shape(depth + 1)?;
                }
                Ok(())
            },
        }
    }

    fn encode(&self, output: &mut Vec<u8>) -> Result<(), CanonicalErrorV1> {
        match self {
            Self::Uint(value) => encode_head(output, 0, *value)?,
            Self::Bytes(bytes) => {
                encode_head(
                    output,
                    2,
                    u64::try_from(bytes.len()).map_err(|_| CanonicalErrorV1::LengthOverflow)?,
                )?;
                output.extend_from_slice(bytes);
            },
            Self::Text(text) => {
                encode_head(
                    output,
                    3,
                    u64::try_from(text.len()).map_err(|_| CanonicalErrorV1::LengthOverflow)?,
                )?;
                output.extend_from_slice(text.as_bytes());
            },
            Self::Array(values) => {
                encode_head(
                    output,
                    4,
                    u64::try_from(values.len()).map_err(|_| CanonicalErrorV1::LengthOverflow)?,
                )?;
                for value in values {
                    value.encode(output)?;
                }
            },
            Self::IntMap(values) => {
                encode_head(
                    output,
                    5,
                    u64::try_from(values.len()).map_err(|_| CanonicalErrorV1::LengthOverflow)?,
                )?;
                for (key, value) in values {
                    encode_head(output, 0, *key)?;
                    value.encode(output)?;
                }
            },
        }
        Ok(())
    }
}

fn encode_head(output: &mut Vec<u8>, major: u8, argument: u64) -> Result<(), CanonicalErrorV1> {
    let major_bits = major << 5;
    if argument < 24 {
        output.push(
            major_bits | u8::try_from(argument).map_err(|_| CanonicalErrorV1::LengthOverflow)?,
        );
    } else if argument <= u64::from(u8::MAX) {
        output.extend_from_slice(&[
            major_bits | 24,
            u8::try_from(argument).map_err(|_| CanonicalErrorV1::LengthOverflow)?,
        ]);
    } else if argument <= u64::from(u16::MAX) {
        output.push(major_bits | 25);
        output.extend_from_slice(
            &u16::try_from(argument)
                .map_err(|_| CanonicalErrorV1::LengthOverflow)?
                .to_be_bytes(),
        );
    } else if argument <= u64::from(u32::MAX) {
        output.push(major_bits | 26);
        output.extend_from_slice(
            &u32::try_from(argument)
                .map_err(|_| CanonicalErrorV1::LengthOverflow)?
                .to_be_bytes(),
        );
    } else {
        output.push(major_bits | 27);
        output.extend_from_slice(&argument.to_be_bytes());
    }
    Ok(())
}

fn usize_to_u64(value: usize) -> Result<u64, CanonicalErrorV1> {
    u64::try_from(value).map_err(|_| CanonicalErrorV1::LengthOverflow)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedCanonicalBytesV1(Vec<u8>);

impl ValidatedCanonicalBytesV1 {
    pub fn validate(bytes: &[u8]) -> Result<Self, CanonicalErrorV1> {
        if bytes.len() > MAX_CANONICAL_INPUT_BYTES_V1 {
            return Err(CanonicalErrorV1::InputTooLarge {
                actual: bytes.len(),
                maximum: MAX_CANONICAL_INPUT_BYTES_V1,
            });
        }
        let mut decoder = Decoder::new(bytes);
        let value = decoder.value(0)?;
        if decoder.remaining() != 0 {
            return Err(CanonicalErrorV1::TrailingBytes(decoder.remaining()));
        }
        if value.to_canonical_bytes()?.as_slice() != bytes {
            return Err(CanonicalErrorV1::NonPreferredEncoding);
        }
        Ok(Self(bytes.to_vec()))
    }

    pub fn as_bytes(&self) -> &[u8] { &self.0 }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    fn byte(&mut self) -> Result<u8, CanonicalErrorV1> {
        let value = *self
            .bytes
            .get(self.position)
            .ok_or(CanonicalErrorV1::Truncated)?;
        self.position += 1;
        Ok(value)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], CanonicalErrorV1> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(CanonicalErrorV1::LengthOverflow)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(CanonicalErrorV1::Truncated)?;
        self.position = end;
        Ok(value)
    }

    fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.position) }

    fn argument(&mut self, additional: u8) -> Result<u64, CanonicalErrorV1> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => {
                let value = u64::from(self.byte()?);
                if value < 24 {
                    return Err(CanonicalErrorV1::NonPreferredEncoding);
                }
                Ok(value)
            },
            25 => {
                let bytes = self.take(2)?;
                let value = u64::from(u16::from_be_bytes([bytes[0], bytes[1]]));
                if value <= u64::from(u8::MAX) {
                    return Err(CanonicalErrorV1::NonPreferredEncoding);
                }
                Ok(value)
            },
            26 => {
                let bytes = self.take(4)?;
                let value = u64::from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
                if value <= u64::from(u16::MAX) {
                    return Err(CanonicalErrorV1::NonPreferredEncoding);
                }
                Ok(value)
            },
            27 => {
                let bytes = self.take(8)?;
                let value = u64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                if value <= u64::from(u32::MAX) {
                    return Err(CanonicalErrorV1::NonPreferredEncoding);
                }
                Ok(value)
            },
            31 => Err(CanonicalErrorV1::IndefiniteValue),
            other => Err(CanonicalErrorV1::UnsupportedAdditionalInfo(other)),
        }
    }

    fn bounded_length(
        &mut self,
        additional: u8,
        maximum: usize,
        kind: LengthKind,
    ) -> Result<usize, CanonicalErrorV1> {
        let length = self.argument(additional)?;
        if length > usize_to_u64(maximum)? {
            return Err(match kind {
                LengthKind::Bytes => CanonicalErrorV1::ByteStringTooLarge {
                    actual: length,
                    maximum,
                },
                LengthKind::Text => CanonicalErrorV1::TextTooLarge {
                    actual: length,
                    maximum,
                },
                LengthKind::Collection => CanonicalErrorV1::CollectionTooLarge {
                    actual: length,
                    maximum,
                },
            });
        }
        usize::try_from(length).map_err(|_| CanonicalErrorV1::LengthOverflow)
    }

    fn value(&mut self, depth: usize) -> Result<CanonicalValueV1, CanonicalErrorV1> {
        if depth > MAX_CANONICAL_DEPTH_V1 {
            return Err(CanonicalErrorV1::DepthExceeded {
                maximum: MAX_CANONICAL_DEPTH_V1,
            });
        }
        let initial = self.byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        match major {
            0 => Ok(CanonicalValueV1::Uint(self.argument(additional)?)),
            2 => {
                let length = self.bounded_length(
                    additional,
                    MAX_CANONICAL_BYTE_STRING_BYTES_V1,
                    LengthKind::Bytes,
                )?;
                Ok(CanonicalValueV1::Bytes(self.take(length)?.to_vec()))
            },
            3 => {
                let length =
                    self.bounded_length(additional, MAX_CANONICAL_TEXT_BYTES_V1, LengthKind::Text)?;
                let text = std::str::from_utf8(self.take(length)?)
                    .map_err(|_| CanonicalErrorV1::InvalidUtf8)?;
                Ok(CanonicalValueV1::Text(text.to_owned()))
            },
            4 => {
                let count = self.bounded_length(
                    additional,
                    MAX_CANONICAL_COLLECTION_ITEMS_V1,
                    LengthKind::Collection,
                )?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.value(depth + 1)?);
                }
                Ok(CanonicalValueV1::Array(values))
            },
            5 => {
                let count = self.bounded_length(
                    additional,
                    MAX_CANONICAL_COLLECTION_ITEMS_V1,
                    LengthKind::Collection,
                )?;
                let mut values = BTreeMap::new();
                let mut previous = None;
                for _ in 0..count {
                    let key_initial = self.byte()?;
                    if key_initial >> 5 != 0 {
                        return Err(CanonicalErrorV1::NonIntegerMapKey);
                    }
                    let key = self.argument(key_initial & 0x1f)?;
                    if let Some(previous_key) = previous {
                        if key == previous_key {
                            return Err(CanonicalErrorV1::DuplicateMapKey(key));
                        }
                        if key < previous_key {
                            return Err(CanonicalErrorV1::UnsortedMapKey {
                                previous: previous_key,
                                actual: key,
                            });
                        }
                    }
                    previous = Some(key);
                    values.insert(key, self.value(depth + 1)?);
                }
                Ok(CanonicalValueV1::IntMap(values))
            },
            7 => Err(CanonicalErrorV1::FloatForbidden),
            unsupported => Err(CanonicalErrorV1::UnsupportedMajor(unsupported)),
        }
    }
}

#[derive(Clone, Copy)]
enum LengthKind {
    Bytes,
    Text,
    Collection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalEnvelopeV1 {
    payload: ValidatedCanonicalBytesV1,
}

impl CanonicalEnvelopeV1 {
    pub const DIGEST_BYTES: usize = 32;
    pub const HEADER_BYTES: usize = 14;
    pub const MAGIC: [u8; 8] = *b"BSTR0D1\0";
    pub const VERSION: u16 = 1;

    pub const fn new(payload: ValidatedCanonicalBytesV1) -> Self { Self { payload } }

    pub fn from_value(value: &CanonicalValueV1) -> Result<Self, CanonicalErrorV1> {
        let bytes = value.to_canonical_bytes()?;
        Ok(Self::new(ValidatedCanonicalBytesV1::validate(&bytes)?))
    }

    pub fn payload(&self) -> &ValidatedCanonicalBytesV1 { &self.payload }

    pub fn payload_sha256(&self) -> [u8; 32] { Sha256::digest(self.payload.as_bytes()).into() }

    pub fn to_bytes(&self) -> Result<Vec<u8>, CanonicalErrorV1> {
        let payload_length = u32::try_from(self.payload.as_bytes().len())
            .map_err(|_| CanonicalErrorV1::LengthOverflow)?;
        let mut output = Vec::with_capacity(
            Self::HEADER_BYTES + self.payload.as_bytes().len() + Self::DIGEST_BYTES,
        );
        output.extend_from_slice(&Self::MAGIC);
        output.extend_from_slice(&Self::VERSION.to_le_bytes());
        output.extend_from_slice(&payload_length.to_le_bytes());
        output.extend_from_slice(self.payload.as_bytes());
        output.extend_from_slice(&self.payload_sha256());
        Ok(output)
    }

    pub fn envelope_sha256(&self) -> Result<[u8; 32], CanonicalErrorV1> {
        Ok(Sha256::digest(self.to_bytes()?).into())
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, CanonicalErrorV1> {
        let minimum = Self::HEADER_BYTES + Self::DIGEST_BYTES;
        if bytes.len() < minimum {
            return Err(CanonicalErrorV1::Truncated);
        }
        if bytes[..8] != Self::MAGIC {
            return Err(CanonicalErrorV1::InvalidEnvelopeMagic);
        }
        let version = u16::from_le_bytes([bytes[8], bytes[9]]);
        if version != Self::VERSION {
            return Err(CanonicalErrorV1::UnsupportedEnvelopeVersion(version));
        }
        let declared = u64::from(u32::from_le_bytes([
            bytes[10], bytes[11], bytes[12], bytes[13],
        ]));
        if declared > usize_to_u64(MAX_CANONICAL_INPUT_BYTES_V1)? {
            return Err(CanonicalErrorV1::InputTooLarge {
                actual: usize::try_from(declared).unwrap_or(usize::MAX),
                maximum: MAX_CANONICAL_INPUT_BYTES_V1,
            });
        }
        let payload_length =
            usize::try_from(declared).map_err(|_| CanonicalErrorV1::LengthOverflow)?;
        let expected = Self::HEADER_BYTES
            .checked_add(payload_length)
            .and_then(|value| value.checked_add(Self::DIGEST_BYTES))
            .ok_or(CanonicalErrorV1::LengthOverflow)?;
        if bytes.len() < expected {
            return Err(CanonicalErrorV1::Truncated);
        }
        if bytes.len() > expected {
            return Err(CanonicalErrorV1::TrailingBytes(bytes.len() - expected));
        }
        let payload_end = Self::HEADER_BYTES + payload_length;
        let payload_bytes = &bytes[Self::HEADER_BYTES..payload_end];
        let actual_digest: [u8; 32] = Sha256::digest(payload_bytes).into();
        if bytes[payload_end..] != actual_digest {
            return Err(CanonicalErrorV1::EnvelopeDigestMismatch);
        }
        Ok(Self::new(ValidatedCanonicalBytesV1::validate(
            payload_bytes,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn empty_manifest() -> CanonicalValueV1 {
        try_int_map([
            (0, CanonicalValueV1::Uint(1)),
            (1, CanonicalValueV1::Text("r0d-empty-v1".to_owned())),
            (2, CanonicalValueV1::Uint(60)),
            (3, CanonicalValueV1::Array(vec![])),
            (4, CanonicalValueV1::Array(vec![])),
            (5, CanonicalValueV1::Array(vec![])),
            (6, CanonicalValueV1::Array(vec![])),
            (7, CanonicalValueV1::Bytes(vec![0; 32])),
        ])
        .unwrap()
    }

    #[test]
    fn canonical_uint_uses_preferred_encoding() {
        let value = CanonicalValueV1::Uint(23);
        assert_eq!(value.to_canonical_bytes().unwrap(), vec![0x17]);
    }

    #[test]
    fn frozen_manifest_bytes_and_digests() {
        let payload = empty_manifest().to_canonical_bytes().unwrap();
        assert_eq!(
            hex_bytes(&payload),
            "a80001016c7230642d656d7074792d763102183c03800480058006800758200000000000000000000000000000000000000000000000000000000000000000"
        );
        let envelope = CanonicalEnvelopeV1::from_value(&empty_manifest()).unwrap();
        assert_eq!(
            hex_bytes(&envelope.payload_sha256()),
            "a10d1bbe7d858d697d5ec07ba8f83adf259408a8d17155d6f89fac22ac08adb3"
        );
        assert_eq!(
            hex_bytes(&envelope.envelope_sha256().unwrap()),
            "43f193ecc6ae8b8fdb4305e2353d128d5f4ac7191c232587058e44e020e236b3"
        );
        assert_eq!(
            CanonicalEnvelopeV1::decode_exact(&envelope.to_bytes().unwrap()),
            Ok(envelope)
        );
    }

    #[test]
    fn decoder_negative_matrix_is_typed() {
        let cases: Vec<(&str, Vec<u8>, CanonicalErrorV1)> = vec![
            (
                "nonpreferred integer",
                vec![0x18, 0x00],
                CanonicalErrorV1::NonPreferredEncoding,
            ),
            (
                "nonpreferred length",
                vec![0x58, 0x01, 0x00],
                CanonicalErrorV1::NonPreferredEncoding,
            ),
            (
                "indefinite",
                vec![0x9f, 0xff],
                CanonicalErrorV1::IndefiniteValue,
            ),
            (
                "duplicate keys",
                vec![0xa2, 0x01, 0x00, 0x01, 0x00],
                CanonicalErrorV1::DuplicateMapKey(1),
            ),
            (
                "unsorted keys",
                vec![0xa2, 0x02, 0x00, 0x01, 0x00],
                CanonicalErrorV1::UnsortedMapKey {
                    previous: 2,
                    actual: 1,
                },
            ),
            (
                "noninteger key",
                vec![0xa1, 0x61, b'a', 0x00],
                CanonicalErrorV1::NonIntegerMapKey,
            ),
            (
                "invalid utf8",
                vec![0x61, 0xff],
                CanonicalErrorV1::InvalidUtf8,
            ),
            ("truncated", vec![0x44, 0x11], CanonicalErrorV1::Truncated),
            (
                "trailing",
                vec![0x00, 0x00],
                CanonicalErrorV1::TrailingBytes(1),
            ),
            (
                "unsupported major",
                vec![0x20],
                CanonicalErrorV1::UnsupportedMajor(1),
            ),
            (
                "reserved additional info",
                vec![0x1c],
                CanonicalErrorV1::UnsupportedAdditionalInfo(28),
            ),
            (
                "float",
                vec![0xf9, 0x00, 0x00],
                CanonicalErrorV1::FloatForbidden,
            ),
            (
                "oversized declared bytes",
                vec![0x5a, 0x00, 0x04, 0x00, 0x01],
                CanonicalErrorV1::ByteStringTooLarge {
                    actual: 262_145,
                    maximum: MAX_CANONICAL_BYTE_STRING_BYTES_V1,
                },
            ),
            (
                "oversized declared text",
                vec![0x7a, 0x00, 0x01, 0x00, 0x01],
                CanonicalErrorV1::TextTooLarge {
                    actual: 65_537,
                    maximum: MAX_CANONICAL_TEXT_BYTES_V1,
                },
            ),
            (
                "oversized collection",
                vec![0x99, 0x10, 0x01],
                CanonicalErrorV1::CollectionTooLarge {
                    actual: 4_097,
                    maximum: MAX_CANONICAL_COLLECTION_ITEMS_V1,
                },
            ),
        ];
        for (name, bytes, expected) in cases {
            assert_eq!(
                ValidatedCanonicalBytesV1::validate(&bytes),
                Err(expected),
                "{name}"
            );
        }
    }

    #[test]
    fn input_and_depth_bounds_are_fail_closed() {
        let oversized = vec![0_u8; MAX_CANONICAL_INPUT_BYTES_V1 + 1];
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&oversized),
            Err(CanonicalErrorV1::InputTooLarge {
                actual: oversized.len(),
                maximum: MAX_CANONICAL_INPUT_BYTES_V1,
            })
        );

        let mut nested = vec![0x81; MAX_CANONICAL_DEPTH_V1 + 1];
        nested.push(0);
        assert_eq!(
            ValidatedCanonicalBytesV1::validate(&nested),
            Err(CanonicalErrorV1::DepthExceeded {
                maximum: MAX_CANONICAL_DEPTH_V1,
            })
        );

        let bytes = CanonicalValueV1::Bytes(vec![0; MAX_CANONICAL_BYTE_STRING_BYTES_V1 + 1]);
        assert_eq!(
            bytes.to_canonical_bytes(),
            Err(CanonicalErrorV1::ByteStringTooLarge {
                actual: (MAX_CANONICAL_BYTE_STRING_BYTES_V1 + 1) as u64,
                maximum: MAX_CANONICAL_BYTE_STRING_BYTES_V1,
            })
        );
        let array = CanonicalValueV1::Array(vec![
            CanonicalValueV1::Uint(0);
            MAX_CANONICAL_COLLECTION_ITEMS_V1 + 1
        ]);
        assert_eq!(
            array.to_canonical_bytes(),
            Err(CanonicalErrorV1::CollectionTooLarge {
                actual: (MAX_CANONICAL_COLLECTION_ITEMS_V1 + 1) as u64,
                maximum: MAX_CANONICAL_COLLECTION_ITEMS_V1,
            })
        );
    }

    #[test]
    fn envelope_rejects_digest_version_trailing_and_declared_oversize() {
        let envelope = CanonicalEnvelopeV1::from_value(&CanonicalValueV1::Uint(1)).unwrap();
        let bytes = envelope.to_bytes().unwrap();

        let mut magic = bytes.clone();
        magic[0] ^= 1;
        assert_eq!(
            CanonicalEnvelopeV1::decode_exact(&magic),
            Err(CanonicalErrorV1::InvalidEnvelopeMagic)
        );

        let mut digest = bytes.clone();
        *digest.last_mut().unwrap() ^= 1;
        assert_eq!(
            CanonicalEnvelopeV1::decode_exact(&digest),
            Err(CanonicalErrorV1::EnvelopeDigestMismatch)
        );

        let mut version = bytes.clone();
        version[8..10].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            CanonicalEnvelopeV1::decode_exact(&version),
            Err(CanonicalErrorV1::UnsupportedEnvelopeVersion(2))
        );

        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            CanonicalEnvelopeV1::decode_exact(&trailing),
            Err(CanonicalErrorV1::TrailingBytes(1))
        );

        let mut oversized = bytes;
        oversized[10..14]
            .copy_from_slice(&((MAX_CANONICAL_INPUT_BYTES_V1 as u32) + 1).to_le_bytes());
        assert_eq!(
            CanonicalEnvelopeV1::decode_exact(&oversized),
            Err(CanonicalErrorV1::InputTooLarge {
                actual: MAX_CANONICAL_INPUT_BYTES_V1 + 1,
                maximum: MAX_CANONICAL_INPUT_BYTES_V1,
            })
        );
    }

    #[test]
    fn arbitrary_short_inputs_do_not_unwind() {
        for initial in 0_u8..=u8::MAX {
            let result =
                std::panic::catch_unwind(|| ValidatedCanonicalBytesV1::validate(&[initial]));
            assert!(
                result.is_ok(),
                "decoder unwound for initial byte {initial:#04x}"
            );
        }
    }
}
