# A3 (THE EAT LOOP) AT n=4 — **RESULTS & ROW DISPOSITION**

Scored against `A3-N4-PREREG.md` (`a30b82d619`). Engine tip `f6e1707988` — no code change.
Attested before the leg: HEAD `f6e1707988`, `dirty .rs : 0`, binary fresh.

## THE SCORE — **3 PASS, 0 FAIL** *(and E2 passed for the wrong reason — see below)*

| bar | verdict | evidence |
|---|---|---|
| **E3** the population changed *(precondition)* | ✅ PASS | `colonists=4` |
| **E1** the loop still closes | ✅ PASS | `tilled 30 · sown 10 · harvested 10 · ATE 2` |
| **E2** ending `food_stock` > 0 | ✅ PASS | ending **2** (n=8 ended **0**) |

## THE COMPARISON

| | n = 8 (A3) | **n = 4 (this row)** |
|---|---|---|
| tilled / sown / harvested | 30 / 10 / 10 | **30 / 10 / 10** — identical |
| ate | 3 | **2** |
| food_stock trajectory | 0 → **6** → 4 → 0 | 0 → **2** |
| ending food_stock | 0 | ✅ **2** |

**The loop closes at half the population.** Till → sow → harvest → eat, live, on founding
stock, with the same derived 28,000-tick window — which was **not** shortened, because A3
went PARTIAL → PASS precisely by deriving it.

## ⚠ E2 PASSED — AND ITS STATED MECHANISM IS REFUTED

I registered: *"the farm's yield is fixed by the plot while consumption scales with
head-count, so at n=4 the ending stock should EXCEED the 0 seen at n=8."* Ending stock
was 2. **The bar passes.**

But the **peak** moved the wrong way: **6 at n=8, 2 at n=4** — with **identical harvest
counts (10 and 10)**. A fixed yield feeding fewer mouths cannot produce a *lower* peak.
So the arithmetic premise is incomplete: **what reaches `food_stock` is not the harvest,
it is the harvest that has been HAULED IN**, and hauling is done by the same colonists —
so the yield-into-stock rate is itself population-scaled, in the *opposite* direction to
consumption.

**Leading candidate, deliberately not asserted:** halved haul throughput leaves more of
the harvest on the ground at window end. It fits, and *a story that fits the specimen is
the weakest kind of evidence* — the peak difference is **reported as an observation, and
its mechanism is registered as untested.** It needs its own bar (a haul-completion count
per population would decide it in one run).

**The honest reading:** E2's *prediction* held and E2's *reasoning* did not. Recording
both, because a passed bar with a broken rationale is exactly the kind of thing that
silently becomes a "known" fact.

## ⚠ ONE FIELD A3 REPORTED THAT THIS RUN CANNOT

A3 reported `min hunger: 0.1509` against the 0.2 gate. **This run has 0 presence-diag
samples** — `BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG` was not set, so hunger is **not
observable here** and is not reported. The eat count is direct evidence the gate was
crossed; the margin is not measured. Stated rather than quietly omitted.

## WHAT I DECLINE TO CLAIM

- **Not** that `ate 2` vs `ate 3` is a finding. Fewer colonists crossing the gate inside
  one window is expected, and the prereg said E1 asks whether the loop **closes**, not how
  many meals it serves.
- **Not** that the peak difference is explained. See above — mechanism untested.
- **Not** that A3 now holds at all populations. It holds at 4 and 8. **6 — the value the
  widget shipped — remains interpolated at every bar in this program.**

## SESSION QUEUE STATE — twelve rows closed

1–7 as recorded · 8. Cancel across restart · 9. Run attestation · 10. Founding colonist
count · 11. Population sensitivity (work-pull) · 12. **A3 at n=4**, this document.

**Next:** the chop yield (prereg `1b08c80864`) — witness written for the felling stagger,
awaiting a build slot; then the haul-throughput question this row opened.
