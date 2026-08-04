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

#### ★★ THE CHECK THAT COULD HAVE KILLED PART (a) — RUN, and it HELD

`stuck_strikes` is only a *persistence* measure if it survives re-claim. **If it
reset whenever a new colonist picked the job up, it would count one colonist's
run of bad luck — and the farm specimen was FOUR DIFFERENT COLONISTS**, so it
would never have reached any threshold. Part (a) rests entirely on this.

**Census at `a85dec2912`:**

| population | count | verdict |
|---|---|---|
| mutations (`.stuck_strikes =`) | **1** — line **11353**, the increment | monotonic |
| struct initialisers (`stuck_strikes: 0,`) | **13** | all at **job creation** |
| resets on an existing job | **0** | none exist |
| re-claim sites (`claimed_by = Some`) | 2 — **13789**, **16152** | **neither touches `stuck_strikes`** |

> **`stuck_strikes` is monotonic for the life of the job and accumulates ACROSS
> colonists.** It already measures persistence of the **JOB**, which is exactly
> the quantity the invariant names — not persistence of an attempt.

**And it retro-validates both the specimen and the threshold.** The farm cell was
claimed and released four times through this path (the releases carry the
**11413** log line, which sits downstream of the **11353** increment in the same
block), so **its `stuck_strikes` reached ≈4 — above `HAUL_DROP_STRIKES` (3).**
The recommended reuse-3 threshold would have fired on the one specimen we have
the most detail about. *Marked as an inference from the release path, not a
direct reading: the corpus does not report the field (see below).*

**★ One more instrument gap, found by the same check:** scanning
`wave19_FULL.json` for `strike` / `starv` / `churn` returns **nothing**. The
number part (a) reads is **not in the corpus.** It is already on
`BastionJobInspect` (**lib.rs:2242**), so exposing it is plumbing that exists —
but until it is emitted, the farm specimen's strike count stays an inference.
**Add `stuck_strikes` at the specimen cells to Row A's instrument list.**

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
| **90 holdout** | **9** | **0, never** | 26/27 complete, one cell never does; all 9 releases the same writer, colonist at the same dead-end each time. **≈1.8× the attempts of a normal cell for zero completions.** ★ **CAUSE UNKNOWN — amended by G1:** the site is multi-layer, the probe is blind to it, and "genuinely unreachable" is **no longer supported**. The *behavioral* reading (9 attempts, 0 completions, never recovers) is untouched and is the only reading the row needs | 5b-TRACE + G1 |
| **78 chop** | — | 0 (`log_sum` 0) | ★ **NEW, 4th costume.** `chop_cleared`/`log_sum` fail; 2 travel timeouts **both against the same repeated target**; release accounting **exact** — 29 completed + 2 timed-out + 2 removed-externally = 33, **`Other: 0`**. Same TimedOut/churn path as Farm and Mine ⇒ **three job kinds, one mechanism** | 5b-TRACE |
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

### ~~Entry point 2 — the haul-drop arm~~ — ★ WITHDRAWN, see below

**Proposed, then killed by G4's follow-through. The withdrawal is kept visible
because the reason generalises.**

The proposal was to record `"haul_strikes_exhausted"` at **11401**, on the
grounds that the escalation already fires there and is simply invisible to #55.
**That reasoning was right about the gap and wrong about the instrument.**

**READ, `a85dec2912:3717`** — the store's semantic unit:

> *"one **designation** Region the auto-access planner gave up on"*

**READ, `a85dec2912:8507-8512`** — a `Haul` job's `pos` is `cell`, **the loose
item's location**. It has no designation. Meanwhile the recorder is keyed by
`board.designated.iter().find(|r| r.contains_point(to))` (**12881**), so a haul
entry would resolve one of two ways:

| case | result |
|---|---|
| item on open ground (the normal case) | **no region found → records nothing.** A `source` count that is silently conditional on an unrelated fact, with a denominator nobody can see |
| item happens to lie inside *any* painted box — including a Stockpile paint, which **does** join `designated` (**4516**, pushed before the Stockpile branch) | **records "this designation is blocked"** — and the chat drain (**15397**) tells the player *"A designation is blocked"* about a farm or mine that is **fine**. The haul failed; the designation didn't. |

> **★ The second case is worse than the first. It is not a gap — it is an active
> mis-attribution that reaches the player.** A fetch failure would be reported as
> a blocked designation, chosen by whichever box the item happened to be
> standing in.

**Disposition:** the haul-drop site is **EXEMPT with a named reason** — *the
store's unit is a designation and a haul drop is an item-level failure with no
designation to blame.* It joins the enclosure sweep and anchor-staging in §5's
exempt list.

**But the gap it named is real and stays on the books.** A colony that silently
drops hauls is indistinguishable from a colony with no hauls to do. That needs a
**counter keyed on the item or the job**, not a `blocked_regions` entry — a
different instrument, a different row. **Filed, not solved.**

### ★ What survived — entry point 1 is structurally sound on the same test

Applying the identical check to `route_exhausted`: `designate()` creates its jobs
in a triple loop bounded by `region.min..=region.max` (**4552-4554**), so **a
mine or farm job's `pos` lies inside its own designation by construction.** The
region lookup at the churn site resolves, the entry means what the struct says it
means, and the chat line names a designation that is genuinely not being
completed.

**The two entry points looked symmetric and were not.** The test that separated
them — *does the store's semantic unit fit this producer?* — is one question,
asked once, and it should be asked of every future producer before it is
written.

### Threshold — the one open decision

`HAUL_DROP_STRIKES = 3` (**1571**) is tuned for *drop* semantics (freeing a
pinned reservation). The new entries only *record*, which is strictly cheaper, so
a lower threshold is defensible — but **recording at 1 recreates the false-alarm
spam the #55 comment correctly refuses** (§7). Recommendation: **the value 3**,
and if it proves wrong the corpus says so via `source` counts without a code
change. Do not invent a second constant to express the same idea.

**★ RATIFIED WITH AN AMENDMENT (Fable): the VALUE stands, the NAME does not.**
A constant called `HAUL_DROP_STRIKES` silently gating **farm and mine**
escalation would be a label-vs-content trap **planted knowingly** — the exact
defect class this campaign has spent two days cataloguing, and the one that has
now bitten this packet six times.

**The form:** one shared constant, **renamed to its actual scope**
(`PERSIST_ESCALATE_STRIKES` or similar), both the `Haul` arm and the new arm
citing it, and **the declaration comment naming BOTH consumers.** One value,
corpus-correctable, and a name that tells the truth. If the rename's blast radius
turns out to be nontrivial, an alias const carrying the same guarantee is
acceptable; **a bare reuse under the haul name is not.**

**★ The same coupling flag applies one level down, to `stuck_strikes` itself
(Fable).** Part (a) gives it a **second consumer**: its documented one is arrival
tolerance (**9625**), and escalation makes its reset semantics load-bearing for
two mechanisms at once. **The write site (11353) gets a named-case comment
declaring both consumers**, so that the first person to adjust a reset for one of
them cannot silently retune the other. This is the write-site/read-site law
applied *prospectively, at design time* — the cheapest it will ever be.

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
| **11401** haul drop | ~~COVERED~~ → **EXEMPT — named reason** | ★ **Flipped by G4.** A haul's `pos` is a loose item's cell (**8510**), and the store's unit is *"one designation Region"* (**3717**). Recording here either finds no region (silent, unknown denominator) or blames whichever painted box the item was standing in — **a false "designation is blocked" chat line for a designation that is fine.** The invisibility it names is real and is re-filed as its own row needing an item-keyed counter. See §4. |
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
| **G1 — column scan** (seed 90 cell) | whether the holdout site is single-surface or multi-layer | ★ **RUN by 5b. Result: MULTI-LAYER — the negative is UNSOUND.** Verdict below. |

### ★★★ G1 VERDICT — RUN, and it came back AGAINST the diagnosis

**5b's multi-column dump, seed 90, `x=[17973,17989] y=[9261,9263] z=[330,345]`:**

| column | terrain | probe's `column_height_near` |
|---|---|---|
| anchor `(17973,9261)` | solid 330–333, **open 334–337**, solid 338+ | **353** |
| dead-end target `(17989,9263)` | solid 330–333, **open 334–335**, solid 336–345 | — |

**The probe's "ground" at z=353 is real and irrelevant.** There is a subterranean
gallery ~20 blocks below it with an open walkable band, and `column_height_near`
returns only the **topmost** solid layer, so it cannot see any of it. This is the
**column-collapse** error model, not the body-width one — and per the pre-stated
rule, **collapse makes the negative unsound in BOTH directions.** Same signature
as seeds 71 / 54 / 61, now extended to 90.

**Pre-stated branch, honoured:** *multi-layer ⇒ negative UNSOUND, caveat
everywhere.* Taken.

#### What this KILLS

**The specimen-level diagnosis.** "Seed 90's holdout is genuinely laterally
unreachable" rested on the probe, and the probe is blind at exactly this site.
The holdout's cause is now **UNKNOWN** — it may be a real obstruction, or a
router that cannot find a path through a multi-layer gallery. **The evidence
table's seed-90 row is amended accordingly.**

#### What this does NOT touch

**Part (c)'s structural claim is a CODE READ, not a probe reading.**
`blocked_regions` is reachable only through the z-gated carve path (**11370**),
and that predicate has no lateral term. That is true by inspection of the source
and does not depend on any terrain instrument. **(a), (b) and (c) all stand
unchanged.**

#### ★★★★ And the bad news for the diagnosis is GOOD NEWS for the design

> **A reviewer with full terrain dumps, a debug binary, and unlimited time could
> not determine why this job fails.** If *we* cannot establish the cause, the
> colony certainly cannot — and **any fix that requires knowing the cause is
> unbuildable.**

The invariant (#54) asks the system to notice *persistent failure*, **not to
diagnose it**. G1 is the strongest evidence yet that a **cause-agnostic
persistence signal is the right instrument**: `stuck_strikes` counts attempts
without caring whether the obstacle is rock, a gallery, or a pathfinder bug.

**G1 therefore redirects rather than fails the row** — exactly as the batch
runbook required of every item.

#### ★★ THE ONE REAL AMENDMENT G1 FORCES: the message must not claim a cause

The existing chat line (**12895-12900**) reads:

> *"A designation is blocked — **obstruction** at (x, y, z) can't be reached."*

**That asserts a terrain fact.** G1 has just shown the mechanism behind it cannot
establish one — the incumbent producer has the same exposure, and has been
shipping this wording. For the new producer it would be the **same defect that
killed entry point 2**: a false statement to a human, differing only in which
coincidence supplies it.

**`route_exhausted` must carry its own message, stating only what is
established:**

```
"Colonists have repeatedly failed to reach a designation at (x, y, z)."
```

A **behavioral** claim — which is precisely what `stuck_strikes` measures — with
no causal attribution. **The distinct-`source` design already makes this free:**
two producers, two messages, and `source` tells the drain which to emit.

**Filed against the incumbent, not fixed here:** `plan_access`'s wording
overclaims by the same argument. Out of this row's scope, stated rather than
silently inherited.
| **G2 — FR15 paired A/B** | the stuck-economy's tuning under the new escalation | Mandatory **for ROW B only** — see R3. A new escalation path **invalidates the stuck-economy's tuning** by construction. Paired A/B, same seeds, both arms. **Row A does not trip this**: the census in R3 proves `blocked_regions` has zero behavior consumers. |
| **G3 — corpus exact-match** | zero drift on all 48 seeds **with `source` counts read** | ★ **Not exact-match alone**, and **not for the reason I first gave.** See the G3 verdict below — the corpus *does* report `blocked_regions`, at the **wrong coordinates**. |

### ★ G3 VERDICT — RUN against `wave19_FULL.json` (48 seeds, on disk, zero new runs)

**I wrote:** *"the corpus has no field that reports `blocked_regions` contents."*
**Checked. False — there are six, and one of them is live:**

| field | distribution across 48 |
|---|---|
| `b5_55_blocked_by` | **`None` × 48** |
| `b5_55_names_blocker` | `False` × 48 |
| `b5_55_notified_once` | `False` × 48 |
| `b5_55_clears_on_cancel` | `True` × 48 |
| `b5_ch_base_blocked_by` | `None` × 45, **3 real cells** |
| `b5_ch_base_blocked_sources` | `[]` × 45, **`['plan_access']` × 3** |

**Two things follow, and they point opposite ways.**

**(1) The attribution mechanism is PROVEN END-TO-END.** `source` reaches the
corpus and discriminates: three seeds carry `['plan_access']`. **A second
producer named `route_exhausted` will therefore be visible and separable in the
fan on day one** — no new plumbing, no new accessor. This is the strongest
single piece of evidence that Row A is small.

**(2) The conclusion survives, with a sharper reason.** The reporting fields are
**queried at fixed probe cells** — `bastion_blocked_by(buried_pos)` (`main.rs:4201`)
and `bastion_blocked_by(trapped_cell)` (`main.rs:5090`), plus the chop base.
**Row A's entries land on mine/farm designation regions, which no corpus field
queries.**

> **The store is not unreported. It is reported AT THE WRONG COORDINATES.**
> `b5_55_blocked_by` would stay `None × 48` through a Row A that works perfectly —
> the null it holds is a null about a *different cell*.

**This is the constant-because-GATED class again, one level out:** four fields
constant across 48 seeds, and the constancy is a property of *where the probe
looks*, not of what the colony did.

**What Row A must add — named concretely, one line:**

- **`bastion_blocked_regions_count()` in the b5 corpus output.** ★ **The accessor
  already exists** (`server/src/lib.rs:3367`) and the harness already calls it in
  the b55 scenario (`main.rs:4190`, `4203`). It is simply **not emitted in the b5
  fan's output.** Colony-wide, coordinate-free, and moves the moment any producer
  fires.
- **plus `blocked_sources` at the specimen cell** (seed 90's holdout, the farm
  corner) — so the fan distinguishes *which* producer fired, not merely that one did.

**Pre-registered, before any code:** `b5_55_blocked_by` stays `None × 48`
(different cell — **if it moves, something is wrong**); the new count field goes
non-zero on the affected seeds and stays `0` on the clean ones;
`b5_ch_base_blocked_sources` keeps its `['plan_access'] × 3` exactly. **Any drift
in that last one indicts the change, not the colony.**
| **G4 — prune-side check** (§5) | `retain` correctness on a populated store | new; produced by the sibling-caller check. **★ RUN — read-only, source-decidable. Verdict below.** |

### ★ G4 VERDICT — the prune is CORRECT for entry point 1, *conditionally*

**READ, `a85dec2912:5151-5157`:**

```rust
if let Some(j) = &job && !self.blocked_regions.is_empty() {
    self.blocked_regions.retain(|b| {
        !b.region.contains_point(j.pos)
            || self.jobs.values().any(|other| b.region.contains_point(other.pos))
    });
}
```

Keep `b` unless *(b covers the removed job)* **and** *(no remaining job lies
inside b)*. That is exactly the stated intent — prune only when the region is
empty of jobs — and it holds on a populated store, because the predicate is
per-entry and never depended on the store being small. **The `is_empty()`
early-out is a performance guard, not a correctness one**; a populated store
simply pays the scan it was always written to pay.

**The condition:** this is only true if the new producer records **the same
Region value** the existing one does — the designated AABB from
`board.designated.iter().find(|r| r.contains_point(..))`. **A point-region would
break three things at once:** the `already_recorded` exact-Region dedupe
(**12884**) would stop collapsing duplicates, the prune's `contains_point(other.pos)`
would almost never find a sibling job and would over-retract, and
`blocked_sources` would return two entries for one target — the precise hazard
its own doc warns about (**5276-5283**).

> **So G4 converts from a gate into a one-line design constraint:
> `route_exhausted` MUST reuse the designated-region lookup.** That is now the
> single most important instruction in the builder prompt, and it is why §8
> lists `3718-3746` in the START-HERE tier rather than as reference.

**What G4 does *not* clear:** the over-prune/under-prune comment describes bugs
found and fixed at n≈0 population. The predicate is *correct*; whether the
resulting player-facing behavior is *sensible* at real population — several
regions blocked, entries retracting as unrelated jobs complete — is a UX
question Row A's corpus field will answer with data instead of argument.

**G3 is the gate this campaign exists because of.** The colony-global access bar
survived every fan for weeks because no field could see it. Do not repeat that
here: **name the field that moves before writing the code that moves it.**

---

## §8 — BUILDER PROMPT: START-HERE TIER

Per the prompt-craft ordering (START HERE / THEN / REFERENCE-ONLY).

> **★ Scope: this tier serves ROW A (report-only) — see R3.** Row B (the
> escalation) is a separate row and does not start until Row A's corpus field
> can see the phenomenon Row B changes.

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

### R3 — ★ CORRECTED: the recording half IS report-only. Census run, claim refuted.

**R3 originally read:** *"even though the new entries only record,
`blocked_regions` is read by the job-selection path via the `blocked_by`
machinery — a recorder that populates a store somebody reads is not
report-only."*

**That was inferred from an accessor's NAME, not from its callers.** I ran the
census. **Every consumer of `blocked_regions` at `a85dec2912`, whole tree:**

| consumer | site | class |
|---|---|---|
| `blocked_by()` | `server/src/lib.rs:2243`, `sys/msg/in_game.rs:696` | **report** — both build `BastionInspectKind::Job(BastionJobInspect{..})`, the inspector struct |
| `blocked_by()` / `blocked_sources()` | `server/src/lib.rs:3384`, `3395` | **report** — the `bastion_*` harness accessors |
| `bastion_blocked_regions_count()` | `server/src/lib.rs:3367` | **report** |
| all harness call sites | `bastion-harness/src/main.rs` ×8 | **report** — corpus fields |
| chat drain | `bastion_jobs.rs:15397` | **report** — player-visible message |
| `retain` ×2 | `bastion_jobs.rs:4926`, `5153` | **prune** — write-side, not a behavior read |
| `already_recorded` dedupe | `bastion_jobs.rs:12884` | **internal to the recorder** |

> **There is no behavior consumer. `blocked_regions` is a pure report store.**
> Not "mostly", not "as far as the row is concerned" — the grep across the tree
> returns inspector, harness, and chat, and nothing else.

**This is the same defect the campaign has now hit six times: I characterised
`blocked_by` from its name.** A function called `blocked_by` sounds like a
predicate the scheduler consults. It is a field on an inspect struct.

### ★★ THE CORRECTION SPLITS THE ROW — and the packet's own G3 already ordered it

The invariant's three clauses do **not** carry the same risk, and I had them
bundled:

| clause | mechanism | behavior risk |
|---|---|---|
| (i) **notice** | read `stuck_strikes` (already incremented) | **none** — a read |
| (iii) **say so** | the two new `blocked_regions` entries | **none** — proven above |
| (ii) **stop paying for it** | actually drop/bench the job as the `Haul` arm does | **full** — changes what work the colony attempts |

**ROW A (report-only): (i) + (iii).** Reads a counter that already exists,
records into a store nothing consults for behavior, adds the corpus field. **No
FR15 exposure. G2 does not apply.** It can land on its own.

**ROW B (behavior): (ii).** The escalation. **Full FR15 paired A/B**, because a
new escalation path invalidates the stuck-economy's tuning by construction.

> **★ The packet's own G3 already demanded this ordering — "ship the reporting
> field before the behavior" — and I wrote it without noticing it implied a row
> split.** The census makes the split free: Row A is exactly the instrument Row B
> needs, and it costs no behavior risk to build.

**And it fixes a real problem with the row as originally scoped:** Row B's
paired A/B has nothing to measure without Row A. The corpus cannot currently see
`blocked_regions` contents at all, so an escalation row landing first would be
gated by a fan that is blind to the thing it changes — the exact failure that
let the colony-global access bar survive for weeks.

**Residual risk on Row A, stated rather than dismissed:** the chat drain
(**15397**) fires a player-visible message per newly-recorded region. Moving
`blocked_regions` from near-always-empty to routinely-populated therefore
changes message volume. That is a **UX** exposure, not a sim-determinism one —
but it is the false-alarm spam the #55 comment explicitly refuses (§7), so the
threshold decision in §4 is load-bearing for Row A, not just Row B.

---

## §10 — WHAT THIS PACKET DOES NOT CLAIM

- It does not claim the lateral-unreachability *diagnosis* is proven. **G1
  gates it.** If the holdout column is multi-layer, the probe's negatives are
  unsound and the specimen must be re-read before the row proceeds.
- It does not claim the other 6 `claimed_by = None` sites are correctly excluded.
  They were **not reviewed** — stated as a gap, not resolved by silence.
- It does not claim exact-match will detect this fix. **§7/G3 says the opposite**
  and requires a new field first.
- It does not claim the packet was right on first draft. **R3 was written from an
  accessor's name and refuted by the census I ran an hour later** — the
  correction is left in place with the original wording quoted, because a
  packet that hides its own reversals teaches nobody what to re-check.

## §11 — SURVIVALS (claims that were checked and HELD)

*Recorded because a ledger that carries only bad news stops being run.*

- **`blocked_regions` has exactly one producer.** Checked against the whole tree,
  not just the jobs file. Held.
- **`BlockedRegionInfo.notified` genuinely supports emitter-less producers**
  (**3724-3729**) — the new entry points need no chat plumbing. Held; this is
  why Row A is small.
- **The `already_recorded` dedupe keys on exact `Region`** (**12884**), and
  `blocked_sources`' doc already warns that two producers pushing *different*
  Region values for the same target will not collapse (**5276-5283**). **Checked
  as a hazard for the new producers and it is REAL but ALREADY DOCUMENTED** — the
  builder inherits a written warning rather than a surprise.
- **The enclosure sweep's exemption is backed by a live re-tester**, not by
  assertion (**15562-15564**). Held — this is the one "transient" claim in the
  file with a named mechanism behind it.
