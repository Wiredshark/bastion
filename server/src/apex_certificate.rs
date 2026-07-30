//! `APEX-T9.3` — real, cited attestations for `common::apex::certificate`'s
//! generator. This module's whole job is to turn what already exists in
//! this tree into typed [`PropertyAttestationV1`] values; it computes
//! nothing new about determinism itself.
//!
//! **Regenerable, not re-transcribed.** Two of these attestations
//! ([`command_retry_crash_reconnect_v1`], [`six_stream_schedules_v1`])
//! read `crate::net_command_canaries::CASE_COVERAGE`/`OPEN_CASE_COUNT`
//! and `crate::net_checkpoint_canaries::CASE_COVERAGE`/`OPEN_CASE_COUNT`
//! DIRECTLY at call time, rather than hand-copying today's numbers into
//! new literals here. That distinction is load-bearing, found the hard
//! way during this row's own premise-check: the point-in-time JSON
//! evidence bundles (`readme/apex/APEX-T3.4-EVIDENCE-BUNDLE-v1.json`,
//! `...T3.5-EVIDENCE-BUNDLE-v1.json`) had already drifted from the live,
//! continuously-enforced coverage maps by the time this row read them —
//! T3.4's bundle claimed 22 open cases where the live map enforces 5,
//! T3.5's claimed 10 where the live map enforces 9. A certificate that
//! read the bundles would have UNDERSTATED this program's real, already-
//! landed coverage. Reading the live maps' own constants is what makes
//! this row's own "cannot drift from evidence" requirement actually
//! true, rather than merely re-committing the same drift one layer up.
//!
//! **Coarser-grained attestations, named as such.** `T3.4`/`T3.5` (and
//! `T2.2`-`T2.5`) already have formal, numbered per-case catalogs this
//! module can read case-by-case. `T1`, `T5.3`/`T7.4`, and `T6.2` do not —
//! no prior row built one. Their attestations here count real, passing
//! test functions as cases (a coarser unit than a numbered spec case),
//! with zero named open cases because none were found at this pass's
//! depth, not because none could exist. `sources` says so explicitly on
//! each; a finer per-case catalog for these three is real follow-on work
//! this chunk does not claim to have done.
//!
//! **`CrossTargetExecution` is the one property with zero covered
//! cases**, per Fable's own ruling merging `T6.4` and `T8.2` into a
//! single open item: one artifact executed against two genuinely
//! distinct compiler/target cells, unbuilt for either citing row.

use common::apex::certificate::{CertifiedPropertyIdV1, OpenCaseV1, PropertyAttestationV1};

fn open_case(id: &str, reason: &str) -> OpenCaseV1 { OpenCaseV1 { id: id.to_owned(), reason: reason.to_owned() } }

/// Splits a `(id, claim)` coverage-map entry into a covered/open
/// classification, stripping the `"OPEN: "` prefix into the reason text
/// rather than carrying it twice.
fn classify_case(id: &str, claim: &str) -> Result<(), OpenCaseV1> {
    match claim.strip_prefix("OPEN: ") {
        Some(reason) => Err(open_case(id, reason)),
        None => Ok(()),
    }
}

/// `T3.3`, `T3.4.22`: six-stream reorder/delay/duplicate schedules.
/// Derived live from `net_checkpoint_canaries::CASE_COVERAGE`.
pub fn six_stream_schedules_v1() -> PropertyAttestationV1 {
    let mut open_cases = Vec::new();
    let mut covered = 0u32;
    for &(id, claim) in crate::net_checkpoint_canaries::CASE_COVERAGE {
        match classify_case(id, claim) {
            Ok(()) => covered += 1,
            Err(open) => open_cases.push(open),
        }
    }
    let total = crate::net_checkpoint_canaries::CASE_COVERAGE.len() as u32;
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::SixStreamSchedules,
        total,
        covered,
        open_cases,
        vec!["T3.4.22 (server/src/net_checkpoint_canaries.rs, live coverage map)"],
    )
    .expect("net_checkpoint_canaries's own self-test already enforces covered+open==total")
}

/// `T3.5.20`, `T9.1`: command retry/crash/reconnect windows. `T3.5`'s own
/// 162-case catalog is derived live; `T9.1`'s own confirmed-absent
/// continuous-frame classification is folded in as a second, separate
/// one-case attestation for the SAME property rather than force-fit into
/// the 162-case numbering that belongs to a different row entirely.
pub fn command_retry_crash_reconnect_v1() -> Vec<PropertyAttestationV1> {
    let mut open_cases = Vec::new();
    let mut covered = 0u32;
    for &(id, claim) in crate::net_command_canaries::CASE_COVERAGE {
        match classify_case(id, claim) {
            Ok(()) => covered += 1,
            Err(open) => open_cases.push(open),
        }
    }
    let total = crate::net_command_canaries::CASE_COVERAGE.len() as u32;
    let from_t35 = PropertyAttestationV1::new(
        CertifiedPropertyIdV1::CommandRetryCrashReconnect,
        total,
        covered,
        open_cases,
        vec!["T3.5.20 (server/src/net_command_canaries.rs, live coverage map)"],
    )
    .expect("net_command_canaries's own self-test already enforces covered+open==total");

    let from_t91 = PropertyAttestationV1::new(
        CertifiedPropertyIdV1::CommandRetryCrashReconnect,
        1,
        0,
        vec![open_case(
            "T9.1-STEP2",
            "continuous-frame classification is genuinely absent -- SemanticStreamIdV1 names streams by role, nothing marks a stream as carrying continuous frames, so the replay rule has no SUBJECT and cannot be enforced or even stated against the present types (T9.1 premise-check, commit 3cec01a713, read-only, no code)",
        )],
        vec!["T9.1 premise-check (3cec01a713)"],
    )
    .expect("a single zero-covered open case always reconciles");

    vec![from_t35, from_t91]
}

/// `T2.2`-`T2.5`: plugin archive/DAG/conflict permutations. One
/// attestation per catalog file's own pinned, self-verifying total-
/// coverage test.
pub fn plugin_permutations_v1() -> Vec<PropertyAttestationV1> {
    vec![
        PropertyAttestationV1::new(
            CertifiedPropertyIdV1::PluginPermutations,
            70,
            70,
            vec![],
            vec!["T2.3 (common/state/tests/apex_t2_3_catalog.rs)"],
        )
        .unwrap(),
        PropertyAttestationV1::new(
            CertifiedPropertyIdV1::PluginPermutations,
            80,
            80,
            vec![],
            vec!["T2.4 (common/state/tests/apex_t2_4_catalog.rs)"],
        )
        .unwrap(),
        PropertyAttestationV1::new(
            CertifiedPropertyIdV1::PluginPermutations,
            120,
            119,
            vec![open_case(
                "EARLY-ASSET-ACCESS",
                "the global cache remains usable pre-install for the ungoverned legacy path; a governed-process early-access guard is .13+ enforcement, deferred not silently dropped",
            )],
            vec!["T2.5 (common/state/tests/apex_t2_5_catalog.rs)"],
        )
        .unwrap(),
    ]
}

/// `T4.3`, `T8.1`, `T8.3`, `T8.4`: world baseline/economy mismatch lanes.
/// Deliberately excludes `T8.2` -- that evidence lives under
/// [`cross_target_execution_v1`] instead, per Fable's merge ruling. Every
/// number here is this session's own closed work, cited by file.
pub fn world_baseline_economy_mismatch_v1() -> PropertyAttestationV1 {
    // T4.3a/b (world_baseline.rs): 13 tests. T8.1 (economy/context.rs):
    // 5 tests. T8.1+T8.3+T8.4 (economy/mod.rs, same file): 20 tests.
    // Every T8.3 order-sensitivity axis traced to null or dead-code-
    // today; every T8.4 swept field (price, stock/demand, surplus,
    // population, smoothing) bounded, none unbounded, no branch crossing
    // -- real closed findings, not gaps, so zero open cases.
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::WorldBaselineEconomyMismatch,
        38,
        38,
        vec![],
        vec![
            "T4.3a/b (common/src/apex/world_baseline.rs)",
            "T8.1 (world/src/site/economy/context.rs)",
            "T8.3+T8.4 (world/src/site/economy/mod.rs)",
        ],
    )
    .unwrap()
}

/// `T4.6`: multi-store crash cutpoints. The `SAVE-001..011` canary set;
/// all eleven covered (three -- `SAVE-008`/`009`/`011` -- via a proven
/// analogous property with the substitution reasoning recorded in
/// `server/src/save_universe.rs`'s own test-module header comment, the
/// same "structural: ..." claim class `T3.5`'s coverage map uses, not a
/// literal fabrication of an OS-level atomicity boundary that admits no
/// observable third state).
pub fn multi_store_crash_cutpoints_v1() -> PropertyAttestationV1 {
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::MultiStoreCrashCutpoints,
        11,
        11,
        vec![],
        vec!["T4.6 (server/src/save_universe.rs, SAVE-001..011)"],
    )
    .unwrap()
}

/// `T4.5`, `T9.2`: historical save migrations and authorized branching.
/// `T4.5`'s 14 tests plus `T9.2`'s 62 (this session), with `T9.2`'s own
/// two already-named, already-banked follow-ons carried through as real
/// open cases rather than silently dropped now that they feed a
/// certificate.
pub fn historical_save_migration_branching_v1() -> PropertyAttestationV1 {
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::HistoricalSaveMigrationBranching,
        78,
        76,
        vec![
            open_case(
                "T9.2-CLI-WIRING",
                "restore_branch_v1 is a real, tested, callable mechanism but is not yet reachable from a running server -- no server-cli TUI/argv subcommand exists (banked follow-on, same mechanism/live-trigger split T4.6 itself used)",
            ),
            open_case(
                "T9.2-STALE-CLIENT-WIRING",
                "decide_stale_branch_v1/StaleBranchRejectionV1 is a real, tested, standalone decision but is not wired into a live reconnect handler -- T9.1 has not yet built the refusal enum this would be the branch-aware arm of",
            ),
        ],
        vec![
            "T4.5 (server/src/save_migration.rs)",
            "T9.2 (common/src/apex/save_universe.rs, server/src/save_universe.rs, commit 0f0d440acc)",
        ],
    )
    .unwrap()
}

/// `T6.4` + `T8.2`, merged per Fable's ruling: cross-target execution
/// vectors. Zero covered cases -- `T6.4`'s own falsifier
/// (`the_golden_vectors_fail_on_a_perturbed_kernel`,
/// `common/src/apex/numeric_profile.rs`) proves the COMPARATOR is
/// correct, which is a claim about the verification mechanism, not about
/// portability across any real second compiler/target cell; no such
/// execution was found anywhere in the tree for either citing row.
pub fn cross_target_execution_v1() -> PropertyAttestationV1 {
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::CrossTargetExecution,
        1,
        0,
        vec![open_case(
            "CROSS-TARGET-EXECUTION",
            "no artifact has been executed against two genuinely distinct compiler/target cells; T6.4's NumericProfileV1/verify_golden_vectors_v1 mechanism is proven correct by a same-process perturbation falsifier only, and T8.2 (the economy-specific cross-target lane) has never run -- Fable-deferred as an environment question. One underlying gap, cited from both rows, not two.",
        )],
        vec!["T6.4 (common/src/apex/numeric_profile.rs)", "T8.2 (never run)"],
    )
    .unwrap()
}

/// `T1`: same-target clean rebuilds. `nix/apex/repro-canaries.nix`
/// (`T1.3.11`): the `stable` canary proves a byte-identical rebuild; the
/// `time`/`random`/`tmppath` canaries are adversarial negative controls
/// proving the comparator can actually SEE nondeterminism (a comparator
/// that can't distinguish `stable` from the other three would prove
/// nothing about either). All four counted: the positive claim is only
/// as good as the negative controls that back it.
pub fn same_target_reproducibility_v1() -> PropertyAttestationV1 {
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::SameTargetReproducibility,
        4,
        4,
        vec![],
        vec!["T1.3.11 (nix/apex/repro-canaries.nix)"],
    )
    .unwrap()
}

/// `T5.3`, `T7.4`: prediction correction and rollback. Coarser-grained
/// than `T3.4`/`T3.5` -- see the module doc. Counts real test functions:
/// `T5.3` (`InputReceiptV1`/`PlayerPredictionProbeV1`, 9+4 tests) and
/// `T7.4` (`common/systems/tests/reconciliation.rs`, 14 tests, items
/// A/B/C: correction accounting, predicted-effect dedup/retraction, and
/// run-twice determinism). Zero named open cases -- none found at this
/// pass's depth, not asserted absent.
pub fn prediction_correction_rollback_v1() -> PropertyAttestationV1 {
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::PredictionCorrectionRollback,
        27,
        27,
        vec![],
        vec![
            "T5.3 (common/src/apex/input_receipt.rs, common/net/src/msg/input_receipt_wire.rs) -- coarse pass, no per-case catalog built",
            "T7.4 (common/systems/tests/reconciliation.rs) -- coarse pass, no per-case catalog built",
        ],
    )
    .unwrap()
}

/// `T6.2`: physics/weather raw+semantic numeric vectors. Coarser-grained
/// than `T3.4`/`T3.5` -- see the module doc. Counts
/// `common/src/apex/numeric_probe.rs`'s 11 real tests for the raw/
/// semantic probe pair independently reused by `T5.3`, `T6.2`, and
/// `T8.1`. Zero named open cases -- none found at this pass's depth, not
/// asserted absent.
pub fn physics_weather_numeric_vectors_v1() -> PropertyAttestationV1 {
    PropertyAttestationV1::new(
        CertifiedPropertyIdV1::PhysicsWeatherNumericVectors,
        11,
        11,
        vec![],
        vec!["T6.2 (common/src/apex/numeric_probe.rs) -- coarse pass, no per-case catalog built"],
    )
    .unwrap()
}

/// Every real attestation this row could ground today. The certificate's
/// own generator (`common::apex::certificate::generate_certificate_v1`)
/// aggregates and decides certified-vs-absent from this set; nothing
/// here decides that itself.
pub fn all_attestations_v1() -> Vec<PropertyAttestationV1> {
    let mut all = vec![
        same_target_reproducibility_v1(),
        cross_target_execution_v1(),
        world_baseline_economy_mismatch_v1(),
        multi_store_crash_cutpoints_v1(),
        historical_save_migration_branching_v1(),
        six_stream_schedules_v1(),
        prediction_correction_rollback_v1(),
        physics_weather_numeric_vectors_v1(),
    ];
    all.extend(plugin_permutations_v1());
    all.extend(command_retry_crash_reconnect_v1());
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apex::certificate::{ApexCertificateRootsV1, generate_certificate_v1};
    use common::apex::digest::hash_artifact_bytes_v1;

    fn placeholder_roots() -> ApexCertificateRootsV1 {
        let d = hash_artifact_bytes_v1(b"placeholder").digest;
        ApexCertificateRootsV1 { build: d, content: d, plugin: d, manifest: d, numeric: d, schedule: d, fixture: d, output: d }
    }

    /// Every property named in the tier spec's evidence matrix has at
    /// least one real attestation feeding it -- no row was silently
    /// skipped.
    #[test]
    fn every_certified_property_has_at_least_one_attestation() {
        let attestations = all_attestations_v1();
        for property in CertifiedPropertyIdV1::ALL {
            assert!(
                attestations.iter().any(|a| a.property == property),
                "{property:?} has no attestation at all -- every row in the tier spec's matrix must be represented, even if only as a zero-covered open item"
            );
        }
    }

    /// `six_stream_schedules_v1`'s derived numbers agree with a direct,
    /// independent re-scan of the same live coverage map -- proves the
    /// derivation logic itself, not just that SOME number came out.
    #[test]
    fn six_stream_schedules_matches_a_direct_rescan_of_the_live_coverage_map() {
        let attestation = six_stream_schedules_v1();
        let direct_open = crate::net_checkpoint_canaries::CASE_COVERAGE.iter().filter(|(_, c)| c.starts_with("OPEN:")).count();
        assert_eq!(attestation.open_cases.len(), direct_open);
        assert_eq!(attestation.total_cases as usize, crate::net_checkpoint_canaries::CASE_COVERAGE.len());
    }

    #[test]
    fn command_retry_crash_reconnect_matches_a_direct_rescan_plus_the_t91_addendum() {
        let attestations = command_retry_crash_reconnect_v1();
        assert_eq!(attestations.len(), 2);
        let direct_open = crate::net_command_canaries::CASE_COVERAGE.iter().filter(|(_, c)| c.starts_with("OPEN:")).count();
        assert_eq!(attestations[0].open_cases.len(), direct_open);
        assert_eq!(attestations[1].open_cases.len(), 1);
        assert_eq!(attestations[1].open_cases[0].id, "T9.1-STEP2");
    }

    /// The certificate this session's real evidence produces: every
    /// property except `CrossTargetExecution` is stated, and
    /// `CrossTargetExecution` appears ONLY in the open set, exactly once
    /// (not twice, per Fable's merge ruling), with the merged reasoning
    /// naming both `T6.4` and `T8.2`.
    #[test]
    fn the_real_certificate_states_every_property_except_cross_target_execution() {
        let cert = generate_certificate_v1(placeholder_roots(), &all_attestations_v1());

        let stated: std::collections::HashSet<CertifiedPropertyIdV1> = cert.certified_properties.iter().map(|p| p.property).collect();
        for property in CertifiedPropertyIdV1::ALL {
            if property == CertifiedPropertyIdV1::CrossTargetExecution {
                assert!(!stated.contains(&property), "CrossTargetExecution has zero covered cases and must be structurally absent");
            } else {
                assert!(stated.contains(&property), "{property:?} has real covered cases and must be stated");
            }
        }

        let cross_target_opens: Vec<_> = cert.open_set.iter().filter(|(p, _)| *p == CertifiedPropertyIdV1::CrossTargetExecution).collect();
        assert_eq!(cross_target_opens.len(), 1, "T6.4 and T8.2 are one gap, named once, not twice");
        assert!(cross_target_opens[0].1.reason.contains("T6.4") || cross_target_opens[0].1.reason.to_lowercase().contains("numericprofilev1"));
    }

    /// The two `T9.2` follow-ons this session's own closing report named
    /// survive all the way into the certificate's open set -- a banked
    /// follow-on that quietly disappeared here would be exactly the
    /// silent-omission failure mode this row exists to prevent.
    #[test]
    fn the_t9_2_named_follow_ons_reach_the_open_set() {
        let cert = generate_certificate_v1(placeholder_roots(), &all_attestations_v1());
        let ids: Vec<&str> = cert
            .open_set
            .iter()
            .filter(|(p, _)| *p == CertifiedPropertyIdV1::HistoricalSaveMigrationBranching)
            .map(|(_, c)| c.id.as_str())
            .collect();
        assert!(ids.contains(&"T9.2-CLI-WIRING"));
        assert!(ids.contains(&"T9.2-STALE-CLIENT-WIRING"));
    }
}
