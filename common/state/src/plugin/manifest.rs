//! `APEX-T2.3` — `PluginManifestV1` static plugin contract (REAL packet
//! `PROJECT-BASTION-APEX-MICROSTEP-APEX-T2.3-PLUGIN-MANIFEST-V1.md`;
//! 70-case canary pin `0c079bcc…` verified; `PluginManifest` digest
//! domain = 8, pre-registered at T0.3 on my own earlier flag — cited,
//! not re-added).
//!
//! This slice: T2.3.02–.06 — the checked scalar vocabulary. Version
//! probe/dispatch, the plugin-ID grammar, exact SemVer with build
//! metadata REJECTED (packet section 5.3: build metadata creates
//! identity ambiguity — two different byte strings compare SemVer-equal),
//! the `veloren:plugin@<semver>` host-API requirement, and the injected
//! limits with NO production defaults (packet T2.3.11).

use common::apex::manifest::MachineTextV1;

pub const PLUGIN_MANIFEST_VERSION_V1: u32 = 1;
pub const HOST_API_PACKAGE_V1: &str = "veloren:plugin";

/// Injected limits (packet section 7) — deliberately no `Default`; every
/// admission names the policy it ran under (same rule as T2.2's archive
/// limits and T0.2's decode limits).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginManifestLimitsV1 {
    pub policy_id: MachineTextV1,
    pub max_manifest_bytes: u64,
    pub max_plugin_id_bytes: u16,
    pub max_display_name_bytes: u16,
    pub max_module_count: u32,
    pub max_dependency_count: u32,
    pub max_runtime_claim_modes: u8,
    pub max_command_claims_per_mode: u32,
    pub max_animation_claims_per_mode: u32,
    pub max_asset_root_count: u32,
    pub max_runtime_key_bytes: u16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PluginManifestEnforcementModeV1 {
    ObserveLegacy,
    StrictV1,
}

/// Typed error families (packet section 8 — the full 40-family taxonomy
/// lands with the raw-decode slice; this slice carries the scalar
/// families it can already produce).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginManifestErrorV1 {
    InvalidPluginId { detail: &'static str },
    InvalidDisplayName { detail: &'static str },
    InvalidPluginVersion,
    PluginVersionBuildMetadataForbidden,
    InvalidHostApi { detail: &'static str },
    UnsupportedHostPackage,
    LimitExceeded { what: &'static str },
    MissingManifestVersion,
    InvalidManifestVersionType,
    UnsupportedManifestVersion { got: i64 },
}

/// Packet section 5.2 grammar — lowercase ASCII, `namespace ":" name`,
/// dotted namespace labels, hyphenated labels with no leading/trailing/
/// repeated hyphen, no underscore. EXACT BYTES ARE IDENTITY.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CanonicalPluginIdV1(String);

fn valid_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    let bytes = label.as_bytes();
    if bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    let mut prev_hyphen = false;
    for &b in bytes {
        match b {
            b'a'..=b'z' | b'0'..=b'9' => prev_hyphen = false,
            b'-' => {
                if prev_hyphen {
                    return false;
                }
                prev_hyphen = true;
            },
            _ => return false,
        }
    }
    true
}

impl CanonicalPluginIdV1 {
    pub fn parse(s: &str, limits: &PluginManifestLimitsV1) -> Result<Self, PluginManifestErrorV1> {
        if s.len() > limits.max_plugin_id_bytes as usize {
            return Err(PluginManifestErrorV1::LimitExceeded { what: "plugin id bytes" });
        }
        let (namespace, name) = s
            .split_once(':')
            .ok_or(PluginManifestErrorV1::InvalidPluginId { detail: "missing ':' separator" })?;
        if name.contains(':') {
            return Err(PluginManifestErrorV1::InvalidPluginId { detail: "more than one ':'" });
        }
        if namespace.is_empty() || namespace.split('.').any(|label| !valid_label(label)) {
            return Err(PluginManifestErrorV1::InvalidPluginId { detail: "invalid namespace label" });
        }
        if !valid_label(name) {
            return Err(PluginManifestErrorV1::InvalidPluginId { detail: "invalid name label" });
        }
        Ok(Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

/// Optional, NON-authoritative display name (never identity).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDisplayNameV1(String);

impl PluginDisplayNameV1 {
    pub fn parse(s: &str, limits: &PluginManifestLimitsV1) -> Result<Self, PluginManifestErrorV1> {
        if s.len() > limits.max_display_name_bytes as usize {
            return Err(PluginManifestErrorV1::LimitExceeded { what: "display name bytes" });
        }
        if s.is_empty() || s.chars().any(|c| c.is_control()) {
            return Err(PluginManifestErrorV1::InvalidDisplayName { detail: "empty or control characters" });
        }
        Ok(Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

/// Exact plugin SemVer. Prerelease permitted; BUILD METADATA REJECTED
/// (packet 5.3 + adversarial 12.5: `1.0.0+a` and `1.0.0+b` are different
/// bytes that compare equal under SemVer — an identity ambiguity strict
/// V1 refuses to admit).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PluginVersionV1(semver::Version);

impl PluginVersionV1 {
    pub fn parse(s: &str) -> Result<Self, PluginManifestErrorV1> {
        let v = semver::Version::parse(s).map_err(|_| PluginManifestErrorV1::InvalidPluginVersion)?;
        if !v.build.is_empty() {
            return Err(PluginManifestErrorV1::PluginVersionBuildMetadataForbidden);
        }
        Ok(Self(v))
    }

    pub fn get(&self) -> &semver::Version { &self.0 }
}

/// Packet 5.4: `host_api = "veloren:plugin@<full-semver>"` — package
/// identity + syntax validated HERE; whether the server supports the
/// version is `T2.5`'s compatibility selection, not this row's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPluginApiRequirementV1 {
    pub package: MachineTextV1,
    pub version: semver::Version,
}

impl HostPluginApiRequirementV1 {
    pub fn parse(s: &str) -> Result<Self, PluginManifestErrorV1> {
        let (package, version) = s
            .split_once('@')
            .ok_or(PluginManifestErrorV1::InvalidHostApi { detail: "missing '@'" })?;
        if package != HOST_API_PACKAGE_V1 {
            return Err(PluginManifestErrorV1::UnsupportedHostPackage);
        }
        let version =
            semver::Version::parse(version).map_err(|_| PluginManifestErrorV1::InvalidHostApi { detail: "bad semver" })?;
        if !version.build.is_empty() {
            return Err(PluginManifestErrorV1::InvalidHostApi { detail: "build metadata forbidden" });
        }
        Ok(Self { package: MachineTextV1::new(package).expect("validated ASCII"), version })
    }
}

/// T2.3.03 — the explicit `manifest_version` probe: performed on the RAW
/// TOML value BEFORE any typed decoding, so a malformed V1 can never fall
/// back to legacy by failing the typed decode (packet adversarial 12.3).
pub fn probe_manifest_version(raw: &toml::Value) -> Result<Option<u32>, PluginManifestErrorV1> {
    match raw.get("manifest_version") {
        None => Ok(None), // absent = legacy observation lane
        Some(toml::Value::Integer(v)) => {
            if *v == PLUGIN_MANIFEST_VERSION_V1 as i64 {
                Ok(Some(PLUGIN_MANIFEST_VERSION_V1))
            } else {
                Err(PluginManifestErrorV1::UnsupportedManifestVersion { got: *v })
            }
        },
        Some(_) => Err(PluginManifestErrorV1::InvalidManifestVersionType),
    }
}

// ---------------------------------------------------------------------------
// T2.3.07-.17 — raw strict decode, validation, semantic root, legacy
// observation, and the binding to the landed T2.2 archive types.
// ---------------------------------------------------------------------------

use super::archive_profile::{CanonicalEntryV1, portable_byte};
use common::apex::digest::{
    ArtifactIdentityV1, DigestDomainIdV1, ProtocolDigestV1, digest_manifest_value_v1, hash_artifact_bytes_v1,
};
use common::apex::manifest::{
    CanonicalFieldMapV1, CanonicalPathV1, FieldIdV1, ManifestEncodeV1, ManifestValueV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginManifestErrorExtV1 {
    ManifestTooLarge,
    ManifestUtf8,
    TomlDecode { detail: String },
    UnknownField { detail: String },
    MissingRequiredField { detail: String },
    InvalidModuleWorld,
    DuplicateModulePath,
    MissingModuleEntry,
    NonRegularModuleEntry,
    ModuleAliasesManifest,
    InvalidDependencyId,
    InvalidDependencyVersion,
    DependencyVersionBuildMetadataForbidden,
    DuplicateDependency,
    ConflictingDependencyVersions,
    SelfDependency,
    MissingClaimsTable,
    DuplicateClaimMode,
    InvalidRuntimeClaim,
    DuplicateRuntimeClaim,
    InvalidAssetRoot,
    DuplicateAssetRoot,
    OverlappingAssetRoots,
    MissingAssetRoot,
    AssetRootAliasesControlFile,
    LegacyManifestRejected,
    CanonicalizationFailure,
    Scalar(PluginManifestErrorV1),
}

impl From<PluginManifestErrorV1> for PluginManifestErrorExtV1 {
    fn from(e: PluginManifestErrorV1) -> Self { Self::Scalar(e) }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PluginModuleWorldV1 {
    Plugin,
    ServerPlugin,
    AnimationPlugin,
}

impl PluginModuleWorldV1 {
    fn parse(s: &str) -> Result<Self, PluginManifestErrorExtV1> {
        match s {
            "plugin" => Ok(Self::Plugin),
            "server-plugin" => Ok(Self::ServerPlugin),
            "animation-plugin" => Ok(Self::AnimationPlugin),
            _ => Err(PluginManifestErrorExtV1::InvalidModuleWorld),
        }
    }

    fn tag(self) -> u64 {
        match self {
            Self::Plugin => 0,
            Self::ServerPlugin => 1,
            Self::AnimationPlugin => 2,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PluginClaimModeV1 {
    Server,
    Client,
    SinglePlayer,
}

impl PluginClaimModeV1 {
    fn parse(s: &str) -> Result<Self, PluginManifestErrorExtV1> {
        match s {
            "server" => Ok(Self::Server),
            "client" => Ok(Self::Client),
            "single-player" => Ok(Self::SinglePlayer),
            _ => Err(PluginManifestErrorExtV1::InvalidRuntimeClaim),
        }
    }

    fn tag(self) -> u64 {
        match self {
            Self::Server => 0,
            Self::Client => 1,
            Self::SinglePlayer => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginModuleDeclV1 {
    pub path: CanonicalPathV1,
    pub world: PluginModuleWorldV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDependencyDeclV1 {
    pub plugin_id: CanonicalPluginIdV1,
    pub version: PluginVersionV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModeRuntimeClaimsV1 {
    pub mode: PluginClaimModeV1,
    pub commands: Vec<String>,
    pub animations: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginClaimsV1 {
    pub asset_roots: Vec<CanonicalPathV1>,
    pub runtime: Vec<ModeRuntimeClaimsV1>,
}

/// Packet section 7. All vectors sorted + duplicate-free; consumers must
/// not re-sort under another policy.
#[derive(Clone, Debug)]
pub struct ValidatedPluginManifestV1 {
    pub manifest_version: u32,
    pub plugin_id: CanonicalPluginIdV1,
    pub display_name: Option<PluginDisplayNameV1>,
    pub plugin_version: PluginVersionV1,
    pub host_api: HostPluginApiRequirementV1,
    pub modules: Vec<PluginModuleDeclV1>,
    pub dependencies: Vec<PluginDependencyDeclV1>,
    pub claims: PluginClaimsV1,
    pub manifest_root: ProtocolDigestV1,
    pub archive_artifact: ArtifactIdentityV1,
    pub archive_semantic_root: ProtocolDigestV1,
    /// Stored in the result, NOT part of the plugin's semantic contract
    /// root (packet section 9).
    pub admission_policy_root: ProtocolDigestV1,
}

/// Packet section 7 — lossless legacy V0 observation: ordered raw arrays
/// (never sets — packet adversarial 12.4), unknown keys retained, no
/// derived authoritative identity.
#[derive(Clone, Debug)]
pub struct LegacyPluginManifestObservationV0 {
    pub raw_manifest_digest: ArtifactIdentityV1,
    pub name: Option<String>,
    pub modules_in_source_order: Vec<String>,
    pub dependencies_in_source_order: Vec<String>,
    pub unknown_top_level_keys: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum PluginManifestAdmissionV1 {
    ValidatedV1(Box<ValidatedPluginManifestV1>),
    ObservedLegacyV0(LegacyPluginManifestObservationV0),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifestV1 {
    #[allow(dead_code)]
    manifest_version: u32,
    plugin: RawPluginV1,
    modules: Vec<RawModuleV1>,
    dependencies: Vec<RawDependencyV1>,
    claims: RawClaimsV1,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPluginV1 {
    id: String,
    display_name: Option<String>,
    version: String,
    host_api: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModuleV1 {
    path: String,
    world: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDependencyV1 {
    id: String,
    version: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClaimsV1 {
    asset_roots: Vec<String>,
    runtime: Vec<RawRuntimeClaimV1>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRuntimeClaimV1 {
    mode: String,
    commands: Vec<String>,
    animations: Vec<String>,
}

fn classify_toml_error(e: impl std::fmt::Display) -> PluginManifestErrorExtV1 {
    let msg = e.to_string();
    if msg.contains("unknown field") {
        PluginManifestErrorExtV1::UnknownField { detail: msg }
    } else if msg.contains("missing field") {
        PluginManifestErrorExtV1::MissingRequiredField { detail: msg }
    } else {
        PluginManifestErrorExtV1::TomlDecode { detail: msg }
    }
}

fn archive_path(s: &str) -> Result<CanonicalPathV1, PluginManifestErrorExtV1> {
    if !s.bytes().all(portable_byte) {
        return Err(PluginManifestErrorExtV1::InvalidAssetRoot);
    }
    CanonicalPathV1::new(s).map_err(|_| PluginManifestErrorExtV1::InvalidAssetRoot)
}

fn runtime_key(s: &str, limits: &PluginManifestLimitsV1) -> Result<String, PluginManifestErrorExtV1> {
    if s.is_empty() || s.len() > limits.max_runtime_key_bytes as usize || !s.is_ascii() || s.chars().any(|c| c.is_control()) {
        return Err(PluginManifestErrorExtV1::InvalidRuntimeClaim);
    }
    Ok(s.to_owned())
}

/// T2.3's full validation pipeline over the landed T2.2 types (T2.3.17's
/// binding: the packet names the profiled-archive type
/// `ProfiledPluginArchiveV1`; the LANDED type is `StrictArchiveV1` +
/// namespace — landed code wins, name delta documented).
#[allow(clippy::too_many_arguments)]
pub fn validate_plugin_manifest_v1(
    manifest_bytes: &[u8],
    namespace: &[CanonicalEntryV1],
    archive_artifact: &ArtifactIdentityV1,
    archive_semantic_root: &ProtocolDigestV1,
    limits: &PluginManifestLimitsV1,
    mode: PluginManifestEnforcementModeV1,
    admission_policy_root: ProtocolDigestV1,
) -> Result<PluginManifestAdmissionV1, PluginManifestErrorExtV1> {
    if manifest_bytes.len() as u64 > limits.max_manifest_bytes {
        return Err(PluginManifestErrorExtV1::ManifestTooLarge);
    }
    let text = std::str::from_utf8(manifest_bytes).map_err(|_| PluginManifestErrorExtV1::ManifestUtf8)?;
    let raw_value: toml::Value =
        toml::from_str(text).map_err(|e| PluginManifestErrorExtV1::TomlDecode { detail: e.to_string() })?;

    // Version probe BEFORE typed decode (a malformed V1 never falls back
    // to legacy — packet 12.3; a legacy manifest never validates as V1).
    match probe_manifest_version(&raw_value)? {
        None => {
            if mode == PluginManifestEnforcementModeV1::StrictV1 {
                return Err(PluginManifestErrorExtV1::LegacyManifestRejected);
            }
            return Ok(PluginManifestAdmissionV1::ObservedLegacyV0(observe_legacy_v0(manifest_bytes, &raw_value)));
        },
        Some(_) => {},
    }

    let raw: RawManifestV1 = {
        use serde::Deserialize as _;
        RawManifestV1::deserialize(raw_value).map_err(classify_toml_error)?
    };

    let plugin_id = CanonicalPluginIdV1::parse(&raw.plugin.id, limits)?;
    let display_name = raw.plugin.display_name.as_deref().map(|d| PluginDisplayNameV1::parse(d, limits)).transpose()?;
    let plugin_version = PluginVersionV1::parse(&raw.plugin.version)?;
    let host_api = HostPluginApiRequirementV1::parse(&raw.plugin.host_api)?;

    // Modules (§5.5): resolve to exactly one regular archive entry,
    // unique, sorted by path bytes; world is identity-relevant.
    if raw.modules.len() as u32 > limits.max_module_count {
        return Err(PluginManifestErrorV1::LimitExceeded { what: "module count" }.into());
    }
    let exact: std::collections::BTreeSet<&str> = namespace.iter().map(|e| e.path.as_str()).collect();
    let mut modules = Vec::with_capacity(raw.modules.len());
    let mut module_paths = std::collections::BTreeSet::new();
    for m in &raw.modules {
        let path = archive_path(&m.path).map_err(|_| PluginManifestErrorExtV1::MissingModuleEntry)?;
        if path.as_str() == "plugin.toml" {
            return Err(PluginManifestErrorExtV1::ModuleAliasesManifest);
        }
        if !module_paths.insert(path.as_str().to_owned()) {
            return Err(PluginManifestErrorExtV1::DuplicateModulePath);
        }
        if !exact.contains(path.as_str()) {
            // V1 is exact-bytes: a case-fold near-miss is still MISSING
            // (the T2.2 namespace already forbids fold collisions, so
            // there is no ambiguity to alias through).
            return Err(PluginManifestErrorExtV1::MissingModuleEntry);
        }
        modules.push(PluginModuleDeclV1 { path, world: PluginModuleWorldV1::parse(&m.world)? });
    }
    modules.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));

    // Dependencies (§5.6): exact, sorted, no dups/conflicts/self.
    if raw.dependencies.len() as u32 > limits.max_dependency_count {
        return Err(PluginManifestErrorV1::LimitExceeded { what: "dependency count" }.into());
    }
    let mut dependencies = Vec::with_capacity(raw.dependencies.len());
    for d in &raw.dependencies {
        let dep_id = CanonicalPluginIdV1::parse(&d.id, limits)
            .map_err(|_| PluginManifestErrorExtV1::InvalidDependencyId)?;
        let version = match PluginVersionV1::parse(&d.version) {
            Ok(v) => v,
            Err(PluginManifestErrorV1::PluginVersionBuildMetadataForbidden) => {
                return Err(PluginManifestErrorExtV1::DependencyVersionBuildMetadataForbidden);
            },
            Err(_) => return Err(PluginManifestErrorExtV1::InvalidDependencyVersion),
        };
        if dep_id == plugin_id {
            return Err(PluginManifestErrorExtV1::SelfDependency);
        }
        dependencies.push(PluginDependencyDeclV1 { plugin_id: dep_id, version });
    }
    dependencies.sort_by(|a, b| {
        a.plugin_id.as_str().cmp(b.plugin_id.as_str()).then_with(|| a.version.get().cmp(b.version.get()))
    });
    for pair in dependencies.windows(2) {
        if pair[0].plugin_id == pair[1].plugin_id {
            return Err(if pair[0].version == pair[1].version {
                PluginManifestErrorExtV1::DuplicateDependency
            } else {
                PluginManifestErrorExtV1::ConflictingDependencyVersions
            });
        }
    }

    // Claims (§5.7): one record per mode, sorted lists, ceilings only.
    if raw.claims.runtime.len() as u8 > limits.max_runtime_claim_modes {
        return Err(PluginManifestErrorV1::LimitExceeded { what: "runtime claim modes" }.into());
    }
    let mut runtime = Vec::with_capacity(raw.claims.runtime.len());
    let mut seen_modes = std::collections::BTreeSet::new();
    for rc in &raw.claims.runtime {
        let mode_v = PluginClaimModeV1::parse(&rc.mode)?;
        if !seen_modes.insert(mode_v.tag()) {
            return Err(PluginManifestErrorExtV1::DuplicateClaimMode);
        }
        if rc.commands.len() as u32 > limits.max_command_claims_per_mode
            || rc.animations.len() as u32 > limits.max_animation_claims_per_mode
        {
            return Err(PluginManifestErrorV1::LimitExceeded { what: "claims per mode" }.into());
        }
        let mut commands =
            rc.commands.iter().map(|c| runtime_key(c, limits)).collect::<Result<Vec<_>, _>>()?;
        let mut animations =
            rc.animations.iter().map(|a| runtime_key(a, limits)).collect::<Result<Vec<_>, _>>()?;
        commands.sort_unstable();
        animations.sort_unstable();
        if commands.windows(2).any(|p| p[0] == p[1]) || animations.windows(2).any(|p| p[0] == p[1]) {
            return Err(PluginManifestErrorExtV1::DuplicateRuntimeClaim);
        }
        runtime.push(ModeRuntimeClaimsV1 { mode: mode_v, commands, animations });
    }
    runtime.sort_by_key(|r| r.mode.tag());

    // Asset roots (§5.8): global; canonical grammar; not the manifest;
    // not a module file; no dups; no ancestor/descendant overlap; must
    // match a file or directory prefix in the archive index.
    if raw.claims.asset_roots.len() as u32 > limits.max_asset_root_count {
        return Err(PluginManifestErrorV1::LimitExceeded { what: "asset root count" }.into());
    }
    let mut asset_roots = Vec::with_capacity(raw.claims.asset_roots.len());
    for r in &raw.claims.asset_roots {
        let root = archive_path(r)?;
        if root.as_str() == "plugin.toml" {
            return Err(PluginManifestErrorExtV1::AssetRootAliasesControlFile);
        }
        if module_paths.contains(root.as_str()) {
            return Err(PluginManifestErrorExtV1::InvalidAssetRoot);
        }
        let prefix = format!("{}/", root.as_str());
        let matches_any = namespace.iter().any(|e| e.path.as_str() == root.as_str() || e.path.as_str().starts_with(&prefix));
        if !matches_any {
            return Err(PluginManifestErrorExtV1::MissingAssetRoot);
        }
        asset_roots.push(root);
    }
    asset_roots.sort_by(|a, b| a.as_str().as_bytes().cmp(b.as_str().as_bytes()));
    for pair in asset_roots.windows(2) {
        if pair[0].as_str() == pair[1].as_str() {
            return Err(PluginManifestErrorExtV1::DuplicateAssetRoot);
        }
        if pair[1].as_str().starts_with(&format!("{}/", pair[0].as_str())) {
            return Err(PluginManifestErrorExtV1::OverlappingAssetRoots);
        }
    }

    let claims = PluginClaimsV1 { asset_roots, runtime };
    let manifest_root = manifest_semantic_root(&plugin_id, &plugin_version, &host_api, &modules, &dependencies, &claims)?;

    Ok(PluginManifestAdmissionV1::ValidatedV1(Box::new(ValidatedPluginManifestV1 {
        manifest_version: PLUGIN_MANIFEST_VERSION_V1,
        plugin_id,
        display_name,
        plugin_version,
        host_api,
        modules,
        dependencies,
        claims,
        manifest_root,
        archive_artifact: archive_artifact.clone(),
        archive_semantic_root: archive_semantic_root.clone(),
        admission_policy_root,
    })))
}

/// §5.9 — lossless legacy observation: ordered raw arrays, unknown keys
/// retained, no authoritative identity derived.
fn observe_legacy_v0(bytes: &[u8], raw: &toml::Value) -> LegacyPluginManifestObservationV0 {
    let table = raw.as_table();
    let get_arr = |key: &str| -> Vec<String> {
        table
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
            .unwrap_or_default()
    };
    let known = ["name", "modules", "dependencies"];
    LegacyPluginManifestObservationV0 {
        raw_manifest_digest: hash_artifact_bytes_v1(bytes),
        name: table.and_then(|t| t.get("name")).and_then(|v| v.as_str()).map(str::to_owned),
        modules_in_source_order: get_arr("modules"),
        dependencies_in_source_order: get_arr("dependencies"),
        unknown_top_level_keys: table
            .map(|t| t.keys().filter(|k| !known.contains(&k.as_str())).cloned().collect())
            .unwrap_or_default(),
    }
}

/// APEX-T2.4.05 hook: recompute a validated manifest's root for candidate
/// admission verification (the resolver must not trust the carried root).
pub fn recompute_manifest_root(v: &ValidatedPluginManifestV1) -> Result<common::apex::digest::ProtocolDigestV1, PluginManifestErrorExtV1> {
    manifest_semantic_root(&v.plugin_id, &v.plugin_version, &v.host_api, &v.modules, &v.dependencies, &v.claims)
}

/// §5.10 — the manifest semantic root under `PluginManifest` (= 8):
/// schema + identity scalars + sorted collections, T0.2-encoded. TOML
/// formatting/order can never move it (inputs are the canonicalized
/// typed projection, not source text).
fn manifest_semantic_root(
    plugin_id: &CanonicalPluginIdV1,
    version: &PluginVersionV1,
    host_api: &HostPluginApiRequirementV1,
    modules: &[PluginModuleDeclV1],
    dependencies: &[PluginDependencyDeclV1],
    claims: &PluginClaimsV1,
) -> Result<ProtocolDigestV1, PluginManifestErrorExtV1> {
    struct W(ManifestValueV1);
    impl ManifestEncodeV1 for W {
        fn to_manifest_value_v1(&self) -> Result<ManifestValueV1, common::apex::manifest::ManifestCodecErrorV1> {
            Ok(self.0.clone())
        }
    }
    let fail = |_| PluginManifestErrorExtV1::CanonicalizationFailure;
    let text = |s: &str| Ok::<_, PluginManifestErrorExtV1>(ManifestValueV1::MachineText(
        common::apex::manifest::MachineTextV1::new(s).map_err(fail)?,
    ));
    let map = |entries: Vec<(u16, ManifestValueV1)>| -> Result<ManifestValueV1, PluginManifestErrorExtV1> {
        let entries = entries.into_iter().map(|(id, v)| (FieldIdV1::new(id), v)).collect();
        Ok(ManifestValueV1::Map(CanonicalFieldMapV1::try_from_entries(entries).map_err(fail)?))
    };
    let modules_v: Vec<ManifestValueV1> = modules
        .iter()
        .map(|m| map(vec![(0, text(m.path.as_str())?), (1, ManifestValueV1::Unsigned(m.world.tag()))]))
        .collect::<Result<_, _>>()?;
    let deps_v: Vec<ManifestValueV1> = dependencies
        .iter()
        .map(|d| map(vec![(0, text(d.plugin_id.as_str())?), (1, text(&d.version.get().to_string())?)]))
        .collect::<Result<_, _>>()?;
    let runtime_v: Vec<ManifestValueV1> = claims
        .runtime
        .iter()
        .map(|r| {
            let cmds = r.commands.iter().map(|c| text(c)).collect::<Result<Vec<_>, _>>()?;
            let anims = r.animations.iter().map(|a| text(a)).collect::<Result<Vec<_>, _>>()?;
            map(vec![
                (0, ManifestValueV1::Unsigned(r.mode.tag())),
                (1, ManifestValueV1::Array(cmds)),
                (2, ManifestValueV1::Array(anims)),
            ])
        })
        .collect::<Result<_, _>>()?;
    let roots_v: Vec<ManifestValueV1> =
        claims.asset_roots.iter().map(|r| text(r.as_str())).collect::<Result<_, _>>()?;
    let top = map(vec![
        (0, text("bastion.plugin-manifest/v1")?),
        (1, text(plugin_id.as_str())?),
        (2, text(&version.get().to_string())?),
        (3, text(&format!("{}@{}", host_api.package.as_str(), host_api.version))?),
        (4, ManifestValueV1::Array(modules_v)),
        (5, ManifestValueV1::Array(deps_v)),
        (6, ManifestValueV1::Array(roots_v)),
        (7, ManifestValueV1::Array(runtime_v)),
    ])?;
    let limits = super::archive_profile::plugin_archive_limits_v1();
    digest_manifest_value_v1(DigestDomainIdV1::PluginManifest, &W(top), &limits)
        .map_err(|_| PluginManifestErrorExtV1::CanonicalizationFailure)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> PluginManifestLimitsV1 {
        PluginManifestLimitsV1 {
            policy_id: MachineTextV1::new("apex-t2-3-test-limits-v1").unwrap(),
            max_manifest_bytes: 1 << 14,
            max_plugin_id_bytes: 64,
            max_display_name_bytes: 64,
            max_module_count: 8,
            max_dependency_count: 8,
            max_runtime_claim_modes: 3,
            max_command_claims_per_mode: 8,
            max_animation_claims_per_mode: 8,
            max_asset_root_count: 4,
            max_runtime_key_bytes: 64,
        }
    }

    #[test]
    fn plugin_id_grammar() {
        let l = limits();
        for ok in ["example:hello", "a.b.c:name", "ex-1:na-2", "x:y", "digit0.d1:n9"] {
            assert!(CanonicalPluginIdV1::parse(ok, &l).is_ok(), "{ok}");
        }
        for bad in [
            "NoCaps:x", "under_score:x", "a:", ":b", "a::b", "a:b:c", "-lead:x", "trail-:x", "dou--ble:x",
            "a..b:x", "a.:x", "sp ace:x", "uni:é",
        ] {
            assert!(CanonicalPluginIdV1::parse(bad, &l).is_err(), "{bad} should fail");
        }
        assert!(matches!(
            CanonicalPluginIdV1::parse(&format!("{}:x", "a".repeat(100)), &l),
            Err(PluginManifestErrorV1::LimitExceeded { .. })
        ));
    }

    #[test]
    fn version_policy_rejects_build_metadata() {
        assert!(PluginVersionV1::parse("1.2.3").is_ok());
        assert!(PluginVersionV1::parse("1.2.3-alpha.1").is_ok(), "prerelease permitted");
        assert_eq!(
            PluginVersionV1::parse("1.2.3+build.5").unwrap_err(),
            PluginManifestErrorV1::PluginVersionBuildMetadataForbidden
        );
        assert_eq!(PluginVersionV1::parse("1.2").unwrap_err(), PluginManifestErrorV1::InvalidPluginVersion);
    }

    #[test]
    fn host_api_requirement() {
        let ok = HostPluginApiRequirementV1::parse("veloren:plugin@0.0.1").unwrap();
        assert_eq!(ok.version, semver::Version::new(0, 0, 1));
        assert_eq!(
            HostPluginApiRequirementV1::parse("other:plugin@0.0.1").unwrap_err(),
            PluginManifestErrorV1::UnsupportedHostPackage
        );
        assert!(HostPluginApiRequirementV1::parse("veloren:plugin").is_err());
        assert!(HostPluginApiRequirementV1::parse("veloren:plugin@1.0.0+meta").is_err());
    }

    use super::super::archive_profile::CanonicalEntryV1;
    use common::apex::manifest::CanonicalPathV1;

    fn ns(paths: &[&str]) -> Vec<CanonicalEntryV1> {
        paths
            .iter()
            .map(|p| CanonicalEntryV1 {
                path: CanonicalPathV1::new(*p).unwrap(),
                portability_key: MachineTextV1::new(p.to_ascii_lowercase()).unwrap(),
                size_bytes: 1,
                content_sha256: [1; 32],
            })
            .collect()
    }

    fn policy_root() -> common::apex::digest::ProtocolDigestV1 {
        common::apex::digest::digest_canonical_bytes_v1(
            common::apex::digest::DigestDomainIdV1::PluginManifest,
            b"test-admission-policy",
            1 << 20,
        )
        .unwrap()
    }

    fn validate(toml_src: &str, namespace: &[CanonicalEntryV1]) -> Result<PluginManifestAdmissionV1, PluginManifestErrorExtV1> {
        let art = common::apex::digest::hash_artifact_bytes_v1(b"archive");
        validate_plugin_manifest_v1(
            toml_src.as_bytes(),
            namespace,
            &art,
            &policy_root(),
            &limits(),
            PluginManifestEnforcementModeV1::StrictV1,
            policy_root(),
        )
    }

    const GOOD: &str = r#"
manifest_version = 1

[plugin]
id = "example:hello"
display_name = "Example Hello"
version = "0.1.0"
host_api = "veloren:plugin@0.0.1"

[[modules]]
path = "modules/hello.wasm"
world = "server-plugin"

[[dependencies]]
id = "example:shared"
version = "1.2.3"

[claims]
asset_roots = ["assets/example"]

[[claims.runtime]]
mode = "server"
commands = ["hello"]
animations = []
"#;

    fn good_ns() -> Vec<CanonicalEntryV1> {
        ns(&["plugin.toml", "modules/hello.wasm", "assets/example/thing.ron"])
    }

    #[test]
    fn packet_example_validates_and_root_is_format_independent() {
        let v = match validate(GOOD, &good_ns()).unwrap() {
            PluginManifestAdmissionV1::ValidatedV1(v) => v,
            other => panic!("{other:?}"),
        };
        assert_eq!(v.plugin_id.as_str(), "example:hello");
        assert_eq!(v.modules.len(), 1);
        assert_eq!(v.dependencies.len(), 1);

        // Reformatted/commented/reordered source, same content => same root.
        let reordered = GOOD.replace("display_name = \"Example Hello\"\n", "")
            .replace("id = \"example:hello\"", "id = \"example:hello\"\ndisplay_name = \"Example Hello\" # moved");
        let v2 = match validate(&reordered, &good_ns()).unwrap() {
            PluginManifestAdmissionV1::ValidatedV1(v) => v,
            other => panic!("{other:?}"),
        };
        assert_eq!(v.manifest_root, v2.manifest_root, "TOML formatting/order must not move the root");

        // Content change moves it.
        let bumped = GOOD.replace("version = \"0.1.0\"", "version = \"0.1.1\"");
        let v3 = match validate(&bumped, &good_ns()).unwrap() {
            PluginManifestAdmissionV1::ValidatedV1(v) => v,
            other => panic!("{other:?}"),
        };
        assert_ne!(v.manifest_root, v3.manifest_root);
    }

    #[test]
    fn validation_families_bite() {
        let e = |src: &str| validate(src, &good_ns()).unwrap_err();

        // A renamed table is BOTH unknown-and-missing — either typed
        // family is a correct strict rejection; a pure extra key drives
        // UnknownField specifically.
        assert!(matches!(
            e(&GOOD.replace("[claims]", "[claimz]")),
            PluginManifestErrorExtV1::UnknownField { .. } | PluginManifestErrorExtV1::MissingRequiredField { .. }
        ));
        assert!(matches!(
            e(&GOOD.replace("manifest_version = 1", "manifest_version = 1\nextra_key = true")),
            PluginManifestErrorExtV1::UnknownField { .. }
        ));
        assert!(matches!(
            e(&GOOD.replace("path = \"modules/hello.wasm\"", "path = \"gone.wasm\"")),
            PluginManifestErrorExtV1::MissingModuleEntry
        ));
        assert!(matches!(
            e(&GOOD.replace("world = \"server-plugin\"", "world = \"kernel\"")),
            PluginManifestErrorExtV1::InvalidModuleWorld
        ));
        assert!(matches!(
            e(&GOOD.replace("id = \"example:shared\"", "id = \"example:hello\"")),
            PluginManifestErrorExtV1::SelfDependency
        ));
        assert!(matches!(
            e(&GOOD.replace("version = \"1.2.3\"", "version = \"1.2.3+m\"")),
            PluginManifestErrorExtV1::DependencyVersionBuildMetadataForbidden
        ));
        assert!(matches!(
            e(&GOOD.replace("asset_roots = [\"assets/example\"]", "asset_roots = [\"assets/nope\"]")),
            PluginManifestErrorExtV1::MissingAssetRoot
        ));
        assert!(matches!(
            e(&GOOD.replace("asset_roots = [\"assets/example\"]", "asset_roots = [\"plugin.toml\"]")),
            PluginManifestErrorExtV1::AssetRootAliasesControlFile
        ));
        assert!(matches!(
            e(&GOOD.replace(
                "asset_roots = [\"assets/example\"]",
                "asset_roots = [\"assets\", \"assets/example\"]"
            )),
            PluginManifestErrorExtV1::OverlappingAssetRoots
        ));

        // Duplicate claim mode.
        let dup_mode = format!("{GOOD}\n[[claims.runtime]]\nmode = \"server\"\ncommands = []\nanimations = []\n");
        assert!(matches!(e(&dup_mode), PluginManifestErrorExtV1::DuplicateClaimMode));

        // Conflicting dependency versions.
        let conflict = format!("{GOOD}\n[[dependencies]]\nid = \"example:shared\"\nversion = \"9.9.9\"\n");
        assert!(matches!(e(&conflict), PluginManifestErrorExtV1::ConflictingDependencyVersions));

        // Legacy in strict mode.
        assert!(matches!(
            e("name = \"old\"\nmodules = []\n"),
            PluginManifestErrorExtV1::LegacyManifestRejected
        ));
        // Malformed V1 never falls back to legacy.
        assert!(matches!(
            e("manifest_version = 1\nname = \"looks-legacy\"\n"),
            PluginManifestErrorExtV1::UnknownField { .. } | PluginManifestErrorExtV1::MissingRequiredField { .. }
        ));
    }

    #[test]
    fn legacy_observation_is_lossless_and_ordered() {
        let art = common::apex::digest::hash_artifact_bytes_v1(b"archive");
        let src = "name = \"old\"\nmodules = [\"z.wasm\", \"a.wasm\", \"z.wasm\"]\nmystery = 1\ndependencies = []\n";
        let got = validate_plugin_manifest_v1(
            src.as_bytes(),
            &good_ns(),
            &art,
            &policy_root(),
            &limits(),
            PluginManifestEnforcementModeV1::ObserveLegacy,
            policy_root(),
        )
        .unwrap();
        match got {
            PluginManifestAdmissionV1::ObservedLegacyV0(o) => {
                assert_eq!(o.modules_in_source_order, vec!["z.wasm", "a.wasm", "z.wasm"], "source order + duplicates preserved");
                assert_eq!(o.unknown_top_level_keys, vec!["mystery"]);
                assert_eq!(o.name.as_deref(), Some("old"));
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn version_probe_dispatch() {
        let v1: toml::Value = toml::from_str("manifest_version = 1\n").unwrap();
        assert_eq!(probe_manifest_version(&v1).unwrap(), Some(1));
        let legacy: toml::Value = toml::from_str("name = \"p\"\n").unwrap();
        assert_eq!(probe_manifest_version(&legacy).unwrap(), None, "absent = legacy observation lane");
        let vx: toml::Value = toml::from_str("manifest_version = 2\n").unwrap();
        assert!(matches!(
            probe_manifest_version(&vx),
            Err(PluginManifestErrorV1::UnsupportedManifestVersion { got: 2 })
        ));
        let vs: toml::Value = toml::from_str("manifest_version = \"1\"\n").unwrap();
        assert_eq!(probe_manifest_version(&vs).unwrap_err(), PluginManifestErrorV1::InvalidManifestVersionType);
    }
}
