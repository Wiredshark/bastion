# PRE-REGISTRATION — adopt-a-town settlement (commit `99a7a80217`)

Written **before** the first live leg on the new binary. Both branches stated,
so the run can refute the fix rather than illustrate it.

## What changed

Adoption no longer waits for, or selects from, a population the architect has
not created. It reads `Site.population` (authoritative membership) and then
settles the shortfall into the town's own **house plots** via
`with_home(site_id)`, which registers each resident in `Site.population`
through `spawn_npc`.

## The prediction

**PASS requires all four:**

1. A line `bastion: ADOPT-A-TOWN — houses this village can put a resident in`
   with **`houses >= 1`**.
2. A line `bastion: ADOPT-A-TOWN roll` with **`settled >= 1`** and
   `adopted_existing + settled == wanted` (or `settled` capped by `wanted`).
3. `bastion: colony population established` with **`colonists == 8`**.
4. The colonists' spawn positions are **at house-plot centres**, not clustered
   at a single origin.

**FAIL / VOID branches, named in advance so they cannot be read as success:**

| Observation | Means |
|---|---|
| `houses = 0` | The plot enumerator found no `House` plots. **Not** an adoption failure — a plot-mapping failure. Check `bastion_adoptable_town_plots`. |
| `houses >= 1` but `settled = 0` | `settle_plan` refused. Either `adopted_existing >= wanted` (**success in disguise — the architect had populated it**) or a wiring defect. *Read `adopted_existing` before judging.* |
| `adopted_existing > 0` | **The best possible outcome.** Real villagers existed and were adopted. Only reachable when founding happens after the architect has run. |
| No `ADOPT-A-TOWN roll` line at all | Adoption never ran. The autofound gate or the town lookup failed *upstream* — not this fix. |
| Colonists exist but at one position | `get_alt_approx` returned `None` for every house centre, so `filter_map` dropped them all and `houses` fell to 0. **This would print `houses=0`, so branch 1 catches it.** |

## What this run does NOT test

- **`adopted_existing > 0`.** At founding tick the architect has never run
  (fires `tick % 32`, autofound founds at tick 30), so this leg exercises the
  **settle** path only. Adopting pre-existing villagers needs a world where the
  player walks to a town *before* founding. **That is a separate leg and it is
  still owed.**
- Whether settled residents *behave* as villagers (jobs, homes, routines).
  Presence is not behaviour.

## Known contaminants in the same binary

Three other commits ride along; if a number moves, they are candidates:

- `703039d927` — guard posts no longer destroyed on arrival. **Expect Guard XP
  > 0 and fewer Flee preempts.** Both are *changes*, not regressions.
- `4100485461` — census gains `engaged`. **`working` is unchanged and still
  excludes travel** — do not read it as productivity.
- Food ratchet, cook granularity and seed-delivery fixes are **NOT** in this
  binary. Expect food to still ratchet into bags, kitchens to still multiply,
  and the adopt arm to still be seedless.
