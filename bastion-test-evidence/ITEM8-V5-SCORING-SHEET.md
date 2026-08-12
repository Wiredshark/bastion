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
| **3** | **F7/F10 position concentration** | `grep 'bastion: job completed' \| sed 's/.*kind=\([A-Za-z()]*\) pos=\(Vec3 {[^}]*}\).*/\1 @ \2/' \| sort \| uniq -c \| sort -rn` | **143/145 at one cell (98.6%)** |
| **4** | **food curve + terminal streak** | `grep -o 'food_stock=[0-9]*' \| sort -t= -k2 -n \| uniq -c` | **peak 18; 517 consecutive zeros** |
| **5** | **farm generation counts** | `grep -c 'bastion: tilled\|bastion: sown\|bastion: harvested'` | **19 · 20 · 20** |

★★★ **Pattern 3 is the F7 bar itself** *(>10% at any single position = FAIL)*, **and it
is calibrated by a real specimen: v4 fails it at 98.6%.**

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
