# ITEM 40 (colony scale) — DISPOSITION: **3/3 bars PASS**, and the headline is a defect the bars were not looking for

Scored against `ITEM40-SCALE-PREREGISTRATION.md`. Three arms run
**sequentially on slot 0 on a quiet machine**, all attested fresh with
`dirty .rs 0`, differing in one declared variable.

## Bar 1 — the triplet is COMPARABLE: **PASS**

All three fresh, same binary, same script, sequential on one host. The declared
variable actually moved: `total=8`, `total=16`, `total=32` in the census. The
first attempt at this measurement was VOID precisely because arms shared the
host; this one did not.

## Bar 2 — cost is MEASURED: **PASS**

| colonists | p50 µs | p95 µs | max µs | jobs |
|---|---|---|---|---|
| 8 | 65 | 154 | 297 | 7 |
| 16 | 56 | 200 | 492 | 20 |
| 32 | 72 | 299 | 613 | 12 |

**The median is flat; the TAIL scales.** p50 sits at 65/56/72 µs — a 9 µs spread
on a ~65 µs median, i.e. noise, and non-monotone, which the pre-registration
named as a possible contention signal. It is not: p95 and max are cleanly
monotone (154→200→299, 297→492→613), which is the shape you get when
per-colonist work lives in periodic passes rather than every tick. A contention
leak would have disturbed all three statistics, not just the median.

Cost roughly **doubles for a 4× population** — sub-linear, and comfortably
inside budget at every size.

## Bar 3 — the colony still WORKS at 32: **PASS**

It does not merely keep ticking: it **completes its work**. Beds built were
**8 / 16 / 32** — one per colonist at every scale, so the founding plan scaled
with population and the colony executed all of it.

## ★ THE HEADLINE, which no bar asked for

| colonists | mean working | working share |
|---|---|---|
| 8 | 2.03 | **25%** |
| 16 | 2.21 | **14%** |
| 32 | 2.77 | **9%** |

**The number of colonists working at once is nearly flat while population
quadruples.** A colony of 32 employs about the same two-to-three people as a
colony of 8; the other 30 stand idle. Stuck stays low (~0.6) at every size, so
this is not the pathfinding failure — they are not *stuck*, they are
*unemployed*.

The work existed and got done (32 beds), so this is throughput, not paralysis:
the board holds only a handful of live jobs at a time (`jobs=7/20/12`), because
generation is quota-capped per firing on a fixed cadence. Population grew;
the rate at which work becomes *claimable* did not.

**This is why bar 3 was written as "must still work" rather than "must not
crash"** — and it still under-asked. A row that only checked cost would have
reported a clean pass and missed that the colony's usable labour is capped near
three people regardless of size.

## Registered as the next row, not fixed here

Job generation throughput should scale with population the way the generators'
per-colonist caps already imply (`MINE_GEN_JOBS_PER_COLONIST`,
`BUILD_GEN_JOBS_PER_COLONIST` are per-colonist; the *cadence* is not). Stated
as a measurement, not a diagnosis: I have the symptom precisely and have not
yet proven which of cadence, quota or claim arbitration is the binding
constraint. Naming the wrong one would be the same mistake this sweep keeps
finding.
