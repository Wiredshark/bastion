# TRAVEL ROW — MAKE "UNREACHED" DISTINGUISHABLE FROM "UNREACHABLE"

**Blob for every cite: `5f8cdf1392`.** Line numbers move — re-locate by symbol.
**Supersedes AUTON-2 §4c**, which retracted into this row (`9a07ec2865`).

> ## ★ THE ROW'S ONE SENTENCE
> **A colonist that COULD have arrived and didn't must not render identically to
> one that COULDN'T.** Everything else here is downstream of that.

## §1 — WHY THIS ROW EXISTS: SIX APPEARANCES, ZERO BUG REPORTS

The motif — *displaced colonists failing to arrive* — has surfaced **six times**:
the bed walk-test, seeds 52/54's vantage split, farm colonists 9 blocks below,
the farm-corner cells, the condemned-cell read, and now **seed 7 with a measured
vector**. **It has never once been filed as a bug.**

★ **The reason is now known and it is not neglect.** `preempt_scenario` specifies
a degradation for an unreachable bed — watchdog releases, orphan sweep removes,
cooldown rate-limits, colonist works meanwhile — and **that degradation is
CORRECT for its intended case.**

> **BUT IT CANNOT DISTINGUISH "THE BED IS UNREACHABLE" FROM "TRAVEL IS BROKEN."**
> Both produce release → sweep → cooldown → work-meanwhile → **no complaint.**
> ★ **The designed degradation is the concealment** — a deliberate, tested
> mechanism silently absorbing a failure it was never built to cover.

**That is the defect. Not the watchdog, not the cooldown, not the sweep.**

## §2 — THE SPECIMEN (5b, measured, `087222a02b`)

**Seed 7 · job 33 · bed `(21872,16025,250)`**

| observation | value |
|---|---|
| start distance | **13.6** units |
| distance after full 10 s `STUCK_TIMEOUT` | ★ **22.46** units — **FARTHER** |
| lateral displacement | **wrong y-direction** |
| z | tracked **correctly** toward the bed's altitude |
| steer / target | correct (the bed) |
| `Goto` issued | **every tick** |
| cross-check | churn line's position **matches** LEGC-DIAG `actual_pos` exactly |

★ **This is not a stalled colonist. It is a MOVING colonist going the wrong way**
— which is a different failure from everything the watchdog was designed around
(pacing, queueing, mutual blocking). **And z tracking correctly while y goes wrong
is a strong, specific signature** worth its own hypothesis.

★ **5b raised and killed one hypothesis already:** `drive=Some(Work)` looked like
a stale arbiter, but `auton_travel_ok` (**~11248**) exempts self-jobs
unconditionally and the `Goto` fired every tick. **Dropped, correctly.**

## §2b — ★★★★★ THE SITE SURVEY (5b, `bbe4dd5698`) — AND THE SPECIMEN IS NOW IN DOUBT

**Eight timeouts · FOUR different targets** (2 bed-retries + 4 distinct mine
jobs) · x/z track whichever target is current · ★ **`y` NEVER MOVES:
y ≈ 16003 ± 0.55, every time.**

> **That shape is not terrain.** A physical obstruction fails **differently by
> approach bearing**; four targets at different x/z would jam in four different
> places. **A fixed `y` across all of them means something CLAMPS `y`, not blocks
> it.** Terrain there is unremarkable — Earth/Grass, one Rock or Wood neighbour.
> **Not a wall, not a pit.**

★ **Both prior candidates are dead** — obstruction (ruled out by the terrain
dump) and per-target geometry (ruled out by cross-target invariance).

### ★★★ AND IT MAY BE A FIXTURE ARTIFACT — TEST BEFORE ATTRIBUTING

**y ≈ 16003 is ONE BLOCK OUTSIDE `preempt_scenario`'s own flattened plateau**
(`y ∈ cy-12..=cy+12`, `cy = 16016` ⇒ seam at **16004**). **The colonist is pinned
at the seam between flattened and unflattened terrain, 8 times out of 8.**

> **A boundary a FIXTURE created is not a defect the GAME has.** If the seam
> causes this, **seed 7 is an instrument defect** and the row loses its specimen.

★ **The corpus has form:** 10.4% of a 144-seed failure set were fixture
false-failures, and **this is exactly their shape.** **Discriminator:** does
y-pinning appear in a scenario with **no flattened plateau** (`b5_scenario`)?
Only-at-a-seam ⇒ fixture. **`BASTION_STUCK_TERRAIN_DIAG` already emits what's
needed.** ★ **Run it AFTER §4.1 — the wiring does not depend on it.**

### ★★★ A SECOND MECHANISM, COMPOSING RATHER THAN COMPETING

**The `± 0.55` is the tell.** The stall-clock reset (R3 fix-1, **~11340**) needs
**≥ 1.0 block of NET progress**:

```rust
if active.reset_dist - sdist >= 1.0 { active.reset_dist = sdist; active.stuck_time = 0.0; }
```

> **A colonist oscillating with amplitude < 1.0 block NEVER zeroes the clock.**
> It is genuinely moving, `Goto` fires every tick, and it times out anyway.
> **The hysteresis exists to stop sub-block wobble from starving the watchdog —
> and here sub-block wobble is indistinguishable from a true stall.**

★ **The seam may CAUSE the oscillation while the hysteresis makes it FATAL.**
Two mechanisms, one symptom — **name both before repairing either**
(sufficient-blocker law). **Confirm `staged_at_anchor` was false**: this case
takes the ordinary pipeline, so the queue-release grace at **~11372** never
applies. *If it was true, that is a different and more interesting story.*

★ **NONE OF THIS WEAKENS THE ROW.** The row was approved on the **observability
criterion**, not on seed 7. **If seed 7 is a fixture seam, the classifier is what
would have said so in one line instead of a survey** — which is the row's case,
made by its own first specimen.

## §2c — ★★★★★★★★ THE SPECIMEN SET NOW SPANS THREE JOB KINDS, ONE SIGNATURE

**AUTON-2 step 1's fixture (2026-08-08 evening) contributed a third specimen, and
it re-shapes the row.**

| specimen | job kind | geometry | signature |
|---|---|---|---|
| seed 7 | ★ **`RestAt` (bed)** | *fixture plateau seam* | y-pinned, never closes |
| ★ **step-1 bed** | ★ **`RestAt`** | ★ **`gz+1` — ONE BLOCK above the floor** | **jump attempts, `sdist` never below ~17, zero net progress in 10s** |
| mine 26/27 | **`Mine`** | **one block short** | same class |
| chopfell egress | **`Chop`** | small-tree egress | same class |

> ## ★★★★★ **THREE JOB KINDS. ONE COMMON SIGNATURE: SMALL-STEP TRAVERSAL FAILING ON FIRST ATTEMPTS.**

★★★ **That is the row's opening design question, named by its own evidence rather
than by a hypothesis** — and it is **far stronger than the single-specimen framing
this spec opened with.**

### ★★ AND THE STEP-1 SPECIMEN IS THE BEST-INSTRUMENTED ONE

**100% first-attempt approach failure, success on retry** — ★ **a matched pair
ACROSS ATTEMPTS rather than across seeds**, *which is a cleaner control than
anything the corpus offered:* **same colonist, same target, same geometry, one
fails and the next succeeds.**

★ **`occupancy_interruptions` counts those retry cycles** *(its NAME is filed as a
defect — it counts failed-approach-then-retry, not interruptions of anything)*.

### ★★★★★ AND A HARD-WON CAVEAT ON THE SIGNAL

> **`bastion_bed_slot.occupant` is set at JOB CREATION, not at arrival** —
> *"reserve the bed at CREATION, not at arrival."*

★★★ **Any arrival assertion in this row MUST use the `ActiveJobState` transition
out of `Traveling`, or distance under the arrive tolerance — NEVER the bed slot.**
★ **Two DECISIONS entries were filed on that misreading before a per-tick trace
killed it.** *Zero code was written, because the queue put the trace before the
build.*

## §3-CORRECTION — ★★★★★★ THE FIELDS ARE ALREADY IN THE CORPUS. §3 BELOW IS WRONG.

**I scanned TOP-LEVEL KEYS ONLY and reported these fields as "reaching
nothing."** They are **already in the corpus**, nested inside
**`b5_mine_reachability_probe`**, together with `timeout_route_states`,
`route_next_idx_pinned`, and `path_exists_{step,jump,scramble}` **from both
spawn and the last timeout position.**

> ★ **"Enumerate the schema at EVERY level" is a rule I had written down, and I
> broke it in exactly the way it warns about. A NESTED field and an ABSENT field
> look identical to a top-level scan** — the campaign's own law, one level down.

**Consequence: §4.1 is much narrower than specced.** For **mine** jobs the
instrument exists and is rich. ★ **The real gap is probably SELF-JOBS** — the
probe is populated for mine targets, and seed 7's failure was a **bed**
(`RestAt`). **That would explain why the bed case stayed invisible while the mine
case has been richly instrumented for waves.** *Confirm before building.*

### ★★★★★ SEED 90 IS FULLY CHARACTERISED FROM DISK — NO RUN NEEDED

Diffing seed 90 across its regression boundary (wave17 pass → wave18 fail): **17
of 73 fields moved**, and the verdict names itself —
`failed_clauses: ['mine_cleared', 'mine_blocks_mined']`, **2 mine jobs unmined.**
**Stuck cell `[17989, 9263, 338]` — the exact dead-end column from Row A's scan.**

```
target [17989,9263,336]:   min_distance_to_target =  3.78
target [17989,9263,338]:   min_distance_to_target = 16.24
from BOTH spawn and last-timeout position:
    path_exists_step     = FALSE
    path_exists_jump     = TRUE
    path_exists_scramble = TRUE
timeout_route_states: route_exists TRUE, route_complete FALSE,
                      route_next_idx pinned at 3, 4, 4, 8, 8
```

> ★★★★★★ **A STEP PATH DOES NOT EXIST. A JUMP PATH DOES.** The route **exists and
> never completes.** And the colonist **got within 3.78 units** — by this row's
> own criterion that is **UNREACHED, not UNREACHABLE.**
>
> ★ **It is not a reachability failure. It is a LOCOMOTION-MODE failure** — the
> colonist got four units away and could not *step* the last bit. **Nothing in
> the pass/fail line says so**, which is this row's whole thesis, demonstrated by
> data that has been on disk since wave18.

**Supporting deltas, all consistent:** `timeouts_on_never_completed_jobs 0 → 6`,
`max_same_target_timeouts 4 → 5`, `mine_jobs_remaining 0 → 2`,
`cells_below_filled 0 → 1` at the stuck cell, `cavein_drop_cells 1 → 2`.
★ **The last two together are a lead worth its own look** — the cell's support
changed underneath it and a cave-in may have altered the approach geometry.
**Offered as a lead, not a conclusion.**

★ **The window read is NOT how this was answered, and could not have been:**
waves 14–18 predate the provenance audit, so wave17 has **no attested commit**.
**The specimen's own diag history answered it instead** — which is what the
architect predicted it would.

## §3 — ★★★★★ (SUPERSEDED BY §3-CORRECTION) THE DISCRIMINATORS ALREADY EXIST. WIRE THEM.

**Two purpose-built fields are in the engine and reach NOTHING.** Neither appears
in the 89-key corpus schema.

| field | site | what it answers |
|---|---|---|
| `min_distance_to_target: HashMap<Vec3<i32>, f32>` | **4071** | ★ **closest approach ever achieved**, per target, *"unconditional, every tick, never reset across claim attempts"* |
| `last_timeout_pos: HashMap<Vec3<i32>, Vec3<f32>>` | **4079** | where the failing attempt **actually stood** |

**`last_timeout_pos`'s own doc states this row's thesis, written 2026-07-30:**

> *"Lets the offline reachability probe run from where the failing attempt
> actually stood, not just from the colony's spawn point — **'reachable from
> spawn' and 'reachable from here' are different questions**, and only the second
> matches the observed failure."*

> ★★★ **THE DISCRIMINATOR IS `min_distance_to_target`.** Small ⇒ the colonist
> **got close**, so the target was reachable and travel failed — **UNREACHED**.
> Large ⇒ never approached — **UNREACHABLE**. **One float already computed every
> tick, for every target, and it has never left the process.**

★ **This is the third instance of instrument-built-then-never-surfaced** (after
`calls − emissions` and the by-kind gap). **The row is mostly WIRING, not
building** — the same shape as FARM-PAINT's *"wire the existing `z_extent`."*

## §4 — THE FIX, IN ORDER

**1. Surface both fields to the harness JSON** — per target, or aggregated as
`min_approach_ratio = min_distance / initial_distance` if the map is too wide for
the schema. ★ **Additive only**, so every historical baseline stays comparable
and holdcheck sees new fields (`--expect-new`).

**2. Classify every travel timeout at the moment it fires**, using the value the
engine already has:

```
UNREACHED    min_distance_to_target <= ARRIVE_DIST * k     // got close, failed
UNREACHABLE  min_distance_to_target  > <threshold>         // never approached
```

★ **`k` and the threshold must be MEASURED, not guessed** — take them from the
corpus's own distribution once §4.1 lands. **A guessed threshold makes a
classifier that reports its own assumption.**

**3. Only then, diagnose seed 7's mechanism.** The two live candidates, neither
yet tested:
- **genuine terrain obstruction** near that bed — the site-survey method that
  resolved the seed-1337/92 corner cells applies unchanged;
- **chaser oscillation** — for which this function already has grace handling
  elsewhere (`staged_at_anchor`, **~11372**), *but that path is gated on being
  staged at an anchor*, and a direct steer takes the ordinary pipeline.

★ **Do not fix before classifying.** If the corpus's timeouts are overwhelmingly
UNREACHABLE, seed 7 is a rare case and the row is small. **If they are
overwhelmingly UNREACHED, the travel system has a general defect and this is a
much larger row.** ★ **Nobody currently knows which**, and that is the whole
point.

## §5 — ACCEPTANCE

- ★ **PRIMARY, and it is not a bug fix:** the harness reports, per travel
  timeout, **which class it was.** *The row succeeds when the distinction is
  VISIBLE*, whether or not any travel behaviour changes.
- **Planted-failure test (required):** construct **both** cases — a genuinely
  unreachable target (`preempt_scenario`'s floating slab is already exactly
  this) and a reachable target the colonist fails to reach — and **assert they
  classify DIFFERENTLY.** ★ *A classifier that cannot separate the two fixtures
  is the defect restated, not fixed.*
- **Regression:** ENDURE still holds for the genuinely-unreachable case —
  `thrash_bounded (1..=3)`, meter decays, work happens, zero embeds. ★ **This row
  must not touch that guard.** §4c's error was proposing exactly that.
- **Budget:** the two maps are **already maintained every tick**; surfacing them
  is **read-at-settle**, not new per-tick work. **Zero added hot-path cost** —
  state it and hold to it, per the observer-effect law.
- **GATE FIELDS:** ★ `min_distance_to_target` / `last_timeout_pos` — **currently
  INSTRUMENT-GAP** (neither is in the fan schema). §4.1 closes the gap; **no gate
  claim is admissible before it does.**

## §6 — WHAT THIS ROW DOES **NOT** CLAIM

- **Not** that travel is broadly broken. **One measured specimen.** The
  classification in §4.2 is what would establish scope, and it hasn't run.
- **Not** that the watchdog, cooldown, or sweep are wrong. **They are correct and
  tested**; §4c's retraction is precisely about having claimed otherwise.
- **Not** that this merges with the rescue row. ★ **Seeds 71/90 fail OUTSIDE the
  rescue-refused set** — registered evidence that at least one *other* mechanism
  is in play. **Keep the rows split until data merges them, not narrative.**
