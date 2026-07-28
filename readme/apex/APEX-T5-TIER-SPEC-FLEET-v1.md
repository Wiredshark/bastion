# APEX-T5 — Session framing and prediction evidence before rollback (fleet-authored spec v1)

Authored by Builder Opus 5 on `bastion/apex-t34` @ `386af45624`, from the
master-order rows `APEX-T5.1`..`T5.4`, grounded in live code reads at that
tip. Symbols cited were read, not recalled.

**The tier's thesis.** T5 is the row that says *do not build rollback
yet*. Every row here exists to make prediction failure **measurable and
attributable** first: a cohort you can compare against (T5.1), an input
frame complete enough to replay (T5.2), receipts that localise the first
divergence (T5.3), and a wind input that does not depend on when a packet
happened to arrive (T5.4). Rollback built on top of an unmeasured
prediction path would be a rewrite with no way to tell whether it helped.

This tier composes directly with T3.6: the physics generation is the
outer eligibility guard, and T5.2/T5.3 are what make a *specific*
correction explicable rather than merely legal.

---

## Shared failure surface (verified)

**Input frames are render-batched, not tick-cadenced.** `Client::tick`
(`client/src/lib.rs:3025`) takes `inputs: ControllerInputs` and a frame
`dt`, and voxygen calls it once per rendered frame
(`voxygen/src/session/mod.rs:1625`, `client.tick(self.inputs.clone(), dt)`).
The inputs are written into the local `Controller` and then sent verbatim
(`client/src/lib.rs:3066`, `send_msg_err(ClientGeneral::ControllerInputs(..))`).
So the *rate and grouping* of authoritative input is a function of
framerate. Two clients at 60 and 144 FPS submit differently-grouped input
for identical player intent, and neither frame carries a sequence number
or a physics generation.

**Wind used by glider prediction is receipt-time interpolated.**
`WeatherLerp` (`client/src/lib.rs:214-238`) holds
`old_local_wind`/`new_local_wind` as `(Vec2<f32>, Instant)` pairs and
lerps by *wall-clock elapsed time since the packet arrived*
(`update_local_wind`, `:227-238`, `self.new_local_wind.1.elapsed()`), with
an explicit `// Assumes updates are regular` and a `TODO` conceding the
interpolation's weakness. That value feeds `local_wind`, and glide
consumes air flow to steer (`common/src/states/glide.rs:154-157`,
`lateral_wind_speed`). Network jitter therefore changes predicted glider
behaviour — the exact thing T5.4's acceptance criterion forbids.

**The physics cohort exists but is a single opt-in bool.**
`PlayerPhysicsSetting` (`common/src/resources.rs:126-139`) is
`{ client_optin: bool }`, keyed per player uuid in `PlayerPhysicsSettings`
(`:163-165`). Its own doc-comment warns that it is "not the only source"
and that `ServerPhysicsForceList`
(`server/src/settings/server_physics.rs`) must also be consulted — that
force list is the **moderation anti-abuse** list, which T5.1 step 2
explicitly forbids reusing as a cohort. The read confirms the row's
premise: today there is no cohort concept distinct from "opted in" and
"forced by moderation".

---

## T5.1 — Existing server-authoritative physics cohort

**Objective.** Make the authority transition independently measurable
*before* anyone extracts rollback from it.

**Verified failure surface.** As above: `client_optin` plus a moderation
force list, with authority decided at
`server/src/sys/entity_sync.rs:834` (`server_authoritative_physics`) and
the client-report gate at `server/src/sys/msg/in_game.rs:191-195`. There
is no cohort label, and no metric attached to membership.

**Selected architecture.** A dedicated cohort policy resource, disjoint
from `ServerPhysicsForceList` by construction — the force list stays a
moderation tool and gains no measurement role, because conflating them
makes every metric a mix of "players who chose this" and "players we
punished". Cohort assignment is explicit and recorded per session.

Record per cohort: network profile, correction frequency, bandwidth,
glider-specific metrics, and responsiveness. The control cohort keeps the
current authority model *unchanged* — that is the point of a control.

**Migration steps.** (1) Cohort policy type + assignment, disjoint from
the force list. (2) Metric collection keyed by cohort. (3) Identical
scenario harness runs across both cohorts. (4) Comparison report,
including failures, not just successes.

**Required tests.** Cohort assignment is stable across a reconnect;
force-list membership does not imply cohort membership (the disjointness
test — this is the one that catches a lazy implementation); the control
cohort's behaviour is byte-identical to pre-change for the same scenario.

**Canary sketch.** `COH-001..` — force-listed player counted in the
treatment cohort; cohort flipping mid-session; metrics attributed to the
wrong cohort after a reconnect; control cohort's authority path altered.

**BUILT (steps 1–2) — `server/src/physics_cohort.rs`, 8 tests, wired at
`sys/msg/in_game.rs`.** `COH-001..004` covered, `COH-005` named OPEN.

- **Disjointness is a property of a type, not of a filter.**
  `CohortInputsV1` has one field, the player's own opt-in. There is no
  force-list field, so assignment *cannot* consult moderation state — a
  lazy implementation cannot include it because there is nothing to
  include. `COH-001` drives a force-listed, non-opted-in player and gets
  `Control`, even though `should_sync_client_physics` returns true for
  them. Moderation is not enrolment.
- **A mid-session flip is refused, not honoured.** The registry pins the
  first assignment and returns `FlipRefused`, counting the attempt.
  Honouring the flip would split one session's metrics across both
  cohorts, and a comparison built from that would be confidently wrong.
  Reports arriving during a refused flip are additionally counted on
  their own axis so they cannot quietly contaminate either total.
- **Reconnect stability comes from the key.** Assignment is keyed by
  `Uuid`, not by entity or session: the entity is gone after a reconnect,
  the account is not.
- **`COH-004` is enforced against the source.** No authority-deciding
  file may BRANCH on a cohort — declaring, destructuring and recording
  are fine, branching is not. Verified by inserting a deliberate branch,
  which turns the test red. The scan's limit is stated in place: a
  condition computed on one line and branched on another escapes it.

Step 2 delivers correction frequency only (per-cohort admitted/rejected
client physics reports, counted at the ingress site where both the
generation gate and the opt-in are already in scope). **Bandwidth,
glider-specific and responsiveness metrics are NOT collected**, and
neither is the identical-scenario harness (step 3) or the comparison
report (step 4) — those need a scenario runner that does not exist. That
is `COH-005`, carried as a named OPEN so nothing here can be read as
measuring them.

---

## T5.2 — Complete input-frame identity

**Objective.** Every physics-affecting input needed for replay belongs to
one frame, or explicitly invalidates prediction.

**Verified failure surface.** The render-batching above, plus: the frame
carries `ControllerInputs` only. Queued input kinds, events, actions,
look/target quantisation, mapping/context, physics generation and
environment reference are either absent or arrive on separate paths with
no shared identity.

**Selected architecture.** A fixed client **input tick** cadence
independent of render batching. Presentation camera stays high-rate and
is *sampled once* at the input tick — the row is explicit that
responsiveness is not to be sacrificed, and sampling is how both are
kept.

One `InputFrameV1` carries: continuous controls, queued input kinds,
events, actions, quantised look/target, mapping/context identity, the
**physics generation** (T3.6's `PhysicsGenerationV1`), and the
environment reference (T5.4's weather snapshot id). A monotonic input
sequence is assigned *inside* (session, connection, generation) — the
same nesting T3.5 uses for command sequences, and for the same reason: a
sequence is only meaningful inside the identity it is scoped to.

Server accepts in sequence with a bounded gap/reject policy. Note the
composition with T3.6: `PlayerPhysics` is `LatestState` under T3.5's
classification, so the input sequence orders frames *within* an eligible
generation while the generation decides eligibility at all. Three
mechanisms, three jobs, none collapsed.

**Required tests.** Split and coalesced OS events produce the same frame
content; FPS variation (60/144/uncapped) produces identical frame
*sequences* for identical intent; a batch boundary landing mid-input does
not drop or duplicate a queued action. The FPS-invariance test is the
row's real acceptance criterion and should be written first.

---

## T5.3 — Input receipts and dual prediction probes

**Objective.** Attribute the first divergence to one field, and never let
a quantised observation certify exact execution.

**Selected architecture.** A receipt per input frame carrying: accepted
or rejected sequence, server tick, generation, correction reason, and an
exact state probe. **Two** probes, kept structurally distinct:

- **Exact probe** — hash of the raw authoritative bits, for divergence
  localisation. This is the one that can certify.
- **Semantic-quantised probe** — a separate, *versioned* hash for
  tolerance analysis. This one can never certify exact execution, and the
  type system should make that impossible rather than the documentation
  asking nicely: distinct types, no `From` between them, no comparison
  operator that accepts one against the other.

  *Lineage, so the next reader knows this is a pattern rather than an
  accident:* this is the same move as T3.5's commit sink, whose methods
  return unit so a recoverable mid-commit failure is unrepresentable, and
  T3.4's `production_checkpoint_profile_v1`, which returns an error
  because there is no production profile to invent. The program's
  standing preference is that a rule the code cannot break beats a rule
  the documentation asks for politely. Write new invariants that way by
  default.

Store client and server records; report the **first component mismatch**,
not a count — the same discipline T3.5.20's perturbation harness follows
(`first_divergence`), because the first mismatch explains the rest.

**Required tests.** Rejection and acceptance paths; stale generation
(composes with T3.6 — a report from an older generation must be rejected
before any probe comparison); hidden raw drift, where the quantised
probes match and the exact probes do not. That last one is the tier's
non-vacuity case: if it passes trivially, the two probes are not actually
independent.

**Canary sketch.** `PROBE-001..` — quantised match with exact mismatch;
exact match with quantised mismatch; receipt for a stale generation;
receipt for an unaccepted sequence; first-mismatch report truncated to a
count; a probe type converted into the other.

---

## T5.4 — Tick-owned weather input

**Objective.** Wall-clock receipt timing cannot change gameplay-prediction
wind state.

**Verified failure surface.** The most concrete in the tier.
`WeatherLerp::update_local_wind` (`client/src/lib.rs:227-238`) lerps on
`Instant::elapsed()` since packet arrival, over a denominator that is the
interval between the last two *arrivals*. Its own comment concedes
`// Assumes updates are regular`. Under jitter the assumption fails and
`local_wind` — which reaches glider steering via air flow
(`common/src/states/glide.rs:154-157`) — takes a different value for
identical server state. Two clients receiving the same weather packets
with different jitter predict different glides.

**Selected architecture.** Split the value in two, by *purpose*:

- **Authoritative weather input** — carries server tick / snapshot
  identity. Input frames reference the snapshot id (T5.2's environment
  reference). Prediction history retains the snapshot it predicted
  under, so a replay uses the same wind the original did.
- **Presentation wind** — keeps receipt-time interpolation, unchanged,
  for rendering only. The current `WeatherLerp` becomes this and is
  explicitly barred from the prediction path.

A missing snapshot forces a snap or non-predictive fallback rather than
an extrapolation, because extrapolating is how the wall-clock dependency
gets back in through a side door.

**Required tests.** Vary receipt delay while holding the snapshot
sequence constant, and require *equal prediction inputs* — that is the
acceptance criterion stated as a test. Also: a dropped snapshot produces
the fallback, not an interpolation; presentation wind may differ between
two clients while prediction wind may not.

---

## Cross-tier notes

**Ordering.** T5.1 is `READY after T3.1–T3.3` and is the only immediately
startable row; T5.2 → T5.3 are a chain (receipts are meaningless without
a complete frame to receipt); T5.4 feeds T5.2's environment reference, so
it should land before or with T5.2 rather than after.

**What T5 must NOT do.** Extract rollback. Every row here is measurement
and framing. The tier's own name says "before rollback extraction", and
the acceptance criteria are all of the form "X is now measurable /
attributable", never "X is now faster".

**Composition with T3.6, stated once so it is not re-derived three
times.** Generation decides which frames are *eligible*; the T5.2 input
sequence orders eligible frames; T3.5's `LatestState` class decides which
of several eligible frames *wins*. Three mechanisms with three jobs.

Per-row re-derivation is how three-mechanism systems drift into two, so
it is stated here once and cited from the rows rather than repeated. Any
row that collapses two of them has introduced a bug that will present as
"prediction is occasionally wrong under load" — the kind of bug that gets
misattributed to netcode for months.
