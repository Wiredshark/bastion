# ROADMAP ITEM 16 — HAUL PRIORITIES ON THE LIVE PATH — **PRE-REGISTRATION**

Written before any code change. ARC 4 item 16, first half.

## 1 · WHAT ALREADY EXISTS — read, not assumed

- `common::bastion::WorkPriorities` — RimWorld-style, **0 = never, 1..=4 rising**, all
  defaulting to **3**.
- The selector **honours it**: `bastion_jobs.rs` skips a job when
  `colonist.work_priorities.get(job.work) == 0`, and a higher priority beats distance
  (`priority > *bp || (priority == *bp && score < *bs)`).
- `Server::bastion_set_work_priority` / `RtSim::bastion_set_work_priority` exist, and
  **the harness uses them** (`bastion-harness/src/main.rs:2859` sets Mine to 0).

**So the mechanism is built and exercised — from the harness.**

## 2 · THE GAP

There is exactly **one** bastion chat command in the whole tree — `bastion_arena`
(`common/src/cmd.rs:376`). **No client message and no chat command can set a work
priority.** A player cannot reach the feature; only a test harness can.

That is the *gate-must-test-live-path* shape inverted: the harness path works, and the
**live path does not exist**. An owner who wants their colonists to stop hauling has no
way to say so.

## 3 · THE BUILD

A chat command `bastion_priority <work> <0..=4>` applying to **every** colonist in the
colony, routed to the existing `Server::bastion_set_work_priority`. **One authority** —
the command must not grow its own copy of the priority write.

*(Per-colonist targeting is deliberately out of scope: the colonist inspector is ARC 2
item 9, and a name-targeted command without a UI to discover names is a worse feature
than a colony-wide one.)*

## 4 · THE BARS

### P1 · **THE COMMAND EXISTS AND IS REACHABLE LIVE**
- **PASS:** the driver issues `cmd bastion_priority haul 0` and the server acknowledges,
  with a witness naming the work type, the value, and how many colonists were changed.

### P2 · **IT BITES — hauling stops** *(the separation, not a magnitude)*
- Arm A: colony runs its window normally ⇒ `haul deposited` **> 0**.
- Arm B: identical, but `bastion_priority haul 0` issued after founding ⇒
  `haul deposited` **== 0**.
- **Scored as a SEPARATION** (>0 vs ==0), per the determinism row's rule — this arm runs
  **through a client**, whose arrival tick is unpinned, so a magnitude would not be
  admissible here.

### P3 · **IT IS REVERSIBLE AND THE ARM IS NOT DEAD**
- Setting haul back to 3 in the same run ⇒ `haul deposited` **> 0** again.
- Without this, P2 passes on a colony that simply never hauled — the same "nothing came
  back" vacuity the cancel row had to guard.

### PLANT
- Make the command write the priority into a **copy** that the selector never reads (or
  drop the write entirely) ⇒ **P2 red**: hauls continue despite `haul 0`. This is the
  realistic defect — a command that logs success and changes nothing.

## 5 · WHAT I WILL **NOT** DO

1. **I will not claim item 16 closed.** This is its *priorities* half. **Hauler-vs-eater
   contention is a separate question** and is not tested here.
2. **I will not score P2 on a magnitude.** Through a client, only separations are
   admissible — established by the determinism row, before this row existed.
3. **I will not add a second priority-write path.** The command routes to the existing
   `bastion_set_work_priority`; if that needs changing, it changes in one place.
4. **I will not skip P3.** "Hauls stopped" is worthless if nothing was hauling.
