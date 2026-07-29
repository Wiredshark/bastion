//! Sealed domain-separation registry (`APEX-T0.3`, packet section 5.4).
//!
//! Both the numeric domain ID and its registered ASCII label are bound
//! into every protocol-root preimage. The label is derived from this
//! sealed registry — callers cannot supply an arbitrary label string.

/// A registered digest-domain purpose. Future Merkle leaf/node purposes
/// register their own domain, owned by their schema; this module does not
/// expose a generic untyped Merkle API.
#[repr(u16)]
/// ID ALLOCATION (standing rule): numbering is blocked per program.
/// <=20 pre-split shared history, frozen. 21-39 ENGINE. 40-99 APEX.
/// A new lane requests a block before its first allocation; two lanes
/// allocating from one range is how ids 12 and 21/22 collided.
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
    /// `APEX-T0.5`: `SubsystemDescriptorV1` content identity.
    SubsystemDescriptor = 9,
    /// `APEX-T0.5`: `CompatibilityProfileV1` content identity. Row-order
    /// allocation (fleet standing rule, 2026-07-27): `T0.5` precedes
    /// `T1.2` in the registry's `sequence_index`, so `T0.5` keeps `9`/`10`
    /// and `T1.2`'s `SourceClosure` domain is `11`.
    CompatibilityProfile = 10,
    /// `APEX-T3.3`: `NET_ENVELOPE_PROFILE_V1` frozen protocol-tag
    /// vocabulary content root (packet section 7.1/8, step `T3.3.01`).
    /// Row-order allocation (fleet standing rule): `T1.2` (registry
    /// `sequence_index=10`) precedes `T3.3` (`sequence_index=21`) and is
    /// building on a separate lane (`bastion/apex-t1t2`) that has almost
    /// certainly already claimed `11` for its own `SourceClosure` domain --
    /// proactively skipping to `12` here rather than colliding at merge
    /// time and losing the tiebreak anyway.
    /// APEX MERGE COLLISION RESOLUTION (Opus, both lanes joined): this
    /// was allocated 12 on the T3.3 lane while the T1.3 lane independently
    /// allocated 12 to `LocalReproSmoke`. The program's row-order rule
    /// (earlier row takes the lower number) keeps T1.3's claim, so T3.3's
    /// profile domain moves to the next free id. Safe to move: the net
    /// envelope's `profile_root` is COMPUTED at runtime and every consumer
    /// recomputes it (no literal digest is pinned anywhere for it), whereas
    /// T1.3/T1.4's evidence records literal roots derived under 12.
    NetEnvelopeProfile = 20,
    CheckpointStreamTranscript = 40,
    CheckpointGlobalTranscript = 41,
    CheckpointDescriptor = 42,
    /// `APEX-T3.5`: identity of one idempotent command.
    CommandDescriptor = 43,
    /// `T4-PV` (APEX lane, ids 44-46; 40-43 were the last allocated in
    /// this lane): the three protocol ROOTS `T4.3`'s world baseline
    /// compares. Allocated together because they widened together --
    /// three separate domains rather than one shared "protocol root"
    /// domain so a worldgen root can never be mistaken for a content
    /// root even if their bytes ever collided. That is the whole point
    /// of carrying the domain in `ProtocolDigestV1` beside the bytes.
    WorldgenProtocolRoot = 44,
    ContentProtocolRoot = 45,
    NumericProtocolRoot = 46,
    // IDs 9 (SubsystemDescriptor) and 10 (CompatibilityProfile) are ALLOCATED
    // to APEX-T0.5's in-flight build (row-order allocation rule, Fable-blessed;
    // collision with SourceClosure caught + resolved at spec stage). Sonnet 5's
    // T0.5 build adds those variants; do not reuse the numbers.
    /// `APEX-T1.2` (fleet-authored spec, Fable-approved): the source-closure
    /// record's roots (rust-source tree, asset tree, LFS report). A distinct
    /// domain from `BuildManifest` (ID 5): the closure is an INPUT that
    /// `APEX-T1.5`'s manifest embeds — separating inputs from the manifest
    /// that embeds them is the point of domain separation.
    SourceClosure = 11,
    /// `APEX-T1.3` (real packet, section 7): the local reproducibility
    /// smoke's canonical evidence record — same-worker exact-output
    /// rebuild + host-path impurity smoke. Distinct from `BuildManifest`
    /// (ID 5, T1.5's manifest) for the same input-vs-embedding reason as
    /// `SourceClosure`.
    LocalReproSmoke = 12,
    /// `APEX-T1.4` (real packet, section 8.1/8.6): the four fresh-rebuild
    /// evidence namespaces, row-order allocated 13-16 (T1.4 precedes
    /// T2.2, whose plugin-archive domain is therefore 17 — resolved in
    /// the T2.2 fleet spec section 4.8).
    FreshBuilderProfile = 13,
    FreshBuilderRun = 14,
    FreshRebuildPair = 15,
    FreshRebuildCanaryCampaign = 16,
    /// `APEX-T2.2` (fleet spec section 4.8, cross-review-resolved): the
    /// plugin archive's SEMANTIC root — content identity over the sorted
    /// regular-file namespace, an INPUT to both `PluginManifest` (8,
    /// T2.3) and `PluginActivationPlan` (3, T2.5), domain-separated from
    /// both for the SourceClosure-vs-BuildManifest reason.
    PluginArchive = 17,
    /// `APEX-T2.4` (real packet, section 9.1/9.5): the resolver's two
    /// evidence namespaces — the admitted candidate set and the resolved
    /// graph. The T2.5 activation-plan root remains domain 3.
    PluginCandidateSet = 18,
    PluginResolvedGraph = 19,
    /// `APEX-T1.114` (engine-list, ledger row): the authoritative replay
    /// bundle's own canonical root — distinct from `ExecutionEvidence` (6,
    /// the paired-run Verdict/FirstDivergence oracle's evidence) because a
    /// replay bundle is a REPLAY INPUT (self-contained enough to re-run a
    /// failure), not a comparison verdict.
    ReplayBundle = 21,
    /// `APEX-T1.107` (engine-list, Part 2 P2-G6): the failure-seed
    /// shrinking record's own canonical root. A distinct domain from
    /// `ReplayBundle` (21): a shrink record CITES seeds/signatures, it
    /// does not embed a replayable run.
    FailureSeedRecord = 22,
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
            Self::SubsystemDescriptor => "bastion/subsystem-descriptor/v1",
            Self::CompatibilityProfile => "bastion/compatibility-profile/v1",
            Self::NetEnvelopeProfile => "bastion/net-envelope-profile/v1",
            Self::CheckpointStreamTranscript => "bastion/checkpoint-stream/v1",
            Self::CheckpointGlobalTranscript => "bastion/checkpoint-global/v1",
            Self::CheckpointDescriptor => "bastion/checkpoint-descriptor/v1",
            Self::CommandDescriptor => "bastion/command-descriptor/v1",
            Self::WorldgenProtocolRoot => "bastion/worldgen-protocol-root/v1",
            Self::ContentProtocolRoot => "bastion/content-protocol-root/v1",
            Self::NumericProtocolRoot => "bastion/numeric-protocol-root/v1",
            Self::SourceClosure => "bastion/source-closure/v1",
            Self::LocalReproSmoke => "bastion/build/local-repro-smoke/v1",
            Self::FreshBuilderProfile => "bastion/fresh-builder-profile/v1",
            Self::FreshBuilderRun => "bastion/fresh-builder-run/v1",
            Self::FreshRebuildPair => "bastion/fresh-rebuild-pair/v1",
            Self::FreshRebuildCanaryCampaign => "bastion/fresh-rebuild-canary-campaign/v1",
            Self::PluginArchive => "bastion/plugin-archive/v1",
            Self::PluginCandidateSet => "bastion/plugin-candidate-set/v1",
            Self::PluginResolvedGraph => "bastion/plugin-resolved-graph/v1",
            Self::ReplayBundle => "bastion/replay-bundle/v1",
            Self::FailureSeedRecord => "bastion/failure-seed-record/v1",
        }
    }

    pub const ALL: [DigestDomainIdV1; 29] = [
        Self::BootstrapManifest,
        Self::SaveUniverseManifest,
        Self::PluginActivationPlan,
        Self::WorldBaselineManifest,
        Self::BuildManifest,
        Self::ExecutionEvidence,
        Self::SemanticContent,
        Self::PluginManifest,
        Self::SubsystemDescriptor,
        Self::CompatibilityProfile,
        Self::NetEnvelopeProfile,
        Self::SourceClosure,
        Self::LocalReproSmoke,
        Self::FreshBuilderProfile,
        Self::FreshBuilderRun,
        Self::FreshRebuildPair,
        Self::FreshRebuildCanaryCampaign,
        Self::PluginArchive,
        Self::PluginCandidateSet,
        Self::PluginResolvedGraph,
        // Engine lane (21-39), from bastion/engine2.
        Self::ReplayBundle,
        Self::FailureSeedRecord,
        // APEX lane (40-99).
        Self::CheckpointStreamTranscript,
        Self::CheckpointGlobalTranscript,
        Self::CheckpointDescriptor,
        Self::CommandDescriptor,
        Self::WorldgenProtocolRoot,
        Self::ContentProtocolRoot,
        Self::NumericProtocolRoot,
    ];
}

/// Which program lane a domain ID's numeric value falls in, per the frozen
/// ID ALLOCATION rule declared on [`DigestDomainIdV1`]: pre-split shared
/// history is `<=20` and frozen, `21-39` is the engine lane, `40-99` is the
/// APEX lane. Pure function of the numeric value so it can classify a lane
/// without touching the enum, and so the classification claim below is
/// directly falsifiable at the boundary values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainIdLaneV1 {
    Frozen,
    Engine,
    Apex,
    OutOfRange,
}

pub const fn domain_id_lane_v1(id: u16) -> DomainIdLaneV1 {
    match id {
        1..=20 => DomainIdLaneV1::Frozen,
        21..=39 => DomainIdLaneV1::Engine,
        40..=99 => DomainIdLaneV1::Apex,
        _ => DomainIdLaneV1::OutOfRange,
    }
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
        assert_eq!(DigestDomainIdV1::SubsystemDescriptor.as_u16(), 9);
        assert_eq!(DigestDomainIdV1::SubsystemDescriptor.label(), "bastion/subsystem-descriptor/v1");
        assert_eq!(DigestDomainIdV1::CompatibilityProfile.as_u16(), 10);
        assert_eq!(DigestDomainIdV1::CompatibilityProfile.label(), "bastion/compatibility-profile/v1");
        assert_eq!(DigestDomainIdV1::NetEnvelopeProfile.as_u16(), 20);
        assert_eq!(DigestDomainIdV1::NetEnvelopeProfile.label(), "bastion/net-envelope-profile/v1");
        assert_eq!(DigestDomainIdV1::SourceClosure.as_u16(), 11);
        assert_eq!(DigestDomainIdV1::SourceClosure.label(), "bastion/source-closure/v1");
        assert_eq!(DigestDomainIdV1::LocalReproSmoke.as_u16(), 12);
        assert_eq!(DigestDomainIdV1::LocalReproSmoke.label(), "bastion/build/local-repro-smoke/v1");
        assert_eq!(DigestDomainIdV1::FreshBuilderProfile.as_u16(), 13);
        assert_eq!(DigestDomainIdV1::FreshBuilderProfile.label(), "bastion/fresh-builder-profile/v1");
        assert_eq!(DigestDomainIdV1::FreshBuilderRun.as_u16(), 14);
        assert_eq!(DigestDomainIdV1::FreshBuilderRun.label(), "bastion/fresh-builder-run/v1");
        assert_eq!(DigestDomainIdV1::FreshRebuildPair.as_u16(), 15);
        assert_eq!(DigestDomainIdV1::FreshRebuildPair.label(), "bastion/fresh-rebuild-pair/v1");
        assert_eq!(DigestDomainIdV1::FreshRebuildCanaryCampaign.as_u16(), 16);
        assert_eq!(DigestDomainIdV1::FreshRebuildCanaryCampaign.label(), "bastion/fresh-rebuild-canary-campaign/v1");
        assert_eq!(DigestDomainIdV1::PluginArchive.as_u16(), 17);
        assert_eq!(DigestDomainIdV1::PluginArchive.label(), "bastion/plugin-archive/v1");
        assert_eq!(DigestDomainIdV1::PluginCandidateSet.as_u16(), 18);
        assert_eq!(DigestDomainIdV1::PluginCandidateSet.label(), "bastion/plugin-candidate-set/v1");
        assert_eq!(DigestDomainIdV1::PluginResolvedGraph.as_u16(), 19);
        assert_eq!(DigestDomainIdV1::PluginResolvedGraph.label(), "bastion/plugin-resolved-graph/v1");
        assert_eq!(DigestDomainIdV1::ReplayBundle.as_u16(), 21);
        assert_eq!(DigestDomainIdV1::ReplayBundle.label(), "bastion/replay-bundle/v1");
        assert_eq!(DigestDomainIdV1::FailureSeedRecord.as_u16(), 22);
        assert_eq!(DigestDomainIdV1::FailureSeedRecord.label(), "bastion/failure-seed-record/v1");
        assert_eq!(DigestDomainIdV1::CheckpointStreamTranscript.as_u16(), 40);
        assert_eq!(DigestDomainIdV1::CheckpointStreamTranscript.label(), "bastion/checkpoint-stream/v1");
        assert_eq!(DigestDomainIdV1::CheckpointGlobalTranscript.as_u16(), 41);
        assert_eq!(DigestDomainIdV1::CheckpointGlobalTranscript.label(), "bastion/checkpoint-global/v1");
        assert_eq!(DigestDomainIdV1::CheckpointDescriptor.as_u16(), 42);
        assert_eq!(DigestDomainIdV1::CheckpointDescriptor.label(), "bastion/checkpoint-descriptor/v1");
        assert_eq!(DigestDomainIdV1::CommandDescriptor.as_u16(), 43);
        assert_eq!(DigestDomainIdV1::CommandDescriptor.label(), "bastion/command-descriptor/v1");
    }

    /// `E11-2b`: every registered domain must land in the lane its own
    /// section comment (frozen / "Engine lane" / "APEX lane") claims. This
    /// is the classification pass the block-allocation rule needs: a new
    /// domain allocated in the wrong lane compiles fine and passes
    /// `registry_ids_and_labels_are_unique` (its ID is merely unique), so
    /// only an explicit lane check catches a boundary-crossing allocation
    /// before merge.
    #[test]
    fn every_registered_domain_lands_in_its_declared_lane() {
        use DigestDomainIdV1::*;

        let frozen = [
            BootstrapManifest, SaveUniverseManifest, PluginActivationPlan, WorldBaselineManifest,
            BuildManifest, ExecutionEvidence, SemanticContent, PluginManifest, SubsystemDescriptor,
            CompatibilityProfile, NetEnvelopeProfile, SourceClosure, LocalReproSmoke,
            FreshBuilderProfile, FreshBuilderRun, FreshRebuildPair, FreshRebuildCanaryCampaign,
            PluginArchive, PluginCandidateSet, PluginResolvedGraph,
        ];
        let engine = [ReplayBundle, FailureSeedRecord];
        let apex = [
            CheckpointStreamTranscript, CheckpointGlobalTranscript, CheckpointDescriptor,
            CommandDescriptor,
            WorldgenProtocolRoot,
            ContentProtocolRoot,
            NumericProtocolRoot,
        ];

        for d in frozen {
            assert_eq!(domain_id_lane_v1(d.as_u16()), DomainIdLaneV1::Frozen, "{:?} not frozen-lane", d);
        }
        for d in engine {
            assert_eq!(domain_id_lane_v1(d.as_u16()), DomainIdLaneV1::Engine, "{:?} not engine-lane", d);
        }
        for d in apex {
            assert_eq!(domain_id_lane_v1(d.as_u16()), DomainIdLaneV1::Apex, "{:?} not apex-lane", d);
        }

        // Exhaustiveness: the three lists above must exactly cover ALL, so
        // a newly-added domain forces this test to be updated rather than
        // silently going unclassified.
        let mut classified: Vec<u16> =
            frozen.iter().chain(engine.iter()).chain(apex.iter()).map(|d| d.as_u16()).collect();
        classified.sort_unstable();
        let mut registered: Vec<u16> = DigestDomainIdV1::ALL.iter().map(|d| d.as_u16()).collect();
        registered.sort_unstable();
        assert_eq!(classified, registered, "lane lists in this test are stale vs DigestDomainIdV1::ALL");
    }

    /// Falsifier / non-vacuity check for `domain_id_lane_v1`: probes every
    /// lane boundary directly, so the classification pass above can't be
    /// passing because the lane function is trivially permissive.
    #[test]
    fn domain_id_lane_v1_discriminates_at_every_boundary() {
        assert_eq!(domain_id_lane_v1(0), DomainIdLaneV1::OutOfRange);
        assert_eq!(domain_id_lane_v1(1), DomainIdLaneV1::Frozen);
        assert_eq!(domain_id_lane_v1(20), DomainIdLaneV1::Frozen);
        assert_eq!(domain_id_lane_v1(21), DomainIdLaneV1::Engine);
        assert_eq!(domain_id_lane_v1(39), DomainIdLaneV1::Engine);
        assert_eq!(domain_id_lane_v1(40), DomainIdLaneV1::Apex);
        assert_eq!(domain_id_lane_v1(99), DomainIdLaneV1::Apex);
        assert_eq!(domain_id_lane_v1(100), DomainIdLaneV1::OutOfRange);
    }
}
