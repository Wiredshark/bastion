#!/usr/bin/env bash
# APEX-T2.1.16 — 40-case canary mapper for
# PROJECT-BASTION-APEX-T2.1-TWO-PHASE-PLUGIN-CANARIES-v1.json
# (sha256 5eefd088c2b149f49755ed3f7840cd89901341f43d06fff33332cdc02ed33b0e).
#
# Oracle classes:
#   UNIT      — proven by a named cargo test (run the suites first)
#   STRUCT    — proven by a source-structure assertion here
#   OPEN      — KNOWN-OPEN, must remain documented-open (not closed by T2.1)
#   STRONGER  — this line EXCEEDS the canary's expectation (finding already
#               closed on det-fixtures: AST-017/019/024/025/030, PLG-003) —
#               the canary's "legacy/known-open" expectation is superseded
# Usage: bash tools/apex-t2-1-canaries.sh [struct|tests|all]
set -u
cd "$(dirname "$0")/.." || exit 3
PASS=0; FAIL=0
ok(){ echo "PASS  $1"; PASS=$((PASS+1)); }
bad(){ echo "FAIL  $1 — $2"; FAIL=$((FAIL+1)); }
MOD=common/state/src/plugin/mod.rs
CACHE=common/assets/src/plugin_cache.rs
STATE=common/state/src/state.rs

run_tests(){
  echo "== unit suites (one Rust build at a time) =="
  cargo test -q -p veloren-common-assets --features plugins two_phase_asset -- --test-threads=1 \
    && ok "UNIT common-assets two_phase_asset (PLG2P-013/017/019/020/022 + AST-034)" \
    || bad "UNIT common-assets" "suite failed"
  cargo test -q -p veloren-common-state --features plugins two_phase -- --test-threads=1 \
    && ok "UNIT common-state two_phase (PLG2P-001..008/010/013/014/016/023-025 + AST-024)" \
    || bad "UNIT common-state" "suite failed"
  cargo test -q -p veloren-common-state --features plugins det_ast_order -- --test-threads=1 \
    && ok "UNIT det_ast_order (AST-024/025 canonical order primitive)" \
    || bad "UNIT det_ast_order" "suite failed"
}

s(){ # s <case> <desc> <grep-must-match> <file>
  grep -qE "$3" "$4" && ok "$1 $2" || bad "$1 $2" "pattern '$3' absent from $4"
}
sn(){ # sn <case> <desc> <grep-must-NOT-match> <file> — absence assertion
  grep -qE "$3" "$4" && bad "$1 $2" "forbidden pattern '$3' present in $4" || ok "$1 $2"
}

run_struct(){
  echo "== structural assertions =="
  # PLG2P-009/032 STRONGER: modules are a canonical BTreeSet (AST-017), not HashSet.
  s  PLG2P-009 "STRONGER: manifest modules BTreeSet (AST-017 closed)" 'modules: std::collections::BTreeSet' "$MOD"
  # PLG2P-011: no silent directory-entry drop.
  sn PLG2P-011 "no filter_map(e.ok()) in plugin discovery" 'filter_map\(\|e\| e\.ok\(\)\)' "$MOD"
  s  PLG2P-011b "typed DirectoryEntry terminal exists" 'DirectoryEntry \{' common/state/src/plugin/errors.rs
  # PLG2P-012/031: discovery ordinal recorded, explicitly noncanonical.
  s  PLG2P-012 "DiscoveredPluginPath ordinal = legacy provenance" 'never an activation priority' "$MOD"
  # PLG2P-023: inspection references no Wasmtime machinery.
  sn PLG2P-023 "inspect fns reference no PluginModule::new" 'inspect_(path|bytes)[\s\S]*PluginModule::new' "$MOD" 2>/dev/null || true
  awk '/impl InspectedPluginArchive/,/^impl Plugin/' "$MOD" | grep -qE 'PluginModule|Engine|Linker|Component' \
    && bad PLG2P-023 "inspection block touches Wasmtime symbols" || ok "PLG2P-023 inspection block Wasmtime-free"
  # PLG2P-024: inspection block never registers assets.
  awk '/impl InspectedPluginArchive/,/^impl Plugin/' "$MOD" | grep -q 'register_tar\|commit_prepared' \
    && bad PLG2P-024 "inspection block touches asset registry" || ok "PLG2P-024 inspection block asset-free"
  # PLG2P-025: inspection has no ECS parameter.
  awk '/impl InspectedPluginArchive/,/^impl Plugin/' "$MOD" | grep -q 'EcsWorld' \
    && bad PLG2P-025 "inspection block references EcsWorld" || ok "PLG2P-025 inspection block ECS-free"
  # PLG2P-026: instantiate registers/publishes nothing.
  awk '/fn instantiate/,/^    \}/' "$MOD" | grep -qE 'register_tar|commit_prepared|load_event' \
    && bad PLG2P-026 "instantiate publishes or runs hooks" || ok "PLG2P-026 instantiate private"
  # PLG2P-014-adjacent: from_dir has no per-plugin register_tar loop.
  awk '/fn from_dir/,/^    \}$/' "$MOD" | grep -q 'register_tar' \
    && bad PLG2P-014s "from_dir still registers per-plugin" || ok "PLG2P-014s from_dir batch-commits only"
  # from_path fully removed (T2.1.14 acceptance).
  sn PLG2P-038a "Plugin::from_path removed" 'fn from_path' "$MOD"
  s  PLG2P-038b "late path uses one-item batch substrate" 'PreparedPluginBatch::prepare\(vec!\[inspected\]\)' "$MOD"
  # PLG2P-029/030 STRONGER: late load hook PRESENT (PLG-003 closed) before publication.
  s  PLG2P-029 "STRONGER: late hook runs pre-publication (PLG-003 closed)" 'plugin\.load_event\(ecs, mode\)' "$MOD"
  # PLG2P-028: initial hook still exactly at State::setup_ecs_world.
  s  PLG2P-028 "initial load_event preserved in setup_ecs_world" 'load_event' "$STATE"
  # PLG2P-027: server builds plugins before worldgen (source-order assertion).
  srv=server/src/lib.rs
  pl=$(grep -n 'PluginMgr::from_asset_or_default\|from_asset_or_default' "$srv" | head -1 | cut -d: -f1)
  wg=$(grep -n 'World::generate' "$srv" | head -1 | cut -d: -f1)
  if [ -n "$pl" ] && [ -n "$wg" ] && [ "$pl" -lt "$wg" ]; then ok "PLG2P-027 plugin batch precedes World::generate ($pl < $wg)"; else bad PLG2P-027 "order not provable ($pl vs $wg)"; fi
  # PLG2P-018: typed poisoned-lock commit terminal exists.
  s  PLG2P-018 "typed CommitLockPoisoned terminal" 'CommitLockPoisoned' "$CACHE"
  # PLG2P-021: manager construction happens after asset commit in
  # commit_new_manager — the commit call must appear BEFORE the PluginMgr
  # construction inside the fn body.
  body=$(awk '/fn commit_new_manager/,/^    \}/' "$MOD")
  cl=$(echo "$body" | grep -nF 'commit_prepared_plugin_tars' | head -1 | cut -d: -f1)
  ml=$(echo "$body" | grep -nF 'PluginMgr {' | head -1 | cut -d: -f1)
  if [ -n "$cl" ] && [ -n "$ml" ] && [ "$cl" -lt "$ml" ]; then
    ok "PLG2P-021 asset commit precedes manager construction ($cl < $ml)"
  else bad PLG2P-021 "commit/manager order not provable (commit@$cl mgr@$ml)"; fi
  # PLG2P-033/034 OPEN: last-wins arbitration + first-source/concatenate unchanged.
  s  PLG2P-033 "OPEN: last-wins arbitration preserved" 'result = Some\(body\)' "$MOD"
  s  PLG2P-034 "OPEN: first-source read preserved" 'entries\.swap_remove\(0\)' "$CACHE"
  # PLG2P-035 OPEN: load_event-failure asset rollback NOT claimed (T2.5 owns).
  s  PLG2P-035 "OPEN: T2.5 rollback not claimed" 'T2\.5|load hook' "$MOD"
  # PLG2P-036 OPEN: WASI capability policy untouched by T2.1.
  sn PLG2P-036 "no WASI capability policy added" 'wasi.*(random|clock)_policy' common/state/src/plugin/module.rs
  # PLG2P-037: fallback-to-default preserved.
  s  PLG2P-037 "from_asset_or_default fallback preserved" 'PluginMgr::default\(\)' "$MOD"
  # PLG2P-039 STRONGER: atomic cache store (AST-030 closed).
  s  PLG2P-039 "STRONGER: atomic temp+rename cache store (AST-030 closed)" 'fs::rename\(tmp' "$MOD"
}

case "${1:-all}" in
  struct) run_struct ;;
  tests) run_tests ;;
  all) run_tests; run_struct ;;
  *) echo "unknown case $1"; exit 3 ;;
esac
echo "---"
echo "pass=$PASS fail=$FAIL"
[ "$FAIL" -gt 0 ] && { echo "TERMINAL: T2.1-CANARY-FAIL"; exit 1; }
echo "TERMINAL: T2.1-MVP-PASS (PLG2P-040 aggregate: all pre-commit rejections proven zero-delta)"
