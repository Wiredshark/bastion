//! Budgeted, strict restricted decoder (`APEX-T0.2`, packet sections
//! T0.2.06-07).
//!
//! Algorithm (packet section 7.4): check input byte limit, parse exactly
//! one restricted value with depth/node/length budgets (rejecting
//! unsupported/indefinite forms and out-of-policy map keys/text before
//! allocation), require the decoder to consume every input byte, then
//! re-encode the parsed value and require the result to equal the input
//! bytes exactly. Any mismatch is `NonPreferredEncoding` — this is the
//! defense against a hand-written parser silently accepting a non-shortest
//! or reordered spelling of an otherwise-valid value.

use super::encode::encode_value_v1;
use super::error::{ManifestCodecErrorCodeV1, ManifestErrorV1};
use super::text::MachineTextV1;
use super::value::{CanonicalFieldMapV1, FieldIdV1, ManifestDecodeLimitsV1, ManifestValueV1};

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    nodes_seen: u64,
    limits: &'a ManifestDecodeLimitsV1,
}

fn err(code: ManifestCodecErrorCodeV1) -> ManifestErrorV1 { ManifestErrorV1::new(code) }

impl<'a> Cursor<'a> {
    fn read_u8(&mut self) -> Result<u8, ManifestErrorV1> {
        let b = *self.bytes.get(self.pos).ok_or_else(|| err(ManifestCodecErrorCodeV1::MalformedCbor).at(self.pos))?;
        self.pos += 1;
        Ok(b)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ManifestErrorV1> {
        let end = self.pos.checked_add(n).ok_or_else(|| err(ManifestCodecErrorCodeV1::MalformedCbor).at(self.pos))?;
        if end > self.bytes.len() {
            return Err(err(ManifestCodecErrorCodeV1::MalformedCbor).at(self.pos).detail("truncated"));
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Reads the header's argument value for additional-info `ai` (the low
    /// 5 bits of the initial byte). Does NOT judge preferred-form here —
    /// that is enforced later by the whole-value re-encode/byte-compare.
    fn read_argument(&mut self, ai: u8) -> Result<u64, ManifestErrorV1> {
        match ai {
            0..=23 => Ok(ai as u64),
            24 => Ok(self.read_u8()? as u64),
            25 => {
                let b = self.read_bytes(2)?;
                Ok(u16::from_be_bytes([b[0], b[1]]) as u64)
            },
            26 => {
                let b = self.read_bytes(4)?;
                Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64)
            },
            27 => {
                let b = self.read_bytes(8)?;
                Ok(u64::from_be_bytes(b.try_into().unwrap()))
            },
            28..=30 => Err(err(ManifestCodecErrorCodeV1::MalformedCbor).at(self.pos).detail("reserved additional info")),
            31 => Err(err(ManifestCodecErrorCodeV1::IndefiniteLengthForbidden).at(self.pos)),
            _ => unreachable!("additional info is 5 bits"),
        }
    }

    fn account_node(&mut self) -> Result<(), ManifestErrorV1> {
        self.nodes_seen += 1;
        if self.nodes_seen > self.limits.max_nodes {
            return Err(err(ManifestCodecErrorCodeV1::NodeLimit).at(self.pos));
        }
        Ok(())
    }

    fn parse_value(&mut self, depth: u16) -> Result<ManifestValueV1, ManifestErrorV1> {
        if depth > self.limits.max_depth {
            return Err(err(ManifestCodecErrorCodeV1::DepthLimit).at(self.pos));
        }
        self.account_node()?;

        let initial = self.read_u8()?;
        let major = initial >> 5;
        let ai = initial & 0x1F;

        match major {
            0 => {
                let v = self.read_argument(ai)?;
                Ok(ManifestValueV1::Unsigned(v))
            },
            1 => {
                let arg = self.read_argument(ai)?;
                // CBOR value = -1 - arg. Reject if that would not fit in i64.
                let value = -1i128 - arg as i128;
                if value < i64::MIN as i128 {
                    return Err(err(ManifestCodecErrorCodeV1::MalformedCbor).at(self.pos).detail("negative out of i64 range"));
                }
                Ok(ManifestValueV1::Negative(value as i64))
            },
            2 => {
                let len = self.read_argument(ai)? as usize;
                if len as u64 > self.limits.max_byte_string_bytes {
                    return Err(err(ManifestCodecErrorCodeV1::ByteStringLimit).at(self.pos));
                }
                let bytes = self.read_bytes(len)?;
                Ok(ManifestValueV1::Bytes(bytes.to_vec()))
            },
            3 => {
                let len = self.read_argument(ai)? as usize;
                if len as u64 > self.limits.max_machine_text_bytes {
                    return Err(err(ManifestCodecErrorCodeV1::TextLimit).at(self.pos));
                }
                let raw = self.read_bytes(len)?;
                let s = core::str::from_utf8(raw)
                    .map_err(|_| err(ManifestCodecErrorCodeV1::MalformedText).at(self.pos).detail("invalid UTF-8"))?;
                let text = MachineTextV1::new(s)?;
                Ok(ManifestValueV1::MachineText(text))
            },
            4 => {
                let count = self.read_argument(ai)?;
                if count > self.limits.max_array_items {
                    return Err(err(ManifestCodecErrorCodeV1::ArrayItemLimit).at(self.pos));
                }
                let mut items = Vec::with_capacity((count as usize).min(1024));
                for _ in 0..count {
                    items.push(self.parse_value(depth + 1)?);
                }
                Ok(ManifestValueV1::Array(items))
            },
            5 => {
                let count = self.read_argument(ai)?;
                if count > self.limits.max_map_entries {
                    return Err(err(ManifestCodecErrorCodeV1::MapEntryLimit).at(self.pos));
                }
                let mut entries: Vec<(FieldIdV1, ManifestValueV1)> = Vec::with_capacity((count as usize).min(1024));
                let mut last_key: Option<u16> = None;
                for _ in 0..count {
                    let key_pos = self.pos;
                    let key_value = self.parse_value(depth + 1)?;
                    let raw_key = match key_value {
                        ManifestValueV1::Unsigned(v) => v,
                        _ => return Err(err(ManifestCodecErrorCodeV1::FieldKeyType).at(key_pos)),
                    };
                    if raw_key > u16::MAX as u64 {
                        return Err(err(ManifestCodecErrorCodeV1::FieldIdOutOfRange).at(key_pos));
                    }
                    let key = raw_key as u16;
                    match last_key {
                        Some(prev) if key == prev => {
                            return Err(err(ManifestCodecErrorCodeV1::DuplicateFieldId).at(key_pos).field(key));
                        },
                        Some(prev) if key < prev => {
                            return Err(err(ManifestCodecErrorCodeV1::FieldIdOrder).at(key_pos).field(key));
                        },
                        _ => {},
                    }
                    last_key = Some(key);
                    let value = self.parse_value(depth + 1)?;
                    entries.push((FieldIdV1::new(key), value));
                }
                Ok(ManifestValueV1::Map(CanonicalFieldMapV1::from_strictly_increasing(entries)))
            },
            6 => Err(err(ManifestCodecErrorCodeV1::TagForbidden).at(self.pos - 1)),
            7 => match ai {
                20 => Ok(ManifestValueV1::Bool(false)),
                21 => Ok(ManifestValueV1::Bool(true)),
                22 => Err(err(ManifestCodecErrorCodeV1::NullForbidden).at(self.pos - 1)),
                23 => Err(err(ManifestCodecErrorCodeV1::SimpleValueForbidden).at(self.pos - 1).detail("undefined")),
                25 | 26 | 27 => Err(err(ManifestCodecErrorCodeV1::FloatForbidden).at(self.pos - 1)),
                31 => Err(err(ManifestCodecErrorCodeV1::MalformedCbor).at(self.pos - 1).detail("unexpected break")),
                _ => Err(err(ManifestCodecErrorCodeV1::SimpleValueForbidden).at(self.pos - 1)),
            },
            _ => unreachable!("major type is 3 bits"),
        }
    }
}

/// Parses exactly one restricted value from `bytes`, then requires the
/// value to re-encode to exactly `bytes` (the canonical-form check).
/// Structural/policy violations (forbidden types, bad field-map shape,
/// non-ASCII text, limit breaches) surface their own specific codes;
/// anything structurally acceptable but not byte-identical to its own
/// canonical encoding surfaces `NonPreferredEncoding`.
pub fn decode_canonical_value_v1(
    bytes: &[u8],
    limits: &ManifestDecodeLimitsV1,
) -> Result<ManifestValueV1, ManifestErrorV1> {
    if bytes.len() as u64 > limits.max_input_bytes {
        return Err(err(ManifestCodecErrorCodeV1::InputByteLimit));
    }
    let mut cursor = Cursor { bytes, pos: 0, nodes_seen: 0, limits };
    let value = cursor.parse_value(0)?;
    if cursor.pos != bytes.len() {
        return Err(err(ManifestCodecErrorCodeV1::TrailingData).at(cursor.pos));
    }

    let mut re_encoded = Vec::with_capacity(bytes.len());
    encode_value_v1(&mut re_encoded, &value)?;
    if re_encoded != bytes {
        let first_diff = re_encoded.iter().zip(bytes.iter()).position(|(a, b)| a != b).unwrap_or(re_encoded.len().min(bytes.len()));
        return Err(err(ManifestCodecErrorCodeV1::NonPreferredEncoding).at(first_diff));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 20,
            max_depth: 32,
            max_nodes: 10_000,
            max_array_items: 10_000,
            max_map_entries: 10_000,
            max_machine_text_bytes: 1 << 16,
            max_byte_string_bytes: 1 << 16,
        }
    }

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn round_trips_unsigned_zero() {
        let bytes = from_hex("00");
        let v = decode_canonical_value_v1(&bytes, &limits()).unwrap();
        assert!(matches!(v, ManifestValueV1::Unsigned(0)));
    }

    #[test]
    fn rejects_nonpreferred_zero() {
        let bytes = from_hex("1800");
        let e = decode_canonical_value_v1(&bytes, &limits()).unwrap_err();
        assert_eq!(e.code, ManifestCodecErrorCodeV1::NonPreferredEncoding);
    }

    #[test]
    fn rejects_indefinite_forms() {
        for hex in ["5fff", "7fff", "9fff", "bfff"] {
            let bytes = from_hex(hex);
            let e = decode_canonical_value_v1(&bytes, &limits()).unwrap_err();
            assert_eq!(e.code, ManifestCodecErrorCodeV1::IndefiniteLengthForbidden, "hex={hex}");
        }
    }

    #[test]
    fn rejects_forbidden_types() {
        assert_eq!(decode_canonical_value_v1(&from_hex("f90000"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::FloatForbidden);
        assert_eq!(decode_canonical_value_v1(&from_hex("c001"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::TagForbidden);
        assert_eq!(decode_canonical_value_v1(&from_hex("f6"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::NullForbidden);
        assert_eq!(decode_canonical_value_v1(&from_hex("f7"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::SimpleValueForbidden);
    }

    #[test]
    fn field_map_violations() {
        assert_eq!(decode_canonical_value_v1(&from_hex("a200010002"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::DuplicateFieldId);
        assert_eq!(decode_canonical_value_v1(&from_hex("a202000101"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::FieldIdOrder);
        assert_eq!(decode_canonical_value_v1(&from_hex("a1616101"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::FieldKeyType);
        assert_eq!(decode_canonical_value_v1(&from_hex("a11a0001000001"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::FieldIdOutOfRange);
    }

    #[test]
    fn text_violations() {
        assert_eq!(decode_canonical_value_v1(&from_hex("62c3a9"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::MachineTextNonAscii);
        assert_eq!(decode_canonical_value_v1(&from_hex("61ff"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::MalformedText);
    }

    #[test]
    fn trailing_data_and_reserved_additional_info() {
        assert_eq!(decode_canonical_value_v1(&from_hex("0000"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::TrailingData);
        assert_eq!(decode_canonical_value_v1(&from_hex("1c"), &limits()).unwrap_err().code, ManifestCodecErrorCodeV1::MalformedCbor);
    }

    #[test]
    fn depth_limit_enforced() {
        // A chain of single-item arrays nested deeper than max_depth.
        let shallow = ManifestDecodeLimitsV1 { max_depth: 2, ..limits() };
        // [[[]]] -- three nested arrays: depth 0 (outer), 1, 2 -- innermost empty array is depth 2, OK;
        // adding one more level should trip DepthLimit.
        let too_deep_hex = "818181 80".replace(' ', ""); // array(1)->array(1)->array(1)->array(0)
        let e = decode_canonical_value_v1(&from_hex(&too_deep_hex), &shallow).unwrap_err();
        assert_eq!(e.code, ManifestCodecErrorCodeV1::DepthLimit);
    }

    #[test]
    fn declared_length_larger_than_remaining_bytes_is_malformed() {
        // major2 (bytes), length 5, but zero bytes follow.
        let e = decode_canonical_value_v1(&from_hex("45"), &limits()).unwrap_err();
        assert_eq!(e.code, ManifestCodecErrorCodeV1::MalformedCbor);
    }

    #[test]
    fn huge_u64_length_is_rejected_by_limit_not_by_oom() {
        // major2 (bytes), 8-byte length = u64::MAX: must be rejected by the
        // declared-length budget check before any allocation is attempted.
        let bytes = from_hex("5bffffffffffffffff");
        let e = decode_canonical_value_v1(&bytes, &limits()).unwrap_err();
        assert_eq!(e.code, ManifestCodecErrorCodeV1::ByteStringLimit);
    }
}
