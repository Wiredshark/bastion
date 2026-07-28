//! Flag-gated renderer pipeline-identity seam.

use super::PipelineModes;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
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
    crate::r0p_observer::record_pass();
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

pub const VISIBLE_SCENE_FIGURE_MASK_V1: u8 = 1 << 0;
pub const VISIBLE_SCENE_TERRAIN_MASK_V1: u8 = 1 << 1;
pub const VISIBLE_SCENE_LOD_TERRAIN_MASK_V1: u8 = 1 << 2;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VisibleSceneCoverageV1 {
    pub mask: u8,
    pub figure_draw_count: u32,
    pub figure_units: u64,
    pub figure_instances: u64,
    pub terrain_draw_count: u32,
    pub terrain_units: u64,
    pub terrain_instances: u64,
    pub lod_terrain_draw_count: u32,
    pub lod_terrain_units: u64,
    pub lod_terrain_instances: u64,
}

impl VisibleSceneCoverageV1 {
    #[must_use]
    pub const fn has_figure(self) -> bool {
        self.mask & VISIBLE_SCENE_FIGURE_MASK_V1 != 0
            && self.figure_draw_count > 0
            && self.figure_units > 0
            && self.figure_instances > 0
    }

    #[must_use]
    pub const fn has_terrain(self) -> bool {
        let terrain_mask = VISIBLE_SCENE_TERRAIN_MASK_V1 | VISIBLE_SCENE_LOD_TERRAIN_MASK_V1;
        self.mask & terrain_mask != 0
            && (self.terrain_draw_count > 0 || self.lod_terrain_draw_count > 0)
            && (self.terrain_units > 0 || self.lod_terrain_units > 0)
            && (self.terrain_instances > 0 || self.lod_terrain_instances > 0)
    }
}

static DRAW_RECORDS: Mutex<Vec<(u16, u32, u32)>> = Mutex::new(Vec::new());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticTraceSnapshotV1 {
    pub generation: u64,
    pub record_count: usize,
    pub digest: [u8; 32],
    pub visible_scene_coverage: VisibleSceneCoverageV1,
    pub presentation_generation: Option<u64>,
}

static LATEST_SEMANTIC_TRACE: Mutex<Option<SemanticTraceSnapshotV1>> = Mutex::new(None);
static PRESENTATION_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn set_presentation_generation_v1(generation: Option<u64>) {
    PRESENTATION_GENERATION.store(generation.unwrap_or(0), Ordering::Release);
}

pub fn record_draw(kind: u16, units: u32, instances: u32) {
    crate::r0p_observer::record_draw(units, instances);
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
    // Once the R1D plan exists, the semantic trace binds the complete
    // renderer-owned representation decision. Draw observation remains
    // diagnostic; a different tier plan cannot alias the same tape.
    if let Some(tiers) = crate::r1d_tiers::latest_evidence() {
        payload.extend_from_slice(b"R1DT");
        payload.extend_from_slice(&tiers.generation.to_le_bytes());
        payload.extend_from_slice(&tiers.frame_digest);
        payload.extend_from_slice(&tiers.decision_root);
        for count in [
            tiers.full_count,
            tiers.reduced_count,
            tiers.lod_count,
            tiers.impostor_count,
            tiers.culled_count,
            tiers.full_shadow_count,
            tiers.proxy_shadow_count,
            tiers.fallback_count,
            tiers.transition_count,
            tiers.max_visible,
            tiers.max_full,
            tiers.max_reduced,
            tiers.max_lod,
            tiers.max_impostor,
            tiers.full_budget_fallbacks,
            tiers.animation_budget_fallbacks,
            tiers.lod_budget_fallbacks,
            tiers.impostor_budget_fallbacks,
            tiers.visible_budget_fallbacks,
            tiers.lod_unavailable_fallbacks,
            tiers.impostor_unavailable_fallbacks,
            tiers.shadow_proxy_unavailable_fallbacks,
        ] {
            payload.extend_from_slice(&count.to_le_bytes());
        }
    }
    if let Some(groups) = crate::r1d_groups::latest_evidence() {
        payload.extend_from_slice(b"R1DG");
        payload.extend_from_slice(&groups.generation.to_le_bytes());
        payload.extend_from_slice(&groups.frame_digest);
        payload.extend_from_slice(&groups.individual_plan_root);
        payload.extend_from_slice(&groups.group_plan_root);
        for count in [
            groups.group_count,
            groups.member_count,
            groups.individual_group_count,
            groups.formation_group_count,
            groups.aggregate_group_count,
            groups.protected_member_count,
            groups.transition_count,
            groups.fixture_group_count,
            groups.authoritative_group_count,
            groups.line_count,
            groups.column_count,
            groups.wedge_count,
            groups.grid_count,
            groups.procession_count,
        ] {
            payload.extend_from_slice(&count.to_le_bytes());
        }
    }
    let digest =
        bastion_renderer_r0d::domain_hash_v1("bastion/r0d/semantic-trace", 1, 0, &payload).ok()?;
    let visible_scene_coverage = visible_scene_coverage_v1(records)?;
    let result = (records.len(), digest);
    if let Ok(mut latest) = LATEST_SEMANTIC_TRACE.lock() {
        let generation = latest
            .map(|snapshot| snapshot.generation)
            .unwrap_or(0)
            .checked_add(1);
        if let Some(generation) = generation {
            *latest = Some(SemanticTraceSnapshotV1 {
                generation,
                record_count: records.len(),
                digest,
                visible_scene_coverage,
                presentation_generation: match PRESENTATION_GENERATION.load(Ordering::Acquire) {
                    0 => None,
                    value => Some(value),
                },
            });
        }
    }
    Some(result)
}

fn visible_scene_coverage_v1(records: &[(u16, u32, u32)]) -> Option<VisibleSceneCoverageV1> {
    let mut coverage = VisibleSceneCoverageV1::default();
    for &(kind, units, instances) in records {
        if units == 0 || instances == 0 {
            continue;
        }
        let (draw_count, unit_count, instance_count, mask) = match kind {
            draw_kind::FIGURE => (
                &mut coverage.figure_draw_count,
                &mut coverage.figure_units,
                &mut coverage.figure_instances,
                VISIBLE_SCENE_FIGURE_MASK_V1,
            ),
            draw_kind::TERRAIN => (
                &mut coverage.terrain_draw_count,
                &mut coverage.terrain_units,
                &mut coverage.terrain_instances,
                VISIBLE_SCENE_TERRAIN_MASK_V1,
            ),
            draw_kind::LOD_TERRAIN => (
                &mut coverage.lod_terrain_draw_count,
                &mut coverage.lod_terrain_units,
                &mut coverage.lod_terrain_instances,
                VISIBLE_SCENE_LOD_TERRAIN_MASK_V1,
            ),
            _ => continue,
        };
        *draw_count = draw_count.checked_add(1)?;
        *unit_count = unit_count.checked_add(u64::from(units))?;
        *instance_count = instance_count.checked_add(u64::from(instances))?;
        coverage.mask |= mask;
    }
    Some(coverage)
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
    if flat_arena && (absolute_time || capture_waits_for_pause_v1(flat_arena, absolute_time)) {
        Some(300)
    } else {
        None
    }
}

pub const CERTIFICATION_SERVER_TICK_V1: u64 = 300;
pub const READINESS_STABLE_RENDER_FRAMES_V1: u64 = 240;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CertificationServerLatchV1 {
    pub completed_tick: u64,
    pub frozen: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificationServerLatchErrorV1 {
    TickRegression,
    AdvancedAfterFreeze,
}

impl CertificationServerLatchV1 {
    pub const fn reset(&mut self) {
        self.completed_tick = 0;
        self.frozen = false;
    }

    pub fn record_completed_tick(
        &mut self,
        completed_tick: u64,
    ) -> Result<(), CertificationServerLatchErrorV1> {
        if completed_tick < self.completed_tick {
            return Err(CertificationServerLatchErrorV1::TickRegression);
        }
        if self.frozen && completed_tick != self.completed_tick {
            return Err(CertificationServerLatchErrorV1::AdvancedAfterFreeze);
        }
        self.completed_tick = completed_tick;
        self.frozen = completed_tick == CERTIFICATION_SERVER_TICK_V1;
        Ok(())
    }
}

static CERTIFICATION_SERVER_LATCH: Mutex<CertificationServerLatchV1> =
    Mutex::new(CertificationServerLatchV1 {
        completed_tick: 0,
        frozen: false,
    });

pub fn reset_certification_server_latch_v1() {
    if let Ok(mut latch) = CERTIFICATION_SERVER_LATCH.lock() {
        latch.reset();
    }
    if let Ok(mut gate) = SETTLED_TRACE_GATE.lock() {
        gate.reset();
    }
    if let Ok(mut state) = CAPTURE_STATE.lock() {
        *state = (0, 0, 0);
    }
    clear_capture_anchor();
}

pub fn record_certification_server_tick_v1(
    completed_tick: u64,
) -> Result<(), CertificationServerLatchErrorV1> {
    CERTIFICATION_SERVER_LATCH
        .lock()
        .map_err(|_| CertificationServerLatchErrorV1::AdvancedAfterFreeze)?
        .record_completed_tick(completed_tick)
}

pub fn certification_server_latch_v1() -> Option<CertificationServerLatchV1> {
    CERTIFICATION_SERVER_LATCH.lock().ok().map(|latch| *latch)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettledTraceFaultV1 {
    ServerAuthorityChangedAfterOpen,
    TraceGenerationRegressed,
    StableFrameOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettledTraceObservationV1 {
    Waiting {
        stable_frames: u64,
    },
    Open {
        digest: [u8; 32],
        coverage: VisibleSceneCoverageV1,
        stable_frames: u64,
        advanced: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettledTraceGateV1 {
    freeze_generation: Option<u64>,
    last_generation: Option<u64>,
    last_digest: Option<[u8; 32]>,
    last_coverage: Option<VisibleSceneCoverageV1>,
    last_anchor_uid: Option<u64>,
    stable_frames: u64,
    open_digest: Option<[u8; 32]>,
    open_coverage: Option<VisibleSceneCoverageV1>,
    open_anchor_uid: Option<u64>,
}

impl SettledTraceGateV1 {
    pub const fn reset(&mut self) {
        self.freeze_generation = None;
        self.last_generation = None;
        self.last_digest = None;
        self.last_coverage = None;
        self.last_anchor_uid = None;
        self.stable_frames = 0;
        self.open_digest = None;
        self.open_coverage = None;
        self.open_anchor_uid = None;
    }

    pub fn observe(
        &mut self,
        authority: CertificationServerLatchV1,
        trace: SemanticTraceSnapshotV1,
        anchor: Option<&CaptureAnchorEvidenceV1>,
    ) -> Result<SettledTraceObservationV1, SettledTraceFaultV1> {
        if authority.completed_tick != CERTIFICATION_SERVER_TICK_V1 || !authority.frozen {
            if self.open_digest.is_some() {
                return Err(SettledTraceFaultV1::ServerAuthorityChangedAfterOpen);
            }
            self.reset();
            return Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 });
        }

        let visible_scene_ready = anchor.is_some_and(CaptureAnchorEvidenceV1::is_valid)
            && trace.visible_scene_coverage.has_figure()
            && trace.visible_scene_coverage.has_terrain();
        if !visible_scene_ready {
            self.freeze_generation = Some(trace.generation);
            self.last_generation = Some(trace.generation);
            self.last_digest = None;
            self.last_coverage = None;
            self.last_anchor_uid = None;
            self.stable_frames = 0;
            self.open_digest = None;
            self.open_coverage = None;
            self.open_anchor_uid = None;
            return Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 });
        }
        let anchor_uid = anchor.map(|anchor| anchor.uid).unwrap_or(0);

        if self.freeze_generation.is_none() {
            self.freeze_generation = Some(trace.generation);
            self.last_generation = Some(trace.generation);
            return Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 });
        }

        let Some(last_generation) = self.last_generation else {
            self.last_generation = Some(trace.generation);
            return Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 });
        };
        if trace.generation < last_generation {
            return Err(SettledTraceFaultV1::TraceGenerationRegressed);
        }
        if trace.generation == last_generation {
            if let (Some(digest), Some(coverage), Some(open_anchor_uid)) =
                (self.open_digest, self.open_coverage, self.open_anchor_uid)
            {
                if trace.visible_scene_coverage != coverage || anchor_uid != open_anchor_uid {
                    self.freeze_generation = Some(trace.generation);
                    self.last_generation = Some(trace.generation);
                    self.last_digest = None;
                    self.last_coverage = None;
                    self.last_anchor_uid = None;
                    self.stable_frames = 0;
                    self.open_digest = None;
                    self.open_coverage = None;
                    self.open_anchor_uid = None;
                    return Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 });
                }
                return Ok(SettledTraceObservationV1::Open {
                    digest,
                    coverage,
                    stable_frames: self.stable_frames,
                    advanced: false,
                });
            }
            return Ok(SettledTraceObservationV1::Waiting {
                stable_frames: self.stable_frames,
            });
        }

        self.last_generation = Some(trace.generation);
        if let (Some(open_digest), Some(open_coverage), Some(open_anchor_uid)) =
            (self.open_digest, self.open_coverage, self.open_anchor_uid)
        {
            if trace.visible_scene_coverage != open_coverage || anchor_uid != open_anchor_uid {
                self.freeze_generation = Some(trace.generation);
                self.last_generation = Some(trace.generation);
                self.last_digest = None;
                self.last_coverage = None;
                self.last_anchor_uid = None;
                self.stable_frames = 0;
                self.open_digest = None;
                self.open_coverage = None;
                self.open_anchor_uid = None;
                return Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 });
            }
            return Ok(SettledTraceObservationV1::Open {
                digest: open_digest,
                coverage: open_coverage,
                stable_frames: self.stable_frames,
                advanced: true,
            });
        }

        self.stable_frames = if self.last_digest == Some(trace.digest)
            && self.last_coverage == Some(trace.visible_scene_coverage)
            && self.last_anchor_uid == Some(anchor_uid)
        {
            self.stable_frames
                .checked_add(1)
                .ok_or(SettledTraceFaultV1::StableFrameOverflow)?
        } else {
            1
        };
        self.last_digest = Some(trace.digest);
        self.last_coverage = Some(trace.visible_scene_coverage);
        self.last_anchor_uid = Some(anchor_uid);
        if self.stable_frames >= READINESS_STABLE_RENDER_FRAMES_V1 {
            self.open_digest = Some(trace.digest);
            self.open_coverage = Some(trace.visible_scene_coverage);
            self.open_anchor_uid = Some(anchor_uid);
            Ok(SettledTraceObservationV1::Open {
                digest: trace.digest,
                coverage: trace.visible_scene_coverage,
                stable_frames: self.stable_frames,
                advanced: true,
            })
        } else {
            Ok(SettledTraceObservationV1::Waiting {
                stable_frames: self.stable_frames,
            })
        }
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
static SETTLED_TRACE_GATE: Mutex<SettledTraceGateV1> = Mutex::new(SettledTraceGateV1 {
    freeze_generation: None,
    last_generation: None,
    last_digest: None,
    last_coverage: None,
    last_anchor_uid: None,
    stable_frames: 0,
    open_digest: None,
    open_coverage: None,
    open_anchor_uid: None,
});

/// Number of capture commands synchronously accepted by the renderer.
///
/// This counter advances before asynchronous readback begins, so callback
/// completion order cannot select a deterministic camera-script sample.
#[must_use]
pub fn capture_requested_ordinal_v1() -> u64 { CAPTURE_STATE.lock().map_or(0, |state| state.1) }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureAnchorEvidenceV1 {
    pub uid: u64,
    pub selected_non_client_colonist: bool,
    pub body_category: String,
    pub body: String,
}

impl CaptureAnchorEvidenceV1 {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.uid != 0
            && self.selected_non_client_colonist
            && self.body_category == "bastion_colonist"
            && !self.body.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureMetadataFieldClassV1 {
    Authority,
    Diagnostic,
    Evidence,
}

pub fn capture_metadata_field_class_v1(field: &str) -> Option<CaptureMetadataFieldClassV1> {
    match field {
        "authoritative_server_tick"
        | "authoritative_frozen"
        | "readiness_trace_sha256"
        | "readiness_stable_frames"
        | "visible_scene_ready"
        | "visible_scene_coverage_mask"
        | "visible_scene_figure_draw_count"
        | "visible_scene_figure_units"
        | "visible_scene_figure_instances"
        | "visible_scene_terrain_draw_count"
        | "visible_scene_terrain_units"
        | "visible_scene_terrain_instances"
        | "visible_scene_lod_terrain_draw_count"
        | "visible_scene_lod_terrain_units"
        | "visible_scene_lod_terrain_instances"
        | "presentation_generation"
        | "presentation_frame_sha256"
        | "presentation_resource_set_sha256"
        | "presentation_semantic_tape_sha256"
        | "figure_gpu_package_sha256"
        | "figure_gpu_material_table_sha256"
        | "figure_gpu_material_shader_interface_sha256"
        | "figure_gpu_assignment_sha256"
        | "figure_gpu_staged_sha256"
        | "figure_gpu_plan_sha256"
        | "figure_gpu_submission_sequence"
        | "figure_gpu_completion_sequence"
        | "r1d_tier_generation"
        | "r1d_tier_frame_sha256"
        | "r1d_tier_decision_root_sha256"
        | "r1d_group_generation"
        | "r1d_group_frame_sha256"
        | "r1d_group_individual_plan_sha256"
        | "r1d_group_plan_sha256"
        | "r1d_scale_sample_ordinal"
        | "r1d_scale_policy_tick"
        | "r1d_scale_camera_distance_mm"
        | "r1d_scale_member_set_sha256"
        | "r1d_scale_sample_sha256"
        | "r1f_environment_presentation_generation"
        | "r1f_environment_simulation_tick"
        | "r1f_environment_frame_sha256"
        | "r1f_environment_material_table_sha256"
        | "r1f_environment_source_sha256"
        | "r1f_environment_projection_sha256"
        | "r1f_environment_availability_bits"
        | "r1f_weather_presentation_generation"
        | "r1f_weather_simulation_tick"
        | "r1f_weather_environment_projection_sha256"
        | "r1f_weather_environment_source_sha256"
        | "r1f_weather_kind"
        | "r1f_weather_rain_milli"
        | "r1f_weather_precipitation_milli"
        | "r1f_weather_wind_x_mm_s"
        | "r1f_weather_wind_y_mm_s"
        | "r1f_weather_phase_milli"
        | "r1f_weather_effect_record_count"
        | "r1f_weather_effect_instance_count"
        | "r1f_weather_presentation_sha256"
        | "anchor_uid"
        | "anchor_selected_non_client_colonist"
        | "ordinal" => Some(CaptureMetadataFieldClassV1::Authority),
        "diagnostic_client_tick"
        | "diagnostic_interpolated_time_bits"
        | "r1f_environment_client_interpolation_diagnostic"
        | "r1f_weather_legacy_rollback" => Some(CaptureMetadataFieldClassV1::Diagnostic),
        "anchor_category"
        | "anchor_body"
        | "pass_tape"
        | "semantic_trace_count"
        | "semantic_trace_sha256"
        | "figure_gpu_instance_count"
        | "figure_gpu_pose_page_count"
        | "figure_gpu_upload_windows"
        | "figure_gpu_upload_operations"
        | "figure_gpu_upload_bytes"
        | "figure_gpu_material_entry_count"
        | "figure_gpu_material_legacy_fallback_count"
        | "figure_batch_visible_figures"
        | "figure_batch_count"
        | "figure_batch_fallback_count"
        | "figure_batch_legacy_draw_equivalent"
        | "figure_batch_actual_draw_count"
        | "figure_batch_main_count"
        | "figure_batch_shadow_count"
        | "figure_batch_rain_count"
        | "figure_batch_capacity_fallbacks"
        | "figure_batch_incompatible_resource_fallbacks"
        | "r1d_tier_full_count"
        | "r1d_tier_reduced_count"
        | "r1d_tier_lod_count"
        | "r1d_tier_impostor_count"
        | "r1d_tier_culled_count"
        | "r1d_tier_full_shadow_count"
        | "r1d_tier_proxy_shadow_count"
        | "r1d_tier_fallback_count"
        | "r1d_tier_transition_count"
        | "r1d_tier_budget_max_visible"
        | "r1d_tier_budget_max_full"
        | "r1d_tier_budget_max_reduced"
        | "r1d_tier_budget_max_lod"
        | "r1d_tier_budget_max_impostor"
        | "r1d_tier_fallback_full_budget"
        | "r1d_tier_fallback_animation_budget"
        | "r1d_tier_fallback_lod_budget"
        | "r1d_tier_fallback_impostor_budget"
        | "r1d_tier_fallback_visible_budget"
        | "r1d_tier_fallback_lod_unavailable"
        | "r1d_tier_fallback_impostor_unavailable"
        | "r1d_tier_fallback_shadow_proxy_unavailable"
        | "r1d_group_count"
        | "r1d_group_member_count"
        | "r1d_group_individual_count"
        | "r1d_group_formation_count"
        | "r1d_group_aggregate_count"
        | "r1d_group_protected_member_count"
        | "r1d_group_protected_uids"
        | "r1d_group_transition_count"
        | "r1d_group_fixture_count"
        | "r1d_group_authoritative_count"
        | "r1d_group_line_count"
        | "r1d_group_column_count"
        | "r1d_group_wedge_count"
        | "r1d_group_grid_count"
        | "r1d_group_procession_count"
        | "r1d_scale_enabled"
        | "r1d_scale_population_count"
        | "width"
        | "height"
        | "pixel_format"
        | "pixel_sha256" => Some(CaptureMetadataFieldClassV1::Evidence),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CaptureRequestContextV1 {
    authoritative_server_tick: u64,
    authoritative_frozen: bool,
    readiness_trace_digest: [u8; 32],
    readiness_stable_frames: u64,
    visible_scene_coverage: VisibleSceneCoverageV1,
    diagnostic_client_tick: u64,
    diagnostic_interpolated_time_bits: u64,
    semantic_trace: SemanticTraceSnapshotV1,
    presentation: bastion_renderer_r0d::presentation::PresentationReadyTokenV1,
}

static CAPTURE_ANCHOR: Mutex<Option<CaptureAnchorEvidenceV1>> = Mutex::new(None);

pub fn set_capture_anchor(anchor: CaptureAnchorEvidenceV1) {
    match CAPTURE_ANCHOR.lock() {
        Ok(mut current) => *current = Some(anchor),
        Err(error) => tracing::warn!(target: "bastion_r0d", "capture anchor lock failed: {error}"),
    }
}

pub fn clear_capture_anchor() {
    match CAPTURE_ANCHOR.lock() {
        Ok(mut current) => *current = None,
        Err(error) => tracing::warn!(target: "bastion_r0d", "capture anchor lock failed: {error}"),
    }
}

#[must_use]
pub fn capture_anchor_v1() -> Option<CaptureAnchorEvidenceV1> {
    CAPTURE_ANCHOR.lock().ok().and_then(|anchor| anchor.clone())
}

#[must_use]
pub fn latest_visible_resource_coverage_v1() -> (Option<u64>, bool, bool) {
    LATEST_SEMANTIC_TRACE
        .lock()
        .ok()
        .and_then(|trace| *trace)
        .map(|trace| {
            (
                trace.presentation_generation,
                trace.visible_scene_coverage.terrain_draw_count > 0
                    || trace.visible_scene_coverage.lod_terrain_draw_count > 0,
                trace.visible_scene_coverage.figure_draw_count > 0,
            )
        })
        .unwrap_or((None, false, false))
}

fn capture_fault_path(output: &Path) -> PathBuf { output.join("capture-faults.log") }

fn write_capture_fault(output: &Path, message: &str) {
    if let Err(error) = fs::create_dir_all(output) {
        tracing::warn!(target: "bastion_r0d", "capture evidence directory failed: {error}");
        return;
    }
    append_evidence_line(&capture_fault_path(output), message);
}

pub(crate) fn record_certification_fixture_fault_v1(message: &str) {
    if let Some((output, _, _)) = capture_config() {
        write_capture_fault(&output, message);
    }
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
    if crate::r1d_scale::enabled() && count != crate::r1d_scale::SCALE_CAPTURE_COUNT_V1 {
        write_capture_fault(
            &output,
            "FAULT-TERMINAL R1D_SCALE_INVALID_CAPTURE_COUNT expected=5\n",
        );
        return true;
    }
    if crate::r1e_cutaway::enabled() && count != crate::r1e_cutaway::CUTAWAY_CAPTURE_COUNT_V1 {
        write_capture_fault(
            &output,
            "FAULT-TERMINAL R1E_CUTAWAY_INVALID_CAPTURE_COUNT expected=3\n",
        );
        return true;
    }
    if crate::r1e_cutaway::enabled() && !crate::r1e_cutaway::ready_for_capture() {
        return false;
    }
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

    let authority = certification_server_latch_v1();
    let Some(presentation) = crate::r1a_presentation::ready_token() else {
        return false;
    };
    let semantic_trace = LATEST_SEMANTIC_TRACE.lock().ok().and_then(|value| *value);
    let anchor = CAPTURE_ANCHOR.lock().ok().and_then(|anchor| anchor.clone());
    let absolute_mode = absolute_time_capture_selected();
    let readiness = if absolute_mode {
        let (Some(authority), Some(semantic_trace)) = (authority, semantic_trace) else {
            return false;
        };
        let observation = match SETTLED_TRACE_GATE.lock() {
            Ok(mut gate) => gate.observe(authority, semantic_trace, anchor.as_ref()),
            Err(error) => {
                write_capture_fault(
                    &output,
                    &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_READINESS_STATE {error}\n"),
                );
                return true;
            },
        };
        match observation {
            Ok(SettledTraceObservationV1::Open {
                digest,
                coverage,
                stable_frames,
                advanced: true,
            }) => Some((authority, semantic_trace, digest, coverage, stable_frames)),
            Ok(
                SettledTraceObservationV1::Waiting { .. }
                | SettledTraceObservationV1::Open {
                    advanced: false, ..
                },
            ) => None,
            Err(error) => {
                write_capture_fault(
                    &output,
                    &format!("FAULT-TERMINAL R0D_INVALID_EVIDENCE_SETTLED_TRACE {error:?}\n"),
                );
                return true;
            },
        }
    } else if frames > warmup {
        authority
            .zip(semantic_trace)
            .map(|(authority, semantic_trace)| {
                (
                    authority,
                    semantic_trace,
                    semantic_trace.digest,
                    semantic_trace.visible_scene_coverage,
                    0_u64,
                )
            })
    } else {
        None
    };
    if requested < count
        && let Some((
            authority,
            semantic_trace,
            readiness_trace_digest,
            visible_scene_coverage,
            stable_frames,
        )) = readiness
        && let Some(anchor) = anchor
    {
        request_one_capture(
            renderer,
            &output,
            requested,
            anchor,
            CaptureRequestContextV1 {
                authoritative_server_tick: authority.completed_tick,
                authoritative_frozen: authority.frozen,
                readiness_trace_digest,
                readiness_stable_frames: stable_frames,
                visible_scene_coverage,
                diagnostic_client_tick: simulation_tick,
                diagnostic_interpolated_time_bits: sim_time.to_bits(),
                semantic_trace,
                presentation,
            },
        );
    }
    let complete = completed >= count;
    if complete {
        crate::r0p_observer::finalize();
    }
    complete
}

fn request_one_capture(
    renderer: &mut super::renderer::Renderer,
    output: &Path,
    ordinal: u64,
    anchor: CaptureAnchorEvidenceV1,
    context: CaptureRequestContextV1,
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
    let pass_tape = LATEST_PASS_TAPE.lock().ok().and_then(|value| value.clone());
    let figure_gpu = super::figure_gpu::latest_evidence();
    let figure_batch = super::figure_batch::latest_evidence();
    let individual_tiers = crate::r1d_tiers::latest_evidence();
    let individual_tier_plan = crate::r1d_tiers::latest_plan();
    let group_representations = crate::r1d_groups::latest_evidence();
    let group_protected_uids = crate::r1d_groups::protected_uid_csv();
    let cutaway = crate::r1e_cutaway::latest_evidence();
    let interiors = crate::r1e_interiors::latest_evidence();
    let islands = crate::r1e_islands::latest_evidence();
    let environment = crate::r1f_environment::latest_evidence();
    let weather = crate::r1f_weather::latest_evidence();
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
                                let pass_tape = pass_tape.as_deref().ok_or_else(|| {
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "pass tape absent",
                                    )
                                })?;
                                let figure_gpu = figure_gpu
                                    .filter(|value| {
                                        value.generation
                                            == context.presentation.client_applied_generation
                                            && value.frame_digest
                                                == context.presentation.frame_digest
                                            && value.resource_set_digest
                                                == context.presentation.resource_set_digest
                                    })
                                    .ok_or_else(|| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "exact figure GPU upload completion absent",
                                        )
                                    })?;
                                let individual_tiers = individual_tiers
                                    .filter(|value| {
                                        value.generation
                                            == context.presentation.client_applied_generation
                                            && value.frame_digest
                                                == context.presentation.frame_digest
                                    })
                                    .ok_or_else(|| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "exact individual tier plan absent",
                                        )
                                    })?;
                                let group_representations = group_representations.filter(|value| {
                                    value.generation
                                        == context.presentation.client_applied_generation
                                        && value.frame_digest == context.presentation.frame_digest
                                        && value.individual_plan_root
                                            == individual_tiers.decision_root
                                });
                                if (std::env::var_os("BASTION_R1D_GROUP_SMOKE").is_some()
                                    || crate::r1d_scale::enabled())
                                    && group_representations.is_none()
                                {
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "exact group representation plan absent",
                                    ));
                                }
                                let group_representations =
                                    group_representations.unwrap_or_default();
                                let cutaway = if crate::r1e_cutaway::enabled() {
                                    let value = cutaway.ok_or_else(|| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "cutaway evidence absent",
                                        )
                                    })?;
                                    if !value.ready
                                        || u64::from(value.stage as u8) != ordinal
                                        || value.presentation_generation
                                            != context.presentation.client_applied_generation
                                        || value.terrain_generation == [0; 32]
                                        || value.policy_digest == [0; 32]
                                        || value.geometry_digest == [0; 32]
                                        || value.reveal_authority == [0; 32]
                                        || !value.fixture_owned
                                    {
                                        return Err(std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "cutaway evidence mismatch",
                                        ));
                                    }
                                    Some(value)
                                } else {
                                    None
                                };
                                let scale_evidence = if crate::r1d_scale::enabled() {
                                    let plan = individual_tier_plan.as_ref().ok_or_else(|| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            "scale individual plan absent",
                                        )
                                    })?;
                                    let member_set_digest =
                                        crate::r1d_scale::canonical_member_set_digest_v1(
                                            plan.decisions
                                                .iter()
                                                .map(|decision| decision.semantic_entity),
                                        )
                                        .map_err(
                                            |error| {
                                                std::io::Error::new(
                                                    std::io::ErrorKind::InvalidData,
                                                    format!("scale member set {error:?}"),
                                                )
                                            },
                                        )?;
                                    let sample =
                                        crate::r1d_scale::camera_sample_for_request_ordinal(
                                            ordinal,
                                        );
                                    let record = crate::r1d_scale::ScaleSampleRecordV1 {
                                        ordinal,
                                        policy_tick: plan.tick,
                                        camera_distance_mm: sample.distance_mm,
                                        population_count: u32::try_from(plan.decisions.len())
                                            .map_err(|_| {
                                                std::io::Error::new(
                                                    std::io::ErrorKind::InvalidData,
                                                    "scale population overflow",
                                                )
                                            })?,
                                        member_set_digest,
                                        tier_root: individual_tiers.decision_root,
                                        full_count: individual_tiers.full_count,
                                        reduced_count: individual_tiers.reduced_count,
                                        lod_count: individual_tiers.lod_count,
                                        impostor_count: individual_tiers.impostor_count,
                                        culled_count: individual_tiers.culled_count,
                                        group_root: group_representations.group_plan_root,
                                        group_count: group_representations.group_count,
                                        group_member_count: group_representations.member_count,
                                        protected_member_count: group_representations
                                            .protected_member_count,
                                        tier_transition_count: individual_tiers.transition_count,
                                        group_transition_count: group_representations
                                            .transition_count,
                                    };
                                    let digest = record.digest().map_err(|error| {
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            format!("scale record {error:?}"),
                                        )
                                    })?;
                                    Some((record, digest))
                                } else {
                                    None
                                };
                                let mut metadata = format!(
                                    concat!(
                                        "schema=RendererCaptureEvidenceV1\n",
                                        "ordinal={}\n",
                                        "authoritative_server_tick={}\n",
                                        "authoritative_frozen={}\n",
                                        "readiness_trace_sha256={}\n",
                                        "readiness_stable_frames={}\n",
                                        "visible_scene_ready=true\n",
                                        "visible_scene_coverage_mask={}\n",
                                        "visible_scene_figure_draw_count={}\n",
                                        "visible_scene_figure_units={}\n",
                                        "visible_scene_figure_instances={}\n",
                                        "visible_scene_terrain_draw_count={}\n",
                                        "visible_scene_terrain_units={}\n",
                                        "visible_scene_terrain_instances={}\n",
                                        "visible_scene_lod_terrain_draw_count={}\n",
                                        "visible_scene_lod_terrain_units={}\n",
                                        "visible_scene_lod_terrain_instances={}\n",
                                        "presentation_generation={}\n",
                                        "presentation_frame_sha256={}\n",
                                        "presentation_resource_set_sha256={}\n",
                                        "presentation_semantic_tape_sha256={}\n",
                                        "figure_gpu_package_sha256={}\n",
                                        "figure_gpu_material_table_sha256={}\n",
                                        "figure_gpu_material_shader_interface_sha256={}\n",
                                        "figure_gpu_assignment_sha256={}\n",
                                        "figure_gpu_staged_sha256={}\n",
                                        "figure_gpu_plan_sha256={}\n",
                                        "figure_gpu_submission_sequence={}\n",
                                        "figure_gpu_completion_sequence={}\n",
                                        "figure_gpu_instance_count={}\n",
                                        "figure_gpu_pose_page_count={}\n",
                                        "figure_gpu_upload_windows={}\n",
                                        "figure_gpu_upload_operations={}\n",
                                        "figure_gpu_upload_bytes={}\n",
                                        "figure_gpu_material_entry_count={}\n",
                                        "figure_gpu_material_legacy_fallback_count={}\n",
                                        "figure_batch_visible_figures={}\n",
                                        "figure_batch_count={}\n",
                                        "figure_batch_fallback_count={}\n",
                                        "figure_batch_legacy_draw_equivalent={}\n",
                                        "figure_batch_actual_draw_count={}\n",
                                        "figure_batch_main_count={}\n",
                                        "figure_batch_shadow_count={}\n",
                                        "figure_batch_rain_count={}\n",
                                        "figure_batch_capacity_fallbacks={}\n",
                                        "figure_batch_incompatible_resource_fallbacks={}\n",
                                        "r1d_tier_generation={}\n",
                                        "r1d_tier_frame_sha256={}\n",
                                        "r1d_tier_decision_root_sha256={}\n",
                                        "r1d_tier_full_count={}\n",
                                        "r1d_tier_reduced_count={}\n",
                                        "r1d_tier_lod_count={}\n",
                                        "r1d_tier_impostor_count={}\n",
                                        "r1d_tier_culled_count={}\n",
                                        "r1d_tier_full_shadow_count={}\n",
                                        "r1d_tier_proxy_shadow_count={}\n",
                                        "r1d_tier_fallback_count={}\n",
                                        "r1d_tier_transition_count={}\n",
                                        "r1d_tier_budget_max_visible={}\n",
                                        "r1d_tier_budget_max_full={}\n",
                                        "r1d_tier_budget_max_reduced={}\n",
                                        "r1d_tier_budget_max_lod={}\n",
                                        "r1d_tier_budget_max_impostor={}\n",
                                        "r1d_tier_fallback_full_budget={}\n",
                                        "r1d_tier_fallback_animation_budget={}\n",
                                        "r1d_tier_fallback_lod_budget={}\n",
                                        "r1d_tier_fallback_impostor_budget={}\n",
                                        "r1d_tier_fallback_visible_budget={}\n",
                                        "r1d_tier_fallback_lod_unavailable={}\n",
                                        "r1d_tier_fallback_impostor_unavailable={}\n",
                                        "r1d_tier_fallback_shadow_proxy_unavailable={}\n",
                                        "r1d_group_generation={}\n",
                                        "r1d_group_frame_sha256={}\n",
                                        "r1d_group_individual_plan_sha256={}\n",
                                        "r1d_group_plan_sha256={}\n",
                                        "r1d_group_count={}\n",
                                        "r1d_group_member_count={}\n",
                                        "r1d_group_individual_count={}\n",
                                        "r1d_group_formation_count={}\n",
                                        "r1d_group_aggregate_count={}\n",
                                        "r1d_group_protected_member_count={}\n",
                                        "r1d_group_protected_uids={}\n",
                                        "r1d_group_transition_count={}\n",
                                        "r1d_group_fixture_count={}\n",
                                        "r1d_group_authoritative_count={}\n",
                                        "r1d_group_line_count={}\n",
                                        "r1d_group_column_count={}\n",
                                        "r1d_group_wedge_count={}\n",
                                        "r1d_group_grid_count={}\n",
                                        "r1d_group_procession_count={}\n",
                                        "diagnostic_client_tick={}\n",
                                        "diagnostic_interpolated_time_bits={:016x}\n",
                                        "width={}\n",
                                        "height={}\n",
                                        "pixel_format=rgb8_srgb\n",
                                        "pixel_sha256={}\n",
                                        "anchor_uid={}\n",
                                        "anchor_selected_non_client_colonist={}\n",
                                        "anchor_category={}\n",
                                        "anchor_body={}\n",
                                        "pass_tape={}\n",
                                        "semantic_trace_count={}\n",
                                        "semantic_trace_sha256={}\n",
                                    ),
                                    ordinal,
                                    context.authoritative_server_tick,
                                    context.authoritative_frozen,
                                    hex_digest(&context.readiness_trace_digest),
                                    context.readiness_stable_frames,
                                    context.visible_scene_coverage.mask,
                                    context.visible_scene_coverage.figure_draw_count,
                                    context.visible_scene_coverage.figure_units,
                                    context.visible_scene_coverage.figure_instances,
                                    context.visible_scene_coverage.terrain_draw_count,
                                    context.visible_scene_coverage.terrain_units,
                                    context.visible_scene_coverage.terrain_instances,
                                    context.visible_scene_coverage.lod_terrain_draw_count,
                                    context.visible_scene_coverage.lod_terrain_units,
                                    context.visible_scene_coverage.lod_terrain_instances,
                                    context.presentation.client_applied_generation,
                                    hex_digest(&context.presentation.frame_digest),
                                    hex_digest(&context.presentation.resource_set_digest),
                                    hex_digest(&context.presentation.semantic_tape_root),
                                    hex_digest(&figure_gpu.package_digest),
                                    hex_digest(&figure_gpu.material_table_digest),
                                    hex_digest(&figure_gpu.material_shader_interface_digest),
                                    hex_digest(&figure_gpu.assignment_digest),
                                    hex_digest(&figure_gpu.staged_digest),
                                    hex_digest(&figure_gpu.plan_digest),
                                    figure_gpu.submission_sequence,
                                    figure_gpu.completion_sequence,
                                    figure_gpu.instance_count,
                                    figure_gpu.pose_page_count,
                                    figure_gpu.upload_windows,
                                    figure_gpu.upload_operations,
                                    figure_gpu.upload_bytes,
                                    figure_gpu.material_entry_count,
                                    figure_gpu.material_legacy_fallback_count,
                                    figure_batch.visible_figures,
                                    figure_batch.batch_count,
                                    figure_batch.fallback_count,
                                    figure_batch.legacy_draw_equivalent,
                                    figure_batch.actual_draw_count,
                                    figure_batch.main_batches,
                                    figure_batch.shadow_batches,
                                    figure_batch.rain_batches,
                                    figure_batch.capacity_fallbacks,
                                    figure_batch.incompatible_resource_fallbacks,
                                    individual_tiers.generation,
                                    hex_digest(&individual_tiers.frame_digest),
                                    hex_digest(&individual_tiers.decision_root),
                                    individual_tiers.full_count,
                                    individual_tiers.reduced_count,
                                    individual_tiers.lod_count,
                                    individual_tiers.impostor_count,
                                    individual_tiers.culled_count,
                                    individual_tiers.full_shadow_count,
                                    individual_tiers.proxy_shadow_count,
                                    individual_tiers.fallback_count,
                                    individual_tiers.transition_count,
                                    individual_tiers.max_visible,
                                    individual_tiers.max_full,
                                    individual_tiers.max_reduced,
                                    individual_tiers.max_lod,
                                    individual_tiers.max_impostor,
                                    individual_tiers.full_budget_fallbacks,
                                    individual_tiers.animation_budget_fallbacks,
                                    individual_tiers.lod_budget_fallbacks,
                                    individual_tiers.impostor_budget_fallbacks,
                                    individual_tiers.visible_budget_fallbacks,
                                    individual_tiers.lod_unavailable_fallbacks,
                                    individual_tiers.impostor_unavailable_fallbacks,
                                    individual_tiers.shadow_proxy_unavailable_fallbacks,
                                    group_representations.generation,
                                    hex_digest(&group_representations.frame_digest),
                                    hex_digest(&group_representations.individual_plan_root),
                                    hex_digest(&group_representations.group_plan_root),
                                    group_representations.group_count,
                                    group_representations.member_count,
                                    group_representations.individual_group_count,
                                    group_representations.formation_group_count,
                                    group_representations.aggregate_group_count,
                                    group_representations.protected_member_count,
                                    group_protected_uids,
                                    group_representations.transition_count,
                                    group_representations.fixture_group_count,
                                    group_representations.authoritative_group_count,
                                    group_representations.line_count,
                                    group_representations.column_count,
                                    group_representations.wedge_count,
                                    group_representations.grid_count,
                                    group_representations.procession_count,
                                    context.diagnostic_client_tick,
                                    context.diagnostic_interpolated_time_bits,
                                    width,
                                    height,
                                    hex_digest(&digest),
                                    anchor.uid,
                                    anchor.selected_non_client_colonist,
                                    anchor.body_category,
                                    anchor.body,
                                    pass_tape,
                                    context.semantic_trace.record_count,
                                    hex_digest(&context.semantic_trace.digest),
                                );
                                if let Some((scale, scale_digest)) = scale_evidence {
                                    metadata.push_str(&format!(
                                        concat!(
                                            "r1d_scale_enabled=true\n",
                                            "r1d_scale_sample_ordinal={}\n",
                                            "r1d_scale_policy_tick={}\n",
                                            "r1d_scale_camera_distance_mm={}\n",
                                            "r1d_scale_population_count={}\n",
                                            "r1d_scale_member_set_sha256={}\n",
                                            "r1d_scale_sample_sha256={}\n",
                                        ),
                                        scale.ordinal,
                                        scale.policy_tick,
                                        scale.camera_distance_mm,
                                        scale.population_count,
                                        hex_digest(&scale.member_set_digest),
                                        hex_digest(&scale_digest),
                                    ));
                                } else {
                                    metadata.push_str("r1d_scale_enabled=false\n");
                                }
                                if let Some(cutaway) = cutaway {
                                    metadata.push_str(&format!(
                                        concat!(
                                            "r1e_cutaway_enabled=true\n",
                                            "r1e_cutaway_stage={}\n",
                                            "r1e_cutaway_presentation_generation={}\n",
                                            "r1e_cutaway_terrain_generation_sha256={}\n",
                                            "r1e_cutaway_terrain_revision={}\n",
                                            "r1e_cutaway_camera_token_sha256={}\n",
                                            "r1e_cutaway_policy_sha256={}\n",
                                            "r1e_cutaway_terrain_root_sha256={}\n",
                                            "r1e_cutaway_geometry_sha256={}\n",
                                            "r1e_cutaway_reveal_authority_sha256={}\n",
                                            "r1e_cutaway_authorized_cell_count={}\n",
                                            "r1e_cutaway_removed_cell_count={}\n",
                                            "r1e_cutaway_cap_face_count={}\n",
                                            "r1e_cutaway_cap_triangle_count={}\n",
                                            "r1e_cutaway_cap_draw_triangle_count={}\n",
                                            "r1e_cutaway_cap_draw_ready={}\n",
                                            "r1e_cutaway_production_target_count={}\n",
                                            "r1e_cutaway_roof_removal_count={}\n",
                                            "r1e_cutaway_wall_removal_count={}\n",
                                            "r1e_cutaway_slice_z={}\n",
                                            "r1e_cutaway_stable_frames={}\n",
                                            "r1e_cutaway_fixture_owned=true\n",
                                        ),
                                        cutaway.stage.label(),
                                        cutaway.presentation_generation,
                                        hex_digest(&cutaway.terrain_generation),
                                        cutaway.terrain_revision,
                                        hex_digest(&cutaway.camera_token),
                                        hex_digest(&cutaway.policy_digest),
                                        hex_digest(&cutaway.terrain_root),
                                        hex_digest(&cutaway.geometry_digest),
                                        hex_digest(&cutaway.reveal_authority),
                                        cutaway.authorized_cell_count,
                                        cutaway.removed_cell_count,
                                        cutaway.cap_face_count,
                                        cutaway.cap_triangle_count,
                                        cutaway.cap_draw_triangle_count,
                                        cutaway.cap_draw_ready,
                                        cutaway.production_target_count,
                                        cutaway.roof_removal_count,
                                        cutaway.wall_removal_count,
                                        cutaway
                                            .slice_z
                                            .map_or_else(|| "none".to_owned(), |z| z.to_string()),
                                        cutaway.stable_frames,
                                    ));
                                } else {
                                    metadata.push_str("r1e_cutaway_enabled=false\n");
                                }
                                if let Some(interiors) = interiors {
                                    metadata.push_str(&format!(
                                        concat!(
                                            "r1e_interiors_enabled=true\n",
                                            "r1e_interiors_source_capability={}\n",
                                            "r1e_interiors_unavailable_room_authority={}\n",
                                            "r1e_interiors_unavailable_portal_authority={}\n",
                                            "r1e_interiors_presentation_generation={}\n",
                                            "r1e_interiors_visibility_sequence={}\n",
                                            "r1e_interiors_snapshot_sha256={}\n",
                                            "r1e_interiors_maximum_visible_z={}\n",
                                            "r1e_interiors_room_count={}\n",
                                            "r1e_interiors_portal_count={}\n",
                                            "r1e_interiors_visible_room_count={}\n",
                                            "r1e_interiors_z_level_fallback={}\n",
                                        ),
                                        interiors.source_capability,
                                        interiors.unavailable_room_authority,
                                        interiors.unavailable_portal_authority,
                                        interiors.presentation_generation,
                                        interiors.visibility_sequence,
                                        hex_digest(&interiors.snapshot_digest),
                                        interiors.maximum_visible_z,
                                        interiors.room_count,
                                        interiors.portal_count,
                                        interiors.visible_room_count,
                                        interiors.z_level_fallback,
                                    ));
                                } else {
                                    metadata.push_str("r1e_interiors_enabled=false\n");
                                }
                                if let Some(islands) = islands {
                                    metadata.push_str(&format!(
                                        concat!(
                                            "r1e_islands_enabled=true\n",
                                            "r1e_islands_source_capability={}\n",
                                            "r1e_islands_unavailable_portal_authority={}\n",
                                            "r1e_islands_presentation_generation={}\n",
                                            "r1e_islands_generation={}\n",
                                            "r1e_islands_publication_sequence={}\n",
                                            "r1e_islands_snapshot_sha256={}\n",
                                            "r1e_islands_island_count={}\n",
                                            "r1e_islands_member_count={}\n",
                                            "r1e_islands_portal_count={}\n",
                                            "r1e_islands_active={}\n",
                                        ),
                                        islands.source_capability,
                                        islands.unavailable_portal_authority,
                                        islands.presentation_generation,
                                        islands.island_generation,
                                        islands.publication_sequence,
                                        hex_digest(&islands.snapshot_digest),
                                        islands.island_count,
                                        islands.member_count,
                                        islands.portal_count,
                                        islands.active,
                                    ));
                                } else {
                                    metadata.push_str("r1e_islands_enabled=false\n");
                                }
                                if let Some(environment) = environment {
                                    metadata.push_str(&format!(
                                        concat!(
                                            "r1f_environment_enabled=true\n",
                                            "r1f_environment_presentation_generation={}\n",
                                            "r1f_environment_simulation_tick={}\n",
                                            "r1f_environment_frame_sha256={}\n",
                                            "r1f_environment_material_table_sha256={}\n",
                                            "r1f_environment_source_sha256={}\n",
                                            "r1f_environment_projection_sha256={}\n",
                                            "r1f_environment_availability_bits={}\n",
                                            "r1f_environment_client_interpolation_diagnostic={}\n",
                                        ),
                                        environment.presentation_generation,
                                        environment.simulation_tick,
                                        hex_digest(&environment.frame_digest),
                                        hex_digest(&environment.material_table_digest),
                                        hex_digest(&environment.environment_identity),
                                        hex_digest(&environment.projection_digest),
                                        environment.availability_bits,
                                        environment.client_interpolation_is_diagnostic,
                                    ));
                                } else {
                                    metadata.push_str("r1f_environment_enabled=false\n");
                                }
                                if let Some(weather) = weather.filter(|weather| {
                                    weather.presentation_generation
                                        == context.presentation.client_applied_generation
                                }) {
                                    metadata.push_str(&format!(
                                        concat!(
                                            "r1f_weather_enabled=true\n",
                                            "r1f_weather_presentation_generation={}\n",
                                            "r1f_weather_simulation_tick={}\n",
                                            "r1f_weather_environment_projection_sha256={}\n",
                                            "r1f_weather_environment_source_sha256={}\n",
                                            "r1f_weather_kind={}\n",
                                            "r1f_weather_rain_milli={}\n",
                                            "r1f_weather_precipitation_milli={}\n",
                                            "r1f_weather_wind_x_mm_s={}\n",
                                            "r1f_weather_wind_y_mm_s={}\n",
                                            "r1f_weather_phase_milli={}\n",
                                            "r1f_weather_effect_record_count={}\n",
                                            "r1f_weather_effect_instance_count={}\n",
                                            "r1f_weather_presentation_sha256={}\n",
                                            "r1f_weather_legacy_rollback={}\n",
                                        ),
                                        weather.presentation_generation,
                                        weather.simulation_tick,
                                        hex_digest(&weather.environment_projection_digest),
                                        hex_digest(&weather.environment_source_identity),
                                        weather.weather_tag,
                                        weather.rain_milli,
                                        weather.precipitation_milli,
                                        weather.wind_mm_s[0],
                                        weather.wind_mm_s[1],
                                        weather.phase_milli,
                                        weather.effect_record_count,
                                        weather.effect_instance_count,
                                        hex_digest(&weather.presentation_digest),
                                        weather.legacy_rollback,
                                    ));
                                } else {
                                    metadata.push_str("r1f_weather_enabled=false\n");
                                }
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

    fn valid_anchor() -> CaptureAnchorEvidenceV1 {
        CaptureAnchorEvidenceV1 {
            uid: 2,
            selected_non_client_colonist: true,
            body_category: "bastion_colonist".to_owned(),
            body: "Humanoid(Dwarf)".to_owned(),
        }
    }

    fn valid_coverage() -> VisibleSceneCoverageV1 {
        visible_scene_coverage_v1(&[(draw_kind::TERRAIN, 24, 1), (draw_kind::FIGURE, 12, 1)])
            .unwrap()
    }

    fn trace(
        generation: u64,
        digest: [u8; 32],
        visible_scene_coverage: VisibleSceneCoverageV1,
    ) -> SemanticTraceSnapshotV1 {
        SemanticTraceSnapshotV1 {
            generation,
            record_count: 10,
            digest,
            visible_scene_coverage,
            presentation_generation: None,
        }
    }

    fn frozen_authority() -> CertificationServerLatchV1 {
        CertificationServerLatchV1 {
            completed_tick: CERTIFICATION_SERVER_TICK_V1,
            frozen: true,
        }
    }

    #[test]
    fn absolute_time_capture_bypasses_client_pause_wait_but_freezes_server() {
        assert!(!capture_waits_for_pause_v1(true, true));
        assert_eq!(
            certification_freeze_tick_v1(true, true),
            Some(CERTIFICATION_SERVER_TICK_V1)
        );
    }

    #[test]
    fn pause_only_capture_remains_flat_arena_opt_in() {
        assert!(capture_waits_for_pause_v1(true, false));
        assert_eq!(certification_freeze_tick_v1(true, false), Some(300));
        assert!(!capture_waits_for_pause_v1(false, false));
        assert_eq!(certification_freeze_tick_v1(false, false), None);
    }

    #[test]
    fn certification_server_latch_resets_updates_and_freezes_exactly() {
        let mut latch = CertificationServerLatchV1::default();
        latch.record_completed_tick(1).unwrap();
        assert_eq!(latch, CertificationServerLatchV1 {
            completed_tick: 1,
            frozen: false,
        });
        latch
            .record_completed_tick(CERTIFICATION_SERVER_TICK_V1)
            .unwrap();
        assert_eq!(latch, CertificationServerLatchV1 {
            completed_tick: CERTIFICATION_SERVER_TICK_V1,
            frozen: true,
        });
        assert_eq!(
            latch.record_completed_tick(CERTIFICATION_SERVER_TICK_V1 + 1),
            Err(CertificationServerLatchErrorV1::AdvancedAfterFreeze)
        );
        latch.reset();
        assert_eq!(latch, CertificationServerLatchV1::default());
    }

    #[test]
    fn ui_only_coverage_keeps_readiness_closed() {
        let coverage = visible_scene_coverage_v1(&[(draw_kind::UI, 12, 1)]).unwrap();
        assert_eq!(coverage, VisibleSceneCoverageV1::default());
        let mut gate = SettledTraceGateV1::default();
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(1, [1; 32], coverage),
                Some(&valid_anchor()),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
    }

    #[test]
    fn terrain_only_coverage_keeps_readiness_closed() {
        let coverage = visible_scene_coverage_v1(&[(draw_kind::TERRAIN, 24, 1)]).unwrap();
        assert!(coverage.has_terrain());
        assert!(!coverage.has_figure());
        let mut gate = SettledTraceGateV1::default();
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(1, [2; 32], coverage),
                Some(&valid_anchor()),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
    }

    #[test]
    fn figure_only_coverage_keeps_readiness_closed() {
        let coverage = visible_scene_coverage_v1(&[(draw_kind::FIGURE, 12, 1)]).unwrap();
        assert!(coverage.has_figure());
        assert!(!coverage.has_terrain());
        let mut gate = SettledTraceGateV1::default();
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(1, [3; 32], coverage),
                Some(&valid_anchor()),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
    }

    #[test]
    fn missing_or_invalid_anchor_keeps_readiness_closed() {
        let coverage = valid_coverage();
        let mut gate = SettledTraceGateV1::default();
        assert_eq!(
            gate.observe(frozen_authority(), trace(1, [4; 32], coverage), None),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
        for (index, anchor) in [
            CaptureAnchorEvidenceV1 {
                uid: 0,
                ..valid_anchor()
            },
            CaptureAnchorEvidenceV1 {
                selected_non_client_colonist: false,
                ..valid_anchor()
            },
            CaptureAnchorEvidenceV1 {
                body_category: "client_spectator".to_owned(),
                ..valid_anchor()
            },
            CaptureAnchorEvidenceV1 {
                body: String::new(),
                ..valid_anchor()
            },
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                gate.observe(
                    frozen_authority(),
                    trace(index as u64 + 2, [4; 32], coverage),
                    Some(&anchor),
                ),
                Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
            );
        }
    }

    #[test]
    fn visible_scene_coverage_loss_resets_stability() {
        let digest = [5; 32];
        let mut gate = SettledTraceGateV1::default();
        let anchor = valid_anchor();
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(1, digest, valid_coverage()),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(2, digest, valid_coverage()),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 1 })
        );
        let figure_only = visible_scene_coverage_v1(&[(draw_kind::FIGURE, 12, 1)]).unwrap();
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(3, digest, figure_only),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(4, digest, valid_coverage()),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 1 })
        );
    }

    #[test]
    fn valid_terrain_and_figure_open_after_full_stability_window() {
        let digest = [7; 32];
        let mut gate = SettledTraceGateV1::default();
        let anchor = valid_anchor();
        let coverage = valid_coverage();
        let running = CertificationServerLatchV1 {
            completed_tick: CERTIFICATION_SERVER_TICK_V1 - 1,
            frozen: false,
        };
        assert_eq!(
            gate.observe(running, trace(5, digest, coverage), Some(&anchor)),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(6, digest, coverage),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Waiting { stable_frames: 0 })
        );
        for index in 1..READINESS_STABLE_RENDER_FRAMES_V1 {
            assert_eq!(
                gate.observe(
                    frozen_authority(),
                    trace(6 + index, digest, coverage),
                    Some(&anchor),
                ),
                Ok(SettledTraceObservationV1::Waiting {
                    stable_frames: index,
                })
            );
        }
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(6 + READINESS_STABLE_RENDER_FRAMES_V1, digest, coverage,),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Open {
                digest,
                coverage,
                stable_frames: READINESS_STABLE_RENDER_FRAMES_V1,
                advanced: true,
            })
        );
        assert_eq!(
            gate.observe(
                frozen_authority(),
                trace(7 + READINESS_STABLE_RENDER_FRAMES_V1, [8; 32], coverage,),
                Some(&anchor),
            ),
            Ok(SettledTraceObservationV1::Open {
                digest,
                coverage,
                stable_frames: READINESS_STABLE_RENDER_FRAMES_V1,
                advanced: true,
            })
        );
    }

    #[test]
    fn capture_metadata_separates_authority_from_client_diagnostics() {
        assert_eq!(
            capture_metadata_field_class_v1("authoritative_server_tick"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("authoritative_frozen"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("readiness_trace_sha256"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("visible_scene_ready"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("visible_scene_figure_draw_count"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("anchor_selected_non_client_colonist"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("figure_gpu_completion_sequence"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("figure_gpu_material_table_sha256"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("r1f_environment_projection_sha256"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("r1f_environment_availability_bits"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("r1d_tier_decision_root_sha256"),
            Some(CaptureMetadataFieldClassV1::Authority)
        );
        assert_eq!(
            capture_metadata_field_class_v1("r1d_tier_lod_count"),
            Some(CaptureMetadataFieldClassV1::Evidence)
        );
        assert_eq!(
            capture_metadata_field_class_v1("figure_gpu_upload_bytes"),
            Some(CaptureMetadataFieldClassV1::Evidence)
        );
        assert_eq!(
            capture_metadata_field_class_v1("figure_gpu_material_entry_count"),
            Some(CaptureMetadataFieldClassV1::Evidence)
        );
        assert_eq!(
            capture_metadata_field_class_v1("diagnostic_client_tick"),
            Some(CaptureMetadataFieldClassV1::Diagnostic)
        );
        assert_eq!(
            capture_metadata_field_class_v1("diagnostic_interpolated_time_bits"),
            Some(CaptureMetadataFieldClassV1::Diagnostic)
        );
        assert_eq!(
            capture_metadata_field_class_v1("r1f_environment_client_interpolation_diagnostic"),
            Some(CaptureMetadataFieldClassV1::Diagnostic)
        );
        assert_eq!(capture_metadata_field_class_v1("simulation_tick"), None);
        assert_eq!(
            capture_metadata_field_class_v1("simulation_time_bits"),
            None
        );
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
        let coverage = visible_scene_coverage_v1(&ordered).unwrap();
        assert_eq!(coverage.mask, 3);
        assert_eq!(coverage.figure_draw_count, 1);
        assert_eq!(coverage.figure_units, 12);
        assert_eq!(coverage.figure_instances, 2);
        assert_eq!(coverage.terrain_draw_count, 1);
        assert_eq!(coverage.terrain_units, 24);
        assert_eq!(coverage.terrain_instances, 1);
    }
}
