# THE RELIEF EMIT'S BLIND SPOT (F2) — **PRE-REGISTRATION**

Written before any code change. Discharges **F2** from `WORLDGEN-PRESET-RESULTS.md`.

## 1 · THE FINDING, AND IT IS AGAINST MY OWN COMMIT

The instrument commit (`38270d8dcb`) says the relief line is emitted **"on every
attempt, success or refusal."** That is **false**. The emit sits inside the
`Some(datum_z)` arm of:

```rust
let refusal = if rtsim.bastion_colony_exists() { … } else {
    match datum {
        Some(datum_z) => { let relief = survey_site(…); tracing::info!(… relief …); … },
        None => Some((FoundingRefusal::Terrain, Some(origin_xy))),   // ← NO EMIT
    }
};
```

So when `resolve_datum` finds no surface at F's own column, the founding is refused
**with no relief line at all**. Measured: the water search made **24** attempts and
produced **8** relief emits. **16 attempts were invisible to the instrument built to make
absence visible** — which is the exact failure mode `ReliefBranch::Absence` exists to
name.

## 2 · WHY IT MATTERS MORE THAN A MISSING LOG LINE

An absent line and a *zero* line render identically. Today, "F is over a hole", "F is in
an unloaded chunk", and "the message never arrived" are **the same observation**: nothing.
The water search could not distinguish *unreachable* from *dry* except by counting
attempts against emits by hand — which is how the gap was found, and is not a method.

## 3 · THE DESIGN

In the `None` arm, survey against the **hint** (`pos.z`) instead of a resolved datum, and
emit with the datum's resolution state named:

```
bastion: founding site relief origin=.. datum=<hint> datum_resolved=false
         columns=60 resolved=.. min_dev=.. max_dev=.. submerged=.. branch=..
```

Surveying against the hint is not a fabricated datum — it is **the same window
`column_surface_z` was already given** and it answers the question the missing line
should have answered: *is the whole neighbourhood unresolvable, or only F's own column?*
`datum_resolved` carries the distinction rather than hiding it inside a plausible-looking
number.

**The refusal itself does not change.** `reason="terrain"`, same column, same behaviour.
This row adds a witness; it does not move a decision.

## 4 · THE BARS

### E1 · **THE INVISIBLE CASE BECOMES VISIBLE**
- **PASS:** an origin whose own column has no resolvable surface emits a relief line
  carrying `datum_resolved=false`.
- **FAIL:** no line — today's behaviour.

### E2 · **THE RESOLVED CASE IS UNTOUCHED** — the non-regression
- **PASS:** the live census reproduces **byte-identically** (datums 416/416/415,
  deviations −5/−6/−1, founding at (15248, 15984) `datum=415`, 6 colony_exists +
  2 terrain), with `datum_resolved=true` added.
- An instrument row that perturbs the measurement it instruments is worthless.

### E3 · **EMITS == ATTEMPTS** — the count bar, and the sharpest one
- The water-search script makes **24** attempts; nothing founds under it, so no attempt
  is short-circuited by `colony_exists`.
- **PASS: 24 relief emits from 24 attempts.** Today: **8**.
- Derived from the script, fixed here, before the run.

### PLANT
- Remove the `None`-arm emit ⇒ **E3 red, returning to exactly 8 of 24**, while E2 stays
  green. This proves E3 counts the *new* arm and not merely "some emits happen".

## 5 · WHAT I WILL **NOT** DO

1. **I will not invent a datum for the unresolved case.** The hint is reported as the
   hint, with `datum_resolved=false` beside it. A number that looks resolved but is not
   is worse than the missing line.
2. **I will not change the refusal.** `reason="terrain"` and the named column stay as
   they are; bars already read them.
3. **I will not accept E3 on a count alone if E2 moves.** If the census shifts by one
   field, the row failed regardless of the emit count.
4. **I will not claim F2 closed unless E1, E2 and E3 all hold**, with the plant red.
