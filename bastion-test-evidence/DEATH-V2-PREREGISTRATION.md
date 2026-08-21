# DEATH v2 — pre-registration (Ben RULED 2026-08-21)

**The ruling, verbatim:** *"they get downed and can eventually die, we should
have rng (deterministic) where a colonist may just die outright depending on
factors with no chance to revive."*

Three states, not two. This supersedes my own banked recommendation (plain
permadeath), and is better than it: permadeath removes the rescue drama
entirely, while this keeps it and puts a real threat underneath it.

| State | Meaning |
|---|---|
| **Alive** | ordinary |
| **Downed** | collapsed, revivable — the vanilla humanoid path colonists already inherit |
| **Dead** | gone. Reached either by *dying while downed and untreated*, or by the **outright-kill roll** at the fatal blow |

## Part A — the outright-kill roll (this row)

At the moment `!is_dead && should_die()` fires, a colonist normally takes the
`DownedEvent` branch because `has_death_protection()` is true for humanoids. A
deterministic roll may instead deny that protection: **no downed state, no
revive window, gone.**

### DETERMINISM IS NOT OPTIONAL HERE

Ben wrote "(deterministic)" into the ruling itself, and this project earned
that parenthesis the hard way *today*: the item-34 raid spawner was caught
drawing OS entropy inside the authoritative tick, which would have made every
same-seed A/B crossing a raid incomparable — silently, because the raid still
fired and still logged.

A death roll is strictly worse to get wrong: two runs of one seed would end
with different colonists alive. The roll is keyed on `(tick, uid, domain)` via
the ChaCha8 shape already used for farm-harvest scatter. **No `rand::rng()`.**

### "Depending on factors" — what the chance reads

v1 uses **overkill** — how far past zero the blow drove them, as a fraction of
max health. A scratch that happens to land last leaves you downed; a blow that
would have killed you twice over kills you outright.

This is chosen because it is *already in the data* (the `HealthChange` amount
is right there), needs no new state, and is legible to a player: massive damage
kills, attrition downs. Personality/trait weighting is a natural successor and
is deliberately NOT in v1 — one factor, measurable, before a mixture.

## BARS

1. **The roll can kill outright.** Under a large overkill, `COLONIST DIED`
   appears with `outright=true` and the colonist leaves the census.
2. **The roll can spare.** Under small overkill the colonist is DOWNED —
   `outright=false` — and remains in `downed=`. Both branches must be observed;
   a roll that always kills is permadeath wearing a roll's clothes, and would
   pass bar 1 alone.
3. **DETERMINISM.** Two runs of one seed produce the *same* outcome for the
   same colonist. Twin-run comparison of the emitted `outright` flag and uid.
4. **Population moves.** An outright death must drop `total` in the census —
   the thing that did NOT happen when death was only `DownedEvent`.

## FALSIFIERS

- Always outright ⇒ bar 2 fails; the factor is not being read.
- Never outright ⇒ either the chance is mis-scaled or the roll is not wired;
  the emit must carry the computed chance so the log says which.
- Twin runs disagree ⇒ determinism is broken and the row FAILS outright,
  whatever else passes.

## Part B — dying while downed (NOT this row, chartered)

"Can eventually die" needs a downed-timer: untreated, a downed colonist dies
after some interval; treated (the medical-care system Ben named as the
successor to injury-rest), they recover. That needs the medical arc and is
recorded here so a green Part A is never read as the whole ruling.
