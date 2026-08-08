# TRAVEL-ROW-SPEC §4.1: min_distance_to_target / last_timeout_pos wired to the b5 harness JSON

Additive, per Opus's ordering. Two new `Server` methods
(`bastion_travel_timeout_min_distances`, `bastion_travel_timeout_last_positions`,
`server/src/lib.rs`, right after `bastion_timeout_count_for_pos`), both
read-only, no world writes, filtering the existing always-maintained
`min_distance_to_target`/`last_timeout_pos` maps down to positions that
incurred at least one travel timeout (`timeout_counts_by_pos`'s keys).
Wired into `b5_scenario`'s JSON as `b5_travel_timeout_min_distances` (a flat
array of floats, one per timed-out position) and
`b5_travel_timeout_last_positions` (an array of `{job_pos, last_pos}`
objects). No existing field's meaning changed.

**Shape decision:** the raw per-position list, not an aggregate or ratio.
Opus asked for `min_approach_ratio` *if* aggregating, to stay comparable
across targets at different starting distances — but the raw list is
strictly more informative than any single-number aggregate (it lets §4.2
derive a real threshold/quantile from the actual distribution, per "do not
guess it") and there is no `initial_distance` currently tracked to compute
a ratio from without adding a THIRD new field. Kept to two, per spec.

## Verified against real data

Seed 42 (b5_scenario): 0 travel timeouts this run → both new fields
correctly empty (`[]`), consistent with `b5_travel_timeouts: 0`.

Seed 76 (known high-friction, corpus-cited "29 timeouts, passes" — this
build's own run shows 31): `b5_travel_timeout_min_distances` has 9 entries
(9 unique timed-out positions, matching `max_same_target_timeouts: 6`
meaning 31 raw events landed on 9 distinct targets), values mostly 9-13 with
one at 29.1 — real, varied data, not a placeholder. `last_positions` shows
9 real `{job_pos, last_pos}` pairs with distinct coordinates.

## Fixture-artifact check (Opus's ask, §1 of his last message)

Seed 76's `last_pos` y-values cluster at **28754.3-28756.6** against
`job_pos` y-values of **28751-28753** — timeouts land 1-3 units past their
own targets, with real z variation (150-165, tracking actual terrain).
**This is a different shape from seed 7's signature**, which put every
timeout at the same y (16003±0.55) **11-22 units** from FOUR different
targets regardless of which target was current.

**Not a fully clean discriminator**, flagged honestly: b5_scenario's own
mine-strip targets are already close together in y (a ~2-unit spread), so
there's no equivalent in this data to seed 7's "two very different targets
converging on the identical position" test. What b5 DOES show is that
travel-timeout positions here vary naturally with terrain (z scatters
150-165) rather than pinning to one exact value — which is at least
consistent with seed 7's pinning being unusual, not the general engine
behavior, without being a full proof either way. A cleaner test would need
a b5-equivalent scenario with two widely-separated target clusters (which
doesn't currently exist) or repeating the same check across more
preempt_scenario seeds first.

## Status

§4.1 complete and verified. §4.2 (derive UNREACHED/UNREACHABLE threshold
from real corpus distribution) is next, per the spec's own ordering — needs
a real corpus run (VM fan), not single local seeds, to be sound.
