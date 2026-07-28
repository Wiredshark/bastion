# Engine Block E1 — List Admission & Unblock Audit (FIRST PASS)

**Status note up front, honestly:** this is a solid first pass, not an exhaustive sweep of all
238 blocked Tier 1–3 rows. Given the true size of that task (row-by-row prerequisite
verification against the entire current tree), I prioritized: (1) the fully-bounded T0
reconciliation, (2) spot-checking the T0.67–89 batch for APEX-tier overlap (I have first-hand
knowledge there from this session), (3) verifying the SHARED-MECHANISM prerequisites Fable named
("clusters over row-by-row") since one shared-mechanism confirmation can unblock many rows at
once. A full row-by-row Tier 1–3 sweep is flagged as follow-up, not silently skipped.

Branch note: `bastion/engine2` off `bastion/apex` doesn't exist yet (gated on the T3.3 merge,
per Fable's own instruction) — this content is staged here pending that, not yet committed to a
branch. Placement TBD once the merge/branch situation is confirmed with Opus.

## 1. Tier-0 reconciliation

**Already-recorded deferrals, re-confirmed still correctly deferred (no tree change contradicts
them):**
- `T0.8-residual` (bounded physics substeps) — still deliberately deferred, HIGH fixture surface,
  justification stands.
- `T0.13` (double-buffer late events) — still correctly deferred; every `emit_event_now` site
  remains networking-adjacent (session/admin), single-player scope confirmed unchanged.
- `T0.44` (physics pair ownership) — still correctly dispositioned/deferred, justification
  (momentum-symmetry-only benefit, HIGH fixture surface) stands.

**T0.67–89 (23 rows, all marked READY, pointing to two external research docs) — spot-check
results, cited against the CURRENT tree (`git branch bastion/apex-t0`, tip at handoff
`306aab7772`):**

| Row | Finding | Citation |
|---|---|---|
| T0.67 (one deterministic algorithm, live+test) | **SUBSTANTIALLY LIVE.** `DETERMINISTIC_WORLDGEN` (atomic bool, `common/src/lib.rs`) and its RTSim twin exist and are load-bearing — this is the exact live-game-determinism opt-in flag this session's own memory records finding a real gap in and fixing (OS-entropy `tick_rng` fallback, `BASTION_DETERMINISTIC`). | `common/src/lib.rs:20-34` |
| T0.74 (canonical numeric ABI / finite values) | **CORRECTED — NOT satisfied by APEX, retracting my own earlier draft claim.** `apex/scalar.rs` line 266 has an explicit compile-fail proof that `f32`/`f64`/`usize`/`isize`/pointers CANNOT implement its fixed-width scalar traits — floats are DELIBERATELY EXCLUDED from APEX's scope, by design. T0.74's own ask (cross-platform float computation determinism for physics/path/worldgen, a Box2D-style contract) is a genuinely different, harder problem than canonical WIRE encoding, and APEX never touches it. Real, undiscounted open work. | `common/src/apex/scalar.rs:266` (the exclusion, not an inclusion) |
| T0.84 (fixed-width protocol IDs, explicit endian) | **MOSTLY LIVE via APEX T0.1/T0.4, one concrete named gap.** `apex/scalar.rs` + `apex/identity/` opaque-ID family (`CommandId`, `ServerBootId`, `SessionId`) cover persisted/network IDs, binary/hash inputs, and protocol DTOs — fixed-width, explicit codec, no native-endian/usize. NOT yet covered: `world_seed` itself is still a plain `u32` (`server/src/settings/mod.rs:184`), not wrapped in an apex-style fixed-width newtype with its own canonical expansion — the row's own "world seed expansion" sub-clause stays open. | `common/src/apex/identity/mod.rs`, `common/src/apex/identity/codec.rs`; gap at `server/src/settings/mod.rs:184` |
| T0.85 (world-scoped causal identity for workflows) | **PARTIALLY LIVE.** `CommandId` (APEX T0.4) is exactly a world-scoped causal identity primitive, but scoped to T3.5's future command-result use, not yet applied to quest/dialogue/trade IDs this row names. Real remaining work, but the PRIMITIVE (not just the concept) already exists — this row is a NARROWER lift than the research doc likely assumed pre-APEX. | `common/src/apex/identity/codec.rs:32,91` |
| T0.69 (creation-intent IDs / UID allocation order) | **NEEDS DEEPER CHECK, not resolved by this pass.** `UidAllocator` (`common/src/uid.rs`) already exists — but that's pre-existing engine infrastructure, not new; the row's actual ask is about ALLOCATION ORDER/quarantine semantics of that existing type, which I did not verify this pass. |  `common/src/uid.rs:32` |
| T0.89 (runtime context manifest) | **GENUINELY OPEN, no overlap found.** No `RuntimeContextV1`/equivalent found anywhere in `common/src`. This one looks like real, undiscounted new work. | (negative grep, `common/src` searched) |
| T0.68, T0.70–73, T0.75–83, T0.86–88 | **NOT YET CHECKED this pass** — flagged for the follow-up sweep, not claimed either way. |  |

**Corrected T0 headline for Fable's own "likely in our favor" prediction:** confirmed directionally
correct but narrower than my own first draft claimed — self-caught before finalizing (see T0.74's
own corrected row above: I initially over-claimed it as APEX-satisfied, then found the opposite,
an EXPLICIT compile-fail exclusion of floats, on closer read). Of the 3 originally proposed: T0.67
is genuinely substantially satisfied; T0.84 is MOSTLY satisfied with one concrete named gap
(`world_seed`); T0.74 is NOT satisfied at all and stays fully open. T0.85 remains partially
satisfied as originally found.

## 1b. FULL mechanism-catalog table reconciliation (Fable's priority-first pass — a
force-multiplier for the whole sweep, since Tier 1–3 rows cite this table's own verdicts as
their blockers)

All 13 rows of the "Mechanism / claim | Fact-check result" table (near the top of the master
list) checked against the current tree, not just the 2 spot-checked in the first pass:

| Mechanism / claim | Doc's verdict | Reconciled verdict |
|---|---|---|
| `ExecutionMode::{Parallel, DeterministicSerial}` + one-worker reference pool | LIVE-VERIFIED | **Unchanged, correct.** |
| Paired frozen-binary `Verdict` / ordered JSONL / `FirstDivergence` | LIVE-VERIFIED, NARROW | **Unchanged, correct.** |
| `CanonicalScheduleV1` / `OperationKey` | NOT LIVE | **Unchanged, confirmed still absent** (no match anywhere in `common/src`, `server/src`). |
| `CanonicalPhysicsV1` / `ContactKey` / `StablePairKey` | NOT LIVE | **Unchanged, confirmed still absent.** |
| `CanonicalPersistenceV1` / `PersistenceOpKey` / `SaveUniverseEnvelopeV1` | NOT LIVE | **Unchanged, confirmed still absent.** |
| `NetEnvelopeV1`, `ContentManifestV1`, `BuildManifestV1` (one combined row in the doc) | NOT LIVE | **NEEDS SPLITTING — mixed result.** `NetEnvelopeV1` is now FULLY LIVE: `SemanticProtocolIdV1::NetEnvelopeV1` + the whole `NetEnvelopeHeaderV1`/`SemanticWireFrameV1` machinery, APEX T3.3 (`common/net/src/msg/envelope.rs`, this session). `ContentManifestV1` is substantially live: `common/src/content_manifest.rs`'s `ContentManifest`/`ContentEntry`/`ProvenanceStatement`, per this doc's own `[T0.57]` DONE entry. `BuildManifestV1` alone is still genuinely absent (no match). |
| `CapturedInputEventV1` / `InputFrameV1` | NOT LIVE | **Unchanged, confirmed still absent.** |
| `DomainHasher` / `stable_hash_u64` | ABSENT; DESIGN/UNMERGED ONLY | **STALE.** No literal type by that name, but `common/src/apex/digest/mod.rs` (APEX T0.3) is the functional equivalent — canonical, domain-separated hashing. Any row blocked specifically on "no domain hasher" needs re-checking against `apex::digest`. |
| A* total frontier key `(f,h,g,node,sequence)` | ABSENT | **STALE.** `common::astar::PathEntry` (`common/src/astar.rs:11-26`) already carries `cost_estimate`/`heuristic`/`cost`/`node_hash` — this doc's own `[DONE.1]` entry. |
| Cross-producer stamped/canonical `EventBus` merge | ABSENT | **STALE.** `EventStamp` (`common/src/event.rs:590`) + `EventBus::recv_all_mut`'s own merge-sort — this doc's own `[T0.29]`/`[T0.30]` DONE entries (`5905a44c3727`). |
| Shared generation-stamped async work/acceptance service | ABSENT | **STALE.** `common/src/async_work.rs` (`AsyncOwnerKey`, `AsyncGeneration`, exhaustive `AsyncTerminal`) — this doc's own `[T0.51]` DONE entry. |
| `AsyncResultEnvelopeV1`, `LifecycleOpKey`, `PresentationWorkKey`, etc (first-rewrite inventions) | NEW INVENTIONS, NOT ESTABLISHED | **Unchanged, still an accurate naming-hygiene note** (not a liveness claim to recheck — these were always meant as "don't treat these prior-pass names as real corpus types," which still holds). |

**Net result: 4 of 13 rows fully stale, 1 row (the combined NetEnvelope/Content/Build row) needs
splitting into 2 live + 1 still-absent. 5 of 13 need the table edited.** Every Tier 1–3 row citing
`DomainHasher`, the A* total key, cross-producer `EventBus` merge, the async acceptance service,
`NetEnvelopeV1`, or `ContentManifestV1` as its blocker should have that specific blocker
re-verified before being counted as still-blocked in the full sweep.

## 1c. Candidate A formal closures

- **T0.67 — CLOSED as substantially satisfied.** The deterministic-flag infrastructure
  (`DETERMINISTIC_WORLDGEN`/`DETERMINISTIC_RTSIM`, `BASTION_DETERMINISTIC` opt-in) is live and
  load-bearing — the same live-game determinism gap this session's own memory records finding
  and fixing (OS-entropy `tick_rng` fallback) proves the mechanism is genuinely exercised, not
  just declared. Scope note: this closes the MECHANISM (one opt-in flag threading through
  live+test), not a claim that every individual RNG/algorithm seam in the engine is audited —
  that's the ongoing T0.32-42-class work, tracked separately, already substantially DONE per this
  same document.
- **T0.74 — NOT CLOSED, stays fully open, AND reclassified `UNBLOCKS-VIA-APEX-T6` /
  cross-program-duplicate (Fable's own addition).** Corrected from my own first-draft overclaim
  (see 1a table above): `apex/scalar.rs` explicitly EXCLUDES floats by design (compile-fail proof
  at line 266). Cross-platform float computation determinism (the actual ask —
  physics/path/worldgen floats, a Box2D-style contract) is untouched by APEX today, BUT it is
  exactly apex T6's own declared territory (T6.1 transcendental inventory, T6.2 dual bit/semantic
  probes, T6.4 `NumericProfileV1`, T6.5 deterministic kernels) — this row IS apex-T6 work seen
  from the engine side, not a separate build. Recorded here so neither lane builds it twice; joins
  the four mechanism clusters below as the fifth convergence-map entry.
- **T0.84 — PARTIALLY CLOSED, one concrete named gap.** `apex/scalar.rs` + `apex/identity/`
  satisfy persisted/network IDs, binary/hash inputs, and protocol DTOs. Still open: `world_seed`
  (`server/src/settings/mod.rs:184`) is a plain `u32`, not an apex-style fixed-width newtype with
  its own canonical expansion — the row's "world seed expansion" sub-clause is real remaining
  work, narrow in scope (one field + its derivation path).

## 1d. Cross-program disposition note (Fable's own ruling)

The five-entry convergence map (four mechanism clusters + T0.74's own float contract):

| Absent mechanism / row | Maps onto |
|---|---|
| `CanonicalScheduleV1` / `OperationKey` | Opus's APEX frontier, scheduling-adjacent tiers |
| `CanonicalPhysicsV1` / `ContactKey` / `StablePairKey` | apex T6 (physics ordering) |
| `CanonicalPersistenceV1` / `PersistenceOpKey` / `SaveUniverseEnvelopeV1` | apex T4 (saves/persistence) |
| `CapturedInputEventV1` / `InputFrameV1` | apex T5 (input frames) |
| T0.74's float computation contract | apex T6 (T6.1 transcendental inventory, T6.2 dual bit/semantic probes, T6.4 `NumericProfileV1`, T6.5 deterministic kernels) |

The four mechanism clusters confirmed genuinely still-absent in section 1b map almost exactly onto
Opus's remaining APEX frontier (T4 saves/persistence, T5 input frames, T6 physics ordering), and
T0.74 itself is apex-T6 territory seen from the engine side. Any Tier 1–3 row blocked on one of
these five should be dispositioned **`UNBLOCKS-VIA-APEX-T4/T5/T6`** in the
full sweep below, not `BLOCKED, needs build` — these clusters unblock progressively as APEX
advances, with no engine-side construction required. This changes their disposition from WORK to
WAITING and keeps the two programs' sequencing coupled honestly (Fable's own framing). Applied
in the sweep table below wherever one of these four clusters is the cited blocker.

## 2. Shared-mechanism cluster check (Tier 1–3's actual unblock leverage)

Per Fable's own steer ("clusters over row-by-row where the blocker is shared") — checked the
`MC-*` mechanism catalog's own claimed-absent items against the current tree, since Tier 1–3's
238 blocked rows cite these by name as prerequisites:

| Mechanism | Doc's own claim (2026-07-22 baseline) | Current tree finding |
|---|---|---|
| `DomainHasher` / `stable_hash_u64` | ABSENT ON LIVE BRANCH | **STALE.** No literal `DomainHasher` type, but `common/src/apex/digest/mod.rs` (APEX T0.3, domain-separated digests) is the functional equivalent — a canonical, domain-separated hashing primitive. Any Tier 1–3 row blocked specifically on "no domain hasher exists" should be re-examined against `apex::digest` before assuming it's still blocked. |
| Shared generation-stamped async work envelope | ABSENT ON LIVE BRANCH | **STALE, and the document's OWN LATER SECTION already says so.** `common/src/async_work.rs` (`AsyncWorkRequest`, `AsyncOwnerKey`, `AsyncGeneration`, exhaustive `AsyncTerminal`) is DONE per this same document's own `[T0.51]` entry (`41c7897c8f`→`bbe94e570f`→`b4afb1772b05`). The mechanism-catalog TABLE near the top of the document was not reconciled against the document's OWN later DONE rows — an internal staleness independent of the tree, catch worth flagging back. Any row blocked on MC-ASYNC's own prerequisite is very likely unblocked NOW. |
| A* total frontier key `(f,h,g,node,sequence)` | ABSENT ON LIVE BRANCH | **STALE, same internal-inconsistency class.** `common::astar::PathEntry` (`common/src/astar.rs:11-26`) already carries `cost_estimate`/`heuristic`/`cost`/`node_hash` — exactly this key, shipped as `[DONE.1]` in this same document. MC-NAV's own "ABSENT LIVE" framing needs the same reconciliation pass. |
| `OperationKey` / `CanonicalScheduleV1` (MC-SCHEDULE) | REAL CORPUS PROPOSAL; NOT LIVE | **Confirmed still absent** — no match anywhere in `common/src` or `server/src`. This one really is new work; the doc's own claim holds up here. |

**Actionable takeaway:** the mechanism-catalog table at the top of the master list needs its OWN
internal reconciliation pass against the DONE section further down in the SAME document, separate
from reconciling against the live tree — two of the four checked mechanisms were stale against
the document's own later content, not just against APEX's newer work. Recommend this as part of
whatever process regenerates/maintains this list going forward.

## 3. Batch E2 proposal — held for Fable's ruling, not built

Given the depth achieved this pass, I have HIGH confidence in exactly one class of "genuinely
buildable, highest-value" candidate and want your ruling on scope before touching anything:

**Candidate A — T0 reclassification pass (not a Tier-1 feature build, but real, bounded,
high-confidence work):** formally close T0.67, T0.74, T0.84 as SATISFIED-BY-APEX-T0.1-4 (with a
short note on what, if anything, is still engine-specific beyond APEX's own scope), and
reconcile the mechanism-catalog table against the document's own DONE section (DomainHasher →
apex::digest, MC-ASYNC → T0.51/async_work.rs, A* total key → DONE.1/astar.rs). This is fast,
low-risk, and immediately shrinks the stale-count Fable already predicted was inflated.

**Everything else (genuinely NEW Tier-1 feature rows) needs the fuller row-by-row sweep before I'd
trust a "highest-value, unblocked" ranking** — proposing specific Tier-1 rows off an incomplete
unblock scan would repeat exactly the kind of overclaim this whole document's own adversarial
fact-check exists to catch. Requesting: either (a) green-light Candidate A now while I continue
the full Tier 1–3 sweep in the background of whatever E2 becomes, or (b) hold everything for one
combined ruling once the sweep is complete.
