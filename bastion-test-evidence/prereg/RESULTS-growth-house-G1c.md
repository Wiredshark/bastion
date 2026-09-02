# RESULTS — the colonists build the worldgen house (G1c): the request fires, the site resolves, and the index is never unshared

Read 2026-09-02 18:08 on arm b1, pair 3249b9116e (G1c), booted 18:08
with the fixture lever BASTION_FORCE_PLOT_REQUEST=1.

| witness                          | first minute |
|----------------------------------|-------------:|
| COLONY SITE RESOLVED             | 1 (site 1, anchor (7722, 6320, 180), footprint 147,168) |
| HOUSE REQUESTED FROM WORLDGEN    | 1 (day 0)    |
| PLOT LAYOUT REFUSED              | 272, every one `IndexShared { strong_count: 2 }` |
| PLOT LAID OUT / PLAN QUEUED / BUILT | 0 / 0 / 0 |
| panics                           | 0            |

The G1b pre-registration named this branch: "if IndexShared fires every
tick on a live server, the design's premise (quiet ticks exist) is
false." It is false, and for a reason the unit tests could not see:
the Server struct keeps its own `IndexOwned` (server/src/lib.rs:283)
and inserts a CLONE into the ECS (line 1034), so the index Arc has two
permanent owners and `Arc::get_mut` can never succeed. The chunk
generator's transient clones were the assumed sharer; the struct field
is the real one.

## The OWN boot (pair 21ab563470, the ECS resource as sole owner; b1, booted 19:04, lever on)

| witness                       | value |
|-------------------------------|-------|
| WORLD INDEX OWNERSHIP         | strong_count=1 at boot |
| COLONY SITE RESOLVED          | site 1, anchor (7722, 6320, 180) |
| HOUSE REQUESTED FROM WORLDGEN | day 0 |
| PLOT LAYOUT REFUSED           | 0     |
| PLOT LAID OUT                 | plot 96, aabr (7618..7642, 6504..6516), door (7633, 6507, 180), 2 beds |
| PLOT PLAN QUEUED              | plan 81: blocks 10,549, cells 1,909, skipped underground 8,017, skipped unchanged 623, no stone bill |
| day 1: HAUL LANE CEILING      | neediest = Build; one demoted hauler returned to the Build trade |
| day 1: Build claims / cells placed | 12 / 12 (remaining 1,897) |
| day 1: stone_owed             | 1,905 (the plan's cells counted as stone demand: G1c-b) |
| panics                        | 0     |

The first worldgen house laid out on a live server, and the colonists
began placing it. Two gaps named by the same read: the mine generator
counted the house's cells as stone owed (G1c-b, queued), and the town
staffs its build with whoever already held the trade — one colonist,
12 cells a day, 158 days for the house (G1c-c: the town drafts
builders from its biggest trades while a plot plan is open; in an
isolated build). `BUILD PROGRESS placed=8652` counts the skipped
foundation as placed; corrected in G1c-c to queued minus remaining.

## Disposition

The G1b premise is now TRUE on a live server (owner count 1) and G1c's
wiring carried a request through to a queued plan and the first placed
cells. Not yet evidenced: PLOT BUILT and the house registered (needs
the builders); the render (Ben's world).

The earlier read on the G1c pair stands as the record of the premise
failing: G1c's wiring PASSED its own instruments up to the seam (site
resolved, request raised, refusal named with its count) and could not
proceed past it. Not a colony defect: an ownership fact of the vanilla server. Fix
in flight (an isolated builder): the ECS resource becomes the sole
owner and the server's dozen uses fetch it, with a boot witness that
prints the strong count (must read 1). Until then no worldgen house
can be laid out on a live server; the G1c boot on b1 is kept for its
other reads. The premise is re-tested by the same boot the moment the
fix lands.
