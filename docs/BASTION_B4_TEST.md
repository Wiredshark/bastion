# B4 self-test results — designation → job board → arbitration + pathing

Run: 2026-07-09, branch `bastion/block-B4` (`6c0ff09..643fa83`), gate per
design doc §B4 Done-when + standing invariants + Tier-1b soak (first block
with the soak in its gate). Result: **PASS**.

## Compiles

`cargo check`/`build`: veloren-server, bastion-harness, veloren-voxygen — green.

## The acceptance scenario (Done-when, all headless-phrased)

`cargo run -p bastion-harness -- --seed 1337 --b4-scenario`:

```json
{"b4_colonists_loaded":5, "b4_jobs_placed":20,
 "b4_claims_always_distinct":true, "b4_arrived_enabled":4,
 "b4_priority_honored":true, "b4_unreachable_marked":true,
 "b4_cancel_cleared_jobs":true, "b4_all_idle_after_cancel":true,
 "b4_soak_avg_tick_ms":3.5}
B4 SCENARIO: PASS
```

- **20 mine designations + 5 colonists** (force-loaded area, real promoted
  agents): every enabled colonist claimed a distinct job and **arrived at its
  site** (log lines `bastion: colonist arrived at job site, ready to work
  (B5)` with block positions). `claims_always_distinct` sampled every second
  of sim time — never a double-claim.
- **Unreachable**: a job 8 blocks underground (placed nearest, under a
  colonist) was claimed, walked at, and released by the progress watchdog —
  `bastion: job unreachable — claim released` — after which the colonist
  **immediately re-arbitrated to another job** (job 0 → job 14 in the log).
  Unreachable jobs stay unclaimed thereafter.
- **Cancel** released all claims and cleared the board within one arbitration
  cycle; all colonists re-idled.
- **Priorities honored**: the colonist with mining=0 never claimed a mine job
  at any sample point.

## Soak (Tier-1b, zero-input)

- Scenario tail: 600 further ticks, no input — no panics, **3.5 ms avg tick**
  with the colony + job board live.
- Plain colony soak: `--ticks 2000 --colony 6` — clean summary, npc count
  baseline+6, no panics across 66 sim-seconds.
- Baseline regression: `--ticks 500` byte-identical shape to pre-B4 runs
  (2355 npcs / 204 sites / 16 factions / colonist_count 0).

## Vanilla regression

Flagless boot: alive at menu after 15s, clean shutdown.

## In-game visual QA — deferred (documented)

The machine locked (user asleep, ~02:30 local) before the scripted
paint-and-watch run could execute; input injection and screen capture are
impossible on a locked desktop. Risk assessed as low: B4's Done-when items
are all headless-phrased and fully covered above, and the voxygen paint path
enters the server through `BastionPlaceDesignation` — the same board function
the scenario drives — over the message channel verified in B2a's gate.
**First action next session: paint a Mine region in-game and watch colonists
converge (the visual demo), before starting B5.**

## Standing invariants

- No panics in any run. No double-claims (sampled). Claims released on
  cancel/unreachable/demote (sweep in the system). Tick time bounded (3.5ms
  avg). Entity counts baseline+N exactly.
