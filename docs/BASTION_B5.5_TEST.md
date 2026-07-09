# B5.5 self-test results — zone deletion + item-drop pile aggregation

Run: 2026-07-09, branch `bastion/block-B5.5` (`b7f01d1..`), gate per the
block prompt's Done-when + standing invariants + Tier-1b soak. Result:
**PASS**.

## Compiles

`cargo check`: veloren-common (+ its unit tests), veloren-client,
veloren-server, veloren-voxygen, bastion-harness — all green.
`cargo test -p veloren-common --lib bastion`: 3/3 (Region::subtract —
disjoint identity, full-cover empties, volume conservation + pairwise
disjointness across interior/face/corner/through cuts).

## The acceptance scenario

`cargo run -p bastion-harness -- --seed 1337 --b55-scenario` — **3/3 PASS**
(first run + two repeats after the terraform fix below):

```json
{"b55_p1_jobs":36,"b55_claims_before_erase":4,
 "b55_jobs_in_half_before":18,"b55_jobs_in_half_after":0,
 "b55_orphans_after_partial":0,"b55_remainder_progressed":true,
 "b55_board_after_whole":0,"b55_orphans_after_whole":0,
 "b55_all_idle_after_whole":true,
 "b55_p2_jobs":200,"b55_p2_cleared":true,
 "b55_stone_sum":200,"b55_stone_entities":25,
 "b55_stone_sum_after_soak":200,"b55_soak_avg_tick_ms":3.9}
B5.5 SCENARIO: PASS
```

- **Partial erase** (Part 1): 6×6 mine zone (36 jobs), 4 colonists
  claimed (all 4, distinct); erasing the +x half removed exactly its 18
  jobs; **zero orphaned claims** one arbitration cycle later (new
  `bastion_orphaned_claims` hook: colonists holding `ActiveJob`s whose id
  is off the board); the remainder kept being worked
  (`remainder_progressed`).
- **Whole-zone delete**: board to 0 within one cycle, zero orphans, all
  colonists idle.
- **Conservation + aggregation at scale** (Part 2): a 20×10 slab (exactly
  200 jobs) mined out by 4 skill-boosted colonists → **exactly 200 stones**
  (amount sum) across only **25 pile entities** (8× aggregation vs. the
  old one-entity-per-block carpet) — and still exactly 200 after the
  600-tick zero-input soak, proving persistent piles carry **no despawn
  timer** (the old 300 s `DeleteAfter` was a latent item-loss bug for any
  pause > 5 min).
- **Soak**: ~3.9–5.2 ms avg tick with 200 stones in piles live.

## Regression suite (all after the final code state)

- `--b5-scenario`: **3/3 PASS** — updated to conservation-exact amount
  sums: `stone_sum == 27` in only **2–3 pile entities**, `log_sum == 1`.
- `--b4-scenario`: **3/3 PASS** (unchanged assertions).
- Vanilla flagless boot (`veloren-server-cli`, rebuilt with all B5.5
  changes): "ready to accept connections", zero panics.
- Vanilla voxygen builds green (the erase tool/radial verb are
  overseer-gated; vanilla input paths untouched).

## A gate-time finding (test geometry, logged to the backlog)

Part 2's first run stalled at 8/200: the slab was forced at a single
z-level across 20×10 blocks of *sloped* natural terrain — burying blocks
inside hillsides (their `+1` arrival cell became a 1-block gap no colonist
fits in) and floating others. The **fourth** manifestation of the standing
vertical-reachability trap (pit / stump / tree / now slab-on-slope). Fixed
by fully determining the terraform (per-column under-fill + single working
level + 3-block headroom + perimeter ring). The job-system code needed no
changes — the mining framework (`BASTION-SYSTEM-FRAMEWORKS.md` §6,
"access is part of the dig plan") owns the real mechanism-level fix.

## In-game visual QA

Not performed this session (headless-only; the voxygen exe is rebuilt and
ready). **Flagged for the next session / Ben's next demo**: T-cycle to the
new **Erase** tool (red drag preview) and erase part of a painted zone;
right-click inside a zone → **Delete zone**; mine a patch and watch drops
coalesce into tier-scaled piles instead of a pebble carpet. Risk low: all
Done-when items are headless-phrased and covered above; the erase path
enters the server through the same `BastionCancelDesignation` message the
scenario drives. Also still open: Ben's TRAVEL_SPEED eyeball verdict.

## Standing invariants

No panics in any run. **No item dupe/loss** — now asserted *exactly*
(amount sums == blocks mined, through merges AND through the soak), which
is stronger than B5's original ≥-bound. No double-claims / no orphaned
claims (new hook makes this directly observable). Entity counts bounded
(aggregation). Tick time bounded. Vanilla loose-loot behavior unchanged
(vanilla emitters all pass `persistent: false`; merge classes never mix).
