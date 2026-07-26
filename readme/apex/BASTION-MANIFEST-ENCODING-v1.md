# `BastionManifestEncodingV1` — normative profile

Implements `APEX-T0.2`. Codec identity: `bastion.manifest-cbor.rfc8949-core/v1`
(`common::apex::manifest::BASTION_MANIFEST_ENCODING_V1`).

Determinism story: exactly one accepted value maps to exactly one byte
string (RFC 8949 Section 4.2.1 core deterministic encoding: shortest
integer/length forms, definite lengths only, no floats/tags/null). Decode
proves this by re-encoding the parsed value and requiring an exact byte
match against the input — see `common/src/apex/manifest/decode.rs`.

## Accepted value model

```
Unsigned(u64)
Negative(i64)          -- invariant: value < 0
Bytes(Vec<u8>)          -- definite-length CBOR byte string (major type 2)
MachineText(String)     -- ASCII only, no NUL/control bytes (major type 3)
Bool(bool)              -- CBOR simple values 20 (false) / 21 (true) only
Array(Vec<Value>)       -- definite-length (major type 4)
Map(CanonicalFieldMapV1) -- definite-length, keys are u16 field IDs in
                            strictly increasing canonical-byte order
                            (major type 5, key encoded as major type 0)
```

Forbidden in V1: all floating-point values (major type 7, additional info
25/26/27), CBOR tags (major type 6), null/undefined (major type 7,
additional info 22/23), any other simple value, indefinite-length strings/
arrays/maps (additional info 31 on major types 2-5), non-`u16` or
non-strictly-increasing map keys, non-ASCII machine text, integers outside
the `u64`/[-2^64, -1] range representable by CBOR major types 0/1.

## Byte grammar (RFC 8949 core deterministic, restricted)

Every value is one CBOR data item: an initial byte `(major << 5) |
additional_info`, optionally followed by 1/2/4/8 argument bytes chosen by
the *shortest* form that represents the argument (additional info
0-23 = immediate; 24/25/26/27 = 1/2/4/8 big-endian follow bytes), followed
by the item's payload (raw bytes for major 2/3, nested items for major
4/5). Field-map keys are themselves major-type-0 unsigned integers.

## Required decode limits (`ManifestDecodeLimitsV1`)

No `Default` exists — every root schema must name its own
`max_input_bytes`/`max_depth`/`max_nodes`/`max_array_items`/
`max_map_entries`/`max_machine_text_bytes`/`max_byte_string_bytes`.
Declared lengths are checked against these budgets **before** any
allocation, so a hostile declared length (e.g. `0xFFFFFFFFFFFFFFFF` bytes)
is rejected by the budget check rather than attempting to allocate.

## Stable error codes

See `common/src/apex/manifest/error.rs`'s `ManifestCodecErrorCodeV1` (100-133)
and its `terminal_class()` mapping to the golden-vector corpus's coarser
class strings (`MALFORMED_CBOR`, `TRAILING_DATA`, `NON_PREFERRED_ENCODING`,
`INDEFINITE_LENGTH_FORBIDDEN`, `TYPE_FORBIDDEN`, `FIELD_KEY_TYPE`,
`FIELD_KEY_RANGE`, `FIELD_ORDER`, `DUPLICATE_FIELD`, `INVALID_UTF8`,
`MACHINE_TEXT_NON_ASCII`, ...).

## Golden-vector conformance

`common/tests/apex_manifest_encoding_v1.rs` loads
`common/tests/fixtures/apex_manifest_v1/golden-vectors.json` (a verbatim
copy of the program's `PROJECT-BASTION-APEX-MANIFEST-CBOR-GOLDEN-VECTORS-v1.json`,
38 vectors — the Drive file's actual SHA-256 does not match the master
build order's pinned digest `8aba6c9b...`; same artifact-version-drift
pattern already flagged for `APEX-A.3`'s seed and `APEX-T0.1`'s boundary
inventory. Content was used as-is since it is internally coherent and
matches this packet's own inline example bytes) and checks every vector —
every valid vector must encode to its `expected_hex` exactly and round-trip
through decode; every invalid vector must decode-fail with its declared
terminal class.

## Scope deviation from the packet: no `minicbor` dependency

Packet section 5.8 specifies adding `minicbor` as a contained low-level
primitive dependency. This implementation instead hand-rolls the RFC 8949
major-type byte encoding directly in `common/src/apex/manifest/{encode,decode}.rs`
(fewer than 250 lines combined) rather than adding a new workspace
dependency. Rationale:

1. The restricted value model has exactly 7 accepted kinds and needs only
   the standard CBOR major-type/preferred-length rules — well within what
   a from-scratch implementation can get byte-exact and fully test against
   the external golden-vector corpus (not self-authored expected bytes).
2. Adding a workspace-wide `Cargo.toml`/`Cargo.lock` dependency during a
   period of heavy concurrent multi-session development on this repository
   carries real (if usually small) lock-contention/rebuild risk for other
   active worktrees, for no behavioral gain here.
3. The packet's own goal — "Bastion owns value restrictions, map order,
   limits, strict decoding, schema traits, and golden vectors" — is met
   either way; `minicbor` was only ever meant to supply low-level
   read/write primitives, which are what this module already provides
   directly.

If a future row needs full general CBOR interoperability (not just this
restricted profile), `minicbor` (or another audited crate) remains
available to add then, gated on its own vector-conformance proof per
section 5.8's upgrade rule.

## Non-goals (unchanged from the packet)

Domain field IDs for bootstrap/save/plugin/build/world manifests, per-schema
size limits, and optional/unknown-field compatibility policy are NOT
decided here — those are the owning root schema's job (`APEX-T0.5`+).
