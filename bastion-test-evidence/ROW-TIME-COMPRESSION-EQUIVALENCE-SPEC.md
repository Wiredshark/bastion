# TIME-COMPRESSED ENDURANCE — **EQUIVALENCE VALIDATION SPEC**

**Ben directive 2026-08-11: promoted to parallel fill.** *"Let the current simulation
run, then build the fast simulation that maxes out our VMs."*

**Constraint, unchanged and absolute: NO SCORED GATE FLIES COMPRESSED UNTIL THIS
EQUIVALENCE PROOF PASSES REVIEW.**

---

## 0 · ★★★★★★ WHAT THE FINGERPRINT SEES — **coverage AND resolution, same section**

> ## **THE FINGERPRINT IS THE INSTRUMENT CERTIFYING THE MODE, SO ITS OWN RESOLUTION
> IS THE PROOF'S RESOLUTION.**

★★★ *A state fingerprint proves equivalence of **what it samples**. A wall-clock
dependency shifts **when** something fires — and a timing shift smaller than the
sampling interval is INVISIBLE.* **Then "identical fingerprints" is compatible with
real divergence, and every fast run afterwards inherits a certification that was
never tested.**

**REQUIRED, stated together and never separately:**

| axis | requirement |
|---|---|
| **COVERAGE** | the enumerated state included — *and, explicitly, what is EXCLUDED* |
| ★★★ **RESOLUTION** | **tick-indexed**, and the **smallest detectable divergence NAMED** *(target: one tick)* |
| **CADENCE** | how often it is taken; a coarser cadence is a weaker proof and must say so |

★ *Coverage and resolution are two axes. Naming only one is the usual failure.*

---

## 1 · ★★★★★ LIVE HOOK, NOT LOG-DERIVED — **decided, on today's own evidence**

**The tempting shortcut is to compute the fingerprint from a run's LOG, which would
let v5 serve as the real-time reference for free.** ★★★★ **Today proved that is a
trap.**

    b5_* counters ......... harness-only; NEVER in a live log        (proven, 0 hits)
    entity event log ...... in-memory ring; no output file           (proven, 0 hits)
    board state ........... not logged at all
    farm completions ...... emitted on a kind-specific line the generic grep misses

> ## **A LOG-DERIVED FINGERPRINT WOULD PROVE THE EQUIVALENCE OF A SHADOW — and its
> coverage gaps are exactly the ones that cost this project three wrong findings
> today.**

**DECISION: the fingerprint is a LIVE HOOK in the tick loop, reading state directly.**

### ★★★ CONSEQUENCE — **v5 CANNOT be the real-time reference**

*v5 is already built, reviewed and cleared; it has no fingerprint hook. Adding one
means rebuilding, which violates the packet's "one binary for the whole run."*

**So the real-time arm is a DEDICATED SHORT PAIR, not the arc-closing run.** ★★ *That
is cheaper anyway: equivalence needs a fingerprint match, not a long run — **a short
pair proves the same theorem for a fraction of the wall clock.***

★ *Answering the question the directive left open, and answering it against the
convenient option.*

---

## 2 · THE WALL-CLOCK-DEPENDENCY HUNT LIST

**The class:** *anything whose behaviour derives from real time rather than the `Time`
resource / tick count.* **Family exemplar: the live-game `tick_rng` falling to OS
entropy.**

| status | subsystem |
|---|---|
| ✅ **CLEAN, verified today** | claim-leak + sweep thresholds — `time.0` (sim clock) and cycles, *not* wall deltas |
| ✅ **CLEAN by construction** | the `tick % 300` heartbeat |
| 🔍 **HUNT** | `Instant::now()` · `SystemTime` · any `Duration` measured between real-time reads |
| 🔍 **HUNT** | IO / chunk-load timing, and anything that waits on it |
| 🔍 **HUNT** | RNG seeding paths *(the exemplar's family)* |
| 🔍 **HUNT** | client-facing rate limits, network timeouts, anything with a "per second" that is not per-tick |

★★ **The search space shrank today**: *the thresholds we most suspected turned out
sim-keyed.* **The hunt is for the inverse pattern — a wall-derived delta standing in
for the `Time` resource.**

---

## 3 · ⛔ **THE FINGERPRINT PROTOCOL IS RETIRED — REFUTED BY ITS OWN FIRST TRIAL**

**REVISED 2026-08-11, on 5b's A/B (binaries at `6eb22114`, gate-0 verified).**

### THE ORIGINAL, NOW VOID

    same seed · same scenario · same binary · one arm CAPPED, one UNCAPPED
    -> fingerprints compared TICK-FOR-TICK
    -> PASS requires bit-identity at every sampled tick

**Capped vs uncapped diverged 100%.** ★★★★★ **So did CAPPED vs CAPPED-CONTROL — a second
capped leg, nothing else changed.** *Promotion-completion tick **624** vs **192** at
identical pacing.*

> ## **TWO SAME-PACING LIVE PROCESSES DIVERGE EXACTLY AS CAPPED-VS-UNCAPPED DID. THE
> INSTRUMENT HAS NO NOISE FLOOR, SO IT CAN NEVER ANSWER THIS QUESTION.**

**Cause, read not guessed:** *colonist promotion `Simulated`→`Loaded` gates on
`chunk_states.0.get(chunk).is_some_and(|c| c.is_some())` — **two sites**,
`server/src/rtsim/tick.rs` `:858` and `:913` — i.e. on **real background chunk
generation**, bounded by wall time and thread scheduling, independent of pacing.*

### ⛔ AND THE PROPOSED HARNESS REDESIGN IS REFUSED — **it would certify vacuously**

**`BASTION_UNCAPPED_TPS` appears in EXACTLY ONE PLACE IN THE TREE:
`server-cli/src/main.rs:465`.** *`bastion-harness/src/main.rs` is a separate binary
and never reads it.*

★★★★★★ **Toggling it across two harness runs runs the same code twice — a guaranteed
PASS certifying compressed mode for the entire programme.** *The §3 vacuity trap,
arriving as a rescue plan.*

---

## 3b · ★★★★★★ THE EQUIVALENCE IS A **CODE PROOF**, AND WE ALREADY HAD IT

**DET-CLK-006: the server feeds the sim `Duration::from_secs_f64(1.0 / TPS)` — a
CONSTANT — never `clock.game_dt()`.**

> ## **SKIPPING `clock.tick()` REMOVES THE SLEEP AND NOTHING ELSE. THE SIMULATION'S
> PER-TICK INPUT IS UNCHANGED BY CONSTRUCTION.**

★★★ *No live A/B could add to this, and none could survive its own noise floor. The
empirical protocol was answering a question the code already closed.*

---

## 3c · ★★★★★★ THE REAL RESIDUAL — **RATE RELATIVE TO ASYNCHRONOUS WORK**

    capped        promotion complete @ tick  624
    capped-ctrl   promotion complete @ tick  192
    uncapped      promotion complete @ tick 2184

**Chunk generation is bounded by real seconds; the tick counter under compression is
not.** ★★★★★ **So colonists spend far longer in `Simulated` mode when compressed.**

> ## **THAT IS #85, THE UNTICKED COLONIST — ITEM 8's ENTIRE SUBJECT MATTER.
> COMPRESSION WOULD SYSTEMATICALLY AMPLIFY THE PHENOMENON UNDER STUDY.**

⚠ **AT ITS TRUE STRENGTH: n=1 uncapped against a capped spread of 192–624. A
CANDIDATE systematic effect, not an established one.**

### THE REPLACEMENT PROTOCOL

    N=8 per arm, capped vs uncapped, same seed/scenario/binary
    -> ARMS INTERLEAVED (capped, uncapped, capped, uncapped, ...)
    -> compare the DISTRIBUTION of promotion-completion tick
    -> PASS requires uncapped's distribution inside capped's spread

### ⛔ THE ARMS MUST BE INTERLEAVED — **added 2026-08-11, mid-fan**

**Promotion tick is bounded by REAL CPU TIME (background chunk generation). Any
concurrent load is therefore part of the measurement.**

> ## **RUNNING THE CAPPED ARM FIRST AND THE UNCAPPED ARM SECOND, WHILE A 2.5-HOUR
> ENDURANCE RUN SHARES THE BOX, IS A BETWEEN-ARM SYSTEMATIC — AND IT WOULD
> MASQUERADE AS THE EFFECT UNDER TEST.**

★★★★ *v5's own load drifts across its lifetime (v4's churn ramped, then saturated at
~71,800/sample). Interleaving makes that drift **common-mode** instead of confounded
with the axis.* **A matched control must match on SYSTEM as well as axis.**

★★ **Every leg records its background condition as a FIELD** *(`v5_concurrent`, wall
offset from the concurrent run's launch)* — **a number must carry its producer, and
"measured while an endurance run shared the box" is provenance, not a footnote.**

⚠ **OPEN, REGISTERED PRE-DATA:** *the first isolated capped legs read **233, 220**
against earlier standalone capped runs of **624** and **192**. Setups differ (driver
script vs isolated leg), so they may not be comparable — but a 192–624 spread
collapsing to 220–233 needs an explanation before the distribution is scored.*
**Discriminator: re-run two capped legs AFTER the concurrent run finishes.**

### ⛔ SAMPLE A IS NOT SCOREABLE — **registered PRE-DATA, and deliberately**

**Written when interleaving was planned to begin at leg 7.** ⚠ **CORRECTED ON THE
FACTS: the builder began interleaving at LEG 3, so only legs 1–2 are unmatched — a far
smaller exposure than this section was written against.**

*The reasoning stands as the general rule; the instance shrank.* ★★ **Recording the
correction rather than leaving the harsher version standing: an over-stated confound is
still a mis-stated fact.**

> ## **THE GENERAL RULE: A LATE-ARRIVING INTERLEAVE DOES NOT RETRO-MATCH THE LEGS THAT
> PRECEDED IT. Ns from before the correction belong to a different sample than Ns
> after it.**

★★★★★★ **DON'T MODEL THE CONFOUND — DELETE IT.** *A full interleaved N=8 after the
concurrent run costs ~15 minutes (legs are ~25 s + boot).*

| sample | what it answers |
|---|---|
| **A** — capped legs under concurrent load | **nothing on its own** |
| **B** — post-run interleaved N=8 | ★★★ **THE SCORED TEST** |
| **A vs B's capped arm** | ★★★★★ **the 220–233 question: cleaner instrument, or uniformly-loaded box?** |

★★★ **Three results from work already done, for ~15 minutes of new runtime.**

⚠ **WHY THIS IS WRITTEN BEFORE THE NUMBERS EXIST:** *eight completed legs create
pressure to score eight completed legs, and "we documented the ordering" will feel
like enough once the data is in front of us.* **Refusal #3 — no partial count as a
pass — applied to a sample rather than a cycle count.**

★★★ **Distributions, not fingerprints** — *bit-identity was never available on the live
path, and 5b's control is what proved it.*

**PLANTED FAILURE, still mandatory and now placeable:** *inject a delay into chunk-gen
and prove the distribution comparison goes RED by name.*

★★ **Cost: 8 runs × ~25 s. The compressed mode's own cheapness is what makes its
certification affordable.**

---

## 3c-RESULT · ★★★★★★ **THE CERTIFICATION RAN. COMPRESSION IS NOT EQUIVALENT.**

**Measured 2026-08-11, binaries `4d918025`, isolated legs, interleaved from leg 3.**

    capped   (n=8):  233 220 221 185 192 232 188 220     range  185– 233, mean  211
    uncapped (n=5): 1134 1347 1458 1177 1205             range 1134–1458, mean 1264

> ## **~6× SHIFT. ZERO OVERLAP. THE NEAREST POINTS ARE 233 AND 1134 — A 4.9× GAP.**

**The registered PASS condition was "uncapped's distribution inside capped's spread."
IT FAILS, and not marginally.**

★★★★★★ **The n=1 candidate is now n=8 vs n=5 with no overlap. Compression's effect on
the loaded/simulated ratio is CONFIRMED.** *v5's real-time ruling rests on measurement,
not suspicion — Ben's law satisfied through a **proven** wall-coupled subsystem, in the
strongest form available.*

### ★★★★★ THE ORDERING CONFOUND CANNOT ACCOUNT FOR IT — **the capped arm refutes it**

*Sample A remains unscoreable as registered. The result survives that ruling anyway:*

> ## **THE CAPPED ARM SPANS THE WHOLE WINDOW AND ITS SPREAD IS 185–233 (~26%). THAT
> SPREAD *IS* THE MEASUREMENT OF BACKGROUND-LOAD SENSITIVITY, AND IT IS SMALL. A 6×
> BETWEEN-ARM GAP IS NOT BACKGROUND LOAD.**

★★ *The earlier standalone **192** falls INSIDE the new capped range; the **624** is the
outlier. That favours "the isolated leg is a cleaner instrument" over "the box was
uniformly loaded" — the post-run clean set confirms.*

### ★★★★★★ THE FALSIFIER IS SATISFIED BY THE DATA — **plant stood down**

| axis — capped vs uncapped | ⛔ **RED: 6×, no overlap** |
|---|---|
| **control — capped vs capped** | ✅ **GREEN: 185–233, tight** |

**The comparison fires on the pacing axis and does NOT fire on same-pacing runs — both
polarities, from the natural experiment.**

★★★★★ **A planted failure exists to prove a GREEN result COULD have gone red. THIS
RESULT WENT RED: the instrument demonstrated its own sensitivity by firing.** *Planting
the chunk-gen delay would prove what the experiment just proved.* **Stood down, with
the reason recorded — [[a-falsifier-needs-its-own-control]] is satisfied, not skipped.**

---

## 3d · ★★★★★ CONSEQUENCE FOR v5 — **REAL TIME, AND THAT OBEYS BEN'S LAW**

**Checklist entry 6 admits three impossibilities. Number 2 is A PROVEN WALL-COUPLED
SUBSYSTEM — "and proven means proven; a suspicion of wall-coupling is not an
impossibility, it is an unread."**

> ## **IT IS NOW READ: measured across three legs, code-cited at two gates, and item 8
> is the row that studies promotion.**

★★★★ **v5 flies REAL TIME with its impossibility NAMED AND PROVEN — which satisfies the
law rather than excepting it.** *Everything not promotion-coupled still goes compressed
once §3c passes.*

★★★ **PLANTED FAILURE, mandatory:** *inject a deliberate wall-clock dependency and
prove the comparison goes RED by name.* **Otherwise a passing A/B is indistinguishable
from a comparison that cannot fail** — *the vacuity trap, and this proof is exactly
the artifact that must not have it.*

★★ **REVALIDATION TRIGGER (standing):** *the A/B re-runs whenever the shell changes —
tick loop, IO, scheduling, anything wall-adjacent.* **The equivalence is a claim with
an expiry date, and the trigger is what stops it decaying silently.**

---

## 4 · ★★★★ WHAT THE PROOF BUYS — **and the honest boundary**

> ## ⛔ **THIS SECTION WAS WRITTEN ASSUMING A PASS. THE CERTIFICATION FAILED. REWRITTEN
> 2026-08-11 AGAINST THE MEASUREMENT.**

★★★★★★ **A NEGATIVE CERTIFICATION IS THE MOST USEFUL KIND WE COULD HAVE GOTTEN:** *it
draws the compressed / real-time boundary in **measured ink** instead of suspicion, and
Ben's fast-mode law now runs on evidence at BOTH ends — fast for everything the
boundary permits, real for what it measures out.*

### ★★★★★ THE DECISION RULE — **is this run PROMOTION-COUPLED?**

**Compression shifts promotion by ~6×. So the question for any test is whether its
subject depends on WHEN colonists become `Loaded`.**

| the run… | mode |
|---|---|
| **observes colonist WORK** *(jobs, hauling, farming, needs, egress)* | ⛔ **REAL TIME** — *work requires `Loaded`; the ratio is the subject* |
| **spans founding / the promotion window** | ⛔ **REAL TIME** |
| **measures anything gated on `chunk_states`** | ⛔ **REAL TIME** |
| **runs flat-arena + `BASTION_DETERMINISTIC`** | ✅ *compression is MOOT there — `BASTION_UNCAPPED_TPS` is read only by `server-cli`, and that path force-loads and serialises anyway* |
| **human-in-the-loop** | ⛔ **REAL TIME** *(unchanged — pacing matters when someone watches)* |

★★★ **Default when unsure: REAL TIME, and name why.** *The burden moved: it is now
compression that must argue, because the coupling is measured and the exemption is not.*

### ⚠ THE FLEET CLAUSE — **RETRACTED AS WRITTEN**

**It promised: compression × the 8-VM pool ≈ 24 endurance-equivalents/hour, and
"ENDURANCE GRADUATES FROM SINGLE-RUN ANECDOTE TO CORPUS-GRADE EVIDENCE."**

> ## **ENDURANCE RUNS STUDY LOADED COLONISTS. THAT IS THE COUPLED CLASS. THE HEADLINE
> PROMISE DOES NOT SURVIVE ITS OWN CERTIFICATION.**

★★★★ **Stated plainly because a stale promise in a spec is the same failure shape as a
doc comment that foresaw its own defect: it keeps being read as true.**

**WHAT SURVIVES, HONESTLY:**

- **the corpus/harness fan is unaffected** — *it never went through `clock.tick()`; its
  speed was never compression's to give*
- **N-seed endurance evidence is still wanted and is still expensive.** ★★ *The
  affordability argument for repetitions must be re-made on other ground — parallel
  REAL-TIME runs across the VM pool, which the fan already supports.*
- ★★★ **8 real-time runs in parallel ≈ 8 endurance-equivalents per 2.5 h.** *Less than
  24/hour by an order of magnitude, and still the difference between an anecdote and a
  rate.*

★★★★★ **The limit v3 named — repetitions costing a day each — is eased by PARALLELISM,
not by compression.** *That was always the sounder half of the argument; compression
was the part that needed proving, and it didn't prove.*
