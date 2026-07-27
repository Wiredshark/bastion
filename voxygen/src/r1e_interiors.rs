//! Honest production adapter for renderer-owned interior visibility.
//!
//! `world::site::plot::building::Room` and similar records are generation-time
//! implementation details. They are not transported in the client state, and
//! Voxygen has no authoritative room/portal graph. Until such an authority is
//! published, the adapter binds all consumers to the existing Z-level slice
//! and records the missing capability explicitly.

use std::sync::{Arc, Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    interior::{
        InteriorSourceCapabilityV1, InteriorVisibilityErrorV1, InteriorVisibilityInputV1,
        InteriorVisibilityPublicationV1, InteriorVisibilitySnapshotV1, MAX_INTERIOR_COORDINATE_V1,
    },
    presentation::PresentationFrameV1,
};

pub const SOURCE_CAPABILITY_V1: &str = "Z_LEVEL_ONLY";
pub const UNAVAILABLE_ROOM_AUTHORITY_V1: &str =
    "WORLD_GENERATION_ROOM_RECORDS_NOT_PUBLISHED_TO_CLIENT";
pub const UNAVAILABLE_PORTAL_AUTHORITY_V1: &str = "NO_AUTHORITATIVE_RUNTIME_ROOM_PORTAL_GRAPH";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteriorProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub visibility_sequence: u64,
    pub snapshot_digest: [u8; 32],
    pub source_capability: &'static str,
    pub unavailable_room_authority: &'static str,
    pub unavailable_portal_authority: &'static str,
    pub maximum_visible_z: i32,
    pub room_count: u32,
    pub portal_count: u32,
    pub visible_room_count: u32,
    pub z_level_fallback: bool,
}

#[derive(Debug, Default)]
pub struct InteriorAdapterStateV1 {
    publication: InteriorVisibilityPublicationV1,
    last_key: Option<(u64, [u8; 32], i32)>,
    next_sequence: u64,
}

static LATEST: OnceLock<Mutex<Option<InteriorProductionEvidenceV1>>> = OnceLock::new();

fn latest() -> &'static Mutex<Option<InteriorProductionEvidenceV1>> {
    LATEST.get_or_init(|| Mutex::new(None))
}

pub fn reset() {
    if let Ok(mut value) = latest().lock() {
        *value = None;
    }
}

#[must_use]
pub fn latest_evidence() -> Option<InteriorProductionEvidenceV1> {
    latest().lock().ok().and_then(|value| value.clone())
}

pub fn maintain_z_level_snapshot(
    state: &mut InteriorAdapterStateV1,
    frame: &PresentationFrameV1,
    slice_z: Option<f32>,
) -> Result<Arc<InteriorVisibilitySnapshotV1>, InteriorVisibilityErrorV1> {
    let maximum_visible_z = canonical_slice_z(slice_z)?;
    let generation = frame.generation().client_applied_generation;
    let key = (generation, frame.frame_digest(), maximum_visible_z);
    if state.last_key == Some(key)
        && let Some(snapshot) = state.publication.acquire()
    {
        return Ok(snapshot);
    }
    state.next_sequence = state
        .next_sequence
        .checked_add(1)
        .ok_or(InteriorVisibilityErrorV1::SizeOverflow)?;
    let policy_digest = z_level_policy_digest(frame, maximum_visible_z)?;
    let snapshot = InteriorVisibilitySnapshotV1::seal(InteriorVisibilityInputV1 {
        presentation_generation: generation,
        visibility_sequence: state.next_sequence,
        presentation_frame_digest: frame.frame_digest(),
        terrain_generation: frame.environment().terrain_root,
        // The coherent client snapshot includes the fixed-point camera input.
        camera_token: frame.generation().coherent_snapshot_root,
        cutaway_policy_digest: policy_digest,
        source_capability: InteriorSourceCapabilityV1::ZLevelOnly,
        selected_room: None,
        occupied_room: None,
        maximum_visible_z,
        rooms: Vec::new(),
        portals: Vec::new(),
    })?;
    let published = state.publication.publish(snapshot)?;
    state.last_key = Some(key);
    let evidence = InteriorProductionEvidenceV1 {
        presentation_generation: published.presentation_generation(),
        visibility_sequence: published.visibility_sequence(),
        snapshot_digest: published.snapshot_digest(),
        source_capability: SOURCE_CAPABILITY_V1,
        unavailable_room_authority: UNAVAILABLE_ROOM_AUTHORITY_V1,
        unavailable_portal_authority: UNAVAILABLE_PORTAL_AUTHORITY_V1,
        maximum_visible_z: published.maximum_visible_z(),
        room_count: u32::try_from(published.rooms().len())
            .map_err(|_| InteriorVisibilityErrorV1::SizeOverflow)?,
        portal_count: u32::try_from(published.portals().len())
            .map_err(|_| InteriorVisibilityErrorV1::SizeOverflow)?,
        visible_room_count: u32::try_from(published.visible_rooms().len())
            .map_err(|_| InteriorVisibilityErrorV1::SizeOverflow)?,
        z_level_fallback: true,
    };
    if let Ok(mut value) = latest().lock() {
        *value = Some(evidence);
    }
    Ok(published)
}

fn canonical_slice_z(value: Option<f32>) -> Result<i32, InteriorVisibilityErrorV1> {
    match value {
        None => Ok(MAX_INTERIOR_COORDINATE_V1),
        Some(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= -(MAX_INTERIOR_COORDINATE_V1 as f32)
                && value <= MAX_INTERIOR_COORDINATE_V1 as f32 =>
        {
            Ok(value as i32)
        },
        Some(_) => Err(InteriorVisibilityErrorV1::InvalidBounds),
    }
}

fn z_level_policy_digest(
    frame: &PresentationFrameV1,
    maximum_visible_z: i32,
) -> Result<[u8; 32], InteriorVisibilityErrorV1> {
    let mut bytes = Vec::with_capacity(76);
    bytes.extend_from_slice(&frame.generation().client_applied_generation.to_le_bytes());
    bytes.extend_from_slice(&frame.frame_digest());
    bytes.extend_from_slice(&frame.environment().terrain_root);
    bytes.extend_from_slice(&maximum_visible_z.to_le_bytes());
    domain_hash_v1("bastion/r1e/z-level-only", 1, 0, &bytes)
        .map_err(InteriorVisibilityErrorV1::Hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::presentation::{
        PresentationEnvironmentV1, PresentationFrameDraftV1, PresentationGenerationV1,
        PresentationVisualPolicyV1,
    };

    fn digest(value: u8) -> [u8; 32] { [value; 32] }

    fn frame(generation: u64) -> PresentationFrameV1 {
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: generation,
                simulation_tick: 300,
                coherent_snapshot_root: digest(1),
            },
            entities: Vec::new(),
            groups: Vec::new(),
            events: Vec::new(),
            environment: PresentationEnvironmentV1 {
                terrain_root: digest(2),
                environment_digest: digest(3),
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 1_000,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: digest(4),
                terrain_view_distance: 64,
                entity_view_distance: 64,
                figure_lod_distance: 32,
                sprite_distance: 32,
                particles_enabled: false,
                weapon_trails_enabled: false,
                flashing_lights_enabled: false,
            },
            renderer_required_resources: vec![digest(5)],
            complete: true,
        }
        .seal()
        .unwrap()
    }

    #[test]
    fn production_capability_is_truthfully_z_level_only() {
        let mut state = InteriorAdapterStateV1::default();
        let snapshot = maintain_z_level_snapshot(&mut state, &frame(7), Some(42.0)).unwrap();
        assert_eq!(
            snapshot.source_capability(),
            InteriorSourceCapabilityV1::ZLevelOnly
        );
        assert_eq!(snapshot.maximum_visible_z(), 42);
        assert!(snapshot.rooms().is_empty());
        assert!(snapshot.portals().is_empty());
        let evidence = latest_evidence().unwrap();
        assert_eq!(evidence.source_capability, "Z_LEVEL_ONLY");
        assert!(evidence.z_level_fallback);
    }

    #[test]
    fn unchanged_input_reuses_the_same_whole_snapshot() {
        let mut state = InteriorAdapterStateV1::default();
        let frame = frame(7);
        let first = maintain_z_level_snapshot(&mut state, &frame, Some(42.0)).unwrap();
        let second = maintain_z_level_snapshot(&mut state, &frame, Some(42.0)).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.visibility_sequence(), 1);
    }

    #[test]
    fn slice_change_publishes_a_new_sequence_without_mixing_frames() {
        let mut state = InteriorAdapterStateV1::default();
        let frame = frame(7);
        let first = maintain_z_level_snapshot(&mut state, &frame, Some(42.0)).unwrap();
        let second = maintain_z_level_snapshot(&mut state, &frame, Some(41.0)).unwrap();
        assert_eq!(second.presentation_generation(), 7);
        assert_eq!(second.visibility_sequence(), 2);
        assert_ne!(first.snapshot_digest(), second.snapshot_digest());
        assert_eq!(first.maximum_visible_z(), 42);
    }

    #[test]
    fn malformed_fractional_or_nonfinite_slice_fails_closed() {
        let mut state = InteriorAdapterStateV1::default();
        let frame = frame(7);
        for slice in [Some(1.5), Some(f32::NAN), Some(f32::INFINITY)] {
            assert!(matches!(
                maintain_z_level_snapshot(&mut state, &frame, slice),
                Err(InteriorVisibilityErrorV1::InvalidBounds)
            ));
        }
    }
}
