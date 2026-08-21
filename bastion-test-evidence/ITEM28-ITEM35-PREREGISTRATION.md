# ITEMS 28 (tool wear) + 35 (injuries) — pre-registration, 2026-08-21

Both rows are marked BUILT in the arc index and **neither has a disposition
file**. This registers their bars before any leg runs. They share one leg
because the same arena exercises both: mining wears tools, and a planted wound
puts a colonist in a bed.

## Item 28 — the blocker had to be fixed first

Tool wear was unmeasurable: the code damages the equipped tool and **emits
nothing at all**. A row whose only effect is a silent field change cannot be
dispositioned from a log, and "I read the code and it looks right" is the
weakest evidence this project accepts. So the blocker became the row — a wear
witness is added, and the bars below are written against it.

**Bar 1 — wear HAPPENS.** A matching equipped tool loses durability on real
completions: the witness fires with a falling `durability_mult`.

**Bar 2 — wear PAYS.** `stats_durability_multiplier` is read at the progress
site, so a worn tool must work *slower*. The multiplier must be observed
strictly below 1.0. If it never leaves 1.0, wear is cosmetic and bar 2 FAILS
even with bar 1 green.

**Bar 3 — the CONTROL.** Wear is gated on the tool KIND matching the work
kind. A colonist doing work its equipped tool does not match must NOT wear it.
Without this, "everything wears always" would pass bar 1 and be a different
(broken) feature.

**Known limit, stated up front:** vanilla stats decay to a 25% floor and the
tool never breaks. Breakage would be a new rule and is banked, not tested.

## Item 35 — injuries

**Bar 1 — a wounded colonist in a bed produces a TEND job.** Witness:
`ITEM 35 tend job created — a wounded colonist is in a bed`.

**Bar 2 — tended rest is DISTINGUISHABLE from ordinary rest.** The sleep
completion already carries `tended` beside `health`, so a tended sleep and a
plain one cannot read alike. Bar 2 requires at least one `slept … tended=true`.

**Bar 3 — healing actually happens.** A colonist's health must be observed
RISING while in a bed. Already seen once in a play session (0.72 → 1.0), but
that was incidental; this asks for it under a planted wound with the value
printed beside the treatment.

## FALSIFIERS

- Item 28: witness never fires ⇒ the wear path is unreachable in practice, and
  BUILT is the wrong word for the row.
- Item 28: fires but `durability_mult` stays 1.0 ⇒ wear is recorded and does
  not pay; bar 2 fails and the row is half-built.
- Item 35: a wound exists, a bed exists, and no tend job appears ⇒ the
  generator's gate is wrong.
- Item 35: `tended=true` never appears ⇒ the multiplier has no live path and
  the 2.5× is a constant nothing reads.
