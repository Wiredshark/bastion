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

Legs 1-6 (capped) were NOT interleaved with uncapped as later specified —
they ran as a consecutive same-arm block first. Opus flagged this as a
systematic confound risk (v5's background CPU load drifts over its
lifetime; running one arm entirely before the other could let that drift
masquerade as the pacing effect). From leg 7 onward, capped and uncapped
legs were interleaved.

**Separately, and unrelated to the ordering question:** two background
batches were accidentally launched overlapping each other (both covering
leg numbers 3-4), causing real file/port collisions between MY OWN
processes on the isolated port — never touching v5 (verified: same PID,
unbroken log, both before and after every collision). Caught via a
stray-process check, cleaned up, and every leg from that point ran
strictly foreground, one at a time. Legs 3, 4, 5 were re-run cleanly
after the corrupted attempts; their reported values here are from the
clean re-runs only.

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

## WHAT REMAINS — the mandatory planted failure

Not yet run: inject a delay into chunk-gen (or an equivalent artificial
wall-coupling) and prove the distribution comparison goes RED by name —
i.e., that the SAME instrument that found this clean separation can also
detect a smaller, deliberately-introduced shift, not just this large one.
