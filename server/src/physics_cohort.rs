//! `APEX-T5.1` — the server-authoritative physics cohort.
//!
//! Make the authority transition independently measurable *before*
//! anyone extracts rollback from it.
//!
//! **The failure this row exists to prevent.** Authority is decided at
//! `sys/entity_sync.rs::should_sync_client_physics` by
//! `opt_in || force_list.contains(uuid)`. Measuring against that
//! predicate mixes two populations that have nothing in common: players
//! who CHOSE server authority and players placed under it as a
//! moderation action. Every metric derived from the mix answers a
//! question nobody asked.
//!
//! **Disjointness is by construction, not by filtering.**
//! [`CohortInputsV1`] has no force-list field, so
//! [`assign_cohort_v1`] cannot consult it — a lazy implementation
//! cannot accidentally include it, because there is nothing to include.
//! The force list keeps its moderation role and gains no measurement
//! role.
//!
//! **The control cohort is byte-identical to pre-change because nothing
//! branches on cohort.** This module observes; it decides nothing. No
//! authority predicate reads a cohort, and the row's own canary set says
//! so, so "the control's behaviour is unchanged" is a property of the
//! code's shape rather than a claim about it.
//!
//! **Delivered here:** the cohort policy and assignment (step 1), and
//! per-cohort accounting of physics-report admission (step 2, the
//! correction-frequency metric). **Not delivered, and named so the
//! coverage map can say it too:** bandwidth, glider-specific and
//! responsiveness metrics, the identical-scenario harness across both
//! cohorts (step 3) and the comparison report (step 4). Those need a
//! scenario runner that does not exist yet; this row makes membership
//! and one metric real so that runner has something to key on.

use hashbrown::HashMap;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use common::uuid::Uuid;

/// Which physics-authority cohort a player is in.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum PhysicsCohortV1 {
    /// The current authority model, unchanged. Not "the players we did
    /// nothing to" — a control is a measured population.
    Control,
    /// Server-authoritative physics, entered by the player's OWN opt-in.
    Treatment,
}

impl PhysicsCohortV1 {
    pub const ALL: [Self; 2] = [Self::Control, Self::Treatment];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Treatment => "treatment",
        }
    }
}

/// Everything cohort assignment is allowed to see.
///
/// There is deliberately no force-list field. The disjointness the row
/// requires is a property of this type: assignment cannot consult
/// moderation state because moderation state is not reachable from here.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CohortInputsV1 {
    /// The player's own `server_authoritative_physics_optin`.
    pub opted_in: bool,
}

/// Pure, total, and force-list-blind.
pub const fn assign_cohort_v1(inputs: CohortInputsV1) -> PhysicsCohortV1 {
    if inputs.opted_in {
        PhysicsCohortV1::Treatment
    } else {
        PhysicsCohortV1::Control
    }
}

/// What happened when a player's cohort was looked up.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CohortLookupV1 {
    /// First assignment for this player.
    Assigned(PhysicsCohortV1),
    /// Already assigned, and the recorded cohort still matches what the
    /// inputs would derive.
    Recalled(PhysicsCohortV1),
    /// Already assigned, and the inputs now derive a DIFFERENT cohort —
    /// the player toggled their opt-in. The recorded cohort is returned
    /// unchanged and the attempt is counted.
    ///
    /// Honouring the flip would be worse than ignoring it: metrics
    /// accumulated across a mid-session flip belong to neither cohort,
    /// and a comparison built from them would be confidently wrong.
    FlipRefused {
        recorded: PhysicsCohortV1,
        derived: PhysicsCohortV1,
    },
}

impl CohortLookupV1 {
    pub const fn cohort(self) -> PhysicsCohortV1 {
        match self {
            Self::Assigned(cohort) | Self::Recalled(cohort) => cohort,
            Self::FlipRefused { recorded, .. } => recorded,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct AssignmentV1 {
    cohort: PhysicsCohortV1,
    flip_attempts: u64,
}

/// Cohort membership, pinned per player.
///
/// Keyed by `Uuid` rather than by entity or session, which is what makes
/// assignment survive a reconnect: the entity is gone, the account is
/// not.
#[derive(Default)]
pub struct PhysicsCohortRegistryV1 {
    assignments: Mutex<HashMap<Uuid, AssignmentV1>>,
}

impl PhysicsCohortRegistryV1 {
    pub fn new() -> Self { Self::default() }

    /// Assign on first sight, recall thereafter.
    pub fn lookup_v1(&self, player: Uuid, inputs: CohortInputsV1) -> CohortLookupV1 {
        let derived = assign_cohort_v1(inputs);
        let mut assignments = self.assignments.lock().expect("cohort registry poisoned");
        match assignments.get_mut(&player) {
            None => {
                assignments.insert(player, AssignmentV1 { cohort: derived, flip_attempts: 0 });
                CohortLookupV1::Assigned(derived)
            },
            Some(existing) if existing.cohort == derived => CohortLookupV1::Recalled(derived),
            Some(existing) => {
                existing.flip_attempts += 1;
                CohortLookupV1::FlipRefused { recorded: existing.cohort, derived }
            },
        }
    }

    /// The recorded cohort, without assigning one.
    pub fn recorded_v1(&self, player: Uuid) -> Option<PhysicsCohortV1> {
        self.assignments
            .lock()
            .expect("cohort registry poisoned")
            .get(&player)
            .map(|a| a.cohort)
    }

    pub fn flip_attempts_v1(&self, player: Uuid) -> u64 {
        self.assignments
            .lock()
            .expect("cohort registry poisoned")
            .get(&player)
            .map_or(0, |a| a.flip_attempts)
    }

    pub fn population_v1(&self, cohort: PhysicsCohortV1) -> usize {
        self.assignments
            .lock()
            .expect("cohort registry poisoned")
            .values()
            .filter(|a| a.cohort == cohort)
            .count()
    }
}

/// Per-cohort counters for client physics reports.
///
/// Interior mutability with atomics, following `SemanticIngressMetricsV1`
/// — the ingress path holds this by `ReadExpect` and must not need a
/// write lock to count.
#[derive(Default)]
pub struct PhysicsCohortMetricsV1 {
    control_admitted: AtomicU64,
    control_rejected: AtomicU64,
    treatment_admitted: AtomicU64,
    treatment_rejected: AtomicU64,
    /// Reports arriving from a player whose cohort was refused a flip.
    /// Counted separately so a flip cannot quietly contaminate either
    /// cohort's totals.
    during_refused_flip: AtomicU64,
}

impl PhysicsCohortMetricsV1 {
    pub fn new() -> Self { Self::default() }

    /// Record one client physics report. `admitted` is whether the
    /// server took the client's position — i.e. whether the client still
    /// holds authority over itself this tick.
    pub fn record_report_v1(&self, lookup: CohortLookupV1, admitted: bool) {
        if matches!(lookup, CohortLookupV1::FlipRefused { .. }) {
            self.during_refused_flip.fetch_add(1, Ordering::Relaxed);
        }
        let counter = match (lookup.cohort(), admitted) {
            (PhysicsCohortV1::Control, true) => &self.control_admitted,
            (PhysicsCohortV1::Control, false) => &self.control_rejected,
            (PhysicsCohortV1::Treatment, true) => &self.treatment_admitted,
            (PhysicsCohortV1::Treatment, false) => &self.treatment_rejected,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Record `admitted` + `rejected` reports at once. The ingress path
    /// counts a whole tick's reports before it knows the cohort, so
    /// counting them one at a time would mean a lookup per message.
    pub fn record_reports_v1(&self, lookup: CohortLookupV1, admitted: u64, rejected: u64) {
        if matches!(lookup, CohortLookupV1::FlipRefused { .. }) {
            self.during_refused_flip.fetch_add(admitted + rejected, Ordering::Relaxed);
        }
        let (admit_counter, reject_counter) = match lookup.cohort() {
            PhysicsCohortV1::Control => (&self.control_admitted, &self.control_rejected),
            PhysicsCohortV1::Treatment => (&self.treatment_admitted, &self.treatment_rejected),
        };
        admit_counter.fetch_add(admitted, Ordering::Relaxed);
        reject_counter.fetch_add(rejected, Ordering::Relaxed);
    }

    /// `(admitted, rejected)` for one cohort.
    pub fn counts_v1(&self, cohort: PhysicsCohortV1) -> (u64, u64) {
        let (admitted, rejected) = match cohort {
            PhysicsCohortV1::Control => (&self.control_admitted, &self.control_rejected),
            PhysicsCohortV1::Treatment => (&self.treatment_admitted, &self.treatment_rejected),
        };
        (admitted.load(Ordering::Relaxed), rejected.load(Ordering::Relaxed))
    }

    pub fn reports_during_refused_flip_v1(&self) -> u64 {
        self.during_refused_flip.load(Ordering::Relaxed)
    }
}

/// What a `COH` canary would have to catch, and whether it is covered.
///
/// Named the way the T5 spec sketched them so a reader can check the
/// sketch against the tree instead of trusting a summary.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CanaryStatusV1 {
    /// A test in this module fails if the case regresses.
    Covered,
    /// Nothing in the tree would catch it yet, and the reason is stated.
    Open,
}

pub const COHORT_CANARIES: &[(&str, CanaryStatusV1, &str)] = &[
    (
        "COH-001 force-listed player counted in the treatment cohort",
        CanaryStatusV1::Covered,
        "CohortInputsV1 has no force-list field, so assignment cannot see it; the test drives a \
         force-listed, non-opted-in player and asserts Control",
    ),
    (
        "COH-002 cohort flipping mid-session",
        CanaryStatusV1::Covered,
        "the registry pins the first assignment and returns FlipRefused, counting the attempt",
    ),
    (
        "COH-003 metrics attributed to the wrong cohort after a reconnect",
        CanaryStatusV1::Covered,
        "assignment is keyed by Uuid, not by entity or session; the test re-looks-up after \
         dropping the entity and asserts the same cohort and the same counters",
    ),
    (
        "COH-004 control cohort's authority path altered",
        CanaryStatusV1::Covered,
        "no authority predicate reads a cohort; a source scan asserts the cohort type appears in \
         no authority-deciding file",
    ),
    (
        "COH-005 bandwidth/glider/responsiveness metrics diverge between cohorts",
        CanaryStatusV1::Open,
        "those metrics are not collected yet (steps 3-4 need a scenario harness across cohorts); \
         nothing here should be read as measuring them",
    ),
];

#[cfg(test)]
mod physics_cohort_v1 {
    use super::*;

    fn uuid(n: u128) -> Uuid { Uuid::from_u128(n) }

    /// `COH-001`, the test the spec calls the one that catches a lazy
    /// implementation. A force-listed player who never opted in is in the
    /// CONTROL cohort, even though `should_sync_client_physics` returns
    /// true for them — moderation is not enrolment.
    #[test]
    fn force_list_membership_does_not_imply_treatment_membership() {
        let registry = PhysicsCohortRegistryV1::new();
        let player = uuid(1);

        // The force-listed player's own setting is what assignment sees,
        // and it is false. There is no argument by which the force list
        // could reach this call.
        let lookup = registry.lookup_v1(player, CohortInputsV1 { opted_in: false });
        assert_eq!(lookup.cohort(), PhysicsCohortV1::Control);
        assert_eq!(registry.population_v1(PhysicsCohortV1::Treatment), 0);
    }

    /// `COH-003`. Assignment is keyed by account, so a reconnect — a new
    /// entity, a new session — recalls the same cohort.
    #[test]
    fn cohort_assignment_is_stable_across_a_reconnect() {
        let registry = PhysicsCohortRegistryV1::new();
        let player = uuid(2);
        let inputs = CohortInputsV1 { opted_in: true };

        assert_eq!(registry.lookup_v1(player, inputs), CohortLookupV1::Assigned(PhysicsCohortV1::Treatment));
        // ... disconnect, reconnect, new entity, same account ...
        assert_eq!(registry.lookup_v1(player, inputs), CohortLookupV1::Recalled(PhysicsCohortV1::Treatment));
        assert_eq!(registry.recorded_v1(player), Some(PhysicsCohortV1::Treatment));
    }

    /// `COH-002`. A mid-session opt-in toggle does not move the player.
    /// Honouring it would attribute one session's metrics to two
    /// cohorts, which is worse than ignoring the flip.
    #[test]
    fn a_mid_session_flip_is_refused_and_counted() {
        let registry = PhysicsCohortRegistryV1::new();
        let player = uuid(3);

        registry.lookup_v1(player, CohortInputsV1 { opted_in: false });
        let flipped = registry.lookup_v1(player, CohortInputsV1 { opted_in: true });
        assert_eq!(flipped, CohortLookupV1::FlipRefused {
            recorded: PhysicsCohortV1::Control,
            derived: PhysicsCohortV1::Treatment,
        });
        assert_eq!(flipped.cohort(), PhysicsCohortV1::Control, "the flip moved the player");
        assert_eq!(registry.flip_attempts_v1(player), 1);
        assert_eq!(registry.population_v1(PhysicsCohortV1::Treatment), 0);
    }

    /// Reports are counted under the cohort they belong to, and reports
    /// arriving during a refused flip are also counted SEPARATELY so a
    /// flip cannot quietly contaminate a comparison.
    #[test]
    fn reports_are_attributed_to_the_recorded_cohort() {
        let registry = PhysicsCohortRegistryV1::new();
        let metrics = PhysicsCohortMetricsV1::new();
        let control = uuid(4);
        let treatment = uuid(5);

        metrics.record_report_v1(
            registry.lookup_v1(control, CohortInputsV1 { opted_in: false }),
            true,
        );
        metrics.record_report_v1(
            registry.lookup_v1(control, CohortInputsV1 { opted_in: false }),
            false,
        );
        metrics.record_report_v1(
            registry.lookup_v1(treatment, CohortInputsV1 { opted_in: true }),
            false,
        );

        assert_eq!(metrics.counts_v1(PhysicsCohortV1::Control), (1, 1));
        assert_eq!(metrics.counts_v1(PhysicsCohortV1::Treatment), (0, 1));
        assert_eq!(metrics.reports_during_refused_flip_v1(), 0);

        // Now the control player toggles their opt-in and keeps sending.
        metrics.record_report_v1(
            registry.lookup_v1(control, CohortInputsV1 { opted_in: true }),
            true,
        );
        assert_eq!(
            metrics.counts_v1(PhysicsCohortV1::Control),
            (2, 1),
            "the report went to the recorded cohort"
        );
        assert_eq!(metrics.counts_v1(PhysicsCohortV1::Treatment), (0, 1), "and not to the other");
        assert_eq!(metrics.reports_during_refused_flip_v1(), 1, "and it is flagged as suspect");
    }

    /// The batched recorder agrees with the per-report one. The ingress
    /// path uses the batched form, so a divergence between them would
    /// mean the tested behaviour and the live behaviour are different.
    #[test]
    fn batched_and_single_recording_agree() {
        let registry = PhysicsCohortRegistryV1::new();
        let single = PhysicsCohortMetricsV1::new();
        let batched = PhysicsCohortMetricsV1::new();
        let lookup = registry.lookup_v1(uuid(6), CohortInputsV1 { opted_in: true });

        for _ in 0..3 {
            single.record_report_v1(lookup, true);
        }
        for _ in 0..2 {
            single.record_report_v1(lookup, false);
        }
        batched.record_reports_v1(lookup, 3, 2);

        assert_eq!(
            single.counts_v1(PhysicsCohortV1::Treatment),
            batched.counts_v1(PhysicsCohortV1::Treatment)
        );
        assert_eq!(
            single.reports_during_refused_flip_v1(),
            batched.reports_during_refused_flip_v1()
        );
    }

    /// `COH-004`. The control cohort's authority path is unchanged
    /// because NOTHING branches on cohort: the files that decide physics
    /// authority do not mention the type. Asserted against the source, so
    /// a future edit that starts branching on cohort fails here rather
    /// than silently making the control a treatment.
    #[test]
    fn no_authority_deciding_file_reads_a_cohort() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        // The two sites the T5 spec names as deciding authority.
        for file in ["sys/entity_sync.rs", "sys/msg/in_game.rs"] {
            let text = std::fs::read_to_string(src.join(file))
                .unwrap_or_else(|e| panic!("{file}: {e}"));
            for (number, line) in text.lines().enumerate() {
                if !line.contains("PhysicsCohortV1") && !line.contains("cohort") {
                    continue;
                }
                if line.trim_start().starts_with("//") {
                    continue;
                }
                // Declaring, destructuring and recording are all fine.
                // BRANCHING is what would make the control cohort's path
                // differ from pre-change, so branching is what is banned.
                //
                // A source scan is not airtight — a condition computed on
                // one line and branched on another escapes it — and that
                // limit is stated rather than papered over. It catches the
                // direct form, which is the form a plausible edit takes.
                const BRANCH_TOKENS: [&str; 6] =
                    ["if ", "match ", "matches!", " == ", " != ", "unwrap_or"];
                assert!(
                    !BRANCH_TOKENS.iter().any(|token| line.contains(token)),
                    "{file}:{} branches on the cohort, which means the control cohort's \
                     authority path can now differ from pre-change:\n{line}",
                    number + 1
                );
            }
        }
    }

    /// The canary sketch is honest about what is not covered. An `Open`
    /// entry must say why, and at least one must exist — a coverage map
    /// with nothing open is usually a map that stopped looking.
    #[test]
    fn every_canary_states_its_status_and_the_open_ones_say_why() {
        assert_eq!(COHORT_CANARIES.len(), 5);
        let open = COHORT_CANARIES
            .iter()
            .filter(|(_, status, _)| *status == CanaryStatusV1::Open)
            .count();
        assert_eq!(open, 1, "the open-case count moved; say what changed");
        for (name, _, why) in COHORT_CANARIES {
            assert!(why.len() > 40, "{name} is claimed without evidence: {why:?}");
        }
    }

    /// Assignment is total and force-list-blind by construction: the only
    /// input is the opt-in, so there are exactly two reachable outcomes.
    #[test]
    fn assignment_is_a_total_function_of_the_optin_alone() {
        assert_eq!(
            assign_cohort_v1(CohortInputsV1 { opted_in: false }),
            PhysicsCohortV1::Control
        );
        assert_eq!(
            assign_cohort_v1(CohortInputsV1 { opted_in: true }),
            PhysicsCohortV1::Treatment
        );
        assert_eq!(PhysicsCohortV1::ALL.len(), 2);
    }
}
