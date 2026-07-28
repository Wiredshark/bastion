//! `APEX-T4.5` — historical save corpus and migration policy.
//!
//! Existing saves must not become collateral damage of manifest
//! adoption.
//!
//! **What this build actually has, established by reading it rather than
//! by assuming the row's shape.** There is no rtsim migration machinery
//! at all: `Data::from_reader` rejects any version that is not
//! `CURRENT_VERSION`, and `server/src/rtsim/mod.rs` responds by PURGING
//! and regenerating — unless the operator sets `RTSIM_IGNORE_VERSION`, in
//! which case the mismatched data is loaded unmigrated. The `.ron_backup`
//! rename that `T4.4` inventories is a different path entirely: it fires
//! on a DECODE failure, not on a version mismatch.
//!
//! So the honest migration graph for rtsim today is **empty**, and every
//! non-current rtsim version is `ExplicitRecoveryOnly` — an operator
//! action exists, and it is not automatic. Building an elaborate graph
//! over steps that do not exist would be theatre; what this row builds
//! instead is the ENGINE plus its law, declared now so that the first
//! real step is born already bound by it.
//!
//! SQLite is the opposite case: refinery applies ordered numbered steps
//! automatically, so its epochs are genuinely `Migratable`.
//!
//! **Deliberately not decided here.** The tombstone, alias, content and
//! world-resolution policies (the row's step 5) are judgement calls about
//! player data. The row says they must be declared *before* code, and a
//! builder should not be making them mid-implementation — so they are
//! carried as stated questions in [`RESOLUTION_POLICIES`], not answered.

use common::apex::digest::{ArtifactIdentityV1, hash_artifact_bytes_v1};

/// How this build treats a save at a given epoch.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SaveSupportV1 {
    /// Loads as-is.
    Supported,
    /// A declared, ordered step path reaches the current epoch and runs
    /// without operator action.
    Migratable,
    /// A path exists but only under an explicit operator action. Never
    /// automatic, and never silently.
    ExplicitRecoveryOnly,
    /// No path. Refuse rather than guess.
    Unsupported,
}

/// A store whose epochs this policy covers.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SaveStoreV1 {
    RtsimData,
    CharacterDb,
}

/// rtsim's support policy, derived from what the loader does.
///
/// Not `Unsupported`: `RTSIM_IGNORE_VERSION=1` loads a mismatched save
/// unmigrated, which is a real recovery path even though it is a blunt
/// one. Not `Migratable` either — nothing transforms the data; serde
/// defaults absorb the difference, which is a very different promise.
pub fn rtsim_support_v1(found_version: u32) -> SaveSupportV1 {
    if found_version == rtsim::data::CURRENT_VERSION {
        SaveSupportV1::Supported
    } else {
        SaveSupportV1::ExplicitRecoveryOnly
    }
}

/// The character db's support policy.
///
/// `applied` is the highest refinery version present in the save;
/// `latest` is the highest this build ships. A save AHEAD of the build is
/// `Unsupported`: refinery has no down-migrations, and guessing would
/// write to player data.
pub fn character_db_support_v1(applied: i32, latest: i32) -> SaveSupportV1 {
    match applied.cmp(&latest) {
        std::cmp::Ordering::Equal => SaveSupportV1::Supported,
        std::cmp::Ordering::Less => SaveSupportV1::Migratable,
        std::cmp::Ordering::Greater => SaveSupportV1::Unsupported,
    }
}

/// One ordered, pure migration step.
///
/// `apply` is a plain `fn`, not a closure: a step that captured state
/// would not be pure, and a migration graph whose steps depend on
/// anything but their input is not a graph, it is a sequence of events.
#[derive(Copy, Clone, Debug)]
pub struct MigrationStepV1 {
    pub store: SaveStoreV1,
    pub from: u32,
    pub to: u32,
    pub name: &'static str,
    pub apply: fn(&[u8]) -> Vec<u8>,
}

/// Why a migration could not be performed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationErrorV1 {
    /// No step leaves this epoch.
    NoPathFrom(u32),
    /// Steps exist but none reaches the target.
    NoPathTo { from: u32, to: u32 },
    /// A step's `to` does not advance. A non-advancing step would make
    /// stepwise application non-terminating, so it is refused at the
    /// graph rather than guarded at every call.
    NonAdvancingStep { name: &'static str, from: u32, to: u32 },
}

/// An ordered set of steps for one store.
#[derive(Debug)]
pub struct MigrationGraphV1 {
    store: SaveStoreV1,
    steps: Vec<MigrationStepV1>,
}

impl MigrationGraphV1 {
    /// Steps must all belong to `store` and must all advance.
    pub fn new_v1(
        store: SaveStoreV1,
        steps: Vec<MigrationStepV1>,
    ) -> Result<Self, MigrationErrorV1> {
        for step in &steps {
            if step.to <= step.from {
                return Err(MigrationErrorV1::NonAdvancingStep {
                    name: step.name,
                    from: step.from,
                    to: step.to,
                });
            }
        }
        Ok(Self { store, steps })
    }

    pub fn store_v1(&self) -> SaveStoreV1 { self.store }

    pub fn is_empty_v1(&self) -> bool { self.steps.is_empty() }

    /// The single-step edge from `from` to `to`, if one is declared.
    fn direct_step(&self, from: u32, to: u32) -> Option<&MigrationStepV1> {
        self.steps.iter().find(|s| s.from == from && s.to == to)
    }

    /// Apply the declared direct edge. `None` when none is declared —
    /// which is the common case and is not an error.
    pub fn apply_direct_v1(&self, from: u32, to: u32, value: &[u8]) -> Option<Vec<u8>> {
        self.direct_step(from, to).map(|step| (step.apply)(value))
    }

    /// Apply the shortest *advancing* path one step at a time.
    ///
    /// Ties are broken by the SMALLEST `to`, so the walk is a pure
    /// function of the graph rather than of the order steps were
    /// declared in. Two builds that list the same steps differently must
    /// migrate identically or the policy is not a policy.
    pub fn apply_stepwise_v1(
        &self,
        from: u32,
        to: u32,
        value: &[u8],
    ) -> Result<Vec<u8>, MigrationErrorV1> {
        let mut current = from;
        let mut payload = value.to_vec();
        while current != to {
            let mut candidates: Vec<&MigrationStepV1> = self
                .steps
                .iter()
                .filter(|s| s.from == current && s.to <= to)
                .collect();
            candidates.sort_by_key(|s| (s.to, s.name));
            let Some(step) = candidates.first() else {
                return Err(if self.steps.iter().any(|s| s.from == current) {
                    MigrationErrorV1::NoPathTo { from: current, to }
                } else {
                    MigrationErrorV1::NoPathFrom(current)
                });
            };
            payload = (step.apply)(&payload);
            current = step.to;
        }
        Ok(payload)
    }

    /// A behaviour fingerprint over a fixed probe corpus.
    ///
    /// NOT a code digest, and deliberately not called one: it is the
    /// digest of what the steps DO to the probes. That catches a step
    /// whose behaviour changes and misses a change that is a no-op on
    /// every probe — which is the honest trade, and better than a code
    /// digest for the thing this row needs (detecting that a migration's
    /// meaning moved). A true per-function code digest would need `T1.2`'s
    /// source closure at function granularity, which does not exist.
    pub fn behaviour_fingerprint_v1(&self, probes: &[&[u8]]) -> ArtifactIdentityV1 {
        let mut bytes = Vec::new();
        let mut steps: Vec<&MigrationStepV1> = self.steps.iter().collect();
        steps.sort_by_key(|s| (s.from, s.to, s.name));
        for step in steps {
            bytes.extend_from_slice(step.name.as_bytes());
            bytes.extend_from_slice(&step.from.to_be_bytes());
            bytes.extend_from_slice(&step.to.to_be_bytes());
            for probe in probes {
                let out = (step.apply)(probe);
                bytes.extend_from_slice(&(out.len() as u64).to_be_bytes());
                bytes.extend_from_slice(&out);
            }
        }
        hash_artifact_bytes_v1(&bytes)
    }

    /// The row's central law: wherever a direct edge and a stepwise path
    /// both exist, they must agree on every probe. Without it the two are
    /// two implementations of one policy, and a save's fate depends on
    /// which one ran.
    pub fn direct_equals_stepwise_v1(&self, probes: &[&[u8]]) -> Result<(), String> {
        for step in &self.steps {
            // Only edges that SKIP something can disagree with a walk.
            let stepwise = self.apply_stepwise_v1(step.from, step.to, b"");
            if stepwise.is_err() {
                continue;
            }
            for probe in probes {
                let Some(direct) = self.apply_direct_v1(step.from, step.to, probe) else {
                    continue;
                };
                let walked = self
                    .apply_stepwise_v1(step.from, step.to, probe)
                    .map_err(|err| format!("{} : stepwise path vanished: {err:?}", step.name))?;
                if direct != walked {
                    return Err(format!(
                        "{}: direct {}->{} disagrees with the stepwise path on a probe of {} \
                         bytes",
                        step.name,
                        step.from,
                        step.to,
                        probe.len()
                    ));
                }
            }
        }
        Ok(())
    }
}

/// The rtsim migration graph as it actually is: empty.
///
/// Returned as a real graph rather than as an `Option`, so a caller
/// cannot forget the case. `is_empty_v1` is the truth this build has to
/// tell about rtsim.
pub fn rtsim_migration_graph_v1() -> MigrationGraphV1 {
    MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, Vec::new())
        .expect("an empty graph has no non-advancing steps")
}

/// A resolution policy the row requires to be DECLARED before code.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PolicyStatusV1 {
    /// Decided, with the decision stated.
    Declared,
    /// Not decided. The question is stated so it can be answered by
    /// whoever is entitled to answer it.
    PendingRuling,
}

/// The four policies, and their status.
///
/// All four are `PendingRuling` on purpose. Each is a judgement call
/// about player data — what happens to a character that no longer has a
/// valid item, whether a renamed asset silently becomes its successor —
/// and the row itself says a builder should not be making them
/// mid-implementation. Recording the QUESTION is the deliverable; an
/// answer invented here would be indistinguishable from a ruling.
pub const RESOLUTION_POLICIES: &[(&str, PolicyStatusV1, &str)] = &[
    (
        "tombstone",
        PolicyStatusV1::PendingRuling,
        "when a save references an entity/site/npc that no longer exists in this build, is the \
         reference dropped, preserved as an inert tombstone, or does the save become \
         ExplicitRecoveryOnly?",
    ),
    (
        "alias",
        PolicyStatusV1::PendingRuling,
        "when an asset is renamed, does the old name silently resolve to the new one (player \
         keeps the item, identity quietly changes) or fail loudly (player loses it, identity is \
         never wrong)?",
    ),
    (
        "content",
        PolicyStatusV1::PendingRuling,
        "when content a save depends on is removed outright, is the dependent object deleted, \
         replaced with a declared substitute, or is the whole save refused?",
    ),
    (
        "world",
        PolicyStatusV1::PendingRuling,
        "when worldgen changes such that a persisted position is no longer valid terrain, is the \
         entity moved, suspended, or is the save declared incompatible with this worldgen epoch?",
    ),
];

/// The row's sequencing rule (its step 7), stated as a value so `T4.6`
/// cannot quietly assume it has been satisfied: save manifests must not
/// be MANDATED until fixtures and offline recovery exist. `T4.6`'s
/// durable-epoch work may land; making it required is gated here.
pub const SAVE_MANIFEST_MANDATE_READY: bool = false;

#[cfg(test)]
mod save_migration_v1 {
    use super::*;

    fn add_byte(value: &[u8]) -> Vec<u8> {
        let mut out = value.to_vec();
        out.push(1);
        out
    }

    fn add_two(value: &[u8]) -> Vec<u8> {
        let mut out = value.to_vec();
        out.push(1);
        out.push(1);
        out
    }

    fn add_wrong(value: &[u8]) -> Vec<u8> {
        let mut out = value.to_vec();
        out.push(9);
        out
    }

    fn step(from: u32, to: u32, name: &'static str, apply: fn(&[u8]) -> Vec<u8>) -> MigrationStepV1 {
        MigrationStepV1 { store: SaveStoreV1::RtsimData, from, to, name, apply }
    }

    const PROBES: [&[u8]; 3] = [b"", b"a", b"a longer probe payload"];

    /// The law, on a graph where the direct edge is correct.
    #[test]
    fn direct_equals_stepwise_on_a_consistent_graph() {
        let graph = MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, vec![
            step(0, 1, "zero-to-one", add_byte),
            step(1, 2, "one-to-two", add_byte),
            step(0, 2, "zero-to-two-direct", add_two),
        ])
        .expect("advancing steps");

        assert_eq!(graph.direct_equals_stepwise_v1(&PROBES), Ok(()));
    }

    /// The law catches a direct edge that drifted from its stepwise
    /// path. This is the failure the row exists to make impossible to
    /// ship, so the test asserts the detection rather than the happy
    /// case.
    #[test]
    fn a_drifting_direct_edge_is_caught() {
        let graph = MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, vec![
            step(0, 1, "zero-to-one", add_byte),
            step(1, 2, "one-to-two", add_byte),
            step(0, 2, "zero-to-two-direct", add_wrong),
        ])
        .expect("advancing steps");

        let err = graph.direct_equals_stepwise_v1(&PROBES).expect_err("drift was not caught");
        assert!(err.contains("zero-to-two-direct"), "{err}");
    }

    /// Stepwise application is a function of the graph, not of the order
    /// its steps were written down in.
    #[test]
    fn stepwise_application_ignores_declaration_order() {
        let forwards = MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, vec![
            step(0, 1, "a", add_byte),
            step(1, 2, "b", add_byte),
        ])
        .expect("advancing");
        let backwards = MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, vec![
            step(1, 2, "b", add_byte),
            step(0, 1, "a", add_byte),
        ])
        .expect("advancing");

        assert_eq!(
            forwards.apply_stepwise_v1(0, 2, b"x"),
            backwards.apply_stepwise_v1(0, 2, b"x")
        );
        assert_eq!(
            forwards.behaviour_fingerprint_v1(&PROBES).digest.bytes.as_array(),
            backwards.behaviour_fingerprint_v1(&PROBES).digest.bytes.as_array()
        );
    }

    /// A non-advancing step is refused at construction. Guarding it here
    /// rather than inside the walk means the walk cannot fail to
    /// terminate.
    #[test]
    fn a_non_advancing_step_cannot_be_put_in_a_graph() {
        let err = MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, vec![step(
            3, 3, "self-loop", add_byte,
        )])
        .expect_err("a self-loop was accepted");
        assert_eq!(err, MigrationErrorV1::NonAdvancingStep {
            name: "self-loop",
            from: 3,
            to: 3
        });
    }

    /// Missing-path cases are distinguished. "Nothing leaves this epoch"
    /// and "steps exist but none reaches the target" are different
    /// findings for whoever has to fix the save.
    #[test]
    fn missing_paths_are_distinguished() {
        let graph = MigrationGraphV1::new_v1(SaveStoreV1::RtsimData, vec![
            step(0, 1, "a", add_byte),
            step(2, 5, "b", add_byte),
        ])
        .expect("advancing");

        // Nothing leaves epoch 7 at all.
        assert_eq!(graph.apply_stepwise_v1(7, 8, b""), Err(MigrationErrorV1::NoPathFrom(7)));
        // Something leaves epoch 2, but it overshoots the target — a
        // different problem for whoever has to fix the save, and one a
        // single "no path" error would have hidden.
        assert_eq!(graph.apply_stepwise_v1(2, 3, b""), Err(MigrationErrorV1::NoPathTo {
            from: 2,
            to: 3
        }));
        // And the walk reports the epoch it got STUCK at, not the one it
        // started from.
        assert_eq!(graph.apply_stepwise_v1(0, 9, b""), Err(MigrationErrorV1::NoPathFrom(1)));
    }

    /// The rtsim graph is empty, and this build says so out loud. If a
    /// real step ever lands, this test fails and forces the support
    /// policy above to be re-derived rather than left stale.
    #[test]
    fn the_rtsim_migration_graph_is_empty_in_this_build() {
        let graph = rtsim_migration_graph_v1();
        assert!(
            graph.is_empty_v1(),
            "an rtsim migration step appeared; rtsim_support_v1 must stop saying \
             ExplicitRecoveryOnly and start saying Migratable"
        );
        assert_eq!(graph.direct_equals_stepwise_v1(&PROBES), Ok(()));
    }

    /// The support policy matches what the loader actually does: current
    /// version loads, anything else needs `RTSIM_IGNORE_VERSION`.
    #[test]
    fn rtsim_support_matches_the_loader() {
        assert_eq!(
            rtsim_support_v1(rtsim::data::CURRENT_VERSION),
            SaveSupportV1::Supported
        );
        assert_eq!(
            rtsim_support_v1(rtsim::data::CURRENT_VERSION - 1),
            SaveSupportV1::ExplicitRecoveryOnly
        );
        assert_eq!(
            rtsim_support_v1(rtsim::data::CURRENT_VERSION + 1),
            SaveSupportV1::ExplicitRecoveryOnly,
            "a FUTURE rtsim save is recoverable by the same operator flag, which is what the \
             loader does — not Unsupported"
        );
    }

    /// A character db ahead of the build is `Unsupported`. refinery has
    /// no down-migrations, and guessing would write to player data.
    #[test]
    fn a_future_character_db_is_unsupported_not_migratable() {
        assert_eq!(character_db_support_v1(70, 70), SaveSupportV1::Supported);
        assert_eq!(character_db_support_v1(40, 70), SaveSupportV1::Migratable);
        assert_eq!(character_db_support_v1(71, 70), SaveSupportV1::Unsupported);
    }

    /// Every resolution policy states its question, and none is answered
    /// here. If one becomes `Declared` this test fails, which is the
    /// point: a policy about player data should not change without
    /// somebody noticing.
    #[test]
    fn every_resolution_policy_is_an_open_question_with_its_question_stated() {
        assert_eq!(RESOLUTION_POLICIES.len(), 4);
        for (name, status, question) in RESOLUTION_POLICIES {
            assert_eq!(
                *status,
                PolicyStatusV1::PendingRuling,
                "{name} was decided in code. The row says these are declared BEFORE code and are \
                 not a builder's call; if it has been ruled, record the ruling and update this \
                 test deliberately"
            );
            assert!(question.contains('?'), "{name} does not state a question: {question:?}");
            assert!(question.len() > 60, "{name}'s question is too vague to answer: {question:?}");
        }
    }

    /// The sequencing rule is a value, so `T4.6` cannot assume it.
    #[test]
    fn the_save_manifest_mandate_is_not_ready() {
        assert!(
            !SAVE_MANIFEST_MANDATE_READY,
            "mandating save manifests is gated on fixtures and offline recovery existing (T4.5 \
             step 7). Neither does yet."
        );
    }
}
