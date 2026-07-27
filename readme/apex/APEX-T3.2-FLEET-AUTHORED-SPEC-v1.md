# APEX-T3.2 — Fleet-authored spec: logical session and connection epochs

> **STATUS: DRAFT — pending cross-review.** Author: Builder Sonnet 5,
> 2026-07-27. Not build-authorized. Per Ben's order (routed via Fable):
> author → Opus 5 spec-review → Fable approval → **then** build starts.
> Registry disposition `specification=FLEET_AUTHORED` per schema doc
> section 6a.
>
> **Grounding trust posture (Fable's standing ruling):** inline
> master-order content is admissible grounding, never inherited
> authority; every code-facing claim below is checked against live code,
> not read off the master order's prose. Landed code wins on conflict.

## 0. Provenance

The standalone prose packet
(`PROJECT-BASTION-APEX-MICROSTEP-APEX-T3.2-AUTHENTICATED-LOGICAL-SESSIONS-CONNECTION-EPOCH.md`)
is `.gdoc`-only (unexported), same hallucination-class as `T0.5`'s. **Not
consulted, not even for inspiration.**

Unlike `T0.5`, this row's **canary vector file survived intact**:
`PROJECT-BASTION-APEX-T3.2-SESSION-EPOCH-CANARIES-v2.json` is a real,
non-`.gdoc` file on Drive. Independently re-hashed before trusting it: its
raw SHA-256 (`b2d0ce00e2ac81e63964ffe56ed60e179693e646ab9cfc5112572617017a8e00`)
and byte count (19,306) both match the master order's own printed pin
exactly (row-summary line, master order line 49290) — this is a verified,
not assumed, provenance distinction from the prose packet. It contains
**128 typed adversarial cases** (`schema:
bastion.apex.t3.2-session-epoch-canaries/v2`, `live_commit:
f7b30de6d916930c96f181919160ff7839aa6d5b`), grouped `identity` (10),
`new-admission` (15), `resume-epoch` (25), `attempt-order` (15),
`capacity` (19), `retention` (21), `transport-fencing` (9),
`wire-boundary` (14). This is this row's primary grounding — richer and
more concrete than the terse inline master-order block, and unlike that
block, independently provenance-verified rather than merely present.

Also grounded in:
- The master order's own inline row block (lines 197214-202347):
  7-step build sequence + acceptance text, reproduced in section 1.
- Live code, verified this session (not recalled from memory):
  `common::apex::identity::{SessionId, ConnectionEpoch}` **already
  exist** (`T0.4`) — `SessionId` is an opaque UUIDv4 lifecycle identity
  (`common/src/apex/identity/opaque.rs`), `ConnectionEpoch` is a
  zero-reserved monotonic `u64` counter with `INVALID`/`FIRST` constants
  (`common/src/apex/identity/counter.rs`). `T3.2`'s job is **not** to
  invent these types — they're built — but to define the session
  lifecycle policy around them and wire them into the live registration
  path, the same shape as `T3.1`'s `ServerBootId` wiring.
  `common_net::msg::server::ServerRegisterAnswer = Result<(),
  RegisterError>` today (`common/net/src/msg/server.rs:97`) — canary
  SES-118 requires this become a typed `SessionAdmission` payload on
  success. `server/src/login_provider.rs`'s `LoginProvider`/`PendingLogin`
  already does async credential/ban/whitelist verification, returning
  `(username, principal Uuid)`; no session registry, capacity accounting,
  or resume/epoch handling exists yet. `server/src/sys/msg/register.rs:134-250`
  already has a `max_players` capacity check (`old_player_count +
  guard.0.len() >= max_players`) — canary SES-083 (`BLOCK-CAPACITY-IN-LOGIN-PROVIDER`)
  requires this be re-derived through the new `SessionRegistry`, not
  computed ahead of it as today.

**Correction, caught re-checking my own first draft:** this section
originally claimed no finding cites `APEX-T3.2` (copying `T0.5`'s
situation without re-verifying it for this row) — that was wrong, and is
recorded here rather than silently fixed, per this program's own
practice of not propagating an unverified claim. Four findings actually
cite `APEX-T3.2` in `readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv`'s
`replacement_rows` / the registry's `closure_rule`:
- `DET-NET-022` (`SupersededBy` `T3.2`+`T3.3`): "do not globalize
  transport `Mid`; the open contract is app-level session/epoch/
  sequence" — directly addressed by policy 3/§4 below (`Cid`/`Mid` never
  drive epoch or session identity).
- `DET-NET-024` (`SupersededBy` `T3.2`+`T3.3`): "remove `Cid` from
  semantic identity/ordering rather than stabilizing arrival allocation"
  — same policy 3 coverage.
- `DET-NET-025` (`AllOf` `T3.2`+`T4.1`, `PARTIAL`): `best_protocol`'s
  existing deterministic-class-preference-over-arrival-`Cid` shape is "a
  real partial primitive; explicit capability/transport policy remains
  open" — `T3.2` alone does not close this one (needs `T4.1` too, per
  its own `AllOf`); this row's session/epoch policy is its contribution,
  not the whole closure.
- `DET-NET-026` (`AllOf` `T3.1`+`T3.2`+`T3.3`, `OPEN`): "no application
  session state machine or full-bootstrap reconnect contract exists" —
  the `SessionRegistry`/`SessionRecordV1` machinery in §3-4 below **is**
  the application session state machine; full-bootstrap reconnect is a
  later row's job (`T9.1`, "Full-bootstrap reconnect under new connection
  epoch" per the registry), consistent with §7's non-goals.

Registry `hard_dependencies`: `APEX-T3.1` only.

## 1. Row block, reproduced verbatim (master order lines 197214-202347)

> **[APEX-T3.2] Logical session and connection epochs**
>
> Create authenticated `SessionId` and one-active-attachment
> `ConnectionEpoch`; retain transport CIDs/MIDs as diagnostics only.
>
> **Research classification:** SPEC-COMPLETE / PREREQUISITE-MISSING.
> **Prerequisite:** `T3.1`.
>
> **Build steps:**
> 1. Issue an authenticated `SessionId` after successful
>    registration/authentication.
> 2. Start `ConnectionEpoch(1)` on first active attachment.
> 3. Increment epoch on every new TCP/QUIC application attachment.
> 4. Permit one active epoch; atomically revoke the older one.
> 5. Keep QUIC path migration inside the same epoch.
> 6. Reject all mutating/staged traffic from old epochs.
> 7. Test concurrent reconnect race and split-brain attachment.
>
> **Acceptance:**
> - Transport CIDs/MIDs never serve as session identity.
> - Old-epoch traffic is mechanically rejectable.

## 2. Determinism story

### 2.1 What "deterministic admission order" does and does not mean

Stated precisely, because this is the crux Fable's elevated-scrutiny
ruling asks this section to answer directly: **T3.2 does not, and could
not, make the real-world arrival order of two different clients'
packets deterministic across different real runs.** Network jitter, OS
scheduling, and the auth server's own response latency are genuine
physical facts of a given run; nothing in this row changes that, and
claiming otherwise would be exactly the kind of overclaim §4.6 exists to
forbid. What T3.2 *does* guarantee, and what "deterministic" means for
this row: **given whatever set of authenticated intents actually
completed within one server tick's admission phase — a real, run-specific
fact — the server's disposition of that set (commit order, capacity
allocation, replacement-vs-reject decisions) is a pure, replayable
function of the set's own content, never of which real async task
happened to finish first.** This is the same scoping this program has
always used for "determinism": hash/reason about canonical logical state,
never wall-clock or worker-order incidentals (the permanent
determinism-by-construction law) — applied here to *admission-set
membership* (real, non-reproducible) versus *admission-set processing*
(must be reproducible given the set).

### 2.2 The mechanism, and why it holds under real contention

The race that actually happens in production: two clients' `ClientRegister`
messages are drained from the network in some real order, and each
triggers an async `LoginProvider::verify` call (a real HTTP round-trip to
the auth server) that completes at a genuinely unpredictable wall-clock
time relative to the other. If commit order were derived from *which
verify future resolves first*, that would be exactly the live nondeterminism
this row exists to close (this is the direct, current-code reading of
`BLOCK-UNSORTED-INTENTS`, SES-025: "authenticated intents are committed
in worker completion order" is the failure mode, not a hypothetical).

The fix is a strict separation of **when the tie-break key is fixed**
from **when the async race resolves**:

1. `AttachmentAttemptSeq` (§3.3) is allocated at message-**receipt** time
   — inside the single-threaded ECS message-drain phase that reads the
   incoming connection queue, before any `await` on auth/stream setup
   begins (SES-051; allocating it *after* auth completes is
   `BLOCK-LATE-ATTEMPT-SEQUENCE`, SES-052). This phase is not raced: it
   is one system reading one queue in one order, once per tick — the same
   structural guarantee every other `specs` system in this codebase
   already relies on for its own per-tick determinism.
2. Between allocation and commit, each intent's actual authentication
   genuinely races in real wall-clock time — `LoginProvider`'s tokio
   tasks resolve in whatever order the auth server and OS scheduler
   produce. **This real race is real and is not neutralized** — a client
   whose auth happens to finish late may simply land in a later tick's
   admission phase than one that finishes early. That is an accepted,
   correct consequence of §2.1, not a gap.
3. What *is* neutralized: for whichever intents *do* land in the same
   phase, commit order is computed by sorting on `(principal bytes,
   descending attempt_seq)` (SES-053/060) — a key that was **already
   fixed in step 1**, before the race in step 2 even began. The sort
   therefore cannot be influenced by which real future resolved first;
   `BLOCK-HASHMAP-WINNER` (SES-058) and `BLOCK-MUTEX-ARRIVAL-WINNER`
   (SES-059) are the two concrete non-deterministic orderings this
   explicitly forecloses (hash-bucket layout and lock-acquisition order
   are both real per-run artifacts of exactly the kind step 1's
   pre-fixed key makes irrelevant). Registry mutation happens only inside
   this single sorted pass — never inside a `par_bridge` closure or other
   worker-order-dependent path (SES-065, `BLOCK-PARALLEL-REGISTRY-MUTATION`),
   and never directly from an authentication thread (SES-024,
   `BLOCK-NONCANONICAL-ADMISSION`).
4. Same-principal races (a reconnect racing itself, or `New` racing
   `Resume` for one principal) resolve the same way: the **larger**
   captured `attempt_seq` wins regardless of which one's auth actually
   finished first (SES-054/SES-055, `OLDER-ATTEMPT-SUPERSEDED`) — again,
   because the comparison key was fixed before the race, the real
   completion-order race has no vote. A genuine `attempt_seq` collision
   (only possible if two intents were assigned the identical sequence
   number, which the receipt-phase's single-threaded allocation should
   make structurally impossible) is `BLOCK-AMBIGUOUS-ATTEMPT` (SES-056) —
   a hard-fail, not a silent tiebreak, since a collision would mean the
   allocation invariant itself was violated. Two *different* entities
   producing two authenticated intents inside one phase is
   `BLOCK-DUPLICATE-INTENT` (SES-063).
5. Capacity is evaluated **inside** the same sorted pass, not before it:
   `BLOCK-CAPACITY-BEFORE-REGISTRY` (SES-023) forbids rejecting on
   capacity before a same-principal replacement's net-zero delta is known
   (a replacement must never be counted as consuming a new slot just
   because capacity was checked too early). Under real concurrent
   contention for the *last* free slot (SES-075, `BLOCK-CAPACITY-RACE`),
   the sorted pass admits exactly the first canonical-order eligible
   intent for that slot (SES-076) and gives every later one in the same
   phase a typed rejection (SES-077) — again a property of the fixed sort
   key, not of which principal's auth happened to resolve first in real
   time.
6. Session/registry lifecycle reads exactly one `SessionMaintenanceNow`
   snapshot per admission phase, never a per-event wall-clock read
   (SES-091; `BLOCK-MULTIPLE-PHASE-TIMES`, SES-092, forbids each
   disconnect reading the clock independently) — the same "one time-read
   per phase" discipline this program has used since `T3.1`'s boot-scope
   work, now applied to session expiry (`expires_at` values in one phase
   all derive from `phase_now + grace`, SES-093). Expired sessions are
   purged **before** the sorted pass reads registry state (SES-064), so
   an expiring-this-tick session cannot flicker between "present" and
   "absent" depending on iteration order within the same phase.

### 2.3 Identity/state isolation (premise-checked against landed T0.4)

`SessionId`/`ConnectionEpoch` must never enter save data, the simulation
state root, or an authoritative RNG key (SES-005/006/007) — this is the
session-layer restatement of the same isolation `T3.1` already applies to
`ServerBootId` ("explicitly excluded from deterministic simulation
state/RNG", per that row's own scope decision). Verified against landed
`T0.4` code, not assumed: `common::apex::identity::opaque`'s manual
`byte_order` function (unsigned lexicographic comparison of the raw 16
UUID octets, deliberately *not* derived from `uuid::Uuid`'s own `Ord` —
see that module's own doc comment) is exactly the tie-break mechanism
SES-099 calls for ("tie-break uses canonical SessionId bytes") — no
divergence found; this row reuses that existing `Ord` impl directly
rather than defining a new comparison.

## 3. Data model

### 3.1 Reused as-is (already built, `APEX-T0.4`)

- `SessionId` — opaque UUIDv4, `common::apex::identity::SessionId`.
  Fresh, distinct from principal/boot/transport/ECS/save/build identities
  (SES-001); server-generated only, never client-chosen or
  time/address-derived (SES-008/SES-009); collision-retry is bounded, not
  infinite (SES-010).
- `ConnectionEpoch` — zero-reserved monotonic `u64`,
  `common::apex::identity::ConnectionEpoch`. `FIRST` on first active
  attachment (build step 2); `checked_next()` on every new attachment
  (build step 3); exhaustion at `u64::MAX` is a typed terminal
  (SES-034/SES-057), never silent wraparound (SES-043 explicitly forbids
  wrap-to-zero).

### 3.2 New: `SessionRequestV1` (client → server intent)

```
enum SessionRequestV1 {
    New,
    Resume { locator: SessionId, expected_epoch: ConnectionEpoch },
}
```
Carried inside `ClientRegister` (extends `T3.1`'s existing struct — see
§5) alongside the already-present `expected_server_boot_id`. Client type
and the request itself are immutable once the intent is authenticated
(SES-061); mutating either between authentication and commit is a
`BLOCK-TOCTOU-INTENT` (SES-062).

### 3.3 New: `SessionAttemptSeqV1`

A per-process monotonic counter (not persisted, not part of any
canonical digest — this is admission-ordering machinery, not simulation
state), allocated at message-receipt time, before authentication starts
(SES-051). Overflow is a typed terminal (`ATTEMPT-SEQUENCE-EXHAUSTED`,
SES-057), a collision between two attempts is `BLOCK-AMBIGUOUS-ATTEMPT`
(SES-056) — fleet design decision: implemented as a
`zero_valid_counter!`-style wrapper over `u64` (same convention as
`T0.4`'s `PhysicsGeneration`/`SnapshotEpoch`), since unlike
`ConnectionEpoch`, zero is a perfectly valid first sequence number here
(no reserved-zero semantics implied by the canary corpus).

### 3.4 New: `SessionRecordV1` (server-side registry entry)

```
struct SessionRecordV1 {
    session_id: SessionId,
    principal: Uuid,                 // authc principal, existing type
    client_type: ClientTypeV1,       // existing ClientType, not re-invented
    epoch: ConnectionEpoch,
    attempt_seq: SessionAttemptSeqV1,
    state: SessionStateV1,
    expires_at: Option<PhaseTimeV1>, // None while Active; Some while Detached
}

enum SessionStateV1 { Active, Detached }
```
`ClientTypeV1` reuses whatever the live `ClientType` enum already is
(SilentSpectator/privileged-bot/etc. per SES-015/SES-016/SES-035..038) —
**not** re-invented; a build-time task is confirming the exact live name/
shape (fleet design decision deferred to implementation, not spec-blocking
since the canary corpus only constrains *behavior* around client type,
not its representation).

### 3.5 New: `SessionAdmissionV1` (replaces bare `()` on
`ServerRegisterAnswer`'s success arm — SES-116/SES-118)

```
enum SessionAdmissionV1 {
    Created { binding: SessionBindingV1 },
    Resumed { binding: SessionBindingV1 },
    Replaced { binding: SessionBindingV1 },  // same-principal New at full capacity, net delta 0 (SES-019/SES-070)
}

struct SessionBindingV1 {
    session_id: SessionId,
    epoch: ConnectionEpoch,
}
```
`GameSync` repeats the identical `SessionBindingV1` (SES-117) — the
client must check `RegisterAnswer`'s binding equals `GameSync`'s binding
**before** constructing `State` (SES-046, directly analogous to `T3.1`'s
own `check_game_sync_boot_scope` pattern — same shape, new field). A
`RegisterAnswer`/`GameSync` pair carrying *different* bindings is
`BLOCK-PARTIAL-ADMISSION` (SES-045).

### 3.6 New: typed terminals (fleet-authored `RegisterError` variants +
in-process session-admission outcomes)

The canary corpus names ~35 distinct terminal identifiers (§ full list in
the fixture). Not every one becomes a wire-level `RegisterError` variant —
several are **in-process invariants** the implementation must uphold
(e.g. `BLOCK-HASHMAP-WINNER`, `BLOCK-HASHMAP-WINNER`,
`BLOCK-HASHMAP-WINNER` are non-vacuity requirements on the sort, not
things a client ever observes over the wire) versus **client-observable
outcomes** (`SESSION-CREATED`/`SESSION-RESUMED`/`ACTIVE-CAPACITY-UNAVAILABLE`/
`SESSION-CLIENT-TYPE-MISMATCH`/etc., which do need a wire representation).
Section 6 below classifies every wire-observable terminal; the purely
structural ones become hostile-test assertions on the implementation, not
new `RegisterError` variants.

## 4. Policy (fleet-authored, cross-review targets)

0. **Session generation is post-auth only**: `SessionId` is issued after
   `LoginProvider` returns success — never before, never speculatively
   (SES-022, `BLOCK-PREAUTH-SESSION-COMMIT`; this is build step 1 made a
   hard invariant, not just a sequencing preference). Baseline
   already-correct behavior this row does not change: credential
   failure/ban/whitelist rejection (SES-012/013/014, pre-existing
   `RegisterError` variants), `ChatOnly`/unprivileged-bot admission below
   capacity (SES-020/021, ordinary `SESSION-CREATED` — no new terminal).
1. **Capacity**: active capacity counts active player-bearing sessions
   only (SES-066); detached sessions never consume `max_players` slots
   (SES-067); admin sessions are capacity-exempt for both `New` and
   detached-`Resume` (SES-017/SES-049/SES-068/SES-078/SES-079).
   Same-principal replacement (`New` while that principal already holds
   an active session) has capacity delta **0**, never counted as a second
   slot (SES-019/SES-070/SES-074 — `BLOCK-DOUBLE-COUNT` explicitly
   forbids counting both old and new attachments after replacement).
   Capacity is decided **through the `SessionRegistry` lookup**, never
   ahead of it in `LoginProvider` (SES-083) — this is the one live-code
   reordering this row requires of the existing `max_players` check.
2. **Resume**: requires exact match of boot ID (already `T3.1`'s job —
   reused, not re-derived), authenticated principal, expected epoch, an
   unexpired detached record, and exact client type (SES-026). Any
   mismatch is a distinct typed terminal, never a generic failure
   (SES-028 principal / SES-029 boot / SES-030-031 expiry / SES-032-033
   epoch staleness-vs-futureness / SES-035-038 client-type). A resume
   that fails must never silently fall through to `New` (SES-041,
   `BLOCK-SILENT-RESUME-TO-NEW`) and must never succeed without
   reauthentication (SES-040, `BLOCK-CREDENTIALLESS-RESUME` — possession
   of the locator UUID alone is not a bearer credential, SES-050).
3. **Epoch fencing**: exactly one active epoch per session; the old
   attachment is atomically revoked, and the new attachment must not
   become `Active` before the old one is marked `Superseded`
   (SES-044, `BLOCK-FENCE-ORDER`). A superseded client's messages must be
   rejected at the central receive gate **before** deserialization/handler
   side effects run (SES-111/SES-112/SES-113 — `BLOCK-GATE-AFTER-HANDLER`
   explicitly forbids checking attachment state only after a handler has
   already mutated game state). Transport-level identity (`Cid`/`Mid`)
   never drives epoch or session identity (SES-003/SES-004/SES-109,
   `BLOCK-TRANSPORT-ID-AS-EPOCH`); QUIC path migration inside the same
   `Participant`/epoch is not a resume (SES-106/SES-110,
   `BLOCK-PATH-MIGRATION-AS-RESUME`); an additional channel on an
   existing `Participant` does not advance the epoch (SES-107); a new
   `Participant` attachment only ever advances the epoch through an
   authenticated `Resume`, never implicitly (SES-108).
4. **Retention/eviction**: a session detaches (not closes) on
   `NetworkError`/timeout (SES-085/SES-086), and closes outright on
   client-requested disconnect, kick, replacement-supersedes-old, or
   invalid client type (SES-087/SES-088/SES-089/SES-090). Detached-slot
   retention is capacity-bounded and **deterministic under contention**:
   ties break on `expires_at` (greatest wins) then canonical `SessionId`
   bytes (SES-099) — `HashMap`-iteration-order eviction is explicitly
   forbidden (SES-100, `BLOCK-NONDETERMINISTIC-EVICTION`), and an
   implementation that exposes *both* an eviction path and a
   `DetachedCapacityUnavailable`-style rejection without one canonical
   rule choosing between them is itself a defect (SES-101,
   `BLOCK-UNSPECIFIED-EVICTION` — this row picks retention-by-tie-break,
   never eviction-by-ambiguity). Disconnect handling keys off the
   **current epoch's binding**, not raw entity ID — a stale disconnect
   for an old binding is a registry no-op (SES-102), and disconnecting by
   entity ID alone must never delete a *newer* session bound to that
   entity (SES-104, `BLOCK-ENTITY-ONLY-DISCONNECT`). The registry is
   **memory-only**: empty again after any server restart (SES-105) — no
   persisted session state, matching this row's explicit non-goal (no
   durable/reconnect-token sessions, §7).
5. **Wire boundary**: `ClientRegister` already carries
   `expected_server_boot_id` (`T3.1`) — this row adds the `SessionRequestV1`
   alongside it, not a parallel message. `RegisterAnswer`'s success arm
   carries `SessionAdmissionV1` (§3.5), replacing bare `()`
   (`BLOCK-WIRE-BINDING-MISSING`, SES-118). A schema change here requires
   the existing network minor-version bump discipline
   (`network/protocol/src/types.rs`'s `VELOREN_NETWORK_VERSION`, already
   bumped once for `T3.1`) — landing this without one is
   `BLOCK-PROTOCOL-VERSION` (SES-120); old/new client-server pairs must
   fail cleanly during the version handshake, not admit and then
   misbehave (SES-121). Resume state lives in client memory only —
   surviving a client **retry** but not a client **process restart**
   (SES-122/SES-123); nothing is ever written to disk
   (`BLOCK-PERSISTENT-SESSION-SECRET`, SES-124).
6. **Explicit non-overclaims** (the corpus itself names these — the
   fleet spec inherits the restraint, does not relax it): this row does
   **not** claim NIST authenticated-session conformance — a bare locator
   with no session secret and no overall (not just per-epoch) timeout
   does not meet that bar, and the spec must not claim it does
   (SES-125, `BLOCK-NIST-CONFORMANCE-OVERCLAIM`); does **not** claim all
   stale wire messages are rejected without `NetEnvelopeV1` — that's
   `T3.3`'s job (SES-126, `BLOCK-T3.3-OVERCLAIM`); does **not** claim
   cross-stream chronology — that's `T3.4`'s (SES-127,
   `BLOCK-T3.4-OVERCLAIM`). `T3.2`'s epoch fencing is **in-process only**;
   universal wire envelope fencing is a later row's job (SES-114).

## 5. Live wire integration (concrete files)

- `common/net/src/msg/client.rs`: `ClientRegister` gains a
  `session_request: SessionRequestV1` field alongside
  `expected_server_boot_id`.
- `common/net/src/msg/server.rs`: `ServerRegisterAnswer` becomes
  `Result<SessionAdmissionV1, RegisterError>` (was `Result<(),
  RegisterError>`); `RegisterError` gains typed variants for every
  wire-observable rejection in §6; `ServerInit::GameSync` gains
  `session_binding: SessionBindingV1`, mirroring how it already repeats
  `server_boot_id` (`T3.1`).
- `server/src/sys/msg/register.rs`: the existing `max_players` check
  (line ~134-250) is re-derived through the new `SessionRegistry`
  resource rather than computed independently; the sorted-commit pass
  (§2) replaces whatever ordering the current admission loop uses.
- `server/src/login_provider.rs`: `PendingLogin`/`LoginProvider` are
  unchanged in their auth/ban/whitelist role — capacity/session decisions
  move **out** of this layer and into the new registry (policy 1, SES-083).
- `client/src/lib.rs`: `register()` sends the client's `SessionRequestV1`
  (New on first connect, Resume with the last-known binding on a retry —
  never across a process restart, policy 5); the `check_game_sync_boot_scope`-adjacent
  binding-equality check (§3.5) is added alongside the existing boot-scope
  check.
- `network/protocol/src/types.rs`: `VELOREN_NETWORK_VERSION` minor bump
  (policy 5).

## 6. Wire-observable terminals (client-visible; become `RegisterError`
variants or `SessionAdmissionV1` arms)

| Terminal | Canary | Wire shape |
|---|---|---|
| `SESSION-CREATED` | SES-011 | `SessionAdmissionV1::Created` |
| `SESSION-RESUMED` | SES-026 | `SessionAdmissionV1::Resumed` |
| `SESSION-REPLACED` | SES-019 | `SessionAdmissionV1::Replaced` |
| `ACTIVE-CAPACITY-UNAVAILABLE` | SES-018 | `RegisterError` variant |
| `SESSION-CLIENT-TYPE-MISMATCH` | SES-035 | `RegisterError` variant |
| `UNKNOWN-SESSION` | SES-027 | `RegisterError` variant |
| `SESSION-PRINCIPAL-MISMATCH` | SES-028 | `RegisterError` variant |
| `SESSION-BOOT-MISMATCH` | SES-029 | reuses existing `ServerBootMismatch` (`T3.1`) |
| `SESSION-EXPIRED` | SES-030 | `RegisterError` variant |
| `STALE-CONNECTION-EPOCH` / `FUTURE-CONNECTION-EPOCH` | SES-032/033 | `RegisterError` variant(s) |
| `CONNECTION-EPOCH-EXHAUSTED` | SES-034 | `RegisterError` variant |
| `INVALID-CLIENT-TYPE` | SES-015 | `RegisterError` variant |
| `AUTH-FAILED` / `BANNED` / `NOT-ON-WHITELIST` | SES-012/013/014 | already exist (`RegisterError`, pre-`T3.2`) |

Everything else in the 128-case corpus not listed here is an in-process
structural invariant (ordering, capacity-accounting, eviction-determinism,
gate-placement) proven by a hostile test against the implementation, not
a new wire terminal.

## 7. Non-goals (corpus-confirmed, §4.6 above)

NIST authenticated-session conformance. Universal wire-envelope stale-
message rejection (`T3.3`). Cross-stream chronology (`T3.4`). Persisted/
durable sessions, reconnect tokens, session secrets on disk, command
replay, prediction-state carryover, or cluster/multi-server session
sharing — all explicitly deferred (SES-128).

## 8. Acceptance (verbatim + corpus-derived checklist)

- [ ] Transport CIDs/MIDs never serve as session identity (master-order
      acceptance; SES-003/004/109).
- [ ] Old-epoch traffic is mechanically rejectable, checked before any
      handler side effect (master-order acceptance; SES-111/112/113).
- [ ] Session admission commit order is sorted (principal bytes, then
      descending attempt sequence / canonical principal order) — never
      insertion, `HashMap`, or mutex-arrival order (SES-053/058/059/060).
- [ ] Capacity accounting has zero double-counting and zero negative
      deltas under same-principal replacement (SES-073/074).
- [ ] Detached-session retention is deterministic under contention
      (expires_at, then SessionId tie-break; SES-099/100/101).
- [ ] A resume can never succeed without reauthentication, and a failed
      resume never silently becomes a `New` (SES-040/041).
- [ ] `RegisterAnswer` and `GameSync` carry identical bindings; the client
      checks equality before constructing `State` (SES-045/046).
- [ ] The session registry is memory-only, empty after every restart
      (SES-105).
- [ ] Every non-goal in §7 stays unclaimed (SES-125/126/127/128).

## 9. Live-code BEFORE/AFTER behavioral delta (Fable's elevated-scrutiny requirement 1)

This is the program's first row editing live server *behavior* (session
admission), not adding leaf types alongside it (`T3.1` added a field and
a rejection path; it did not change what a normal successful connection
experiences). Every touched file, exact delta, and what a real client
observes:

**`server/src/sys/msg/register.rs` (`max_players` check, lines ~134-250):**
- **Before:** capacity is checked directly against `old_player_count +
  guard.0.len() >= max_players` at admission time, with no concept of
  same-principal replacement, detached sessions, or resume. A reconnecting
  player counts as a brand-new slot consumer even if their old connection
  is still technically present; two connections racing for the last slot
  are decided by whatever order the existing loop processes them in
  (today: an ordinary iteration order, not adversarially audited for this
  row's determinism bar).
- **After:** the same numeric threshold (`max_players`) is preserved
  exactly, but capacity is evaluated *through* the new `SessionRegistry`
  (policy 1): same-principal replacement is capacity-neutral (delta 0),
  detached sessions don't count against the active limit, and contested
  final-slot admission is resolved by the sorted commit pass (§2.2 item
  5), not iteration order.
- **Client-observable difference:** a legitimate reconnect (same
  principal, valid resume) that previously might have been rejected as
  "server full" (because the old session's slot wasn't recognized as
  already theirs) now succeeds via `SessionAdmissionV1::Resumed`/`Replaced`.
  A genuinely new player at capacity still gets rejected — same outcome,
  now via a typed `ACTIVE-CAPACITY-UNAVAILABLE` `RegisterError` instead of
  whatever generic path exists today. No previously-succeeding connection
  attempt starts failing; no previously-failing one is silently masked
  (SES-082, `BLOCK-LIVE-BEHAVIOR-REGRESSION`, is this row's own named
  guard against exactly that risk — a hostile test asserts it directly).

**`server/src/login_provider.rs` (`LoginProvider`/`PendingLogin`):**
- **Before:** owns credential/ban/whitelist verification only; no
  capacity or session concept.
- **After:** unchanged in scope — this row does not touch auth logic
  itself, only removes any capacity decision that might currently live
  here or immediately downstream of it before the registry lookup
  (SES-083). If no such early capacity check exists in this exact file
  today, this delta is a no-op for it (confirmed at implementation time,
  not assumed).
- **Client-observable difference:** none — auth failure/ban/whitelist
  rejection behavior is byte-for-byte unchanged (SES-012/013/014 are
  explicitly baseline, not new).

**`common/net/src/msg/client.rs` (`ClientRegister`):**
- **Before:** carries `expected_server_boot_id` only (post-`T3.1`).
- **After:** gains `session_request: SessionRequestV1` (`New` or
  `Resume{locator, expected_epoch}`).
- **Client-observable difference:** a network-protocol-version bump
  (policy 5) — an old client talking to a new server (or vice versa)
  fails cleanly at the version handshake (SES-121), never partially
  admits with a missing field. Every *new* client always sends `New` on
  first connect and only ever sends `Resume` with a binding it actually
  holds in memory from the same process (never across a restart,
  SES-122/123) — from the player's perspective, first connect behaves
  identically to today; a mid-session network hiccup that recovers within
  the detached-retention window now reconnects to the *same* character/
  session state instead of what currently happens (a fresh connection
  with no continuity concept, since none exists yet).

**`common/net/src/msg/server.rs` (`ServerRegisterAnswer`, `ServerInit::GameSync`):**
- **Before:** `ServerRegisterAnswer = Result<(), RegisterError>`;
  `GameSync` carries `server_boot_id` only.
- **After:** `Result<SessionAdmissionV1, RegisterError>`; `GameSync`
  gains `session_binding: SessionBindingV1`, checked for equality against
  `RegisterAnswer`'s binding before `State` construction (SES-045/046).
- **Client-observable difference:** none for a successful connection's
  actual gameplay — the extra payload is consumed internally by the
  client's own binding-equality check, never surfaced to the player
  except as a new, more specific error message on the rejection paths
  that already existed in some form (capacity-full, banned, etc.).

**`client/src/lib.rs` (`register()`):** sends `session_request`, performs
the binding-equality check. **Client-observable difference:** none on the
golden path; a connection that would previously silently proceed with a
`GameSync` that (hypothetically, pre-this-row) disagreed with
`RegisterAnswer` now fails fast with a typed error instead of constructing
`State` from inconsistent data (this exact failure mode did not have a
name before this row; `T3.1`'s own `check_game_sync_boot_scope` pattern
is the direct precedent being extended, not invented fresh).

**`network/protocol/src/types.rs` (`VELOREN_NETWORK_VERSION`):** minor
bump. **Client-observable difference:** covered above (policy 5/SES-120/121).

**Net summary for requirement 1:** every legitimate existing connection
path (new player below capacity, banned/unwhitelisted rejection, at-capacity
rejection) produces the *same* accept/reject outcome after this row as
before it — SES-082 makes that a named, tested invariant, not an
assumption. The *new* behavior is additive: resume/reconnect continuity
that does not exist today, and same-principal replacement no longer
double-counting capacity (an existing latent bug class this row also
happens to close, not something it introduces).

## 10. Rollback / flag-gate discipline (Fable's elevated-scrutiny requirement 2)

Per the packet standard's byte-exact-rollback expectation, applied to
this row's live-path edits specifically:

- **Wire schema changes are the hard-to-revert part** (`ClientRegister`/
  `ServerRegisterAnswer`/`GameSync` field additions, network minor-version
  bump) — once a version is shipped, rolling back the *protocol* requires
  another version bump, not just a code revert (this is inherent to any
  wire-format change, not unique to this row; `T3.1` had the exact same
  property with `VELOREN_NETWORK_VERSION` and no flag-gate was used
  there either, for the same reason: a version-negotiated field cannot be
  silently toggled without also toggling the version both sides check).
  Mitigation: land the wire change in one atomic, easily-`git revert`-able
  commit, separate from the `SessionRegistry` policy logic (build-step
  ordering in §5 already reflects this: wire types first, policy second),
  so a revert of the policy commit alone is possible without touching the
  version-negotiated wire shape if only the *policy* (not the schema)
  needs to roll back.
- **`SessionRegistry` policy logic (capacity/retention/ordering) is
  cleanly revertible**: it is new code introduced behind the new registry
  resource, not a rewrite of existing logic in place. Reverting the
  commit(s) that introduce `SessionRegistry`/`SessionRecordV1`/the sorted
  commit pass restores `register.rs`'s prior direct `max_players` check
  exactly, byte-for-byte, since that check is *replaced*, not deleted and
  reimplemented elsewhere in a way that would leave residue.
- **No feature flag is added for the registry itself** — fleet design
  decision, stated explicitly rather than left implicit: unlike
  `BASTION_DETERMINISTIC` (an opt-in for a *parallel* code path that
  coexists with the pre-existing one, used where live-game determinism
  needed to be provable without changing default behavior for players),
  `T3.2`'s session registry has no meaningful "opt-out while still
  running" state — a server either has session/resume/capacity accounting
  or it doesn't; a flag that disabled it mid-flight would leave orphaned
  `SessionRecordV1` entries with no consumer, a worse failure mode than a
  clean revert. The `T1.1.02`-style declared/ambient-fallback pattern
  (used for build-identity stamping) is not analogous here — that pattern
  exists for *build-time* provenance, not *runtime* session policy — so
  it is not reused for this row's rollback story, and that non-reuse is
  itself the honest answer rather than force-fitting an existing pattern
  where it does not apply.
- **Revert path, concretely:** `git revert` of this row's commit(s), in
  reverse order, restores: (a) `register.rs`'s original direct capacity
  check, (b) the pre-`T3.2` wire shapes (paired with a network
  minor-version revert, unavoidable per the first bullet), (c) removal of
  the `SessionRegistry` resource and its systems, cleanly, since nothing
  outside this row's own files depends on `SessionRecordV1`/
  `SessionRegistry` (verified at implementation time: this row does not
  touch save data, simulation state, or RNG per §2.3's own isolation
  guarantee, so nothing else in the codebase can have grown a dependency
  on session state that a revert would strand).

## 11. Canary coverage audit (Fable's grounding-tier ruling: premise-checked, not silently dropped)

Every one of the 128 cases in
`PROJECT-BASTION-APEX-T3.2-SESSION-EPOCH-CANARIES-v2.json`, mapped to the
section of this spec that resolves it. Built from the fixture directly
(not retyped by hand) to avoid transcription drift.

| ID | Terminal | Section |
|---|---|---|
| SES-001 | PASS | §3.1/2.3 |
| SES-002 | BLOCK-IDENTITY-CONFLATION | §3.1/2.3 |
| SES-003 | BLOCK-IDENTITY-CONFLATION | §3.1/2.3 |
| SES-004 | BLOCK-IDENTITY-CONFLATION | §3.1/2.3 |
| SES-005 | BLOCK-IDENTITY-CONFLATION | §3.1/2.3 |
| SES-006 | BLOCK-IDENTITY-CONFLATION | §3.1/2.3 |
| SES-007 | BLOCK-IDENTITY-CONFLATION | §3.1/2.3 |
| SES-008 | BLOCK-INVALID-SESSION-ID | §3.1/2.3 |
| SES-009 | BLOCK-INVALID-SESSION-ID | §3.1/2.3 |
| SES-010 | SESSION-ID-COLLISION-EXHAUSTED | §3.1/2.3 |
| SES-011 | SESSION-CREATED | §4 policy 0/1 |
| SES-012 | AUTH-FAILED | §4 policy 0/1 |
| SES-013 | BANNED | §4 policy 0/1 |
| SES-014 | NOT-ON-WHITELIST | §4 policy 0/1 |
| SES-015 | INVALID-CLIENT-TYPE | §4 policy 0/1 |
| SES-016 | INVALID-CLIENT-TYPE | §4 policy 0/1 |
| SES-017 | SESSION-CREATED | §4 policy 0/1 |
| SES-018 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 0/1 |
| SES-019 | SESSION-REPLACED | §4 policy 0/1 |
| SES-020 | SESSION-CREATED | §4 policy 0/1 |
| SES-021 | SESSION-CREATED | §4 policy 0/1 |
| SES-022 | BLOCK-PREAUTH-SESSION-COMMIT | §4 policy 0/1 |
| SES-023 | BLOCK-CAPACITY-BEFORE-REGISTRY | §4 policy 0/1 |
| SES-024 | BLOCK-NONCANONICAL-ADMISSION | §4 policy 0/1 |
| SES-025 | BLOCK-UNSORTED-INTENTS | §4 policy 0/1 |
| SES-026 | SESSION-RESUMED | §4 policy 2 |
| SES-027 | UNKNOWN-SESSION | §4 policy 2 |
| SES-028 | SESSION-PRINCIPAL-MISMATCH | §4 policy 2 |
| SES-029 | SESSION-BOOT-MISMATCH | §4 policy 2 |
| SES-030 | SESSION-EXPIRED | §4 policy 2 |
| SES-031 | SESSION-EXPIRED | §4 policy 2 |
| SES-032 | STALE-CONNECTION-EPOCH | §4 policy 2 |
| SES-033 | FUTURE-CONNECTION-EPOCH | §4 policy 2 |
| SES-034 | CONNECTION-EPOCH-EXHAUSTED | §4 policy 2 |
| SES-035 | SESSION-CLIENT-TYPE-MISMATCH | §4 policy 2 |
| SES-036 | SESSION-CLIENT-TYPE-MISMATCH | §4 policy 2 |
| SES-037 | SESSION-CLIENT-TYPE-MISMATCH | §4 policy 2 |
| SES-038 | SESSION-CLIENT-TYPE-MISMATCH | §4 policy 2 |
| SES-039 | SESSION-RESUMED | §4 policy 2 |
| SES-040 | BLOCK-CREDENTIALLESS-RESUME | §4 policy 2 |
| SES-041 | BLOCK-SILENT-RESUME-TO-NEW | §4 policy 2 |
| SES-042 | BLOCK-EPOCH-NONMONOTONIC | §4 policy 2 |
| SES-043 | BLOCK-EPOCH-NONMONOTONIC | §4 policy 2 |
| SES-044 | BLOCK-FENCE-ORDER | §4 policy 2 |
| SES-045 | BLOCK-PARTIAL-ADMISSION | §4 policy 2 |
| SES-046 | BLOCK-CLIENT-PUBLISH-BEFORE-EQUALITY | §4 policy 2 |
| SES-047 | SESSION-RESUMED | §4 policy 2 |
| SES-048 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 2 |
| SES-049 | SESSION-RESUMED | §4 policy 2 |
| SES-050 | BLOCK-SESSION-ID-AS-BEARER | §4 policy 2 |
| SES-051 | PASS | §2.2 |
| SES-052 | BLOCK-LATE-ATTEMPT-SEQUENCE | §2.2 |
| SES-053 | PASS | §2.2 |
| SES-054 | OLDER-ATTEMPT-SUPERSEDED | §2.2 |
| SES-055 | OLDER-ATTEMPT-SUPERSEDED | §2.2 |
| SES-056 | BLOCK-AMBIGUOUS-ATTEMPT | §2.2 |
| SES-057 | ATTEMPT-SEQUENCE-EXHAUSTED | §2.2 |
| SES-058 | BLOCK-HASHMAP-WINNER | §2.2 |
| SES-059 | BLOCK-MUTEX-ARRIVAL-WINNER | §2.2 |
| SES-060 | PASS | §2.2 |
| SES-061 | PASS | §2.2 |
| SES-062 | BLOCK-TOCTOU-INTENT | §2.2 |
| SES-063 | BLOCK-DUPLICATE-INTENT | §2.2 |
| SES-064 | PASS | §2.2 |
| SES-065 | BLOCK-PARALLEL-REGISTRY-MUTATION | §2.2 |
| SES-066 | PASS | §4 policy 1/2.2 item 5 |
| SES-067 | PASS | §4 policy 1/2.2 item 5 |
| SES-068 | PASS | §4 policy 1/2.2 item 5 |
| SES-069 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 1/2.2 item 5 |
| SES-070 | SESSION-REPLACED | §4 policy 1/2.2 item 5 |
| SES-071 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 1/2.2 item 5 |
| SES-072 | SESSION-RESUMED | §4 policy 1/2.2 item 5 |
| SES-073 | BLOCK-NEGATIVE-CAPACITY-DELTA | §4 policy 1/2.2 item 5 |
| SES-074 | BLOCK-DOUBLE-COUNT | §4 policy 1/2.2 item 5 |
| SES-075 | BLOCK-CAPACITY-RACE | §4 policy 1/2.2 item 5 |
| SES-076 | PASS | §4 policy 1/2.2 item 5 |
| SES-077 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 1/2.2 item 5 |
| SES-078 | PASS | §4 policy 1/2.2 item 5 |
| SES-079 | PASS | §4 policy 1/2.2 item 5 |
| SES-080 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 1/2.2 item 5 |
| SES-081 | ACTIVE-CAPACITY-UNAVAILABLE | §4 policy 1/2.2 item 5 |
| SES-082 | BLOCK-LIVE-BEHAVIOR-REGRESSION | §4 policy 1/2.2 item 5 |
| SES-083 | BLOCK-CAPACITY-IN-LOGIN-PROVIDER | §4 policy 1/2.2 item 5 |
| SES-084 | PASS | §4 policy 1/2.2 item 5 |
| SES-085 | SESSION-DETACHED | §4 policy 4 |
| SES-086 | SESSION-DETACHED | §4 policy 4 |
| SES-087 | SESSION-CLOSED | §4 policy 4 |
| SES-088 | SESSION-CLOSED | §4 policy 4 |
| SES-089 | SESSION-CLOSED | §4 policy 4 |
| SES-090 | SESSION-CLOSED | §4 policy 4 |
| SES-091 | PASS | §4 policy 4 |
| SES-092 | BLOCK-MULTIPLE-PHASE-TIMES | §4 policy 4 |
| SES-093 | PASS | §4 policy 4 |
| SES-094 | SESSION-EXPIRED | §4 policy 4 |
| SES-095 | PASS | §4 policy 4 |
| SES-096 | SESSION-DETACHED | §4 policy 4 |
| SES-097 | DETACHED-NOT-RETAINED-CAPACITY | §4 policy 4 |
| SES-098 | DETACHED-EVICTED-CAPACITY | §4 policy 4 |
| SES-099 | PASS | §4 policy 4 |
| SES-100 | BLOCK-NONDETERMINISTIC-EVICTION | §4 policy 4 |
| SES-101 | BLOCK-UNSPECIFIED-EVICTION | §4 policy 4 |
| SES-102 | PASS | §4 policy 4 |
| SES-103 | PASS | §4 policy 4 |
| SES-104 | BLOCK-ENTITY-ONLY-DISCONNECT | §4 policy 4 |
| SES-105 | PASS | §4 policy 4 |
| SES-106 | PASS | §4 policy 3 |
| SES-107 | PASS | §4 policy 3 |
| SES-108 | PASS | §4 policy 3 |
| SES-109 | BLOCK-TRANSPORT-ID-AS-EPOCH | §4 policy 3 |
| SES-110 | BLOCK-PATH-MIGRATION-AS-RESUME | §4 policy 3 |
| SES-111 | STALE-ATTACHMENT | §4 policy 3 |
| SES-112 | PASS | §4 policy 3 |
| SES-113 | BLOCK-GATE-AFTER-HANDLER | §4 policy 3 |
| SES-114 | PASS | §4 policy 3 |
| SES-115 | PASS | §3.5/4 policy 5/5 |
| SES-116 | PASS | §3.5/4 policy 5/5 |
| SES-117 | PASS | §3.5/4 policy 5/5 |
| SES-118 | BLOCK-WIRE-BINDING-MISSING | §3.5/4 policy 5/5 |
| SES-119 | BLOCK-WIRE-BINDING-MISSING | §3.5/4 policy 5/5 |
| SES-120 | BLOCK-PROTOCOL-VERSION | §3.5/4 policy 5/5 |
| SES-121 | PASS | §3.5/4 policy 5/5 |
| SES-122 | PASS | §3.5/4 policy 5/5 |
| SES-123 | PASS | §3.5/4 policy 5/5 |
| SES-124 | BLOCK-PERSISTENT-SESSION-SECRET | §3.5/4 policy 5/5 |
| SES-125 | BLOCK-NIST-CONFORMANCE-OVERCLAIM | §3.5/4 policy 5/5 |
| SES-126 | BLOCK-T3.3-OVERCLAIM | §3.5/4 policy 5/5 |
| SES-127 | BLOCK-T3.4-OVERCLAIM | §3.5/4 policy 5/5 |
| SES-128 | PASS | §3.5/4 policy 5/5 |

**Audit conclusion:** all 128 cases resolve to a section above; none
required a divergence-from-landed-code resolution (the one place a real
conflict was checked for -- SessionId's canonical byte-order tie-break,
SES-099 -- matched the landed T0.4 byte_order implementation exactly,
section 2.3). One real gap was found and closed during this revision,
not silently dropped: SES-005/006/007 (session identity must never enter
save data, simulation state root, or authoritative RNG key) was absent
from the first draft and is now explicit in section 2.3, mirroring
T3.1's existing ServerBootId isolation.
