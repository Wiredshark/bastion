# THE THREE-WAY REACHABILITY TAXONOMY — ALREADY ON DISK, AND §4.2's THRESHOLD DOES NOT EXIST

**Computed from `wave25_BASELINE_e86fe79893_FULL.json`, 48 seeds, no run.**
Unit = **unique (seed, target)** probe result: **99** of them.

## §1 — ★★★★★★ THE DISTANCE THRESHOLD IS NOT JUST HARD TO PICK. IT IS THE WRONG VARIABLE.

§4.2 said: *derive an UNREACHED/UNREACHABLE threshold on `min_distance_to_target`
from the corpus distribution.* **The corpus answers: there is no such threshold.**

| | n | min | median | max |
|---|--:|--:|--:|--:|
| `step` path **FAILS** | 47 | **2.6** | **4.3** | 66.8 |
| `step` path **exists** | 40 | 2.9 | **6.5** | 53.8 |

> ★★★ **The ranges overlap almost completely, and the median runs BACKWARDS** —
> colonists with **no** step path got **closer** (4.3) than colonists **with**
> one (6.5). **Distance is not merely a weak predictor; it points the wrong
> way.** A threshold fitted to this would encode noise and read as a finding.

★ **A guessed threshold makes a classifier that reports its own assumption. A
FITTED one on this data would too.**

## §2 — ★★★★★★★ THE REAL CLASSIFIER IS ALREADY COMPUTED: THE MODE TRIPLE

`path_exists_{step, jump, scramble}` is a **categorical** answer already in the
corpus, and it yields **three** classes — better than the two the spec asked for:

| class | all | **with a real timeout** | meaning |
|---|--:|--:|---|
| **A. REACHABLE BY STEP** | 31 | ★ **22** | a step path EXISTS and the colonist still failed |
| **B. MODE-LIMITED** | 9 | **6** | **no step path — but jump/scramble EXISTS** |
| **C. UNREACHABLE** | 59 | 12 | no mode works; giving up is correct |

**Class A is the travel-bug population.** ★ *"A path existed, by the ordinary
locomotion mode, and the colonist timed out anyway."* **That is seed 7's class,
and it is 22 timeouts across 5 seeds — not one specimen.** Seeds: **52, 54, 61,
66, 71.**

**Class C is correct behaviour**, and it is the largest bucket — worth stating
plainly, because *"the rescue/travel system failed 92 times"* would have been a
gross mischaracterisation. **Most of those failures are honest.**

## §3 — ★★★★★★★★ CLASS B IS EXACTLY SEEDS 71 AND 90

> **The MODE-LIMITED class spans TWO seeds in the entire corpus: 71 and 90 — the
> campaign's two Row A specimens, and the pair already carrying a registered
> fork-marker.**

**Nothing was fitted to produce that.** The classes come from three booleans the
engine already computes; the seed list falls out. ★ **The fork-marker said
*"whatever fails 71/90 is a different mechanism."* This names the mechanism:
THE PATH REQUIRES A JUMP AND THE COLONIST ONLY STEPS.**

★ **Seed 71 appears in BOTH A and B** — it has targets of each kind, which is why
it has resisted a single explanation.

★ **Seed 90's own regression fits exactly:** stuck at `[17989,9263,338]`,
`path_exists_step: False`, `jump`/`scramble` `True`, route **exists** and never
**completes**, pinned at index 3–8 — **a route planned in a mode the colonist
cannot execute.**

## §4 — WHAT THIS DOES **NOT** CLAIM

- ★ **Not** that Class A is one bug. **22 timeouts across 5 seeds may be several
  mechanisms.** The class is defined by *what the probe says*, not by cause.
- ★ **Not** that the probe is ground truth. It is an **offline reachability
  model**; if its step-model disagrees with the live locomotion, **Class B could
  be a PROBE defect rather than a colonist defect.** ★ **That is the single most
  important thing to check before anyone builds a jump.**
- **Not** applicable to self-jobs. **The probe runs for MINE targets only** —
  seed 7's bed is outside this data entirely, which remains 5b's §4.1 gap.
- **Not** a 48-seed claim: 99 probe results are **not** 99 independent seeds.

## §5 — CONSEQUENCE FOR THE ROW

1. ★★★ **§4.2 as specced is DEAD** — no VM fan needed to derive a threshold that
   does not exist. **The fan is saved.**
2. ★★★ **Classification is a JOIN, not a measurement:** emit the mode triple
   beside each timeout. **The engine already computes both halves.**
3. **The row's acceptance criterion is unchanged and now over-satisfied** — it
   asked for *unreachable vs unreached*; the data supports **three** classes and
   the middle one is the interesting one nobody had named.
4. ★ **Next read, before any fix: validate the probe's step-model against live
   locomotion.** If Class B is real, the fix is a locomotion/pathing-mode
   question. **If the probe is wrong, Class B evaporates and Class A grows.**
