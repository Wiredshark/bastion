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
# THE RUN'S CONFIGURATION IS PART OF ITS PROVENANCE. A Bastion run's
# behaviour is decided by BASTION_* env vars (flat arena, autofound count,
# determinism, uncapped TPS) far more than by anything else, and until
# 2026-08-15 no evidence file recorded ANY of them: HEAD and binary mtime
# were attested while the configuration that decided what the run DID was
# not written down anywhere a later reader could find it.
#
# IT IS PASSED IN, NOT OBSERVED. Runners set these vars INLINE on the
# server command line (`BASTION_DETERMINISTIC=1 ... veloren-server-cli`),
# so they are not exported and this script's own environment does not
# contain them -- reading `env` here would print an empty list and exit 0,
# a clean-looking attestation of a run whose entire configuration is
# invisible. The runner therefore DECLARES the config once in a variable
# and passes that same variable both here (recorded) and to the server
# (applied): one definition, two consumers, so the recorded config cannot
# drift from the applied one.
#
# AND AN UNDECLARED CONFIG IS NOT AN EMPTY ONE. `BASTION_ENV` unset prints
# an explicit NO CONFIG DECLARED marker rather than a blank line, because
# an empty list is exactly what BOTH the healthy no-config case and the
# broken cannot-see-the-config case would produce.
#
# Usage:  attest-run.sh <evidence-file> [binary ...]
#   env:  BASTION_ENV   the run's declared config, e.g. "BASTION_DETERMINISTIC=1 ..."
#         BASTION_LOGS  the evidence files this run will write (space-separated)
set -u
WT=/e/veloren-master/.engine-integration-wt
OUT="${1:?usage: attest-run.sh <evidence-file> [binary ...]}"
shift || true
# Collected inside the `{ } | tee` block, which runs in a SUBSHELL -- a plain
# variable set there would not survive to the exit check below.
FLAG=$(mktemp)
: > "$FLAG"

{
  echo "=== RUN ATTESTATION $(date '+%F %T') ==="
  echo "HEAD          : $(git -C "$WT" rev-parse --short HEAD 2>/dev/null)"
  DIRTY=$(git -C "$WT" status --porcelain -- '*.rs' 2>/dev/null | wc -l | tr -d ' ')
  echo "dirty .rs     : $DIRTY"
  if [ "$DIRTY" != "0" ]; then
    echo "  !! THE BINARY'S BAKED HASH NAMES HEAD, NOT THIS TREE."
    git -C "$WT" status --porcelain -- '*.rs' 2>/dev/null | sed 's/^/     /'
  fi
  # DECLARED, not observed -- see the header. `+x` distinguishes "unset"
  # from "set but empty"; a bare `-z` test would collapse them into the
  # same blank output, which is the one rendering this bar forbids.
  if [ -z "${BASTION_ENV+x}" ]; then
    echo "run config    : !! NO CONFIG DECLARED (BASTION_ENV unset)"
    echo "  the run's BASTION_* settings are NOT recorded in this evidence file."
  elif [ -z "$BASTION_ENV" ]; then
    echo "run config    : declared EMPTY (no BASTION_* vars set for this run)"
  else
    echo "run config    : $(printf '%s' "$BASTION_ENV" | tr -s ' \t' ' ')"
    printf '%s' "$BASTION_ENV" | tr -s ' \t' '\n' | grep -c . | sed 's/^/  vars declared : /'
  fi
  # THE ATTESTATION MUST NAME ITS OUTPUTS (2026-08-15). Until now this file
  # recorded what went IN -- HEAD, dirty tree, config, binaries -- and not one
  # word about what came OUT. The consequence was measured: reconciling the
  # ledger's 24 attested tags against the 143 server logs on disk was VOID,
  # because the only link between an attestation and its evidence lived inside
  # each launcher's source. `run-prio-powered.sh` writes `server-pw-<ARM>.log`
  # while its attestation is `powered-attest.txt`, and NOTHING anywhere relates
  # `powered` to `pw`.
  #
  # DECLARED, not observed -- same reason as the config above. The preamble
  # runs BEFORE the run, so the files do not exist yet and there is nothing to
  # observe; every path here is a PROMISE, which is why `run-ledger.sh` checks
  # each one against the disk instead of trusting it.
  #
  # And `+x` again: an undeclared output set must not render as an empty one.
  # A run that writes four logs and declares none, and a run that genuinely
  # writes nothing, are the two cases a blank line would collapse.
  #
  # AND SILENCE IS REFUSED, NOT MERELY RECORDED (2026-08-15). The field above
  # was honest about its own absence and nothing prevented it: a runner that
  # declared nothing still passed the gate, so the attestation kept being
  # correct about being useless. The refusal lives HERE and not in
  # `launch-preamble.sh` on purpose -- a check in the preamble would gate
  # exactly the runners that already source it, which are the same three that
  # already declare, so it would refuse nothing that exists. This is where the
  # exit code is produced, so it also gates a direct caller; 12 of the first
  # 13 attestations in this directory were hand-run.
  #
  # ONLY SILENCE. `declared EMPTY` PASSES: a run that genuinely writes no
  # evidence files is legitimate, and it is a CHOICE the runner recorded.
  # This distinction is only expressible because `+x` separates unset from
  # empty -- a `-z` test would have to refuse both or neither, and refusing
  # `declared EMPTY` would punish the honest case.
  #
  # WHAT THIS DOES NOT BUY: a runner can now satisfy the gate by declaring
  # EMPTY falsely, and nothing here can tell. At this point the run has not
  # happened, so there is no output to compare a declaration against. This
  # converts silence into an explicit claim; it does not make the claim true.
  # `run-ledger.sh` checks the one contradiction that is observable after the
  # fact -- a tag that declared EMPTY whose `$TAG.log` exists on disk.
  if [ -z "${BASTION_LOGS+x}" ]; then
    echo "outputs       : !! NO OUTPUTS DECLARED (BASTION_LOGS unset)"
    echo "  this run's evidence files are NOT recoverable from this attestation."
    echo "UNDECLARED" >> "$FLAG"
  elif [ -z "$BASTION_LOGS" ]; then
    echo "outputs       : declared EMPTY (this run writes no evidence files)"
  else
    printf '%s' "$BASTION_LOGS" | tr -s ' \t' '\n' | grep -c . \
      | sed 's/^/outputs       : /'
    # One path per line, machine-readable prefix, so a reader needs no parsing
    # of the summary line above.
    printf '%s' "$BASTION_LOGS" | tr -s ' \t' '\n' | grep . | sed 's/^/output        : /'
  fi
  for BIN in "$@"; do
    if [ -f "$BIN" ]; then
      echo "binary        : $(basename "$BIN")  built $(stat -c %y "$BIN" 2>/dev/null | cut -c1-19)"
      # Freshness against the newest tracked source: a binary older than its
      # own inputs is the void that cost this session a scored run.
      #
      # SOURCE SET IS PER-BINARY (2026-08-15). `client/src` was omitted
      # entirely, so a driver-source edit could not trip STALE even when the
      # driver was passed -- a confident "fresh" for a binary whose source had
      # changed, the provenance theatre this file's header warns about.
      #
      # But adding it GLOBALLY was worse: the server binary would then read
      # stale on any client edit, and a gate that always fails is no more a
      # gate than one that never does. Each binary is checked against the
      # crates it is actually built from.
      case "$(basename "$BIN")" in
        bastion_playtest*)
          SRC="$WT/client/src $WT/common/src" ;;
        *)
          SRC="$WT/server/src $WT/bastion-server/src $WT/common/src $WT/rtsim/src" ;;
      esac
      # shellcheck disable=SC2086
      NEWEST=$(find $SRC -name '*.rs' -newer "$BIN" -print -quit 2>/dev/null)
      if [ -n "$NEWEST" ]; then
        echo "  !! STALE: source is NEWER than this binary, e.g. ${NEWEST#$WT/}"
        echo "STALE" >> "$FLAG"
      else
        echo "  fresh: no tracked .rs source is newer than the binary"
      fi
    else
      echo "binary        : $BIN  (MISSING)"
      echo "MISSING" >> "$FLAG"
    fi
  done
} | tee -a "$OUT"

# REFUSE, do not merely warn. A warning depends on the operator reading it; an
# exit code lets a runner gate on it (`attest-run.sh .. || exit 1`). Both
# directions matter: an exit code that is always 0 is not a gate, and one that
# is always non-zero is not one either.
if [ -s "$FLAG" ]; then
  echo "ATTESTATION FAILED: $(tr '\n' ' ' < "$FLAG")" | tee -a "$OUT"
  rm -f "$FLAG"
  exit 1
fi
rm -f "$FLAG"
exit 0
