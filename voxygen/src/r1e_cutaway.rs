//! Narrow production seam for the R1E cutaway fixture and capture lane.

use std::sync::Mutex;

use bastion_renderer_r0d::{
    cutaway::{
        CellPositionV1, CutawayGeometryV1, CutawayModeV1, CutawayPolicyInputV1, CutawayPolicyV1,
        CutawayTransitionKindV1, CutawayTransitionV1, TerrainCellV1, TerrainSliceInputV1,
        derive_cutaway_geometry_v1,
    },
    domain_hash_v1,
    presentation::PresentationFrameV1,
};
use common::{terrain::TerrainGrid, vol::ReadVol};
use vek::Vec3;

use crate::scene::{DebugShape, DebugShapeId, Scene};

pub const CUTAWAY_CAPTURE_COUNT_V1: u64 = 3;
pub const CUTAWAY_SETTLE_FRAMES_V1: u16 = 120;
pub const CUTAWAY_FIXTURE_RADIUS_V1: i32 = 4;
pub const CUTAWAY_FIXTURE_DEPTH_V1: i32 = 6;
pub const CUTAWAY_CAP_MATERIAL_V1: u16 = 17;
pub const CUTAWAY_VISUAL_RADIUS_V1: f32 = 5.5;
const CUTAWAY_DIAGNOSTIC_CADENCE_V1: u16 = 120;
const CUTAWAY_DIAGNOSTIC_LIMIT_V1: u8 = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CutawayCaptureStageV1 {
    Surface = 0,
    Sliced = 1,
    Restored = 2,
}

impl CutawayCaptureStageV1 {
    #[must_use]
    pub const fn for_requested_ordinal(ordinal: u64) -> Self {
        match ordinal {
            0 => Self::Surface,
            1 => Self::Sliced,
            _ => Self::Restored,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Surface => "surface",
            Self::Sliced => "sliced",
            Self::Restored => "restored",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CutawayCaptureEvidenceV1 {
    pub stage: CutawayCaptureStageV1,
    pub presentation_generation: u64,
    pub terrain_generation: [u8; 32],
    pub terrain_revision: u64,
    pub camera_token: [u8; 32],
    pub policy_digest: [u8; 32],
    pub terrain_root: [u8; 32],
    pub geometry_digest: [u8; 32],
    pub reveal_authority: [u8; 32],
    pub authorized_cell_count: u32,
    pub removed_cell_count: u32,
    pub cap_face_count: u32,
    pub cap_triangle_count: u32,
    pub cap_draw_triangle_count: u32,
    pub cap_draw_ready: bool,
    pub production_target_count: u32,
    pub roof_removal_count: u32,
    pub wall_removal_count: u32,
    pub slice_z: Option<i32>,
    pub stable_frames: u16,
    pub ready: bool,
    pub fixture_owned: bool,
}

static LATEST_EVIDENCE: Mutex<Option<CutawayCaptureEvidenceV1>> = Mutex::new(None);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct AuthorityChangeBitsV1 {
    presentation_generation: bool,
    terrain_generation: bool,
    terrain_revision: bool,
    camera_token: bool,
    camera_sequence: bool,
}

impl AuthorityChangeBitsV1 {
    const fn any(self) -> bool {
        self.presentation_generation
            || self.terrain_generation
            || self.terrain_revision
            || self.camera_token
            || self.camera_sequence
    }
}

#[derive(Debug, Default)]
pub struct CutawayFixtureStateV1 {
    last_stage: Option<CutawayCaptureStageV1>,
    stage_anchor: Option<Vec3<f32>>,
    stage_entry_generation: u64,
    cap_shape: Option<DebugShapeId>,
    cap_shape_lifetime_frames: u32,
    stable_frames: u16,
    diagnostic_frames_since_emit: u16,
    diagnostic_emissions: u8,
    latest_geometry: Option<CutawayGeometryV1>,
}

#[must_use]
pub fn enabled() -> bool { std::env::var_os("BASTION_R1E_CUTAWAY_SMOKE").is_some() }

pub fn reset() {
    if let Ok(mut latest) = LATEST_EVIDENCE.lock() {
        *latest = None;
    }
}

#[must_use]
pub fn latest_evidence() -> Option<CutawayCaptureEvidenceV1> {
    LATEST_EVIDENCE.lock().ok().and_then(|value| *value)
}

#[must_use]
pub fn ready_for_capture() -> bool {
    !enabled() || latest_evidence().is_some_and(|value| value.ready)
}

pub fn apply_fixture(
    state: &mut CutawayFixtureStateV1,
    scene: &mut Scene,
    terrain: &TerrainGrid,
    frame: &PresentationFrameV1,
    anchor: Vec3<f32>,
    requested_ordinal: u64,
) -> Result<CutawayCaptureEvidenceV1, &'static str> {
    if !enabled() {
        return Err("cutaway fixture is disabled");
    }
    let stage = CutawayCaptureStageV1::for_requested_ordinal(requested_ordinal);
    let generation = frame.generation();
    let source_authority_changed = state.latest_geometry.as_ref().is_some_and(|geometry| {
        !geometry_matches_source_authority(
            geometry,
            generation.client_applied_generation,
            frame.environment().terrain_root,
            generation.simulation_tick,
            u64::from(stage as u8),
        )
    });
    let stage_changed = state.last_stage != Some(stage);
    let stage_anchor =
        state.latch_stage_anchor(stage, anchor, stage_changed || source_authority_changed);
    let expected_camera_token =
        camera_token(stage_anchor, stage).map_err(|_| "camera token hash")?;
    let change_bits = state
        .latest_geometry
        .as_ref()
        .map(|geometry| {
            authority_change_bits(
                geometry,
                generation.client_applied_generation,
                frame.environment().terrain_root,
                generation.simulation_tick,
                expected_camera_token,
                u64::from(stage as u8),
            )
        })
        .unwrap_or_default();
    let authority_changed = change_bits.any();
    let reset_reason = if stage_changed {
        Some("stage_changed")
    } else if change_bits.presentation_generation {
        Some("presentation_generation_changed")
    } else if change_bits.terrain_generation {
        Some("terrain_generation_changed")
    } else if change_bits.terrain_revision {
        Some("terrain_revision_changed")
    } else if change_bits.camera_token {
        Some("camera_token_changed")
    } else if change_bits.camera_sequence {
        Some("camera_sequence_changed")
    } else {
        None
    };
    if stage_changed || authority_changed {
        if let Some(shape) = state.cap_shape.take() {
            scene.debug.remove_shape(shape);
        }
        state.stable_frames = 0;
        state.cap_shape_lifetime_frames = 0;
        state.diagnostic_frames_since_emit = 0;
        state.latest_geometry = None;
        let geometry = build_geometry(frame, terrain, stage_anchor, stage)?;
        if stage == CutawayCaptureStageV1::Sliced {
            let triangles = cap_triangles(&geometry)?;
            if !triangles.is_empty() {
                let shape = scene.debug.add_shape(DebugShape::ConformedTris(triangles));
                scene
                    .debug
                    .set_context(shape, [0.0; 4], [0.82, 0.24, 0.04, 1.0], [
                        0.0, 0.0, 0.0, 1.0,
                    ]);
                state.cap_shape = Some(shape);
            }
        }
        state.latest_geometry = Some(geometry);
        state.last_stage = Some(stage);
        state.stage_entry_generation = generation.client_applied_generation;
    }
    if state.cap_shape.is_some() && state.cap_shape_lifetime_frames < u32::MAX {
        state.cap_shape_lifetime_frames += 1;
    }
    if state.diagnostic_frames_since_emit < CUTAWAY_DIAGNOSTIC_CADENCE_V1 {
        state.diagnostic_frames_since_emit += 1;
    }
    // Scene maintenance precedes this fixture hook. Reasserting the production
    // state is idempotent and leaves the newly created cap shape intact until
    // the next maintenance/render pass can produce its draw receipt.
    let geometry = state
        .latest_geometry
        .as_ref()
        .ok_or("cutaway geometry unavailable")?;
    configure_scene(scene, stage, stage_anchor, geometry)?;
    state.stable_frames = state
        .stable_frames
        .checked_add(1)
        .ok_or("cutaway stability counter overflow")?
        .min(CUTAWAY_SETTLE_FRAMES_V1);
    let authorized_cell_count = u32::try_from(
        fixture_positions(stage_anchor)
            .map_err(|_| "cutaway fixture bounds invalid")?
            .len(),
    )
    .map_err(|_| "authorized cell count overflow")?;
    let cap_face_count =
        u32::try_from(geometry.cap_faces.len()).map_err(|_| "cap face count overflow")?;
    let cap_triangle_count = cap_face_count
        .checked_mul(2)
        .ok_or("cap triangle count overflow")?;
    let cap_has_draw_model = match state.cap_shape {
        Some(shape) => scene.debug.has_draw_model(shape),
        None => stage != CutawayCaptureStageV1::Sliced,
    };
    let cap_draw_ready = match stage {
        CutawayCaptureStageV1::Sliced => cap_has_draw_model,
        CutawayCaptureStageV1::Surface | CutawayCaptureStageV1::Restored => true,
    };
    let production_target_count = u32::try_from(scene.bastion_occlusion().targets.len())
        .map_err(|_| "production target count overflow")?;
    let evidence = CutawayCaptureEvidenceV1 {
        stage,
        presentation_generation: geometry.presentation_generation,
        terrain_generation: geometry.terrain_generation,
        terrain_revision: geometry.terrain_revision,
        camera_token: geometry.camera_token,
        policy_digest: geometry.policy_digest,
        terrain_root: geometry.terrain_root,
        geometry_digest: geometry.geometry_digest,
        reveal_authority: reveal_authority(frame).map_err(|_| "reveal authority hash")?,
        authorized_cell_count,
        removed_cell_count: u32::try_from(geometry.removed_cells.len())
            .map_err(|_| "removed cell count overflow")?,
        cap_face_count,
        cap_triangle_count,
        cap_draw_triangle_count: if cap_draw_ready && stage == CutawayCaptureStageV1::Sliced {
            cap_triangle_count
        } else {
            0
        },
        cap_draw_ready,
        production_target_count,
        roof_removal_count: geometry.roof_removals,
        wall_removal_count: geometry.wall_removals,
        slice_z: slice_z(stage_anchor, stage),
        stable_frames: state.stable_frames,
        ready: state.stable_frames == CUTAWAY_SETTLE_FRAMES_V1
            && cap_draw_ready
            && (stage != CutawayCaptureStageV1::Sliced || production_target_count == 1),
        fixture_owned: true,
    };
    if reset_reason.is_some() || state.diagnostic_frames_since_emit == CUTAWAY_DIAGNOSTIC_CADENCE_V1
    {
        emit_transition_diagnostic(
            state,
            stage,
            generation.client_applied_generation,
            change_bits,
            reset_reason.unwrap_or("cadence"),
            cap_has_draw_model,
            cap_draw_ready,
            production_target_count,
        );
    }
    if let Ok(mut latest) = LATEST_EVIDENCE.lock() {
        *latest = Some(evidence);
    }
    Ok(evidence)
}

fn emit_transition_diagnostic(
    state: &mut CutawayFixtureStateV1,
    stage: CutawayCaptureStageV1,
    current_generation: u64,
    change_bits: AuthorityChangeBitsV1,
    reset_reason: &'static str,
    cap_has_draw_model: bool,
    cap_draw_ready: bool,
    production_target_count: u32,
) {
    if state.diagnostic_emissions >= CUTAWAY_DIAGNOSTIC_LIMIT_V1 {
        return;
    }
    tracing::info!(
        target: "bastion_r1e_cutaway",
        stage = stage.label(),
        stage_entry_generation = state.stage_entry_generation,
        current_generation,
        reset_reason,
        authority_presentation_generation_changed = change_bits.presentation_generation,
        authority_terrain_generation_changed = change_bits.terrain_generation,
        authority_terrain_revision_changed = change_bits.terrain_revision,
        authority_camera_token_changed = change_bits.camera_token,
        authority_camera_sequence_changed = change_bits.camera_sequence,
        cap_shape_id = ?state.cap_shape,
        cap_shape_lifetime_frames = state.cap_shape_lifetime_frames,
        cap_has_draw_model,
        cap_draw_ready,
        production_target_count,
        stable_frames = state.stable_frames,
        diagnostic_ordinal = state.diagnostic_emissions,
        "bounded cutaway stage transition diagnostic"
    );
    state.diagnostic_emissions += 1;
    state.diagnostic_frames_since_emit = 0;
}

fn authority_change_bits(
    geometry: &CutawayGeometryV1,
    presentation_generation: u64,
    terrain_generation: [u8; 32],
    terrain_revision: u64,
    camera_token: [u8; 32],
    camera_sequence: u64,
) -> AuthorityChangeBitsV1 {
    AuthorityChangeBitsV1 {
        presentation_generation: geometry.presentation_generation != presentation_generation,
        terrain_generation: geometry.terrain_generation != terrain_generation,
        terrain_revision: geometry.terrain_revision != terrain_revision,
        camera_token: geometry.camera_token != camera_token,
        camera_sequence: geometry.camera_sequence != camera_sequence,
    }
}

impl CutawayFixtureStateV1 {
    fn latch_stage_anchor(
        &mut self,
        stage: CutawayCaptureStageV1,
        incoming: Vec3<f32>,
        replace: bool,
    ) -> Vec3<f32> {
        if replace || self.last_stage != Some(stage) || self.stage_anchor.is_none() {
            self.stage_anchor = Some(incoming);
        }
        self.stage_anchor.unwrap_or(incoming)
    }
}

fn build_geometry(
    frame: &PresentationFrameV1,
    terrain: &TerrainGrid,
    anchor: Vec3<f32>,
    stage: CutawayCaptureStageV1,
) -> Result<CutawayGeometryV1, &'static str> {
    let positions = fixture_positions(anchor)?;
    let mut cells = Vec::with_capacity(positions.len());
    for position in &positions {
        let world = Vec3::new(position.x, position.y, position.z);
        let block = terrain.get(world).ok();
        let filled = block.is_some_and(|block| block.is_filled());
        let kind = block
            .map(|block| format!("{:?}", block.kind()))
            .unwrap_or_else(|| "UNLOADED".to_owned());
        let mut payload = Vec::with_capacity(12 + kind.len());
        payload.extend_from_slice(&position.x.to_le_bytes());
        payload.extend_from_slice(&position.y.to_le_bytes());
        payload.extend_from_slice(&position.z.to_le_bytes());
        payload.extend_from_slice(kind.as_bytes());
        let content_digest = domain_hash_v1("bastion/r1e/live-terrain-cell", 1, 0, &payload)
            .map_err(|_| "terrain cell hash")?;
        cells.push(TerrainCellV1 {
            position: *position,
            material: if filled { 1 } else { 0 },
            filled,
            reveal_eligible: true,
            content_digest: if filled { content_digest } else { [0; 32] },
        });
    }
    let generation = frame.generation();
    let terrain_generation = frame.environment().terrain_root;
    let camera_token = camera_token(anchor, stage).map_err(|_| "camera token hash")?;
    let mode = match stage {
        CutawayCaptureStageV1::Surface | CutawayCaptureStageV1::Restored => CutawayModeV1::Off,
        CutawayCaptureStageV1::Sliced => CutawayModeV1::Layer {
            maximum_visible_z: slice_z(anchor, stage).ok_or("slice height absent")? - 1,
        },
    };
    let transition = if matches!(mode, CutawayModeV1::Off) {
        bastion_renderer_r0d::cutaway::CutawayTransitionV1 {
            kind: CutawayTransitionKindV1::Off,
            step: 0,
            total_steps: 0,
        }
    } else {
        CutawayTransitionV1 {
            kind: CutawayTransitionKindV1::Held,
            step: 1,
            total_steps: 1,
        }
    };
    let policy = CutawayPolicyV1::new(
        CutawayPolicyInputV1 {
            presentation_generation: generation.client_applied_generation,
            terrain_generation,
            camera_token,
            camera_sequence: u64::from(stage as u8),
            mode,
            transition,
            cap_material: CUTAWAY_CAP_MATERIAL_V1,
            reveal_authority: reveal_authority(frame).map_err(|_| "reveal authority hash")?,
        },
        positions,
    )
    .map_err(|_| "cutaway policy invalid")?;
    let center = anchor_cell(anchor)?;
    let bounds_minimum = CellPositionV1::new(
        center
            .x
            .checked_sub(CUTAWAY_FIXTURE_RADIUS_V1)
            .ok_or("fixture x minimum overflow")?,
        center
            .y
            .checked_sub(CUTAWAY_FIXTURE_RADIUS_V1)
            .ok_or("fixture y minimum overflow")?,
        center
            .z
            .checked_sub(CUTAWAY_FIXTURE_DEPTH_V1)
            .ok_or("fixture z minimum overflow")?,
    );
    let bounds_maximum = CellPositionV1::new(
        center
            .x
            .checked_add(CUTAWAY_FIXTURE_RADIUS_V1)
            .ok_or("fixture x maximum overflow")?,
        center
            .y
            .checked_add(CUTAWAY_FIXTURE_RADIUS_V1)
            .ok_or("fixture y maximum overflow")?,
        center
            .z
            .checked_add(3)
            .ok_or("fixture z maximum overflow")?,
    );
    derive_cutaway_geometry_v1(&policy, TerrainSliceInputV1 {
        presentation_generation: generation.client_applied_generation,
        terrain_generation,
        terrain_revision: generation.simulation_tick,
        camera_token,
        camera_sequence: u64::from(stage as u8),
        bounds_minimum,
        bounds_maximum,
        cells,
    })
    .map_err(|_| "cutaway geometry invalid")
}

fn fixture_positions(anchor: Vec3<f32>) -> Result<Vec<CellPositionV1>, &'static str> {
    let center = anchor_cell(anchor)?;
    let minimum_x = center
        .x
        .checked_sub(CUTAWAY_FIXTURE_RADIUS_V1)
        .ok_or("fixture x minimum overflow")?;
    let maximum_x = center
        .x
        .checked_add(CUTAWAY_FIXTURE_RADIUS_V1)
        .ok_or("fixture x maximum overflow")?;
    let minimum_y = center
        .y
        .checked_sub(CUTAWAY_FIXTURE_RADIUS_V1)
        .ok_or("fixture y minimum overflow")?;
    let maximum_y = center
        .y
        .checked_add(CUTAWAY_FIXTURE_RADIUS_V1)
        .ok_or("fixture y maximum overflow")?;
    let minimum_z = center
        .z
        .checked_sub(CUTAWAY_FIXTURE_DEPTH_V1)
        .ok_or("fixture z minimum overflow")?;
    let maximum_z = center
        .z
        .checked_add(3)
        .ok_or("fixture z maximum overflow")?;
    let mut positions = Vec::new();
    for x in minimum_x..=maximum_x {
        for y in minimum_y..=maximum_y {
            for z in minimum_z..=maximum_z {
                positions.push(CellPositionV1::new(x, y, z));
            }
        }
    }
    Ok(positions)
}

fn configure_scene(
    scene: &mut Scene,
    stage: CutawayCaptureStageV1,
    anchor: Vec3<f32>,
    geometry: &CutawayGeometryV1,
) -> Result<(), &'static str> {
    let occlusion = scene.bastion_occlusion_mut();
    match stage {
        CutawayCaptureStageV1::Sliced => {
            let z = slice_z(anchor, stage).ok_or("slice height absent")?;
            if geometry.removed_cells.is_empty() || geometry.cap_faces.is_empty() {
                return Err("cutaway fixture produced no slice or cap");
            }
            occlusion.view_mode = crate::bastion::occlusion::ViewMode::Slice;
            occlusion.slice_enabled = true;
            occlusion.proximity_enabled = false;
            // The finite camera-to-anchor mask is an existing production
            // terrain-fragment path. Combining it with the global Z slice
            // opens the bounded fixture in the camera frustum instead of
            // leaving the canonical cap hidden under an uncut surface.
            occlusion.cutaway_enabled = true;
            occlusion.roof_enabled = false;
            occlusion.slice_z = Some(z as f32);
            occlusion.fade_band = 0.25;
            occlusion.strength = 0.0;
            occlusion.cutaway_radius = CUTAWAY_VISUAL_RADIUS_V1;
            occlusion.targets.clear();
            occlusion.targets.push(anchor);
        },
        CutawayCaptureStageV1::Surface | CutawayCaptureStageV1::Restored => {
            if !geometry.surface_passthrough
                || !geometry.removed_cells.is_empty()
                || !geometry.cap_faces.is_empty()
            {
                return Err("off-mode parity failure");
            }
            occlusion.view_mode = crate::bastion::occlusion::ViewMode::Solid;
            occlusion.slice_z = None;
            occlusion.cutaway_enabled = false;
            occlusion.targets.clear();
        },
    }
    Ok(())
}

fn cap_triangles(geometry: &CutawayGeometryV1) -> Result<Vec<[Vec3<f32>; 3]>, &'static str> {
    let capacity = geometry
        .cap_faces
        .len()
        .checked_mul(2)
        .ok_or("cap triangle capacity overflow")?;
    let mut output = Vec::with_capacity(capacity);
    for face in &geometry.cap_faces {
        let vertices = face
            .vertices
            .map(|vertex| Vec3::new(vertex[0] as f32, vertex[1] as f32, vertex[2] as f32));
        output.push([vertices[0], vertices[1], vertices[2]]);
        output.push([vertices[0], vertices[2], vertices[3]]);
    }
    Ok(output)
}

#[cfg(test)]
fn geometry_matches_authority(
    geometry: &CutawayGeometryV1,
    presentation_generation: u64,
    terrain_generation: [u8; 32],
    terrain_revision: u64,
    camera_token: [u8; 32],
    camera_sequence: u64,
) -> bool {
    geometry.presentation_generation == presentation_generation
        && geometry.terrain_generation == terrain_generation
        && geometry.terrain_revision == terrain_revision
        && geometry.camera_token == camera_token
        && geometry.camera_sequence == camera_sequence
}

fn geometry_matches_source_authority(
    geometry: &CutawayGeometryV1,
    presentation_generation: u64,
    terrain_generation: [u8; 32],
    terrain_revision: u64,
    camera_sequence: u64,
) -> bool {
    geometry.presentation_generation == presentation_generation
        && geometry.terrain_generation == terrain_generation
        && geometry.terrain_revision == terrain_revision
        && geometry.camera_sequence == camera_sequence
}

fn slice_z(anchor: Vec3<f32>, stage: CutawayCaptureStageV1) -> Option<i32> {
    (stage == CutawayCaptureStageV1::Sliced)
        .then(|| anchor_cell(anchor).ok()?.z.checked_sub(1))
        .flatten()
}

fn anchor_cell(anchor: Vec3<f32>) -> Result<Vec3<i32>, &'static str> {
    fn component(value: f32) -> Result<i32, &'static str> {
        if !value.is_finite() {
            return Err("nonfinite anchor");
        }
        let value = f64::from(value).floor();
        if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            return Err("anchor coordinate out of range");
        }
        Ok(value as i32)
    }
    Ok(Vec3::new(
        component(anchor.x)?,
        component(anchor.y)?,
        component(anchor.z)?,
    ))
}

fn reveal_authority(frame: &PresentationFrameV1) -> Result<[u8; 32], ()> {
    let mut payload = Vec::with_capacity(64);
    payload.extend_from_slice(&frame.frame_digest());
    payload.extend_from_slice(b"R1E-CUTAWAY-001:flat-arena-bounded-reveal-fixture");
    domain_hash_v1("bastion/r1e/reveal-authority", 1, 0, &payload).map_err(|_| ())
}

fn camera_token(anchor: Vec3<f32>, stage: CutawayCaptureStageV1) -> Result<[u8; 32], ()> {
    let mut payload = Vec::with_capacity(25);
    for component in [anchor.x, anchor.y, anchor.z] {
        if !component.is_finite() {
            return Err(());
        }
        payload.extend_from_slice(&component.to_bits().to_le_bytes());
    }
    payload.push(stage as u8);
    payload.extend_from_slice(&6_000_u32.to_le_bytes());
    payload.extend_from_slice(&core::f32::consts::FRAC_PI_4.to_bits().to_le_bytes());
    payload.extend_from_slice(&0.3_f32.to_bits().to_le_bytes());
    domain_hash_v1("bastion/r1e/camera-token", 1, 0, &payload).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_stage_is_surface_slice_restore_and_clamped() {
        assert_eq!(
            CutawayCaptureStageV1::for_requested_ordinal(0),
            CutawayCaptureStageV1::Surface
        );
        assert_eq!(
            CutawayCaptureStageV1::for_requested_ordinal(1),
            CutawayCaptureStageV1::Sliced
        );
        assert_eq!(
            CutawayCaptureStageV1::for_requested_ordinal(2),
            CutawayCaptureStageV1::Restored
        );
        assert_eq!(
            CutawayCaptureStageV1::for_requested_ordinal(u64::MAX),
            CutawayCaptureStageV1::Restored
        );
    }

    #[test]
    fn bounded_fixture_positions_are_unique_and_canonical() {
        let positions = fixture_positions(Vec3::new(10.5, -3.25, 20.0)).unwrap();
        assert_eq!(positions.len(), 810);
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(positions[0], CellPositionV1::new(6, -8, 14));
        assert_eq!(*positions.last().unwrap(), CellPositionV1::new(14, 0, 23));
        assert!(fixture_positions(Vec3::new(f32::MAX, 0.0, 0.0)).is_err());
    }

    #[test]
    fn camera_token_binds_stage_and_anchor_without_clock() {
        let anchor = Vec3::new(1.0, 2.0, 3.0);
        let surface = camera_token(anchor, CutawayCaptureStageV1::Surface).unwrap();
        assert_eq!(
            surface,
            camera_token(anchor, CutawayCaptureStageV1::Surface).unwrap()
        );
        assert_ne!(
            surface,
            camera_token(anchor, CutawayCaptureStageV1::Sliced).unwrap()
        );
        assert_ne!(
            surface,
            camera_token(Vec3::new(1.0, 2.0, 4.0), CutawayCaptureStageV1::Surface).unwrap()
        );
    }

    #[test]
    fn cap_conversion_preserves_face_order_and_winding() {
        let face = bastion_renderer_r0d::cutaway::CapFaceV1 {
            retained_cell: CellPositionV1::new(0, 0, 0),
            direction: bastion_renderer_r0d::cutaway::FaceDirectionV1::PositiveZ,
            vertices: [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
            normal: [0, 0, 1],
            material: CUTAWAY_CAP_MATERIAL_V1,
        };
        let geometry = CutawayGeometryV1 {
            presentation_generation: 1,
            terrain_generation: [1; 32],
            terrain_revision: 1,
            camera_token: [2; 32],
            camera_sequence: 1,
            policy_digest: [3; 32],
            terrain_root: [4; 32],
            removed_cells: vec![CellPositionV1::new(0, 0, 1)],
            cap_faces: vec![face],
            roof_removals: 1,
            wall_removals: 0,
            surface_passthrough: false,
            geometry_digest: [5; 32],
        };
        let triangles = cap_triangles(&geometry).unwrap();
        assert_eq!(triangles.len(), 2);
        assert_eq!(triangles[0][0], Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(triangles[1][2], Vec3::new(0.0, 1.0, 1.0));
    }

    #[test]
    fn sliced_visual_contract_is_bounded_and_nonzero() {
        assert!(CUTAWAY_VISUAL_RADIUS_V1.is_finite());
        assert!(CUTAWAY_VISUAL_RADIUS_V1 > 0.0);
        assert!(
            CUTAWAY_VISUAL_RADIUS_V1 <= (CUTAWAY_FIXTURE_RADIUS_V1 + 2) as f32,
            "production cutaway mask must remain within the declared fixture neighborhood"
        );
    }

    #[test]
    fn geometry_authority_requires_every_generation_and_camera_field() {
        let geometry = CutawayGeometryV1 {
            presentation_generation: 7,
            terrain_generation: [1; 32],
            terrain_revision: 9,
            camera_token: [2; 32],
            camera_sequence: 1,
            policy_digest: [3; 32],
            terrain_root: [4; 32],
            removed_cells: vec![CellPositionV1::new(0, 0, 1)],
            cap_faces: Vec::new(),
            roof_removals: 1,
            wall_removals: 0,
            surface_passthrough: false,
            geometry_digest: [5; 32],
        };
        assert!(geometry_matches_authority(
            &geometry, 7, [1; 32], 9, [2; 32], 1
        ));
        assert!(!geometry_matches_authority(
            &geometry, 8, [1; 32], 9, [2; 32], 1
        ));
        assert!(!geometry_matches_authority(
            &geometry, 7, [9; 32], 9, [2; 32], 1
        ));
        assert!(!geometry_matches_authority(
            &geometry, 7, [1; 32], 10, [2; 32], 1
        ));
        assert!(!geometry_matches_authority(
            &geometry, 7, [1; 32], 9, [8; 32], 1
        ));
        assert!(!geometry_matches_authority(
            &geometry, 7, [1; 32], 9, [2; 32], 2
        ));
    }

    #[test]
    fn client_interpolation_does_not_recreate_the_stage_receipt() {
        let initial = Vec3::new(10.25, 20.5, 30.75);
        let interpolated = Vec3::new(10.250_1, 20.499_9, 30.750_2);
        let mut state = CutawayFixtureStateV1::default();

        let latched = state.latch_stage_anchor(CutawayCaptureStageV1::Sliced, initial, true);
        state.last_stage = Some(CutawayCaptureStageV1::Sliced);
        let retained = state.latch_stage_anchor(CutawayCaptureStageV1::Sliced, interpolated, false);

        assert_eq!(latched, initial);
        assert_eq!(retained, initial);
        assert_eq!(
            camera_token(retained, CutawayCaptureStageV1::Sliced).unwrap(),
            camera_token(initial, CutawayCaptureStageV1::Sliced).unwrap()
        );
    }

    #[test]
    fn source_or_stage_change_replaces_the_latched_anchor() {
        let initial = Vec3::new(10.25, 20.5, 30.75);
        let replacement = Vec3::new(11.0, 21.0, 31.0);
        let mut state = CutawayFixtureStateV1::default();
        state.latch_stage_anchor(CutawayCaptureStageV1::Surface, initial, true);
        state.last_stage = Some(CutawayCaptureStageV1::Surface);

        let latched = state.latch_stage_anchor(CutawayCaptureStageV1::Sliced, replacement, true);

        assert_eq!(latched, replacement);
    }

    #[test]
    fn authority_diagnostics_identify_the_exact_reset_fields() {
        let geometry = CutawayGeometryV1 {
            presentation_generation: 7,
            terrain_generation: [1; 32],
            terrain_revision: 9,
            camera_token: [2; 32],
            camera_sequence: 1,
            policy_digest: [3; 32],
            terrain_root: [4; 32],
            removed_cells: vec![CellPositionV1::new(0, 0, 1)],
            cap_faces: Vec::new(),
            roof_removals: 1,
            wall_removals: 0,
            surface_passthrough: false,
            geometry_digest: [5; 32],
        };
        assert_eq!(
            authority_change_bits(&geometry, 8, [1; 32], 10, [2; 32], 1),
            AuthorityChangeBitsV1 {
                presentation_generation: true,
                terrain_generation: false,
                terrain_revision: true,
                camera_token: false,
                camera_sequence: false,
            }
        );
    }

    #[test]
    fn transition_diagnostics_are_hard_bounded() {
        let mut state = CutawayFixtureStateV1::default();
        for _ in 0..u16::from(CUTAWAY_DIAGNOSTIC_LIMIT_V1) + 4 {
            emit_transition_diagnostic(
                &mut state,
                CutawayCaptureStageV1::Sliced,
                1,
                AuthorityChangeBitsV1::default(),
                "test",
                false,
                false,
                1,
            );
        }
        assert_eq!(state.diagnostic_emissions, CUTAWAY_DIAGNOSTIC_LIMIT_V1);
    }
}
