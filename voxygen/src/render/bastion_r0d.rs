//! Flag-gated renderer pipeline-identity seam.

use super::PipelineModes;
use std::sync::Mutex;

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

fn append_evidence_line(path: &std::path::Path, line: &str) {
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

pub use bastion_renderer_r0d::pass_graph::voxygen_ranks as ranks;

pub fn record_pass(rank: u16, name: &'static str) {
    if !manifest_enabled() {
        return;
    }
    match PASS_RECORDS.lock() {
        Ok(mut records) => records.push((rank, name)),
        Err(error) => tracing::warn!(target: "bastion_r0d", "pass tape lock failed: {error}"),
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
    }
}

/// Capture-only switch. Normal play retains its ordinary culling policy.
pub fn deterministic_capture_enabled() -> bool {
    std::env::var_os("BASTION_R0D_FREEZE_TIME").is_some()
}

pub fn freeze_time() -> bool { deterministic_capture_enabled() }

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

pub fn drive_capture(renderer: &mut super::renderer::Renderer, sim_time: f64) -> bool {
    let Some((output, warmup, count)) = capture_config() else {
        return false;
    };
    let (frames, requested, completed) = match CAPTURE_STATE.lock() {
        Ok(mut state) => {
            let Some(next_frame) = state.0.checked_add(1) else {
                append_evidence_line(
                    &output,
                    "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_FRAME_OVERFLOW\n",
                );
                return true;
            };
            state.0 = next_frame;
            *state
        },
        Err(error) => {
            append_evidence_line(
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
        request_one_capture(renderer, &output, requested);
    }
    completed >= count
}

fn request_one_capture(
    renderer: &mut super::renderer::Renderer,
    output: &std::path::Path,
    ordinal: u64,
) {
    match CAPTURE_STATE.lock() {
        Ok(mut state) => {
            let Some(requested) = state.1.checked_add(1) else {
                append_evidence_line(
                    output,
                    "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_REQUEST_OVERFLOW\n",
                );
                return;
            };
            state.1 = requested;
        },
        Err(error) => {
            append_evidence_line(
                output,
                &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_STATE {error}\n"),
            );
            return;
        },
    }
    let output = output.to_path_buf();
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
                    Ok(digest) => append_evidence_line(
                        &output,
                        &format!(
                            "capture {ordinal} {width}x{height} {}\n",
                            hex_digest(&digest)
                        ),
                    ),
                    Err(error) => append_evidence_line(
                        &output,
                        &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_HASH {error:?}\n"),
                    ),
                }
            },
            Err(error) => append_evidence_line(
                &output,
                &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE {error}\n"),
            ),
        }
        match CAPTURE_STATE.lock() {
            Ok(mut state) => {
                if let Some(completed) = state.2.checked_add(1) {
                    state.2 = completed;
                } else {
                    append_evidence_line(
                        &output,
                        "FAULT-TERMINAL R0D_INVALID_EVIDENCE_CAPTURE_COMPLETION_OVERFLOW\n",
                    );
                }
            },
            Err(error) => append_evidence_line(
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
}
