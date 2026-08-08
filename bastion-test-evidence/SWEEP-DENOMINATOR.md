# THE SWEEP'S ANSWER IS "NEITHER BRANCH" — THE INSTRUMENT LOOKED AT 6 SEEDS

**wave25 + wave26, byte-identical results.** Read from disk, no run.

## §1 — THE PRE-STATED BRANCHES CANNOT BE RESOLVED, BECAUSE THE DENOMINATOR IS NOT 48

| denominator | class-B share | reads as |
|---|--:|---|
| all seeds in the wave | 2 / **48** | **4% — "isolated geometry"** |
| timeout-bearing probe results | 6 / **40** | 15% |
| ★ **seeds the probe ACTUALLY EXAMINED** | 2 / **6** | ★ **33% — "named mechanism"** |

> **Same data. Three denominators. Opposite branches.** ★ **I nearly reported the
> 4% — the flattering, defensible-looking one — and it is the most wrong of the
> three.**

## §2 — ★★★★★★ THE COVERAGE FACT

```
seeds in wave                        48
seeds with a reachability probe       6   (52, 54, 61, 66, 71, 90)
seeds with travel timeouts           31   <- 25 of them UNPROBED
seeds FAILING                        11   <- 6 of them UNPROBED
```

**The probe examines 6 of the 31 seeds that actually experienced travel
timeouts — 19%.** Everything it says is conditional on a selection nobody
declared.

> ★★★ **"Class B is rare" and "the probe rarely runs" are indistinguishable in
> this data.** Absence of a probe is not absence of the class.

## §3 — ★★★★★★ RETRACTED: THE DIAGNOSTIC IS CORRECTLY SCOPED, NOT BLIND

**I claimed the permanent core is "unseen by the campaign's best diagnostic" and
that "nobody noticed the diagnostic wasn't pointed at it." WRONG. I read the
coverage number without reading the producer.**

`mine_cell_diag` (harness `3338-3397`) scans the **mine designation volume** and
includes every cell that **still holds an open Job**. Its own comment says it:

> *"this cell being present in `mine_cell_diag` at all already means it never
> completed"*

★ **It is not a gate. It is a DEFINITION.** Non-empty **iff the mine has
unfinished cells**. So the 6 probed seeds are **exactly the seeds whose mine did
not finish** — the correct and intended population for a mine probe.

**And the split is perfect:**

| | failing seeds | fail a MINE clause |
|---|--:|--:|
| **probed** | 5 | **4** |
| **UNPROBED** | 6 | ★ **0** |

> **The 6 unprobed failures are not mine failures at all** — they are
> `chop_cleared` / `log_sum` / `build_placed` / `any_needs_materials` /
> `ch_mixed`. **A mine diagnostic correctly declines to describe them.**

★★ **The coverage gap is real but different from what I said:** the **mine**
family has a rich diagnostic; **the chop/build family has NONE.** That is the
honest statement, and it is a smaller and more actionable claim.

## §3b — ★★★★★ AND THE CORE IS PROBABLY ~3 FAMILIES, NOT 10 BUGS

I wrote that the 10 core seeds *"may be ten unrelated bugs."* **The clause
structure says otherwise:**

| family | seeds | clauses |
|---|---|---|
| **MINE** | 54, 61, 71 (+90 regressed in) | `mine_cleared`, `mine_blocks_mined` |
| ★ **CHOP → MATERIALS → BUILD** | 78, 80, 85, 92 (62 partial) | `chop_cleared`, `log_sum`, `build_placed`, `any_needs_materials` |
| isolated | 66 (`tl_ok`), 68 (`ch_mixed`) | — |

★★★ **The second family looks like ONE upstream failure cascading:** chopping
fails → no logs → no materials → build fails. **Four clauses, one cause** —
`build_placed` and `any_needs_materials` co-occur **6 times**, `chop_cleared` and
`log_sum` **4 times.**

> **The standing worklist is probably ~3 root causes, not 10.** ★ *Offered as
> structure, not proof* — co-occurrence is consistent with a cascade **and** with
> correlated difficulty, and only a read of the chop path settles it.

## §3c — (SUPERSEDED) THE ORIGINAL CLAIM

**6 of the 10 always-failing seeds have NO probe and NO `mine_cell_diag` at
all:** **62, 68, 78, 80, 85, 92.**

> **THE CAMPAIGN'S STANDING WORKLIST IS UNSEEN BY THE CAMPAIGN'S BEST
> DIAGNOSTIC.** Ten seeds have failed every wave; **six of them have never been
> looked at by the instrument built to explain exactly this.**

★ **That is why the core has never been diagnosed** — not neglect, and not a hard
problem. **Nobody noticed the diagnostic wasn't pointed at it.**

★ `mine_cell_diag` and the reachability probe populate for **the same 6 seeds**,
so **whatever gates `mine_cell_diag` gates the whole diagnostic chain.** **Finding
that gate is the highest-value next read in this area** — it would take the
permanent core from *undiagnosed* to *diagnosable* without a single new field.

## §4 — WHAT THE SWEEP DOES ESTABLISH

- **Among the 6 examined seeds, the taxonomy is stable and replicates exactly
  across both waves:** A=31/22, B=9/6, C=59/12, class B = {71, 90}.
- ★ **Class B being exactly the two registered fork-marker specimens still
  stands** — and is *more* striking now: **of only 6 seeds ever examined, the two
  showing mode-limitation are the two independently flagged months earlier.**
- **§4.2's distance threshold is still dead** — that finding does not depend on
  coverage, since the overlap is *within* the examined population.

## §5 — THE HONEST BRANCH ANSWER

> **NEITHER.** *"Large fraction ⇒ named mechanism"* and *"small/zero ⇒ isolated
> geometry"* both presuppose the sweep saw the population. **It saw 19% of the
> relevant seeds, selected by an undeclared gate.**

★ **The mechanism is REAL where observed** (2 of 6, replicated, landing on the
pre-registered pair) and its **PREVALENCE IS UNMEASURED.** Those are different
claims and only the first is supported.

**Recommended order:** find `mine_cell_diag`'s gate → widen or remove it → re-run
the sweep on a population that includes the permanent core. ★ **Do not size the
fix family off a 6-seed sample**, and do not let *"4% of seeds"* enter any ledger.
