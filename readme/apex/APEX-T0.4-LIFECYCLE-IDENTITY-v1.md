# APEX-T0.4 — Authoritative Lifecycle Identity Foundations

Implements `common::apex::identity`. **Foundations only** — no live
issuance, transport integration, or subsystem wiring. Owning rows:

| Type | Owning row (issuance/integration) |
|---|---|
| `ServerBootId` | `APEX-T3.1` |
| `SessionId` | `APEX-T3.2` |
| `ConnectionEpoch` | `APEX-T3.2` |
| `CommandId` | `APEX-T3.5` |
| `PhysicsGeneration` | `APEX-T3.6` |
| `SnapshotEpoch` / `SaveEpoch` / `UniverseBranchId` | `APEX-T4` |

## Opaque UUIDv4 identities

`ServerBootId`, `SessionId`, `CommandId`, `UniverseBranchId` — all
`#[repr(transparent)]` wrappers over `uuid::Uuid`. Every constructor routes
through `uuid::Builder::from_random_bytes`, the single owner of UUIDv4
version/variant bit layout; `IdRandomBytesSourceV1` supplies raw octets
only, never a preformatted UUID.

`Ord`/`PartialOrd` are implemented **manually** as unsigned lexicographic
comparison of the raw 16 octets — not derived, so this V1's canonical
collection order can never silently change if `uuid::Uuid`'s own internal
representation changes. This ordering is a deterministic tiebreaker only;
it is never creation time or causal order.

CBOR encoding: a definite-length 16-byte string (`0x50` + 16 raw bytes),
verified against the golden vector's exact `cbor_bytestring_hex`.

Text form: `"<prefix>/<hyphenated-lowercase-uuid>"` (`boot/`, `session/`,
`command/`, `branch/`).

## Scoped monotonic counters

`ConnectionEpoch` reserves `0` (`ZeroReserved` — only reachable via the
`INVALID` constant, never via `new`). `PhysicsGeneration`, `SnapshotEpoch`,
`SaveEpoch` treat `0` as a legitimate value (`INITIAL`); their owning
schemas decide genesis/reserved-value policy on top, not this module.
`checked_next()` returns `CounterAdvanceErrorV1::Exhausted` at `u64::MAX`
rather than wrapping.

## Golden-vector conformance

`common/src/apex/identity/{opaque,counter,codec,mod}.rs` tests cover, from
`PROJECT-BASTION-APEX-IDENTITY-GOLDEN-VECTORS-v1.json` (SHA-256 matches the
master build order's pinned digest exactly — no drift):

- `uuid_v4_from_random_bytes` (all 3 cases: all-zero, mixed, all-one).
- `canonical_ordering` (unsigned lexicographic, exact expected order).
- `uuid_v4`'s exact CBOR bytestring encoding.
- `counter_vectors` (all 8: `ConnectionEpoch` zero-reserved/advance/exhaust,
  `PhysicsGeneration` zero-valid/advance/exhaust, `SnapshotEpoch`/`SaveEpoch`
  zero-valid).
- `negative_vectors` (all 6: wrong text prefix, UUIDv7 rejected, nil UUID
  rejected, connection-epoch-zero rejected, counter-wrap rejected, and the
  pre-masked-entropy-contract-forbidden rule documented structurally by
  `IdRandomBytesSourceV1`'s signature — it can only return raw octets, so
  there is no code path for a source to hand back an already-versioned
  UUID).
- `typed_text` (all 4 prefixes round-trip).

91/91 total `apex::` unit tests green (13 T0.1 + 43 T0.2 + 26 T0.3 + 22 T0.4;
T0.2's 38-vector external conformance test is separate and also green).
