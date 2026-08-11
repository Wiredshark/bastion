# Item 7 (farm-to-table): seed-supply deadlock — live confirmation

**Boot stamp:** commit `8488b65f497e835426a523f9177890a35538ef8e`, branch `bastion/wip-batch-verify`.

**Status: CONFIRMED.** FARM-SOW-PHANTOM-RETIRE-FINDING.md's addendum named the
seed-supply chicken-and-egg deadlock as a code-level, two-independent-fact
finding, "not yet LIVE-TESTED... that is the cheap, decisive confirmation
left for item 7." This run is that confirmation.

## Setup correction caught mid-run (worth recording so it doesn't recur)

The first script attempt reused `script-08-farmfix.txt`'s Aug-04 coordinates
against a **fresh** `VELOREN_USERDATA` dir. A fresh userdata dir means fresh
worldgen — the old run's "known good" terrain coordinates describe a
*different world instance*, not this one. The driver's own
`player pos at script start: Vec3 { x: 15216.5, y: 16016.5, z: 419.0 }` line
immediately showed the mismatch (nowhere near the old x=15198-15204,
y=15998-15999 region). Killed before wasting the confirm window rather than
letting a stale-assumption run produce a void result. Re-ran in two phases:
phase 1 (`script-13a-farmseed-survey.txt`) surveyed the real spawn area
(`gap=0` reports every column's surface height, not just overhangs);
phase 2 (`script-13b-farmseed-confirm.txt`) designated against the verified
flat z=418 block found there.

## The confirm

Farm plot (5x6, 27 SOW jobs after TILL) and an adjacent stockpile designated
on verified-flat ground next to the player's actual spawn. Let TILL complete
and SOW jobs open, then checked every `job claimed` event against the known
SOW-job-id set:

    SOW job ids (sow=true):      8 9 13 14 18 19 22 23 24 28-48  (27 total)
    Jobs claimed before seeds:   0 1 2 3 4 5 6 7 10 11 12 15 16 17 20 21 25 26 27

The pre-seed claim set is **exactly** the 19 TILL job ids — zero overlap
with SOW. Not one SOW job claimed while no seeds existed, for the full
~40-second window TILL had to complete and SOW sat open.

`cmd give_item common.items.bastion.wheat_seeds 20` (server confirmed:
`command-give-inventory-success`) then `cmd dropall true` (the same
persistent-pile mechanism item 6's acceptance run proved) — a haul job
(job 49) carried the seeds into the stockpile, and SOW jobs began claiming
and completing within ~15 seconds:

    job claimed job=24 colonist=100          03:18:17.397
    sown pos=(15213,16016,419)               03:18:21.368
    sown pos=(15213,16015,419)               03:18:24.954
    ...

**20 seeds given → exactly 20 `sown` completions** by run end (checked: the
run's total `sown` line count is 20). The remaining 7 SOW jobs simply ran
out of seed supply — not stuck for any other reason, a clean 1:1
correspondence.

## What this does and doesn't establish

**Confirms:** the deadlock is real and the manual-grant fix genuinely breaks
it — seeds hauled from a stockpile are consumed by SOW exactly as the B6
fetch contract describes. This is the decisive live test the addendum asked
for; the finding moves from "code-verified, not yet run" to "run, and it
behaves exactly as predicted."

**Does not confirm (out of this run's scope):** HARVEST/maturity. Zero
`crop MATURE`/harvest lines in this run's log — expected, matching the
addendum's own noted budget confound (`FARM_GROWTH_MAX * FARM_STAGE_SECS`
~84-90s minimum after a real sow, and sows here were staggered from ~40s to
~170s into a ~340s sim window, so most crops didn't reach the growth window
before the run's own checkpoint schedule ended it). The full sow→harvest→
seed-regeneration cycle remains untested; a longer run is a separate,
straightforward follow-up, not a new finding.

## Item 4 rider: fail-safe observation

80 `GOTO-STAND-RESCUE` events fired across the run (03:17:27–03:19:30),
26 distinct uids. Raw counts only — not re-scored against item 4's
egress-verdicts-vs-plans signature here; that interpretation belongs to
whoever owns item 4's re-score, and the population question Opus flagged
(live colony + possibly non-colonist entities sharing the same uid space vs
the acceptance fan's harness-scenario population) is still open. Logged as
an observation rider, not a verdict.

## Permanent fix, not yet built

This confirms the DIAGNOSIS; it does not by itself ship a fix. Per the
addendum's own fix-shape options: a small starting seed stock, a permanent
`/give_item`-equivalent bootstrap at colony founding, or implementing the
already-named-but-unbuilt worldgen-volunteer harvest. Not chosen or built
here — this run's job was the cheap confirm, not the permanent shape.

## Evidence

    bastion-test-evidence/live-playthrough/script-13a-farmseed-survey.txt
    bastion-test-evidence/live-playthrough/script-13b-farmseed-confirm.txt
    bastion-test-evidence/live-playthrough/driver-farmseed-survey.log
    bastion-test-evidence/live-playthrough/driver-farmseed-confirm.log
    bastion-test-evidence/live-playthrough/server-stdout-farmseed.log
    bastion-test-evidence/live-playthrough/userdata-farmseed/

---

# ADDENDUM (2026-08-11): the full cycle closes — first HARVEST in the
# project's history

Fable requested the harvest observation as the survival arc's closing leg.
Ran a follow-up (`script-13c-farmseed-harvest.txt`) reusing the SAME
verified z=418 coordinates and the proven till/seed/sow sequence, but with
the post-sow wait extended to ~490s (vs. the first confirm's ~230s) purely
to give crop growth (`FARM_GROWTH_MAX=15 * FARM_STAGE_SECS=6.0` = 90
sim-seconds minimum) room to complete. World/terrain is confirmed to
persist across reboots of the same `VELOREN_USERDATA` dir (identical player
spawn position across three separate boots); `JobBoard`/farm growth state
does not, so the till/seed/sow sequence had to be repeated from scratch.

## Result: 20 sown -> 20 crop MATURE -> 20 harvested. A closed, 100% cycle.

    sown pos=...      (20 total, same clean 1:1 with the 20 given seeds)
    crop MATURE pos=...  (20 total)
    harvested (cell returns to tilled) pos=...  (20 total)

Every sown cell matured and was harvested; none stalled. This is the first
time this project's farming system has completed a full TILL -> SOW ->
GROW -> MATURE -> HARVEST cycle, live.

## What's still open: self-sustaining reseed not observed in this window

Harvest completion's own code path (`bastion_jobs.rs` ~13692-13700) emits
`FARM_SEED_YIELD` new seed-item drops on every successful harvest -- in
principle enough to refill the stockpile and start a SECOND sow wave
without another manual grant. Each of the 20 harvested cells did generate
a fresh SOW-eligible slot (`farm job created` fired 20 more `sow=true`
jobs after the first harvest, one per re-tilled cell), but **zero of those
second-wave SOW jobs had been claimed by the time the run ended** (`sown`
stayed at exactly 20 for the whole run, no second wave observed) -- the
harvest-yielded seeds' haul-to-stockpile-then-fetch pipeline either needed
more real time than this run's remaining window gave it, or there is a
genuine second gap in the reseed loop that has not been isolated. Named
as an open question, not a claim either way -- a longer run (or one that
starts its extended-wait clock from the FIRST harvest rather than from the
FIRST sow) is the natural next confirm, not built here.

## Item 4 rider (this run)

373 `GOTO-STAND-RESCUE` events, 37 distinct uids, 03:27:18-03:35:29 (the
full ~490s window). Same caveat as the first run: raw counts only, no
re-score attempted here.

## Additional evidence

    bastion-test-evidence/live-playthrough/script-13c-farmseed-harvest.txt
    bastion-test-evidence/live-playthrough/driver-farmharvest.log
    bastion-test-evidence/live-playthrough/server-stdout-farmharvest.log
