# N=8 promotion-tick distribution test — results

Opus-specified (replaces the retired flat-arena fingerprint A/B): does
`BASTION_UNCAPPED_TPS` shift the colonist-promotion tick distribution
relative to real chunk-gen time? 16 live legs (8 capped, 8 uncapped),
each: fresh boot, `spawn 8`, driver disconnects, server runs 40 real
wall-clock seconds unattended, last "colonist promoted to loaded" tick
recorded. All legs ran on an isolated port (24004) while `v5` flew on the
default port (14004) — every result carries `v5_concurrent=true` and
`v5_offset_secs` (wall-clock seconds since v5's own launch) as a
provenance field.

## RESULT

    capped:   [185, 188, 192, 220, 220, 221, 232, 233]  mean=211.4 median=220.0
    uncapped: [1134, 1177, 1205, 1314, 1347, 1389, 1450, 1458]  mean=1309.2 median=1330.5

**Zero overlap. Gap of 901 ticks between capped's max (233) and
uncapped's min (1134).** Uncapped's promotion tick sits entirely outside
the capped spread — the answerable question Opus specified is answered:
yes, compression systematically shifts promotion tick, exactly as the
chunk-gen wall-coupling mechanism (real CPU time, independent of tick
pacing) predicts.

## METHOD NOTE — a self-inflicted ordering defect, caught and fixed mid-run

**CORRECTED (an earlier version of this doc said interleaving began at
leg 7; that was wrong — a prose-summary error, not what the log shows.
The table below is read directly from each leg's own `Server version`
boot line, not from memory, per Opus's flag that the two reports he
received disagreed.)**

Ground truth, boot order:

    00:06:16  capped-1     )  pre-interleaving: no uncapped leg exists yet
    00:07:44  capped-2     )
    00:13:17  capped-3     -- interleaving begins HERE
    00:14:33  uncapped-1
    00:15:39  capped-6     -- see note below: a STRAY background process,
                              not part of the intended sequence
    00:18:17  capped-4     -- retry, after a collision with that same stray
    00:19:29  uncapped-2
    00:20:43  capped-5
    00:22:06  uncapped-3
    00:23:23  capped-7
    00:24:37  uncapped-4
    00:25:52  capped-8
    00:27:06  uncapped-5
    00:28:59  uncapped-6
    00:30:16  uncapped-7
    00:31:31  uncapped-8

**Interleaving began at leg 3** (capped-3 immediately followed by
uncapped-1), not leg 7. Only legs 1-2 (capped) ran before any uncapped
leg existed, and that's because interleaving hadn't been specified yet
at that point in real time — those two legs predate the request, they
weren't skipped past it.

**Separately, and unrelated to the ordering question:** two background
batches were accidentally launched overlapping each other (both covering
leg numbers 3-4), causing real file/port collisions between MY OWN
processes on the isolated port — never touching v5 (verified: same PID,
unbroken log, both before and after every collision). The `capped-6`
boot at 00:15:39 in the table above is that stray batch's own leg-6
attempt, completing successfully (232) in a gap between two of my
foreground runs — its result is kept (nothing about ITS run was
corrupted, only legs 3/4 that directly collided with it were). Caught
via a stray-process check after the fact, cleaned up, and every leg
after `capped-4`'s retry ran strictly foreground, one at a time. Legs 3,
4, 5 were re-run cleanly after the corrupted attempts; their reported
values here are from the clean re-runs only.

## THE SANITY FLAG, REGISTERED PRE-DATA (Opus)

Legs 1-2 (233, 220) are much tighter than the earlier standalone capped
pair from the retired fingerprint work (192, 624). Two live hypotheses,
not yet distinguished:

1. **The isolated leg (spawn-only, no designations, 40s fixed window) is
   a cleaner instrument** than the earlier full-founding-script pair.
2. **The box is uniformly loaded by v5's concurrent run**, compressing
   the spread that would otherwise appear on an idle machine.

**Planned to distinguish them:** re-run 2-3 capped legs after v5
finishes and the box is idle again, compare against this set.

**Partial answer already visible (Opus):** the earlier standalone **192**
sits INSIDE this run's capped range (185-233); the earlier **624** does
not. That points toward "the isolated leg is a cleaner instrument," not
"the box was uniformly loaded" — the post-v5 clean set will confirm.

## WHY THE ORDERING DEFECT DOES NOT EXPLAIN THE RESULT (Opus's argument,
registered here)

Sample A (legs 1-6, capped-only block under v5 load) stays formally
UNSCOREABLE against the compression axis — that ruling stands, no
walking it back. But the finding survives it anyway: **the capped arm
spans the ENTIRE window this dataset covers (legs 1-8, v5_offset_secs
992 through 1892) and its own spread is only 185-233, ~26%.** That
spread IS the direct measurement of how much v5's background load moves
this number across the whole run. A 6x, zero-overlap gap to the uncapped
arm is not explainable by a confound whose own measured size is ~26%.
The interleaving fix (legs 7+) is still the right process fix going
forward; it just isn't what's carrying this particular result.

## THE MANDATORY PLANTED FAILURE — STOOD DOWN, not skipped (Opus's ruling)

The required structure was RED on the axis (capped vs uncapped) AND
GREEN on a matched control (capped vs capped). Both already exist in
this data: axis = 6x shift, zero overlap (RED); control = the capped
arm's own 185-233 spread, tight, consistent across the whole run
(GREEN). A planted chunk-gen delay exists to prove a comparison that
found nothing COULD have found something — this comparison already
found something, decisively. Injecting a synthetic failure now would
prove a fact the real data already proved. Not run; will run on request
(Fable) if wanted for the permanent record, but not required to certify
this result.
