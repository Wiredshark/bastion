# FOUNDING PRESET ON REAL WORLDGEN — **PRE-REGISTRATION**

**Written before any data exists.** No founding has been attempted on real worldgen at
the time of writing. Every threshold below is **derived from the system's own
constants**, cited at file::symbol, and none of them may move after a result is seen.

Predecessor row: FOUNDING PRESET v1, closed PASS at 7/8 — **on the flat arena only**.
Its disposition names this row as the successor and states the caveat this row exists to
discharge: *the arena tests the z-datum for free; it does not test slope, water, or
chunk boundaries.*

---

## 1 · THE GEOMETRY, DERIVED

`bastion_founding_preset.rs::FOUNDING_PRESET_V1` gives three elements. Their xy offsets:

| role | x offsets | y offsets |
|---|---|---|
| Stockpile | −2 … +2 | −4 … +1 |
| Farm | −7 … −3 | −4 … +1 |
| Bed | −3 … −2 | −3 … −2 |

Farm (−7…−3) and Stockpile (−2…+2) are **contiguous**, and Bed is strictly inside their
union. So `footprint_columns` yields exactly the bounding box:

> **x ∈ [ox−7, ox+2], y ∈ [oy−4, oy+1] — 10 × 6 = 60 columns.**

Bed adds zero new columns. **60** is the denominator for everything below; any emit that
reports a different column count falsifies this derivation and must be reported, not
reconciled.

## 2 · THE ACCEPTANCE CONDITION, DERIVED

`validate_site` refuses when `(surface + 1 - origin.z).abs() > MAX_DATUM_DEVIATION`, and
`MAX_DATUM_DEVIATION = 1` (`bastion_founding_preset.rs:55`). The datum is
`column_surface_z(origin) + 1`, so the `+1`s cancel:

> **A site is accepted iff every one of its 60 columns has a surface within ±1 block of
> the origin column's surface.** These are integers, so `> 1` means **≥ 2**.

Consequences, all fixed now:

- **Total relief over the 60-column patch must be ≤ 2 blocks**, with the origin column
  central. This is a *strong* constraint on real terrain. If real worldgen turns out to
  satisfy it only rarely, **that is a finding about the preset's siting policy**, not a
  test failure to be tuned away.
- **Uniform-slope refusal threshold.** The longest lever arm is the farm's `dx = −7`;
  deviation reaches 2 at gradient `g ≥ 2/7 = 0.2857` blocks/column = **15.95°**. On the
  `+x` side the arm is only 2, so that direction needs `g ≥ 1.0` = 45°. **The binding
  direction is −x, the farm side** — a site can therefore refuse for slope that a
  centred eyeball would call symmetric.

## 3 · WHAT WATER WILL DO — **registered mechanism, not just outcome**

`is_surface_terrain` (`bastion_jobs.rs:2206`) matches Rock/WeakRock/GlowingRock/
GlowingWeakRock/Grass/Snow/ArtSnow/Earth/Sand/Ice. **`Water` is not in the list.**
`column_surface_z` scans `hint−96 ..= hint+48` downward (`SURFACE_SCAN_DOWN = 96`,
`SURFACE_SCAN_UP = 48`) for the topmost such block.

> **PREDICTION:** a water column does **not** return `None`. It returns the **lakebed**,
> which lies far below the datum, so the refusal fires through the **deviation branch**
> (`validate_site:251`), **not** the absence branch (`:255`). The absence branch is
> reachable only for water deeper than 96 blocks, or an unloaded chunk.

Both branches emit `reason="terrain"`, so **outcome alone cannot tell them apart.** The
instrument in §5 exists so this prediction is testable. If the water refusal fires
through the absence branch, **my model is wrong and I report it as a finding** against
this document.

## 4 · THE CHUNK-BOUNDARY CASE, DERIVED

`TERRAIN_CHUNK_BLOCKS_LG = 5` ⇒ `TerrainChunkSize::RECT_SIZE = 1<<5 = 32`.
Choosing `ox ≡ 0 (mod 32)` puts `ox−7` in the previous chunk and `ox+2` in this one;
`oy ≡ 0 (mod 32)` does the same in y. So:

> **An origin with both coordinates ≡ 0 (mod 32) makes the 60-column footprint span
> exactly 4 chunks.** That is the W5 origin, and it is chosen by this rule, not by taste.

## 5 · THE INSTRUMENT I WILL BUILD FIRST — **one producer, two consumers**

Today a refusal says `reason="terrain"` and a column, with **no number**. That cannot
distinguish §3's two branches, cannot report relief, and cannot tell a 2-block deviation
from a 90-block one. So, before any data:

`validate_site` will **return the relief it already computes** — resolved-column count,
min/max deviation, and the worst column — on **both** the `Ok` and `Err` paths. The
founding decision and the new emit then consume **the same value from the same
computation**. I am explicitly *not* writing a second function that re-derives relief
alongside the real one: that is the F8 defect (a test that re-implements its subject),
and it is why one of the eight planted mutations failed to fire earlier this week.

Named emit, on **every** attempt, success or refusal:

```
bastion: founding site relief origin=.. datum=.. columns=60 resolved=..
         min_dev=.. max_dev=.. worst=(x,y) branch=deviation|absence
```

`branch` is what makes §3 falsifiable.

## 6 · THE BARS

Each bar names its witness emit and its planted failure. **A bar that was never red
proves nothing**, so every plant below is a real code mutation, red-demonstrated, then
reverted and re-run green — and each plant's control matches on **system and axis**
(same site, same script, same profile, one line different).

### W1 · **A FOUNDING SUCCEEDS ON REAL WORLDGEN**
- **PASS:** `colony founded preset="v1" … complete=true elements=stockpile,farm,bed`,
  three `founding preset plot placed` lines, `farm plot registered … unresolved=0`.
- **PLANT:** `MAX_DATUM_DEVIATION: 1 → 0`. The same site must then **refuse**. This is
  the sharpest available plant because it moves the *acceptance constant itself* while
  holding the site fixed.

### W2 · **THE DATUM IS DERIVED FROM REAL TERRAIN, NOT A CONSTANT**
- **PASS:** the founded emit's `datum` **≠ 400** (the arena's constant) and equals the
  W1 site's reported origin surface **+ 1**.
- **PLANT:** `resolve_datum` returns `surface` instead of `surface + 1`. Registered
  prediction: the preset then sits one block into the ground and the run goes red **on
  the datum field**, not merely on some downstream symptom.

### W3 · **A SLOPED SITE IS REFUSED, WITH A NUMBER**
- **PASS:** `founding refused reason="terrain"` at a sloped origin, **and** the relief
  emit shows `max_dev ≥ 2` with `branch=deviation`.
- **PLANT:** `MAX_DATUM_DEVIATION: 1 → 64`. The **same sloped site must now be
  accepted.** This is the matched control in the opposite direction from W1's: W1 proves
  acceptance is not unconditional, W3 proves refusal is not unconditional.

### W4 · **A WATER SITE IS REFUSED — THROUGH THE PREDICTED BRANCH**
- **PASS:** `reason="terrain"` **and** `branch=deviation` **and** `max_dev` far exceeding
  any dry site's, consistent with a lakebed.
- **FINDING (not failure):** if `branch=absence`, §3's prediction is refuted and the row
  reports that, having registered it here first.
- **PLANT:** shares W3's `MAX_DATUM_DEVIATION` plant — with the bound at 64 a shallow
  water site should be **accepted**, proving the refusal is the deviation test's doing
  and not a separate water special-case. *(There is no water special-case in the code I
  read; this plant is what makes that reading falsifiable rather than asserted.)*

### W5 · **A CHUNK-STRADDLING FOOTPRINT IS DECIDED BY TERRAIN, NOT BY CHUNK EDGES**
- **Origin:** both coordinates ≡ 0 (mod 32), per §4 — footprint spans 4 chunks.
- **PASS:** the relief emit reports `resolved=60` (**every** column resolved, no
  chunk-edge holes) and `farm plot registered … unresolved=0`. Whether the site is then
  founded or refused is terrain's business; the bar is that **no column fails for being
  on the far side of a boundary.**
- **PLANT:** none available as a code mutation without inventing a defect this code does
  not have. Instead the **discriminator is `resolved`**: if any chunk-edge column failed
  to resolve, `resolved < 60` reports it by name. I register now that **W5's evidence is
  weaker than W1–W4's** and will say so in the disposition rather than presenting five
  equally-supported bars.

## 7 · SITE SELECTION — **the rule, fixed before any site is seen**

Selecting sites after seeing results would be optional stopping wearing a lab coat. So:

- Candidate origins are the **3×3 lattice at 32-block spacing** (§4's chunk pitch)
  centred on **(15216, 16016)** — the coordinate the F8-C1 control already proved is
  real-worldgen land the driver can reach. That is
  `x ∈ {15184, 15216, 15248} × y ∈ {15984, 16016, 16048}`, attempted in ascending
  `(y, x)` order.
- **A refusal does not consume the one-colony boundary** (`colony_exists` is checked
  first and only a *success* creates colonists), so a single world can absorb every
  refusal. The run therefore walks the lattice and **stops at the first success** — that
  origin is W1's and W2's site.
- **Every attempt's relief emit is recorded**, including the ones before the first
  success. The lattice is not a search for a green; it is a census, and the census is
  reported whole.
- **W3's sloped site** = the lattice origin with the **largest `max_dev` that is still
  finite and dry**. **W4's water site** is not in the lattice; it must be located, and if
  no water site can be reached, **W4 is VOID-BY-PREMISE and reported as void — never
  silently dropped, and never counted toward the row's score.**
- Each bar runs on its **own fresh throwaway `VELOREN_USERDATA`**, never Ben's world.

## 8 · WHAT I WILL **NOT** DO AT SCORING TIME

Written now, while no data exists to tempt me:

1. **I will not move `MAX_DATUM_DEVIATION`, the lattice, or the ±1 acceptance condition
   after seeing a result.** If real worldgen refuses every lattice origin, the row's
   output is *"the preset cannot site itself on real terrain"* — a finding, and a
   perfectly good row output. It is **not** a licence to widen the bound until something
   greens.
2. **I will not accept a refusal as W3/W4's pass on `reason="terrain"` alone.** The
   reason string is shared by both branches and by every relief magnitude. Without the
   number and the branch, the bar is vacuous — and I would rather ship the instrument
   than a vacuous green.
3. **I will not report a bar whose plant did not fire as a pass.** If a plant fails to go
   red, the bar is UNSOUND (as A2's was), and it gets retired and replaced, not scored.
4. **I will not let a harness-green stand in for the live path.** Every bar here is
   scored on a real server's log with the named emit observed. Unit tests on
   `validate_site` are corroboration, not the bar.
5. **I will not present W5 as equal in strength to W1–W4** — see §6 W5.
6. **I will not reuse a userdata across bars**, so no result can be an artefact of a
   colony or a save left by the previous one. *(The A4 leg proved how quietly that goes
   wrong: rtsim persists only every 60 s, so a hard kill silently voids a restart test.)*

## 9 · THE ONE THING THAT WOULD VOID THIS WHOLE ROW

If the driver cannot reach real-worldgen coordinates and spawn there, every bar is
untestable. The F8-C1 chop control already demonstrated a live real-worldgen session at
(15216.5, 16016.5, 419), so this is **expected to hold** — but it is checked **first**,
as an explicit precondition, and if it fails the row is blocked at that line and reported
as blocked rather than half-run.
