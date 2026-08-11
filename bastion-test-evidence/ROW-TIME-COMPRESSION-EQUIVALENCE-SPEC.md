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

## 3 · THE VALIDATION PROTOCOL

    same seed · same scenario · same binary · one arm CAPPED, one UNCAPPED
    -> fingerprints compared TICK-FOR-TICK
    -> PASS requires bit-identity at every sampled tick

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
