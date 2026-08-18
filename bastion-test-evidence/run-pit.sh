#!/usr/bin/env bash
# THE VERTICAL FIXTURE: does the colony reach `RestingToClimb`?
#
# Four arms from ONE binary:
#   pit     -- the mine outcrop sits on the floor of a 4-deep excavation
#   flat    -- V3's matched control; `status` must read None for every colonist
#   pitdiag -- `pit` plus BASTION_ACCESS_CLAIM_DIAG, which un-silences the
#              transition-gated `bastion F3-BRANCH` line. NOT a control for
#              `flat`: it changes a second variable, and that is declared.
#   pitwood -- `pitdiag` plus a stockpiled wood drop, which clears the material
#              STARVATION hold. `material_held` means material is MISSING, not
#              possessed (F3-BRANCH-LIVE-AMENDMENT-1) -- so supplying wood is
#              what can put branch B on the board, not withholding it.
#
# The arm/control split is an ENV VAR, not a second build, so a difference
# cannot be a compilation difference. That is the same "one definition"
# discipline the config attestation rests on.
#
# WHY A PIT: `RestingToClimb` is written inside the emergency-egress
# machinery (`grounded_clear && !route_energy_ready`), not by climbing. The
# colonist must be STUCK on an escape route with depleted energy -- the
# status surface's own doc calls these "the four indistinguishable PIT
# states". See VERTICAL-FIXTURE-AMENDMENT-1.md.
#
# Usage:  run-pit.sh <pit|flat|pitdiag|pitwood|pitnowood|pitwood2|pitnowood2|pitwood3|pitplant|pitread|sweep|stamp|shaft|contend|queuewait|flee|fleecost|fleehealth|fleehealthctl|wallctl|wall|recr|recrctl|shaftwolf|hostile|wall2ctl|wall2|wallcheck|wallcheckctl|wallsurvey|wallsurveyctl|wallsurvey2|wallsurvey2ctl|wallverdict|wallverdictctl>
set -u
# ── ROOTS ARE OVERRIDABLE SO THIS SCRIPT CAN LEAVE THIS MACHINE ──────────────
# These were hardcoded Windows paths, which is WHY every pit leg to date ran
# locally at n=1: the harness could not physically execute on a Linux VM. The
# defaults keep every existing local invocation byte-identical; a VM sets the
# four env vars and nothing else changes.
WT="${PIT_WT:-/e/veloren-master/.engine-integration-wt}"
EV="${PIT_EV:-/e/veloren-master/bastion-test-evidence}"
B="${PIT_B:-$WT/target/no_overflow}"
A="${PIT_A:-E:/veloren-master/.engine-integration-wt/assets}"
# Windows builds emit .exe; Linux does not. One definition, used everywhere.
EXE="${PIT_EXE-.exe}"
ARM="${1:?usage: run-pit.sh <pit|flat|pitdiag|pitwood|pitnowood|pitwood2|pitnowood2|pitwood3|pitplant|pitread|sweep|stamp|shaft|contend|queuewait|flee|fleecost|fleehealth|fleehealthctl|wallctl|wall|recr|recrctl|shaftwolf|hostile|wall2ctl|wall2|wallcheck|wallcheckctl|wallsurvey|wallsurveyctl|wallsurvey2|wallsurvey2ctl|wallverdict|wallverdictctl>}"
TAG="pit-$ARM"; GAME=26024; WEB=26025; METRICS=18026
UD="E:/veloren-master/.engine-integration-wt/userdata-$TAG"

# THE SINGLE DEFINITION -- recorded by the attestation and applied to the
# server from the same string, so the evidence file cannot disagree with
# what ran.
# Arms: `pit` and `flat` are the original A/B and are left BYTE-IDENTICAL, so
# the pit row's evidence stays reproducible. `pitdiag` adds ONE further
# variable -- BASTION_ACCESS_CLAIM_DIAG, which un-silences the transition-gated
# `bastion F3-BRANCH` line -- and is therefore NOT a control for `flat`: it is
# the pit arm re-run with an extra observer, and the observer is declared.
PITVAR=""; SCRIPT="script-pit.txt"
case "$ARM" in
  pit)     PITVAR=" BASTION_FLAT_ARENA_PIT=1" ;;
  pitdiag) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1" ;;
  # pitwood: pitdiag PLUS a stockpiled wood drop, to clear the material
  # STARVATION hold (material_held) so the F3 chain can select branch B.
  # Its own script, because the drop needs a stockpile designated first.
  pitwood) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
           SCRIPT="script-pitwood.txt" ;;
  # pitnowood: pitwood's MATCHED CONTROL. Identical env, identical script
  # except the two `cmd` lines that deliver wood. One axis -- wood present or
  # absent -- so a branch=B difference is attributable to the wood and not to
  # the stockpile designation, the count_items calls, or the sample cadence,
  # all of which pitdiag also lacked. See WOOD-CONTROL-PREREG.md.
  pitnowood) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
             SCRIPT="script-pitnowood.txt" ;;
  # REPLICATION arms (REPLICATION-PREREG.md). Identical env and identical
  # script to their base arm -- the ONLY difference is the boot, which is the
  # whole point. Distinct TAG gives a fresh userdata dir and fresh logs; they
  # run SEQUENTIALLY on the same ports so the replication does not differ from
  # the original pair by port assignment as well.
  pitwood2) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
            SCRIPT="script-pitwood.txt" ;;
  pitnowood2) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
              SCRIPT="script-pitnowood.txt" ;;
  # pitwood3: the COLONY-TICK leg. Same config and script as pitwood; the only
  # difference is the BINARIES, which now stamp the colony payload with the
  # server Tick. A new tag so the earlier arms' committed evidence is not
  # overwritten -- their payloads have no tick and their dispositions stand.
  pitwood3) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
            SCRIPT="script-pitwood.txt" ;;
  # pitplant: the REGISTERED PLANT leg for T1. Identical to pitwood3; the
  # server binary under it fills the colony tick with 0 instead of the real
  # Tick. T1's monotonicity check must go RED while the F3 lines are
  # unchanged -- isolating the new field from the emit it aligns against.
  pitplant) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
            SCRIPT="script-pitwood.txt" ;;
  # pitread: the NEXT-TICK READBACK leg (READBACK-PREREG.md). pitdiag's env
  # plus exactly one new variable -- BASTION_MINE_READBACK_DIAG -- so a
  # readback line appearing here and not in pitdiag is attributable to the
  # flag and nothing else, the same one-axis discipline pitdiag itself used.
  pitread) PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1 BASTION_MINE_READBACK_DIAG=1" ;;
  # sweep: THE UNEXERCISED-FIELD SWEEP (FIELD-SWEEP-PREREG.md). Same env as
  # pitdiag; the whole difference is the SCRIPT, which drops wood then
  # mushrooms into one stockpile and samples the colony across both.
  sweep)   PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1"
           SCRIPT="script-sweep.txt" ;;
  # stamp: THE STAMP EMIT (STAMP-EMIT-PREREG.md). pitdiag's env plus exactly
  # one new variable, so a `status stamp` line here and not in pitdiag is
  # attributable to the flag alone.
  stamp)   PITVAR=" BASTION_FLAT_ARENA_PIT=1 BASTION_ACCESS_CLAIM_DIAG=1 BASTION_STATUS_STAMP_DIAG=1" ;;
  # shaft: THE SHAFT FIXTURE (SHAFT-FIXTURE-PREREG.md). The pit CANNOT trap
  # (EGRESS-ENTRY-READ.md: 4 deep is inside the scan's -4 hop-down band, and
  # 10 across is past its >7 evasion limit), so this arm swaps the geometry
  # for one the detector reads as trapped: 8 deep, 3 across, single-column
  # outcrop. Carries the stamp-emit flag, whose zero in the pit is the OFF
  # control this arm is the ON side of.
  # AMENDMENT 1: its own script. script-pit.txt's mine box is hardcoded to
  # the PIT floor (z 395..399), which sits ABOVE the shaft's outcrop
  # (392..394) -- the first shaft leg designated AIR and its zeros were VOID.
  shaft)   PITVAR=" BASTION_FLAT_ARENA_SHAFT=1 BASTION_ACCESS_CLAIM_DIAG=1 BASTION_STATUS_STAMP_DIAG=1 BASTION_EGRESS_DIAG=1"
           SCRIPT="script-shaft.txt" ;;
  # contend: LADDER CONTENTION (LADDER-CONTENTION-PREREG.md). Identical env
  # to `shaft` -- the ONLY difference is the script, which widens the mine
  # box across the shaft floor and walls so several colonists need the one
  # route out at once. One axis, so a WaitingForLadder sighting here and not
  # in `shaft` is attributable to the work distribution.
  contend) PITVAR=" BASTION_FLAT_ARENA_SHAFT=1 BASTION_ACCESS_CLAIM_DIAG=1 BASTION_STATUS_STAMP_DIAG=1 BASTION_EGRESS_DIAG=1"
           SCRIPT="script-contend.txt" ;;
  # queuewait: M3's LAST ATTEMPT (STILLNESS-WATCH-RESULTS.md). Six arms have
  # produced six zero stamp counts; WaitingForLadder arrived by the PHASE
  # route, never the stamp. The queue-wait stamp site needs a colonist
  # INSIDE its budget, and the site's own author left a TEST-ONLY override
  # for exactly this -- BASTION_M3_QUEUE_WAIT_BUDGET_TICKS, read once,
  # never set by live binaries. Shaft env plus that one knob, raised so a
  # queued colonist stays within budget long enough to be stamped.
  queuewait) PITVAR=" BASTION_FLAT_ARENA_SHAFT=1 BASTION_ACCESS_CLAIM_DIAG=1 BASTION_STATUS_STAMP_DIAG=1 BASTION_EGRESS_DIAG=1 BASTION_M3_QUEUE_WAIT_BUDGET_TICKS=100000"
             SCRIPT="script-shaft.txt" ;;
  # flee: ITEM 13 -- hostiles near the colony, the Flee drive's live test.
  # Flat arena (no pit): the question is a hostile's effect on DRIVE, and a
  # pit would add a traversal variable the bar does not need. Its own
  # script spawns an enemy mid-run with two clean baselines first.
  flee)    PITVAR=" BASTION_ACCESS_CLAIM_DIAG=1"
           SCRIPT="script-flee.txt" ;;
  # fleecost: ITEM 13b -- what fleeing COSTS. Same env as `flee`; the whole
  # difference is the script, which designates mine work and lets a rhythm
  # build BEFORE the hostile arrives, so completions can be compared across
  # three windows (peaceful / flee / recovery) inside one run.
  fleecost) PITVAR=" BASTION_ACCESS_CLAIM_DIAG=1"
            SCRIPT="script-fleecost.txt" ;;
  # fleehealth: ITEM 13c -- flee_sig's HEALTH branch, the half item 13 could
  # not reach. Spawns NOTHING: a hostile sets the TARGET branch, so damage
  # dealt by one arrives with branch A already true and the two become
  # unattributable. Damage via /sudo <uid> health <hp>, no hostile in world.
  # ITEM 15a control: resourced arena, NO wall. This arm is the PREMISE --
  # if no by_health emit appears here, nothing was ever kept out and the
  # walled arm cannot be scored.
  # ITEM 11 restore: recreation crosses comfort only after 3000 SIM-seconds
  # (decay 0.0002, comfort 0.4), so 'recr' is a CALIBRATION arm that measures
  # how much sim-time a client-tick window buys before the A/B is sized.
  recr)    PITVAR=" BASTION_RECREATION=1"
              SCRIPT="script-recr.txt" ;;
  # Matched control: identical script, restore+preempt flag OFF.
  recrctl) PITVAR=""
              SCRIPT="script-recr.txt" ;;
  # ITEM 11 A/B, SIZED BY THE CALIBRATION ABOVE (0.035 sim-s per client tick,
  # ITEM11-CALIBRATION-RESULTS.md): ~84,000 ticks to reach comfort, +3,430 for
  # one full RECREATION_BREAK_SECS, so the script runs 92,000 with a dense
  # 1,000-tick phase over the crossing. ~22 wall-minutes per arm.
  # ★ The A/B works through the JOB, not a second gate: the restore at
  # bastion_jobs.rs applies to any holder of a Recreate job, and Recreate jobs
  # have exactly ONE creation site, reached only via the preempt that
  # recreation_enabled() gates. Flag off => no Recreate job => restore never
  # applies => recreation strictly non-increasing. That is the control.
  recrab)  PITVAR=" BASTION_RECREATION=1"
              SCRIPT="script-recr-ab.txt" ;;
  recrabctl) PITVAR=""
              SCRIPT="script-recr-ab.txt" ;;
  # ARC 3 SHARED BLOCKER (unblocks items 14 AND 15): the SHAFT removes the
  # colonist's escape rather than adding force, so the hostile is identical to
  # item 15a's and exactly one variable changes -- can it flee?
  # BASTION_FOUNDING_NO_FARM: the ARC 3 blocker's ACTUAL fix. Four scripted
  # attempts proved `cancel` cannot touch founding work -- it never enters the
  # designation board -- so the farm is suppressed at its SOURCE instead.
  shaftwolf) PITVAR=" BASTION_FLAT_ARENA_SHAFT=1 BASTION_FOUNDING_NO_FARM=1"
              SCRIPT="script-shaftwolf.txt" ;;
  # #93 THE HOSTILE PROXIMITY CENSUS. The PLAINEST fixture available -- flat,
  # resourced, no pit/shaft/wall -- because every geometric variable that voided
  # an earlier ARC 3 attempt is a variable this row does not need. The only
  # event is the spawn, and the only new thing is that the HOSTILE is measured.
  hostileplant|hostileplant2|hostilerepro) PITVAR=" BASTION_HOSTILE_PROXIMITY_DIAG=1"
              SCRIPT="script-hostile-plant.txt" ;;
  hostile) PITVAR=" BASTION_HOSTILE_PROXIMITY_DIAG=1"
              SCRIPT="script-hostile.txt" ;;
  # ITEM 15 (ITEM15-WALL-EFFECT-PREREGISTRATION). wall2ctl/wall2 replace the
  # 15a pair: same fixture, but the driver goto-s OUTSIDE the ring before the
  # spawn, because /spawn places the hostile AT THE DRIVER and the driver stands
  # at the ring's CENTRE -- 15a would have spawned the wolf inside the wall.
  # Both carry the proximity census, which is the harm witness (by_health needs
  # a 60-70% loss and real bites produce 22%).
  # ITEM 15 wall-existence check (AMENDMENT 1). Two arms, identical script,
  # differing only in the wall flag -- so 'solid here / air there' is
  # attributable to the flag and nothing else.
  # TAKE 2, with an instrument that can answer: inspect_cell reads DESIGNATIONS,
  # not terrain. `survey` scans terrain columns and reports how many had no
  # surface in the z-range -- so a window containing only the wall's courses
  # turns "is the wall there" into a count.
  # TAKE 3: adds a POSITIVE control window (z[395,405] must contain the slab),
  # because take 2's within-arm control was negative-only and so could not
  # distinguish "no wall" from "survey blind".
  # TAKE 4: one self-contained leg carrying BOTH windows AND its own provenance.
  # Every earlier take failed on provenance, not on the question -- four legs
  # talked to an orphan and a fifth's server had not finished booting. The gate
  # emit must appear in the SAME log as the surveys, or the reading is refused.
  wallverdict) PITVAR=" BASTION_FLAT_ARENA_WALLED=1"
              SCRIPT="script-wallverdict.txt" ;;
  wallverdictctl) PITVAR=""
              SCRIPT="script-wallverdict.txt" ;;
  wallsurvey2) PITVAR=" BASTION_FLAT_ARENA_WALLED=1"
              SCRIPT="script-wallsurvey2.txt" ;;
  wallsurvey2ctl) PITVAR=""
              SCRIPT="script-wallsurvey2.txt" ;;
  wallsurvey) PITVAR=" BASTION_FLAT_ARENA_WALLED=1"
              SCRIPT="script-wallsurvey.txt" ;;
  wallsurveyctl) PITVAR=""
              SCRIPT="script-wallsurvey.txt" ;;
  wallcheck) PITVAR=" BASTION_FLAT_ARENA_WALLED=1"
              SCRIPT="script-wallcheck.txt" ;;
  wallcheckctl) PITVAR=""
              SCRIPT="script-wallcheck.txt" ;;
  wall2ctl) PITVAR=" BASTION_HOSTILE_PROXIMITY_DIAG=1"
              SCRIPT="script-wall2.txt" ;;
  # ITEM 14 v1: measures the LIVE per-colonist hold-threshold spread. No env
  # flag -- the parameterization already exists in stagger_interrupt and needs
  # no gate; what it needed was 16 colonists, sized so an all-identical draw is
  # ~1% instead of the ~10% that made the 8-colonist read ambiguous.
  guardspread) PITVAR=""
              SCRIPT="script-guardspread.txt" ;;
  # TICK-LOADING ROW, measurement arm. UNCAPPED, real-time, live drain -- this
  # is the baseline whose promotions-per-tick distribution the budget must be
  # DERIVED from. No cap, no compression, no deterministic gate: changing any
  # of those before the baseline exists would mean deriving the budget from a
  # population the live server never runs.
  provbase)   PITVAR=" BASTION_TERRAIN_PROVISION_DIAG=1"
              # ★ CLEARS PITDET -- this is the ONE arm that must take the LIVE
              # free-running drain. Take 1 asserted this in a comment and got
              # deterministic_drain=true on all 3763 ticks; now it is expressed
              # in the config surface, where it can actually take effect.
              PITDET=""
              SCRIPT="script-guardspread.txt" ;;
  # TICK-LOADING BASELINE take 3: free-running drain AND real terrain AND a
  # moving observer. Take 2 had the first only and was VOID at 6/3382 working
  # ticks -- a stationary colony in a pre-generated arena requests no chunks.
  provtrav)   PITVAR=" BASTION_TERRAIN_PROVISION_DIAG=1"
              PITDET=""      # live free-running drain (as provbase)
              PITARENA=""    # REAL TERRAIN -- there must be something to generate
              SCRIPT="script-provtraverse.txt" ;;
  # A1's CAPPED arm. IDENTICAL to provtrav except the deterministic barrier is
  # ON -- which is the only path where DETERMINISTIC_PROMOTION_BUDGET applies.
  # Same script, same terrain, same traversal: the ONLY difference between the
  # arms is the drain. That is what makes the distributions comparable; a
  # matched control has to match on everything except the axis under test, and
  # this row has already been burned once by an arm that differed in a way its
  # own comment denied.
  provtravdet) PITVAR=" BASTION_TERRAIN_PROVISION_DIAG=1"
              PITARENA=""    # REAL TERRAIN (PITDET left at its default = ON)
              SCRIPT="script-provtraverse.txt" ;;
  # A2 PROPER: identical to provtravdet EXCEPT the tick rate is CAPPED, so a
  # fixed number of client ticks maps to a fixed number of server ticks and the
  # tick axis means the same thing in every run. Without this the comparison
  # indexes two timelines running at different rates -- which is exactly how I
  # produced a false "loading is not deterministic" verdict.
  provtravcap) PITVAR=" BASTION_TERRAIN_PROVISION_DIAG=1"
              PITARENA=""    # real terrain
              PITTPS=""      # CAPPED: the tick axis is now comparable across runs
              SCRIPT="script-provtraverse.txt" ;;
  wall2)   PITVAR=" BASTION_FLAT_ARENA_WALLED=1 BASTION_HOSTILE_PROXIMITY_DIAG=1"
              SCRIPT="script-wall2.txt" ;;
  wallctl) PITVAR=""
              SCRIPT="script-wall.txt" ;;
  # ITEM 15a treatment: identical script, identical fixture, PLUS the ring.
  wall)    PITVAR=" BASTION_FLAT_ARENA_WALLED=1"
              SCRIPT="script-wall.txt" ;;
  fleehealth) PITVAR=" BASTION_ACCESS_CLAIM_DIAG=1"
              SCRIPT="script-fleehealth.txt" ;;
  # The matched control for fleehealth: treatment disabled at the TARGET, so
  # every other stage of the arm is byte-identical. A bar needs its own control.
  fleehealthctl) PITVAR=" BASTION_ACCESS_CLAIM_DIAG=1"
              SCRIPT="script-fleehealth-ctl.txt" ;;
  flat)    PITVAR="" ;;
  *)       echo "usage: run-pit.sh <pit|flat|pitdiag|pitwood|pitnowood|pitwood2|pitnowood2|pitwood3|pitplant|pitread|sweep|stamp|shaft|contend|queuewait|flee|fleecost|fleehealth|fleehealthctl|wallctl|wall|recr|recrctl|shaftwolf|hostile|wall2ctl|wall2|wallcheck|wallcheckctl|wallsurvey|wallsurveyctl|wallsurvey2|wallsurvey2ctl|wallverdict|wallverdictctl>" >&2; exit 2 ;;
esac
# TICK-LOADING ROW: determinism is now PER-ARM, not hard-coded for every leg.
#
# ★★★ WHY THIS CHANGED, WITH THE COST ATTACHED. The provbase arm was written to
# measure the LIVE free-running chunk drain and its own comment claimed "no
# deterministic gate". That comment sat ELEVEN LINES ABOVE this export, which
# forced BASTION_DETERMINISTIC=1 onto it anyway. The run emitted 3763 census
# ticks, ALL of them deterministic_drain=true, and the budget derived from them
# had to be withheld -- it described a drain the live server never takes.
#
# A COMMENT CANNOT ENFORCE. The arm table could not EXPRESS determinism-off, so
# no amount of care in the arm's own entry could produce it. Same defect class
# as #100 (the harness cannot express an arena-OFF arm): a config surface with
# no OFF branch silently makes every measurement an ON measurement.
#
# PITDET defaults to ON so every existing arm is byte-identical to before; only
# an arm that explicitly clears it gets the free-running drain.
PITDET="${PITDET-BASTION_DETERMINISTIC=1 }"
# ★★★ THIS CLOSES #100 ("the harness cannot express an arena-OFF arm"). The
# flat-arena flags were hard-coded here exactly as BASTION_DETERMINISTIC was,
# so no arm could ask for REAL TERRAIN. #99 had to prove the arena-off branch
# via a bare manual boot for that reason. Same defect, same fix: default ON so
# every existing arm is byte-identical, and let an arm clear it.
#
# It also unblocks the tick-loading baseline: a flat pre-generated arena has
# nothing left to generate, so loading cannot be measured inside one.
PITARENA="${PITARENA-BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 }"
# ★★★★ TPS IS NOW PER-ARM, AND THIS IS THE FIX FOR A2's FALSE RED.
#
# With BASTION_UNCAPPED_TPS=1 the server runs flat out while the driver waits a
# fixed number of CLIENT ticks. So the number of SERVER ticks per script step
# varies with machine speed: two runs of the same script covered 17726 and 23143
# ticks. I then diffed tick-N against tick-N across those two timelines and
# reported "loading is not deterministic -- the row's central claim is failing".
#
# The loading was IDENTICAL: 64 promoting ticks, same values, same order, in
# both runs. Only the tick LABELS shifted. The defect was the measurement.
#
# Uncapped stays the DEFAULT because every existing arm is calibrated for it and
# it is what makes the legs affordable. But an arm that MEASURES anything
# tick-indexed must cap the rate, or its tick axis is wall-clock in disguise.
PITTPS="${PITTPS-BASTION_UNCAPPED_TPS=1}"
# PITPLANT: registered failure-injection, empty by default so every existing arm
# is byte-identical. The server is launched via `env $BASTION_ENV`, so a plant
# variable that is NOT in this string never reaches the server process -- it
# would run the arm as its own control and report a null result as a refutation.
PITPLANT="${PITPLANT:-}"
export BASTION_ENV="${PITDET}BASTION_AUTOFOUND_COLONY=8 ${PITARENA}${PITTPS}${PITPLANT}$PITVAR"

# THE FOUR FILES THIS RUN WILL WRITE, declared before it writes any of them.
# Same one-definition discipline as BASTION_ENV above: these strings are used
# BOTH here (recorded in the attestation) and below (the actual redirects), so
# the evidence file cannot name a path the run did not produce. Every one is a
# PROMISE at this point -- `run-ledger.sh` resolves each against the disk after
# the fact and reports a declared-but-absent path by name.
export BASTION_LOGS="$EV/server-$TAG.log $EV/$TAG.log $EV/driver-$TAG.log $EV/driverout-$TAG.log"

. "$EV/launch-preamble.sh"

rm -rf "$WT/userdata-$TAG"
VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A "$B/veloren-server-cli$EXE" \
    --no-auth admin add "$TAG" admin > /dev/null 2>&1
S=$WT/userdata-$TAG/server/server_config/settings.ron
sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$METRICS\"/g" "$S"
sed "s/:14005\"/:$WEB\"/" "$WT/userdata-$TAG/server-cli/settings.template.ron" \
    > "$WT/userdata-$TAG/server-cli/settings.ron"

( cd "$WT" && VELOREN_USERDATA="$UD" VELOREN_ASSETS=$A \
    exec env $BASTION_ENV \
    "$B/veloren-server-cli$EXE" --no-auth > "$EV/server-$TAG.log" 2>&1 ) &
SRV=$!
echo "server pid=$SRV arm=$ARM (started by this script)" > "$EV/$TAG.log"

# DECLARED WINDOW, fixed before launch: 300s wall for the port, then the
# driver script's own budget -- four `inspect_colonists` samples 600 apart.
#
# `wait n` COUNTS SIM-SECONDS, not ticks: the driver waits on the
# server-tracked `Time` resource (bastion_playtest.rs:507), which advances
# by `scaled_dt` every tick. So the scored window is 2400 SIM-seconds, and
# under BASTION_UNCAPPED_TPS that is a much shorter wall time. Stated in the
# unit the script actually uses, because "600" meaning ticks and "600"
# meaning sim-seconds differ by more than an order of magnitude here.
#
# Declared before launch and not extended if the run looks promising.
t=0
while [ $t -lt 300 ]; do
  if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then exec 3<&- 3>&-; break; fi
  sleep 3; t=$((t+3))
done
echo "port $GAME open after ${t}s" >> "$EV/$TAG.log"

# THE DRIVER IS THE INSTRUMENT, NOT THE LOG. `status` is an INSPECTOR field:
# the charter names `inspect_colonists` as its consumer, and the driver
# already prints `status={:?}` (bastion_playtest.rs:723). Scoring it from the
# server log would be reading a different surface than the one the field is
# specified against.
"$B/bastion_playtest$EXE" "127.0.0.1:$GAME" "$TAG" \
    "$EV/$SCRIPT" "$EV/driver-$TAG.log" > "$EV/driverout-$TAG.log" 2>&1
echo "driver exited rc=$?" >> "$EV/$TAG.log"

. "$EV/launch-postamble.sh"
