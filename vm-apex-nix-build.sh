#!/usr/bin/env bash
# APEX-T1.1.09 — exact-commit Nix package build (runs ON the repro-base VM /
# any Linux+Nix host). Consumes ONE immutable full commit from APEX-A.1
# admission; a moving branch is rejected as input (T1.1-BLOCK-BRANCH-INPUT).
#
#   ADMITTED_COMMIT=<40 lower hex> [REPO_URL=…] bash vm-apex-nix-build.sh
#
# Terminals: T1.1-PACKAGE-READY | T1.1-BLOCK-BRANCH-INPUT |
#   T1.1-BLOCK-LOCK-MUTATION | T1.1-BLOCK-UNKNOWN-REVISION | T1.1-BLOCK-NCI-OUTPUT
set -euo pipefail
echo "BUILD_LANE=APEX-NIX-V1"

ADMITTED_COMMIT="${ADMITTED_COMMIT:-}"
REPO_URL="${REPO_URL:-https://github.com/Wiredshark/bastion.git}"
WORKDIR="${WORKDIR:-$HOME/apex-src}"

# ── input admission: full 40-lower-hex commit, nothing else ──────────────────
case "$ADMITTED_COMMIT" in
  "" ) echo "TERMINAL: T1.1-BLOCK-BRANCH-INPUT (ADMITTED_COMMIT unset — a branch default is forbidden)"; exit 9 ;;
esac
if ! printf '%s' "$ADMITTED_COMMIT" | grep -Eq '^[0-9a-f]{40}$'; then
  echo "TERMINAL: T1.1-BLOCK-BRANCH-INPUT (ADMITTED_COMMIT is not 40 lowercase hex: ${ADMITTED_COMMIT})"; exit 9
fi

# ── exact detached checkout (no branch resolution after admission) ───────────
if [ ! -d "$WORKDIR/.git" ]; then
  git clone --no-checkout "$REPO_URL" "$WORKDIR"
fi
cd "$WORKDIR"
git fetch origin "$ADMITTED_COMMIT" || git fetch origin
git -c advice.detachedHead=false checkout --detach "$ADMITTED_COMMIT"
HEAD_NOW=$(git rev-parse HEAD)
[ "$HEAD_NOW" = "$ADMITTED_COMMIT" ] || { echo "TERMINAL: T1.1-BLOCK-UNKNOWN-REVISION (HEAD $HEAD_NOW != admitted)"; exit 8; }
echo "RAN_COMMIT=$HEAD_NOW"

# ── locked package build; the lock must not move ─────────────────────────────
LOCK_PRE=$(sha256sum flake.lock | cut -d' ' -f1)
OUT=$(nix build ".#bastion-harness" \
  --no-update-lock-file --no-write-lock-file --no-link --print-out-paths) \
  || { echo "TERMINAL: T1.1-BLOCK-NCI-OUTPUT (locked build failed)"; exit 7; }
LOCK_POST=$(sha256sum flake.lock | cut -d' ' -f1)
[ "$LOCK_PRE" = "$LOCK_POST" ] || { echo "TERMINAL: T1.1-BLOCK-LOCK-MUTATION"; exit 6; }
echo "flake_lock_sha256=$LOCK_PRE"
echo "store_path=$OUT"

# ── metadata-only canary: stamp must map to the admitted revision ────────────
[ -x "$OUT/bin/bastion-harness" ] || { echo "TERMINAL: T1.1-BLOCK-NCI-OUTPUT (no executable)"; exit 7; }
STAMP=$("$OUT/bin/bastion-harness" --print-git-hash | tail -1)
EXPECT="${ADMITTED_COMMIT:0:10}"
if [ "${STAMP%%+*}" != "$EXPECT" ]; then
  echo "TERMINAL: T1.1-BLOCK-UNKNOWN-REVISION (stamp ${STAMP} != admitted prefix ${EXPECT})"; exit 8
fi
echo "package_stamp=$STAMP admitted_prefix=$EXPECT"
echo "TERMINAL: T1.1-PACKAGE-READY"
