# APEX-T7.1 — prediction-state boundary: PROPOSAL (v1)

**Status: APPROVED 2026-07-28** by the orchestrator, after independent
review by Builder 5b. This document is now the boundary `T7.2`–`T7.5`
build against, not a proposal. The header below is kept as written
because how a boundary was arrived at is part of what it means.

**Both self-flagged low-confidence items were resolved in the
proposal's favour by the independent review, and neither on my say-so:**

- **`energy` as a transition input is PROVABLE, not a guess.** The
  review traced the `StateUpdate` `From` impl chain —
  `behavior.rs:153` → `character_state.rs:94` value copy →
  `requirements_paid` boolean. Closed as settled; the "first thing I'd
  want a second reader on" got a second reader and held.
- **The per-frame chunk-key cost is measured, so its gate is REMOVED.**
  Computed from the real `Spiral2d::new().take(9)`: ~1.05 KiB per client,
  **1.6% of Decision 5's own 64 KiB budget**. The measurement gate and the
  coarser fallback are struck; Decision 2 stands exactly as proposed.

**The three items I deliberately did not decide are now ruled**, and the
rulings are recorded in place beneath each one.

---

## AMENDMENT 1 — 2026-07-28, found while implementing this boundary

**Decision 1 was incomplete, and the arithmetic said so in the approved
text.** It listed **14** transition inputs and **22** ambient fields, and
called `JoinData` **38** fields wide. 14 + 22 = 36. The gap sat in an
approved document through its author *and* an independent reviewer.

The two missing fields are **`entity`** and **`uid`**. They appear in the
struct and in neither list, because they are neither: they are
**identity** — WHO is transitioning. Fixed for the whole of a history,
never replayed from it, never re-read from ambient authority, because a
replay of a different entity is not a replay of the same frame.

A fourth role is therefore added: **`Identity`**, pinned at exactly two
members. Final split: **14 transition inputs · 21 ambient · 1 write
channel · 2 identity = 38.** (The proposal's "22 ambient" counted
`updater`, which Decision 1 already separates as a write channel.)

**Why this is recorded as an amendment rather than a silent fix.** An
approved boundary that quietly matches its implementation is worse than
one that shows its repair: the next reader cannot tell which parts were
reviewed and which were adjusted afterwards. The approval above stands
unedited; this is what changed after it.

**The transferable part.** The hole fell out the moment the
classification stopped being prose and became a testable constant — the
count assertion failed on the first run. `entity` and `uid` were omitted
precisely *because* they are obvious, and **"too obvious to write down"
is how a boundary acquires a hole.** Prose boundaries carry holes
forward; typed boundaries surface them.

---

## AMENDMENT 2 — 2026-07-29, `T7.4` item B premise-check on Decision 4

**The question, verbatim, as `T7.4` (`APEX-T7-TIER-SPEC-FLEET-v1.md`)
states it:** "Deduplicate and retract presentation effects by
*deterministic event identity*... An effect emitted during a discarded
prediction must be identifiable well enough to retract; an effect
re-emitted during replay must be identifiable well enough not to
double."

**Premise-check found this question is not one question.** The
DOUBLE-FIRE half is already solved — `T7.3b`'s `CharacterStateEventSinkV1`
(`common/src/event.rs`) discards every event a replayed frame emits, with
its own doc comment stating exactly why: the original predicted pass
already fired them once, live; re-delivering on replay would be the
double-fire hazard Decision 4 exists to prevent. That ruling stands
unamended.

**The RETRACTION half decomposes by a property Decision 4 did not
need to state, because it precedes effect *identity* entirely: whether
the effect physically CAN be retracted.** A played sound cannot be
un-played. A spawned particle fades on its own schedule regardless of
what any ledger says. For a transient effect, "retraction" is
infrastructure serving a physical impossibility — there is nothing to
retract, only something to wait out. Decision #31 (orchestrator-ruled,
2026-07-28, ability sounds) already covers this shape exactly:
**deduplicated, late beats double** — a corrected replay that concludes
an effect should have fired may emit it late; it must never emit it
twice. That law generalizes to every transient effect, not just sounds,
by the same reasoning: a late effect is honest (merely late), a doubled
one asserts two events happened, which is false.

**So the only class that needs a retraction mechanism is one Decision 4
never separately named: effects that persist visibly past their own
moment.** An over-fired instance of one of THESE does not expire on its
own — it is a standing visual lie until something removes it. Whether
this class is empty decides whether `T7.4` item B needs a ledger at
all.

**It is not empty. Six of `CharacterStateEvents`' emitters qualify**
(count corrected below), which reopens Decision 4 rather than merely
extending it: predicting these effects at all is questionable — several
already read as Decision 4 class 3 in spirit (authority-only,
observable by another player, or entity-creating) even though nothing
today enforces that at the type level for events the way
`MayInsertComponentsV1` enforces it for `LazyUpdate` — a separate,
disclosed finding, not silently folded into this one:
`MayEmitAuthorityEffectsV1` (`prediction_boundary.rs`) is implemented
for `LiveContextV1` and required by NOTHING — the fourth unwired
instrument this tier's own family has turned up (after
`admit_report_v1`, `PredictionHistoryV1`'s methods on
`ClientPredictionBufferV1`, and `adopt_generation_v1`'s own live call).

**Count correction.** `CharacterStateEvents` (`common/src/comp/
character_state.rs:39-61`) carries **21** emitter channels, not the
"20" both my own first premise-check message and the orchestrator's
reply said. Same arithmetic-catches-holes lesson as Amendment 1: a
count nobody re-verified against the actual declaration carried an off-
by-one through two messages before this table forced the recount.

**The classification, by what a discarded-versus-replayed divergence
actually does to each:**

| Channel | Payload names | Class | Why |
|---|---|---|---|
| `energy_change` | `EnergyChangeEvent{entity,change,reset_rate}` | **STATE-COVERED** | `RollingStateV1.energy` already carries the corrected truth; no separate effect identity needed. |
| `knockback` | `KnockbackEvent{entity,impulse}` | **STATE-COVERED (self case), BORDERLINE (other-target case)** | Self-applied impulse lands in `RollingStateV1.vel`; the payload does not distinguish self-recoil from knocking back a DIFFERENT entity, and this scan did not trace every call site to rule the second case out. Flagged, not asserted. |
| `teleport_to` | `TeleportToEvent{entity,target,max_range}` | **STATE-COVERED** | The outcome lands in `RollingStateV1.pos`; the visual snap is transient and covered by the transient ruling below regardless. |
| `combo` | `ComboChangeEvent{entity,change}` | **GAP, NOT ITEM B'S** | Not in `RollingStateV1` today (checked: 7 fields, no combo). Self-targeted deterministic counter delta — the same shape as `energy`, and arguably belongs in Decision 1's predicted-component list, not Decision 4's effect-scope list. Named here because the search that found it was item B's; fixing it is a Decision-1 question. |
| `change_stance` | `ChangeStanceEvent{entity,stance}` | **NEEDS VERIFICATION** | Plausibly covered by `RollingStateV1.character_activity` already; not traced far enough this pass to assert either way. |
| `shoot`, `throw`, `shockwave`, `explosion`, `beam_pillar_summon` | projectile/AoE ability activations | **TRANSIENT-PRESENTATION** | Decision 4's own class-2 example ("ability activation effects... `CommandId`"). Expire or resolve on their own; Decision #31's law applies directly: late beats double, no ledger. |
| `event` (Aura), `buff`, `sprite_summon`, `sprite_light`, `transform`, `regrow_head` | `AuraEvent`, `BuffEvent`, `CreateSpriteEvent`, `ToggleSpriteLightEvent`, `TransformEvent`, `RegrowHeadEvent` | **DURABLE-PRESENTATION — the non-empty class** | Each persists past its own moment (an aura zone, a buff icon, a placed sprite, a toggled light, a transformed body, a regrown head) and is visible to something other than a private, self-expiring animation. An over-fired instance from a discarded prediction does not fade on its own. |
| `inventory_manip`, `create_npc`, `create_object`, `create_aura_entity`, `help_downed` | item transfer, entity creation ×3, reviving another entity | **ALREADY OUT OF BOUNDS (unenforced)** | Decision 4 class 3 by its own named examples ("item transfer" is verbatim; entity creation and affecting another entity's downed state are the same shape). Should already be unreachable from a predicted frame; today nothing enforces that for events specifically (see the `MayEmitAuthorityEffectsV1` finding above) — a compliance gap orthogonal to retraction. |

**Verdict on the empty-or-not question: NOT EMPTY.** Six members
(`event`/Aura, `buff`, `sprite_summon`, `sprite_light`, `transform`,
`regrow_head`) durably persist. Per the orchestrator's own instruction,
this reopens the row rather than sizing B2 unilaterally: several of the
six read as questionable to predict AT ALL once named this explicitly
(a self-cast buff or transform is a much larger commitment to predict
than a footstep sound), which is a Decision-4-class-3-adjacent question
the classification table surfaces but does not resolve.

> **RULED (`DECISIONS #34`, 2026-07-29) — the six durable-presentation
> channels are EXCLUDED FROM PREDICTION in v1.** The question this
> ruling answers, verbatim: should `event`/Aura, `buff`, `sprite_summon`,
> `sprite_light`, `transform`, and `regrow_head` be predictable at all,
> given each persists past its own moment?
>
> **The strengthened law.** Decision #31's "late beats double" holds for
> TRANSIENT effects because a WRONG transient effect expires by itself —
> the cost of being wrong is bounded by the effect's own lifetime. For
> DURABLE effects the same reasoning runs the other way: **wrong is
> worse than late, by definition, because the lie persists.** A buff,
> transform, aura, or summoned sprite appearing one round-trip late is a
> minor responsiveness cost; appearing and then turning out to be FALSE
> is a standing falsehood — exactly what a retraction ledger would exist
> to claw back, and exactly what gating on confirmation makes
> unnecessary, the same way Decision 3 made rider-prediction divergence
> unrepresentable by simply not predicting it. Don't predict what you
> can't afford to retract.
>
> **The rule.** All six channels are confirmation-gated: never
> predictively presented, applied only once the authoritative CompSync
> (or equivalent server-confirmed path) delivers them.
>
> **Revisit trigger, same shape as Decision 3's carried-entity revisit.**
> A named channel graduates from confirmation-gated to predicted only on
> playtest feel evidence, per channel — not as a blanket reconsideration
> of this ruling, and whatever machinery that graduation needs (partial
> retraction? a narrower durable-but-cheap-to-undo subclass?) is scoped
> at that time, not invented now against a hypothetical.

**Conformance check — grounding `item B`'s actual build against
`DECISIONS #34`, per the orchestrator's "ground first, then build or
close" instruction.**

`DECISIONS #34`'s law governs what **`T7`'s own machinery** presents.
`T7.3b`'s replay sink (`CharacterStateEventSinkV1`) already conforms:
every one of the 21 channels a REPLAYED frame emits is discarded,
durable or not, so replay never presents anything predictively. That
half needed no change.

**The six durable channels are NON-CONFORMANT TODAY on the FIRST
(non-replay) tick — confirmed for two, traced no further for four.**
`common_systems::add_local_systems` (`common/systems/src/lib.rs:25-52`,
the client's own local dispatch) runs `character_behavior::Sys`
alongside `buff::Sys` and `aura::Sys` every client tick. `buff::Sys`'s
own `event_emitters!` declaration (`buff: BuffEvent`,
`common/systems/src/buff.rs:37-49`) reads the SAME live
`EventBus<BuffEvent>` `character_behavior::Sys`'s normal (non-replay)
tick writes into — read directly, not inferred. So a self-cast
buff-granting ability the client predicts is picked up by the client's
own `buff::Sys` the SAME tick, applied to the live `Buffs` component
and shown — unconfirmed, before any `CompSync`. `aura::Sys` is the same
shape for `event`/Aura. `sprite_summon`, `sprite_light`, `transform`,
and `regrow_head` were not traced to this depth this pass; their
conformance is UNKNOWN, not asserted either way.

**Disposition (orchestrator-ruled): BANKED, not `item B`'s scope.**
Deciding precedent: `CKPT-174` — a hardening row does not change live
player-facing behavior as a side effect. This leak PRE-DATES the whole
`T7` rollback program (self-buff/transform responsiveness has worked
this way since before any of this machinery existed); its wrongness
rate is bounded by how often a predicted frame is actually refused
(rare); and the real fix cost is a FEEL change (self-buff/transform
latency, +one round-trip) — the same class of call as Decision 3's
carried-entity ruling, not a mechanical wiring gap `item B` can close
as a rider.

**The banked row: `T7-DURABLE-GATE`.** Scope: gate the client's own
presentation of the six durable channels on confirmation, closing the
leak documented above. The design fork is named, not decided — a
future builder chooses WITH feel-testing available, not blind:
- **Source-block-self**: `character_behavior::Sys` (or its call sites)
  withholds these six emissions for the LOCAL player's own predicted
  frame specifically, letting every other entity's already-confirmed
  state flow through `buff::Sys`/`aura::Sys` unchanged.
- **Consumption-filter**: `buff::Sys`/`aura::Sys` (and whatever
  consumes the other four) filter out the local player's own
  not-yet-confirmed instances at the point of application, rather than
  at the point of emission.
- `MayEmitAuthorityEffectsV1` (the fourth unwired instrument named
  above) is a candidate type-level home for whichever shape the row
  picks, not a commitment to either.

Revisit trigger for `T7-DURABLE-GATE` itself: T5.1-cohort/playtest
feel-measurement machinery existing, or a real standing-falsehood
incident — whichever comes first names when the row is worth taking.

**What this amendment does NOT do:** it does not build a retraction
ledger, does not close the documented leak, and does not wire
`MayEmitAuthorityEffectsV1`. It is the ground `T7-DURABLE-GATE`'s own
future scoping needs, and the record that `item B` closed here rather
than silently.

---

**Original status line, kept verbatim:** *PROPOSAL. Not approved, and not
approvable by its author.*

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

> **RULED: carried entities get NO prediction in v1, in the mount rule's
> shape — with a NAMED REVISIT CONDITION.** If carry-latency feel
> complaints emerge, it becomes a `T5.1`-cohort experiment. That
> infrastructure exists for exactly this: a treatment cohort, disjoint
> from moderation, with per-cohort metrics. The revisit is a measurement,
> not a re-argument.

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

> **RULED: presentation namespace, DEDUPLICATED — late beats double.**
> The reason given is better than mine: a doubled sound asserts that TWO
> EVENTS HAPPENED, which is a false fact about the world. A late sound is
> merely late, and honest. Correctness argument, not a feel argument.

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

> **RULED: adopted as reasoned NAMED CONSTS, tuned later from `T5`'s
> cohort metrics.** `T7` does not gate on a measurement programme that
> `T5` produces anyway — waiting would have serialised two tiers for a
> number that arrives on its own. Named constants so the later tuning
> edits one place with a visible diff.

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
| 1 | Predicted components and replay-legal transitions | **APPROVED**, then **AMENDED** (see Amendment 1): `energy` proved from the StateUpdate From chain; `entity`/`uid` added as a fourth `Identity` role — 14 input · 21 ambient · 1 write channel · 2 identity |
| 2 | World revision requirements and chunk-unload invalidation | **APPROVED as proposed**; chunk-key cost measured at ~1.05 KiB/client (1.6% of budget), gate and fallback removed |
| 3 | Ridden/mounted ownership | **RULED** — no prediction for riders or carriers in v1; carry revisits as a T5.1-cohort experiment if feel complaints emerge |
| 4 | Predicted side-effect scope | **RULED**, **REOPENED**, then **RE-RULED** (see Amendment 2): three classes as proposed, ability sounds deduplicated (late beats double, transient effects only); the 6 durable-presentation channels (`event`/Aura, `buff`, `sprite_summon`, `sprite_light`, `transform`, `regrow_head`) are EXCLUDED FROM PREDICTION in v1 (`DECISIONS #34`) for `T7`'s own machinery — the replay sink already conforms. A PRE-EXISTING leak (2 of 6 confirmed non-conformant on the first tick, 4 untraced) is documented, not fixed here — BANKED as `T7-DURABLE-GATE`, its own row, design fork stated not decided |
| 5 | Duration, budget, fallback | **RULED** — adopted as named consts, tuned later from T5 cohort metrics |
