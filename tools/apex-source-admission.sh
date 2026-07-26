#!/usr/bin/env bash
# APEX-A.1 source-current admission tool.
#
# Produces one immutable SourceAdmissionRecordV1 describing how a target
# commit relates to an audited source basis. Never builds code, never
# modifies the checkout, never resets a branch, never silently updates the
# impact policy. See readme/APEX-SOURCE-ADMISSION-SCHEMA-v1.md for the full
# vocabulary and readme/APEX-SOURCE-IMPACT-POLICY-v1.toml for path rulings.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOL_VERSION="1.0.0"

PYTHON_BIN="${APEX_ADMISSION_PYTHON:-python3}"
HELPER="$SCRIPT_DIR/apex_source_admission_helper.py"

EXPECTED_REPOSITORY=""
REMOTE="origin"
ALLOW_MIRROR="0"
AUDIT_COMMIT_INPUT=""
TARGET_REF=""
TARGET_COMMIT_INPUT=""
CHECK_WORKTREE="0"
POLICY_PATH="$REPO_ROOT/readme/APEX-SOURCE-IMPACT-POLICY-v1.toml"
OUTPUT_DIR="$REPO_ROOT/target/apex-source-admission"
GIT_DIR_OVERRIDE=""

usage() {
  cat >&2 <<'EOF'
usage: apex-source-admission.sh --expected-repository <name> --audit-commit <rev>
         (--target-ref <ref> | --target-commit <sha>)
         [--remote <name>] [--allow-mirror] [--check-worktree]
         [--policy <path>] [--output-dir <dir>] [--git-dir <path>]
EOF
  exit 64
}

while [ $# -gt 0 ]; do
  case "$1" in
    --expected-repository) EXPECTED_REPOSITORY="$2"; shift 2 ;;
    --remote) REMOTE="$2"; shift 2 ;;
    --allow-mirror) ALLOW_MIRROR="1"; shift ;;
    --audit-commit) AUDIT_COMMIT_INPUT="$2"; shift 2 ;;
    --target-ref) TARGET_REF="$2"; shift 2 ;;
    --target-commit) TARGET_COMMIT_INPUT="$2"; shift 2 ;;
    --check-worktree) CHECK_WORKTREE="1"; shift ;;
    --policy) POLICY_PATH="$2"; shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    --git-dir) GIT_DIR_OVERRIDE="$2"; shift 2 ;;
    -h|--help) usage ;;
    --*) echo "unknown option: $1" >&2; usage ;;
    *) echo "unexpected positional argument: $1" >&2; usage ;;
  esac
done

if [ -z "$EXPECTED_REPOSITORY" ] || [ -z "$AUDIT_COMMIT_INPUT" ]; then
  echo "missing required --expected-repository or --audit-commit" >&2
  usage
fi
if [ -n "$TARGET_REF" ] && [ -n "$TARGET_COMMIT_INPUT" ]; then
  echo "specify exactly one of --target-ref / --target-commit" >&2
  usage
fi
if [ -z "$TARGET_REF" ] && [ -z "$TARGET_COMMIT_INPUT" ]; then
  echo "specify exactly one of --target-ref / --target-commit" >&2
  usage
fi

if [ -n "$GIT_DIR_OVERRIDE" ]; then
  g() { git -C "$GIT_DIR_OVERRIDE" "$@"; }
else
  g() { git -C "$REPO_ROOT" "$@"; }
fi

TERMINAL=""
MERGE_BASE="null"

mkdir -p "$OUTPUT_DIR"

# --- 1. repository identity -------------------------------------------------
OBSERVED_REMOTE_URL="$(g remote get-url "$REMOTE" 2>/dev/null || true)"
if [ -z "$OBSERVED_REMOTE_URL" ]; then
  echo "no such remote: $REMOTE" >&2
  REPO_MATCH="0"
else
  case "$OBSERVED_REMOTE_URL" in
    *"$EXPECTED_REPOSITORY"*) REPO_MATCH="1" ;;
    *) REPO_MATCH="0" ;;
  esac
fi
if [ "$REPO_MATCH" != "1" ] && [ "$ALLOW_MIRROR" != "1" ]; then
  TERMINAL="BLOCK-REPOSITORY-MISMATCH"
fi

# --- 2. resolve target commit ------------------------------------------------
TARGET_COMMIT=""
if [ -z "$TERMINAL" ]; then
  if [ -n "$TARGET_REF" ]; then
    if g fetch --prune "$REMOTE" "refs/heads/$TARGET_REF:refs/remotes/$REMOTE/$TARGET_REF" >/dev/null 2>&1; then
      TARGET_COMMIT="$(g rev-parse --verify --end-of-options "refs/remotes/$REMOTE/$TARGET_REF^{commit}" 2>/dev/null || true)"
    fi
    if [ -z "$TARGET_COMMIT" ]; then
      TERMINAL="BLOCK-INVALID-REVISION"
    fi
  else
    TARGET_COMMIT="$(g rev-parse --verify --end-of-options "${TARGET_COMMIT_INPUT}^{commit}" 2>/dev/null || true)"
    if [ -z "$TARGET_COMMIT" ]; then
      TERMINAL="BLOCK-INVALID-REVISION"
    fi
  fi
fi

# --- 3. verify audit commit, capture trees ----------------------------------
AUDIT_COMMIT=""
AUDIT_TREE=""
TARGET_TREE=""
if [ -z "$TERMINAL" ]; then
  AUDIT_COMMIT="$(g rev-parse --verify --end-of-options "${AUDIT_COMMIT_INPUT}^{commit}" 2>/dev/null || true)"
  if [ -z "$AUDIT_COMMIT" ]; then
    TERMINAL="BLOCK-INVALID-REVISION"
  else
    AUDIT_TREE="$(g show -s --format=%T "$AUDIT_COMMIT")"
    TARGET_TREE="$(g show -s --format=%T "$TARGET_COMMIT")"
  fi
fi

# --- 4. classify relation ----------------------------------------------------
SOURCE_RELATION=""
if [ -z "$TERMINAL" ]; then
  if [ "$AUDIT_COMMIT" = "$TARGET_COMMIT" ]; then
    SOURCE_RELATION="ExactAuditBasis"
    MB="$AUDIT_COMMIT"
  else
    set +e
    g merge-base --is-ancestor "$AUDIT_COMMIT" "$TARGET_COMMIT"
    RC=$?
    set -e
    MB="$(g merge-base "$AUDIT_COMMIT" "$TARGET_COMMIT" 2>/dev/null || true)"
    if [ $RC -eq 0 ]; then
      SOURCE_RELATION="Descendant"
    elif [ $RC -eq 1 ]; then
      SOURCE_RELATION="DivergedHistory"
    else
      SOURCE_RELATION="Unresolved"
      TERMINAL="BLOCK-SHALLOW-MISSING-HISTORY"
    fi
  fi
  if [ -n "$MB" ]; then
    MERGE_BASE="\"$MB\""
  fi
fi

if [ -z "$TERMINAL" ] && [ "$SOURCE_RELATION" = "DivergedHistory" ]; then
  TERMINAL="BLOCK-DIVERGED-HISTORY"
fi

# --- 5. changed-path inventory + policy classification -----------------------
CHANGED_PATHS_JSON="[]"
IMPACT_VERDICT="NoChanges"
POLICY_DIGEST="$($PYTHON_BIN "$HELPER" policy-digest "$POLICY_PATH")"

if [ -z "$TERMINAL" ] && [ "$SOURCE_RELATION" = "Descendant" ]; then
  DT_RAW="$OUTPUT_DIR/changed-files.raw.z"
  NS_RAW="$OUTPUT_DIR/changed-files.name-status.z"
  g diff-tree --no-commit-id -r --raw -z --no-abbrev "$AUDIT_COMMIT" "$TARGET_COMMIT" > "$DT_RAW"
  g diff --name-status -z --find-renames=50% "$AUDIT_COMMIT" "$TARGET_COMMIT" > "$NS_RAW"
  CLASSIFY_OUT="$($PYTHON_BIN "$HELPER" classify "$POLICY_PATH" "$DT_RAW" "$NS_RAW")"
  CHANGED_PATHS_JSON="$(printf '%s' "$CLASSIFY_OUT" | $PYTHON_BIN -c 'import json,sys; print(json.dumps(json.load(sys.stdin)["changed_paths"]))')"
  IMPACT_VERDICT="$(printf '%s' "$CLASSIFY_OUT" | $PYTHON_BIN -c 'import json,sys; print(json.load(sys.stdin)["impact_verdict"])')"

  HAS_UNKNOWN="$(printf '%s' "$CHANGED_PATHS_JSON" | $PYTHON_BIN -c 'import json,sys; d=json.load(sys.stdin); print("1" if any(p["impact"]=="UnknownImpact" for p in d) else "0")')"
  if [ "$HAS_UNKNOWN" = "1" ]; then
    TERMINAL="BLOCK-UNKNOWN-IMPACT"
  elif [ "$IMPACT_VERDICT" = "ProductionOrBuildChanged" ]; then
    TERMINAL="READMIT-PRODUCTION"
  elif [ "$IMPACT_VERDICT" = "EvidenceOrToolingChanged" ]; then
    TERMINAL="RECHECK-EVIDENCE"
  fi
fi

# --- 6. optional checkout admission ------------------------------------------
CHECKOUT_VERDICT="NotChecked"
if [ "$CHECK_WORKTREE" = "1" ]; then
  HEAD_COMMIT="$(g rev-parse --verify HEAD^{commit} 2>/dev/null || true)"
  UNMERGED_FILE="$OUTPUT_DIR/.unmerged.raw.z"
  STATUS_FILE="$OUTPUT_DIR/.status.raw.z"
  g ls-files --unmerged -z > "$UNMERGED_FILE" 2>/dev/null || true
  g status --porcelain=v2 -z --untracked-files=all > "$STATUS_FILE" 2>/dev/null || true
  UNMERGED_COUNT="$(wc -c < "$UNMERGED_FILE" | tr -d ' ')"
  if [ -z "$HEAD_COMMIT" ]; then
    CHECKOUT_VERDICT="Unresolved"
  elif [ "$UNMERGED_COUNT" != "0" ]; then
    CHECKOUT_VERDICT="Unmerged"
  elif [ "$HEAD_COMMIT" != "$TARGET_COMMIT" ]; then
    CHECKOUT_VERDICT="WrongHead"
  else
    STATUS_JSON="$($PYTHON_BIN "$HELPER" checkout-status "$STATUS_FILE")"
    TRACKED="$(printf '%s' "$STATUS_JSON" | $PYTHON_BIN -c 'import json,sys; print("1" if json.load(sys.stdin)["tracked"] else "0")')"
    UNTRACKED="$(printf '%s' "$STATUS_JSON" | $PYTHON_BIN -c 'import json,sys; print("1" if json.load(sys.stdin)["untracked"] else "0")')"
    if [ "$TRACKED" = "1" ]; then
      CHECKOUT_VERDICT="TrackedChanges"
    elif [ "$UNTRACKED" = "1" ]; then
      CHECKOUT_VERDICT="UntrackedChanges"
    else
      CHECKOUT_VERDICT="ExactAndClean"
    fi
  fi
  rm -f "$UNMERGED_FILE" "$STATUS_FILE"

  if [ -z "$TERMINAL" ]; then
    case "$CHECKOUT_VERDICT" in
      WrongHead) TERMINAL="BLOCK-WRONG-HEAD" ;;
      TrackedChanges) TERMINAL="BLOCK-DIRTY-TRACKED" ;;
      UntrackedChanges) TERMINAL="BLOCK-DIRTY-UNTRACKED" ;;
      Unmerged) TERMINAL="BLOCK-UNMERGED" ;;
      Unresolved) TERMINAL="BLOCK-SHALLOW-MISSING-HISTORY" ;;
    esac
  fi
fi

# --- 7. final admit terminal (only reached with no block/recheck) -----------
if [ -z "$TERMINAL" ]; then
  if [ "$SOURCE_RELATION" = "ExactAuditBasis" ]; then
    TERMINAL="ADMIT-EXACT"
  elif [ "$SOURCE_RELATION" = "Descendant" ]; then
    TERMINAL="ADMIT-DOC-ONLY"
  else
    TERMINAL="BLOCK-DIVERGED-HISTORY"
  fi
fi

# --- 8. emit record -----------------------------------------------------------
if [ -n "$TARGET_REF" ]; then
  TARGET_NAMED_REF_JSON="\"$TARGET_REF\""
else
  TARGET_NAMED_REF_JSON="null"
fi

PARTIAL_JSON="$OUTPUT_DIR/.partial-record.json"
CHANGED_PATHS_FILE="$OUTPUT_DIR/.changed-paths.json"
printf '%s' "$CHANGED_PATHS_JSON" > "$CHANGED_PATHS_FILE"

export TOOL_VERSION EXPECTED_REPOSITORY OBSERVED_REMOTE_URL TARGET_NAMED_REF_JSON \
       AUDIT_COMMIT AUDIT_TREE TARGET_COMMIT TARGET_TREE MERGE_BASE SOURCE_RELATION \
       POLICY_PATH POLICY_DIGEST IMPACT_VERDICT CHECKOUT_VERDICT TERMINAL

$PYTHON_BIN - "$PARTIAL_JSON" "$CHANGED_PATHS_FILE" <<'PYEOF'
import json, sys, os
out_path = sys.argv[1]
changed_paths_path = sys.argv[2]
with open(changed_paths_path, "r", encoding="utf-8") as f:
    changed_paths = json.load(f)
env = os.environ
record = {
    "schema": "bastion.source-admission/v1",
    "admission_tool_version": env["TOOL_VERSION"],
    "repository_expected": env["EXPECTED_REPOSITORY"],
    "repository_observed_remote": env["OBSERVED_REMOTE_URL"],
    "target_named_ref": json.loads(env["TARGET_NAMED_REF_JSON"]),
    "audit_commit": env["AUDIT_COMMIT"],
    "audit_tree": env["AUDIT_TREE"],
    "target_commit": env["TARGET_COMMIT"],
    "target_tree": env["TARGET_TREE"],
    "merge_base": json.loads(env["MERGE_BASE"]),
    "source_relation": env["SOURCE_RELATION"],
    "impact_policy_path": env["POLICY_PATH"],
    "impact_policy_digest": env["POLICY_DIGEST"],
    "changed_paths": changed_paths,
    "impact_verdict": env["IMPACT_VERDICT"],
    "checkout_verdict": env["CHECKOUT_VERDICT"],
    "terminal_code": env["TERMINAL"],
}
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(record, f)
PYEOF
rm -f "$CHANGED_PATHS_FILE"

FINAL_JSON="$OUTPUT_DIR/source-admission-v1.json"
TMP_JSON="$OUTPUT_DIR/.source-admission-v1.json.tmp.$$"
$PYTHON_BIN "$HELPER" emit-record "$PARTIAL_JSON" > "$TMP_JSON"
mv -f "$TMP_JSON" "$FINAL_JSON"
rm -f "$PARTIAL_JSON"

SHA_LINE="$(sha256sum "$FINAL_JSON" | awk '{print $1"  source-admission-v1.json"}')"
echo "$SHA_LINE" > "$FINAL_JSON.sha256"

echo "TERMINAL_CODE=$TERMINAL"
echo "RECORD=$FINAL_JSON"
cat "$FINAL_JSON"

case "$TERMINAL" in
  ADMIT-*) exit 0 ;;
  RECHECK-EVIDENCE) exit 0 ;;
  *) exit 1 ;;
esac
