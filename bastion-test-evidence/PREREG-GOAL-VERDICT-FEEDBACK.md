# PRE-REGISTRATION — the exhaust verdict reaches the item pick

Successor to the cell-identity PASS (163d0a1030). The residual it names: the
top goal cell still draws ~1,200 Longest searches per treatment leg because
the eat scan and fetch-reservation pick choose `min_by_key(distance)` with no
reachability term, re-reserving the same wall-shadowed item forever. Before
cell identity an exhaust meant "budget died at ~2,500 cells" and was WORTHLESS
as a verdict; now it means "the search saw the whole region" — trustworthy for
the first time. Prior art: RimWorld marks items unreachable per pawn with a
retry cooldown; DF suspends the job. Ours is goal-keyed with a TTL.

## Mechanism (v1)

- On a chaser terminal exhaust (`(Longest, Exhausted)` — the site that already
  feeds `chaser_terminal_releases`), record `goal_cell -> expiry_tick` in
  `board.goal_verdicts` (one entry per cell, overwrite = additive-capped;
  keyed by GOAL, position-free for the colonist; TTL-decayed — the E2 revert
  constraints hold by construction).
- TTL: 2 sim-minutes (3,600 ticks) — terrain changes (doors, digs) can open
  routes, and a verdict must expire on its own clock, never wait for amnesty.
- Consumers: the eat scan, the fetch-reservation pick, and claim scoring skip
  a candidate whose cell holds a live verdict — UNLESS no unblocked candidate
  exists (fail-open identity: refusal must not starve the protectee; a
  starving colonist with only blocked food still tries the least-bad one).
- Env: `BASTION_GOAL_VERDICT=1` enables; DEFAULT OFF until the fleet A/B
  passes (the claim-penalty discipline: unproven behavior does not ship on).

## PASS / FAIL, declared

- PASS: treatment legs' top-goal search concentration collapses (top-3 share
  of Longest searches falls by non-overlapping ranges at 3v3) AND meals eaten
  does not fall AND fetch deliveries do not fall.
- FAIL(no-effect): concentration unchanged — the pick is not the re-aim door,
  or verdicts expire faster than the pick cycles.
- FAIL(overcorrect): meals eaten falls or reservation census shows starved
  fetches while food exists — the fail-open arm is broken; visible as
  eat_total dropping with food_stock high.
- VOID: no terminal exhaust occurs in a leg (nothing to record — witnessed by
  chaser_terminal_releases = 0 AND goal_verdicts never populated).

## What this run cannot test

Whether the TTL is right for terrain-change reopening (needs a dig-through
scenario; separate fixture). Whether claim-door coverage matters beyond the
two item doors (the shadow said claim candidates never included the doomed
cells; if concentration persists with both item doors gated, that finding is
falsified and the claim door gets its own pass).

## Tests to pin (both directions, planted red)

1. A verdict-blocked item is skipped when an unblocked sibling exists.
2. The fail-open: ALL candidates blocked -> the pick proceeds as if ungated.
3. Expiry: a verdict past its tick admits again.
4. Recording is overwrite, not accumulation (one entry per cell).

## Run shape

3v3 on the VM fleet (testing framework rule 9 — never local), same binary,
one env var, 50,000-tick town legs, judged by the concentration metric plus
the famine guards above.
