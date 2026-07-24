//! Opt-in production renderer observatory for the R0P measurement lane.
//!
//! The observer is inert unless `BASTION_R0P_OUTPUT` is set. It records actual
//! production frame work; it never feeds measurements back into rendering.

use std::{
    collections::VecDeque,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const GPU_PENDING_LIMIT: usize = 4;
const MAX_BUFFERED_RECORDS: usize = 4_096;

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
    pub figures: u64,
    pub visible_figures: u64,
    pub particles: u64,
    pub visible_particles: u64,
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
    pending_gpu_frames: VecDeque<u64>,
    work: WorkCountersV1,
    scene: SceneCountersV1,
    frame_lines: VecDeque<String>,
    gpu_lines: VecDeque<String>,
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
            if atomic_write(&output.join("observer-session.json"), metadata.as_bytes()).is_err() {
                tracing::error!(target: "bastion_r0p", "failed to initialize observer files");
                return None;
            }
            Some(Mutex::new(ObserverStateV1 {
                output,
                frame_sequence: 0,
                gpu_sequence: 0,
                pending_gpu_frames: VecDeque::new(),
                work: WorkCountersV1::default(),
                scene: SceneCountersV1::default(),
                frame_lines: VecDeque::new(),
                gpu_lines: VecDeque::new(),
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

/// Mirror `wgpu-profiler`'s four-frame pending policy so a drained timing is
/// associated with the production frame that submitted it.
pub fn gpu_frame_submitted() {
    with_state(|state| {
        submit_gpu_pending(&mut state.pending_gpu_frames, state.frame_sequence);
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
        push_bounded(&mut state.gpu_lines, line);
    });
}

pub fn record_cpu_frame(phases: CpuFramePhasesV1) {
    with_state(|state| {
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
        let line = format!(
            concat!(
                "{{\"schema\":\"R0PFrameV1\",\"frame_sequence\":{},",
                "\"phases_reconcile\":{},\"tick_ns\":{},\"render_submit_ns\":{},",
                "\"post_render_ns\":{},\"pacing_ns\":{},\"maintain_ns\":{},",
                "\"total_wall_ns\":{},\"busy_ns\":{},\"pass_count\":{},",
                "\"draw_count\":{},\"geometry_units\":{},\"instances\":{},",
                "\"bind_group_sets\":{},\"buffer_upload_ops\":{},",
                "\"buffer_upload_bytes\":{},\"texture_upload_ops\":{},",
                "\"texture_upload_bytes\":{},\"submissions\":{},",
                "\"terrain_chunks\":{},\"visible_terrain_chunks\":{},",
                "\"shadow_terrain_chunks\":{},\"figures\":{},\"visible_figures\":{},",
                "\"particles\":{},\"visible_particles\":{},",
                "\"presentation_generation\":{},\"presentation_frame_sha256\":\"{}\",",
                "\"presentation_resource_set_sha256\":\"{}\"}}\n"
            ),
            state.frame_sequence,
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
            scene.figures,
            scene.visible_figures,
            scene.particles,
            scene.visible_particles,
            presentation_generation,
            presentation_frame,
            presentation_resources,
        );
        push_bounded(&mut state.frame_lines, line);
        state.frame_sequence = state.frame_sequence.saturating_add(1);
        state.work = WorkCountersV1::default();
    });
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
        let frames = state.frame_lines.iter().cloned().collect::<String>();
        let gpu_frames = state.gpu_lines.iter().cloned().collect::<String>();
        if atomic_write(&state.output.join("frames.jsonl"), frames.as_bytes()).is_err()
            || atomic_write(
                &state.output.join("gpu-frames.jsonl"),
                gpu_frames.as_bytes(),
            )
            .is_err()
        {
            state.failed = true;
            return;
        }
        let terminal = format!(
            concat!(
                "{{\"schema\":\"R0PObserverTerminalV1\",\"terminal\":\"RAW_MEASUREMENT_COMPLETE\",",
                "\"frames\":{},\"gpu_frames\":{},\"pending_gpu_frames\":{}}}\n"
            ),
            state.frame_sequence,
            state.gpu_sequence,
            state.pending_gpu_frames.len()
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

fn push_bounded(records: &mut VecDeque<String>, record: String) {
    if records.len() == MAX_BUFFERED_RECORDS {
        records.pop_front();
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

#[cfg(test)]
mod tests {
    use super::*;

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
        for index in 0..=MAX_BUFFERED_RECORDS {
            push_bounded(&mut records, index.to_string());
        }
        assert_eq!(records.len(), MAX_BUFFERED_RECORDS);
        assert_eq!(records.front().map(String::as_str), Some("1"));
        assert_eq!(
            records.back().and_then(|value| value.parse::<usize>().ok()),
            Some(MAX_BUFFERED_RECORDS)
        );
    }

    #[test]
    fn gpu_pending_queue_mirrors_profiler_drop_newest_policy() {
        let mut pending = VecDeque::new();
        for frame in 0..=GPU_PENDING_LIMIT as u64 {
            submit_gpu_pending(&mut pending, frame);
        }
        assert_eq!(pending.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 4]);
    }
}
