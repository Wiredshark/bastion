# DISPOSITION — ITEM 14 bar 1b re-run, leg 1 (Arc 3)

Scored against `ITEM14-BAR1B-RERUN-PREREGISTRATION.md`, written before the leg.
Slot 4, arena arm, port 26064, pinned binaries, tip `e72a712f1f`.

## Condition 1 — the assignment SURVIVES: **PASS**

This is the one the moot bug made impossible, and it is now met on both a
positive and a negative witness:

```
INSPECT uid=6 name=Eira Longstride ... activity=Some((Guard, 1.0))   ← at ~80s
moot events in the entire server log: 0
```

Eira's guard skill is **1**, so under the pre-fix predicate her assignment would
have been destroyed at `6.0/(1+0.2·1)` = **5.00 s** after arrival. She is still
holding the post at ~80 s, and `designations` held at 7 across every sample
(6 → 7 on the paint, then flat).

The **zero moot events** matter as much as the surviving job: the failure this
replaces announced itself in the log every time it fired, so its absence across
a whole leg is a real negative witness rather than a silence.

> **Precondition met for the first time.** Every other bar-1b measurement rested
> on "the colonist still has a guard job," and until `703039d927` that was false
> within seconds.

## Condition 2 — Guard XP > 0: **NOT SCORED**

Eira read `("guard", 1)` at baseline and `("guard", 1)` at ~80 s. That is **not**
a failure and must not be recorded as one — she had been on the post for well
under a minute, and no XP rate was pre-registered. Scoring this needs a leg long
enough that a *known* XP rate predicts a level change. **Owed.**

## Condition 3 — brave holds / timid flees at identical health: **NOT SCORABLE ON THIS POPULATION**

```
bravery=0.50   ×16 samples (8 colonists, 2 inspections) — zero variance
```

**This is not a defect, and I nearly filed it as one.** The colonists have
sharply different traits (`Worried`, `Stable`, `Neurotic`, `SadLoner`) and
identical bravery, which reads like broken personality coupling. Reading the
producer says otherwise — `BastionColonist::generate`:

> *"ITEM 14 axis 2: the NEUTRAL default, not an invented spread. The ruling says
> bravery varies by the individual (personality/veterancy) — but the
> DISTRIBUTION is a balance choice, and `rng` is right here, so it would have
> been one line to invent one. Banked for Ben instead; the fixture pins two
> distinct values via `BASTION_GUARD_BRAVERY` to score bar 1."*

So the constant is a **deliberate refusal to invent a balance number**, already
banked ("guard-bravery distribution" is in the roadmap's banked-for-Ben list).
A natural-spawn colony cannot score axis 2 **by design**; the pin is the
instrument.

**Next leg:** boot with `BASTION_GUARD_BRAVERY=0.2,0.8` — pins the first
guard-assigned colonist timid and the second brave — then drive a threat to the
post and compare hold-vs-flee **at matched health**.

## What this leg does not claim

- Nothing about axis 1 (fight vs alarm escalation).
- Nothing about patrol, which remains a separate open finding.
- Nothing about whether a guard *wins* — holding is the bar.
- **No raid arrived during the leg.** Wendigo sightings were chatted
  (`npc-speech-tell_monster`, north) but nothing engaged the post, so this leg
  could not have scored axis 2 even with the pin in place. The next one must
  assert threat arrival as a precondition and VOID without it.

## Item 14 status

**3/4 bars → bar 1b's blocker is cleared and its precondition is met.** The bar
itself is still unscored, now for an ordinary reason (it needs a pinned
population and a threat that actually arrives) rather than because guard
assignments could not survive long enough to be measured.
