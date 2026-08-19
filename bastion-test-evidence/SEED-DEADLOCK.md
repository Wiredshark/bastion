# THE COLONY DEADLOCK: a blocked job prevents the haul that would unblock it

Found by reading, from banked evidence, with two fans running. No new spend.

## The circular wait

| step | code |
|---|---|
| 1. A farm job is created **at the farm cell**, needing a seed | `bastion_jobs.rs:11235` — `JobKind::Designated(Farm)`, `pos`, `required_item: req` |
| 2. No seed in the stockpile ⇒ the job is refused for **`materials`** | claim census: `materials 240` |
| 3. The earlier harvest dropped its seeds **on that same cell** | `emit_drop(…, job.pos, FARM_SEED_ITEM, …)` ×2 per harvest |
| 4. The haul generator **skips any cell that already holds a job** | `if … occupied.contains(&cell) { continue; }`, where `occupied` = **every job position on the board** |
| 5. The seeds never reach the stockpile ⇒ **back to step 2, forever** | `blocked_materials` pinned at 28–30 for 271,000 ticks |

**The blocked job's own presence is what prevents the delivery that would unblock
it.** `FARM_SEED_ITEM` *is* on the haul allow-list — the seeds are eligible in
every respect except that something is standing on them.

## Why this produces a coin flip rather than a consistent failure

Two periodic processes race:

- the **haul generator** runs on `tick % ARBITRATION_INTERVAL == 7`, and only
  while a stockpile exists and `pending < cap`
- the **farm generator** re-creates the sow job on the freed cell

If a haul pass lands in the window **after** the harvest and **before** the farm
job re-occupies the cell, the seeds move to the stockpile and the cycle
continues. If it does not, the cell is occupied and the seeds are invisible to
hauling from then on.

That is exactly the observed shape: **the first cycle always succeeds** (no
blocked job exists yet — every run sows 8, matures 8, harvests 8), and what
differs afterwards is whether one timing window was hit.

★ It also explains the sweep counts: collapsed runs show **276** *"unclaimed
designation swept"* against **8** in thriving ones. The sweeper reaps the
unclaimed farm job and frees the cell — then the generator re-creates it. The
two fight, and the haul window is the gap between them.

## What is established, and what is not

**Established by reading:** the farm job's position, the drop position, and the
`occupied` exclusion are all in the code and all coincide. The allow-list
includes seeds, so eligibility is not the issue.

**NOT established:** I have not *observed* a seed being skipped. The chain is
read, not measured. Consistent evidence exists — a collapsed run has **5** haul
events against a thriving run's **9,167**, and its `blocked_materials` never
moves — but consistency is not observation.

## ★ The registered prediction, testable on the running fan

If this is right, the `BASTION_DROP_TOSS_DIAG` runs must show:

| | |
|---|---|
| seed drops **emitted** in a collapsed run | **2 per harvest** — the yield fires normally |
| haul jobs created for those drops | **≈ 0** |

⇒ the conservation invariant fails at **delivery**, not at emission.

If instead the collapsed runs show **no seed drops at all**, this whole chain is
wrong and the defect is upstream in the harvest.

## The fix shape (not applied — a live-path change)

The exclusion exists to stop hauling items *out of a cell that is itself a work
site*. But a **loose item lying on a blocked job's cell is not part of that
job** — it is the input the job is waiting for. Candidate: exempt items whose
def matches a blocked job's `required_item` on that cell, so a job can never
starve on the resource sitting underneath it.

**Filed, not built.** It touches the live claim path and wants its own
red-demonstration; the fix is worthless without a planted control proving the
deadlock can be reproduced on demand.
