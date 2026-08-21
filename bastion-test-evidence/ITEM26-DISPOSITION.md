# Item 26 (Crafting chains) — DISPOSITION: bars 1–2 **PASS** (chain18 cookdiag)

**The build:** `assets/common/bastion_recipes.ron` (the thought-table
pattern) + `bastion_recipes.rs` (one loader; defs leaked once to the B6
`&'static` contract). The generator's raw gate, the idle witness, and the
completion's product all read THE SAME ROW — a chain cannot half-exist.
The no-recipe completion refuses loudly (couldn't-happen witness, 0 fired).

**Bar 1 (the cook chain survives the migration): PASS, exceeded.**
47 dishes, 0 stalls, 47:47 XP — versus 32 dishes on the pre-table leg of
the same shape. Same witnesses, now carrying `product=`/`n=` from the row.

**Bar 2 (a second chain as PURE DATA): PASS.** The wheat→curry recipe is
one RON entry — zero new Rust, demonstrated in the build commit itself
(ea7cbc2f77's diff) — and it produced **42 of the 47 dishes**: the farm's
whole wheat output became cook input the moment the row existed.
Multi-recipe-per-station dispatch (first-with-available-input) proven live.

**Bar 3 (missing-input refusal): covered by the existing claim-gate
materials machinery** — the same witnesses items 27's chain exercised for
tens of thousands of counted refusals; nothing recipe-specific to add.

**Bar 4 (twin determinism): rides the standing twin queue.**

Design door now open (banked, not built): a Sawmill/Mill station is one
vocabulary append + one RON row; a materials-class store (item 30's
selector is already class-generic) pairs with it.
