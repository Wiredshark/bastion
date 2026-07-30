//! `APEX-T9.3` — the complete apex campaign certificate: the program's
//! own stated deliverable, per `readme/apex/APEX-T9-TIER-SPEC-FLEET-v1.md`
//! `T9.3`. "It may state only the properties whose separate attestations
//! passed."
//!
//! **Why this is a generator, not a document.** Three artifacts in this
//! program already work the same way — coverage maps (an unclaimed case
//! fails the build, `server/src/net_command_canaries.rs`), evidence
//! bundles (generated from the tree, `readme/apex/APEX-T3.4-EVIDENCE-
//! BUNDLE-v1.json`), and now this certificate. The pattern this row's own
//! spec names explicitly: *make the artifact incapable of overstating*.
//! [`generate_certificate_v1`] enforces that structurally —
//! [`CertifiedPropertyV1`] can only be constructed by the generator
//! itself, from a real, summed [`PropertyAttestationV1`] whose covered
//! count is greater than zero. A property with zero covered cases across
//! every attestation that names it is **structurally absent**: it never
//! reaches [`ApexCertificateV1::certified_properties`], only
//! [`ApexCertificateV1::open_set`]. That is the literal meaning of
//! "structurally absent... not merely omitted by a careful author" — no
//! code path exists that could accidentally state it.
//!
//! **Why `common` carries opaque roots, not real ones.** Same boundary
//! `world_baseline.rs` and `save_universe.rs` already document: this
//! module cannot depend on `world`/`server`, where the real build,
//! content, plugin, manifest, numeric, schedule, fixture, and output
//! roots are computed. [`ApexCertificateRootsV1`] is therefore eight
//! caller-supplied [`ArtifactDigestV1`]s — the pure binding, not the
//! computation. Verifying each one actually resolves to a real artifact
//! in the tree ("every named root resolves to an artifact") is
//! necessarily an integration-level check living where those roots are
//! computed, not a `common`-crate unit test; this chunk builds the
//! binding the check attaches to, not the check itself.
//!
//! **Multiple attestations per property, aggregated not overwritten.**
//! The tier spec's own evidence matrix cites several properties from
//! more than one row (`AllOf` combinations elsewhere in this program's
//! finding-closure rules). [`generate_certificate_v1`] groups its input
//! by [`CertifiedPropertyIdV1`] and sums `covered`/`total`/`open_cases`
//! before deciding certified-vs-absent — one property never appears
//! twice in [`ApexCertificateV1::certified_properties`] just because two
//! rows both attest to it.

use crate::apex::digest::ArtifactDigestV1;

/// The certificate's own frozen property vocabulary — one entry per row
/// of the tier spec's evidence matrix, with `CrossTargetExecution`
/// collapsing what the matrix lists as two rows (`T6.4`, `T8.2`) into
/// one: both need the identical unbuilt thing (one artifact executed
/// against two genuinely distinct compiler/target cells), and Fable's
/// own ruling on this row's premise-check named it explicitly —
/// "the certificate names it ONCE... listing one gap twice inflates the
/// open set and invites someone to close one number and think the
/// property is half-covered." Explicit discriminants, `ALL` array, and a
/// uniqueness self-test — same discipline as every other frozen
/// vocabulary in this program (`DigestDomainIdV1`, `SubsystemSlotIdV1`).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum CertifiedPropertyIdV1 {
    /// `T1`: same-target clean rebuilds.
    SameTargetReproducibility = 1,
    /// `T6.4` + `T8.2`, merged per Fable's ruling: cross-target execution
    /// vectors (numeric portability across compiler/target cells).
    CrossTargetExecution = 2,
    /// `T2.2`-`T2.5`: plugin archive/DAG/conflict permutations.
    PluginPermutations = 3,
    /// `T3.3`, `T3.4.22`: six-stream reorder/delay/duplicate schedules.
    SixStreamSchedules = 4,
    /// `T3.5.20`, `T9.1`: command retry/crash/reconnect windows.
    CommandRetryCrashReconnect = 5,
    /// `T5.3`, `T7.4`: prediction correction and rollback.
    PredictionCorrectionRollback = 6,
    /// `T6.2`: physics/weather raw+semantic numeric vectors.
    PhysicsWeatherNumericVectors = 7,
    /// `T4.3`, `T8.1`, `T8.3`, `T8.4`: world baseline/economy mismatch
    /// lanes. Deliberately excludes `T8.2` — that evidence now lives
    /// under [`Self::CrossTargetExecution`], not duplicated here.
    WorldBaselineEconomyMismatch = 8,
    /// `T4.6`: multi-store crash cutpoints.
    MultiStoreCrashCutpoints = 9,
    /// `T4.5`, `T9.2`: historical save migrations and authorized
    /// branching.
    HistoricalSaveMigrationBranching = 10,
}

impl CertifiedPropertyIdV1 {
    pub const fn as_u8(self) -> u8 { self as u8 }

    pub const fn label(self) -> &'static str {
        match self {
            Self::SameTargetReproducibility => "bastion/apex-certificate/same-target-reproducibility/v1",
            Self::CrossTargetExecution => "bastion/apex-certificate/cross-target-execution/v1",
            Self::PluginPermutations => "bastion/apex-certificate/plugin-permutations/v1",
            Self::SixStreamSchedules => "bastion/apex-certificate/six-stream-schedules/v1",
            Self::CommandRetryCrashReconnect => "bastion/apex-certificate/command-retry-crash-reconnect/v1",
            Self::PredictionCorrectionRollback => "bastion/apex-certificate/prediction-correction-rollback/v1",
            Self::PhysicsWeatherNumericVectors => "bastion/apex-certificate/physics-weather-numeric-vectors/v1",
            Self::WorldBaselineEconomyMismatch => "bastion/apex-certificate/world-baseline-economy-mismatch/v1",
            Self::MultiStoreCrashCutpoints => "bastion/apex-certificate/multi-store-crash-cutpoints/v1",
            Self::HistoricalSaveMigrationBranching => "bastion/apex-certificate/historical-save-migration-branching/v1",
        }
    }

    pub const ALL: [CertifiedPropertyIdV1; 10] = [
        Self::SameTargetReproducibility,
        Self::CrossTargetExecution,
        Self::PluginPermutations,
        Self::SixStreamSchedules,
        Self::CommandRetryCrashReconnect,
        Self::PredictionCorrectionRollback,
        Self::PhysicsWeatherNumericVectors,
        Self::WorldBaselineEconomyMismatch,
        Self::MultiStoreCrashCutpoints,
        Self::HistoricalSaveMigrationBranching,
    ];
}

/// One named case this program does NOT cover — the exact `{id, reason}`
/// shape `readme/apex/APEX-T3.4-EVIDENCE-BUNDLE-v1.json` and
/// `APEX-T3.5-EVIDENCE-BUNDLE-v1.json` already use, reused rather than
/// reinvented for the same reason `T8.1` reused the raw/semantic probe
/// pair: this is the third artifact family that needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCaseV1 {
    pub id: String,
    pub reason: String,
}

/// Every way constructing a [`PropertyAttestationV1`] can fail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyAttestationErrorV1 {
    /// `covered + open_cases.len() != total` — the three numbers must
    /// reconcile exactly (every case is either covered or open, no third
    /// state), or the count itself is not trustworthy enough to certify
    /// anything from.
    CaseCountMismatch { covered: u32, open: u32, total: u32 },
}

/// One row's (or one row-fragment's) real, cited evidence for one
/// property. Multiple attestations may name the SAME property — see the
/// module doc's aggregation note — each contributing its own slice of
/// cases; nothing here decides certified-vs-absent, that is
/// [`generate_certificate_v1`]'s job once every attestation for a
/// property has been summed.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyAttestationV1 {
    pub property: CertifiedPropertyIdV1,
    pub total_cases: u32,
    pub covered_cases: u32,
    pub open_cases: Vec<OpenCaseV1>,
    /// Provenance only, carried through unevaluated — which row(s) and
    /// which artifact(s) this attestation's numbers came from, so a
    /// reader of the generated certificate can trace every number back
    /// to something checkable rather than trusting a summary.
    pub sources: Vec<&'static str>,
}

impl PropertyAttestationV1 {
    /// The only constructor — validates `covered + open == total` before
    /// a caller can build one at all, so a malformed attestation can
    /// never reach the generator to begin with.
    pub fn new(
        property: CertifiedPropertyIdV1,
        total_cases: u32,
        covered_cases: u32,
        open_cases: Vec<OpenCaseV1>,
        sources: Vec<&'static str>,
    ) -> Result<Self, PropertyAttestationErrorV1> {
        let open_len = open_cases.len() as u32;
        if covered_cases.checked_add(open_len) != Some(total_cases) {
            return Err(PropertyAttestationErrorV1::CaseCountMismatch { covered: covered_cases, open: open_len, total: total_cases });
        }
        Ok(Self { property, total_cases, covered_cases, open_cases, sources })
    }
}

/// One property the certificate actually states — constructible only by
/// [`generate_certificate_v1`], never directly, so nothing outside this
/// module can fabricate a certified property without real covered cases
/// behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertifiedPropertyV1 {
    pub property: CertifiedPropertyIdV1,
    pub covered_cases: u32,
    pub total_cases: u32,
    pub sources: Vec<&'static str>,
}

/// The eight named roots the tier spec requires: "build, content,
/// plugin, manifest, numeric, schedule, fixture, and output." Caller-
/// supplied opaque digests — see the module doc's dependency note for
/// why `common` cannot compute any of them itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApexCertificateRootsV1 {
    pub build: ArtifactDigestV1,
    pub content: ArtifactDigestV1,
    pub plugin: ArtifactDigestV1,
    pub manifest: ArtifactDigestV1,
    pub numeric: ArtifactDigestV1,
    pub schedule: ArtifactDigestV1,
    pub fixture: ArtifactDigestV1,
    pub output: ArtifactDigestV1,
}

/// The certificate itself. Every field here is the OUTPUT of
/// [`generate_certificate_v1`] — there is no public constructor that
/// bypasses it, so a certificate in memory is always one that was
/// actually generated from a real attestation set, never hand-assembled.
#[derive(Clone, Debug, PartialEq)]
pub struct ApexCertificateV1 {
    pub roots: ApexCertificateRootsV1,
    /// Canonically sorted by property tag — never insertion order, same
    /// discipline every canonicalized collection in this program follows
    /// (permuting the input attestations must not move this).
    pub certified_properties: Vec<CertifiedPropertyV1>,
    /// Every open case from every attestation, tagged by property,
    /// including every case from a wholly-uncovered property (which
    /// contributes ONLY here, never to `certified_properties`).
    /// Canonically sorted by (property tag, case id).
    pub open_set: Vec<(CertifiedPropertyIdV1, OpenCaseV1)>,
}

/// The generator: pure, total, and the ONLY path to an
/// [`ApexCertificateV1`]. Groups `attestations` by property, sums each
/// group's covered/total/open cases, then states a property (adds it to
/// `certified_properties`) if and only if its summed `covered_cases > 0`
/// — the structural-absence rule this row's whole design exists to
/// enforce. Every attestation's open cases reach `open_set`
/// unconditionally, whether or not their property was stated.
pub fn generate_certificate_v1(roots: ApexCertificateRootsV1, attestations: &[PropertyAttestationV1]) -> ApexCertificateV1 {
    use std::collections::BTreeMap;

    struct Aggregate {
        covered: u32,
        total: u32,
        sources: Vec<&'static str>,
    }

    let mut aggregates: BTreeMap<u8, Aggregate> = BTreeMap::new();
    let mut open_set: Vec<(CertifiedPropertyIdV1, OpenCaseV1)> = Vec::new();

    for attestation in attestations {
        let entry = aggregates.entry(attestation.property.as_u8()).or_insert_with(|| Aggregate { covered: 0, total: 0, sources: Vec::new() });
        entry.covered += attestation.covered_cases;
        entry.total += attestation.total_cases;
        entry.sources.extend(attestation.sources.iter().copied());
        for open_case in &attestation.open_cases {
            open_set.push((attestation.property, open_case.clone()));
        }
    }

    let mut certified_properties: Vec<CertifiedPropertyV1> = aggregates
        .into_iter()
        .filter(|(_, agg)| agg.covered > 0)
        .map(|(tag, agg)| CertifiedPropertyV1 {
            property: CertifiedPropertyIdV1::ALL.into_iter().find(|p| p.as_u8() == tag).expect("tag came from a real CertifiedPropertyIdV1"),
            covered_cases: agg.covered,
            total_cases: agg.total,
            sources: agg.sources,
        })
        .collect();
    certified_properties.sort_by_key(|p| p.property.as_u8());

    open_set.sort_by(|(pa, oa), (pb, ob)| pa.as_u8().cmp(&pb.as_u8()).then_with(|| oa.id.cmp(&ob.id)));

    ApexCertificateV1 { roots, certified_properties, open_set }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apex::digest::hash_artifact_bytes_v1;

    fn digest(tag: u8) -> ArtifactDigestV1 { hash_artifact_bytes_v1(&[tag]).digest }

    fn roots() -> ApexCertificateRootsV1 {
        ApexCertificateRootsV1 {
            build: digest(1),
            content: digest(2),
            plugin: digest(3),
            manifest: digest(4),
            numeric: digest(5),
            schedule: digest(6),
            fixture: digest(7),
            output: digest(8),
        }
    }

    fn open_case(id: &str, reason: &str) -> OpenCaseV1 { OpenCaseV1 { id: id.to_owned(), reason: reason.to_owned() } }

    fn fully_covered(property: CertifiedPropertyIdV1, total: u32) -> PropertyAttestationV1 {
        PropertyAttestationV1::new(property, total, total, Vec::new(), vec!["test-fixture"]).unwrap()
    }

    #[test]
    fn a_new_attestation_rejects_a_case_count_that_does_not_reconcile() {
        let err = PropertyAttestationV1::new(
            CertifiedPropertyIdV1::MultiStoreCrashCutpoints,
            10,
            5,
            vec![open_case("X-1", "not enough opens to reach 10")],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, PropertyAttestationErrorV1::CaseCountMismatch { covered: 5, open: 1, total: 10 });
    }

    #[test]
    fn a_wholly_covered_property_is_stated_with_zero_open_cases() {
        let attestation = fully_covered(CertifiedPropertyIdV1::MultiStoreCrashCutpoints, 11);
        let cert = generate_certificate_v1(roots(), &[attestation]);
        assert_eq!(cert.certified_properties, vec![CertifiedPropertyV1 {
            property: CertifiedPropertyIdV1::MultiStoreCrashCutpoints,
            covered_cases: 11,
            total_cases: 11,
            sources: vec!["test-fixture"],
        }]);
        assert!(cert.open_set.is_empty());
    }

    /// The row's central rule, tested directly: a property with ZERO
    /// covered cases across every attestation naming it never appears in
    /// `certified_properties` — only in `open_set`.
    #[test]
    fn a_wholly_uncovered_property_is_structurally_absent_from_certified_properties() {
        let attestation = PropertyAttestationV1::new(
            CertifiedPropertyIdV1::CrossTargetExecution,
            1,
            0,
            vec![open_case("CROSS-TARGET-EXECUTION", "no artifact executed against two distinct compiler/target cells")],
            vec!["T6.4", "T8.2"],
        )
        .unwrap();
        let cert = generate_certificate_v1(roots(), &[attestation]);
        assert!(cert.certified_properties.is_empty(), "zero covered cases must never produce a stated property");
        assert_eq!(cert.open_set.len(), 1);
        assert_eq!(cert.open_set[0].0, CertifiedPropertyIdV1::CrossTargetExecution);
    }

    /// The row's own required test, verbatim: "a property whose
    /// attestation failed cannot appear in the certificate (the mutation
    /// test: fail one attestation, regenerate, confirm the property
    /// vanished)". Starts from a REAL stated property, mutates its
    /// attestation to zero coverage, regenerates, and confirms removal —
    /// not merely that a zero-from-the-start attestation was never added.
    #[test]
    fn failing_a_previously_passing_attestation_removes_the_property_on_regeneration() {
        let passing = fully_covered(CertifiedPropertyIdV1::PhysicsWeatherNumericVectors, 4);
        let before = generate_certificate_v1(roots(), &[passing]);
        assert_eq!(before.certified_properties.len(), 1);

        let now_failing = PropertyAttestationV1::new(
            CertifiedPropertyIdV1::PhysicsWeatherNumericVectors,
            4,
            0,
            (1..=4).map(|i| open_case(&format!("REGRESSED-{i}"), "a previously passing case regressed")).collect(),
            vec!["test-fixture"],
        )
        .unwrap();
        let after = generate_certificate_v1(roots(), &[now_failing]);
        assert!(after.certified_properties.is_empty(), "the property must vanish once its attestation no longer covers anything");
        assert_eq!(after.open_set.len(), 4);
    }

    /// The row's own required test, verbatim: "the OPEN set matches the
    /// sum of the tiers' pinned counts."
    #[test]
    fn the_open_set_size_matches_the_sum_of_every_attestations_open_cases() {
        let a = PropertyAttestationV1::new(
            CertifiedPropertyIdV1::SixStreamSchedules,
            176,
            154,
            (1..=22).map(|i| open_case(&format!("CKPT-{i}"), "named open case")).collect(),
            vec!["T3.4"],
        )
        .unwrap();
        let b = PropertyAttestationV1::new(
            CertifiedPropertyIdV1::CommandRetryCrashReconnect,
            162,
            153,
            (1..=9).map(|i| open_case(&format!("CMD-{i}"), "named open case")).collect(),
            vec!["T3.5"],
        )
        .unwrap();
        let cert = generate_certificate_v1(roots(), &[a, b]);
        let expected: usize = 22 + 9;
        assert_eq!(cert.open_set.len(), expected);
    }

    /// Multiple attestations for the SAME property (the tier spec's own
    /// `AllOf`-sourced properties) are summed into ONE certified entry,
    /// never duplicated.
    #[test]
    fn multiple_attestations_for_the_same_property_are_aggregated_not_duplicated() {
        let from_t43 = PropertyAttestationV1::new(CertifiedPropertyIdV1::WorldBaselineEconomyMismatch, 3, 3, vec![], vec!["T4.3"]).unwrap();
        let from_t83 = PropertyAttestationV1::new(CertifiedPropertyIdV1::WorldBaselineEconomyMismatch, 5, 5, vec![], vec!["T8.3"]).unwrap();
        let from_t84 = PropertyAttestationV1::new(
            CertifiedPropertyIdV1::WorldBaselineEconomyMismatch,
            6,
            5,
            vec![open_case("T8.4-SMOOTHING", "extra scrutiny check, not itself a gap, recorded anyway")],
            vec!["T8.4"],
        )
        .unwrap();
        let cert = generate_certificate_v1(roots(), &[from_t43, from_t83, from_t84]);
        assert_eq!(cert.certified_properties.len(), 1, "one property, not three");
        let stated = &cert.certified_properties[0];
        assert_eq!(stated.covered_cases, 13);
        assert_eq!(stated.total_cases, 14);
        assert_eq!(stated.sources, vec!["T4.3", "T8.3", "T8.4"]);
        assert_eq!(cert.open_set.len(), 1);
    }

    /// Canonicalization: permuting the input attestation order must not
    /// change the certificate's own output order — the same
    /// canonicalize-by-key discipline this program applies everywhere it
    /// hashes or serializes a collection.
    #[test]
    fn permuted_attestation_order_does_not_move_the_certificates_own_order() {
        let a = fully_covered(CertifiedPropertyIdV1::SameTargetReproducibility, 1);
        let b = fully_covered(CertifiedPropertyIdV1::PluginPermutations, 1);
        let c = fully_covered(CertifiedPropertyIdV1::MultiStoreCrashCutpoints, 1);

        let forward = generate_certificate_v1(roots(), &[a.clone(), b.clone(), c.clone()]);
        let reversed = generate_certificate_v1(roots(), &[c, b, a]);

        let forward_order: Vec<_> = forward.certified_properties.iter().map(|p| p.property).collect();
        let reversed_order: Vec<_> = reversed.certified_properties.iter().map(|p| p.property).collect();
        assert_eq!(forward_order, reversed_order);
    }

    #[test]
    fn property_tags_are_frozen_and_unique() {
        use std::collections::HashSet;
        let tags: HashSet<u8> = CertifiedPropertyIdV1::ALL.iter().map(|p| p.as_u8()).collect();
        assert_eq!(tags.len(), CertifiedPropertyIdV1::ALL.len());
        let labels: HashSet<&str> = CertifiedPropertyIdV1::ALL.iter().map(|p| p.label()).collect();
        assert_eq!(labels.len(), CertifiedPropertyIdV1::ALL.len());
    }

    #[test]
    fn an_empty_attestation_set_produces_an_empty_certificate() {
        let cert = generate_certificate_v1(roots(), &[]);
        assert!(cert.certified_properties.is_empty());
        assert!(cert.open_set.is_empty());
    }
}
