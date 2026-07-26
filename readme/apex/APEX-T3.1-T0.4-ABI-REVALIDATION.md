# APEX-T3.1.01 — T0.4 identity ABI revalidation

Required by the T3.1 packet's own admission gate before any code mutation
("block if T0.4 implementation differs materially; no code mutation"
until resolved). Landed type: `common::apex::identity::ServerBootId`
(`common/src/apex/identity/opaque.rs`, opaque-ID macro instantiation).

## Naming differences (cosmetic — consumed as-landed, no risk)

The T3.1 packet's inline sketch of "the T0.4 contract" uses different
names than what T0.4 actually landed (and Builder Opus 5 independently
reviewed and approved). This is expected drift — the T3.1 packet predates
T0.4's actual implementation by design (research-first) — not a real
mismatch:

| T3.1 packet assumed | Actually landed | Verdict |
|---|---|---|
| `ServerBootId::try_generate(source)` | `ServerBootId::generate(source)` | same semantics, different name — use the landed name |
| `ServerBootId::from_test_random_bytes(bytes)` | no direct equivalent; use `FixedRandomBytesSourceV1([u8;16])` + `generate()` | same capability via the injectable-source pattern, not a const-fn shortcut |
| `ServerBootId::as_bytes() -> &[u8;16]` | `ServerBootId::as_uuid() -> &Uuid` | `Uuid::as_bytes()` is reachable via `.as_uuid().as_bytes()` when raw bytes are specifically needed |
| `IdRandomBytesSourceV1` with associated `type Error` + `try_fill_16` | `IdRandomBytesSourceV1` with fixed `IdentityGenerationErrorV1` + `fill_random_bytes` | fixed-error-type trait is simpler and already implemented by `OsRandomBytesSourceV1`/`FixedRandomBytesSourceV1`; T3.1.02's `SystemIdRandomBytesSourceV1` is therefore **already satisfied by T0.4's existing `OsRandomBytesSourceV1`** — no new adapter needed |

## Substantive gap (real, resolved by additive extension)

`ServerBootId` did **not** derive `Serialize`/`Deserialize` in T0.4, by
explicit T0.4 policy: *"Do not derive generic Serde in T0.4; canonical
manifest encoding is explicit, and live wire migration belongs to owning
rows."* T3.1 **is** that owning row — `ServerBootId` needs to serialize
over the existing bincode-legacy wire protocol (`ServerInfo`,
`ClientRegister`, `ServerInit::GameSync` are not `BastionManifestEncodingV1`
manifests; they are the live game's ordinary bincode messages).

**Resolution:** add manual `Serialize`/`Deserialize` impls (not a plain
`#[derive]`) to the four T0.4 opaque ID newtypes — additive; no existing
method signature or behavior changes; already-reviewed T0.1-T0.4 tests are
unaffected.

**Real finding while implementing this, corrected before it shipped:** a
naive `#[derive(Serialize, Deserialize)]` inherits `uuid::Uuid`'s own Serde
impl, which for a non-human-readable format calls `serializer.serialize_bytes`
— bincode treats that as a variable-length blob and prepends an 8-byte
little-endian length header, producing **24 bytes on the wire**, not the
compact 16-byte field the T3.1 packet's acceptance gate literally asks for
("client receives full 16-byte ID"). Confirmed empirically
(`common/src/apex/identity/opaque.rs` tests), not assumed. Fixed by a
manual `Serialize` that delegates to `self.0.as_bytes(): &[u8; 16]`
(a fixed-size array serializes without a length prefix under bincode) and a
manual `Deserialize` that decodes `[u8; 16]` and re-validates through
`from_uuid_v4` — which also closes a second gap the derive would have had:
the derived impl would accept *any* 16 bytes on decode (nil UUID, wrong
version/variant) with zero revalidation, since it trusts `uuid::Uuid`'s own
permissive deserializer. The manual impl now rejects invalid wire bytes
(covered by `bincode_deserialize_rejects_invalid_uuid_bytes`), matching
every other ingestion path's fail-closed policy.

## Verdict

**Admitted with one additive, anticipated extension** (Serde derive) —
not `PrerequisiteAbiMismatch`. The naming differences require no code
change to T0.4 (T3.1 consumes the landed names). The Serde gap is exactly
what T0.4's own packet text predicted this row would need to add, and
adding a derive is non-breaking to every existing T0.1–T0.4 consumer and
test.
