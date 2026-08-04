# AUTON-2 prior-art dossier — DF needs/labor model vs our Drive arbiter — 2026-08-03

Read-only design input for the queued needs-as-drives row (AUTON-2, `readme/AUTONOMY-ARBITRATION-SPEC.md` §13).
Per-directive (Ben's prior-art-first rule): survey before design. Sourced entirely from in-repo research notes
already banked from the earlier DF-REF pass — no new external research, just consolidated and pointed at AUTON-2
specifically, since the original fold answered "does the arbiter's shape hold up" and this row needs "how do we
actually tune it."

---

## 1. How DF interrupts labor for needs — what our notes actually say

Two separate DF mechanisms are already folded into our spec, and they answer different questions:

**(a) The needs/focus model (DF-REF fold, `AUTONOMY-ARBITRATION-SPEC.md` §"DF-REF FOLD"):** a need is
`{ focus_level: satisfaction (decays 0→400), need_level: craving weight }`. Two independent numbers — *how well
fed the need currently is* (focus_level, a decaying satisfaction pool) times *how much that need matters to this
individual* (need_level, a per-need craving weight) — multiply together into the urgency curve. This is already
adopted into our §6 data model as the shape for B7's needs (`{focus_level, need_level}` → the non-linear Survive
spike is explicitly `need_level` high × `focus_level` low). **This is the THRESHOLD/CURVE half of the DF model —
already banked, not new for this row.**

**(b) The labor-preference model (`AGENT-SYSTEMS-RESEARCH.md` table row 68):** "per-dwarf labour prefs +
nearest-available-job scan; needs/mood/thoughts." This is DF's WORK selection, not its needs interruption — a
dwarf idle-scans for the nearest job matching its enabled labors. Our notes already map this onto our
`work_priorities` (per-colonist labor-enable/weight table) and — critically — flag DF's own scan as **the source
of a known failure mode we've already designed around** (see §2).

**What our notes DON'T have, and this row needs:** DF's actual **priority-BAND structure** between needs and
labor — i.e. not just "how is one need's urgency computed" but "how does DF decide a need-satisfying job
outranks whatever labor job a dwarf was about to pick, and by how much." Our spec's §5A step 5 (commitment/
anti-thrash) already answers this for OUR design (Survive/Flee = per-tick preemption, tier above Work; Work =
per-15-tick same-tier reselect) — this is a design decision already made, not one borrowed from DF. **Open
question for this row: is DF's own tier structure (documented as roughly need > mandate > standing-labor-order >
idle-labor-scan, though our notes don't have DF's exact internal priority list) worth cross-checking against, or
is our tier already sufficiently DF-validated by the "want vs can-do" split alone?**

---

## 2. Known failure modes DF evolved against (or suffers from) — per our own notes

**(a) "Everyone eats, no one farms" — THE one already named and gated (CHANGE C / registry G12).** Spec
§"REVISION" CHANGE C: in a colony-wide shortage deep enough that every colonist's Survive-urgency spikes
simultaneously, nobody stays below the spike to work the food generator's jobs, so the labor-feedback that's
supposed to self-correct the shortage never engages — recovery starves. This is explicitly named as the crux of
E1 (§8, "THE DEEP-SHORTAGE FAILURE"), and the mitigation is already designed: trait-modulation (E2) **staggers**
the spike (hardy/dutiful colonists spike later, so SOME keep farming), producing a **recoverable band** below
which the feedback works and past which the colony is meant to degrade gracefully rather than freeze. The gate
(§14c, and CHANGE C's own acceptance note) explicitly requires testing a shortage **deep enough to stress the
stagger**, not a shallow dip the feedback trivially survives.

**(b) DF's own swarm/job-stealing problem — named as a problem we're already fixing, not adopting.**
`AGENT-SYSTEMS-RESEARCH.md` row 68's own verdict column: "its swarm/job-stealing = the problem we fix." DF's
nearest-available-job scan, when many dwarves idle-scan the same board simultaneously, produces a classic
race/crowding failure — multiple dwarves converge on the same nearest job, or a fast dwarf's aggressive scanning
starves slower dwarves of nearby work. Our answer (already chosen, not open) is **stigmergy + response-threshold
coordination** (`AGENT-SYSTEMS-RESEARCH.md` row 21/38: a decaying saturation field + Bonabeau/Theraulaz
response-threshold division of labor — act when local stimulus exceeds a threshold, task completion lowers the
stimulus, auto-rebalancing which colonist takes which job without a central scheduler). This is a SEPARATE
subsystem from the need/Drive arbiter — it governs WHICH job a Work-drive colonist claims among several
candidates, not WHETHER the colonist is on Work vs Survive vs Flee. **Worth being explicit about in the AUTON-2
row: the arbiter answers "what drive," stigmergy answers "which job within Work" — don't conflate the two when
scoping the row.**

**(c) Thrash / oscillation — the generic utility-AI failure, not DF-specific.** Not named in our notes as a DF
failure specifically, but it's the standard risk of any per-tick utility-rescore (two near-equal-urgency options
flip-flopping). Already addressed in our design (§5A step 5's two-cadence commitment: 15-tick hysteresis for
same-tier reselection, per-tick only for cross-tier preemption) — listed as an OPEN reviewer question in §15
("is the 15-tick window the right anti-thrash granularity, or does a drive need its own min-commit?"). **Still
open; AUTON-2 inherits this question rather than resolving it.**

---

## 3. What maps onto our Drive arbiter vs what needs new machinery

**Maps cleanly (already designed, AUTON-2 just has to BUILD it, not invent it):**
- The `{focus_level, need_level}` needs schema → directly adoptable into `Need` (§6's `SelfNeed(Need)` drive
  variant) as-is.
- The non-linear urgency curve (spike near-critical, negligible while fed) → already specified as the shape
  Survive must take (§8).
- The trait-modulation stagger → already scoped to E2/AUTON-3, but AUTON-2's gate (§14c, "recoverable-band
  shortage auto-recovers") DEPENDS on it landing first or alongside — this is a sequencing dependency the row
  needs to state explicitly, not just a "nice to have."
- The tier structure (Survive/Flee per-tick preempt > Work per-15-tick reselect > Idle floor) → already specified
  in §5A, reusable verbatim for SelfNeed's tier placement (open question: does SelfNeed sit at Work's tier, or
  its own tier between Work and Survive? Spec's drive list order — Survive, Flee, Work, SelfNeed, Idle — implies
  SelfNeed is BELOW Work in urgency terms generally, i.e. a personal want, not a survival need, but this isn't
  stated as a tier RULE anywhere, only as list order.)

**Needs new machinery (not in the existing fold, AUTON-2-specific):**
- The actual per-need curve SHAPES and constants (decay rate, spike threshold, how many needs exist, which are
  Survive-tier vs SelfNeed-tier) — the DF-REF fold gives the SCHEMA, not the tuning. This is Ben-tuning work per
  §13 AUTON-3's own done-when ("the urgency curves are Ben-tuned"), not prior-art-derivable.
- The food-generator feedback loop itself (§8's "self-correcting labour" — job-urgency rising as food falls) —
  described at the level of intent, not implementation; AUTON-2 has to build the actual generator-urgency
  function.
- The `--autonomy-death-spiral-scenario` fixture (§14/CHANGE C) — doesn't exist yet; needs a planted shortage
  deep enough to stress the stagger, matching this session's own "planted fixture over organic repro" lesson
  (organic reproes are perishable — a scenario built to JUST BARELY trigger the recoverable band today could
  stop triggering it after an unrelated timing change, same failure class as seed 148's cave-in repro going
  stale this session).
- SelfNeed's actual want-satisfaction JOB shape (what does a colonist DO to satisfy a personal need — is it
  always a Haul/Goto-and-consume like Survive/eat, or does it need its own execution primitive per need type?)
  — CHANGE B already flags Survive/eat as net-new Bastion code (not reused from `npc_ai`); SelfNeed likely
  inherits the same "reuse movement, write behavior" pricing but isn't priced per-need yet.

---

## 4. Open design questions for the row (stated as questions, not answers)

1. **Tier placement for SelfNeed relative to Work.** Is a personal need (e.g. social, a DF-FOCUS "wants to see
   the sky" style want) ever urgent enough to preempt an in-progress Work job per-tick, or does it only compete
   at the per-15-tick same-tier reselect? The spec's drive ordering suggests the latter but never states it as a
   rule.
2. **Does AUTON-2 depend on AUTON-3 (trait stagger) landing first, or can the recoverable-band gate be tested
   with a STUB stagger (e.g. a fixed per-colonist offset) and the real trait-driven stagger backfilled later?**
   The gate's acceptance (§14c) needs SOME stagger to produce a recoverable band at all — worth deciding whether
   AUTON-2 blocks on AUTON-3 or can fake it forward.
3. **How many distinct SelfNeed types land in the first cut?** DF has many (a dozen-plus personal needs); our
   spec never commits to a count. A single generic SelfNeed with per-facet-weighted sub-needs, or N independent
   Drive-eligible needs from day one?
4. **Does the food-generator feedback need its OWN response-threshold/stigmergy treatment (§2b's coordination
   layer), or is the arbiter's per-colonist urgency scoring alone sufficient to avoid a "everyone rushes the
   same farm tile" version of DF's swarm problem once labor shifts toward food?** The two subsystems (arbiter
   tier-selection vs stigmergy job-selection) are designed to compose, but AUTON-2 is the first row where a
   DEMAND SPIKE (many colonists switching to Work=farm simultaneously) actually stress-tests that composition —
   worth flagging as a specific thing to watch in the gate, not assumed safe by construction.
5. **Should the planted `--autonomy-death-spiral-scenario` be designed DETERMINISTIC-by-construction from the
   start** (matching this session's own lesson: a hand-placed shortage depth/timing that can't organically drift
   out from under unrelated future changes), **rather than tuned against current behavior and re-validated
   later?** Recommend yes, explicitly, given the seed-148 cave-in precedent from this same session (a
   "permanent exact repro" that stopped reproducing after two unrelated scheduling-perturbing commits).

---

## Sources consulted (in-repo only, no external research this pass)

- `readme/AUTONOMY-ARBITRATION-SPEC.md` — the DF-REF fold, §5A/§6/§7/§8/§13-AUTON-2/§14/§15.
- `readme/AGENT-SYSTEMS-RESEARCH.md` — coordination-layer survey (rows 21/38/68), verdict table.
- `readme/AGENT-CULTURE-CHARACTERIZATION-design.md` — checked for DF-FOCUS/needs detail; contributes only the
  B-AG3 facet/DF-FOCUS pointer already covered above, no additional labor/threshold specifics.
