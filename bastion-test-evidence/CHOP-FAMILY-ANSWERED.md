# THE CHOP FAMILY — SAME MECHANISM AS BUILD, AND A **THIRD INSTRUMENT** ARRIVES

**`b5_ch_job_diag` landed in wave 30 (first appearance) and closes the gap
`CHOP-INSTRUMENT-SPEC.md` opened with:** *"Chop is not uninstrumented — it is
instrumented for REACHABILITY and for nothing else. **We can say where the tree
is and whether a path exists. We cannot say whether anyone ever tried.**"*

> ★★★★★ **We can now say whether anyone tried. They did.**

## ★★★★★★★ THE MEASUREMENT — ALL FOUR CHOP-FAMILY SEEDS

| seed | `unreachable` | `times_offered` | `timeouts` | `starvation_cycles` | `blocked_by` |
|---|---|--:|--:|--:|---|
| **78** | ★ **True** | 2 | **2** | **320** | null |
| **80** | ★ **True** | 2 | **2** | **213** | ★ **== pos** |
| **85** | ★ **True** | 2 | **2** | **283** | ★ **== pos** |
| **92** | ★ **True** | 4 | **4** | **247** | ★ **== pos** |

★ **Passing seed 49 emits NO chop diag entries** — *the diag surfaces only
still-open chop jobs, so its emptiness on a passing seed is correct, not a gap.*

> ## ★★★★★ **IDENTICAL SIGNATURE TO THE BUILD FAMILY: `times_offered ==
> timeouts_on_this_cell` ON ALL FOUR. EVERY OFFER ENDED IN A TIMEOUT.**

★★★ **And the SAME two-mechanism split, by the same field:** ★ **planner-refused
(80, 85, 92 — `blocked_by == pos`)** · ★ **strike-released (78 — `blocked_by`
null)**.

## ★★★★★★★★ SEED 85 — THE CASE THE SPEC CALLED "THE WHOLE CASE", NOW WITH ITS OTHER HALF

**`CHOP-INSTRUMENT-SPEC.md` §2, from wave25:**

> *"`b5_chop_reachability_probe` for seed 85: **`path_exists_step / jump /
> scramble = TRUE, TRUE, TRUE`**. `min_distance = 8.5`. And `chop_cleared =
> False`, `log_sum = 0`."*

**Wave 30 supplies the half it couldn't see:**

> ★★★★★★★ **THE PROBE SAYS REACHABLE BY ALL THREE MODES. THE JOB IS FLAGGED
> `unreachable`, WAS OFFERED TWICE, TIMED OUT BOTH TIMES, AND STARVED FOR 283
> CYCLES.**

## ★★★★★ WHY THIS MATTERS BEYOND CHOP — **IT IS A TIE-BREAKER**

**`ROUTER-VS-PROBE-DISAGREE.md` (already filed, wave25) measured:**
★★★ ***18 of 44 — 41% — of probed timeouts have the offline PROBE and the live
ROUTER giving OPPOSITE answers. Nine each way. They cannot both be right.***
★★ **And it names the stake: *"CLASS B is a PROBE OUTPUT. If the probe is the
instrument that's wrong, Class B is an artifact — and 'colonists can't jump' was
never real."***

> ## ★★★★★★★ **THE PER-JOB DIAG IS A THIRD INSTRUMENT, AND IT IS NOT AN OPINION —
> IT IS WHAT ACTUALLY HAPPENED. Colonists were offered the job, went, and failed.**

★★★★★ **On seed 85 the live attempts side with the ROUTER against the PROBE.**
★★ **Two instruments to one, and the third is the only one made of real
traversals rather than an offline query.**

### ★ STATED AT ITS REAL STRENGTH — NOT MORE

- ★★★ **This is FOUR seeds, and the tight probe-vs-live comparison is quoted for
  ONE (85).** ★ **The other three need their probe fields read the same way
  before the claim generalizes.**
- ★★ **A timeout is not proof a path is absent** — *a colonist can fail to walk a
  path that exists* **(that is the whole travel row).** ★★★★★ **But it IS proof
  the probe's answer did not predict the outcome, which is exactly what the 41%
  row is about.**
- ★ **NOT a declaration that the probe is wrong.** *It is one more case where the
  probe's optimism was not borne out, on a seed already nominated as decisive.*

## ★★★ WHAT IT ADDS TO THE ROW

1. ★★★★★ **The chop family folds into the travel row on the SAME evidence as the
   build family** — *same signature, same split, same field doing the typing.*
2. ★★★ **`ROUTER-VS-PROBE-DISAGREE` gets a third instrument** — *and it should be
   re-run on wave 30, where per-job attempt data now exists for chop AND build.*
3. ★★ **The hard core's picture is now: TEN seeds, TWO mechanisms, ONE subsystem.**
   *Build 6 + chop 4 overlap on 80/85/92 — three seeds carry BOTH families, and
   both fail the same way.*

> ★★★★★ **THE OVERLAP IS THE STRONGEST HINT: seeds 80, 85 and 92 fail build AND
> chop, both with `unreachable`, both with every offer timing out.** ★★★ **That is
> not two coincident failures — it is one colony that cannot get anywhere.**

## ★ PRE-READ TRIO, APPLIED AND PAID

★★★ **Step 1 (grep the ledger) surfaced `CHOP-INSTRUMENT-SPEC.md` and
`ROUTER-VS-PROBE-DISAGREE.md` BEFORE I read a number** — *so this landed as a
contribution to a filed question instead of a rediscovery of it.* ★★ **The spec
had already nominated seed 85 and already stated exactly which half was
missing.** ★ **I supplied the half; I did not find the question.**
