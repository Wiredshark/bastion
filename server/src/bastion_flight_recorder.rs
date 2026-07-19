//! REQ-0092/0094 read-only colonist trajectory recorder.
//!
//! The recorder is disabled unless `BASTION_FLIGHT_RECORDER_DIR` is set.  It
//! is deliberately process-local and bounded; calls made while disabled do not
//! initialize the recorder, create output, perform I/O, mutate ECS state, or
//! alter scheduling. The server and the focused harness use the same JSONL
//! schema.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_MAX_SAMPLES: usize = 500_000;
const DEFAULT_MAX_EVENTS: usize = 500_000;
type RecorderResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FlightSample {
    pub schema: String,
    pub tick: u64,
    pub simulated_seconds: f64,
    pub wall_unix_millis: Option<u128>,
    pub uid: u64,
    pub entity: u32,
    pub episode: u64,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub character_state: String,
    pub phase: String,
    pub on_ground: bool,
    pub on_wall: Option<[f32; 3]>,
    pub support_clear: bool,
    pub body_clear: bool,
    pub head_clear: bool,
    pub active_job: Option<u64>,
    pub active_job_state: Option<String>,
    pub route_kind: Option<String>,
    pub route_owner: Option<u64>,
    pub link_id: Option<String>,
    pub frontier_job: Option<u64>,
    pub corridor_cursor: Option<usize>,
    pub corridor_waypoint: Option<[i32; 3]>,
    pub goto_target: Option<[f32; 3]>,
    pub chaser_last_target: Option<[f32; 3]>,
    pub chaser_route_target: Option<[f32; 3]>,
    pub chaser_route_head: Option<[i32; 3]>,
    pub chaser_next_idx: Option<usize>,
    pub chaser_path_state: String,
    pub chaser_recent_states: usize,
    pub controller_move_dir: [f32; 2],
    pub controller_move_z: f32,
    pub movement_writer: String,
    pub energy: Option<f32>,
    pub terrain_revision: Option<u64>,
    pub exit_plane_z: Option<f32>,
    pub endpoint_distance: Option<f32>,
    /// R10 (schema v2, additive): the owned-traversal fencing epoch this
    /// sample was driven under (`None` = no live task). Absent from v1
    /// tapes (`serde(default)` reads them fine); v1↔v1 comparator hashes
    /// untouched by construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ownership_epoch: Option<u64>,
    /// FABLE-004 F1 (schema v2, additive): the climb TOKEN WITNESS —
    /// `Some(true)` = this sample's climb is driven by the OWNED ladder
    /// token (live task in a Traversing* phase with ladder contact);
    /// `Some(false)` = the character is climbing WITHOUT an owned task
    /// (the named vanilla-leak inside an owned window — fork #15's
    /// evidence substrate); `None` = not climbing and no owned task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub climb_token_witness: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WriterEvent {
    pub schema: String,
    pub tick: u64,
    pub uid: u64,
    /// Monotonic observation label within the recorder schema. This is not a
    /// Specs scheduling-order claim unless `dispatcher_dependency_proven` is
    /// true.
    pub observation_sequence: u16,
    pub snapshot_stage: String,
    pub dispatcher_dependency_proven: bool,
    pub writer: String,
    pub move_dir: [f32; 2],
    pub move_z: f32,
    pub target: Option<[f32; 3]>,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct OwnerFlightSummary {
    pub samples: u64,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
    pub min_z: Option<f32>,
    pub max_z: Option<f32>,
    pub total_distance: f64,
    pub no_progress_intervals: u64,
    pub direction_flips: u64,
    pub downward_fallbacks: u64,
    pub retry_count: u64,
    pub phase_samples: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecorderSummary {
    pub schema: String,
    pub enabled: bool,
    pub bounded: bool,
    pub sample_limit: usize,
    pub event_limit: usize,
    pub samples_written: usize,
    pub events_written: usize,
    pub truncated_samples: bool,
    pub truncated_events: bool,
    pub owners: BTreeMap<u64, OwnerFlightSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RecorderMetadata {
    schema: &'static str,
    executable: Option<String>,
    artifact_sha256: Option<String>,
    source_head: Option<String>,
    source_branch: Option<String>,
    source_dirty: Option<String>,
    seed: Option<String>,
    command: Option<String>,
    session_id: Option<String>,
    uid_filter: Option<u64>,
    sample_every: u64,
    max_samples: usize,
    max_events: usize,
    compiled_git_hash: String,
}

#[derive(Clone, Debug)]
struct RecorderConfig {
    dir: PathBuf,
    uid_filter: Option<u64>,
    sample_every: u64,
    max_samples: usize,
    max_events: usize,
}

impl RecorderConfig {
    fn from_env() -> Option<Self> { Self::from_lookup(|key| std::env::var_os(key)) }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<std::ffi::OsString>) -> Option<Self> {
        let dir = lookup("BASTION_FLIGHT_RECORDER_DIR").map(PathBuf::from)?;
        let uid_filter = lookup("BASTION_FLIGHT_RECORDER_UID")
            .and_then(|value| value.into_string().ok())
            .and_then(|value| value.parse().ok());
        let sample_every = lookup("BASTION_FLIGHT_RECORDER_SAMPLE_EVERY")
            .and_then(|value| value.into_string().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1)
            .max(1);
        let max_samples = lookup("BASTION_FLIGHT_RECORDER_MAX_SAMPLES")
            .and_then(|value| value.into_string().ok())
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_SAMPLES)
            .max(1);
        let max_events = lookup("BASTION_FLIGHT_RECORDER_MAX_EVENTS")
            .and_then(|value| value.into_string().ok())
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_EVENTS)
            .max(1);
        Some(Self {
            dir,
            uid_filter,
            sample_every,
            max_samples,
            max_events,
        })
    }
}

#[derive(Clone, Debug)]
struct LastOwnerState {
    position: [f32; 3],
    velocity: [f32; 3],
    phase: String,
    route_owner: Option<u64>,
    goto_target: Option<[f32; 3]>,
    movement_writer: String,
    on_wall: bool,
    endpoint_distance: Option<f32>,
    max_z: f32,
}

struct Recorder {
    config: RecorderConfig,
    trajectory: BufWriter<File>,
    events: BufWriter<File>,
    csv: BufWriter<File>,
    sample_count: usize,
    event_count: usize,
    truncated_samples: bool,
    truncated_events: bool,
    owners: BTreeMap<u64, OwnerFlightSummary>,
    last: HashMap<u64, LastOwnerState>,
    episodes: HashMap<u64, u64>,
}

impl Recorder {
    fn new(config: RecorderConfig) -> RecorderResult<Self> {
        fs::create_dir_all(&config.dir)?;
        let trajectory = BufWriter::new(File::create(config.dir.join("trajectory.jsonl"))?);
        let events = BufWriter::new(File::create(config.dir.join("events.jsonl"))?);
        let mut csv = BufWriter::new(File::create(config.dir.join("trajectory.csv"))?);
        writeln!(
            csv,
            "tick,sim_seconds,uid,entity,episode,x,y,z,vx,vy,vz,state,phase,on_ground,active_job,\
             route_owner,frontier_job,waypoint_x,waypoint_y,waypoint_z,target_x,target_y,target_z,\
             writer,endpoint_distance"
        )?;
        let metadata = RecorderMetadata {
            schema: "bastion.flight-recorder.metadata/v1",
            executable: std::env::current_exe()
                .ok()
                .map(|path| path.display().to_string()),
            artifact_sha256: std::env::var("BASTION_FLIGHT_RECORDER_ARTIFACT_SHA256").ok(),
            source_head: std::env::var("BASTION_FLIGHT_RECORDER_SOURCE_HEAD").ok(),
            source_branch: std::env::var("BASTION_FLIGHT_RECORDER_SOURCE_BRANCH").ok(),
            source_dirty: std::env::var("BASTION_FLIGHT_RECORDER_SOURCE_DIRTY").ok(),
            seed: std::env::var("BASTION_FLIGHT_RECORDER_SEED").ok(),
            command: std::env::var("BASTION_FLIGHT_RECORDER_COMMAND").ok(),
            session_id: std::env::var("BASTION_FLIGHT_RECORDER_SESSION_ID").ok(),
            uid_filter: config.uid_filter,
            sample_every: config.sample_every,
            max_samples: config.max_samples,
            max_events: config.max_events,
            compiled_git_hash: format!("{:x}", *common::util::GIT_HASH),
        };
        serde_json::to_writer_pretty(File::create(config.dir.join("metadata.json"))?, &metadata)?;
        Ok(Self {
            config,
            trajectory,
            events,
            csv,
            sample_count: 0,
            event_count: 0,
            truncated_samples: false,
            truncated_events: false,
            owners: BTreeMap::new(),
            last: HashMap::new(),
            episodes: HashMap::new(),
        })
    }

    fn accepts(&self, tick: u64, uid: u64) -> bool {
        tick % self.config.sample_every == 0
            && self.config.uid_filter.is_none_or(|filter| filter == uid)
    }

    fn record_sample(&mut self, mut sample: FlightSample) -> RecorderResult<()> {
        if !self.accepts(sample.tick, sample.uid) {
            return Ok(());
        }
        if self.sample_count >= self.config.max_samples {
            self.truncated_samples = true;
            return Ok(());
        }

        let previous = self.last.get(&sample.uid).cloned();
        let episode = self.episodes.entry(sample.uid).or_insert(0);
        if previous
            .as_ref()
            .is_some_and(|state| state.route_owner.is_none() && sample.route_owner.is_some())
            || (previous.is_none() && sample.route_owner.is_some())
        {
            *episode = episode.saturating_add(1);
        }
        sample.episode = *episode;

        let summary = self.owners.entry(sample.uid).or_default();
        summary.samples += 1;
        summary.first_tick.get_or_insert(sample.tick);
        summary.last_tick = Some(sample.tick);
        summary.min_z = Some(
            summary
                .min_z
                .map_or(sample.position[2], |z| z.min(sample.position[2])),
        );
        summary.max_z = Some(
            summary
                .max_z
                .map_or(sample.position[2], |z| z.max(sample.position[2])),
        );
        *summary
            .phase_samples
            .entry(sample.phase.clone())
            .or_default() += 1;

        if let Some(previous) = &previous {
            let delta = [
                sample.position[0] - previous.position[0],
                sample.position[1] - previous.position[1],
                sample.position[2] - previous.position[2],
            ];
            let distance =
                ((delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]) as f64).sqrt();
            summary.total_distance += distance;
            let endpoint_not_improved = matches!(
                (previous.endpoint_distance, sample.endpoint_distance),
                (Some(previous_distance), Some(current_distance))
                    if current_distance >= previous_distance - 0.01
            );
            let no_progress =
                endpoint_not_improved || (distance < 0.01 && sample.endpoint_distance.is_some());
            if no_progress {
                summary.no_progress_intervals += 1;
            }
            let velocity_dot = sample.velocity[0] * previous.velocity[0]
                + sample.velocity[1] * previous.velocity[1]
                + sample.velocity[2] * previous.velocity[2];
            let direction_flip = velocity_dot < -0.05;
            if direction_flip {
                summary.direction_flips += 1;
            }
            if previous.max_z - sample.position[2] > 1.0 {
                summary.downward_fallbacks += 1;
            }
            if previous.phase.contains("Reacquire") && sample.phase.contains("Reacquire") {
                summary.retry_count += 1;
            }
            if previous.phase != sample.phase {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_000,
                    snapshot_stage: "derived-phase-transition".into(),
                    dispatcher_dependency_proven: false,
                    writer: "phase_transition".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!("{} -> {}", previous.phase, sample.phase),
                })?;
            }
            if previous.route_owner != sample.route_owner {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_010,
                    snapshot_stage: "derived-route-ownership-change".into(),
                    dispatcher_dependency_proven: false,
                    writer: "route_ownership_change".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!(
                        "route_owner {:?} -> {:?}",
                        previous.route_owner, sample.route_owner
                    ),
                })?;
            }
            if previous.goto_target != sample.goto_target {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_020,
                    snapshot_stage: "derived-target-change".into(),
                    dispatcher_dependency_proven: false,
                    writer: "target_change".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!(
                        "target {:?} -> {:?}",
                        previous.goto_target, sample.goto_target
                    ),
                })?;
            }
            if previous.movement_writer != sample.movement_writer {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_030,
                    snapshot_stage: "derived-movement-owner-change".into(),
                    dispatcher_dependency_proven: false,
                    writer: "movement_owner_change".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!(
                        "writer {} -> {}",
                        previous.movement_writer, sample.movement_writer
                    ),
                })?;
            }
            if previous.on_wall && sample.on_wall.is_none() {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_040,
                    snapshot_stage: "derived-authoritative-contact-loss".into(),
                    dispatcher_dependency_proven: false,
                    writer: "authoritative_contact_loss".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!("contact lost at z={:.3}", sample.position[2]),
                })?;
            }
            if previous.position[2] - sample.position[2] > 0.5 {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_050,
                    snapshot_stage: "derived-downward-displacement".into(),
                    dispatcher_dependency_proven: false,
                    writer: "downward_displacement".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!("z {:.3} -> {:.3}", previous.position[2], sample.position[2]),
                })?;
            }
            if no_progress {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_060,
                    snapshot_stage: "derived-no-progress".into(),
                    dispatcher_dependency_proven: false,
                    writer: "no_progress".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!(
                        "distance_delta={distance:.6}; endpoint={:?}->{:?}",
                        previous.endpoint_distance, sample.endpoint_distance
                    ),
                })?;
            }
            if direction_flip {
                self.record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: sample.tick,
                    uid: sample.uid,
                    observation_sequence: 30_070,
                    snapshot_stage: "derived-direction-flip".into(),
                    dispatcher_dependency_proven: false,
                    writer: "direction_flip".into(),
                    move_dir: sample.controller_move_dir,
                    move_z: sample.controller_move_z,
                    target: sample.goto_target,
                    note: format!("velocity_dot={velocity_dot:.6}"),
                })?;
            }
        }

        serde_json::to_writer(&mut self.trajectory, &sample)?;
        writeln!(self.trajectory)?;
        let waypoint = sample.corridor_waypoint.unwrap_or([0; 3]);
        let target = sample.goto_target.unwrap_or([f32::NAN; 3]);
        writeln!(
            self.csv,
            "{},{:.6},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{},{},{:.\
             6},{:.6},{:.6},{},{:.6}",
            sample.tick,
            sample.simulated_seconds,
            sample.uid,
            sample.entity,
            sample.episode,
            sample.position[0],
            sample.position[1],
            sample.position[2],
            sample.velocity[0],
            sample.velocity[1],
            sample.velocity[2],
            csv_field(&sample.character_state),
            csv_field(&sample.phase),
            sample.on_ground,
            sample.active_job.map_or(String::new(), |id| id.to_string()),
            sample
                .route_owner
                .map_or(String::new(), |id| id.to_string()),
            sample
                .frontier_job
                .map_or(String::new(), |id| id.to_string()),
            waypoint[0],
            waypoint[1],
            waypoint[2],
            target[0],
            target[1],
            target[2],
            csv_field(&sample.movement_writer),
            sample.endpoint_distance.unwrap_or(f32::NAN),
        )?;
        self.sample_count += 1;
        self.last.insert(sample.uid, LastOwnerState {
            position: sample.position,
            velocity: sample.velocity,
            phase: sample.phase,
            route_owner: sample.route_owner,
            goto_target: sample.goto_target,
            movement_writer: sample.movement_writer,
            on_wall: sample.on_wall.is_some(),
            endpoint_distance: sample.endpoint_distance,
            max_z: previous.as_ref().map_or(sample.position[2], |state| {
                state.max_z.max(sample.position[2])
            }),
        });
        Ok(())
    }

    fn record_event(&mut self, event: WriterEvent) -> RecorderResult<()> {
        if self.config.uid_filter.is_some_and(|uid| uid != event.uid) {
            return Ok(());
        }
        if self.event_count >= self.config.max_events {
            self.truncated_events = true;
            return Ok(());
        }
        serde_json::to_writer(&mut self.events, &event)?;
        writeln!(self.events)?;
        self.event_count += 1;
        Ok(())
    }

    fn finalize(mut self) -> RecorderResult<()> {
        self.trajectory.flush()?;
        self.events.flush()?;
        self.csv.flush()?;
        let summary = RecorderSummary {
            schema: "bastion.flight-recorder.summary/v1".into(),
            enabled: true,
            bounded: true,
            sample_limit: self.config.max_samples,
            event_limit: self.config.max_events,
            samples_written: self.sample_count,
            events_written: self.event_count,
            truncated_samples: self.truncated_samples,
            truncated_events: self.truncated_events,
            owners: self.owners,
        };
        serde_json::to_writer_pretty(
            File::create(self.config.dir.join("summary.json"))?,
            &summary,
        )?;
        Ok(())
    }
}

fn csv_field(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

static RECORDER: OnceLock<Mutex<Option<Recorder>>> = OnceLock::new();

fn recorder_slot() -> &'static Mutex<Option<Recorder>> { RECORDER.get_or_init(|| Mutex::new(None)) }

fn with_recorder(mut f: impl FnMut(&mut Recorder) -> RecorderResult<()>) {
    let Some(slot) = RECORDER.get().or_else(|| {
        RecorderConfig::from_env()?;
        Some(recorder_slot())
    }) else {
        return;
    };
    let Ok(mut slot) = slot.lock() else {
        return;
    };
    if slot.is_none() {
        let Some(config) = RecorderConfig::from_env() else {
            return;
        };
        match Recorder::new(config) {
            Ok(recorder) => *slot = Some(recorder),
            Err(error) => {
                tracing::warn!(%error, "bastion flight recorder failed to initialize");
                return;
            },
        }
    }
    if let Some(recorder) = slot.as_mut()
        && let Err(error) = f(recorder)
    {
        tracing::warn!(%error, "bastion flight recorder write failed");
    }
}

pub fn enabled() -> bool {
    if std::env::var_os("BASTION_FLIGHT_RECORDER_DIR").is_some() {
        return true;
    }
    RECORDER
        .get()
        .and_then(|slot| slot.lock().ok())
        .is_some_and(|slot| slot.is_some())
}

/// Test-tooling introspection for the disabled-by-default lifecycle proof.
/// This reports whether the process-global slot was ever initialized; it does
/// not initialize the slot itself.
#[doc(hidden)]
pub fn global_slot_initialized() -> bool { RECORDER.get().is_some() }

/// Begin an explicitly bounded recorder session after the UID is known. This
/// is used only by focused harness probes so boot-time samples cannot exhaust
/// the bounds before the production Agent/Bastion snapshots under test.
#[doc(hidden)]
pub fn start_probe_session(
    dir: &Path,
    uid_filter: Option<u64>,
    sample_every: u64,
    max_samples: usize,
    max_events: usize,
) -> Result<(), String> {
    let config = RecorderConfig {
        dir: dir.to_path_buf(),
        uid_filter,
        sample_every: sample_every.max(1),
        max_samples: max_samples.max(1),
        max_events: max_events.max(1),
    };
    let mut slot = recorder_slot()
        .lock()
        .map_err(|_| "flight recorder slot lock poisoned".to_owned())?;
    if slot.is_some() {
        return Err("flight recorder session already active".to_owned());
    }
    *slot = Some(Recorder::new(config).map_err(|error| error.to_string())?);
    Ok(())
}

pub fn wall_unix_millis() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis())
}

pub fn record_sample(sample: FlightSample) {
    if enabled() {
        with_recorder(|recorder| recorder.record_sample(sample.clone()));
    }
}

pub fn record_writer(event: WriterEvent) {
    if enabled() {
        with_recorder(|recorder| recorder.record_event(event.clone()));
    }
}

pub fn finalize() {
    let Some(slot) = RECORDER.get() else {
        return;
    };
    let Ok(mut slot) = slot.lock() else {
        return;
    };
    if let Some(recorder) = slot.take()
        && let Err(error) = recorder.finalize()
    {
        tracing::warn!(%error, "bastion flight recorder finalization failed");
    }
}

pub fn write_disabled_summary(path: &Path) -> RecorderResult<()> {
    let summary = RecorderSummary {
        schema: "bastion.flight-recorder.summary/v1".into(),
        enabled: false,
        bounded: true,
        sample_limit: 0,
        event_limit: 0,
        samples_written: 0,
        events_written: 0,
        truncated_samples: false,
        truncated_events: false,
        owners: BTreeMap::new(),
    };
    serde_json::to_writer_pretty(File::create(path)?, &summary)?;
    Ok(())
}

/// Write a deterministic, server-free recorder fixture for REQ-0094 review.
///
/// This deliberately bypasses the process-global recorder slot so it can be
/// run repeatedly in one process without affecting the live env-gated path.
/// The emitted trajectory/events/CSV/summary use the production schemas and
/// bounds, but contain no ECS or gameplay mutation.
pub fn write_local_schema_fixture(output_dir: &Path) -> Result<(), String> {
    let config = RecorderConfig {
        dir: output_dir.to_path_buf(),
        uid_filter: Some(7),
        sample_every: 1,
        max_samples: 8,
        max_events: 16,
    };
    let mut recorder = Recorder::new(config).map_err(|error| error.to_string())?;
    for (tick, x, velocity, phase, endpoint_distance) in [
        (100, 0.0, [1.0, 0.0, 0.0], "Approach", 1.5),
        (101, 0.5, [1.0, 0.0, 0.0], "Approach", 1.0),
        (102, 0.5, [-1.0, 0.0, 0.0], "Reserved", 1.0),
        (103, 0.25, [-1.0, 0.0, -0.6], "Reacquire", 1.25),
    ] {
        recorder
            .record_sample(focused_sample(tick, x, velocity, phase, endpoint_distance))
            .map_err(|error| error.to_string())?;
    }
    for (observation_sequence, writer, snapshot_stage) in [
        (100, "agent", "synthetic-agent-snapshot"),
        (300, "bastion", "synthetic-bastion-snapshot"),
    ] {
        recorder
            .record_event(WriterEvent {
                schema: "bastion.flight-recorder.event/v1".into(),
                tick: 103,
                uid: 7,
                observation_sequence,
                snapshot_stage: snapshot_stage.into(),
                dispatcher_dependency_proven: false,
                writer: writer.into(),
                move_dir: [-1.0, 0.0],
                move_z: 0.0,
                target: Some([1.5, 2.0, 3.0]),
                note: "local-schema-observation-sequence; not dispatcher-order proof".into(),
            })
            .map_err(|error| error.to_string())?;
    }
    recorder.finalize().map_err(|error| error.to_string())
}

fn focused_sample(
    tick: u64,
    x: f32,
    velocity: [f32; 3],
    phase: &str,
    endpoint_distance: f32,
) -> FlightSample {
    FlightSample {
        schema: "bastion.flight-recorder.sample/v1".into(),
        tick,
        simulated_seconds: tick as f64 / 30.0,
        wall_unix_millis: None,
        uid: 7,
        entity: 3,
        episode: 0,
        position: [x, 2.0, 3.0 + velocity[2]],
        velocity,
        character_state: "Idle".into(),
        phase: phase.into(),
        on_ground: velocity[2] == 0.0,
        on_wall: (phase == "Reserved").then_some([0.0, 1.0, 0.0]),
        support_clear: true,
        body_clear: true,
        head_clear: true,
        active_job: Some(1240),
        active_job_state: Some("Traveling".into()),
        route_kind: Some("ConstructedLadder".into()),
        route_owner: Some(7),
        link_id: Some("7:1240".into()),
        frontier_job: Some(1240),
        corridor_cursor: Some(0),
        corridor_waypoint: Some([1, 2, 3]),
        goto_target: Some([1.5, 2.0, 3.0]),
        chaser_last_target: None,
        chaser_route_target: None,
        chaser_route_head: None,
        chaser_next_idx: None,
        chaser_path_state: "None".into(),
        chaser_recent_states: 0,
        controller_move_dir: [velocity[0], velocity[1]],
        controller_move_z: velocity[2],
        movement_writer: if phase == "Reacquire" {
            "bastion_traversal"
        } else {
            "agent_chaser"
        }
        .into(),
        energy: Some(100.0),
        terrain_revision: Some(4),
        exit_plane_z: Some(10.0),
        endpoint_distance: Some(endpoint_distance),
        // R10 v2 fields: absent in the focused probe (v1-shaped fixture).
        ownership_epoch: None,
        climb_token_witness: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(tick: u64, x: f32) -> FlightSample {
        focused_sample(tick, x, [1.0, 0.0, 0.0], "Approach", (1.5 - x).abs())
    }

    #[test]
    fn bounded_recorder_preserves_tick_order_and_writes_summary() {
        let root = std::env::temp_dir().join(format!(
            "bastion-flight-recorder-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let config = RecorderConfig {
            dir: root.clone(),
            uid_filter: Some(7),
            sample_every: 1,
            max_samples: 2,
            max_events: 4,
        };
        let mut recorder = Recorder::new(config).unwrap();
        recorder.record_sample(sample(1, 0.0)).unwrap();
        recorder.record_sample(sample(2, 0.5)).unwrap();
        recorder.record_sample(sample(3, 1.0)).unwrap();
        recorder.finalize().unwrap();
        let lines = fs::read_to_string(root.join("trajectory.jsonl")).unwrap();
        let ticks = lines
            .lines()
            .map(|line| serde_json::from_str::<FlightSample>(line).unwrap().tick)
            .collect::<Vec<_>>();
        assert_eq!(ticks, vec![1, 2]);
        let summary: RecorderSummary =
            serde_json::from_reader(File::open(root.join("summary.json")).unwrap()).unwrap();
        assert_eq!(summary.samples_written, 2);
        assert!(summary.truncated_samples);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn disabled_summary_is_explicit_and_empty() {
        assert!(RecorderConfig::from_lookup(|_| None).is_none());
        let path = std::env::temp_dir().join(format!(
            "bastion-flight-recorder-disabled-{}.json",
            std::process::id()
        ));
        write_disabled_summary(&path).unwrap();
        let summary: RecorderSummary = serde_json::from_reader(File::open(&path).unwrap()).unwrap();
        assert!(!summary.enabled);
        assert_eq!(summary.samples_written, 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn short_episode_is_lossless_and_writer_events_keep_observation_order() {
        let root = std::env::temp_dir().join(format!(
            "bastion-flight-recorder-lossless-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let config = RecorderConfig {
            dir: root.clone(),
            uid_filter: Some(7),
            sample_every: 1,
            max_samples: 32,
            max_events: 32,
        };
        let mut recorder = Recorder::new(config).unwrap();
        for tick in 100..=110 {
            recorder.record_sample(sample(tick, tick as f32)).unwrap();
        }
        for (observation_sequence, writer, snapshot_stage) in [
            (100, "agent", "synthetic-agent-snapshot"),
            (300, "bastion", "synthetic-bastion-snapshot"),
        ] {
            recorder
                .record_event(WriterEvent {
                    schema: "bastion.flight-recorder.event/v1".into(),
                    tick: 110,
                    uid: 7,
                    observation_sequence,
                    snapshot_stage: snapshot_stage.into(),
                    dispatcher_dependency_proven: false,
                    writer: writer.into(),
                    move_dir: [1.0, 0.0],
                    move_z: 0.0,
                    target: Some([111.0, 2.0, 3.0]),
                    note: "focused-observation-proof".into(),
                })
                .unwrap();
        }
        recorder.finalize().unwrap();

        let trajectory = fs::read_to_string(root.join("trajectory.jsonl")).unwrap();
        let ticks = trajectory
            .lines()
            .map(|line| serde_json::from_str::<FlightSample>(line).unwrap().tick)
            .collect::<Vec<_>>();
        assert_eq!(ticks, (100..=110).collect::<Vec<_>>());

        let events = fs::read_to_string(root.join("events.jsonl")).unwrap();
        let observation_sequences = events
            .lines()
            .filter_map(|line| serde_json::from_str::<WriterEvent>(line).ok())
            .filter(|event| event.note == "focused-observation-proof")
            .map(|event| event.observation_sequence)
            .collect::<Vec<_>>();
        assert_eq!(observation_sequences, vec![100, 300]);

        let summary: RecorderSummary =
            serde_json::from_reader(File::open(root.join("summary.json")).unwrap()).unwrap();
        assert_eq!(summary.samples_written, 11);
        assert!(!summary.truncated_samples);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn local_schema_fixture_emits_inspectable_bounded_outputs() {
        let root = std::env::temp_dir().join(format!(
            "bastion-flight-recorder-focused-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        write_local_schema_fixture(&root).unwrap();
        let summary: RecorderSummary =
            serde_json::from_reader(File::open(root.join("summary.json")).unwrap()).unwrap();
        assert_eq!(summary.samples_written, 4);
        assert!(summary.events_written >= 2);
        assert!(!summary.truncated_samples);
        assert!(!summary.truncated_events);
        assert!(root.join("trajectory.jsonl").is_file());
        assert!(root.join("events.jsonl").is_file());
        assert!(root.join("trajectory.csv").is_file());
        let _ = fs::remove_dir_all(root);
    }
}
