#!/usr/bin/env bash
# ITEM 11 A/B SCORER -- WRITTEN AND COMMITTED BEFORE EITHER ARM'S DATA EXISTS.
#
# ★ WHY THAT MATTERS: a scorer written after seeing the numbers can be shaped to
# them without anyone (including me) noticing. The bars come from
# ITEM11-RESTORE-PREREGISTRATION.md + AMENDMENT-1 and are transcribed here, not
# invented here.
#
# Usage: bash score-recr-ab.sh <TAG>        e.g. recrab | recrabctl
set -u
EV="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAG="${1:?usage: score-recr-ab.sh <TAG>}"
DRV="$EV/driverout-pit-$TAG.log"
SRV="$EV/server-pit-$TAG.log"
ATT="$EV/pit-$TAG-attest.txt"

echo "=== ITEM 11 A/B SCORE: $TAG ==="

# ---------------------------------------------------------------- PRECONDITION
# ★ THE LAST ATTESTATION BLOCK, NEVER THE FIRST. The file APPENDS, and a failed
# early attempt sits at the head -- the calibration's own file opens with
# "ATTESTATION FAILED: STALE". A VOID run and a RED run look identical unless the
# precondition is printed ABOVE the result, so it is printed first and the
# scorer REFUSES rather than scoring past a failure.
if [ -f "$ATT" ]; then
  echo "--- PRECONDITION (last attestation block) ---"
  awk '/=== RUN ATTESTATION/{buf=""} {buf=buf $0 "\n"} END{printf "%s", buf}' "$ATT" \
    | grep -E "RUN ATTESTATION|HEAD|dirty|run config|ATTESTATION FAILED|STALE|fresh:" | sed 's/^/  /'
  if awk '/=== RUN ATTESTATION/{buf=""} {buf=buf $0 "\n"} END{printf "%s", buf}' "$ATT" | grep -q "ATTESTATION FAILED"; then
    echo "  !! LAST ATTESTATION FAILED -> RUN IS VOID. Not scoring."; exit 3
  fi
else
  echo "  !! NO ATTESTATION FILE -> VOID (no provenance, no score)."; exit 3
fi

[ -f "$DRV" ] || { echo "!! no $DRV -> the arm produced no client output. VOID, not a result."; exit 3; }

# --------------------------------------------------------------- SAMPLE PARSE
# ★ INSPECT lines precede their `note RECR-AB SAMPLE N` label -- the script emits
# the inspect FIRST. Buffer and flush on the note. This exact off-by-one lost 8
# of 40 lines when scoring the calibration; the leftover count is printed BECAUSE
# that is what caught it.
TMP=$(mktemp)
awk '
/^INSPECT/ {
  if (match($0,/uid=[0-9]+/))         { u=substr($0,RSTART+4,RLENGTH-4) }
  if (match($0,/recreation=[0-9.]+/)) { r=substr($0,RSTART+11,RLENGTH-11)+0; n++; bu[n]=u; br[n]=r }
}
/RECR-AB SAMPLE [0-9]+/ {
  if (n>0) { s=substr($0, index($0,"SAMPLE")+7); sub(/[^0-9].*/,"",s)
             for(i=1;i<=n;i++) print s, bu[i], br[i]
             n=0 }
}
END { print "LEFTOVER", n+0, 0 }
' "$DRV" > "$TMP"

LEFT=$(awk '$1=="LEFTOVER"{print $2}' "$TMP")
NS=$(awk '$1!="LEFTOVER"{print $1}' "$TMP" | sort -un | wc -l)
NROWS=$(awk '$1!="LEFTOVER"' "$TMP" | wc -l)
echo "--- PARSE ---"
echo "  samples: $NS   rows: $NROWS   leftover unattributed INSPECT lines: $LEFT"
if [ "$NROWS" -eq 0 ]; then echo "  !! n=0 -> NO RATE OR VERDICT QUOTED. VOID."; rm -f "$TMP"; exit 3; fi
[ "$LEFT" -gt 0 ] && echo "  !! $LEFT INSPECT lines unattributed -- parse is INCOMPLETE, treat verdicts as provisional"

# ------------------------------------------------------------------- THE BARS
echo "--- MONOTONICITY (per colonist, across samples in order) ---"
awk '$1!="LEFTOVER"{ key=$2; s=$1+0; r=$3+0
       if (!(key in seen) || s>last_s[key]) { }
       rec[key,s]=r; if (!(key in seen)) { seen[key]=1; uids[++nu]=key }
       if (s>maxs) maxs=s }
     END{
       rises=0; falls=0; flat=0
       for (i=1;i<=nu;i++) { u=uids[i]; prev=-1
         for (s=0;s<=maxs;s++) if ((u,s) in rec) {
           v=rec[u,s]
           if (prev>=0) { if (v>prev+1e-9) { rises++; printf "  RISE  uid=%s sample %d: %.4f -> %.4f\n", u, s, prev, v }
                          else if (v<prev-1e-9) falls++; else flat++ }
           prev=v } }
       printf "  totals: rises=%d falls=%d flat=%d  (colonists=%d)\n", rises, falls, flat, nu
       print  "RISES=" rises > "/dev/stderr"
     }' "$TMP" 2> "$TMP.r"
RISES=$(sed 's/RISES=//' "$TMP.r" 2>/dev/null | tail -1)

echo "--- BAR 3: the witness reaches someone (server emit) ---"
if [ -f "$SRV" ]; then
  W=$(grep -c "recreation restore applied" "$SRV" 2>/dev/null || true)
  echo "  'recreation restore applied' lines: ${W:-0}"
  grep -oE "colonists_on_break=[0-9]+" "$SRV" 2>/dev/null | sort | uniq -c | sed 's/^/    /' | head -6
  MAXN=$(grep -oE "colonists_on_break=[0-9]+" "$SRV" 2>/dev/null | cut -d= -f2 | sort -rn | head -1)
  echo "  max colonists_on_break: ${MAXN:-0}"
else
  echo "  !! no server log -> bar 3 UNMEASURED (not failed)"
fi

echo "--- VERDICT (bars transcribed from the preregistration) ---"
case "$TAG" in
  *ctl) echo "  CONTROL expectation: recreation strictly NON-INCREASING => rises must be 0"
        [ "${RISES:-0}" -eq 0 ] && echo "  => CONTROL HOLDS (ratchet is one-way with the flag off)" \
                                || echo "  => *** CONTROL ROSE ($RISES times). A/B IS VOID and the rise is the finding. ***" ;;
  *)    echo "  TREATMENT expectation: at least one rise (secondary bar), and colonists_on_break >= 1 (bar 3)"
        [ "${RISES:-0}" -gt 0 ] && echo "  => SECONDARY BAR MET: $RISES rises. NOT yet net-across-a-break -- that needs the server-emit window." \
                                || echo "  => NO RISE. The restore did not outpace decay in any sampled interval." ;;
esac
echo "  ⚠ PRIMARY BAR (net across a whole break) is scored by hand against the"
echo "    tick-stamped server emit window; a sampled rise is the WEAKER statement."
rm -f "$TMP" "$TMP.r"
