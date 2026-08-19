# BIMODAL COLONY OUTCOME — CONFIRMED against its own falsifier

> ## ★★ SUPERSEDED IN PART (2026-08-19): **the gap is NOT empty.**
> A later run finished at **exactly 50 maturations** — inside the registered
> 50–500 band — so the registered falsifier fired and **"bimodal" is withdrawn
> as a description of the MECHANISM**. The two clusters are still real; the claim
> that nothing lands between them is not. See `DROPTOSS-RESULTS.md`.
> Everything below was true of the corpus as it stood (n=26) and is kept intact.


Both pre-registered questions answered on one 6-host fan, 12 runs, all arms
returning both twins.

## 1. The anchor fix — **DEMONSTRATED**

The previous fan was scored **VOID** for this: bar A passed 6/6 but bar B
(*the fix was NEEDED*) was satisfied by none, because `Pos` arrived at tick 47-48
every run — inside the old 60-tick spin. `BASTION_PLANT_POS_DELAY=90` forces it.

| registered bar | twin1 | twin2 |
|---|---|---|
| `PLANT ACTIVE` logged (plant fired — precondition) | ✅ | ✅ |
| **`OLD BEHAVIOUR WOULD HAVE ANCHORED AT THE WORLD ORIGIN`** | ✅ | ✅ |
| anchored at the colony `(15216.5, 16016.5)` | ✅ | ✅ |
| promoted class **304 = core + real viewer** | ✅ | ✅ |
| **origin-block chunks** | **0** | **0** |

Registered outcome: *"logs the counterfactual **and** anchors at the colony ⇒
fix DEMONSTRATED."* Both hold on both twins. **The fix works, and now it is
shown working on the case it was built for rather than on hosts where the race
never fired.**

## 2. Bimodality — **CONFIRMED, and the falsifier could have killed it**

Registered before launch: `THRIVE > 500`, `COLLAPSE < 50`, **any run landing in
50–500 withdraws the claim.**

| | |
|---|---|
| runs | **14** (10 new + 4 banked) |
| colonist name-hash — every run | `d6a994cb04e6` (**precondition holds**) |
| **THRIVE** | **7** — range **936 … 2,015** |
| **COLLAPSE** | **7** — range **8 … 46** |
| **runs in the registered gap 50–500** | **0** |

**The observed gap is wider than the registered one: 46 → 936, a factor of 20.**
Ten fresh runs on a continuum would very likely have produced a mid-range point;
none did.

★ Identical world seed, identical founding tick, identical colonists, identical
script — and the colony either builds a working farm loop or never gets food
above 46 in 271,000 ticks. **One collapsed run peaked at `food_stock = 2`.**

## What I checked before believing it

**Within-pair structure.** Five of six twin pairs split one-thrive/one-collapse,
which looks like a host-level mechanism. It is not significant: **P(≥5 of 6 |
fair coin) = 0.109**, and the sixth pair collapsed on both sides. I had that
counterexample in hand before I ran the test and still had to compute it to stop
myself calling the pattern systematic.

**Twin-order effect.** twin1 thrives 5/8, twin2 2/6 — **n far too small**; no
order effect is claimed.

**Shared state between twins.** `run-pit.sh:409` does `rm -rf "$WT/userdata-$TAG"`
before every run, so twin2 is not a restart of twin1.

## Consequences

1. **A single endurance run in this scenario carries almost no information** —
   the same configuration returns 8 or 2,015 maturations, with nothing between.
   Any conclusion drawn from one long run needs re-reading.
2. **#114 stays withdrawn.** Its registered bar (maturations > 32 ⇒ starvation is
   the cause) sits *inside* the collapse cluster: 7 of 14 runs fall below it and
   7 sit 30–60× above it. The bar does not discriminate what it was written to
   discriminate — it discriminates **which regime the run landed in**.
3. **The certification's failure is not an edge case.** Non-determinism with the
   barrier on does not merely perturb timing; it selects between two colony
   outcomes that differ by 20×.

---

# ★★ THE REGIME SELECTOR IS THE MATERIALS BLOCKAGE — 14/14

Chasing what picks the regime, on the driver's `COLONY` samples.

**Predictor: does `blocked_materials` reach 0 before the final sample?**

| | cleared | never cleared |
|---|---|---|
| **THRIVE** (n=7) | **7** | 0 |
| **COLLAPSE** (n=7) | 0 | **7** |

**14 of 14.** Collapsed runs sit pinned at **28–30 blocked jobs for the entire
271,000 ticks**; thriving runs drop to **0 and stay there**.

The *starting* backlog does **not** decide it — `start=8` gives 3 THRIVE / 1
COLLAPSE, `start=28` gives 4 / 6. **What matters is whether the backlog ever
clears, not how big it begins.**

## ★ This RELOCATES the question; it does not answer it

The correlation is partly **definitional** and I am not going to dress it up as
a mechanism: sowing consumes a seed, a seed is a material, so "materials stay
blocked" and "no crops mature" are close to two descriptions of one state.

What it genuinely buys is a **named resource and a named event**. The question
moves from the useless *"why does the colony collapse?"* to the sharp
*"why does the materials blockage clear in exactly half of otherwise identical
runs?"* — and it puts the answer in the job/materials path rather than in
farming, pathing, or the food economy.

★ It also partly **rehabilitates #114**. Its hypothesis — seed starvation drives
the materials refusals — points at the right subsystem. Its **bar** was wrong:
`maturations > 32` measures the *downstream consequence*, so it reports which
regime a run landed in rather than whether starvation is causal. Right
subsystem, wrong observable.

## The next question, stated for whoever takes it

`blocked_materials` sits at 28–30 in collapsed runs across the whole run. **What
are those ~30 jobs blocked on, and what does the clearing event look like in the
7 runs where it happens?** That is a single-log read on a banked corpus, and the
runs to read are named above.

---

# ★★★ THE CHAIN, READ END TO END

Both regimes start **identically**: 8 plots sown, 8 matured, 8 harvested. In the
collapsed run that entire history spans **15 seconds of wall time**, and then the
farm is dead for the remaining ~40 minutes of the run.

| | COLLAPSE | THRIVE |
|---|---|---|
| sown | **8** | 1,975 |
| matured / harvested | 8 / 8 | 1,948 / 1,948 |
| **haul** | **5** | **9,167** |
| job claimed | 48 | 6,664 |
| designations swept unclaimed | **276** | 8 |
| claim census (final) | considered 304, **eligible 0**, refused 304 | considered 54, eligible 7, assigned 2 |
| refusal reasons | `self_job_kind` 64 + **`materials` 240** | `self_job_kind` 46, **`materials` 0** |

★ The claim census reproduces **#114's original signature exactly** —
*"every candidate refused, and most refusals were `materials`"*.

## What is established

The farm loop **runs exactly one cycle and stops**. Produce is harvested but the
haul count is 5 against 8 harvests, so the crop never reliably reaches a
stockpile, `materials` never becomes available, every subsequent claim is refused
for `materials`, designations age out and are swept, and the colony never sows
again.

## ★ What is NOT established — the causal direction

**Hauling is itself a job that must be claimed**, and claims are refused 304/304.
So `haul = 5` may be the **cause** of the materials famine or a **symptom** of the
same claim failure. The two are not separated by this data.

There is one suggestive asymmetry worth the next read: a **haul job moves
materials, so it should not itself be refused *for* materials**. If the 240
`materials` refusals are all sow-type jobs while haul jobs are refused for some
other reason — or not refused at all and simply never generated — those are
different defects. **That is the next single-log read**, and it is on banked
data.

## ★ Consequence for #114's intervention

`endurseed` exists to add **seed stock**. If the binding constraint is the
**haul** loop rather than seed availability, then the arm's intervention is aimed
one link away from the failure — the colony has seeds it cannot move. That would
explain why adding seed stock produces a 50/50 outcome instead of a fix.

**Stated as a consequence to test, not a conclusion:** it follows only if haul is
causal, which is exactly what is not yet established.
