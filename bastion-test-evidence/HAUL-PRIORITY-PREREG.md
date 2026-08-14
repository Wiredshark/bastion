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

## 4b · AMENDMENT 1 — **THE SCORING WINDOW OPENS AT THE COMMAND, NOT AT BOOT**

*Added 2026-08-14 while the re-run arms were still in worldgen — **no arm had produced
a single haul line yet.** Recorded here rather than applied silently at scoring time.*

The server starts ticking at boot; the driver connects **minutes** later, because worldgen
(especially three in parallel) is slow. So a haul can land **before the command is even
reachable**. P2 as originally written — *`haul deposited` == 0* over the whole log — would
then read RED on a command that worked perfectly, purely because the colony hauled during
worldgen-lag.

**P2 and P3 are therefore scored over the window that OPENS at the tick of the command's
own witness emit**, and A over the tick of its `anchor`. Everything before that boundary is
outside the command's causal reach and is reported separately as `pre_window=`.

Two reasons this is legitimate rather than convenient:
1. **The boundary is machine-read, not chosen.** The witness emit already carries its tick;
   I am not picking a cut after seeing where the hauls fell.
2. **It is registered before the data exists** — which is the whole point of the practice,
   and is why this amendment is dated and diffable rather than folded into the results.

**It also cannot rescue a failure:** if the post-window count is non-zero, P2 is RED. The
amendment narrows what counts as evidence; it does not soften the bar.

## 5 · WHAT I WILL **NOT** DO

1. **I will not claim item 16 closed.** This is its *priorities* half. **Hauler-vs-eater
   contention is a separate question** and is not tested here.
2. **I will not score P2 on a magnitude.** Through a client, only separations are
   admissible — established by the determinism row, before this row existed.
3. **I will not add a second priority-write path.** The command routes to the existing
   `bastion_set_work_priority`; if that needs changing, it changes in one place.
4. **I will not skip P3.** "Hauls stopped" is worthless if nothing was hauling.
