#!/bin/sh
# score-lod-gate.sh — row #89 bar 2: does tick-gating the CLIENT's LoD request
# timer remove the chunk-SCHEDULE divergence?
#
# WHAT BAR 2 ACTUALLY FAILS ON. Its own decomposition:
#     membership (which chunks) IDENTICAL 30/30
#     schedule   (which tick)   DIFFERS   31/31
# Three SERVER-side fixes were eliminated by measurement. The standing finding
# is that the CLIENT's demand is the uncontrolled half.
#
# THE LEVER. `BASTION_TICK_GATED_REQUESTS` already gated the chunk-request
# retain, and its comment claimed that was "the only wall-clock read left in
# that chain" — true of the CHUNK chain. Five lines later the LoD zone request
# timer runs on `elapsed() > 5s`, ungated, in the same request path. That is now
# tick-based under the same flag.
#
# REGISTERED OUTCOMES, declared before the run:
#   FIXED     — schedule IDENTICAL across twins with the flag ON, and DIFFERS
#               with it OFF. The LoD timer was a cause.
#   NO EFFECT — schedule differs in BOTH arms. The LoD timer is exonerated; bar
#               2's timing failure has a cause still unnamed. A real result.
#   PARTIAL   — fewer differing ticks ON than OFF but not zero. Report the
#               counts; do NOT round to "fixed".
#   VOID      — the census emit is absent (needs BASTION_TERRAIN_PROVISION_DIAG),
#               or the LoD witness never fired in the ON arm. Then the arms are
#               not what they claim and no verdict is printed.
#
# ★ The VOID branch is the one that matters. "The probe did nothing" and "the
#   probe never ran" produce identical schedules, and axis 1's first run was
#   VOID for exactly that reason.
#
# Usage: sh score-lod-gate.sh <dir-with-logs>
#   Expects four server logs: off-a, off-b, on-a, on-b.
set -u
# ★ corpus-first, enforced for LOCAL runs too (see corpus-first.sh)
. "$(dirname "$0")/corpus-first.sh"
D="${1:?usage: score-lod-gate.sh <dir>}"

seq_of() {  # (tick -> keys) sequence, the thing bar 2 compares
  sed 's/\x1b\[[0-9;]*m//g' "$1" 2>/dev/null \
    | grep "bastion: terrain provisioning census" \
    | grep -oE 'tick=[0-9]+ promoted=[0-9]+' \
    | sed 's/ promoted=/:/'
}
keys_of() { # membership only, order-independent
  sed 's/\x1b\[[0-9;]*m//g' "$1" 2>/dev/null \
    | grep "bastion: terrain provisioning census" \
    | grep -oE 'keys=\[[^]]*\]' | tr -d ' ' | sort -u
}

for f in off-a off-b on-a on-b; do
  [ -s "$D/$f.log" ] || { echo "VOID: $D/$f.log missing or empty" >&2; exit 3; }
done

# Precondition 1: the census emit must exist at all.
n=$(seq_of "$D/off-a.log" | wc -l)
if [ "$n" -eq 0 ]; then
  echo "VOID: no 'terrain provisioning census' emits — was BASTION_TERRAIN_PROVISION_DIAG set?" >&2
  exit 3
fi
echo "census emits (off-a): $n"

# Precondition 2: the probe must have ACTED in the ON arm.
w=$(grep -c "row89 LoD request TICK-GATED" "$D/on-a.log" 2>/dev/null)
echo "LoD tick-gate witness (on-a): $w"
if [ "$w" -eq 0 ]; then
  echo "VOID: the LoD probe never fired in the ON arm. 'No effect' and 'never ran'" >&2
  echo "      are the same evidence without this witness." >&2
  exit 3
fi
wo=$(grep -c "row89 LoD request TICK-GATED" "$D/off-a.log" 2>/dev/null)
[ "$wo" -eq 0 ] || echo "!! WARNING: the witness fired in the OFF arm ($wo) — arms are not separated"
echo

off_m=$(diff <(keys_of "$D/off-a.log") <(keys_of "$D/off-b.log") | wc -l)
on_m=$(diff <(keys_of "$D/on-a.log")  <(keys_of "$D/on-b.log")  | wc -l)
off_s=$(diff <(seq_of "$D/off-a.log") <(seq_of "$D/off-b.log") | grep -c '^[<>]')
on_s=$(diff <(seq_of "$D/on-a.log")  <(seq_of "$D/on-b.log")  | grep -c '^[<>]')

echo "                     membership-diff   schedule-diff"
printf "  OFF (wall clock)   %-17s %s\n" "$off_m" "$off_s"
printf "  ON  (tick gated)   %-17s %s\n" "$on_m" "$on_s"
echo
if [ "$off_s" -eq 0 ]; then
  echo "VERDICT: VOID — the OFF arm did not diverge either, so this run cannot"
  echo "         show a fix. Bar 2's failure is not reproduced here."
elif [ "$on_s" -eq 0 ]; then
  echo "VERDICT: FIXED — schedule identical with the LoD timer tick-gated, and"
  echo "         divergent without it. The client's LoD request timer was a cause"
  echo "         of bar 2's timing failure."
elif [ "$on_s" -lt "$off_s" ]; then
  echo "VERDICT: PARTIAL — $on_s differing vs $off_s. Report both; do NOT round"
  echo "         this to 'fixed'. Something else also drives the schedule."
else
  echo "VERDICT: NO EFFECT — schedule differs in both arms ($on_s vs $off_s)."
  echo "         The LoD timer is EXONERATED. Bar 2's timing cause is still"
  echo "         unnamed, and this eliminates the fourth candidate."
fi
