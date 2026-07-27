# APEX-T3.3.05 — Row status (middle-tier gate)

**Ruling:** Fable, middle tier (not T3.2's full elevated gate). Three
requirements below; documentation level is this row-status doc plus the
boundary package (commit messages + review request to Opus), no separate
spec round-trip.

## Requirement 1 — Sequential-phase confinement (the tier hinge)

Every new decision this step adds executes inside `T3.2`'s already-sequential
phases. Exact insertion points:

1. **"Requested protocol is server-supported" check** — `server/src/sys/msg/register.rs`
   phase 1 (the sequential `ClientRegister`-drain loop, single-threaded,
   before the `par_bridge()` parallel auth phase begins). Placed as a
   sibling check right after the existing `check_register_boot_scope` call
   and before `login_provider.verify` is invoked — identical placement
   pattern to that already-accepted check. This check reads only
   `ClientRegister.requested_semantic_protocol` (from the message just
   drained) against a fixed constant set (`server_supported_semantic_protocols_v1()`,
   not per-connection state, not `SessionRegistry`) — no shared mutable
   state is touched.
2. **"No protocol switch on resume" check** — `server/src/session_registry.rs`'s
   `admit_resume`, which is part of `admit_sorted`'s single sequential
   commit pass (never the parallel auth-collection phase). Compares the
   resume intent's `requested_semantic_protocol` against the existing
   `SessionRecordV1.semantic_protocol` already committed for that session
   — the same phase that already makes every other admission decision
   (principal/client-type/epoch checks), so this is not a new kind of
   access, just one more field on an existing sequential comparison.

Nothing new is decided inside the parallel auth-collection phase (phase 2)
or inside any Rayon-parallel system. Phase 2 only *carries* the
already-phase-1-validated `requested_semantic_protocol` value forward
(copied field, not a new decision) so phase 3 has it available.

No part of this step needed new parallel-section work, so it does not
escalate to the full elevated gate.

## Requirement 2 — Before/after wire-compat delta (both directions)

**CORRECTION (Opus 5's boundary-review finding on this row's original
text, self-flagged by Opus as also a miss in his own T3.2 elevated
review): this section originally claimed the version-handshake rejection
mechanism below was already in effect, citing "T3.2's own spec section 9"
as precedent. That was false: `VELOREN_NETWORK_VERSION` was never
actually bumped for either T3.2's or this row's bincode-schema-breaking
wire changes (`network/protocol/src/types.rs` sat at `[0, 7, 0]`,
last touched by T3.1). The §9 rollback *text* referenced a version bump
without the constant itself ever having been changed — verified only at
the prose level, not against the live constant, in both the original
authoring pass and Opus's review. Consequence: a post-T3.1/pre-T3.2 peer
would have passed the `0.7.0 == 0.7.0` handshake and then failed at
bincode decode — exactly the partial/ambiguous failure a clean
version-mismatch rejection exists to preclude. Fixed in the same commit
as this correction: `VELOREN_NETWORK_VERSION` bumped `[0,7,0]` ->
`[0,8,0]`, covering T3.2's and this row's cumulative wire changes in one
bump (no release shipped between them, no cross-version population
exists on this branch). The paragraphs below describe the TRUE,
now-real mechanism.**

**Pre-T3.3 client against a post-T3.3 server:** a pre-T3.3 `ClientRegister`
has no `requested_semantic_protocol` field, and advertises the pre-bump
`[0, 7, 0]` network version against this row's `[0, 8, 0]` — the version
handshake rejects the mismatch cleanly before any partial admission;
there is no code path where an old client's `ClientRegister` decodes
successfully with a silently-defaulted or missing new field.

**Post-T3.3 client against a pre-T3.3 server:** symmetric — the server
advertises the pre-bump version and does not recognize the new
`ClientRegister` shape or the new `ServerInfo.supported_semantic_protocols`
field the client would expect; same version-handshake rejection, not a
partial/ambiguous accept.

**Post-T3.3 client against a post-T3.3 server (the actual golden path
this row changes):** the live client (this codebase's own
`client/src/lib.rs::register`) always sends
`requested_semantic_protocol: SemanticProtocolIdV1::Legacy` -- no
V1 sender exists yet (that lands in `T3.3.07`). The live server always
advertises `[Legacy, NetEnvelopeV1]` (both) via
`server_supported_semantic_protocols_v1()` -- no certified-mode
restriction to `[NetEnvelopeV1]`-only exists yet (`T4.1`'s
`BootstrapManifestV1` owns that surface per packet section 5.9). So
`Legacy ∈ {Legacy, NetEnvelopeV1}` always holds today:
`IncompatibleSemanticProtocol` is unreachable in practice for the real
client/server pair in this tree, and `selected_semantic_protocol` is
always `Legacy` -- **no previously-succeeding connection attempt starts
failing, and no previously-failing one is silently masked** (the same
named invariant `T3.2`'s `SES-082` protects, now covering this field
too since `selected_semantic_protocol` lives on `SessionBindingV1`,
which that invariant's regression suite already exercises end to end).

`IncompatibleSemanticProtocol` and the mode-switch reject are exercised
by direct unit/integration tests (harness-level, not live-client-level,
since no live client can construct a request that triggers them today)
-- this is a deliberately narrow, currently-dormant negotiation
mechanism, matching the packet's own framing ("T4.1 later subsumes this
narrow negotiation").

## Requirement 3 — T3.2 invariant suite as regression guard

`selected_semantic_protocol` is added directly to `SessionBindingV1`,
the exact struct `T3.2`'s `check_session_binding_equality` (client) and
the `RegisterAnswer`/`GameSync` binding-echo contract (server) already
enforce byte-for-byte. No new equality-check code was written for this
field -- it is protected for free by machinery `T3.2` already built and
tested. `SES-082` (behavioral-preservation invariant, session_registry's
full suite, and the server/client/common/common-net lib suites) must
stay green after this change and is cited in the boundary package as
the regression guard, per Fable's ruling.

## Rollback

Follows `T3.2`'s own accepted wire-change precedent: clean-revert,
commit separation (this row's commits are separable from `T3.3.01`-`04`,
which do not depend on it). No new rollback analysis needed -- same
additive-field shape as `T3.2`'s `session_request`/`SessionBindingV1`
additions.
