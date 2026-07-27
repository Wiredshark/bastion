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
        }
    }
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
