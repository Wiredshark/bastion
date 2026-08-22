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

---

# AMENDMENTS — written before any leg was spent

Four checks fired between writing the above and running anything. All four are
recorded here because three of them changed the plan and one of them would have
quietly invalidated the result.

### 1. FAIL branch 3 was already true — caught by grep, not by a leg

The branch "reasons are ~all `Other`" was checked statically against the push
sites: **32 `to_release.push` sites, only 4 carrying a real reason.** The other
28 push the documented step-1 placeholder. The census as first committed would
have reported `eat/Other=83` — reconcilable, honest, and naming no code path.

Fixed by putting the **site line in the key**. The discriminator already
existed: every one of those 32 sites was already appending an env-gated diag
carrying `line!()`. The value was in reach the whole time and never reached the
tally.

### 2. The treatment did not reach the population — caught before measuring

Slot 8 was booted with `PLAY_EXTRA_ENV="BASTION_NEEDS_DECAY_MULT=3"` via
`export` ahead of a `nohup`'d background shell. **The export did not survive.**
The baseline logs `DECAY_MULT active mult=3.0` right after its mood config;
slot 8 logged the mood config and no decay line. That leg would have run at 1×
against a 3× baseline — not comparable, and starved of the very events the
census counts. Slot 8 was stopped, not measured. Env now goes inline.

### 3. A prediction I am NOT banking

The decay constants (0.000889/s × 3, comfort 0.5 → interrupt 0.2) give a
112 sim-sec hunger cycle; over the measured 1500 sim-sec × 8 colonists that
predicts **~107 preempts against 103 measured.** A 4% match.

**I am discarding it as probable coincidence.** The model assumes colonists
cycle comfort → interrupt, which requires them to eat, and with 20 eats against
103 preempts most of them never return to comfort at all. Two different
mechanisms can produce the same number, and a satisfying agreement derived from
an assumption the data contradicts is exactly the "fitting story" this row
exists to avoid.

Measured sim:wall for the baseline is **0.91** (45,012 ticks / 1,657 s at 30
ticks/sim-sec), so the conversion itself is sound — it is the cycle model that
is wrong, not the arithmetic.

### 4. …and the honest quantity that replaced it, then died too

103 serial trips across 8 colonists over 1500 sim-sec puts a failed eat trip at
**~115 sim-seconds**. That looked like a timeout, so the timeout was checked:
`derived_access_stall_default_secs = 120 + 90×8 + 90 = 930 s` (confirmed live —
the log's `unclaimed_secs=930.49`). **Eight times too long.** Not the mechanism.

Three guesses killed by static checks costing seconds each. None of them
reached a leg, and none of them reached the write-up as a finding.

### 5. THE PREDICTION ITSELF WAS WRONG — corrected before the run

The original PASS said "`Completed` should land near 20". **It will not.**
Reading the sites rather than assuming them:

| site | what it is | expected |
|---|---|---|
| `@18874` | the SUCCESSFUL eat — emits `ate — hunger restored` | ≈ 20 |
| `@19076` | the snipe — emits `food sniped — eat moot` | ≈ 24 |
| `@15867` | moved-meal release — **unreachable for eat**, 15857 `continue`s to re-target first | ≈ 0 |

The successful eat path pushes **`ReleaseReason::Other`**, not `Completed`.
`Completed` is pushed at exactly one site (`@20470`) which is a work-region
completion, not an eat.

So the corrected PASS is: **`eat/Other@18874 ≈ 20`, `eat/Other@19076 ≈ 24`, and
a THIRD eat site carrying ≈ 59** — and the identity of that third site is the
entire question this row exists to answer.

Note what the site key bought here. Keyed by `(class, reason)` alone, all three
of these collapse into one indistinguishable `eat/Other ≈ 103` bucket — success,
snipe and the unknown killer summed together, reconciling perfectly against the
preempt count while hiding every distinction that matters. It would have looked
like a clean PASS.

## The mistake this row is guarding against

I reported `food_stock` climbing as evidence the colony was healthy in an
earlier leg. It is equally consistent with a colony that cannot reach its own
pantry, and this run is the first to distinguish them. The number was never
wrong; the reading was.
