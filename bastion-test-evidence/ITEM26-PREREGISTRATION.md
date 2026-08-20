# Item 26 (Crafting chains) — PRE-REGISTRATION

**Substrate:** item 27's cook pipeline IS the first chain instance
(raw→station→product with fetch, reservation, completion, drop). Item 26
generalizes it: the transform becomes DATA, not a second hand-built loop.

## Build shape

1. **A recipe table (RON, the thought-table pattern)**: station kind →
   (inputs: Vec<(def, n)>, output: (def, n), work_secs). The cook arm's
   hardcoded mushroom→curry becomes the first row of the table.
2. **The generator and completion read the table** — one generic
   material-job path (the cook generator + completion generalized over
   recipe rows; sow/build keep their own arms).
3. **A SECOND chain proves generality**: wheat → flour → bread needs a
   Mill station kind (vocabulary tail-append) OR wood → planks at a saw.
   v1 picks ONE second recipe; the bar is that ZERO new Rust is needed
   beyond the vocabulary (the table row does the work).
4. Witnesses: per-completion input-consumed/output-produced counts (the
   conservation pair item 27 already prints).

## BARS

1. The cook chain still passes end-to-end AFTER the table migration
   (regression bar — same witnesses, same counts).
2. The second recipe completes live with its own conservation pair,
   added as DATA (the commit diff proves the no-new-Rust claim).
3. A missing-input chain REFUSES at the claim gate with the materials
   witness (the item-27 instrument reused).
4. Twin determinism.

VOID branches: item 27's own pipeline still red (this row builds ON its
green — sequence, not parallel); the second station's build materials
starve (the reservation-wall lesson: check the claim-gate witnesses first).
