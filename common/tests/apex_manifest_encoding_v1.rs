//! Golden-vector conformance for `BastionManifestEncodingV1` (`APEX-T0.2`).
//!
//! Loads `fixtures/apex_manifest_v1/golden-vectors.json` (a verbatim copy
//! of the program's `PROJECT-BASTION-APEX-MANIFEST-CBOR-GOLDEN-VECTORS-v1.json`)
//! and checks every one of its vectors — not a hand-picked subset — against
//! the real `veloren_common::apex::manifest` encoder/decoder. `valid`
//! vectors must encode to `expected_hex` exactly; `invalid` vectors must
//! decode-fail with the fixture's declared terminal class.

use serde_json::Value as Json;
use veloren_common::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestDecodeLimitsV1, ManifestValueV1, MachineTextV1, decode_value_bytes_v1,
    encode_value_bytes_v1,
};

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

fn hex_to_bytes(s: &str) -> Vec<u8> { (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect() }

fn bytes_to_hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

/// Converts the golden-vector JSON's polymorphic "value" shape into a
/// `ManifestValueV1` tree. This mirrors the vector corpus's own
/// conventions (documented in its `notes` array), not general JSON<->CBOR
/// mapping.
fn json_to_manifest_value(v: &Json) -> ManifestValueV1 {
    match v {
        Json::Bool(b) => ManifestValueV1::Bool(*b),
        Json::Number(n) => {
            if let Some(u) = n.as_u64() {
                ManifestValueV1::Unsigned(u)
            } else if let Some(i) = n.as_i64() {
                ManifestValueV1::negative(i).expect("negative test values in the corpus are always < 0")
            } else {
                panic!("unsupported numeric vector value: {n}")
            }
        },
        Json::String(s) => ManifestValueV1::MachineText(MachineTextV1::new(s.clone()).expect("vector machine-text values are ASCII")),
        Json::Object(map) if map.contains_key("bytes_hex") => {
            let hex = map["bytes_hex"].as_str().expect("bytes_hex is a string");
            ManifestValueV1::Bytes(hex_to_bytes(hex))
        },
        Json::Array(items) => {
            // Two shapes share JSON-array syntax: a plain sequence, or a
            // field map spelled as `[{"field": N, "value": V}, ...]`.
            let looks_like_field_map = !items.is_empty()
                && items.iter().all(|it| it.is_object() && it.as_object().unwrap().contains_key("field"));
            if looks_like_field_map {
                let entries = items
                    .iter()
                    .map(|it| {
                        let obj = it.as_object().unwrap();
                        let field = obj["field"].as_u64().expect("field id fits u64") as u16;
                        let value = json_to_manifest_value(&obj["value"]);
                        (FieldIdV1::new(field), value)
                    })
                    .collect();
                ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries).expect("vector field maps are already sorted/unique"))
            } else {
                ManifestValueV1::Array(items.iter().map(json_to_manifest_value).collect())
            }
        },
        Json::Null => panic!("golden vectors never encode a null value directly"),
        Json::Object(_) => panic!("unexpected object shape in vector value"),
    }
}

#[test]
fn all_valid_vectors_encode_to_expected_hex() {
    let raw = std::fs::read_to_string("tests/fixtures/apex_manifest_v1/golden-vectors.json").expect("fixture present");
    let doc: Json = serde_json::from_str(&raw).expect("fixture is valid JSON");

    assert_eq!(doc["profile"], "bastion.manifest-cbor.rfc8949-core/v1");

    let vectors = doc["vectors"].as_array().expect("vectors is an array");
    let mut valid_count = 0;
    let mut invalid_count = 0;

    for vec in vectors {
        let id = vec["id"].as_str().unwrap();
        let kind = vec["kind"].as_str().unwrap();
        match kind {
            "valid" => {
                valid_count += 1;
                let expected_hex = vec["expected_hex"].as_str().unwrap();
                let value = json_to_manifest_value(&vec["value"]);
                let bytes = encode_value_bytes_v1(&value).unwrap_or_else(|e| panic!("vector {id}: encode failed: {e}"));
                assert_eq!(bytes_to_hex(&bytes), expected_hex, "vector {id}: encoded hex mismatch");

                // Round-trip: decoding our own canonical bytes must succeed
                // and reproduce the same value's hex on re-encode.
                let decoded = decode_value_bytes_v1(&bytes, &limits()).unwrap_or_else(|e| panic!("vector {id}: decode of our own bytes failed: {e}"));
                let re_encoded = encode_value_bytes_v1(&decoded).unwrap();
                assert_eq!(bytes_to_hex(&re_encoded), expected_hex, "vector {id}: decode->re-encode mismatch");
            },
            "invalid" => {
                invalid_count += 1;
                let input_hex = vec["input_hex"].as_str().unwrap();
                let expected_error = vec["expected_error"].as_str().unwrap();
                let bytes = hex_to_bytes(input_hex);
                let err = decode_value_bytes_v1(&bytes, &limits())
                    .expect_err(&format!("vector {id}: expected decode failure ({expected_error}), got success"));
                assert_eq!(err.code.terminal_class(), expected_error, "vector {id}: wrong terminal class");
            },
            other => panic!("vector {id}: unknown kind {other}"),
        }
    }

    // Non-vacuity: this fixture must actually contain both kinds, or the
    // two branches above would be silently untested.
    assert!(valid_count >= 10, "expected a substantial valid-vector corpus, got {valid_count}");
    assert!(invalid_count >= 10, "expected a substantial invalid-vector corpus, got {invalid_count}");
    println!("apex_manifest_encoding_v1: {valid_count} valid + {invalid_count} invalid vectors, all conformant");
}
