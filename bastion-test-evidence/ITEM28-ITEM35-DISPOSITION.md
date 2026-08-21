# ITEMS 28 (tool wear) + 35 (injuries) — DISPOSITION: **BOTH VOID, and the VOIDs are the finding**

Scored against `ITEM28-ITEM35-PREREGISTRATION.md`. Arm `injury`, attested
fresh, `dirty .rs 0`. Neither row FAILED its mechanism. Both are **unreachable
in live play**, which is the same defect class as F13 and was invisible while
each sat marked "BUILT".

## Item 28 — tool wear: VOID, no colonist has a tool

The wear witness (added for this leg, because the row previously emitted
nothing at all) fired **0 times**, on a leg where mining demonstrably worked —
`mine generator STATE demand=8 supply=8` and 8 beds built from self-mined
stone.

**Precondition unmet, and it is not the fixture's fault.** Wear is gated on a
*matching equipped tool*, and the only thing in the codebase that equips a
colonist is `bastion_equip_tool` — a **harness hook**, called from scenarios.
Nothing in founding, adoption, or any live path ever puts a tool in a
colonist's hands.

**So: tool wear cannot fire in a real game.** The code is correct and
unreachable. Bars 1–3 are VOID because the population under test never
received the treatment; scoring them FAIL would blame the wear logic for a
missing tool economy.

The row's real gap is upstream: **colonists have no way to acquire or equip
tools.** That is a genuine unbuilt feature, not a defect in item 28.

## Item 35 — injuries: bar 1 VOID, and wounds have no behavioural consequence

- Wounded colonists existed: health **0.39 / 0.73 / 0.76 / 0.78**.
- Beds existed: **8**, self-built.
- Tend jobs created: **0**.
- Sleeps: **7**, and every single one logged `health=1.0 tended=false`.

That last line settles it. The tend generator requires a wounded colonist
*occupying* a bed, and **every sleeper was at full health**. It did not fail to
fire; it was never eligible to.

**The finding: a wound does not send a colonist to bed.** Only low `rest`
drives sleep. A hurt colonist keeps working at full tilt, so tending can only
happen by *coincidence* — when someone happens to be tired and injured in the
same moment. Bed healing and the 2.5× tend multiplier are both real and both
sit behind a door nothing opens.

Bar 2 (`tended=true` observed) and bar 3 (health rising in a bed) are
consequently VOID as well. Bar 3 has been seen once, incidentally, in a play
session (0.72 → 1.0) — which confirms bed healing works, and is not a bar.

## What this pair says about the sweep

Two rows marked **BUILT** in the arc index, neither with a disposition, and
both turn out to be mechanisms nothing reaches. That is exactly what F13 was,
and it is the argument for the sweep: "BUILT" recorded that code exists, not
that a colony ever executes it. A disposition would have caught both the day
they landed.

## Banked

**Should a wounded colonist seek rest?** Recommendation: yes — a rest
interrupt weighted by injury, so a hurt colonist goes to bed and the tend
economy has a live path. It is a behavioural rule with real gameplay feel
(a colony that downs tools when hurt reads very differently from one that
doesn't), so it is Ben's, not mine.
