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

**Legs 1–6 ran capped-only in one early window; interleaving began at leg 7. The
resulting set would be:**

    capped:   6 legs EARLY  +  2 late
    uncapped: 8 legs        ALL late

> ## **THE CONFOUND SURVIVES THE INTERLEAVING — IT ONLY BECOMES HARDER TO SEE. 6-of-8
> vs 0-of-8 is an UNMATCHED pair with a caveat, not a matched pair.**

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

**Once passed, compressed becomes the DEFAULT for all unattended runs, including
certifying ones.** *Real time survives as exactly two things: the revalidation
trigger, and human-in-the-loop sessions (perceived pacing is not in the fingerprint;
real time matters exactly when someone is watching).*

★★★ **The certify-under-named-conditions law is not weakened — the named condition
becomes "the proven-equivalent execution", which is STRONGER, because the equivalence
is itself a certified claim carrying its own falsifier.**

### THE FLEET CLAUSE

**compression × the 8-VM pool ≈ 24 endurance-equivalents/hour.**

> ★★★★ **ENDURANCE GRADUATES FROM SINGLE-RUN ANECDOTE TO CORPUS-GRADE EVIDENCE** —
> *N-seed survival rates, which is what run-many-diagnose-aggregate has always
> demanded and what the RL substrate needs for rollouts.*

★★★ **AND IT RETIRES A LIMIT NAMED TODAY:** *the crash's intermittency (1 detonation
in 2 runs) made a fix's live validation need REPETITIONS, which cost a day each.*
**At 20 minutes a run, N=10 is affordable — the exact thing v3's finding said we
could not have.**
