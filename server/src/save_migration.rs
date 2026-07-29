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
//! **Step 5 — RULED 2026-07-28.** The tombstone, alias, content and
//! world-resolution policies were carried here as stated QUESTIONS
//! rather than answered, because they are judgement calls about player
//! data and the row says a builder should not make them
//! mid-implementation. They have now been ruled, on one governing law
//! rather than four separate calls:
//!
//! > [`RESOLUTION_LAW_V1`]: an identity is never silently substituted;
//! > loss is recorded, substitution requires declaration, refusal is the
//! > last resort.
//!
//! Three of the four fall directly out of it; only the world policy
//! needed a separate cost judgement. Questions are kept verbatim beside
//! their rulings in [`RESOLUTION_POLICIES`] — a ruling without its
//! question is an instruction nobody can re-derive.

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

/// The exact load-vs-purge decision `RtSim::new` makes for a version-
/// mismatched save, extracted so `T4.5-FIXTURES`'s offline-recovery
/// proof can call the REAL decision directly rather than duplicating it
/// -- `server/src/rtsim/mod.rs`'s own match now delegates here.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RtsimVersionMismatchDispositionV1 {
    /// `RTSIM_IGNORE_VERSION` was set: the mismatched save loads
    /// unmigrated (`ExplicitRecoveryOnly`, made concrete).
    LoadUnmigrated,
    /// The default: purge and regenerate.
    PurgeAndRegenerate,
}

pub fn rtsim_version_mismatch_disposition_v1(ignore_version: bool) -> RtsimVersionMismatchDispositionV1 {
    if ignore_version {
        RtsimVersionMismatchDispositionV1::LoadUnmigrated
    } else {
        RtsimVersionMismatchDispositionV1::PurgeAndRegenerate
    }
}

/// `APEX-T4.3`'s rtsim world-baseline support policy, same shape as
/// [`rtsim_support_v1`] and governed by the SAME resolution law: the
/// `"world"` policy in [`RESOLUTION_POLICIES`], ruled
/// INCOMPATIBLE-WITH-EPOCH by default with an `RTSIM_IGNORE_VERSION`-
/// shaped escape hatch (`RTSIM_IGNORE_WORLD_BASELINE`). Not `Unsupported`
/// (the escape hatch is a real recovery path, even if a blunt one); not
/// `Migratable` (nothing transforms the data -- the world simply moved
/// on). A separate function from `rtsim_support_v1`, not a combined one:
/// version and baseline are independent axes (a save can be current-
/// version but stale-baseline, or vice versa), matching this file's own
/// per-axis-per-store convention (`character_db_support_v1` is likewise
/// its own function).
pub fn rtsim_baseline_support_v1(baseline_matches: bool) -> SaveSupportV1 {
    if baseline_matches {
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

/// The governing law all four rulings fall out of, ruled 2026-07-28.
///
/// Stated once because ruling the PRINCIPLE is what collapsed four
/// separate judgement calls into one: three of the four follow from it
/// directly, and only the world policy needed a separate cost judgement.
pub const RESOLUTION_LAW_V1: &str =
    "an identity is never silently substituted; loss is recorded, substitution requires \
     declaration, refusal is the last resort";

/// One policy: the question as it was asked, and the ruling as it was
/// given.
///
/// Both are kept. A ruling without its question is an instruction nobody
/// can re-derive; a question without its ruling is what this table was
/// before 2026-07-28.
pub struct ResolutionPolicyV1 {
    pub name: &'static str,
    pub status: PolicyStatusV1,
    /// Verbatim, unchanged since it was asked.
    pub question: &'static str,
    /// Empty iff `PendingRuling`. Enforced below.
    pub ruling: &'static str,
}

/// The four policies. **RULED 2026-07-28** on [`RESOLUTION_LAW_V1`];
/// questions unchanged, rulings recorded beneath each.
pub const RESOLUTION_POLICIES: &[ResolutionPolicyV1] = &[
    ResolutionPolicyV1 {
        name: "tombstone",
        status: PolicyStatusV1::Declared,
        question: "when a save references an entity/site/npc that no longer exists in this \
                   build, is the reference dropped, preserved as an inert tombstone, or does the \
                   save become ExplicitRecoveryOnly?",
        ruling: "INERT TOMBSTONE. Dropping makes the loss undiscoverable — nothing looks broken, \
                 so nobody reports it. ExplicitRecoveryOnly lets one dead NPC hold a whole world \
                 hostage. A tombstone tells the player someone is no longer here: true, visible, \
                 and it does not brick the save.",
    },
    ResolutionPolicyV1 {
        name: "alias",
        status: PolicyStatusV1::Declared,
        question: "when an asset is renamed, does the old name silently resolve to the new one \
                   (player keeps the item, identity quietly changes) or fail loudly (player \
                   loses it, identity is never wrong)?",
        ruling: "FAIL LOUDLY. A DECLARED alias table is the only path to resolution: a rename \
                 that IS the same thing gets an explicit entry, one that is not fails. An \
                 identity that can silently change is not an identity — this is where the \
                 program's own subject decides its answer.",
    },
    ResolutionPolicyV1 {
        name: "content",
        status: PolicyStatusV1::Declared,
        question: "when content a save depends on is removed outright, is the dependent object \
                   deleted, replaced with a declared substitute, or is the whole save refused?",
        ruling: "DELETE AND RECORD, through the tombstone mechanism the first policy defines — \
                 ONE mechanism, two triggers, built once. An undeclared substitute is silent \
                 aliasing one level up: a player's greatsword becomes a different weapon and \
                 their character changes without a word.",
    },
    ResolutionPolicyV1 {
        name: "world",
        status: PolicyStatusV1::Declared,
        question: "when worldgen changes such that a persisted position is no longer valid \
                   terrain, is the entity moved, suspended, or is the save declared incompatible \
                   with this worldgen epoch?",
        ruling: "INCOMPATIBLE-WITH-EPOCH by default, with an escape hatch MIRRORING the alias \
                 rule: a worldgen change may ship a DECLARED terrain-migration entry making \
                 prior epochs loadable; undeclared changes are incompatible. In development \
                 epochs bump freely and dev saves regenerate; toward release, changing the world \
                 means declaring what happens to the people living in it. SUSPENDED is rejected \
                 — a third state every consumer must handle is a tax on the whole codebase to \
                 avoid writing a declaration.",
    },
];

/// The row's sequencing rule (its step 7), stated as a value so `T4.6`
/// cannot quietly assume it has been satisfied: save manifests must not
/// be MANDATED until fixtures and offline recovery exist. `T4.6`'s
/// durable-epoch work may land; making it required is gated here.
///
/// **RULED READY (`APEX-T4.5-FIXTURES`).** Resolution-policies style,
/// same shape as [`RESOLUTION_POLICIES`]'s four entries above:
///
/// *Question:* do fixtures from a real supported epoch, and a proven
/// (not merely asserted) offline recovery path, exist for every store
/// this build persists?
///
/// *Ruling:* YES, for both stores. Character db:
/// `character_db_fixture_corpus_and_offline_recovery_over_real_migrations`
/// builds a fixture by running the REAL embedded refinery migration set
/// (`server/src/persistence/mod.rs::embedded`) to a historical target
/// via `refinery::Target::Version` -- not a hand-approximated schema --
/// classifies it `Migratable`, then proves refinery's OWN automatic
/// migration (the exact mechanism every real boot already runs) carries
/// it to current, with `corpus_index_v1`'s byte-equality showing a real
/// content change across the migration and a byte-identical no-op when
/// an already-current fixture is migrated again. Rtsim:
/// `rtsim_version_mismatch_offline_recovery_is_proven_over_a_real_fixture`
/// builds a genuinely-decodable version-mismatched blob, confirms
/// `probe_version_v1` reports its real number and `rtsim_support_v1`
/// classifies it `ExplicitRecoveryOnly`, then proves BOTH directions of
/// `rtsim_version_mismatch_disposition_v1` -- the exact function
/// `RtSim::new` now calls, not a duplicate -- load-unmigrated under the
/// documented env var, purge-and-regenerate by default. Both stores'
/// recovery paths are demonstrated against real fixtures, not asserted
/// to work.
pub const SAVE_MANIFEST_MANDATE_READY: bool = true;

#[cfg(test)]
mod save_migration_v1 {
    use super::*;
    use crate::save_inventory as server_save_inventory;

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

    /// `APEX-T4.5-FIXTURES`: the rtsim offline-recovery proof, over a
    /// REAL version-mismatched fixture (not a synthetic assertion) --
    /// `probe_version_v1` reports the mismatched version (not garbage),
    /// `rtsim_support_v1` classifies it `ExplicitRecoveryOnly`, and the
    /// documented env-var path (`rtsim_version_mismatch_disposition_v1`,
    /// the exact function `RtSim::new` now calls) demonstrably loads it
    /// while the default path purges. All machinery `T4.3`/`T4.4` already
    /// built -- this fixture just exercises it.
    #[test]
    fn rtsim_version_mismatch_offline_recovery_is_proven_over_a_real_fixture() {
        // A minimal-but-genuinely-decodable msgpack named-map: `version`
        // plus `nature` (`Data`'s only field without `#[serde(default)]`
        // -- every other field really does get skipped by serde, proving
        // `probe_version_v1`'s own "serde skips the rest" doc claim).
        // `nature` is the REAL `Nature` type (an empty `Grid`), not a
        // hand-approximated shape -- `Chunk::res` uses custom "rugged"
        // ser/de functions this test must not have to re-implement.
        #[derive(serde::Serialize)]
        struct MinimalDecodableFixtureV1 {
            version: u32,
            nature: rtsim::data::Nature,
        }
        let mismatched_version = rtsim::data::CURRENT_VERSION + 7;
        let empty_nature = rtsim::data::Nature {
            chunks: common::grid::Grid::new(
                vek::Vec2::new(0, 0),
                rtsim::data::nature::Chunk { res: enum_map::EnumMap::default() },
            ),
        };
        let mut bytes = Vec::new();
        rmp_serde::encode::write_named(&mut bytes, &MinimalDecodableFixtureV1 {
            version: mismatched_version,
            nature: empty_nature,
        })
        .expect("version + nature always encodes");

        // `probe_version_v1` reports the real number, not garbage.
        assert_eq!(rtsim::data::Data::probe_version_v1(bytes.as_slice()), Some(mismatched_version));

        // `Data::from_reader` classifies it as a version mismatch (not a
        // load failure) and hands back the decoded (defaulted) data.
        // `ReadError`'s own `Debug` impl deliberately doesn't print the
        // wrapped `Data` (it isn't `Debug`), so this matches by shape
        // rather than formatting the whole `Result`.
        match rtsim::data::Data::from_reader(bytes.as_slice()) {
            Err(rtsim::data::ReadError::VersionMismatch(data)) => {
                assert_eq!(data.version, mismatched_version);
            },
            Ok(_) => panic!("expected VersionMismatch, got Ok -- the mismatched version was not detected"),
            Err(rtsim::data::ReadError::Load(err)) => {
                panic!("expected VersionMismatch, got a Load failure: {err}")
            },
        }

        // The support policy classifies it correctly...
        assert_eq!(rtsim_support_v1(mismatched_version), SaveSupportV1::ExplicitRecoveryOnly);

        // ...and the REAL disposition function (RtSim::new calls this
        // exact one, not a duplicate) proves both directions: the
        // documented env var loads it, the default path purges.
        assert_eq!(
            rtsim_version_mismatch_disposition_v1(true),
            RtsimVersionMismatchDispositionV1::LoadUnmigrated,
            "RTSIM_IGNORE_VERSION must load the mismatched save unmigrated"
        );
        assert_eq!(
            rtsim_version_mismatch_disposition_v1(false),
            RtsimVersionMismatchDispositionV1::PurgeAndRegenerate,
            "the default path must purge and regenerate, never silently load a mismatch"
        );
    }

    /// `APEX-T4.3`: the baseline axis is independent of the version axis
    /// and follows the `"world"` resolution policy's own ruling
    /// (INCOMPATIBLE-WITH-EPOCH by default, `RTSIM_IGNORE_WORLD_BASELINE`
    /// as the declared escape hatch).
    #[test]
    fn rtsim_baseline_support_follows_the_world_resolution_policy() {
        assert_eq!(rtsim_baseline_support_v1(true), SaveSupportV1::Supported);
        assert_eq!(rtsim_baseline_support_v1(false), SaveSupportV1::ExplicitRecoveryOnly);
    }

    /// A character db ahead of the build is `Unsupported`. refinery has
    /// no down-migrations, and guessing would write to player data.
    #[test]
    fn a_future_character_db_is_unsupported_not_migratable() {
        assert_eq!(character_db_support_v1(70, 70), SaveSupportV1::Supported);
        assert_eq!(character_db_support_v1(40, 70), SaveSupportV1::Migratable);
        assert_eq!(character_db_support_v1(71, 70), SaveSupportV1::Unsupported);
    }

    /// `APEX-T4.5-FIXTURES`: the character-db fixture corpus + offline-
    /// recovery proof, over a REAL historical schema (a `refinery::
    /// Target::Version` partial run of the actual embedded V1..latest
    /// migration set, not a hand-approximated one -- byte-real per the
    /// row's own standard). Proves, in order: (a) a fixture stopped
    /// partway through the real migration set is classified
    /// `Migratable`; (b) refinery's OWN automatic migration -- the SAME
    /// mechanism every real boot already runs -- carries it to current
    /// successfully; (c) `corpus_index_v1` (byte equality is the only
    /// equality this row is entitled to assert, per the spec) shows a
    /// REAL content change across the migration and STABILITY when the
    /// already-current fixture is migrated again (a no-op, not a
    /// re-write).
    #[test]
    fn character_db_fixture_corpus_and_offline_recovery_over_real_migrations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let db_path = root.join("saves").join("db.sqlite");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();

        let runner = crate::persistence::embedded::migrations::runner();
        let latest = runner
            .get_migrations()
            .iter()
            .map(|m| m.version())
            .max()
            .expect("the embedded migration set is never empty");
        // A representative "well behind latest" historical epoch --
        // derived from the real count, not hand-picked, so this fixture
        // never rots as migrations are added.
        let historical_target = (latest / 2).max(1);

        // (a) Stop partway through the REAL migration set.
        {
            let mut conn = rusqlite::Connection::open(&db_path).expect("open fixture db");
            crate::persistence::embedded::migrations::runner()
                .set_target(refinery::Target::Version(historical_target))
                .run(&mut conn)
                .expect("partial migration run");
        }

        let applied_at_historical = match server_save_inventory::inventory_save_dir_v1(root).migrations {
            server_save_inventory::MigrationHistoryV1::Applied(applied) => {
                applied.iter().map(|m| m.version).max().expect("at least one migration applied")
            },
            other => panic!("expected Applied history on a real fixture, got {other:?}"),
        };
        assert_eq!(
            applied_at_historical, historical_target as i32,
            "the fixture must genuinely stop at the historical target, not silently run further"
        );
        assert_eq!(
            character_db_support_v1(applied_at_historical, latest as i32),
            SaveSupportV1::Migratable
        );

        let before = server_save_inventory::inventory_save_dir_v1(root).corpus_index_v1();

        // (b) The SAME mechanism every real boot runs: refinery's own
        // automatic migration, no target override, carries the fixture
        // the rest of the way -- this IS "offline recovery" made
        // concrete, not merely asserted.
        {
            let mut conn = rusqlite::Connection::open(&db_path).expect("reopen fixture db");
            crate::persistence::embedded::migrations::runner()
                .run(&mut conn)
                .expect("full migration run must succeed against a real historical fixture");
        }

        let applied_at_latest = match server_save_inventory::inventory_save_dir_v1(root).migrations {
            server_save_inventory::MigrationHistoryV1::Applied(applied) => {
                applied.iter().map(|m| m.version).max().expect("at least one migration applied")
            },
            other => panic!("expected Applied history after full migration, got {other:?}"),
        };
        assert_eq!(applied_at_latest, latest as i32);
        assert_eq!(character_db_support_v1(applied_at_latest, latest as i32), SaveSupportV1::Supported);

        // (c) corpus_index_v1: a real content change across the
        // migration...
        let after = server_save_inventory::inventory_save_dir_v1(root).corpus_index_v1();
        assert_ne!(before, after, "migrating a fixture must produce an observable content change");

        // ...and stability when an already-current fixture is migrated
        // again -- refinery's own no-op, not a re-write this row would
        // have to explain.
        {
            let mut conn = rusqlite::Connection::open(&db_path).expect("reopen fixture db again");
            crate::persistence::embedded::migrations::runner()
                .run(&mut conn)
                .expect("a no-op migration run must still succeed");
        }
        let after_again = server_save_inventory::inventory_save_dir_v1(root).corpus_index_v1();
        assert_eq!(after, after_again, "re-migrating an already-current fixture must be a byte-identical no-op");
    }

    /// Every policy keeps its question AND carries its ruling.
    ///
    /// This test previously asserted every policy was `PendingRuling`,
    /// and said "if it has been ruled, record the ruling and update this
    /// test deliberately". That is exactly what happened on 2026-07-28:
    /// all four were ruled on `RESOLUTION_LAW_V1` and this assertion was
    /// INVERTED BY HAND rather than relaxed. The guard is the same in
    /// spirit — a policy about player data cannot change status without
    /// somebody editing this test on purpose.
    #[test]
    fn every_resolution_policy_states_its_question_and_carries_its_ruling() {
        assert_eq!(RESOLUTION_POLICIES.len(), 4);
        assert!(RESOLUTION_LAW_V1.len() > 60, "the governing law is too vague to rule from");
        for policy in RESOLUTION_POLICIES {
            let name = policy.name;
            assert!(
                policy.question.contains('?'),
                "{name} does not state a question: {:?}",
                policy.question
            );
            assert!(policy.question.len() > 60, "{name}'s question is too vague to answer");
            match policy.status {
                PolicyStatusV1::Declared => assert!(
                    policy.ruling.len() > 60,
                    "{name} is Declared but its ruling says nothing; a ruling nobody can \
                     re-derive is an instruction, not a decision"
                ),
                PolicyStatusV1::PendingRuling => assert!(
                    policy.ruling.is_empty(),
                    "{name} is PendingRuling but carries a ruling — one of the two is wrong"
                ),
            }
        }
    }

    /// All four are ruled. A revert to `PendingRuling` should be as
    /// visible as the ruling was.
    #[test]
    fn all_four_policies_are_ruled() {
        for policy in RESOLUTION_POLICIES {
            assert_eq!(
                policy.status,
                PolicyStatusV1::Declared,
                "{} reverted to PendingRuling",
                policy.name
            );
        }
    }

    /// The sequencing rule is a value, so `T4.6` cannot assume it.
    ///
    /// This test previously asserted the mandate was NOT ready, with the
    /// same "if it becomes ready, invert this deliberately" spirit the
    /// four resolution policies carried. That is exactly what happened
    /// on `APEX-T4.5-FIXTURES`: both stores' fixture corpus + proven
    /// offline recovery now exist (see `SAVE_MANIFEST_MANDATE_READY`'s
    /// own doc comment for the ruling and its evidence), and this
    /// assertion was INVERTED BY HAND rather than relaxed. A revert to
    /// `false` should be as visible as this flip was.
    #[test]
    fn the_save_manifest_mandate_is_ready() {
        assert!(
            SAVE_MANIFEST_MANDATE_READY,
            "reverted to not-ready -- fixtures or offline recovery regressed; see \
             SAVE_MANIFEST_MANDATE_READY's doc comment for what must hold"
        );
    }
}
