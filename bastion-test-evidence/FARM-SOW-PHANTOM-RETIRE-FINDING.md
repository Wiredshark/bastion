# Farm sowing never completes: every SOW job is phantom-retired before any colonist claims it

**Status: REFRAMED by the addendum below (Fable's parallel-fill task,
2026-08-10). The phantom-retire mechanism this document originally centers on
is REAL but INCIDENTAL -- a same-instant, end-of-run cleanup triggered by the
driver's own disconnect, not the reason sowing never happens. The actual root
cause is upstream and structural: see "THE REAL ANSWER" in the addendum. The
original body below is preserved because it correctly establishes the
retirement mechanism's *code path* (still true, still useful for item 7's fix
shape) -- read it for that, not for "why sowing fails," which the addendum
now answers. Routed to item 7 (farm-to-table).**

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

---

# ADDENDUM (2026-08-10): why `terrain.get` fails, the timing dimension, and
# the real answer to "why does sowing never happen"

Fable's parallel-fill task, answered against the code and the raw log
directly -- no citation taken on faith.

## Thread 1: enumerating `terrain.get`'s error paths

`terrain: &TerrainGrid` at every relevant call site
(`bastion_jobs.rs` -- confirmed by grep, not assumed), and `TerrainGrid =
VolGrid2d<TerrainChunk>` (`common/src/terrain/mod.rs:271`). Read both layers
at the source, not the trait signature alone:

**`VolGrid2d::get`** (`common/src/volumes/vol_grid_2d.rs:65-74`):

    fn get(&self, pos: Vec3<i32>) -> Result<&V::Vox, VolGrid2dError<V::Error>> {
        let ck = Self::chunk_key(pos);
        self.get_key(ck)
            .ok_or(VolGrid2dError::NoSuchChunk)
            .map(|chunk| chunk.get_unchecked(Self::chunk_offs(pos)))
    }

**It calls the chunk's `get_unchecked`, not its fallible `get`** -- the
comment right above it says why: *"Always within bounds of the chunk, so we
can use the get_unchecked form."* `VolGrid2dError` has four variants
(`NoSuchChunk`, `ChunkError`, `DynaError`, `InvalidChunkSize`), but only
`NoSuchChunk` is reachable through this specific method -- the other three
belong to other `VolGrid2d` operations (insert/remove), never `get`.

**`Chonk::get_unchecked`** (`common/src/terrain/chonk.rs:187-203`, `TerrainChunk
= Chonk<Block, TerrainChunkSize, TerrainChunkMeta>`) is UNCONDITIONALLY
infallible for every `z`:

    fn get_unchecked(&self, pos: Vec3<i32>) -> &V {
        if pos.z < self.get_min_z() { &self.below }
        else if pos.z >= self.get_max_z() { &self.above }
        else { /* real sub-chunk lookup, always in-bounds by construction */ }
    }

Below the generated column: returns the homogeneous `below` voxel. Above it:
returns the homogeneous `above` voxel (this is the ordinary "air above
ground" case -- a crop cell one block over tilled earth). Within it: a
sub-chunk index that the surrounding `if`/`else if` already proved in range.
**No branch returns an error, for any `z`.**

> ## THE ONE-BLOCK-DISCRIMINATION FILTER, APPLIED: IT ELIMINATES EVERY
> Z-SPECIFIC EXPLANATION
>
> `TerrainGrid::get(pos)` can fail ONLY via `NoSuchChunk` -- meaning the
> chunk's `(x, y)` key isn't resident at all. That check depends on `(x, y)`
> only; `z` never enters it. **A TILL job's `gpos` and its SOW job's
> `cpos = gpos + 1` share the same `(x, y)` and therefore the same chunk key.**
> If the chunk is resident, BOTH read successfully, at any `z`, always. If the
> chunk is NOT resident, BOTH fail identically. There is no code path by
> which one succeeds and the other fails for a reason rooted in `z` alone.
> **The original "read fails at the crop cell specifically" framing is
> refuted by the source, not just observed as surprising.**

## Thread 2: the timing dimension -- and it closes the question outright

Checked directly against `row89-green/run.log` (stripped of ANSI codes) and
`row89-green/driver.log` (its actual paired run was `script-10-milestone-food`
into `script-11`'s continuation, per the driver's own script-name line):

**All 48 farm phantom-retirements fire in the SAME arbitration pass, not
staggered by each job's own age:**

    2026-08-09T23:09:24.002689Z  through  2026-08-09T23:09:24.003804Z

48 lines spanning 1.1 milliseconds -- one sweep, one moment, every remaining
open farm job at once. `ARBITRATION_INTERVAL = SIM_TPS / 2 = 15` ticks (30
TPS -- `bastion-server/src/lib.rs:48`, `bastion_jobs.rs:1662`), so this sweep
fires roughly twice a second. Job `703`'s own creation-to-retirement gap was
~40 minutes; that is ~4800 sweep opportunities the job SURVIVED before the one
that retired it. **This did not fail from the moment of creation. It failed
once, at one specific instant, after being fine for the entire run.**

**That instant is the driver's disconnect, not a coincidence:**

    driver.log (script-10/11 continuation), last line:
      [1786316963325] === script complete, disconnecting ===
      -> Sun Aug 9 23:09:23 UTC 2026 (converted directly, not eyeballed)

    server log, first farm retirement:
      2026-08-09T23:09:24.002689Z

**Under one second between the connecting player disconnecting and every
open farm job retiring in a single sweep.** Combined with Thread 1 (the only
real failure mode is `NoSuchChunk`, a whole-chunk-residency fact): the
mechanism this data points to is **the farm plot's chunk unloading once the
last connected client leaves its view distance, and the very next F3 sweep
(within half a second, by construction) finding every remaining open
designated job's target cell unreadable and retiring all of them at once.**
Not directly observed via a "chunk unloaded" log line (none exists to check
against) -- named as the mechanism the evidence points to, not as something
independently confirmed by a third source.

## THE REAL ANSWER: the retirement was never why sowing fails

This reframes the whole document. **The retirement happens at the END of the
run, all at once, triggered by disconnect -- it cannot be the reason sowing
never happened DURING the 40 minutes the world was loaded and the jobs were
perfectly readable.** The real question was always: why were 0 of 48 sow jobs
ever claimed while the run was live? That is answered structurally, not by a
timing artifact:

    bastion_jobs.rs:9683 (comment on SOW job creation):
      "crop cell, required_item = seeds -- B6's material-haul delivers
      stockpiled seeds for free"

    bastion_jobs.rs:13692-13700 (the ONLY site in the entire crate that
    creates a wheat-seed item):
      for _ in 0..FARM_SEED_YIELD {
          crate::bastion_actions::emit_drop(..., Item::new_from_asset_expect(FARM_SEED_ITEM), ...);
      }
      -- inside the HARVEST completion arm. Nowhere else.

    bastion_jobs.rs, the GROWING match arm's own comment (Growth == 0):
      "Growth 0 on a farm cell = a worldgen volunteer -- left alone (the
      reserved stage; harvesting volunteers is a later nicety)."
      -- explicitly NOT auto-harvested.

> ## A CHICKEN-AND-EGG SEED DEADLOCK, NOT A TERRAIN OR TRAVERSAL BUG
>
> `FARM_SEED_ITEM`'s only producer in the codebase is a successful HARVEST.
> A HARVEST requires a MATURE crop. A mature crop requires a prior SOW. A SOW
> requires the material-haul to deliver a stockpiled `FARM_SEED_ITEM` (B6's
> own fetch contract, the same one `required_item` gates on for build/ladder
> materials). **On a fresh colony with no `/give_item` seed grant, no
> pre-stocked stockpile, and worldgen volunteers explicitly left unharvested,
> there is no possible source for the first seed.** Every sow job sits
> unclaimable for exactly as long as the colony exists, because the one thing
> that could satisfy it can only be produced by completing it first.

This is now a code-level, two-independent-fact finding (the sole producer
site, and the fetch-contract's dependency on stockpiled supply), not a
further hypothesis -- though it has not been LIVE-TESTED (no run has yet
`give_item`'d wheat seeds directly to confirm claim/completion succeeds once
supply exists; that is the cheap, decisive confirmation left for item 7).

## Revised summary for item 7

Two separate, now-distinct findings, not one:

1. **Why nothing ever gets sown**: a seed-supply chicken-and-egg deadlock,
   structural, code-verified at two independent sites. Fix shape: a bootstrap
   seed source (a small starting stock, a `/give_item`-style admin grant path,
   or implementing the already-named-but-unbuilt worldgen-volunteer harvest).
2. **Why the stuck sow jobs eventually vanish as "phantom" instead of sitting
   forever**: task #57's sweep retiring an entire region's open jobs at once
   when the driver disconnects and the chunk unloads, mis-logging a
   whole-chunk residency loss as a per-job "designation mismatch." Fix shape,
   unchanged from the original body above: split the read-failure case from
   the genuine-mismatch case in `job_still_wanted`'s caller, and don't treat a
   `NoSuchChunk` result as authoritative about whether a job is still wanted.

Neither finding depends on the other. (1) is the reason a live colony run
never produces wheat. (2) is a real, separate defect in how abandoned jobs get
cleaned up when a client leaves -- worth fixing on its own terms, but not the
explanation this document originally treated it as.
