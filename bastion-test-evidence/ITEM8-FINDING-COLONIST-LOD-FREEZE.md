# FINDING: founding colonists demote to Simulated ~20s after founding and
# never re-promote — the need-preemption system has been INERT for every
# `bastion_spawn_colony`-founded colony to date, not just item 8's unattended
# window

**Status: BLOCKS item 8's launch. Retroactively touches #105's already-
ACCEPTED leg. Reported for a ruling, not decided unilaterally.**

## How this was found

Item 8's real scored run was launched (`9d1c7c7c9e`) and monitored via the
liveness protocol. At the second check (tick 99000, ≈55 sim-min elapsed —
nearly double the predicted ~30-min first interrupt crossing),
`preempt_attempts` was still exactly `0` and zero `ate`/`slept`/`BREAKDOWN`
lines existed anywhere in the log. Since `decay_needs` runs unconditionally
every tick (no `is_loaded` gate), hunger alone should have hit its floor by
~15 sim-min — the total silence was not "hasn't crossed yet," it was a dead
instrument. **The run was killed rather than let the remaining ~2 hours burn
on a scenario that could not produce data**, per this arc's own "a run whose
own trend can't be reconstructed from its log is void by construction" law.

## Root-caused, not just correlated

Booted a throwaway diagnostic server (`BASTION_NEED_LOAD_FILTER_DIAG=1
BASTION_DECAY_JOIN_DIAG=1`), founded 8 colonists, and watched the raw log:

    07:35:59.027  all 8 colonists: "bastion: colonist promoted to loaded entity"
    07:36:18.930  all 8 colonists: "bastion: colonist demoted to SimulationMode::Simulated"
    (+3.5 min, driver still/then disconnected)  — ZERO re-promotions

**The demotion happened ~20 seconds after founding, WHILE THE DRIVER WAS
STILL CONNECTED AND ANCHORED AT THE EXACT COLONY POSITION** — this is not
the "disconnect kills the observer" mechanism the pre-registration doc
assumed. It is earlier and more fundamental than that.

The `IS-LOADED-FILTER-DIAG` line confirms the consequence precisely:
`b_count` (the RAW `(entities, colonists, uids, needs_storage)` join, before
any `is_loaded` filtering) itself drops from `8` to `0` at the same moment —
not just filtered out, the ECS components are gone. This matches
`hook_rtsim_entity_unload`'s own documented behavior (`bastion_force_demote`'s
doc: "the sync loop's demote arm... deletes the entity").

**Traced the demotion trigger to `server/src/lib.rs:5320-5353`** — a periodic
sweep that deletes any rtsim-linked entity whose anchor condition says its
chunk is unloaded:

    Anchor::Chunk(hc)  => delete if BOTH current chunk AND home_chunk unloaded
    Anchor::Entity(e)  => delete if e is no longer alive
    None               => delete if current chunk unloaded

`bastion_spawn_colony_seeded` (`server/src/rtsim/mod.rs:472`) sets each
colonist's rtsim `home` to the **nearest EXISTING SITE** (a real town, e.g.
"Strach" in this world — visible in the driver logs' ambient NPC chatter),
**not the founding position**. Exactly which `Anchor` variant these
colonists get and why the demotion fires only ~20s post-promotion despite an
active terrain anchor at the colony's own position is the part **not yet
fully traced** — the mechanism is real and reproduced twice (this diagnostic
AND independently below), but the precise reason the anchor doesn't keep the
chunk `real`-loaded long enough needs someone to go deeper into
`server/src/sys/terrain.rs`'s chunk-eviction sweep and the anchor
view-distance plumbing.

## This retroactively touches #105's ACCEPTED leg

Checked `script-14`'s own already-committed log (`server-stdout-
founding105b.log`, the run #105 was ACCEPTED on — continuously connected for
the entire ~670s run, never disconnecting early):

    promoted: 8   demoted: 8   ate/slept/BREAKDOWN: 0

**The exact same promote-once-then-permanently-Simulated pattern, in a run
that was never unattended at all.** #105's acceptance is about the FARM/HAUL
loop specifically (sow→grow→harvest→reseed, 56/56/56, still correct and
still a real result — that system evidently runs through a different path
that doesn't require `Loaded` mode, matching that job-claim activity WAS
observed in item 8's real run too). **But #105's own doc never claimed
anything about needs/rest/hunger/despondency, so nothing there needs
retracting — this is a new, separate finding about a system #105 never
exercised**, not a contradiction of it.

## What this means for item 8

Measures 1 (despondency trend), 2 (eats/cycle), 3 (sleeps/cycle), and 5
(no permanent stall via `NeedCrossed`) are **ALL** downstream of the
need-order loop, which requires `is_loaded`, which requires `Loaded` mode,
which — per this finding — no founded colonist holds for more than ~20
seconds under the CURRENT founding mechanism. **Launching the scored run as
designed would not produce a "recoverable but degraded" result — it would
produce a permanent, structural VOID on 4 of 6 measures, indistinguishable
from a dead instrument**, which is exactly the trap this arc's own laws
exist to catch before a multi-hour run, not after.

Measure 4 (food stock) and measure 6 (fail-safe rate) are NOT downstream of
`is_loaded` and would still produce real data — but a 4-of-6 void is not a
scoreable gate.

## Open questions for a ruling (not decided here)

1. **Is this a genuine engine bug** (the anchor should keep colonists Loaded
   and doesn't), or **is `Simulated` mode supposed to run its own coarse
   need/mood simulation** that simply hasn't been built yet for bastion
   colonists specifically (the WORK/farm path clearly has a Simulated-mode
   analog; the need-preemption path apparently does not)?
2. If it's a fix-before-launch bug: is the right fix in the anchor/chunk
   sweep (`server/src/lib.rs:5320`, `server/src/sys/terrain.rs`), or in how
   `bastion_spawn_colony_seeded` sets `home` (nearest site, not the founding
   position — plausibly the actual root cause, worth checking first since
   it's a one-line, low-risk hypothesis to test before touching the LOD
   sweep itself)?
3. Does this change how "unattended" should even be defined for item 8, or
   is the real prerequisite simply "founded colonists must stay Loaded,"
   or "keep the driver connected during the whole scored window" — a
   different item 8 design entirely?

## Evidence

    bastion-test-evidence/live-playthrough/script-16-item8-isloaded-diag.txt
    bastion-test-evidence/live-playthrough/server-stdout-isloaded-diag.log
    bastion-test-evidence/live-playthrough/driver-isloaded-diag.log
    bastion-test-evidence/live-playthrough/server-stdout-item8-endurance.log (killed real run, 0 events after 55 sim-min)
    bastion-test-evidence/live-playthrough/server-stdout-founding105b.log (retroactive check, ALREADY committed, 0 events over 670s)
