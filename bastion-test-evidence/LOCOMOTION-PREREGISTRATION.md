# PRE-REGISTRATION — why a colonist stops dead on flat ground

Written **before** the run. No code change: the instrument already exists and
has simply never been switched on.

## Why this row

The release census makes it the largest cost in the colony by a wide margin,
in both legs, across every job class at once:

| class | route-exhausted timeouts (baseline → treatment) |
|---|---|
| deposit | 150 → 205 |
| haul | 65 → 101 |
| cook | 57 → 73 |
| work | 48 → 23 |
| rest | 26 → 25 |
| eat | 16 → 31 |
| **total** | **362 → 458** |

Everything else the colony fails at is downstream of this. Food is not scarce
(`food_stock` 1,551); it is unreachable.

## What is already known, and what it rules out

- Colonists stall at **dz = 0** in 85% of samples — not a climbing problem.
- **71% stop at their closest approach** — they get as near as they ever get,
  then halt.
- 310 simultaneous stuck pairs, **0 within 2 blocks of each other** — they are
  not standing in each other; median separation is 53.9 blocks.
- `job.unreachable` is set on the **stuck-strike / churn** path, not on a
  pathfinding result, and `route_exhausted` fires **0 times in both legs**.

So this is **not** a failure to find a route. It is a failure to follow one —
which also means a pathfinding *cost* change (door/road preference) could not
fix it, and that idea is withdrawn until this run says otherwise.

## The instrument

`BASTION_LEGC_DIAG` gates `bastion LEGC-DIAG: travel timeout firing`, whose own
comment states its purpose: *"disambiguates (a) Drive-gated no-Goto vs (b)
path-degraded beeline."* It fired **0 times** in both legs because nothing ever
set the variable. `BASTION_STUCK_TERRAIN_DIAG` adds what is physically at the
colonist's feet at the same moment.

Fields: `path_cached`, `steer`, `target`, `sdist`, `stuck_time`, `drive`,
`auton_travel_ok`, `has_agent`, `detour_active`, `job_pos`, `actual_pos`.

## The branches — each is a DIFFERENT root cause with a different fix

| observation | means | the fix lives in |
|---|---|---|
| `path_cached=false` | no route was ever computed; the colonist is beelining and stops at the first obstruction | pathfinding / route request |
| `path_cached=true`, `sdist` not falling | a route exists and is not being followed | steering / physics / waypoint advance |
| `has_agent=false` | no agent component — the movement system cannot drive this entity at all | entity setup, not navigation |
| `auton_travel_ok=false` | travel is **gated off** by a freeze guard; the colonist is forbidden to move, not unable | whatever sets that guard |
| `drive` ≠ Work/Personal | the need arbiter is holding it in a state that does not travel | arbiter |
| `detour_active=true` at timeout | a detour installed and still delivered no progress | detour planner |

**The branch most likely to fool me:** `path_cached=true` with `sdist` *slowly*
falling. That reads as "following the path" and would let me call the
navigation healthy, when a colonist creeping at 0.08 blocks/s still misses
every deadline. The stuck census already measured **91.6% of stalls at speed
≤ 0.1**, so I must compare `sdist` **between consecutive samples of the same
colonist**, not judge a single line.

## PASS for this run

PASS is **not** "the colony improves" — nothing here changes behaviour. PASS is
that LEGC-DIAG fires at all and the branch above is decided by counting, with
the losing branches ruled out by their own fields rather than by argument.

## What this run cannot test

- Any fix. This is diagnosis only.
- Whether the 3 downward rescues from the previous row help — unchanged here.
- Anything about doors or roads, which is the idea this run may revive or bury.

## The mistake this row is guarding against

Twice today I compared numbers across two binaries whose line numbers had
shifted, and once I read an absent key as a zero. Every count below is taken
from **one** leg, and any cross-leg comparison states the shift explicitly.
