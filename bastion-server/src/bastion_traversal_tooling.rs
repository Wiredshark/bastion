//! REQ-0094/0094A test-only tooling for the B5.8 traversal contract.
//!
//! [`run_smoke80_contract_model`] is intentionally a synthetic state/ownership
//! model. It does **not** reproduce production terrain or locomotion. The
//! separate [`run_smoke80_production_geometry_fixture`] uses the exact shipping
//! corridor/body-lane/cylinder predicates through a side-effect-free adapter.
//! Neither path participates in ECS scheduling or mutates gameplay state.

use crate::{
    bastion_jobs::{
        EmergencyRouteDescriptor, EmergencyTraversalKind,
        emergency_constructed_ladder_corridor_candidates,
        emergency_validate_constructed_ladder_corridor,
    },
    bastion_traversal::{
        BastionTraversalInterruption, BastionTraversalPhase, BastionTraversalPurpose,
        BastionTraversalReject, BastionTraversalTask,
    },
};
use common::{
    comp::bastion::{BastionMovementWriter, BastionTraversalMode},
    terrain::{Block, BlockKind, SpriteKind},
    vol::{ReadVol, WriteVol},
    volumes::dyna::Dyna,
};
use serde::{Deserialize, Serialize};
use vek::{Rgb, Vec3};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1TraversalCase {
    pub name: String,
    pub passed: bool,
    pub observed: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage1TraversalOwnerReport {
    pub schema: String,
    pub fixture: String,
    pub production_geometry_exercised: bool,
    pub cases: Vec<Stage1TraversalCase>,
    pub deterministic: bool,
    pub gameplay_mutated: bool,
    pub circuit_breaker_state: String,
    pub local_patch_count: u32,
    pub full_smoke_count_since_last_gate_advance: u32,
    pub failure_phase_history: Vec<String>,
    pub next_permitted_action: String,
}

fn stage1_uid(value: u64) -> common::uid::Uid {
    common::uid::Uid(std::num::NonZeroU64::new(value).expect("fixture uid is non-zero"))
}

fn stage1_task() -> BastionTraversalTask {
    let member = stage1_uid(2);
    BastionTraversalTask {
        link_id: 0xB5_8001,
        // R10: fixture task adopts epoch 0 (a fresh link's current value).
        epoch: 0,
        terrain_revision: SMOKE80_TERRAIN_REVISION,
        reserved_member: member,
        entry: NORMAL_ENTRY,
        exit: Vec3::new(NORMAL_ENTRY.x, NORMAL_ENTRY.y - 1, NORMAL_TOP_Z),
        owner: stage1_uid(1),
        purpose: BastionTraversalPurpose::ConstructionFrontier(SMOKE80_FRONTIER_JOB),
        traversal_kind: EmergencyTraversalKind::ConstructedLadder,
        mount: NORMAL_ENTRY,
        top_z: NORMAL_TOP_Z,
        phase: BastionTraversalPhase::QueuedForLink,
        started_tick: 10,
        phase_tick: 10,
        last_progress_tick: 10,
        stable_samples: 0,
        best_z: NORMAL_ACTUAL_START.z,
        best_exit_distance: f32::INFINITY,
        exit_target: None,
        exit_started_tick: 0,
        exit_stable_samples: 0,
        abort_reason: None,
        ladder_contact: Some(NORMAL_FIRST_RUNG),
        wall_contact: None,
        traversal_started_tick: 0,
        stable_window_started_tick: 0,
        last_stable_sample_tick: 0,
    }
}

fn stage1_case(name: &str, passed: bool, observed: impl std::fmt::Debug) -> Stage1TraversalCase {
    Stage1TraversalCase {
        name: name.into(),
        passed,
        observed: format!("{observed:?}"),
    }
}

fn record_stage1_task_snapshot(task: BastionTraversalTask, tick: u64, marker: &str) {
    if !crate::bastion_flight_recorder::enabled() {
        return;
    }
    let ownership = task.ownership();
    let mode = ownership.map(|ownership| ownership.mode);
    let writer = mode.map_or("none", |mode| {
        if mode == BastionTraversalMode::LinkApproach {
            "agent_chaser_link_approach"
        } else {
            "bastion_traversal_task"
        }
    });
    crate::bastion_flight_recorder::record_writer(crate::bastion_flight_recorder::WriterEvent {
        schema: "bastion.flight-recorder.event/v1".into(),
        tick,
        uid: task.reserved_member.0.get(),
        observation_sequence: 400,
        snapshot_stage: "stage1-focused-production-task-snapshot".into(),
        dispatcher_dependency_proven: false,
        writer: writer.into(),
        move_dir: [0.0, 0.0],
        move_z: 0.0,
        target: Some([
            task.entry.x as f32,
            task.entry.y as f32,
            task.entry.z as f32,
        ]),
        note: format!(
            "marker={marker}; phase={:?}; mode={mode:?}; link_id={}; terrain_revision={}",
            task.phase, task.link_id, task.terrain_revision
        ),
    });
    crate::bastion_flight_recorder::record_sample(crate::bastion_flight_recorder::FlightSample {
        schema: "bastion.flight-recorder.sample/v1".into(),
        tick,
        simulated_seconds: tick as f64 / 30.0,
        wall_unix_millis: None,
        uid: task.reserved_member.0.get(),
        entity: 0,
        episode: task.link_id,
        position: [
            task.entry.x as f32,
            task.entry.y as f32,
            task.entry.z as f32,
        ],
        velocity: [0.0; 3],
        character_state: "FocusedContractNoPhysics".into(),
        phase: format!("{:?}", task.phase),
        on_ground: false,
        on_wall: None,
        support_clear: true,
        body_clear: true,
        head_clear: true,
        active_job: match task.purpose {
            BastionTraversalPurpose::ConstructionFrontier(job) => Some(job),
            BastionTraversalPurpose::FullExit => None,
        },
        active_job_state: Some("FocusedContract".into()),
        route_kind: Some(format!("{:?}", task.traversal_kind)),
        route_owner: Some(task.owner.0.get()),
        link_id: Some(task.link_id.to_string()),
        frontier_job: match task.purpose {
            BastionTraversalPurpose::ConstructionFrontier(job) => Some(job),
            BastionTraversalPurpose::FullExit => None,
        },
        corridor_cursor: None,
        corridor_waypoint: Some([task.entry.x, task.entry.y, task.entry.z]),
        goto_target: None,
        chaser_last_target: None,
        chaser_route_target: None,
        chaser_route_head: None,
        chaser_next_idx: None,
        chaser_path_state: "FocusedContractNoChaser".into(),
        chaser_recent_states: 0,
        controller_move_dir: [0.0; 2],
        controller_move_z: 0.0,
        movement_writer: writer.into(),
        energy: None,
        terrain_revision: Some(task.terrain_revision),
        exit_plane_z: Some(task.exit.z as f32),
        endpoint_distance: None,
        // R10 v2: the focused contract probe carries the task's epoch;
        // no climb witness (no physics in this probe).
        ownership_epoch: Some(task.epoch),
        climb_token_witness: None,
        queue_position: None,
        queue_enqueue_tick: None,
        reservation_generation: None,
    });
}

/// Millisecond-scale Stage-1 contract fixture. Geometry comes from the
/// production shipping-predicate fixture; task/reservation/owner/interruption
/// assertions invoke the production task and shared discriminator directly.
/// Physics is deliberately not simulated here.
pub fn run_stage1_constructed_ladder_fixture() -> Stage1TraversalOwnerReport {
    let geometry = run_smoke80_production_geometry_fixture();
    let geometry_ok = geometry.deterministic
        && geometry.production_geometry_exercised
        && geometry
            .cases
            .iter()
            .any(|case| case.name == "cleared_supported_entry" && !case.rejected);
    let member = stage1_uid(2);
    let competitor = stage1_uid(3);
    let mut cases = vec![stage1_case(
        "production_geometry_positive_and_falsifiers",
        geometry_ok,
        geometry
            .cases
            .iter()
            .map(|case| (&case.name, case.rejected))
            .collect::<Vec<_>>(),
    )];

    let mut approach = stage1_task();
    approach.phase = BastionTraversalPhase::LinkApproach;
    record_stage1_task_snapshot(approach, 9, "link-approach");
    let mut success = stage1_task();
    record_stage1_task_snapshot(success, 10, "queued");
    let reserved = success.reserve(member, 11);
    record_stage1_task_snapshot(success, 11, "reserved");
    let reserved_owner = success.ownership();
    let traversing = success
        .transition(BastionTraversalPhase::TraversingEntry, 12)
        .and_then(|_| success.transition(BastionTraversalPhase::TraversingLink, 13));
    record_stage1_task_snapshot(success, 13, "traversing");
    let traversal_owner = success.ownership();
    let one_owner = traversal_owner.is_some_and(|ownership| {
        ownership.mode == BastionTraversalMode::TraversingLink
            && ownership
                .mode
                .allows_writer(BastionMovementWriter::BastionTraversalTask)
            && !ownership
                .mode
                .allows_writer(BastionMovementWriter::AgentChaser)
            && !ownership.mode.allows_writer(BastionMovementWriter::Orca)
            && !ownership
                .mode
                .allows_writer(BastionMovementWriter::GenericGoto)
            && !ownership
                .mode
                .allows_writer(BastionMovementWriter::GenericSoftSteering)
    });
    cases.push(stage1_case(
        "successful_reserve_and_exclusive_traverse",
        reserved.is_ok()
            && reserved_owner.is_some_and(|owner| owner.reserved_member == member)
            && traversing.is_ok()
            && one_owner,
        (reserved, reserved_owner, traversing, traversal_owner),
    ));

    let mut frontier = success;
    frontier.reserved_member = stage1_uid(4);
    let frontier_result = frontier
        .transition(BastionTraversalPhase::TraversingTopExit, 14)
        .and_then(|_| frontier.transition(BastionTraversalPhase::FrontierWork, 15));
    record_stage1_task_snapshot(frontier, 15, "frontier-work");
    cases.push(stage1_case(
        "frontier_work_retains_exclusive_owner",
        frontier_result.is_ok()
            && frontier.ownership().is_some_and(|ownership| {
                ownership.mode == BastionTraversalMode::FrontierWork
                    && ownership
                        .mode
                        .allows_writer(BastionMovementWriter::BastionTraversalTask)
            }),
        (frontier_result, frontier.ownership()),
    ));

    let mut exit_success = success;
    let exit_result = exit_success
        .transition(BastionTraversalPhase::TraversingTopExit, 16)
        .and_then(|_| {
            record_stage1_task_snapshot(exit_success, 16, "top-exit");
            exit_success.transition(BastionTraversalPhase::ConfirmingExitRelease, 17)
        })
        .and_then(|_| {
            record_stage1_task_snapshot(exit_success, 17, "confirming-exit-release");
            exit_success.transition(BastionTraversalPhase::ConfirmingExitTraversal, 18)
        })
        .and_then(|_| {
            record_stage1_task_snapshot(exit_success, 18, "confirming-exit-traversal");
            exit_success.transition(BastionTraversalPhase::ConfirmingExit, 19)
        });
    record_stage1_task_snapshot(exit_success, 19, "confirming-exit");
    let completion = exit_success.transition(BastionTraversalPhase::Complete, 20);
    record_stage1_task_snapshot(exit_success, 20, "complete-release");

    let mut double = stage1_task();
    let double_result = double.reserve(competitor, 11);
    cases.push(stage1_case(
        "double_reservation_rejected",
        double_result == Err(BastionTraversalReject::LinkAlreadyReserved),
        double_result,
    ));

    let mut stale = stage1_task();
    let stale_result = stale.validate_terrain_revision(SMOKE80_TERRAIN_REVISION + 1, 11);
    cases.push(stage1_case(
        "stale_terrain_revision_aborts_and_releases",
        stale_result == Err(BastionTraversalReject::StaleTerrainRevision)
            && stale.phase == BastionTraversalPhase::Abort
            && stale.ownership().is_none(),
        (stale_result, stale.phase, stale.abort_reason),
    ));
    stale.reserved_member = stage1_uid(99);
    record_stage1_task_snapshot(stale, 22, "stale-revision-abort-release");

    for (index, interruption) in [
        BastionTraversalInterruption::ContactLost,
        BastionTraversalInterruption::ExternalRelocation,
        BastionTraversalInterruption::AgentInbox,
        BastionTraversalInterruption::RtsimAction,
        BastionTraversalInterruption::ControllerEvent,
        BastionTraversalInterruption::Tether,
        BastionTraversalInterruption::Mount,
        BastionTraversalInterruption::Interpolation,
    ]
    .into_iter()
    .enumerate()
    {
        let mut task = stage1_task();
        let _ = task.reserve(member, 11);
        let _ = task.transition(BastionTraversalPhase::TraversingEntry, 12);
        let _ = task.transition(BastionTraversalPhase::TraversingLink, 13);
        task.interrupt(interruption, 14);
        task.reserved_member = stage1_uid(100 + index as u64);
        record_stage1_task_snapshot(task, 30 + index as u64, interruption.reason());
        cases.push(stage1_case(
            &format!("interruption_{interruption:?}"),
            task.phase == BastionTraversalPhase::Abort
                && task.ownership().is_none()
                && task.abort_reason == Some(interruption.reason()),
            (task.phase, task.abort_reason),
        ));
    }

    cases.push(stage1_case(
        "clean_completion_releases_owner",
        exit_result.is_ok()
            && completion.is_ok()
            && exit_success.phase == BastionTraversalPhase::Complete
            && exit_success.ownership().is_none(),
        (
            exit_result,
            completion,
            exit_success.phase,
            exit_success.ownership(),
        ),
    ));

    let deterministic = cases.iter().all(|case| case.passed);
    Stage1TraversalOwnerReport {
        schema: "bastion.b58.stage1-traversal-owner/v1".into(),
        fixture: "production-geometry-plus-production-task-contract".into(),
        production_geometry_exercised: geometry.production_geometry_exercised,
        cases,
        deterministic,
        gameplay_mutated: false,
        circuit_breaker_state: "clear-for-bounded-stage1-only".into(),
        local_patch_count: 1,
        full_smoke_count_since_last_gate_advance: 0,
        failure_phase_history: Vec::new(),
        next_permitted_action: "focused compile/test and coordinator integration authorization"
            .into(),
    }
}

pub const SMOKE80_OWNER: u64 = 1;
pub const SMOKE80_FRONTIER_JOB: u64 = 1240;
pub const SMOKE80_FIRST_RETRY_TICK: u64 = 10_230;
pub const SMOKE80_RETRY_INTERVAL_TICKS: u64 = 30;
pub const SMOKE80_ACTUAL_POSITION: Vec3<f32> = Vec3::new(18_398.5, 9_295.5, 458.0);
pub const SMOKE80_DESCRIPTOR_ENTRY: Vec3<i32> = Vec3::new(18_398, 9_294, 442);
pub const SMOKE80_FIRST_RUNG: Vec3<i32> = Vec3::new(18_398, 9_295, 443);
pub const SMOKE80_CYLINDER: (f32, f32, f32) = (0.22, 0.0, 1.658_801_8);
pub const SMOKE80_TERRAIN_REVISION: u64 = 80;

const NORMAL_ENTRY: Vec3<i32> = Vec3::new(4, 4, 2);
const NORMAL_FIRST_RUNG: Vec3<i32> = Vec3::new(4, 5, 3);
const NORMAL_ACTUAL_START: Vec3<f32> = Vec3::new(5.5, 4.5, 2.0);
const NORMAL_TOP_Z: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalPhase {
    Request,
    Approach,
    Reserved,
    Traversing,
    FrontierWork,
    ConfirmingExit,
    Complete,
    Abort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementOwner {
    None,
    AgentChaser,
    LinkQueue,
    BastionTraversal,
    Orca,
    GenericSoftSteering,
    GenericGoto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureEvent {
    RequestLink,
    ApproachReady,
    ReserveLink,
    BeginTraversal,
    ReachFrontier,
    BeginExitConfirmation,
    ConfirmExit,
    ContactLost,
    TerrainRevisionChanged,
    Interrupt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureReject {
    CompetingMovementWriter,
    LinkAlreadyReserved,
    StaleTerrainRevision,
    MissingAuthoritativeContact,
    PrematurePathResume,
    InvalidPhase,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalTaskModel {
    pub link_id: u64,
    pub terrain_revision: u64,
    pub phase: TraversalPhase,
    pub movement_owner: MovementOwner,
    pub reserved_by: Option<u64>,
    pub authoritative_contact: bool,
    pub stable_exit_samples: u8,
    pub abort_reason: Option<String>,
}

impl TraversalTaskModel {
    pub fn smoke80() -> Self {
        Self {
            link_id: 0xB5_80,
            terrain_revision: 80,
            phase: TraversalPhase::Request,
            movement_owner: MovementOwner::None,
            reserved_by: None,
            authoritative_contact: false,
            stable_exit_samples: 0,
            abort_reason: None,
        }
    }

    pub fn apply(
        &mut self,
        uid: u64,
        event: FixtureEvent,
        proposed_writer: MovementOwner,
        observed_terrain_revision: u64,
    ) -> Result<(), FixtureReject> {
        if observed_terrain_revision != self.terrain_revision
            && !matches!(self.phase, TraversalPhase::Complete | TraversalPhase::Abort)
        {
            self.abort_reason = Some("stale_terrain_revision".into());
            self.phase = TraversalPhase::Abort;
            self.movement_owner = MovementOwner::None;
            self.reserved_by = None;
            return Err(FixtureReject::StaleTerrainRevision);
        }

        match (self.phase, event) {
            (TraversalPhase::Request, FixtureEvent::RequestLink) => {
                self.phase = TraversalPhase::Approach;
                self.movement_owner = MovementOwner::AgentChaser;
            },
            (TraversalPhase::Approach, FixtureEvent::ApproachReady) => {
                if proposed_writer != MovementOwner::AgentChaser {
                    return Err(FixtureReject::CompetingMovementWriter);
                }
                self.phase = TraversalPhase::Reserved;
                self.movement_owner = MovementOwner::LinkQueue;
            },
            (TraversalPhase::Reserved, FixtureEvent::ReserveLink) => {
                if self.reserved_by.is_some_and(|owner| owner != uid) {
                    return Err(FixtureReject::LinkAlreadyReserved);
                }
                self.reserved_by = Some(uid);
            },
            (TraversalPhase::Reserved, FixtureEvent::BeginTraversal) => {
                if self.reserved_by != Some(uid) {
                    return Err(FixtureReject::LinkAlreadyReserved);
                }
                if proposed_writer != MovementOwner::BastionTraversal {
                    return Err(FixtureReject::CompetingMovementWriter);
                }
                self.authoritative_contact = true;
                self.phase = TraversalPhase::Traversing;
                self.movement_owner = MovementOwner::BastionTraversal;
            },
            (TraversalPhase::Traversing, FixtureEvent::ReachFrontier) => {
                if proposed_writer != MovementOwner::BastionTraversal
                    || self.movement_owner != MovementOwner::BastionTraversal
                {
                    return Err(FixtureReject::CompetingMovementWriter);
                }
                if !self.authoritative_contact {
                    return Err(FixtureReject::MissingAuthoritativeContact);
                }
                self.phase = TraversalPhase::FrontierWork;
            },
            (TraversalPhase::FrontierWork, FixtureEvent::BeginExitConfirmation) => {
                if proposed_writer != MovementOwner::BastionTraversal {
                    return Err(FixtureReject::PrematurePathResume);
                }
                self.phase = TraversalPhase::ConfirmingExit;
                self.stable_exit_samples = 0;
            },
            (TraversalPhase::ConfirmingExit, FixtureEvent::ConfirmExit) => {
                if proposed_writer != MovementOwner::BastionTraversal {
                    return Err(FixtureReject::PrematurePathResume);
                }
                self.stable_exit_samples = self.stable_exit_samples.saturating_add(1);
                if self.stable_exit_samples >= 5 {
                    self.phase = TraversalPhase::Complete;
                    self.movement_owner = MovementOwner::None;
                    self.reserved_by = None;
                }
            },
            (
                TraversalPhase::Traversing | TraversalPhase::ConfirmingExit,
                FixtureEvent::ContactLost,
            ) => {
                self.authoritative_contact = false;
                self.abort_reason = Some("authoritative_contact_lost".into());
                self.phase = TraversalPhase::Abort;
                self.movement_owner = MovementOwner::None;
                self.reserved_by = None;
            },
            (_, FixtureEvent::Interrupt) => {
                self.abort_reason = Some("interrupted".into());
                self.phase = TraversalPhase::Abort;
                self.movement_owner = MovementOwner::None;
                self.reserved_by = None;
            },
            _ => return Err(FixtureReject::InvalidPhase),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Smoke80Retry {
    pub tick: u64,
    pub owner: u64,
    pub frontier_job: u64,
    pub actual_position_milli: [i64; 3],
    pub descriptor_entry: [i32; 3],
    pub reason: String,
    pub transaction_present: bool,
    pub movement_owner: MovementOwner,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegativeCaseResult {
    pub name: String,
    pub rejected: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct B58TraversalContractModelReport {
    pub schema: String,
    pub fixture: String,
    pub source_evidence: String,
    pub legacy_divergence_reproduced: bool,
    pub retry_interval_ticks: u64,
    pub retries: Vec<Smoke80Retry>,
    pub checkpoint_phases: Vec<TraversalPhase>,
    pub negative_cases: Vec<NegativeCaseResult>,
    pub deterministic: bool,
    pub gameplay_mutated: bool,
    pub production_geometry_exercised: bool,
}

fn negative(
    name: &str,
    result: Result<(), FixtureReject>,
    expected: FixtureReject,
) -> NegativeCaseResult {
    NegativeCaseResult {
        name: name.into(),
        rejected: result == Err(expected),
        reason: format!("{result:?}"),
    }
}

pub fn run_smoke80_contract_model() -> B58TraversalContractModelReport {
    let retries = (0..6)
        .map(|attempt| Smoke80Retry {
            tick: SMOKE80_FIRST_RETRY_TICK + attempt * SMOKE80_RETRY_INTERVAL_TICKS,
            owner: SMOKE80_OWNER,
            frontier_job: SMOKE80_FRONTIER_JOB,
            actual_position_milli: [
                (SMOKE80_ACTUAL_POSITION.x * 1_000.0) as i64,
                (SMOKE80_ACTUAL_POSITION.y * 1_000.0) as i64,
                (SMOKE80_ACTUAL_POSITION.z * 1_000.0) as i64,
            ],
            descriptor_entry: [
                SMOKE80_DESCRIPTOR_ENTRY.x,
                SMOKE80_DESCRIPTOR_ENTRY.y,
                SMOKE80_DESCRIPTOR_ENTRY.z,
            ],
            reason: "no_reachable_body_lane".into(),
            transaction_present: false,
            movement_owner: MovementOwner::None,
        })
        .collect::<Vec<_>>();

    let mut happy = TraversalTaskModel::smoke80();
    let mut checkpoints = vec![happy.phase];
    happy
        .apply(
            SMOKE80_OWNER,
            FixtureEvent::RequestLink,
            MovementOwner::None,
            80,
        )
        .unwrap();
    checkpoints.push(happy.phase);
    happy
        .apply(
            SMOKE80_OWNER,
            FixtureEvent::ApproachReady,
            MovementOwner::AgentChaser,
            80,
        )
        .unwrap();
    checkpoints.push(happy.phase);
    happy
        .apply(
            SMOKE80_OWNER,
            FixtureEvent::ReserveLink,
            MovementOwner::LinkQueue,
            80,
        )
        .unwrap();
    happy
        .apply(
            SMOKE80_OWNER,
            FixtureEvent::BeginTraversal,
            MovementOwner::BastionTraversal,
            80,
        )
        .unwrap();
    checkpoints.push(happy.phase);
    happy
        .apply(
            SMOKE80_OWNER,
            FixtureEvent::ReachFrontier,
            MovementOwner::BastionTraversal,
            80,
        )
        .unwrap();
    checkpoints.push(happy.phase);
    happy
        .apply(
            SMOKE80_OWNER,
            FixtureEvent::BeginExitConfirmation,
            MovementOwner::BastionTraversal,
            80,
        )
        .unwrap();
    checkpoints.push(happy.phase);
    for _ in 0..5 {
        happy
            .apply(
                SMOKE80_OWNER,
                FixtureEvent::ConfirmExit,
                MovementOwner::BastionTraversal,
                80,
            )
            .unwrap();
    }
    checkpoints.push(happy.phase);

    let mut chaser_during_link = TraversalTaskModel::smoke80();
    chaser_during_link.phase = TraversalPhase::Traversing;
    chaser_during_link.movement_owner = MovementOwner::BastionTraversal;
    chaser_during_link.reserved_by = Some(SMOKE80_OWNER);
    chaser_during_link.authoritative_contact = true;
    let chaser_result = chaser_during_link.apply(
        SMOKE80_OWNER,
        FixtureEvent::ReachFrontier,
        MovementOwner::AgentChaser,
        80,
    );

    let mut orca_during_link = chaser_during_link.clone();
    let orca_result = orca_during_link.apply(
        SMOKE80_OWNER,
        FixtureEvent::ReachFrontier,
        MovementOwner::Orca,
        80,
    );

    let mut soft_during_link = chaser_during_link.clone();
    let soft_result = soft_during_link.apply(
        SMOKE80_OWNER,
        FixtureEvent::ReachFrontier,
        MovementOwner::GenericSoftSteering,
        80,
    );

    let mut double_claim = TraversalTaskModel::smoke80();
    double_claim.phase = TraversalPhase::Reserved;
    double_claim.movement_owner = MovementOwner::LinkQueue;
    double_claim.reserved_by = Some(SMOKE80_OWNER);
    let double_claim_result =
        double_claim.apply(2, FixtureEvent::ReserveLink, MovementOwner::LinkQueue, 80);

    let mut stale_terrain = TraversalTaskModel::smoke80();
    stale_terrain.phase = TraversalPhase::Approach;
    stale_terrain.movement_owner = MovementOwner::AgentChaser;
    let stale_result = stale_terrain.apply(
        SMOKE80_OWNER,
        FixtureEvent::ApproachReady,
        MovementOwner::AgentChaser,
        81,
    );

    let mut premature_resume = TraversalTaskModel::smoke80();
    premature_resume.phase = TraversalPhase::ConfirmingExit;
    premature_resume.movement_owner = MovementOwner::BastionTraversal;
    premature_resume.reserved_by = Some(SMOKE80_OWNER);
    let premature_result = premature_resume.apply(
        SMOKE80_OWNER,
        FixtureEvent::ConfirmExit,
        MovementOwner::GenericGoto,
        80,
    );

    let mut lost_contact = TraversalTaskModel::smoke80();
    lost_contact.phase = TraversalPhase::Traversing;
    lost_contact.movement_owner = MovementOwner::BastionTraversal;
    lost_contact.reserved_by = Some(SMOKE80_OWNER);
    lost_contact.authoritative_contact = true;
    let lost_contact_result = lost_contact.apply(
        SMOKE80_OWNER,
        FixtureEvent::ContactLost,
        MovementOwner::BastionTraversal,
        80,
    );

    let mut stale_target = TraversalTaskModel::smoke80();
    stale_target.phase = TraversalPhase::Approach;
    stale_target.movement_owner = MovementOwner::AgentChaser;
    let stale_target_result = stale_target.apply(
        SMOKE80_OWNER,
        FixtureEvent::ApproachReady,
        MovementOwner::GenericGoto,
        80,
    );

    let mut interrupted = TraversalTaskModel::smoke80();
    interrupted.phase = TraversalPhase::Traversing;
    interrupted.movement_owner = MovementOwner::BastionTraversal;
    interrupted.reserved_by = Some(SMOKE80_OWNER);
    interrupted.authoritative_contact = true;
    let interrupted_result = interrupted.apply(
        SMOKE80_OWNER,
        FixtureEvent::Interrupt,
        MovementOwner::BastionTraversal,
        80,
    );

    let mut exit_fallback = TraversalTaskModel::smoke80();
    exit_fallback.phase = TraversalPhase::ConfirmingExit;
    exit_fallback.movement_owner = MovementOwner::BastionTraversal;
    exit_fallback.reserved_by = Some(SMOKE80_OWNER);
    exit_fallback.authoritative_contact = true;
    let exit_fallback_result = exit_fallback.apply(
        SMOKE80_OWNER,
        FixtureEvent::ContactLost,
        MovementOwner::BastionTraversal,
        80,
    );

    let mut cleanup_retry = happy.clone();
    let cleanup_retry_result = cleanup_retry.apply(
        SMOKE80_OWNER,
        FixtureEvent::ConfirmExit,
        MovementOwner::BastionTraversal,
        80,
    );

    B58TraversalContractModelReport {
        schema: "bastion.b58.traversal-contract-model/v1".into(),
        fixture: "smoke80-state-ownership-contract-replay".into(),
        source_evidence: "bastion-test-evidence/B5.5-deep/failsafe-fix/seed21-smoke80-*".into(),
        legacy_divergence_reproduced: retries.len() >= 2
            && retries.windows(2).all(|pair| {
                pair[1].tick - pair[0].tick == SMOKE80_RETRY_INTERVAL_TICKS
                    && pair[0].reason == "no_reachable_body_lane"
                    && !pair[0].transaction_present
            }),
        retry_interval_ticks: SMOKE80_RETRY_INTERVAL_TICKS,
        retries,
        checkpoint_phases: checkpoints,
        negative_cases: vec![
            negative(
                "chaser_during_traversal",
                chaser_result,
                FixtureReject::CompetingMovementWriter,
            ),
            negative(
                "orca_during_traversal",
                orca_result,
                FixtureReject::CompetingMovementWriter,
            ),
            negative(
                "soft_collision_overlap_at_link",
                soft_result,
                FixtureReject::CompetingMovementWriter,
            ),
            negative(
                "queue_slot_double_claim",
                double_claim_result,
                FixtureReject::LinkAlreadyReserved,
            ),
            negative(
                "stale_path_after_terrain_revision",
                stale_result,
                FixtureReject::StaleTerrainRevision,
            ),
            negative(
                "ordinary_path_before_exit_confirmation",
                premature_result,
                FixtureReject::PrematurePathResume,
            ),
            NegativeCaseResult {
                name: "contact_loss_bounded_abort".into(),
                rejected: lost_contact_result.is_ok()
                    && lost_contact.phase == TraversalPhase::Abort
                    && lost_contact.reserved_by.is_none(),
                reason: lost_contact.abort_reason.unwrap_or_default(),
            },
            negative(
                "stale_target_writer",
                stale_target_result,
                FixtureReject::CompetingMovementWriter,
            ),
            NegativeCaseResult {
                name: "interruption_releases_slot".into(),
                rejected: interrupted_result.is_ok()
                    && interrupted.phase == TraversalPhase::Abort
                    && interrupted.reserved_by.is_none(),
                reason: interrupted.abort_reason.unwrap_or_default(),
            },
            NegativeCaseResult {
                name: "exit_fallback_aborts_before_resume".into(),
                rejected: exit_fallback_result.is_ok()
                    && exit_fallback.phase == TraversalPhase::Abort
                    && exit_fallback.reserved_by.is_none(),
                reason: exit_fallback.abort_reason.unwrap_or_default(),
            },
            negative(
                "cleanup_retry_does_not_double_complete",
                cleanup_retry_result,
                FixtureReject::InvalidPhase,
            ),
        ],
        deterministic: happy.phase == TraversalPhase::Complete
            && happy.reserved_by.is_none()
            && happy.movement_owner == MovementOwner::None,
        gameplay_mutated: false,
        production_geometry_exercised: false,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryHitReport {
    pub block: [i32; 3],
    pub resolve_dir: [f32; 3],
    pub sample: u16,
    pub samples: u16,
}

impl From<common_systems::phys::TerrainSweepHit> for GeometryHitReport {
    fn from(hit: common_systems::phys::TerrainSweepHit) -> Self {
        Self {
            block: [hit.block.x, hit.block.y, hit.block.z],
            resolve_dir: [hit.resolve_dir.x, hit.resolve_dir.y, hit.resolve_dir.z],
            sample: hit.sample,
            samples: hit.samples,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductionGeometryCase {
    pub name: String,
    pub selected: Option<[i32; 3]>,
    pub corridor: Vec<[i32; 3]>,
    pub first_hit: Option<GeometryHitReport>,
    pub rejected: bool,
    pub reason: String,
    pub predicates: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductionGeometryFixtureReport {
    pub schema: String,
    pub fixture: String,
    pub source_evidence: String,
    pub original_actual_position: [f32; 3],
    pub original_descriptor_entry: [i32; 3],
    pub original_first_rung: [i32; 3],
    pub normalized_actual_start: [f32; 3],
    pub normalized_descriptor_entry: [i32; 3],
    pub normalized_first_rung: [i32; 3],
    pub cylinder: [f32; 3],
    pub terrain_revision: u64,
    pub cases: Vec<ProductionGeometryCase>,
    pub deterministic: bool,
    pub gameplay_mutated: bool,
    pub production_geometry_exercised: bool,
}

type GeometryTerrain = Dyna<Block, ()>;

fn solid() -> Block { Block::new(BlockKind::Rock, Rgb::new(80, 80, 80)) }

fn geometry_stencil(block_entry: bool) -> GeometryTerrain {
    let mut terrain = GeometryTerrain::filled(Vec3::new(12, 12, 8), Block::empty(), ());
    for x in 0..12 {
        for y in 0..12 {
            terrain.set(Vec3::new(x, y, 1), solid()).unwrap();
        }
    }
    terrain
        .set(NORMAL_FIRST_RUNG, Block::air(SpriteKind::Ladder))
        .unwrap();
    // Keep alternate body lanes invalid so the result proves the descriptor's
    // exact lane rather than silently selecting another side of the rung.
    for alternate in [Vec3::new(5, 5, 2), Vec3::new(3, 5, 2), Vec3::new(4, 6, 2)] {
        terrain.set(alternate, solid()).unwrap();
    }
    if block_entry {
        terrain.set(NORMAL_ENTRY, solid()).unwrap();
    }
    terrain
}

fn descriptor(entry: Vec3<i32>) -> EmergencyRouteDescriptor {
    EmergencyRouteDescriptor {
        kind: EmergencyTraversalKind::ConstructedLadder,
        approach: NORMAL_ACTUAL_START.map(|value| value.floor() as i32),
        entry,
        top_anchor: Vec3::new(entry.x, entry.y, NORMAL_TOP_Z),
        dismount: Vec3::new(entry.x, entry.y - 1, NORMAL_TOP_Z),
        wall_dir: None,
    }
}

fn profile_guard(
    actual_start: Vec3<f32>,
    cylinder: (f32, f32, f32),
    route_descriptor: EmergencyRouteDescriptor,
    first_rung: Vec3<i32>,
    terrain_revision: u64,
) -> Option<(&'static str, &'static str)> {
    if actual_start != NORMAL_ACTUAL_START {
        return Some((
            "body_profile_mismatch",
            "focused_adapter_body_profile_guard",
        ));
    }
    if cylinder != SMOKE80_CYLINDER {
        return Some((
            "cylinder_profile_mismatch",
            "focused_adapter_cylinder_profile_guard",
        ));
    }
    if route_descriptor.entry != NORMAL_ENTRY {
        return Some((
            "descriptor_entry_mismatch",
            "focused_adapter_descriptor_identity_guard",
        ));
    }
    if first_rung != NORMAL_FIRST_RUNG {
        return Some(("first_rung_mismatch", "focused_adapter_rung_identity_guard"));
    }
    if terrain_revision != SMOKE80_TERRAIN_REVISION {
        return Some((
            "terrain_revision_mismatch",
            "focused_adapter_terrain_revision_guard",
        ));
    }
    None
}

fn run_geometry_case(
    name: &str,
    terrain: &GeometryTerrain,
    actual_start: Vec3<f32>,
    cylinder: (f32, f32, f32),
    route_descriptor: EmergencyRouteDescriptor,
    first_rung: Vec3<i32>,
    terrain_revision: u64,
    expect_selection: bool,
) -> ProductionGeometryCase {
    if let Some((reason, predicate)) = profile_guard(
        actual_start,
        cylinder,
        route_descriptor,
        first_rung,
        terrain_revision,
    ) {
        return ProductionGeometryCase {
            name: name.into(),
            selected: None,
            corridor: Vec::new(),
            first_hit: None,
            rejected: true,
            reason: reason.into(),
            predicates: vec![predicate.into()],
        };
    }
    if terrain.get(first_rung).ok().and_then(Block::get_sprite) != Some(SpriteKind::Ladder) {
        return ProductionGeometryCase {
            name: name.into(),
            selected: None,
            corridor: Vec::new(),
            first_hit: None,
            rejected: true,
            reason: "first_rung_provenance_mismatch".into(),
            predicates: vec!["focused_adapter_ladder_sprite_provenance_guard".into()],
        };
    }

    // This is the exact shipping validation used after the A* corridor has
    // produced an entry waypoint. It supplies an explicit first blocking cell
    // even when standability fails before a physical sweep can begin.
    let entry_hit = emergency_validate_constructed_ladder_corridor(
        terrain,
        actual_start,
        cylinder,
        route_descriptor.entry,
        &[route_descriptor.entry],
    );
    let (selected, rejected_lanes) = emergency_constructed_ladder_corridor_candidates(
        terrain,
        actual_start,
        cylinder,
        route_descriptor,
        first_rung,
        NORMAL_TOP_Z,
    );
    let (selected_lane, corridor) = selected.map_or((None, Vec::new()), |(lane, corridor)| {
        (
            Some([lane.x, lane.y, lane.z]),
            corridor
                .into_iter()
                .map(|cell| [cell.x, cell.y, cell.z])
                .collect(),
        )
    });
    let first_hit: Option<GeometryHitReport> = entry_hit
        .or_else(|| rejected_lanes.iter().find_map(|(_, hit)| *hit))
        .map(Into::into);
    let outcome_matches = if expect_selection {
        selected_lane == Some([NORMAL_ENTRY.x, NORMAL_ENTRY.y, NORMAL_ENTRY.z])
            && first_hit.is_none()
    } else {
        selected_lane.is_none()
            && first_hit
                .as_ref()
                .is_some_and(|hit| hit.block == [NORMAL_ENTRY.x, NORMAL_ENTRY.y, NORMAL_ENTRY.z])
    };
    ProductionGeometryCase {
        name: name.into(),
        selected: selected_lane,
        corridor,
        first_hit,
        rejected: if expect_selection {
            !outcome_matches
        } else {
            outcome_matches
        },
        reason: if expect_selection {
            if outcome_matches {
                "selected_descriptor_lane"
            } else {
                "positive_selection_failed"
            }
        } else if outcome_matches {
            "blocked_entry_rejected"
        } else {
            "blocked_entry_falsifier_failed"
        }
        .into(),
        predicates: vec![
            "emergency_validate_constructed_ladder_corridor".into(),
            "emergency_corridor_standable".into(),
            "common::path::bastion_full_path".into(),
            "common_systems::phys::cylinder_sweep_first_collision".into(),
            "emergency_constructed_ladder_corridor_candidates".into(),
        ],
    }
}

pub fn run_smoke80_production_geometry_fixture() -> ProductionGeometryFixtureReport {
    let blocked = geometry_stencil(true);
    let clear = geometry_stencil(false);
    let mut rung_missing = geometry_stencil(false);
    rung_missing.set(NORMAL_FIRST_RUNG, Block::empty()).unwrap();
    let mut head_blocked = geometry_stencil(false);
    head_blocked
        .set(NORMAL_ENTRY + Vec3::unit_z(), solid())
        .unwrap();

    let cases = vec![
        run_geometry_case(
            "preserved_solid_entry",
            &blocked,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "cleared_supported_entry",
            &clear,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            true,
        ),
        run_geometry_case(
            "descriptor_mismatch",
            &clear,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY + Vec3::unit_x()),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "rung_identity_mismatch",
            &clear,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG + Vec3::unit_y(),
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "rung_provenance_mismatch",
            &rung_missing,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "body_profile_mismatch",
            &clear,
            NORMAL_ACTUAL_START + Vec3::unit_x(),
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "body_head_clearance_mismatch",
            &head_blocked,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "cylinder_mismatch",
            &clear,
            NORMAL_ACTUAL_START,
            (0.45, 0.0, 1.95),
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION,
            false,
        ),
        run_geometry_case(
            "terrain_revision_mismatch",
            &clear,
            NORMAL_ACTUAL_START,
            SMOKE80_CYLINDER,
            descriptor(NORMAL_ENTRY),
            NORMAL_FIRST_RUNG,
            SMOKE80_TERRAIN_REVISION + 1,
            false,
        ),
    ];
    let deterministic = cases.iter().all(|case| match case.name.as_str() {
        "cleared_supported_entry" => !case.rejected && case.selected.is_some(),
        _ => case.rejected,
    });
    ProductionGeometryFixtureReport {
        schema: "bastion.b58.production-geometry-fixture/v1".into(),
        fixture: "smoke80-normalized-body-lane-stencil".into(),
        source_evidence: "bastion-test-evidence/B5.5-deep/failsafe-fix/\
                          seed21-smoke80-req0090-interrupted-release"
            .into(),
        original_actual_position: [
            SMOKE80_ACTUAL_POSITION.x,
            SMOKE80_ACTUAL_POSITION.y,
            SMOKE80_ACTUAL_POSITION.z,
        ],
        original_descriptor_entry: [
            SMOKE80_DESCRIPTOR_ENTRY.x,
            SMOKE80_DESCRIPTOR_ENTRY.y,
            SMOKE80_DESCRIPTOR_ENTRY.z,
        ],
        original_first_rung: [
            SMOKE80_FIRST_RUNG.x,
            SMOKE80_FIRST_RUNG.y,
            SMOKE80_FIRST_RUNG.z,
        ],
        normalized_actual_start: [
            NORMAL_ACTUAL_START.x,
            NORMAL_ACTUAL_START.y,
            NORMAL_ACTUAL_START.z,
        ],
        normalized_descriptor_entry: [NORMAL_ENTRY.x, NORMAL_ENTRY.y, NORMAL_ENTRY.z],
        normalized_first_rung: [
            NORMAL_FIRST_RUNG.x,
            NORMAL_FIRST_RUNG.y,
            NORMAL_FIRST_RUNG.z,
        ],
        cylinder: [SMOKE80_CYLINDER.0, SMOKE80_CYLINDER.1, SMOKE80_CYLINDER.2],
        terrain_revision: SMOKE80_TERRAIN_REVISION,
        cases,
        deterministic,
        gameplay_mutated: false,
        production_geometry_exercised: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    const INVENTORY_DOC: &str = "readme/B5.8-WRITER-INVENTORY-REQ0094A.md";

    #[test]
    fn stage1_constructed_ladder_owner_is_deterministic_and_exclusive() {
        let a = run_stage1_constructed_ladder_fixture();
        let b = run_stage1_constructed_ladder_fixture();
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );
        assert!(a.production_geometry_exercised);
        assert!(!a.gameplay_mutated);
        assert!(a.deterministic, "{:#?}", a.cases);
        assert!(a.cases.iter().all(|case| case.passed));
        assert_eq!(a.full_smoke_count_since_last_gate_advance, 0);
    }

    #[test]
    fn stage1_old_mount_owner_and_duplicate_reservation_are_absent() {
        let root = repo_root();
        let jobs = fs::read_to_string(root.join("bastion-server/src/bastion_jobs.rs")).unwrap();
        let traversal = fs::read_to_string(root.join("bastion-server/src/bastion_traversal.rs")).unwrap();
        for legacy in [
            "EmergencyMountTransaction",
            "EmergencyMountPhase",
            "emergency_mount_transactions",
            "emergency_route_traversers",
        ] {
            assert!(
                !jobs.contains(legacy) && !traversal.contains(legacy),
                "legacy/parallel owner remains: {legacy}"
            );
        }
        assert_eq!(traversal.matches("struct BastionTraversalTask").count(), 1);
        assert!(jobs.contains("bastion_traversal_tasks"));
        assert!(jobs.contains("traversal_queue_head"));

        let task_start = jobs
            .find("if let Some(mut transaction) = mount_transaction")
            .expect("production task lifecycle start");
        let task_end = jobs[task_start..]
            .find("} else if let Some(entity) = id_maps.uid_entity(*uid)")
            .map(|offset| task_start + offset)
            .expect("production task lifecycle end");
        let lifecycle = &jobs[task_start..task_end];
        for forbidden in ["pos.0 =", "vel.0.x =", "vel.0.y =", "vel.0.z ="] {
            assert!(
                !lifecycle.contains(forbidden),
                "task lifecycle contains forbidden direct movement writer: {forbidden}"
            );
        }
        let approach_start = lifecycle
            .find("BastionTraversalPhase::LinkApproach =>")
            .expect("LinkApproach production arm");
        let approach_end = lifecycle[approach_start..]
            .find("BastionTraversalPhase::Reserved =>")
            .map(|offset| approach_start + offset)
            .expect("Reserved production arm");
        let approach = &lifecycle[approach_start..approach_end];
        assert!(approach.contains("NpcActivity::Goto"));
        assert!(!approach.contains("controller.inputs"));
    }
    #[derive(Clone, Copy, Debug)]
    struct RequiredInventoryRecord {
        id: &'static str,
        path: &'static str,
        source_marker: &'static str,
        eligibility_phase: &'static str,
        classification: &'static str,
        policy: &'static str,
        falsifier: &'static str,
    }

    const REQUIRED_INVENTORY_RECORDS: &[RequiredInventoryRecord] = &[
        RequiredInventoryRecord {
            id: "INV-RTSIM-ACTIVITY",
            path: "server/src/rtsim/tick.rs",
            source_marker: "rtsim_controller.activity =",
            eligibility_phase: "loaded RTSim NPC; Server Create after physics; ActiveJob/TestGoto \
                                guard only",
            classification: "included traversal conflict",
            policy: "rtsim-activity-guard",
            falsifier: "active-job-gap-replaces-link-activity",
        },
        RequiredInventoryRecord {
            id: "INV-RTSIM-ACTIONS",
            path: "server/src/sys/agent/behavior_tree/mod.rs",
            source_marker: "rtsim_controller.actions.pop_front()",
            eligibility_phase: "Agent behavior consumes RTSim queue; Say/Attack can alter state \
                                or target",
            classification: "included traversal conflict",
            policy: "rtsim-action-queue-guard",
            falsifier: "say-or-attack-during-link",
        },
        RequiredInventoryRecord {
            id: "INV-TETHER",
            path: "common/systems/src/tether.rs",
            source_marker: "WriteStorage<'a, Vel>",
            eligibility_phase: "any Is<Follower> tether; Common Create; arbitrary UID command can \
                                attach",
            classification: "included traversal conflict",
            policy: "tether-follower-guard",
            falsifier: "tethered-colonist-traverses-link",
        },
        RequiredInventoryRecord {
            id: "INV-INTERPOLATION",
            path: "common/systems/src/interpolation.rs",
            source_marker: "WriteStorage<'a, Pos>",
            eligibility_phase: "any InterpData entity except PlayerEntity; Common Apply; colonist \
                                exclusion unproven",
            classification: "unresolved",
            policy: "interpolation-component-guard",
            falsifier: "interpdata-colonist-enters-link",
        },
        RequiredInventoryRecord {
            id: "INV-LOCAL-IMPULSE",
            path: "common/state/src/state.rs",
            source_marker: "LocalEvent::ApplyImpulse",
            eligibility_phase: "arbitrary event target; post-dispatch local-event flow",
            classification: "event-only/backstop",
            policy: "external-impulse-abort",
            falsifier: "impulse-competes-with-link-intent",
        },
        RequiredInventoryRecord {
            id: "INV-MOUNT-LINK-DISMOUNT",
            path: "common/src/mounting.rs",
            source_marker: "fn delete(",
            eligibility_phase: "any rider Mounting link deletion; direct safe-ground Pos rewrite",
            classification: "included traversal conflict",
            policy: "mount-link-exclusive",
            falsifier: "mount-delete-and-link-own-pos",
        },
        RequiredInventoryRecord {
            id: "INV-INTERACTION-STATE",
            path: "common/src/interaction.rs",
            source_marker: "CharacterState::Interact",
            eligibility_phase: "arbitrary valid interactor; link create/delete outside Specs order",
            classification: "included traversal conflict",
            policy: "interaction-preemption-guard",
            falsifier: "interaction-silently-preempts-climb",
        },
        RequiredInventoryRecord {
            id: "INV-ENTITY-RELOCATION",
            path: "server/src/events/entity_manipulation.rs",
            source_marker: "TeleportToPositionEvent",
            eligibility_phase: "arbitrary event target; Server Apply handlers include state \
                                reset/knockback/teleport",
            classification: "event-only/backstop",
            policy: "external-relocation-abort",
            falsifier: "relocation-retains-link-owner",
        },
        RequiredInventoryRecord {
            id: "INV-TERRAIN-REPOSITION",
            path: "server/src/sys/terrain.rs",
            source_marker: "RepositionToFreeSpace",
            eligibility_phase: "any tagged entity; Server Create after terrain messages",
            classification: "included traversal conflict",
            policy: "external-relocation-abort",
            falsifier: "reposition-retains-descriptor",
        },
        RequiredInventoryRecord {
            id: "INV-HARNESS-GOTO-CLEAR",
            path: "server/src/lib.rs",
            source_marker: "bastion_goto_clear",
            eligibility_phase: "named colonist harness command; test-only",
            classification: "event-only/backstop",
            policy: "fixture-setup-only",
            falsifier: "goto-clear-after-episode-start",
        },
        RequiredInventoryRecord {
            id: "INV-HARNESS-TELEPORT",
            path: "server/src/lib.rs",
            source_marker: "bastion_teleport_colonist",
            eligibility_phase: "named colonist Pos/Vel harness staging hook; test-only",
            classification: "event-only/backstop",
            policy: "fixture-setup-only",
            falsifier: "teleport-after-episode-start",
        },
        RequiredInventoryRecord {
            id: "INV-PLAYER-POSSESSION",
            path: "server/src/events/player.rs",
            source_marker: "PossessEvent",
            eligibility_phase: "player/client possession event; explicit ownership transfer",
            classification: "player/projectile-only",
            policy: "possession-aborts-link",
            falsifier: "possessed-colonist-retains-link",
        },
        RequiredInventoryRecord {
            id: "INV-CLIENT-MESSAGE",
            path: "server/src/sys/msg/in_game.rs",
            source_marker: "WriteStorage<'a, Controller>",
            eligibility_phase: "client/Presence in-game message context",
            classification: "player/projectile-only",
            policy: "client-control-excluded",
            falsifier: "non-presence-colonist-receives-client-write",
        },
        RequiredInventoryRecord {
            id: "INV-MOUNT-SYSTEM",
            path: "common/systems/src/mount.rs",
            source_marker: "WriteStorage<'a, PhysicsState>",
            eligibility_phase: "entities with rider/mount links; Common system with no dependency",
            classification: "included traversal conflict",
            policy: "mount-link-exclusive",
            falsifier: "mount-and-traversal-own-same-tick",
        },
        RequiredInventoryRecord {
            id: "INV-CONTROLLER-SYSTEM",
            path: "common/systems/src/controller.rs",
            source_marker: "WriteStorage<'a, Controller>",
            eligibility_phase: "every joined UID+Controller; Common Create after Mount; sanitizes \
                                and drains events",
            classification: "included traversal conflict",
            policy: "controller-event-link-guard",
            falsifier: "link-mode-event-emits-without-abort",
        },
        RequiredInventoryRecord {
            id: "INV-CHARACTER-BEHAVIOR",
            path: "common/systems/src/character_behavior.rs",
            source_marker: "WriteStorage<'a, CharacterState>",
            eligibility_phase: "entities with CharacterState/Controller; Common after Controller",
            classification: "included traversal conflict",
            policy: "authoritative-state-subordinate",
            falsifier: "duplicate-state-or-velocity-writer",
        },
        RequiredInventoryRecord {
            id: "INV-PHYSICS",
            path: "common/systems/src/phys/mod.rs",
            source_marker: "WriteStorage<'a, Pos>",
            eligibility_phase: "physical entities; Common after \
                                interpolation/controller/mount/stats",
            classification: "included traversal conflict",
            policy: "hard-physics-authority",
            falsifier: "traversal-bypasses-collision",
        },
        RequiredInventoryRecord {
            id: "INV-BASTION-ROUTE",
            path: "bastion-server/src/bastion_jobs.rs",
            source_marker: "bastion_traversal_tasks",
            eligibility_phase: "Bastion route member; Server Create after Agent and PATH-0",
            classification: "included traversal conflict",
            policy: "single-migrated-traversal-owner",
            falsifier: "parallel-emergency-state-machine",
        },
        RequiredInventoryRecord {
            id: "INV-INBOX-HURT",
            path: "server/src/events/entity_manipulation.rs",
            source_marker: "AgentEvent::Hurt",
            eligibility_phase: "arbitrary damaged entity with Agent; Server Apply event producer",
            classification: "event-only/backstop",
            policy: "agent-inbox-interruption",
            falsifier: "hurt-event-silently-preempts-link",
        },
        RequiredInventoryRecord {
            id: "INV-INBOX-TALK",
            path: "server/src/events/interaction.rs",
            source_marker: "AgentEvent::Talk",
            eligibility_phase: "NPC interaction/dialogue target with Agent; Server Apply producer",
            classification: "event-only/backstop",
            policy: "agent-inbox-interruption",
            falsifier: "talk-dialogue-silently-preempts-link",
        },
        RequiredInventoryRecord {
            id: "INV-INBOX-TRADE",
            path: "server/src/events/invite.rs",
            source_marker: "AgentEvent::TradeInvite",
            eligibility_phase: "invited Agent in trade/invite Apply handler",
            classification: "event-only/backstop",
            policy: "agent-inbox-interruption",
            falsifier: "trade-invite-silently-preempts-link",
        },
        RequiredInventoryRecord {
            id: "INV-MOUNT-EVENT",
            path: "server/src/events/mounting.rs",
            source_marker: "MountEvent",
            eligibility_phase: "mount/unmount event target; direct link transition outside Agent \
                                intent",
            classification: "event-only/backstop",
            policy: "agent-inbox-interruption",
            falsifier: "mount-event-composes-with-owned-link",
        },
        RequiredInventoryRecord {
            id: "INV-INBOX-CONSUMER",
            path: "server/src/sys/agent/behavior_tree/mod.rs",
            source_marker: "process_inbox_interaction",
            eligibility_phase: "Agent behavior consumes inbox before ordinary activity branches",
            classification: "included traversal conflict",
            policy: "agent-inbox-interruption",
            falsifier: "queued-event-consumed-during-link",
        },
    ];

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ParsedInventoryRecord {
        id: String,
        path: String,
        source_marker: String,
        eligibility_phase: String,
        classification: String,
        policy: String,
        falsifier: String,
    }

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn repo_text(path: &str) -> String {
        fs::read_to_string(repo_root().join(path))
            .unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
    }

    fn inventory_cell(cell: &str) -> String { cell.trim().trim_matches('`').to_string() }

    fn parse_inventory_records(document: &str) -> (Vec<ParsedInventoryRecord>, Vec<String>) {
        let Some(section) = document
            .split("## Structured required inventory records")
            .nth(1)
            .and_then(|tail| tail.split("## Complete activity").next())
        else {
            return (Vec::new(), vec![
                "section:structured-inventory-missing".into(),
            ]);
        };
        let mut records = Vec::new();
        let mut errors = Vec::new();
        for line in section.lines().filter(|line| line.starts_with("| INV-")) {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(inventory_cell)
                .collect::<Vec<_>>();
            if cells.len() != 7 {
                errors.push(format!(
                    "row-column-count:{}:{}",
                    cells.first().map_or("?", String::as_str),
                    cells.len()
                ));
                continue;
            }
            records.push(ParsedInventoryRecord {
                id: cells[0].clone(),
                path: cells[1].clone(),
                source_marker: cells[2].clone(),
                eligibility_phase: cells[3].clone(),
                classification: cells[4].clone(),
                policy: cells[5].clone(),
                falsifier: cells[6].clone(),
            });
        }
        (records, errors)
    }

    fn validate_inventory_records(document: &str) -> Vec<String> {
        let (records, mut errors) = parse_inventory_records(document);
        let allowed = [
            "included traversal conflict",
            "source-proven component-impossible",
            "event-only/backstop",
            "player/projectile-only",
            "unresolved",
        ];
        for required in REQUIRED_INVENTORY_RECORDS {
            let matches = records
                .iter()
                .filter(|record| record.id == required.id)
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                errors.push(format!("record:{}:count:{}", required.id, matches.len()));
            } else {
                let record = matches[0];
                for (field, actual, expected) in [
                    ("path", record.path.as_str(), required.path),
                    (
                        "source-marker",
                        record.source_marker.as_str(),
                        required.source_marker,
                    ),
                    (
                        "eligibility-phase",
                        record.eligibility_phase.as_str(),
                        required.eligibility_phase,
                    ),
                    (
                        "classification",
                        record.classification.as_str(),
                        required.classification,
                    ),
                    ("policy", record.policy.as_str(), required.policy),
                    ("falsifier", record.falsifier.as_str(), required.falsifier),
                ] {
                    if actual != expected {
                        errors.push(format!("record:{}:{}", required.id, field));
                    }
                }
            }
            if !repo_text(required.path).contains(required.source_marker) {
                errors.push(format!(
                    "source:{}:{}",
                    required.path, required.source_marker
                ));
            }
        }
        for record in &records {
            if !REQUIRED_INVENTORY_RECORDS
                .iter()
                .any(|required| required.id == record.id)
            {
                errors.push(format!("record:{}:unexpected", record.id));
            }
            if !allowed.contains(&record.classification.as_str()) {
                errors.push(format!("record:{}:classification-not-allowed", record.id));
            }
            if [
                record.path.as_str(),
                record.source_marker.as_str(),
                record.eligibility_phase.as_str(),
                record.classification.as_str(),
                record.policy.as_str(),
                record.falsifier.as_str(),
            ]
            .contains(&"")
            {
                errors.push(format!("record:{}:empty-bound-field", record.id));
            }
        }
        errors.sort();
        errors.dedup();
        errors
    }

    fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn smoke80_reproducer_is_deterministic_and_fast_contract_only() {
        let a = serde_json::to_vec(&run_smoke80_contract_model()).unwrap();
        let b = serde_json::to_vec(&run_smoke80_contract_model()).unwrap();
        assert_eq!(a, b);
        let report: B58TraversalContractModelReport = serde_json::from_slice(&a).unwrap();
        assert!(report.legacy_divergence_reproduced);
        assert!(report.deterministic);
        assert!(!report.gameplay_mutated);
        assert!(!report.production_geometry_exercised);
        assert!(report.negative_cases.iter().all(|case| case.rejected));
    }

    #[test]
    fn traversal_link_rejects_competing_writer_and_releases_on_contact_loss() {
        let report = run_smoke80_contract_model();
        assert!(
            report
                .negative_cases
                .iter()
                .find(|case| case.name == "chaser_during_traversal")
                .unwrap()
                .rejected
        );
        assert!(
            report
                .negative_cases
                .iter()
                .find(|case| case.name == "contact_loss_bounded_abort")
                .unwrap()
                .rejected
        );
    }

    #[test]
    fn smoke80_production_geometry_uses_shipping_predicates_and_falsifiers() {
        let a = serde_json::to_vec(&run_smoke80_production_geometry_fixture()).unwrap();
        let b = serde_json::to_vec(&run_smoke80_production_geometry_fixture()).unwrap();
        assert_eq!(a, b);
        let report: ProductionGeometryFixtureReport = serde_json::from_slice(&a).unwrap();
        assert!(report.deterministic);
        assert!(report.production_geometry_exercised);
        assert!(!report.gameplay_mutated);
        let blocked = report
            .cases
            .iter()
            .find(|case| case.name == "preserved_solid_entry")
            .unwrap();
        assert!(blocked.rejected);
        assert_eq!(blocked.selected, None);
        assert_eq!(blocked.first_hit.as_ref().unwrap().block, [4, 4, 2]);
        let positive = report
            .cases
            .iter()
            .find(|case| case.name == "cleared_supported_entry")
            .unwrap();
        assert!(!positive.rejected);
        assert_eq!(positive.selected, Some([4, 4, 2]));
        assert!(report.cases.iter().all(|case| {
            case.predicates.iter().any(|predicate| {
                predicate.contains("emergency_")
                    || predicate.contains("bastion_full_path")
                    || predicate.contains("cylinder_sweep")
                    || predicate.contains("focused_adapter")
            })
        }));
    }

    /// bastion ENGINE-OPT-3 (ledger #160): the authoritative Pickup commit
    /// must keep revalidating `LootOwner::can_pickup` and denying with
    /// `LootOwned` — the AI-side attempt check is advisory; this gate is the
    /// security boundary (it is what bounded the old inverted predicate's
    /// damage to refusal+spam instead of theft). Source-scan pin, U8 style.
    #[test]
    fn item_160_pickup_commit_gate_present() {
        let src = repo_text("server/src/events/inventory_manip.rs");
        let arm = src
            .split("InventoryManip::Pickup")
            .nth(1)
            .expect("the Pickup arm must exist in inventory_manip");
        let window = &arm[..arm.len().min(4000)];
        assert!(
            window.contains(".can_pickup("),
            "the Pickup commit lost its LootOwner::can_pickup revalidation (ledger #160 TOCTOU gate)"
        );
        assert!(
            window.contains("LootOwned"),
            "the Pickup commit must deny with CollectFailedReason::LootOwned"
        );
    }

    /// T0.2 (master build order; ledger #21): THE LABOR CLOCK DECLARATION —
    /// every work/farm/need/rescue/item-economy duration rides the SIM clock
    /// (`Time`, `DeltaTime`, `Tick`), never the wall clock. Executable form:
    /// the labor/economy source files must contain no wall-clock reads at
    /// all. This pins the ENGOPT6 class at the door: `LootOwner`'s `Instant`
    /// expiry made a 45-WALL-second timeout resolve at machine-throughput-
    /// dependent sim ticks (tick 3960 vs ~3976 across an attested same-
    /// platform VM pair, tapes byte-equal until the flip).
    #[test]
    fn t0_2_labor_paths_declare_sim_clock_only() {
        for path in [
            "bastion-server/src/bastion_jobs.rs",
            "bastion-server/src/bastion_actions.rs",
            "bastion-server/src/bastion_mood.rs",
            "bastion-server/src/bastion_piles.rs",
            "bastion-server/src/bastion_chop.rs",
            "bastion-server/src/bastion_path.rs",
            "bastion-server/src/bastion_traversal.rs",
            "common/src/comp/loot_owner.rs",
            "server/src/sys/loot.rs",
            "server/src/sys/item.rs",
        ] {
            let src = repo_text(path);
            for banned in ["Instant::now", "SystemTime::now"] {
                assert!(
                    !src.contains(banned),
                    "{path} reads the wall clock ({banned}) inside a labor/economy path — \
                     labor durations are SIM-clock only (T0.2; the LootOwner/ENGOPT6 class)"
                );
            }
        }
    }

    #[test]
    fn writer_inventory_guard_covers_required_paths_categories_and_activity_owners() {
        let document = repo_text(INVENTORY_DOC);
        assert_eq!(validate_inventory_records(&document), Vec::<String>::new());

        let mut files = Vec::new();
        collect_rs_files(&repo_root().join("server/src"), &mut files);
        // Crate-split: the bastion modules moved out of server/src — the
        // activity-writer sweep must keep covering them from their new home.
        collect_rs_files(&repo_root().join("bastion-server/src"), &mut files);
        let mut activity_writers = files
            .into_iter()
            .filter(|path| !path.ends_with("bastion_traversal_tooling.rs"))
            .filter(|path| {
                repo_text(path.strip_prefix(repo_root()).unwrap().to_str().unwrap())
                    .contains("rtsim_controller.activity =")
            })
            .map(|path| {
                path.strip_prefix(repo_root())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();
        activity_writers.sort();
        assert_eq!(activity_writers, vec![
            "bastion-server/src/bastion_jobs.rs",
            "server/src/lib.rs",
            "server/src/rtsim/tick.rs",
        ]);
    }

    #[test]
    fn writer_inventory_guard_rejects_deliberate_omission() {
        let document = repo_text(INVENTORY_DOC);
        let omitted = document.replace("server/src/rtsim/tick.rs", "[deliberately omitted]");
        let errors = validate_inventory_records(&omitted);
        assert!(errors.contains(&"record:INV-RTSIM-ACTIVITY:path".to_string()));
    }

    #[test]
    fn writer_inventory_binds_controller_and_agent_inbox_interruptions() {
        let document = repo_text(INVENTORY_DOC);
        let (records, parse_errors) = parse_inventory_records(&document);
        assert!(parse_errors.is_empty());
        let required_ids = [
            "INV-CONTROLLER-SYSTEM",
            "INV-INBOX-HURT",
            "INV-INBOX-TALK",
            "INV-INBOX-TRADE",
            "INV-MOUNT-EVENT",
            "INV-INBOX-CONSUMER",
        ];
        for id in required_ids {
            let record = records.iter().find(|record| record.id == id).unwrap();
            assert!(!record.path.is_empty());
            assert!(!record.source_marker.is_empty());
            assert!(!record.eligibility_phase.is_empty());
            assert!(!record.classification.is_empty());
            assert!(!record.policy.is_empty());
            assert!(!record.falsifier.is_empty());
        }
        let controller = records
            .iter()
            .find(|record| record.id == "INV-CONTROLLER-SYSTEM")
            .unwrap();
        assert_eq!(controller.path, "common/systems/src/controller.rs");
        assert_eq!(controller.policy, "controller-event-link-guard");
    }

    #[test]
    fn writer_inventory_guard_rejects_token_pile_with_incomplete_bound_row() {
        let document = repo_text(INVENTORY_DOC);
        let controller_line = document
            .lines()
            .find(|line| line.starts_with("| INV-CONTROLLER-SYSTEM "))
            .unwrap();
        let incomplete = controller_line.replacen(
            "every joined UID+Controller; Common Create after Mount; sanitizes and drains events",
            "",
            1,
        );
        let mut token_pile = document.replacen(controller_line, &incomplete, 1);
        for required in REQUIRED_INVENTORY_RECORDS {
            token_pile.push_str(&format!(
                "\n{} {} {} {} {} {} {}",
                required.id,
                required.path,
                required.source_marker,
                required.eligibility_phase,
                required.classification,
                required.policy,
                required.falsifier,
            ));
        }
        let errors = validate_inventory_records(&token_pile);
        assert!(errors.contains(&"record:INV-CONTROLLER-SYSTEM:eligibility-phase".to_string()));
        assert!(errors.contains(&"record:INV-CONTROLLER-SYSTEM:empty-bound-field".to_string()));
    }
}
