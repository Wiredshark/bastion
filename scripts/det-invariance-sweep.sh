#!/usr/bin/env bash
# Determinism boost-it-up / break-it invariance sweep.
#
# Runs each bastion-harness determinism scenario under cranked knobs — worker
# count / process order (--schedule-seed), ECS-join / injection order (the
# per-scenario --*-permute-order), and a BIGGER-SCALE / longer config — and
# asserts the emitted FinalStateCertificate is BYTE-IDENTICAL (HOLD) versus BREAK,
# plus non-vacuity (a different --seed must produce a different certificate).
#
# A HOLD means the scenario's authoritative outcome is invariant to the perturbed
# knob; a BREAK prints exactly which knob leaked non-determinism — the isolation
# signal for incremental bisection. Extend by adding a `sweep` line per scenario
# as new *-CERTIFICATE emitters land (col/esim/phy/ter/mf/farm/cavein/gather today).
#
# Usage:  bash scripts/det-invariance-sweep.sh   (from the repo/worktree root)
#   Override the binary with BASTION_HARNESS_BIN=/path/to/bastion-harness.
set -u
BIN="${BASTION_HARNESS_BIN:-./target/verify/bastion-harness.exe}"
[ -f "$BIN" ] || BIN="${BIN%.exe}"
[ -f "$BIN" ] || { echo "NO_BIN: build it first (cargo build --profile verify -p bastion-harness) or set BASTION_HARNESS_BIN"; exit 3; }

cert() { local flag="$1" pfx="$2"; shift 2; "$BIN" "$flag" --seed 1337 "$@" 2>/dev/null | grep -oE "${pfx}: .*" | head -1; }

sweep() { # $1=flag $2=prefix $3=permute("" for none) ; $4..=extra config args
  local flag="$1" pfx="$2" perm="$3"; shift 3
  echo "=== $flag ${*:-baseline} ==="
  local base s5 s12 permd ok=1
  base=$(cert "$flag" "$pfx" "$@")
  [ -n "$base" ] || { echo "  FAIL: no certificate emitted"; return; }
  s5=$(cert "$flag" "$pfx" "$@" --schedule-seed 5)
  s12=$(cert "$flag" "$pfx" "$@" --schedule-seed 12)
  [ "$base" = "$s5" ]  || { echo "  BREAK: --schedule-seed 5";  ok=0; }
  [ "$base" = "$s12" ] || { echo "  BREAK: --schedule-seed 12"; ok=0; }
  if [ -n "$perm" ]; then
    permd=$(cert "$flag" "$pfx" "$@" "$perm")
    [ "$base" = "$permd" ] || { echo "  BREAK: $perm"; ok=0; }
  fi
  [ "$ok" = 1 ] && echo "  HOLD: byte-identical across serial / schedule-seed 5 / 12${perm:+ / $perm}"
  local vac
  vac=$("$BIN" "$flag" --seed 9999 "$@" 2>/dev/null | grep -oE "${pfx}: .*" | head -1)
  if [ -n "$vac" ] && [ "$vac" != "$base" ]; then echo "  NON-VACUOUS: seed 9999 differs"; else echo "  WARNING: seed 9999 same/absent (vacuous?)"; fi
}

echo "########## BASELINE INVARIANCE ##########"
sweep --col-scenario           COL-CERTIFICATE    --col-permute-order
sweep --esim-scenario          ESIM-CERTIFICATE   --esim-permute-order
sweep --phy-scenario           PHY-CERTIFICATE    --phy-permute-order
sweep --ter-scenario           TER-CERTIFICATE    --ter-permute-order
sweep --mine-fidelity-scenario MF-CERTIFICATE     ""
sweep --farm-scenario          FARM-CERTIFICATE   ""
sweep --cavein-scenario        CAVEIN-CERTIFICATE ""
sweep --gather-scenario        GATHER-CERTIFICATE ""

echo "########## BIGGER-SCALE / LONGER STRESS ##########"
sweep --col-scenario  COL-CERTIFICATE  --col-permute-order  --colony 8 --col-arb-rounds 6
sweep --phy-scenario  PHY-CERTIFICATE  --phy-permute-order  --phy-grid 16 --phy-ticks 200
sweep --esim-scenario ESIM-CERTIFICATE --esim-permute-order --esim-reports 24 --esim-ticks 400
echo "########## sweep done ##########"
