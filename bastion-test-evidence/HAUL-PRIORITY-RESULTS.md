# ROADMAP ITEM 16 (priorities half) — **RESULTS & ROW DISPOSITION**

Scored against `HAUL-PRIORITY-PREREG.md` + its Amendment 1. Engine tip `8173de1bfa`.
Binary `veloren-server-cli.exe` built 2026-08-14 10:52:50; attestation
`prio2-attest.txt` — **no tracked `.rs` newer than the binary**, dirty set = this row's
own `common/src/cmd.rs` + `server/src/cmd.rs`.

## THE SCORE

| bar | verdict | evidence |
|---|---|---|
| **P1** the command exists and is reachable live | ✅ **PASS** | driver receives `bastion: haul priority set to 0 for 8 colonist(s)`; server emits `work=Haul priority=0 changed=8 colonists=8` |
| **P2** it bites — hauling stops | ⚠ **NOT SCOREABLE at n=1** — see below | control 3 vs treatment 0, but **0 is inside the control's own distribution** |
| **P3** it is reversible and the arm is not dead | ⛔ **NOT DEMONSTRATED** | arm C hauled **0** after re-enabling to 3, in a window that covered the phase where the control hauled |

> **The row does NOT close item 16's priorities half.** The command is built, reachable,
> and self-witnessing. Its *effect* and its *reversibility* are unproven.

## ⛔ WHY P2 CANNOT BE SCORED — my own earlier row refutes my separation

The registered bar was a separation: control `> 0` vs treatment `== 0`. Measured:

| arm | command | hauls | harvested | sim window |
|---|---|---|---|---|
| prioA (control) | none | **3** | 22 | 9.8 → 409.8 |
| prioB | `haul 0` | **0** | 8 | 7.5 → 407.5 |
| prioC | `haul 0` then `haul 3` | **0** | 8 | 102.2 → 502.2 |

3-vs-0 reads like a clean separation. It is not, and the document that says so is
**`HAUL-THROUGHPUT-RESULTS.md`, written by this same loop**: two n=8 legs running *one
identical script* through a client hauled **5 and 0**. A control leg in this exact
configuration is already known to produce zero.

> **So "treatment hauled 0" is a value the control itself emits.** With n=1 per arm and a
> control magnitude of 3, the experiment cannot distinguish "the command works" from "this
> leg was a quiet one." **Scoring it PASS would have been the H3 mistake again** — reading
> a separation off a spread I had already measured and written down as too wide.

This is not a failure of the feature. It is a failure of the *instrument*, and it is
caught by the prereg's own P3 guard rather than by hindsight.

## THE THREE VOIDS BEFORE ANY DATA EXISTED

All three were preconditions, all three read **identically to a feature that does not
work**, and each was caught by printing the precondition above the result.

1. **The Admin grant silently did nothing.** `admin add <user> admin` without `--no-auth`
   resolves the username through the auth server, fails, writes an **empty `admins.ron`**,
   and **exits 0**. The driver got `command-no-permission`; all three arms reported
   `cmd_witness=0`. That is indistinguishable from "the handler is broken."
   **Fix:** `--no-auth` on the grant, and verify the *artefact* (`admins.ron` has an
   entry), never the step that should have written it.
2. **The control hauled nothing.** At a 5400-tick window arm A hauled 0 — which by the
   prereg's own words makes B and C void, *"because hauling stopped means nothing when
   nothing was hauling."* Window raised to 12000.
3. **Two servers died after announcing they were ready.** Parallel legs were isolated on
   the *game* port only. Each server then logged `Server is ready to accept connections`
   and panicked a moment later binding `web_address` **14005**, shared by all three.
   A parallel leg needs **every listening socket** moved — game, web, metrics — not just
   the one the test talks to.

A fourth, in the same family: **the driver has no connect retry.** A fixed `sleep 45`
before connecting is a guess about worldgen duration; three parallel worldgens blew
through it and every driver panicked on `ConnectionRefused`. Replaced with a port poll.

## ★ A STRUCTURAL FINDING THE ROW DID NOT GO LOOKING FOR

**`insert_eat_job` sets `work: WorkType::Haul`.** Eating is classified as haul work. Read
naively, `bastion_priority haul 0` should therefore starve the colony.

It does not — and the reason is an accident:

- eat jobs are inserted **pre-claimed** (`claimed_by: Some(uid)`), through the B7-2
  preemption path;
- the only work-priority gate lives in the **claim selector**, which filters
  `job.claimed_by.is_none()` before it ever reaches `work_priorities.get(job.work)`.

> **So the feature is safe today by claim-ordering, not by design.** Any future change that
> routes self-jobs through the selector — or adds a second priority check keyed on
> `job.work` — turns `haul 0` into a starvation command. This is registered as a landmine,
> **not** as a tested property: no colonist got hungry inside a 400-second window, so the
> live witness for it is **dormant-by-premise** in this fixture.

## AMENDMENT 1 — registered before any arm produced a haul line

The scoring window opens at the **command's own witness emit**, not at server boot, because
the server ticks from boot while the driver connects minutes later. Recorded in the prereg
(`232279e4cf`) while all six arms were still in worldgen. It narrows what counts as
evidence and **cannot rescue a failure**: a non-zero post-window count is still RED.

Measured `pre_window = 0` on every arm, so in the event it changed nothing — which is the
outcome an honest amendment should usually have.

## THE POWERED RE-RUN — *pending*

Six arms, two replicates each, 36000-tick windows (3×), all six in parallel with full
socket isolation. The control needs a magnitude large enough that zero is meaningful, and
a distribution rather than a point.

*(table to be filled from `powered.log` / `server-pw-*.log`)*

## WHAT I DECLINE TO CLAIM

- **Not** that the priority command is ineffective. P2 is under-powered, not refuted.
- **Not** that P3 failed *because the feature is broken*. C's colony diverged early
  (harvest 8 vs the control's 22) and never re-entered a hauling regime; whether the
  re-enable is inert or merely arrived at a colony in a different state is untested.
- **Not** that `harvest 22 → 8` is a *consequence* of `haul 0`. It is a difference of the
  same size as the known between-run spread, and I have one leg per arm.
- **Not** that eating survives `haul 0` *live*. That is a producer read, and the fixture
  never made anyone hungry.
