# SELF-RESCUE HAS A 100% REFUSAL RATE IN THE CORPUS — 71 CALLS, ZERO PLANS

**Computed from `wave25_BASELINE_e86fe79893_FULL.json` and
`wave26_ROWA_d5b56d1c79_FULL.json`, 48 seeds each. READ FROM DISK, NO RUN.**
Counter semantics verified at `5f8cdf1392`. **Identical in both waves.**

## §1 — THE NUMBERS

| caller | calls | emissions | **refused** | rate | seeds w/ refusal |
|---|--:|--:|--:|--:|--:|
| **`self_rescue`** | **71** | **0** | **71** | **100%** | 15/48 |
| `emergency` | 579 | 78 | **501** | **86.5%** | 41/48 |
| `proactive_descent` | 0 | 0 | 0 | — | 0/48 |

**`self_rescue_starved` = 0 in every seed of both waves.**

## §2 — WHY THE NUMBERS MEAN WHAT THEY APPEAR TO MEAN

**The fields' own doc comments settle the semantics** — I did not infer them:

> `access_plan_calls`: *"Counts every invocation **regardless of outcome**."*
> `access_plan_emissions`: *"per-caller **SUCCESSFUL** `plan_access` emissions
> (the `Some((kind, steps))` arm) … `emissions <= calls` always; **the gap is
> refusals, not non-calls.**"*

And the **non-call** half is separately instrumented: `access_plan_self_rescue_
starved` counts requests killed by the colony-global `access_pending` bar before
the loop body ran. **It is zero in all 96 seed-runs.**

> ★ **So all 71 self-rescue requests genuinely reached `plan_access`, and
> `plan_access` refused every single one.** Not starved, not un-called, not
> rarely-successful. **Zero plans, ever.**

## §3 — ★★★★★ WHAT THIS IS

**The mechanism that exists to dig a stranded colonist out has never once
produced a plan in the entire corpus.** The need arises regularly — **71 times,
across 15 of 48 seeds** — and the answer is always nothing.

And it is **deterministic**: byte-identical totals across two waves at different
commits. **This is not a flake; it is a standing property of the system.**

★ **The emergency path is the same defect at 86.5%**, in **41/48 seeds** — the
broader and better-exercised population, refusing five of every six requests.

## §4 — WHAT THIS DOES **NOT** ESTABLISH — the WHY is still open

`plan_access` has **several** refusal gates, and this measurement does not
distinguish them:

1. ★ **the designation-mask gate** — `allowed = in_access_mask(mask, p)`, the
   condemned-cell hypothesis (`CONDEMNED-CELL-FINDING.md`)
2. `unavailable_cells` — plan cells overlapping other jobs / live emergency routes
3. **geometric failure** — no walkable stair base adjacent to the digger
4. the **M2 walkability validation** — vertical rises the digger cannot stand to work

> **The approved discriminator is still exactly the right next step, and it is
> now MUCH better motivated:** it no longer asks *"does this ever happen?"* — the
> corpus answers that — but *"which of four gates closes on 71 out of 71."*

★ **Do not read this as confirming the mask hypothesis.** It confirms the
**refusal**, which the mask hypothesis predicts and so do three others.

## §5 — ★★★ THE META-FINDING: THE INSTRUMENT WAS BUILT FOR THIS AND NEVER READ

These counters exist **because of DECISIONS #49** — the previous instance of
*"the corpus cannot see access-plan state."* The field doc says so outright:

> *"the question #61's non-call-vs-rejection falsification needed and the corpus
> couldn't answer (zero access-plan state visible anywhere)."*

**The instrument was correctly identified, correctly specified, correctly built,
and has been faithfully recording a 100% failure rate ever since — and nobody
subtracted one column from the other.**

> **AN UNREAD FIELD AND AN ABSENT FIELD PRODUCE THE SAME SILENCE.** Adding the
> instrument was never the finish line. **A measurement nobody computes is
> indistinguishable from one nobody took.**

★ This is [[enumerate-what-the-instrument-can-see]]'s companion failure, and the
cheaper one to fix: **the corpus is UNDER-READ**, and the fix is a standing
derived-quantity check — *calls − emissions*, per caller, every wave — not a new
field.

## §6 — CONSEQUENCE FOR ROW PRIORITY

The condemned-cell row was banked **third**. **A rescue mechanism with a measured
100% failure rate, deterministic, in 15/48 seeds, is not a third-priority row** —
and the emergency path's 86.5% across 41/48 seeds is a *wider* population than
anything AUTON-2 touches. **Re-ranking is Fable's call; the number is the
argument.**
