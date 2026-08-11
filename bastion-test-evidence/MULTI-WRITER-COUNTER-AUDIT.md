# MULTI-WRITER COUNTER AUDIT — `bastion_jobs.rs`

**Chartered by Fable after two union counters each cost an investigation.**
*`b5_f3_prunes_fired` failed 8 of 48 seeds and nearly indicted a shipped row;
`egress_verdicts` currently blocks item 4b's scoring.* **The class recurs, so it
gets a sweep instead of another discovery.**

**Method:** enumerate every counter with ≥2 increment sites at `58cd1e8bef`, then
read each site's meaning. **A counter is only sound if every writer means the
same thing.**

---

## RESULTS — 7 multi-writer counters, 4 dispositions

### ★★★ 1 · `egress_verdicts` — **CONFIRMED UNION, 4 WRITERS, AT LEAST 4 MEANINGS**

| site | what it actually records |
|---|---|
| `:14922` | ★ **churn-detector fire** — a stuck claim released for rescue. **Not about egress at all.** |
| `:15095` | `egress_scan` OK + below-grade → **target INSERTED** |
| `:15133` | `egress_scan` OK but **`rim` is None** → **NO target found** |
| `:15168` | the post-`rim` path |

> ## **SITES `:15095` AND `:15133` ARE OPPOSITES — "found a target" and "found no target" increment THE SAME COUNTER.** *And `:14922` is a different subsystem entirely.*

★★ **This is why item 4b cannot be scored.** *uid=81's `egress_verdicts=6` may be
any mixture of target-found, target-not-found, and churn fires — and those
license opposite conclusions about whether the planner is at fault.*

★ **I under-reported this myself**: an ad-hoc grep found 2 sites; the systematic
sweep found 4. **The audit's first act was to correct the person who requested
it** — which is the argument for sweeps over spot-checks in one line.

### 2 · `b5_f3_prunes_fired` — **KNOWN UNION, HANDLED**

Two writers: branch-B stale sweep (`:15129`) and branch-C stall sweep (`:15199`).
**Documented, and the validator now uses an ATTRIBUTION clause** — fails only when
*neither* threshold was reached. *Left as-is: the union is intentional and its
consumers know.*

### ★★ 3 · `access_plan_calls` — **THE PATTERN TO COPY, ALREADY IN THE TREE**

    :14775   access_plan_calls.entry("self_rescue")
    :15694   access_plan_calls.entry("emergency")

> **Two writers, and it is NOT a union — the map is KEYED BY PRODUCER.** *Each
> caller's count is separately readable and a third caller cannot silently merge
> into the others.*

★★★ **This is the fix `egress_verdicts` needs, and it already exists one row
over.** *Recommended split: `egress_verdicts.entry("churn" | "target_found" |
"no_rim" | "post_rim")` — no new mechanism, just the established pattern applied.*

### 4 · `total_claims` (5 writers) · `preempt_attempts` (4 writers) — **AGGREGATE BY NAME, PROVISIONALLY SOUND**

**Both are plural-by-construction: every writer means "one more of the named
event."** *No opposites, no cross-subsystem mixing found.* ★ **Recorded as
`single-meaning, justified` rather than audited clean** — *a full read of all 9
sites was not done, and that is stated rather than implied.*

### 5 · `next_id` (12) · `next_zone` (4) — **EXEMPT, NOT COUNTERS**

**Monotonic allocators, not measurements.** *Nothing reads them as a quantity of
anything.*

---

## THE RULE THIS SWEEP LEAVES BEHIND

> ## ★★★★★ **A COUNTER WITH TWO WRITERS NEEDS EITHER A KEY OR A DOCUMENTED
> SINGLE MEANING. THERE IS NO THIRD OPTION.**

★★ **And the diagnostic that finds them cheaply: for each writer, write the
sentence "this increment means ___." If two sentences differ, the counter is a
union and every reader of it is guessing.**

★ **Cost of learning this at retail: two investigations.** *`prunes_fired` — 8
false-failed seeds and a builder's shipped row nearly indicted. `egress_verdicts`
— item 4b unscoreable until it splits.* **Cost of the sweep: one grep and four
reads.**

## STANDING

- **`egress_verdicts` split is item 4b's named instrument dependency** — rides
  item 8's build if pre-launch, waits if not. *The run is worth more than the field.*
- **`total_claims` / `preempt_attempts`** — provisionally sound, full read owed if
  either is ever used as evidence in a scoring argument.
- **New counters:** key them at birth. *The pattern costs nothing when the second
  writer arrives and everything when it arrives unnoticed.*
