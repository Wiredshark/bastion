#!/usr/bin/env bash
# RUN-TIME BUILD ATTESTATION for scored Bastion runs.
#
# WHY THIS IS A SCRIPT AND NOT A BUILD-SCRIPT FIELD:
# `common/build.rs` bakes the version from `git log -n 1` and declares
# `cargo::rerun-if-changed` on `.git/HEAD` and `.git/logs/HEAD` ONLY -- a
# deliberate fix for a real staleness bug (its own comment explains it).
# That optimisation makes a DIRTY-TREE flag unimplementable there: editing
# `server/src/lib.rs` touches neither watched path, so the build script does
# not re-run, and any dirt it recorded would be the dirt as of the last
# COMMIT. It would report "clean" for a dirty build -- worse than nothing,
# because it would look like provenance.
#
# Evaluated at RUN time, this cannot go stale.
#
# THE STALENESS CHECK IS MTIME-BASED AND DELIBERATELY CONSERVATIVE. `git
# checkout --` or `touch` rewrites a file's mtime without changing a byte, so
# it will warn on a binary that is in fact current. That is the correct bias:
# the failure this exists to prevent is UNDER-warning -- a stale binary that
# produced a confident, internally consistent, wrong result. Over-warning
# costs a rebuild; under-warning cost this session a scored run.
#
# The two checks are INDEPENDENT and were red-demonstrated separately:
# `touch` alone trips STALE with `dirty .rs : 0`; a real content edit trips
# both. Neither subsumes the other -- a committed-then-rebuilt tree is clean
# but can still be stale, and a dirty tree can still be freshly built.
#
# Usage:  attest-run.sh <evidence-file> [binary ...]
set -u
WT=/e/veloren-master/.engine-integration-wt
OUT="${1:?usage: attest-run.sh <evidence-file> [binary ...]}"
shift || true

{
  echo "=== RUN ATTESTATION $(date '+%F %T') ==="
  echo "HEAD          : $(git -C "$WT" rev-parse --short HEAD 2>/dev/null)"
  DIRTY=$(git -C "$WT" status --porcelain -- '*.rs' 2>/dev/null | wc -l | tr -d ' ')
  echo "dirty .rs     : $DIRTY"
  if [ "$DIRTY" != "0" ]; then
    echo "  !! THE BINARY'S BAKED HASH NAMES HEAD, NOT THIS TREE."
    git -C "$WT" status --porcelain -- '*.rs' 2>/dev/null | sed 's/^/     /'
  fi
  for BIN in "$@"; do
    if [ -f "$BIN" ]; then
      echo "binary        : $(basename "$BIN")  built $(stat -c %y "$BIN" 2>/dev/null | cut -c1-19)"
      # Freshness against the newest tracked source: a binary older than its
      # own inputs is the void that cost this session a scored run.
      NEWEST=$(find "$WT/server/src" "$WT/bastion-server/src" "$WT/common/src" "$WT/rtsim/src" \
                 -name '*.rs' -newer "$BIN" -print -quit 2>/dev/null)
      if [ -n "$NEWEST" ]; then
        echo "  !! STALE: source is NEWER than this binary, e.g. ${NEWEST#$WT/}"
      else
        echo "  fresh: no tracked .rs source is newer than the binary"
      fi
    else
      echo "binary        : $BIN  (MISSING)"
    fi
  done
} | tee -a "$OUT"
