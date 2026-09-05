# RESULTS — G1d: a grown plot survives a restart (the growth log)

Live test 2026-09-04 16:23-16:56 on arm b3, pair a2bdd04273 (G1d
integrated over F2's predecessors), house lever on, PLAY.ps1
-KeepWorld for the second boot.

## What was done

Fresh boot; PLOT PLAN QUEUED at 16:24 (plan 81, 1,909 cells,
growth_log=1); the first day line at 16:50 read placed=61; 150 s more
for two rtsim saves; the server stopped by PID (Stop-Process -Force);
booted again on the same userdata.

## What the second boot said

| witness | value |
|---|---|
| GROWTH LOG READ FROM SAVE | entries=1 registered=0 |
| PLOT RE-GROWN FROM THE LOG | entry 0, House, seed 0, the same aabr (7618..7642, 6504..6516), blocks 10,549, cells_remaining 1,909, already_standing 8,640, plan 94 |
| GROWTH LOG REPLAY DEFERRED | 5 (IndexShared, then it went through) |
| GROWTH LOG REPLAYED | entries 1, re_grown 1, skipped 0, refused 0 |
| HOUSE REQUESTED / HELD / PLOT PLAN QUEUED (new) | 0 / 0 / 0 |
| panics | 0 |
| HOUSEHOLDS | houses 116, occupied 48 (before the restart: 58 / 49) |

## Disposition

The growth log round-trips: the entry was saved, read back, re-grown
into the site at the same footprint, and no second house was asked
for. That is G1d's claim and it holds.

Two things the same boot showed that are NOT G1d's:

1. **The placed cells did not survive the kill.** `cells_remaining
   1,909 / already_standing 8,640` is the first plan exactly (8,017
   underground + 623 unchanged); the 61 placed blocks were gone.
   Veloren writes modified chunks on unload and on shutdown
   (`TerrainPersistence::unload_chunk`, `unload_all` from the
   server's shutdown, and `Drop`), not on a timer; a hard kill loses
   every edited chunk still loaded. The test's stop was
   `Stop-Process -Force`. A retest needs a graceful stop; Ben's own
   quits are graceful (the client's quit and the server's Ctrl-C).
2. **A restarted world founds itself twice.** Boot 2 replayed the 72
   saved orders ("colony orders replayed from save") AND ran the
   autofound again ("ADOPT-A-TOWN site chosen ... chosen_houses=58",
   source "spawn-fallback"), so the 58 houses were registered twice:
   HOUSEHOLDS 58 -> 116. On a kept world the housing gate would count
   double and the settler gate open on phantom beds. This is the
   restore seam, not the growth log: a saved colony must stop the
   founding from running again (row R1).

## Still not evidenced

A half-built house resuming with fewer cells than its first plan
(needs the graceful stop); a finished house skipped as registered;
HOUSE REQUEST HELD followed by a request once the replay lands.

## R1: a kept world is not founded twice (f9622e2b73)

The first restart test's second boot replayed the saved orders AND ran
the autofound again: `colony orders replayed from save` (72 orders)
and, in the same minute, `ADOPT-A-TOWN site chosen ... chosen_houses=58
source=spawn-fallback`; HOUSEHOLDS went from houses=58 occupied=49
before the stop to houses=116 occupied=48 after it. Every house
registered twice; the housing gate counted double.

Mechanism: the founding branch reads rtsim `bastion_home_anchor`
first; with a colony saved, the marker wait and the town pick stand
down and the boot prints `COLONY RESTORED, NOT FOUNDED`.
`bastion_founding_decision(colony_saved, dtick, blocked_on_marker)` ->
restored | wait | found is the pure decision, pinned by
`a_saved_colony_is_restored_not_founded`.

Falsified at the commit: the saved arm dropped (`if false &&
colony_saved`) turned the pin red at `server/src/lib.rs:8985`.

Live evidence: the second restart test (the graceful-stop lever, then
`-KeepWorld` on this pair) reads HOUSEHOLDS across the boot and counts
`ADOPT-A-TOWN site chosen` lines; its section follows.

## The second restart test (R1 pair f9622e2b73, b3, 18:40-19:30)

| step | result |
|---|---|
| boot 1, plan queued | plan=79 cells=1,909 growth_log=1 at 18:41 |
| placed before the stop | 0 (the 20-cell wait ran out; the accidental builders had not started) |
| graceful stop (`BASTION_SHUTDOWN_FILE`) | ports closed in ~9 s; `HARNESS SHUTDOWN FILE seen` 1; "Rtsim save thread finished" |
| boot 2 (`-KeepWorld`) | GROWTH LOG READ FROM SAVE 1, PLOT RE-GROWN FROM THE LOG 1 (cells_remaining 1,909, already_standing 8,640), GROWTH LOG REPLAYED 1, colony orders replayed 8, panics 0 |
| the double founding | **STILL THERE**: `ADOPT-A-TOWN site chosen` 1, `COLONY RESTORED, NOT FOUNDED` 0; households 58 -> 116 |

The graceful stop works and the growth log round-trips. R1 did not
fire: it keyed `colony_saved` on `Data.bastion_home_anchor`, and that
field is `#[serde(skip)]` -- its own doc says "EPHEMERAL: recomputed
every server tick ... never persisted, None on load". The pin
(`a_saved_colony_is_restored_not_founded`) guards the pure decision and
went red when planted, but the live input it was fed is always None on
a kept world. The treatment never reached the population. R1b keys the
decision on a persisted founding mark written once at the founding.

## The third restart test (R1b pair 81dfe35b03, b3, 21:09-21:22)

| step | result |
|---|---|
| boot 1 | plan queued on day 0; 36 cells placed before the stop |
| graceful stop | ports closed in ~6 s; shutdown witness 1 |
| boot 2 (`-KeepWorld`) | `COLONY RESTORED, NOT FOUNDED` 1; `ADOPT-A-TOWN site chosen` 0; `GROWTH LOG READ FROM SAVE` 1; panics 0 |
| but | `colony orders replayed` 0, `PLOT RE-GROWN` 0, HOUSEHOLDS never printed; the tick census reads `colonists=0 designations=0 jobs=0` |

The double founding is gone and R1b's pin was red when planted
(`lib.rs:8994`). What the read exposed instead: on a headless server
the founding branch was also the thing that loaded the town's chunks
and spawned the colonists; with it standing down and no client, no
chunk loads, no rtsim colonist spawns, `pending_restore` never becomes
ready, and the restored colony is an empty server. The second test's
"replayed 8 orders" happened only because the refounding loaded the
town. R1c must make the restored branch load the town (the spawn point
and the kept-loaded region the founding sets) without founding; the
honest live test of a restore needs a client on the arm or that
loading. Not evidenced: what Ben's client sees on a kept world (the
player spawns at the world spawn, not the town, until R1c).

## R1c: a restored colony is loaded, not founded (dc66b50ece)

`bastion_autofound_restore()` runs from the restore arm: the founding
arm's spawn-point derivation (mirrored), `bastion_adoption` (site pick,
plot census, maps), the map ingestion (roads, wall margins, interiors,
settlement bounds, tile graph, buildings, the plaza anchor),
`bastion_adopt_stream_plot_chunks`, `bastion_found_colony_presence` and
the spawn point; it does not adopt residents, spawn seeded colonists or
re-place plots. Pin `the_restore_path_streams_the_town_and_adopts_nobody`
scans the fn body for the three calls present and the three absent.

Registered before the fourth test: COLONY RESTORED 1, ADOPT-A-TOWN site
chosen 1 (the site pick runs, no residents), colony orders replayed > 0,
PLOT RE-GROWN 1, the tick census `colonists` near the pre-stop roster,
HOUSEHOLDS houses=58 (not 116). Falsified if colonists stay 0 or the
houses read 116.

### The fourth restart test (R1c pair dc66b50ece, b3, 21:57-22:10)

| step | result |
|---|---|
| boot 1 | 21 cells placed before the stop |
| graceful stop | ports closed in ~6 s; shutdown witness 1 |
| boot 2 | `COLONY RESTORED, NOT FOUNDED` 1; `COLONY RESTORED — maps re-derived, chunks streamed` 1 (plots_streamed=70); `ADOPT-A-TOWN site chosen` 1 (the site pick, no residents: `ADOPT-NPCS` 0, `colony population established` 0); `colony orders replayed` 4; `PLOT RE-GROWN FROM THE LOG` 1 (cells_remaining 1,909); 8 farm plots registered; tick census `colonists=49` (the pre-stop roster); panics 0 |
| but | `ITEM 39 tick cost tick=312 jobs=376300 minted_delta=376308 p95_us=102419` (boot 1: jobs=250, p95 662 us); claim census `considered=376299 materials=375048`; mine generator `demand=375048` |

The colony restores -- people, fields, plan, no second founding -- and
the restored board mints a storm of jobs that want a material, so the
server runs at 100 ms a tick. The pin (`the_restore_path_streams_the_town_and_adopts_nobody`)
went red when planted (`lib.rs:9106`); it guards the restore's calls,
not the generators that run on what they restored. No witness names
the storm's kind; R1d-i adds the top five (kind, required item) pairs
to the tick census, and the fifth restart test reads it. HOUSEHOLDS
had not printed by the read. A kept world is still not for Ben's
session.

### The fifth restart test (R1d-i pair 60818fe1fc, b3, 22:44-23:05)

The restored boot's tick census: `jobs=374640 top_kinds=Designated(Bed):stones=368150
Designated(Build):stones=5234 Designated(Chop):-=1050 Designated(Mine):-=98
Designated(Farm):-=47`, p95 100-114 ms, colonists 49. The replay's
own lines: `designation placed kind=Bed jobs=6811 / 8629 / 7402 / 5677
/ 2773 / 8505` per saved house order; `ADOPT-IN-PLACE house registered`
0. So the storm is the replay handing each saved Bed order (a standing
house, registered at the founding by `adopt_beds_surface` with no
jobs) to `place_designation`, whose construction tail mints a
stone-wanting job per wanted cell of the house's region; the plan's
saved Build order did the same at 5,234.

R1d (registered before its binary): `replay_placement(kind)` -- a Bed
order re-registers its standing beds in place over the saved region, a
Build order goes back on the designated list for the generators, every
other kind replays as before. Bars for the sixth test: the restored
boot's jobs within a few hundred of the pre-stop boot's, p95 under 5
ms, no Bed kind above 100, ADOPT-IN-PLACE 58, colonists near 49,
HOUSEHOLDS houses=58. Falsified if any kind reads above 10,000 or the
tick p95 stays above 20 ms.

R1d landed as 43008d9fdb (23:37). Falsified at the commit: a Bed order
replayed as a build order (`Bed => ReplayPlacement::Place`) turned
`the_replay_registers_and_does_not_mint` red (23:40); the tree restored
to 0 dirty files. The sixth restart test runs on b3 on this pair; its
section follows.

### The sixth restart test (R1d pair 43008d9fdb, b3, 23:38-23:52)

| bar | registered | read | verdict |
|---|---|---|---|
| jobs after the restore | within a few hundred of the pre-stop boot | 1,369-1,405 (pre-stop boot ~1,400: the work zone's 1,049 Chop are minted at the founding too) | PASS |
| tick p95 | < 5 ms | 592-735 us (fifth test: 100-114 ms) | PASS |
| top kinds | no Bed above 100 | Chop 1,049, Build 98, Mine 73, Cook 58, Farm 32; no Bed | PASS |
| ADOPT-IN-PLACE | 58 | 58 | PASS |
| colonists | near 49 | 49 | PASS |
| COLONY RESTORED / ADOPT-A-TOWN site chosen | 1 / 1, no residents | 1 / 1 (`no residents adopted, no plots re-placed`) | PASS |
| GROWTH LOG READ / RE-GROWN / REPLAYED | 1 / 1 / 1 | 1 / 1 / 1 (plan 82, the same aabr) | PASS |
| colony orders replayed | > 0 | 6 | PASS |
| panics | 0 | 0 | PASS |
| HOUSEHOLDS houses=58 | 58 | not printed yet (once a day; boot 2 was day 0's second half) | pending |

The job storm is gone and the kept world comes back as itself. What
this test also settled, the other way: **the 23 cells placed before the
GRACEFUL stop did not survive it either** (`cells_remaining 1,909,
already_standing 8,640`, the first plan exactly). The reason is not
the stop: `experimental_terrain_persistence` defaults to `false`
(`server/src/settings/mod.rs:234`), the arms' `settings.ron` does not
set it, and the persistence module is only inserted when it is true
(`server/src/lib.rs:775`). Nothing the colonists place, dig or sow is
ever written to disk on these arms. A kept world without it re-grows
every plot from the log and the builders start the house again. That
is the next restart row: turn terrain persistence on in the harness
and read `already_standing` above 8,640 after a stop.

## R2: a kept world keeps the colony's work (registered 00:05, before the binary)

Read from the producer, the setting alone would not have been enough:
`TerrainPersistence::set_block` has exactly two callers, both admin
commands (`cmd.rs:836`, `:1018`). Every colony write goes through
`BlockChange` and is applied by `State::apply_terrain_changes` through
`on_block_update` in `Server::tick`, where nothing recorded it.

Mechanism: `bastion_record_applied_changes(persist, diffs)` at that one
apply site records every applied diff into the store when it is present
(`BASTION_TERRAIN_BLOCKS_RECORDED`, witness `TERRAIN PERSISTED`); the
harness splices `experimental_terrain_persistence: true` into
`settings.ron` for fresh and kept worlds (a false is flipped, an absent
key inserted; BOM-free; refuses to boot if the splice missed). The
boot witness is the server's own `terrain persistence path` line.

Pin `the_applied_changes_are_recorded_for_the_next_boot` (veloren-server):
two blocks recorded into a temp-dir store, `unload_all`, a fresh instance
applies them into a blank chunk at their chunk-relative cells. Planted
defect: the loop counts but never writes.

Bars for the seventh restart test (b3, `g1d-restart-test7.sh`):

| bar | sixth test | R2 bar |
|---|---|---|
| `terrain persistence path` at boot | 0 | 1 per boot |
| TERRAIN PERSISTED blocks_total, boot 1 | -- | thousands within minutes (the founding planting alone ~7,000) |
| terrain/ files after the graceful stop | -- | > 0 |
| PLOT RE-GROWN already_standing after the restore | 8,640 (placed 23) | 8,640 + placed (within a few) |
| cells_remaining | 1,909 | 1,909 - placed |
| R1d's bars | held | still held (jobs, p95, ADOPT-IN-PLACE 58, colonists 49) |

Falsified if already_standing reads 8,640 with TERRAIN PERSISTED > 0
(recorded but not re-applied on chunk load), or TERRAIN PERSISTED stays
0 (the setting did not reach the server).

R2 landed as 211da28ef6 (01:38; its first chain refused on a binary
target's zero-test line while the library pin had passed, and the
resume chain ran the pin with `--lib`). Falsified at the commit: the
record loop counting without writing turned the pin red at
`server/src/lib.rs:9177` (01:54); the tree restored clean. The seventh
test's boot 1 (01:45): `experimental_terrain_persistence: true`
spliced without a BOM, one `terrain persistence path` line, no parse
failure, `TERRAIN PERSISTED blocks_total=7736` within a minute (the
founding planting).

## R3: a kept world keeps its larder (registered 00:31, before the binary)

The sixth test's restored boot, read further: `FOOD-WIPE DISCRIMINATOR
tick=300 in_stockpile=65 on_ground_total=65` (a fresh boot: 3,636);
`SETTLER GATE CLOSED — famine day=0 roster=49 food_stock=0`, then
`day=1 food_stock=256 days_of_food=1.6`; `STARVING COLONISTS` by day 1;
`YEAR CENSUS day=1 store_units=529 store_seeds=6` (fresh: ~12,000 and
8,400). The colony restores and starves: the larder only ever lived in
`PickupItem` entities in store cells, and the server persists no item
entity. Every kept world loses the founding larder, every harvest and
every haul at each boot.

Mechanism: `JobBoard.store_snapshot` (every pickup item in a store
cell, from the discriminator's join at its tick, aggregated per
(cell, def) by `store_snapshot_from`, sorted); rtsim
`Data.bastion_store_items` (`#[serde(default)]`) written from it at
every save (`STORE SNAPSHOT SAVED cells= units=`); read once at boot
into `JobBoard.pending_store_restore` (`STORE READ FROM SAVE`) and
drained once into `PendingSeedItems`, the founding's deferred-delivery
queue (`STORE RESTORED`), which lands the items in the town's general
store when its chunks load (`deferred seed items DELIVERED`). Pin
`a_kept_world_keeps_its_larder` (bastion-server); planted defect: the
store filter dropped, road litter saved.

Bars for the eighth restart test (b3, `g1d-restart-test8.sh`):

| bar | sixth test | R3 bar |
|---|---|---|
| STORE SNAPSHOT SAVED units, boot 1 | -- | within a few hundred of STORAGE SUMMARY general_units (~12,000) |
| STORE READ FROM SAVE / STORE RESTORED, boot 2 | -- | 1 / 1, the same units |
| DELIVERED units, boot 2 | -- | ≈ the restored units |
| discriminator in_stockpile, boot 2 (tick 300-900 / +5 min) | 65 | within 15% of boot 1's last read |
| SETTLER GATE CLOSED (famine) on boot 2, day 0 | 1 | 0 |
| R1d's and R2's bars | -- | still held |

Falsified if STORE RESTORED prints and the discriminator stays under
1,000 (held or dropped delivery), or the saved units read far below the
summary (the store filter misses the town's stores). Not evidenced by
this row: private shelves (they come back into the general store),
bags, a save taken mid-delivery.

Also seen on that boot, an instrument gap and not a defect:
`HOUSEHOLDS` prints only when a bed assignment changed that pass
(`assigned > 0 || released > 0`), so a restored boot with a settled
roster never prints it; `BED-HOUSE CENSUS` on the same boot reads
`beds_total=116 beds_owned=49 household_members=49 houses=58`, which
is the restored roster in its houses. R3 carries the rider (R3-i): the
line also prints on the first pass of each game day, so the eighth
test reads `houses=58` on the restored boot's first day line.
