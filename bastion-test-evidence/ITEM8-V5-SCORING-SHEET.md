# ITEM 8 v5 — **SCORING COMMAND SHEET**

**Written BEFORE teardown so scoring is mechanical, not exploratory.** *Every pattern
below is marked **VALIDATED** (run against v4's capture, where ground truth is known)
or **UNVALIDATED** (targets a field v4 does not have).*

★★★★★ **Tonight cost three findings to guessed patterns. A grep pattern is a claim
about naming, and these claims get checked before the log that matters arrives.**

---

## 0 · THE UNIVERSAL PREFIX — **never omit**

    sed 's/\x1b\[[0-9;]*m//g'

⚠ **`kind=` MATCHES NOTHING WITHOUT THIS.** *ANSI escapes sit between key and `=`.*
★★ **And confirm the path resolves before reading any zero** — *the v4 capture lives
under `bastion-test-evidence/live-playthrough/…`, not `bastion-test-evidence/…`.*

---

## 1 · VALIDATED PATTERNS — *exercised against v4, known answers*

| # | measure | command core | v4's answer |
|---|---|---|---|
| **1** | **gate 0** | `grep 'Server version:'` | `b96830d1` |
| **2** | **message vocabulary** | `grep -o 'bastion: [a-z_ ]*' \| sort \| uniq -c \| sort -rn` | **22 distinct** |
| **3** | ⛔ **F7/F10 — SEE THE CORRECTION BELOW** | ~~`grep 'bastion: job completed' …`~~ | *v4: 143/145 at one cell (98.6%)* |
| **4** | **food curve + terminal streak** | `grep -o 'food_stock=[0-9]*' \| sort -t= -k2 -n \| uniq -c` | **peak 18; 517 consecutive zeros** |
| **5** | **farm generation counts** | `grep -c 'bastion: tilled\|bastion: sown\|bastion: harvested'` | **19 · 20 · 20** |

★★★ **Pattern 3 is the F7 bar itself** *(>10% at any single position = FAIL)*, **and it
is calibrated by a real specimen: v4 fails it at 98.6%.**

---

### 🛑 CORRECTION — **PATTERN 3 IS WRONG FOR v5, CAUGHT BEFORE TEARDOWN**

**`grep -c 'bastion: job completed'` on v5 returns ZERO.** *F8's fix moved
emergency-access completions onto their own labelled line, and farm completions were
always on their own (`tilled`/`sown`/`harvested`).*

> ## **THE GENERIC LINE NO LONGER CARRIES THE COMPLETION POPULATION. SCORING F7 OFF IT
> WOULD DIVIDE BY A DENOMINATOR OF ZERO — OR WORSE, SCORE 100% OF A THREE-ROW SAMPLE.**

★★★★★★ **THIS IS THE SINGLE-CHANNEL COMPLETION COUNT ERROR — THE SAME ONE THAT
PRODUCED THE 0/87 MISCOUNT EARLIER IN THIS ARC.** *I made it then; my pre-written
pattern would have made it again at teardown.* **The sheet earned its keep before the
run ended.**

**F7/F10 IS SCORED OVER THE UNION OF ALL THREE COMPLETION CHANNELS:**

    1.  farm arm     'bastion: tilled' | 'bastion: sown' | 'bastion: harvested'
    2.  emergency    'bastion: emergency access job completed'
    3.  generic      'bastion: job completed'      (real Mine/Chop/Build)

★★★ **Then group by `pos` across the union, and apply the >10% bar to that total.**
⚠ **A new emit channel changes every count's DENOMINATOR** — *and F8 created one
DURING the arc it is scored in.*

---

## 2 · ⚠ UNVALIDATED — **fields that DO NOT EXIST in v4**

**These target emits landed only in v5's build. I cannot check them against ground
truth, so I have REQUESTED ONE SAMPLE LINE OF EACH rather than guess the field
names.**

| # | measure | needs |
|---|---|---|
| **6** | **`completed_kind`** *(defect 1's deciding read)* | *the completion line's new field — exact name and placement* |
| **7** | **`kind` on arrival** | *the arrival line's new field* |
| **8** | **the material-stall emit** | *its message text + `required_item` field* |
| **9** | **F8 — the honest emergency-completion line** | *its distinct message text* |
| **10** | **rescue uid distribution** | *`ULTIMATE FAIL-SAFE` line — confirm `uid=` is on it* |
| **11** | **`watch_wipe` reason trace** | *`BASTION_EGRESS_DIAG` output shape* |

> ## **AN UNVALIDATED PATTERN THAT RETURNS ZERO IS NOT A RESULT. IT IS AN UNREAD.**

★★★★ *Every one of these will be validated against a real sample line before it is used
to score anything.*

---

## 2b · ★★★★★★ F1's EXACT TEST — **operationalised BEFORE the data, not at 22:30**

**F1 reads "generation-2 completions > 0 — sows continuing past the first seed wave."
That sentence needs one unambiguous query, or it becomes negotiable at teardown.**

> ## **THE TEST: DOES ANY `sown` EVENT CARRY A TIMESTAMP LATER THAN THE FIRST
> `harvested` EVENT?**

★★★★★ **A sow that follows a harvest is second-generation BY DEFINITION** — *it can
only be planting into a cell the first wave already returned.* **No inference about
plot counts, no ratio to argue over, one comparison of two timestamps.**

    first_harvest = earliest 'bastion: harvested' timestamp
    gen2_sows     = count of 'bastion: sown' with timestamp > first_harvest
    F1 PASSES iff gen2_sows > 0

### WHY NOT THE RATIO

*`sown` (563) vs `tilled` (56) ≈ 10 cycles per plot is a **correct and compelling
argument** — the builder's reasoning is sound and source-backed.* ⚠ **But it is an
INFERENCE about plot reuse, and F1 should be settled by an OBSERVATION.**

★★★ **Report the ratio as the magnitude. Score the bar on the timestamp.** *The ratio
says how well; the timestamp says whether.*

---

## 2c · ⚠ THE DIAG-DENSITY CONFOUND — **registered before scoring, not after**

**v5 runs with strictly more instrumentation than v4:** `BASTION_EGRESS_DIAG=1` *(the
wipe-reason trace, across ~a dozen wipe sites)*, `BASTION_ENTITY_EVENT_LOG=1`, plus
`completed_kind`, `kind`-on-arrival and the stall emit.

★★★★★ **[[the-instrument-changes-what-it-sees]] — diag density is budgeted, and two
extra diag reads once broke bit-reproducibility on this project.**

> ## **HEAVIER LOG IO COMPETES WITH BACKGROUND CHUNK GENERATION — THE EXACT MECHANISM
> THE N=8 TEST JUST MEASURED AT 6×.**

### ⚠ ONE OF MY OWN COMPARISONS IS AFFECTED

*I compared v5's **39 rescues** against v3's **4 in 75 min** as though the runs were
alike.* **They are not: heavier logging → slower ticks → colonists plausibly stick
more.** ★★★ *That does not explain a 7× gap by itself, but it is an uncontrolled
variable, and it is named BEFORE scoring rather than discovered in review.*

| ✅ **UNAFFECTED — score freely** | *counts and ratios*: farm cycles, reaps, F5/F6, completions-by-position, the F1 timestamp test |
|---|---|
| ⚠ **CONFOUNDED — caveat required** | *anything WALL-CLOCK*: rescue rate, promotion timing, any cross-run "per minute" figure |

★★ **The bars are all in the unaffected column.** *This constrains the MEASURES and the
cross-run colour, not the verdict.*

### ★★★★★★ MEASURED — **and the absolute refutes my own alarm**

**Baseline corrected by the builder, and correctly: v4's byte total is contaminated by
the churn bug (468,593 sweep-spam lines — a DEFECT artifact, not a diagnostic-density
fact). v3 is the clean comparator.**

    v3:  1,123,528 B / 154 min  ≈    7,296 B/min
    v5: 81,571,861 B /  98 min  ≈  832,366 B/min      ->  ~114x

> ## ⚠ **BUT 114× A VERY SMALL NUMBER IS STILL A SMALL NUMBER: ~14 KB/s, ~34 LINES/s.**

★★★★★ **I raised this alarm on a RATIO. The absolute magnitude refutes its severity.**
*34 lines/second of buffered stdout will not meaningfully compete with worldgen, which
is CPU-bound, not IO-bound.*

> ## **A RATIO WITHOUT ITS BASE IS A CLAIM ABOUT NOTHING** — *the same disease as a
> count without its denominator, one layer up.*

**REVISED POSITION, and it is the honest one:** *the confound is **REGISTERED, REAL, AND
ESTIMATED SMALL** — not withdrawn (the mechanism exists and the ratio is genuine), not
alarming (the absolute makes a material effect implausible).* ★★★ **Wall-clock
cross-run figures still carry the note; nobody should treat it as explaining a 7× rescue
gap.**

★★ *Disk: 55 GB free; this log needs ~50–120 MB more. **Negligible.** The drive being
95% full generally is a real but separate standing item.*

---

## 3 · THE THREE TRIANGULATING READS

| read | pattern | decides |
|---|---|---|
| **food SHAPE** | **#4** *(validated)* | *sawtooth vs terminal, against v4's 517* |
| **rescue DISTRIBUTION** | **#10** *(unvalidated)* | *benign traffic vs a rescue loop* |
| **`completed_kind`** | **#6** *(unvalidated)* | *did defect 1 recur?* |

★★★ **Any two agreeing constrains the third.** *Cluster on one uid + constant
`completed_kind` at that position = the same finding down two channels.*

---

## 4 · ORDER OF OPERATIONS AT TEARDOWN

1. **Gate 0 FIRST** — *a stale binary voids everything downstream; v3 died here.*
2. **Vocabulary enumeration** — *establishes what the instrument can see, and validates
   patterns 6–11 in one pass.*
3. **The bar** — *F1, F2, F3, F5, F6, F7, F8, S1.*
4. **The three triangulating reads.**
5. **The measures** *(F9, fail-safe count, emergency completions).*

★★ **Refusals in force:** *no zero as a result until path AND pattern are verified · no
partial count as a pass · no promoting the elegant story · no cross-carrying between
co-resident results.*
