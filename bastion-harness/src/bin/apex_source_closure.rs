//! `APEX-T1.2` capture tool (spec `readme/apex/
//! APEX-T1.2-DECLARED-SOURCE-ASSET-CLOSURE-FLEET-v1.md`, minute steps
//! T1.2.03–T1.2.07): emits the `SourceClosureRecordV1` for the admitted
//! commit, or dies on a typed `T1.2-BLOCK-*` terminal.
//!
//! COMMIT-PURITY: every root is computed from GIT CONTENT — `git ls-tree
//! -r HEAD` for (mode, oid, path), `git cat-file --batch` for blob bytes,
//! and the VERIFIED LFS pointer oid for LFS files — never from
//! checkout-materialized bytes. A Windows autocrlf checkout and a Linux
//! checkout of the same commit therefore produce byte-identical records
//! (T1.2.06's requirement). The working tree is read exactly once, in the
//! LFS verification pass, whose job is precisely to prove disk bytes match
//! the pointer's declared oid.
//!
//! Entry gate REUSES A.1 (`tools/apex-source-admission.sh
//! --check-worktree`) — no second dirty-detector in the tree (spec
//! acceptance gate 5).

use common::apex::digest::{ArtifactIdentityV1, DigestBytes32V1, hash_artifact_bytes_v1};
use common::apex::manifest::{CanonicalPathV1, MachineTextV1, decode_manifest_v1, encode_manifest_v1};
use common::apex::source_closure::{
    BuildScriptPinV1, ClosureTreeEntryV1, ClosureTreeV1, FilterSpecV1, GitHexIdV1, LfsReportEntryV1, LfsReportV1,
    PinnedFileV1, SourceClosureCountsV1, SourceClosureErrorV1, SourceClosureRecordV1, source_closure_limits_v1,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TOOL_VERSION: &str = "apex-source-closure/v1";

/// The flake's `pathsToIgnore`, replicated LITERALLY (order included). The
/// single-fileset rule: this constant is verified against `flake.nix`'s
/// own text at every run (`verify_filter_spec_matches_flake`) — if the
/// flake's list drifts, capture blocks rather than silently exporting a
/// stale scope.
const FLAKE_PATHS_TO_IGNORE: [&str; 10] = [
    "flake.nix",
    "flake.lock",
    "nix",
    "assets",
    "README.md",
    "CONTRIBUTING.md",
    "CHANGELOG.md",
    "CODE_OF_CONDUCT.md",
    ".github",
    ".gitlab",
];

const LFS_POINTER_VERSION_LINE: &str = "version https://git-lfs.github.com/spec/v1";

fn die(terminal: &str, code: i32, detail: &str) -> ! {
    if !detail.is_empty() {
        eprintln!("{detail}");
    }
    println!("TERMINAL: {terminal}");
    std::process::exit(code);
}

fn die_admission(detail: &str) -> ! { die("T1.2-BLOCK-ADMISSION", 10, detail) }
fn die_emit(detail: &str) -> ! { die("T1.2-BLOCK-EMIT", 16, detail) }

fn die_closure_error(e: &SourceClosureErrorV1, context: &str) -> ! {
    match e {
        SourceClosureErrorV1::ForbiddenGitMode { .. } | SourceClosureErrorV1::CaseFoldCollision { .. } => {
            die("T1.2-BLOCK-TREE-HAZARD", 19, &format!("{context}: {e}"))
        },
        SourceClosureErrorV1::NonCanonicalPath { .. } => die("T1.2-BLOCK-SCOPE-ESCAPE", 15, &format!("{context}: {e}")),
        _ => die("T1.2-BLOCK-EMIT", 16, &format!("{context}: {e}")),
    }
}

fn run_git(repo_root: &Path, args: &[&str]) -> Vec<u8> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .unwrap_or_else(|e| die_emit(&format!("failed to spawn git {args:?}: {e}")));
    if !out.status.success() {
        die_emit(&format!("git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    out.stdout
}

fn run_git_string(repo_root: &Path, args: &[&str]) -> String {
    String::from_utf8(run_git(repo_root, args))
        .unwrap_or_else(|_| die_emit(&format!("git {args:?} produced non-UTF8 output")))
        .trim()
        .to_owned()
}

// ---------------------------------------------------------------------------
// Entry gate: A.1 reuse
// ---------------------------------------------------------------------------

fn admission_gate(repo_root: &Path, expected_repository: &str, remote: &str, head: &str) -> String {
    let out = Command::new("bash")
        .arg("tools/apex-source-admission.sh")
        .args(["--expected-repository", expected_repository])
        .args(["--remote", remote])
        .args(["--audit-commit", head])
        .args(["--target-commit", head])
        .arg("--check-worktree")
        .current_dir(repo_root)
        .output()
        .unwrap_or_else(|e| die_admission(&format!("failed to spawn A.1 admission tool via bash: {e}")));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let terminal = stdout
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("TERMINAL_CODE="))
        .unwrap_or("<no TERMINAL_CODE line>")
        .to_owned();
    if !out.status.success() || terminal != "ADMIT-EXACT" {
        die_admission(&format!(
            "A.1 verdict: {terminal}\n--- A.1 stdout ---\n{stdout}\n--- A.1 stderr ---\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    terminal
}

// ---------------------------------------------------------------------------
// Tree capture (commit-pure)
// ---------------------------------------------------------------------------

struct RawTreeEntry {
    mode: String,
    oid: String,
    path: String,
}

fn list_tree(repo_root: &Path, head: &str) -> Vec<RawTreeEntry> {
    let raw = run_git(repo_root, &["ls-tree", "-r", "-z", "--full-tree", head]);
    let mut entries = Vec::new();
    for record in raw.split(|&b| b == 0).filter(|r| !r.is_empty()) {
        let text = std::str::from_utf8(record)
            .unwrap_or_else(|_| die("T1.2-BLOCK-SCOPE-ESCAPE", 15, "non-UTF8 path in ls-tree output"));
        // "<mode> SP <type> SP <oid> TAB <path>"
        let (meta, path) = text
            .split_once('\t')
            .unwrap_or_else(|| die_emit(&format!("malformed ls-tree record: {text:?}")));
        let mut fields = meta.split(' ');
        let mode = fields.next().unwrap_or_default().to_owned();
        let _type = fields.next().unwrap_or_default();
        let oid = fields.next().unwrap_or_default().to_owned();
        entries.push(RawTreeEntry { mode, oid, path: path.to_owned() });
    }
    entries
}

/// Which tree paths are LFS-filtered, per the COMMIT's own
/// `.gitattributes` (batch `git check-attr` — the classification is a
/// closure input, never hardcoded pattern guesses).
fn lfs_path_set(repo_root: &Path, paths: &[String]) -> BTreeSet<String> {
    let mut child = Command::new("git")
        .args(["check-attr", "--stdin", "-z", "filter"])
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| die_emit(&format!("failed to spawn git check-attr: {e}")));
    let mut stdin = child.stdin.take().expect("piped stdin");
    let feed: Vec<u8> = paths.iter().flat_map(|p| p.as_bytes().iter().copied().chain(std::iter::once(0u8))).collect();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&feed);
        drop(stdin);
    });
    let mut raw = Vec::new();
    child.stdout.take().expect("piped stdout").read_to_end(&mut raw).unwrap_or_else(|e| die_emit(&format!("read check-attr: {e}")));
    writer.join().expect("check-attr writer thread");
    let status = child.wait().unwrap_or_else(|e| die_emit(&format!("wait check-attr: {e}")));
    if !status.success() {
        die_emit("git check-attr failed");
    }
    // -z output: <path> NUL <attr> NUL <value> NUL repeated.
    let fields: Vec<&[u8]> = raw.split(|&b| b == 0).collect();
    let mut set = BTreeSet::new();
    for triple in fields.chunks_exact(3) {
        let path = std::str::from_utf8(triple[0]).unwrap_or_default();
        let value = std::str::from_utf8(triple[2]).unwrap_or_default();
        if value == "lfs" {
            set.insert(path.to_owned());
        }
    }
    set
}

struct BlobFacts {
    /// sha256 of the blob bytes (for LFS paths, of the POINTER text).
    sha256: [u8; 32],
    size_bytes: u64,
    /// Whether the blob STARTS WITH the LFS pointer version line — the
    /// content-based LFS classification (see the premise delta at the
    /// `main` LFS section: attr-classification alone is falsified by the
    /// live tree).
    is_lfs_pointer_prefix: bool,
    /// Raw bytes retained only for paths the caller flagged (pins, LFS
    /// pointers) — 12k blobs would not all fit in patience, let alone RAM.
    bytes: Option<Vec<u8>>,
}

/// Streams every tree blob through one `git cat-file --batch` child.
/// Responses arrive in request order; a writer thread owns stdin so the
/// pipe can never deadlock on a full buffer.
fn batch_blob_facts(
    repo_root: &Path,
    entries: &[RawTreeEntry],
    keep_bytes_for: &BTreeSet<String>,
) -> BTreeMap<String, BlobFacts> {
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| die_emit(&format!("failed to spawn git cat-file --batch: {e}")));
    let mut stdin = child.stdin.take().expect("piped stdin");
    let requests: Vec<u8> = entries.iter().flat_map(|e| format!("{}\n", e.oid).into_bytes()).collect();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&requests);
        drop(stdin);
    });

    let mut stdout = std::io::BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut facts: BTreeMap<String, BlobFacts> = BTreeMap::new();
    let mut header = String::new();
    for entry in entries {
        header.clear();
        std::io::BufRead::read_line(&mut stdout, &mut header).unwrap_or_else(|e| die_emit(&format!("cat-file header: {e}")));
        let parts: Vec<&str> = header.trim_end().split(' ').collect();
        if parts.len() != 3 || parts[1] != "blob" {
            die_emit(&format!("unexpected cat-file header for {} ({}): {header:?}", entry.path, entry.oid));
        }
        let size: u64 = parts[2].parse().unwrap_or_else(|_| die_emit(&format!("bad blob size in {header:?}")));
        let mut bytes = vec![0u8; size as usize];
        stdout.read_exact(&mut bytes).unwrap_or_else(|e| die_emit(&format!("cat-file body for {}: {e}", entry.path)));
        let mut lf = [0u8; 1];
        stdout.read_exact(&mut lf).unwrap_or_else(|e| die_emit(&format!("cat-file trailer: {e}")));
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256: [u8; 32] = hasher.finalize().into();
        let is_lfs_pointer_prefix = bytes.starts_with(LFS_POINTER_VERSION_LINE.as_bytes());
        // Pointer blobs are ~130 bytes; retain them even when the caller
        // didn't flag the path so content-classified LFS files (pointer
        // without attr) can still be parsed + verified.
        let keep = keep_bytes_for.contains(&entry.path) || (is_lfs_pointer_prefix && size < 4096);
        facts.insert(
            entry.path.clone(),
            BlobFacts { sha256, size_bytes: size, is_lfs_pointer_prefix, bytes: keep.then_some(bytes) },
        );
    }
    writer.join().expect("cat-file writer thread");
    let _ = child.wait();
    facts
}

// ---------------------------------------------------------------------------
// LFS pointer parse + working-tree verification (T1.2.04)
// ---------------------------------------------------------------------------

struct LfsPointer {
    oid_sha256: [u8; 32],
    size_bytes: u64,
}

fn parse_lfs_pointer(path: &str, pointer_bytes: &[u8]) -> LfsPointer {
    let text = std::str::from_utf8(pointer_bytes)
        .unwrap_or_else(|_| die("T1.2-BLOCK-LFS-OID-MISMATCH", 13, &format!("{path}: pointer blob is not UTF-8")));
    if !text.starts_with(LFS_POINTER_VERSION_LINE) {
        die("T1.2-BLOCK-LFS-OID-MISMATCH", 13, &format!("{path}: tracked blob is not an LFS pointer"));
    }
    let mut oid = None;
    let mut size = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("oid ") {
            let (_algo, bytes) = DigestBytes32V1::parse_human_v1(rest)
                .unwrap_or_else(|e| die("T1.2-BLOCK-LFS-OID-MISMATCH", 13, &format!("{path}: bad pointer oid: {e}")));
            oid = Some(*bytes.as_array());
        } else if let Some(rest) = line.strip_prefix("size ") {
            size = rest.trim().parse::<u64>().ok();
        }
    }
    match (oid, size) {
        (Some(oid_sha256), Some(size_bytes)) => LfsPointer { oid_sha256, size_bytes },
        _ => die("T1.2-BLOCK-LFS-OID-MISMATCH", 13, &format!("{path}: pointer missing oid/size line")),
    }
}

/// The ONE working-tree read in this tool: prove the checkout's resolved
/// LFS content byte-matches every pointer's declared oid. The old
/// single-JPEG sentinel (`checkIfLfsIsSetup`) survives only as a metaphor
/// for what this replaces.
fn verify_lfs_on_disk(repo_root: &Path, path: &str, pointer: &LfsPointer) {
    let disk_path = repo_root.join(path);
    let bytes = match std::fs::read(&disk_path) {
        Ok(b) => b,
        Err(e) => die("T1.2-BLOCK-LFS-MISSING", 12, &format!("{path}: unreadable in working tree: {e}")),
    };
    if bytes.starts_with(LFS_POINTER_VERSION_LINE.as_bytes()) {
        die("T1.2-BLOCK-LFS-STUB", 11, &format!("{path}: un-smudged pointer stub on disk"));
    }
    if bytes.len() as u64 != pointer.size_bytes {
        die(
            "T1.2-BLOCK-LFS-OID-MISMATCH",
            13,
            &format!("{path}: on-disk size {} != pointer-declared {}", bytes.len(), pointer.size_bytes),
        );
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got: [u8; 32] = hasher.finalize().into();
    if got != pointer.oid_sha256 {
        die("T1.2-BLOCK-LFS-OID-MISMATCH", 13, &format!("{path}: on-disk sha256 != pointer oid"));
    }
}

// ---------------------------------------------------------------------------
// Filter-spec single-fileset check
// ---------------------------------------------------------------------------

/// Extracts `pathsToIgnore = [ "…" … ];` from the COMMIT's `flake.nix`
/// bytes and requires it to be exactly `FLAKE_PATHS_TO_IGNORE` (content
/// AND order). Reported under `T1.2-BLOCK-SCOPE-ESCAPE`: a drifted filter
/// means the exported `rust_source_scope` no longer describes the set the
/// derivation consumes — the scope has escaped its declared source.
fn verify_filter_spec_matches_flake(flake_bytes: &[u8]) {
    let text = std::str::from_utf8(flake_bytes)
        .unwrap_or_else(|_| die("T1.2-BLOCK-SCOPE-ESCAPE", 15, "flake.nix is not UTF-8"));
    let start = text
        .find("pathsToIgnore = [")
        .unwrap_or_else(|| die("T1.2-BLOCK-SCOPE-ESCAPE", 15, "flake.nix has no pathsToIgnore list"));
    let rest = &text[start..];
    let end = rest.find("];").unwrap_or_else(|| die("T1.2-BLOCK-SCOPE-ESCAPE", 15, "unterminated pathsToIgnore list"));
    let block = &rest[..end];
    let from_flake: Vec<&str> = block.split('"').skip(1).step_by(2).collect();
    if from_flake != FLAKE_PATHS_TO_IGNORE {
        die(
            "T1.2-BLOCK-SCOPE-ESCAPE",
            15,
            &format!(
                "single-fileset rule violated: flake pathsToIgnore {from_flake:?} != tool constant {:?}",
                FLAKE_PATHS_TO_IGNORE
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Pins
// ---------------------------------------------------------------------------

fn scan_declared_env_inputs(script_bytes: &[u8]) -> Vec<MachineTextV1> {
    let text = String::from_utf8_lossy(script_bytes);
    let mut names = BTreeSet::new();
    for (marker_start, _) in text.match_indices("rerun-if-env-changed=") {
        let after = &text[marker_start + "rerun-if-env-changed=".len()..];
        let name: String =
            after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            names.insert(name);
        }
    }
    names
        .into_iter()
        .map(|n| MachineTextV1::new(n).unwrap_or_else(|_| die_emit("non-ASCII env name in build script")))
        .collect()
}

fn pin_artifact(facts: &BTreeMap<String, BlobFacts>, path: &str) -> ArtifactIdentityV1 {
    let f = facts
        .get(path)
        .unwrap_or_else(|| die_emit(&format!("required pin file missing from commit tree: {path}")));
    let bytes = f.bytes.as_ref().unwrap_or_else(|| die_emit(&format!("pin bytes not retained for {path}")));
    hash_artifact_bytes_v1(bytes)
}

// ---------------------------------------------------------------------------
// Toolchain drift (T1.2's BLOCK-TOOLCHAIN-DRIFT)
// ---------------------------------------------------------------------------

fn check_toolchain_drift(declared_channel: &str) -> String {
    let out = Command::new("rustup").args(["show", "active-toolchain"]).output();
    match out {
        Ok(o) if o.status.success() => {
            let active = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            if !active.starts_with(declared_channel) {
                die(
                    "T1.2-BLOCK-TOOLCHAIN-DRIFT",
                    14,
                    &format!("rust-toolchain declares {declared_channel:?} but active toolchain is {active:?}"),
                );
            }
            format!("verified: active {active:?} matches declared {declared_channel:?}")
        },
        // The nix lane has no rustup — the flake itself pins the compiler
        // there, and the resolved `rustc -Vv` still lands in the evidence
        // sidecar. Only rustup-managed hosts can drift THIS way.
        _ => "skipped: rustup not available (nix-lane pinning applies)".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// Emission (T1.2.07 — atomic, partial records never visible)
// ---------------------------------------------------------------------------

fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) {
    let tmp = dir.join(format!("{name}.tmp"));
    let target = dir.join(name);
    let mut f = std::fs::File::create(&tmp).unwrap_or_else(|e| die_emit(&format!("create {tmp:?}: {e}")));
    f.write_all(bytes).unwrap_or_else(|e| die_emit(&format!("write {tmp:?}: {e}")));
    f.sync_all().unwrap_or_else(|e| die_emit(&format!("fsync {tmp:?}: {e}")));
    drop(f);
    std::fs::rename(&tmp, &target).unwrap_or_else(|e| die_emit(&format!("rename to {target:?}: {e}")));
}

fn hex32(bytes: &[u8; 32]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

fn json_mirror(record: &SourceClosureRecordV1, canonical_sha256: &str) -> serde_json::Value {
    let art = |a: &ArtifactIdentityV1| {
        serde_json::json!({ "sha256": hex32(a.digest.bytes.as_array()), "size_bytes": a.size_bytes })
    };
    serde_json::json!({
        "schema": "bastion.source-closure/v1",
        "authoritative": false,
        "canonical_cbor_sha256": canonical_sha256,
        "commit": record.commit.as_str(),
        "tree": record.tree.as_str(),
        "rust_source_root": hex32(record.rust_source_root.bytes.as_array()),
        "asset_tree_root": hex32(record.asset_tree_root.bytes.as_array()),
        "filter_spec_digest": hex32(record.filter_spec_digest.bytes.as_array()),
        "toolchain_file": art(&record.toolchain_file),
        "cargo_lock": art(&record.cargo_lock),
        "cargo_config": art(&record.cargo_config),
        "flake_nix": art(&record.flake_nix),
        "flake_lock": art(&record.flake_lock),
        "build_scripts": record.build_scripts.iter().map(|b| serde_json::json!({
            "path": b.path.as_str(),
            "artifact": art(&b.artifact),
            "declared_env_inputs": b.declared_env_inputs.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "lfs_report_root": hex32(record.lfs_report_root.bytes.as_array()),
        "file_counts": {
            "rust_files": record.file_counts.rust_files,
            "asset_files": record.file_counts.asset_files,
            "lfs_files": record.file_counts.lfs_files,
        },
        "gitattributes": art(&record.gitattributes),
        "workspace_manifests": record.workspace_manifests.iter().map(|p| serde_json::json!({
            "path": p.path.as_str(),
            "artifact": art(&p.artifact),
        })).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------

fn main() {
    let mut repo_root = std::env::current_dir().expect("cwd");
    let mut out_dir: Option<PathBuf> = None;
    let mut expected_repository = "bastion".to_owned();
    let mut remote = "bastion-origin".to_owned();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut val = |flag: &str| args.next().unwrap_or_else(|| die_emit(&format!("{flag} needs a value")));
        match arg.as_str() {
            "--repo-root" => repo_root = PathBuf::from(val("--repo-root")),
            "--out-dir" => out_dir = Some(PathBuf::from(val("--out-dir"))),
            "--expected-repository" => expected_repository = val("--expected-repository"),
            "--remote" => remote = val("--remote"),
            other => die_emit(&format!("unknown argument {other:?}")),
        }
    }
    let out_dir = out_dir.unwrap_or_else(|| repo_root.join("target/apex-source-closure"));
    let started = std::time::SystemTime::now();

    // 1. Identify the commit, then admit it (A.1 reuse — the ONLY dirty /
    //    wrong-head / unmerged detector).
    let head = run_git_string(&repo_root, &["rev-parse", "HEAD"]);
    let tree_id = run_git_string(&repo_root, &["rev-parse", "HEAD^{tree}"]);
    let admission_terminal = admission_gate(&repo_root, &expected_repository, &remote, &head);
    println!("admission: {admission_terminal} ({head})");

    // 2. Commit tree inventory + LFS classification.
    let raw_entries = list_tree(&repo_root, &head);
    let all_paths: Vec<String> = raw_entries.iter().map(|e| e.path.clone()).collect();
    let lfs_paths = lfs_path_set(&repo_root, &all_paths);
    println!("tree: {} entries, {} LFS-classified", raw_entries.len(), lfs_paths.len());

    // 3. Which blobs need retained bytes: pins + every LFS pointer.
    let is_build_script = |p: &str| p == "build.rs" || p.ends_with("/build.rs");
    let is_cargo_toml = |p: &str| p == "Cargo.toml" || p.ends_with("/Cargo.toml");
    let fixed_pins =
        ["rust-toolchain", "Cargo.lock", ".cargo/config.toml", "flake.nix", "flake.lock", ".gitattributes"];
    let mut keep_bytes: BTreeSet<String> = lfs_paths.clone();
    for p in &all_paths {
        if fixed_pins.contains(&p.as_str()) || is_build_script(p) || is_cargo_toml(p) {
            keep_bytes.insert(p.clone());
        }
    }
    let facts = batch_blob_facts(&repo_root, &raw_entries, &keep_bytes);

    // 4. Single-fileset rule: the tool's exclusion constant must be the
    //    flake's own list, at THIS commit.
    let flake_bytes = facts
        .get("flake.nix")
        .and_then(|f| f.bytes.as_deref())
        .unwrap_or_else(|| die_emit("flake.nix missing from commit tree"));
    verify_filter_spec_matches_flake(flake_bytes);

    // 5. LFS verification pass (T1.2.04): parse every pointer, prove the
    //    working tree holds the resolved object.
    //
    //    PREMISE DELTA (live-tree falsification, first live run): the spec
    //    assumed attr-classified ⇒ pointer blob. The real tree has many
    //    attr-LFS paths (e.g. `assets/common/voxel/**.vox`,
    //    `assets/common/canary.canary`) whose tracked blob is a REGULAR
    //    blob — committed un-migrated, upstream-inherited. Those files'
    //    content is fully pinned by the commit tree itself, so integrity
    //    is intact; the closure therefore classifies LFS BY BLOB CONTENT
    //    (blob parses as a pointer ⇒ LFS, verified on disk) and reports
    //    both mismatch directions in the evidence sidecar rather than
    //    blocking on upstream's partial migration.
    let pointer_paths: Vec<&String> =
        all_paths.iter().filter(|p| facts[p.as_str()].is_lfs_pointer_prefix).collect();
    let mut lfs_report_entries = Vec::with_capacity(pointer_paths.len());
    let mut lfs_resolved: BTreeMap<&str, LfsPointer> = BTreeMap::new();
    for path in &pointer_paths {
        let f = &facts[path.as_str()];
        let pointer_bytes = f.bytes.as_deref().expect("LFS pointer bytes retained");
        let pointer = parse_lfs_pointer(path, pointer_bytes);
        verify_lfs_on_disk(&repo_root, path, &pointer);
        lfs_report_entries.push(LfsReportEntryV1 {
            path: CanonicalPathV1::new(path.as_str()).unwrap_or_else(|e| {
                die("T1.2-BLOCK-SCOPE-ESCAPE", 15, &format!("{path}: {e}"))
            }),
            oid_sha256: pointer.oid_sha256,
            size_bytes: pointer.size_bytes,
        });
        lfs_resolved.insert(path.as_str(), pointer);
    }
    let attr_lfs_not_pointer: Vec<&String> =
        lfs_paths.iter().filter(|p| !facts[p.as_str()].is_lfs_pointer_prefix).collect();
    let pointer_not_attr: Vec<&&String> = pointer_paths.iter().filter(|p| !lfs_paths.contains(p.as_str())).collect();
    println!(
        "lfs: {} pointers parsed + disk-verified; {} attr-classified-but-unmigrated (regular blobs, tree-pinned); {} pointer-without-attr",
        lfs_report_entries.len(),
        attr_lfs_not_pointer.len(),
        pointer_not_attr.len()
    );

    // 6. Closure scopes. Tree entries carry RESOLVED content identity for
    //    LFS files (verified pointer oid/size) and blob identity otherwise.
    let filter_spec = FilterSpecV1::new(
        FLAKE_PATHS_TO_IGNORE
            .iter()
            .map(|p| MachineTextV1::new(*p).expect("filter prefixes are ASCII"))
            .collect(),
    );
    let mut asset_entries = Vec::new();
    let mut rust_entries = Vec::new();
    for e in &raw_entries {
        let f = &facts[&e.path];
        let (sha256, size_bytes) = match lfs_resolved.get(e.path.as_str()) {
            Some(p) => (p.oid_sha256, p.size_bytes),
            None => (f.sha256, f.size_bytes),
        };
        let entry = ClosureTreeEntryV1 {
            path: CanonicalPathV1::new(e.path.as_str()).unwrap_or_else(|err| {
                die("T1.2-BLOCK-SCOPE-ESCAPE", 15, &format!("{}: {err}", e.path))
            }),
            git_mode: MachineTextV1::new(e.mode.as_str()).unwrap_or_else(|_| {
                die("T1.2-BLOCK-TREE-HAZARD", 19, &format!("{}: non-ASCII git mode", e.path))
            }),
            size_bytes,
            sha256,
        };
        if e.path.starts_with("assets/") {
            asset_entries.push(entry);
        } else if !filter_spec.excludes(&e.path) {
            rust_entries.push(entry);
        }
    }
    let asset_tree = ClosureTreeV1::try_new(asset_entries).unwrap_or_else(|e| die_closure_error(&e, "asset scope"));
    let rust_tree = ClosureTreeV1::try_new(rust_entries).unwrap_or_else(|e| die_closure_error(&e, "rust-source scope"));
    let lfs_report = LfsReportV1::try_new(lfs_report_entries).unwrap_or_else(|e| die_closure_error(&e, "lfs report"));

    // 7. Pins.
    let mut build_scripts = Vec::new();
    let mut workspace_manifests = Vec::new();
    for p in &all_paths {
        if is_build_script(p) {
            let bytes = facts[p].bytes.as_deref().expect("build script bytes retained");
            build_scripts.push(BuildScriptPinV1 {
                path: CanonicalPathV1::new(p.as_str()).expect("already canonical"),
                artifact: hash_artifact_bytes_v1(bytes),
                declared_env_inputs: scan_declared_env_inputs(bytes),
            });
        } else if is_cargo_toml(p) {
            workspace_manifests.push(PinnedFileV1 {
                path: CanonicalPathV1::new(p.as_str()).expect("already canonical"),
                artifact: hash_artifact_bytes_v1(facts[p].bytes.as_deref().expect("manifest bytes retained")),
            });
        }
    }
    build_scripts.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));
    workspace_manifests.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));

    // 8. Toolchain drift.
    let toolchain_bytes = facts
        .get("rust-toolchain")
        .and_then(|f| f.bytes.as_deref())
        .unwrap_or_else(|| die_emit("rust-toolchain missing from commit tree"));
    let declared_channel = String::from_utf8_lossy(toolchain_bytes).trim().to_owned();
    let toolchain_note = check_toolchain_drift(&declared_channel);
    println!("toolchain: {toolchain_note}");

    // 9. Assemble + self-check + emit.
    let record = SourceClosureRecordV1 {
        commit: GitHexIdV1::new(head.as_str()).unwrap_or_else(|e| die_emit(&format!("HEAD: {e}"))),
        tree: GitHexIdV1::new(tree_id.as_str()).unwrap_or_else(|e| die_emit(&format!("tree: {e}"))),
        rust_source_root: rust_tree.root().unwrap_or_else(|e| die_closure_error(&e, "rust root")),
        asset_tree_root: asset_tree.root().unwrap_or_else(|e| die_closure_error(&e, "asset root")),
        filter_spec_digest: filter_spec.digest().unwrap_or_else(|e| die_closure_error(&e, "filter digest")),
        toolchain_file: pin_artifact(&facts, "rust-toolchain"),
        cargo_lock: pin_artifact(&facts, "Cargo.lock"),
        cargo_config: pin_artifact(&facts, ".cargo/config.toml"),
        flake_nix: pin_artifact(&facts, "flake.nix"),
        flake_lock: pin_artifact(&facts, "flake.lock"),
        build_scripts,
        lfs_report_root: lfs_report.root().unwrap_or_else(|e| die_closure_error(&e, "lfs root")),
        file_counts: SourceClosureCountsV1 {
            rust_files: rust_tree.len() as u64,
            asset_files: asset_tree.len() as u64,
            lfs_files: lfs_report.len() as u64,
        },
        gitattributes: pin_artifact(&facts, ".gitattributes"),
        workspace_manifests,
    };

    let limits = source_closure_limits_v1();
    let cbor = encode_manifest_v1(&record, &limits).unwrap_or_else(|e| die_emit(&format!("encode: {e}")));
    let decoded: SourceClosureRecordV1 =
        decode_manifest_v1(&cbor, &limits).unwrap_or_else(|e| die_emit(&format!("self-check decode: {e}")));
    if decoded != record {
        die_emit("self-check: decoded record != assembled record");
    }
    let re_encoded = encode_manifest_v1(&decoded, &limits).unwrap_or_else(|e| die_emit(&format!("re-encode: {e}")));
    if re_encoded != cbor {
        die_emit("self-check: record is not a fixed point of decode->encode");
    }

    let mut hasher = Sha256::new();
    hasher.update(&cbor);
    let cbor_sha256: [u8; 32] = hasher.finalize().into();
    let cbor_hex = hex32(&cbor_sha256);

    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| die_emit(&format!("create {out_dir:?}: {e}")));
    let base = format!("apex-source-closure-{head}");
    write_atomic(&out_dir, &format!("{base}.cbor"), &cbor);
    write_atomic(&out_dir, &format!("{base}.cbor.sha256"), format!("{cbor_hex}  {base}.cbor\n").as_bytes());
    let mirror = serde_json::to_vec_pretty(&json_mirror(&record, &cbor_hex)).expect("mirror serializes");
    write_atomic(&out_dir, &format!("{base}.json"), &mirror);

    let rustc_vv = Command::new("rustc")
        .arg("-Vv")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_else(|e| format!("<rustc unavailable: {e}>"));
    let evidence = serde_json::json!({
        "schema": "bastion.source-closure-evidence/v1",
        "authoritative": false,
        "tool_version": TOOL_VERSION,
        "admission_terminal": admission_terminal,
        "toolchain_check": toolchain_note,
        "rustc_vv": rustc_vv,
        "host_os": std::env::consts::OS,
        "host_arch": std::env::consts::ARCH,
        "captured_unix_time": started.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        // Premise-delta visibility: attr/blob LFS classification mismatches
        // (content identity of these files is pinned by the commit tree —
        // this list is audit surface, not authority).
        "lfs_attr_classified_count": lfs_paths.len(),
        "lfs_attr_classified_but_unmigrated": attr_lfs_not_pointer.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
        "lfs_pointer_without_attr": pointer_not_attr.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
    });
    write_atomic(
        &out_dir,
        &format!("{base}.evidence.json"),
        &serde_json::to_vec_pretty(&evidence).expect("evidence serializes"),
    );

    println!("record: {}", out_dir.join(format!("{base}.cbor")).display());
    println!("canonical_cbor_sha256={cbor_hex}");
    println!("rust_source_root={}", hex32(record.rust_source_root.bytes.as_array()));
    println!("asset_tree_root={}", hex32(record.asset_tree_root.bytes.as_array()));
    println!("lfs_report_root={}", hex32(record.lfs_report_root.bytes.as_array()));
    println!(
        "counts: rust={} asset={} lfs={}",
        record.file_counts.rust_files, record.file_counts.asset_files, record.file_counts.lfs_files
    );
    println!("TERMINAL: T1.2-CLOSURE-READY");
}
