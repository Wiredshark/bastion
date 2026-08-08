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
