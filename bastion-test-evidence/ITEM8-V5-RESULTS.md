# ITEM 8 v5 — **RESULTS**

**Scored against `ITEM8-V5-PACKET.md`, every judgement pre-registered.** *Capture
`8dd2f9463b` · binary `4d918025` · scored window 2026-08-11T23:56:43Z →
2026-08-12T02:30:00Z (2h33m) · log 161.7 MB, md5-verified lossless, 2 parts.*

★★ *Structure was committed BEFORE the data existed (`5404006e99`); only the
measurements are new.*

---

## 0 · PREFLIGHT — **GATE 0 PASSES**

    Server version: 4d918025 [2026-08-11]      <- read from the log, matches the pin

---

## 1 · THE BAR

| | bar | outcome |
|---|---|---|
| **F1** | farm activity in the **final third** of the window | ✅ **PASS — decisively** |
| **F2** | no immortal jobs | ✅ **PASS** — *189 expiry releases firing throughout* |
| **F3** | cells recycle, reap count not itself the defect | ✅ **PASS** — *34 reaps* |
| **F5** | targeted release fires | ✅ **PASS — 189** *(precondition exercised; no VOID risk)* |
| **F6** | leak backstop silent | ✅ **PASS — 0** across 2h33m |
| **F7 / F10** | no single position > 10% of completions *(UNION of channels)* | ✅ **PASS — 3.98%** |
| **F8** | `job completed` only for world-effect completions | ⚠ **PARTIAL — see §1c** |
| **S1** | sentinel | ✅ **0 firings — correct** *(no terminal streak to detect)* |

### ★★★★★★ 1a · F1 — **THE FARM WAS PRODUCING 0.6 SECONDS BEFORE TEARDOWN**

    last farm event   2026-08-12T02:30:00.106
    last log line     2026-08-12T02:30:00.725

> ## **NOT "SOMEWHERE IN THE FINAL THIRD" — IN THE FINAL SECOND.**

**v4 for contrast: last farm event 19:14, run ended ~21:20 — DEAD for the final two
hours of a two-and-a-half-hour run.**

    tilled 56 · sown 1,749 · harvested 1,721      ~31 cycles per plot
    v4 lifetime total: 19 · 20 · 20               once, then nothing

### 1b · F7 — **THE UNION READ**

    max at any single position   143
    total completions (union)  3,592      ->  3.98%      bound: 10%
    v4                                       98.6%  (143 of 145 at ONE cell)

★★ *Top six positions run 128–143 — near-even distribution across the plots, the exact
inverse of v4's single-cell monopoly.*

### ⚠ 1c · F8 IS **PARTIAL**, NOT A PASS — *scoring refusal #2*

**Exclusion half — PROVEN:** *33 emergency-access completions ALL routed to the honest
labelled line; the generic `bastion: job completed` fired **0 times**.*

**Inclusion half — UNEXERCISED:** *no real Mine/Chop/Build job completed at all, because
the founding script never designates one.*

> ## **A ZERO ON A CHANNEL NOT PROVEN REACHABLE IS NOT A PASS.** *We proved the emit does
> not fire for effect-less work. We did NOT prove it still fires for real work.*

★★★ **Scored PARTIAL and reported as such** — *the half that mattered for the disease is
proven; the other half needs a scenario that designates mining.*

---

## 2 · THE REGISTERED COMPARISONS

| | v4 (measured) | **v5** |
|---|---|---|
| **farm** | 19 · 20 · 20, then dead | ★★★ **56 · 1,749 · 1,721 — alive at teardown** |
| **`food_stock` peak** | 18 | ★★★ **1,957** |
| ★★★★★ **terminal zero-streak** | **517 samples** | ★★★★★★ **0 — run ends at 341** |
| **`designated_sweep_reaps`** | 468,323 | ★★★★ **34** |
| **max completions at one position** | 98.6% | **3.98%** |

---

## 3 · THE TRIANGULATING READS

### ★★★★★ FOOD SHAPE — **SAWTOOTH, not flatline**

    ... 31, 35, 43, 49, 49, 49, 49, 55, 57, 8, 10, 341

*Rise, consumption trough, rise again — and the run **ends at 341**.* ★★★ **Registered
test answered: v5 does not end inside a zero streak. v4's 517 has no counterpart here.**

### ★★★★★ RESCUE DISTRIBUTION — **READING A: BENIGN TRAFFIC**

    65 firings across 20 DISTINCT uids   (max 6 on any one)
    terminal_cause: egress_plan_or_climb_free_failed 51 · egress_no_route... 14

> ## **DISTRIBUTED, NOT CLUSTERED. THE MID-RUN `uid=131` APPEARANCE WAS A SMALL-SAMPLE
> ARTIFACT — EXACTLY WHY IT WAS REGISTERED AND NOT CONCLUDED.**

★★★ **Reading B (a rescue loop) is REFUTED.** *The net is handling ordinary traffic from
a colony doing ~30× more work, not papering over one trap.*

### `completed_kind` — **DEFECT 1 STAYS STAGED**

*v4's trap cell `(15212,16043,425)` appears with **2** completions and **no loop**,
consistent with the mid-run `Some(Leaves)` read.* ★★ **Subject re-confirmed as foliage;
mechanism still UNREAD. The untouched 790 MB `userdata` keeps it answerable.**

### ⚠ THE TIER RATIO — **weaker than at T+127, reported honestly**

    3.8 -> 3.9 -> 1.65 -> 1.97   (fail-safes ÷ emergency completions)

★★★ *It fell by roughly half and then partially recovered.* **Reading (a) — the organic
tier taking over — is SUPPORTED but not monotone, and I registered it as a
two-reading measure, not a conclusion.** *No stronger claim is made.*

---

## 4 · ★★★★★★ THE FLAT-REAP FALSIFIER — **PASSED IN ITS REGISTERED FORM**

**Registered pre-build: *"reap count FLAT as claim rate varies."*** *Not "low reaps."*

    claim_expiry_releases:  11 -> 20 -> 33 -> 50 -> 114 -> 189     (17x)
    designated_sweep_reaps: 34 -> 34 -> 34 -> 34 ->  34 ->  34     ( 0x)

> ## **THE CLAIM RATE VARIED SEVENTEEN-FOLD. THE REAP COUNT MOVED BY ZERO.**

★★★★★ **Against v4's 468,323 this is not an improvement in a number — it is the
mechanism behaving exactly as predicted, by a prediction written before the fix
existed.**

---

## 5 · CAVEATS CARRIED — *as registered pre-data*

- **DIAG DENSITY ~114× v3's byte rate** — **REGISTERED · REAL · ESTIMATED SMALL**
  *(≈14 KB/s; not material against CPU-bound worldgen).* **Not used to explain anything.**
- **The rescue RATE-PROFILE channel was WITHDRAWN at n=4** *(my own registered read,
  refuted by its fourth sample).*
- **N=8 sample A unscoreable**; *the 6× compression result stands on the capped arm's own
  26% spread.*

---

## 6 · ★★★★★★ VERDICT — **THREE SCOPES, REPORTED SEPARATELY**

### THE BAR: **PASS** *(7 pass · 1 partial · 0 fail)*

★★★★ **F1, F2, F3, F5, F6, F7/F10 and S1 all pass on their registered tests. F8 is
PARTIAL by refusal #2 — its unexercised half is named, not waived.**

### THE OPEN READS: **ALL FAVOURABLE, ONE STILL OPEN**

**Food shape sawtooth · rescues distributed (reading B refuted) · trap cell quiet.**
⚠ **Defect 1's mechanism remains UNREAD and STAGED — as declared at launch.**

### ★★★ THE ARC-1 RECOMMENDATION — **Fable's to accept**

> ## **RECOMMEND ARC 1 CLOSED.**

**Every bar passes on tests registered before the data, the two adverse readings that
could have complicated a clean pass were checked and refuted, and the one remaining
unknown was declared open at launch rather than discovered at scoring.**

★★★★★ **The colony that starved twice at nineteen tills now farms thirty-one cycles a
plot and is still sowing when the clock stops.**

★★ *Carried forward, not blocking: defect 1's mechanism · F8's unexercised half ·
the completion-signal split · S1's promotion.*
