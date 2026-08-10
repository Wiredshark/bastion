# Farm sowing never completes: every SOW job is phantom-retired before any colonist claims it

**Status: ROOT CAUSE CLOSED BY DEDUCTION (verified against both the raw log and
the live code, not taken on citation). One open question remains (why the
terrain read fails) and one fix shape is named. Routed to item 7 (farm-to-table).**

## Origin

Surfaced as an aside during item 6's live acceptance work (2026-08-09/10, row89-green
GREEN leg) -- not the row's own subject, named honestly rather than folded into a
subordinate clause per Fable's standing instruction. Investigated jointly with Opus
across several corrections; this document supersedes the intermediate reads below
because each was checked against the raw log rather than against the prior read.

## The evidence

Source: `bastion-test-evidence/row89-green/run.log` (40,983 lines). The log carries
ANSI colour codes that split `key=value` pairs across escape sequences (e.g.
`sow` + ESC + `=` + ESC + `false`) -- a naive `grep "sow=false"` returns zero hits
and looks like the field doesn't exist. Strip codes first
(`sed -r 's/\x1b\[[0-9;]*m//g'`) before grepping this or any bastion log for a
`key=value` field.

**Farm plot registered** (per-column surface resolution, already live on this
branch -- see "ruled out" below):

    farm plot registered, per-column surface resolved zone=1
    region=Region{min:(15198,15998,418), max:(15204,16004,419)} resolved=49 unresolved=0

**96 farm jobs created for the 49-cell plot** (some cells generate more than one
job across the run as TILL completes and SOW becomes eligible):

| cohort | created | `job claimed` | `colonist arrived...working` | `phantom job retired` | `sown`/wheat-placed log line |
|---|---|---|---|---|---|
| `sow=false` (TILL) | 48 | 48/48 | 48/48 | 0/48 | n/a |
| `sow=true` (SOW) | 48 | **0/48** | **0/48** | **48/48** | **0 anywhere in the log** |

Every TILL job was claimed by a colonist, arrived-at, and worked -- none were
phantom-retired. Every SOW job was created and then phantom-retired
(`task #57`, `"target cell no longer matches the designation"`) with **no
intervening claim or arrival line for any of the 48** -- checked individually,
not sampled. The string `sown` does not appear anywhere in the 40,983-line log.

Example, traced end-to-end (representative of all 48, not cherry-picked):

    22:29:31.41  farm job created job=551 pos=(15204,16004,418) sow=true
    23:09:24.00  phantom job retired (task #57) job=551 pos=(15204,16004,418) kind=Designated(Farm)

40 minutes elapse between creation and retirement with zero log activity referencing
job 551 in between.

## What this rules out

- **Registration divergence** (does the paint path register where the job
  generator looks?) -- ruled out independently in
  `bastion-test-evidence/live-playthrough/FARM-DIAGNOSIS.md` (DECISIONS #62,
  2026-08-04): both paths read/write the same `board.farms`, confirmed live.
- **Zero-tolerance plot-height mismatch** (the same doc's own finding, from an
  older build) -- **does not apply to the current code.** That build required
  `ground.is_filled()` at exactly `plot.min.z` with no per-column resolution.
  row89-green's own registration log line above shows `resolved=49
  unresolved=0` -- per-column surface resolution is already live on this
  branch (matches `bastion_jobs.rs`'s `board.farm_column_z` map, read directly).
  A z-mismatch hypothesis for row89-green was raised and **withdrawn** after
  checking this.
- **Traversal / can't-reach-the-cell** -- ruled out: every SOW cell sits
  directly above (or beside) a TILL cell 48 colonists successfully reached
  and worked on the very same plot, same run, same colonists. Reach is not
  the limiting factor.
- **"Retiring on sow = success, not failure"** -- this is `FARM-DIAGNOSIS.md`'s
  own conclusion, but for a *different* run (`script-08-farmfix.txt`, its own
  13/13 sow jobs). That conclusion does not transfer to row89-green: a success
  retirement should follow a claim, an arrival, and (per the code) an
  inventory hand-off of the seed item; row89-green's 48 show none of the
  three, and zero `sown`/wheat-placement lines exist anywhere in the log to
  corroborate a placement having happened silently.

## The mechanism -- closed by deduction, verified against the live code

Only one call site emits the exact retirement message
(`bastion_jobs.rs:8570-8593`, the `ARBITRATION_INTERVAL`-cadenced phantom-sweep
task #57 itself; the OTHER `job_still_wanted` call site, ~line 10767, emits a
different string, `"job moot mid-travel -- target block changed; dropped"`, and
requires an active claimant, which none of the 48 ever had):

    let wanted = terrain.get(j.pos).ok().is_some_and(|b| job_still_wanted(&j.kind, b));
    (!wanted).then_some(*id)

and `job_still_wanted`'s Farm arm (`bastion_jobs.rs:1408-1418`, read directly,
not quoted secondhand):

    JobKind::Designated(DesignationKind::Farm) => true,   // unconditional

**`job_still_wanted` cannot return `false` for a Farm job.** So `wanted` can
only go false if `terrain.get(j.pos)` itself fails -- `.ok()` turns an `Err`
into `None`, and `.is_some_and(..)` treats `None` the same as "predicate
returned false". **Every farm phantom-retirement in this log is therefore a
terrain-read failure at the job's position, not a designation mismatch** --
deductive from the code given the log's own data (zero claims, zero arrivals,
100% retirement), not a further hypothesis.

**The retirement message actively misattributes its own cause.** "target cell
no longer matches the designation" is printed for BOTH `job_still_wanted`
returning false AND `terrain.get` failing -- the same absence-vs-exclusion
collapse this codebase has hit before, this time inside a boolean guard rather
than a data store. That collapse is why this took multiple reads to close: the
log line asserts a conclusion ("mismatch") the code never actually established
("read failed").

**Corroborating detail** (checked directly against the same log): SOW jobs
target `cpos = gpos + 1` (the crop cell, one block above the TILL job's ground
cell) -- confirmed exactly in the z-histograms, which line up 1:1 by column
count (14 cells at z=415->416, 22 at z=416->417, 12 at z=417->418, `sow=false`
-> `sow=true` respectively). Every TILL job's `terrain.get(gpos)` succeeds
(48/48 claimed and worked); every SOW job's `terrain.get(cpos)`, one block
higher, fails (48/48 phantom-retired). The read failure is specific to the
crop cell, not the plot.

## The tension the fix must resolve

A crop cell one block above tilled ground is ordinarily just air, and air is a
valid, readable block -- `terrain.get(cpos)` should succeed, `job_still_wanted`
should return `true` unconditionally for Farm, and the job should not retire.
**The deduction says "read failure"; the physics says "that cell should be
readable." Both cannot be true as stated**, and the code as it stands cannot
tell us which `false` path actually fired -- a genuinely surprising result
(read truly fails) is indistinguishable from a mundane one (some other bug
sends the wrong `job.pos` into the read). Splitting the two `false` cases
turns this contradiction into a measurement instead of leaving it a puzzle;
that is what makes the fix worth doing on its own, not merely tidy.

## What remains open

**Why `terrain.get` fails at the crop cell specifically** -- unloaded chunk
edge, a column-height/extent limit one block above the tilled surface, or
something else. Not yet determined; the z-clustering above narrows it but does
not answer it.

**Budget confound, separately noted, not resolved here:** even in
`FARM-DIAGNOSIS.md`'s successful 13/13 run, `crop MATURE`/harvest was never
observed within its session's time budget (`FARM_GROWTH_MAX=15 *
FARM_STAGE_SECS=6.0` -> ~84s minimum after a real sow). Since row89-green never
gets a real sow at all, this run says nothing about maturity/harvest either
way -- that stage remains fully untested on this branch.

## For item 7 (farm-to-table)

Farming is not "broken" in the diffuse sense the parked task list entry
implies -- registration, per-column height resolution, and TILL all work
correctly live. The defect is narrow and its mechanism is closed: SOW jobs are
retired by a terrain-read failure at the crop cell, misreported as a
designation mismatch, before any colonist can claim them -- 48/48, zero
exceptions. Two things left for item 7: (1) find why the read fails at
`cpos = gpos + 1` specifically, (2) fix shape regardless of cause -- a read
failure must not be reported as a designation mismatch, and probably must not
silently retire the job at all. `FARM-DIAGNOSIS.md` stays valid for its own
run (sow jobs there retired *after* being worked, a different path to the same
message) and is not superseded by this document.
