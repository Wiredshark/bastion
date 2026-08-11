# ROW — **SPLIT THE COMPLETION SIGNAL**

> ## **"JOB ENDED" (bookkeeping) AND "WORK COMPLETED WITH EFFECT" (metric · watchdog ·
> XP · drops) ARE TWO DIFFERENT EVENTS. THE CODE EMITS ONE.**

**Chartered 2026-08-11 out of the indestructible-mine-cell row
(`ROW-INDESTRUCTIBLE-MINE-CELL.md`). Fable-raised, Opus-enumerated.**

---

## 0 · ★★★★★★ THE LAW THIS ROW EXISTS TO ENFORCE

> ## **A FALSE SIGNAL PROPAGATES TO EVERY CONSUMER.**

**One unverified completion event in item 8's v4 fed:**

| consumer | what the lie did |
|---|---|
| **the health metric** | *counted phantom work as production — completions ROSE 50→166→145 while the colony starved* |
| ★★★★ **the safety watchdog** | **DISARMED IT.** *A completion every ~15 s against `STUCK_TELEPORT_SECS = 60.0` — the clock never reached a third of what the teleport needed, for 2.5 hours* |
| **the investigation channel** | *silent — `watch_wipe` emits only under `BASTION_EGRESS_DIAG`, so the discriminator was OFF during the exact recurrence its own doc comment predicted* |

★★★ **To the watchdog, the trapped colonist read as the colony's most productive
member.**

---

## 1 · THE ENUMERATION — **every consumer of the completion signal**

*Read at `5d0dc72b9a`, `bastion-server/src/bastion_jobs.rs`, cited by symbol.*

| # | consumer | state |
|---|---|---|
| **1** | `info!("bastion: job completed")` | ✅ **fixed** — emits only for real production; emergency-access gets its own labelled line |
| **2** | `watch_wipe(…, "job-completed")` | **gated in the same commit** |
| **3** | ★★★ `watch_wipe(…, "work-progress")` | **gated in the same commit — FOUND LATE, was ungated** |
| **4** | `grant_xp(job.work, COMPLETION_XP)` | ✅ pre-existing `!is_emergency_access` |
| **5** | `emit_drop(MINE_DROP_ITEM)` | ✅ pre-existing gate |
| **6** | cave-in `floating_chunk` | ✅ pre-existing gate |
| **7** | ⚠ **`done_regions` — designation AABB retirement** | **UNREAD.** *Can a phantom completion retire a designation outline with undone work left? Flagged, not asserted.* |
| **8** | `to_release` / `remove_job` / `emergency_access_jobs.remove` | ✅ correct for BOTH kinds — *this is genuinely the "job ended" consumer* |

★★ *`board.plans_completed` is a different signal (plans, not jobs) — not implicated.*

### ★★★★★★ SITE 3's COMMENT NAMES THE BROKEN PREMISE ITSELF

> *"completing a job is the ground-truth 'making progress' signal — reset the universal
> stuck-watch … only a colonist that completes NOTHING for the full window is
> teleported."*

**The premise is that completion is ground truth. The entire defect is that it isn't.**
★★★ *Third shipped comment in one evening that foresaw its own failure and could not
page anyone.*

---

## 2 · ★★★★★ WHY PER-CONSUMER PREDICATES ARE THE WRONG SHAPE — *demonstrated on the reviewer*

**The reviewer who found the disease, named two victims, and wrote the precondition
STILL under-enumerated by one site, one message later.**

> ## **PER-CONSUMER PREDICATES REQUIRE FINDING THEM ALL — AND THE FAILURE MODE IS
> "YOU MISSED ONE", WHICH IS SILENT BY CONSTRUCTION.**

★★★★ **The predicate belongs AT THE SIGNAL'S ORIGIN, once.** *Table row 8 shows the
clean seam already exists conceptually: bookkeeping legitimately fires for both kinds,
everything else should fire for one.*

---

## 3 · THE WORK

1. **Split the emission at the origin** — *a job ending produces `JobEnded` always, and
   `WorkCompleted { effect }` only when the completion had a world-effect.*
2. **Re-point consumers 1–6 at `WorkCompleted`; leave 8 on `JobEnded`.**
3. ★★ **Read consumer 7 and place it deliberately** *(the one open item above)*.
4. **Planted tests, both arms:** *an effect-less completion must NOT reach any
   `WorkCompleted` consumer (**RED by name on today's code**), and a normal completion
   must still reach all of them (**GREEN control**).*

### STAGING

**Tonight's commit gates sites 2 and 3 per-consumer** — *honest staging, not the fix.*
★★★ **This row is the unification, and it holds the enumeration so the next consumer
added cannot subscribe to the wrong event.**

**SEQUENCE:** *after v5. v5 needs the gates, not the refactor* — *and per
[[stop-proposing-and-instrument]], v5's data may also settle consumer 7.*
