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
