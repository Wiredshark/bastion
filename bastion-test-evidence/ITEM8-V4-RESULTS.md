# ITEM 8 v4 — **RESULTS**

**Scored against `ITEM8-V4-PACKET.md` (`989caddfa1`/`817ac1da2e`), every judgement
pre-registered.** Capture `1bedd79602` · log split in 3 parts, ~945K lines.

---

## 0 · ★★★ PREFLIGHT — **GATE 0 PASSES. FIRST TIME IN THE ARC.**

    Server version: b96830d1 [2026-08-11]         <- matches the approved commit
    bastion effective ITEM8-V4 config ... generic_claim_leak_secs=1860.0
                                          colony_terminal_zero_streak_samples=10

★★ **Read from the log, not from a report.** *This is the check whose absence voided
v3 — entry 5 of the checklist, working on its first outing.*

---

## 1 · ★★★★★★ F1 — **THE DAMNING NUMBER**

    v4 farm completions:  19 tilled · 20 sown · 20 harvested  =  59
    v3 farm completions:  19 tilled · 20 sown · 20 harvested  =  59

> ## **IDENTICAL. THE FARM DID EXACTLY AS MUCH WORK AS BEFORE THE FIX, AND STOPPED
> AT THE SAME POINT.**

★★★★ **F1 FAILS, and in the most informative way available.** *Not "less" — **precisely
the same**, which says route 2 (claim expiry) never engaged the mechanism that stops
farming.* **A fix that changes a number is wrong about magnitude; a fix that changes
nothing is wrong about the mechanism.**

---

## 2 · THE CHURN — **CONFIRMED, with my framing CORRECTED**

    farm job created ....... 468,672
    designated_sweep_reaps.. 468,323        <- within 349 of creations
    job claimed ............     536        <- destroyed ~874x more often than claimed

★★★ **Registered discriminating read (a) CONFIRMED: near-equal and enormous** —
*create→reap cycling, exactly as predicted from code before the capture existed.*

### ⚠ MY "IGNITION" FRAMING WAS WRONG — the profile is RAMP-THEN-SATURATE

    per-sample deltas: 10,126 · 62,443 · 71,758 · 72,000 · 71,636 · 71,881 · 71,879

> **It ramped for two intervals and then went DEAD FLAT at ~71,800. A CEILING —
> every eligible cell reaped every pass — not unbounded runaway.**

★★ *I predicted acceleration from two coarse points and got acceleration; the profile
shows it terminates in saturation.* **Corrected rather than quietly restated.**

### ⚠ THE FAMINE-COUPLING READ IS **INCONCLUSIVE**, NOT CONFIRMED

**Registered prediction: reap rate rises as claim rate falls, inversely.**

★★★ **Not cleanly testable here — claims collapsed at minute 3, so the claim rate had
almost no dynamic range to correlate against.** *536 claims across the whole run.*
**Recorded as inconclusive; the coupling story remains unproven, and v5's flat-reap
falsifier is the better test regardless.**

---

## 3 · THE COLONY — ★★★ **A REGRESSION AGAINST v3**

    ate ......... 11     (v3: 40)
    slept ........ 0     (v3: 48)        <- ZERO sleeps in 2.5 hours
    BREAKDOWN .. 897     (v3: 331)       distinct 8 of 8

★★★★ **The churn did not merely fail to help — it consumed the colony's capacity to
do anything else.** *v4 is worse on every colony measure than the run whose famine it
was built to fix.*

---

## 4 · THE BAR

| | outcome |
|---|---|
| **F1** farm completions improve | ⛔ **FAIL** — 59, identical to v3 |
| **F2** no immortal jobs | ⛔ **FAIL** — churn |
| **F3** cells recycle | ⛔ **FAIL** — 468,672 recycles *is* the defect |
| **F5** targeted release fires | ✅ **2** — mechanism live, barely exercised |
| ★★ **F6** backstop silent | ✅ **0** across 2.5 h of pathological churn |
| **sentinel S1** | ✅ **3 firings**, log-only |

### ★★★ TWO CLEAN POSITIVES INSIDE A FAILED RUN

- **F6's zero is SPECIFICITY, not just silence.** *The leak witness sat inside a
  subsystem's runaway and did not respond to any of it* — **it measures its own
  subject, not "something is wrong nearby."** *A property a firing could not have
  demonstrated.*
- **The sentinel's 3 firings are CALIBRATION CASE #2**, from an independent famine.
  **Two cases is where a threshold stops being a guess.**

### ★★★★ REFUSAL #5 HELD, UNDER EXACTLY ITS CASE

> *"I will not let route 2's success hide route 1's failure."*

**Route 2's mechanism fired (F5=2). Route 3's defect destroyed the colony. Two
results, separately reported, neither carrying the other.** *Written before the data
existed, for the situation that then occurred.*

---

## 5 · WHAT v4 ESTABLISHED

1. ★★★ **Gate 0 works** — the binary-provenance check earned its checklist entry on
   its first run.
2. ★★★★ **The churn mechanism is confirmed end-to-end**: predicted from code at
   T+15, confirmed in the capture at 468K:468K. *Hypothesis to confirmation in one
   working session, because the heartbeat counters existed to see it.*
3. **Route 2 is not the famine's fix** — *it is a correct guard against a real leak
   (F5=2 proves the leak exists) that does not touch the thing stopping farming.*
4. ★★ **v5's target is isolated**: the sweep must read the JOB's own contiguous
   unclaimed duration, and its falsifier is **reap count FLAT as claim rate varies**.

★ **What v4 did NOT establish:** *the famine's remaining first cause. F1's identity
with v3 says the farm stops for a reason neither route 2 nor route 3 addresses, and
that question is now the arc's open frontier.*
