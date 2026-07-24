//! Flag-gated renderer pipeline-identity seam.

use super::PipelineModes;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub const PIPELINE_IDENTITY_SCHEMA_V1: (u16, u16) = (1, 0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipelineIdentityErrorV1 {
    UnsortedShaders,
    InvalidDomain,
}

#[must_use]
pub fn backend_tag(backend: wgpu::Backend) -> u8 {
    match backend {
        wgpu::Backend::Noop => 0,
        wgpu::Backend::Vulkan => 1,
        wgpu::Backend::Metal => 2,
        wgpu::Backend::Dx12 => 3,
        wgpu::Backend::Gl => 4,
        wgpu::Backend::BrowserWebGpu => 5,
    }
}

pub fn pipeline_identity_digest_v1(
    sorted_shader_sources: &[(String, String)],
    modes: &PipelineModes,
    backend_tag: u8,
    surface_format: &str,
    intermediate_format: &str,
) -> Result<[u8; 32], PipelineIdentityErrorV1> {
    if sorted_shader_sources
        .windows(2)
        .any(|pair| pair[0].0 > pair[1].0)
    {
        return Err(PipelineIdentityErrorV1::UnsortedShaders);
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(
        &u64::try_from(sorted_shader_sources.len())
            .map_err(|_| PipelineIdentityErrorV1::InvalidDomain)?
            .to_le_bytes(),
    );
    for (name, source) in sorted_shader_sources {
        for bytes in [name.as_bytes(), source.as_bytes()] {
            payload.extend_from_slice(
                &u64::try_from(bytes.len())
                    .map_err(|_| PipelineIdentityErrorV1::InvalidDomain)?
                    .to_le_bytes(),
            );
            payload.extend_from_slice(bytes);
        }
    }
    payload.extend_from_slice(
        &modes
            .bastion_identity_bytes()
            .map_err(|_| PipelineIdentityErrorV1::InvalidDomain)?,
    );
    payload.push(backend_tag);
    for label in [surface_format, intermediate_format] {
        payload.extend_from_slice(
            &u64::try_from(label.len())
                .map_err(|_| PipelineIdentityErrorV1::InvalidDomain)?
                .to_le_bytes(),
        );
        payload.extend_from_slice(label.as_bytes());
    }
    bastion_renderer_r0d::domain_hash_v1("bastion/r0d/pipeline-identity", 1, 0, &payload)
        .map_err(|_| PipelineIdentityErrorV1::InvalidDomain)
}

pub fn manifest_output_path() -> Option<std::path::PathBuf> {
    std::env::var_os("BASTION_R0D_MANIFEST").map(std::path::PathBuf::from)
}

#[must_use]
pub fn manifest_enabled() -> bool { std::env::var_os("BASTION_R0D_MANIFEST").is_some() }

fn hex_digest(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn append_evidence_line(path: &Path, line: &str) {
    if let Err(error) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
    {
        tracing::warn!(target: "bastion_r0d", "R0D evidence write failed: {error}");
    }
}

pub fn emit_manifest(context: &str, digest: &[u8; 32]) {
    let digest = hex_digest(digest);
    tracing::info!(target: "bastion_r0d", "R0D-PIPELINE-IDENTITY[{context}]: {digest}");
    if let Some(path) = manifest_output_path()
        && path.as_os_str() != "1"
        && !path.as_os_str().is_empty()
    {
        append_evidence_line(&path, &format!("{context} {digest}\n"));
    }
}

static PASS_RECORDS: Mutex<Vec<(u16, &'static str)>> = Mutex::new(Vec::new());
static LATEST_PASS_TAPE: Mutex<Option<String>> = Mutex::new(None);

pub use bastion_renderer_r0d::pass_graph::voxygen_ranks as ranks;

pub fn record_pass(rank: u16, name: &'static str) {
    if !manifest_enabled() {
        return;
    }
    match PASS_RECORDS.lock() {
        Ok(mut records) => records.push((rank, name)),
        Err(error) => tracing::warn!(target: "bastion_r0d", "pass tape lock failed: {error}"),
    }
    match DRAW_RECORDS.lock() {
        Ok(mut records) => records.push((0, u32::from(rank), 0)),
        Err(error) => tracing::warn!(target: "bastion_r0d", "draw tape lock failed: {error}"),
    }
}

fn pass_tape_snapshot(records: &[(u16, &'static str)]) -> (String, bool) {
    let monotonic = records.windows(2).all(|pair| pair[0].0 <= pair[1].0);
    let tape = records
        .iter()
        .map(|(rank, name)| format!("{rank}:{name}"))
        .collect::<Vec<_>>()
        .join(",");
    (tape, monotonic)
}

pub fn emit_pass_tape() {
    if !manifest_enabled() {
        return;
    }
    let records = match PASS_RECORDS.lock() {
        Ok(mut records) => std::mem::take(&mut *records),
        Err(error) => {
            tracing::warn!(target: "bastion_r0d", "pass tape lock failed: {error}");
            return;
        },
    };
    if records.is_empty() {
        return;
    }
    let (tape, monotonic) = pass_tape_snapshot(&records);
    if let Ok(mut latest) = LATEST_PASS_TAPE.lock() {
        *latest = Some(format!("{tape} monotonic={monotonic}"));
    }
    let draw_tape = take_draw_tape();
    tracing::info!(
        target: "bastion_r0d",
        "R0D-PASS-TAPE[{}]: {tape} monotonic={monotonic}",
        records.len()
    );
    if let Some(path) = manifest_output_path()
        && path.as_os_str() != "1"
        && !path.as_os_str().is_empty()
    {
        append_evidence_line(&path, &format!("pass-tape {tape} monotonic={monotonic}\n"));
        if let Some((count, digest)) = draw_tape {
            append_evidence_line(
                &path,
                &format!(
                    "semantic-trace count={count} digest={}\n",
                    hex_digest(&digest)
                ),
            );
        }
    }
}

pub mod draw_kind {
    pub const SKYBOX: u16 = 1;
    pub const DEBUG: u16 = 2;
    pub const LOD_TERRAIN: u16 = 3;
    pub const FIGURE: u16 = 4;
    pub const TERRAIN: u16 = 5;
    pub const FLUID: u16 = 6;
    pub const SPRITE: u16 = 7;
    pub const LOD_OBJECT: u16 = 8;
    pub const PARTICLE: u16 = 9;
    pub const ROPE: u16 = 10;
    pub const TRAIL: u16 = 11;
    pub const CLOUDS: u16 = 12;
    pub const POSTPROCESS: u16 = 13;
    pub const UI: u16 = 14;
    pub const FIGURE_SHADOW: u16 = 15;
    pub const TERRAIN_SHADOW: u16 = 16;
    pub const DEBUG_SHADOW: u16 = 17;
    pub const POINT_SHADOW: u16 = 18;
    pub const BLOOM: u16 = 21;
    pub const UI_PREMULTIPLY: u16 = 22;
    pub const BLIT: u16 = 23;
}

static DRAW_RECORDS: Mutex<Vec<(u16, u32, u32)>> = Mutex::new(Vec::new());
static LATEST_SEMANTIC_TRACE: Mutex<Option<(usize, [u8; 32])>> = Mutex::new(None);

pub fn record_draw(kind: u16, units: u32, instances: u32) {
    if !manifest_enabled() {
        return;
    }
    match DRAW_RECORDS.lock() {
        Ok(mut records) => records.push((kind, units, instances)),
        Err(error) => tracing::warn!(target: "bastion_r0d", "draw tape lock failed: {error}"),
    }
}

fn take_draw_tape() -> Option<(usize, [u8; 32])> {
    let records = match DRAW_RECORDS.lock() {
        Ok(mut records) => std::mem::take(&mut *records),
        Err(error) => {
            tracing::warn!(target: "bastion_r0d", "draw tape lock failed: {error}");
            return None;
        },
    };
    if records.is_empty() {
        return None;
    }
    draw_tape_snapshot(&records)
}

fn draw_tape_snapshot(records: &[(u16, u32, u32)]) -> Option<(usize, [u8; 32])> {
    if records.is_empty() {
        return None;
    }
    let mut payload = Vec::with_capacity(8 + records.len() * 10);
    payload.extend_from_slice(&u64::try_from(records.len()).ok()?.to_le_bytes());
    for (kind, units, instances) in records {
        payload.extend_from_slice(&kind.to_le_bytes());
        payload.extend_from_slice(&units.to_le_bytes());
        payload.extend_from_slice(&instances.to_le_bytes());
    }
    let digest =
        bastion_renderer_r0d::domain_hash_v1("bastion/r0d/semantic-trace", 1, 0, &payload).ok()?;
    let result = (records.len(), digest);
    if let Ok(mut latest) = LATEST_SEMANTIC_TRACE.lock() {
        *latest = Some(result);
    }
    Some(result)
}

/// Capture-only switch. Normal play retains its ordinary culling policy.
pub fn deterministic_capture_enabled() -> bool {
    std::env::var_os("BASTION_R0D_FREEZE_TIME").is_some()
}

pub fn freeze_time() -> bool { deterministic_capture_enabled() }

pub fn absolute_time_capture_selected() -> bool {
    std::env::var_os("BASTION_R0D_CAPTURE_AT").is_some()
}

pub const fn capture_waits_for_pause_v1(flat_arena: bool, absolute_time: bool) -> bool {
    flat_arena && !absolute_time
}

pub const fn certification_freeze_tick_v1(flat_arena: bool, absolute_time: bool) -> Option<u64> {
    if capture_waits_for_pause_v1(flat_arena, absolute_time) {
        Some(300)
    } else {
        None
    }
}

pub const FROZEN_SHADER_TIME: (f64, f64, f64) = (60.0 * 60.0 * 9.0, 0.0, 0.0);

pub fn capture_config() -> Option<(std::path::PathBuf, u64, u64)> {
    let output = std::env::var_os("BASTION_R0D_CAPTURE_OUT")?;
    let warmup = std::env::var("BASTION_R0D_CAPTURE_WARMUP")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(240);
    let count = std::env::var("BASTION_R0D_CAPTURE_COUNT")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|count| *count > 0)
        .unwrap_or(10);
    Some((std::path::PathBuf::from(output), warmup, count))
}

static CAPTURE_STATE: Mutex<(u64, u64, u64)> = Mutex::new((0, 0, 0));

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureAnchorEvidenceV1 {
    pub uid: u64,
    pub body_category: String,
    pub body: String,
}

static CAPTURE_ANCHOR: Mutex<Option<CaptureAnchorEvidenceV1>> = Mutex::new(None);

pub fn set_capture_anchor(anchor: CaptureAnchorEvidenceV1) {
    match CAPTURE_ANCHOR.lock() {
        Ok(mut current) => *current = Some(anchor),
        Err(error) => tracing::warn!(target: "bastion_r0d", "capture anchor lock failed: {error}"),
    }
}

fn capture_fault_path(output: &Path) -> PathBuf { output.join("capture-faults.log") }

fn write_capture_fault(output: &Path, message: &str) {
    if let Err(error) = fs::create_dir_all(output) {
        tracing::warn!(target: "bastion_r0d", "capture evidence directory failed: {error}");
        return;
    }
    append_evidence_line(&capture_fault_path(output), message);
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid capture file name",
            )
        })?;
    let staged = path.with_file_name(format!(".{file_name}.staging"));
    let mut file = fs::File::create(&staged)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(staged, path)
}

pub fn drive_capture(
    renderer: &mut super::renderer::Renderer,
    sim_time: f64,
    simulation_tick: u64,
) -> bool {
    let Some((output, warmup, count)) = capture_config() else {
        return false;
    };
    let (frames, requested, completed) = match CAPTURE_STATE.lock() {
        Ok(mut state) => {
            let Some(next_frame) = state.0.checked_add(1) else {
                write_capture_fault(
                    &output,
                    "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_FRAME_OVERFLOW\n",
                );
                return true;
            };
            state.0 = next_frame;
            *state
        },
        Err(error) => {
            write_capture_fault(
                &output,
                &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_STATE {error}\n"),
            );
            return true;
        },
    };

    let should_request = if let Ok(at) = std::env::var("BASTION_R0D_CAPTURE_AT") {
        let at = at.parse::<f64>().unwrap_or(30.0);
        let every = std::env::var("BASTION_R0D_CAPTURE_EVERY")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.5);
        sim_time >= at + requested as f64 * every
    } else {
        frames > warmup
    };
    if requested < count && should_request {
        request_one_capture(
            renderer,
            &output,
            requested,
            simulation_tick,
            sim_time.to_bits(),
        );
    }
    completed >= count
}

fn request_one_capture(
    renderer: &mut super::renderer::Renderer,
    output: &Path,
    ordinal: u64,
    simulation_tick: u64,
    simulation_time_bits: u64,
) {
    match CAPTURE_STATE.lock() {
        Ok(mut state) => {
            let Some(requested) = state.1.checked_add(1) else {
                write_capture_fault(
                    output,
                    "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_REQUEST_OVERFLOW\n",
                );
                return;
            };
            state.1 = requested;
        },
        Err(error) => {
            write_capture_fault(
                output,
                &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_STATE {error}\n"),
            );
            return;
        },
    }
    let output = output.to_path_buf();
    let anchor = CAPTURE_ANCHOR.lock().ok().and_then(|anchor| anchor.clone());
    let pass_tape = LATEST_PASS_TAPE.lock().ok().and_then(|value| value.clone());
    let semantic_trace = LATEST_SEMANTIC_TRACE.lock().ok().and_then(|value| *value);
    renderer.create_screenshot(move |result| {
        match result {
            Ok(image) => {
                let (width, height) = image.dimensions();
                match bastion_renderer_r0d::domain_hash_v1(
                    "bastion/r0d/live-capture",
                    1,
                    0,
                    image.as_raw(),
                ) {
                    Ok(digest) => {
                        let stem = format!("capture-{ordinal:04}");
                        let raw_path = output.join(format!("{stem}.rgb"));
                        let png_path = output.join(format!("{stem}.png"));
                        let metadata_path = output.join(format!("{stem}.meta"));
                        let staged_png = output.join(format!(".{stem}.png.staging"));
                        let result = fs::create_dir_all(&output)
                            .and_then(|()| write_atomic(&raw_path, image.as_raw()))
                            .and_then(|()| {
                                image
                                    .save_with_format(&staged_png, image::ImageFormat::Png)
                                    .map_err(std::io::Error::other)
                            })
                            .and_then(|()| fs::rename(&staged_png, &png_path))
                            .and_then(|()| {
                                let anchor = anchor.as_ref().ok_or_else(|| {
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "capture anchor absent",
                                    )
                                })?;
                                let pass_tape = pass_tape.as_deref().ok_or_else(|| {
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "pass tape absent",
                                    )
                                })?;
                                let (semantic_trace_count, semantic_trace_digest) = semantic_trace
                                    .ok_or_else(|| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "semantic trace absent",
                                        )
                                    })?;
                                let metadata = format!(
                                    concat!(
                                        "schema=RendererCaptureEvidenceV1\n",
                                        "ordinal={}\n",
                                        "simulation_tick={}\n",
                                        "simulation_time_bits={:016x}\n",
                                        "width={}\n",
                                        "height={}\n",
                                        "pixel_format=rgb8_srgb\n",
                                        "pixel_sha256={}\n",
                                        "anchor_uid={}\n",
                                        "anchor_category={}\n",
                                        "anchor_body={}\n",
                                        "pass_tape={}\n",
                                        "semantic_trace_count={}\n",
                                        "semantic_trace_sha256={}\n",
                                    ),
                                    ordinal,
                                    simulation_tick,
                                    simulation_time_bits,
                                    width,
                                    height,
                                    hex_digest(&digest),
                                    anchor.uid,
                                    anchor.body_category,
                                    anchor.body,
                                    pass_tape,
                                    semantic_trace_count,
                                    hex_digest(&semantic_trace_digest),
                                );
                                write_atomic(&metadata_path, metadata.as_bytes())
                            });
                        if let Err(error) = result {
                            write_capture_fault(
                                &output,
                                &format!(
                                    "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_PERSIST \
                                     ordinal={ordinal} {error}\n"
                                ),
                            );
                        }
                    },
                    Err(error) => write_capture_fault(
                        &output,
                        &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_HASH {error:?}\n"),
                    ),
                }
            },
            Err(error) => write_capture_fault(
                &output,
                &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE {error}\n"),
            ),
        }
        match CAPTURE_STATE.lock() {
            Ok(mut state) => {
                if let Some(completed) = state.2.checked_add(1) {
                    state.2 = completed;
                } else {
                    write_capture_fault(
                        &output,
                        "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_COMPLETION_OVERFLOW\n",
                    );
                }
            },
            Err(error) => write_capture_fault(
                &output,
                &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_STATE {error}\n"),
            ),
        }
    });
}

/// Select the lowest semantic entity identity while explicitly excluding the
/// client's spectator entity. Candidate order cannot affect the result.
pub fn capture_anchor_uid_v1(
    client_uid: u64,
    candidates: impl IntoIterator<Item = u64>,
) -> Option<u64> {
    candidates
        .into_iter()
        .filter(|uid| *uid != client_uid)
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::RenderMode;

    #[test]
    fn absolute_time_capture_bypasses_pause_and_server_freeze() {
        assert!(!capture_waits_for_pause_v1(true, true));
        assert_eq!(certification_freeze_tick_v1(true, true), None);
    }

    #[test]
    fn pause_only_capture_remains_flat_arena_opt_in() {
        assert!(capture_waits_for_pause_v1(true, false));
        assert_eq!(certification_freeze_tick_v1(true, false), Some(300));
        assert!(!capture_waits_for_pause_v1(false, false));
        assert_eq!(certification_freeze_tick_v1(false, false), None);
    }

    #[test]
    fn capture_anchor_excludes_client_and_ignores_input_order() {
        assert_eq!(capture_anchor_uid_v1(1, [9, 1, 3, 7]), Some(3));
        assert_eq!(capture_anchor_uid_v1(1, [7, 3, 1, 9]), Some(3));
        assert_eq!(capture_anchor_uid_v1(1, [1]), None);
    }

    #[test]
    fn pipeline_identity_is_stable_and_requires_shader_order() {
        let modes = RenderMode::default().split().0;
        let sorted = vec![
            ("a.glsl".to_owned(), "void a() {}".to_owned()),
            ("b.glsl".to_owned(), "void b() {}".to_owned()),
        ];
        let first = pipeline_identity_digest_v1(&sorted, &modes, 1, "rgba8", "rgba16").unwrap();
        let second = pipeline_identity_digest_v1(&sorted, &modes, 1, "rgba8", "rgba16").unwrap();
        assert_eq!(first, second);
        let reversed = vec![sorted[1].clone(), sorted[0].clone()];
        assert_eq!(
            pipeline_identity_digest_v1(&reversed, &modes, 1, "rgba8", "rgba16"),
            Err(PipelineIdentityErrorV1::UnsortedShaders)
        );
    }

    #[test]
    fn backend_tags_and_pass_ranks_are_frozen() {
        assert_eq!(backend_tag(wgpu::Backend::Vulkan), 1);
        assert_eq!(backend_tag(wgpu::Backend::Gl), 4);
        assert!(ranks::THIRD < ranks::UI_PREMULTIPLY);
    }

    #[test]
    fn pass_tape_reports_actual_order() {
        let ordered = [
            (ranks::SHADOW, "shadow"),
            (ranks::FIRST, "first"),
            (ranks::THIRD, "third"),
            (ranks::UI_PREMULTIPLY, "ui_premultiply"),
        ];
        assert_eq!(
            pass_tape_snapshot(&ordered),
            (
                "20:shadow,30:first,70:third,80:ui_premultiply".to_owned(),
                true
            )
        );
        let reversed = [(ranks::UI_PREMULTIPLY, "ui"), (ranks::THIRD, "third")];
        assert!(!pass_tape_snapshot(&reversed).1);
    }

    #[test]
    fn draw_tape_binds_order_and_counts() {
        let ordered = [(draw_kind::FIGURE, 12, 2), (draw_kind::TERRAIN, 24, 1)];
        let first = draw_tape_snapshot(&ordered).unwrap();
        assert_eq!(draw_tape_snapshot(&ordered).unwrap(), first);
        let reversed = [ordered[1], ordered[0]];
        assert_ne!(draw_tape_snapshot(&reversed).unwrap().1, first.1);
    }
}
