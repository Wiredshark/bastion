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

## §3 — ★★★★★ THE DISCRIMINATORS ALREADY EXIST. WIRE THEM.

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
