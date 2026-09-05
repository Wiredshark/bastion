# RESULTS — W8: the route fault at the stall

## The defect (b2 F2c'-b, 2026-09-04 day 1; b1 F2c'-b day 2)

b2 day 1: 59 wedge probes, 48 of them cooks, 45 of those at two spots
between the kitchens (y 6388-6397, z 181) and the store (y ~6356, z
182): (7649,6390,183) and (7685,6390,183). `assist_why=head_far`,
`path_state Exhausted`, the route head three blocks south at the
walker's own z, `last_push_site chaser-refused-rock`; the 5x5 map: the
cell one south solid at z+0, air at z+1, solid at z+2 -- a one-block
slot. The spot pre-exists the dense founding planting (ten cook
stalls at the same feet on the F2c' pair) and no adopted field spans
y=6390. A non-eat fetch expires on its first stall (`FETCH STALLED ...
tolerated=false`) and the expiry shuns the target cell 13,500 ticks.

b1 day 2, the consequence: `cooked_today` 90 -> 40, `targets_shunned`
12 -> 110, 8 starving at the evening line with 3,698 units in store;
`STORE WOULD CLOSE` 7 times ("three stalls on one spot aimed at this
store"). The kitchen could not fetch from its own store.

## Instruments before the fix

W8-i (071312b7c2): `nearby_bodies` and `nearby_bodies_3` on the probe.
First read (b2, F2c'-c pair, 24 probes): 0 at every one. Crowding
refuted; the stalls are geometry.

## The mechanism (W8-f, b4a1eb9aa6)

`route_fault_at_stall(assist_why, search_exhausted)` = `head_far` and
Exhausted. At the first stall, when it holds and no climb was taken,
the chaser's route is dropped (a fresh search from the feet next
tick), the stall warning is cleared so the clock re-arms, `expires` is
cleared (no shun, no early end), and the job's one re-path
(`ROUTE_FAULT_REPATHS_PER_JOB`) is spent; the second stall on the same
job expires as before, so a true trap still ends the fetch, thirty
seconds in instead of fifteen. Witness `ROUTE FAULT AT THE STALL`.
`BASTION_NO_ROUTE_FAULT_REPATH` restores the old path.

Falsified at the commit: `||` for `&&` turned
`only_a_far_head_with_a_spent_search_is_a_route_fault` red at
`bastion_jobs.rs:52450`.

## Registered bars (the W8-f boot of both arms, from 22:22)

targets_shunned per day under 20 (was 59-110); cook stalls at the two
spots a handful, with `ROUTE FAULT` lines in their place (~40 a day);
`cooked_today` at 80 or more; no `STORE WOULD CLOSE`. If the
re-searched route runs into the same slot, the shun count falls only
by half and W8-ii's node verdicts name the next fix.

## Day 1 on the W8-f pair

(pending)
