# APEX Digest Domain Registry v1

Implements `APEX-T0.3`. Two deliberately different identities:

1. **Exact artifact identity** (`common::apex::digest::ArtifactIdentityV1`)
   — plain `SHA256(raw bytes)` + byte count. No domain framing, ever.
2. **Protocol/semantic root** (`common::apex::digest::ProtocolDigestV1`) —
   `SHA256` of a domain-framed preimage.

## Algorithm registry

| ID | Name | Digest bytes |
|---:|---|---:|
| 1 | `sha-256` | 32 |

Truncation is forbidden. A future algorithm requires a new ID, new golden
vectors, and explicit compatibility rules — it may not replace ID 1.

## Domain registry

| ID | Label | Owner |
|---:|---|---|
| 1 | `bastion/bootstrap-manifest/v1` | `APEX-T4.1` |
| 2 | `bastion/save-universe-manifest/v1` | `APEX-T4.5`–`T4.6` |
| 3 | `bastion/plugin-activation-plan/v1` | `APEX-T2.5` |
| 4 | `bastion/world-baseline-manifest/v1` | `APEX-T4.3` |
| 5 | `bastion/build-manifest/v1` | `APEX-T1.5` |
| 6 | `bastion/execution-evidence/v1` | `APEX-T1.5`/`T8` |
| 7 | `bastion/semantic-content/v1` | schema-specific later owners |

Future Merkle-tree leaf/node purposes register their own domain under their
owning schema; this module does not expose a generic untyped Merkle API.

## Preimage layout (big-endian throughout)

```
magic:                "bastion-digest/v1\0"  (18 bytes, NUL-terminated ASCII)
algorithm_id:          u16
domain_id:              u16
domain_label_len:        u16
domain_label:              ASCII bytes (length domain_label_len)
canonical_payload_len:       u64
canonical_payload_bytes
root = SHA256(preimage)
```

Both the numeric domain ID and its registered ASCII label are bound into
every preimage — the label is derived from the sealed registry
(`common/src/apex/digest/domain.rs`), never supplied by a caller.

## Golden-vector conformance

`common/src/apex/digest/protocol.rs`'s tests hash all 8 `protocol_vectors`
(4 domains × 2 payloads) from
`PROJECT-BASTION-APEX-DIGEST-GOLDEN-VECTORS-v1.json` against their exact
expected SHA-256, plus a hand-reconstructed preimage byte check independent
of this module's own construction code. `common/src/apex/digest/artifact.rs`'s
tests hash all 3 `artifact_vectors` (empty, `"abc"`, a canonical CBOR field
map) against their expected SHA-256, including the standard NIST empty/`abc`
SHA-256 test vectors. This is the one T0.3 vector file whose SHA-256
(`f5793d8f19a18257a03231bb5ca52a53c4d83be39d81bae2ec998d85569e996b`)
**matches** the master build order's pinned digest exactly — no
artifact-version drift this time, unlike `APEX-A.3`'s seed, `APEX-T0.1`'s
boundary inventory, and `APEX-T0.2`'s vector file.

## Negative canaries (all covered by tests)

- `artifact-must-remain-plain-sha256` — `content::tests::artifact_digest_is_never_domain_framed`.
- `domain-id-label-registry-fixed` — the label is a `const fn` match arm, not caller-suppliable; `mod.rs`'s `ProtocolDigestV1::from_manifest_value_v1` rejects any domain ID not in the sealed registry.
- `semantic-never-integrity` — `content::tests::semantic_root_is_additive_not_a_substitute`; no API accepts a `SemanticRootV1` in place of an `ArtifactIdentityV1` check.
- `full-256-bits` — `DigestBytes32V1` has no constructor accepting fewer than 32 bytes (`algorithm::tests::exact_32_byte_construction`).
