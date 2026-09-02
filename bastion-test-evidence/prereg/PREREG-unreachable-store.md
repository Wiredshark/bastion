# PREREG — an unreachable store is withdrawn on the search's word (W1)

Registered 2026-09-02 11:10, before the binary exists.

## What the probe found

Arm b1 on 96022099c9, game day 0: 12 of 42 stalled fetches carried
`path_state` None with an empty route, all aimed at store zone 45 (a
raised terrace behind a two-high wall; the walkers stood at (7660-7666,
6433, 180) with the item six blocks south and two up). The search said
"no path"; the choosers kept aiming at it. The S6 store-closing row was
made opt-in after it closed the barn on stalls; this row closes on the
search's own verdict, which the barn's jam spot does not produce (the
barn's fetches read Exhausted or a route with a head).

## Mechanism

At the fetch's first stall: search made + no path + no route node ->
the fetch expires now, the cell is shunned (row E2), the store takes a
strike; three strikes in the window withdraw the store for
STORE_CLOSE_TICKS (one game day) with a `STORE UNREACHABLE` line. The
eat scan, the eat re-target and the haul chooser skip a withdrawn
store's items; deposit-run and haul mints already did.
`BASTION_NO_UNREACHABLE_STORE` restores the old path. Pinned predicate.

## Pre-registered outcomes (arm b1's first day on the W1 pair)

Instrument validation first: `FETCH UNREACHABLE` lines must name zone 45
(or its successor id on that boot: the store whose region contains
(7672, 6426)); if none appear while probes with `path_state` None and no
head do, the branch is not reached.

- PASS: a `STORE UNREACHABLE` line names the terrace store within the
  first afternoon; after it, probes aimed at that store's cells stop
  (<= 2 more that day) and `haul deposited` / `forage deposited` into it
  read 0; no `STORE UNREACHABLE` line names the barn or the store at
  (7776-7780, 6356); the day's EAT CENSUS meals / eat_minted is not below
  the P1b arm-day's (47/56).
- FAIL branches: `STORE UNREACHABLE` names a store that hauls reached
  the same day (`haul deposited` into it after the line) -> the search's
  None is not a verdict on this terrain and the row is reverted; the
  terrace store is withdrawn but its food was the town's last (starving
  rises at the evening census above the P1b day) -> the town needs its
  terrace store reachable, which is a ramp or door row, not a chooser
  row; no None probes at all on this pair -> W2's bans changed the
  searches and this row is moot for the day (kept, unexercised).
- NOT a fix for Exhausted routes (W2/W3). NOT evidenced live yet.
