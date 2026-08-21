# Item 27 (Cooking) — DISPOSITION: bars 1–2 **PASS** (chain16 cookdiag)

**The numbers:** 32 dishes produced, 0 stalls, 0 convergent-retry
firings, 32 Cook XP grants (1:1 with dishes — the conservation pair's
free cross-check), across 9 stations, in one leg.

**Bar 1 (the pipeline, end to end): PASS.** Paint → per-cell station
build → registration → generator (raw-gated, idle witness carrying
stocked/reserved) → claim with fetch reservation → fetch leg → arrival
consume (ONE raw per dish, at threshold) → completion → dish dropped at
the station → the dish is EDIBLE (FOOD_DEFS) with a cooked premium
(`food_restore_for`), and the pot cannot cook its own output (RAW_DEFS
split).

**Bar 2 (conservation): PASS.** One raw consumed per dish by
construction (the arrival consume is the single take; the completion
produces only — the double-consume was found and removed), witnessed
per event, 32:32 against XP.

**The five roots this row burned through, each measured before fixed:**
1. The self-job reservation leak (34,813 RESERVATION-ONLY refusals; 27×
   reduction verified).
2. Entity-level reservations vs merged piles (one eater locked a 64-unit
   larder; the gate went unit-aware — 31,910 → 0 refusals).
3. The orphaned-rid cache leak in the vanish branch (census-caught,
   routed through the one releaser).
4. The eat path consuming in-flight job ingredients (inventory shortcut
   now skips protected defs).
5. **THE ROOT: `completion_block == None ⇒ continue` skipped the entire
   completion tail for non-terrain kinds** — the threshold consume
   re-fired per tick (the measured 1-per-tick drain: 14 held 151 ticks,
   then −1/tick) while the completion arms sat dead below the continue.

**Bars 3–4 (the premium visible in outcomes) — PARTIAL, follow-on:** the
dish is eaten (eat events stream in-leg) but the eat witness does not yet
carry the def/restore, so premium-vs-raw is not yet a printed number; a
def-carrying eat witness + one A/B leg closes it. Station granularity
(3×3 = 9 stations) is banked as a design ruling with a recommendation.
