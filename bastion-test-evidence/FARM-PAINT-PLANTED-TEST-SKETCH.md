# FARM-PAINT planted-test sketch (read-only prep, ahead of Opus's spec)

**Status: SKETCH ONLY.** No code written. This exists so the planted-test
half of the acceptance framework is ready the moment Opus's flesh-out
lands, rather than starting cold. Everything here is provisional on his
actual design — expect to redraw once his spec arrives, especially the
exact shape of "surface-relative resolution" for Farm.

## What FARM-PAINT is fixing (from this session's own live-observed defect)

Farm's paint path (`bastion_place_designation`, `DesignationKind::Farm`,
`z_extent: None`) takes `plot.min.z` **literally**, with **zero
tolerance**. The trigger pass (`bastion_jobs.rs` ~9028-9041) requires
`ground.is_filled()` at exactly that z for every column; one block of
error and the whole plot silently produces zero jobs, forever, with no
message. I hit this myself, live, this session (`RUN1-SCORECARD.md`'s
farm row) — a real player mistake, not a contrived one, and the failure
mode is indistinguishable from "the game is doing nothing because there's
nothing to do yet."

Per Fable's framing (2026-08-08): the fix is **surface-relative
resolution** (matching the `z_extent: Some(_)` path other designations
already have — resolve each column's real surface server-side rather than
trusting the client's guessed z) **plus refusal-with-message** — if
resolution genuinely can't find a valid surface for some or all of the
painted footprint, say so, never just silently generate zero jobs.

## The planted failure (per the acceptance-framework law: every row needs one)

**Plant:** paint a Farm designation whose `min.z` is deliberately wrong —
some blocks above (or below) the real ground — the exact mistake I made
live. Use a footprint straddling BOTH a correct-height sub-area and a
wrong-height sub-area, so the test can distinguish "the whole plot failed"
from "only the mis-specified columns failed."

**Two failure-mode branches, both need a planted case (mirrors Opus's own
"could a player do this by accident" trap criterion from the live
playthrough campaign):**

1. **The whole plot is wrong-height.** Every column mis-specified.
   - Pre-fix (today): zero jobs, silence, `farm_paint_jobs_zero` reads as
     if nothing was painted rather than "painted, but couldn't be
     resolved."
   - Post-fix, if surface-relative resolution finds the real ground on
     its own: the plot generates TILL jobs at the CORRECTED height, and
     the test asserts the corrected `z` disagrees with the painted `z` —
     proving the resolution actually ran, not just accidentally matched.
   - Post-fix, if resolution genuinely can't determine a surface for some
     reason (e.g., painted underground/underwater, no valid ground within
     a reasonable search range): a `ChatType::CommandError`-class refusal
     fires, naming the cause, same pattern as the existing
     `BastionPlaceDesignation` accept/reject branch
     (`server/src/sys/msg/in_game.rs:575-593`). Test asserts the message
     text, not just that *a* message fired — the same "assert types, never
     guard them away" discipline the runbook already holds everyone to.

2. **Mixed footprint, half correct-height, half wrong.** Tests whether
   resolution is genuinely per-column (like `z_extent`'s existing
   per-column surface resolution for other kinds) or an all-or-nothing
   plot-level guess.
   - Correct-height columns should till/sow/mature/harvest exactly as the
     current PASSING `farm_scenario` already proves they can (seed 42,
     this session).
   - Wrong-height columns should EITHER resolve individually (best
     outcome — no player-visible seam) OR refuse individually with a
     per-cell message (acceptable — matches `blocked_regions`'s existing
     per-cell-cause precedent, task #55's lineage) — but must NOT silently
     drop those columns with zero signal, which is the exact defect being
     fixed.

## Success measures (draft, pending Opus's actual acceptance bar)

- A farm plot painted with the live-observed mistake (1-3 blocks off real
  ground) either self-corrects (preferred) or refuses with a specific,
  truthful message — **never silently produces zero jobs again.**
- The existing `farm_scenario`'s clean-height path (seed 42, currently
  PASS) must stay PASS — this is a tolerance widening, not a behavior
  change for correctly-painted plots. Byte-identical when the painted
  height is already correct, matching the "absent means the exact
  pre-existing behavior" convention this whole batch has held to.
- The corner-cell false-negative (`CORNER-CELL-AND-SEED92-COLUMN-SCANS.md`)
  is a SEPARATE, plan_access-owned defect, not this row's to fix or to
  regress-test against — noting so scope doesn't creep between the two
  open rows.

## Open questions for Opus's spec, not guessed at here

- How far does surface-relative resolution search vertically before
  giving up and refusing? (Mine/other kinds' own `z_extent` precedent is
  the natural reference point, not reinvented here.)
- Does resolution snap to the topmost solid surface unconditionally, or
  does it need a flatness/consistency check across the footprint first
  (a plot straddling a slope is a different question than a plot painted
  at a flat but wrong height)?
- Where does the refusal message live — same `ChatType::CommandError`
  channel as `BastionSpawnColony`'s reject path, or the
  `blocked_regions`/task-#55 chat-meta channel Farm's own trigger pass
  already touches indirectly via `plan_access`? (Different code paths,
  different precedent each.)

Redrawing this the moment his spec lands rather than building against a
guess.
