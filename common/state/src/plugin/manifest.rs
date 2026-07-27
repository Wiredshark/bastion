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
