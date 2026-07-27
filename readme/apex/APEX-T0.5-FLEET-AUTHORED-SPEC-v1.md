# APEX-T0.5 — Fleet-authored spec: shared subsystem descriptors and compatibility profiles

> **STATUS: APPROVED, BUILD-AUTHORIZED.** Author: Builder Sonnet 5,
> 2026-07-27. Opus 5 spec-reviewed (verdict: approved with one domain-ID
> coordination fix, already resolved — see §4 note — and one clarification,
> folded into §3.7/§6 below). Fable granted build authorization
> conditional on that clarification landing, self-attested in the
> implementing commit. Domain-ID collision with `APEX-T1.2`'s
> `SourceClosure` domain resolved by row-order allocation (`T0.5`
> `sequence_index=8` precedes `T1.2` `sequence_index=10`): `T0.5` keeps
> `9`/`10`, `T1.2`'s moved to `11`. This is now the fleet's standing
> collision rule for future domain-ID additions.
>
> **Grounding trust posture (Fable's standing ruling, 2026-07-27,
> applies retroactively to this document and to all future
> fleet-authored specs):** inline master-order row content is admissible
> grounding, never inherited authority — the same document that grounded
> `T0.1`–`T0.4` cleanly also contains the fabricated `8aba6c9b` pin and
> three phantom filenames, so no code-facing claim rides in on a quote
> unchecked. This document already followed that posture in practice
> (every T0.1–T0.4 symbol cited in §0.3 was verified against the actual
> landed source, not read off the master-order's prose, e.g.
> `ProtocolVersion` — not the master order's own casual
> `ProtocolVersionV1` naming — because that is what `T0.1` actually
> shipped); recorded here explicitly now that it is a named standing
> rule rather than an implicit practice.

## 0. Provenance (read this before the rest)

Per Ben's 2026-07-27 order (routed via Fable): the standalone packet
(`PROJECT-BASTION-APEX-MICROSTEP-APEX-T0.5-SUBSYSTEM-DESCRIPTORS-COMPATIBILITY-PROFILES.md`)
and its vectors file were never real — the ChatGPT-side artifacts that were
supposed to deliver them were hallucinated, and the two non-`.gdoc` files
sitting in the Drive folder under those names are explicitly marked
`INVALID-WRONG-CONTENT-DO-NOT-USE` and were **not consulted, not even for
inspiration**, in writing this document.

This spec is grounded in exactly three kinds of real material:

1. **The master build order's own row block** — not the missing separate
   packet, but the row-level summary/objective/atomic-build-sequence/
   acceptance text embedded directly inside
   `PROJECT-BASTION-APEX-DETERMINISM-STEP-BY-STEP-MASTER-BUILD-ORDER.md`
   itself (the document `APEX-A.1` already admitted as this program's
   source of truth, and the same document every prior row — `T0.1`–`T0.4`,
   `T3.1` — was built from). Confirmed present and non-blank at lines
   113993 (row header), 115018–115021 (status/packet/vectors/adversarial-
   verdict metadata — these name the *missing* files, they are not those
   files), 116046 (scope correction), 117071–118107 (12-step atomic build
   sequence), 119132 (acceptance). The huge blank-line spans surrounding
   these lines are the known padded-file export artifact, not missing
   content — every non-blank line in the row's 7189-line span was read and
   is reproduced verbatim below.
2. **The verified finding-status matrix**
   (`readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv`): no finding cites
   `APEX-T0.5` directly (checked: zero `APEX-T0.5` occurrences in
   `replacement_rows`). `T0.5` is pure infrastructure — its downstream
   consumers `APEX-T4.1`, `APEX-T4.3{a,b}`, `APEX-T4.4`, `APEX-T6.1`
   (registry `hard_dependencies`) are what actually close findings
   (`DET-SVC-021`, `DET-ESIM-007`, `DET-PHY-008`, `DET-PHY-024`,
   `DET-WTH-003`).
3. **Live code seams** — the actual `APEX-T0.1`–`T0.4` types this row must
   compose, cited by symbol (files grow; line numbers rot):
   - `common::apex::scalar`: sealed `FixedWidthScalar` trait,
     `fixed_scalar_newtype!` macro, `ProtocolVersion(u32)`,
     `SchemaVersion(u32)`, `CanonicalOrdinal(u32)`, `CanonicalCount(u64)`,
     `CanonicalByteLength(u64)`, `CanonicalSequence(u64)`.
   - `common::apex::manifest`: `ManifestEncodeV1`/`ManifestDecodeV1`
     traits, `encode_manifest_v1`/`decode_manifest_v1`,
     `ManifestValueV1`, `FieldIdV1(u16)`, `VariantTagV1(u16)`,
     `StructFieldsV1`, `CanonicalFieldMapV1`, `ArraySemanticsV1`,
     `ManifestDecodeLimitsV1`.
   - `common::apex::digest`: `DigestDomainIdV1` (8 existing variants,
     `as_u16()` + `ALL` const array + duplicate-ID/duplicate-label
     self-tests), `ContentIdentityV1 { artifact, semantic: Option<..> }`,
     `SemanticRootV1`, `digest_canonical_bytes_v1`.
   - `common::apex::identity`: `opaque_lifecycle_id!` macro,
     `IdentityKindV1`, `OsRandomBytesSourceV1`/`FixedRandomBytesSourceV1`,
     `zero_valid_counter!` macro (used for `ConnectionEpoch(u64)`).

Anything below not directly traceable to one of these three sources is
marked **(fleet design decision)** and justified inline — never presented
as if it were recovered content.

## 1. Row block, reproduced verbatim (master build order lines 113993–119132)

> **[APEX-T0.5] Shared subsystem descriptors and compatibility profiles**
>
> Implement the shared subsystem descriptor/profile registry on T0.1–T0.4:
> fixed wire tags, `ContentIdentityV1` artifact identity, separately typed
> optional protocol semantic roots, tagged compatibility rules, explicit
> negotiation selector ownership, direct transform registration/lookup,
> complete total-ordered compatibility reports, critical/noncritical
> unknown-extension rules, golden/mutation vectors, and schema/capacity
> bounds; defer multi-hop transform planning and execution to save
> migration/runtime rows → shared common apex manifest modules → **typed
> compatibility vocabulary without duplicated identity fields or ambiguous
> class semantics**.
>
> **Scope correction:** T0.5 implements only the shared descriptor/profile
> vocabulary, direct transform registration/lookup, total compatibility
> evaluation, and evidence. It does not create bootstrap/save/plugin/
> build/world manifests, execute migrations, or invent one universal
> compatibility rule for all subsystems.
>
> **Atomic build sequence:**
> 1. Freeze subsystem slot IDs, descriptor/profile tags, tagged
>    compatibility-rule variants, transform semantics, negotiation-selector
>    ownership, capacity bounds, and unknown-extension rules.
> 2. Add `SubsystemSlotV1`, `SubsystemDescriptorV1`, `ProtocolVersionV1`,
>    and typed optional semantic protocol roots using T0.1–T0.3
>    primitives; reuse `ContentIdentityV1` without duplicating artifact
>    fields.
> 3. Add tagged `CompatibilityRuleV1` variants: exact, accept-set/range,
>    negotiated capabilities, direct transform, and provenance-only;
>    invalid field combinations must be unrepresentable.
> 4. Add `CompatibilityProfileV1` as a canonical typed-array of unique slot
>    rules with complete cardinality and bounds validation.
> 5. Add capability catalogs and an explicit selector owner/algorithm/
>    version so negotiated outcomes are deterministic and cannot silently
>    swap client-preference for server-preference.
> 6. Add direct `TransformRegistryV1` lookup keyed by exact
>    `(transform_id,from_schema,to_schema,implementation_root)`; reject
>    duplicates/conflicts; defer multi-hop graph planning/execution.
> 7. Add complete `CompatibilityReportV1` with one result per profile rule
>    sorted by subsystem slot and typed outcomes
>    `Compatible/Incompatible/InvalidInput`; never stop at first mismatch.
> 8. Define unknown extension behavior: unknown critical slot/rule/
>    transform fails closed; unknown noncritical extension may be ignored
>    but retained in report evidence.
> 9. Add profile/descriptor encoding plus protocol roots with distinct
>    T0.3 domains and full T0.2 vector coverage.
> 10. Add hostile tests for duplicate slots, wrong artifact bytes,
>     semantic-root substitution, contradictory rules, selector drift,
>     unauthorized transform, invalid input vs incompatibility, extension
>     handling, truncation, and cross-target equality.
> 11. Add source fitness scans preventing independent manifest-specific
>     descriptor/compatibility enums from appearing before justified
>     specialization.
> 12. Register the packet/vectors/evidence in A.3; later rows compose the
>     shared vocabulary into separate lifecycle manifests.
>
> **Acceptance:** every descriptor/profile has one canonical encoding;
> duplicated artifact identity is impossible; compatibility rules are
> typed variants; negotiation selection is explicitly owned/versioned; all
> rule results are reported in total order; critical unknowns and invalid
> inputs fail closed; direct transforms require exact authorization;
> vectors and mutation canaries pass cross-target.

Note: step 12's "register the packet/vectors/evidence in A.3" refers to
the missing separate packet/vectors files this program never received —
this document plus its accompanying golden-vector fixture is what gets
registered instead, per Ben's ruling (`specification=FLEET_AUTHORED`, not
`SPECIFICATION_COMPLETE`, so the registry never claims this went through
the normal packet-delivery path).

## 2. Determinism story (required before any code lands)

Every type in this row is either a closed, explicitly-tagged enum with a
frozen numeric discriminant (never derived from declaration order) or a
canonical fixed-order collection built on T0.1's checked scalars and
T0.2's manifest codec. Nothing here reads wall-clock time, worker
scheduling order, or target pointer width. The one place non-determinism
could sneak in — negotiation selection when multiple capabilities are
mutually acceptable — is closed by `NegotiationSelectorV1` making the
owner, algorithm, and version an explicit, encoded, checked field: two
runs given the same profile and the same catalog input always select the
same outcome, and if a build's selector fields ever disagree with what a
peer expects, that is a typed `InvalidInput` report entry, never a silent
default.

## 3. Data model (fleet design decisions, built on T0.1–T0.4)

### 3.1 `SubsystemSlotIdV1` (fleet design decision)

A closed, `u16`-tagged enum, same shape as `DigestDomainIdV1`
(`as_u16()` + `label()` + an `ALL` const array + duplicate-ID/duplicate-
label self-tests) — chosen because that pattern is already proven in this
codebase for exactly this problem (a small frozen vocabulary that may gain
new variants later, appended, never renumbered). Not an opaque UUID
(`T0.4`'s `opaque_lifecycle_id!`) because slot identity is build-time-
frozen vocabulary, not a runtime-generated instance identity.

Initial variants, one per subsystem the row block's own text and the
registry's *known downstream consumers* (`T4.1`, `T4.3{a,b}`, `T4.4`,
`T6.1`) actually name — no slot is invented speculatively beyond what
those rows' own one-line objectives require:

```
Worldgen        = 1   -- T4.3: world seed / worldgen protocol root
Content         = 2   -- T4.1/T4.3: content protocol root, ContentIdentityV1 reuse
Numeric         = 3   -- T4.3/T6.1: numeric protocol root, numeric attack surface
Schedule        = 4   -- T4.1: schedule identity
Plugin          = 5   -- T4.1: plugin activation plan
Economy         = 6   -- T4.3b: economy baseline root
SaveInventory   = 7   -- T4.4: non-authoritative existing-save inventory
Build           = 8   -- T4.1: build identity
```

### 3.2 `SubsystemDescriptorV1`

```
struct SubsystemDescriptorV1 {
    slot: SubsystemSlotIdV1,
    schema: SchemaVersion,           // T0.1
    content: ContentIdentityV1,      // T0.3, reused verbatim (build step 2)
}
```

One descriptor identifies one subsystem artifact at one schema version.
`ContentIdentityV1` is reused, not re-derived — this is the row's own
explicit anti-goal ("without duplicated identity fields").

### 3.3 Typed optional semantic protocol roots (fleet design decision)

The row block says "separately typed optional protocol semantic roots" —
read literally: not one shared `Option<ProtocolVersion>` field reused
across subsystems (that would be the "ambiguous class semantics" the
acceptance criterion explicitly forbids), but one distinct newtype per
protocol root a downstream row actually needs:

```
struct WorldgenProtocolVersion(ProtocolVersion);   // T4.3
struct ContentProtocolVersion(ProtocolVersion);    // T4.1, T4.3
struct NumericProtocolVersion(ProtocolVersion);    // T4.3, T6.1
```

via the same `fixed_scalar_newtype`-adjacent transparent-wrapper
convention as T0.1 (each is `Copy, Eq, Ord, Hash, Serialize/Deserialize
#[serde(transparent)]`, delegating to the inner `ProtocolVersion`). A
`SubsystemDescriptorV1` for a slot that has a protocol root carries
`Option<T>` of the matching typed root — never a bare `Option<ProtocolVersion>`
that could be silently mixed up between worldgen and numeric.

### 3.4 `CompatibilityRuleV1` — tagged, invalid combinations unrepresentable

```
enum CompatibilityRuleV1 {
    Exact { content: ContentIdentityV1 },
    AcceptSet { schemas: Vec<SchemaVersion> },              // non-empty, checked at construction
    AcceptRange { min: SchemaVersion, max: SchemaVersion },  // min <= max, checked at construction
    NegotiatedCapability { requirement: CapabilityRequirementV1 },
    DirectTransform { key: TransformKeyV1 },
    ProvenanceOnly,   // informational only, never gates compatibility
    Unknown { tag: VariantTagV1, criticality: ExtensionCriticalityV1, raw_payload: Vec<u8> },
}
```

`Unknown` is the wire-level catch-all a build whose enum is older than a
peer's needs to represent a rule it cannot interpret — see §3.7 for why
this exists and how it differs from T0.2's core codec, which already
unconditionally rejects unknown *manifest fields*
(`common/tests/apex_manifest_encoding_v1.rs`'s
`decode_rejects_unknown_field`). `CompatibilityRuleV1` is evaluated by
*this row's* logic, not decoded by T0.2's generic codec directly — T0.2
only has to round-trip the tag/criticality/payload bytes faithfully; T0.5
owns interpreting them.

`AcceptSet`/`AcceptRange` construction is checked, not raw: an empty
`AcceptSet` or an inverted `AcceptRange` (`min > max`) cannot be built —
this is what "invalid field combinations must be unrepresentable" means
in practice, mirroring T0.2's own `StructFieldsV1`/`CanonicalFieldMapV1`
checked-constructor pattern.

### 3.5 `CompatibilityProfileV1`

```
struct CompatibilityProfileV1(Vec<(SubsystemSlotIdV1, CompatibilityRuleV1)>);
```

Checked constructor: rejects a profile with two entries for the same
slot (duplicate-slot detection, one of the row's own named hostile
tests), and enforces a max cardinality bound expressed as a
`CanonicalCount` (T0.1) — "complete cardinality and bounds validation."
Canonical iteration order is slot-tag order (`SubsystemSlotIdV1::as_u16()`),
not insertion order — this is what makes `CompatibilityReportV1`'s "total
order" possible without a separate sort step hiding non-determinism.

### 3.6 Negotiation: capabilities, selector, transforms

```
struct CapabilityRequirementV1 {
    catalog: Vec<CapabilityIdV1>,   // non-empty, checked; CapabilityIdV1 = u32-tagged newtype
}

struct NegotiationSelectorV1 {
    owner: SelectorOwnerV1,          // enum { ServerAuthoritative, ClientPreferred } -- explicit, never implied
    algorithm: SelectorAlgorithmV1,  // enum { HighestMutualVersion, ExactMatchOnly } -- closed vocabulary, extensible by new variant
    version: SchemaVersion,          // T0.1 -- the selector's own algorithm/owner contract version
}

struct TransformKeyV1 {
    transform_id: TransformIdV1,     // u32-tagged newtype, build step 1's frozen transform semantics
    from_schema: SchemaVersion,
    to_schema: SchemaVersion,
    implementation_root: ContentIdentityV1,
}

struct TransformRegistryV1(BTreeMap<TransformKeyV1, ()>); // exact-key lookup; registration rejects a duplicate/conflicting key
```

`TransformRegistryV1` explicitly does **not** compute or execute
multi-hop transform chains (`from -> mid -> to`) — the row block's own
anti-goal, deferred to the save-migration/runtime rows (`T4.5`/`T4.6`)
where multi-hop planning is actually in scope.

### 3.7 `CompatibilityReportV1` and unknown-extension policy

```
enum CompatibilityOutcomeV1 {
    Compatible,
    Incompatible { reason: IncompatibilityReasonV1 },
    InvalidInput { reason: InvalidInputReasonV1 },
}

struct CompatibilityResultV1 {
    slot: SubsystemSlotIdV1,
    outcome: CompatibilityOutcomeV1,
}

struct CompatibilityReportV1(Vec<CompatibilityResultV1>);  // one entry per profile rule, sorted by slot tag, never short-circuits
```

```
enum ExtensionCriticalityV1 { Critical, Noncritical }
```

Evaluation rule (build step 8, made concrete): when the evaluator meets a
`CompatibilityRuleV1::Unknown` entry, `criticality: Critical` always
produces `Incompatible` (fail closed — an unrecognized rule this build
cannot interpret must never be silently treated as satisfied);
`criticality: Noncritical` produces `Compatible` but the `Unknown` entry's
raw tag/payload is still carried in the report's evidence (never dropped
silently) so a human/log reviewing the report can see an extension was
present and ignored, not infer it from an absent field.

**Known-variant, invalid wire content (clarification requested in spec
review, folded in — Opus 5, 2026-07-27):** `Unknown` covers a variant
*this build's enum cannot name at all*. A different case is a variant it
*does* name — `AcceptRange`, `AcceptSet` — arriving over the wire with
content that violates that variant's own construction invariant (`min >
max`, an empty `schemas` list): content a local caller could never
produce (the checked constructors in §3.4 make it locally
unrepresentable) but a decoder must still be able to meet, because it is
reading peer-supplied bytes, not calling the constructor. Resolution:
this is a **decode-time failure, not an evaluation-time
`InvalidInput`**. `CompatibilityRuleV1`'s `ManifestDecodeV1` impl
reconstructs each variant through the *same* checked constructor
`AcceptRange::new`/`AcceptSet::new` (§3.4) uses locally; when that
constructor rejects the decoded fields, decode returns
`Err(ManifestDecodeErrorV1)` for the whole rule, which — because
`CompatibilityProfileV1::ManifestDecodeV1` decodes its rule list
strictly, one entry at a time, propagating the first error — fails the
*entire profile's* decode. No partial `CompatibilityReportV1` is ever
produced from a profile that failed to decode; evaluation never runs
against it. This keeps the fail-closed policy from §3.4 ("invalid field
combinations must be unrepresentable") consistent end-to-end: they are
unrepresentable **in memory** by construction, and unrepresentable **on
the wire** by decode failure — never silently accepted as some other
in-memory shape and only caught later as a soft `InvalidInput` report
entry. `CompatibilityOutcomeV1::InvalidInput` stays reserved for what it
already covers: a *structurally well-formed, successfully decoded* rule
whose semantics the evaluator cannot resolve in context (for example, a
rule slot with no matching descriptor supplied to the evaluator at all —
malformed input at the evaluation call boundary, not malformed wire
bytes). This mirrors T0.2's own existing decode philosophy exactly — "a
decoder must never call [the checked constructor] to 'fix up'
noncanonical received bytes" (`CanonicalFieldMapV1::try_from_entries`'s
doc comment) applies equally here: decode either produces a value that
already satisfies every local invariant, or it fails; it never repairs.

## 4. Encoding

New `DigestDomainIdV1` variants (T0.3, appended non-destructively — same
pattern as `PluginManifest = 8`'s own addition):

```
SubsystemDescriptor  = 9    // digest domain for SubsystemDescriptorV1 content identity
CompatibilityProfile = 10   // digest domain for CompatibilityProfileV1 content identity
```

9/10 allocated here; `APEX-T1.2`'s `SourceClosure` domain is `11` (cross-lane
collision on `9` caught in spec review, resolved by row-order allocation —
`T0.5` `sequence_index=8` precedes `T1.2` `sequence_index=10` in the
registry; the earlier row keeps the lower numbers. Fable adopted this as
the fleet's standing collision rule for future domain-ID additions.)

`SubsystemDescriptorV1`, `CompatibilityProfileV1`, `CompatibilityReportV1`,
and every nested type implement `ManifestEncodeV1`/`ManifestDecodeV1`
(T0.2) using `FieldIdV1`-tagged struct fields (`StructFieldsV1`) and
`VariantTagV1`-tagged enum variants, exactly like `ContentIdentityV1`'s
own existing `ManifestEncodeV1`/`ManifestDecodeV1` impl in
`common/src/apex/digest/mod.rs`. `CompatibilityRuleV1::Unknown`'s
`raw_payload` is encoded as a length-prefixed `Bytes` value
(`ManifestValueV1`'s existing byte-string variant) so it round-trips
without this build needing to understand its contents.

## 5. Build steps (concrete, mapped to files)

1. `common/src/apex/subsystem/slot.rs` — `SubsystemSlotIdV1` (§3.1),
   `as_u16()`/`label()`/`ALL`, duplicate-ID/label self-tests (mirrors
   `common/src/apex/digest/domain.rs`).
2. `common/src/apex/subsystem/descriptor.rs` — `SubsystemDescriptorV1`
   (§3.2), the three typed protocol-root newtypes (§3.3).
3. `common/src/apex/subsystem/rule.rs` — `CompatibilityRuleV1` (§3.4) with
   checked constructors for `AcceptSet`/`AcceptRange`.
4. `common/src/apex/subsystem/profile.rs` — `CompatibilityProfileV1`
   (§3.5), checked constructor (duplicate-slot rejection, cardinality
   bound).
5. `common/src/apex/subsystem/negotiate.rs` — `CapabilityRequirementV1`,
   `CapabilityIdV1`, `NegotiationSelectorV1`, `SelectorOwnerV1`,
   `SelectorAlgorithmV1` (§3.6).
6. `common/src/apex/subsystem/transform.rs` — `TransformKeyV1`,
   `TransformIdV1`, `TransformRegistryV1` with duplicate/conflict-checked
   registration (§3.6).
7. `common/src/apex/subsystem/report.rs` — `CompatibilityOutcomeV1`,
   `CompatibilityResultV1`, `CompatibilityReportV1`,
   `ExtensionCriticalityV1`, the evaluator function that walks a profile
   against a set of descriptors/catalogs and produces a report in slot-tag
   order without short-circuiting (§3.7).
8. `common/src/apex/digest/domain.rs` — append `SubsystemDescriptor = 9`,
   `CompatibilityProfile = 10` to `DigestDomainIdV1::ALL` (§4).
9. `ManifestEncodeV1`/`ManifestDecodeV1` impls for every new type (§4),
   modeled on `ContentIdentityV1`'s existing impl.
10. `common/src/apex/subsystem/mod.rs` — module wiring + re-exports;
    `common/src/apex/mod.rs` gains `pub mod subsystem;`.
11. `common/tests/apex_subsystem_compatibility_v1.rs` — hostile tests
    (§6) plus golden vectors (§7), following
    `common/tests/apex_manifest_encoding_v1.rs`'s existing structure
    (fixture JSON + round-trip + negative cases in one file).
12. `readme/apex/APEX-SUBSYSTEM-COMPATIBILITY-GOLDEN-VECTORS-v1.json` —
    this program's own fleet-generated vectors (not the missing/
    hallucinated file of the same conceptual role under a different
    name), generated the same way T0.1's `SCALAR-GOLDEN-VECTORS` and
    T0.2's fixture were: self-generated from the real encoder, never
    hand-typed hex.

No source-fitness-scan tooling (row step 11: "prevent independent
manifest-specific descriptor/compatibility enums from appearing before
justified specialization") is built as a separate CI tool in this pass —
folded into the PR-review-time check that later rows (`T4.1`, `T4.3`,
`T4.4`, `T6.1`) reuse this row's types rather than inventing their own,
same as how this program has relied on human/architect review rather than
tooling for analogous cross-cutting invariants elsewhere (documented
honestly here rather than silently skipped).

## 6. Hostile test plan (row step 10, made concrete)

- Duplicate slots in one profile → rejected at construction.
- Wrong artifact bytes (`ContentIdentityV1.artifact` mutated one byte) →
  `Exact` rule evaluates `Incompatible`, never `InvalidInput`.
- Semantic-root substitution (a descriptor's `semantic: Some(..)` swapped
  for a different valid root) → `Exact` rule still keys off `artifact`
  only if the rule doesn't itself pin `semantic`; a rule that does pin
  `semantic` must catch the substitution as `Incompatible`.
- Contradictory rules (same slot present twice pre-construction-check,
  proving the checked constructor — not just a doc comment — is what
  blocks it).
- Selector drift: two profiles with the same rules but different
  `NegotiationSelectorV1.algorithm` → `InvalidInput`, not a silent
  fallback to one side's algorithm.
- Unauthorized transform: a `DirectTransform` rule whose `TransformKeyV1`
  is absent from the registry → `Incompatible`, not panic/`Ok`.
- `InvalidInput` vs `Incompatible` vs decode-failure boundary (§3.7
  clarification): a rule slot with no matching descriptor supplied to the
  evaluator → `InvalidInput` (structurally fine, semantically
  unresolvable at the call boundary); a well-formed rule that is simply
  not satisfied → `Incompatible`. Corrected from an earlier draft of this
  test plan: a wire-crafted `AcceptRange` with `min > max` (or an empty
  `AcceptSet`) — content that could not have been constructed locally at
  all — is **not** an `InvalidInput` report entry. Decoding
  `CompatibilityRuleV1` runs the same checked constructor decode-side;
  feed `decode_manifest_v1::<CompatibilityRuleV1>` bytes encoding
  `min > max` directly (bypassing `AcceptRange::new`) and assert
  `Err(ManifestDecodeErrorV1)`, then assert that embedding those same
  bytes as one rule inside an otherwise-valid multi-rule profile fails
  the *whole profile's* decode (`Err`, not a report with 3-of-4 entries
  and one `InvalidInput`) — one poisoned rule poisons the whole decode,
  never a partial report built from partially-untrusted input.
- Extension handling: one `Critical` `Unknown` entry → whole report
  contains an `Incompatible` for that slot; one `Noncritical` `Unknown`
  entry → `Compatible` for that slot with the raw entry preserved in the
  report for evidence (asserted by reading the report's `Unknown` payload
  back out, not just checking outcome).
- Truncation: a manifest-encoded profile with its final byte(s) dropped →
  `ManifestDecodeErrorV1`, not a partially-decoded profile.
- Cross-target equality: the same profile encoded on two different builds
  (or the same build twice) produces byte-identical output — this is a
  restatement of T0.2's own guarantee, re-asserted here because it is
  T0.5's new types exercising it, not a new mechanism.

## 7. Golden/mutation vectors

Self-generated (never hand-typed), following the exact precedent set by
`common/tests/fixtures/apex_manifest_v1/golden-vectors.json` (T0.2) and
`readme/apex/APEX-BOUNDARY-INVENTORY-SEED-v1.csv` (T0.1): a fixture file
covering at minimum one instance of every `CompatibilityRuleV1` variant
(including `Unknown` with both criticality values), one multi-slot
profile, and one full report — each with its `ManifestEncodeV1` bytes and
`DigestDomainIdV1::SubsystemDescriptor`/`CompatibilityProfile` digest
recorded, generated by a `bastion-harness` bin (same pattern as
`apex_emit_manifest_cbor`), SHA-256-pinned, and round-trip-verified at
emission time.

## 8. Acceptance (verbatim, restated as a checklist)

- [ ] every descriptor/profile has one canonical encoding
- [ ] duplicated artifact identity is impossible (via `ContentIdentityV1`
      reuse, not a parallel identity field)
- [ ] compatibility rules are typed variants (no boolean-soup struct)
- [ ] negotiation selection is explicitly owned/versioned
      (`NegotiationSelectorV1`)
- [ ] all rule results are reported in total order (slot-tag sorted,
      never short-circuited)
- [ ] critical unknowns and invalid inputs fail closed
- [ ] direct transforms require exact authorization (registry lookup,
      no chain execution)
- [ ] vectors and mutation canaries pass cross-target

## 9. Explicit non-goals (row's own scope correction, §1)

No `BootstrapManifestV1`/`SaveUniverseManifestV1`/`PluginActivationPlan`/
`WorldBaselineManifestV1` construction — those are `T4.1`/`T4.3`/`T4.6`'s
job, consuming this row's types. No multi-hop transform graph
planning/execution — deferred to `T4.5`/`T4.6`. No single universal
compatibility rule spanning all subsystems — `CompatibilityRuleV1`'s tag
variants are the abstraction; nothing here assumes one rule shape fits
every future slot.
