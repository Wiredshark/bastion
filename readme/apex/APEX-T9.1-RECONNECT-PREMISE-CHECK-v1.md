# APEX-T9.1 — reconnect progression: PREMISE-CHECK (v1)

**Status: PREMISE-CHECK. Read-only, no code, no design.** The question
is only: of `T9.1`'s four build steps, which are ALREADY TRUE, which are
WIRING over machinery that exists, and which are GENUINELY ABSENT.

Surveyed from live reads at `48e2b621b8`. Cited by symbol and file, not
by line where the file is hot.

**Headline: three of the four steps are mostly built. The one that is
not is not "unfinished" — it has no subject to act on.**

---

## Step 1 — new connection epoch + full manifest-validated bootstrap on reconnect

**Verdict: ALREADY TRUE for the mechanism; WIRING for the reconnect
case specifically.**

Everything the step names exists and is live:

- `ConnectionEpoch` is a real identity (`apex::identity`), carried in
  the freshness ledger (`bootstrap_freshness.rs`).
- `BootstrapFreshnessLedgerV1::rebind_epoch_v1` is the epoch-advance
  operation, and epoch REGRESSION is a typed refusal —
  `BootstrapFreshnessRejectionV1::Freeze { ledger_epoch,
  candidate_epoch }`. A stale epoch cannot quietly win.
- The server BUILDS and SENDS a manifest on connect:
  `bootstrap_manifest_v1` in `server/src/sys/msg/register.rs`, encoded
  through `bootstrap_manifest_limits_v1`.
- The client VALIDATES it, and the validation is typed rather than
  best-effort: `client/src/error.rs` carries
  `BootstrapManifestMissing`, `BootstrapManifestIncompatible {
  mismatches }` and `BootstrapFreshnessRejected(..)`. A missing
  manifest is an error, not a shrug.

What is NOT established by this read: whether a RECONNECT specifically
mints a fresh epoch and re-runs that same path, or whether the epoch is
only minted on first connect. That is a wiring question with a definite
answer in `server/src/client.rs`'s connect path, and it is the first
thing the builder should establish.

## Step 2 — do not replay unknown unacknowledged CONTINUOUS frames after bootstrap

**Verdict: GENUINELY ABSENT — and absent in a way that matters more
than "not built yet".**

There is no continuous-vs-discrete stream classification in the wire
layer. `SemanticStreamIdV1` names streams by ROLE (`Bootstrap`,
`CharacterScreen`, …); nothing in `common/net/src/msg/envelope.rs`
marks a stream as carrying continuous frames versus discrete commands.

So the rule "do not replay unknown unacknowledged continuous frames"
currently **has no subject**. It cannot be enforced, and — more
usefully — it cannot even be STATED against the present types, because
"continuous frame" is not a thing the wire layer distinguishes.

That makes this step's real first move a taxonomy question, not a
replay-logic question: what makes a stream continuous, and is that a
property of the stream, the message, or the subscription? Answering it
in code before answering it in the type system would produce a rule
enforced at one call site and forgotten at the next — the shape this
program has spent several campaigns removing.

## Step 3 — discrete command IDs resolved through the terminal journal

**Verdict: ALREADY TRUE.** `T3.5`'s journal
(`common/net/src/msg/command.rs`) is exactly this mechanism and is
built to the required discipline:

- the sequence advances **only when a terminal is acknowledged**
  (`CMD-060`), so an in-flight command cannot be mistaken for a
  finished one;
- a seen-and-finished command replays its **terminal bytes** rather
  than re-executing (`CMD-075`);
- gaps, reuse and unacked terminals are all TYPED outcomes rather than
  silent cases, pinned by
  `sequence_gaps_reuse_and_unacked_terminals_are_all_typed` and carried
  as canaries `CMD-037`/`CMD-045`.

The remaining work here is connecting reconnect to it, not building it.

## Step 4 — retained suspended sessions + per-stream replay windows

**Verdict: SPLIT — retention is BUILT, replay windows are ABSENT.**

- **Retention exists**: `server/src/session_registry.rs` has
  `SessionRegistry` with `detach(..)`, `DETACHED_RETENTION_GRACE` (60s),
  `DEFAULT_DETACHED_RETENTION_CAP` (64) and `purge_expired(..)`, and
  `server/src/events/player.rs` already routes a disconnect into a
  *retained, resumable* session for the qualifying reason only
  (`SES-085`/`SES-086`), every other reason falling through.
- **Per-stream replay windows do not exist**: no replay-window type in
  the wire layer. And note this step depends on Step 2's missing
  taxonomy — a per-STREAM replay window presupposes knowing what each
  stream carries.

The row's own gate ("only after full bootstrap proves stable") is
therefore already the right ordering, and the dependency is stronger
than the row states: step 4 is blocked on step 2's taxonomy, not merely
on step 1's stability.

---

## What a builder should take from this

1. **Two of the four steps are wiring, not building** (1 and 3). The
   machinery landed in `T4.1`/`T4.2`/`T3.5` and is typed, tested and
   live.
2. **Step 2 is the row's real content**, and its first deliverable is a
   TAXONOMY (what is a continuous stream?), not replay logic. It is
   also the dependency step 4 silently sits on.
3. **Nothing here needs inventing from scratch**, which is the useful
   half of the answer: the row is smaller than its four-step shape
   suggests, with one genuinely open design question at its centre.

**Not settled by this check**, and flagged rather than assumed: whether
reconnect currently re-runs the bootstrap path at all (step 1's wiring
question). That is one read of `server/src/client.rs`, and it decides
whether step 1 is "already true" or "one call away".
