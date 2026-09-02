# PREREG — a promised scramble is taken (W6)

Registered 2026-09-02 17:12, before the binary exists and BEFORE the
instrument that decides the mechanism has reported (W6a rides in the
D1c pair). The design branches on that read; both branches are named
here so neither is chosen after the fact.

## Defect (two arms, one town, the W4 pair family)

- b1 (W4 boot, day 0-1): 42 of 60 wedge probes have a route head two
  blocks UP onto a standable cell (solid below, air at); the climb
  assist fired once all day; the climb ban 44 times; 23 probes at
  (7748, 6328, 181), Designated(Farm) legs.
- b2 (F1 pair, 8-day year, day 1 15:00): 20 of 50 starving with 882
  units in store; 209 of 228 probes at the same cell — 135 EatFrom, 44
  Cook, 30 Farm — walking to the store at (7774, 6354, 182) on the
  plateau above the step; searches exhaust at Medium (not Longest), so
  W1's withdrawal never fires; EAT RE-TARGET x100 resets the tier.
- The route head is (7748, 6327, 183) with route_ahead (7749, 6327,
  184): the router's scramble edge (dz 2) the body cannot jump.

## Mechanism (read, not guessed)

path.rs emits scramble edges (colonist-gated, `scramble_reach`) with
dz up to 2-3 onto standable landings and prices them; the body's only
consumer for them is the MOVE ASSIST "climb" class, which is gated by
STUCK_TIMEOUT (10 s), the standability triple (head air, above air,
below solid), the 2-xy / 3-z reach, and the committed-walker filter;
the CLIMB BAN RECORDER fires on the same timeout. Generator and
consumer disagree: the router promises what the body is not allowed to
take.

## The instrument decides (W6a: `assist_why` on every probe)

Read the D1c-pair boot's probes at (7748, 6328):

| dominant `assist_why`   | mechanism to build                                   |
|-------------------------|------------------------------------------------------|
| eligible_climb          | W6-A: the climb assist fires at VAULT_TIMEOUT (1.5 s) for a promised scramble (dz 2..3, xy <= 1, standable) — before the ban clock; the body is placed on the ledge like a vault |
| ceiling_solid           | W6-B: the router's scramble landing must have TWO air cells above (agree with the body); the search then finds a ramp or exhausts at Longest and W1 withdraws the store |
| committed_walker        | W6-C: a trunk walker may take a promised scramble (the filter exempts dz >= 2 standable heads) |
| head_far / no_floor     | re-read: the probe's head is not the assist's head; instrument first |

Whichever branch: BASTION_NO_SCRAMBLE_ASSIST (or the branch's own flag)
restores today's behaviour; a pure predicate is pinned with its
planted defect.

## Pre-registered outcomes (arm b2-style run, 8-day year, by day 1 15:00; control = this b2 run)

| measure                                   | control | bar     |
|-------------------------------------------|--------:|---------|
| probes at (7748, 6328)                    | 209     | <= 20   |
| probes total                              | 228     | <= 60   |
| starving at day-1 15:00                   | 20/50   | <= 3    |
| EatFrom probes                            | 150     | <= 15   |
| climb assists that stick (DID NOT STICK class climb) | -- | repeats <= 10% of climb assists |
| falls / embeds (the embed net's counters) | --      | not above control |
| panics                                    | 0       | 0       |

FAIL branches: the assist fires and DID NOT STICK repeats -> the third
writer (W4b's line names it); probes move to the plateau's far edge ->
the descent (dz -2) is the next class; starving stays -> the store is
not the target the eaters want (read to_item again).

NOT evidenced live yet. Ben's session: colonists hopping a terrace step
instead of queuing at its foot.
