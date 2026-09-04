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
