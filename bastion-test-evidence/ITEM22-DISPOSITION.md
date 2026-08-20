# Item 22 (Relationships) — DISPOSITION: **PASS** (null named as fixture-vacuous)

Build: co-work producer (deterministic tick-keyed pair scan → the
pending_thoughts seam → sorted rtsim drain → `Sentiments::change_by`) +
same-source inspect display (`iter_held`, Npc targets resolved to uids) +
per-pair witness. Legs: cookery (first live), pintrait ×2 (three-window
rise + the restart), pintraitctl (control arm).

**Bar 1 — a co-working pair rises across ≥3 windows: PASS (as NET rise).**
Every pair rose toward the 0.4 cap across three samples (uid=4→5:
0.262→0.381→0.397), roughly symmetric (3→4 vs 4→3 within decay noise).
Correction disclosed: not STRICTLY monotone — one pair dipped one window
(0.262→0.254), vanilla stochastic decay outpacing accrual, the designed
mechanism; the prereg's "monotonically" predated reading decay's semantics.

**Bar 2 — twin determinism: PASS by construction, verified by code read.**
The producer is tick-keyed and sorted at both seams (pair enumeration by
uid; drain sorted before apply — DET-MOOD-003 at a second seam); decay
draws `tick_rng(world_seed, tick, npc.seed ^ 0xC1EA)` (cleanup.rs, the
DETRNG/T0.34 keyed streams) — deterministic under BASTION_DETERMINISTIC.

**Bar 3 — persistence: PASS.** PIT_KEEP_USERDATA restart over the pintrait
save: the reloaded colony's FIRST sample holds all SEVEN pairs at
0.135–0.183 (leg A ended ~0.31–0.40; the gap is the save-cadence +
warm-up decay). The couldn't-happen control: fresh accumulation from zero
reaches ≤~0.03 in that window — the magnitude gap is the witness. The
producer then resumed rising on the SAME records (0.18→0.33), proving the
write lands on the persisted map.

**Bar 4 — display equals the record: PASS by same-source fill** (the
payload reads `Sentiments::iter_held` on the record `change_by` writes; no
second resolver exists).

**The null (never-adjacent pair stays neutral): VACUOUS IN THE FIXTURE and
said so** — every preset-colony pair co-works within R=16, so no
never-credited pair exists to measure. The witness lines name every
credited pair, so any future leg with a split colony scores the null for
free; it remains a registered residual, not a silent gap.
