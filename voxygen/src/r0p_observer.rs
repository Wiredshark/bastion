//! Opt-in production renderer observatory for the R0P measurement lane.
//!
//! The observer is inert unless `BASTION_R0P_OUTPUT` is set. It records actual
//! production frame work; it never feeds measurements back into rendering.

use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const GPU_PENDING_LIMIT: usize = 4;
const MAX_BUFFERED_RECORDS: usize = 4_096;
const MAX_DURABLE_CHUNK_RECORDS: usize = 60;
const MAX_DURABLE_CHUNK_AGE: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Eq, PartialEq)]
struct BufferedRecordV1 {
    ordinal: u64,
    line: String,
}

#[derive(Debug)]
struct DurableFrameSinkV1 {
    output: PathBuf,
    frames: VecDeque<BufferedRecordV1>,
    gpu_frames: VecDeque<BufferedRecordV1>,
    chunk_sequence: u64,
    dropped_frame_records: u64,
    dropped_gpu_records: u64,
    durable_through_ordinal: Option<u64>,
    chunk_started: Instant,
}

impl DurableFrameSinkV1 {
    fn initialize(output: &Path, now: Instant) -> std::io::Result<Self> {
        for name in ["frames.jsonl", "gpu-frames.jsonl", "observer-chunks.jsonl"] {
            let file = fs::File::create(output.join(name))?;
            file.sync_data()?;
        }
        Ok(Self {
            output: output.to_owned(),
            frames: VecDeque::new(),
            gpu_frames: VecDeque::new(),
            chunk_sequence: 0,
            dropped_frame_records: 0,
            dropped_gpu_records: 0,
            durable_through_ordinal: None,
            chunk_started: now,
        })
    }

    fn push_frame(&mut self, ordinal: u64, line: String, now: Instant) -> std::io::Result<()> {
        push_bounded(
            &mut self.frames,
            BufferedRecordV1 { ordinal, line },
            &mut self.dropped_frame_records,
        );
        self.flush_if_due(now)
    }

    fn push_gpu_frame(&mut self, ordinal: u64, line: String) {
        push_bounded(
            &mut self.gpu_frames,
            BufferedRecordV1 { ordinal, line },
            &mut self.dropped_gpu_records,
        );
    }

    fn flush_if_due(&mut self, now: Instant) -> std::io::Result<()> {
        let age_due = now
            .checked_duration_since(self.chunk_started)
            .is_some_and(|elapsed| elapsed >= MAX_DURABLE_CHUNK_AGE);
        if self.frames.len() >= MAX_DURABLE_CHUNK_RECORDS || (age_due && !self.frames.is_empty()) {
            self.flush_one_chunk(now)?;
        }
        Ok(())
    }

    fn flush_one_chunk(&mut self, now: Instant) -> std::io::Result<()> {
        let record_count = self.frames.len().min(MAX_DURABLE_CHUNK_RECORDS);
        if record_count == 0 {
            self.chunk_started = now;
            return Ok(());
        }

        let first_ordinal = self
            .frames
            .front()
            .map(|record| record.ordinal)
            .unwrap_or(0);
        let last_ordinal = self
            .frames
            .get(record_count - 1)
            .map(|record| record.ordinal)
            .unwrap_or(first_ordinal);
        let frame_bytes = self
            .frames
            .iter()
            .take(record_count)
            .map(|record| record.line.as_str())
            .collect::<String>();

        let gpu_count = self
            .gpu_frames
            .iter()
            .take_while(|record| record.ordinal <= last_ordinal)
            .count();
        let gpu_bytes = self
            .gpu_frames
            .iter()
            .take(gpu_count)
            .map(|record| record.line.as_str())
            .collect::<String>();

        append_sync(&self.output.join("frames.jsonl"), frame_bytes.as_bytes())?;
        if !gpu_bytes.is_empty() {
            append_sync(&self.output.join("gpu-frames.jsonl"), gpu_bytes.as_bytes())?;
        }
        let acknowledgement = format!(
            concat!(
                "{{\"schema\":\"R0PObserverChunkV1\",\"chunk_sequence\":{},",
                "\"first_frame_ordinal\":{},\"last_frame_ordinal\":{},",
                "\"record_count\":{},\"gpu_record_count\":{},",
                "\"dropped_record_count\":{},\"dropped_gpu_record_count\":{},",
                "\"durable_through_ordinal\":{}}}\n"
            ),
            self.chunk_sequence,
            first_ordinal,
            last_ordinal,
            record_count,
            gpu_count,
            self.dropped_frame_records,
            self.dropped_gpu_records,
            last_ordinal,
        );
        append_sync(
            &self.output.join("observer-chunks.jsonl"),
            acknowledgement.as_bytes(),
        )?;

        for _ in 0..record_count {
            self.frames.pop_front();
        }
        for _ in 0..gpu_count {
            self.gpu_frames.pop_front();
        }
        self.durable_through_ordinal = Some(last_ordinal);
        self.chunk_sequence = self.chunk_sequence.saturating_add(1);
        self.chunk_started = now;
        Ok(())
    }

    fn flush_all(&mut self, now: Instant) -> std::io::Result<()> {
        while !self.frames.is_empty() {
            self.flush_one_chunk(now)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PresentedFrameTimingV1 {
    frame_ordinal: u64,
    frame_begin_ns: u64,
    present_end_ns: u64,
    presented_interval_ns: Option<u64>,
    overflowed: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuFramePhasesV1 {
    pub tick_ns: u64,
    pub render_submit_ns: u64,
    pub post_render_ns: u64,
    pub pacing_ns: u64,
    pub maintain_ns: u64,
    pub total_wall_ns: u64,
}

impl CpuFramePhasesV1 {
    #[must_use]
    pub fn reconciles(self) -> bool {
        self.tick_ns
            .checked_add(self.render_submit_ns)
            .and_then(|value| value.checked_add(self.post_render_ns))
            .and_then(|value| value.checked_add(self.pacing_ns))
            .and_then(|value| value.checked_add(self.maintain_ns))
            == Some(self.total_wall_ns)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SceneCountersV1 {
    pub terrain_chunks: u64,
    pub visible_terrain_chunks: u64,
    pub shadow_terrain_chunks: u64,
    pub terrain_requested_view_distance_chunks: u64,
    pub terrain_server_authorized_view_distance_chunks: u64,
    pub terrain_server_authority_available: bool,
    pub terrain_chunks_received_total: u64,
    pub terrain_resident_chunks: u64,
    pub terrain_pending_chunk_requests: u64,
    pub terrain_server_completed_tick: u64,
    pub loaded_distance_blocks: u64,
    pub terrain_view_distance_chunks: u64,
    pub terrain_mesh_queue: u64,
    pub terrain_mesh_queue_pruned_total: u64,
    pub visible_horizon_fixture_selected: bool,
    pub visible_horizon_camera_valid: bool,
    pub visible_horizon_camera_mode: u64,
    pub visible_horizon_projection: u64,
    pub visible_horizon_camera_focus_mm: [i64; 3],
    pub visible_horizon_camera_position_mm: [i64; 3],
    pub visible_horizon_camera_yaw_microradians: i64,
    pub visible_horizon_camera_pitch_microradians: i64,
    pub visible_horizon_camera_distance_mm: u64,
    pub visible_horizon_configured_base_fov_microradians: u64,
    pub visible_horizon_camera_base_fov_microradians: u64,
    pub visible_horizon_camera_target_base_fov_microradians: u64,
    pub visible_horizon_camera_fov_microradians: u64,
    pub visible_horizon_camera_fixation_millionths: u64,
    pub visible_horizon_camera_target_fixation_millionths: u64,
    pub visible_horizon_camera_aspect_millionths: u64,
    pub visible_horizon_frustum_ground_width_mm: u64,
    pub visible_horizon_frustum_ground_depth_mm: u64,
    pub visible_horizon_camera_token: [u8; 32],
    pub horizon_camera_path_id: u64,
    pub horizon_camera_path_ordinal: u64,
    pub horizon_camera_path_token: [u8; 32],
    pub horizon_surface_authority_available: bool,
    pub horizon_cutaway_solid: bool,
    pub horizon_underworld_rejected: bool,
    pub horizon_sky_ground_expected: bool,
    pub horizon_focus_surface_mm: i64,
    pub horizon_camera_surface_mm: i64,
    pub horizon_minimum_clearance_mm: i64,
    pub horizon_terrain_revision: u64,
    pub horizon_meshed_high_detail_chunks: u64,
    pub visible_horizon_near_0_8_chunks: u64,
    pub visible_horizon_reference_9_16_chunks: u64,
    pub visible_horizon_far_17_24_chunks: u64,
    pub visible_horizon_beyond_24_chunks: u64,
    pub visible_horizon_max_radius_chunks: u64,
    pub visible_horizon_max_distance_blocks: u64,
    pub visible_horizon_lod_terrain_draw_ready: bool,
    pub visible_horizon_lod_terrain_detail: u64,
    pub visible_horizon_lod_distance_blocks: u64,
    pub figures: u64,
    pub visible_figures: u64,
    pub particles: u64,
    pub visible_particles: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PostApplyHorizonCameraCountersV1 {
    pub camera_valid: bool,
    pub camera_mode: u64,
    pub projection: u64,
    pub focus_mm: [i64; 3],
    pub position_mm: [i64; 3],
    pub yaw_microradians: i64,
    pub pitch_microradians: i64,
    pub distance_mm: u64,
    pub configured_base_fov_microradians: u64,
    pub base_fov_microradians: u64,
    pub target_base_fov_microradians: u64,
    pub effective_fov_microradians: u64,
    pub fixation_millionths: u64,
    pub target_fixation_millionths: u64,
    pub aspect_millionths: u64,
    pub frustum_ground_width_mm: u64,
    pub frustum_ground_depth_mm: u64,
    pub camera_token: [u8; 32],
    pub path_id: u64,
    pub path_ordinal: u64,
    pub path_token: [u8; 32],
    pub surface_authority_available: bool,
    pub cutaway_solid: bool,
    pub underworld_rejected: bool,
    pub sky_ground_expected: bool,
    pub focus_surface_mm: i64,
    pub camera_surface_mm: i64,
    pub minimum_clearance_mm: i64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WorkCountersV1 {
    pass_count: u64,
    draw_count: u64,
    geometry_units: u64,
    instances: u64,
    bind_group_sets: u64,
    buffer_upload_ops: u64,
    buffer_upload_bytes: u64,
    texture_upload_ops: u64,
    texture_upload_bytes: u64,
    submissions: u64,
}

#[derive(Debug)]
struct ObserverStateV1 {
    output: PathBuf,
    frame_sequence: u64,
    gpu_sequence: u64,
    process_started: Instant,
    active_frame_started: Option<Instant>,
    last_presented_at: Option<Instant>,
    completed_presented_frame: Option<PresentedFrameTimingV1>,
    pending_gpu_frames: VecDeque<u64>,
    work: WorkCountersV1,
    scene: SceneCountersV1,
    durable: DurableFrameSinkV1,
    failed: bool,
}

static OBSERVER: OnceLock<Option<Mutex<ObserverStateV1>>> = OnceLock::new();

fn observer() -> Option<&'static Mutex<ObserverStateV1>> {
    OBSERVER
        .get_or_init(|| {
            let output = std::env::var_os("BASTION_R0P_OUTPUT").map(PathBuf::from)?;
            if let Err(error) = fs::create_dir_all(&output) {
                tracing::error!(target: "bastion_r0p", ?error, "failed to create observer output");
                return None;
            }
            let metadata = format!(
                concat!(
                    "{{\"schema\":\"R0PObserverSessionV1\",",
                    "\"scenario\":\"{}\",",
                    "\"source_commit\":\"{}\",",
                    "\"source_tree\":\"{}\",",
                    "\"asset_tree\":\"{}\",",
                    "\"graphics_policy\":\"{}\"}}\n"
                ),
                json_escape(&env_text("BASTION_R0P_SCENARIO")),
                json_escape(&env_text("BASTION_R0P_SOURCE_COMMIT")),
                json_escape(&env_text("BASTION_R0P_SOURCE_TREE")),
                json_escape(&env_text("BASTION_R0P_ASSET_TREE")),
                json_escape(&env_text("BASTION_R0P_GRAPHICS_POLICY")),
            );
            if let Err(error) =
                atomic_write(&output.join("observer-session.json"), metadata.as_bytes())
            {
                tracing::error!(
                    target: "bastion_r0p",
                    ?error,
                    "failed to initialize observer files"
                );
                return None;
            }
            let process_started = Instant::now();
            let durable = match DurableFrameSinkV1::initialize(&output, process_started) {
                Ok(durable) => durable,
                Err(error) => {
                    tracing::error!(
                        target: "bastion_r0p",
                        ?error,
                        "failed to initialize durable observer sink"
                    );
                    return None;
                },
            };
            Some(Mutex::new(ObserverStateV1 {
                output,
                frame_sequence: 0,
                gpu_sequence: 0,
                process_started,
                active_frame_started: None,
                last_presented_at: None,
                completed_presented_frame: None,
                pending_gpu_frames: VecDeque::new(),
                work: WorkCountersV1::default(),
                scene: SceneCountersV1::default(),
                durable,
                failed: false,
            }))
        })
        .as_ref()
}

#[must_use]
pub fn enabled() -> bool { observer().is_some() }

fn with_state(action: impl FnOnce(&mut ObserverStateV1)) {
    let Some(observer) = observer() else {
        return;
    };
    match observer.lock() {
        Ok(mut state) if !state.failed => action(&mut state),
        Ok(_) => {},
        Err(error) => tracing::error!(target: "bastion_r0p", ?error, "observer lock poisoned"),
    }
}

pub fn record_pass() {
    with_state(|state| {
        state.work.pass_count = state.work.pass_count.saturating_add(1);
    });
}

pub fn record_draw(units: u32, instances: u32) {
    with_state(|state| {
        state.work.draw_count = state.work.draw_count.saturating_add(1);
        state.work.geometry_units = state.work.geometry_units.saturating_add(u64::from(units));
        state.work.instances = state.work.instances.saturating_add(u64::from(instances));
    });
}

pub fn record_bind_group_set() {
    with_state(|state| {
        state.work.bind_group_sets = state.work.bind_group_sets.saturating_add(1);
    });
}

pub fn record_buffer_upload(bytes: usize) {
    with_state(|state| {
        state.work.buffer_upload_ops = state.work.buffer_upload_ops.saturating_add(1);
        state.work.buffer_upload_bytes = state
            .work
            .buffer_upload_bytes
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    });
}

pub fn record_texture_upload(bytes: usize) {
    with_state(|state| {
        state.work.texture_upload_ops = state.work.texture_upload_ops.saturating_add(1);
        state.work.texture_upload_bytes = state
            .work
            .texture_upload_bytes
            .saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    });
}

pub fn record_submission() {
    with_state(|state| {
        state.work.submissions = state.work.submissions.saturating_add(1);
    });
}

pub fn record_scene_counters(counters: SceneCountersV1) {
    with_state(|state| state.scene = counters);
}

pub fn record_post_apply_horizon_camera_counters(counters: PostApplyHorizonCameraCountersV1) {
    with_state(|state| apply_post_apply_horizon_camera_counters(&mut state.scene, counters));
}

fn apply_post_apply_horizon_camera_counters(
    scene: &mut SceneCountersV1,
    counters: PostApplyHorizonCameraCountersV1,
) {
    scene.visible_horizon_camera_valid = counters.camera_valid;
    scene.visible_horizon_camera_mode = counters.camera_mode;
    scene.visible_horizon_projection = counters.projection;
    scene.visible_horizon_camera_focus_mm = counters.focus_mm;
    scene.visible_horizon_camera_position_mm = counters.position_mm;
    scene.visible_horizon_camera_yaw_microradians = counters.yaw_microradians;
    scene.visible_horizon_camera_pitch_microradians = counters.pitch_microradians;
    scene.visible_horizon_camera_distance_mm = counters.distance_mm;
    scene.visible_horizon_configured_base_fov_microradians =
        counters.configured_base_fov_microradians;
    scene.visible_horizon_camera_base_fov_microradians = counters.base_fov_microradians;
    scene.visible_horizon_camera_target_base_fov_microradians =
        counters.target_base_fov_microradians;
    scene.visible_horizon_camera_fov_microradians = counters.effective_fov_microradians;
    scene.visible_horizon_camera_fixation_millionths = counters.fixation_millionths;
    scene.visible_horizon_camera_target_fixation_millionths = counters.target_fixation_millionths;
    scene.visible_horizon_camera_aspect_millionths = counters.aspect_millionths;
    scene.visible_horizon_frustum_ground_width_mm = counters.frustum_ground_width_mm;
    scene.visible_horizon_frustum_ground_depth_mm = counters.frustum_ground_depth_mm;
    scene.visible_horizon_camera_token = counters.camera_token;
    scene.horizon_camera_path_id = counters.path_id;
    scene.horizon_camera_path_ordinal = counters.path_ordinal;
    scene.horizon_camera_path_token = counters.path_token;
    scene.horizon_surface_authority_available = counters.surface_authority_available;
    scene.horizon_cutaway_solid = counters.cutaway_solid;
    scene.horizon_underworld_rejected = counters.underworld_rejected;
    scene.horizon_sky_ground_expected = counters.sky_ground_expected;
    scene.horizon_focus_surface_mm = counters.focus_surface_mm;
    scene.horizon_camera_surface_mm = counters.camera_surface_mm;
    scene.horizon_minimum_clearance_mm = counters.minimum_clearance_mm;
}

pub fn record_adapter(name: &str, vendor: u32, device: u32, backend: &str, device_type: &str) {
    with_state(|state| {
        let record = format!(
            concat!(
                "{{\"schema\":\"R0PAdapterV1\",\"name\":\"{}\",\"vendor\":{},",
                "\"device\":{},\"backend\":\"{}\",\"device_type\":\"{}\"}}\n"
            ),
            json_escape(name),
            vendor,
            device,
            json_escape(backend),
            json_escape(device_type),
        );
        if atomic_write(&state.output.join("adapter.json"), record.as_bytes()).is_err() {
            state.failed = true;
        }
    });
}

/// Starts diagnostic timing for the current production frame. The timestamp is
/// process-relative and is never read by renderer policy.
pub fn frame_begin() {
    with_state(|state| {
        state.active_frame_started = Some(Instant::now());
    });
}

/// Completes diagnostic timing immediately after queue submission and
/// `SurfaceTexture::present` return. The result is serialized later with the
/// reconciled CPU phases and never feeds rendering decisions.
pub fn frame_presented() {
    with_state(|state| {
        let now = Instant::now();
        let Some(frame_started) = state.active_frame_started.take() else {
            return;
        };
        let (frame_begin_ns, begin_overflow) =
            checked_duration_ns(state.process_started, frame_started);
        let (present_end_ns, end_overflow) = checked_duration_ns(state.process_started, now);
        let (presented_interval_ns, interval_overflow) = match state.last_presented_at {
            Some(previous) => {
                let (interval, overflowed) = checked_duration_ns(previous, now);
                (Some(interval), overflowed)
            },
            None => (None, false),
        };
        state.last_presented_at = Some(now);
        state.completed_presented_frame = Some(PresentedFrameTimingV1 {
            frame_ordinal: state.frame_sequence,
            frame_begin_ns,
            present_end_ns,
            presented_interval_ns,
            overflowed: begin_overflow || end_overflow || interval_overflow,
        });
    });
}

/// Mirror `wgpu-profiler`'s four-frame pending policy so a drained timing is
/// associated with the production frame that submitted it.
pub fn gpu_frame_submitted() {
    with_state(|state| {
        let frame_ordinal = state
            .completed_presented_frame
            .map(|timing| timing.frame_ordinal)
            .unwrap_or(state.frame_sequence);
        submit_gpu_pending(&mut state.pending_gpu_frames, frame_ordinal);
    });
}

pub fn record_gpu_timings(timings: &[(u8, &str, f64)]) {
    with_state(|state| {
        let Some(frame_sequence) = state.pending_gpu_frames.pop_front() else {
            return;
        };
        let total_top_level_ns = timings
            .iter()
            .filter(|(depth, _, _)| *depth == 0)
            .map(|(_, _, seconds)| seconds_to_ns(*seconds))
            .fold(0_u64, u64::saturating_add);
        let mut encoded = String::new();
        for (index, (depth, label, seconds)) in timings.iter().enumerate() {
            if index > 0 {
                encoded.push(',');
            }
            encoded.push_str(&format!(
                "{{\"depth\":{},\"label\":\"{}\",\"duration_ns\":{}}}",
                depth,
                json_escape(label),
                seconds_to_ns(*seconds)
            ));
        }
        let line = format!(
            concat!(
                "{{\"schema\":\"R0PGpuFrameV1\",\"gpu_sequence\":{},",
                "\"frame_sequence\":{},\"top_level_ns\":{},\"timings\":[{}]}}\n"
            ),
            state.gpu_sequence, frame_sequence, total_top_level_ns, encoded
        );
        state.gpu_sequence = state.gpu_sequence.saturating_add(1);
        state.durable.push_gpu_frame(frame_sequence, line);
    });
}

pub fn record_cpu_frame(phases: CpuFramePhasesV1) {
    with_state(|state| {
        let Some(timing) = state.completed_presented_frame.take() else {
            state.work = WorkCountersV1::default();
            return;
        };
        let presentation = crate::r1a_presentation::ready_token();
        let presentation_generation = presentation
            .map(|token| token.client_applied_generation)
            .unwrap_or(0);
        let presentation_frame = presentation
            .map(|token| hex_digest(&token.frame_digest))
            .unwrap_or_else(|| "0".repeat(64));
        let presentation_resources = presentation
            .map(|token| hex_digest(&token.resource_set_digest))
            .unwrap_or_else(|| "0".repeat(64));
        let busy_ns = phases.total_wall_ns.saturating_sub(phases.pacing_ns);
        let work = state.work;
        let scene = state.scene;
        let terrain_streaming = terrain_streaming_json_fields_v1(scene);
        let visible_horizon = visible_horizon_json_fields_v1(scene);
        let interval = timing
            .presented_interval_ns
            .map_or_else(|| "null".to_owned(), |value| value.to_string());
        let timing_status = if timing.overflowed {
            "OVERFLOW"
        } else if timing.presented_interval_ns.is_none() {
            "FIRST_FRAME_INTERVAL_UNAVAILABLE"
        } else {
            "AVAILABLE"
        };
        let line = format!(
            concat!(
                "{{\"schema\":\"R0PFrameV2\",\"frame_sequence\":{},",
                "\"frame_ordinal\":{},\"frame_begin_ns\":{},\"present_end_ns\":{},",
                "\"presented_frame_interval_ns\":{},\"timing_status\":\"{}\",",
                "\"gpu_timing_status\":\"SEPARATE_STREAM_OR_UNAVAILABLE\",",
                "\"phases_reconcile\":{},\"tick_ns\":{},\"render_submit_ns\":{},",
                "\"post_render_ns\":{},\"pacing_ns\":{},\"maintain_ns\":{},",
                "\"total_wall_ns\":{},\"busy_ns\":{},\"pass_count\":{},",
                "\"draw_count\":{},\"geometry_units\":{},\"instances\":{},",
                "\"bind_group_sets\":{},\"buffer_upload_ops\":{},",
                "\"buffer_upload_bytes\":{},\"texture_upload_ops\":{},",
                "\"texture_upload_bytes\":{},\"submissions\":{},",
                "\"terrain_chunks\":{},\"visible_terrain_chunks\":{},",
                "\"shadow_terrain_chunks\":{},",
                "{},",
                "{},",
                "\"loaded_distance_blocks\":{},",
                "\"terrain_view_distance_chunks\":{},\"terrain_mesh_queue\":{},",
                "\"terrain_mesh_queue_pruned_total\":{},",
                "\"figures\":{},\"visible_figures\":{},",
                "\"particles\":{},\"visible_particles\":{},",
                "\"presentation_generation\":{},\"presentation_frame_sha256\":\"{}\",",
                "\"presentation_resource_set_sha256\":\"{}\"}}\n"
            ),
            state.frame_sequence,
            timing.frame_ordinal,
            timing.frame_begin_ns,
            timing.present_end_ns,
            interval,
            timing_status,
            phases.reconciles(),
            phases.tick_ns,
            phases.render_submit_ns,
            phases.post_render_ns,
            phases.pacing_ns,
            phases.maintain_ns,
            phases.total_wall_ns,
            busy_ns,
            work.pass_count,
            work.draw_count,
            work.geometry_units,
            work.instances,
            work.bind_group_sets,
            work.buffer_upload_ops,
            work.buffer_upload_bytes,
            work.texture_upload_ops,
            work.texture_upload_bytes,
            work.submissions,
            scene.terrain_chunks,
            scene.visible_terrain_chunks,
            scene.shadow_terrain_chunks,
            terrain_streaming,
            visible_horizon,
            scene.loaded_distance_blocks,
            scene.terrain_view_distance_chunks,
            scene.terrain_mesh_queue,
            scene.terrain_mesh_queue_pruned_total,
            scene.figures,
            scene.visible_figures,
            scene.particles,
            scene.visible_particles,
            presentation_generation,
            presentation_frame,
            presentation_resources,
        );
        state.frame_sequence = state.frame_sequence.saturating_add(1);
        state.work = WorkCountersV1::default();
        if state
            .durable
            .push_frame(timing.frame_ordinal, line, Instant::now())
            .is_err()
        {
            state.failed = true;
        }
    });
}

fn visible_horizon_json_fields_v1(scene: SceneCountersV1) -> String {
    format!(
        concat!(
            "\"visible_horizon_fixture_selected\":{},",
            "\"visible_horizon_camera_valid\":{},",
            "\"visible_horizon_camera_mode\":{},",
            "\"visible_horizon_projection\":{},",
            "\"visible_horizon_camera_focus_mm\":[{},{},{}],",
            "\"visible_horizon_camera_position_mm\":[{},{},{}],",
            "\"visible_horizon_camera_yaw_microradians\":{},",
            "\"visible_horizon_camera_pitch_microradians\":{},",
            "\"visible_horizon_camera_distance_mm\":{},",
            "\"visible_horizon_configured_base_fov_microradians\":{},",
            "\"visible_horizon_camera_base_fov_microradians\":{},",
            "\"visible_horizon_camera_target_base_fov_microradians\":{},",
            "\"visible_horizon_camera_fov_microradians\":{},",
            "\"visible_horizon_camera_fixation_millionths\":{},",
            "\"visible_horizon_camera_target_fixation_millionths\":{},",
            "\"visible_horizon_camera_aspect_millionths\":{},",
            "\"visible_horizon_frustum_ground_width_mm\":{},",
            "\"visible_horizon_frustum_ground_depth_mm\":{},",
            "\"visible_horizon_camera_token\":\"{}\",",
            "\"horizon_camera_path_id\":{},",
            "\"horizon_camera_path_ordinal\":{},",
            "\"horizon_camera_path_token\":\"{}\",",
            "\"horizon_surface_authority_available\":{},",
            "\"horizon_cutaway_solid\":{},",
            "\"horizon_underworld_rejected\":{},",
            "\"horizon_sky_ground_expected\":{},",
            "\"horizon_focus_surface_mm\":{},",
            "\"horizon_camera_surface_mm\":{},",
            "\"horizon_minimum_clearance_mm\":{},",
            "\"horizon_terrain_revision\":{},",
            "\"horizon_meshed_high_detail_chunks\":{},",
            "\"visible_horizon_near_0_8_chunks\":{},",
            "\"visible_horizon_reference_9_16_chunks\":{},",
            "\"visible_horizon_far_17_24_chunks\":{},",
            "\"visible_horizon_beyond_24_chunks\":{},",
            "\"visible_horizon_max_radius_chunks\":{},",
            "\"visible_horizon_max_distance_blocks\":{},",
            "\"visible_horizon_lod_terrain_draw_ready\":{},",
            "\"visible_horizon_lod_terrain_detail\":{},",
            "\"visible_horizon_lod_distance_blocks\":{}"
        ),
        scene.visible_horizon_fixture_selected,
        scene.visible_horizon_camera_valid,
        scene.visible_horizon_camera_mode,
        scene.visible_horizon_projection,
        scene.visible_horizon_camera_focus_mm[0],
        scene.visible_horizon_camera_focus_mm[1],
        scene.visible_horizon_camera_focus_mm[2],
        scene.visible_horizon_camera_position_mm[0],
        scene.visible_horizon_camera_position_mm[1],
        scene.visible_horizon_camera_position_mm[2],
        scene.visible_horizon_camera_yaw_microradians,
        scene.visible_horizon_camera_pitch_microradians,
        scene.visible_horizon_camera_distance_mm,
        scene.visible_horizon_configured_base_fov_microradians,
        scene.visible_horizon_camera_base_fov_microradians,
        scene.visible_horizon_camera_target_base_fov_microradians,
        scene.visible_horizon_camera_fov_microradians,
        scene.visible_horizon_camera_fixation_millionths,
        scene.visible_horizon_camera_target_fixation_millionths,
        scene.visible_horizon_camera_aspect_millionths,
        scene.visible_horizon_frustum_ground_width_mm,
        scene.visible_horizon_frustum_ground_depth_mm,
        hex_digest(&scene.visible_horizon_camera_token),
        scene.horizon_camera_path_id,
        scene.horizon_camera_path_ordinal,
        hex_digest(&scene.horizon_camera_path_token),
        scene.horizon_surface_authority_available,
        scene.horizon_cutaway_solid,
        scene.horizon_underworld_rejected,
        scene.horizon_sky_ground_expected,
        scene.horizon_focus_surface_mm,
        scene.horizon_camera_surface_mm,
        scene.horizon_minimum_clearance_mm,
        scene.horizon_terrain_revision,
        scene.horizon_meshed_high_detail_chunks,
        scene.visible_horizon_near_0_8_chunks,
        scene.visible_horizon_reference_9_16_chunks,
        scene.visible_horizon_far_17_24_chunks,
        scene.visible_horizon_beyond_24_chunks,
        scene.visible_horizon_max_radius_chunks,
        scene.visible_horizon_max_distance_blocks,
        scene.visible_horizon_lod_terrain_draw_ready,
        scene.visible_horizon_lod_terrain_detail,
        scene.visible_horizon_lod_distance_blocks,
    )
}

fn terrain_streaming_json_fields_v1(scene: SceneCountersV1) -> String {
    format!(
        concat!(
            "\"terrain_requested_view_distance_chunks\":{},",
            "\"terrain_server_authorized_view_distance_chunks\":{},",
            "\"terrain_server_authority_available\":{},",
            "\"terrain_chunks_received_total\":{},\"terrain_resident_chunks\":{},",
            "\"terrain_pending_chunk_requests\":{},\"terrain_server_completed_tick\":{}"
        ),
        scene.terrain_requested_view_distance_chunks,
        scene.terrain_server_authorized_view_distance_chunks,
        scene.terrain_server_authority_available,
        scene.terrain_chunks_received_total,
        scene.terrain_resident_chunks,
        scene.terrain_pending_chunk_requests,
        scene.terrain_server_completed_tick,
    )
}

fn hex_digest(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub fn finalize() {
    with_state(|state| {
        if state.durable.flush_all(Instant::now()).is_err() {
            state.failed = true;
            return;
        }
        let terminal = format!(
            concat!(
                "{{\"schema\":\"R0PObserverTerminalV1\",\"terminal\":\"RAW_MEASUREMENT_COMPLETE\",",
                "\"frames\":{},\"gpu_frames\":{},\"pending_gpu_frames\":{},",
                "\"durable_chunks\":{},\"durable_through_ordinal\":{},",
                "\"dropped_frame_records\":{},\"dropped_gpu_records\":{}}}\n"
            ),
            state.frame_sequence,
            state.gpu_sequence,
            state.pending_gpu_frames.len(),
            state.durable.chunk_sequence,
            state
                .durable
                .durable_through_ordinal
                .map_or_else(|| "null".to_owned(), |value| value.to_string()),
            state.durable.dropped_frame_records,
            state.durable.dropped_gpu_records,
        );
        if atomic_write(
            &state.output.join("observer-terminal.json"),
            terminal.as_bytes(),
        )
        .is_err()
        {
            state.failed = true;
        }
    });
}

fn env_text(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| "UNDECLARED".to_owned())
}

fn seconds_to_ns(seconds: f64) -> u64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        0
    } else {
        let nanos = seconds * 1_000_000_000.0;
        if nanos >= u64::MAX as f64 {
            u64::MAX
        } else {
            nanos.round() as u64
        }
    }
}

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", u32::from(character)));
            },
            character => output.push(character),
        }
    }
    output
}

fn push_bounded(
    records: &mut VecDeque<BufferedRecordV1>,
    record: BufferedRecordV1,
    dropped: &mut u64,
) {
    if records.len() == MAX_BUFFERED_RECORDS {
        records.pop_front();
        *dropped = dropped.saturating_add(1);
    }
    records.push_back(record);
}

fn submit_gpu_pending(pending: &mut VecDeque<u64>, frame_sequence: u64) {
    if pending.len() == GPU_PENDING_LIMIT {
        pending.pop_back();
    }
    pending.push_back(frame_sequence);
}

fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let staged = path.with_extension("staging");
    let mut file = fs::File::create(&staged)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(staged, path)
}

fn append_sync(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_data()
}

fn checked_duration_ns(start: Instant, end: Instant) -> (u64, bool) {
    let Some(duration) = end.checked_duration_since(start) else {
        return (0, true);
    };
    match u64::try_from(duration.as_nanos()) {
        Ok(value) => (value, false),
        Err(_) => (u64::MAX, true),
    }
}

#[cfg(test)]
fn recover_durable_frames(output: &Path) -> std::io::Result<Vec<String>> {
    let acknowledged = fs::read_to_string(output.join("observer-chunks.jsonl"))?
        .lines()
        .try_fold(0_usize, |total, line| {
            let marker = "\"record_count\":";
            let Some(start) = line.find(marker).map(|index| index + marker.len()) else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "missing durable record count",
                ));
            };
            let digits = line[start..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>();
            let count = digits.parse::<usize>().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid durable record count",
                )
            })?;
            total.checked_add(count).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "durable record count overflow",
                )
            })
        })?;
    let frames = fs::read_to_string(output.join("frames.jsonl"))?;
    Ok(frames
        .lines()
        .take(acknowledged)
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_OUTPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn test_output(name: &str) -> PathBuf {
        let sequence = TEST_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bastion-r0p-{name}-{}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn cpu_phase_denominator_reconciles_exactly() {
        let valid = CpuFramePhasesV1 {
            tick_ns: 1,
            render_submit_ns: 2,
            post_render_ns: 3,
            pacing_ns: 4,
            maintain_ns: 5,
            total_wall_ns: 15,
        };
        assert!(valid.reconciles());
        assert!(
            !CpuFramePhasesV1 {
                total_wall_ns: 14,
                ..valid
            }
            .reconciles()
        );
    }

    #[test]
    fn json_escape_is_single_line_and_round_trip_safe() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn invalid_gpu_seconds_fail_closed_to_bounded_integer() {
        assert_eq!(seconds_to_ns(f64::NAN), 0);
        assert_eq!(seconds_to_ns(-1.0), 0);
        assert_eq!(seconds_to_ns(f64::INFINITY), 0);
        assert_eq!(seconds_to_ns(0.000_001), 1_000);
    }

    #[test]
    fn evidence_buffer_retains_latest_bounded_window() {
        let mut records = VecDeque::new();
        let mut dropped = 0;
        for index in 0..=MAX_BUFFERED_RECORDS {
            push_bounded(
                &mut records,
                BufferedRecordV1 {
                    ordinal: u64::try_from(index).unwrap_or(u64::MAX),
                    line: index.to_string(),
                },
                &mut dropped,
            );
        }
        assert_eq!(records.len(), MAX_BUFFERED_RECORDS);
        assert_eq!(
            records.front().map(|record| record.line.as_str()),
            Some("1")
        );
        assert_eq!(
            records
                .back()
                .and_then(|record| record.line.parse::<usize>().ok()),
            Some(MAX_BUFFERED_RECORDS)
        );
        assert_eq!(dropped, 1);
    }

    #[test]
    fn gpu_pending_queue_mirrors_profiler_drop_newest_policy() {
        let mut pending = VecDeque::new();
        for frame in 0..=GPU_PENDING_LIMIT as u64 {
            submit_gpu_pending(&mut pending, frame);
        }
        assert_eq!(pending.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 4]);
    }

    #[test]
    fn durable_chunks_survive_forced_stop_without_finalizer() {
        let output = test_output("forced-stop");
        fs::create_dir_all(&output).expect("temporary observer output");
        let started = Instant::now();
        let mut sink = DurableFrameSinkV1::initialize(&output, started).expect("durable sink");
        for ordinal in 0..121 {
            sink.push_frame(
                ordinal,
                format!("{{\"frame_ordinal\":{ordinal}}}\n"),
                started + Duration::from_millis(17 * ordinal),
            )
            .expect("append frame");
        }
        drop(sink);

        let recovered = recover_durable_frames(&output).expect("recover acknowledged chunks");
        assert_eq!(recovered.len(), 120);
        assert_eq!(
            recovered.first().map(String::as_str),
            Some("{\"frame_ordinal\":0}")
        );
        assert_eq!(
            recovered.last().map(String::as_str),
            Some("{\"frame_ordinal\":119}")
        );
        fs::remove_dir_all(output).expect("remove observer output");
    }

    #[test]
    fn durable_chunk_acknowledgements_are_monotonic_and_bounded() {
        let output = test_output("chunks");
        fs::create_dir_all(&output).expect("temporary observer output");
        let started = Instant::now();
        let mut sink = DurableFrameSinkV1::initialize(&output, started).expect("durable sink");
        for ordinal in 0..120 {
            sink.push_frame(
                ordinal,
                format!("{{\"frame_ordinal\":{ordinal}}}\n"),
                started + Duration::from_millis(17 * ordinal),
            )
            .expect("append frame");
        }
        let chunks =
            fs::read_to_string(output.join("observer-chunks.jsonl")).expect("read durable chunks");
        assert_eq!(chunks.lines().count(), 2);
        assert!(chunks.contains("\"chunk_sequence\":0"));
        assert!(chunks.contains("\"first_frame_ordinal\":0"));
        assert!(chunks.contains("\"last_frame_ordinal\":59"));
        assert!(chunks.contains("\"chunk_sequence\":1"));
        assert!(chunks.contains("\"first_frame_ordinal\":60"));
        assert!(chunks.contains("\"last_frame_ordinal\":119"));
        fs::remove_dir_all(output).expect("remove observer output");
    }

    #[test]
    fn timing_conversion_is_checked_and_process_relative() {
        let start = Instant::now();
        let end = start + Duration::from_micros(123);
        assert_eq!(checked_duration_ns(start, end), (123_000, false));
        assert_eq!(checked_duration_ns(end, start), (0, true));
    }

    #[test]
    fn streaming_stage_telemetry_serializes_every_authority_boundary() {
        let fields = terrain_streaming_json_fields_v1(SceneCountersV1 {
            terrain_requested_view_distance_chunks: 24,
            terrain_server_authorized_view_distance_chunks: 24,
            terrain_server_authority_available: true,
            terrain_chunks_received_total: 1_797,
            terrain_resident_chunks: 1_790,
            terrain_pending_chunk_requests: 7,
            terrain_server_completed_tick: 9_000,
            ..SceneCountersV1::default()
        });
        assert_eq!(
            fields,
            concat!(
                "\"terrain_requested_view_distance_chunks\":24,",
                "\"terrain_server_authorized_view_distance_chunks\":24,",
                "\"terrain_server_authority_available\":true,",
                "\"terrain_chunks_received_total\":1797,\"terrain_resident_chunks\":1790,",
                "\"terrain_pending_chunk_requests\":7,\"terrain_server_completed_tick\":9000"
            )
        );
    }

    #[test]
    fn post_apply_camera_snapshot_replaces_pre_maintenance_authority() {
        let mut scene = SceneCountersV1 {
            visible_horizon_camera_valid: false,
            visible_horizon_camera_focus_mm: [1, 1, 1],
            horizon_camera_path_ordinal: 8,
            ..SceneCountersV1::default()
        };
        let counters = PostApplyHorizonCameraCountersV1 {
            camera_valid: true,
            camera_mode: 3,
            projection: 1,
            focus_mm: [16_384_500, 16_384_430, 3_268],
            position_mm: [16_384_500, 16_320_432, 3_827],
            yaw_microradians: 0,
            pitch_microradians: 8_727,
            distance_mm: 64_000,
            configured_base_fov_microradians: 1_221_730,
            base_fov_microradians: 1_221_730,
            target_base_fov_microradians: 1_221_730,
            effective_fov_microradians: 1_221_730,
            fixation_millionths: 1_000_000,
            target_fixation_millionths: 1_000_000,
            aspect_millionths: 1_777_778,
            frustum_ground_width_mm: 159_336,
            frustum_ground_depth_mm: 10_270_161,
            camera_token: [3; 32],
            path_id: 1,
            path_ordinal: 1_484,
            path_token: [4; 32],
            surface_authority_available: true,
            cutaway_solid: true,
            underworld_rejected: false,
            sky_ground_expected: true,
            focus_surface_mm: 0,
            camera_surface_mm: 0,
            minimum_clearance_mm: 2_016,
        };
        apply_post_apply_horizon_camera_counters(&mut scene, counters);
        assert!(scene.visible_horizon_camera_valid);
        assert_eq!(scene.visible_horizon_camera_focus_mm, counters.focus_mm);
        assert_eq!(
            scene.visible_horizon_camera_position_mm,
            counters.position_mm
        );
        assert_eq!(scene.visible_horizon_camera_token, counters.camera_token);
        assert_eq!(scene.horizon_camera_path_ordinal, counters.path_ordinal);
        assert_eq!(scene.horizon_minimum_clearance_mm, 2_016);
    }

    #[test]
    fn visible_horizon_telemetry_binds_camera_frustum_rings_and_lod_draw() {
        let fields = visible_horizon_json_fields_v1(SceneCountersV1 {
            visible_horizon_fixture_selected: true,
            visible_horizon_camera_valid: true,
            visible_horizon_camera_mode: 3,
            visible_horizon_projection: 1,
            visible_horizon_camera_focus_mm: [1, 2, 1_000],
            visible_horizon_camera_position_mm: [3, 4, 5],
            visible_horizon_camera_yaw_microradians: 0,
            visible_horizon_camera_pitch_microradians: 349_066,
            visible_horizon_camera_distance_mm: 384_000,
            visible_horizon_configured_base_fov_microradians: 1_100_000,
            visible_horizon_camera_base_fov_microradians: 1_100_000,
            visible_horizon_camera_target_base_fov_microradians: 1_100_000,
            visible_horizon_camera_fov_microradians: 1_100_000,
            visible_horizon_camera_fixation_millionths: 1_000_000,
            visible_horizon_camera_target_fixation_millionths: 1_000_000,
            visible_horizon_camera_aspect_millionths: 1_777_778,
            visible_horizon_frustum_ground_width_mm: 837_000,
            visible_horizon_frustum_ground_depth_mm: 1_376_000,
            visible_horizon_camera_token: [0xab; 32],
            horizon_camera_path_id: 4,
            horizon_camera_path_ordinal: 9_001,
            horizon_camera_path_token: [0xcd; 32],
            horizon_surface_authority_available: true,
            horizon_cutaway_solid: true,
            horizon_underworld_rejected: false,
            horizon_sky_ground_expected: true,
            horizon_focus_surface_mm: 40_000,
            horizon_camera_surface_mm: 41_000,
            horizon_minimum_clearance_mm: 8_000,
            horizon_terrain_revision: 9_001,
            horizon_meshed_high_detail_chunks: 1_790,
            visible_horizon_near_0_8_chunks: 17,
            visible_horizon_reference_9_16_chunks: 53,
            visible_horizon_far_17_24_chunks: 89,
            visible_horizon_beyond_24_chunks: 2,
            visible_horizon_max_radius_chunks: 25,
            visible_horizon_max_distance_blocks: 800,
            visible_horizon_lod_terrain_draw_ready: true,
            visible_horizon_lod_terrain_detail: 400,
            visible_horizon_lod_distance_blocks: 675,
            ..SceneCountersV1::default()
        });
        assert!(fields.contains("\"visible_horizon_fixture_selected\":true"));
        assert!(fields.contains("\"visible_horizon_camera_valid\":true"));
        assert!(fields.contains("\"visible_horizon_camera_focus_mm\":[1,2,1000]"));
        assert!(fields.contains("\"visible_horizon_camera_pitch_microradians\":349066"));
        assert!(fields.contains("\"visible_horizon_configured_base_fov_microradians\":1100000"));
        assert!(fields.contains("\"visible_horizon_camera_base_fov_microradians\":1100000"));
        assert!(fields.contains("\"visible_horizon_camera_target_base_fov_microradians\":1100000"));
        assert!(fields.contains("\"visible_horizon_camera_fixation_millionths\":1000000"));
        assert!(fields.contains("\"visible_horizon_camera_target_fixation_millionths\":1000000"));
        assert!(fields.contains("\"visible_horizon_frustum_ground_depth_mm\":1376000"));
        assert!(fields.contains(&format!(
            "\"visible_horizon_camera_token\":\"{}\"",
            "ab".repeat(32)
        )));
        assert!(fields.contains("\"horizon_camera_path_id\":4"));
        assert!(fields.contains("\"horizon_camera_path_ordinal\":9001"));
        assert!(fields.contains(&format!(
            "\"horizon_camera_path_token\":\"{}\"",
            "cd".repeat(32)
        )));
        assert!(fields.contains("\"horizon_surface_authority_available\":true"));
        assert!(fields.contains("\"horizon_cutaway_solid\":true"));
        assert!(fields.contains("\"horizon_underworld_rejected\":false"));
        assert!(fields.contains("\"horizon_minimum_clearance_mm\":8000"));
        assert!(fields.contains("\"horizon_terrain_revision\":9001"));
        assert!(fields.contains("\"horizon_meshed_high_detail_chunks\":1790"));
        assert!(fields.contains("\"visible_horizon_far_17_24_chunks\":89"));
        assert!(fields.contains("\"visible_horizon_max_distance_blocks\":800"));
        assert!(fields.contains("\"visible_horizon_lod_terrain_draw_ready\":true"));
        assert!(fields.contains("\"visible_horizon_lod_terrain_detail\":400"));
        assert!(fields.ends_with("\"visible_horizon_lod_distance_blocks\":675"));
    }
}
