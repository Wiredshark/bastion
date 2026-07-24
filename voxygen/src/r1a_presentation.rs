//! Production compatibility seam for the renderer-owned R1A presentation
//! contract. Capture and measurement stay closed until one exact
//! client-applied generation has a complete terrain/figure/policy resource
//! acknowledgement.

use std::sync::{Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    figure_gpu::UploadReceiptV1,
    presentation::{
        PresentationEntityV1, PresentationEnvironmentV1, PresentationErrorV1,
        PresentationFrameDraftV1, PresentationFrameV1, PresentationGenerationV1,
        PresentationHandoffErrorV1, PresentationHandoffV1, PresentationReadyTokenV1,
        PresentationVisualPolicyV1,
    },
};

pub const RESOURCE_COMPLETION_FRAMES_V1: u16 = 240;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProductionPresentationEntityInputV1 {
    pub uid: u64,
    pub body: String,
    pub position_mm: [i64; 3],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionPresentationInputV1 {
    pub simulation_tick: u64,
    /// Fixed-point renderer camera authority for deterministic individual tier
    /// selection, sampled with the coherent presentation read.
    pub camera_position_mm: [i64; 3],
    pub anchor_uid: u64,
    pub anchor_body: String,
    pub anchor_position_mm: [i64; 3],
    /// Complete, canonically ordered figure-visible colonist slice. The anchor
    /// must occur exactly once. This is one coherent client-applied read.
    pub entities: Vec<ProductionPresentationEntityInputV1>,
    pub terrain_resource: [u8; 32],
    pub environment_digest: [u8; 32],
    pub cloud_milli: u16,
    pub rain_milli: u16,
    pub wind_mm_s: [i32; 2],
    pub daylight_milli: u16,
    pub policy: PresentationVisualPolicyV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RendererResourceEvidenceV1 {
    /// Generation attached to the diagnostic semantic draw trace. This can
    /// establish visible-scene settling only; it is never upload authority.
    pub presentation_generation: Option<u64>,
    pub terrain_draw_coverage: bool,
    pub figure_draw_coverage: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionPresentationErrorV1 {
    InvalidAnchor,
    InvalidBody,
    InvalidPosition,
    InvalidResource,
    GenerationOverflow,
    NoPendingFrame,
    UploadReceipt,
    Hash,
    Frame(PresentationErrorV1),
    Handoff(PresentationHandoffErrorV1),
}

#[derive(Debug, Default)]
struct CompatibilityStateV1 {
    handoff: PresentationHandoffV1,
    next_generation: u64,
    pending_snapshot_root: Option<[u8; 32]>,
    pending_frame: Option<PresentationFrameV1>,
    stable_resource_frames: u16,
    upload_ready: Option<PresentationReadyTokenV1>,
}

static STATE: OnceLock<Mutex<CompatibilityStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<CompatibilityStateV1> {
    STATE.get_or_init(|| Mutex::new(CompatibilityStateV1::default()))
}

pub fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = CompatibilityStateV1::default();
    }
    crate::render::bastion_r0d::set_presentation_generation_v1(None);
}

#[must_use]
pub fn ready_token() -> Option<PresentationReadyTokenV1> {
    state().lock().ok().and_then(|state| {
        (state.stable_resource_frames >= RESOURCE_COMPLETION_FRAMES_V1)
            .then_some(state.upload_ready)
            .flatten()
    })
}

/// Exact presentation/package/upload completion authority for renderer work.
/// Settled-scene capture readiness is intentionally a later, separate gate.
#[must_use]
pub(crate) fn upload_complete_token() -> Option<PresentationReadyTokenV1> {
    state().lock().ok().and_then(|state| state.upload_ready)
}

#[must_use]
pub fn ready_for_capture_measurement() -> bool { ready_token().is_some() }

#[must_use]
pub fn maintain_paused_scene_required() -> bool {
    crate::render::bastion_r0d::capture_config().is_some() && !ready_for_capture_measurement()
}

#[must_use]
pub fn upload_required(frame: &PresentationFrameV1) -> bool {
    state().lock().is_ok_and(|state| {
        state.upload_ready.is_none()
            && state
                .pending_frame
                .as_ref()
                .is_some_and(|pending| pending.frame_digest() == frame.frame_digest())
    })
}

pub fn prepare_frame(
    input: &ProductionPresentationInputV1,
    package_digest: [u8; 32],
) -> Result<PresentationFrameV1, ProductionPresentationErrorV1> {
    let snapshot_root = coherent_snapshot_root(input)?;
    if package_digest == [0; 32] {
        return Err(ProductionPresentationErrorV1::InvalidResource);
    }
    let mut state = state().lock().map_err(|_| {
        ProductionPresentationErrorV1::Handoff(PresentationHandoffErrorV1::NoPendingFrame)
    })?;
    if state.pending_snapshot_root == Some(snapshot_root)
        && let Some(frame) = &state.pending_frame
        && frame.renderer_required_resources() == [package_digest]
    {
        return Ok(frame.clone());
    }
    state.next_generation = state
        .next_generation
        .checked_add(1)
        .ok_or(ProductionPresentationErrorV1::GenerationOverflow)?;
    let frame = build_frame(input, state.next_generation, snapshot_root, package_digest)?;
    state
        .handoff
        .stage(frame.clone())
        .map_err(ProductionPresentationErrorV1::Handoff)?;
    state.pending_snapshot_root = Some(snapshot_root);
    state.pending_frame = Some(frame.clone());
    state.stable_resource_frames = 0;
    state.upload_ready = None;
    crate::render::bastion_r0d::set_presentation_generation_v1(Some(state.next_generation));
    Ok(frame)
}

pub fn acknowledge_upload(
    receipt: &UploadReceiptV1,
) -> Result<PresentationReadyTokenV1, ProductionPresentationErrorV1> {
    let mut state = state().lock().map_err(|_| {
        ProductionPresentationErrorV1::Handoff(PresentationHandoffErrorV1::NoPendingFrame)
    })?;
    let frame = state
        .pending_frame
        .as_ref()
        .ok_or(ProductionPresentationErrorV1::NoPendingFrame)?;
    let completion = receipt
        .to_renderer_completion(frame)
        .map_err(|_| ProductionPresentationErrorV1::UploadReceipt)?;
    let token = state
        .handoff
        .acknowledge_uploads(completion)
        .map_err(ProductionPresentationErrorV1::Handoff)?;
    state.upload_ready = Some(token);
    Ok(token)
}

pub fn observe_visible_scene(
    resources: RendererResourceEvidenceV1,
) -> Result<Option<PresentationReadyTokenV1>, ProductionPresentationErrorV1> {
    let mut state = state().lock().map_err(|_| {
        ProductionPresentationErrorV1::Handoff(PresentationHandoffErrorV1::NoPendingFrame)
    })?;
    if resources.presentation_generation == Some(state.next_generation)
        && resources.terrain_draw_coverage
        && resources.figure_draw_coverage
        && state.upload_ready.is_some()
    {
        state.stable_resource_frames = state
            .stable_resource_frames
            .checked_add(1)
            .ok_or(ProductionPresentationErrorV1::GenerationOverflow)?
            .min(RESOURCE_COMPLETION_FRAMES_V1);
    } else {
        state.stable_resource_frames = 0;
    }
    if state.stable_resource_frames < RESOURCE_COMPLETION_FRAMES_V1 {
        return Ok(None);
    }
    Ok(state.upload_ready)
}

fn build_frame(
    input: &ProductionPresentationInputV1,
    generation: u64,
    snapshot_root: [u8; 32],
    figure_resource: [u8; 32],
) -> Result<bastion_renderer_r0d::presentation::PresentationFrameV1, ProductionPresentationErrorV1>
{
    let mut entities = Vec::with_capacity(input.entities.len());
    for entity in &input.entities {
        let semantic_id = production_entity_semantic_id(entity.uid)?;
        let mut state_bytes = Vec::with_capacity(32 + 8 + 24 + entity.body.len());
        state_bytes.extend_from_slice(&snapshot_root);
        state_bytes.extend_from_slice(&entity.uid.to_le_bytes());
        for component in entity.position_mm {
            state_bytes.extend_from_slice(&component.to_le_bytes());
        }
        state_bytes.extend_from_slice(entity.body.as_bytes());
        let state_digest = hash("bastion/r1a/production-entity-state", &state_bytes)?;
        entities.push(PresentationEntityV1 {
            semantic_id,
            figure_resource,
            group_id: None,
            position_mm: entity.position_mm,
            orientation_q30: [0, 0, 0, 1 << 30],
            scale_milli: 1_000,
            state_tag: 1,
            state_digest,
        });
    }
    PresentationFrameDraftV1 {
        generation: PresentationGenerationV1 {
            run_epoch: 1,
            client_applied_generation: generation,
            simulation_tick: input.simulation_tick,
            coherent_snapshot_root: snapshot_root,
        },
        entities,
        groups: Vec::new(),
        events: Vec::new(),
        environment: PresentationEnvironmentV1 {
            terrain_root: input.terrain_resource,
            environment_digest: input.environment_digest,
            cloud_milli: input.cloud_milli,
            rain_milli: input.rain_milli,
            wind_mm_s: input.wind_mm_s,
            daylight_milli: input.daylight_milli,
        },
        visual_policy: input.policy,
        // Terrain/environment/policy remain bound in the complete frame
        // digest. The only package/upload resource in this first modular
        // figure cutover is the exact accepted figure package.
        renderer_required_resources: vec![figure_resource],
        complete: true,
    }
    .seal()
    .map_err(ProductionPresentationErrorV1::Frame)
}

pub(crate) fn production_entity_semantic_id(
    uid: u64,
) -> Result<[u8; 32], ProductionPresentationErrorV1> {
    hash("bastion/r1a/production-entity", &uid.to_le_bytes())
}

fn coherent_snapshot_root(
    input: &ProductionPresentationInputV1,
) -> Result<[u8; 32], ProductionPresentationErrorV1> {
    if input.anchor_uid == 0 {
        return Err(ProductionPresentationErrorV1::InvalidAnchor);
    }
    if input.anchor_body.is_empty() || input.anchor_body.len() > 4_096 {
        return Err(ProductionPresentationErrorV1::InvalidBody);
    }
    const LIMIT: i64 = 9_000_000_000_000;
    if input
        .anchor_position_mm
        .iter()
        .any(|value| *value < -LIMIT || *value > LIMIT)
    {
        return Err(ProductionPresentationErrorV1::InvalidPosition);
    }
    if input.terrain_resource == [0; 32] || input.environment_digest == [0; 32] {
        return Err(ProductionPresentationErrorV1::InvalidResource);
    }
    if input.entities.is_empty()
        || input.entities.len() > bastion_renderer_r0d::presentation::MAX_PRESENTATION_ENTITIES_V1
    {
        return Err(ProductionPresentationErrorV1::InvalidAnchor);
    }
    let mut entities = input.entities.clone();
    entities.sort();
    if entities != input.entities
        || entities.windows(2).any(|pair| pair[0].uid == pair[1].uid)
        || entities
            .iter()
            .filter(|entity| entity.uid == input.anchor_uid)
            .count()
            != 1
        || entities.iter().any(|entity| {
            entity.uid == 0
                || entity.body.is_empty()
                || entity.body.len() > 4_096
                || entity
                    .position_mm
                    .iter()
                    .any(|value| *value < -LIMIT || *value > LIMIT)
        })
    {
        return Err(ProductionPresentationErrorV1::InvalidAnchor);
    }
    let body_len =
        u64::try_from(input.anchor_body.len()).map_err(|_| ProductionPresentationErrorV1::Hash)?;
    let mut payload = Vec::with_capacity(8 + 8 + 24 + input.anchor_body.len() + 96);
    payload.extend_from_slice(&input.simulation_tick.to_le_bytes());
    payload.extend_from_slice(&input.anchor_uid.to_le_bytes());
    for component in input.anchor_position_mm {
        payload.extend_from_slice(&component.to_le_bytes());
    }
    payload.extend_from_slice(&body_len.to_le_bytes());
    payload.extend_from_slice(input.anchor_body.as_bytes());
    payload.extend_from_slice(
        &u64::try_from(input.entities.len())
            .map_err(|_| ProductionPresentationErrorV1::Hash)?
            .to_le_bytes(),
    );
    for entity in &input.entities {
        payload.extend_from_slice(&entity.uid.to_le_bytes());
        payload.extend_from_slice(
            &u64::try_from(entity.body.len())
                .map_err(|_| ProductionPresentationErrorV1::Hash)?
                .to_le_bytes(),
        );
        payload.extend_from_slice(entity.body.as_bytes());
        for component in entity.position_mm {
            payload.extend_from_slice(&component.to_le_bytes());
        }
    }
    payload.extend_from_slice(&input.terrain_resource);
    payload.extend_from_slice(&input.environment_digest);
    payload.extend_from_slice(&input.policy.policy_digest);
    hash("bastion/r1a/production-snapshot", &payload)
}

fn hash(domain: &str, payload: &[u8]) -> Result<[u8; 32], ProductionPresentationErrorV1> {
    domain_hash_v1(domain, 1, 0, payload).map_err(|_| ProductionPresentationErrorV1::Hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        figure_asset::{
            CachePublicationRecordV1, CachePublicationTerminalV1, CompiledFigurePackageV1,
            FigureAssetRoleV1, FigurePackageTargetV1, FigureSourceInputV1, MaterialBindingV1,
            MaterialKindV1, PackageReceiptV1,
        },
        figure_gpu::{
            BackendCompletionV1, FigureGpuBoneV1, FigureGpuEntityInputV1, FigureGpuPoolConfigV1,
            FigureGpuPoolV1, SubmissionIdentityV1, UploadReceiptV1,
        },
    };

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn input(tick: u64) -> ProductionPresentationInputV1 {
        ProductionPresentationInputV1 {
            simulation_tick: tick,
            camera_position_mm: [0, 0, 2_000],
            anchor_uid: 2,
            anchor_body: "Humanoid(Dwarf)".to_owned(),
            anchor_position_mm: [1_000, 2_000, 3_000],
            entities: vec![ProductionPresentationEntityInputV1 {
                uid: 2,
                body: "Humanoid(Dwarf)".to_owned(),
                position_mm: [1_000, 2_000, 3_000],
            }],
            terrain_resource: digest(1),
            environment_digest: digest(2),
            cloud_milli: 100,
            rain_milli: 0,
            wind_mm_s: [0, 0],
            daylight_milli: 800,
            policy: PresentationVisualPolicyV1 {
                policy_digest: digest(3),
                terrain_view_distance: 16,
                entity_view_distance: 16,
                figure_lod_distance: 350,
                sprite_distance: 250,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: false,
            },
        }
    }

    fn complete() -> RendererResourceEvidenceV1 {
        RendererResourceEvidenceV1 {
            presentation_generation: Some(1),
            terrain_draw_coverage: true,
            figure_draw_coverage: true,
        }
    }

    fn package() -> CompiledFigurePackageV1 {
        CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            digest(10),
            digest(11),
            vec![MaterialBindingV1 {
                slot: 1,
                kind: MaterialKindV1::OpaqueVoxel,
                base_color_rgba: [255; 4],
                flags: 0,
            }],
            vec![FigureSourceInputV1 {
                logical_path: "fixture/body.vox".to_owned(),
                role: FigureAssetRoleV1::CoreBody,
                material_slot: 1,
                bytes: b"body".to_vec(),
                deterministic_fixture: false,
            }],
        )
        .unwrap()
    }

    fn upload_for(
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
    ) -> UploadReceiptV1 {
        let package_receipt =
            PackageReceiptV1::from_publication(frame, package, &CachePublicationRecordV1 {
                authority_digest: package.authority_digest(),
                package_digest: package.package_digest(),
                terminal: CachePublicationTerminalV1::Published,
            })
            .unwrap();
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let entity = &frame.entities()[0];
        let staged = pool
            .begin_generation(frame, package, &package_receipt, vec![
                FigureGpuEntityInputV1 {
                    generation: frame.generation().client_applied_generation,
                    semantic_entity: entity.semantic_id,
                    package_digest: package.package_digest(),
                    authority_digest: package.authority_digest(),
                    composition_digest: digest(20),
                    palette_digest: digest(21),
                    transform_digest: digest(22),
                    pose_digest: digest(23),
                    lod_level: 0,
                    section_id: 1,
                    material_id: 1,
                    flags: 0,
                    bones: vec![FigureGpuBoneV1 {
                        matrix_q20: [1 << 20; 12],
                    }],
                },
            ])
            .unwrap();
        let submission = SubmissionIdentityV1::for_plan(1, &staged.plan).unwrap();
        UploadReceiptV1::from_backend_completion(
            frame,
            package,
            &package_receipt,
            &staged.plan,
            submission,
            BackendCompletionV1::Completed(submission),
        )
        .unwrap()
    }

    #[test]
    fn production_entity_identity_is_the_shared_draw_and_upload_authority() {
        let input = input(300);
        let snapshot_root = coherent_snapshot_root(&input).unwrap();
        let frame = build_frame(&input, 1, snapshot_root, digest(9)).unwrap();
        assert_eq!(
            frame.entities()[0].semantic_id,
            production_entity_semantic_id(input.entities[0].uid).unwrap()
        );
    }

    #[test]
    fn exact_upload_receipt_is_required_before_visible_scene_can_open() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let package = package();
        let frame = prepare_frame(&input(300), package.package_digest()).unwrap();
        for _ in 0..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(observe_visible_scene(complete()).unwrap(), None);
        }
        assert!(!ready_for_capture_measurement());
        assert_eq!(upload_complete_token(), None);
        let upload = upload_for(&frame, &package);
        let token = acknowledge_upload(&upload).unwrap();
        assert_eq!(token.client_applied_generation, 1);
        assert_eq!(upload_complete_token(), Some(token));
        assert!(!ready_for_capture_measurement());
        for _ in 1..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(observe_visible_scene(complete()).unwrap(), None);
        }
        assert_eq!(
            observe_visible_scene(complete())
                .unwrap()
                .unwrap()
                .client_applied_generation,
            1
        );
    }

    #[test]
    fn diagnostic_draw_coverage_only_settles_after_upload_and_resets_on_loss() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let package = package();
        let frame = prepare_frame(&input(300), package.package_digest()).unwrap();
        acknowledge_upload(&upload_for(&frame, &package)).unwrap();
        for _ in 0..100 {
            observe_visible_scene(complete()).unwrap();
        }
        assert_eq!(
            observe_visible_scene(RendererResourceEvidenceV1 {
                figure_draw_coverage: false,
                ..complete()
            })
            .unwrap(),
            None
        );
        for _ in 1..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(observe_visible_scene(complete()).unwrap(), None);
        }
        assert!(observe_visible_scene(complete()).unwrap().is_some());
    }

    #[test]
    fn newer_snapshot_supersedes_the_old_upload_generation() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let package = package();
        let old = prepare_frame(&input(300), package.package_digest()).unwrap();
        let old_upload = upload_for(&old, &package);
        let mut newer = input(301);
        newer.anchor_position_mm[0] += 1;
        let new_frame = prepare_frame(&newer, package.package_digest()).unwrap();
        assert_eq!(
            acknowledge_upload(&old_upload),
            Err(ProductionPresentationErrorV1::UploadReceipt)
        );
        let new_upload = upload_for(&new_frame, &package);
        assert_eq!(
            acknowledge_upload(&new_upload)
                .unwrap()
                .client_applied_generation,
            2
        );
    }

    #[test]
    fn trace_from_prior_generation_is_diagnostic_only_and_cannot_open_capture() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let package = package();
        let frame = prepare_frame(&input(300), package.package_digest()).unwrap();
        acknowledge_upload(&upload_for(&frame, &package)).unwrap();
        for _ in 0..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(
                observe_visible_scene(RendererResourceEvidenceV1 {
                    presentation_generation: None,
                    ..complete()
                })
                .unwrap(),
                None
            );
        }
        assert!(!ready_for_capture_measurement());
    }

    #[test]
    fn malformed_projection_fails_closed() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        let mut invalid = input(300);
        invalid.anchor_uid = 0;
        assert_eq!(
            prepare_frame(&invalid, digest(9)),
            Err(ProductionPresentationErrorV1::InvalidAnchor)
        );
        assert!(!ready_for_capture_measurement());
    }
}
