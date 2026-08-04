# FIX-ROW PACKET — ARB-PERSIST (persistent-block detection, escalation, recording)

**Assigned by** Fable (architect) after DECISIONS #54.
**Drafted** 2026-08-04, same session as the evidence — deliberately, so nothing
here is reconstruction.
**Engine tip this packet indexes: `a85dec2912`.**

> **Every line number below indexes the blob `a85dec2912:bastion-server/src/bastion_jobs.rs`**
> (17303 lines), extracted with `git show` and read directly. The main checkout
> (`e69dd80d61`, `bastion/block-B6HAUL`) has this file at a *different path*
> (`server/src/bastion_jobs.rs`) and *different line numbers* — a line number
> without its blob is not a citation.
> Verify before use:
> ```bash
> git show a85dec2912:bastion-server/src/bastion_jobs.rs | sed -n '11370,11425p'
> ```

## §0 — READ MARKS (the legend this packet is graded against)

| mark | meaning |
|---|---|
| **READ** | I read the code at `a85dec2912`, line cited. Semantic claim is mine. |
| **UNREAD** | named but not read by me — treat as a pointer, not a claim |
| **5b-TRACE** | rests on 5b's step-2 per-attempt trace. **See §10 — that work is UNCOMMITTED.** |
| **CORPUS** | rests on fan data on disk (`bastion-test-evidence/corpus-waves/`) |

No claim in this packet is marked READ on the strength of a single call site.
The rule that forced this (`unreachable` characterised from one of three
writers) is the reason §6 exists at all.

---

## §1 — THE NAMED INVARIANT (verdict #54)

> **A job that has been claimed, attempted, and released N times with zero
> progress is not experiencing transient congestion. The system must be able to
> (i) notice that, (ii) stop paying for it, and (iii) say so.**

Today the system does none of the three for `Mine` and `Farm` work. The three
capabilities are not missing in general — **each one already exists somewhere in
this file, for some other case.** That is the whole shape of this row.

| capability | exists? | where | who it covers |
|---|---|---|---|
| (a) per-job persistence signal | **YES** | `stuck_strikes`, incremented **11353** | **every kind** |
| (b) escalation on that signal | **YES** | **11374–11401** | **`Haul` only** |
| (c) recording into #55 visibility | **YES** | **12886** | **plan_access's `None` arm only** |

**The defect is not absence. It is that (a) is computed for everyone and read by
(b) for one kind, and (c) sits behind a gate that (b) never reaches.**

---

## §2 — THE THREE PARTS

### (a-revised) — READ, `a85dec2912:11350-11353`

**Revised from the original framing ("add per-job persistence detection").**
The counter already exists and is already kind-agnostic:

```rust
job.claimed_by = None;
// B5.8-E: strike — grows the remote-work
// arrival tolerance (see the arrive calc).
job.stuck_strikes = job.stuck_strikes.saturating_add(1);
```

**READ:** the increment is unconditional on `JobKind` — it runs on the travel
timeout path for every claimed job, Mine and Farm included. Its *documented*
purpose is to widen arrival tolerance (`stuck_strikes.min(3)`, consumed at
**9625**).

> **★ The persistence signal for mine/farm jobs is already being computed every
> cycle and then discarded.** Part (a) is not construction. It is **reading a
> number the system already keeps.**

This is the single most important revision in the packet and it is *why* the
original framing would have produced a worse fix: a builder told to "add
persistence detection" adds a second counter beside a live one, and the two
diverge the first time somebody resets one.

**What (a) actually needs:** nothing new on the write side. One decision —
whether the mine/farm threshold is `HAUL_DROP_STRIKES` (3, **1571**) or its own
constant. See §5.

### (b) — READ, `a85dec2912:11374-11401`

The escalation exists **as the sibling branch of the code that lacks it.** Not
in another module, not in another system — the adjacent `else if`:

```rust
if !job.carve_attempted && !job.is_access && job.pos.z - feet.z > reach {
    carve_requests.push((feet, job.pos, active.job));          // ← z-GATED
} else if matches!(job.kind, JobKind::Haul { .. })
    && job.stuck_strikes >= HAUL_DROP_STRIKES                  // ← ESCALATES
{
    …  "bastion: churning haul dropped — reservation freed (49.2)"
    haul_drops.push(active.job);
} else {
    job.unreachable = true;                                    // ← CHURNS FOREVER
    churn_events.push((entity, pos.0, feet, reach));
    …  "bastion: job unreachable — claim released"
}
```

**READ:** the first arm requires `job.pos.z - feet.z > reach` — a **vertical**
predicate with **no lateral term**. A target that is laterally unreachable at the
same z never enters it. The second arm requires `JobKind::Haul`. **Everything
else falls to the third arm, which sets a flag, logs, and returns the job to the
pool with no memory that this already happened three times.**

The `Haul` arm's own comment states the general principle in general terms —
*"a HAUL that keeps striking out is DROPPED, not eternally churned"* — and then
the guard scopes it to one kind. **This is exactly the object
`sufficiency-claims-must-name-their-case` describes: a correct rationale frozen
at the scope of the case that motivated it.** The comment names its case
(reserve-at-generation pinning an item), which is what makes the gap checkable
rather than invisible.

### (c) — READ, `a85dec2912:12869-12894` + `3718-3746`

`blocked_regions` has **exactly one producer** (**12886**), inside
`plan_access`'s `None` arm. The recording is sound and edge-triggered. Its
problem is purely **reach**: to get there a job must first have been routed into
`plan_access`, which happens via `carve_requests` — the **z-gated** first arm
above.

> **A laterally-unreachable mine or farm target is structurally incapable of
> being recorded as blocked.** Not unlikely. Incapable — there is no path
> through the code that reaches the recorder.

And the recorder was **built anticipating this fix**. `BlockedRegionInfo.source`
(**3730-3744**), READ:

> *"which mechanism recorded this entry — currently always `"plan_access"` (the
> carve-planner failure site, the only producer) … this field is kept because
> it's cheap, has zero runtime cost, and is the instrument that will answer the
> same question again **the moment a real second producer exists**."*

**The fix's new recorders are that second producer.** The attribution field, the
dedup-by-region check, the deferred-notification design (`notified: bool`,
**3724-3729**, explicitly built so *"ANY code path"* can record without emitter
plumbing) are already in place and already generalised past their one caller.

---

## §3 — EVIDENCE TABLE

The spectrum, recovered end to permanent end. **The point of the table is that
these are the same mechanism at different persistence levels, not five findings.**

| seed / case | attempts | completions | reading | mark |
|---|---|---|---|---|
| **farm till job** | **4** (4 colonists, 4 vantages) | **0, never** | the specimen: released `unreachable` every time, no recovery, against a path whose stated premise is that these resolve by retry | CORPUS |
| **90 holdout** | **9** | **0, never** | 26/27 complete, one cell never does; all 9 releases the same writer, colonist at the same dead-end each time. **≈1.8× the attempts of a normal cell for zero completions** | 5b-TRACE |
| **71 frontier** | — | 5/27 blocks | `access_emissions_max: 3` — emitted three access plans, **never starved**. Kills the "the colony-global bar starves it" reading | CORPUS |
| **66 transient** | — | resolves | contention genuinely resolves here; terrain 27/27 vs board 10 live jobs. **The premise is TRUE for this seed** | 5b-TRACE |
| **61 recovered end** | — | 27/27 @ 450 iters | not stalled, **slow** — completes at 2.5× the default window. Branch A: window artifact | 5b-TRACE |

**Why the table is built this way.** 66 and 61 are in it *because they survive*.
A ledger that lists only the four-attempt farm cell argues that the churn path is
wrong; a ledger that includes 66 and 61 argues the true thing — **the churn
path's premise is correct for a majority of cases and has no exit for the
minority.** That distinction decides the fix: an exit ramp, not a replacement.

The `1.8×-attempts / zero-completions` ratio is the discriminator that makes
persistence *measurable* rather than judged: a cell doing normal work has
attempts proportional to completions; the holdout's attempts grow while
completions stay pinned at zero.

---

## §4 — THE LATERAL-ENTRY DESIGN

Two new recording entry points. **Both are gated on persistence, neither adds a
counter, neither writes to the world.**

### Entry point 1 — the churn else-arm (`a85dec2912:11412`)

```
gate    job.stuck_strikes >= <threshold>   (the counter from §2a, already there)
source  "route_exhausted"
meaning "this job has been claimed and released N times with zero progress and
         no carve plan was ever attempted (the z-gate never fired)"
```

**Distinct from `"plan_access"` by construction:** `plan_access` means *the
planner was asked and refused*. `route_exhausted` means *the planner was never
asked, because the only door to it is a vertical predicate.* Conflating them
would make "is this cell blocked?" a void test of which mechanism fired — the
exact defect `BlockedRegionInfo.source` was added to prevent (**3736-3741**,
READ).

### Entry point 2 — the haul-drop arm (`a85dec2912:11401`)

```
gate    ALREADY EXISTS — stuck_strikes >= HAUL_DROP_STRIKES
source  "haul_strikes_exhausted"
meaning "an item-fetch was abandoned after 3 strikes"
```

**This entry point adds no gate, no counter, and no policy.** The escalation
already fires; it is simply invisible to #55. A colony that drops hauls silently
looks identical to a colony with no hauls to do.

### Threshold — the one open decision

`HAUL_DROP_STRIKES = 3` (**1571**) is tuned for *drop* semantics (freeing a
pinned reservation). The new entries only *record*, which is strictly cheaper, so
a lower threshold is defensible — but **recording at 1 recreates the false-alarm
spam the #55 comment correctly refuses** (§7). Recommendation: **reuse 3**, and
if it proves wrong, the corpus says so via `source` counts without a code change.
Do not invent a second constant to express the same idea.

---

## §5 — THE SIBLING-CALLER CHECK, RUN ON THE DESIGN ITSELF

*Eating the dogfood: the standing review step applied to the two entry points
this packet proposes, before a builder writes them.*

**Census at `a85dec2912`** (comment/doc occurrences excluded by reading each
site, not by pattern):

| population | count | lines |
|---|---|---|
| `blocked_regions.push` (recorders) | **1** | 12886 |
| `blocked_regions.retain` (clearers) | **1** | 5153 |
| `job.unreachable = true` writers | **3** | 11413, 12863, 15564 |
| `job.claimed_by = None` sites | **8** | 8095, 8787, 11285, 11350, 12497, 12999, 14526, 15009 |
| `to_release.push` producers | **26** | (shared consumer — see §10) |
| `haul_drops.push` | **1** | 11401 |
| `churn_events.push` | **2** | 11284, 11414 |

**The check asks: if these two entry points are right, which siblings are also
right and being left out?**

| sibling | verdict | reason |
|---|---|---|
| **11413** churn else-arm | **COVERED** — entry point 1 | the specimen path |
| **11401** haul drop | **COVERED** — entry point 2 | escalates already, records nothing |
| **12863** plan_access `None` | **already covered** | the existing producer (12886) |
| **15564** enclosure sweep | **EXEMPT — named reason** | READ: *"the periodic retry sweep re-tests as the dig opens the shell"* — this flag is **expected** to flip back, and there is a live mechanism that flips it. Recording it would report the colony's own dig progress as a blockage. **This is the one case where "transient" is backed by a named re-tester.** |
| **11284** staged-at-anchor churn | **EXEMPT — named reason** | READ (11279-11282): FR15 scopes this to anchor-staging specifically; a stalled *traveler* is a movement failure, not a blocked *designation*. Recording it would attribute a pathing stall to the target cell. |
| **5153** the clearer | **REVIEW REQUIRED — see below** | |
| the other 6 `claimed_by = None` sites | **NOT REVIEWED** | out of this row's scope; flagged, not claimed |

### ★ The clearer is the finding this check produced

`blocked_regions.retain` (**5153**) is guarded, **5151**:

```rust
&& !self.blocked_regions.is_empty()
```

with a comment (**5122-5140**, READ) recording two prior corrections — an
over-prune and an under-prune — and noting that `blocked_regions.is_empty()` is
*"true the overwhelming majority"* of the time.

> **That "overwhelming majority" is a measurement of a store with ONE producer
> behind a z-gate.** Adding two lateral producers moves `blocked_regions` from
> near-always-empty to routinely-populated, which puts the retain predicate on a
> hot path it has never run on, and re-opens both of the prune bugs its comment
> says were fixed.

**This is a real prerequisite, not a caveat.** The row must include a prune-side
check, or the fix trades an invisible starvation for a visible-but-wrong blocked
list. **The sibling-caller check earned its place in the packet by finding this
before a builder did.**

---

## §6 — THE #55-INTENT FRAMING

The churn-path exclusion comment (**11421-11434**, READ) says:

> *"deliberately NOT hooked here. This is the routine churn path … transient
> congestion that RETRIES and often resolves itself, not a permanent block.
> Firing 'designation is blocked' on every occurrence would be a false-alarm
> spam source; only the carve-planner's genuine 'no route exists' failure …
> is a strong enough signal to surface."*

**Every clause of that is correct.** Seed 66 is congestion that resolves. Seed 61
is slowness that completes. Recording on *every occurrence* would be spam.

The comment's one unstated assumption is that **occurrence count carries no
information** — that a first release and a ninth release are the same event. The
farm cell's 4-of-4 and the holdout's 9-of-9 are the counterexample, and
`stuck_strikes` is the number that separates them, already incremented, one
`else` away.

> **The fix completes a design rather than amending one.** #55 chose to surface
> only signals strong enough to be worth a message. This row does not overturn
> that choice — it supplies the second such signal, using the strength measure
> (#59's persistence counting) that did not exist when #55 was written.

The comment stays. It gets one added clause naming the new threshold and why 1
is still refused.

---

## §7 — GATES

| gate | what it must show | why this gate |
|---|---|---|
| **G1 — column scan** (seed 90 cell) | whether the holdout site is single-surface or multi-layer | The probe caveat's **soundness direction is a property of the error model**: body-width error ⇒ negatives sound; multi-layer collapse ⇒ **both directions unsound**. Until the column is scanned, "laterally unreachable" is an inference from an instrument that may be wrong in the direction that matters. **G1 gates the row's premise, not its implementation.** |
| **G2 — FR15 paired A/B** | the stuck-economy's tuning under the new escalation | Mandatory: a new escalation path **invalidates the stuck-economy's tuning** by construction. Paired A/B, same seeds, both arms. |
| **G3 — corpus exact-match** | zero drift on all 48 seeds **with `source` counts read** | ★ **Not exact-match alone.** Exact-match on the current schema would return GREEN for a fix that never fires — the corpus has no field that reports `blocked_regions` contents. **The row must add the reporting field before it adds the behavior**, per the acceptance framework's own ordering. A GREEN with no named field that moves is a measurement of nothing. |
| **G4 — prune-side check** (§5) | `retain` correctness on a populated store | new; produced by the sibling-caller check |

**G3 is the gate this campaign exists because of.** The colony-global access bar
survived every fan for weeks because no field could see it. Do not repeat that
here: **name the field that moves before writing the code that moves it.**

---

## §8 — BUILDER PROMPT: START-HERE TIER

Per the prompt-craft ordering (START HERE / THEN / REFERENCE-ONLY).

**START HERE** — read these four, at `a85dec2912`, before writing anything:

1. `bastion_jobs.rs:11350-11353` — `stuck_strikes` increment (part a already exists)
2. `bastion_jobs.rs:11370-11420` — the three-arm chain (the z-gate, the Haul escalation, the churn else)
3. `bastion_jobs.rs:12869-12894` — the one existing recorder and its edge-trigger
4. `bastion_jobs.rs:3718-3746` — `BlockedRegionInfo`, especially `source`'s doc

**THEN:**
- `bastion_jobs.rs:5122-5160` — the retain guard and its two prior prune bugs (G4)
- `bastion_jobs.rs:1571` + `9625` — `HAUL_DROP_STRIKES` and the other consumer of `stuck_strikes`
- `bastion_jobs.rs:15558-15566` — the enclosure writer (the exempt sibling; understand *why* it's exempt)

**REFERENCE-ONLY:**
- `readme/INSTRUMENT-PER-ATTEMPT-RECORD-spec.md`
- `bastion-test-evidence/SCENARIO-MAP.md` (probe caveat, error models)
- this packet

**Explicit non-goals for the builder:** no world writes, no new counter, no
change to `plan_access`, no change to the z-gate predicate. **The z-gate is not
the bug** — widening it would send laterally-blocked jobs into a carve planner
that would try to dig at them.

---

## §9 — RISK REGISTER

### R1 — ★ THE STEP-2 CLASSIFICATION IS UNCOMMITTED

The release-reason classification that separates **shape A** (lost the
comparison) from **shape C** (selected, attempted, rejected downstream) is the
load-bearing evidence for verdict #54. At `a85dec2912`:

```
ReleaseReason { Other }          ← ONE variant, the step-1 placeholder
ReleaseReason::  — 26 uses, all Other
TimedOut         — 0 occurrences
```

**Scanned every local and remote ref matching sonnet/5b/arb/attempt/batch/bed/
block-B6: `TimedOut` appears in none of them.** The step-2 variants exist only in
5b's working tree.

> **The evidence the verdict rests on is unversioned.** Per the standing rule
> that evidence lands in the E: worktree and not on someone else's disk, this is
> the highest-priority action item in the packet — **higher than the row itself.**
> Flagged to 5b this session.

Marked **5b-TRACE** throughout; those rows are *reported*, not *verified by me*.

### R2 — `to_release`'s 26 producers

One of the release sites is a shared consumer fed by 26 producers, each with its
own reason invisible at the consumer. Any per-site reasoning about *which*
release fired must be done at the producer. This already caused one correction to
the instrument spec.

### R3 — threshold tuning is a behavior change under FR15

Even though the new entries only *record*, `blocked_regions` is read by the
job-selection path via the `blocked_by` machinery. **A recorder that populates a
store somebody reads is not report-only.** G2 is not optional and the builder
must confirm the read path's behavior on a populated store.

---

## §10 — WHAT THIS PACKET DOES NOT CLAIM

- It does not claim the lateral-unreachability *diagnosis* is proven. **G1
  gates it.** If the holdout column is multi-layer, the probe's negatives are
  unsound and the specimen must be re-read before the row proceeds.
- It does not claim the other 6 `claimed_by = None` sites are correctly excluded.
  They were **not reviewed** — stated as a gap, not resolved by silence.
- It does not claim exact-match will detect this fix. **§7/G3 says the opposite**
  and requires a new field first.
