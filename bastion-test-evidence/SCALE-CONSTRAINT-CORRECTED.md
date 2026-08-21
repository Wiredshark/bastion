# The scale constraint, corrected — and what I still cannot claim

## What the corrected instrument says

`scale32diag`, attested fresh, with the refusal emit finally asking the gate's
own question:

```
req="…bastion.wheat"  stocked=1 reserved=1  units=2 reserved_units=2   × 558
req="…bastion.wheat"  stocked=1 reserved=1  units=4 reserved_units=4   × 40
```

`reserved_units == units`. Every unit **is** spoken for, so
`stockpile_has_material` is refusing **correctly** — the gate is not the defect.

The real number is the other one: the stockpile holds **2–4 units of wheat**
while 30 colonists want it. The colony is not blocked by a reservation bug. It
is simply, genuinely, nearly out of the material — and what little arrives is
claimed the moment it lands.

## The chain, as far as it is actually evidenced

1. Haul deposits fall with population — 62 at 8 colonists, 37 at 32.
2. So the stockpile stays nearly empty (2–4 units).
3. Those units are reserved instantly by the first claimants.
4. The other ~30 colonists refuse for `materials` (372 refusals).
5. Working share collapses: 25% → 14% → 9%.

Steps 2–5 are measured. **Step 1's cause is still unknown**, and the haul
census refuted the obvious candidate (`pending=1 cap=64` — the quota is nowhere
near binding, and only 15 loose pickups existed at all to haul).

## ★ What I explicitly cannot yet distinguish

**Is this a scaling defect, or a warm-up artifact?**

The contended material is `wheat` — a *farmed* good, not the seeded stone. A
32-colonist colony plants a larger farm (the founding plan scales with
population) but harvests take time, and the leg is ~12,000 ticks. It is
entirely possible that a colony of 32 simply had not yet grown enough food
within the window, and that a longer run resolves it.

That is a different finding from "throughput does not scale", and **I have
already published one wrong mechanism tonight by moving faster than the
evidence** (see the retraction in the previous commit). The distinguishing test
is a long-window run at 32 with food production tracked over time — not another
argument.

## Standing / retracted, stated plainly

- **STANDS:** working share collapse (25/14/9); haul deposits falling 62→37;
  372 materials refusals at 32; the haul quota is *not* binding; the material
  that exists *is* in a stockpile; every unit of it *is* reserved.
- **RETRACTED:** "reservations are per-stack, not per-unit." False —
  `stockpile_has_material` has been unit-aware since ITEM 27 (2026-08-20), and
  I cited that fix's own historical bug report as if it were current.
- **UNKNOWN:** why haul deposits fall with population, and whether the food
  shortage at 32 is a scaling property or a window artifact.
