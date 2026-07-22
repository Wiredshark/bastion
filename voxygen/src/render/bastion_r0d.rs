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
    if let Some(v) = std::env::var_os("BASTION_R0D_MANIFEST") {
        let s = v.to_string_lossy();
        if s != "1" && !s.is_empty() {
            let line = format!("pass-tape {} monotonic={monotonic}\n", tape.join(","));
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(s.as_ref())
                .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
        }
    }
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
    pub const UI_PREMULTIPLY: u16 = 70;
    pub const THIRD: u16 = 80;
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
