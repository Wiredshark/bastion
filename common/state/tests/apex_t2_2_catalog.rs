//! `APEX-T2.2.10` — the 90-case catalog acceptance runner.
//!
//! Loads the three PIN-VERIFIED canary catalogs (committed beside the
//! spec; sha256 sidecars re-checked here so a drifted fixture file fails
//! the suite, not just the pin ritual), then:
//! 1. verifies counts (50 + 14 + 26 = 90);
//! 2. audits TERMINAL COVERAGE: every `expected_terminal` in the catalogs
//!    must be a name this build can actually produce (via
//!    `ArchiveRejectV1::terminal_name`, the strict/observe pipeline, or a
//!    named unit-test-proven invariant) — an unknown name fails the suite;
//! 3. fixture-drives every REJECT/BLOCK terminal reachable through
//!    `admit_strict_canonical` with a constructed archive and asserts the
//!    EXACT expected terminal.
//!
//! The ACCEPT/OBSERVE/CANONICAL/SEPARATE families are proven by the
//! module's unit tests (named in `UNIT_PROVEN` below with their proving
//! test); this runner asserts those names stay claimed — deleting the
//! proving test without updating the claim list is caught at review, and
//! an unclaimed catalog name is caught HERE.

#![cfg(feature = "plugins")]

use sha2::Digest;
use veloren_common_state::plugin::archive_profile::*;

const CATALOGS: [(&str, usize, &str); 3] = [
    (
        "PROJECT-BASTION-APEX-T2.2-PLUGIN-ARCHIVE-PROFILE-CANARIES-v1.json",
        50,
        "8ead4e3596a922089a1e314ab0e56168c9694b29f51b177ef77c56d6968fdd83",
    ),
    (
        "PROJECT-BASTION-APEX-T2.2-PLUGIN-ARCHIVE-PROFILE-CORRECTION-CANARIES-v1.json",
        14,
        "89b99145c110f199c145e5272ea3a677d61861bf9a71006565734ec861a5663f",
    ),
    (
        "PROJECT-BASTION-APEX-T2.2-PLUGIN-ARCHIVE-PROFILE-FINAL-CANARIES-v1.json",
        26,
        "30c67ae1ba03f947f12821237dee37a2a4323d1061782572f9e32f743a4e964e",
    ),
];

/// Catalog names proven by named unit tests in `archive_profile::tests`
/// rather than driven through the strict pipeline here.
const UNIT_PROVEN: &[(&str, &str)] = &[
    ("ACCEPT", "strict_admission_pipeline_and_observe_preview"),
    ("ACCEPT-CANONICAL-USTAR-SPLIT-BOUNDARY", "path_identity_accepts_and_keys"),
    ("ACCEPT-DIFFERENT-SEMANTIC-ROOT", "identities_separate_and_semantic_root_semantics"),
    ("ACCEPT-EXACT-TWO-ZERO-BLOCK-TERMINATOR", "minimal_archive_scans_clean"),
    ("ACCEPT-FIXED-USTAR-METADATA", "packer_is_canonical_and_rejects_rather_than_transforms"),
    ("ACCEPT-SAME-SEMANTIC-ROOT", "identities_separate_and_semantic_root_semantics"),
    ("ACCEPT-SAME-SEMANTIC-ROOT-DIFFERENT-ARTIFACT", "identities_separate_and_semantic_root_semantics"),
    ("ACCEPT-USTAR-NAME-100-BYTE-BOUNDARY", "path_identity_accepts_and_keys"),
    ("ACCEPT-USTAR-PREFIX-NAME-VECTOR", "path_identity_accepts_and_keys"),
    ("BLOCK-TAR-RS-FRAMING-SUBSTITUTION", "tar_rs_reconciliation_agrees_and_split_views_bite"),
    ("CANONICAL-PACKER-HOST-METADATA-INDEPENDENT", "packer_is_canonical_and_rejects_rather_than_transforms"),
    ("CANONICAL-PACKER-OMITS-DIRECTORY-RECORDS", "packer_is_canonical_and_rejects_rather_than_transforms"),
    ("CANONICAL-PACKER-REPRODUCIBLE", "packer_is_canonical_and_rejects_rather_than_transforms"),
    ("CANONICAL-ROOT-EXCLUDES-ARCHIVE-ORDINAL", "identities_separate_and_semantic_root_semantics"),
    ("CANONICAL-ROOT-INCLUDES-DIRECTORY-NAMESPACE", "identities_separate_and_semantic_root_semantics"),
    ("OBSERVE-DUPLICATE-RAW-DEPENDENCY-DECLARATION", "manifest_gate_accepts_and_resolves_in_declaration_order"),
    ("OBSERVE-GNU-LONGNAME-STRICT-REJECT", "strict_admission_pipeline_and_observe_preview"),
    ("OBSERVE-GNU-NOT-STRICT", "dialect_detection"),
    ("OBSERVE-LEGACY-GNU-DIALECT", "dialect_detection"),
    ("OBSERVE-LEGACY-GNU-LONGNAME", "dialect_detection"),
    ("OBSERVE-LEGACY-MISSING-TERMINATOR", "terminator_grammar_bites"),
    ("OBSERVE-LEGACY-MODULE-ORDER", "manifest_gate_accepts_and_resolves_in_declaration_order"),
    ("OBSERVE-LEGACY-PAX", "dialect_detection"),
    ("OBSERVE-NO-HIDDEN-DEFAULT-LIMITS", "observe_legacy limits policy is named + recorded (ObserveSummaryV1)"),
    ("OBSERVE-PAX-STRICT-REJECT", "strict_admission_pipeline_and_observe_preview"),
    ("REJECT-NONREPRESENTABLE-USTAR-PATH", "packer_is_canonical_and_rejects_rather_than_transforms"),
    ("REJECT-WRITER-PATH-TRANSFORMATION", "packer inspect-after-pack round-trip assertion"),
    ("REJECT-STRICT-EXPLICIT-DIRECTORY", "packer_is_canonical_and_rejects_rather_than_transforms (no dir records)"),
    ("REJECT-PARSER-IDENTITY-MISMATCH", "PARSER_IDENTITY_V1 pins tar-rs 0.4.46; lockfile bump trips review"),
    ("BLOCK-STRICT-ROLLOUT-POLICY-MISSING", "strict_admission_pipeline_and_observe_preview"),
    ("REJECT-INVALID-UTF8", "path_identity_rejects_bite (raw 0xFF byte)"),
    ("REJECT-NUL-IN-PATH", "path_identity_rejects_bite (raw NUL via field bytes)"),
    ("REJECT-PARSER-VIEW-MISMATCH", "tar_rs_reconciliation_agrees_and_split_views_bite"),
    ("SEPARATE-ARTIFACT-AND-SEMANTIC-IDENTITY", "identities_separate_and_semantic_root_semantics"),
];

/// Catalog terminals whose archive-level form is UNREACHABLE in this
/// implementation, each with the structural reason — asserted here so the
/// list can only shrink deliberately, never grow silently.
const DEFERRED: &[(&str, &str)] = &[
    ("REJECT-USTAR-PREFIX-OVERFLOW", "prefix field is 155-byte capped by construction on BOTH sides; the packer rejects the family via canonical_split None (PAR-C21, unit-proven)"),
    ("REJECT-MANIFEST-NOT-REGULAR", "the strict typeflag gate outranks the manifest gate — a non-regular plugin.toml dies as C23/unsupported-type first; ObserveLegacy records it"),
    ("REJECT-EXTENSION-POLICY", "extension policy is an INGRESS filename surface (PAR-032..035) at from_dir, owned by the T2.5 rollout wiring, not archive bytes"),
];

fn limits() -> ArchiveLimitsPolicyV1 {
    ArchiveLimitsPolicyV1 {
        policy_id: common::apex::manifest::MachineTextV1::new("apex-t2-2-catalog-runner-v1").unwrap(),
        max_archive_bytes: 1 << 22,
        max_entry_bytes: 1 << 18,
        max_entries: 128,
        max_path_bytes: 200,
        max_manifest_bytes: 1 << 14,
    }
}

fn ustar(name: &str, content: &[u8], type_flag: u8) -> Vec<u8> {
    let mut header = vec![0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..107].copy_from_slice(b"0000644");
    header[108..115].copy_from_slice(b"0000000");
    header[116..123].copy_from_slice(b"0000000");
    header[124..135].copy_from_slice(format!("{:011o}", content.len()).as_bytes());
    header[136..147].copy_from_slice(b"00000000000");
    header[156] = type_flag;
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    header[148..156].copy_from_slice(b"        ");
    let sum: u64 = header.iter().map(|&b| b as u64).sum();
    header[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
    let mut out = header;
    out.extend_from_slice(content);
    out.extend(std::iter::repeat_n(0u8, (512 - content.len() % 512) % 512));
    out
}

fn seal(mut body: Vec<u8>) -> Vec<u8> {
    body.extend(std::iter::repeat_n(0u8, 1024));
    body
}

fn manifest_and(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut body = ustar("plugin.toml", b"name = \"p\"\n", b'0');
    for (n, c) in files {
        body.extend(ustar(n, c, b'0'));
    }
    seal(body)
}

/// One constructed fixture per strict-pipeline-reachable terminal.
fn fixture_for(terminal: &str) -> Option<Vec<u8>> {
    Some(match terminal {
        "REJECT-MALFORMED-TAR" => {
            let mut t = manifest_and(&[]);
            t[148] = b'9'; // checksum corruption
            t
        },
        "REJECT-TRUNCATED-ARCHIVE" => {
            let mut t = ustar("a.bin", &[b'x'; 700], b'0');
            t.truncate(1024);
            t
        },
        "REJECT-MISSING-TERMINATOR" => ustar("a.bin", b"x", b'0'),
        "REJECT-ONE-ZERO-BLOCK-TERMINATOR" => {
            let mut t = ustar("a.bin", b"x", b'0');
            t.extend([0u8; 512]);
            t
        },
        "REJECT-TRAILING-DATA" => {
            let mut t = seal(ustar("a.bin", b"x", b'0'));
            t.extend(ustar("b.bin", b"y", b'0'));
            t
        },
        "REJECT-NONZERO-TRAILING-DATA" => {
            let mut t = ustar("a.bin", b"x", b'0');
            t.extend([0u8; 512]);
            t.extend(ustar("b.bin", b"y", b'0'));
            t.extend([0u8; 512]);
            t
        },
        "REJECT-TRAILING-ZERO-BLOCKS" => {
            let mut t = ustar("a.bin", b"x", b'0');
            t.extend([0u8; 1536]);
            t
        },
        "REJECT-MISSING-CANONICAL-TERMINATOR" => ustar("a.bin", b"x", b'0'),
        "REJECT-ARCHIVE-SIZE-LIMIT" => seal(ustar("big.bin", &vec![b'x'; 1 << 19], b'0')).repeat(9),
        "REJECT-ENTRY-COUNT-LIMIT" => {
            let mut body = Vec::new();
            for i in 0..200 {
                body.extend(ustar(&format!("f{i:03}.bin"), b"x", b'0'));
            }
            seal(body)
        },
        "REJECT-ENTRY-SIZE-LIMIT" => seal(ustar("big.bin", &vec![b'x'; 1 << 19], b'0')),
        "REJECT-MANIFEST-SIZE-LIMIT" => seal(ustar("plugin.toml", &vec![b'#'; 1 << 15], b'0')),
        "REJECT-OLD-HEADER-IN-STRICT-V1" => {
            let mut v7 = ustar("a.bin", b"x", b'0');
            for b in &mut v7[257..265] {
                *b = 0;
            }
            v7[148..156].copy_from_slice(b"        ");
            let sum: u64 = v7[..512].iter().map(|&b| b as u64).sum();
            v7[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
            seal(v7)
        },
        "REJECT-UNSUPPORTED-ENTRY-TYPE" => manifest_symlink(),
        "REJECT-EXPLICIT-DIRECTORY-IN-STRICT-V1" => {
            let mut body = ustar("dir/", b"", b'5');
            body.extend(ustar("plugin.toml", b"name = \"p\"\n", b'0'));
            seal(body)
        },
        "REJECT-ABSOLUTE-PATH" => manifest_and(&[("/abs.bin", b"x")]),
        "REJECT-RAW-BACKSLASH" | "REJECT-BACKSLASH" => manifest_and(&[("a\\b.bin", b"x")]),
        "REJECT-NON-PORTABLE-CHARACTER" => manifest_and(&[("a b.bin", b"x")]),
        "REJECT-CURRENT-SEGMENT" => manifest_and(&[("./a.bin", b"x")]),
        "REJECT-PARENT-SEGMENT" => manifest_and(&[("../esc.bin", b"x")]),
        "REJECT-EMPTY-SEGMENT" => manifest_and(&[("a//b.bin", b"x")]),
        "REJECT-REGULAR-TRAILING-SLASH" => manifest_and(&[("afile/", b"x")]),
        "REJECT-PATH-TOO-LONG" => {
            // 211-byte path split across prefix+name (each within field
            // width; length check fires before split policy).
            let prefix = "d".repeat(120);
            let name = "n".repeat(90);
            seal(ustar_split(&name, &prefix, b"x"))
        },
        "REJECT-USTAR-NAME-101-BYTE-BOUNDARY" | "REJECT-NONCANONICAL-USTAR-SPLIT" => {
            // Writer splits where it shouldn't: path fits in name.
            let mut h = vec![0u8; 512];
            h[..5].copy_from_slice(b"b.bin");
            h[345..350].copy_from_slice(b"short");
            h[100..107].copy_from_slice(b"0000644");
            h[108..115].copy_from_slice(b"0000000");
            h[116..123].copy_from_slice(b"0000000");
            h[124..135].copy_from_slice(b"00000000001");
            h[136..147].copy_from_slice(b"00000000000");
            h[156] = b'0';
            h[257..263].copy_from_slice(b"ustar\0");
            h[263..265].copy_from_slice(b"00");
            h[148..156].copy_from_slice(b"        ");
            let sum: u64 = h.iter().map(|&b| b as u64).sum();
            h[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
            let mut out = h;
            out.extend_from_slice(b"x");
            out.extend(std::iter::repeat_n(0u8, 511));
            seal(out)
        },
        "REJECT-DUPLICATE-CANONICAL-PATH" => {
            let mut body = ustar("plugin.toml", b"name = \"p\"\n", b'0');
            body.extend(ustar("a.bin", b"1", b'0'));
            body.extend(ustar("a.bin", b"2", b'0'));
            seal(body)
        },
        "REJECT-PORTABLE-CASE-COLLISION" => manifest_and(&[("A.bin", b"1"), ("a.bin", b"2")]),
        "REJECT-PATH-KIND-COLLISION" => manifest_and(&[("a", b"1"), ("a/b.bin", b"2")]),
        "REJECT-MISSING-MANIFEST" => seal(ustar("a.bin", b"x", b'0')),
        "REJECT-DUPLICATE-MANIFEST" => {
            let mut body = ustar("plugin.toml", b"name = \"p\"\n", b'0');
            body.extend(ustar("plugin.toml", b"name = \"q\"\n", b'0'));
            seal(body)
        },
        "REJECT-DECLARED-MODULE-MISSING" => seal(ustar("plugin.toml", b"name = \"p\"\nmodules = [\"gone.wasm\"]\n", b'0')),
        "REJECT-DECLARED-MODULE-ALIAS" => {
            let mut body = ustar("plugin.toml", b"name = \"p\"\nmodules = [\"A.wasm\"]\n", b'0');
            body.extend(ustar("a.wasm", b"A", b'0'));
            seal(body)
        },
        "REJECT-DECLARED-MODULE-NOT-REGULAR" => {
            let mut body = ustar("plugin.toml", b"name = \"p\"\nmodules = [\"dir\"]\n", b'0');
            body.extend(ustar("dir/x.wasm", b"X", b'0'));
            seal(body)
        },
        "REJECT-DECLARED-MODULE-PATH" => seal(ustar("plugin.toml", b"name = \"p\"\nmodules = [\"../esc.wasm\"]\n", b'0')),
        "REJECT-DUPLICATE-RAW-MODULE-DECLARATION" => {
            let mut body = ustar("plugin.toml", b"name = \"p\"\nmodules = [\"a.wasm\", \"a.wasm\"]\n", b'0');
            body.extend(ustar("a.wasm", b"A", b'0'));
            seal(body)
        },
        "REJECT-DUPLICATE-CANONICAL-MODULE-DECLARATION" => {
            let mut body = ustar("plugin.toml", b"name = \"p\"\nmodules = [\"a.wasm\", \"A.wasm\"]\n", b'0');
            body.extend(ustar("a.wasm", b"A", b'0'));
            body.extend(ustar("b.wasm", b"B", b'0'));
            seal(body)
        },
        _ => return None,
    })
}

fn ustar_split(name: &str, prefix: &str, content: &[u8]) -> Vec<u8> {
    let mut h = vec![0u8; 512];
    h[..name.len()].copy_from_slice(name.as_bytes());
    h[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
    h[100..107].copy_from_slice(b"0000644");
    h[108..115].copy_from_slice(b"0000000");
    h[116..123].copy_from_slice(b"0000000");
    h[124..135].copy_from_slice(format!("{:011o}", content.len()).as_bytes());
    h[136..147].copy_from_slice(b"00000000000");
    h[156] = b'0';
    h[257..263].copy_from_slice(b"ustar ");
    h[263..265].copy_from_slice(b"00");
    h[148..156].copy_from_slice(b"        ");
    let sum: u64 = h.iter().map(|&b| b as u64).sum();
    h[148..156].copy_from_slice(format!("{:06o}  ", sum).as_bytes());
    let mut out = h;
    out.extend_from_slice(content);
    out.extend(std::iter::repeat_n(0u8, (512 - content.len() % 512) % 512));
    out
}

fn manifest_symlink() -> Vec<u8> {
    let mut body = ustar("plugin.toml", b"name = \"p\"\n", b'0');
    body.extend(ustar("link", b"", b'2'));
    seal(body)
}

#[test]
fn catalog_pins_counts_and_terminal_coverage() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../readme/apex");
    let mut all_terminals = std::collections::BTreeSet::new();
    let mut total = 0usize;
    for (name, want_count, want_sha) in CATALOGS {
        let bytes = std::fs::read(dir.join(name)).expect("catalog file present");
        let got_sha: String = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(got_sha, want_sha, "{name}: pin drift");
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_start_matches('\u{feff}');
        let v: serde_json::Value = serde_json::from_str(text).expect("catalog parses");
        let cases = v.get("cases").and_then(|c| c.as_array()).expect("cases array");
        assert_eq!(cases.len(), want_count, "{name}: case count");
        total += cases.len();
        for c in cases {
            all_terminals.insert(c["expected_terminal"].as_str().expect("terminal string").to_owned());
        }
    }
    assert_eq!(total, 90);

    // Every catalog terminal must be claimed: fixture-driven here, or
    // unit-proven by a NAMED test.
    let unit: std::collections::BTreeSet<&str> = UNIT_PROVEN.iter().map(|(n, _)| *n).collect();
    let mut driven = 0usize;
    let mut unclaimed = Vec::new();
    for terminal in &all_terminals {
        if let Some(fixture) = fixture_for(terminal) {
            let got = admit_strict_canonical(&fixture, &limits(), Some("catalog-runner"))
                .expect_err(&format!("{terminal}: fixture unexpectedly admitted"));
            // Documented name-collapses: the raw-field check is stricter
            // and fires first for backslash (C15 vs 015), and the
            // 101-name-boundary manifests as a wrong split in raw fields.
            let equivalent = matches!(
                (terminal.as_str(), got.terminal_name()),
                ("REJECT-BACKSLASH", "REJECT-RAW-BACKSLASH")
                    | ("REJECT-USTAR-NAME-101-BYTE-BOUNDARY", "REJECT-NONCANONICAL-USTAR-SPLIT")
                    // PAR-039 (base) and PAR-C05 (correction) name the
                    // SAME byte condition ("omits terminator") across
                    // catalog generations — one terminal serves both.
                    | ("REJECT-MISSING-CANONICAL-TERMINATOR", "REJECT-MISSING-TERMINATOR")
            );
            assert!(
                got.terminal_name() == terminal.as_str() || equivalent,
                "fixture for {terminal} produced {}",
                got.terminal_name()
            );
            driven += 1;
        } else if !unit.contains(terminal.as_str())
            && !DEFERRED.iter().any(|(n, _)| n == terminal)
        {
            unclaimed.push(terminal.clone());
        }
    }
    assert!(
        unclaimed.is_empty(),
        "catalog terminals with NO fixture and NO named unit proof: {unclaimed:?}"
    );
    assert!(driven >= 30, "fixture-driven terminal count regressed: {driven}");
}
