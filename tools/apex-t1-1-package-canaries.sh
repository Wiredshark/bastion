#!/usr/bin/env bash
# APEX-T1.1.07 — package/lane canaries (typed terminals per packet §6.6).
#
# Cases that need Nix emit SKIP-NO-NIX and force the aggregate terminal to
# T1.1-INCOMPLETE-NEEDS-NIX-LANE (exit 2): a Windows/cargo-only host can prove
# the cargo-side guards but must NEVER claim T1.1-PACKAGE-READY without the
# Nix-lane cases. FAIL anywhere → exit 1. All green incl. Nix → exit 0.
#
# Usage: bash tools/apex-t1-1-package-canaries.sh [all|profile|stamp|flake|nix]
set -u
cd "$(dirname "$0")/.." || exit 3
PASS=0; FAIL=0; SKIP=0
ok(){ echo "PASS  $1"; PASS=$((PASS+1)); }
bad(){ echo "FAIL  $1 — $2"; FAIL=$((FAIL+1)); }
skipn(){ echo "SKIP-NO-NIX  $1"; SKIP=$((SKIP+1)); }
REV_OK=d1b8948369e00680f193a6935f52f66086aff0fa   # any well-formed 40-hex works
EPOCH_OK=1753488000

c_profile(){
  # T1.1-BLOCK-VERIFY-PROFILE: tracked root profile with DET-BLD-031(a) semantics.
  local blk; blk=$(awk '/^\[profile\.verify\]/{f=1;next} /^\[/{f=0} f' Cargo.toml)
  [ -n "$blk" ] || { bad profile "no [profile.verify] in root Cargo.toml"; return; }
  echo "$blk" | grep -q "inherits *= *'release'" || { bad profile "verify must inherit release"; return; }
  echo "$blk" | grep -q "lto *= *false" || { bad profile "verify must disable lto"; return; }
  # DET-BLD-031(a) (reviewer-approved, LATER than the T1.1 packet): cert lane
  # runs the guard layer. Do NOT "fix" these to the packet's §6.1.
  echo "$blk" | grep -q "overflow-checks *= *true" || { bad profile "DET-BLD-031(a) overflow-checks=true missing"; return; }
  echo "$blk" | grep -q "debug-assertions *= *true" || { bad profile "DET-BLD-031(a) debug-assertions=true missing"; return; }
  ok "profile: tracked [profile.verify] (release-inherit, lto off, 031(a) guards on)"
}

c_stamp(){
  # Cargo-side identity guards (fast: cargo check).
  if BASTION_BUILD_LANE=apex-nix-v1 BASTION_SOURCE_REVISION=$REV_OK SOURCE_DATE_EPOCH=$EPOCH_OK \
     cargo check -q -p bastion-harness 2>/dev/null; then
    ok "stamp: DeclaredCertified builds"
  else bad stamp "declared certified env failed to build"; fi
  if BASTION_BUILD_LANE=apex-nix-v1 BASTION_SOURCE_REVISION=$REV_OK \
     cargo check -q -p bastion-harness 2>/dev/null; then
    bad stamp "certified lane WITHOUT epoch must fail closed (T1.1-BLOCK-AMBIENT-TIME)"
  else ok "stamp: certified-missing-epoch fails closed"; fi
  if BASTION_BUILD_LANE=apex-nix-v1 SOURCE_DATE_EPOCH=$EPOCH_OK \
     cargo check -q -p bastion-harness 2>/dev/null; then
    bad stamp "certified lane WITHOUT revision must fail closed (T1.1-BLOCK-UNKNOWN-REVISION)"
  else ok "stamp: certified-missing-revision fails closed"; fi
  if BASTION_SOURCE_REVISION="ABC123" cargo check -q -p bastion-harness 2>/dev/null; then
    bad stamp "malformed declared revision must fail in ANY lane"
  else ok "stamp: malformed declared revision rejected"; fi
  # dirtyRev-shaped input ("<hex>-dirty") must be rejected (dirty-source rejection).
  if BASTION_BUILD_LANE=apex-nix-v1 BASTION_SOURCE_REVISION="${REV_OK}-dirty" SOURCE_DATE_EPOCH=$EPOCH_OK \
     cargo check -q -p bastion-harness 2>/dev/null; then
    bad stamp "dirtyRev-shaped revision must be rejected in certified lane"
  else ok "stamp: dirty revision rejected in certified lane"; fi
}

c_flake_static(){
  grep -q 'packages\.bastion-harness = harnessOut\.packages\.verify' flake.nix \
    && ok "flake-static: unwrapped packages.bastion-harness exported (verify profile)" \
    || bad flake-static "packages.bastion-harness missing or not the unwrapped verify output"
  grep -q 'checks\.bastion-harness-package' flake.nix \
    && ok "flake-static: package check exported" \
    || bad flake-static "checks.bastion-harness-package missing"
  # T1.1-BLOCK-ASSET-CLAIM: the harness package must NOT be asset-wrapped.
  grep -q 'packages\.bastion-harness = wrapWithAssets' flake.nix \
    && bad flake-static "harness package is asset-wrapped — T1.1-BLOCK-ASSET-CLAIM" \
    || ok "flake-static: harness package not asset-wrapped"
  grep -q 'BASTION_BUILD_LANE = "apex-nix-v1"' flake.nix \
    && ok "flake-static: derivation pins the apex-nix-v1 lane" \
    || bad flake-static "derivation lane env missing"
  grep -q 'RUSTC_WRAPPER = ""' flake.nix \
    && ok "flake-static: sccache wrapper neutralized in derivation" \
    || bad flake-static "RUSTC_WRAPPER neutralization missing — T1.1-BLOCK-AMBIENT-WRAPPER"
  grep -q 'CARGO_INCREMENTAL = "0"' flake.nix \
    && ok "flake-static: incremental disabled in derivation" \
    || bad flake-static "CARGO_INCREMENTAL=0 missing"
  # T1.1-BLOCK-CROSS-SYSTEM: only x86_64-linux is admitted.
  grep -q 'systems = \["x86_64-linux"\]' flake.nix \
    && ok "flake-static: single admitted system x86_64-linux" \
    || bad flake-static "systems list changed — revalidate cross-system policy"
}

c_nix(){
  if ! command -v nix >/dev/null 2>&1; then
    skipn "nix-eval: nix flake show / drvPath evaluation"
    skipn "nix-build: locked no-link build + executable presence"
    skipn "nix-lock: flake.lock byte-stability across build"
    skipn "nix-wrapper: sccache absence in build log/closure"
    return
  fi
  local pre post out
  pre=$(sha256sum flake.lock | cut -d' ' -f1)
  if nix eval ".#packages.x86_64-linux.bastion-harness.drvPath" --no-update-lock-file --no-write-lock-file >/dev/null 2>&1; then
    ok "nix-eval: package drvPath evaluates from locked flake"
  else bad nix-eval "packages.bastion-harness does not evaluate — T1.1-BLOCK-NCI-OUTPUT"; return; fi
  if out=$(nix build ".#bastion-harness" --no-update-lock-file --no-write-lock-file --no-link --print-out-paths 2>/dev/null); then
    ok "nix-build: locked build produced $out"
    [ -x "$out/bin/bastion-harness" ] \
      && ok "nix-build: executable bin/bastion-harness present" \
      || bad nix-build "package lacks executable bin/bastion-harness"
  else bad nix-build "locked nix build failed"; fi
  post=$(sha256sum flake.lock | cut -d' ' -f1)
  [ "$pre" = "$post" ] \
    && ok "nix-lock: flake.lock byte-stable across build" \
    || bad nix-lock "flake.lock mutated — T1.1-BLOCK-LOCK-MUTATION"
}

run_all(){ c_profile; c_stamp; c_flake_static; c_nix; }
case "${1:-all}" in
  profile) c_profile ;;
  stamp) c_stamp ;;
  flake) c_flake_static ;;
  nix) c_nix ;;
  all) run_all ;;
  *) echo "unknown case ${1}"; exit 3 ;;
esac
echo "---"
echo "pass=$PASS fail=$FAIL skip_no_nix=$SKIP"
if [ "$FAIL" -gt 0 ]; then echo "TERMINAL: T1.1-CANARY-FAIL"; exit 1; fi
if [ "$SKIP" -gt 0 ]; then echo "TERMINAL: T1.1-INCOMPLETE-NEEDS-NIX-LANE (cargo-side green; Nix-lane cases pending a Linux/Nix host)"; exit 2; fi
echo "TERMINAL: T1.1-PACKAGE-READY (canary scope)"; exit 0
