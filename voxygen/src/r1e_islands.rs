//! Production adapter for renderer-owned moving local spaces.
//!
//! Veloren currently publishes authoritative entity-volume mounting
//! relationships (`VolumeMounting` / `VolumeRiders`) and parent `Pos` / `Ori`.
//! It does not publish general room, ship-interior, or portal-cell authority.
//! This adapter consumes only visible entity-volume membership and records the
//! absent portal capability explicitly.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, OnceLock},
};

use crate::r1a_presentation::ProductionRenderIslandInputV1;
use bastion_renderer_r0d::{
    domain_hash_v1,
    island::{
        RenderIslandErrorV1, RenderIslandInputV1, RenderIslandNodeV1, RenderIslandPublicationV1,
        RenderIslandV1,
    },
    presentation::PresentationFrameV1,
};

pub const SOURCE_CAPABILITY_V1: &str = "ENTITY_VOLUME_MOUNTING";
pub const UNAVAILABLE_PORTAL_AUTHORITY_V1: &str =
    "NO_AUTHORITATIVE_RUNTIME_LOCAL_SPACE_PORTAL_CELLS";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IslandProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub island_generation: u64,
    pub publication_sequence: u64,
    pub snapshot_digest: [u8; 32],
    pub source_capability: &'static str,
    pub unavailable_portal_authority: &'static str,
    pub island_count: u32,
    pub member_count: u32,
    pub portal_count: u32,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionIslandErrorV1 {
    GenerationMismatch,
    InvalidAuthorityInput,
    UnsupportedPortalAuthority,
    SizeOverflow,
    Hash,
    Island(RenderIslandErrorV1),
}

#[derive(Debug, Default)]
pub struct IslandAdapterStateV1 {
    publication: RenderIslandPublicationV1,
    last_source_digest: Option<[u8; 32]>,
    next_sequence: u64,
}

static LATEST: OnceLock<Mutex<Option<IslandProductionEvidenceV1>>> = OnceLock::new();

#[cfg(test)]
static TEST_LOCK_V1: Mutex<()> = Mutex::new(());

fn latest() -> &'static Mutex<Option<IslandProductionEvidenceV1>> {
    LATEST.get_or_init(|| Mutex::new(None))
}

pub fn reset() {
    if let Ok(mut value) = latest().lock() {
        *value = None;
    }
}

#[must_use]
pub fn latest_evidence() -> Option<IslandProductionEvidenceV1> {
    latest().lock().ok().and_then(|value| value.clone())
}

pub fn maintain_snapshot(
    state: &mut IslandAdapterStateV1,
    frame: &PresentationFrameV1,
    interior: &crate::r1e_interiors::InteriorProductionEvidenceV1,
    mut inputs: Vec<ProductionRenderIslandInputV1>,
) -> Result<Arc<RenderIslandV1>, ProductionIslandErrorV1> {
    let generation = frame.generation().client_applied_generation;
    if interior.presentation_generation != generation {
        return Err(ProductionIslandErrorV1::GenerationMismatch);
    }
    inputs.sort_unstable_by_key(|input| input.parent_uid);
    validate_inputs(frame, &inputs)?;
    let source_digest = source_digest(frame, interior, &inputs)?;
    if state.last_source_digest == Some(source_digest)
        && let Some(snapshot) = state.publication.acquire()
    {
        return Ok(snapshot);
    }

    state.next_sequence = state
        .next_sequence
        .checked_add(1)
        .ok_or(ProductionIslandErrorV1::SizeOverflow)?;
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(inputs.len())
        .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?;
    for input in &inputs {
        let semantic_id = island_semantic_id(input.parent_uid)?;
        let parent_island = input
            .parent_island_uid
            .map(island_semantic_id)
            .transpose()?;
        let mut member_ids = Vec::new();
        member_ids
            .try_reserve_exact(input.member_uids.len())
            .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?;
        for uid in &input.member_uids {
            member_ids.push(
                crate::r1a_presentation::production_entity_semantic_id(*uid)
                    .map_err(|_| ProductionIslandErrorV1::Hash)?,
            );
        }
        nodes.push(RenderIslandNodeV1 {
            semantic_id,
            parent_island,
            parent_transform: input.parent_transform,
            member_ids,
            portal_cells: input.portal_cells.clone(),
        });
    }
    let snapshot = RenderIslandV1::seal(RenderIslandInputV1 {
        presentation_generation: generation,
        island_generation: generation,
        publication_sequence: state.next_sequence,
        presentation_frame_digest: frame.frame_digest(),
        interior_snapshot_digest: interior.snapshot_digest,
        cutaway_policy_digest: interior.cutaway_policy_digest,
        nodes,
        complete: true,
    })
    .map_err(ProductionIslandErrorV1::Island)?;
    let snapshot = state
        .publication
        .publish(snapshot)
        .map_err(ProductionIslandErrorV1::Island)?;
    state.last_source_digest = Some(source_digest);
    let member_count = snapshot.nodes().iter().try_fold(0_usize, |total, node| {
        total
            .checked_add(node.member_ids.len())
            .ok_or(ProductionIslandErrorV1::SizeOverflow)
    })?;
    let portal_count = snapshot.nodes().iter().try_fold(0_usize, |total, node| {
        total
            .checked_add(node.portal_cells.len())
            .ok_or(ProductionIslandErrorV1::SizeOverflow)
    })?;
    let evidence = IslandProductionEvidenceV1 {
        presentation_generation: snapshot.presentation_generation(),
        island_generation: snapshot.island_generation(),
        publication_sequence: snapshot.publication_sequence(),
        snapshot_digest: snapshot.snapshot_digest(),
        source_capability: SOURCE_CAPABILITY_V1,
        unavailable_portal_authority: UNAVAILABLE_PORTAL_AUTHORITY_V1,
        island_count: u32::try_from(snapshot.nodes().len())
            .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?,
        member_count: u32::try_from(member_count)
            .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?,
        portal_count: u32::try_from(portal_count)
            .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?,
        active: !snapshot.nodes().is_empty(),
    };
    if let Ok(mut value) = latest().lock() {
        *value = Some(evidence);
    }
    Ok(snapshot)
}

fn validate_inputs(
    frame: &PresentationFrameV1,
    inputs: &[ProductionRenderIslandInputV1],
) -> Result<(), ProductionIslandErrorV1> {
    let frame_members = frame
        .entities()
        .iter()
        .map(|entity| entity.semantic_id)
        .collect::<BTreeSet<_>>();
    let mut parents = BTreeSet::new();
    let mut members = BTreeSet::new();
    for input in inputs {
        if input.parent_uid == 0
            || !parents.insert(input.parent_uid)
            || input
                .parent_island_uid
                .is_some_and(|parent| parent == 0 || parent == input.parent_uid)
            || input.member_uids.is_empty()
            || !input.portal_cells.is_empty()
        {
            return if input.portal_cells.is_empty() {
                Err(ProductionIslandErrorV1::InvalidAuthorityInput)
            } else {
                Err(ProductionIslandErrorV1::UnsupportedPortalAuthority)
            };
        }
        let mut canonical_members = input.member_uids.clone();
        canonical_members.sort_unstable();
        if canonical_members != input.member_uids
            || canonical_members.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(ProductionIslandErrorV1::InvalidAuthorityInput);
        }
        for uid in &input.member_uids {
            if *uid == 0 || !members.insert(*uid) {
                return Err(ProductionIslandErrorV1::InvalidAuthorityInput);
            }
            let semantic_id = crate::r1a_presentation::production_entity_semantic_id(*uid)
                .map_err(|_| ProductionIslandErrorV1::Hash)?;
            if !frame_members.contains(&semantic_id) {
                return Err(ProductionIslandErrorV1::InvalidAuthorityInput);
            }
        }
    }
    Ok(())
}

fn source_digest(
    frame: &PresentationFrameV1,
    interior: &crate::r1e_interiors::InteriorProductionEvidenceV1,
    inputs: &[ProductionRenderIslandInputV1],
) -> Result<[u8; 32], ProductionIslandErrorV1> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&frame.frame_digest());
    bytes.extend_from_slice(&interior.snapshot_digest);
    bytes.extend_from_slice(&interior.cutaway_policy_digest);
    bytes.extend_from_slice(
        &u64::try_from(inputs.len())
            .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?
            .to_le_bytes(),
    );
    for input in inputs {
        bytes.extend_from_slice(&input.parent_uid.to_le_bytes());
        match input.parent_island_uid {
            None => bytes.push(0),
            Some(parent) => {
                bytes.push(1);
                bytes.extend_from_slice(&parent.to_le_bytes());
            },
        }
        for value in input.parent_transform.translation_mm {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in input.parent_transform.orientation_q30 {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&input.parent_transform.scale_milli.to_le_bytes());
        bytes.extend_from_slice(
            &u64::try_from(input.member_uids.len())
                .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
        for member in &input.member_uids {
            bytes.extend_from_slice(&member.to_le_bytes());
        }
        bytes.extend_from_slice(
            &u64::try_from(input.portal_cells.len())
                .map_err(|_| ProductionIslandErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
    }
    domain_hash_v1("bastion/r1e/production-island-source", 1, 0, &bytes)
        .map_err(|_| ProductionIslandErrorV1::Hash)
}

fn island_semantic_id(uid: u64) -> Result<[u8; 32], ProductionIslandErrorV1> {
    if uid == 0 {
        return Err(ProductionIslandErrorV1::InvalidAuthorityInput);
    }
    domain_hash_v1(
        "bastion/r1e/production-render-island",
        1,
        0,
        &uid.to_le_bytes(),
    )
    .map_err(|_| ProductionIslandErrorV1::Hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        island::{Q30_ONE_V1, RenderIslandTransformV1},
        presentation::{
            PresentationEntityV1, PresentationEnvironmentV1, PresentationFrameDraftV1,
            PresentationGenerationV1, PresentationVisualPolicyV1,
        },
    };

    fn digest(value: u8) -> [u8; 32] { [value; 32] }

    fn frame(generation: u64) -> PresentationFrameV1 {
        let semantic_id = crate::r1a_presentation::production_entity_semantic_id(7).unwrap();
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: generation,
                simulation_tick: 300,
                coherent_snapshot_root: digest(1),
            },
            entities: vec![PresentationEntityV1 {
                semantic_id,
                figure_resource: digest(5),
                group_id: None,
                position_mm: [1, 2, 3],
                orientation_q30: [0, 0, 0, Q30_ONE_V1],
                scale_milli: 1_000,
                state_tag: 1,
                state_digest: digest(6),
            }],
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

    fn interior(generation: u64) -> crate::r1e_interiors::InteriorProductionEvidenceV1 {
        crate::r1e_interiors::InteriorProductionEvidenceV1 {
            presentation_generation: generation,
            visibility_sequence: 1,
            snapshot_digest: digest(8),
            cutaway_policy_digest: digest(9),
            source_capability: crate::r1e_interiors::SOURCE_CAPABILITY_V1,
            unavailable_room_authority: crate::r1e_interiors::UNAVAILABLE_ROOM_AUTHORITY_V1,
            unavailable_portal_authority: crate::r1e_interiors::UNAVAILABLE_PORTAL_AUTHORITY_V1,
            maximum_visible_z: 42,
            room_count: 0,
            portal_count: 0,
            visible_room_count: 0,
            z_level_fallback: true,
        }
    }

    fn authority(x: i64) -> ProductionRenderIslandInputV1 {
        ProductionRenderIslandInputV1 {
            parent_uid: 100,
            parent_island_uid: None,
            parent_transform: RenderIslandTransformV1 {
                translation_mm: [x, 2, 3],
                orientation_q30: [0, 0, 0, Q30_ONE_V1],
                scale_milli: 1_000,
            },
            member_uids: vec![7],
            portal_cells: Vec::new(),
        }
    }

    #[test]
    fn real_entity_volume_capability_publishes_bound_membership() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        let mut state = IslandAdapterStateV1::default();
        let frame = frame(3);
        let snapshot =
            maintain_snapshot(&mut state, &frame, &interior(3), vec![authority(1)]).unwrap();
        assert_eq!(snapshot.nodes().len(), 1);
        assert_eq!(snapshot.nodes()[0].member_ids.len(), 1);
        assert_eq!(snapshot.presentation_frame_digest(), frame.frame_digest());
        let evidence = latest_evidence().unwrap();
        assert_eq!(evidence.source_capability, "ENTITY_VOLUME_MOUNTING");
        assert!(evidence.active);
        assert_eq!(evidence.portal_count, 0);
    }

    #[test]
    fn no_visible_members_is_an_explicit_dormant_snapshot() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        let mut state = IslandAdapterStateV1::default();
        let snapshot = maintain_snapshot(&mut state, &frame(3), &interior(3), Vec::new()).unwrap();
        assert!(snapshot.nodes().is_empty());
        assert!(!latest_evidence().unwrap().active);
    }

    #[test]
    fn movement_changes_snapshot_while_member_selection_stays_stable() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        let mut state = IslandAdapterStateV1::default();
        let frame = frame(3);
        let first =
            maintain_snapshot(&mut state, &frame, &interior(3), vec![authority(1)]).unwrap();
        let second =
            maintain_snapshot(&mut state, &frame, &interior(3), vec![authority(2)]).unwrap();
        let member = crate::r1a_presentation::production_entity_semantic_id(7).unwrap();
        assert_eq!(
            first.island_for_member(member),
            second.island_for_member(member)
        );
        assert_ne!(first.snapshot_digest(), second.snapshot_digest());
    }

    #[test]
    fn stale_generation_duplicate_members_and_portals_fail_closed() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        let mut state = IslandAdapterStateV1::default();
        assert_eq!(
            maintain_snapshot(&mut state, &frame(3), &interior(2), vec![authority(1)]),
            Err(ProductionIslandErrorV1::GenerationMismatch)
        );
        let mut duplicate = authority(1);
        duplicate.member_uids.push(7);
        assert_eq!(
            maintain_snapshot(&mut state, &frame(3), &interior(3), vec![duplicate]),
            Err(ProductionIslandErrorV1::InvalidAuthorityInput)
        );
        let mut portal = authority(1);
        portal.portal_cells.push([1, 2, 3]);
        assert_eq!(
            maintain_snapshot(&mut state, &frame(3), &interior(3), vec![portal]),
            Err(ProductionIslandErrorV1::UnsupportedPortalAuthority)
        );
    }
}
