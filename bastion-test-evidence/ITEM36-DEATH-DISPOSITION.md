# ITEM 36 (death) — DISPOSITION: **MECHANISM RESOLVED**, ruling banked

Scored against `ITEM36-DEATH-PREREGISTRATION-V2.md`, registered before any leg
ran. The result landed on **falsifier #2**, which the pre-registration named as
*worse than no death at all* — and it was right to.

## What the pre-registration was trying to settle

The record and the evidence disagreed. Row F11 said colonists **cannot die**
("health reached 0.0 under repeated smite: no death event, no despawn,
population unchanged"), while three play sessions watched populations fall
8→1, 8→2 and 8→4. Both could not be true.

## Three VOID legs first, and they were not wasted

| Arm | Result | Why VOID |
|---|---|---|
| `smite` | health floored at **0.92** | The god-hand does ~4% health a cast; two casts cannot kill. This arm tests the FAVOR GATE (re-confirmed live: refuses at 2 < 5, fires at 12+), not death |
| `hostile` | no health moved at all | Wolves killed nobody inside 12,000 ticks |
| — | — | The play sessions' deaths took 50,000–90,000 ticks under raids |

**Death was UNTESTABLE, not absent.** Every "colonists cannot die" claim on
record rested on a treatment that never reached the population. That is the
finding the three VOIDs bought, and it is why they were scored as VOID rather
than as evidence for F11.

## The fixture that made the question answerable

`BASTION_PLANT_LETHAL=<n>` kills n colonists at a fixed tick, victims chosen by
lowest uid (sorted — ECS join order is not a promise, and a fixture that killed
a different colonist on two runs of one seed would make every downstream
comparison incomparable).

**It emits a `HealthChangeEvent`; it does not call `change_by`.** This is the
whole design of the fixture. `!is_dead && should_die()` runs in exactly ONE
place — the event handler — so a direct write to zero produces a colonist
sitting at 0.0 health who never dies. That is *precisely* what F11 recorded, and
a fixture that reproduced the same mistake would have "confirmed" the bug while
testing nothing.

## The result

```
bastion: ITEM 36 LETHAL PLANT — killing a colonist by event uid=3
bastion: COLONIST DIED uid=Some(3) damage=-200.0 by=None protected=true
census tick=9300 total=8      ← unchanged
pop=8                          ← unchanged
```

1. **`COLONIST DIED` appears — PASS.** The death transition fires correctly.
2. **Population drops — FAIL.** Both counters hold at 8.

## The mechanism, and it is not a bastion bug

`protected=true`. Every colonist is a `Body::Humanoid`, and vanilla's
`has_death_protection()` is `matches!(self, Body::Humanoid(_))` — so colonists
inherit **the player's own downed mechanic**. The handler emits `DownedEvent`
instead of `DestroyEvent`. They go down and stay down.

**F11 is therefore RESOLVED and was misdiagnosed.** Death detection was never
broken and no event was missing; colonists are simply death-protected. The
symptom ("health 0.0, no despawn, population unchanged") is exactly what being
downed looks like from outside.

## Fixed here, regardless of the ruling

The EXPERIENCE census now reports `downed=N`. A body on the floor was counted
as an able-bodied worker, so food-per-cap, beds-short and the colony drive were
all being computed against a colonist who cannot work. **Added rather than
subtracted from `total`** on purpose: changing `total` would silently rebase
every banked corpus number that quotes this census.

## Banked, not decided

Whether colonists should die outright or stay downed-and-revivable is a core
design identity call (permadeath vs. rescue), so it is Ben's.
**Recommendation on file: they should die** — a colony sim where nobody can be
lost has no stakes, and every consequence system this arc built (raids,
injuries, starvation) is weightless without it. Reversible in one line at the
colonist spawn.
