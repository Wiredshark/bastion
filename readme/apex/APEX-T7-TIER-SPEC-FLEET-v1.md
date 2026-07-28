# APEX-T7 — Prediction rollback (fleet-authored spec v1)

Authored by Builder Opus 5 on `bastion/apex-t34` @ `5639194cbb`, from the
master-order rows `APEX-T7.1`..`T7.5`, grounded in live code reads at that
tip. Symbols cited were read, not recalled.

**The tier's thesis.** T7 is the payoff tier, and it opens with a row
that forbids building it. `T7.1` is `NEEDS-DESIGN` and ends with *"Do not
implement rollback until this state boundary is approved."* That is not
ceremony: rollback is defined entirely by its state boundary, and a
boundary chosen implicitly during implementation is a boundary nobody
reviewed. Every other row in the tier assumes T7.1's answer as an input.

**Read T5, T6 and T7 as one arc.** T5 makes prediction failure
measurable, T6 makes execution reproducible, T7 acts on both. Rollback
built before either would be unfalsifiable — you could not tell a fixed
divergence from a moved one.

---

## Shared failure surface (verified)

**A shared tick already exists, and it is not a kernel.** `State::tick`
(`common/state/src/state.rs:1102`) is called by both sides — client at
`client/src/lib.rs:3128` and server at `server/src/lib.rs:3953`. So the
crate is shared, which is the good news. What it is not is a *pure
transition function*: it takes `&mut self` over a whole `specs::World`,
mutates global clocks (`TimeOfDay`, `Time`, `ProgramTime`, `:1133-1139`),
runs the full dispatcher, and takes a `block_update` callback. It advances
a world; it does not compute one state from another.

T7.2 asks for something narrower: a *pure* `(state, input, environment) →
state` API for the local player, sharing one crate and version between
client and server. The gap is not "no shared code" — it is that the
shared code is not shaped like a function.

**Character transitions are trait-dispatched over live ECS data.**
`CharacterBehavior::behavior(&self, data: &JoinData, output_events)`
(`common/src/states/behavior.rs:20-21`) reads through `JoinData`
(`:139`, `character: &'a CharacterState`; `:180`,
`char_state: FlaggedAccessMut<...>`). Extracting the pure kernel means
deciding which of those `JoinData` fields are genuinely *inputs to the
transition* and which are ambient world access — and that decision **is**
T7.1's scope question, not a refactor detail.

**Clock advancement is already order-and-clamp sensitive.** The T0.8
comment at `state.rs:1122-1132` records that all clocks now lag together
under a hitch, and that bounded fixed substeps were deliberately left
out. Rollback replay must reuse this exact clamp discipline; a replay
that advances clocks differently from the original run produces a
different answer for reasons that have nothing to do with the correction.

---

## T7.1 — Prediction-state scope decision

**Objective.** Get the state boundary *approved* before any rollback code
exists.

**This is a decision row, not a build row.** Its output is a written,
reviewed boundary. The required decisions, each of which changes the
other rows:

1. **Exact local-player components and transitions.** Which components
   are predicted, and which `CharacterState` transitions may occur during
   replay. Concretely: this is a list drawn against `JoinData`'s fields
   (`states/behavior.rs:139-180`), separating transition inputs from
   ambient access.
2. **Terrain / dynamic-collider / environment revision requirements.**
   What revision of the world a replayed frame needs, and what happens
   when it is gone. This is the row that decides whether a chunk unload
   invalidates history.
3. **Carried / ridden / mounted ownership.** Whether a predicted local
   player carries or rides something whose state is another authority's.
   Mounting already crosses this line in-tree (`common/src/mounting.rs`
   forces updates), so the answer cannot be "it does not happen".
4. **Predicted side-effect scope.** Which effects a predicted frame may
   emit — sounds, particles, events with identity — and which must wait
   for authority. T7.4's deduplication depends entirely on this answer.
5. **Acceptable history duration, memory, and fallback.** A number, a
   byte budget, and a named behaviour when either is exceeded.

**Deliverable.** A reviewed boundary document, with each decision stated
as a rule a test could check. **No rollback code lands until it is
approved** — and the tier should treat an unapproved boundary as a hard
blocker, not a soft one, because the cost of discovering the boundary was
wrong is a rewrite of T7.2 through T7.5.

---

## T7.2 — Shared fixed-tick local-player kernel

**Objective.** Client prediction and server authority execute **one**
versioned transition kernel.

**Verified failure surface.** As above: `State::tick` is shared but is
not a transition function, and character behaviour is trait-dispatched
over ambient ECS access.

**Selected architecture.** Extract a pure API — state in, input in,
environment in, state out — carrying locomotion, orientation, glider, and
*only the character transitions T7.1 approved*. Same crate, same version,
both sides; the version is part of the kernel's identity and belongs in
the T4.1 bootstrap manifest as equality-critical, because a client
predicting with a different kernel version than the server executes is
the failure this row exists to prevent.

Rendering, UI and audio stay outside — not as a style preference but
because they are the things that must be replayable-without-side-effects
in T7.4.

The server sequences and executes authoritative frames through the same
kernel. That is what makes the per-frame raw probe comparison (T5.3,
T6.2) meaningful: same function, same inputs, compare bits.

**Migration steps.** (1) Define the pure API against T7.1's approved
boundary. (2) Move the approved transitions behind it, leaving ambient
access outside. (3) Route the server's authoritative execution through
it. (4) Route client prediction through it. (5) Compare per-frame raw
probes under identical inputs — the acceptance test.

**Required tests.** Identical inputs through client and server paths
produce identical raw probes; the kernel version mismatch is detected at
bootstrap, not at divergence; a transition *not* in the approved set
cannot be reached through the kernel API.

---

## T7.3 — Prediction history ring

**Objective.** Every replayable frame has exactly the state and
environment needed to rerun it — no more, no less.

**Selected architecture.** Entries keyed by the full identity nest this
program has been building toward: boot / session / connection epoch,
**physics generation** (T3.6's `PhysicsGenerationV1`), and input sequence
(T5.2). That is not four keys stapled together; it is the same nesting
rule stated in T5.2 — a sequence is only meaningful inside the identity
it is scoped to.

Each entry stores: the complete input frame, the before *and* after
state, the environment revisions it depended on, and the identities of
any predicted events it generated (T7.4 needs those to retract them).

Bounded by time and memory, evicting **in sequence order** — eviction by
any other policy leaves holes, and a hole in the middle of a replay range
is indistinguishable from a lost frame.

Invalidate wholesale on manifest, numeric-profile, kernel-version or
generation mismatch. `PredictionHistoryV1::adopt_generation_v1`
(`common/src/apex/physics_generation.rs`, T3.6) already implements the
generation half of this and should be extended rather than duplicated.

**Required tests.** A frame's entry contains everything needed to rerun
it with no ambient reads (the "exactly" half of the acceptance
criterion); eviction never leaves a hole; each invalidation trigger
clears history; a missing environment revision is detectable *before*
replay rather than during it.

---

## T7.4 — Restore and replay

**Objective.** The same correction and input history yields the same
final predicted state.

**Selected architecture.** Find the authoritative acknowledged
(sequence, generation). **Reject stale corrections** — T3.6's
`admit_report_v1` already distinguishes stale from forged and should be
the mechanism, not a parallel check. Restore authoritative state *and*
environment, then replay every later valid frame through the T7.2 kernel
**without rendering intermediates**.

Deduplicate and retract presentation effects by *deterministic event
identity* — the same derivation discipline as T3.5's command identity and
E5-B's attack instances. An effect emitted during a discarded prediction
must be identifiable well enough to retract; an effect re-emitted during
replay must be identifiable well enough not to double.

Compare the final probe and record correction magnitude. That magnitude
is the T5.1 cohort metric, so it should be recorded in the form that row
consumes.

**Required tests.** The acceptance criterion is a determinism property
and should be tested as one: the same correction plus the same input
history, run twice, produces the same final state — and, separately,
produces the same *event* set, since a replay that silently re-emits is
the failure mode this row is most likely to ship with. A stale correction
must be rejected without touching history.

---

## T7.5 — Safe fallback and smoothing

**Objective.** Failure degrades responsiveness and visual continuity,
never authoritative correctness.

**Selected architecture.** On missing history, missing environment, or
protocol mismatch: **full snap**. Clear invalid predicted effects and
history rather than attempting partial reuse — partial reuse is how a
fallback path becomes a second, untested prediction implementation.

Smooth **only the presentation transform** toward corrected authority,
and never feed smoothing back into simulation. This is precisely the
T5.4 split applied at the other end of the pipeline: authoritative value
for simulation, interpolated value for the eye, and a hard rule that the
second never reaches the first. Stating it the same way in both tiers is
deliberate.

**Required tests.** Large correction; teleport; terrain revision loss;
generation reset. Each must snap rather than replay, and in each the
simulation state after the snap must be byte-identical to the
authoritative state — smoothing may not have touched it. That
byte-identity assertion is the row's real acceptance test: it is the only
way to prove the smoothing did not leak.

---

## Cross-tier notes

**Ordering, and the gate.** T7.1 gates everything. T7.2 → T7.3 → T7.4 is
a strict chain; T7.5 is `READY after T7.4`. Nothing in this tier should
start before T5 has produced measurement and T6.3 has produced a
reproducible contribution tape — not because the code will not compile,
but because without them a rollback change cannot be evaluated.

**What T7 inherits and must not re-derive.** Physics generation and its
history invalidation (T3.6), input sequence and frame completeness
(T5.2), receipts and dual probes (T5.3), the weather-snapshot split
(T5.4), ordered contributions (T6.3), and the numeric profile whose
mismatch invalidates history (T6.4). If a row here finds itself inventing
an identity or an ordering rule, it has almost certainly duplicated one
of those.

**The one-way door.** T7.2's kernel extraction changes where authoritative
transitions live. Once client prediction and server authority share it,
every later change to character behaviour is a change to both. That is
the point, and it is also the reason T7.1's boundary must be approved
first: the kernel's surface is the boundary, made permanent.
