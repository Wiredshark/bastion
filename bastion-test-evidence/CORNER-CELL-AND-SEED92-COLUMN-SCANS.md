# Seed 1337 farm corner-cell: a sound plan_access false negative

Fable-directed survey, answering the pre-registered fork from the farm
map-seed confirming run: does the corner cell at `[24072-24074, 20239,
454-457]` sit on worldgen leaking through the fixture's flattening, or on
flat constructed ground with no obstruction?

**Answer: flat constructed ground with no obstruction. This is a real
`plan_access` false negative, not a terrain-class defect.**

## Method

`bastion_harness --farm-scenario` (seed 1337, current tip) builds its
plateau unconditionally over the whole play area
(`bastion-harness/src/main.rs` ~11126-11137: every `(x,y,z)` in a 33×25
column range, z `gz-6..=gz` set to `Rock`, z `gz+1..=gz+8` set to `Air` —
no conditional skip, full overwrite regardless of what was there before).
Added a small env-gated diagnostic (`BASTION_FARM_CORNER_DIAG`, zero cost
when unset) dumping `bastion_block_kind` over a 5×3×4 window
(`x=24071-24075, y=20238-20240, z=454-457`) centered on the cited blocking
cell, right before the scenario's final JSON. Committed alongside this
writeup.

## Result (raw dump in `bastion-test-evidence/seed1337-corner-diag.clean.log`)

| z | contents |
|---|---|
| 457 | uniform `Air`, all 15 cells |
| 456 | uniform `Air`, all 15 cells |
| 455 | `Rock`/`Earth` mix — the ground surface. `(24072,20239,455)=Rock`, `(24073,20239,455)=Earth`, **`(24074,20239,455)=Earth`** (the blocking cell itself — already tilled) |
| 454 | uniform `Rock`, all 15 cells |

Every cell in the surveyed volume matches exactly what the fixture's
flattening code would produce — solid rock floor, a flat surface where
tilling has (partially) happened, open air above. No gap, no floating
shelf, no cave, no obstruction in any direction within the survey window.
This is about as flat and unobstructed as constructed terrain gets — and
the blocking cell itself isn't even raw rock anymore, it's already been
tilled to `Earth` by a colonist at some point in the run.

## What this means

`plan_access` returned `None` ("no route exists") for cell
`[24074,20239,455]` against genuinely flat, open, walkable terrain on all
sides. Per the pre-registered fork: this is the **first sound specimen**
of a `plan_access` false negative — seeds 80 and 90 (the prior two cited
specimens) both died to real multi-layer/floating-shelf terrain that
retired their "genuinely unreachable" credential; this one comes with a
**fully-surveyable, fixture-built site and a scenario that reproduces at
will**. Unlike 80/90, there's no terrain-class explanation available here
— the geometry itself rules it out.

A wrongly-condemned cell is worse than a stuck one: `plan_access`'s "no
route" verdict is treated as strong and definitive throughout the system
(it sets `unreachable=true` immediately, fires the blocked-designation
message, and — per the trap-run investigation two sessions ago — bypasses
the churn/retry counter entirely). A false negative here means a
genuinely-workable cell gets permanently written off with no retry
mechanism, silently, the moment `plan_access` gets it wrong once.

## Reproduction

```
BASTION_FARM_CORNER_DIAG=1 target/no_overflow/bastion-harness.exe --farm-scenario
```

Deterministic (seed 1337 default, no other flags needed). `farm_tilled:
false`, `farm_sown: false` on this run, matching the map's prior "8/9
tilled" reading exactly.

## Not yet done

I haven't looked at `plan_access`'s own implementation to find WHY it
fails here — that's the natural next question (a bug in the planner's own
search, not the terrain), but it's a different kind of work than the
survey I was asked for, and Opus owns the plan_access/access-planning
lane. Routing this there per Fable's instruction rather than digging
further myself.

---

# Addendum: seed 92's chop-target column scan — CORRECTION, multi-layer

Fable's second, paired scan (same session, same tool): does seed 92's raised-
cap probe negative (`SEED92-RAISED-CAP-PROBE`, this branch, commit
`350d04630f`) hold at a **single-layer** site, or does the site turn out
**multi-layer** the way 80 and 90 did?

**Answer: multi-layer. Correcting my earlier claim** — seed 92's completed
probe negative is **not** a confirmed-sound unreachable specimen. It joins
80/90 as honest-unknown; the point-negative caveat claims its third scalp.

## Scope correction (Fable's point 1, folded in here)

The raised-cap run only answered the CAP question: the probe is no longer
budget-limited, it completes and returns a real (not truncated) negative.
That is a POINT-MODEL negative, and per the standing per-error-model
caveat, a point-model negative is sound only at single-layer sites — which
is exactly why 80 and 90 were retired as specimens rather than confirmed:
their sites scanned multi-layer, so the probe's own single-column mental
model doesn't hold there, whatever it reports. Seed 92 needed the same
check before its "known" carried any weight beyond "not cap-limited."

## Method

Added `BASTION_CHOP_COLUMN_SCAN` (env-gated, zero cost when unset) to
`b5_scenario`, dumping `bastion_block_kind` down the chop target's own
`(x, y)` column, `z` 250 down to 50 (generous headroom, matching the scale
of the 80/90 scans). Target column from the run's own
`b5_ch_base_blocked_by`: `(12964, 26352)`.

## Result (raw dump in `bastion-test-evidence/seed92-column-scan.clean.log`)

| z range | contents |
|---|---|
| 250–157 | `Air` |
| 156 | `Wood` — the chop target itself (the tree trunk `plan_access` was asked to route to) |
| 155–154 | `Rock` (2 cells) |
| 153–140 | **`Air` — a 14-cell open gap** |
| 139–50 | `Rock` (solid, continuing below the scan floor) |

The target tree sits on a 2-block floating shelf at z=154-155, itself
suspended 14 cells above the real ground (which starts at z=139 and keeps
going). Same shape as seed 80's site (`solid 90–139, 12-cell gap 140–151,
one solid block at 152`) and seed 90's (`open band at z=334–337, twenty
blocks under the probe's reported ground`) — a third instance of the same
terrain class.

## Verdict

Seed 92 does **not** become the first confirmed-sound unreachable
specimen. It joins 80 and 90 as **honest-unknown**: the probe completed
and returned a real, non-cap-limited negative, but the site's own geometry
means a single-column point-model can't be trusted there regardless of
what the probe says. Three for three now on multi-layer sites defeating
this class of specimen — worth naming as its own pattern (not one unlucky
seed) if this comes up again: a completed reachability-probe negative is
never sufficient on its own; it needs a paired single-layer confirmation,
every time, or it stays unknown no matter how definitive the probe's own
report looks.

The corner-cell case (main body of this document, above) stays the
opposite and more interesting result BECAUSE it was checked the same way
and came back single-layer — the discriminator did real work distinguishing
the two, rather than both scans confirming the same thing.
