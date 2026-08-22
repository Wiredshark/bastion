# PRE-REGISTRATION — why 57% of eat trips die with no witness

Written **before** the change. This is an INSTRUMENT, not a fix. Four food
hypotheses have been refuted this session by reasoning ahead of measurement;
this row deliberately builds no remedy at all.

## The measurement that forced this row

Aggregated over the whole of `play/server-4.log` (~30 min wall, ANSI stripped
first — the raw log defeats key=value greps):

| emit | count |
|---|---|
| `ITEM 27 cooked` | **155** |
| `need preempt — hunger below interrupt` | **103** |
| `bastion: ate` | **20** |
| `food sniped` | **24** |
| `slept` | 4 |

`food_stock` over the run: **0 → 663 → 1577**, climbing monotonically.

Binned per minute, cooking is steady across all 27 minutes and preempts fire
steadily from minute 5 to minute 27. So production is healthy and hunger keeps
asking. **20 of 103 preempts end in an eat (19%). 24 more die sniped (23%).
The remaining 59 — 57% — produce no emit of any kind.**

This kills the two rows I had queued (death-loot persistence, merge-theft).
Both are SUPPLY fixes, and supply is not the constraint: the colony starves
beside 1,577 units of food in the pantry.

## The mechanism, read from the producer

`bastion_jobs.rs`, the release drain (`for (entity, release_reason) in
&to_release`): an `EatFrom` is a SELF-JOB, so on release it is not merely
unclaimed — `board.remove_job(job_id)` **destroys it**, silently. There is no
`info!` on that path. That is the shape of a 57% null.

The drain already computes exactly the field I need:

```rust
*board.release_reason_counts.entry(*release_reason).or_insert(0) += 1;
```

`release_reason_counts` is incremented at that one site and **read nowhere in
the crate** (grep: two hits, the declaration and this increment). It is a
witness with no consumer — the counter that would explain the null has been
accumulating in the dark the whole time.

## The change

1. Key the counter by **(job class, reason)** instead of reason alone. Global
   across all job kinds it cannot answer this question at all: farm and haul
   releases outnumber eat releases and would swamp the bucket. Aggregate late,
   keep the structure.
2. Give it a consumer: one line at the existing `tick.0 % 300` census cadence,
   beside `claim refusal census`.
3. Classify a release whose job is already gone as `"gone"` rather than
   dropping it, so the census's own total reconciles against `to_release`.

## The prediction

The five reasons are `Other`, `TimedOut`, `Completed`, `RemovedExternally`,
`TargetChanged`. For `EatFrom`, over a leg of this length, PASS is simply:

**the census accounts for the gap.** `EatFrom` releases summed across all
reasons should land near **103 − 20 = 83**, and `Completed` should land near
**20**. Those two numbers are already known from independent emits, so the
instrument is checkable against ground truth I did not derive from it.

That check is the point. A census that reports a plausible-looking histogram
which does NOT reconcile with the known 103/20/24 is a broken instrument, and
I would rather find that now than build a fix on top of it.

**FAIL / VOID branches, named now:**

| Observation | Means |
|---|---|
| `EatFrom` total ≈ 0 while preempts continue | Eat jobs do NOT die through this drain. Some other teardown removes them and I have instrumented the wrong path — the 57% is elsewhere. |
| `EatFrom` total ≫ 83 | Eat jobs are released and recreated repeatedly within one hunger episode. The preempt count undercounts attempts, and "57% of trips fail" is the wrong framing — it would be far worse, a churn. |
| Reasons are ~all `Other` | The classification never reached this site; `Other` is documented as a step-1 placeholder meaning unclassified. The histogram would be real but useless, and the next row is classifying the site, not fixing eating. |
| Reasons ~all `RemovedExternally` | The food entity is being destroyed — which would revive the supply story I just discarded, and I should say so plainly rather than defend the reframe. |
| `gone` dominates | The job is already removed before the drain sees it, so the drain is downstream of the real teardown. |
| Census prints but every bucket is 0 | The emit is placed where `to_release` is empty, or the cadence gate never coincides. A zero histogram and an unreached instrument render identically — this is the branch most likely to fool me. |

## What this run cannot test

- **Whether anything eats better.** Nothing here changes behaviour. If the
  colony's numbers improve after this lands, that is noise or another change,
  and I must not read it as this row working.
- Whether the 24 snipes and the 59 silent deaths share a cause.
- Sleep, pathing, teleports, or anything visual.

## The mistake this row is guarding against

I reported `food_stock` climbing as evidence the colony was healthy in an
earlier leg. It is equally consistent with a colony that cannot reach its own
pantry, and this run is the first to distinguish them. The number was never
wrong; the reading was.
