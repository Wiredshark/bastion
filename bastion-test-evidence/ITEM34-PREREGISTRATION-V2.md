# ITEM 34 (raids scale with wealth) — pre-registration, 2026-08-21

Marked BUILT in the arc index with **no disposition**. Bars registered before
the legs run.

## The A/B is free, and it is a real one

Wealth bands are `0..=63 → 0 raiders`, `64..=255 → 1`, `256..=1023 → 2`,
`_ → 3`. Two arms that already exist differ in exactly the variable this row
claims to be driven by:

- **`injury`** — the colony now mines only to its par-stock floor (8 stone).
  Wealth sits in **band 0**.
- **`matbeds`** — identical arm plus `BASTION_SEED_MATERIALS=64`. Wealth
  crosses into **band 1+**.

One declared variable, opposite predictions. That makes this a comparison
rather than a demonstration.

## BARS

**Bar 1 — raids FIRE when there is something worth taking.** On `matbeds`:
`ITEM 34 RAID` appears, with `raiders >= 1` and a `wealth` value in the band
its raider count claims.

**Bar 2 — SILENCE is the null, and it must be observed, not assumed.** On
`injury`: **zero** raid emits. A colony with nothing worth stealing is never
raided — band 0 is silence by design, and this is the row's own
couldn't-happen witness. If raids fire here too, the wealth signal is not
driving anything and bar 1 passes for the wrong reason.

**Bar 3 — raiders arrive from OUTSIDE.** The spawn is `origin + 48·(cos,sin)`,
so a raider must not appear on the pantry. An adversary play session read the
emitted `origin` as the spawn point and reported raiders "spawning on top of
the colony's own stockpile" — that is the LOG being ambiguous, and the bar is
here to settle which it is.

**Bar 4 — DETERMINISM.** This spawner drew from OS entropy until it was
self-caught this session. Two runs of one seed must now place raiders
identically. Owed as a twin run; recorded as owed rather than assumed if the
budget does not reach it.

## FALSIFIERS

- Raids on `injury` ⇒ band 0 is not silent; the wealth gate is decorative.
- No raids on `matbeds` ⇒ either the cadence never fired in-window or wealth
  never crossed 64 — the emit carries both, so the log must say which.
- Raider spawn == origin ⇒ bar 3 fails and the play session's reading was right.
