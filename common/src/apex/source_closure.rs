//! `APEX-T1.2` — declared source/asset closure record (fleet-authored spec
//! `readme/apex/APEX-T1.2-DECLARED-SOURCE-ASSET-CLOSURE-FLEET-v1.md`).
//!
//! The `SourceClosureRecordV1` is a PURE FUNCTION of (commit tree, resolved
//! LFS content, toolchain/config bytes). Nothing here reads the filesystem
//! or runs git — this module owns the canonical shapes, hazard rules, and
//! digests; capture lives in the `apex_source_closure` harness bin. Tree
//! entries carry GIT-recorded modes and GIT blob content hashes (LFS files
//! contribute their verified pointer oid), never filesystem bits or
//! checkout-materialized bytes, so the record is byte-identical across
//! checkout paths, operating systems, and eol configs by construction.
//!
//! Roots are digested under `DigestDomainIdV1::SourceClosure` (= 11): the
//! closure is an INPUT that `APEX-T1.5`'s `BuildManifest` (= 5) embeds —
//! separating inputs from the manifest that embeds them is the point of
//! domain separation.

use super::digest::{
    ArtifactIdentityV1, DigestDomainIdV1, DigestErrorV1, ProtocolDigestV1, digest_manifest_value_v1,
};
use super::manifest::{
    CanonicalFieldMapV1, CanonicalPathV1, FieldIdV1, MachineTextV1, ManifestCodecErrorCodeV1, ManifestCodecErrorV1,
    ManifestDecodeLimitsV1, ManifestDecodeV1, ManifestEncodeV1, ManifestErrorV1, ManifestSchemaErrorV1,
    ManifestValueV1, StructFieldsV1,
};

pub const SOURCE_CLOSURE_SCHEMA_V1: &str = "bastion.source-closure/v1";

/// Own limits (T0.2's `ManifestDecodeLimitsV1` deliberately has no
/// `Default`). Sized for the live tree with headroom: ~12k full-tree /
/// ~10.6k asset entries, each a 4-field map (~6 nodes) → ~80k nodes per
/// scope manifest; paths are short ASCII; the largest byte string is a
/// 32-byte sha256.
pub const fn source_closure_limits_v1() -> ManifestDecodeLimitsV1 {
    ManifestDecodeLimitsV1 {
        max_input_bytes: 64 << 20,
        max_depth: 8,
        max_nodes: 1 << 21,
        max_array_items: 1 << 17,
        max_map_entries: 32,
        max_machine_text_bytes: 4096,
        max_byte_string_bytes: 4096,
    }
}

/// Typed failures for closure construction (schema-level hazards; the
/// capture tool maps these onto the packet's `T1.2-BLOCK-*` terminals).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceClosureErrorV1 {
    /// Symlink (120000), gitlink (160000), or any mode other than the two
    /// portable blob modes — `T1.2-BLOCK-TREE-HAZARD`.
    ForbiddenGitMode { mode: String },
    /// Two tree paths collide under ASCII case-folding (ambiguous on a
    /// case-insensitive filesystem) — `T1.2-BLOCK-TREE-HAZARD`.
    CaseFoldCollision { path: String },
    /// Entries not in strictly increasing path-byte order (duplicates
    /// included) — the canonical walk is broken.
    UnsortedOrDuplicatePath { path: String },
    /// Path failed the `CanonicalPathV1` grammar (absolute, `..`, empty
    /// component, non-ASCII, backslash) — `T1.2-BLOCK-SCOPE-ESCAPE`.
    NonCanonicalPath { detail: String },
    /// A 40-lower-hex git id was malformed.
    InvalidGitHexId { detail: String },
    /// T0.2/T0.3 encode or digest failure while building a root.
    Digest(DigestErrorV1),
}

impl std::fmt::Display for SourceClosureErrorV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}

/// A validated 40-lower-hex git object id (commit or tree). Checked at
/// construction — spec section 5's "checked constructor, not caller
/// convention".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitHexIdV1(MachineTextV1);

impl GitHexIdV1 {
    pub fn new(s: impl Into<String>) -> Result<Self, SourceClosureErrorV1> {
        let s: String = s.into();
        if s.len() != 40 || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return Err(SourceClosureErrorV1::InvalidGitHexId { detail: format!("not 40-lower-hex: {s:?}") });
        }
        Ok(Self(MachineTextV1::new(s).expect("40-lower-hex is ASCII")))
    }

    pub fn as_str(&self) -> &str { self.0.as_str() }
}

/// One canonical tree entry: `(path, git_mode, size_bytes, sha256)` per
/// spec §7a — mode is the GIT-recorded token (`100644`/`100755`), the hash
/// is of GIT CONTENT (blob bytes; for LFS paths the verified resolved
/// content = pointer oid).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureTreeEntryV1 {
    pub path: CanonicalPathV1,
    pub git_mode: MachineTextV1,
    pub size_bytes: u64,
    pub sha256: [u8; 32],
}

const GIT_MODE_BLOB: &str = "100644";
const GIT_MODE_BLOB_EXEC: &str = "100755";

/// A hazard-checked canonical file tree (one closure scope). The
/// constructor — not caller convention — enforces the tree rules the
/// T0.2 path codec explicitly does NOT own: portable blob modes only,
/// strictly increasing path-byte order, no ASCII case-fold collisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureTreeV1 {
    entries: Vec<ClosureTreeEntryV1>,
}

impl ClosureTreeV1 {
    pub fn try_new(mut entries: Vec<ClosureTreeEntryV1>) -> Result<Self, SourceClosureErrorV1> {
        // Sort here so callers may feed any walk order: the packet's
        // path-order canary (same set, shuffled walk → same root) is a
        // property of this constructor, not of caller discipline.
        entries.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));
        let mut folded: Vec<String> = Vec::with_capacity(entries.len());
        for pair in entries.windows(2) {
            if pair[0].path.as_str().as_bytes() >= pair[1].path.as_str().as_bytes() {
                return Err(SourceClosureErrorV1::UnsortedOrDuplicatePath { path: pair[1].path.as_str().to_owned() });
            }
        }
        for e in &entries {
            let mode = e.git_mode.as_str();
            if mode != GIT_MODE_BLOB && mode != GIT_MODE_BLOB_EXEC {
                return Err(SourceClosureErrorV1::ForbiddenGitMode { mode: mode.to_owned() });
            }
            folded.push(e.path.as_str().to_ascii_lowercase());
        }
        folded.sort_unstable();
        for pair in folded.windows(2) {
            if pair[0] == pair[1] {
                return Err(SourceClosureErrorV1::CaseFoldCollision { path: pair[0].clone() });
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[ClosureTreeEntryV1] { &self.entries }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// The scope's protocol root under the `SourceClosure` domain.
    pub fn root(&self) -> Result<ProtocolDigestV1, SourceClosureErrorV1> {
        digest_manifest_value_v1(DigestDomainIdV1::SourceClosure, self, &source_closure_limits_v1())
            .map_err(SourceClosureErrorV1::Digest)
    }
}

/// One verified-LFS-file verdict entry: the pointer's declared oid and
/// size, admitted only after the capture tool proved the on-disk bytes
/// match (stub / missing / mismatch each die on a typed terminal BEFORE a
/// report is built — an `LfsReportV1` contains only proven rows).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LfsReportEntryV1 {
    pub path: CanonicalPathV1,
    pub oid_sha256: [u8; 32],
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LfsReportV1 {
    entries: Vec<LfsReportEntryV1>,
}

impl LfsReportV1 {
    pub fn try_new(mut entries: Vec<LfsReportEntryV1>) -> Result<Self, SourceClosureErrorV1> {
        entries.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));
        for pair in entries.windows(2) {
            if pair[0].path.as_str().as_bytes() >= pair[1].path.as_str().as_bytes() {
                return Err(SourceClosureErrorV1::UnsortedOrDuplicatePath { path: pair[1].path.as_str().to_owned() });
            }
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[LfsReportEntryV1] { &self.entries }

    pub fn len(&self) -> usize { self.entries.len() }

    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn root(&self) -> Result<ProtocolDigestV1, SourceClosureErrorV1> {
        digest_manifest_value_v1(DigestDomainIdV1::SourceClosure, self, &source_closure_limits_v1())
            .map_err(SourceClosureErrorV1::Digest)
    }
}

/// The rust-source exclusion list — the flake's `pathsToIgnore` string
/// prefixes, in the flake's literal order (single-fileset rule: this list
/// is asserted textually identical to `flake.nix`'s by a canary; there is
/// no second hand-maintained include list to drift).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterSpecV1 {
    prefixes: Vec<MachineTextV1>,
}

impl FilterSpecV1 {
    pub fn new(prefixes: Vec<MachineTextV1>) -> Self { Self { prefixes } }

    pub fn prefixes(&self) -> &[MachineTextV1] { &self.prefixes }

    /// Mirrors the flake's `ignorePaths` semantics EXACTLY: exclude when
    /// any listed name is a string-prefix of the repo-relative path (the
    /// flake uses `lib.hasPrefix`, i.e. "nix" also excludes a hypothetical
    /// "nixfoo" — replicated, not "improved", per the single-fileset rule).
    pub fn excludes(&self, repo_relative_path: &str) -> bool {
        self.prefixes.iter().any(|p| repo_relative_path.starts_with(p.as_str()))
    }

    pub fn digest(&self) -> Result<ProtocolDigestV1, SourceClosureErrorV1> {
        digest_manifest_value_v1(DigestDomainIdV1::SourceClosure, self, &source_closure_limits_v1())
            .map_err(SourceClosureErrorV1::Digest)
    }
}

/// A pinned workspace build script + the env inputs it DECLARES
/// (`cargo:rerun-if-env-changed` statements, statically scanned), sorted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildScriptPinV1 {
    pub path: CanonicalPathV1,
    pub artifact: ArtifactIdentityV1,
    pub declared_env_inputs: Vec<MachineTextV1>,
}

/// A pinned auxiliary manifest file (workspace member `Cargo.toml`s, spec
/// §7a adopted pin).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinnedFileV1 {
    pub path: CanonicalPathV1,
    pub artifact: ArtifactIdentityV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceClosureCountsV1 {
    pub rust_files: u64,
    pub asset_files: u64,
    pub lfs_files: u64,
}

/// The record itself — spec section 5 field IDs 0–13 frozen at
/// cross-review; 14 (`.gitattributes`) and 15 (workspace `Cargo.toml`s)
/// APPENDED for the §7a-adopted extended pins (no shipped record predates
/// them; frozen means never renumbered, additions append).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceClosureRecordV1 {
    pub commit: GitHexIdV1,
    pub tree: GitHexIdV1,
    pub rust_source_root: ProtocolDigestV1,
    pub asset_tree_root: ProtocolDigestV1,
    pub filter_spec_digest: ProtocolDigestV1,
    pub toolchain_file: ArtifactIdentityV1,
    pub cargo_lock: ArtifactIdentityV1,
    pub cargo_config: ArtifactIdentityV1,
    pub flake_nix: ArtifactIdentityV1,
    pub flake_lock: ArtifactIdentityV1,
    pub build_scripts: Vec<BuildScriptPinV1>,
    pub lfs_report_root: ProtocolDigestV1,
    pub file_counts: SourceClosureCountsV1,
    pub gitattributes: ArtifactIdentityV1,
    pub workspace_manifests: Vec<PinnedFileV1>,
}

fn schema_err(code: ManifestCodecErrorCodeV1) -> ManifestSchemaErrorV1 { ManifestErrorV1::new(code) }

fn map_value(entries: Vec<(u16, ManifestValueV1)>) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
    let entries = entries.into_iter().map(|(id, v)| (FieldIdV1::new(id), v)).collect();
    Ok(ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries)?))
}

fn take_unsigned(value: ManifestValueV1) -> Result<u64, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Unsigned(v) => Ok(v),
        _ => Err(schema_err(ManifestCodecErrorCodeV1::FieldKeyType)),
    }
}

fn take_text(value: ManifestValueV1) -> Result<MachineTextV1, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::MachineText(t) => Ok(t),
        _ => Err(schema_err(ManifestCodecErrorCodeV1::FieldKeyType)),
    }
}

fn take_path(value: ManifestValueV1) -> Result<CanonicalPathV1, ManifestSchemaErrorV1> {
    CanonicalPathV1::new(take_text(value)?.as_str())
}

fn take_sha256(value: ManifestValueV1) -> Result<[u8; 32], ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Bytes(b) if b.len() == 32 => {
            let mut out = [0u8; 32];
            out.copy_from_slice(&b);
            Ok(out)
        },
        _ => Err(schema_err(ManifestCodecErrorCodeV1::FieldKeyType).detail("sha256 must be a 32-byte string")),
    }
}

fn take_array(value: ManifestValueV1) -> Result<Vec<ManifestValueV1>, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Array(items) => Ok(items),
        _ => Err(schema_err(ManifestCodecErrorCodeV1::FieldKeyType)),
    }
}

fn take_map(value: ManifestValueV1) -> Result<StructFieldsV1, ManifestSchemaErrorV1> {
    match value {
        ManifestValueV1::Map(m) => Ok(StructFieldsV1::new(m)),
        _ => Err(schema_err(ManifestCodecErrorCodeV1::FieldKeyType)),
    }
}

impl ManifestEncodeV1 for ClosureTreeEntryV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(self.path.as_str())?)),
            (1, ManifestValueV1::MachineText(self.git_mode.clone())),
            (2, ManifestValueV1::Unsigned(self.size_bytes)),
            (3, ManifestValueV1::Bytes(self.sha256.to_vec())),
        ])
    }
}

impl ManifestDecodeV1 for ClosureTreeEntryV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut fields = take_map(value)?;
        let path = take_path(fields.take_required(FieldIdV1::new(0))?)?;
        let git_mode = take_text(fields.take_required(FieldIdV1::new(1))?)?;
        let size_bytes = take_unsigned(fields.take_required(FieldIdV1::new(2))?)?;
        let sha256 = take_sha256(fields.take_required(FieldIdV1::new(3))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { path, git_mode, size_bytes, sha256 })
    }
}

impl ManifestEncodeV1 for ClosureTreeV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        Ok(ManifestValueV1::Array(self.entries.iter().map(|e| e.to_manifest_value_v1()).collect::<Result<_, _>>()?))
    }
}

impl ManifestDecodeV1 for ClosureTreeV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let entries = take_array(value)?
            .into_iter()
            .map(ClosureTreeEntryV1::from_manifest_value_v1)
            .collect::<Result<Vec<_>, _>>()?;
        Self::try_new(entries)
            .map_err(|_| schema_err(ManifestCodecErrorCodeV1::FieldKeyType).detail("tree hazard"))
    }
}

impl ManifestEncodeV1 for LfsReportEntryV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(self.path.as_str())?)),
            (1, ManifestValueV1::Bytes(self.oid_sha256.to_vec())),
            (2, ManifestValueV1::Unsigned(self.size_bytes)),
        ])
    }
}

impl ManifestDecodeV1 for LfsReportEntryV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut fields = take_map(value)?;
        let path = take_path(fields.take_required(FieldIdV1::new(0))?)?;
        let oid_sha256 = take_sha256(fields.take_required(FieldIdV1::new(1))?)?;
        let size_bytes = take_unsigned(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { path, oid_sha256, size_bytes })
    }
}

impl ManifestEncodeV1 for LfsReportV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        Ok(ManifestValueV1::Array(self.entries.iter().map(|e| e.to_manifest_value_v1()).collect::<Result<_, _>>()?))
    }
}

impl ManifestDecodeV1 for LfsReportV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let entries = take_array(value)?
            .into_iter()
            .map(LfsReportEntryV1::from_manifest_value_v1)
            .collect::<Result<Vec<_>, _>>()?;
        Self::try_new(entries)
            .map_err(|_| schema_err(ManifestCodecErrorCodeV1::FieldKeyType).detail("lfs report hazard"))
    }
}

impl ManifestEncodeV1 for FilterSpecV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        Ok(ManifestValueV1::Array(
            self.prefixes.iter().map(|p| ManifestValueV1::MachineText(p.clone())).collect(),
        ))
    }
}

impl ManifestDecodeV1 for FilterSpecV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        Ok(Self::new(take_array(value)?.into_iter().map(take_text).collect::<Result<Vec<_>, _>>()?))
    }
}

impl ManifestEncodeV1 for BuildScriptPinV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(self.path.as_str())?)),
            (1, self.artifact.to_manifest_value_v1()?),
            (2, ManifestValueV1::Array(
                self.declared_env_inputs.iter().map(|t| ManifestValueV1::MachineText(t.clone())).collect(),
            )),
        ])
    }
}

impl ManifestDecodeV1 for BuildScriptPinV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut fields = take_map(value)?;
        let path = take_path(fields.take_required(FieldIdV1::new(0))?)?;
        let artifact = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        let declared_env_inputs =
            take_array(fields.take_required(FieldIdV1::new(2))?)?.into_iter().map(take_text).collect::<Result<Vec<_>, _>>()?;
        fields.finish_no_unknown()?;
        Ok(Self { path, artifact, declared_env_inputs })
    }
}

impl ManifestEncodeV1 for PinnedFileV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(MachineTextV1::new(self.path.as_str())?)),
            (1, self.artifact.to_manifest_value_v1()?),
        ])
    }
}

impl ManifestDecodeV1 for PinnedFileV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut fields = take_map(value)?;
        let path = take_path(fields.take_required(FieldIdV1::new(0))?)?;
        let artifact = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(1))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { path, artifact })
    }
}

impl ManifestEncodeV1 for SourceClosureCountsV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::Unsigned(self.rust_files)),
            (1, ManifestValueV1::Unsigned(self.asset_files)),
            (2, ManifestValueV1::Unsigned(self.lfs_files)),
        ])
    }
}

impl ManifestDecodeV1 for SourceClosureCountsV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut fields = take_map(value)?;
        let rust_files = take_unsigned(fields.take_required(FieldIdV1::new(0))?)?;
        let asset_files = take_unsigned(fields.take_required(FieldIdV1::new(1))?)?;
        let lfs_files = take_unsigned(fields.take_required(FieldIdV1::new(2))?)?;
        fields.finish_no_unknown()?;
        Ok(Self { rust_files, asset_files, lfs_files })
    }
}

impl ManifestEncodeV1 for SourceClosureRecordV1 {
    fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, ManifestCodecErrorV1> {
        map_value(vec![
            (0, ManifestValueV1::MachineText(
                MachineTextV1::new(SOURCE_CLOSURE_SCHEMA_V1)?,
            )),
            (1, ManifestValueV1::MachineText(self.commit.0.clone())),
            (2, ManifestValueV1::MachineText(self.tree.0.clone())),
            (3, self.rust_source_root.to_manifest_value_v1()?),
            (4, self.asset_tree_root.to_manifest_value_v1()?),
            (5, self.filter_spec_digest.to_manifest_value_v1()?),
            (6, self.toolchain_file.to_manifest_value_v1()?),
            (7, self.cargo_lock.to_manifest_value_v1()?),
            (8, self.cargo_config.to_manifest_value_v1()?),
            (9, self.flake_nix.to_manifest_value_v1()?),
            (10, self.flake_lock.to_manifest_value_v1()?),
            (11, ManifestValueV1::Array(
                self.build_scripts.iter().map(|b| b.to_manifest_value_v1()).collect::<Result<_, _>>()?,
            )),
            (12, self.lfs_report_root.to_manifest_value_v1()?),
            (13, self.file_counts.to_manifest_value_v1()?),
            (14, self.gitattributes.to_manifest_value_v1()?),
            (15, ManifestValueV1::Array(
                self.workspace_manifests.iter().map(|p| p.to_manifest_value_v1()).collect::<Result<_, _>>()?,
            )),
        ])
    }
}

impl ManifestDecodeV1 for SourceClosureRecordV1 {
    fn from_manifest_value_v1(value: ManifestValueV1) -> Result<Self, ManifestSchemaErrorV1> {
        let mut fields = take_map(value)?;
        let schema = take_text(fields.take_required(FieldIdV1::new(0))?)?;
        if schema.as_str() != SOURCE_CLOSURE_SCHEMA_V1 {
            return Err(schema_err(ManifestCodecErrorCodeV1::FieldKeyType).detail("wrong schema tag"));
        }
        let hex = |t: MachineTextV1| {
            GitHexIdV1::new(t.as_str())
                .map_err(|_| schema_err(ManifestCodecErrorCodeV1::FieldKeyType).detail("git hex id"))
        };
        let commit = hex(take_text(fields.take_required(FieldIdV1::new(1))?)?)?;
        let tree = hex(take_text(fields.take_required(FieldIdV1::new(2))?)?)?;
        let rust_source_root = ProtocolDigestV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(3))?)?;
        let asset_tree_root = ProtocolDigestV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(4))?)?;
        let filter_spec_digest = ProtocolDigestV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(5))?)?;
        let toolchain_file = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(6))?)?;
        let cargo_lock = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(7))?)?;
        let cargo_config = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(8))?)?;
        let flake_nix = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(9))?)?;
        let flake_lock = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(10))?)?;
        let build_scripts = take_array(fields.take_required(FieldIdV1::new(11))?)?
            .into_iter()
            .map(BuildScriptPinV1::from_manifest_value_v1)
            .collect::<Result<Vec<_>, _>>()?;
        let lfs_report_root = ProtocolDigestV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(12))?)?;
        let file_counts = SourceClosureCountsV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(13))?)?;
        let gitattributes = ArtifactIdentityV1::from_manifest_value_v1(fields.take_required(FieldIdV1::new(14))?)?;
        let workspace_manifests = take_array(fields.take_required(FieldIdV1::new(15))?)?
            .into_iter()
            .map(PinnedFileV1::from_manifest_value_v1)
            .collect::<Result<Vec<_>, _>>()?;
        fields.finish_no_unknown()?;
        Ok(Self {
            commit,
            tree,
            rust_source_root,
            asset_tree_root,
            filter_spec_digest,
            toolchain_file,
            cargo_lock,
            cargo_config,
            flake_nix,
            flake_lock,
            build_scripts,
            lfs_report_root,
            file_counts,
            gitattributes,
            workspace_manifests,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;
    use crate::apex::manifest::{decode_manifest_v1, encode_manifest_v1};

    fn entry(path: &str, byte: u8) -> ClosureTreeEntryV1 {
        ClosureTreeEntryV1 {
            path: CanonicalPathV1::new(path).unwrap(),
            git_mode: MachineTextV1::new("100644").unwrap(),
            size_bytes: 3,
            sha256: [byte; 32],
        }
    }

    #[test]
    fn git_hex_id_checked() {
        assert!(GitHexIdV1::new("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(GitHexIdV1::new("0123456789ABCDEF0123456789ABCDEF01234567").is_err(), "upper hex rejected");
        assert!(GitHexIdV1::new("0123456789abcdef").is_err(), "short rejected");
        assert!(GitHexIdV1::new("0123456789abcdef0123456789abcdef0123456g").is_err(), "non-hex rejected");
    }

    #[test]
    fn walk_order_does_not_change_root_but_byte_flip_does() {
        let a = ClosureTreeV1::try_new(vec![entry("a/one", 1), entry("b/two", 2), entry("c/three", 3)]).unwrap();
        let b = ClosureTreeV1::try_new(vec![entry("c/three", 3), entry("a/one", 1), entry("b/two", 2)]).unwrap();
        assert_eq!(a.root().unwrap(), b.root().unwrap(), "shuffled walk must not move the root");

        let mut flipped = entry("b/two", 2);
        flipped.sha256[7] ^= 0x01;
        let c = ClosureTreeV1::try_new(vec![entry("a/one", 1), flipped, entry("c/three", 3)]).unwrap();
        assert_ne!(a.root().unwrap(), c.root().unwrap(), "one flipped content byte must move the root");
    }

    #[test]
    fn tree_hazards_bite() {
        let mut sym = entry("a/link", 1);
        sym.git_mode = MachineTextV1::new("120000").unwrap();
        assert!(matches!(
            ClosureTreeV1::try_new(vec![sym]),
            Err(SourceClosureErrorV1::ForbiddenGitMode { .. })
        ));

        let mut gitlink = entry("sub", 1);
        gitlink.git_mode = MachineTextV1::new("160000").unwrap();
        assert!(matches!(
            ClosureTreeV1::try_new(vec![gitlink]),
            Err(SourceClosureErrorV1::ForbiddenGitMode { .. })
        ));

        assert!(matches!(
            ClosureTreeV1::try_new(vec![entry("a/File", 1), entry("a/file", 2)]),
            Err(SourceClosureErrorV1::CaseFoldCollision { .. })
        ));

        assert!(matches!(
            ClosureTreeV1::try_new(vec![entry("a/one", 1), entry("a/one", 1)]),
            Err(SourceClosureErrorV1::UnsortedOrDuplicatePath { .. })
        ));
    }

    #[test]
    fn filter_spec_matches_flake_prefix_semantics() {
        let spec = FilterSpecV1::new(vec![
            MachineTextV1::new("nix").unwrap(),
            MachineTextV1::new("assets").unwrap(),
        ]);
        assert!(spec.excludes("assets/voxel/foo.vox"));
        assert!(spec.excludes("nix/shell.nix"));
        assert!(spec.excludes("nixfoo.txt"), "hasPrefix is a STRING prefix — replicated, not corrected");
        assert!(!spec.excludes("common/src/lib.rs"));
    }

    fn sample_record() -> SourceClosureRecordV1 {
        let tree = ClosureTreeV1::try_new(vec![entry("common/src/lib.rs", 4)]).unwrap();
        let assets = ClosureTreeV1::try_new(vec![entry("assets/voxel/x.vox", 5)]).unwrap();
        let lfs = LfsReportV1::try_new(vec![LfsReportEntryV1 {
            path: CanonicalPathV1::new("assets/voxel/x.vox").unwrap(),
            oid_sha256: [5; 32],
            size_bytes: 3,
        }])
        .unwrap();
        let filter = FilterSpecV1::new(vec![MachineTextV1::new("assets").unwrap()]);
        let art = |b: &[u8]| hash_artifact_bytes_v1(b);
        SourceClosureRecordV1 {
            commit: GitHexIdV1::new("0123456789abcdef0123456789abcdef01234567").unwrap(),
            tree: GitHexIdV1::new("fedcba9876543210fedcba9876543210fedcba98").unwrap(),
            rust_source_root: tree.root().unwrap(),
            asset_tree_root: assets.root().unwrap(),
            filter_spec_digest: filter.digest().unwrap(),
            toolchain_file: art(b"nightly-2026-06-13"),
            cargo_lock: art(b"lock"),
            cargo_config: art(b"config"),
            flake_nix: art(b"flake"),
            flake_lock: art(b"flake-lock"),
            build_scripts: vec![BuildScriptPinV1 {
                path: CanonicalPathV1::new("common/build.rs").unwrap(),
                artifact: art(b"build script"),
                declared_env_inputs: vec![MachineTextV1::new("BASTION_SOURCE_REVISION").unwrap()],
            }],
            lfs_report_root: lfs.root().unwrap(),
            file_counts: SourceClosureCountsV1 { rust_files: 1, asset_files: 1, lfs_files: 1 },
            gitattributes: art(b"attrs"),
            workspace_manifests: vec![PinnedFileV1 {
                path: CanonicalPathV1::new("Cargo.toml").unwrap(),
                artifact: art(b"workspace manifest"),
            }],
        }
    }

    #[test]
    fn record_round_trips_canonically() {
        let record = sample_record();
        let limits = source_closure_limits_v1();
        let bytes = encode_manifest_v1(&record, &limits).unwrap();
        let decoded: SourceClosureRecordV1 = decode_manifest_v1(&bytes, &limits).unwrap();
        assert_eq!(decoded, record);
        let re_encoded = encode_manifest_v1(&decoded, &limits).unwrap();
        assert_eq!(bytes, re_encoded, "record must be a fixed point of decode->encode");
    }

    #[test]
    fn record_decode_rejects_wrong_schema_and_unknown_field() {
        let record = sample_record();
        let limits = source_closure_limits_v1();

        // Wrong schema tag.
        let ManifestValueV1::Map(map) = record.to_manifest_value_v1().unwrap() else { panic!() };
        let mut entries: Vec<(FieldIdV1, ManifestValueV1)> = map.into_entries();
        entries[0].1 = ManifestValueV1::MachineText(MachineTextV1::new("bastion.other/v1").unwrap());
        let wrong = ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries).unwrap());
        assert!(SourceClosureRecordV1::from_manifest_value_v1(wrong).is_err());

        // Unknown trailing field is fail-closed.
        let ManifestValueV1::Map(map) = record.to_manifest_value_v1().unwrap() else { panic!() };
        let mut entries = map.into_entries();
        entries.push((FieldIdV1::new(99), ManifestValueV1::Bool(true)));
        let extended = ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries).unwrap());
        assert!(SourceClosureRecordV1::from_manifest_value_v1(extended).is_err());
        let _ = limits;
    }
}
