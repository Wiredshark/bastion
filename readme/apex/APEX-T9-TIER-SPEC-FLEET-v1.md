# APEX-T9 — Final lifecycle, recovery, and certification endpoints (fleet-authored spec v1)

Authored by Builder Opus 5 on `bastion/apex-t34` @ `5e5cd0f1bc`, from the
master-order rows `APEX-T9.1`..`T9.3`, grounded in live code reads at that
tip. Symbols cited were read, not recalled.

**The tier's thesis.** T9 spends what every earlier tier banked. It adds
almost no new mechanism: reconnect resolves through T3.5's command
journal, branching through T4.6's committed epochs and T0.4's
`UniverseBranchId`, and the campaign is an evidence matrix over
attestations the other tiers already produce. If T9 finds itself
inventing a mechanism, an earlier tier is incomplete and the honest move
is to say so rather than to build the missing piece here.

**The certificate is the program's actual deliverable**, and T9.3
constrains it sharply: *"It may state only the properties whose separate
attestations passed."* That sentence is the whole quality bar. A
certificate that summarises, rounds up, or infers is worse than none,
because it launders unproven claims into a signed artifact.

---

## Shared failure surface (verified)

**Reconnect already has a resume request shape and no resume policy.**
`SessionRequestV1::Resume { locator: SessionId, expected_epoch:
ConnectionEpoch }` exists (`common/net/src/msg/client.rs:78-79`) and its
own doc calls it *"a bearer-free continuation request, not a
credential"* (`:71`). So the wire shape T9.1 needs is present; what is
absent is the policy deciding what may be resumed and what must be
re-bootstrapped.

**Branch identity exists and is unused for branching.**
`UniverseBranchId` is a registered T0.4 opaque identity with a manifest
codec (`common/src/apex/identity/codec.rs:92`,
`impl_opaque_manifest_codec!(UniverseBranchId,
IdentityKindV1::UniverseBranch)`) and appears in tests (`:197`). Nothing
in the live tree creates a branch. T9.2 is therefore a *use* of an
existing identity, not a new identity — which is exactly what a
well-sequenced program should look like at this depth.

---

## T9.1 — Reconnect progression

**Objective.** Reconnect cannot apply old traffic or duplicate uncertain
commands.

**Selected architecture, staged as the row stages it.** The MVP is
deliberately the *expensive* option: a new connection epoch and a full
manifest-validated bootstrap (T4.1/T4.2). Cheap resumption comes later,
and only *after* full bootstrap has proven stable — the row's own step 4.
That ordering is a safety property, not caution: a resume path that ships
before the bootstrap path is trustworthy has nothing to fall back to.

Two rules do the real work:

- **Do not replay unknown unacknowledged continuous frames after
  bootstrap.** Continuous frames are `LatestState` under T3.5's
  classification — replaying them is meaningless at best and, after a
  generation change (T3.6), actively wrong. The generation is what makes
  "unknown" decidable here.
- **Resolve discrete command IDs through the terminal journal.** This is
  T3.5's `CommandJournalV1` doing exactly the job it was built for: a
  reconnecting client's outstanding command is either retired (below the
  floor), terminal (replay the outcome), or in flight. The row's
  "ambiguous outcome policy" is the fourth case — a command whose fate
  the journal genuinely cannot determine — and it must be *declared*,
  not inferred at the call site.

Later stages retain suspended sessions and per-stream replay windows,
resuming only from validated watermarks and the manifest root.

**QUIC path migration stays in the current epoch; a new transport
connection increments it.** This composes with T3.5's `CMD-145` finding —
transport identity does not appear in the command journal at all, which
is exactly why a path migration cannot disturb it. State that
compatibility explicitly rather than leaving two rules to be reconciled
by a reader.

**Required tests.** A reconnect after an unacknowledged command yields
the original outcome, not a second execution; continuous frames from
before the reconnect are not replayed; a path migration does not
increment the epoch and a new connection does; an ambiguous command hits
the declared policy rather than a default.

---

## T9.2 — Authorized historical save branching

**Objective.** Intentional rollback is explicit history, never silent
stale-manifest acceptance.

**Selected architecture.** Selecting an older committed checkpoint is an
**explicit offline or UI action** — never an automatic recovery
behaviour. Verify its full T4.6 manifest and every payload before
anything else happens.

Then the rule that gives the row its name: create a **new
`UniverseBranchId`** and a new epoch-zero lineage whose parent is the
restored checkpoint. **Never continue the old forward epoch sequence.**
Continuing it would make two different futures share an epoch numbering,
and every freshness check in T4.2 and T9.1 would then be comparing
incomparable things.

Preserve the abandoned branch's records, the new branch's records, and
the operator decision. The audit trail is the deliverable — "we rolled
back" must be answerable months later, including *who decided*.

**Required tests.** Repeated restoration from the same checkpoint yields
distinct branch ids; a concurrent server start against a branching
directory is refused rather than racing; a stale client holding the old
branch's manifest is rejected with a typed terminal that names the branch
change rather than a generic mismatch.

---

## T9.3 — Complete apex campaign

**Objective.** One certificate, naming exact roots, stating only what
separately passed.

**The evidence matrix**, each row consuming a tier's own output:

| Evidence | Source |
|---|---|
| Same-target clean rebuilds | T1 |
| Cross-target execution vectors | T6.4 |
| Plugin archive/DAG/conflict permutations | T2.2–T2.5 |
| Six-stream reorder/delay/duplicate schedules | T3.3, T3.4.22 |
| Command retry/crash/reconnect windows | T3.5.20, T9.1 |
| Prediction correction and rollback | T5.3, T7.4 |
| Physics/weather raw+semantic numeric vectors | T6.2 |
| World baseline/economy mismatch lanes | T4.3, T8.2–T8.4 |
| Multi-store crash cutpoints | T4.6 |
| Historical save migrations and authorized branching | T4.5, T9.2 |

**The certificate names exact roots** — build, content, plugin, manifest,
numeric, schedule, fixture, and output. Names, not summaries: a root is
checkable, a claim is not.

**Selected architecture for the constraint.** "May state only the
properties whose separate attestations passed" should be enforced by
construction, in the lineage this program has used since T3.4: the
certificate is *generated from* the attestation set, and a property with
no passing attestation is structurally absent rather than omitted by a
careful author. Same move as T3.4's evidence bundle, which is generated
from the tree rather than hand-asserted, and as the coverage maps, where
an unclaimed case fails the build.

The certificate should also carry its **OPEN set** — the named-OPEN
cases from every tier's coverage map, with counts. A certificate that
lists what it does not cover is trustworthy in a way a clean one is not.

**Why this row is `DEFERRED`.** It cannot be built before its inputs
exist, and attempting it early produces a certificate whose gaps are
invisible. It is the last row in the program for a structural reason, not
a scheduling one.

**Required tests.** A property whose attestation failed cannot appear in
the certificate (the mutation test: fail one attestation, regenerate,
confirm the property vanished); every named root resolves to an artifact
in the tree; the OPEN set matches the sum of the tiers' pinned counts.

---

## Cross-tier notes

**T9 consumes; it should not invent.** Every mechanism above already
exists or is specified: `SessionRequestV1::Resume`,
`CommandJournalV1`, `UniverseBranchId`, T4.6's committed epochs, the
coverage maps, the evidence bundles. If a builder reaches for something
new here, that is a signal that an earlier tier under-delivered — and the
right response is to raise it, not to close the gap locally where it will
be invisible.

**The certificate's honesty rule generalises.** Three artifacts in this
program now work the same way — coverage maps (unclaimed name fails),
evidence bundles (generated from the tree), and now the certificate
(generated from attestations). The pattern is: *make the artifact
incapable of overstating*. It is the documentation-side twin of the
type-side rule stated in the T5 spec, and worth naming as one principle
rather than three habits.
