# WAVE 30 — RESULTS, READ AGAINST A PRE-REGISTRATION WRITTEN BEFORE THE FAN

**`wave30_PRESITE6_0fb7ca07b7_FULL.json` — 48 seeds, `COMMIT=0fb7ca07` ×4,
`DONE=12` ×4, 0 create-fails, 824s, $0.217.**
★ **Fanned against the PINNED REF `bastion/baseline-pre-site6`** — *all four VMs
attested the same commit, which is what the pin was for.*
★★ **Baseline: `wave26_ROWA_d5b56d1c79` (49 commits back, 58 code-file touches).**

## ★★★★★★★ HEADLINE: 6 OF 114 SHARED FIELDS MOVED. **ZERO UNEXPLAINED.**

**Every mover maps to a named, registered source. ★★★ And the verdict did not
move at all:**

> ## ★★★★★ **FAIL SET IDENTICAL — NOT JUST THE COUNT, THE MEMBERSHIP.**
> **`11/48` both waves: `54 61 62 66 68 71 78 80 85 90 92`. Across 49 commits.**

★★ **That is the strongest possible ground for the site-6 exact-match bar: the
work economy is effectively frozen, so any post-row movement will be legible.**

## THE SIX MOVERS, EACH ASSIGNED

| field | seeds | source |
|---|---|---|
| `b5_tool_ok` / `_steel` / `_stone` | ★ **66 only** | **REG-1** |
| `chop_reachability_probe/route_next_idx_pinned` | 4 | **REG-2** |
| `mine_reachability_probe` *(nested same field)* | 6 | **REG-2** |
| `b5_mine_cell_diag` | 6 | ★ **Row B′ — adds `benched_until_tick`** |

★★★ **Plus 16 new keys / 11 removed, all accounted for by REG-3 and REG-4's
renames and the additive instrument windows.**

## ★★★★★ THE REGISTRATIONS VERIFIED, ONE BY ONE

### REG-1 — ★★★ EXACT

**`b5_tool_ok: false → null`, `b5_tool_steel: 0.0 → null`, `b5_tool_stone: 0.0 →
null`, on SEED 66 ONLY, all 47 others held.**

> ★★★★★ **AND `tool_ok` WENT TO `null`, NOT `true`** — *the registration's own
> trap, written days ago: "the honest post-fix value is unknown; flipping it to
> `true` would assert the tools were fine, which the guard never established."*
> ★★ **The build honoured it.**

### REG-3 / REG-4 — ★★★★★ **PURE RENAMES, 0 VIOLATIONS**

**8 renamed fields × 48 seeds, `value(new) == value(old)` checked mechanically:
`0` violations.** ★ *The registration's falsifier was "any value change at all —
a rename that moves a value is not a rename." It didn't move one.*

### REG-2 — ★★★★★★★ **TRANSFORMATION CORRECT, COUNTS SUPERSEDED BY A NEW PRODUCER**

| | `null` | `true` | `false` | total |
|---|--:|--:|--:|--:|
| **wave26** | 89 | 8 | 6 | **103** |

| | `too_few_samples` | `no_route_present` | `compared: pinned` | `compared: advancing` | total |
|---|--:|--:|--:|--:|--:|
| **wave30** | 88 | 11 | 12 | 11 | ★ **122** |

**Registered split was `79 / 10 / 8 / 6`, summing to 103.** ★★★ **The actual
population is 122 — NINETEEN more probe results than the registration's
denominator.**

> ## ★★★★★★★ **THIS IS NOT A FAILED REGISTRATION. IT IS THE NEW-PRODUCER LAW,
> FIRING ON MY OWN PAPERWORK.**
> **`a2745d5a7d` (self-job mode-triple wiring) extended the SAME probe to
> self-job positions. A new producer changes the DENOMINATOR of every count over
> the field it feeds — and a baseline is only a baseline for the producers it
> contained.**

★★★★★ **REG-2 PASSES on its real invariant: ZERO old-form values survive.** *No
`null`, no `true`, no `false` remain anywhere in the field — the three-way
migration is complete.* ★★ **Its per-value COUNTS are simply measured against a
population that grew for a named, independent reason.**

★ **Had I checked only *"does the split sum to 89?"* I would have declared REG-2
FAILED.** ★★★ *The sum-check was the cheap proxy; the migration-completeness
check was the real one.*

## ★★★★★★★★ P3 — **THE THROUGHPUT PREDICTION DID NOT MATERIALISE**

**I registered: *work-job throughput DOWN by roughly the fed/rested fraction*,
with "unchanged ⇒ the unification is INERT in this scenario" as falsifier A.**

> ★★★ **NO WORK-COUNT FIELD MOVED. NOT ONE. The fail set is byte-identical.**

**AND YET THE MECHANISM IS EXERCISED:**

> ★★★★★ **`b5_self_job_reachability_probe` is NON-EMPTY on 14 of 48 seeds**
> *(49 54 56 61 62 64 66 67 69 71 78 80 85 92)* — **self-jobs are created,
> travel, and TIME OUT on 29% of the corpus.**

### ★★★ SO THE HONEST READING, AND IT IS NOT "PASS"

**Self-jobs run on 14 seeds and the work economy is unchanged to the byte.** ★★
**Two accounts fit, and this corpus cannot separate them:**

| | account |
|---|---|
| **(a)** | self-jobs are rare/short enough not to measurably displace work |
| ★★★★★ **(b)** | **they never COMPLETE** — *they time out (which is what the probe records), the colonist returns to work, no rest is banked and no work is lost* |

> ★★★★★★★ **(b) IS A DEFECT WEARING A PASS'S CLOTHES: colonists that go to bed,
> fail to arrive, and come back to work would produce EXACTLY this signature —
> an unchanged economy and a non-empty timeout probe.**

★★ **Registered in advance as falsifier A′ in a slightly different form** *(I
predicted it paired with a throughput FALL)*. ★★★ **The real signature is
throughput UNCHANGED plus timeouts, which is the same defect with the cost
hidden rather than visible.**

★ **NOT resolvable from this corpus** *(no rest/completion field exists — see
`NEED-SUBSYSTEM-OBSERVABILITY-SPEC.md` N1-N6).* ★★★★★ **Resolvable from the
14-seed exposure set with one instrumented run, and that is the next question,
not a wave-30 conclusion.**

## ★★ P4 — ONE PREDICTION CONFIRMED BY AN ABSENCE

**`b5_self_job_reachability_probe` LANDED** *(first self-job field this corpus has
ever carried)*. ★★★★★ **`settle_invariant_*` DID NOT** — **confirming tonight's
independent finding that it is emitted only inside `auton2_needs_probe`, never in
`b5_scenario`.** ★ *Instrumented everywhere, observed in one probe — now measured,
not inferred.*

## ★★★★★ WHAT I GOT WRONG, PRE-REGISTERED AND CORRECTED

**I inverted P1 an hour before the fan, claiming REG-1..4 were "registered but
never built," on the strength of grepping the harness for their field names.**

> ★★★★★★★ **`b5_tool_ok` KEPT ITS NAME WHILE ITS VALUE SEMANTICS CHANGED. I read
> a NAME and concluded about MEANING** — *the day's law, for the fifth time, in
> the one place I had already written it down.*

★★ **And my evidence for REG-3 was that `b5_55_blocked_by` appeared "only in a
comment" — which was evidence the emitted field had been RENAMED, i.e. proof of
exactly the opposite of what I concluded.**

★ **Cost: zero.** ★★★ *The original P1 was right, the wave confirms it, and the
inversion never reached a build or a ruling — because the pre-registration was
written down where the data could contradict it.*

## ★★★★★★★★ P3 **RETRACTED** — `b5_self_job_reachability_probe` DOES NOT MEASURE SELF-JOBS

**MEASURED, matched control, one run** *(seed 71, `--b5-scenario`, both env-gated
diags on, same binary)*:

| diag | hits |
|---|--:|
| `BASTION_RELEASE_DIAG` *(known-working control)* | ★★★★★ **34** |
| `BASTION_SELFJOB_COMPLETION_DIAG` | ★★★★★ **0** |

★★★ **Env propagation works, logging works, the job pipeline runs. ★★★★★ THE
SELF-JOB CREATION SITES NEVER EXECUTE IN `--b5-scenario` — zero across all 14
seeds.** ★ **The instrument is fine: `--preempt-scenario` seed 49, same binary,
CREATED 4 / COMPLETED 1.**

### ★★★★★ WHY THE CLAIM WAS WRONG

**The field's producer, quoted verbatim in my own report hours earlier:**

> *"subtracts `mine_cell_diag`'s own positions … whatever's left is every
> non-mine **(i.e. self-job, in this scenario)** position that ever timed out."*

★★★★★ **THE FIELD IS "NON-MINE TIMEOUT POSITIONS." The parenthetical is the
author's ASSUMPTION — refuted by this same night's work, which showed BUILD and
CHOP jobs timing out constantly in this scenario.**

> ## ★★★★★★★ **"14/48 SEEDS EXERCISE SELF-JOBS" WAS ACTUALLY "14/48 SEEDS HAVE
> NON-MINE TIMEOUTS" — THE BUILD AND CHOP FAMILIES, CHARACTERIZED IN THIS SAME
> DOCUMENT SET.**

★★★ **I read the producer, quoted its exact words, and still took the NAME.**
★★★★★ **A PARENTHETICAL DID THE WORK A NAME CANNOT.** ★★ *Eighth costume of the
day's law, and the costliest: an entire two-arm measurement was designed on it.*

## ★★★ WHAT CHANGES — AND MOST OF IT IS GOOD

- ★★★★★ **P3's falsifier A is CONFIRMED: the unification is INERT in
  `--b5-scenario`.**
- ★★★ **The "suspicious null" was never a null.** *No mechanism ran, so nothing
  could move. There is no defect hiding here.*
- ★★★★★★★ **THE FAN'S BAR IS BLANKET EXACT-MATCH WITH NO CARVE-OUTS** — *the
  concern that it would indict a correct row evaporates, because the row's
  mechanism cannot touch this scenario at all.*
- ★★ **The completion question moves to `preempt_scenario`**, where self-jobs
  exist. **First number, before-arm seed 49: 4 created, 1 completed.**

## ★ WHAT SURVIVES UNCHANGED

★★★★★ **Everything else in this document.** *The 6-of-114 movers and their
assignments, the identical fail-set MEMBERSHIP across 49 commits, REG-1's exact
delta, REG-3/4's zero-violation renames, and REG-2's denominator finding — none
of them depended on P3.*

## ★★★★★ WHAT CAUGHT IT

**The zero-guard written INTO the extractor before any data existed:**
*"BOTH ZERO — the instrument did not fire. That is 'measured nothing', NOT
'nothing happened'. Do not read a diagnosis out of it."*

★★★ **It fired on its first use, against its own author's measurement, and
stopped a zero from being read as evidence for account (a).** ★ **One matched
control then diagnosed it.** ★★ **Cost: two builds and a 14-seed run. No
conclusion reached the bar.**
