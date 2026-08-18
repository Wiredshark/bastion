#!/usr/bin/env bash
# SWEEP EVERY SCENARIO THE HARNESS EXPOSES — the whole testable surface in one
# pass, at max local concurrency.
#
# WHY THIS EXISTS. `vm-pool.sh` fans SEEDS of ONE scenario across VMs; it takes a
# single `ARGS` string and cannot vary the scenario per seed. So it answers
# "how does scenario X behave across 96 seeds" and can never answer "which of
# the 75 scenarios are green right now". The build list is tested by the SECOND
# question, and nothing was asking it.
#
# ★ THE SCENARIO LIST IS DERIVED, NEVER HARDCODED, so a scenario added tomorrow
# is swept tomorrow. A hardcoded list is a claim about the harness that rots
# silently — this repo has already paid for one of those today (a pin whose
# prose described code it had never matched).
#
# ⚠ AND THE DERIVATION IS DELIBERATELY OVER-BROAD — corrected 2026-08-17, having
# first claimed it read "the flag declarations". IT DOES NOT. It greps EVERY
# identifier matching `*_scenario|_fixture|_model|_probe` anywhere in main.rs,
# which also catches function names, match arms and prose. First run: 122 names,
# against ~75 actual `bool` flags.
#
# ★★ THAT IS KEPT, NOT FIXED, AND THE REASON IS THE FOUR-STATE DETECTOR. A name
# that is not a flag dies instantly at clap and lands in ERROR — which by design
# cannot collapse into FAIL. So over-broad costs a few fast failures and CANNOT
# manufacture a red, while over-narrow would silently skip a real scenario and
# call the surface covered. ★ Given a choice between a list that over-reports
# into a harmless bucket and one that under-reports into silence, take the first:
# the run then QUANTIFIES its own imprecision (122 − valid = ERROR).
#
# ★★ PASS/FAIL IS DETECTED GENERICALLY, AND "NO VERDICT" IS ITS OWN ANSWER.
# Scenarios print their own marker (`B5.8 SCENARIO: PASS`, `LOD0 SCENARIO:
# FAIL`, `M2-LADDER-EPISODE 3: PASS`, ...) — there is no single string. So the
# detector greps for `: PASS` / `: FAIL` and reports THREE states:
#   PASS    — a PASS marker and no FAIL marker
#   FAIL    — any FAIL marker
#   NOVERDICT — the run finished but printed neither
#   ERROR   — non-zero exit (needs an arg, panicked, unknown flag)
# ★★★ NOVERDICT MUST NOT COLLAPSE INTO FAIL. A scenario that needs an extra
# argument (`--ladder-episode`) or that has no verdict line is NOT a failing
# scenario, and counting it as one would manufacture a red. An absence and a
# refusal must never render identically.
#
# Usage: bash sweep-all-scenarios.sh [SEED] [JOBS] [TICKS]
set -u
WT=/e/veloren-master/.engine-integration-wt
SEED="${1:-4242}"; JOBS="${2:-8}"; TICKS="${3:-3000}"
OUT="/tmp/sweep-$SEED"; mkdir -p "$OUT"
# .exe on this host, bare elsewhere — resolve rather than assume. Getting this
# wrong makes the refusal below fire on a binary that is present, which is the
# most annoying kind of false negative: it looks like "not built yet".
BIN="$WT/target/verify/bastion-harness.exe"
[ -x "$BIN" ] || BIN="$WT/target/verify/bastion-harness"

# PROVENANCE FIRST, as every scored thing here does.
echo "=== SCENARIO SWEEP $(date '+%Y-%m-%d %H:%M:%S') ==="
echo "HEAD        : $(git -C "$WT" rev-parse --short HEAD)"
echo "dirty .rs   : $(git -C "$WT" status --short -- '*.rs' | wc -l)"
[ -x "$BIN" ] || { echo "!! no harness at $BIN — build with: cargo build --profile verify -p bastion-harness"; exit 1; }
echo "binary      : $(date -r "$BIN" '+%Y-%m-%d %H:%M:%S')"
# ★★★ THE CLASSIFIER'S OWN IDENTITY. Added 2026-08-17 after a three-seed
# comparison where ~50 of ~59 "differences" were THIS SCRIPT changing, not the
# engine: seed 1337 ran a run_one() that split ERROR into four states, so every
# ERROR->NOTAFLAG row looked like seed sensitivity and would have inflated the
# finding roughly tenfold.
# ★★ A SWEEP COMPARISON MUST HOLD THE CLASSIFIER FIXED, NOT JUST THE BINARY —
# and it cannot, unless the classifier SAYS WHICH ONE IT IS. Provenance that
# covers the subject but not the instrument is half a provenance.
_SELF="${BASH_SOURCE[0]}"
echo "classifier  : $(git -C "$(dirname "$_SELF")" log -1 --format=%h -- "$(basename "$_SELF")" 2>/dev/null || echo unknown) \
($(git -C "$(dirname "$_SELF")" status --short -- "$(basename "$_SELF")" 2>/dev/null | grep -q . && echo DIRTY || echo clean))"
echo "seed=$SEED jobs=$JOBS ticks=$TICKS"

# THE LIST, DERIVED FROM THE SOURCE.
SCEN=$(grep -oE "[a-z0-9_]+_(scenario|fixture|model|probe)" "$WT/bastion-harness/src/main.rs" \
       | sort -u \
       | grep -vE "^(b58_paired|b5_rowb_paired)$")
N=$(echo "$SCEN" | wc -l)
echo "scenarios   : $N (derived from main.rs, not hardcoded)"
echo

run_one() {
  flag="--$(echo "$1" | tr '_' '-')"
  log="$OUT/$1.log"
  timeout 600 "$BIN" "$flag" --seed "$SEED" --ticks "$TICKS" --tps 30 \
      --data-dir "/tmp/sw-$1-$SEED" > "$log" 2>&1
  rc=$?
  # ★★★ ANSI FIRST, ALWAYS. `tracing` colours the FIELD NAMES, so the bytes on
  # disk are `\x1b[3mconsidered\x1b[0m=0` and a plain `grep "considered="` finds
  # NOTHING. A zero-match grep and a real zero render IDENTICALLY, and that cost
  # a full triage today: 23 of 32 logs carried a census I reported as absent
  # from all 32. Strip once, classify off the stripped text.
  txt=$(sed -e 's/\x1b\[[0-9;]*m//g' "$log")

  # ★★★ THE OLD `ERROR` STATE COLLAPSED FOUR DIFFERENT THINGS INTO ONE WORD, in
  # a script whose own header says an absence and a refusal must never render
  # identically. Measured over the first sweep's 50 ERRORs: 44 were not flags at
  # all, 3 were real flags missing a required argument, 2 were PREMISE-UNMET
  # REFUSALS, 1 was a probe signalling by exit code — and ZERO were defects.
  # ★★ The refusals are the ones that matter: `col`/`colneed` need the ECS join
  # order to DIFFER from Uid order or an order-dependence bug is undetectable,
  # and when the premise fails they say so loudly instead of passing vacuously.
  # Counting THAT as an error punishes the single behaviour most worth having.
  if echo "$txt" | grep -q "premise unmet"; then
    echo "REFUSED   $1 (premise unmet — NOT a failure)"
  elif echo "$txt" | grep -q "unexpected argument"; then
    echo "NOTAFLAG  $1"
  elif echo "$txt" | grep -q "but none was supplied"; then
    echo "NEEDSARG  $1"
  elif echo "$txt" | grep -qE ": FAIL"; then
    echo "FAIL      $1"
  elif echo "$txt" | grep -qE ": PASS"; then
    echo "PASS      $1"
  elif [ $rc -ne 0 ]; then
    # ★ Ran, produced no verdict marker, exited non-zero. `*_probe` names signal
    # by EXIT CODE and never print a marker — a third contract, distinct from
    # both a self-judging scenario and an emitting fixture.
    echo "ERROR     $1 (exit $rc)"
  else
    echo "NOVERDICT $1"
  fi
}
export -f run_one; export BIN OUT SEED TICKS

echo "$SCEN" | xargs -P "$JOBS" -I{} bash -c 'run_one "$@"' _ {} | sort | tee "$OUT/RESULTS.txt"

echo
# ★★★ TALLIED BY CONTRACT, BECAUSE A FIXTURE'S "PASS" AND A SCENARIO'S "PASS"
# ARE NOT THE SAME EVENT AND MUST NEVER SUM (added 2026-08-17, after the first
# sweep's greens were audited):
#   *_scenario  SELF-JUDGES — computes `pass` and prints its own verdict.
#   *_fixture   EMITS       — writes an observation; the comparison happens
#                             LATER, across runs, in the wave tooling. Its
#                             "PASS" means THE EMITTER RAN.
# ★ The first sweep summed them and reported 21 PASS / 32 FAIL. Split, the
# self-judging population was 17 PASS / 30 FAIL — 64% red, not the ~40% that
# went out. ★★ The conflation flattered the result, which is exactly why the
# GREENS are worth auditing: a false red gets investigated and a false green
# never does.
echo "=== TALLY BY CONTRACT (denominator = $N names, one seed each) ==="
printf "  %-10s %6s %10s %9s %7s\n" "" "total" "*_scenario" "*_fixture" "other"
# ⚠ NO `|| echo 0` HERE, and that is deliberate. `grep -c` ALREADY PRINTS `0`
# when it matches nothing — and it also EXITS 1, so `$(grep -c ... || echo 0)`
# prints "0" and then appends another "0", yielding the two-line string "0\n0".
# ★ `$((tot-sc-fx))` then dies with `syntax error in expression (error token is
# "0")` and the whole tally row vanishes. First run of this tally lost the
# REFUSED / NEEDSARG / NOTAFLAG rows exactly that way — ★★ the states I had just
# added to stop an absence from rendering as something else were themselves
# erased by a zero-match idiom.
for k in PASS FAIL NOVERDICT REFUSED NEEDSARG NOTAFLAG ERROR; do
  tot=$(grep -c "^$k " "$OUT/RESULTS.txt" 2>/dev/null)
  sc=$(grep "^$k " "$OUT/RESULTS.txt" 2>/dev/null | awk '{print $2}' | grep -c "_scenario$")
  fx=$(grep "^$k " "$OUT/RESULTS.txt" 2>/dev/null | awk '{print $2}' | grep -c "_fixture$")
  printf "  %-10s %6s %10s %9s %7s\n" "$k" "${tot:-0}" "${sc:-0}" "${fx:-0}" "$(( ${tot:-0} - ${sc:-0} - ${fx:-0} ))"
done
# ★ The headline rate is quoted over SELF-JUDGING scenarios only. A fixture that
# ran cannot be evidence that a check passed.
sp=$(grep "^PASS " "$OUT/RESULTS.txt" 2>/dev/null | awk '{print $2}' | grep -c "_scenario$" || echo 0)
sf=$(grep "^FAIL " "$OUT/RESULTS.txt" 2>/dev/null | awk '{print $2}' | grep -c "_scenario$" || echo 0)
if [ $((sp+sf)) -gt 0 ]; then
  awk -v p=$sp -v f=$sf 'BEGIN{printf "\n  ★ SELF-JUDGING SCENARIOS WITH A VERDICT: %d PASS / %d FAIL = %.1f%% green\n", p, f, 100*p/(p+f)}'
else
  echo "  ★ NO self-judging scenario returned a verdict — no rate is quoted (n=0)."
fi
echo
echo "★ ONE SEED EACH — this is a BREADTH sweep, not a rate. It answers 'what is"
echo "  green right now', never 'how often'. A scenario that passes here can still"
echo "  fail 30% of seeds; that is what the vm-pool fans measure, per scenario."
echo "SWEEP-COMPLETE: $OUT/RESULTS.txt"
