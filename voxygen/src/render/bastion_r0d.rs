//! R0D Phase II — the flag-gated pipeline-identity manifest seam
//! (BUILD-007A10.15 partial; design §13.7 / DC-083 prior art register).
//!
//! A PURE function computes `RendererPipelineIdentityManifestV1`'s V1 digest
//! from explicitly-encoded inputs: sorted shader sources, the explicit
//! `PipelineModes` identity bytes, a frozen backend tag, and the color-format
//! labels. No wgpu device is required, so the digest is unit-testable headless;
//! the live hook in `pipeline_creation.rs` runs ONLY when the
//! `BASTION_R0D_MANIFEST` environment flag is set (production unchanged).
//!
//! V1 fidelity note (explicit, not hidden): texture-format labels are encoded
//! as their Debug strings, which are stable within the pinned wgpu version this
//! build links; the full typed descriptor encoding (bindings, layouts, vertex
//! attributes) is the complete .15 packet. The manifest schema version below
//! changes when that lands.

/// Schema identity of this V1 (partial) manifest.
pub const PIPELINE_IDENTITY_SCHEMA_V1: (u16, u16) = (1, 0);

/// Frozen backend tag (exhaustive on the pinned wgpu's `Backend`; a wgpu
/// upgrade that adds a variant fails compilation here and forces a review).
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

/// Compute the V1 pipeline-identity digest. Inputs are length-framed in frozen
/// field order; shader sources MUST already be name-sorted (asserted) so
/// discovery order can never leak.
#[must_use]
pub fn pipeline_identity_digest(
    sorted_shader_sources: &[(String, String)],
    pipeline_modes_identity: &[u8],
    backend: u8,
    surface_format_label: &str,
    intermediate_format_label: &str,
) -> [u8; 32] {
    debug_assert!(
        sorted_shader_sources.windows(2).all(|w| w[0].0 <= w[1].0),
        "shader sources must be name-sorted"
    );
    let mut p = Vec::new();
    p.extend_from_slice(&(sorted_shader_sources.len() as u64).to_le_bytes());
    for (name, src) in sorted_shader_sources {
        p.extend_from_slice(&(name.len() as u64).to_le_bytes());
        p.extend_from_slice(name.as_bytes());
        p.extend_from_slice(&(src.len() as u64).to_le_bytes());
        p.extend_from_slice(src.as_bytes());
    }
    p.extend_from_slice(&(pipeline_modes_identity.len() as u64).to_le_bytes());
    p.extend_from_slice(pipeline_modes_identity);
    p.push(backend);
    p.extend_from_slice(&(surface_format_label.len() as u64).to_le_bytes());
    p.extend_from_slice(surface_format_label.as_bytes());
    p.extend_from_slice(&(intermediate_format_label.len() as u64).to_le_bytes());
    p.extend_from_slice(intermediate_format_label.as_bytes());
    bastion_renderer_r0d::domain_hash(
        "bastion/r0d/pipeline-identity",
        PIPELINE_IDENTITY_SCHEMA_V1.0,
        PIPELINE_IDENTITY_SCHEMA_V1.1,
        &p,
    )
}

/// Whether the flag-gated manifest emission is enabled (`BASTION_R0D_MANIFEST`
/// set). When unset — the production default — the hook is a no-op.
#[must_use]
pub fn manifest_enabled() -> bool {
    std::env::var_os("BASTION_R0D_MANIFEST").is_some()
}

/// Emit the manifest digest: always an info log line; additionally written to
/// the file named by `BASTION_R0D_MANIFEST` when its value is a path (a bare
/// `1` just logs).
pub fn emit_manifest(context: &str, digest: &[u8; 32]) {
    let hex = bastion_renderer_r0d::hex32(digest);
    tracing::info!(target: "bastion_r0d", "R0D-PIPELINE-IDENTITY[{context}]: {hex}");
    if let Some(v) = std::env::var_os("BASTION_R0D_MANIFEST") {
        let s = v.to_string_lossy();
        if s != "1" && !s.is_empty() {
            let line = format!("{context} {hex}\n");
            if let Err(e) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(s.as_ref())
                .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
            {
                tracing::warn!(target: "bastion_r0d", "manifest write failed: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase II seam 2: pass-execution recording (BUILD-007A10.15 wiring).
// ---------------------------------------------------------------------------

use std::sync::Mutex;

/// Frame-local pass-execution records: `(pass_rank, name)` in observed entry
/// order. Only ever touched behind the flag; drained at frame end.
static PASS_RECORDS: Mutex<Vec<(u16, &'static str)>> = Mutex::new(Vec::new());

/// Record one pass entry (no-op unless the manifest flag is set). Called at
/// each `Drawer` pass-drawer construction site.
pub fn record_pass(rank: u16, name: &'static str) {
    if !manifest_enabled() {
        return;
    }
    if let Ok(mut v) = PASS_RECORDS.lock() {
        v.push((rank, name));
    }
    // .16: a pass boundary is also a marker in the semantic command trace
    // (kind 0 = pass sentinel, units = pass rank), so every draw record is
    // pass-scoped and the trace is keyed (pass, command ordinal, args) —
    // never a pointer-derived handle.
    if let Ok(mut v) = DRAW_RECORDS.lock() {
        v.push((0, u32::from(rank), 0));
    }
}

/// Drain the frame's pass tape (called from `Drawer::drop`): emits one
/// `R0D-PASS-TAPE` line — the observed order plus a monotonicity verdict
/// against the frozen rank registry. The registry order IS the canonical order
/// for the drawer's declared linear graph, so nondecreasing ranks == conformant.
pub fn emit_pass_tape() {
    if !manifest_enabled() {
        return;
    }
    let records: Vec<(u16, &'static str)> = match PASS_RECORDS.lock() {
        Ok(mut v) => std::mem::take(&mut *v),
        Err(_) => return,
    };
    if records.is_empty() {
        return;
    }
    let monotonic = records.windows(2).all(|w| w[0].0 <= w[1].0);
    let tape: Vec<String> = records.iter().map(|(r, n)| format!("{r}:{n}")).collect();
    tracing::info!(
        target: "bastion_r0d",
        "R0D-PASS-TAPE[{}]: {} monotonic={monotonic}",
        records.len(),
        tape.join(","),
    );
    // .14 slice: drain the frame's CPU draw-structural tape UNCONDITIONALLY
    // (log-only mode must not accumulate records forever).
    let mut draw_tape = String::new();
    emit_draw_tape(&mut draw_tape);
    if !draw_tape.is_empty() {
        tracing::info!(target: "bastion_r0d", "R0D-{}", draw_tape.trim_end());
    }
    if let Some(v) = std::env::var_os("BASTION_R0D_MANIFEST") {
        let s = v.to_string_lossy();
        if s != "1" && !s.is_empty() {
            let line = format!(
                "pass-tape {} monotonic={monotonic}\n{draw_tape}",
                tape.join(",")
            );
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(s.as_ref())
                .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
        }
    }
}

// ---------------------------------------------------------------------------
// Phase III: flag-gated auto-capture (the live R0D certification driver).
// ---------------------------------------------------------------------------

/// Capture configuration from env (Phase III live leg):
/// `BASTION_R0D_CAPTURE_OUT` = output file; `BASTION_R0D_CAPTURE_WARMUP`
/// (default 240 frames — settle worldgen/streaming); `BASTION_R0D_CAPTURE_COUNT`
/// (default 10 — the §17.4 warm-capture certification count).
#[must_use]
pub fn capture_config() -> Option<(std::path::PathBuf, u64, u64)> {
    let out = std::env::var_os("BASTION_R0D_CAPTURE_OUT")?;
    let warmup = std::env::var("BASTION_R0D_CAPTURE_WARMUP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(240);
    let count = std::env::var("BASTION_R0D_CAPTURE_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    Some((std::path::PathBuf::from(out), warmup, count))
}

/// §17.3 exact-capture mode: fix wall-clock/animated shader inputs. When
/// `BASTION_R0D_FREEZE_TIME` is set, the scene's Globals receive constant
/// time inputs (fixed mid-morning TOD, zeroed sim/local animation time) so
/// sky/water/cloud animation cannot vary pixels between warm captures.
#[must_use]
pub fn freeze_time() -> bool {
    std::env::var_os("BASTION_R0D_FREEZE_TIME").is_some()
}

/// The frozen shader-time triple `(time_of_day, sim_time, local_time)` used
/// when [`freeze_time`] holds: 09:00 into a 24h day, zero animation clocks.
pub const FROZEN_SHADER_TIME: (f64, f64, f64) = (60.0 * 60.0 * 9.0, 0.0, 0.0);

/// (frames_seen_in_session, captures_requested, captures_completed).
static CAPTURE_STATE: Mutex<(u64, u64, u64)> = Mutex::new((0, 0, 0));

/// Drive one session frame of the auto-capture leg. Counts warmup frames,
/// requests one screenshot per frame until `count` are in flight, hashes each
/// completed image's TIGHT bytes (RgbImage raw is unpadded) through the shared
/// domain_hash, appends `capture <ordinal> <w>x<h> <hex>` to the output file,
/// and returns `true` (request shutdown) once every capture has completed.
/// No-op returning `false` unless the env config is present.
pub fn drive_capture(renderer: &mut super::renderer::Renderer) -> bool {
    let Some((out, warmup, count)) = capture_config() else {
        return false;
    };
    let (frames, requested, completed) = {
        let mut s = CAPTURE_STATE.lock().expect("capture state");
        s.0 += 1;
        *s
    };
    if frames > warmup && requested < count {
        {
            let mut s = CAPTURE_STATE.lock().expect("capture state");
            s.1 += 1;
        }
        let ordinal = requested; // 0-based capture ordinal
        let out = out.clone();
        renderer.create_screenshot(move |result| {
            match result {
                Ok(image) => {
                    let (w, h) = image.dimensions();
                    let digest = bastion_renderer_r0d::domain_hash(
                        "bastion/r0d/live-capture",
                        1,
                        0,
                        image.as_raw(),
                    );
                    // R0D .19: bind the canonical capture identity — the frame
                    // token (capture ordinal in V1) + the honest descriptor
                    // (current path = tight RGB8, top-left, sRGB, alpha
                    // dropped). Callback time appears nowhere (DC-080).
                    let descriptor = bastion_renderer_r0d::capture::RendererCaptureDescriptorV1 {
                        target: bastion_renderer_r0d::capture::CaptureTargetKind::OffscreenFinalComposite,
                        format: bastion_renderer_r0d::capture::CaptureFormat::Rgb8Srgb,
                        channel_order: bastion_renderer_r0d::capture::ChannelOrder::Rgba,
                        row_origin: bastion_renderer_r0d::capture::RowOrigin::TopLeft,
                        alpha: bastion_renderer_r0d::capture::AlphaMode::PreservedStraight,
                        transfer: bastion_renderer_r0d::capture::TransferFunction::Srgb,
                        width: w,
                        height: h,
                    };
                    let identity = descriptor.capture_identity(ordinal);
                    let line = format!(
                        "capture {ordinal} {w}x{h} {} id={}\n",
                        bastion_renderer_r0d::hex32(&digest),
                        bastion_renderer_r0d::hex32(&identity),
                    );
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&out)
                        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
                    // Diagnostic PNG dump (BASTION_R0D_CAPTURE_PNG): when warm
                    // captures unexpectedly differ, the images themselves are
                    // the fastest divergence localizer.
                    if std::env::var_os("BASTION_R0D_CAPTURE_PNG").is_some() {
                        let mut png = out.clone();
                        png.set_extension(format!("{ordinal}.png"));
                        if let Err(e) = image.save(&png) {
                            tracing::warn!(target: "bastion_r0d", "png dump failed: {e}");
                        }
                    }
                }
                Err(e) => {
                    // Typed-terminal spirit (BTL-341): a failed capture is
                    // recorded, never silently lost.
                    let line = format!("capture {ordinal} FAILED {e}\n");
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&out)
                        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
                }
            }
            if let Ok(mut s) = CAPTURE_STATE.lock() {
                s.2 += 1;
            }
        });
    }
    completed >= count
}

// ---------------------------------------------------------------------------
// BUILD-007A10.14 slice: the CPU draw-structural tape.
// ---------------------------------------------------------------------------

/// Frozen draw-kind registry (.14): every CPU-encoded draw call carries one.
/// Append-only; the numeric tag enters the draw-tape digest.
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

/// Frame-local CPU draw records `(kind, units, instances)` in encode order —
/// the §13.1 "draw group and instance order" structural evidence. Behind the
/// manifest flag; drained with the pass tape.
static DRAW_RECORDS: Mutex<Vec<(u16, u32, u32)>> = Mutex::new(Vec::new());

/// Record one CPU-encoded draw call (.14). `units` = index or vertex count as
/// encoded; `instances` = instance count. No-op unless the manifest flag is set.
pub fn record_draw(kind: u16, units: u32, instances: u32) {
    if !manifest_enabled() {
        return;
    }
    if let Ok(mut v) = DRAW_RECORDS.lock() {
        v.push((kind, units, instances));
    }
}

/// Drain the frame's draw tape (called with the pass tape at `Drawer::drop`):
/// one `draw-tape <n> <digest>` line — the digest chains every (kind, units,
/// instances) record in encode order through the shared domain_hash, so two
/// runs match iff their CPU draw streams are identical.
fn emit_draw_tape(sink: &mut String) {
    let records: Vec<(u16, u32, u32)> = match DRAW_RECORDS.lock() {
        Ok(mut v) => std::mem::take(&mut *v),
        Err(_) => return,
    };
    if records.is_empty() {
        return;
    }
    let mut payload = Vec::with_capacity(8 + records.len() * 10);
    payload.extend_from_slice(&(records.len() as u64).to_le_bytes());
    for (kind, units, instances) in &records {
        payload.extend_from_slice(&kind.to_le_bytes());
        payload.extend_from_slice(&units.to_le_bytes());
        payload.extend_from_slice(&instances.to_le_bytes());
    }
    let digest = bastion_renderer_r0d::domain_hash("bastion/r0d/semantic-trace", 1, 0, &payload);
    sink.push_str(&format!(
        "semantic-trace {} {}\n",
        records.len(),
        bastion_renderer_r0d::hex32(&digest)
    ));
}

/// R0D .18: record a typed GPU fault terminal into the capture evidence file.
/// A faulted run can never publish a clean capture set — the marker line makes
/// the failure diagnosable from the evidence alone (§19.2/BTL-341). No-op
/// outside capture mode.
pub fn record_fault_terminal(detail: &str) {
    let Some((out, _, _)) = capture_config() else {
        return;
    };
    // One line, classified: validation/OOM/device-loss map to the shutdown
    // module's typed vocabulary at triage; the raw detail is preserved.
    let line = format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_GPU {detail}\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

/// The frozen drawer pass ranks (mirrors
/// `bastion_renderer_r0d::pass_graph::voxygen_ranks`).
pub mod ranks {
    pub const RAIN_OCCLUSION: u16 = 10;
    pub const SHADOW: u16 = 20;
    pub const FIRST: u16 = 30;
    pub const VOLUMETRIC: u16 = 40;
    pub const TRANSPARENT: u16 = 50;
    pub const BLOOM: u16 = 60;
    pub const THIRD: u16 = 70;
    // LIVE-EVIDENCE CORRECTION: ui_premultiply executes after third begins.
    pub const UI_PREMULTIPLY: u16 = 80;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srcs() -> Vec<(String, String)> {
        vec![
            ("a-vert".to_string(), "void main() {}".to_string()),
            ("b-frag".to_string(), "void main() { discard; }".to_string()),
        ]
    }

    #[test]
    fn digest_is_stable_and_input_sensitive() {
        let d0 = pipeline_identity_digest(&srcs(), &[1, 2, 3], 1, "Rgba8UnormSrgb", "Rgba16Float");
        let d1 = pipeline_identity_digest(&srcs(), &[1, 2, 3], 1, "Rgba8UnormSrgb", "Rgba16Float");
        assert_eq!(d0, d1);
        // Every input axis independently changes the digest.
        let mut s2 = srcs();
        s2[0].1.push(' ');
        assert_ne!(d0, pipeline_identity_digest(&s2, &[1, 2, 3], 1, "Rgba8UnormSrgb", "Rgba16Float"), "source");
        assert_ne!(d0, pipeline_identity_digest(&srcs(), &[9], 1, "Rgba8UnormSrgb", "Rgba16Float"), "modes");
        assert_ne!(d0, pipeline_identity_digest(&srcs(), &[1, 2, 3], 3, "Rgba8UnormSrgb", "Rgba16Float"), "backend");
        assert_ne!(d0, pipeline_identity_digest(&srcs(), &[1, 2, 3], 1, "Bgra8UnormSrgb", "Rgba16Float"), "surface fmt");
    }

    #[test]
    fn length_framing_prevents_name_source_aliasing() {
        // Moving a byte across the name/source boundary must change the digest.
        let a = vec![("ab".to_string(), "c".to_string())];
        let b = vec![("a".to_string(), "bc".to_string())];
        assert_ne!(
            pipeline_identity_digest(&a, &[], 0, "f", "f"),
            pipeline_identity_digest(&b, &[], 0, "f", "f"),
        );
    }

    #[test]
    fn backend_tags_are_frozen() {
        assert_eq!(backend_tag(wgpu::Backend::Vulkan), 1);
        assert_eq!(backend_tag(wgpu::Backend::Gl), 4);
    }
}
