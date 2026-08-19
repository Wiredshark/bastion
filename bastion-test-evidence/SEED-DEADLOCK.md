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

## ★ THE GENERATOR ORDERING MAKES IT WORSE — measured from the tick gates

`ARBITRATION_INTERVAL = SIM_TPS / 2 = 15`, and each generator runs on a fixed
offset inside that cycle:

| offset | generator |
|---|---|
| **3** | **farm** — creates the sow job on a free cell (`!board.farms.is_empty()`) |
| **7** | **haul** — skips cells that hold a job |

**The farm generator runs FIRST, every cycle.** So a cell freed at any point is
re-occupied at offset 3 before the haul pass at offset 7 ever sees it empty.
Within a cycle there is no window at all — the ordering is not a coincidence of
load, it is fixed in the tick gates.

### The part the code does NOT settle, stated plainly

The **sweeper** reaps unclaimed jobs on a *time threshold*
(`access_stall_secs`), not on a tick offset, so it can free a cell at offsets
4–6 and open a genuine window. Collapsed runs show **276** sweeps, which is a
lot of chances.

I cannot close that by reading: I do not know how quickly a reaped cell is
re-taken, and the arithmetic I can do (≈9 sweeps per cell × a 3-in-15 chance of
landing in the window) predicts far more haul events than the **5** observed.
**Either the window closes faster than a tick, or something else also blocks
those hauls.**

★ That is the honest boundary of the read, and it is why the witness below was
built rather than another paragraph of reasoning: the instrument counts skips
directly and does not care which story is right.

## What is established, and what is not

**Established by reading:** the farm job's position, the drop position, and the
`occupied` exclusion are all in the code and all coincide. The allow-list
includes seeds, so eligibility is not the issue.

**NOT established:** I have not *observed* a seed being skipped. The chain is
read, not measured. Consistent evidence exists — a collapsed run has **5** haul
events against a thriving run's **9,167**, and its `blocked_materials` never
moves — but consistency is not observation.

## ★★ THE GEOMETRIC PREMISE IS NOW MEASURED, NOT READ — from banked logs

The chain needs the seed-needing sow job to sit on the cell where the harvest
dropped. That is checkable in the bank, using emits that already exist
(`farm job created … pos=… sow=true` and `harvested … pos=…`):

| run | harvest cells | sow-job cells | **overlap** |
|---|---|---|---|
| COLLAPSE | 8 | 30 | **8 — 100%** |
| THRIVE | 30 | 30 | **30 — 100%** |

**Every harvest cell is also a seed-needing sow-job cell**, at the same z. The
geometry the deadlock requires holds in every run.

★ And note the collapsed run creates **344** farm jobs (**306** needing a seed)
across its life — the generator is running constantly, re-creating the job on
the cell whose seeds are lying there unhauled. It is not that the colony stopped
trying.

### ★ The overlap is 100% in the THRIVING run too — and that is the point

Geometry alone does not cause the collapse: both regimes have it. **The
deadlock-prone arrangement is universal; what differs is whether a haul pass
lands in the window.** That is precisely what the mechanism predicts — a race,
not a static configuration — and it is why the collapsed run shows 5 haul events
against the thriving run's 9,167 with identical geometry.

Had the overlap been 100% in COLLAPSE and low in THRIVE, the mechanism would be
simpler and *wrong*: it would make the arrangement the cause, and no timing story
would be needed.

## ★ The registered prediction, testable on the running fan

If this is right, the `BASTION_DROP_TOSS_DIAG` runs must show:

| | |
|---|---|
| seed drops **emitted** in a collapsed run | **2 per harvest** — the yield fires normally |
| haul jobs created for those drops | **≈ 0** |

⇒ the conservation invariant fails at **delivery**, not at emission.

If instead the collapsed runs show **no seed drops at all**, this whole chain is
wrong and the defect is upstream in the harvest.

## ★★ IT IS NOT A FARM BUG — FOUR JOB KINDS HAVE THIS SHAPE

Every job kind carrying a `required_item` whose def is also on the haul
allow-list can starve on the resource lying underneath it:

| job kind | `required_item` | dropped by | on the haul allow-list |
|---|---|---|---|
| **Farm** | `wheat_seeds` | harvest | ✓ |
| **Build** | `crafting_ing.stones` | mining | ✓ |
| **Bed** | `crafting_ing.stones` | mining | ✓ |
| **Ladder** | `log.wood` | chopping | ✓ |

★ `BUILD_MATERIAL_ITEM` and `MINE_DROP_ITEM` are **the same constant** —
`common.items.crafting_ing.stones`. So a Build or Bed job sitting on a cell where
mined stone has dropped is in exactly the same circular wait as the farm case,
and a Ladder job is in it with wood.

**The farm case is simply the one that showed up first**, because the endurance
fixture runs the farm loop hundreds of times while building happens once. That
makes farming the *detector*, not the defect's boundary.

★ The fix below matches on `required_item` **generically**, not on `Farm`, so it
covers all four kinds without naming any of them.

## ★★★ THE DEADLOCK BREAKS A DOCUMENTED CONTRACT — the fix RESTORES it

`FARM_SEED_ITEM`'s own doc states the intended path:

> *"the sow verb's consumed item — a REAL item (**the B6 fetch contract:
> `required_item` + the material-haul machinery deliver stockpiled seeds to sow
> jobs for free**)."*

The design is **harvest → haul to stockpile → machinery delivers to the sow job**.
The `occupied` skip severs the middle step: seeds never reach the stockpile, so
the delivery machinery has nothing to deliver, and `required_item` can never be
satisfied.

**So this is not a patch bolted onto working behaviour — the documented contract
is already broken, and the exemption restores the path the contract names.**

★ And **no test asserts the skip as intended.** The only haul tests are
`canonical_haul_pickup_order_is_join_order_independent` (ordering) and
`surface_teleport_skips_occupied_column` (unrelated). The behaviour is unguarded,
so the fix contradicts nothing pinned — which also means nothing would have
caught this.

## The fix shape (not applied — a live-path change)

The exclusion exists to stop hauling items *out of a cell that is itself a work
site*. But a **loose item lying on a blocked job's cell is not part of that
job** — it is the input the job is waiting for. Candidate: exempt items whose
def matches a blocked job's `required_item` on that cell, so a job can never
starve on the resource sitting underneath it.

**Filed, not built.** It touches the live claim path and wants its own
red-demonstration; the fix is worthless without a planted control proving the
deadlock can be reproduced on demand.
