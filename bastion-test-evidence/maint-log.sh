#!/usr/bin/env bash
# RECORD DESTRUCTIVE MAINTENANCE, AT THE MOMENT IT HAPPENS.
#
# WHY: a coverage measure built on `userdata-<TAG>` directories was voided
# because 12 of 15 attested tags had no directory -- they had been deleted
# hours earlier when a full disk (masquerading as a compile error) forced a
# 52 GB cleanup. That cleanup was necessary and correct, and it silently
# changed the denominator of every future count over run artefacts. Nothing
# on disk recorded that it happened.
#
# An unrecorded deletion is a confounder with no expiry. An action that
# leaves no artefact is indistinguishable from an action that never
# happened -- a law this project has applied to attestation, config,
# teardown and run accounting, and never to the operator's own hands.
#
# THIS RECORDS; IT DOES NOT REFUSE. A gate that blocked a disk-full
# recovery would be actively harmful at the worst possible moment, so an
# entry is written even when no reason is supplied -- an unexplained
# deletion that is LOGGED is strictly better than one that is not, and
# withholding the record to punish the omission would lose the fact too.
#
# Usage:  maint-log.sh "<what was removed>" "<why>"
set -u
EV="${EV:-/e/veloren-master/bastion-test-evidence}"
WT="${WT:-/e/veloren-master/.engine-integration-wt}"
LOG="$EV/MAINTENANCE.md"
WHAT="${1:?usage: maint-log.sh \"<what>\" \"<why>\"}"
WHY="${2-}"

if [ ! -f "$LOG" ]; then
  # Created on first use, with its own purpose at the top -- a bare list of
  # deletions with no statement of what it is for reads, later, like debris.
  printf '# MAINTENANCE LOG\n\nDestructive actions on this tree, recorded AT THE TIME.\nAn unrecorded deletion silently changes the denominator of every later count.\n\n' > "$LOG"
fi

# APPEND, never overwrite. A maintenance log that loses history is the
# defect it exists to prevent.
{
  printf -- '- **%s** · HEAD `%s`\n' \
    "$(date '+%F %T')" "$(git -C "$WT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  printf -- '  - removed: %s\n' "$WHAT"
  if [ -z "$WHY" ]; then
    printf -- '  - **!! NO REASON GIVEN**\n'
  else
    printf -- '  - why: %s\n' "$WHY"
  fi
} >> "$LOG"

echo "maintenance recorded in $LOG"
