//! `APEX-T1.3.12` — canonical smoke-record emitter. The python
//! orchestrator (`bastion-harness/tools/apex_local_repro_smoke.py`)
//! gathers RAW evidence into one JSON file; this bin performs every
//! canonical operation — path tokenization (raw host paths never enter
//! the record), output-manifest and canary roots under `LocalReproSmoke`
//! (= 12), T0.2 encoding, decode-roundtrip self-check (which structurally
//! enforces the PASS admission rules), and atomic emission with the
//! canonical root printed for the terminal marker.
//!
//! Usage: `apex_local_repro_record <evidence.json> <out-dir>`

use common::apex::build::local_repro::{
    BuildExecutionKindV1, BuildExecutionV1, HostPathEvaluationV1, LocalReproTerminalV1, LocalReproducibilitySmokeV1,
    local_repro_limits_v1,
};
use common::apex::digest::{
    ArtifactDigestV1, ArtifactIdentityV1, DigestAlgorithmIdV1, DigestBytes32V1, DigestDomainIdV1, ProtocolDigestV1,
    digest_manifest_value_v1, hash_artifact_bytes_v1,
};
use common::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestEncodeV1, ManifestValueV1, decode_manifest_v1,
    encode_manifest_v1,
};
use common::apex::source_closure::GitHexIdV1;
use sha2::{Digest, Sha256};
use std::io::Write;

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    println!("TERMINAL: T1.3-BLOCK-EVIDENCE-PARTIAL");
    std::process::exit(20);
}

fn hex32(b: &[u8; 32]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

fn token(path: &str) -> ArtifactDigestV1 { hash_artifact_bytes_v1(path.as_bytes()).digest }

fn s<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_else(|| die(&format!("missing string field {k}")))
}

fn u(v: &serde_json::Value, k: &str) -> u64 {
    v.get(k).and_then(|x| x.as_u64()).unwrap_or_else(|| die(&format!("missing unsigned field {k}")))
}

fn b(v: &serde_json::Value, k: &str) -> bool {
    v.get(k).and_then(|x| x.as_bool()).unwrap_or_else(|| die(&format!("missing bool field {k}")))
}

fn digest32(v: &serde_json::Value, k: &str) -> [u8; 32] {
    let hexs = s(v, k);
    let bytes = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(hexs.get(i..i + 2).unwrap_or_else(|| die(&format!("{k}: short hex"))), 16))
        .collect::<Result<Vec<u8>, _>>()
        .unwrap_or_else(|_| die(&format!("{k}: bad hex")));
    bytes.try_into().unwrap_or_else(|_| die(&format!("{k}: not 32 bytes")))
}

fn artifact(v: &serde_json::Value, hash_key: &str, size_key: &str) -> ArtifactIdentityV1 {
    ArtifactIdentityV1 {
        digest: ArtifactDigestV1 {
            algorithm: DigestAlgorithmIdV1::Sha256,
            bytes: DigestBytes32V1::from_array(digest32(v, hash_key)),
        },
        size_bytes: u(v, size_key),
    }
}

fn text(v: &str) -> MachineTextV1 {
    MachineTextV1::new(v).unwrap_or_else(|_| die(&format!("non-ASCII machine text: {v:?}")))
}

fn execution(v: &serde_json::Value) -> BuildExecutionV1 {
    let kind = match s(v, "kind") {
        "baseline" => BuildExecutionKindV1::Baseline,
        "rebuild" => BuildExecutionKindV1::RebuildCheck,
        other => die(&format!("unknown execution kind {other:?}")),
    };
    BuildExecutionV1 {
        ordinal: u(v, "ordinal") as u32,
        execution_kind: kind,
        locally_executed: b(v, "locally_executed"),
        source_path_token: token(s(v, "source_path")),
        out_link_path_token: token(s(v, "out_link_path")),
        exit_code: v.get("exit_code").and_then(|x| x.as_i64()).unwrap_or_else(|| die("missing exit_code")),
        log_identity: artifact(v, "log_sha256", "log_size"),
        started_at: text(s(v, "started_at")),
        finished_at: text(s(v, "finished_at")),
    }
}

/// Leaf manifest root: array of {path, kind, mode, size, sha256|target}
/// maps in path-byte order, digested under `LocalReproSmoke` (packet
/// section 7: "Output leaf key = raw relative path bytes").
fn manifest_root(leaves: &[serde_json::Value]) -> ProtocolDigestV1 {
    struct Wrapper(Vec<ManifestValueV1>);
    impl ManifestEncodeV1 for Wrapper {
        fn to_manifest_value_v1(
            &self,
        ) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
            Ok(ManifestValueV1::Array(self.0.clone()))
        }
    }
    let mut sorted: Vec<&serde_json::Value> = leaves.iter().collect();
    sorted.sort_by(|a, b| s(a, "path").as_bytes().cmp(s(b, "path").as_bytes()));
    let items = sorted
        .iter()
        .map(|leaf| {
            let mut entries = vec![
                (FieldIdV1::new(0), ManifestValueV1::MachineText(text(s(leaf, "path")))),
                (FieldIdV1::new(1), ManifestValueV1::MachineText(text(s(leaf, "kind")))),
                (FieldIdV1::new(2), ManifestValueV1::Unsigned(u(leaf, "mode"))),
            ];
            match s(leaf, "kind") {
                "symlink" => entries.push((FieldIdV1::new(3), ManifestValueV1::MachineText(text(s(leaf, "target"))))),
                _ => {
                    entries.push((FieldIdV1::new(4), ManifestValueV1::Unsigned(u(leaf, "size"))));
                    entries.push((FieldIdV1::new(5), ManifestValueV1::Bytes(digest32(leaf, "sha256").to_vec())));
                },
            }
            ManifestValueV1::Map(
                CanonicalFieldMapV1::try_from_entries(entries).unwrap_or_else(|_| die("leaf map")),
            )
        })
        .collect();
    digest_manifest_value_v1(DigestDomainIdV1::LocalReproSmoke, &Wrapper(items), &local_repro_limits_v1())
        .unwrap_or_else(|_| die("manifest root digest"))
}

/// Canary root: array of {id, expected_terminal, observed_terminal, pass}
/// in id order.
fn canary_root(canaries: &[serde_json::Value]) -> ProtocolDigestV1 {
    struct Wrapper(Vec<ManifestValueV1>);
    impl ManifestEncodeV1 for Wrapper {
        fn to_manifest_value_v1(
            &self,
        ) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
            Ok(ManifestValueV1::Array(self.0.clone()))
        }
    }
    let mut sorted: Vec<&serde_json::Value> = canaries.iter().collect();
    sorted.sort_by(|a, b| s(a, "id").as_bytes().cmp(s(b, "id").as_bytes()));
    let items = sorted
        .iter()
        .map(|c| {
            ManifestValueV1::Map(
                CanonicalFieldMapV1::try_from_entries(vec![
                    (FieldIdV1::new(0), ManifestValueV1::MachineText(text(s(c, "id")))),
                    (FieldIdV1::new(1), ManifestValueV1::MachineText(text(s(c, "expected")))),
                    (FieldIdV1::new(2), ManifestValueV1::MachineText(text(s(c, "observed")))),
                    (FieldIdV1::new(3), ManifestValueV1::Bool(b(c, "pass"))),
                ])
                .unwrap_or_else(|_| die("canary map")),
            )
        })
        .collect();
    digest_manifest_value_v1(DigestDomainIdV1::LocalReproSmoke, &Wrapper(items), &local_repro_limits_v1())
        .unwrap_or_else(|_| die("canary root digest"))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let evidence_path = args.next().unwrap_or_else(|| die("usage: apex_local_repro_record <evidence.json> <out-dir>"));
    let out_dir = std::path::PathBuf::from(args.next().unwrap_or_else(|| die("missing out-dir")));

    let raw = std::fs::read(&evidence_path).unwrap_or_else(|e| die(&format!("read {evidence_path}: {e}")));
    let v: serde_json::Value = serde_json::from_slice(&raw).unwrap_or_else(|e| die(&format!("parse evidence: {e}")));

    let terminal_name = s(&v, "terminal");
    let terminal = LocalReproTerminalV1::ALL
        .into_iter()
        .find(|t| format!("{t:?}") == terminal_name)
        .unwrap_or_else(|| die(&format!("unknown terminal {terminal_name:?}")));

    let empty = Vec::new();
    let leaves = v.get("output_leaves").and_then(|x| x.as_array()).unwrap_or(&empty);
    let canaries = v.get("canaries").and_then(|x| x.as_array()).unwrap_or(&empty);
    let hpe = v.get("host_path_evaluation").unwrap_or_else(|| die("missing host_path_evaluation"));

    let record = LocalReproducibilitySmokeV1 {
        admitted_commit: GitHexIdV1::new(s(&v, "admitted_commit"))
            .unwrap_or_else(|_| die("admitted_commit not 40-lower-hex")),
        source_closure_root: ProtocolDigestV1 {
            algorithm: DigestAlgorithmIdV1::Sha256,
            domain: DigestDomainIdV1::SourceClosure,
            bytes: DigestBytes32V1::from_array(digest32(&v, "source_closure_root_sha256")),
        },
        derivation_path: text(s(&v, "derivation_path")),
        derivation_identity: artifact(&v, "derivation_json_sha256", "derivation_json_size"),
        output_store_path: text(s(&v, "output_store_path")),
        output_nar_identity: artifact(&v, "output_nar_sha256", "output_nar_size"),
        baseline: execution(v.get("baseline").unwrap_or_else(|| die("missing baseline"))),
        baseline_built_this_run: b(&v, "baseline_built_this_run"),
        rebuilds: v
            .get("rebuilds")
            .and_then(|x| x.as_array())
            .unwrap_or_else(|| die("missing rebuilds"))
            .iter()
            .map(execution)
            .collect(),
        host_path_evaluation: HostPathEvaluationV1 {
            materialization_a_token: token(s(hpe, "path_a")),
            materialization_b_token: token(s(hpe, "path_b")),
            closure_roots_equal: b(hpe, "closure_roots_equal"),
            derivations_equal: b(hpe, "derivations_equal"),
        },
        output_manifest_root: manifest_root(leaves),
        canary_root: canary_root(canaries),
        terminal,
    };

    let limits = local_repro_limits_v1();
    let cbor = encode_manifest_v1(&record, &limits).unwrap_or_else(|e| die(&format!("encode: {e}")));
    // Decode-roundtrip self-check — this is where the PASS admission rules
    // (two current-run executions, ordinals, kinds) structurally bite.
    let decoded: LocalReproducibilitySmokeV1 =
        decode_manifest_v1(&cbor, &limits).unwrap_or_else(|e| die(&format!("self-check decode: {e}")));
    if decoded != record || encode_manifest_v1(&decoded, &limits).unwrap_or_else(|e| die(&format!("re-encode: {e}"))) != cbor {
        die("self-check: record is not a fixed point of decode->encode");
    }
    let root = record.canonical_root().unwrap_or_else(|e| die(&format!("canonical root: {e}")));

    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| die(&format!("create {out_dir:?}: {e}")));
    let base = format!("apex-local-repro-smoke-{}", record.admitted_commit.as_str());
    let write_atomic = |name: &str, bytes: &[u8]| {
        let tmp = out_dir.join(format!("{name}.tmp"));
        let mut f = std::fs::File::create(&tmp).unwrap_or_else(|e| die(&format!("create {tmp:?}: {e}")));
        f.write_all(bytes).unwrap_or_else(|e| die(&format!("write {tmp:?}: {e}")));
        f.sync_all().unwrap_or_else(|e| die(&format!("fsync {tmp:?}: {e}")));
        drop(f);
        std::fs::rename(&tmp, out_dir.join(name)).unwrap_or_else(|e| die(&format!("rename {name}: {e}")));
    };
    write_atomic(&format!("{base}.cbor"), &cbor);
    let cbor_sha = hex32(&sha256_of(&cbor));
    write_atomic(&format!("{base}.cbor.sha256"), format!("{cbor_sha}  {base}.cbor\n").as_bytes());

    println!("record={}", out_dir.join(format!("{base}.cbor")).display());
    println!("canonical_cbor_sha256={cbor_sha}");
    println!("record_root={}", hex32(root.bytes.as_array()));
    println!("terminal_encoded={terminal_name}");
    if terminal == LocalReproTerminalV1::Pass {
        println!("TERMINAL: T1.3-LOCAL-REPRO-SMOKE-PASS");
    } else {
        println!("TERMINAL: T1.3-{}", terminal_name.to_uppercase());
        std::process::exit(9);
    }
}
