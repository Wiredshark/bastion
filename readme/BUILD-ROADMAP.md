# BUILD ROADMAP — the living list

**Maintained by Fable (orchestrator) — updated at EVERY item close, finding, or Ben directive.
Ben's standing order (2026-08-10): "you need to be constantly thinking about this stuff as we
finish features, you need to continually update this build list."**
Rulings live in `DECISIONS-FOR-BEN.md` (rows cited); this file is the current-state view.
Format: append changelog at bottom; statuses edited in place. Newest ruling wins.

## Standing design riders (apply to every item below)
- **Plot/plan data model** (#102): anything placing structures/zones designs against plots, never raw positions.
- **Live-emit declaration** (#88): every new harness accessor declares ported / harness-only-with-reason.
- **Diagnostics never verdict terms** (#70 pattern); **event-driven emits only** (diag-density budget).
- **Instrument + registry row land in the same commit**; **asset tuning touches compiled default same commit**.
- **Ruling acknowledged = packet edited, one operation** (#101). **Claims are not artifacts.**
- **Every bar names its window AND its arena (who else can act)** (#95); success signature in every observable.

## ARC 1 — Close the survival loop (items 1–8)
| # | item | status |
|---|------|--------|
| 1 | Name the food thief | **DONE** (#95 — ambient NPCs; attribution complete, zero residual) |
| 2 | Stall counter (rescue cleanup counts progress, not claims) | PACKETED (ROW-ITEM2), builds after 6's acceptance |
| 3 | THE ARRIVAL FIX (sit trap) | **LANDED** 6797b5c409 + witness d0ce0a58e1; proof pair score PENDING |
| 4 | Egress planner (verdicts-without-plans) | PARKED — likely symptom of 3; re-scores on own signature post-3; VOID-on-zero recorded |
| 5 | Server-authoritative test timing (Wait(n)) | PACKETED (ROW-WAIT), builds after 2 |
| 6 | Protected persistent provisioning + stockpile eating | **BUILT** cbfb8ae977 (membership-only + #97 two-layer gate); acceptance run IN FLIGHT (script-12, field-pile case decisive) |
| 7 | Farm-to-table self-sufficiency | **BLOCKED by located bug**: sow never fires — farm jobs created z=415 under z=418-419 surface; two code reads queued (job-Z producer; shared-origin check vs arrival math) |
| 8 | Multi-day endurance run (TIER-1 AUTONOMY GATE) | waits on 2,3,6,7 — the "colony runs itself" milestone; Ben gets observer seat |

## ARC 1.5 — Infrastructure (runs alongside)
- **Entity event log** (#99/#100): designed (DESIGN-ENTITY-EVENT-LOG.md, premise-checked chassis, per-entity rings, promotion persistence). BUILDS when arc-1 wave clears. Acceptance: the Voonoo query = one query.
- Reasoning dossiers (#98): FOOD pilot written; travel dossier at item 3's close; dossier per arc-close is ritual.
- Self-terminating run harness (#73 — server+driver teardown); collect_wave --baseline DONE (#67).
- Farming writeup (5b, owed); .engine-integration-wt target scoped cleanup (5b, owed).

## ARC 2 — Legibility (9–12)
9 Colonist inspector HUD · 10 Colony dashboard · 11 Recreation/idle life · 12 Chronicle UI (becomes the entity-log's player view).

## ARC 3 — Threats (13–15)
13 Hostiles near colony (Flee drive's live test) · 14 Guard job · 15 Fortification designations.

## ARC 4 — Economy & persistence (16–18)
16 Haul priorities + hauler-vs-eater contention (**plot-model rider applies**) · 17 Skills visible+felt · 18 Save/load integrity (entity-log promoted-set migration rides this).

## Capstones (19–20)
19 Renderer horizon retest (one-constant fixture fix — arena radius ≥ tested horizon) · 20 **Milestone playthrough v2** — LLM-player session scoring everything (#62 recurring gate).

## Band 21–40 (directional; design docs exist where noted)
21 Personalities visible · 22 Relationships · 23 Morale events · 24 Seasons that bite · 25 Shelter quality ·
26 Crafting chains · 27 Cooking · 28 Tool quality/wear · 29 Trade with vanilla world · 30 Stockpile zones by type (**plot rider**) ·
31 God-hand v1 (founding docs exist) · 32 Faith economy · 33 Miracles ·
34 Raids scaling with wealth + **THIEVERY as designed feature** (#97 — gate lift is line one; chronicle wiring pre-built per #100) ·
35 Injuries & medicine · 36 Death that matters ·
37 LLM-player harness v2 · 38 RL-tuning groundwork · 39 Performance row (sub-threshold tick degradation) · 40 Colony scale 16–32.
**SETTLEMENT PLAN LAYER (#102)** slots as its own arc in this band, before 31: plot grammar reuse, district zoning, cost-driven form, desire-line roads, layout-quality corpus metrics.

## Changelog
- 2026-08-10: Created (rows #93–#102 folded in). Items 1 done, 3 landed-pending-score, 6 in acceptance, 7 blocked-with-coordinate; settlement layer added with immediate rider; thievery moved to 34's family; entity log designed.
