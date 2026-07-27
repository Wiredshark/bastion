#!/usr/bin/env bash
# APEX-T1.2.09 — typed canary suite for the source-closure capture tool +
# the T1.2.08 runtime binding gate (spec readme/apex/
# APEX-T1.2-DECLARED-SOURCE-ASSET-CLOSURE-FLEET-v1.md, Section 6 terminals
# + Section 9 acceptance gate).
#
# Every case builds a REAL fixture git repo and drives the REAL bins — no
# mocked verdicts. LFS disk-state divergence (stub/missing/mismatch) is
# fixtured with `git update-index --assume-unchanged`, which makes git
# status report CLEAN over a lying working tree — exactly the false-green
# shape the closure exists to catch, so A.1 admission passes and the
# closure's own check must be the one that bites.
#
# Usage: tools/apex-t1-2-closure-canaries.sh   (from the repo root; expects
# target/debug/apex_source_closure + target/debug/bastion-harness built)
set -u
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOL="$REPO_ROOT/target/debug/apex_source_closure"
HARNESS="$REPO_ROOT/target/debug/bastion-harness"
[ -x "$TOOL" ] || [ -x "$TOOL.exe" ] || { echo "build apex_source_closure first"; exit 2; }
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0; FAIL=0
report() { # report <name> <ok:0/1> <detail>
  if [ "$2" = 0 ]; then PASS=$((PASS+1)); echo "PASS: $1"; else FAIL=$((FAIL+1)); echo "FAIL: $1 — $3"; fi
}

# expect_terminal <name> <expected-exit> <expected-terminal> <workdir> [extra args...]
run_tool() { # <workdir> [args...] -> stdout file $WORK/out.txt, returns exit
  local wd="$1"; shift
  "$TOOL" --repo-root "$wd" --out-dir "$wd/closure-out" \
    --remote origin --expected-repository fixture-closure "$@" \
    > "$WORK/out.txt" 2> "$WORK/err.txt"
}
expect_terminal() {
  local name="$1" want_exit="$2" want_term="$3" wd="$4"
  run_tool "$wd"; local got=$?
  local term; term=$(grep -o 'TERMINAL: .*' "$WORK/out.txt" | tail -1)
  if [ "$got" = "$want_exit" ] && [ "$term" = "TERMINAL: $want_term" ]; then
    report "$name" 0 ""
  else
    report "$name" 1 "exit=$got (want $want_exit), $term (want $want_term)"
  fi
}

# ── fixture builder ──────────────────────────────────────────────────────────
# Content whose sha256 backs the green LFS pointer:
LFS_CONTENT="canary-lfs-resolved-content"
LFS_OID=$(printf '%s' "$LFS_CONTENT" | sha256sum | cut -d' ' -f1)
LFS_SIZE=$(printf '%s' "$LFS_CONTENT" | wc -c)

# git ops with LFS filters neutralized: blobs are committed EXACTLY as
# written (so a pointer file's blob IS the pointer text).
gitf() { git -c filter.lfs.clean=cat -c filter.lfs.smudge=cat -c filter.lfs.process= -c filter.lfs.required=false "$@"; }

make_fixture() { # <dir>
  local d="$1"
  mkdir -p "$d" && cd "$d" || exit 2
  git init -q -b main .
  git config user.email fixture@bastion.test
  git config user.name fixture
  git config core.autocrlf false
  git config commit.gpgsign false
  printf 'nightly-2026-06-13\n' > rust-toolchain
  printf '# fixture lock\n' > Cargo.lock
  mkdir -p .cargo && printf '# fixture config\n' > .cargo/config.toml
  # flake.nix carries the REAL pathsToIgnore list (single-fileset rule).
  cat > flake.nix <<'FLAKE'
{ # fixture flake — only the filter list matters to the closure
      pathsToIgnore = [
        "flake.nix"
        "flake.lock"
        "nix"
        "assets"
        "README.md"
        "CONTRIBUTING.md"
        "CHANGELOG.md"
        "CODE_OF_CONDUCT.md"
        ".github"
        ".gitlab"
      ];
}
FLAKE
  printf '{ "fixture": true }\n' > flake.lock
  printf '*.bin filter=lfs diff=lfs merge=lfs -text\n' > .gitattributes
  printf '[package]\nname = "fixture"\n' > Cargo.toml
  printf 'fn main() { println!("cargo:rerun-if-env-changed=FIXTURE_ENV_A"); }\n' > build.rs
  mkdir -p src assets
  printf 'fn main() {}\n' > src/main.rs
  printf '(fixture_asset: 1)\n' > assets/data.ron
  # Green LFS state: pointer committed as the blob, resolved content on
  # disk, index told to trust the (deliberately divergent) worktree.
  printf 'version https://git-lfs.github.com/spec/v1\noid sha256:%s\nsize %s\n' "$LFS_OID" "$LFS_SIZE" > assets/big.bin
  mkdir -p tools
  cp "$REPO_ROOT/tools/apex-source-admission.sh" tools/
  cp "$REPO_ROOT/tools/apex_source_admission_helper.py" tools/
  gitf add -A
  gitf commit -qm fixture
  printf '%s' "$LFS_CONTENT" > assets/big.bin
  git update-index --assume-unchanged assets/big.bin
  # Local bare "origin" whose URL contains the expected repository name.
  git init -q --bare ../fixture-closure.git
  git remote add origin ../fixture-closure.git
  git push -q origin main
  cd - >/dev/null || exit 2
}

commit_all() { ( cd "$1" && gitf add -A && gitf commit -qm "$2" ); }

FX="$WORK/fx"
make_fixture "$FX"

# ── 1-3: green + determinism + cross-checkout equality ──────────────────────
expect_terminal "green fixture reaches CLOSURE-READY" 0 "T1.2-CLOSURE-READY" "$FX"
CBOR1=$(ls "$FX"/closure-out/*.cbor)
SHA1=$(sha256sum "$CBOR1" | cut -d' ' -f1)
run_tool "$FX"
SHA2=$(sha256sum "$CBOR1" | cut -d' ' -f1)
[ "$SHA1" = "$SHA2" ] && report "re-run is byte-identical" 0 "" || report "re-run is byte-identical" 1 "$SHA1 vs $SHA2"

( cd "$FX" && git worktree add --detach ../fx-other-path -q HEAD )
# The second worktree needs the same divergent-but-trusted LFS state.
( cd "$WORK/fx-other-path" && printf '%s' "$LFS_CONTENT" > assets/big.bin && git update-index --assume-unchanged assets/big.bin )
run_tool "$WORK/fx-other-path"
CBOR_B=$(ls "$WORK/fx-other-path"/closure-out/*.cbor 2>/dev/null)
if [ -n "$CBOR_B" ] && cmp -s "$CBOR1" "$CBOR_B"; then
  report "cross-checkout records byte-identical (T1.2.06)" 0 ""
else
  report "cross-checkout records byte-identical (T1.2.06)" 1 "records differ or missing"
fi
( cd "$FX" && git worktree remove --force ../fx-other-path )

ROOT_RUST1=$(grep -o 'rust_source_root=[0-9a-f]*' "$WORK/out.txt")
ROOT_ASSET1=$(grep -o 'asset_tree_root=[0-9a-f]*' "$WORK/out.txt")

# ── 4-7: sensitivity — each mutated input flips the record ──────────────────
printf 'fn main() { let _x = 1; }\n' > "$FX/src/main.rs"; commit_all "$FX" "rust flip"
run_tool "$FX"
R=$(grep -o 'rust_source_root=[0-9a-f]*' "$WORK/out.txt"); A=$(grep -o 'asset_tree_root=[0-9a-f]*' "$WORK/out.txt")
{ [ "$R" != "$ROOT_RUST1" ] && [ "$A" = "$ROOT_ASSET1" ]; } \
  && report "rust byte flip moves ONLY rust_source_root" 0 "" \
  || report "rust byte flip moves ONLY rust_source_root" 1 "rust $ROOT_RUST1->$R asset $ROOT_ASSET1->$A"

printf '(fixture_asset: 2)\n' > "$FX/assets/data.ron"; commit_all "$FX" "asset flip"
run_tool "$FX"
R2=$(grep -o 'rust_source_root=[0-9a-f]*' "$WORK/out.txt"); A2=$(grep -o 'asset_tree_root=[0-9a-f]*' "$WORK/out.txt")
{ [ "$A2" != "$A" ] && [ "$R2" = "$R" ]; } \
  && report "asset byte flip moves ONLY asset_tree_root" 0 "" \
  || report "asset byte flip moves ONLY asset_tree_root" 1 "asset $A->$A2 rust $R->$R2"

SHA_PRE=$(sha256sum "$FX"/closure-out/*.cbor | cut -d' ' -f1)
printf '# fixture lock v2\n' > "$FX/Cargo.lock"; commit_all "$FX" "lock edit"
run_tool "$FX"
SHA_POST=$(sha256sum "$FX"/closure-out/*.cbor | cut -d' ' -f1)
[ "$SHA_PRE" != "$SHA_POST" ] && report "Cargo.lock edit flips the record" 0 "" || report "Cargo.lock edit flips the record" 1 "record unchanged"

printf 'fn main() { println!("cargo:rerun-if-env-changed=FIXTURE_ENV_B"); }\n' > "$FX/build.rs"; commit_all "$FX" "build.rs edit"
run_tool "$FX"
if grep -q 'FIXTURE_ENV_B' "$FX"/closure-out/*.json && ! grep -q 'FIXTURE_ENV_A' "$FX"/closure-out/*.json; then
  report "build.rs env declaration lands in the record" 0 ""
else
  report "build.rs env declaration lands in the record" 1 "declared_env_inputs not updated"
fi

# ── 8-9: admission (A.1 reuse) ──────────────────────────────────────────────
printf 'dirty\n' >> "$FX/src/main.rs"
expect_terminal "dirty tracked file blocks (A.1)" 10 "T1.2-BLOCK-ADMISSION" "$FX"
( cd "$FX" && git checkout -q -- src/main.rs )
printf 'stray\n' > "$FX/untracked.txt"
expect_terminal "untracked file blocks (A.1)" 10 "T1.2-BLOCK-ADMISSION" "$FX"
rm "$FX/untracked.txt"

# ── 10-12: LFS disk-state divergence (the false-green class) ────────────────
POINTER_TEXT=$(git -C "$FX" cat-file blob HEAD:assets/big.bin)
printf '%s\n' "$POINTER_TEXT" > "$FX/assets/big.bin"    # un-smudged stub
expect_terminal "stub-on-disk blocks" 11 "T1.2-BLOCK-LFS-STUB" "$FX"
rm "$FX/assets/big.bin"
expect_terminal "missing resolved object blocks" 12 "T1.2-BLOCK-LFS-MISSING" "$FX"
printf 'entirely-wrong-content-x' > "$FX/assets/big.bin"   # same byte count as LFS_CONTENT? not required
expect_terminal "on-disk oid mismatch blocks" 13 "T1.2-BLOCK-LFS-OID-MISMATCH" "$FX"
printf '%s' "$LFS_CONTENT" > "$FX/assets/big.bin"          # restore green

# ── 13: malformed pointer blob ──────────────────────────────────────────────
FXM="$WORK/fxm"; make_fixture "$FXM"
printf 'version https://git-lfs.github.com/spec/v1\noid sha256:NOT-HEX\nsize 5\n' > "$FXM/assets/big.bin"
( cd "$FXM" && git update-index --no-assume-unchanged assets/big.bin )
commit_all "$FXM" "malformed pointer"
expect_terminal "malformed pointer blocks" 13 "T1.2-BLOCK-LFS-OID-MISMATCH" "$FXM"

# ── 14-16: tree hazards (plumbing-committed; worktree stays A.1-clean) ──────
hazard_fixture() { # <dir> <mode> <path> -> commits an entry of that mode
  local d="$1" mode="$2" path="$3"
  ( cd "$d" \
    && blob=$(printf 'hazard' | git hash-object -w --stdin) \
    && git update-index --add --cacheinfo "$mode,$blob,$path" \
    && git update-index --assume-unchanged "$path" 2>/dev/null; \
       gitf commit -qm "hazard $mode" )
}
FXH="$WORK/fxh"; make_fixture "$FXH"
hazard_fixture "$FXH" 120000 "src/evil-symlink"
expect_terminal "symlink entry blocks" 19 "T1.2-BLOCK-TREE-HAZARD" "$FXH"

FXG="$WORK/fxg"; make_fixture "$FXG"
( cd "$FXG" \
  && git update-index --add --cacheinfo "160000,$(git rev-parse HEAD),vendored-sub" \
  && gitf commit -qm gitlink )
expect_terminal "gitlink entry blocks" 19 "T1.2-BLOCK-TREE-HAZARD" "$FXG"

FXC="$WORK/fxc"; make_fixture "$FXC"
( cd "$FXC" \
  && blob=$(printf 'a' | git hash-object -w --stdin) \
  && git update-index --add --cacheinfo "100644,$blob,src/Case.rs" --cacheinfo "100644,$blob,src/case.rs" \
  && git update-index --assume-unchanged src/Case.rs 2>/dev/null; true )
( cd "$FXC" && git update-index --assume-unchanged src/case.rs 2>/dev/null; gitf commit -qm collision )
expect_terminal "case-fold collision blocks" 19 "T1.2-BLOCK-TREE-HAZARD" "$FXC"

# ── 17-19: filter drift, missing pin, toolchain drift ───────────────────────
FXF="$WORK/fxf"; make_fixture "$FXF"
sed -i 's/"CHANGELOG.md"//' "$FXF/flake.nix"; commit_all "$FXF" "filter drift"
expect_terminal "flake pathsToIgnore drift blocks (single-fileset rule)" 15 "T1.2-BLOCK-SCOPE-ESCAPE" "$FXF"

FXP="$WORK/fxp"; make_fixture "$FXP"
( cd "$FXP" && gitf rm -q Cargo.lock && gitf commit -qm "drop lock" )
expect_terminal "missing pin file blocks emission" 16 "T1.2-BLOCK-EMIT" "$FXP"

FXT="$WORK/fxt"; make_fixture "$FXT"
printf 'nightly-1999-01-01\n' > "$FXT/rust-toolchain"; commit_all "$FXT" "toolchain drift"
expect_terminal "unresolvable declared toolchain blocks" 14 "T1.2-BLOCK-TOOLCHAIN-DRIFT" "$FXT"

# ── 20: premise-delta — attr-classified-but-unmigrated is REPORTED, not blocked
FXU="$WORK/fxu"; make_fixture "$FXU"
printf '*.dat filter=lfs diff=lfs merge=lfs -text\n' >> "$FXU/.gitattributes"
printf 'real content, never migrated' > "$FXU/assets/plain.dat"
commit_all "$FXU" "unmigrated attr file"
run_tool "$FXU"; got=$?
if [ "$got" = 0 ] && grep -q 'assets/plain.dat' "$FXU"/closure-out/*.evidence.json; then
  report "unmigrated attr-LFS file: READY + evidence-listed (premise delta)" 0 ""
else
  report "unmigrated attr-LFS file: READY + evidence-listed (premise delta)" 1 "exit=$got or not listed"
fi

# ── 21-23: emission integrity ───────────────────────────────────────────────
if ! ls "$FXH"/closure-out/*.cbor >/dev/null 2>&1 && ! ls "$FXH"/closure-out/*.tmp >/dev/null 2>&1; then
  report "blocked run leaves NO record and NO .tmp (atomicity)" 0 ""
else
  report "blocked run leaves NO record and NO .tmp (atomicity)" 1 "partial output exists"
fi
( cd "$FX/closure-out" && sha256sum -c ./*.cbor.sha256 >/dev/null 2>&1 ) \
  && report "sha256 sidecar verifies the canonical bytes" 0 "" \
  || report "sha256 sidecar verifies the canonical bytes" 1 "sidecar mismatch"
MIRROR_ASSET=$(grep -o '"asset_tree_root": "[0-9a-f]*"' "$FX"/closure-out/*.json | grep -o '[0-9a-f]\{64\}')
run_tool "$FX"
STDOUT_ASSET=$(grep -o 'asset_tree_root=[0-9a-f]*' "$WORK/out.txt" | cut -d= -f2)
[ "$MIRROR_ASSET" = "$STDOUT_ASSET" ] && report "JSON mirror agrees with canonical roots" 0 "" \
  || report "JSON mirror agrees with canonical roots" 1 "$MIRROR_ASSET vs $STDOUT_ASSET"

# ── 24-26: T1.2.08 runtime binding gate (real harness bin, real repo) ───────
if [ -x "$HARNESS" ] || [ -x "$HARNESS.exe" ]; then
  ( cd "$REPO_ROOT" && VELOREN_ASSETS_OVERRIDE=/nonexistent "$HARNESS" --help >"$WORK/h1.txt" 2>/dev/null ); hx=$?
  { [ "$hx" = 41 ] && grep -q 'T1.2-BLOCK-ASSET-OVERRIDE' "$WORK/h1.txt"; } \
    && report "runtime gate: override env bites (41)" 0 "" \
    || report "runtime gate: override env bites (41)" 1 "exit=$hx"
  ( cd "$REPO_ROOT" && BASTION_VERIFY_ASSET_ROOT=$(printf '0%.0s' $(seq 64)) "$HARNESS" --help >"$WORK/h2.txt" 2>/dev/null ); hx=$?
  { [ "$hx" = 42 ] && grep -q 'T1.2-BLOCK-ASSET-ROOT-MISMATCH' "$WORK/h2.txt"; } \
    && report "runtime gate: declared-root mismatch bites (42)" 0 "" \
    || report "runtime gate: declared-root mismatch bites (42)" 1 "exit=$hx"
  # Green: expected root = the LIVE record's asset_tree_root (recompute
  # from a real emitted record keeps this canary honest end-to-end).
  LIVE_OUT="$WORK/live-closure"
  "$TOOL" --repo-root "$REPO_ROOT" --out-dir "$LIVE_OUT" > "$WORK/live.txt" 2>&1
  if [ $? = 0 ]; then
    LIVE_ROOT=$(grep -o 'asset_tree_root=[0-9a-f]*' "$WORK/live.txt" | cut -d= -f2)
    ( cd "$REPO_ROOT" && BASTION_VERIFY_ASSET_ROOT="$LIVE_ROOT" "$HARNESS" --help >/dev/null 2>"$WORK/h3.txt" ); hx=$?
    { [ "$hx" = 0 ] && grep -q 'asset root verified pre-sim' "$WORK/h3.txt"; } \
      && report "runtime gate: live record's root verifies pre-sim" 0 "" \
      || report "runtime gate: live record's root verifies pre-sim" 1 "exit=$hx"
  else
    report "runtime gate: live record's root verifies pre-sim" 1 "live capture failed (dirty tree during dev is expected — rerun on a clean tip)"
  fi
else
  echo "SKIP: runtime-gate canaries (bastion-harness bin not built)"
fi

echo
echo "canaries: $PASS passed, $FAIL failed"
if [ "$FAIL" = 0 ]; then
  echo "TERMINAL: T1.2-CANARIES-GREEN"
  exit 0
else
  echo "TERMINAL: T1.2-CANARY-RED"
  exit 1
fi
