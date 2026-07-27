# Project Bastion — Apex Micro-Step Packet (FLEET-AUTHORED)
## APEX-T1.2 — Declared Source and Asset Closure

**Canonical row:** `APEX-T1.2`
**Specification provenance:** `FLEET_AUTHORED` — the original ChatGPT packet is
confirmed hallucination-class (never delivered; see APEX-VECTOR-PIN-PROVENANCE-v1.md
for the precedent class). Authored by Builder Opus 5 per Ben's 2026-07-26 order;
grounded ONLY in: (a) the master build order's own T1.2 row objective (real,
quoted below), (b) the status matrix's verified finding targets, (c) live code
seams verified on `bastion/apex` this session. Cross-reviewed by Builder Sonnet 5
and approved by Fable (spec-owner of record) before build start.
**Master-order objective (verbatim ground):** "Define and verify the complete
source closure: exact Git commit, dirty-state rejection, Cargo.lock, toolchain,
Git LFS pointer/object integrity, canonical asset-tree digest → build scripts
and CI → declared build inputs."
**Prerequisites:** APEX-A.1 (admission tool — reused, not reimplemented),
APEX-T0.2 (BastionManifestEncodingV1), APEX-T0.3 (domain-separated digests),
APEX-T1.1 (verify-profile package + env-first stamping + lane scripts).
**Blocks:** APEX-T1.3, APEX-T1.4, APEX-T1.5.
**Mapped findings:** DET-BLD-032 (primary; PARTIAL — closes the "source closure"
half), supporting DET-BLD-019/023/029 residuals (ambient inputs the closure must
declare), A.1 §12.8 residual (ignored/untracked build inputs), T1.1 unknowns
#3/#5 (installer/base-image manifest ownership; workspace build-script inventory).
**Canary catalog:** `tools/apex-t1-2-closure-canaries.sh`, ~22 typed cases
(fleet-authored alongside the row; every negative must bite a specific terminal).

---
## 1. Row purpose

Freeze ONE machine-readable record — `SourceClosureRecordV1` — that declares
every source-side input a certified build consumes, such that:
1. two checkouts of the same commit on different machines/paths produce a
   **byte-identical** closure record (T1.3's entry requirement);
2. any undeclared or drifted input (dirty tree, stale lock, wrong toolchain,
   un-smudged LFS pointer, mutated asset byte, changed build script, changed
   cargo config) flips the record or blocks with a typed terminal — never
   silently;
3. the record's roots are embeddable in T1.5's `BuildManifestV1` without
   re-derivation.

This row does NOT prove rebuild equality (T1.3/T1.4), does not bundle runnable
assets into the Nix package, and does not sign anything.

## 2. Determinism contract (per the law)

The closure record is a **pure function of (commit tree, LFS object content,
toolchain/config file bytes)**. Specifically:
- All paths in the record are repo-relative with `/` separators, byte-ordered;
  no absolute paths, no OS path separators, no checkout-root leakage —
  **checkout-path independence is a hard requirement** (T1.3 diffs records from
  two randomized roots).
- No wall-clock, hostname, username, or environment value enters the record.
- All digests are SHA-256, domain-separated via T0.3 under ONE new permanent
  domain: `DigestDomainIdV1::SourceClosure = 9`, label
  `"bastion/source-closure/v1"` (registry addition, PluginManifest=8 precedent;
  flagged in the implementing commit).
- The record's canonical bytes are produced by `BastionManifestEncodingV1`
  (T0.2): integer field IDs, canonical map order, no floats, ASCII machine
  text; a JSON mirror is emitted for humans and is explicitly
  NON-AUTHORITATIVE.
- Failure modes are typed terminals (Section 6); a partial record is never
  emitted (write-temp + fsync + atomic rename, A.1.10 pattern).

## 3. LIVE-VERIFIED seams (bastion/apex, this session)

1. **Build-script inventory is exactly three:** `bastion-harness/build.rs`
   (env-first stamping, T1.1.02), `common/build.rs`, `voxygen/build.rs`. Each
   is a certified-build input; their bytes must be pinned and their declared
   env inputs enumerated.
2. **Toolchain:** `rust-toolchain` = `nightly-2026-06-13` (file bytes pinned +
   the RESOLVED `rustc -Vv` identity recorded separately — file-says vs
   resolver-gave are different facts).
3. **Ambient cargo config is a build input:** `.cargo/config.toml` requests
   `rustc-wrapper = "sccache"` + mold rustflags; `Cargo.lock` pins the crate
   graph. Both files' digests belong in the closure (the T1.1 derivation
   NEUTRALIZES the wrapper, but the neutralization target's identity must
   still be declared).
4. **LFS surface:** `.gitattributes` declares 8+ LFS pattern classes
   (png/jpg/jpeg/vox/ttf/wav/ogg/ico…); `assets/` = ~10,610 files, ~437 MB —
   a full content-hash pass is feasible (<60 s) and is the gate.
   **Current LFS verification is a single-JPEG sentinel** in `flake.nix`
   (`checkIfLfsIsSetup` on `bg_main.jpg`) — the exact false-green T1.2 kills.
5. **The Nix source filter EXCLUDES `assets/`** (`flake.nix` filteredSource
   ignores `assets`, `nix`, `flake.*`, docs): therefore the closure must
   record TWO scopes explicitly — `rust_source_scope` (the filtered set the
   derivation consumes) and `full_tree_scope` (everything at the commit) — so
   T1.3/T1.4 compare the right sets and asset drift cannot hide behind the
   filter.
6. **A.1's admission tool already owns checkout admission**
   (`tools/apex-source-admission.sh --check-worktree`: WrongHead/
   DirtyTracked/DirtyUntracked/Unmerged verdicts, NUL-safe). T1.2 REUSES it as
   the entry gate; reimplementing dirty-detection would create drift.

## 4. Bastion policy decisions (fleet-authored, cross-review targets)

1. **Fail-closed scope rule:** every file in `full_tree_scope` at the admitted
   commit is a closure input. There is NO default ignore-glob. The only
   exclusions are the git-ignored build outputs (`target/`,
   `*.partial`) — and those are excluded because they are not IN the commit
   tree, not by pattern courtesy. Untracked files under the worktree at
   capture time = `BLOCK-DIRTY-UNTRACKED` (via A.1).
2. **LFS integrity = pointer↔object proof, per file:** for every path whose
   `.gitattributes` class is LFS, the on-disk bytes must be the RESOLVED
   object (not a pointer stub) AND `sha256(bytes)` must equal the pointer's
   declared `oid`. Missing object, stub-on-disk, or oid mismatch each get a
   distinct terminal. The one-JPEG sentinel is retained only as a fast
   pre-check, never as the verdict.
3. **Asset-tree root:** canonical manifest of `(relative_path_bytes,
   size_bytes, sha256)` triples in path-byte order over `assets/**`,
   T0.2-encoded, digested under `SourceClosure`. LFS files contribute their
   RESOLVED content hash (= pointer oid, verified), so the root is identical
   on any machine with correct LFS state.
4. **Rust-source root:** same construction over the `rust_source_scope`
   (the commit tree MINUS the flake's declared exclusion list, which is
   itself recorded + digested so a filter change flips the record).
5. **Toolchain/lock/config/build-script pins:** file-byte sha256 of
   `rust-toolchain`, `Cargo.lock`, `.cargo/config.toml`, each `build.rs`,
   `flake.nix`, `flake.lock` + the resolved `rustc -Vv` string (recorded as
   RESOLVED evidence, excluded from the pure-function root — resolver output
   is machine-fact, not source-fact; it lives in a separate evidence field so
   the ROOT stays commit-pure).
6. **One new digest domain** (`SourceClosure = 9`) — not reusing
   `BuildManifest = 5`: the closure is an input to T1.5's manifest, and
   domain-separating inputs from the manifest that embeds them is the whole
   point of domain separation.
7. **Certified-lane runtime asset BINDING** (amendment; grounded in the REAL
   T1.3 packet §2's revalidation list, which records that T1.2's intent
   includes runtime-asset binding, + live seams verified:
   `common/assets/src/lib.rs:361-392` ASSETS_PATH multi-location search with
   the existing DET-AST-007 `BASTION_REQUIRE_EXPLICIT_ASSETS` strictness gate;
   `common/assets/src/fs.rs:10-27` `VELOREN_ASSETS_OVERRIDE` per-file
   substitution source). A closure that digests `assets/` but lets the runtime
   load from an arbitrary path or override channel does not bind the binary to
   the closed set. Policy: in the certified lane (a) `VELOREN_ASSETS` MUST be
   declared and its content identity MUST match `asset_tree_root` — enforced
   by REUSING the DET-AST-007 gate (`BASTION_REQUIRE_EXPLICIT_ASSETS=1`), not
   a second mechanism; (b) `VELOREN_ASSETS_OVERRIDE` set in the certified lane
   is a typed block (`T1.2-BLOCK-ASSET-OVERRIDE`) — the override channel is a
   development affordance, never a certified input.

## 5. Exact data contract

```rust
// NEW-SPEC (fleet). Field IDs frozen; encodes via ManifestEncodeV1.
struct SourceClosureRecordV1 {          // field id
    schema: MachineTextV1,              // 0  "bastion.source-closure/v1"
    commit: MachineTextV1,              // 1  40-lower-hex (admitted commit)
    tree: MachineTextV1,                // 2  full tree id
    rust_source_root: ProtocolDigestV1, // 3  SourceClosure domain
    asset_tree_root: ProtocolDigestV1,  // 4  SourceClosure domain
    filter_spec_digest: ProtocolDigestV1,// 5 the exclusion list itself
    toolchain_file: ArtifactIdentityV1, // 6  rust-toolchain bytes
    cargo_lock: ArtifactIdentityV1,     // 7
    cargo_config: ArtifactIdentityV1,   // 8  .cargo/config.toml
    flake_nix: ArtifactIdentityV1,      // 9
    flake_lock: ArtifactIdentityV1,     // 10
    build_scripts: Vec<BuildScriptPinV1>,// 11 path-byte order
    lfs_report_root: ProtocolDigestV1,  // 12 per-file verdicts manifest
    file_counts: SourceClosureCountsV1, // 13 (rust_files, asset_files, lfs_files) u64s
}
struct BuildScriptPinV1 {
    path: CanonicalPathV1,              // 0
    artifact: ArtifactIdentityV1,       // 1
    declared_env_inputs: Vec<MachineTextV1>, // 2 sorted; from rerun-if-env-changed
}
```
Resolved-evidence sidecar (JSON, non-authoritative, NOT in any root):
`rustc -Vv` output, capture host os/arch, capture tool version, wall time.

## 6. Typed terminals

| Terminal | Meaning |
|---|---|
| `T1.2-CLOSURE-READY` | record emitted; all gates green |
| `T1.2-BLOCK-ADMISSION` | A.1 checkout verdict not ExactAndClean (carries A.1's code) |
| `T1.2-BLOCK-LFS-STUB` | pointer stub on disk (un-smudged) |
| `T1.2-BLOCK-LFS-MISSING` | LFS object absent |
| `T1.2-BLOCK-LFS-OID-MISMATCH` | on-disk bytes ≠ pointer oid |
| `T1.2-BLOCK-TOOLCHAIN-DRIFT` | resolved toolchain ≠ rust-toolchain file channel |
| `T1.2-BLOCK-SCOPE-ESCAPE` | absolute path / non-repo path in any manifest |
| `T1.2-BLOCK-EMIT` | partial/failed atomic emission |
| `T1.2-BLOCK-ASSET-OVERRIDE` | VELOREN_ASSETS_OVERRIDE set in the certified lane |

## 7. Minute steps

- **T1.2.01** Schema doc + this packet reviewed; field IDs frozen. Gate: cross-review sign-off.
- **T1.2.02** Register `DigestDomainIdV1::SourceClosure = 9` (+label, +ALL list, +domain doc). Gate: apex digest tests extended, green.
- **T1.2.03** `tools/apex-source-closure.sh` (or a small Rust bin `apex-source-closure` in bastion-harness/tools if NUL-safe hashing in shell gets unwieldy — implementer's call, mirroring A.1's helper precedent): entry gate = A.1 `--check-worktree` reuse; then capture per Sections 4–5. Gate: runs on the live repo → `T1.2-CLOSURE-READY`.
- **T1.2.04** LFS verification pass (pointer parse → oid compare, per file; distinct terminals). Gate: canaries for stub/missing/mismatch each bite.
- **T1.2.05** Asset-tree + rust-source roots (canonical walk, T0.2 encode, T0.3 digest). Gate: byte-flip canary flips the root; path-order canary (same set, shuffled walk) does NOT.
- **T1.2.06** Checkout-path independence proof: capture from TWO worktrees of the same commit at different absolute paths → byte-identical canonical records. Gate: this IS T1.3's precondition; must pass locally before T1.3 runs.
- **T1.2.07** Atomic emission (CBOR authoritative + sha256 + JSON mirror) + `vm-apex-nix-build.sh` integration (closure capture precedes `nix build`; record travels with package evidence). Gate: kill-before-rename leaves no record.
- **T1.2.08** Certified-lane asset-binding gate: reuse the DET-AST-007 explicit-assets mechanism; declared-root content identity vs asset_tree_root; override-env typed block. Gate: override-set canary bites; declared-root-mismatch canary bites.
- **T1.2.09** Canary suite `tools/apex-t1-2-closure-canaries.sh` (~24 cases; every Section-6 terminal + mutation-hardening: weaken any guard → its canary REDs).

## 8. Non-goals
Rebuild equality (T1.3/T1.4); runnable asset bundling; signing/attestation
(T1.5+); offline acquisition freezing (T1.4's store discipline); Nix installer
/ base-image manifests (recorded already in the T1.1.08 bake evidence; formal
ownership lands in T1.5's BuildManifestV1).

## 9. Row acceptance gate
1. Same commit, two machines/paths → byte-identical canonical record.
2. Every Section-6 terminal has a biting canary; mutation-hardening proven.
3. One flipped asset byte, one un-smudged pointer, one lock edit, one
   build-script edit each flip/block the record.
4. The record's roots verify via `digest_canonical_bytes_v1` recomputation.
5. A.1 reused for admission (no second dirty-detector in the tree).
6. `T1.2-CLOSURE-READY` emitted on the live repo at the current apex tip.
