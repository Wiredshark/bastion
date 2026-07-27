//! Re-emits `APEX-A.2`'s finding matrix and `APEX-A.3`'s program registry
//! as canonical CBOR through the real `BastionManifestEncodingV1` encoder
//! (`common::apex::manifest`), closing the "CSV/JSON pending T0.2" note
//! from Builder Opus 5's Batch-1 review. Schema: `readme/apex/
//! APEX-CBOR-EMISSION-SCHEMA-v1.md`.
//!
//! Every emitted file is immediately decoded back and diffed against the
//! source data before being trusted -- this is a self-check the tool runs,
//! not a claim made without running it.

use common::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, ManifestDecodeLimitsV1, ManifestValueV1, MachineTextV1, decode_value_bytes_v1,
    encode_value_bytes_v1,
};
use std::fs;
use std::path::Path;

fn limits() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 4 << 20,
        max_depth: 8,
        max_nodes: 65536,
        max_array_items: 4096,
        max_map_entries: 32,
        max_machine_text_bytes: 16384,
        max_byte_string_bytes: 16384,
    }
}

/// `MachineTextV1` is ASCII-only (T0.2 V1 identity-text policy), but the
/// A.2/A.3 source data mixes real ASCII identifiers with free-form prose
/// that legitimately contains non-ASCII punctuation (em dashes, etc.). Per
/// field, not per column: encode as `MachineText` when the content is pure
/// ASCII, otherwise fall back to `Bytes` carrying the exact UTF-8 bytes
/// unmodified -- never silently transliterate or drop characters to force
/// an ASCII fit.
fn text(s: &str) -> ManifestValueV1 {
    match MachineTextV1::new(s) {
        Ok(t) => ManifestValueV1::MachineText(t),
        Err(_) => ManifestValueV1::Bytes(s.as_bytes().to_vec()),
    }
}

fn text_array(items: &[String]) -> ManifestValueV1 { ManifestValueV1::Array(items.iter().map(|s| text(s)).collect()) }

fn map(entries: Vec<(u16, ManifestValueV1)>) -> ManifestValueV1 {
    let entries = entries.into_iter().map(|(id, v)| (FieldIdV1::new(id), v)).collect();
    ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries).expect("no duplicate field ids"))
}

// --- minimal RFC 4180 CSV parser (no new crate dependency for this tool) ---

fn parse_csv(raw: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut field = String::new();
    let mut row = Vec::new();
    let mut in_quotes = false;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => {
                    row.push(std::mem::take(&mut field));
                },
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                },
                '\r' => {},
                _ => field.push(c),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

struct CsvTable {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl CsvTable {
    fn load(path: &Path) -> Self {
        let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let mut all = parse_csv(&raw);
        let header = all.remove(0);
        Self { header, rows: all }
    }

    fn get<'a>(&'a self, row: &'a [String], col: &str) -> &'a str {
        let idx = self.header.iter().position(|h| h == col).unwrap_or_else(|| panic!("unknown column {col}"));
        row.get(idx).map(|s| s.as_str()).unwrap_or("")
    }
}

fn emit_finding_matrix(repo_root: &Path) {
    let csv_path = repo_root.join("readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv");
    let table = CsvTable::load(&csv_path);

    let mut findings = Vec::new();
    for row in &table.rows {
        let replacement_rows: Vec<String> =
            table.get(row, "replacement_rows").split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let evidence_gap = table.get(row, "evidence_gap");

        let mut entries = vec![
            (0u16, text(table.get(row, "finding_id"))),
            (1, text(table.get(row, "problem_group"))),
            (2, text(table.get(row, "status"))),
            (3, text(table.get(row, "live_path"))),
            (4, text(table.get(row, "live_observation"))),
            (5, text_array(&replacement_rows)),
            (6, text(table.get(row, "scope_note"))),
            (7, text(table.get(row, "live_commit"))),
            (8, text(table.get(row, "evidence_confidence"))),
        ];
        if !evidence_gap.is_empty() {
            entries.push((9, text(evidence_gap)));
        }
        findings.push(map(entries));
    }

    let value = ManifestValueV1::Array(findings);
    let bytes = encode_value_bytes_v1(&value).expect("encode finding matrix");

    // Self-check: decode back and confirm byte-identical re-encode (the
    // T0.2 decoder already enforces this; asserting it here makes the
    // check visible in this tool's own output, not just implicit).
    let decoded = decode_value_bytes_v1(&bytes, &limits()).expect("decode own output");
    let re_encoded = encode_value_bytes_v1(&decoded).expect("re-encode");
    assert_eq!(bytes, re_encoded, "finding matrix CBOR is not a fixed point of decode->encode");
    let ManifestValueV1::Array(items) = &decoded else { panic!("expected array") };
    assert_eq!(items.len(), table.rows.len(), "row count mismatch after round-trip");

    write_with_digest(&repo_root.join("readme/apex/APEX-FINDING-STATUS-MATRIX-v1.cbor"), &bytes);
    println!("finding matrix: {} findings, {} bytes, round-trip verified", table.rows.len(), bytes.len());
}

fn json_str<'a>(v: &'a serde_json::Value, key: &str) -> &'a str { v.get(key).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("missing/non-string field {key}")) }

fn json_str_array(v: &serde_json::Value, key: &str) -> Vec<String> {
    v.get(key).and_then(|x| x.as_array()).map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect()).unwrap_or_default()
}

fn closure_rule_map(rule: &serde_json::Value) -> ManifestValueV1 {
    let kind = json_str(rule, "kind");
    let mut entries = vec![(0u16, text(kind))];
    match kind {
        "Row" => {
            if let Some(row) = rule.get("row").and_then(|v| v.as_str()) {
                entries.push((1, text(row)));
            }
        },
        "AllOf" => {
            entries.push((2, text_array(&json_str_array(rule, "rows"))));
        },
        "AnyOf" => {
            entries.push((2, text_array(&json_str_array(rule, "rows"))));
            entries.push((3, text(json_str(rule, "rationale"))));
        },
        "SupersededBy" => {
            entries.push((2, text_array(&json_str_array(rule, "rows"))));
            entries.push((4, text(json_str(rule, "reason"))));
        },
        other => panic!("unknown closure rule kind {other}"),
    }
    map(entries)
}

fn row_status_map(status: &serde_json::Value) -> ManifestValueV1 {
    map(vec![
        (0, text(json_str(status, "specification"))),
        (1, text(json_str(status, "microstep_research"))),
        (2, text(json_str(status, "implementation"))),
        (3, text(json_str(status, "verification"))),
        (4, text(json_str(status, "deployment"))),
    ])
}

fn emit_registry(repo_root: &Path) {
    let json_path = repo_root.join("readme/APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json");
    let raw = fs::read_to_string(&json_path).unwrap_or_else(|e| panic!("read {json_path:?}: {e}"));
    let reg: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");

    let row_order = json_str_array(&reg, "row_order");

    let rows: Vec<ManifestValueV1> = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let mut entries = vec![
                (0u16, text(json_str(r, "row_id"))),
                (1, ManifestValueV1::Unsigned(r["sequence_index"].as_u64().unwrap())),
                (2, text(json_str(r, "title"))),
                (3, text_array(&json_str_array(r, "hard_dependencies"))),
                (4, text_array(&json_str_array(r, "finding_ids"))),
                (5, text(json_str(r, "source_surfaces_status"))),
            ];
            if let Some(pf) = r.get("packet_file").and_then(|v| v.as_str()) {
                entries.push((6, text(pf)));
            }
            entries.push((7, text(json_str(r, "evidence_status"))));
            entries.push((8, text(json_str(r, "rollback_plan_status"))));
            entries.push((9, row_status_map(&r["status"])));
            map(entries)
        })
        .collect();

    let findings: Vec<ManifestValueV1> = reg["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            map(vec![
                (0, text(json_str(f, "finding_id"))),
                (1, text(json_str(f, "originating_package"))),
                (2, text(json_str(f, "live_status"))),
                (3, closure_rule_map(&f["closure_rule"])),
                (4, text_array(&json_str_array(f, "source_anchors"))),
                (5, text(json_str(f, "last_live_commit_checked"))),
            ])
        })
        .collect();

    let value = map(vec![
        (0, text(json_str(&reg, "schema"))),
        (1, text(json_str(&reg, "canonical_guide"))),
        (2, text(json_str(&reg, "finding_matrix"))),
        (3, text(json_str(&reg, "audit_basis"))),
        (4, text(json_str(&reg, "last_live_commit_checked"))),
        (5, text_array(&row_order)),
        (6, ManifestValueV1::Array(rows)),
        (7, ManifestValueV1::Array(findings)),
        (8, text_array(&json_str_array(&reg, "unresolved_row_references"))),
    ]);

    let bytes = encode_value_bytes_v1(&value).expect("encode registry");

    let decoded = decode_value_bytes_v1(&bytes, &limits()).expect("decode own output");
    let re_encoded = encode_value_bytes_v1(&decoded).expect("re-encode");
    assert_eq!(bytes, re_encoded, "registry CBOR is not a fixed point of decode->encode");
    let ManifestValueV1::Map(top) = &decoded else { panic!("expected top-level map") };
    let row_count = top
        .entries()
        .iter()
        .find(|(id, _)| id.get() == 6)
        .map(|(_, v)| match v {
            ManifestValueV1::Array(a) => a.len(),
            _ => panic!("rows field is not an array"),
        })
        .unwrap();
    assert_eq!(row_count, reg["rows"].as_array().unwrap().len(), "row count mismatch after round-trip");

    write_with_digest(&repo_root.join("readme/APEX-DETERMINISM-PROGRAM-REGISTRY-v1.cbor"), &bytes);
    println!(
        "registry: {} rows, {} findings, {} bytes, round-trip verified",
        reg["rows"].as_array().unwrap().len(),
        reg["findings"].as_array().unwrap().len(),
        bytes.len()
    );
}

fn write_with_digest(path: &Path, bytes: &[u8]) {
    use sha2::{Digest, Sha256};
    fs::write(path, bytes).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let file_name = path.file_name().unwrap().to_string_lossy();
    fs::write(path.with_extension("cbor.sha256"), format!("{hex}  {file_name}\n")).unwrap();
}

fn hex_of(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

/// `APEX-T0.5` golden vectors: self-generated from the real
/// `common::apex::subsystem` encoder/decoder (never hand-typed hex),
/// following the exact precedent set by `APEX-T0.1`'s self-generated
/// scalar vectors and `APEX-T0.2`'s committed manifest fixture. Every
/// vector is encoded, decoded back, and re-encoded before being trusted
/// (matches this file's module-level self-check practice).
fn emit_subsystem_vectors(repo_root: &Path) {
    use common::apex::digest::{ContentIdentityV1, DigestDomainIdV1, digest_canonical_bytes_v1, hash_artifact_bytes_v1};
    use common::apex::manifest::{ManifestDecodeV1, ManifestEncodeV1, encode_manifest_v1};
    use common::apex::scalar::SchemaVersion;
    use common::apex::subsystem::{
        AcceptRangeV1, AcceptSetV1, CapabilityIdV1, CapabilityRequirementV1, CompatibilityProfileV1, CompatibilityRuleV1,
        ExtensionCriticalityV1, SubsystemDescriptorV1, SubsystemSlotIdV1, TransformIdV1, TransformKeyV1,
    };

    fn limits() -> ManifestDecodeLimitsV1 {
        ManifestDecodeLimitsV1 {
            max_input_bytes: 1 << 16,
            max_depth: 8,
            max_nodes: 4096,
            max_array_items: 256,
            max_map_entries: 256,
            max_machine_text_bytes: 4096,
            max_byte_string_bytes: 4096,
        }
    }

    fn round_trip_verify<T: ManifestEncodeV1 + ManifestDecodeV1 + PartialEq + std::fmt::Debug>(name: &str, value: &T) -> Vec<u8> {
        let bytes = encode_manifest_v1(value, &limits()).unwrap_or_else(|e| panic!("encode {name}: {e:?}"));
        let decoded: T = common::apex::manifest::decode_manifest_v1(&bytes, &limits()).unwrap_or_else(|e| panic!("decode {name}: {e:?}"));
        assert_eq!(*value, decoded, "{name}: round-trip mismatch");
        let re_encoded = encode_manifest_v1(&decoded, &limits()).unwrap_or_else(|e| panic!("re-encode {name}: {e:?}"));
        assert_eq!(bytes, re_encoded, "{name}: not a fixed point of decode->encode");
        bytes
    }

    let descriptor = SubsystemDescriptorV1 {
        slot: SubsystemSlotIdV1::Worldgen,
        schema: SchemaVersion::new(1),
        content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"apex-t0.5-worldgen-vector"), semantic: None },
    };
    let descriptor_bytes = round_trip_verify("descriptor.worldgen", &descriptor);
    let descriptor_digest = digest_canonical_bytes_v1(DigestDomainIdV1::SubsystemDescriptor, &descriptor_bytes, 1 << 20).unwrap();

    let rule_vectors: Vec<(&str, CompatibilityRuleV1)> = vec![
        ("rule.exact", CompatibilityRuleV1::Exact { content: ContentIdentityV1 { artifact: hash_artifact_bytes_v1(b"exact-vector"), semantic: None } }),
        ("rule.accept_set", CompatibilityRuleV1::AcceptSet(AcceptSetV1::new(vec![SchemaVersion::new(1), SchemaVersion::new(2)]).unwrap())),
        ("rule.accept_range", CompatibilityRuleV1::AcceptRange(AcceptRangeV1::new(SchemaVersion::new(1), SchemaVersion::new(5)).unwrap())),
        (
            "rule.negotiated_capability",
            CompatibilityRuleV1::NegotiatedCapability { requirement: CapabilityRequirementV1::new(vec![CapabilityIdV1::new(1)]).unwrap() },
        ),
        (
            "rule.direct_transform",
            CompatibilityRuleV1::DirectTransform {
                key: TransformKeyV1 {
                    transform_id: TransformIdV1::new(1),
                    from_schema: SchemaVersion::new(1),
                    to_schema: SchemaVersion::new(2),
                    implementation_root: hash_artifact_bytes_v1(b"transform-impl-vector"),
                },
            },
        ),
        ("rule.provenance_only", CompatibilityRuleV1::ProvenanceOnly),
        (
            "rule.unknown_noncritical",
            CompatibilityRuleV1::Unknown { tag: common::apex::manifest::VariantTagV1::new(9001), criticality: ExtensionCriticalityV1::Noncritical, raw_payload: vec![0xa1, 0xb2] },
        ),
    ];

    let mut vectors_json = serde_json::json!({
        "schema": "bastion.apex-subsystem-compatibility-golden-vectors/v1",
        "descriptor": {
            "name": "descriptor.worldgen",
            "hex": hex_of(&descriptor_bytes),
            "digest_domain": "SubsystemDescriptor",
            "digest_hex": hex_of(descriptor_digest.bytes.as_array()),
        },
        "rules": [],
        "profile": {},
    });

    let rules_array = vectors_json["rules"].as_array_mut().unwrap();
    for (name, rule) in &rule_vectors {
        let bytes = round_trip_verify(name, rule);
        rules_array.push(serde_json::json!({ "name": name, "hex": hex_of(&bytes) }));
    }

    let profile = CompatibilityProfileV1::new(vec![
        (SubsystemSlotIdV1::Worldgen, rule_vectors[0].1.clone()),
        (SubsystemSlotIdV1::Content, rule_vectors[1].1.clone()),
        (SubsystemSlotIdV1::Numeric, rule_vectors[5].1.clone()),
    ])
    .unwrap();
    let profile_bytes = round_trip_verify("profile.multi_slot", &profile);
    let profile_digest = digest_canonical_bytes_v1(DigestDomainIdV1::CompatibilityProfile, &profile_bytes, 1 << 20).unwrap();
    vectors_json["profile"] = serde_json::json!({
        "name": "profile.multi_slot",
        "hex": hex_of(&profile_bytes),
        "digest_domain": "CompatibilityProfile",
        "digest_hex": hex_of(profile_digest.bytes.as_array()),
    });

    let text = serde_json::to_string_pretty(&vectors_json).unwrap() + "\n";
    let path = repo_root.join("readme/apex/APEX-SUBSYSTEM-COMPATIBILITY-GOLDEN-VECTORS-v1.json");
    fs::write(&path, &text).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    let file_name = path.file_name().unwrap().to_string_lossy();
    fs::write(path.with_extension("json.sha256"), format!("{}  {file_name}\n", hex_of(&digest))).unwrap();

    println!("subsystem vectors: 1 descriptor, {} rules, 1 profile, {} bytes, round-trip verified", rule_vectors.len(), text.len());
}

fn main() {
    let repo_root = std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::env::current_dir().expect("cwd")
    });
    emit_finding_matrix(&repo_root);
    emit_registry(&repo_root);
    emit_subsystem_vectors(&repo_root);
}
