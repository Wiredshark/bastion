# ITEM 36 (death) — pre-registration v2, 2026-08-21

**Registered before the leg runs.** Written because the RECORD and the
EVIDENCE disagree, which is itself the finding worth resolving first.

- **Row F11 says colonists CANNOT die:** "Health reached 0.0 under repeated
  smite: no death event, no despawn, population unchanged."
- **Three play sessions this session watched populations fall** 8→1, 8→2 and
  8→4, with deaths clustered (four inside 600 ticks in one run).

Both cannot be true. Either F11 is stale, or the population drop is something
other than death (despawn, unload, a counting artifact). Nobody could tell,
because until `623bf76b3f` **nothing in the build emitted a death at all** —
one session said so in as many words, and could not separate "starved" from
"eaten" with the instruments available.

## PREDICTION

Arm `smite` — the god-hand drives a colonist's health to 0.

1. **`bastion: COLONIST DIED` appears**, carrying uid, damage and `by`. This
   is the emitter landed this session at the single authoritative death
   transition (the only place `!is_dead && should_die()` runs).
2. **The colony's population drops.** `total=` in the EXPERIENCE census, and
   `pop=` in the colony-drive line, must both fall. Two independent counters,
   because one falling alone would mean a bookkeeping divergence rather than a
   death.

## FALSIFIERS, each with a distinct meaning

- **No `COLONIST DIED` and no population drop** ⇒ F11 stands: death is still
  impossible, and the play sessions' population collapse was something else
  entirely — which would make those reports' headline claims wrong and is a
  result I would rather find than not.
- **`COLONIST DIED` fires but population does not fall** ⇒ the death event
  happens and the colony's own census cannot see it. Worse than no death,
  because every downstream signal (drive, wealth, food-per-cap) would then be
  computed against a phantom colonist.
- **Population falls with no `COLONIST DIED`** ⇒ colonists are leaving the
  census by some path that is not death (unload, despawn, component loss), and
  the play sessions were watching a disappearance, not a killing.

## Explicitly NOT claimed by this row

That death is *tuned* — no lethality curve, no starvation lethality (F10 is a
banked gameplay ruling and stays banked). This row asks only whether death
EXISTS and is VISIBLE. Everything else is a later row.
