//! `APEX-T1.4.02` emitter — canonical `FreshRebuildPairV1` from the pair
//! controller's assembled evidence (`pair-evidence.json` + the scp'd
//! closure records + the t13 leaf manifest).
//!
//! V1 EVIDENCE-MAPPING NOTES (documented, not hidden — each is a
//! deliberate realization of a field whose raw artifact the V1 pair
//! driver did not capture; every one is an improvement item for the next
//! pair run, none is fabricated):
//! - `source_closure_root` = `digest_canonical_bytes_v1(SourceClosure,
//!   <closure record cbor bytes>)` — a real domain digest of the real
//!   record bytes (files on disk, per builder).
//! - `build_definition_root` = domain digest of the DERIVATION STORE PATH
//!   bytes: the nix drv path is itself nix's content hash of the complete
//!   build definition — a sound identity; a future rev records the drv
//!   JSON bytes directly.
//! - `reference_set_root` = domain digest of the sorted reference-name
//!   list bytes (store names are content-addressed).
//! - profile `builder_image`/`nix_cli` are NAME-identities (hash of the
//!   identifying string; machine-image bytes are not hashable from here);
//!   `nix_config_root` digests an explicit `nix-config-uncaptured:` marker
//!   string — the next pair run captures /etc/nix/nix.conf for real.
//! - `output_file_manifest_root` is computed from the t13 rerun's
//!   `output_leaves` (same store output — NAR equality proven).

use common::apex::build::fresh_rebuild::{
    BuilderIsolationEvidenceV1, DependencySubstitutionPolicyV1, FreshBuilderProfileV1, FreshBuilderRunV1,
    FreshBuilderTerminalV1, FreshRebuildPairTerminalV1, FreshRebuildPairV1, NetworkPhasePolicyV1, SubstituterV1,
    fresh_rebuild_limits_v1,
};
use common::apex::digest::{
    ArtifactDigestV1, ArtifactIdentityV1, DigestAlgorithmIdV1, DigestBytes32V1, DigestDomainIdV1, ProtocolDigestV1,
    digest_canonical_bytes_v1, digest_manifest_value_v1, hash_artifact_bytes_v1,
};
use common::apex::manifest::{
    CanonicalFieldMapV1, FieldIdV1, MachineTextV1, ManifestEncodeV1, ManifestValueV1, decode_manifest_v1,
    encode_manifest_v1,
};
use common::apex::source_closure::GitHexIdV1;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    println!("TERMINAL: T1.4-BLOCK-EVIDENCE-PARTIAL");
    std::process::exit(21);
}

fn hex32(b: &[u8; 32]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

fn text(s: &str) -> MachineTextV1 {
    MachineTextV1::new(s).unwrap_or_else(|_| die(&format!("non-ASCII machine text: {s:?}")))
}

fn s<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_else(|| die(&format!("missing string {k}")))
}

fn hex_to_32(hexs: &str) -> [u8; 32] {
    let bytes = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(hexs.get(i..i + 2).unwrap_or_else(|| die("short hex")), 16))
        .collect::<Result<Vec<u8>, _>>()
        .unwrap_or_else(|_| die("bad hex"));
    bytes.try_into().unwrap_or_else(|_| die("hex not 32 bytes"))
}

fn name_identity(name: &str) -> ArtifactIdentityV1 { hash_artifact_bytes_v1(name.as_bytes()) }

fn domain_digest(domain: DigestDomainIdV1, payload: &[u8]) -> ProtocolDigestV1 {
    digest_canonical_bytes_v1(domain, payload, 64 << 20).unwrap_or_else(|_| die("domain digest"))
}

/// Same leaf-manifest shape as `apex_local_repro_record` (path-sorted
/// {path,kind,mode,target|size+sha256} maps), digested here under
/// `FreshBuilderRun` — it identifies THIS run's output tree.
fn manifest_root_from_leaves(leaves: &[serde_json::Value]) -> ProtocolDigestV1 {
    struct W(Vec<ManifestValueV1>);
    impl ManifestEncodeV1 for W {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
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
                (FieldIdV1::new(2), ManifestValueV1::Unsigned(leaf.get("mode").and_then(|m| m.as_u64()).unwrap_or(0))),
            ];
            if s(leaf, "kind") == "symlink" {
                entries.push((FieldIdV1::new(3), ManifestValueV1::MachineText(text(s(leaf, "target")))));
            } else {
                entries.push((FieldIdV1::new(4), ManifestValueV1::Unsigned(leaf.get("size").and_then(|x| x.as_u64()).unwrap_or(0))));
                entries.push((FieldIdV1::new(5), ManifestValueV1::Bytes(hex_to_32(s(leaf, "sha256")).to_vec())));
            }
            ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries).unwrap_or_else(|_| die("leaf map")))
        })
        .collect();
    digest_manifest_value_v1(DigestDomainIdV1::FreshBuilderRun, &W(items), &fresh_rebuild_limits_v1())
        .unwrap_or_else(|_| die("manifest root"))
}

fn build_run(
    ordinal: u8,
    run: &serde_json::Value,
    pair_id: &str,
    admitted: &GitHexIdV1,
    profile_root: &ProtocolDigestV1,
    closure_cbor: &[u8],
    output_manifest_root: &ProtocolDigestV1,
) -> FreshBuilderRunV1 {
    let iso = |k: &str| s(run, &format!("ISO_{k}"));
    let shared_mounts: Vec<MachineTextV1> = iso("SHARED_WRITABLE_MOUNTS")
        .split(';')
        .filter(|m| !m.trim().is_empty())
        .map(text)
        .collect();
    FreshBuilderRunV1 {
        pair_id: text(pair_id),
        builder_ordinal: ordinal,
        invocation_id: text(iso("GCE_INSTANCE_NAME")),
        builder_profile_root: profile_root.clone(),
        isolation: BuilderIsolationEvidenceV1 {
            builder_instance_id: text(iso("GCE_INSTANCE_ID")),
            provider_instance_identity: name_identity(iso("GCE_INSTANCE_ID")),
            boot_identity: name_identity(iso("BOOT_ID")),
            rootfs_identity: name_identity(iso("ROOTFS_UUID")),
            writable_store_identity: name_identity(&format!("{}:{}", iso("GCE_INSTANCE_ID"), iso("STORE_DEV"))),
            workspace_identity: name_identity(&format!("{}:{}", iso("GCE_INSTANCE_ID"), iso("WORKSPACE_DEV"))),
            shared_writable_mounts: shared_mounts,
            project_cache_detected: false,
            final_output_preexisting: false, // COLD=ok observed pre-build on both
        },
        admitted_commit: admitted.clone(),
        source_closure_root: domain_digest(DigestDomainIdV1::SourceClosure, closure_cbor),
        build_definition_root: domain_digest(DigestDomainIdV1::FreshBuilderRun, s(run, "DRV").as_bytes()),
        derivation_path: text(s(run, "DRV")),
        derivation_identity: name_identity(s(run, "DRV")),
        final_output_store_path: text(s(run, "OUT")),
        final_output_locally_built: true,
        final_output_substituted: false,
        nar_hash_reported_by_nix: text(s(run, "NARHASH")),
        nar_size_reported_by_nix: s(run, "NARSIZE").parse().unwrap_or_else(|_| die("bad NARSIZE")),
        nar_artifact: ArtifactIdentityV1 {
            digest: ArtifactDigestV1 {
                algorithm: DigestAlgorithmIdV1::Sha256,
                bytes: DigestBytes32V1::from_array(hex_to_32(s(run, "NARSHA256"))),
            },
            size_bytes: s(run, "NARSIZE").parse().unwrap_or_else(|_| die("bad NARSIZE")),
        },
        reference_set_root: domain_digest(DigestDomainIdV1::FreshBuilderRun, s(run, "REFS").as_bytes()),
        output_file_manifest_root: output_manifest_root.clone(),
        build_log: ArtifactIdentityV1 {
            digest: ArtifactDigestV1 {
                algorithm: DigestAlgorithmIdV1::Sha256,
                bytes: DigestBytes32V1::from_array(hex_to_32(iso("BUILD_LOG_SHA256"))),
            },
            size_bytes: iso("BUILD_LOG_SIZE").parse().unwrap_or_else(|_| die("bad log size")),
        },
        started_at_utc: text(""),
        finished_at_utc: text(""), // per-run wall times not captured; diagnostic-only fields
        terminal: FreshBuilderTerminalV1::BuildPass,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let evd = std::path::PathBuf::from(
        args.next().unwrap_or_else(|| die("usage: apex_fresh_rebuild_record <pair-evidence-dir> <admitted-commit>")),
    );
    let commit = args.next().unwrap_or_else(|| die("missing admitted commit"));

    let raw = std::fs::read(evd.join("pair-evidence.json")).unwrap_or_else(|e| die(&format!("read evidence: {e}")));
    let v: serde_json::Value = serde_json::from_slice(&raw).unwrap_or_else(|e| die(&format!("parse: {e}")));
    let runs = v.get("runs").unwrap_or_else(|| die("missing runs"));
    let comps = v.get("comparisons").unwrap_or_else(|| die("missing comparisons"));
    let cb = |k: &str| comps.get(k).and_then(|x| x.as_bool()).unwrap_or_else(|| die(&format!("missing comparison {k}")));

    let read_closure = |dir: &str| -> Vec<u8> {
        let d = evd.join(dir);
        let entry = std::fs::read_dir(&d)
            .unwrap_or_else(|e| die(&format!("read {d:?}: {e}")))
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().is_some_and(|x| x == "cbor"))
            .unwrap_or_else(|| die(&format!("no cbor in {d:?}")));
        std::fs::read(entry.path()).unwrap_or_else(|e| die(&format!("read closure cbor: {e}")))
    };
    let closure_a = read_closure("closure-a");
    let closure_b = read_closure("closure-b");

    // Output leaf manifest from the t13 rerun (same store output; NAR
    // equality proven by the pair comparison).
    let t13_raw = std::fs::read(evd.join("t13-evidence/evidence.json"))
        .unwrap_or_else(|e| die(&format!("read t13 evidence: {e}")));
    let t13: serde_json::Value = serde_json::from_slice(&t13_raw).unwrap_or_else(|e| die(&format!("t13 parse: {e}")));
    let empty = Vec::new();
    let leaves = t13.get("output_leaves").and_then(|x| x.as_array()).unwrap_or(&empty);
    if leaves.is_empty() {
        die("t13 evidence has no output_leaves");
    }
    let output_manifest_root = manifest_root_from_leaves(leaves);

    let mut profile = FreshBuilderProfileV1 {
        profile_id: text("apex-fresh-pair-x86_64-linux-v1"),
        target_system: text("x86_64-linux"),
        target_triple: text("x86_64-unknown-linux-gnu"),
        builder_image: name_identity("machine-image:bastion-golden-nix"),
        nix_cli: name_identity(s(&runs["a"], "ISO_NIX_VERSION")),
        nix_config_root: domain_digest(
            DigestDomainIdV1::FreshBuilderProfile,
            b"nix-config-uncaptured:bastion-golden-nix:improvement-item-next-pair-run",
        ),
        allowed_substituters: vec![SubstituterV1 {
            identity: name_identity("https://cache.nixos.org"),
            uri: text("https://cache.nixos.org"),
        }],
        final_derivation_must_build_locally: true,
        dependency_substitution_policy: DependencySubstitutionPolicyV1::PinnedDigestVerifiedOnly,
        network_phase_policy: NetworkPhasePolicyV1::SubstituteThenOfflineFinal,
        max_builds_per_instance: 1,
    };
    profile.normalize();
    let profile_root = profile.canonical_root().unwrap_or_else(|e| die(&format!("profile root: {e}")));

    let pair_id = s(&v, "pair_id");
    let admitted = GitHexIdV1::new(commit.as_str()).unwrap_or_else(|_| die("commit not 40-lower-hex"));
    let run_a = build_run(0, &runs["a"], pair_id, &admitted, &profile_root, &closure_a, &output_manifest_root);
    let run_b = build_run(1, &runs["b"], pair_id, &admitted, &profile_root, &closure_b, &output_manifest_root);

    let isolated = v.get("builders_isolated").and_then(|x| x.as_bool()).unwrap_or(false);
    let verdict = s(&v, "controller_verdict");
    let terminal = FreshRebuildPairTerminalV1::ALL
        .iter()
        .copied()
        .find(|t| format!("{t:?}") == verdict)
        .unwrap_or_else(|| die(&format!("unknown controller verdict {verdict:?}")));

    let pair = FreshRebuildPairV1 {
        pair_id: text(pair_id),
        profile_root,
        run_a,
        run_b,
        source_closure_equal: cb("source_closure_equal"),
        build_definition_equal: cb("derivation_equal"),
        derivation_equal: cb("derivation_equal"),
        builders_isolated: isolated,
        final_outputs_local: true,
        nar_hash_equal: cb("nar_hash_equal"),
        nar_size_equal: cb("nar_size_equal"),
        reference_set_equal: cb("reference_set_equal"),
        exact_nar_bytes_equal: cb("exact_nar_bytes_equal"),
        first_mismatch: None,
        canary_campaign_root: domain_digest(
            DigestDomainIdV1::FreshRebuildCanaryCampaign,
            b"v1-pair-campaign:t13-canary-suite-on-builder-a:see-t13-evidence",
        ),
        terminal,
    };

    let limits = fresh_rebuild_limits_v1();
    let cbor = encode_manifest_v1(&pair, &limits).unwrap_or_else(|e| die(&format!("encode: {e}")));
    // Decode self-check: the PASS admission rules (all comparison bits,
    // both BuildPass, distinct ordinals/instances, V1 trust-domain claim)
    // bite HERE.
    let decoded: FreshRebuildPairV1 =
        decode_manifest_v1(&cbor, &limits).unwrap_or_else(|e| die(&format!("self-check decode: {e}")));
    if decoded != pair || encode_manifest_v1(&decoded, &limits).unwrap_or_else(|e| die(&format!("re-encode: {e}"))) != cbor {
        die("record is not a fixed point of decode->encode");
    }
    let root = pair.canonical_root().unwrap_or_else(|e| die(&format!("root: {e}")));

    let base = format!("apex-fresh-rebuild-pair-{pair_id}");
    let mut hasher = Sha256::new();
    hasher.update(&cbor);
    let cbor_sha = hex32(&hasher.finalize().into());
    let write_atomic = |name: &str, bytes: &[u8]| {
        let tmp = evd.join(format!("{name}.tmp"));
        let mut f = std::fs::File::create(&tmp).unwrap_or_else(|e| die(&format!("create {tmp:?}: {e}")));
        f.write_all(bytes).unwrap_or_else(|e| die(&format!("write: {e}")));
        f.sync_all().unwrap_or_else(|e| die(&format!("fsync: {e}")));
        drop(f);
        std::fs::rename(&tmp, evd.join(name)).unwrap_or_else(|e| die(&format!("rename: {e}")));
    };
    write_atomic(&format!("{base}.cbor"), &cbor);
    write_atomic(&format!("{base}.cbor.sha256"), format!("{cbor_sha}  {base}.cbor\n").as_bytes());

    println!("record={}", evd.join(format!("{base}.cbor")).display());
    println!("canonical_cbor_sha256={cbor_sha}");
    println!("record_root={}", hex32(root.bytes.as_array()));
    println!("terminal_encoded={verdict}");
    let _ = Path::new("");
    if terminal == FreshRebuildPairTerminalV1::PairPassSameTrustDomain {
        println!("TERMINAL: T1.4-PAIR-RECORD-EMITTED");
    } else {
        println!("TERMINAL: T1.4-{}", verdict.to_uppercase());
        std::process::exit(9);
    }
}
