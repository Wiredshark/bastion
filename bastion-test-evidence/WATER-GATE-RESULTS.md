# THE WATER GATE — **RESULTS & ROW DISPOSITION**

Scored against `WATER-GATE-PREREG.md` (`551482516c`). Engine tip `8aff058228`.
Discharges **F1** from `WORLDGEN-PRESET-RESULTS.md`.

## THE SCORE — **4 PASS, 0 FAIL**

| bar | verdict | evidence |
|---|---|---|
| **G1** flat lakebed refused | ✅ PASS | `branch="submerged"`, `Err(Submerged, column)` — with `max_dev=0` |
| **G1b** one column is enough | ✅ PASS | partial flood, still perfectly flat, still refused |
| **G2** dry land still founds | ✅ PASS | live census **byte-identical** to the control |
| **G3** ordering holds | ✅ PASS | sloped **and** submerged ⇒ `reason="terrain"` |

| plant | required red | observed |
|---|---|---|
| gate removed | G1 red | G1 **and** G1b red; G3 + dry-ground green |
| threshold → all 60 columns | isolate the threshold | **G1 stays GREEN, G1b goes RED** |

Restored: **123/123**, live census unchanged.

## WHY G2 WAS THE BAR THAT MATTERED

The worldgen row measured only **4 of 13** real origins as siteable. A water gate that
also refused dry ground would have looked perfect at fixture tier and been catastrophic
live — it would simply have made founding rarer, and the census is the only instrument
that could tell the difference. It reproduces **exactly**: datums 416/416/415, deviations
−5/−6/−1, the same worst columns, founded at (15248, 15984) `datum=415`, the same
**6 colony_exists + 2 terrain**.

## ★ THE SECOND PLANT WAS SHARPER THAN THE ONE I REGISTERED

The prereg's plant 2 was an *unreachable* threshold (`submerged > 60`) — but that is
indistinguishable from plant 1: both simply switch the gate off, and both redden the same
two bars. It would have added no information.

So I ran a plant that **does** discriminate: move the threshold to **all 60 columns**.
The full-lakebed bar stays **green** while the single-column bar goes **red** — which
isolates *the threshold value of 1* from *the gate's existence*. That is the difference
between "there is a water check" and "the water check is the right one", and only the
second plant can see it. **Registered plants are a floor, not a ceiling.**

## ⚠ AND G1b's SPECIMEN WAS WRONG THE FIRST TIME

My first partial-flood world was not partial: the footprint at (15216, 16016) sits inside
**one chunk**, so flooding "one chunk" floods all 60 columns. The test failed on its own
**specimen assertion** (`submerged` between 1 and 59) before it could score anything —
which is the pre-registration discipline working exactly as intended, and the third time
this session that a footprint's single-chunk geometry has bitten. The test now asserts
its specimen is genuinely partial *before* scoring, and uses the straddling origin.

## WHAT I DECLINE TO CLAIM

- **Not** a live submerged refusal. **G4 was registered as a non-regression bar, not a
  positive water bar**, because the worldgen row established that no water lies within
  the chunk-loading radius (8 reachable probes, all `submerged=0`). The gate's positive
  case is proven at **fixture tier only**, and that is stated rather than dressed up.
- **Not** that `submerged` sees every kind of water. It reads the cell directly above the
  resolved surface via `is_liquid()`. A column under *deep* water still refuses — via the
  deviation branch, or `absence` past the 96-block scan — but for a different reason,
  which the emit now names.

## SESSION QUEUE STATE

1. ✅ Founding preset on real worldgen — PASS (`f51213cc4c`)
2. ✅ Arena trees / F8-C1 — CLOSED (`793df9401a`)
3. ✅ S1 sentinel scored-bar — PASS (`dcc0b950e9`)
4. ✅ **The water gate — PASS**, this document
5. → Next: **the relief emit's `Some(datum_z)` blind spot** (F2 — an absence at the
   origin column produces no emit at all), then **a driver-binary freshness guard** (F3),
   then the roadmap's open items.
