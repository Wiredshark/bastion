# DISPOSITION — FLAT TOWN, leg 2 (radius 45)

Scored against `FLAT-TOWN-PREREGISTRATION.md`. Leg 1 was VOID (village outside
the disc). This one puts the village inside it.

```
FLAT TOWN — radius_chunks=45  chunks_flattened=6361  target_alt=399.57
            max_drop=620.11   max_lift=91.41
```

## Criterion 1 — a village inside the radius: **PASS**

The adopted village sits at (15216, 16016) — 1,168 blocks from centre, i.e.
**36.5 chunks against a 45-chunk radius.** Inside.

Independently corroborated: the founding altitude fell **420.77 → 404.65**
between leg 1 and leg 2. The flatten reached the ground the village stands on.

## Criterion 3 — the ground is level: **SUPPORTED, with an instrument caveat**

Surface heights across a 3,136-column band over the village:

```
402 ×816   ← dominant mode
401 ×172   403 ×218   404 ×66   405 ×101
406 ×9  407 ×32  408 ×33  409 ×32  410 ×58  411 ×63  412 ×52 …
```

Against leg 1's **409–425 with no mode at all**, this is a real change.

**The caveat is load-bearing:** `survey` returns the topmost *solid* block, so a
house roof reads as "surface". The tail above 405 is therefore *probably*
buildings, not terrain — but I cannot separate them with this instrument and I
am not going to claim ≤1 block variation, which is what the pre-registration
asked for. **Criterion 3 is supported, not met.**

## Criterion 4 — pathing without magic: **PASS on the number, UNATTRIBUTED**

```
ultimate fail-safe teleport   0
emergency access              0
GOTO-STAND-RESCUE            14
job unreachable               5
```

**Zero teleports.** Earlier sessions logged 30 in a run, each emitting *"a
colonist was moved by magic because nothing else could reach it."* None here.

**But I will not credit the flatten for it.** This leg is 4,200 ticks and has no
matched control on unflattened terrain. Zero-in-a-short-leg and
zero-because-the-terrain-is-fixable render identically. The honest claim is:
*no colonist was moved by magic during this leg*. Attribution needs a paired
run at equal length.

14 `GOTO-STAND-RESCUE` says pathing is still working hard, without resorting to
magic.

## Criterion 2 — the village renders: **NOT VERIFIED, and a cost appeared**

**The village got smaller when the terrain was flattened:**

| | leg 1 (unflattened) | leg 2 (flattened) |
|---|---|---|
| adopted plots | 5 | **2** |
| houses | 4 | **2** |
| barns | 1 | **0** |
| farm fields | 0 | 0 |

Flattening before civ placement changes what civ placement *builds*. Eight
colonists into two houses is four to a room — which fails the first acceptance
criterion ("each one has a house") by construction, no matter how flat the
ground is.

I have not looked at it rendered. Criterion 2 was written as "looked at, not
counted" precisely because a plot count proves placement and not rendering.

## Honest overall

**This is not yet a town.** The live census reads `engaged=1 working=0 stuck=1
idle=5` — five of seven loaded colonists doing nothing. Flat ground and zero
teleports are necessary and nowhere near sufficient.

## What the next row must decide

The village shrinking is the blocker, and it points at a design question rather
than a bug: **flattening a mountain range (max_drop 620) changes the world civ
generation reasons about.** Two candidate answers, and they are different games:

1. **Flatten less aggressively** — a gentler target that levels rolling terrain
   but leaves the region's character, so civ placement still builds a full
   village.
2. **Stop flattening the world and pick a village that is already flat** — the
   plot census already computes houses/fields/barns per site, so adoption could
   simply prefer a large village on level ground.

**(2) is cheaper, less invasive, and keeps Veloren's own generator entirely
untouched** — which is the thing Ben pointed at. Recommending it, and banking
the choice rather than deciding it, because "how the player's starting site is
chosen" is a gameplay-identity call.
