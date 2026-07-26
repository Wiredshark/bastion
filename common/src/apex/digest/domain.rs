//! Sealed domain-separation registry (`APEX-T0.3`, packet section 5.4).
//!
//! Both the numeric domain ID and its registered ASCII label are bound
//! into every protocol-root preimage. The label is derived from this
//! sealed registry — callers cannot supply an arbitrary label string.

/// A registered digest-domain purpose. Future Merkle leaf/node purposes
/// register their own domain, owned by their schema; this module does not
/// expose a generic untyped Merkle API.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DigestDomainIdV1 {
    BootstrapManifest = 1,
    SaveUniverseManifest = 2,
    PluginActivationPlan = 3,
    WorldBaselineManifest = 4,
    BuildManifest = 5,
    ExecutionEvidence = 6,
    SemanticContent = 7,
    /// Registered ahead of `APEX-T2.3` at Opus 5's flag: `PluginManifestV1`'s
    /// semantic root is a distinct object from `PluginActivationPlan` (ID 3,
    /// `APEX-T2.5`'s resolved *plan*) and must not reuse that domain.
    PluginManifest = 8,
}

impl DigestDomainIdV1 {
    pub const fn as_u16(self) -> u16 { self as u16 }

    /// The frozen ASCII label bound into the preimage alongside the
    /// numeric ID (packet section 5.3: "Both numeric domain ID and
    /// registered ASCII label are included").
    pub const fn label(self) -> &'static str {
        match self {
            Self::BootstrapManifest => "bastion/bootstrap-manifest/v1",
            Self::SaveUniverseManifest => "bastion/save-universe-manifest/v1",
            Self::PluginActivationPlan => "bastion/plugin-activation-plan/v1",
            Self::WorldBaselineManifest => "bastion/world-baseline-manifest/v1",
            Self::BuildManifest => "bastion/build-manifest/v1",
            Self::ExecutionEvidence => "bastion/execution-evidence/v1",
            Self::SemanticContent => "bastion/semantic-content/v1",
            Self::PluginManifest => "bastion/plugin-manifest/v1",
        }
    }

    pub const ALL: [DigestDomainIdV1; 8] = [
        Self::BootstrapManifest,
        Self::SaveUniverseManifest,
        Self::PluginActivationPlan,
        Self::WorldBaselineManifest,
        Self::BuildManifest,
        Self::ExecutionEvidence,
        Self::SemanticContent,
        Self::PluginManifest,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn registry_ids_and_labels_are_unique() {
        let ids: HashSet<u16> = DigestDomainIdV1::ALL.iter().map(|d| d.as_u16()).collect();
        assert_eq!(ids.len(), DigestDomainIdV1::ALL.len(), "duplicate domain ID");
        let labels: HashSet<&str> = DigestDomainIdV1::ALL.iter().map(|d| d.label()).collect();
        assert_eq!(labels.len(), DigestDomainIdV1::ALL.len(), "duplicate domain label");
    }

    #[test]
    fn labels_are_ascii() {
        for d in DigestDomainIdV1::ALL {
            assert!(d.label().is_ascii(), "{:?} label must be ASCII", d);
        }
    }

    #[test]
    fn exact_registered_table() {
        assert_eq!(DigestDomainIdV1::BootstrapManifest.as_u16(), 1);
        assert_eq!(DigestDomainIdV1::BootstrapManifest.label(), "bastion/bootstrap-manifest/v1");
        assert_eq!(DigestDomainIdV1::SaveUniverseManifest.as_u16(), 2);
        assert_eq!(DigestDomainIdV1::SaveUniverseManifest.label(), "bastion/save-universe-manifest/v1");
        assert_eq!(DigestDomainIdV1::PluginActivationPlan.as_u16(), 3);
        assert_eq!(DigestDomainIdV1::PluginActivationPlan.label(), "bastion/plugin-activation-plan/v1");
        assert_eq!(DigestDomainIdV1::WorldBaselineManifest.as_u16(), 4);
        assert_eq!(DigestDomainIdV1::WorldBaselineManifest.label(), "bastion/world-baseline-manifest/v1");
        assert_eq!(DigestDomainIdV1::BuildManifest.as_u16(), 5);
        assert_eq!(DigestDomainIdV1::BuildManifest.label(), "bastion/build-manifest/v1");
        assert_eq!(DigestDomainIdV1::ExecutionEvidence.as_u16(), 6);
        assert_eq!(DigestDomainIdV1::ExecutionEvidence.label(), "bastion/execution-evidence/v1");
        assert_eq!(DigestDomainIdV1::SemanticContent.as_u16(), 7);
        assert_eq!(DigestDomainIdV1::SemanticContent.label(), "bastion/semantic-content/v1");
        assert_eq!(DigestDomainIdV1::PluginManifest.as_u16(), 8);
        assert_eq!(DigestDomainIdV1::PluginManifest.label(), "bastion/plugin-manifest/v1");
    }
}
