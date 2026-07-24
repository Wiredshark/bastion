//! Production compatibility seam for the renderer-owned R1A presentation
//! contract. Capture and measurement stay closed until one exact
//! client-applied generation has a complete terrain/figure/policy resource
//! acknowledgement.

use std::sync::{Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    presentation::{
        PresentationEntityV1, PresentationEnvironmentV1, PresentationErrorV1,
        PresentationFrameDraftV1, PresentationGenerationV1, PresentationHandoffErrorV1,
        PresentationHandoffV1, PresentationReadyTokenV1, PresentationVisualPolicyV1,
        RendererUploadCompletionV1,
    },
};

pub const RESOURCE_COMPLETION_FRAMES_V1: u16 = 240;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionPresentationInputV1 {
    pub simulation_tick: u64,
    pub anchor_uid: u64,
    pub anchor_body: String,
    pub anchor_position_mm: [i64; 3],
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
    /// Generation attached to the completed semantic draw trace. `None`
    /// means the trace predates the active presentation frame.
    pub presentation_generation: Option<u64>,
    pub renderer_maintain_completed: bool,
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
    Hash,
    Frame(PresentationErrorV1),
    Handoff(PresentationHandoffErrorV1),
}

#[derive(Debug, Default)]
struct CompatibilityStateV1 {
    handoff: PresentationHandoffV1,
    next_generation: u64,
    pending_snapshot_root: Option<[u8; 32]>,
    stable_resource_frames: u16,
    ready: Option<PresentationReadyTokenV1>,
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
    state().lock().ok().and_then(|state| state.ready)
}

#[must_use]
pub fn ready_for_capture_measurement() -> bool { ready_token().is_some() }

#[must_use]
pub fn maintain_paused_scene_required() -> bool {
    crate::render::bastion_r0d::capture_config().is_some() && !ready_for_capture_measurement()
}

pub fn observe(
    input: &ProductionPresentationInputV1,
    resources: RendererResourceEvidenceV1,
) -> Result<Option<PresentationReadyTokenV1>, ProductionPresentationErrorV1> {
    let snapshot_root = coherent_snapshot_root(input)?;
    let figure_resource = hash(
        "bastion/r1a/production-figure-resource",
        input.anchor_body.as_bytes(),
    )?;
    let semantic_id = hash(
        "bastion/r1a/production-entity",
        &input.anchor_uid.to_le_bytes(),
    )?;

    let mut state = state().lock().map_err(|_| {
        ProductionPresentationErrorV1::Handoff(PresentationHandoffErrorV1::NoPendingFrame)
    })?;
    if state.ready.is_some() && state.pending_snapshot_root == Some(snapshot_root) {
        return Ok(state.ready);
    }
    if state.pending_snapshot_root != Some(snapshot_root) {
        state.next_generation = state
            .next_generation
            .checked_add(1)
            .ok_or(ProductionPresentationErrorV1::GenerationOverflow)?;
        let frame = build_frame(
            input,
            state.next_generation,
            snapshot_root,
            semantic_id,
            figure_resource,
        )?;
        state
            .handoff
            .stage(frame)
            .map_err(ProductionPresentationErrorV1::Handoff)?;
        state.pending_snapshot_root = Some(snapshot_root);
        state.stable_resource_frames = 0;
        state.ready = None;
    }
    crate::render::bastion_r0d::set_presentation_generation_v1(Some(state.next_generation));

    if resources.presentation_generation == Some(state.next_generation)
        && resources.renderer_maintain_completed
        && resources.terrain_draw_coverage
        && resources.figure_draw_coverage
    {
        state.stable_resource_frames = state.stable_resource_frames.saturating_add(1);
    } else {
        state.stable_resource_frames = 0;
    }
    if state.stable_resource_frames < RESOURCE_COMPLETION_FRAMES_V1 {
        return Ok(None);
    }

    let frame = build_frame(
        input,
        state.next_generation,
        snapshot_root,
        semantic_id,
        figure_resource,
    )?;
    let completion = RendererUploadCompletionV1 {
        client_applied_generation: state.next_generation,
        frame_digest: frame.frame_digest(),
        resource_set_digest: frame.resource_set_digest(),
        completed_resources: frame.renderer_required_resources().to_vec(),
    };
    let token = state
        .handoff
        .acknowledge_uploads(completion)
        .map_err(ProductionPresentationErrorV1::Handoff)?;
    state.ready = Some(token);
    Ok(Some(token))
}

fn build_frame(
    input: &ProductionPresentationInputV1,
    generation: u64,
    snapshot_root: [u8; 32],
    semantic_id: [u8; 32],
    figure_resource: [u8; 32],
) -> Result<bastion_renderer_r0d::presentation::PresentationFrameV1, ProductionPresentationErrorV1>
{
    let state_digest = hash("bastion/r1a/production-entity-state", &snapshot_root)?;
    PresentationFrameDraftV1 {
        generation: PresentationGenerationV1 {
            run_epoch: 1,
            client_applied_generation: generation,
            simulation_tick: input.simulation_tick,
            coherent_snapshot_root: snapshot_root,
        },
        entities: vec![PresentationEntityV1 {
            semantic_id,
            figure_resource,
            group_id: None,
            position_mm: input.anchor_position_mm,
            orientation_q30: [0, 0, 0, 1 << 30],
            scale_milli: 1_000,
            state_tag: 1,
            state_digest,
        }],
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
        renderer_required_resources: vec![
            input.terrain_resource,
            figure_resource,
            input.policy.policy_digest,
        ],
        complete: true,
    }
    .seal()
    .map_err(ProductionPresentationErrorV1::Frame)
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

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn input(tick: u64) -> ProductionPresentationInputV1 {
        ProductionPresentationInputV1 {
            simulation_tick: tick,
            anchor_uid: 2,
            anchor_body: "Humanoid(Dwarf)".to_owned(),
            anchor_position_mm: [1_000, 2_000, 3_000],
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
            renderer_maintain_completed: true,
            terrain_draw_coverage: true,
            figure_draw_coverage: true,
        }
    }

    #[test]
    fn refuses_capture_until_exact_resource_completion_window() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        for _ in 1..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(observe(&input(300), complete()).unwrap(), None);
            assert!(!ready_for_capture_measurement());
        }
        let token = observe(&input(300), complete()).unwrap().unwrap();
        assert_eq!(token.client_applied_generation, 1);
    }

    #[test]
    fn partial_resource_evidence_resets_progress() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        for _ in 0..100 {
            observe(&input(300), complete()).unwrap();
        }
        assert_eq!(
            observe(&input(300), RendererResourceEvidenceV1 {
                figure_draw_coverage: false,
                ..complete()
            })
            .unwrap(),
            None
        );
        for _ in 1..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(observe(&input(300), complete()).unwrap(), None);
        }
        assert!(observe(&input(300), complete()).unwrap().is_some());
    }

    #[test]
    fn newer_snapshot_supersedes_incomplete_generation() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        for _ in 0..100 {
            observe(&input(300), complete()).unwrap();
        }
        let mut newer = input(301);
        newer.anchor_position_mm[0] += 1;
        for _ in 1..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(
                observe(&newer, RendererResourceEvidenceV1 {
                    presentation_generation: Some(2),
                    ..complete()
                })
                .unwrap(),
                None
            );
        }
        assert_eq!(
            observe(&newer, RendererResourceEvidenceV1 {
                presentation_generation: Some(2),
                ..complete()
            })
            .unwrap()
            .unwrap()
            .client_applied_generation,
            2
        );
    }

    #[test]
    fn completed_trace_from_prior_generation_cannot_acknowledge_uploads() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();
        for _ in 0..RESOURCE_COMPLETION_FRAMES_V1 {
            assert_eq!(
                observe(&input(300), RendererResourceEvidenceV1 {
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
            observe(&invalid, complete()),
            Err(ProductionPresentationErrorV1::InvalidAnchor)
        );
        assert!(!ready_for_capture_measurement());
    }
}
