#!/bin/sh
# corpus-first.sh — the banked-corpus question, enforced for LOCAL runs.
#
# ★★ WHY THIS EXISTS, and it is not a hypothetical. On 2026-08-19 I enforced the
# corpus-first rule in `vm-fan.sh` (exit 9 without CORPUS), wrote in that commit
# that a rule fired only from memory "is not a default" — and then, two hours
# later, skipped the question on a LOCAL run and spent two harness passes
# re-deriving a result the corpus already held. On the WRONG fixture. Producing
# a null I nearly published as "the fix is harmless".
#
# The fan guard could not have stopped it: the fan guard refuses SPEND, and that
# run was free. **The rule is "ask before RUNNING", not "ask before spending" —
# and free runs are exactly where the question feels skippable.**
#
# Cost of the check that was skipped: one `ls`. Cost of skipping it: two harness
# runs and a wrong-fixture null.
#
# Usage — source it at the top of any script that runs the harness:
#     . "$(dirname "$0")/corpus-first.sh"
#
# CORPUS must say what was checked. Two explicit escapes, both loud, because a
# guard with no honest escape just gets fed noise:
#     CORPUS=none-exists        no banked corpus covers this axis
#     CORPUS=checked:<what>     named corpus checked and insufficient
#
# ★ The escapes are deliberately WORDS, not a bare flag. Typing what you checked
# is the check; a `SKIP=1` would be satisfied without anyone looking at anything.

: "${CORPUS:=}"
if [ -z "$CORPUS" ]; then
  echo "!! REFUSING TO RUN: CORPUS is not set." >&2
  echo "   Before ANY run — free ones included — does an attested banked corpus" >&2
  echo "   already answer this? Local runs are where this question gets skipped." >&2
  echo "   Say what you checked:" >&2
  echo "     CORPUS='checked: HAUL-DEADLOCK-AB-RESULTS.md — b5 does not farm'" >&2
  echo "     CORPUS=none-exists" >&2
  echo "   Look first:  ls bastion-test-evidence/*.md | grep -i <topic>" >&2
  exit 9
fi
echo "corpus-first     : $CORPUS"
