# APEX-T2.2 — Fleet-authored spec: canonical plugin archive profile

> **STATUS: DRAFT — pending cross-review.** Author: Builder Opus 5,
> 2026-07-27. Not build-authorized. Per Ben's order (routed via Fable):
> author → Sonnet 5 cross-review → Fable approval → then build. Registry
> disposition `specification=FLEET_AUTHORED`.
>
> Grounding trust posture (Fable's standing ruling): inline master-order
> content is ADMISSIBLE GROUNDING, never inherited authority; landed code
> wins on conflict. This row's canary catalog is PIN-VERIFIED REAL (the
> T3.2 pattern: prose packet hallucinated, raw canary JSON survived).

## 0. Provenance

The prose packet (`PROJECT-BASTION-APEX-MICROSTEP-APEX-T2.2-CANONICAL-
PLUGIN-ARCHIVE-PROFILE.md`) is `.gdoc`-only — hallucination-class,
unrecoverable per Ben's confirmation. Not consulted.

**All three canary catalogs survived intact and are the primary
grounding.** Independently re-hashed this session (not taken from any
prior report):

| file | cases | bytes | SHA-256 (verified == master-order pin) |
|---|---|---|---|
| `…-PROFILE-CANARIES-v1.json` (base) | 50 (PAR-001..050) | 8,342 | `8ead4e3596a922089a1e314ab0e56168c9694b29f51b177ef77c56d6968fdd83` |
| `…-CORRECTION-CANARIES-v1.json` | 14 (PAR-C01..C14) | 3,471 | `89b99145c110f199c145e5272ea3a677d61861bf9a71006565734ec861a5663f` |
| `…-FINAL-CANARIES-v1.json` | 26 (PAR-C15..C40) | 6,462 | `30c67ae1ba03f947f12821237dee37a2a4323d1061782572f9e32f743a4e964e` |

Total logical catalog: **90 cases, 74 distinct expected terminals**
(full generated coverage table in section 10 — every terminal name in
this spec is the catalog's own, never invented here).

Also grounded in: the master order's inline row block (12 build steps +
acceptance, reproduced in section 1) and LIVE T2.1 code (mine, landed
`0ca81f3826`, reviewed): `InspectedPluginArchive` (side-effect-free,
single-buffer, sequential entry inventory), private
`Plugin::instantiate`, `PreparedPluginBatch` (all-inspect →
all-instantiate → all-asset-prepare → ONE commit),
`CombinedCache::prepare_tar`/`commit_prepared_tars`, `from_dir` /
`load_server_plugin` ingress. Today the archive is read EXCLUSIVELY
through the `tar` crate (tar-rs) over one immutable buffer; there is no
raw framing check, no path grammar, no namespace index, no archive
identity of any kind.

## 1. Inline row block (admissible grounding, reproduced)

Deferred production decisions (owned by T2.5, NOT T2.2 blockers):
inventory-backed numerical limit values; deployed legacy archive
repack/waiver deadlines; production StrictCanonicalV1 rollout point.
T2.2 freezes: mandatory injected limits, ASCII/lowercase portability
policy, legacy Observe-only semantics, exact UStar bytes, test-only
strict admission. Prerequisites: T0.2–T0.3, T2.1.

Build steps 1–12 and acceptance: as printed in the master order (T2.2
row block); each step maps to a minute step in section 8 — no step
dropped, two sharpened against landed code (noted inline there).

## 2. Determinism story (required before building — permanent law)

The archive profile makes plugin-archive identity a PURE FUNCTION of
regular-file content, never of host or packaging incidentals:

1. **Framing truth is the raw 512-byte block grammar** — checksum,
   declared sizes, padding bytes, exactly-two-zero-block terminator,
   trailing data. tar-rs's view is RECONCILED against the same immutable
   bytes and any disagreement is a typed reject
   (`REJECT-PARSER-VIEW-MISMATCH`, PAR-C17); tar-rs is never substituted
   for framing truth (`BLOCK-TAR-RS-FRAMING-SUBSTITUTION`, PAR-C04). Two
   parsers agreeing on the same bytes is the cross-check; one parser's
   tolerance can never widen admission.
2. **Path identity comes from raw UStar name+prefix field bytes** —
   never a host `PathBuf` (no OS-dependent separator/unicode behavior can
   enter identity; `REJECT-RAW-BACKSLASH` PAR-C15 rejects at the raw
   byte, before any host path type exists). The frozen ASCII grammar +
   ASCII-lowercase collision key make the namespace portable across
   case-insensitive filesystems (`REJECT-PORTABLE-CASE-COLLISION`).
3. **Namespace order is canonical** (path-byte sorted), never archive
   entry order; duplicate paths are REJECTED, so last-entry-wins is
   unrepresentable (`REJECT-DUPLICATE-CANONICAL-PATH`).
4. **Two separated identities:** exact artifact identity
   (`hash_artifact_bytes_v1` over the archive bytes — moves with ANY
   byte) and the domain-separated SEMANTIC root over sorted
   (path, kind, size, content) regular-file records — excludes raw
   ordinal and tar metadata (`CANONICAL-ROOT-EXCLUDES-ARCHIVE-ORDINAL`
   PAR-C11, `SEPARATE-ARTIFACT-AND-SEMANTIC-IDENTITY` PAR-C40,
   `ACCEPT-SAME-SEMANTIC-ROOT-DIFFERENT-ARTIFACT` PAR-004). Same
   content repacked differently ⇒ same semantic root, different
   artifact id — both facts recorded, neither conflated.
5. **The canonical packer is reproducible by construction**: fixed UStar
   metadata (zeroed mtime/uid/gid/uname/gname, fixed mode policy),
   path-sorted entries, no directory records, inspect-after-pack
   verification (`CANONICAL-PACKER-REPRODUCIBLE` PAR-C13,
   `-HOST-METADATA-INDEPENDENT` PAR-C22, `-OMITS-DIRECTORY-RECORDS`
   PAR-C33). Prior art, named: reproducible-builds.org tar guidance
   (--sort=name, --mtime pinning, numeric owners) — realized here as a
   repository-owned writer rather than tar-flag discipline, because a
   writer we own is inspectable and the flags are not portable.
6. **No hidden defaults:** every limit is an injected, recorded policy
   value (`OBSERVE-NO-HIDDEN-DEFAULT-LIMITS` PAR-C09) — T0.2's
   no-Default-limits precedent applied to archive admission.

## 3. Data model (NEW-SPEC; field IDs frozen at cross-review)

```rust
// common/state/src/plugin/archive_profile.rs (NEW), types encoding via
// T0.2 ManifestEncodeV1 where they enter evidence.
enum ArchiveAdmissionModeV1 { ObserveLegacy, StrictCanonicalV1 }   // typed, explicit
enum TarDialectV1 { UstarStrict, Gnu, Pax, OldV7 }                 // observed, never inferred twice
struct ArchiveLimitsPolicyV1 {          // MANDATORY injection; no Default impl
    policy_id: MachineTextV1,           // recorded in every observation
    max_archive_bytes: u64, max_entry_bytes: u64,
    max_entries: u64, max_path_bytes: u64, max_manifest_bytes: u64,
}
struct RawEntryObservationV1 {          // ObserveLegacy: EVERY tar entry, pre-reduction
    ordinal: u64,                       // observation only — excluded from semantic root
    raw_name: Vec<u8>, raw_prefix: Vec<u8>,   // exact UStar field bytes
    type_flag: u8, declared_size: u64,
    header_checksum_ok: bool, dialect: TarDialectV1,
}
struct CanonicalEntryV1 {               // strict namespace member (regular files only)
    path: CanonicalPathV1,              // frozen ASCII grammar (T0.2 codec)
    portability_key: MachineTextV1,     // ASCII-lowercase collision key
    size_bytes: u64, content_sha256: [u8; 32],
}
struct ArchiveObservationV1 {           // per-archive result, both modes
    mode: ArchiveAdmissionModeV1, dialect: TarDialectV1,
    extension_observed: MachineTextV1,  // ".tar" vs anything else (policy: reject in strict)
    parser_identity: MachineTextV1,     // exact framing-scanner version + tar-rs version
    limits_policy: ArchiveLimitsPolicyV1,
    raw_entries: Vec<RawEntryObservationV1>,        // complete, pre-reduction
    namespace: Vec<CanonicalEntryV1>,               // path-byte sorted, collision-free
    root_manifest: Option<CanonicalPathV1>,         // exactly one "plugin.toml" in strict
    legacy_module_order: Vec<CanonicalPathV1>,      // + LegacyModuleOrderUnfrozen marker
    artifact: ArtifactIdentityV1,
    semantic_root: ProtocolDigestV1,                // plugin-archive domain (section 5)
    terminal: MachineTextV1,                        // catalog terminal name
}
```

## 4. Policy (fleet-authored, cross-review targets)

1. **ObserveLegacy is side-effect-free and total**: every tar entry gets
   a `RawEntryObservationV1` BEFORE any legacy compatibility reduction;
   GNU/PAX/old-V7 dialects, missing terminators, longname records are
   OBSERVED with their own terminals (PAR-C01/C02/C03/C06/C12), never
   silently normalized. Observe mode can never reject an archive the
   current loader accepts — it annotates; the legacy loader's admission
   behavior is byte-for-byte unchanged (before/after in section 7).
2. **StrictCanonicalV1 is fail-closed and test-only until T2.5**: strict
   admission without T2.5's rollout policy is itself a typed block
   (`BLOCK-STRICT-ROLLOUT-POLICY-MISSING`, PAR-C14). No legacy fallback
   from strict — a strict rejection is final (no "try legacy" ladder).
3. **Strict rejection has ZERO side effects**: no Wasmtime instance, no
   manager entry, no ECS effect, no global asset source (inline
   acceptance; enforced at the T2.1 seam — rejection happens in
   `InspectedPluginArchive` inventory phase, before
   `PreparedPluginBatch` construction, which is the landed no-commit
   boundary).
4. **Exactly one bounded regular root `plugin.toml`** in strict mode
   (missing/duplicate/non-regular/oversize each typed: PAR-007/008/009/
   044); only the CURRENT legacy schema is parsed — T2.3 owns the V1
   manifest schema (no forward-parsing here).
5. **UStar split policy**: canonical rightmost-valid-slash name/prefix
   split; noncanonical-but-representable splits are rejected in strict
   (`REJECT-NONCANONICAL-USTAR-SPLIT` PAR-C30) — one path, one canonical
   byte encoding, or the packer/writer is lying
   (`REJECT-WRITER-PATH-TRANSFORMATION` PAR-C31). Boundary vectors
   PAR-C26/C27/C28/C29 freeze the 100/155-byte edges.
6. **Directory records**: strict archives contain none (implied
   directories only — `REJECT-EXPLICIT-DIRECTORY-IN-STRICT-V1` PAR-C23,
   `REJECT-STRICT-EXPLICIT-DIRECTORY` PAR-C34, packer omits PAR-C33) but
   the DIRECTORY NAMESPACE (implied parents) IS part of the semantic
   root (`CANONICAL-ROOT-INCLUDES-DIRECTORY-NAMESPACE` PAR-C10):
   `a/b.wasm` and `a-b.wasm` must not collide into one identity space.
7. **Module declarations** resolve through the canonical path/index gate
   (missing/alias/non-regular/path-form each typed: PAR-045..048;
   duplicates raw + canonical: PAR-C18/C19); legacy module ORDER is
   observed and reported `LegacyModuleOrderUnfrozen` (PAR-C12) — T2.4
   owns freezing it.
8. **Domain allocation**: the semantic root's label is FROZEN here as
   `bastion/plugin-archive/v1`; the NUMBER is resolved at registration
   time per the row-order rule — T1.4's real packet is checked for a
   domain claim first (T1.4 precedes T2.2); if it claims none, T2.2
   takes the next free ID after `LocalReproSmoke = 12`. Not
   `PluginManifest = 8` (that is T2.3's manifest-semantic root) and not
   `PluginActivationPlan = 3` (T2.5's plan) — the archive's content
   identity is an INPUT to both, and inputs are domain-separated from
   the things that embed them (SourceClosure-vs-BuildManifest
   precedent).

## 5. Live wire integration (concrete, landed-code seams)

- `common/state/src/plugin/mod.rs`: `InspectedPluginArchive` gains the
  framing scanner + observation pass; `Plugin::from_reader` path stays
  tar-rs-backed for extraction but every extraction is preceded by the
  raw framing verdict over the SAME buffer (single-buffer invariant is
  already landed T2.1 — no re-read window).
- Observe wiring: every T2.1 ingress (`from_dir`, `load_server_plugin`,
  the network plugin path through `CombinedCache::prepare_tar`) records
  an `ArchiveObservationV1`; none of them changes admission.
- The canonical packer is a REPOSITORY tool (`bastion-harness` bin or
  `tools/`, implementer's call per A.1 helper precedent), not a runtime
  server capability — the server never packs.

## 6. Typed terminals

The terminal namespace is the catalog's own 74 names (section 10) —
this spec adds NONE. Classes: `ACCEPT*` (9 names), `OBSERVE-*` (10,
ObserveLegacy annotations + strict-mode cross-references), `REJECT-*`
(53, strict fail-closed), `BLOCK-*` (2, meta: tar-rs substitution,
strict rollout without policy), `CANONICAL-*`/`SEPARATE-*` (packer +
identity invariants).

## 7. Live-code BEFORE/AFTER (per the T3.2 elevated precedent)

- **`from_dir`/`load_server_plugin`/network prepare (T2.1 ingress):**
  Before: tar-rs inventory only (landed T2.1), no identity, no framing
  check. After: same admission decisions in ObserveLegacy (annotation
  only — a plugin that loads today still loads, with an observation
  record attached); strict mode exists but is unreachable in production
  until T2.5 (PAR-C14 enforced). Client/server-observable delta in
  production: NONE beyond the observation records + log lines. Perf: one
  additional O(bytes) pass over an already-in-memory buffer at plugin
  load (rare, startup-dominated).
- **Packer:** new tool, no live path.
- **Rollback:** all new code behind the observation pass + a new bin;
  reverting the T2.2 commits restores the exact T2.1 loader (the
  Observe hook is additive at the inventory seam, not a rewrite).
  No wire schema changes in this row ⇒ no version-bump entanglement.

## 8. Minute steps

- **T2.2.01** Types + limits policy + terminal table (this spec §3/§6
  frozen). Gate: cross-review sign-off.
- **T2.2.02** Raw 512-byte framing scanner (checksum, sizes, padding,
  exact-two-zero-block terminator, concatenation, trailing data —
  PAR-037..040, C05..C07, C35..C37). Gate: framing canaries bite.
- **T2.2.03** tar-rs reconciliation against the same bytes
  (PAR-C04/C08/C17 + parser-identity recording). Gate: a synthetic
  disagreement archive REJECTS, never widens.
- **T2.2.04** Raw UStar path identity + ASCII grammar + portability key
  + split policy (PAR-014..024, C15/C16, C21, C26..C31). Gate: the
  Windows/Linux path vectors pass identically (grammar is byte-level,
  host-independent by construction).
- **T2.2.05** Namespace index + duplicate/collision rejection + implied
  directory namespace (PAR-010..013, C10, C23/C34). Gate: collision
  canaries bite; last-entry-wins impossible by test.
- **T2.2.06** Root manifest gate + legacy module resolution
  (PAR-007..009, 044..048, C12, C18/C19). Gate: module canaries bite.
- **T2.2.07** Artifact identity + semantic root (domain per §4.8;
  PAR-003/004, 049/050, C11, C40). Gate: repack-invariance +
  ordinal-exclusion canaries.
- **T2.2.08** ObserveLegacy wiring into all T2.1 ingresses (PAR-C01..C03,
  C06, C09, C12, C20, C24, C38/C39). Gate: legacy corpus (the real
  shipped plugin archives in-tree) loads UNCHANGED with observations
  attached.
- **T2.2.09** Canonical packer + inspect-after-pack (PAR-C13, C22,
  C31..C33). Gate: pack→inspect→byte-identical repack.
- **T2.2.10** StrictCanonicalV1 assembly + rollout-policy block
  (PAR-001..006, 025..036, 041..043, C14, C25). Gate: full 90-case run
  (`tools/apex-t2-2-archive-canaries.sh`) + entry permutations +
  host-metadata perturbations + mutation guards.

## 9. Non-goals

T2.5 owns: production limit VALUES, legacy repack/waiver deadlines,
strict rollout point, conflict/override policy, activation order. T2.3
owns the V1 manifest schema; T2.4 owns the dependency DAG + module-order
freeze. This row makes no admission-behavior change in production.

## 10. Canary coverage table (generated from the verified catalogs)

90 cases → minute steps; every ID appears exactly once. (Generated
programmatically from the three pinned JSON files, not hand-transcribed;
regenerate with the one-liner in the build packet when auditing.)

| terminal | cases | resolved by |
|---|---|---|
| ACCEPT | PAR-001,002,005,006 | T2.2.10 |
| ACCEPT-CANONICAL-USTAR-SPLIT-BOUNDARY | PAR-C28 | T2.2.04 |
| ACCEPT-DIFFERENT-SEMANTIC-ROOT | PAR-049 | T2.2.07 |
| ACCEPT-EXACT-TWO-ZERO-BLOCK-TERMINATOR | PAR-C35 | T2.2.02 |
| ACCEPT-FIXED-USTAR-METADATA | PAR-C32 | T2.2.09 |
| ACCEPT-SAME-SEMANTIC-ROOT | PAR-003,050 | T2.2.07 |
| ACCEPT-SAME-SEMANTIC-ROOT-DIFFERENT-ARTIFACT | PAR-004 | T2.2.07 |
| ACCEPT-USTAR-NAME-100-BYTE-BOUNDARY | PAR-C26 | T2.2.04 |
| ACCEPT-USTAR-PREFIX-NAME-VECTOR | PAR-C16 | T2.2.04 |
| BLOCK-STRICT-ROLLOUT-POLICY-MISSING | PAR-C14 | T2.2.10 |
| BLOCK-TAR-RS-FRAMING-SUBSTITUTION | PAR-C04 | T2.2.03 |
| CANONICAL-PACKER-HOST-METADATA-INDEPENDENT | PAR-C22 | T2.2.09 |
| CANONICAL-PACKER-OMITS-DIRECTORY-RECORDS | PAR-C33 | T2.2.09 |
| CANONICAL-PACKER-REPRODUCIBLE | PAR-C13 | T2.2.09 |
| CANONICAL-ROOT-EXCLUDES-ARCHIVE-ORDINAL | PAR-C11 | T2.2.07 |
| CANONICAL-ROOT-INCLUDES-DIRECTORY-NAMESPACE | PAR-C10 | T2.2.05 |
| OBSERVE-DUPLICATE-RAW-DEPENDENCY-DECLARATION | PAR-C20 | T2.2.08 |
| OBSERVE-GNU-LONGNAME-STRICT-REJECT | PAR-C38 | T2.2.08 |
| OBSERVE-GNU-NOT-STRICT | PAR-C24 | T2.2.08 |
| OBSERVE-LEGACY-GNU-DIALECT | PAR-C01 | T2.2.08 |
| OBSERVE-LEGACY-GNU-LONGNAME | PAR-C02 | T2.2.08 |
| OBSERVE-LEGACY-MISSING-TERMINATOR | PAR-C06 | T2.2.08 |
| OBSERVE-LEGACY-MODULE-ORDER | PAR-C12 | T2.2.06 |
| OBSERVE-LEGACY-PAX | PAR-C03 | T2.2.08 |
| OBSERVE-NO-HIDDEN-DEFAULT-LIMITS | PAR-C09 | T2.2.08 |
| OBSERVE-PAX-STRICT-REJECT | PAR-C39 | T2.2.08 |
| REJECT-ABSOLUTE-PATH | PAR-014 | T2.2.04 |
| REJECT-ARCHIVE-SIZE-LIMIT | PAR-042 | T2.2.10 |
| REJECT-BACKSLASH | PAR-015 | T2.2.04 |
| REJECT-CURRENT-SEGMENT | PAR-017 | T2.2.04 |
| REJECT-DECLARED-MODULE-ALIAS | PAR-047 | T2.2.06 |
| REJECT-DECLARED-MODULE-MISSING | PAR-045 | T2.2.06 |
| REJECT-DECLARED-MODULE-NOT-REGULAR | PAR-046 | T2.2.06 |
| REJECT-DECLARED-MODULE-PATH | PAR-048 | T2.2.06 |
| REJECT-DUPLICATE-CANONICAL-MODULE-DECLARATION | PAR-C19 | T2.2.06 |
| REJECT-DUPLICATE-CANONICAL-PATH | PAR-011 | T2.2.05 |
| REJECT-DUPLICATE-MANIFEST | PAR-008 | T2.2.06 |
| REJECT-DUPLICATE-RAW-MODULE-DECLARATION | PAR-C18 | T2.2.06 |
| REJECT-EMPTY-SEGMENT | PAR-018 | T2.2.04 |
| REJECT-ENTRY-COUNT-LIMIT | PAR-041 | T2.2.10 |
| REJECT-ENTRY-SIZE-LIMIT | PAR-043 | T2.2.10 |
| REJECT-EXPLICIT-DIRECTORY-IN-STRICT-V1 | PAR-C23 | T2.2.05 |
| REJECT-EXTENSION-POLICY | PAR-032..035 | T2.2.10 |
| REJECT-INVALID-UTF8 | PAR-021 | T2.2.04 |
| REJECT-MALFORMED-TAR | PAR-037 | T2.2.02 |
| REJECT-MANIFEST-NOT-REGULAR | PAR-009 | T2.2.06 |
| REJECT-MANIFEST-SIZE-LIMIT | PAR-044 | T2.2.06 |
| REJECT-MISSING-CANONICAL-TERMINATOR | PAR-C05 | T2.2.02 |
| REJECT-MISSING-MANIFEST | PAR-007 | T2.2.06 |
| REJECT-MISSING-TERMINATOR | PAR-039 | T2.2.02 |
| REJECT-NON-PORTABLE-CHARACTER | PAR-022,023 | T2.2.04 |
| REJECT-NONCANONICAL-USTAR-SPLIT | PAR-C30 | T2.2.04 |
| REJECT-NONREPRESENTABLE-USTAR-PATH | PAR-C21 | T2.2.04 |
| REJECT-NONZERO-TRAILING-DATA | PAR-C07 | T2.2.02 |
| REJECT-NUL-IN-PATH | PAR-020 | T2.2.04 |
| REJECT-OLD-HEADER-IN-STRICT-V1 | PAR-C25 | T2.2.10 |
| REJECT-ONE-ZERO-BLOCK-TERMINATOR | PAR-C36 | T2.2.02 |
| REJECT-PARENT-SEGMENT | PAR-016 | T2.2.04 |
| REJECT-PARSER-IDENTITY-MISMATCH | PAR-C08 | T2.2.03 |
| REJECT-PARSER-VIEW-MISMATCH | PAR-C17 | T2.2.03 |
| REJECT-PATH-KIND-COLLISION | PAR-013 | T2.2.05 |
| REJECT-PATH-TOO-LONG | PAR-024 | T2.2.04 |
| REJECT-PORTABLE-CASE-COLLISION | PAR-010,012 | T2.2.05 |
| REJECT-RAW-BACKSLASH | PAR-C15 | T2.2.04 |
| REJECT-REGULAR-TRAILING-SLASH | PAR-019 | T2.2.04 |
| REJECT-STRICT-EXPLICIT-DIRECTORY | PAR-C34 | T2.2.05 |
| REJECT-TRAILING-DATA | PAR-040 | T2.2.02 |
| REJECT-TRAILING-ZERO-BLOCKS | PAR-C37 | T2.2.02 |
| REJECT-TRUNCATED-ARCHIVE | PAR-038 | T2.2.02 |
| REJECT-UNSUPPORTED-ENTRY-TYPE | PAR-025..031,036 | T2.2.10 |
| REJECT-USTAR-NAME-101-BYTE-BOUNDARY | PAR-C27 | T2.2.04 |
| REJECT-USTAR-PREFIX-OVERFLOW | PAR-C29 | T2.2.04 |
| REJECT-WRITER-PATH-TRANSFORMATION | PAR-C31 | T2.2.09 |
| SEPARATE-ARTIFACT-AND-SEMANTIC-IDENTITY | PAR-C40 | T2.2.07 |

## 11. Row acceptance gate

1. All 90 catalog canaries pass with their exact expected terminals.
2. The in-tree legacy plugin corpus loads byte-for-byte unchanged in
   ObserveLegacy with observations attached (no admission delta).
3. Canonical packer: pack → inspect → repack is byte-identical, and its
   output is host-metadata independent (two hosts, same bytes).
4. Strict rejection provably creates zero Wasmtime/manager/ECS/asset
   effects (asserted at the landed T2.1 no-commit boundary).
5. tar-rs never decides framing: the substitution mutation makes the
   suite RED (PAR-C04 as mutation guard).
6. Entry-permutation + host-metadata perturbation campaigns green
   (semantic root invariant; artifact id moves — both asserted).
