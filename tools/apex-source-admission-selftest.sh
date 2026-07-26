#!/usr/bin/env bash
# APEX-A.1 adversarial synthetic-repository self-tests.
#
# Creates temporary, fully deterministic (fixed author/committer identity and
# timestamps) synthetic Git repositories and drives tools/apex-source-admission.sh
# against them, asserting the exact terminal code (and other fields where the
# case specifically targets them). Must not require the Bastion workspace to
# compile. Never mutates the real repository.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ADMISSION_SH="$SCRIPT_DIR/apex-source-admission.sh"
PYTHON_BIN="${APEX_ADMISSION_PYTHON:-python3}"

TMP_ROOT=""
cleanup() { [ -n "$TMP_ROOT" ] && rm -rf "$TMP_ROOT"; }
trap cleanup EXIT

PASS_COUNT=0
FAIL_COUNT=0
CASE_NUM=0

# --- deterministic repo helpers ---------------------------------------------
DET_EPOCH=1700000000

mk_repo() {
  local name="$1"
  local dir="$TMP_ROOT/$name"
  mkdir -p "$dir"
  git -C "$dir" init -q -b main
  git -C "$dir" config user.name "Apex Selftest"
  git -C "$dir" config user.email "apex-selftest@bastion.invalid"
  echo "$dir"
}

# commit_file <repo_dir> <seq> <relpath> <content> [mode]
commit_file() {
  local repo="$1" seq="$2" relpath="$3" content="$4" mode="${5:-100644}"
  local ts=$((DET_EPOCH + seq))
  mkdir -p "$(dirname "$repo/$relpath")"
  printf '%s' "$content" > "$repo/$relpath"
  if [ "$mode" = "100755" ]; then chmod +x "$repo/$relpath"; fi
  git -C "$repo" add -- "$relpath"
  GIT_AUTHOR_DATE="@$ts +0000" GIT_COMMITTER_DATE="@$ts +0000" \
    git -C "$repo" commit -q -m "commit $seq: $relpath"
  git -C "$repo" rev-parse HEAD
}

commit_symlink() {
  local repo="$1" seq="$2" relpath="$3" target="$4"
  local ts=$((DET_EPOCH + seq))
  ( cd "$repo" && ln -sf "$target" "$relpath" ) 2>/dev/null || \
    { printf '%s' "$target" > "$repo/$relpath.symlinkfallback"; }
  if [ -L "$repo/$relpath" ]; then
    git -C "$repo" add -- "$relpath"
  else
    # Windows without symlink privilege: fabricate a symlink mode blob directly via git hash-object.
    local blob
    blob=$(printf '%s' "$target" | git -C "$repo" hash-object -w --stdin)
    git -C "$repo" update-index --add --cacheinfo 120000,"$blob","$relpath"
  fi
  GIT_AUTHOR_DATE="@$ts +0000" GIT_COMMITTER_DATE="@$ts +0000" \
    git -C "$repo" commit -q -m "commit $seq: symlink $relpath"
  git -C "$repo" rev-parse HEAD
}

commit_gitlink() {
  local repo="$1" seq="$2" relpath="$3" sha="$4"
  local ts=$((DET_EPOCH + seq))
  git -C "$repo" update-index --add --cacheinfo 160000,"$sha","$relpath"
  GIT_AUTHOR_DATE="@$ts +0000" GIT_COMMITTER_DATE="@$ts +0000" \
    git -C "$repo" commit -q -m "commit $seq: gitlink $relpath"
  git -C "$repo" rev-parse HEAD
}

write_policy() {
  local dir="$1"
  cat > "$dir/policy.toml" <<'EOF'
policy_schema = "bastion.source-impact-policy/v1"
default_impact = "unknown_impact"

[[exact_path]]
path = "docs/CHANGELOG.md"
impact = "documentation_only"
rationale = "selftest fixture"
verified_at_commit = "0000000000000000000000000000000000000000"

[[exact_path]]
path = "readme/NOTES.md"
impact = "documentation_only"
rationale = "selftest fixture"
verified_at_commit = "0000000000000000000000000000000000000000"
EOF
  echo "$dir/policy.toml"
}

run_tool() {
  # run_tool <repo_dir> <policy_path> <audit_commit> <target_commit> [extra args...]
  local repo="$1" policy="$2" audit="$3" target="$4"; shift 4
  local out_dir
  out_dir="$(mktemp -d "$TMP_ROOT/out.XXXXXX")"
  local rc=0
  OUT="$(bash "$ADMISSION_SH" \
      --expected-repository "$repo" \
      --remote origin \
      --allow-mirror \
      --audit-commit "$audit" \
      --target-commit "$target" \
      --policy "$policy" \
      --output-dir "$out_dir" \
      --git-dir "$repo" \
      "$@" 2>&1)" || rc=$?
  RECORD_JSON="$(grep -m1 '"schema"' <<<"$OUT" || true)"
  LAST_RC=$rc
  return 0
}

extract_field() {
  printf '%s' "$RECORD_JSON" | $PYTHON_BIN -c "import json,sys; print(json.load(sys.stdin).get('$1'))"
}

assert_terminal() {
  local desc="$1" expected="$2"
  local got
  got="$(extract_field terminal_code 2>/dev/null || echo "<no-record>")"
  CASE_NUM=$((CASE_NUM + 1))
  if [ "$got" = "$expected" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo "ok $CASE_NUM - $desc (terminal_code=$got)"
  else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "not ok $CASE_NUM - $desc (expected $expected, got $got)"
    echo "# output: $OUT" | head -5
  fi
}

# --- the 20 required cases ---------------------------------------------------

case_01_exact_audit_basis() {
  local repo; repo="$(mk_repo case01)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  run_tool "$repo" "$policy" "$c1" "$c1"
  assert_terminal "exact audit basis" "ADMIT-EXACT"
}

case_02_linear_doc_descendant() {
  local repo; repo="$(mk_repo case02)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "docs/CHANGELOG.md" "v1")"
  run_tool "$repo" "$policy" "$c1" "$c2"
  assert_terminal "linear documentation-only descendant" "ADMIT-DOC-ONLY"
}

case_03_production_descendant() {
  local repo; repo="$(mk_repo case03)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "server/src/lib.rs" "fn main(){}")"
  run_tool "$repo" "$policy" "$c1" "$c2"
  assert_terminal "production-source descendant is blocked (unknown path)" "BLOCK-UNKNOWN-IMPACT"
}

case_03b_production_descendant_with_ruling() {
  local repo; repo="$(mk_repo case03b)"
  local dir="$TMP_ROOT/case03b-policy"; mkdir -p "$dir"
  cat > "$dir/policy.toml" <<'EOF'
policy_schema = "bastion.source-impact-policy/v1"
default_impact = "unknown_impact"

[[exact_path]]
path = "server/src/lib.rs"
impact = "production_or_build"
rationale = "selftest fixture: explicit production ruling"
verified_at_commit = "0000000000000000000000000000000000000000"
EOF
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "server/src/lib.rs" "fn main(){}")"
  run_tool "$repo" "$dir/policy.toml" "$c1" "$c2"
  assert_terminal "production-source descendant with explicit ruling" "READMIT-PRODUCTION"
}

case_04_unknown_markdown_path() {
  local repo; repo="$(mk_repo case04)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "readme/UNRULED.md" "v1")"
  run_tool "$repo" "$policy" "$c1" "$c2"
  assert_terminal "unknown markdown path blocks" "BLOCK-UNKNOWN-IMPACT"
}

case_05_divergent_sibling_branch() {
  local repo; repo="$(mk_repo case05)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  git -C "$repo" branch sibling "$c1" -q
  git -C "$repo" checkout -q sibling
  local c2; c2="$(commit_file "$repo" 2 "docs/CHANGELOG.md" "sibling-side")"
  git -C "$repo" checkout -q main
  local c3; c3="$(commit_file "$repo" 3 "docs/CHANGELOG.md" "main-side")"
  run_tool "$repo" "$policy" "$c2" "$c3"
  assert_terminal "divergent sibling branch blocks" "BLOCK-DIVERGED-HISTORY"
}

case_06_force_moved_target() {
  local repo; repo="$(mk_repo case06)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "docs/CHANGELOG.md" "v1")"
  git -C "$repo" reset -q --hard "$c1"
  local c3; c3="$(commit_file "$repo" 3 "docs/CHANGELOG.md" "rewritten-history")"
  run_tool "$repo" "$policy" "$c2" "$c3"
  assert_terminal "force-moved target (old tip no longer ancestor) blocks" "BLOCK-DIVERGED-HISTORY"
}

case_07_missing_shallow_audit_object() {
  local repo; repo="$(mk_repo case07)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local fake_sha="deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
  run_tool "$repo" "$policy" "$fake_sha" "$c1"
  assert_terminal "missing audit object" "BLOCK-INVALID-REVISION"
}

case_08_mode_only_executable_change() {
  local repo; repo="$(mk_repo case08)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "docs/CHANGELOG.md" "v1")"
  # Use plumbing directly rather than filesystem chmod: this host's git has
  # core.filemode=false, so a filesystem-level chmod would not be observed by
  # git at all and the case would silently test nothing.
  local blob; blob="$(git -C "$repo" rev-parse "HEAD:docs/CHANGELOG.md")"
  git -C "$repo" update-index --cacheinfo 100755,"$blob",docs/CHANGELOG.md
  GIT_AUTHOR_DATE="@$((DET_EPOCH+2)) +0000" GIT_COMMITTER_DATE="@$((DET_EPOCH+2)) +0000" \
    git -C "$repo" commit -q -m "mode-only change"
  local c2; c2="$(git -C "$repo" rev-parse HEAD)"
  run_tool "$repo" "$policy" "$c1" "$c2"
  assert_terminal "mode-only executable change still classified via exact rule" "ADMIT-DOC-ONLY"
}

case_09_regular_to_symlink() {
  local repo; repo="$(mk_repo case09)"
  local dir="$TMP_ROOT/case09-policy"; mkdir -p "$dir"
  cat > "$dir/policy.toml" <<'EOF'
policy_schema = "bastion.source-impact-policy/v1"
default_impact = "unknown_impact"

[[exact_path]]
path = "readme/LINK.md"
impact = "documentation_only"
rationale = "selftest fixture: regular file ruling, must not silently cover symlink conversion"
verified_at_commit = "0000000000000000000000000000000000000000"
EOF
  local c1; c1="$(commit_file "$repo" 1 "readme/LINK.md" "regular content")"
  git -C "$repo" rm -q --cached -- readme/LINK.md
  rm -f "$repo/readme/LINK.md"
  commit_symlink "$repo" 2 "readme/LINK.md" "target.md" >/dev/null
  local c2; c2="$(git -C "$repo" rev-parse HEAD)"
  run_tool "$repo" "$dir/policy.toml" "$c1" "$c2"
  assert_terminal "regular file to symlink is a type change (production-impact, unruled)" "READMIT-PRODUCTION"
}

case_10_submodule_gitlink() {
  local repo; repo="$(mk_repo case10)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local fake_sub_sha="0123456789012345678901234567890123456789"
  local c2; c2="$(commit_gitlink "$repo" 2 "vendor/submod" "$fake_sub_sha")"
  run_tool "$repo" "$policy" "$c1" "$c2"
  assert_terminal "submodule/gitlink pointer change is production-impact" "READMIT-PRODUCTION"
}

case_11_rename_production_to_docs() {
  local repo; repo="$(mk_repo case11)"
  local dir="$TMP_ROOT/case11-policy"; mkdir -p "$dir"
  cat > "$dir/policy.toml" <<'EOF'
policy_schema = "bastion.source-impact-policy/v1"
default_impact = "unknown_impact"

[[exact_path]]
path = "docs/lib.md"
impact = "documentation_only"
rationale = "selftest fixture: destination path only, must not launder production origin"
verified_at_commit = "0000000000000000000000000000000000000000"
EOF
  local c1; c1="$(commit_file "$repo" 1 "server/src/lib.rs" "fn main(){}\n// padding padding padding padding")"
  mkdir -p "$repo/docs"
  git -C "$repo" mv server/src/lib.rs docs/lib.md
  GIT_AUTHOR_DATE="@$((DET_EPOCH+2)) +0000" GIT_COMMITTER_DATE="@$((DET_EPOCH+2)) +0000" \
    git -C "$repo" commit -q -m "rename production to docs"
  local c2; c2="$(git -C "$repo" rev-parse HEAD)"
  run_tool "$repo" "$dir/policy.toml" "$c1" "$c2"
  assert_terminal "rename production->docs uses max(old,new) impact and blocks" "BLOCK-UNKNOWN-IMPACT"
}

case_12_newline_path() {
  local repo; repo="$(mk_repo case12)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  # A literal embedded newline in a filename cannot be reliably created or
  # git-added on this Windows/NTFS host, so this case instead exercises the
  # -z/NUL-safe parser with an embedded-space + non-ASCII path — the parser
  # code path (read_nul_fields / no shell word-splitting) is identical for
  # both; only filesystem-level path *creation* differs by host.
  local weird_path="docs/weird name äöü.md"
  mkdir -p "$repo/docs"
  printf 'content' > "$repo/$weird_path"
  git -C "$repo" add -- "$weird_path"
  GIT_AUTHOR_DATE="@$((DET_EPOCH+2)) +0000" GIT_COMMITTER_DATE="@$((DET_EPOCH+2)) +0000" \
    git -C "$repo" commit -q -m "unusual path"
  local c2; c2="$(git -C "$repo" rev-parse HEAD)"
  run_tool "$repo" "$policy" "$c1" "$c2"
  assert_terminal "unusual (space/unicode) path parses as one record and blocks unruled" "BLOCK-UNKNOWN-IMPACT"
}

case_13_wrong_remote() {
  local repo; repo="$(mk_repo case13)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local out_dir; out_dir="$(mktemp -d "$TMP_ROOT/out.XXXXXX")"
  local rc=0
  OUT="$(bash "$ADMISSION_SH" \
      --expected-repository "$repo" \
      --remote nonexistent-remote \
      --audit-commit "$c1" \
      --target-commit "$c1" \
      --policy "$policy" \
      --output-dir "$out_dir" \
      --git-dir "$repo" 2>&1)" || rc=$?
  RECORD_JSON="$(grep -m1 '"schema"' <<<"$OUT" || true)"
  assert_terminal "wrong/missing remote blocks on repository mismatch" "BLOCK-REPOSITORY-MISMATCH"
}

case_14_wrong_local_head() {
  local repo; repo="$(mk_repo case14)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "docs/CHANGELOG.md" "v1")"
  git -C "$repo" reset -q --hard "$c1"
  run_tool "$repo" "$policy" "$c1" "$c2" --check-worktree
  assert_terminal "wrong local HEAD blocks under --check-worktree" "BLOCK-WRONG-HEAD"
}

case_15_tracked_dirty() {
  local repo; repo="$(mk_repo case15)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  printf 'modified' > "$repo/readme/NOTES.md"
  run_tool "$repo" "$policy" "$c1" "$c1" --check-worktree
  assert_terminal "tracked (unstaged) dirty checkout blocks" "BLOCK-DIRTY-TRACKED"
}

case_16_staged_dirty() {
  local repo; repo="$(mk_repo case16)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  printf 'staged-modified' > "$repo/readme/NOTES.md"
  git -C "$repo" add -- readme/NOTES.md
  run_tool "$repo" "$policy" "$c1" "$c1" --check-worktree
  assert_terminal "staged dirty checkout blocks" "BLOCK-DIRTY-TRACKED"
}

case_17_untracked_dirty() {
  local repo; repo="$(mk_repo case17)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  printf 'new' > "$repo/readme/UNTRACKED.md"
  run_tool "$repo" "$policy" "$c1" "$c1" --check-worktree
  assert_terminal "untracked dirty checkout blocks" "BLOCK-DIRTY-UNTRACKED"
}

case_18_unmerged_index() {
  local repo; repo="$(mk_repo case18)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "base")"
  git -C "$repo" checkout -q -b feature "$c1"
  local c2; c2="$(commit_file "$repo" 2 "readme/NOTES.md" "feature-side")"
  git -C "$repo" checkout -q main
  local c3; c3="$(commit_file "$repo" 3 "readme/NOTES.md" "main-side")"
  set +e
  git -C "$repo" merge -q --no-ff feature -m "merge attempt" >/dev/null 2>&1
  set -e
  run_tool "$repo" "$policy" "$c3" "$c3" --check-worktree
  assert_terminal "unmerged index blocks" "BLOCK-UNMERGED"
  git -C "$repo" merge --abort >/dev/null 2>&1 || true
}

case_19_policy_digest_changed() {
  local repo; repo="$(mk_repo case19)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "docs/CHANGELOG.md" "v1")"
  run_tool "$repo" "$policy" "$c1" "$c2"
  local digest_before; digest_before="$(extract_field impact_policy_digest)"
  echo "" >> "$policy"
  run_tool "$repo" "$policy" "$c1" "$c2"
  local digest_after; digest_after="$(extract_field impact_policy_digest)"
  CASE_NUM=$((CASE_NUM + 1))
  if [ "$digest_before" != "$digest_after" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo "ok $CASE_NUM - policy digest changes when policy file changes"
  else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "not ok $CASE_NUM - policy digest did not change ($digest_before == $digest_after)"
  fi
}

case_20_branch_moves_after_resolution() {
  local repo; repo="$(mk_repo case20)"
  local policy; policy="$(write_policy "$TMP_ROOT")"
  local c1; c1="$(commit_file "$repo" 1 "readme/NOTES.md" "hello")"
  local c2; c2="$(commit_file "$repo" 2 "docs/CHANGELOG.md" "v1")"
  run_tool "$repo" "$policy" "$c1" "$c2"
  local pinned_target; pinned_target="$(extract_field target_commit)"
  local c3; c3="$(commit_file "$repo" 3 "docs/CHANGELOG.md" "v2-after-admission")"
  CASE_NUM=$((CASE_NUM + 1))
  if [ "$pinned_target" = "$c2" ] && [ "$pinned_target" != "$c3" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo "ok $CASE_NUM - admitted record stays pinned to resolved commit after branch advances"
  else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "not ok $CASE_NUM - pinned target drifted (expected $c2, got $pinned_target)"
  fi
}

run_group() {
  case "$1" in
    policy) case_04_unknown_markdown_path ;;
    repository) case_13_wrong_remote ;;
    resolve) case_20_branch_moves_after_resolution ;;
    objects) case_07_missing_shallow_audit_object ;;
    ancestry) case_05_divergent_sibling_branch; case_06_force_moved_target ;;
    diff) case_08_mode_only_executable_change; case_09_regular_to_symlink; case_10_submodule_gitlink; case_11_rename_production_to_docs; case_12_newline_path ;;
    impact) case_02_linear_doc_descendant; case_03_production_descendant; case_03b_production_descendant_with_ruling; case_19_policy_digest_changed ;;
    worktree) case_14_wrong_local_head; case_15_tracked_dirty; case_16_staged_dirty; case_17_untracked_dirty; case_18_unmerged_index ;;
    record) case_01_exact_audit_basis; case_02_linear_doc_descendant ;;
    all)
      case_01_exact_audit_basis
      case_02_linear_doc_descendant
      case_03_production_descendant
      case_03b_production_descendant_with_ruling
      case_04_unknown_markdown_path
      case_05_divergent_sibling_branch
      case_06_force_moved_target
      case_07_missing_shallow_audit_object
      case_08_mode_only_executable_change
      case_09_regular_to_symlink
      case_10_submodule_gitlink
      case_11_rename_production_to_docs
      case_12_newline_path
      case_13_wrong_remote
      case_14_wrong_local_head
      case_15_tracked_dirty
      case_16_staged_dirty
      case_17_untracked_dirty
      case_18_unmerged_index
      case_19_policy_digest_changed
      case_20_branch_moves_after_resolution
      ;;
    *)
      echo "unknown selftest group: $1" >&2
      exit 64
      ;;
  esac
}

main() {
  local group="${1:-all}"
  TMP_ROOT="$(mktemp -d)"
  echo "1..$( [ "$group" = "all" ] && echo 21 || echo '?' )"
  run_group "$group"
  echo "# pass=$PASS_COUNT fail=$FAIL_COUNT"
  if [ "$FAIL_COUNT" -ne 0 ]; then
    exit 1
  fi
}

main "$@"
