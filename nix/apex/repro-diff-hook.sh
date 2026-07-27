#!/usr/bin/env bash
# APEX-T1.3.10 — Nix diff hook. Runs ONLY after Nix has already decided
# two outputs differ; its job is to preserve diagnostics before the
# `.check` path disappears. Its exit code can never convert the failure
# into success (Nix ignores hook exit for the verdict; we exit 0 anyway so
# a hook crash is never mistaken for anything else — capture failure is
# reported by the orchestrator as BLOCK-DIAGNOSTIC-CAPTURE when the bundle
# is missing).
# Args from Nix: $1 = existing output, $2 = rejected .check output.
set -u
EVIDENCE_DIR="${APEX_REPRO_EVIDENCE_DIR:-/tmp/apex-repro-evidence}"
mkdir -p "$EVIDENCE_DIR/diff" || exit 0
{
  echo "diff-hook: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "existing=$1"
  echo "rejected=$2"
  # Bounded recursive diff (diffoscope may not be in the closure; plain
  # diff -r is the guaranteed baseline diagnostic).
  timeout 120 diff -r "$1" "$2" 2>&1 | head -c 262144
} > "$EVIDENCE_DIR/diff/diff-$(basename "$1").txt" 2>&1 || true
exit 0
