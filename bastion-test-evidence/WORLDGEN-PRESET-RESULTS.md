# FOUNDING PRESET ON REAL WORLDGEN — **RESULTS & ROW DISPOSITION**

Scored against `WORLDGEN-PRESET-PREREG.md` (committed `2777be0d21`, **before any
data existed**). Nothing below moved a threshold, a lattice, or a bar.

Engine tips: instrument `38270d8dcb`, `submerged` `287be840bd`.

---

## 0 · THE PRECONDITION (§9) — **HOLDS**

The driver reached real worldgen and anchored at `(15216.481, 16016.53, 419)`. The row
was not blocked.

## ⚠ AND THE FIRST CENSUS WAS **VOID** — a stale binary, failing silently

Every `spawn 8 <x> <y> <z>` in the first run sent `pos=15216.481, 16016.53` — the
anchor, not the lattice point. All nine attempts went to the same place.

The `no_overflow` `bastion_playtest.exe` was built **2026-08-11 18:39**; targeted spawn
was committed **2026-08-12 18:13**. The old parser took the count and **silently
discarded the coordinates**. Had the driver not echoed the position it actually sent,
the census would have read as *"all nine lattice points are identical"* — a perfectly
consistent lie, and one I would have had no reason to doubt.

Caught by the `targeted=` field being **absent** from the driver log. The debug driver
(built 18:10, source 18:05) does contain the feature, so the earlier bars scored with it
stand. The A4 plant/control pair did use the stale `no_overflow` driver — its conclusion
is unaffected, because both arms used identical positions and the one-colony boundary
does not depend on position.

---

## 1 · THE CENSUS — reported whole, as §7 required

Nine attempts, full accounting: **2 terrain + 6 colony_exists + 1 founded = 9.**

| origin | datum | min_dev | max_dev | branch | outcome |
|---|---|---|---|---|---|
| 15184, 15984 | 416 | **−5** | 0 | deviation | refused `terrain` |
| 15216, 15984 | 416 | **−6** | 0 | deviation | refused `terrain` |
| 15248, 15984 | 415 | −1 | 0 | **ok** | **FOUNDED** |
| *(remaining 6)* | — | — | — | — | `colony_exists` by design |

Three datums — **416, 416, 415** — on three sites 32 blocks apart. The arena's constant
400 is gone, and it did not merely change once: it **varies per site**.

## 2 · THE BARS

### W1 · **A FOUNDING SUCCEEDS ON REAL WORLDGEN** — ✅ PASS
`colony founded preset="v1" … datum=415 colonists=8 elements=stockpile,farm,bed
complete=true jobs=8 designated_regions=3`, **3** `plot placed` lines, and
`farm plot registered … unresolved=0`.

**PLANT A — `MAX_DATUM_DEVIATION: 1 → 0`.** Origin 3, which founded in the control at
`branch="ok"`, becomes `branch="deviation"` and is **refused**. RED on the claimed axis.
The plant run reports datums **416, 416, 415** — identical to the control — so the pair
is matched on *worldgen* as well as on profile and script.

### W2 · **THE DATUM IS DERIVED FROM REAL TERRAIN** — ✅ PASS
Measured datums across sites: **415, 416, 417, 418, 419**. All differ from the arena's
400, and all but one differ from the `pos.z=419` **hint** — at the founded site the
resolver moved the datum **4 blocks down** from the hint it was given.

**PLANT C — `resolve_datum`: `surface + 1 → surface`.** Every datum drops by exactly
one: 416→**415**, 416→**415**, 415→**414**. The founded emit reads `datum=414` against
the control's 415, and the farm region sinks to `min.z=413` from 414 — the preset
sitting one block into the ground, which is the failure this plant was registered to
produce, **named on the datum field itself** rather than on a downstream symptom.

### W3 · **A SLOPED SITE IS REFUSED, WITH A NUMBER** — ✅ PASS
Two sites refused `reason="terrain"` with `branch="deviation"` and worst deviations of
**−5** and **−6** — both ≥ 2, the derived threshold. Named columns: `(15177, 15980)` and
`(15214, 15980)`.

**PLANT B — `MAX_DATUM_DEVIATION: 1 → 64`.** Origin 1 reports **identical** relief
(`min_dev=−5`, `datum=416`, same worst column) but `branch="ok"`, and **founds**. The
same measurement, a different bound, the opposite verdict — which is exactly what
proves the refusal was the deviation test's doing and not a hidden special-case.

### W4 · **A WATER SITE** — ⚠ **VOID-BY-PREMISE at live tier** *(as §7 pre-registered)*
Twenty-four search origins; **8** reachable, all reporting `submerged=0`. **No water
lies within the chunk-loading radius of the spawn**, so no live water site could be
scored. Reported void, not dropped, and **not counted toward the row's score**.

But the search's instrument found the substantive thing anyway — see §3 F1.

### W5 · **A CHUNK-STRADDLING FOOTPRINT** — ✅ PASS *(weaker evidence, as registered)*
Origin `(15200, 16000)`, both coordinates on the 32-block pitch, footprint over **4
chunks**: `columns=60 resolved=60 … branch="deviation"`. **Every column resolved** —
no chunk-edge holes, and the branch is `deviation`, not `absence`. The site was then
refused for its terrain (`min_dev=−2`), which is terrain's business and not the bar's.

Its geometry claim is also asserted at unit level: `the_straddling_origin_really_spans_
four_chunks` proves this origin is genuinely the hard case, and that the arena-style
origin is **not** — otherwise the contrast would be empty.

**Score: 4 PASS, 1 VOID-BY-PREMISE, 0 FAIL.** Every plant fired.

---

## 3 · FINDINGS

### ⛔ F1 · **THE PRESET WILL FOUND ON A FLAT LAKEBED — there is no water gate**
`is_surface_terrain` does not match `Water`, so a lake column resolves its **bed**.
Sixty level columns under open water deviate by **zero** and are **accepted**: the
colony founds underwater.

Real worldgen usually hides this, because a lake is a depression and the *deviation*
test refuses the site for its **shape** — so the missing water test never shows. That is
precisely why it survived: **the gap is masked by a correlate, not covered by a check.**
`submerged` is now measured and **nothing consumes it**;
`a_flat_lakebed_is_accepted_because_nothing_consumes_submerged` pins today's behaviour
so that adding a gate fails loudly rather than the gap being rediscovered a third time.

*This also corrects §3 of the pre-registration.* I predicted water would refuse through
the **deviation** branch. On a sloped lakebed it would — but on a flat one it does not
refuse **at all**, which my prediction did not contemplate. The prediction was too
narrow, and I am recording that against myself rather than claiming the branch call.

### ⚠ F2 · **THE RELIEF EMIT DOES NOT FIRE ON EVERY ATTEMPT** — my own commit overclaimed
The emit sits inside the `Some(datum_z)` arm, so when `resolve_datum` finds no surface
at F itself the attempt is refused **with no relief emit at all**. **16 of 24** search
attempts produced no emit. My instrument commit says "on every attempt"; that is wrong,
and the absence-at-origin case is currently invisible to the instrument that was built
to make absence visible.

### ⚠ F3 · **A STALE BINARY DISCARDED TARGETED COORDINATES SILENTLY** — see §0.

### ★ F4 · **THE SITING RATE ON REAL TERRAIN — 4 of 13 (30.8%)**
Thirteen **distinct** real-worldgen origins were measured. Applying the pre-registered
condition (`min_dev ≥ −1` and `max_dev ≤ 1`):

| accepted | rejected |
|---|---|
| **4** — (15248,15984) −1/0 · (15216,16016) 0/0 · (15216,15888) 0/+1 · (15344,16016) −1/0 | **9** — deviations of −2, −2, −5, −6, −6, −6, −7, −8, and +4 |

**30.8%.** The preset's ±1 condition is strict but *not* prohibitive: roughly one real
site in three can carry it. This answers the open question the predecessor row could not
even ask, and it is a **measurement**, not a target — no bound was moved to produce it.

---

## 4 · ROW DISPOSITION — **PASS**

The predecessor row closed 7/8 **on the flat arena** and named exactly one caveat: the
arena tests the z-datum for free and tests nothing about slope, water, or chunk
boundaries. **That caveat is now discharged on all three axes** — slope measured and
refused with numbers, chunk boundaries crossed with `resolved=60`, and water reached at
fixture tier where it produced the row's most consequential finding.

**What is now true that was not before:** the preset founds on real terrain, its datum
is derived per-site rather than constant, its refusals carry magnitudes and a named
branch instead of the bare word `terrain`, and its siting rate is a measured **30.8%**.

**What I decline to claim:**
- **Not** that water is handled. F1 says the opposite, and W4's live tier is void.
- **Not** that W5's evidence equals W1–W3's. It has no code-mutation plant — registered
  as weaker in advance, and reported as weaker here.
- **Not** that 30.8% generalises beyond this world seed and this 13-site sample. It is a
  measurement of *these* sites, and it is reported as one.

**Successor rows, in the order I would take them:**
1. **The water gate** (F1) — `submerged` already exists and is already measured; the
   work is deciding the policy and giving it a plant.
2. **Move the relief emit out of the `Some(datum_z)` arm** (F2) so absence-at-origin is
   witnessed. Small, and it repairs an instrument this row depends on.
3. **A driver-binary freshness guard** (F3) — the failure was silent, and silence is the
   part worth fixing.
