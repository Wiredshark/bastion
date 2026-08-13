# THE WATER GATE — **PRE-REGISTRATION**

Written before any code change. Discharges **F1** from the worldgen row
(`WORLDGEN-PRESET-RESULTS.md`, `f51213cc4c`).

## 1 · THE FINDING

**The preset will found on a flat lakebed.** `is_surface_terrain` does not match `Water`,
so a lake column resolves its **bed**; 60 level columns under open water deviate by
**zero** and are accepted. `submerged` is measured by `survey_site` and **nothing
consumes it** — the row that added the field deliberately left it unconsumed and pinned
today's behaviour with
`a_flat_lakebed_is_accepted_because_nothing_consumes_submerged`.

Real worldgen usually masks this: a lake is a depression, so the *deviation* test refuses
the site for its **shape** and the missing water test never shows. **The gap is hidden by
a correlate, not covered by a check** — which is exactly why it needs a gate rather than
a comment.

## 2 · THE POLICY, AND ITS DERIVATION

The preset's three elements — stockpile, farm, bed — are **surface structures**. There is
no partially-submerged design: a colonist standing in the farm is standing in water or is
not. So:

> **A site is refused if ANY of its 60 columns is submerged** (`submerged > 0`).

Not a fraction, not a majority. A threshold anywhere between 1 and 60 would be a number I
invented; **1 is the only value derivable from "these plots sit on the ground"**, and it
is fixed here before any run.

## 3 · THE REFUSAL MUST HAVE ITS OWN NAME

`FoundingRefusal` currently has `ColonyExists` and `Terrain`, and `reason()` is the string
the bars read. A submerged refusal reported as `terrain` would be **indistinguishable
from slope** in every log — the same collapse the worldgen row had to build `branch=` to
escape. So this row adds `FoundingRefusal::Submerged`, `reason() = "submerged"`, and its
own `player_message`, per refusal-needs-refusal-aware-consumers.

**Ordering, stated now:** the water test runs **after** the deviation test. A site that is
both sloped and submerged reports `terrain`, because the deviation test is the older,
cheaper, and more common refusal — and because changing the reason of an existing refusal
would break bars that already read it.

## 4 · THE BARS

### G1 · **A FLAT LAKEBED IS REFUSED** — fixture tier
- **PASS:** `flooded_world` (60 columns, `submerged=60`, `max_dev=0`) ⇒
  `verdict()` is `Err(Submerged)`.
- This is the **inversion of an existing passing test**. That test asserted today's
  behaviour precisely so this change would fail it loudly; when it fails, I update it
  deliberately and say so in the disposition — **the test doing its job is the evidence,
  not an obstacle.**

### G2 · **DRY LAND STILL FOUNDS** — the non-regression, and it is the risk
- The worldgen row measured **4 of 13** real origins as acceptable. A water gate that
  also refused dry ground would be invisible at fixture tier and catastrophic live.
- **PASS:** the same census lattice still founds at **(15248, 15984), datum=415**, with
  the same 2 terrain refusals — **byte-identical to the control** already recorded.

### G3 · **THE ORDERING HOLDS**
- **PASS:** a *sloped* dry site still reports `reason="terrain"`, not `"submerged"`.

### G4 · **LIVE** — gate-must-test-live-path
- **PASS:** the relief emit shows `submerged=0` on the census sites and the founding
  still succeeds live. A live *submerged refusal* is **VOID-BY-PREMISE** unless water is
  reachable — the worldgen row established that **no water lies within the chunk-loading
  radius** (8 reachable probes, all `submerged=0`). I register that expectation now:
  **G4 is a non-regression bar, not a positive water bar**, and I will not dress it up as
  one.

### PLANTS
1. **Gate removed** ⇒ **G1 red** (the lakebed founds again — F1's behaviour returns).
2. **Threshold moved to `submerged > 60`** (unreachable) ⇒ **G1 red** while G2 stays
   green — proving G1 tests the *threshold*, not merely the field's existence.

## 5 · WHAT I WILL **NOT** DO

1. **I will not soften the threshold if G2 goes red.** If refusing any submerged column
   also refuses dry sites, the *measurement* is wrong, not the policy — and I fix
   `submerged` rather than raising the bound until the census greens.
2. **I will not claim a live submerged refusal** without a reachable water site. Fixture
   tier is where this bar lives today, and the disposition will say so.
3. **I will not reuse `Terrain`** for the new refusal. A refusal that cannot be
   distinguished in the log is the defect this row exists to fix, one level up.
4. **I will not quietly edit** `a_flat_lakebed_is_accepted_because_nothing_consumes_
   submerged` out of existence. It is inverted, renamed to say what is now true, and the
   inversion is reported.
