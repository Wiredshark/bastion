# INSTRUMENT SPEC — per-attempt claim outcome record (ARB-ATTEMPT-01)

**Status: SPEC ONLY, unbuilt.** Authored 2026-08-04 by the Opus investigation
lane. **Gates DECISIONS #53's ARB-STARVATION row**, whose discriminator was
shown to be *provably* underivable from the current corpus.

## Why the existing instruments cannot answer the question

`mine_cell_diag` aggregates **per CELL**. #53's question — *why does seed 71
never recover while seed 66 does, at identical contention ratio 1.000* — is
**per ATTEMPT**. Worse, every existing per-cell field is **coupled to the
outcome**:

| field | why it cannot discriminate |
|---|---|
| `unreachable` | "all six faces currently solid" — tracks dig progress |
| `timeouts_on_this_cell` | an uncompleted cell is re-claimed and re-timed-out |
| `times_offered` (= `claims_by_pos`) | same coupling, and it counts claims GRANTED, not offers made |
| `starvation_cycles` | counts cycles unclaimed — grows when work isn't finishing |

**Normalising helps and is not enough:** timeouts-per-claim orders the seeds
(66 = 0.33; 52 = 0.64; 61 = 0.60; 90 = 0.75; 54 = 0.74; 71 = 0.77) but **does
not separate them** — seed 52 fully mined at 0.64, above seed 61's 0.60 at one
block short. **More reading will not produce the discriminator.**

## What makes this cheap: the outcomes are ALREADY structurally distinct

**One grant site. Seven release sites.** No inference is required — each release
is already its own code path and can stamp its own reason.

**GRANT (open an attempt)** — `bastion-server/src/bastion_jobs.rs`, the
arbitration grant where `job.claimed_by = Some(*uid)` sits immediately above
`board.total_claims += 1`, and the following block already does
`cycles_since_last_claim.insert(pos, 0)` and bumps `claims_by_pos`. **That block
is the natural hook — it is already the "this is the attempt" marker, by its own
comment.**

**RELEASE (close an attempt with a reason)** — each `job.claimed_by = None`:

| site | apparent reason (verify before stamping) |
|---|---|
| ~11152 | carve / unreachable pipeline release |
| ~11217 | (second arm of the same region) |
| ~12364 | owner-entity release — "still claimed by us, free it" |
| ~12855 | stuck-claim release so the colonist can work its own rescue |
| ~14380 | (to identify) |
| ~14863 | despawn/leak guard — "so work never leaks" |

**Completion is the seventh outcome** and does not appear in this list — find
where a finished job is removed rather than released, and stamp it there.

> **Cite these by SYMBOL after re-reading, not by these line numbers** — this
> file is hot and the numbers rot. They are given as a starting index only.

## ★★★ SPEC CORRECTION (5b, verified 2026-08-04) — the STRUCTURE above is wrong

**The count was right; the shape was not.** Verified counts:

```
claimed_by = None sites   :  7    (as stated)
to_release.push producers : 26    (NEVER MENTIONED ABOVE)
to_release: Vec<specs::Entity>    — carries the entity, NOT a reason
```

**One of the seven is a shared `to_release` DRAIN fed by 26 producers.** So
stamping at the seven assignment sites yields six real reasons plus **one bucket
labelled "released via the sweep" covering 26 distinct causes.** The original
text asserted "seven release sites, each stamping its own reason" — **written
from a grep of the assignment sites without reading what fed them.** 5b caught
it before it cost a build.

### The corrected implementation — carry the reason AT THE PUSH

```rust
Vec<specs::Entity>  →  Vec<(specs::Entity, ReleaseReason)>
```

Each producer already knows its reason from its own surrounding lines. **This is
26 one-word edits behind one enum, not a research pass.**

**Two steps, so the build is never broken:**
1. **Introduce `ReleaseReason` with an `Other` variant; change the type; push
   `Other` everywhere.** Compiles immediately, zero behavior change, trace works
   end to end with one coarse bucket.
2. **Replace `Other` producer by producer** as each site is read. Incremental,
   interruptible, each replacement independently correct.

## ★★★★ STEP 2 IS NOW LOAD-BEARING (2026-08-04, after item 1's first data)

**The question moved underneath this spec.** A third shape appeared that the
original A/B split did not contemplate:

| shape | meaning | **release reasons it carries** | fix lives at |
|---|---|---|---|
| **A** | lost the comparison | **timeout / preemption** (competition) | the scheduler |
| **B** | never evaluated | *(no attempts at all)* | the scheduler |
| **C** | **selected, attempted, REJECTED DOWNSTREAM** | **stance / material / work-start** (gates) | **the REJECTION SITE — a different owner entirely** |

**A and C both show attempts that fail. ONLY THE REASON SEPARATES THEM.**

> **So a trace carrying only `ReleaseReason::Other` cannot answer the question
> this instrument now exists to answer.** Step 1 separates *completed* from
> *released-via-sweep* — sufficient for "attempts exist vs zero attempts,"
> insufficient for A vs C.

**And the stakes are OWNERSHIP:** if the shape is C, the row moves off
arbitration entirely.

**Scoping that keeps step 2 an afternoon, not a research pass: classify ONLY the
producers that actually FIRE on seeds 71 and 66.** Run with `Other`, observe
which sites appear, label those. **The other producers can stay `Other`
indefinitely** — the enum's own comment already makes a residual `Other`
non-benign, so nothing is silently lost.

> **★ Note what happened to this spec: it was correct when written and became
> insufficient without changing.** The acceptance clause below still holds for
> the question it was written against — **a spec's sufficiency is dated, like
> any other sufficiency claim.**

> **★ Step 1 ALONE satisfies this spec's acceptance clause** — it separates
> *completed* from *released-via-sweep* from the two direct paths, which is
> enough to answer **shape A vs shape B** (attempts exist and fail vs zero
> attempts recorded). That is the trace's first deliverable, so **step 1
> unblocks the batch and step 2 refines it.**

**Lesson for this document's own class:** a spec may assert a COUNT it verified
and a STRUCTURE it did not. **The count was checkable by grep; the structure
required reading the consumer.** Same family as everything else in this
campaign — and the builder refusing to implement against it is what made the
error cheap.

## The record

Per attempt, append one row:

```
{ pos, uid, cycle_granted, cycle_closed,
  outcome: Completed | TimedOut | Preempted | ReleasedStuck | Despawned | ...,
  progress_at_close }
```

**Report-only. Never gates `pass`. No world writes** — same contract as the
task #59 counters it complements (`starvation_cycles` is explicitly
*"Report-only, never gates `pass`, no world writes"*).

## Acceptance — pre-stated, per the framework

**The instrument is accepted iff it distinguishes seeds 71 and 66**, which are
deterministic, already in the standing 48-seed fan, and reproduce exactly across
independent fans:

- **seed 66** — 27/27 mined, 27 claims, 9 timeouts, contention ratio 1.000
- **seed 71** — 5/27 mined, 22 claims, 17 timeouts, contention ratio 1.000

**If the per-attempt outcome distribution is the same shape for both, the
instrument has not answered the question and the row needs a different
approach.** That is a real possible outcome and must be reportable as one —
**a null here is information, not failure.**

## ★ Traps this instrument must not repeat (all found 2026-08-04)

1. **No sentinel inside the valid range.** `tool_factor`'s `.unwrap_or(0.0)`
   put "could not measure" *below the metric's own 1.0 floor* and it was
   counted as a real failure for weeks. Use `Option`, or a value outside the
   domain.
2. **Zero must not be ambiguous.** `starvation_cycles: 0` means both "never
   starved" and "never open/unclaimed" — per the hook's own doc. **Any
   aggregate over these records must exclude the never-attempted case
   deliberately, not silently.**
3. **Do not name a field for a CONCLUSION.** `unreachable` means "six faces
   currently solid" and three separate readings took it as a pathing verdict.
   **Name the measurement** (`faces_open`, `cycle_closed`), not the inference.
4. **Emit at one level.** A recursive dump of the corpus found **161 leaf paths,
   only 68 top-level** — 93 fields invisible to a name scan, which is how a live
   access-emission counter went unnoticed while three of us said it didn't
   exist. **Do not bury this record inside an unrelated parent.**

## Not in scope

**No fix design.** DECISIONS #53 gates arbitration changes on (a) this
discriminator, (b) a prior-art survey (aging / priority escalation / cooldown
are textbook starvation cures), (c) an FR15 paired A/B — an arbitration change
re-rolls the whole colony economy. **This spec delivers (a)'s prerequisite and
nothing else.**
