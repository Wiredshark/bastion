#!/bin/sh
# score-desires-ab.sh — the societal axis's paired A/B (charter FR15 pattern).
#
# Usage: score-desires-ab.sh <meritocratic-server-log> <meritocratic-driver-log>
#                            <individualist-server-log> <individualist-driver-log>
#
# THROUGHPUT: "job completed" count in the server log (same script, same
# window length by construction — the arms are comparable because ONLY α
# differs; that is the whole design).
# MOOD: mean of every mood= sample in the driver's INSPECT lines.
#
# VERDICT TABLE (registered in DESIRES-V1-PREREGISTRATION.md):
#   PASS               merit throughput > indiv AND merit mood < indiv
#   AXIS-FAILS         any other combination — reported in the charter's own
#                      words ("decorative"), never softened to partial
#   VOID               either arm has 0 completions or 0 mood samples
set -u
MS="${1:?merit server log}"; MD="${2:?merit driver log}"
IS="${3:?indiv server log}"; ID="${4:?indiv driver log}"
count() { sed 's/\x1b\[[0-9;]*m//g' "$1" | grep -c "bastion: job completed"; }
mood() {
  sed 's/\x1b\[[0-9;]*m//g' "$1" | grep -oE 'mood=[0-9.]+' | sed 's/mood=//' \
    | awk '{s+=$1; n+=1} END {if (n>0) printf "%.4f %d", s/n, n; else print "0 0"}'
}
mt=$(count "$MS"); it=$(count "$IS")
set -- $(mood "$MD"); mm=$1; mn=$2
set -- $(mood "$ID"); im=$1; in_=$2
echo "throughput: meritocratic=$mt individualist=$it"
echo "mood      : meritocratic=$mm (n=$mn) individualist=$im (n=$in_)"
if [ "$mt" -eq 0 ] || [ "$it" -eq 0 ] || [ "$mn" -eq 0 ] || [ "$in_" -eq 0 ]; then
  echo "VOID: a leg produced no completions or no mood samples — precondition"
  echo "      failed; this is not evidence about the axis."; exit 3
fi
thr_ok=$(awk -v a="$mt" -v b="$it" 'BEGIN{print (a>b)?1:0}')
mood_ok=$(awk -v a="$mm" -v b="$im" 'BEGIN{print (a<b)?1:0}')
if [ "$thr_ok" = 1 ] && [ "$mood_ok" = 1 ]; then
  echo "VERDICT: PASS — the tradeoff MEASURES in both directions."
else
  echo "VERDICT: AXIS-FAILS-DECORATIVE — the charter's words, not softened:"
  [ "$thr_ok" = 1 ] || echo "  throughput direction MISSING (merit $mt !> indiv $it)"
  [ "$mood_ok" = 1 ] || echo "  mood direction MISSING (merit $mm !< indiv $im)"
fi
