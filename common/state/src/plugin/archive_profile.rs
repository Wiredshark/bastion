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
        }
    }
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
    fn terminal_names_are_catalog_names() {
        assert_eq!(ArchiveRejectV1::TruncatedArchive.terminal_name(), "REJECT-TRUNCATED-ARCHIVE");
        assert_eq!(ArchiveRejectV1::OneZeroBlockTerminator.terminal_name(), "REJECT-ONE-ZERO-BLOCK-TERMINATOR");
        assert_eq!(
            ArchiveRejectV1::UnsupportedEntryType { type_flag: b'2' }.terminal_name(),
            "REJECT-UNSUPPORTED-ENTRY-TYPE"
        );
    }
}
