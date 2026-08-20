# Item 21 (Personalities visible) — DISPOSITION: **PASS 3/3**

Legs: `pintrait` (BASTION_PIN_TRAIT=Adventurous) + `pintraitctl` (Closed),
2026-08-20, chain8 (commit 15105225df binary), 8 colonists each, seeded.

**Bar 1 — payload carries all three, pinned archetype matches the record: PASS.**
Treatment witnessed per subject (8/8 "personality PINNED trait_=Adventurous");
every INSPECT line carries `traits=["Open", "Adventurous", "Stable"]` — exactly
`Personality::pinned(Adventurous)`'s record (openness=MAX + neuroticism=MIN
satisfies the three nested traits; co-occurrence is test-bounded in
`pinned_co_occurrence_is_bounded_by_nesting`). `desires=` and `bravery=` ride
the same payload (bravery 0.50 × 24 samples). Display = record by same-source
fill; no second resolver exists to drift.

**Bar 2 — the opposite pin FLIPS the display: PASS.** Control arm: 8/8
pinned Closed, every line `traits=["Closed"]` — disjoint from the treatment
arm's set on the pinned axis, same leg shape, same seed.

**Bar 3 — no new sim state: PASS by construction** (display-only tail-appends,
`Personality::is` / desires / bravery all read from the records gameplay reads).

**Finding attached (feeds two banked rulings): personality is BEHAVIOUR-BLIND
end to end.** Vanilla `Psyche::flee_health` derives from BODY (agent.rs:286);
`guard_bravery` is uncoupled (0.50 flat under both pins). Adventurous is
neither reckless nor exploration-seeking today — it is inert outside
display/desires. Recorded per Ben's #110 caveat ("say so rather than forcing
it"); the guard-bravery-distribution ruling is the natural coupling vehicle.
