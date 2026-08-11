# ROW — **THE INDESTRUCTIBLE MINE CELL**

**Item 8's famine has a first cause, and it is not seeds, not claim expiry, and not
the sweep churn.** *Found 2026-08-11 in the committed v4 capture `1bedd79602`; no
rerun required, every read below reproduces from that pin.*

---

## 0 · ★★★★★★ THE FINDING

> ## **EVERY `bastion: job completed` IN 945,462 LINES IS `Designated(Mine)`, AND 328
> OF 361 ARE THE SAME TWO ADJACENT CELLS.**

| completed job | part-000 | part-001 | part-002 |
|---|---|---|---|
| **`Vec3 { x: 15212, y: 16043, z: 425 }`** | **47** | 19 | 0 |
| **`Vec3 { x: 15211, y: 16044, z: 425 }`** | 1 | **138** | **143** |
| *all other positions, all kinds* | 3 | 6 | 2 |

**A mine job completes without consuming its cell.** *The cell is re-designated,
re-claimed and re-completed — 281 times on one block — and the trap **walks to its
diagonal neighbour** when the first cell stops yielding.*

★★★★★ **This is TASK #61, filed as "do not run now — mine cell, progress 0.0,
claimed."** *It ran itself, live, for two and a half hours.*

---

## 1 · ★★★★★★ THE PRODUCTION COLLAPSE, ALONGSIDE IT

| | part-000 | part-001 | part-002 |
|---|---|---|---|
| **tilled · sown · harvested · crop** | 19 · 20 · 20 · 20 | ⛔ **0 · 0 · 0 · 0** | ⛔ **0 · 0 · 0 · 0** |
| **ate** | 11 | ⛔ **0** | ⛔ **0** |
| **haul delivered** | 11 | 2 | ⛔ **0** |
| ★★ **`job completed`** | 50 | **166** | **145** |

> ## **COMPLETIONS ROSE WHILE EVERY FORM OF PRODUCTION WENT TO ZERO.**

---

## 1b · ★★★★★★ THE MECHANISM — **A 3-BEAT CYCLE, ONE COLONIST, ONE REQUEST**

    20:45:27.210  job completed job=319037 kind=Designated(Mine) pos=(15211,16044,425)
    20:45:27.548  emergency route exhausted with invalid exit; member released
    20:45:27.548  emergency access restored (REQ-0040) owner=80 cells=1
    20:45:42.390  job completed job=319941 kind=Designated(Mine) pos=(15211,16044,425)
    20:45:42.743  emergency route exhausted with invalid exit; member released
    20:45:42.743  emergency access restored (REQ-0040) owner=80 cells=1
                  ... every ~15s, for 2.5 hours

> ## **`owner=80` — ONE colonist. `REQ-0040` — the SAME request. `cells=1`. THE
> EGRESS PLANNER RE-ISSUES AN IDENTICAL ONE-CELL FIX FOREVER AGAINST AN EXIT IT HAS
> ITSELF JUST DECLARED INVALID.**

★★★★★ **These are `is_emergency_access` completions** — *which is why the run shows no
drops, no XP and no cave-in for 361 mine completions.* **The emergency arm skips all
three** (`bastion_jobs.rs`, the `!is_emergency_access` guards around
`MINE_DROP_ITEM`, `grant_xp` and `floating_chunk`) — *but `info!("bastion: job
completed")` is emitted **unconditionally**, which is why the health metric counted
phantom work as production.*

### ★★★★★★ DEFECT 1's MECHANISM READ — **NARROWED TO ONE FACT, THEN STOPPED**

**Read, cited, at `5d0dc72b9a`:**

- **`completion_block(Designated(Mine))` → `Some(Block::empty())`** *(`bastion_actions.rs`)*.
  **Air. The write is issued.**
- **`block_change.set(job.pos, new_block)` runs BEFORE every `!is_emergency_access`
  guard** — *so the emergency arm does not skip it.*
- ★★★★ **"emergency access restored (REQ-0040)" is a ROUTE TEARDOWN, not a terrain
  restore** — *it retains/removes `emergency_approach_corridors`,
  `emergency_route_sequences`, `emergency_partial_route_entries` and retires traversal
  tasks.* **It never touches a block.**
- **`bastion: job moot` fires ZERO times in 945K lines** — *so the cell NEVER read air
  at a later completion.*

> ## **THEREFORE: THE BLOCK IS FILLED AT EVERY ONE OF THE 281 COMPLETIONS. AN
> `is_filled()` PRE-CHECK AND AN AIR WRITE, ALTERNATING, 281 TIMES — SO THE WRITE IS
> NOT STICKING.**

★★★ **WHY it doesn't stick is NOT in this capture, and I am not proposing a third
mechanism for it** — [[stop-proposing-and-instrument]].

### ★★★★★★ THE INSTRUMENT IS ALREADY IN SCOPE — **a one-word change**

**The completion arm ALREADY computes the pre-removal block kind:**

    let completed_kind = terrain.get(job.pos).ok().map(|b| b.kind());

*It is captured for Chop's drop branch and **never logged**.* ★★★★★ **Adding
`completed_kind` to the `job completed` emit costs one field and zero new reads —
and it decides this outright:** *a constant `Rock` across 281 completions says the
write never landed; anything else names what is re-filling the cell.*

**BUILD ITEM: a THIRD log field, free, alongside `kind`-on-arrival and the
material-stall emit.**

### THE TWO SEPARABLE DEFECTS

1. ★★★★ **THE BLOCK NEVER LEAVES.** *`still_valid` requires `b.is_filled()` at
   completion (`DesignationKind::Mine => terrain.get(job.pos).ok().is_some_and(|b|
   b.is_filled())`), and it passed **281 consecutive times on one cell**.* **Either
   `block_change.set` never lands or the block returns** — *and the corroborating
   fact is that `bastion: job moot — target block changed under it; dropped` fires
   **ZERO times in 945K lines**: no mine job in the entire run ever found its target
   already gone.*
2. ★★★★★ **THE EGRESS REQUEST HAS NO TERMINATION.** *`route exhausted with INVALID
   EXIT` is a self-diagnosed dead end, and the planner's response is to re-issue the
   same one-cell job.* **A lifecycle that cannot end** — *the same requirement class
   as the orphan sweep's.*

★★★ **Defect 2 is sufficient on its own**: *even with the block removed correctly, an
invalid exit re-planned identically forever is an infinite loop.* **Both need
clearing — [[each-sufficient-blocker-must-be-cleared]].**

---

## 2 · WHAT IT OVERTURNS

**1 — THE COLONY WAS NEVER PARALYSED.** ★★★★ *This kills churn-starvation as the
farm's cause: if the 468K-cycle sweep were consuming colony capacity, non-farm work
would fall too. It rose — because the work was a phantom.*

> ⚠ **PRECISION CORRECTION, against my own first report:** *I wrote "the entire
> labour force was captured by phantom work," and that is MORE than the data
> supports.* ★★★ **What is established: the COMPLETION CHANNEL is 100% this loop,
> and its owner is a SINGLE colonist (`owner=80`).** *Meanwhile `colonist arrived at
> job site` fires 400 · 533 · 479 — **other colonists keep arriving at job sites and
> completing nothing**.* **That is a second, unexplained fact, not a restatement of
> this one** — *whether the rest of the labour force is trapped, idle, or failing at
> a different seam is UNREAD, and it is the next question.*

**2 — ★★★★★ `job completed` IS A COUNT A BROKEN SYSTEM RELIABLY PRODUCES.** *It is
the colony's own health metric and it pointed UP while the colony starved to death.*
**The identical error to F1's, one level down — and it needs the same treatment.**

**3 — THE SEED HYPOTHESIS IS REFUTED AS A CAUSE.** *The complete message vocabulary
is 22 distinct lines; there is no seed line, no fetch line, no refusal line anywhere.*
★★★ **And it does not matter: the farm stopped because every colonist was permanently
employed on an indestructible block.** *19·20·20 is the founding wave finishing before
the trap captured the whole labour force.*

**4 — ROUTES 2 AND 3 ARE CORRECT FIXES FOR REAL DEFECTS THAT WERE NEVER WHAT STOPPED
THE FARM.** *Predicted from F1's identity with v3, before this read existed.*

---

## 2b · ★★★★★ THE ARRIVALS THAT COMPLETE NOTHING — **READ, and a candidate marked as such**

**Part-002, 479 arrivals over 188 distinct sites — but FIVE sites absorb 285 of them:**

    157  (15211, 16044, 425)     <- the trap cell; completes (falsely)
     40  (15211, 16044, 423)     <- SAME COLUMN, two down; ZERO completions
     39  (15217, 15984, 416)     <- zero completions
     35  (15246, 16016, 418)     <- zero completions
     14  (15229, 15999, 425)     <- zero completions

★★★★ **Repeat arrival at a site that never completes is the same disease at a
different z** — *a colonist walks there, "works", is released, re-claims, and walks
back, forever.*

### ★★★ TWO FACTS FROM THE CODE — **READ, cited**

1. **The material-stall path is SILENT.** *In the completion arm, `if let Some(required)
   = job.required_item` → on `taken.is_none()` it sets `job.progress = 0.0;
   job.needs_materials = true;` and releases — **with no `info!` of any kind**.*
   ★★★★ **A colonist can loop arrive→stall→release forever and emit nothing but the
   arrival.** *That is exactly task #61's "progress 0.0, claimed."*
2. **Emergency-access mine completions SUPPRESS THE DROP** (`!is_emergency_access &&
   … Some(DesignationKind::Mine) => Some(MINE_DROP_ITEM)`).

### ⚠ CANDIDATE — **INFERRED, NOT READ. Marked so it cannot harden into a story.**

> *IF the stalled sites are Build/Ladder jobs, the loop closes on itself: the
> emergency-access mine yields NO stone → the ladder that would build the egress
> stalls on materials, silently → the exit stays invalid → the planner re-issues the
> mine → which again yields nothing.* **The one path that could supply the material is
> the same path that suppresses the drop.**

★★★★★ **THE DECIDING READ DOES NOT EXIST IN THIS LOG.** *The arrival line carries only
`job` and `pos` — **no `kind`** (verified against a sample line, not assumed), and
there is no generic job-creation line. **The job kind at `(15211,16044,423)` is not
recoverable from this capture.***

**So this is an INSTRUMENT REQUIREMENT, not an analysis step** —
[[enumerate-what-the-instrument-can-see]]:

- **emit `kind` on the arrival line**
- **emit on the material-stall path** *(a stall that logs nothing cannot be counted)*

★★ **Both are one-line additions and both must land before v5**, *or v5 reproduces this
exact blind spot.*

---

## 3 · ⚠ TWO METHOD DEFECTS THIS READ EXPOSED — **both silent, both zero-shaped**

1. ★★★★ **A WRONG PATH AND AN EMPTY FILE RENDER IDENTICALLY.** *The first search ran
   against `bastion-test-evidence/…-v4-split/`; the capture is at
   `bastion-test-evidence/**live-playthrough**/…-v4-split/`.* **Zero matches was
   reported as an unresolved naming question. It was an unread directory.**
2. ★★★★★ **`kind=` MATCHES NOTHING IN THESE LOGS.** *ANSI escape codes sit between key
   and `=`:* `kind[2m=[0mDesignated(Mine)`. **Every past `key=value` grep of this
   corpus returned a silent zero.** *Strip with `sed 's/\x1b\[[0-9;]*m//g'` first.*

★★ **Both are the same class: a zero that means "the instrument was not pointed at
the data," presented in the shape of a result.**

---

## 4 · THE ROW

**The mine-completion path completes a job without verifying the block was removed.**

**Two candidate seams, both already on our books:**

- **#61** — progress 0.0 on a claimed mine cell
- **#52** — `blocked_regions` entry keyed on region alone

★★★ **The read is narrow now: the cell, the timestamps, and the handover to the
diagonal neighbour are all in hand.**

### CONSEQUENCE FOR v5

**F1 (generation-2 completions) survives unchanged and is better motivated.** ★★ *But
the fix under test should be THIS, not another claim-lifetime tweak — and the
`job completed` metric itself needs a kind/position breakdown before it is trusted as
a health signal again.*
