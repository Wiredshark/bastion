//! Preferred-form, definite-length encoder (`APEX-T0.2`, packet section
//! T0.2.05). Hand-rolled rather than wrapping a general CBOR crate: the
//! accepted value model (`ManifestValueV1`) is narrow enough that every
//! byte this module can produce is already proven canonical by
//! construction, and every byte it produces is verified against the
//! program's golden vectors — see `common/tests/apex_manifest_encoding_v1.rs`.
//! Emits exactly RFC 8949 Section 4.2.1 core deterministic bytes: shortest
//! integer/length forms, definite lengths only, no floats/tags/null.

use super::error::{ManifestCodecErrorCodeV1, ManifestErrorV1};
use super::value::{CanonicalFieldMapV1, ManifestDecodeLimitsV1, ManifestValueV1};

const MAJOR_UNSIGNED: u8 = 0;
const MAJOR_NEGATIVE: u8 = 1;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
const MAJOR_ARRAY: u8 = 4;
const MAJOR_MAP: u8 = 5;

/// Appends the shortest RFC 8949 header for `major` with argument `value`.
fn push_header(out: &mut Vec<u8>, major: u8, value: u64) {
    let top = major << 5;
    if value <= 23 {
        out.push(top | value as u8);
    } else if value <= 0xFF {
        out.push(top | 24);
        out.push(value as u8);
    } else if value <= 0xFFFF {
        out.push(top | 25);
        out.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= 0xFFFF_FFFF {
        out.push(top | 26);
        out.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        out.push(top | 27);
        out.extend_from_slice(&value.to_be_bytes());
    }
}

fn encode_field_map(out: &mut Vec<u8>, map: &CanonicalFieldMapV1) -> Result<(), ManifestErrorV1> {
    push_header(out, MAJOR_MAP, map.entries().len() as u64);
    for (id, value) in map.entries() {
        push_header(out, MAJOR_UNSIGNED, id.get() as u64);
        encode_value_v1(out, value)?;
    }
    Ok(())
}

/// Encodes one restricted value, appending to `out`. This is the single
/// place in the codec that walks `ManifestValueV1`; every variant is
/// matched exhaustively so a future added variant fails to compile here
/// until handled.
pub fn encode_value_v1(out: &mut Vec<u8>, value: &ManifestValueV1) -> Result<(), ManifestErrorV1> {
    match value {
        ManifestValueV1::Unsigned(v) => push_header(out, MAJOR_UNSIGNED, *v),
        ManifestValueV1::Negative(v) => {
            debug_assert!(*v < 0, "ManifestValueV1::Negative invariant: value must be < 0");
            // CBOR major type 1 argument is (-1 - v) for v < 0.
            let arg = (-1i128 - *v as i128) as u64;
            push_header(out, MAJOR_NEGATIVE, arg);
        },
        ManifestValueV1::Bytes(b) => {
            push_header(out, MAJOR_BYTES, b.len() as u64);
            out.extend_from_slice(b);
        },
        ManifestValueV1::MachineText(t) => {
            let bytes = t.as_str().as_bytes();
            push_header(out, MAJOR_TEXT, bytes.len() as u64);
            out.extend_from_slice(bytes);
        },
        ManifestValueV1::Bool(false) => out.push(0xF4),
        ManifestValueV1::Bool(true) => out.push(0xF5),
        ManifestValueV1::Array(items) => {
            push_header(out, MAJOR_ARRAY, items.len() as u64);
            for item in items {
                encode_value_v1(out, item)?;
            }
        },
        ManifestValueV1::Map(map) => encode_field_map(out, map)?,
    }
    Ok(())
}

/// Encodes one restricted value into a fresh byte vector, enforcing the
/// same limits a decoder would apply to the result (so encoding never
/// silently produces an artifact its own decoder would reject).
pub fn encode_manifest_value_v1(
    value: &ManifestValueV1,
    limits: &ManifestDecodeLimitsV1,
) -> Result<Vec<u8>, ManifestErrorV1> {
    check_limits_pre_encode(value, limits, 0)?;
    let mut out = Vec::new();
    encode_value_v1(&mut out, value)?;
    if out.len() as u64 > limits.max_input_bytes {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::EncodeLimit)
            .detail("encoded output exceeds max_input_bytes"));
    }
    Ok(out)
}

fn check_limits_pre_encode(
    value: &ManifestValueV1,
    limits: &ManifestDecodeLimitsV1,
    depth: u16,
) -> Result<(), ManifestErrorV1> {
    if depth > limits.max_depth {
        return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::DepthLimit));
    }
    match value {
        ManifestValueV1::Bytes(b) => {
            if b.len() as u64 > limits.max_byte_string_bytes {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::ByteStringLimit));
            }
        },
        ManifestValueV1::MachineText(t) => {
            if t.as_str().len() as u64 > limits.max_machine_text_bytes {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::TextLimit));
            }
        },
        ManifestValueV1::Array(items) => {
            if items.len() as u64 > limits.max_array_items {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::ArrayItemLimit));
            }
            for item in items {
                check_limits_pre_encode(item, limits, depth + 1)?;
            }
        },
        ManifestValueV1::Map(map) => {
            if map.entries().len() as u64 > limits.max_map_entries {
                return Err(ManifestErrorV1::new(ManifestCodecErrorCodeV1::MapEntryLimit));
            }
            for (_, v) in map.entries() {
                check_limits_pre_encode(v, limits, depth + 1)?;
            }
        },
        ManifestValueV1::Unsigned(_) | ManifestValueV1::Negative(_) | ManifestValueV1::Bool(_) => {},
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::manifest::text::MachineTextV1;
    use crate::apex::manifest::value::{CanonicalFieldMapV1, FieldIdV1};

    fn hex(v: &ManifestValueV1) -> String {
        let mut out = Vec::new();
        encode_value_v1(&mut out, v).unwrap();
        out.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn unsigned_boundaries_match_rfc8949() {
        assert_eq!(hex(&ManifestValueV1::Unsigned(0)), "00");
        assert_eq!(hex(&ManifestValueV1::Unsigned(23)), "17");
        assert_eq!(hex(&ManifestValueV1::Unsigned(24)), "1818");
        assert_eq!(hex(&ManifestValueV1::Unsigned(255)), "18ff");
        assert_eq!(hex(&ManifestValueV1::Unsigned(256)), "190100");
        assert_eq!(hex(&ManifestValueV1::Unsigned(65535)), "19ffff");
        assert_eq!(hex(&ManifestValueV1::Unsigned(65536)), "1a00010000");
        assert_eq!(hex(&ManifestValueV1::Unsigned(u64::MAX)), "1bffffffffffffffff");
    }

    #[test]
    fn negative_boundaries_match_rfc8949() {
        assert_eq!(hex(&ManifestValueV1::negative(-1).unwrap()), "20");
        assert_eq!(hex(&ManifestValueV1::negative(-24).unwrap()), "37");
        assert_eq!(hex(&ManifestValueV1::negative(-25).unwrap()), "3818");
    }

    #[test]
    fn bytes_and_text_and_bool() {
        assert_eq!(hex(&ManifestValueV1::Bytes(vec![])), "40");
        assert_eq!(
            hex(&ManifestValueV1::Bytes(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff])),
            "5000112233445566778899aabbccddeeff"
        );
        assert_eq!(hex(&ManifestValueV1::MachineText(MachineTextV1::new("").unwrap())), "60");
        assert_eq!(
            hex(&ManifestValueV1::MachineText(MachineTextV1::new("bastion.manifest/v1").unwrap())),
            "7362617374696f6e2e6d616e69666573742f7631"
        );
        assert_eq!(hex(&ManifestValueV1::Bool(false)), "f4");
        assert_eq!(hex(&ManifestValueV1::Bool(true)), "f5");
    }

    #[test]
    fn array_and_field_map() {
        assert_eq!(
            hex(&ManifestValueV1::Array(vec![
                ManifestValueV1::Unsigned(1),
                ManifestValueV1::Unsigned(2),
                ManifestValueV1::Unsigned(3),
            ])),
            "83010203"
        );

        let map = CanonicalFieldMapV1::try_from_entries(vec![
            (FieldIdV1::new(0), ManifestValueV1::Unsigned(1)),
            (FieldIdV1::new(1), ManifestValueV1::MachineText(MachineTextV1::new("alpha").unwrap())),
            (FieldIdV1::new(2), ManifestValueV1::Bool(true)),
        ])
        .unwrap();
        assert_eq!(hex(&ManifestValueV1::Map(map)), "a300010165616c70686102f5");
    }

    #[test]
    fn encode_limit_rejects_oversized_output() {
        let tiny_limits = ManifestDecodeLimitsV1 {
            max_input_bytes: 1,
            max_depth: 8,
            max_nodes: 100,
            max_array_items: 100,
            max_map_entries: 100,
            max_machine_text_bytes: 100,
            max_byte_string_bytes: 100,
        };
        let err = encode_manifest_value_v1(&ManifestValueV1::Unsigned(u64::MAX), &tiny_limits).unwrap_err();
        assert_eq!(err.code, ManifestCodecErrorCodeV1::EncodeLimit);
    }
}
