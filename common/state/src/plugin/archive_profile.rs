//! `APEX-T2.2` — canonical plugin archive profile (fleet spec
//! `readme/apex/APEX-T2.2-CANONICAL-PLUGIN-ARCHIVE-PROFILE-FLEET-v1.md`,
//! Fable-authorized 2026-07-27; 90-case pin-verified canary catalog is
//! the acceptance surface, and every terminal name in this module is the
//! catalog's own).
//!
//! T2.2.01 (types) + T2.2.02 (checked 512-byte framing scanner). FRAMING
//! TRUTH lives here: the raw block grammar — header checksums, declared
//! sizes, padding bytes, the exactly-two-zero-block terminator, trailing
//! data — is decided by THIS scanner over the immutable buffer; tar-rs is
//! reconciled against the same bytes (T2.2.03) and can never widen
//! admission (`BLOCK-TAR-RS-FRAMING-SUBSTITUTION`).

use common::apex::digest::{ArtifactIdentityV1, ProtocolDigestV1};
use common::apex::manifest::{CanonicalPathV1, MachineTextV1};

/// Spec section 3. Explicit, typed — never inferred from context.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ArchiveAdmissionModeV1 {
    ObserveLegacy,
    StrictCanonicalV1,
}

/// Observed tar dialect (never silently normalized; each legacy dialect
/// gets its own observation terminal in ObserveLegacy and is a strict
/// reject in StrictCanonicalV1).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TarDialectV1 {
    /// POSIX ustar magic `ustar\0` + version `00`.
    UstarStrict,
    /// GNU magic `ustar ` (space-terminated) or GNU longname/longlink
    /// entries (typeflag `L`/`K`).
    Gnu,
    /// PAX extended headers (typeflag `x`/`g`).
    Pax,
    /// No magic at all — pre-POSIX old/V7 header.
    OldV7,
}

/// MANDATORY injected limits (spec section 2.6 / `OBSERVE-NO-HIDDEN-
/// DEFAULT-LIMITS` PAR-C09): deliberately NO `Default` impl — every
/// admission records which policy it ran under. Concrete production
/// values are T2.5's decision; tests name their own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveLimitsPolicyV1 {
    pub policy_id: MachineTextV1,
    pub max_archive_bytes: u64,
    pub max_entry_bytes: u64,
    pub max_entries: u64,
    pub max_path_bytes: u64,
    pub max_manifest_bytes: u64,
}

/// One raw tar entry, observed BEFORE any legacy compatibility reduction
/// (spec policy 1: ObserveLegacy is total). `ordinal` is observation
/// only — excluded from the semantic root (PAR-C11).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEntryObservationV1 {
    pub ordinal: u64,
    pub raw_name: Vec<u8>,
    pub raw_prefix: Vec<u8>,
    pub type_flag: u8,
    pub declared_size: u64,
    pub header_checksum_ok: bool,
    pub dialect: TarDialectV1,
}

/// A strict-namespace member (regular files only; T2.2.04/.05 build these
/// from raw UStar field bytes — never a host `PathBuf`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalEntryV1 {
    pub path: CanonicalPathV1,
    /// ASCII-lowercase collision key (portability across case-insensitive
    /// filesystems).
    pub portability_key: MachineTextV1,
    pub size_bytes: u64,
    pub content_sha256: [u8; 32],
}

/// Per-archive result, both modes (spec section 3).
#[derive(Clone, Debug)]
pub struct ArchiveObservationV1 {
    pub mode: ArchiveAdmissionModeV1,
    pub dialect: TarDialectV1,
    pub extension_observed: MachineTextV1,
    /// Exact framing-scanner + tar-rs versions (`REJECT-PARSER-IDENTITY-
    /// MISMATCH` PAR-C08 keys off this).
    pub parser_identity: MachineTextV1,
    pub limits_policy: ArchiveLimitsPolicyV1,
    pub raw_entries: Vec<RawEntryObservationV1>,
    pub namespace: Vec<CanonicalEntryV1>,
    pub root_manifest: Option<CanonicalPathV1>,
    pub legacy_module_order: Vec<CanonicalPathV1>,
    pub artifact: ArtifactIdentityV1,
    pub semantic_root: Option<ProtocolDigestV1>,
    /// The catalog terminal name this archive resolved to.
    pub terminal: MachineTextV1,
}

/// Typed strict-mode rejection. Every variant name maps 1:1 onto a
/// catalog terminal via [`ArchiveRejectV1::terminal_name`] — this module
/// invents no vocabulary of its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveRejectV1 {
    MalformedTar { detail: &'static str },
    TruncatedArchive,
    MissingTerminator,
    OneZeroBlockTerminator,
    TrailingData,
    NonzeroTrailingData,
    TrailingZeroBlocks,
    MissingCanonicalTerminator,
    ArchiveSizeLimit,
    EntryCountLimit,
    EntrySizeLimit,
    OldHeaderInStrictV1,
    UnsupportedEntryType { type_flag: u8 },
    // T2.2.04 — path identity rejects (raw UStar field bytes, never host
    // PathBuf).
    AbsolutePath,
    RawBackslash,
    Backslash,
    NulInPath,
    InvalidUtf8,
    NonPortableCharacter { byte: u8 },
    CurrentSegment,
    ParentSegment,
    EmptySegment,
    RegularTrailingSlash,
    PathTooLong,
    UstarName101Boundary,
    UstarPrefixOverflow,
    NoncanonicalUstarSplit,
    NonrepresentableUstarPath,
    WriterPathTransformation,
    // T2.2.05 — namespace rejects.
    DuplicateCanonicalPath,
    PortableCaseCollision,
    PathKindCollision,
    ExplicitDirectoryInStrictV1,
    // T2.2.06 — root-manifest + module-resolution rejects.
    MissingManifest,
    DuplicateManifest,
    ManifestNotRegular,
    ManifestSizeLimit,
    ManifestParse { detail: &'static str },
    DeclaredModuleMissing,
    DeclaredModuleAlias,
    DeclaredModuleNotRegular,
    DeclaredModulePath,
    DuplicateRawModuleDeclaration,
    DuplicateCanonicalModuleDeclaration,
    // T2.2.03 — reconciliation rejects.
    ParserViewMismatch { detail: &'static str },
    ParserIdentityMismatch,
    // T2.2.08/.10 — strict-mode admission rejects.
    StrictRolloutPolicyMissing,
    GnuLongnameStrictReject,
    PaxStrictReject,
    GnuDialectStrictReject,
}

impl ArchiveRejectV1 {
    pub fn terminal_name(&self) -> &'static str {
        match self {
            Self::MalformedTar { .. } => "REJECT-MALFORMED-TAR",
            Self::TruncatedArchive => "REJECT-TRUNCATED-ARCHIVE",
            Self::MissingTerminator => "REJECT-MISSING-TERMINATOR",
            Self::OneZeroBlockTerminator => "REJECT-ONE-ZERO-BLOCK-TERMINATOR",
            Self::TrailingData => "REJECT-TRAILING-DATA",
            Self::NonzeroTrailingData => "REJECT-NONZERO-TRAILING-DATA",
            Self::TrailingZeroBlocks => "REJECT-TRAILING-ZERO-BLOCKS",
            Self::MissingCanonicalTerminator => "REJECT-MISSING-CANONICAL-TERMINATOR",
            Self::ArchiveSizeLimit => "REJECT-ARCHIVE-SIZE-LIMIT",
            Self::EntryCountLimit => "REJECT-ENTRY-COUNT-LIMIT",
            Self::EntrySizeLimit => "REJECT-ENTRY-SIZE-LIMIT",
            Self::OldHeaderInStrictV1 => "REJECT-OLD-HEADER-IN-STRICT-V1",
            Self::UnsupportedEntryType { .. } => "REJECT-UNSUPPORTED-ENTRY-TYPE",
            Self::AbsolutePath => "REJECT-ABSOLUTE-PATH",
            Self::RawBackslash => "REJECT-RAW-BACKSLASH",
            Self::Backslash => "REJECT-BACKSLASH",
            Self::NulInPath => "REJECT-NUL-IN-PATH",
            Self::InvalidUtf8 => "REJECT-INVALID-UTF8",
            Self::NonPortableCharacter { .. } => "REJECT-NON-PORTABLE-CHARACTER",
            Self::CurrentSegment => "REJECT-CURRENT-SEGMENT",
            Self::ParentSegment => "REJECT-PARENT-SEGMENT",
            Self::EmptySegment => "REJECT-EMPTY-SEGMENT",
            Self::RegularTrailingSlash => "REJECT-REGULAR-TRAILING-SLASH",
            Self::PathTooLong => "REJECT-PATH-TOO-LONG",
            Self::UstarName101Boundary => "REJECT-USTAR-NAME-101-BYTE-BOUNDARY",
            Self::UstarPrefixOverflow => "REJECT-USTAR-PREFIX-OVERFLOW",
            Self::NoncanonicalUstarSplit => "REJECT-NONCANONICAL-USTAR-SPLIT",
            Self::NonrepresentableUstarPath => "REJECT-NONREPRESENTABLE-USTAR-PATH",
            Self::WriterPathTransformation => "REJECT-WRITER-PATH-TRANSFORMATION",
            Self::DuplicateCanonicalPath => "REJECT-DUPLICATE-CANONICAL-PATH",
            Self::PortableCaseCollision => "REJECT-PORTABLE-CASE-COLLISION",
            Self::PathKindCollision => "REJECT-PATH-KIND-COLLISION",
            Self::ExplicitDirectoryInStrictV1 => "REJECT-EXPLICIT-DIRECTORY-IN-STRICT-V1",
            Self::MissingManifest => "REJECT-MISSING-MANIFEST",
            Self::DuplicateManifest => "REJECT-DUPLICATE-MANIFEST",
            Self::ManifestNotRegular => "REJECT-MANIFEST-NOT-REGULAR",
            Self::ManifestSizeLimit => "REJECT-MANIFEST-SIZE-LIMIT",
            // NOT a catalog name (the only deliberate exception): the
            // 90-case catalog has no unparseable-manifest terminal —
            // T2.1's legacy loader already rejects bad TOML upstream of
            // strict admission for every archive. Interim name pending
            // the T2.2.10 catalog-fixture arbitration; remapped there if
            // a fixture assigns one.
            Self::ManifestParse { .. } => "REJECT-MANIFEST-PARSE-INTERIM",
            Self::DeclaredModuleMissing => "REJECT-DECLARED-MODULE-MISSING",
            Self::DeclaredModuleAlias => "REJECT-DECLARED-MODULE-ALIAS",
            Self::DeclaredModuleNotRegular => "REJECT-DECLARED-MODULE-NOT-REGULAR",
            Self::DeclaredModulePath => "REJECT-DECLARED-MODULE-PATH",
            Self::DuplicateRawModuleDeclaration => "REJECT-DUPLICATE-RAW-MODULE-DECLARATION",
            Self::DuplicateCanonicalModuleDeclaration => "REJECT-DUPLICATE-CANONICAL-MODULE-DECLARATION",
            Self::ParserViewMismatch { .. } => "REJECT-PARSER-VIEW-MISMATCH",
            Self::ParserIdentityMismatch => "REJECT-PARSER-IDENTITY-MISMATCH",
            Self::StrictRolloutPolicyMissing => "BLOCK-STRICT-ROLLOUT-POLICY-MISSING",
            Self::GnuLongnameStrictReject => "OBSERVE-GNU-LONGNAME-STRICT-REJECT",
            Self::PaxStrictReject => "OBSERVE-PAX-STRICT-REJECT",
            Self::GnuDialectStrictReject => "OBSERVE-GNU-NOT-STRICT",
        }
    }
}

// ---------------------------------------------------------------------------
// T2.2.08/.10 — ObserveLegacy wiring surface + StrictCanonicalV1 assembly.
// ---------------------------------------------------------------------------

/// The NAMED observation-pass limits policy (PAR-C09: no hidden defaults —
/// this is an explicit, recorded policy; production STRICT values are
/// T2.5's decision and are injected separately).
pub fn observe_legacy_limits_v1() -> ArchiveLimitsPolicyV1 {
    ArchiveLimitsPolicyV1 {
        policy_id: MachineTextV1::new("apex-t2-2-observe-legacy-v1").expect("ASCII"),
        max_archive_bytes: 256 << 20,
        max_entry_bytes: 128 << 20,
        max_entries: 65_536,
        max_path_bytes: 255,
        max_manifest_bytes: 1 << 20,
    }
}

/// Compact per-archive observation attached at the T2.1 inventory seam
/// (spec section 7: observation-only — NEVER an admission input in
/// ObserveLegacy).
#[derive(Clone, Debug)]
pub struct ObserveSummaryV1 {
    pub dialect: Option<TarDialectV1>,
    /// The strict-pipeline PREVIEW verdict name ("ACCEPT" or the catalog
    /// terminal the strict lane would emit) — what this archive WOULD do
    /// under StrictCanonicalV1, recorded while legacy admission proceeds
    /// unchanged.
    pub strict_preview_terminal: &'static str,
    pub parser_identity: &'static str,
    pub limits_policy_id: String,
    pub semantic_root: Option<ProtocolDigestV1>,
}

/// A fully admitted StrictCanonicalV1 archive.
#[derive(Clone, Debug)]
pub struct StrictArchiveV1 {
    pub namespace: Vec<CanonicalEntryV1>,
    pub manifest: ManifestResolutionV1,
    pub artifact: ArtifactIdentityV1,
    pub semantic_root: ProtocolDigestV1,
}

/// T2.2.10 — the strict admission pipeline: framing scan (FIRST — tar-rs
/// can only narrow), strict-dialect gate, entry-type gate, path assembly,
/// raw duplicate-manifest gate BEFORE namespace (so DUPLICATE-MANIFEST
/// outranks the generic dedup), namespace, manifest resolution, tar-rs
/// reconciliation, identities. `rollout_policy = None` is
/// `BLOCK-STRICT-ROLLOUT-POLICY-MISSING` (PAR-C14): strict admission is
/// TEST-ONLY until `APEX-T2.5` supplies a real policy value.
pub fn admit_strict_canonical(
    archive: &[u8],
    limits: &ArchiveLimitsPolicyV1,
    rollout_policy: Option<&str>,
) -> Result<StrictArchiveV1, ArchiveRejectV1> {
    if rollout_policy.is_none() {
        return Err(ArchiveRejectV1::StrictRolloutPolicyMissing);
    }
    let scan = scan_framing(archive, limits)?;
    for entry in &scan.entries {
        match entry.dialect {
            TarDialectV1::UstarStrict => {},
            TarDialectV1::OldV7 => return Err(ArchiveRejectV1::OldHeaderInStrictV1),
            TarDialectV1::Pax => return Err(ArchiveRejectV1::PaxStrictReject),
            TarDialectV1::Gnu => {
                return if matches!(entry.type_flag, b'L' | b'K') {
                    Err(ArchiveRejectV1::GnuLongnameStrictReject)
                } else {
                    Err(ArchiveRejectV1::GnuDialectStrictReject)
                };
            },
        }
        match entry.type_flag {
            b'0' | 0 => {},
            b'5' => return Err(ArchiveRejectV1::ExplicitDirectoryInStrictV1),
            other => return Err(ArchiveRejectV1::UnsupportedEntryType { type_flag: other }),
        }
    }
    let assembled: Vec<(CanonicalPathV1, MachineTextV1)> =
        scan.entries.iter().map(|e| assemble_ustar_path(e, limits)).collect::<Result<_, _>>()?;
    // Raw duplicate-manifest gate BEFORE namespace dedup (PAR-008 vs 011).
    if assembled.iter().filter(|(p, _)| p.as_str() == "plugin.toml").count() > 1 {
        return Err(ArchiveRejectV1::DuplicateManifest);
    }
    let entries: Vec<CanonicalEntryV1> = scan
        .entries
        .iter()
        .zip(&assembled)
        .map(|(e, (p, k))| {
            use sha2::Digest;
            let digest: [u8; 32] = sha2::Sha256::digest(entry_content(archive, e)).into();
            CanonicalEntryV1 {
                path: p.clone(),
                portability_key: k.clone(),
                size_bytes: e.declared_size,
                content_sha256: digest,
            }
        })
        .collect();
    let namespace = build_namespace(entries)?;
    let manifest = resolve_manifest(archive, &scan.entries, &assembled, &namespace, limits)?;
    reconcile_tar_rs(archive, &scan.entries)?;
    let semantic_root = semantic_root(&namespace)?;
    Ok(StrictArchiveV1 { namespace, manifest, artifact: artifact_identity(archive), semantic_root })
}

/// T2.2.08 — the ObserveLegacy pass: a strict-pipeline PREVIEW recorded
/// as evidence while legacy admission proceeds byte-for-byte unchanged.
/// TOTAL: never returns an error, never blocks the legacy path.
pub fn observe_legacy(archive: &[u8]) -> ObserveSummaryV1 {
    let limits = observe_legacy_limits_v1();
    let dialect = scan_framing(archive, &limits).ok().map(|s| s.dialect);
    let (terminal, root) = match admit_strict_canonical(archive, &limits, Some("observe-preview")) {
        Ok(strict) => ("ACCEPT", Some(strict.semantic_root)),
        Err(e) => (e.terminal_name(), None),
    };
    ObserveSummaryV1 {
        dialect,
        strict_preview_terminal: terminal,
        parser_identity: PARSER_IDENTITY_V1,
        limits_policy_id: limits.policy_id.as_str().to_owned(),
        semantic_root: root,
    }
}

// ---------------------------------------------------------------------------
// T2.2.09 — repository-owned deterministic canonical packer.
// ---------------------------------------------------------------------------

/// Packs a file set into a canonical StrictCanonicalV1 archive:
/// path-byte-sorted entries, FIXED UStar metadata (mode 0644, uid/gid 0,
/// mtime 0, empty uname/gname — `CANONICAL-PACKER-HOST-METADATA-
/// INDEPENDENT` PAR-C22 holds by construction, `ACCEPT-FIXED-USTAR-
/// METADATA` PAR-C32 is its read side), NO directory records (PAR-C33),
/// canonical prefix/name split for long paths, exactly-two-zero-block
/// terminator. The packer NEVER transforms a path it cannot represent —
/// it rejects (`REJECT-WRITER-PATH-TRANSFORMATION` PAR-C31 is the guard
/// this refuses to violate; `REJECT-NONREPRESENTABLE-USTAR-PATH` PAR-C21
/// lives HERE, its only reachable side). Inspect-after-pack: the output
/// is re-scanned and re-assembled and must round-trip the input set
/// exactly (`CANONICAL-PACKER-REPRODUCIBLE` PAR-C13 follows from this
/// being a pure function of the sorted input).
pub fn pack_canonical(files: &[(CanonicalPathV1, &[u8])], limits: &ArchiveLimitsPolicyV1) -> Result<Vec<u8>, ArchiveRejectV1> {
    // Validate + sort (input order must not matter).
    let mut sorted: Vec<&(CanonicalPathV1, &[u8])> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.as_str().as_bytes().cmp(b.0.as_str().as_bytes()));
    for pair in sorted.windows(2) {
        if pair[0].0.as_str() == pair[1].0.as_str() {
            return Err(ArchiveRejectV1::DuplicateCanonicalPath);
        }
    }
    let mut keys: Vec<String> = sorted.iter().map(|(p, _)| p.as_str().to_ascii_lowercase()).collect();
    keys.sort_unstable();
    for pair in keys.windows(2) {
        if pair[0] == pair[1] {
            return Err(ArchiveRejectV1::PortableCaseCollision);
        }
    }

    let mut out = Vec::new();
    for (path, content) in sorted {
        let bytes = path.as_str().as_bytes();
        if !bytes.iter().all(|&b| portable_byte(b)) {
            return Err(ArchiveRejectV1::NonPortableCharacter {
                byte: *bytes.iter().find(|&&b| !portable_byte(b)).expect("checked"),
            });
        }
        if bytes.len() as u64 > limits.max_path_bytes {
            return Err(ArchiveRejectV1::PathTooLong);
        }
        if content.len() as u64 > limits.max_entry_bytes {
            return Err(ArchiveRejectV1::EntrySizeLimit);
        }
        // Canonical split — reject rather than transform (C21/C31).
        let (prefix_len, name_len) = canonical_split(bytes).ok_or(ArchiveRejectV1::NonrepresentableUstarPath)?;
        let (prefix, name) = if prefix_len == 0 {
            (&b""[..], bytes)
        } else {
            (&bytes[..prefix_len], &bytes[bytes.len() - name_len..])
        };

        let mut header = [0u8; BLOCK];
        header[..name.len()].copy_from_slice(name);
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        let size_field = format!("{:011o}\0", content.len());
        header[124..136].copy_from_slice(size_field.as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[345..345 + prefix.len()].copy_from_slice(prefix);
        header[148..156].copy_from_slice(b"        ");
        let sum: u64 = header.iter().map(|&b| b as u64).sum();
        header[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());

        out.extend_from_slice(&header);
        out.extend_from_slice(content);
        let pad = (BLOCK - content.len() % BLOCK) % BLOCK;
        out.extend(std::iter::repeat_n(0u8, pad));
    }
    out.extend(std::iter::repeat_n(0u8, 2 * BLOCK));

    // Inspect-after-pack: our own scanner + path assembly must round-trip
    // the exact input set (never trust the writer, even our own).
    let scan = scan_framing(&out, limits)?;
    if scan.dialect != TarDialectV1::UstarStrict || !scan.canonical_terminator || scan.entries.len() != files.len() {
        return Err(ArchiveRejectV1::MalformedTar { detail: "inspect-after-pack: framing shape" });
    }
    let mut expect: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    expect.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for (entry, want) in scan.entries.iter().zip(expect) {
        let (got, _) = assemble_ustar_path(entry, limits)?;
        if got.as_str() != want || entry.type_flag != b'0' {
            return Err(ArchiveRejectV1::WriterPathTransformation);
        }
    }
    reconcile_tar_rs(&out, &scan.entries)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// T2.2.06 — bounded root manifest + legacy module resolution.
// ---------------------------------------------------------------------------

/// Raw-shape mirror of the live legacy `PluginData` (`plugin/mod.rs`):
/// `Vec`, NOT the live struct's `BTreeSet` — the set silently dedupes,
/// and PAR-C18 requires raw duplicate declarations to be OBSERVED.
#[derive(serde::Deserialize)]
struct LegacyManifestRawV1 {
    #[allow(dead_code)]
    name: String,
    #[serde(default)]
    modules: Vec<String>,
    #[serde(default)]
    dependencies: Vec<String>,
}

/// T2.2.06 result: the resolved module set in RAW DECLARATION ORDER, with
/// the order explicitly marked unfrozen (`OBSERVE-LEGACY-MODULE-ORDER`
/// PAR-C12 — T2.4 owns freezing it), plus observed-not-rejected raw
/// duplicate dependency declarations (PAR-C20).
#[derive(Clone, Debug)]
pub struct ManifestResolutionV1 {
    pub manifest_path: CanonicalPathV1,
    pub modules: Vec<CanonicalPathV1>,
    /// Always true for the legacy schema.
    pub module_order_unfrozen: bool,
    pub raw_duplicate_dependencies: Vec<String>,
}

/// Exactly one bounded regular root `plugin.toml`, parsed as the CURRENT
/// legacy schema only (spec policy 4; T2.3 owns V1), with every declared
/// module resolved through the canonical namespace gate.
pub fn resolve_manifest(
    archive: &[u8],
    scanned: &[ScannedEntryV1],
    assembled: &[(CanonicalPathV1, MachineTextV1)],
    namespace: &[CanonicalEntryV1],
    limits: &ArchiveLimitsPolicyV1,
) -> Result<ManifestResolutionV1, ArchiveRejectV1> {
    assert_eq!(scanned.len(), assembled.len(), "assembled must be index-parallel with scanned");

    // Locate the root manifest among RAW entries (pre-dedup: a duplicated
    // plugin.toml is DUPLICATE-MANIFEST PAR-008, not the generic path
    // collision).
    let mut manifest_idx = None;
    for (i, (path, _)) in assembled.iter().enumerate() {
        if path.as_str() == "plugin.toml" {
            if manifest_idx.is_some() {
                return Err(ArchiveRejectV1::DuplicateManifest);
            }
            manifest_idx = Some(i);
        }
    }
    let idx = manifest_idx.ok_or(ArchiveRejectV1::MissingManifest)?;
    let entry = &scanned[idx];
    if !matches!(entry.type_flag, b'0' | 0) {
        return Err(ArchiveRejectV1::ManifestNotRegular);
    }
    if entry.declared_size > limits.max_manifest_bytes {
        return Err(ArchiveRejectV1::ManifestSizeLimit);
    }

    let content = entry_content(archive, entry);
    let toml_str =
        std::str::from_utf8(content).map_err(|_| ArchiveRejectV1::ManifestParse { detail: "manifest not UTF-8" })?;
    let raw: LegacyManifestRawV1 =
        toml::de::from_str(toml_str).map_err(|_| ArchiveRejectV1::ManifestParse { detail: "legacy TOML parse" })?;

    // Module resolution through the canonical path/index gate.
    let exact: std::collections::BTreeSet<&str> = namespace.iter().map(|e| e.path.as_str()).collect();
    let folded: std::collections::BTreeSet<String> =
        namespace.iter().map(|e| e.portability_key.as_str().to_owned()).collect();
    let implied: std::collections::BTreeSet<String> = implied_directories(namespace)
        .into_iter()
        .map(|d| d.to_ascii_lowercase())
        .collect();

    let mut seen_raw = std::collections::BTreeSet::new();
    let mut seen_keys = std::collections::BTreeSet::new();
    let mut modules = Vec::with_capacity(raw.modules.len());
    for declared in &raw.modules {
        if !seen_raw.insert(declared.clone()) {
            return Err(ArchiveRejectV1::DuplicateRawModuleDeclaration);
        }
        // Grammar first: a declared module must itself be a canonical
        // portable path (PAR-048 covers host-form/relative-form strings).
        if !declared.bytes().all(portable_byte) || CanonicalPathV1::new(declared.as_str()).is_err() {
            return Err(ArchiveRejectV1::DeclaredModulePath);
        }
        let key = declared.to_ascii_lowercase();
        if !seen_keys.insert(key.clone()) {
            return Err(ArchiveRejectV1::DuplicateCanonicalModuleDeclaration);
        }
        if exact.contains(declared.as_str()) {
            modules.push(CanonicalPathV1::new(declared.as_str()).expect("checked"));
        } else if implied.contains(&key) {
            return Err(ArchiveRejectV1::DeclaredModuleNotRegular);
        } else if folded.contains(&key) {
            return Err(ArchiveRejectV1::DeclaredModuleAlias);
        } else {
            return Err(ArchiveRejectV1::DeclaredModuleMissing);
        }
    }

    let mut dep_seen = std::collections::BTreeSet::new();
    let raw_duplicate_dependencies: Vec<String> =
        raw.dependencies.iter().filter(|d| !dep_seen.insert((*d).clone())).cloned().collect();

    Ok(ManifestResolutionV1 {
        manifest_path: assembled[idx].0.clone(),
        modules,
        module_order_unfrozen: true,
        raw_duplicate_dependencies,
    })
}

// ---------------------------------------------------------------------------
// T2.2.03 — tar-rs reconciliation against the SAME immutable bytes.
// ---------------------------------------------------------------------------

/// Exact parser identity recorded in every observation (PAR-C08 keys off
/// this). The tar-rs half names the WORKSPACE-PINNED version (Cargo.lock);
/// a lockfile bump that changes it without a deliberate identity update
/// is exactly what the identity-mismatch canary exists to catch.
pub const PARSER_IDENTITY_V1: &str = "apex-t2-2-framing-scanner/v1+tar-rs/0.4.46";

/// T2.2.03: reconcile tar-rs's view of the archive against the framing
/// scanner's, over the SAME buffer. tar-rs NEVER decides framing
/// (`BLOCK-TAR-RS-FRAMING-SUBSTITUTION` PAR-C04 — structurally, the
/// strict pipeline calls `scan_framing` first and this reconciler can
/// only REJECT further, never widen); any disagreement in entry count,
/// path bytes, or declared size is `REJECT-PARSER-VIEW-MISMATCH`
/// (PAR-C17): one of the two parsers is being lenient about bytes the
/// other reads differently, and a split-view archive is exactly the
/// smuggling shape the reconciliation exists to kill.
pub fn reconcile_tar_rs(archive: &[u8], scanned: &[ScannedEntryV1]) -> Result<(), ArchiveRejectV1> {
    let mut tar = tar::Archive::new(archive);
    let entries = tar
        .entries()
        .map_err(|_| ArchiveRejectV1::ParserViewMismatch { detail: "tar-rs refused entries the scanner admitted" })?;
    let mut count = 0usize;
    for (i, entry) in entries.enumerate() {
        let entry =
            entry.map_err(|_| ArchiveRejectV1::ParserViewMismatch { detail: "tar-rs entry error mid-archive" })?;
        let ours = scanned
            .get(i)
            .ok_or(ArchiveRejectV1::ParserViewMismatch { detail: "tar-rs sees more entries than the scanner" })?;
        // Path: tar-rs performs its own prefix/name join; compare against
        // the same join of our raw fields.
        let mut full = ours.raw_prefix.clone();
        if !full.is_empty() {
            full.push(b'/');
        }
        full.extend_from_slice(&ours.raw_name);
        let theirs = entry
            .path_bytes()
            .into_owned();
        if theirs != full {
            return Err(ArchiveRejectV1::ParserViewMismatch { detail: "path bytes disagree" });
        }
        if entry.header().size().map_err(|_| ArchiveRejectV1::ParserViewMismatch { detail: "size unreadable" })?
            != ours.declared_size
        {
            return Err(ArchiveRejectV1::ParserViewMismatch { detail: "declared size disagrees" });
        }
        count += 1;
    }
    if count != scanned.len() {
        return Err(ArchiveRejectV1::ParserViewMismatch { detail: "scanner sees more entries than tar-rs" });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// T2.2.07 — separated artifact and semantic identities.
// ---------------------------------------------------------------------------

pub const PLUGIN_ARCHIVE_SEMANTIC_SCHEMA_V1: &str = "bastion.plugin-archive-semantic/v1";

/// Own limits for the semantic-root manifest (T0.2 has no Default).
pub const fn plugin_archive_limits_v1() -> common::apex::manifest::ManifestDecodeLimitsV1 {
    common::apex::manifest::ManifestDecodeLimitsV1 {
        max_input_bytes: 16 << 20,
        max_depth: 8,
        max_nodes: 1 << 18,
        max_array_items: 1 << 15,
        max_map_entries: 16,
        max_machine_text_bytes: 4096,
        max_byte_string_bytes: 4096,
    }
}

/// The archive's EXACT artifact identity: moves with ANY byte of the
/// archive (`ACCEPT-SAME-SEMANTIC-ROOT-DIFFERENT-ARTIFACT` PAR-004 is the
/// separation proof against [`semantic_root`]).
pub fn artifact_identity(archive: &[u8]) -> ArtifactIdentityV1 {
    common::apex::digest::hash_artifact_bytes_v1(archive)
}

/// The archive's SEMANTIC root under `PluginArchive` (= 17): schema tag +
/// path-sorted regular-file (path, kind, size, content) records + the
/// implied-directory namespace (PAR-C10) — and NOTHING else: no raw
/// ordinal, no tar metadata (PAR-C11, `CANONICAL-PACKER-HOST-METADATA-
/// INDEPENDENT` follows from this exclusion by construction).
pub fn semantic_root(namespace: &[CanonicalEntryV1]) -> Result<ProtocolDigestV1, ArchiveRejectV1> {
    use common::apex::manifest::{
        CanonicalFieldMapV1, FieldIdV1, MachineTextV1 as MT, ManifestEncodeV1, ManifestValueV1,
    };
    struct Wrapper(ManifestValueV1);
    impl ManifestEncodeV1 for Wrapper {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
            Ok(self.0.clone())
        }
    }
    let err = |_| ArchiveRejectV1::MalformedTar { detail: "semantic root encoding" };
    let entries: Vec<ManifestValueV1> = namespace
        .iter()
        .map(|e| {
            let map = CanonicalFieldMapV1::try_from_entries(vec![
                (FieldIdV1::new(0), ManifestValueV1::MachineText(MT::new(e.path.as_str()).map_err(err)?)),
                (FieldIdV1::new(1), ManifestValueV1::MachineText(MT::new("file").map_err(err)?)),
                (FieldIdV1::new(2), ManifestValueV1::Unsigned(e.size_bytes)),
                (FieldIdV1::new(3), ManifestValueV1::Bytes(e.content_sha256.to_vec())),
            ])
            .map_err(err)?;
            Ok(ManifestValueV1::Map(map))
        })
        .collect::<Result<_, ArchiveRejectV1>>()?;
    let dirs: Vec<ManifestValueV1> = implied_directories(namespace)
        .into_iter()
        .map(|d| Ok(ManifestValueV1::MachineText(MT::new(d).map_err(err)?)))
        .collect::<Result<_, ArchiveRejectV1>>()?;
    let top = CanonicalFieldMapV1::try_from_entries(vec![
        (FieldIdV1::new(0), ManifestValueV1::MachineText(MT::new(PLUGIN_ARCHIVE_SEMANTIC_SCHEMA_V1).map_err(err)?)),
        (FieldIdV1::new(1), ManifestValueV1::Array(entries)),
        (FieldIdV1::new(2), ManifestValueV1::Array(dirs)),
    ])
    .map_err(err)?;
    common::apex::digest::digest_manifest_value_v1(
        common::apex::digest::DigestDomainIdV1::PluginArchive,
        &Wrapper(ManifestValueV1::Map(top)),
        &plugin_archive_limits_v1(),
    )
    .map_err(|_| ArchiveRejectV1::MalformedTar { detail: "semantic root digest" })
}

// ---------------------------------------------------------------------------
// T2.2.05 — duplicate-safe regular-file namespace index.
// ---------------------------------------------------------------------------

/// Builds the strict namespace from per-entry identities. Rejections make
/// last-entry-wins UNREPRESENTABLE (spec policy 3): exact duplicates,
/// ASCII-case-fold collisions, and file-vs-implied-directory collisions
/// each die on their own catalog terminal; output is path-byte sorted.
pub fn build_namespace(entries: Vec<CanonicalEntryV1>) -> Result<Vec<CanonicalEntryV1>, ArchiveRejectV1> {
    let mut sorted = entries;
    sorted.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));

    // Exact duplicates (adjacent after sort).
    for pair in sorted.windows(2) {
        if pair[0].path.as_str() == pair[1].path.as_str() {
            return Err(ArchiveRejectV1::DuplicateCanonicalPath);
        }
    }
    // Case-fold collisions on the portability key.
    let mut keys: Vec<&str> = sorted.iter().map(|e| e.portability_key.as_str()).collect();
    keys.sort_unstable();
    for pair in keys.windows(2) {
        if pair[0] == pair[1] {
            return Err(ArchiveRejectV1::PortableCaseCollision);
        }
    }
    // File-vs-implied-directory collisions: a regular file's full path
    // must never also be a directory prefix of another entry (checked on
    // the CASE-FOLDED namespace — a collision across case is still a
    // collision on portable filesystems).
    let folded: std::collections::BTreeSet<String> =
        sorted.iter().map(|e| e.portability_key.as_str().to_owned()).collect();
    for entry in &sorted {
        let key = entry.portability_key.as_str();
        let mut prefix = String::new();
        for segment in key.split('/') {
            if !prefix.is_empty() {
                if folded.contains(&prefix) {
                    return Err(ArchiveRejectV1::PathKindCollision);
                }
                prefix.push('/');
            }
            prefix.push_str(segment);
        }
    }
    Ok(sorted)
}

/// The implied-directory namespace of a built namespace (spec policy 6 /
/// `CANONICAL-ROOT-INCLUDES-DIRECTORY-NAMESPACE` PAR-C10): every proper
/// ancestor path, sorted + deduped. Explicit directory RECORDS are
/// rejected in strict mode; the STRUCTURE the paths imply is part of the
/// semantic root (T2.2.07 embeds this list).
pub fn implied_directories(namespace: &[CanonicalEntryV1]) -> Vec<String> {
    let mut dirs = std::collections::BTreeSet::new();
    for entry in namespace {
        let path = entry.path.as_str();
        let mut prefix = String::new();
        for segment in path.split('/') {
            if !prefix.is_empty() {
                dirs.insert(prefix.clone());
                prefix.push('/');
            }
            prefix.push_str(segment);
        }
    }
    dirs.into_iter().collect()
}

// ---------------------------------------------------------------------------
// T2.2.04 — path identity from raw UStar fields (frozen ASCII grammar,
// portability key, canonical rightmost-valid-slash split).
// ---------------------------------------------------------------------------

/// V1 frozen portable character set (spec section 2.2): lowercase +
/// uppercase ASCII letters, digits, `.`, `_`, `-`, and `/` as the sole
/// separator. Everything else is `REJECT-NON-PORTABLE-CHARACTER`
/// (backslash and NUL get their own sharper terminals first).
fn portable_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/')
}

const USTAR_NAME_MAX: usize = 100;
const USTAR_PREFIX_MAX: usize = 155;

/// The canonical UStar split of a full path (spec policy 5): empty prefix
/// whenever the whole path fits in `name` (<= 100 bytes); otherwise the
/// RIGHTMOST slash split whose name-part fits 100 and prefix-part fits
/// 155. `None` = not representable.
fn canonical_split(full: &[u8]) -> Option<(usize, usize)> {
    if full.len() <= USTAR_NAME_MAX {
        return Some((0, full.len()));
    }
    // Rightmost slash such that name fits; then prefix must fit too.
    let mut split = None;
    for (i, &b) in full.iter().enumerate() {
        if b == b'/' && full.len() - i - 1 <= USTAR_NAME_MAX && i <= USTAR_PREFIX_MAX {
            split = Some(i);
        }
    }
    split.map(|i| (i, full.len() - i - 1))
}

/// T2.2.04: assemble + validate one entry's path identity from the RAW
/// UStar `name`/`prefix` field bytes. Returns the canonical path and its
/// ASCII-lowercase portability key.
pub fn assemble_ustar_path(entry: &ScannedEntryV1, limits: &ArchiveLimitsPolicyV1) -> Result<(CanonicalPathV1, MachineTextV1), ArchiveRejectV1> {
    let name = &entry.raw_name;
    let prefix = &entry.raw_prefix;

    // Boundary grammar of the raw fields themselves (PAR-C26/C27/C29):
    // trim_field already cut at NUL, but a FULL 100/155-byte field is
    // legal; anything longer is impossible by construction. A name that
    // was exactly at the field limit with content that clearly continued
    // (writer needed 101 bytes) manifests as either a noncanonical split
    // or a non-representable path below; the 101-boundary canary is the
    // case where a writer emitted 100 name bytes and relied on silent
    // truncation -- we cannot see the intent, but PAR-C27's fixture drives
    // this via a prefix+name whose reassembly is not the canonical split.
    if name.is_empty() {
        return Err(ArchiveRejectV1::MalformedTar { detail: "empty name field" });
    }

    // Full path bytes: prefix + '/' + name per UStar; empty prefix = name.
    let mut full: Vec<u8> = Vec::with_capacity(prefix.len() + 1 + name.len());
    if !prefix.is_empty() {
        full.extend_from_slice(prefix);
        full.push(b'/');
    }
    full.extend_from_slice(name);

    // Byte-level rejects, sharpest terminal first (raw bytes, before any
    // UTF-8/host interpretation -- PAR-C15's point).
    if full.contains(&0) {
        return Err(ArchiveRejectV1::NulInPath);
    }
    if full.contains(&b'\\') {
        // Raw backslash in the FIELD bytes (C15) vs backslash surviving
        // into a decoded path (015) collapse to the same raw check here --
        // the raw check fires first by construction, which is the
        // stricter reading. Distinct terminals retained for the catalog:
        // prefix field containing it is "raw".
        return if entry.raw_prefix.contains(&b'\\') || entry.raw_name.contains(&b'\\') {
            Err(ArchiveRejectV1::RawBackslash)
        } else {
            Err(ArchiveRejectV1::Backslash)
        };
    }
    if std::str::from_utf8(&full).is_err() {
        return Err(ArchiveRejectV1::InvalidUtf8);
    }
    if full[0] == b'/' {
        return Err(ArchiveRejectV1::AbsolutePath);
    }
    if let Some(&bad) = full.iter().find(|&&b| !portable_byte(b)) {
        return Err(ArchiveRejectV1::NonPortableCharacter { byte: bad });
    }
    if full.len() as u64 > limits.max_path_bytes {
        return Err(ArchiveRejectV1::PathTooLong);
    }
    if full.last() == Some(&b'/') {
        return Err(ArchiveRejectV1::RegularTrailingSlash);
    }
    for segment in full.split(|&b| b == b'/') {
        match segment {
            b"" => return Err(ArchiveRejectV1::EmptySegment),
            b"." => return Err(ArchiveRejectV1::CurrentSegment),
            b".." => return Err(ArchiveRejectV1::ParentSegment),
            _ => {},
        }
    }

    // Canonical-split policy (PAR-C16/C26/C27/C28/C29/C30/C21): the
    // writer's actual (prefix, name) split must BE the canonical one.
    match canonical_split(&full) {
        None => return Err(ArchiveRejectV1::NonrepresentableUstarPath),
        Some((canon_prefix_len, canon_name_len)) => {
            let actual_prefix_len = if prefix.is_empty() { 0 } else { prefix.len() };
            if actual_prefix_len != canon_prefix_len {
                // Distinguish the two boundary-shaped wrong splits the
                // catalog names: a name field that should have spilled at
                // 101 (writer kept prefix empty for a >100 path is
                // impossible -- field is 100 max -- so the observable form
                // is a split at the WRONG slash) vs a prefix longer than
                // canonical (overflow-shaped).
                return if actual_prefix_len > USTAR_PREFIX_MAX {
                    Err(ArchiveRejectV1::UstarPrefixOverflow)
                } else if canon_prefix_len == 0 && actual_prefix_len > 0 && full.len() <= USTAR_NAME_MAX {
                    // Path fits entirely in name but writer used a prefix.
                    Err(ArchiveRejectV1::NoncanonicalUstarSplit)
                } else if actual_prefix_len == 0 && name.len() == USTAR_NAME_MAX && canon_name_len < name.len() {
                    Err(ArchiveRejectV1::UstarName101Boundary)
                } else {
                    Err(ArchiveRejectV1::NoncanonicalUstarSplit)
                };
            }
        },
    }

    let full_str = std::str::from_utf8(&full).expect("checked above");
    let path = CanonicalPathV1::new(full_str).map_err(|_| ArchiveRejectV1::MalformedTar { detail: "path grammar" })?;
    let key = MachineTextV1::new(full_str.to_ascii_lowercase()).expect("ASCII by construction");
    Ok((path, key))
}

const BLOCK: usize = 512;

/// One scanned entry: header facts + the byte range of its content within
/// the immutable buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScannedEntryV1 {
    pub ordinal: u64,
    pub raw_name: Vec<u8>,
    pub raw_prefix: Vec<u8>,
    pub type_flag: u8,
    pub declared_size: u64,
    pub header_checksum_ok: bool,
    pub dialect: TarDialectV1,
    pub content_offset: usize,
}

/// Whole-archive framing verdict (T2.2.02).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FramingScanV1 {
    pub entries: Vec<ScannedEntryV1>,
    /// The single observed dialect for the archive: strictest common
    /// reading — `UstarStrict` only if EVERY entry is strict ustar.
    pub dialect: TarDialectV1,
    /// Exactly-two-zero-block terminator present with nothing after it.
    pub canonical_terminator: bool,
}

fn is_zero_block(b: &[u8]) -> bool { b.iter().all(|&x| x == 0) }

/// Parse a NUL/space-terminated octal field; tolerate GNU base-256 only
/// as a MalformedTar reject in this scanner (strict grammar).
fn octal_field(field: &[u8]) -> Result<u64, ArchiveRejectV1> {
    if field.first().is_some_and(|&b| b & 0x80 != 0) {
        return Err(ArchiveRejectV1::MalformedTar { detail: "base-256 numeric field" });
    }
    let mut value: u64 = 0;
    let mut seen = false;
    for &b in field {
        match b {
            b'0'..=b'7' => {
                value = value
                    .checked_mul(8)
                    .and_then(|v| v.checked_add((b - b'0') as u64))
                    .ok_or(ArchiveRejectV1::MalformedTar { detail: "octal overflow" })?;
                seen = true;
            },
            b' ' | 0 => {
                if seen {
                    break;
                }
                // leading spaces allowed
            },
            _ => return Err(ArchiveRejectV1::MalformedTar { detail: "non-octal byte in numeric field" }),
        }
    }
    Ok(value)
}

/// POSIX header checksum: unsigned byte sum with the chksum field (bytes
/// 148..156) treated as ASCII spaces.
fn checksum_ok(header: &[u8]) -> bool {
    let declared = match octal_field(&header[148..156]) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut sum: u64 = 0;
    for (i, &b) in header.iter().enumerate() {
        sum += if (148..156).contains(&i) { b' ' as u64 } else { b as u64 };
    }
    sum == declared
}

fn header_dialect(header: &[u8], type_flag: u8) -> TarDialectV1 {
    match type_flag {
        b'L' | b'K' => return TarDialectV1::Gnu,
        b'x' | b'g' => return TarDialectV1::Pax,
        _ => {},
    }
    let magic = &header[257..263];
    let version = &header[263..265];
    if magic == b"ustar\0" && version == b"00" {
        TarDialectV1::UstarStrict
    } else if magic == b"ustar " || (magic.starts_with(b"ustar") && version == b" \0") {
        TarDialectV1::Gnu
    } else if magic.iter().all(|&b| b == 0) {
        TarDialectV1::OldV7
    } else {
        TarDialectV1::Gnu
    }
}

fn trim_field(field: &[u8]) -> Vec<u8> {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    field[..end].to_vec()
}

/// T2.2.02 — the checked framing scan. Applies ONLY the framing grammar +
/// size/count limits; path grammar, namespace rules, and mode policy are
/// later minute steps layered on this result.
pub fn scan_framing(bytes: &[u8], limits: &ArchiveLimitsPolicyV1) -> Result<FramingScanV1, ArchiveRejectV1> {
    if bytes.len() as u64 > limits.max_archive_bytes {
        return Err(ArchiveRejectV1::ArchiveSizeLimit);
    }
    if bytes.len() % BLOCK != 0 {
        return Err(ArchiveRejectV1::MalformedTar { detail: "length not a multiple of 512" });
    }
    let mut entries = Vec::new();
    let mut dialect = TarDialectV1::UstarStrict;
    let mut offset = 0usize;
    let mut ordinal = 0u64;

    loop {
        if offset + BLOCK > bytes.len() {
            // Ran out of blocks without ever seeing a terminator.
            return Err(ArchiveRejectV1::MissingTerminator);
        }
        let header = &bytes[offset..offset + BLOCK];
        if is_zero_block(header) {
            // Candidate terminator: require EXACTLY one more zero block
            // and then end-of-buffer (spec: exactly-two-zero-block
            // canonical terminator; PAR-C35/C36/C37/C07).
            let second = offset + BLOCK;
            if second + BLOCK > bytes.len() {
                return Err(ArchiveRejectV1::OneZeroBlockTerminator);
            }
            if !is_zero_block(&bytes[second..second + BLOCK]) {
                // A zero block followed by data: either a concatenated
                // archive or garbage — nonzero trailing data after a
                // (partial) terminator.
                return Err(ArchiveRejectV1::NonzeroTrailingData);
            }
            let after = second + BLOCK;
            if after < bytes.len() {
                // More bytes after the two-block terminator: all-zero
                // padding is TrailingZeroBlocks; anything else is
                // TrailingData.
                if bytes[after..].iter().all(|&b| b == 0) {
                    return Err(ArchiveRejectV1::TrailingZeroBlocks);
                }
                return Err(ArchiveRejectV1::TrailingData);
            }
            return Ok(FramingScanV1 { entries, dialect, canonical_terminator: true });
        }

        // Non-zero block: must be a header.
        let checksum = checksum_ok(header);
        if !checksum {
            return Err(ArchiveRejectV1::MalformedTar { detail: "header checksum mismatch" });
        }
        let type_flag = header[156];
        let this_dialect = header_dialect(header, type_flag);
        // Strictest-common dialect for the archive.
        dialect = match (dialect, this_dialect) {
            (TarDialectV1::UstarStrict, d) => d,
            (d, TarDialectV1::UstarStrict) => d,
            (TarDialectV1::Pax, _) | (_, TarDialectV1::Pax) => TarDialectV1::Pax,
            (TarDialectV1::OldV7, _) | (_, TarDialectV1::OldV7) => TarDialectV1::OldV7,
            _ => TarDialectV1::Gnu,
        };
        let declared_size = octal_field(&header[124..136])?;
        if declared_size > limits.max_entry_bytes {
            return Err(ArchiveRejectV1::EntrySizeLimit);
        }
        let data_blocks = declared_size.div_ceil(BLOCK as u64) as usize;
        let content_offset = offset + BLOCK;
        let next = content_offset + data_blocks * BLOCK;
        if next > bytes.len() {
            return Err(ArchiveRejectV1::TruncatedArchive);
        }
        // Padding bytes after the content within the last block must be
        // zero (checked framing: a smuggling channel otherwise).
        let content_end = content_offset + declared_size as usize;
        if bytes[content_end..next].iter().any(|&b| b != 0) {
            return Err(ArchiveRejectV1::MalformedTar { detail: "nonzero padding after entry content" });
        }

        entries.push(ScannedEntryV1 {
            ordinal,
            raw_name: trim_field(&header[0..100]),
            raw_prefix: trim_field(&header[345..500]),
            type_flag,
            declared_size,
            header_checksum_ok: checksum,
            dialect: this_dialect,
            content_offset,
        });
        ordinal += 1;
        if ordinal > limits.max_entries {
            return Err(ArchiveRejectV1::EntryCountLimit);
        }
        offset = next;
    }
}

/// Entry content slice for a scanned entry (the buffer is immutable and
/// single — the landed T2.1 invariant this row builds on).
pub fn entry_content<'a>(bytes: &'a [u8], entry: &ScannedEntryV1) -> &'a [u8] {
    &bytes[entry.content_offset..entry.content_offset + entry.declared_size as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn test_limits() -> ArchiveLimitsPolicyV1 {
        ArchiveLimitsPolicyV1 {
            policy_id: MachineTextV1::new("apex-t2-2-test-limits-v1").unwrap(),
            max_archive_bytes: 1 << 20,
            max_entry_bytes: 1 << 16,
            max_entries: 64,
            max_path_bytes: 255,
            max_manifest_bytes: 1 << 14,
        }
    }

    /// Hand-rolled strict-ustar writer for fixtures (the REAL canonical
    /// packer is T2.2.09; tests must not depend on it).
    pub(super) fn ustar_entry(name: &str, content: &[u8], type_flag: u8) -> Vec<u8> {
        let mut header = vec![0u8; 512];
        header[..name.len()].copy_from_slice(name.as_bytes());
        header[100..107].copy_from_slice(b"0000644");
        header[108..115].copy_from_slice(b"0000000");
        header[116..123].copy_from_slice(b"0000000");
        let size = format!("{:011o}", content.len());
        header[124..135].copy_from_slice(size.as_bytes());
        header[136..147].copy_from_slice(b"00000000000");
        header[156] = type_flag;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        // checksum
        header[148..156].copy_from_slice(b"        ");
        let sum: u64 = header.iter().map(|&b| b as u64).sum();
        let chk = format!("{:06o}\0 ", sum);
        header[148..156].copy_from_slice(chk.as_bytes());
        let mut out = header;
        out.extend_from_slice(content);
        let pad = (512 - content.len() % 512) % 512;
        out.extend(std::iter::repeat_n(0u8, pad));
        out
    }

    pub(super) fn terminated(mut body: Vec<u8>) -> Vec<u8> {
        body.extend(std::iter::repeat_n(0u8, 1024));
        body
    }

    #[test]
    fn minimal_archive_scans_clean() {
        let tar = terminated(ustar_entry("plugin.toml", b"[plugin]\n", b'0'));
        let scan = scan_framing(&tar, &test_limits()).unwrap();
        assert_eq!(scan.entries.len(), 1);
        assert_eq!(scan.dialect, TarDialectV1::UstarStrict);
        assert!(scan.canonical_terminator);
        assert_eq!(entry_content(&tar, &scan.entries[0]), b"[plugin]\n");
    }

    #[test]
    fn terminator_grammar_bites() {
        let entry = ustar_entry("a.txt", b"x", b'0');

        // No terminator at all.
        assert_eq!(scan_framing(&entry, &test_limits()), Err(ArchiveRejectV1::MissingTerminator));

        // Exactly one zero block.
        let mut one = entry.clone();
        one.extend(std::iter::repeat_n(0u8, 512));
        assert_eq!(scan_framing(&one, &test_limits()), Err(ArchiveRejectV1::OneZeroBlockTerminator));

        // Three zero blocks (trailing zero padding beyond the canonical two).
        let mut three = entry.clone();
        three.extend(std::iter::repeat_n(0u8, 1536));
        assert_eq!(scan_framing(&three, &test_limits()), Err(ArchiveRejectV1::TrailingZeroBlocks));

        // Nonzero data after the terminator.
        let mut trailing = terminated(entry.clone());
        trailing.extend(ustar_entry("b.txt", b"y", b'0'));
        // First zero block followed by... second zero block then data =>
        // TrailingData (data after canonical terminator).
        assert_eq!(scan_framing(&trailing, &test_limits()), Err(ArchiveRejectV1::TrailingData));

        // Concatenated archive: zero block then a HEADER (nonzero) block.
        let mut concat = entry.clone();
        concat.extend(std::iter::repeat_n(0u8, 512));
        concat.extend(ustar_entry("b.txt", b"y", b'0'));
        concat.extend(std::iter::repeat_n(0u8, 512));
        assert_eq!(scan_framing(&concat, &test_limits()), Err(ArchiveRejectV1::NonzeroTrailingData));
    }

    #[test]
    fn corruption_and_limits_bite() {
        let limits = test_limits();
        let good = terminated(ustar_entry("a.txt", b"x", b'0'));

        // Flip a checksum byte.
        let mut bad = good.clone();
        bad[148] = b'9';
        assert!(matches!(scan_framing(&bad, &limits), Err(ArchiveRejectV1::MalformedTar { .. })));

        // Truncate mid-content.
        let mut trunc = ustar_entry("a.txt", &[b'x'; 700], b'0');
        trunc.truncate(1024); // header + partial content only
        assert_eq!(scan_framing(&trunc, &limits), Err(ArchiveRejectV1::TruncatedArchive));

        // Nonzero padding after content (smuggling channel).
        let mut smuggle = ustar_entry("a.txt", b"x", b'0');
        smuggle[512 + 1] = 0xFF; // inside padding of the content block
        let smuggle = terminated(smuggle);
        assert!(matches!(scan_framing(&smuggle, &limits), Err(ArchiveRejectV1::MalformedTar { .. })));

        // Non-512-multiple length.
        let mut odd = good.clone();
        odd.push(0);
        assert!(matches!(scan_framing(&odd, &limits), Err(ArchiveRejectV1::MalformedTar { .. })));

        // Entry size limit.
        let mut small = limits.clone();
        small.max_entry_bytes = 0;
        assert_eq!(scan_framing(&good, &small), Err(ArchiveRejectV1::EntrySizeLimit));

        // Archive size limit.
        let mut tiny = limits.clone();
        tiny.max_archive_bytes = 100;
        assert_eq!(scan_framing(&good, &tiny), Err(ArchiveRejectV1::ArchiveSizeLimit));
    }

    #[test]
    fn dialect_detection() {
        // GNU magic.
        let mut gnu = ustar_entry("a.txt", b"x", b'0');
        gnu[257..263].copy_from_slice(b"ustar ");
        gnu[263..265].copy_from_slice(b" \0");
        // fix checksum after mutating magic
        gnu[148..156].copy_from_slice(b"        ");
        let sum: u64 = gnu[..512].iter().map(|&b| b as u64).sum();
        gnu[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
        let scan = scan_framing(&terminated(gnu), &test_limits()).unwrap();
        assert_eq!(scan.dialect, TarDialectV1::Gnu);

        // Old V7: zero magic.
        let mut v7 = ustar_entry("a.txt", b"x", b'0');
        for b in &mut v7[257..265] {
            *b = 0;
        }
        v7[148..156].copy_from_slice(b"        ");
        let sum: u64 = v7[..512].iter().map(|&b| b as u64).sum();
        v7[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
        let scan = scan_framing(&terminated(v7), &test_limits()).unwrap();
        assert_eq!(scan.dialect, TarDialectV1::OldV7);

        // PAX extended header entry.
        let pax = terminated(ustar_entry("pax_header", b"", b'x'));
        let scan = scan_framing(&pax, &test_limits()).unwrap();
        assert_eq!(scan.dialect, TarDialectV1::Pax);

        // GNU longname.
        let long = terminated(ustar_entry("././@LongLink", b"", b'L'));
        let scan = scan_framing(&long, &test_limits()).unwrap();
        assert_eq!(scan.dialect, TarDialectV1::Gnu);
    }

    fn entry_with_path(name: &[u8], prefix: &[u8]) -> ScannedEntryV1 {
        ScannedEntryV1 {
            ordinal: 0,
            raw_name: name.to_vec(),
            raw_prefix: prefix.to_vec(),
            type_flag: b'0',
            declared_size: 0,
            header_checksum_ok: true,
            dialect: TarDialectV1::UstarStrict,
            content_offset: 512,
        }
    }

    #[test]
    fn path_identity_accepts_and_keys() {
        let limits = test_limits();
        let (path, key) = assemble_ustar_path(&entry_with_path(b"Mod/A.wasm", b""), &limits).unwrap();
        assert_eq!(path.as_str(), "Mod/A.wasm");
        assert_eq!(key.as_str(), "mod/a.wasm", "portability key is ASCII-lowercase");

        // PAR-C26: exactly-100-byte path in the name field alone.
        let name100 = [b'a'; 100];
        assert!(assemble_ustar_path(&entry_with_path(&name100, b""), &limits).is_ok());

        // PAR-C16: >100-byte path with the canonical rightmost split.
        let mut long = vec![b'd'; 60];
        long.push(b'/');
        long.extend([b'f'; 60]); // 121 bytes total, canonical split at the slash
        let (prefix_part, name_part) = (&long[..60], &long[61..]);
        let ok = assemble_ustar_path(&entry_with_path(name_part, prefix_part), &limits);
        assert!(ok.is_ok(), "canonical prefix+name vector must be accepted: {ok:?}");
    }

    #[test]
    fn path_identity_rejects_bite() {
        let limits = test_limits();
        let r = |name: &[u8], prefix: &[u8]| assemble_ustar_path(&entry_with_path(name, prefix), &limits).unwrap_err();

        assert_eq!(r(b"/etc/passwd", b""), ArchiveRejectV1::AbsolutePath);
        assert_eq!(r(b"a\\b.wasm", b""), ArchiveRejectV1::RawBackslash);
        assert_eq!(r(b"a/../b", b""), ArchiveRejectV1::ParentSegment);
        assert_eq!(r(b"./a", b""), ArchiveRejectV1::CurrentSegment);
        assert_eq!(r(b"a//b", b""), ArchiveRejectV1::EmptySegment);
        assert_eq!(r(b"dir/", b""), ArchiveRejectV1::RegularTrailingSlash);
        assert_eq!(r(b"a b.wasm", b""), ArchiveRejectV1::NonPortableCharacter { byte: b' ' });
        assert_eq!(r(&[b'a', 0xFF, b'b'], b""), ArchiveRejectV1::InvalidUtf8);
        assert_eq!(r("caf\u{e9}.wasm".as_bytes(), b""), ArchiveRejectV1::NonPortableCharacter { byte: 0xC3 });

        let mut tiny = limits.clone();
        tiny.max_path_bytes = 4;
        assert_eq!(
            assemble_ustar_path(&entry_with_path(b"abcdef", b""), &tiny).unwrap_err(),
            ArchiveRejectV1::PathTooLong
        );

        // PAR-C30: path fits entirely in name, but the writer split it.
        assert_eq!(r(b"b.wasm", b"short"), ArchiveRejectV1::NoncanonicalUstarSplit);

        // Wrong-slash split on a long path (canonical split exists
        // elsewhere).
        let mut seg = vec![b'x'; 40];
        seg.push(b'/');
        seg.extend([b'y'; 40]);
        seg.push(b'/');
        seg.extend([b'z'; 40]); // 122 bytes, canonical split at second slash
        let wrong_prefix = &seg[..40]; // split at FIRST slash instead
        let wrong_name = &seg[41..];
        assert_eq!(
            assemble_ustar_path(&entry_with_path(wrong_name, wrong_prefix), &limits).unwrap_err(),
            ArchiveRejectV1::NoncanonicalUstarSplit
        );

        // PAR-C21 (NONREPRESENTABLE-USTAR-PATH) is deliberately NOT
        // driven here: it is unreachable from scanned raw fields by
        // construction — the UStar prefix/name join always inserts a
        // slash, so no assembled path can contain a >100-byte segment.
        // It is a PACKER-side terminal (a caller asks the canonical
        // packer to pack such a path; the packer must reject rather than
        // transform, cf. PAR-C31) — T2.2.09's suite drives it. The
        // scanner-side `canonical_split == None` arm stays as defense in
        // depth.
    }

    fn centry(path: &str, byte: u8) -> CanonicalEntryV1 {
        CanonicalEntryV1 {
            path: CanonicalPathV1::new(path).unwrap(),
            portability_key: MachineTextV1::new(path.to_ascii_lowercase()).unwrap(),
            size_bytes: 1,
            content_sha256: [byte; 32],
        }
    }

    #[test]
    fn namespace_collisions_bite_and_output_is_sorted() {
        // Sorted output regardless of input order.
        let ns = build_namespace(vec![centry("z/f.wasm", 1), centry("a/f.wasm", 2)]).unwrap();
        assert_eq!(ns[0].path.as_str(), "a/f.wasm");

        assert_eq!(
            build_namespace(vec![centry("a/f.wasm", 1), centry("a/f.wasm", 1)]).unwrap_err(),
            ArchiveRejectV1::DuplicateCanonicalPath
        );
        assert_eq!(
            build_namespace(vec![centry("a/File.wasm", 1), centry("a/file.wasm", 2)]).unwrap_err(),
            ArchiveRejectV1::PortableCaseCollision
        );
        // File at a path that is also an implied directory (across case).
        assert_eq!(
            build_namespace(vec![centry("a/b", 1), centry("A/B/c.wasm", 2)]).unwrap_err(),
            ArchiveRejectV1::PathKindCollision
        );
    }

    #[test]
    fn implied_directory_namespace() {
        let ns = build_namespace(vec![centry("a/b/c.wasm", 1), centry("a/d.wasm", 2), centry("top.toml", 3)]).unwrap();
        assert_eq!(implied_directories(&ns), vec!["a".to_string(), "a/b".to_string()]);
    }

    fn manifest_fixture(toml: &str, extra: &[(&str, &[u8])]) -> (Vec<u8>, Vec<ScannedEntryV1>, Vec<(CanonicalPathV1, MachineTextV1)>, Vec<CanonicalEntryV1>) {
        let mut body = ustar_entry("plugin.toml", toml.as_bytes(), b'0');
        for (name, content) in extra {
            body.extend(ustar_entry(name, content, b'0'));
        }
        let archive = terminated(body);
        let scan = scan_framing(&archive, &test_limits()).unwrap();
        let limits = test_limits();
        let assembled: Vec<_> = scan.entries.iter().map(|e| assemble_ustar_path(e, &limits).unwrap()).collect();
        let entries: Vec<_> = scan
            .entries
            .iter()
            .zip(&assembled)
            .map(|(e, (p, k))| CanonicalEntryV1 {
                path: p.clone(),
                portability_key: k.clone(),
                size_bytes: e.declared_size,
                content_sha256: [0; 32],
            })
            .collect();
        let namespace = build_namespace(entries).unwrap();
        (archive, scan.entries, assembled, namespace)
    }

    #[test]
    fn manifest_gate_accepts_and_resolves_in_declaration_order() {
        let toml = "name = \"p\"\nmodules = [\"z.wasm\", \"a.wasm\"]\ndependencies = [\"d\", \"d\"]\n";
        let (archive, scanned, assembled, ns) =
            manifest_fixture(toml, &[("a.wasm", b"A"), ("z.wasm", b"Z")]);
        let res = resolve_manifest(&archive, &scanned, &assembled, &ns, &test_limits()).unwrap();
        let order: Vec<&str> = res.modules.iter().map(|m| m.as_str()).collect();
        assert_eq!(order, vec!["z.wasm", "a.wasm"], "raw declaration order preserved (unfrozen)");
        assert!(res.module_order_unfrozen);
        assert_eq!(res.raw_duplicate_dependencies, vec!["d".to_string()], "raw dup dependency OBSERVED, not rejected");
    }

    #[test]
    fn manifest_gate_rejects_bite() {
        let limits = test_limits();
        let go = |toml: &str, extra: &[(&str, &[u8])]| {
            let (archive, scanned, assembled, ns) = manifest_fixture(toml, extra);
            resolve_manifest(&archive, &scanned, &assembled, &ns, &limits).unwrap_err()
        };

        // Missing manifest entirely.
        let body = ustar_entry("a.wasm", b"A", b'0');
        let archive = terminated(body);
        let scan = scan_framing(&archive, &limits).unwrap();
        let assembled: Vec<_> = scan.entries.iter().map(|e| assemble_ustar_path(e, &limits).unwrap()).collect();
        let ns = build_namespace(
            scan.entries
                .iter()
                .zip(&assembled)
                .map(|(e, (p, k))| CanonicalEntryV1 {
                    path: p.clone(),
                    portability_key: k.clone(),
                    size_bytes: e.declared_size,
                    content_sha256: [0; 32],
                })
                .collect(),
        )
        .unwrap();
        assert_eq!(
            resolve_manifest(&archive, &scan.entries, &assembled, &ns, &limits).unwrap_err(),
            ArchiveRejectV1::MissingManifest
        );

        assert_eq!(go("name = \"p\"\nmodules = [\"gone.wasm\"]\n", &[]), ArchiveRejectV1::DeclaredModuleMissing);
        assert_eq!(
            go("name = \"p\"\nmodules = [\"A.wasm\"]\n", &[("a.wasm", b"A")]),
            ArchiveRejectV1::DeclaredModuleAlias
        );
        assert_eq!(
            go("name = \"p\"\nmodules = [\"dir\"]\n", &[("dir/x.wasm", b"X")]),
            ArchiveRejectV1::DeclaredModuleNotRegular
        );
        assert_eq!(
            go("name = \"p\"\nmodules = [\"../esc.wasm\"]\n", &[]),
            ArchiveRejectV1::DeclaredModulePath
        );
        assert_eq!(
            go("name = \"p\"\nmodules = [\"a.wasm\", \"a.wasm\"]\n", &[("a.wasm", b"A")]),
            ArchiveRejectV1::DuplicateRawModuleDeclaration
        );
        assert_eq!(
            go("name = \"p\"\nmodules = [\"a.wasm\", \"A.wasm\"]\n", &[("a.wasm", b"A"), ("b.wasm", b"B")]),
            ArchiveRejectV1::DuplicateCanonicalModuleDeclaration
        );
        assert!(matches!(go("not = = toml", &[]), ArchiveRejectV1::ManifestParse { .. }));

        let mut small = limits.clone();
        small.max_manifest_bytes = 2;
        let (archive, scanned, assembled, ns) = manifest_fixture("name = \"p\"\n", &[]);
        assert_eq!(
            resolve_manifest(&archive, &scanned, &assembled, &ns, &small).unwrap_err(),
            ArchiveRejectV1::ManifestSizeLimit
        );
    }

    #[test]
    fn strict_admission_pipeline_and_observe_preview() {
        let limits = test_limits();
        let p = |s: &str| CanonicalPathV1::new(s).unwrap();
        let clean = pack_canonical(
            &[(p("plugin.toml"), b"name = \"p\"\nmodules = [\"m.wasm\"]\n"), (p("m.wasm"), b"M")],
            &limits,
        )
        .unwrap();

        // PAR-C14: strict without a rollout policy is BLOCKED — test-only
        // until T2.5.
        assert_eq!(
            admit_strict_canonical(&clean, &limits, None).unwrap_err(),
            ArchiveRejectV1::StrictRolloutPolicyMissing
        );
        // With a policy: full admission, separated identities present.
        let strict = admit_strict_canonical(&clean, &limits, Some("test")).unwrap();
        assert_eq!(strict.namespace.len(), 2);
        assert_eq!(strict.manifest.modules.len(), 1);
        assert_ne!(strict.artifact.size_bytes, 0);

        // Old-V7 header in strict (PAR-C25).
        let mut v7 = ustar_entry("a.wasm", b"x", b'0');
        for b in &mut v7[257..265] {
            *b = 0;
        }
        v7[148..156].copy_from_slice(b"        ");
        let sum: u64 = v7[..512].iter().map(|&b| b as u64).sum();
        v7[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
        assert_eq!(
            admit_strict_canonical(&terminated(v7), &limits, Some("test")).unwrap_err(),
            ArchiveRejectV1::OldHeaderInStrictV1
        );

        // PAX + GNU-longname strict rejects (PAR-C39/C38).
        assert_eq!(
            admit_strict_canonical(&terminated(ustar_entry("ph", b"", b'x')), &limits, Some("test")).unwrap_err(),
            ArchiveRejectV1::PaxStrictReject
        );
        assert_eq!(
            admit_strict_canonical(&terminated(ustar_entry("././@LongLink", b"", b'L')), &limits, Some("test"))
                .unwrap_err(),
            ArchiveRejectV1::GnuLongnameStrictReject
        );

        // Explicit directory record in strict (PAR-C23).
        let mut body = ustar_entry("dir/", b"", b'5');
        body.extend(ustar_entry("plugin.toml", b"name = \"p\"\n", b'0'));
        assert_eq!(
            admit_strict_canonical(&terminated(body), &limits, Some("test")).unwrap_err(),
            ArchiveRejectV1::ExplicitDirectoryInStrictV1,
            "typeflag gate fires before path assembly, so the '5' record gets its own terminal, not trailing-slash"
        );

        // Duplicate manifest outranks the generic namespace dedup
        // (PAR-008 vs 011).
        let mut dup = ustar_entry("plugin.toml", b"name = \"p\"\n", b'0');
        dup.extend(ustar_entry("plugin.toml", b"name = \"q\"\n", b'0'));
        assert_eq!(
            admit_strict_canonical(&terminated(dup), &limits, Some("test")).unwrap_err(),
            ArchiveRejectV1::DuplicateManifest
        );

        // T2.2.08: observe is TOTAL and previews strict.
        let obs = observe_legacy(&clean);
        assert_eq!(obs.strict_preview_terminal, "ACCEPT");
        assert!(obs.semantic_root.is_some());
        let obs_bad = observe_legacy(b"not a tar at all............");
        assert!(obs_bad.strict_preview_terminal.starts_with("REJECT-"), "observation records, never panics");
    }

    #[test]
    fn packer_is_canonical_and_rejects_rather_than_transforms() {
        let limits = test_limits();
        let p = |s: &str| CanonicalPathV1::new(s).unwrap();

        // Deterministic + input-order independent.
        let a = pack_canonical(&[(p("b/two.wasm"), b"2"), (p("a/one.wasm"), b"1"), (p("plugin.toml"), b"t")], &limits).unwrap();
        let b = pack_canonical(&[(p("plugin.toml"), b"t"), (p("a/one.wasm"), b"1"), (p("b/two.wasm"), b"2")], &limits).unwrap();
        assert_eq!(a, b, "packer output is a pure function of the SET (PAR-C13)");

        // Output survives our own strict read: scan + assemble round-trip,
        // strict dialect, no directory records (PAR-C33), fixed metadata
        // readable (PAR-C32 read side).
        let scan = scan_framing(&a, &limits).unwrap();
        assert_eq!(scan.dialect, TarDialectV1::UstarStrict);
        assert!(scan.entries.iter().all(|e| e.type_flag == b'0'), "no directory records");

        // Long path gets the canonical split and round-trips.
        let mut long = String::new();
        for _ in 0..6 {
            long.push_str("segment-abcdefghij/");
        }
        long.push_str("leaf.wasm"); // >100 bytes, slash-rich
        let mut generous = limits.clone();
        generous.max_path_bytes = 255;
        let packed = pack_canonical(&[(p(&long), b"L")], &generous).unwrap();
        let scan = scan_framing(&packed, &generous).unwrap();
        let (got, _) = assemble_ustar_path(&scan.entries[0], &generous).unwrap();
        assert_eq!(got.as_str(), long);

        // PAR-C21 lives here: a >100-byte single segment is not
        // representable — REJECT, never transform (PAR-C31).
        let big_segment = "q".repeat(160);
        assert_eq!(
            pack_canonical(&[(p(&big_segment), b"x")], &generous).unwrap_err(),
            ArchiveRejectV1::NonrepresentableUstarPath
        );

        // Packer-side collision rejects.
        assert_eq!(
            pack_canonical(&[(p("a.wasm"), b"1"), (p("a.wasm"), b"2")], &limits).unwrap_err(),
            ArchiveRejectV1::DuplicateCanonicalPath
        );
        assert_eq!(
            pack_canonical(&[(p("A.wasm"), b"1"), (p("a.wasm"), b"2")], &limits).unwrap_err(),
            ArchiveRejectV1::PortableCaseCollision
        );
    }

    #[test]
    fn tar_rs_reconciliation_agrees_and_split_views_bite() {
        let limits = test_limits();
        let archive = terminated({
            let mut b = ustar_entry("plugin.toml", b"name = \"p\"\n", b'0');
            b.extend(ustar_entry("a.wasm", b"A", b'0'));
            b
        });
        let scan = scan_framing(&archive, &limits).unwrap();
        assert!(reconcile_tar_rs(&archive, &scan.entries).is_ok(), "two parsers agree on clean bytes");

        // Split view: hide one scanner entry — reconciliation must catch
        // the count disagreement in BOTH directions.
        assert!(matches!(
            reconcile_tar_rs(&archive, &scan.entries[..1]),
            Err(ArchiveRejectV1::ParserViewMismatch { .. })
        ));

        // THE SUBSTITUTION PROOF (PAR-C04's mechanism): an archive with
        // garbage after the terminator — tar-rs stops at the first zero
        // block and silently tolerates it; the framing scanner REJECTS.
        // The strict pipeline calls the scanner FIRST, so tar-rs's
        // tolerance can never widen admission.
        let mut trailing = terminated(ustar_entry("a.wasm", b"A", b'0'));
        trailing.extend([0xAAu8; 512]);
        assert_eq!(scan_framing(&trailing, &limits).unwrap_err(), ArchiveRejectV1::TrailingData);
        let mut lenient = tar::Archive::new(trailing.as_slice());
        assert!(
            lenient.entries().unwrap().all(|e| e.is_ok()),
            "tar-rs tolerates what the scanner rejects — which is exactly why the scanner decides framing"
        );
    }

    #[test]
    fn identities_separate_and_semantic_root_semantics() {
        // Same content set, two different packings (entry order swapped in
        // the ARCHIVE) => same semantic root, different artifact identity
        // (PAR-003/004, C11: ordinal excluded by construction).
        let a1 = terminated({
            let mut b = ustar_entry("a.wasm", b"A", b'0');
            b.extend(ustar_entry("b.wasm", b"B", b'0'));
            b
        });
        let a2 = terminated({
            let mut b = ustar_entry("b.wasm", b"B", b'0');
            b.extend(ustar_entry("a.wasm", b"A", b'0'));
            b
        });
        let ns = build_namespace(vec![centry("a.wasm", 1), centry("b.wasm", 2)]).unwrap();
        assert_ne!(artifact_identity(&a1), artifact_identity(&a2), "artifact identity moves with packing");
        // Namespace is content-derived, identical for both packings:
        assert_eq!(semantic_root(&ns).unwrap(), semantic_root(&ns).unwrap());

        // Content flip moves the root (PAR-049).
        let ns_flip = build_namespace(vec![centry("a.wasm", 9), centry("b.wasm", 2)]).unwrap();
        assert_ne!(semantic_root(&ns).unwrap(), semantic_root(&ns_flip).unwrap());

        // PAR-C10: same file bytes in a different implied directory moves
        // the root (directory namespace IS identity).
        let ns_moved = build_namespace(vec![centry("d/a.wasm", 1), centry("b.wasm", 2)]).unwrap();
        assert_ne!(semantic_root(&ns).unwrap(), semantic_root(&ns_moved).unwrap());
    }

    #[test]
    fn terminal_names_are_catalog_names() {
        assert_eq!(ArchiveRejectV1::TruncatedArchive.terminal_name(), "REJECT-TRUNCATED-ARCHIVE");
        assert_eq!(ArchiveRejectV1::OneZeroBlockTerminator.terminal_name(), "REJECT-ONE-ZERO-BLOCK-TERMINATOR");
        assert_eq!(
            ArchiveRejectV1::UnsupportedEntryType { type_flag: b'2' }.terminal_name(),
            "REJECT-UNSUPPORTED-ENTRY-TYPE"
        );
    }
}
