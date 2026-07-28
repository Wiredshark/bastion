# APEX-T7.1 — prediction-state boundary: PROPOSAL (v1)

**Status: PROPOSAL. Not approved, and not approvable by its author.**

T7.1 is a decision row whose deliverable is a *reviewed* boundary. This
document is the thing to review, not the review. Nothing in T7.2–T7.5
may start on it until it is ruled on, and the tier spec is explicit that
an unapproved boundary is a hard blocker rather than a soft one — the
cost of discovering it was wrong is a rewrite of four rows.

Authored by Builder Opus 5 on `bastion/apex-t34` @ `5b988cbaa7`, from
live reads at that tip. Symbols cited were read, not recalled.

Each decision below is stated as **a rule a test could check**, per the
row's deliverable requirement, followed by what it costs and what the
alternative would have been. Where I do not think a builder should be
the one deciding, the decision is marked **RULING NEEDED** and the
options are laid out rather than resolved.

---

## Decision 1 — Predicted components and replay-legal transitions

`JoinData` (`common/src/states/behavior.rs`, the struct beginning at
`character`) carries **38 fields**. They are not one kind of thing, and
the whole tier depends on separating them.

**Proposed rule.** A `JoinData` field is a **transition input** iff a
replayed frame's output can differ when it differs. Everything else is
**ambient access** and must be read from authority at replay time, never
from history.

Applying that rule to the live struct:

**Transition inputs (predicted, stored in history):**
`character`, `character_activity`, `pos`, `vel`, `ori`, `dt`, `time`,
`controller`, `inputs`, `energy`, `physics`, `mount_data`,
`volume_mount_data`, `stance`.

**Ambient access (read from authority at replay, not stored):**
`scale`, `mass`, `density`, `body`, `health`, `heads`, `inventory`,
`stats`, `skill_set`, `active_abilities`, `ability_map`, `msm`, `combo`,
`alignment`, `terrain`, `melee_attack`, `updater`, `id_maps`,
`alignments`, `prev_phys_caches`, `bodies`,
`constructed_ladder_traversal`.

**Testable form.** A replay harness that mutates one ambient field
between the original run and the replay must produce the same predicted
output; mutating any transition input must produce a different one. That
is a single parameterised test over 38 cases, and it is the test that
catches a field classified by intuition rather than by behaviour.

**The uncomfortable part, stated rather than hidden.** `health`,
`energy` and `inventory` are on opposite sides of a line that is not
obvious. `energy` is a transition input because ability entry compares
against it *within the frame*; `health` is ambient because death is
authoritative and a predicted frame must never decide it; `inventory` is
ambient because item state is another authority's and predicting an
equip would be predicting a server decision. **I am not confident about
`energy`**, and the cost of being wrong is that every ability entry
becomes mispredicted under latency. It is the first thing I would want a
second reader on.

`updater` (`LazyUpdate`) deserves its own note: it is not state at all,
it is a *write channel*. A predicted frame that uses it is emitting a
side effect, which makes it Decision 4's problem rather than Decision
1's. **Proposed rule: `LazyUpdate` is unavailable during replay** — not
"discouraged", unavailable, so a predicted frame physically cannot
queue a component insertion.

---

## Decision 2 — Terrain, dynamic colliders and environment revision

**What the world revision must be.** A replayed frame reads `terrain`
(`&TerrainGrid`) and, through `T5.4`, a weather snapshot.

**Proposed rule.** A history entry stores the **identity** of the world
revision it was predicted under — never a copy of the terrain. On
replay, if that revision is not still available, the frame is **not
replayable** and the client snaps. It does not replay against current
terrain.

**Testable form.** Unload a chunk the history depends on, then replay:
the result must be the fallback, not a frame computed against the new
terrain. Same shape as `T5.4`'s `PredictionWindSourceV1::Unavailable`,
and for the same reason — a plausible substitute is what a caller
silently uses.

**Does a chunk unload invalidate history?** Yes, and only for frames
that actually read that chunk. **Proposed rule:** a history entry
records the set of chunk keys its physics query touched; an unload
invalidates exactly the entries naming that key. Invalidating all
history on any unload is simpler and would make prediction useless at a
chunk boundary, which is where players spend most of their time moving.

**Cost, stated honestly.** Recording touched chunk keys per frame is not
free, and I have not measured it. If it proves too expensive, the
fallback is to invalidate history whose *position* lies within one chunk
of an unloaded key — coarser, cheap, and still not "invalidate
everything".

---

## Decision 3 — Carried, ridden and mounted ownership

**This cannot be answered "it does not happen".** `common/src/mounting.rs`
forces updates today, and `JoinData` carries both `mount_data`
(`Is<Rider>`) and `volume_mount_data` (`Is<VolumeRider>`).

**Proposed rule.** A local player who is a rider or volume-rider is
**not predicted at all**. Their position is a function of a mount whose
state is another authority's, so predicting it is predicting that
authority. Entering or leaving a mount **terminates the current
prediction history** rather than continuing it.

**Testable form.** With `Is<Rider>` present, the prediction path must
produce no predicted frame; the transition into and out of a mount must
leave history empty rather than shortened.

**Why not the alternative.** Predicting the rider against a *predicted*
mount is technically possible and is how this goes wrong slowly: it
works in testing, where mount and rider are the same client's problem,
and diverges in play, where they are not. Refusing is cheap now and
expensive to retrofit.

**RULING NEEDED — carried entities.** A player *carrying* something is
less clear than one riding it. I propose treating carry the same as
mount (not predicted), but the gameplay cost is higher: carrying is
common and mounting is not, so a no-prediction rule for carriers is felt
by more players more often. This is a gameplay/latency trade, not a
determinism question, and it is not mine to make.

---

## Decision 4 — Predicted side-effect scope

`T7.4`'s deduplication depends *entirely* on this answer, so it is
stated as a closed list rather than a principle.

**Proposed rule — three classes, and the type should make them
distinguishable rather than the documentation asking:**

1. **Predictable, no identity.** Movement sounds, footfall particles,
   local animation state. May be emitted by a predicted frame and are
   never deduplicated, because a duplicate is imperceptible and a
   missing one is felt.
2. **Predictable, with identity.** Ability activation effects that carry
   a `CommandId`. May be emitted by a predicted frame, and `T3.5`'s
   command identity is what deduplicates them when authority arrives.
   This is the class `T7.4` exists for, and it is only tractable because
   `T3.5` already built the identity.
3. **Authority-only.** Anything that changes another entity, persists,
   or is observable by another player: damage, item transfer, block
   changes, chat, death. A predicted frame **may not emit these at all**,
   and the emitter should be unavailable during replay rather than
   guarded at each call — see Decision 1's `LazyUpdate` rule, which is
   the same move.

**Testable form.** A source-level scan asserting no authority-only
emitter is reachable from the replay path, in the shape `T5.1`'s
`no_authority_deciding_file_reads_a_cohort` already uses. Plus a runtime
test that a replayed frame emitting a class-3 effect fails loudly rather
than silently double-applying.

**RULING NEEDED — where sounds sit.** I have put ability *sounds* in
class 2 (identity, deduplicated) rather than class 1. A player hearing
a sword swing twice under latency is worse than hearing it late; but
that is a judgement about feel, not about correctness, and reasonable
people will disagree.

---

## Decision 5 — History duration, memory budget, and fallback

The row demands **a number, a byte budget, and a named behaviour when
either is exceeded**, so this section gives all three rather than a
principle.

**Proposed rule.**

- **Duration: 500 ms** of history, at the server tick rate. Chosen
  against the connection this program actually has to survive rather
  than against a target: beyond ~500 ms a replay is reconstructing a
  world the player has already visibly left, and snapping is more honest
  than a long silent correction.
- **Budget: 64 KiB per client**, hard.
- **Behaviour when duration is exceeded:** oldest entries are dropped,
  and eviction is **by age, not by access** — the same rule `T5.4`'s
  snapshot store uses, because retention that depends on access timing
  is a wall-clock dependency wearing a different hat.
- **Behaviour when the budget is exceeded:** the client **stops
  predicting and snaps**, and the event is recorded. It does not silently
  shorten the window. A silently shortened window is a prediction system
  that degrades exactly when it is under load, which is when its failures
  are least explicable.

**Testable form.** Fill history past both limits and assert the named
behaviours; assert that budget exhaustion produces a recorded event and
not merely a smaller buffer.

**The numbers are the weakest part of this document.** They are
reasoned, not measured — there is no latency distribution from real play
to derive them from. I would rather state them concretely and be
corrected than state a principle that cannot be tested. If the ruling is
"measure first", that is a legitimate answer and the row's real blocker
is a measurement, not a decision.

---

## What I am NOT proposing

- **Not proposing that T7.1 is satisfied by this document.** It is
  satisfied when someone with the authority to rule has ruled.
- **Not proposing any T7.2 work.** T7.2 is a one-way door: once client
  prediction and server authority share a kernel, every later
  character-behaviour change changes both. That is the row's purpose and
  the reason this row gates it.
- **Not proposing the `energy` classification as settled.** See
  Decision 1.

## Summary of what needs ruling

| # | Decision | State |
|---|---|---|
| 1 | Predicted components and replay-legal transitions | Proposed; `energy` classification low-confidence |
| 2 | World revision requirements and chunk-unload invalidation | Proposed; per-frame chunk-key cost unmeasured |
| 3 | Ridden/mounted ownership | Proposed (not predicted); **carried entities RULING NEEDED** |
| 4 | Predicted side-effect scope | Proposed as three classes; **sound classification RULING NEEDED** |
| 5 | Duration, budget, fallback | Proposed with concrete numbers; numbers are reasoned, not measured |
