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

Session admission order is the one place non-determinism could enter:
concurrent authentication completions racing to commit into a shared
registry. Closed the same way T0.5 closed negotiation-selection
ambiguity: define one canonical **commit order** and make it structural,
not incidental. Per the canary corpus (`attempt-order` group,
SES-051..SES-065): every authenticated intent carries an
`AttachmentAttemptSeq` (monotonic, allocated **before** the awaited
stream/auth work begins — SES-051/SES-052 — so slower auth can still lose
to a later-arriving-but-earlier-allocated attempt, never the reverse).
Commit order is **sorted, not insertion/HashMap/mutex-arrival order**
(SES-058/SES-059 name-and-forbid exactly those two non-deterministic
orderings): same-principal intents sort by principal bytes then
descending attempt sequence (SES-053); different principals commit in
canonical principal-byte order (SES-060). Expired sessions are purged
**before** the sorted pass reads registry state (SES-064), and the whole
admission phase reads one `SessionMaintenanceNow` snapshot, never
per-event wall-clock reads (SES-091/SES-092/SES-093) — this is the same
"one time-read per phase" discipline this program has used since `T3.1`'s
boot-scope work, now applied to session expiry. Registry mutation happens
only in the single sorted commit pass, never inside a `par_bridge`
closure or other worker-order-dependent path (SES-065).

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
