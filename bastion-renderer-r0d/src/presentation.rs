//! R1A immutable presentation generation and resource-completion handoff.
//!
//! This is the renderer-owned boundary between one coherent client-applied
//! snapshot and renderer-visible state. It contains no clocks, worker IDs,
//! ECS handles, GPU observations, or mutable simulation references.

use std::sync::Arc;

use crate::{
    DomainHashErrorV1, domain_hash_v1,
    tape::{DomainRank, FinalizedTapeV1, TapeError, TapeKeyV1, TapeRecordV1},
};

pub const PRESENTATION_FRAME_VERSION_V1: u16 = 1;
pub const MAX_PRESENTATION_BYTES_V1: usize = 4 * 1024 * 1024;
pub const MAX_PRESENTATION_ENTITIES_V1: usize = 4_096;
pub const MAX_PRESENTATION_GROUPS_V1: usize = 1_024;
pub const MAX_PRESENTATION_EVENTS_V1: usize = 4_096;
pub const MAX_PRESENTATION_RESOURCES_V1: usize = 8_192;
pub const MAX_PRESENTATION_GROUP_MEMBERS_V1: usize = 8_192;
const MAGIC: &[u8; 8] = b"BASR1PF1";
const SEALED_TAG: u8 = 1;

pub type PresentationDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PresentationGenerationV1 {
    pub run_epoch: u64,
    pub client_applied_generation: u64,
    pub simulation_tick: u64,
    pub coherent_snapshot_root: PresentationDigestV1,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PresentationEntityV1 {
    pub semantic_id: PresentationDigestV1,
    pub figure_resource: PresentationDigestV1,
    pub group_id: Option<PresentationDigestV1>,
    pub position_mm: [i64; 3],
    pub orientation_q30: [i32; 4],
    pub scale_milli: u32,
    pub state_tag: u16,
    pub state_digest: PresentationDigestV1,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PresentationGroupV1 {
    pub semantic_id: PresentationDigestV1,
    pub kind_tag: u16,
    pub member_ids: Vec<PresentationDigestV1>,
    pub state_digest: PresentationDigestV1,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PresentationEventV1 {
    pub semantic_id: PresentationDigestV1,
    pub kind_tag: u16,
    pub source_id: PresentationDigestV1,
    pub target_id: PresentationDigestV1,
    pub payload_digest: PresentationDigestV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationEnvironmentV1 {
    pub terrain_root: PresentationDigestV1,
    pub environment_digest: PresentationDigestV1,
    pub cloud_milli: u16,
    pub rain_milli: u16,
    pub wind_mm_s: [i32; 2],
    pub daylight_milli: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationVisualPolicyV1 {
    pub policy_digest: PresentationDigestV1,
    pub terrain_view_distance: u16,
    pub entity_view_distance: u16,
    pub figure_lod_distance: u16,
    pub sprite_distance: u16,
    pub particles_enabled: bool,
    pub weapon_trails_enabled: bool,
    pub flashing_lights_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationFrameDraftV1 {
    pub generation: PresentationGenerationV1,
    pub entities: Vec<PresentationEntityV1>,
    pub groups: Vec<PresentationGroupV1>,
    pub events: Vec<PresentationEventV1>,
    pub environment: PresentationEnvironmentV1,
    pub visual_policy: PresentationVisualPolicyV1,
    pub renderer_required_resources: Vec<PresentationDigestV1>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationFrameV1 {
    generation: PresentationGenerationV1,
    entities: Vec<PresentationEntityV1>,
    groups: Vec<PresentationGroupV1>,
    events: Vec<PresentationEventV1>,
    environment: PresentationEnvironmentV1,
    visual_policy: PresentationVisualPolicyV1,
    renderer_required_resources: Vec<PresentationDigestV1>,
    canonical_bytes: Vec<u8>,
    frame_digest: PresentationDigestV1,
    resource_set_digest: PresentationDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PresentationErrorV1 {
    UnsupportedVersion(u16),
    InvalidMagic,
    UnsealedOrPartial,
    InvalidRunEpoch,
    InvalidGeneration,
    InvalidEntity,
    InvalidGroup,
    InvalidEvent,
    InvalidEnvironment,
    InvalidVisualPolicy,
    TooManyEntities(usize),
    TooManyGroups(usize),
    TooManyEvents(usize),
    TooManyResources(usize),
    TooManyGroupMembers(usize),
    DuplicateEntity(PresentationDigestV1),
    DuplicateGroup(PresentationDigestV1),
    DuplicateEvent(PresentationDigestV1),
    DuplicateResource(PresentationDigestV1),
    DuplicateGroupMember(PresentationDigestV1),
    UnknownGroupMember(PresentationDigestV1),
    EntityGroupMismatch(PresentationDigestV1),
    NonCanonicalOrder,
    EncodedSizeExceeded(usize),
    Truncated,
    TrailingBytes(usize),
    MalformedBoolean(u8),
    MalformedOptionalTag(u8),
    DigestMismatch,
    SizeOverflow,
    AllocationFailure,
    HashFailure(DomainHashErrorV1),
    TapeFailure(TapeError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PresentationHandoffErrorV1 {
    Frame(PresentationErrorV1),
    StaleOrEqualGeneration { current: u64, offered: u64 },
    NoPendingFrame,
    AcknowledgementGenerationMismatch { pending: u64, acknowledged: u64 },
    AcknowledgementFrameMismatch,
    AcknowledgementResourceSetMismatch,
    PartialResourceCompletion { required: usize, completed: usize },
    SupersededGeneration { pending: u64, acknowledged: u64 },
    ConsumerGenerationMismatch { visible: u64, requested: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererUploadCompletionV1 {
    pub client_applied_generation: u64,
    pub frame_digest: PresentationDigestV1,
    pub resource_set_digest: PresentationDigestV1,
    pub completed_resources: Vec<PresentationDigestV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationReadyTokenV1 {
    pub client_applied_generation: u64,
    pub frame_digest: PresentationDigestV1,
    pub resource_set_digest: PresentationDigestV1,
    pub semantic_tape_root: PresentationDigestV1,
}

impl PresentationFrameDraftV1 {
    pub fn seal(mut self) -> Result<PresentationFrameV1, PresentationErrorV1> {
        if !self.complete {
            return Err(PresentationErrorV1::UnsealedOrPartial);
        }
        validate_generation(self.generation)?;
        validate_counts(&self)?;
        validate_environment(self.environment)?;
        validate_policy(self.visual_policy)?;

        for entity in &self.entities {
            validate_entity(entity)?;
        }
        for group in &mut self.groups {
            if group.kind_tag == 0 || is_zero(&group.semantic_id) || is_zero(&group.state_digest) {
                return Err(PresentationErrorV1::InvalidGroup);
            }
            group.member_ids.sort_unstable();
            reject_duplicate(&group.member_ids, PresentationErrorV1::DuplicateGroupMember)?;
        }
        for event in &self.events {
            if event.kind_tag == 0 || is_zero(&event.semantic_id) || is_zero(&event.payload_digest)
            {
                return Err(PresentationErrorV1::InvalidEvent);
            }
        }

        self.entities
            .sort_unstable_by_key(|value| value.semantic_id);
        self.groups.sort_unstable_by_key(|value| value.semantic_id);
        self.events.sort_unstable_by_key(|value| value.semantic_id);
        self.renderer_required_resources.sort_unstable();
        reject_duplicate(
            &self
                .entities
                .iter()
                .map(|value| value.semantic_id)
                .collect::<Vec<_>>(),
            PresentationErrorV1::DuplicateEntity,
        )?;
        reject_duplicate(
            &self
                .groups
                .iter()
                .map(|value| value.semantic_id)
                .collect::<Vec<_>>(),
            PresentationErrorV1::DuplicateGroup,
        )?;
        reject_duplicate(
            &self
                .events
                .iter()
                .map(|value| value.semantic_id)
                .collect::<Vec<_>>(),
            PresentationErrorV1::DuplicateEvent,
        )?;
        reject_duplicate(
            &self.renderer_required_resources,
            PresentationErrorV1::DuplicateResource,
        )?;
        validate_group_links(&self.entities, &self.groups)?;

        let resource_set_digest = resource_set_digest(&self.renderer_required_resources)?;
        let mut frame = PresentationFrameV1 {
            generation: self.generation,
            entities: self.entities,
            groups: self.groups,
            events: self.events,
            environment: self.environment,
            visual_policy: self.visual_policy,
            renderer_required_resources: self.renderer_required_resources,
            canonical_bytes: Vec::new(),
            frame_digest: [0; 32],
            resource_set_digest,
        };
        frame.canonical_bytes = frame.encode()?;
        frame.frame_digest.copy_from_slice(
            frame
                .canonical_bytes
                .get(frame.canonical_bytes.len().saturating_sub(32)..)
                .ok_or(PresentationErrorV1::Truncated)?,
        );
        Ok(frame)
    }
}

impl PresentationFrameV1 {
    #[must_use]
    pub const fn generation(&self) -> PresentationGenerationV1 { self.generation }

    #[must_use]
    pub fn entities(&self) -> &[PresentationEntityV1] { &self.entities }

    #[must_use]
    pub fn groups(&self) -> &[PresentationGroupV1] { &self.groups }

    #[must_use]
    pub fn events(&self) -> &[PresentationEventV1] { &self.events }

    #[must_use]
    pub const fn environment(&self) -> PresentationEnvironmentV1 { self.environment }

    #[must_use]
    pub const fn visual_policy(&self) -> PresentationVisualPolicyV1 { self.visual_policy }

    #[must_use]
    pub fn renderer_required_resources(&self) -> &[PresentationDigestV1] {
        &self.renderer_required_resources
    }

    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] { &self.canonical_bytes }

    #[must_use]
    pub const fn frame_digest(&self) -> PresentationDigestV1 { self.frame_digest }

    #[must_use]
    pub const fn resource_set_digest(&self) -> PresentationDigestV1 { self.resource_set_digest }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, PresentationErrorV1> {
        if bytes.len() > MAX_PRESENTATION_BYTES_V1 {
            return Err(PresentationErrorV1::EncodedSizeExceeded(bytes.len()));
        }
        let mut reader = Reader::new(bytes);
        if reader.take(8)? != MAGIC {
            return Err(PresentationErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != PRESENTATION_FRAME_VERSION_V1 {
            return Err(PresentationErrorV1::UnsupportedVersion(version));
        }
        if reader.u8()? != SEALED_TAG {
            return Err(PresentationErrorV1::UnsealedOrPartial);
        }
        if reader.u8()? != 0 {
            return Err(PresentationErrorV1::NonCanonicalOrder);
        }
        let generation = PresentationGenerationV1 {
            run_epoch: reader.u64()?,
            client_applied_generation: reader.u64()?,
            simulation_tick: reader.u64()?,
            coherent_snapshot_root: reader.digest()?,
        };
        let entity_count = reader.count(MAX_PRESENTATION_ENTITIES_V1)?;
        let group_count = reader.count(MAX_PRESENTATION_GROUPS_V1)?;
        let event_count = reader.count(MAX_PRESENTATION_EVENTS_V1)?;
        let resource_count = reader.count(MAX_PRESENTATION_RESOURCES_V1)?;
        let mut entities = try_vec(entity_count)?;
        for _ in 0..entity_count {
            entities.push(decode_entity(&mut reader)?);
        }
        let mut groups = try_vec(group_count)?;
        for _ in 0..group_count {
            groups.push(decode_group(&mut reader)?);
        }
        let mut events = try_vec(event_count)?;
        for _ in 0..event_count {
            events.push(decode_event(&mut reader)?);
        }
        let environment = decode_environment(&mut reader)?;
        let visual_policy = decode_policy(&mut reader)?;
        let mut resources = try_vec(resource_count)?;
        for _ in 0..resource_count {
            resources.push(reader.digest()?);
        }
        let encoded_resource_digest = reader.digest()?;
        let encoded_frame_digest = reader.digest()?;
        if reader.remaining() != 0 {
            return Err(PresentationErrorV1::TrailingBytes(reader.remaining()));
        }

        let rebuilt = PresentationFrameDraftV1 {
            generation,
            entities,
            groups,
            events,
            environment,
            visual_policy,
            renderer_required_resources: resources,
            complete: true,
        }
        .seal()?;
        if rebuilt.resource_set_digest != encoded_resource_digest
            || rebuilt.frame_digest != encoded_frame_digest
            || rebuilt.canonical_bytes.as_slice() != bytes
        {
            return Err(PresentationErrorV1::DigestMismatch);
        }
        Ok(rebuilt)
    }

    pub fn semantic_tape(&self) -> Result<FinalizedTapeV1, PresentationErrorV1> {
        let records = vec![
            TapeRecordV1 {
                key: TapeKeyV1 {
                    simulation_tick: self.generation.simulation_tick,
                    render_frame_or_zero: 0,
                    domain_rank: DomainRank::ClientApplied as u16,
                    authority_rank: 1,
                    owner_digest: self.generation.coherent_snapshot_root,
                    leaf_kind_rank: 1,
                    local_ordinal: self.generation.client_applied_generation,
                },
                payload: self.frame_digest.to_vec(),
            },
            TapeRecordV1 {
                key: TapeKeyV1 {
                    simulation_tick: self.generation.simulation_tick,
                    render_frame_or_zero: 0,
                    domain_rank: DomainRank::SceneProjection as u16,
                    authority_rank: 1,
                    owner_digest: self.frame_digest,
                    leaf_kind_rank: 2,
                    local_ordinal: self.generation.client_applied_generation,
                },
                payload: self.resource_set_digest.to_vec(),
            },
        ];
        FinalizedTapeV1::finalize(records).map_err(PresentationErrorV1::TapeFailure)
    }

    fn encode(&self) -> Result<Vec<u8>, PresentationErrorV1> {
        let estimated = 256_usize
            .checked_add(
                self.entities
                    .len()
                    .checked_mul(184)
                    .ok_or(PresentationErrorV1::SizeOverflow)?,
            )
            .and_then(|value| value.checked_add(self.events.len().checked_mul(130)?))
            .and_then(|value| {
                value.checked_add(self.renderer_required_resources.len().checked_mul(32)?)
            })
            .ok_or(PresentationErrorV1::SizeOverflow)?;
        if estimated > MAX_PRESENTATION_BYTES_V1 {
            return Err(PresentationErrorV1::EncodedSizeExceeded(estimated));
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(estimated)
            .map_err(|_| PresentationErrorV1::AllocationFailure)?;
        output.extend_from_slice(MAGIC);
        put_u16(&mut output, PRESENTATION_FRAME_VERSION_V1);
        output.push(SEALED_TAG);
        output.push(0);
        put_u64(&mut output, self.generation.run_epoch);
        put_u64(&mut output, self.generation.client_applied_generation);
        put_u64(&mut output, self.generation.simulation_tick);
        output.extend_from_slice(&self.generation.coherent_snapshot_root);
        put_count(&mut output, self.entities.len())?;
        put_count(&mut output, self.groups.len())?;
        put_count(&mut output, self.events.len())?;
        put_count(&mut output, self.renderer_required_resources.len())?;
        for entity in &self.entities {
            encode_entity(entity, &mut output);
        }
        for group in &self.groups {
            encode_group(group, &mut output)?;
        }
        for event in &self.events {
            encode_event(event, &mut output);
        }
        encode_environment(self.environment, &mut output);
        encode_policy(self.visual_policy, &mut output);
        for resource in &self.renderer_required_resources {
            output.extend_from_slice(resource);
        }
        output.extend_from_slice(&self.resource_set_digest);
        let digest = frame_digest(&output)?;
        output.extend_from_slice(&digest);
        if output.len() > MAX_PRESENTATION_BYTES_V1 {
            return Err(PresentationErrorV1::EncodedSizeExceeded(output.len()));
        }
        Ok(output)
    }
}

#[derive(Debug, Default)]
pub struct PresentationHandoffV1 {
    pending: Option<PresentationFrameV1>,
    visible: Option<Arc<PresentationFrameV1>>,
}

impl PresentationHandoffV1 {
    pub fn stage(&mut self, frame: PresentationFrameV1) -> Result<(), PresentationHandoffErrorV1> {
        let offered = frame.generation.client_applied_generation;
        let current = self
            .pending
            .as_ref()
            .map(|value| value.generation.client_applied_generation)
            .into_iter()
            .chain(
                self.visible
                    .as_ref()
                    .map(|value| value.generation.client_applied_generation),
            )
            .max()
            .unwrap_or(0);
        if offered <= current {
            return Err(PresentationHandoffErrorV1::StaleOrEqualGeneration { current, offered });
        }
        self.pending = Some(frame);
        Ok(())
    }

    pub fn acknowledge_uploads(
        &mut self,
        mut completion: RendererUploadCompletionV1,
    ) -> Result<PresentationReadyTokenV1, PresentationHandoffErrorV1> {
        let pending = self
            .pending
            .as_ref()
            .ok_or(PresentationHandoffErrorV1::NoPendingFrame)?;
        let pending_generation = pending.generation.client_applied_generation;
        if completion.client_applied_generation < pending_generation {
            return Err(PresentationHandoffErrorV1::SupersededGeneration {
                pending: pending_generation,
                acknowledged: completion.client_applied_generation,
            });
        }
        if completion.client_applied_generation != pending_generation {
            return Err(
                PresentationHandoffErrorV1::AcknowledgementGenerationMismatch {
                    pending: pending_generation,
                    acknowledged: completion.client_applied_generation,
                },
            );
        }
        if completion.frame_digest != pending.frame_digest {
            return Err(PresentationHandoffErrorV1::AcknowledgementFrameMismatch);
        }
        if completion.resource_set_digest != pending.resource_set_digest {
            return Err(PresentationHandoffErrorV1::AcknowledgementResourceSetMismatch);
        }
        completion.completed_resources.sort_unstable();
        completion.completed_resources.dedup();
        if completion.completed_resources != pending.renderer_required_resources {
            return Err(PresentationHandoffErrorV1::PartialResourceCompletion {
                required: pending.renderer_required_resources.len(),
                completed: completion.completed_resources.len(),
            });
        }
        let semantic_tape_root = pending
            .semantic_tape()
            .map_err(PresentationHandoffErrorV1::Frame)?
            .final_root();
        let token = PresentationReadyTokenV1 {
            client_applied_generation: pending_generation,
            frame_digest: pending.frame_digest,
            resource_set_digest: pending.resource_set_digest,
            semantic_tape_root,
        };
        let ready = self
            .pending
            .take()
            .ok_or(PresentationHandoffErrorV1::NoPendingFrame)?;
        self.visible = Some(Arc::new(ready));
        Ok(token)
    }

    #[must_use]
    pub fn acquire_visible(&self) -> Option<Arc<PresentationFrameV1>> { self.visible.clone() }

    pub fn authorize_consumer(
        &self,
        generation: u64,
    ) -> Result<PresentationReadyTokenV1, PresentationHandoffErrorV1> {
        let visible = self
            .visible
            .as_ref()
            .ok_or(PresentationHandoffErrorV1::NoPendingFrame)?;
        let actual = visible.generation.client_applied_generation;
        if actual != generation {
            return Err(PresentationHandoffErrorV1::ConsumerGenerationMismatch {
                visible: actual,
                requested: generation,
            });
        }
        Ok(PresentationReadyTokenV1 {
            client_applied_generation: actual,
            frame_digest: visible.frame_digest,
            resource_set_digest: visible.resource_set_digest,
            semantic_tape_root: visible
                .semantic_tape()
                .map_err(PresentationHandoffErrorV1::Frame)?
                .final_root(),
        })
    }
}

fn validate_generation(value: PresentationGenerationV1) -> Result<(), PresentationErrorV1> {
    if value.run_epoch == 0 {
        return Err(PresentationErrorV1::InvalidRunEpoch);
    }
    if value.client_applied_generation == 0 || is_zero(&value.coherent_snapshot_root) {
        return Err(PresentationErrorV1::InvalidGeneration);
    }
    Ok(())
}

fn validate_counts(draft: &PresentationFrameDraftV1) -> Result<(), PresentationErrorV1> {
    if draft.entities.len() > MAX_PRESENTATION_ENTITIES_V1 {
        return Err(PresentationErrorV1::TooManyEntities(draft.entities.len()));
    }
    if draft.groups.len() > MAX_PRESENTATION_GROUPS_V1 {
        return Err(PresentationErrorV1::TooManyGroups(draft.groups.len()));
    }
    if draft.events.len() > MAX_PRESENTATION_EVENTS_V1 {
        return Err(PresentationErrorV1::TooManyEvents(draft.events.len()));
    }
    if draft.renderer_required_resources.len() > MAX_PRESENTATION_RESOURCES_V1 {
        return Err(PresentationErrorV1::TooManyResources(
            draft.renderer_required_resources.len(),
        ));
    }
    let members = draft.groups.iter().try_fold(0_usize, |total, group| {
        total
            .checked_add(group.member_ids.len())
            .ok_or(PresentationErrorV1::SizeOverflow)
    })?;
    if members > MAX_PRESENTATION_GROUP_MEMBERS_V1 {
        return Err(PresentationErrorV1::TooManyGroupMembers(members));
    }
    Ok(())
}

fn validate_entity(entity: &PresentationEntityV1) -> Result<(), PresentationErrorV1> {
    const LIMIT: i64 = 9_000_000_000_000;
    const Q30: i32 = 1 << 30;
    if is_zero(&entity.semantic_id)
        || is_zero(&entity.figure_resource)
        || is_zero(&entity.state_digest)
        || entity.state_tag == 0
        || entity.scale_milli == 0
        || entity.scale_milli > 100_000
        || entity
            .position_mm
            .iter()
            .any(|value| *value < -LIMIT || *value > LIMIT)
        || entity
            .orientation_q30
            .iter()
            .any(|value| *value < -Q30 || *value > Q30)
        || entity.orientation_q30 == [0; 4]
    {
        return Err(PresentationErrorV1::InvalidEntity);
    }
    Ok(())
}

fn validate_environment(value: PresentationEnvironmentV1) -> Result<(), PresentationErrorV1> {
    if is_zero(&value.terrain_root)
        || is_zero(&value.environment_digest)
        || value.cloud_milli > 1_000
        || value.rain_milli > 1_000
        || value.daylight_milli > 1_000
        || value.wind_mm_s.iter().any(|value| value.abs() > 1_000_000)
    {
        return Err(PresentationErrorV1::InvalidEnvironment);
    }
    Ok(())
}

fn validate_policy(value: PresentationVisualPolicyV1) -> Result<(), PresentationErrorV1> {
    if is_zero(&value.policy_digest)
        || value.terrain_view_distance == 0
        || value.entity_view_distance == 0
        || value.figure_lod_distance == 0
        || value.sprite_distance == 0
    {
        return Err(PresentationErrorV1::InvalidVisualPolicy);
    }
    Ok(())
}

fn validate_group_links(
    entities: &[PresentationEntityV1],
    groups: &[PresentationGroupV1],
) -> Result<(), PresentationErrorV1> {
    for group in groups {
        for member in &group.member_ids {
            let entity = entities
                .binary_search_by_key(member, |value| value.semantic_id)
                .ok()
                .and_then(|index| entities.get(index))
                .ok_or(PresentationErrorV1::UnknownGroupMember(*member))?;
            if entity.group_id != Some(group.semantic_id) {
                return Err(PresentationErrorV1::EntityGroupMismatch(*member));
            }
        }
    }
    for entity in entities {
        if let Some(group_id) = entity.group_id {
            let group = groups
                .binary_search_by_key(&group_id, |value| value.semantic_id)
                .ok()
                .and_then(|index| groups.get(index))
                .ok_or(PresentationErrorV1::EntityGroupMismatch(entity.semantic_id))?;
            if group.member_ids.binary_search(&entity.semantic_id).is_err() {
                return Err(PresentationErrorV1::EntityGroupMismatch(entity.semantic_id));
            }
        }
    }
    Ok(())
}

fn reject_duplicate(
    values: &[PresentationDigestV1],
    error: fn(PresentationDigestV1) -> PresentationErrorV1,
) -> Result<(), PresentationErrorV1> {
    if let Some(pair) = values.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(error(pair[0]));
    }
    Ok(())
}

fn is_zero(value: &PresentationDigestV1) -> bool { *value == [0; 32] }

fn resource_set_digest(
    resources: &[PresentationDigestV1],
) -> Result<PresentationDigestV1, PresentationErrorV1> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(
            resources
                .len()
                .checked_mul(32)
                .ok_or(PresentationErrorV1::SizeOverflow)?,
        )
        .map_err(|_| PresentationErrorV1::AllocationFailure)?;
    for resource in resources {
        bytes.extend_from_slice(resource);
    }
    domain_hash_v1("bastion/r1a/resource-set", 1, 0, &bytes)
        .map_err(PresentationErrorV1::HashFailure)
}

fn frame_digest(bytes: &[u8]) -> Result<PresentationDigestV1, PresentationErrorV1> {
    domain_hash_v1("bastion/r1a/presentation-frame", 1, 0, bytes)
        .map_err(PresentationErrorV1::HashFailure)
}

fn put_count(output: &mut Vec<u8>, value: usize) -> Result<(), PresentationErrorV1> {
    let value = u32::try_from(value).map_err(|_| PresentationErrorV1::SizeOverflow)?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_u16(output: &mut Vec<u8>, value: u16) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_u32(output: &mut Vec<u8>, value: u32) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_u64(output: &mut Vec<u8>, value: u64) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_i32(output: &mut Vec<u8>, value: i32) { output.extend_from_slice(&value.to_le_bytes()); }
fn put_i64(output: &mut Vec<u8>, value: i64) { output.extend_from_slice(&value.to_le_bytes()); }

fn encode_entity(value: &PresentationEntityV1, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.semantic_id);
    output.extend_from_slice(&value.figure_resource);
    match value.group_id {
        Some(group) => {
            output.push(1);
            output.extend_from_slice(&group);
        },
        None => {
            output.push(0);
            output.extend_from_slice(&[0; 32]);
        },
    }
    for component in value.position_mm {
        put_i64(output, component);
    }
    for component in value.orientation_q30 {
        put_i32(output, component);
    }
    put_u32(output, value.scale_milli);
    put_u16(output, value.state_tag);
    output.extend_from_slice(&value.state_digest);
}

fn encode_group(
    value: &PresentationGroupV1,
    output: &mut Vec<u8>,
) -> Result<(), PresentationErrorV1> {
    output.extend_from_slice(&value.semantic_id);
    put_u16(output, value.kind_tag);
    put_count(output, value.member_ids.len())?;
    for member in &value.member_ids {
        output.extend_from_slice(member);
    }
    output.extend_from_slice(&value.state_digest);
    Ok(())
}

fn encode_event(value: &PresentationEventV1, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.semantic_id);
    put_u16(output, value.kind_tag);
    output.extend_from_slice(&value.source_id);
    output.extend_from_slice(&value.target_id);
    output.extend_from_slice(&value.payload_digest);
}

fn encode_environment(value: PresentationEnvironmentV1, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.terrain_root);
    output.extend_from_slice(&value.environment_digest);
    put_u16(output, value.cloud_milli);
    put_u16(output, value.rain_milli);
    for component in value.wind_mm_s {
        put_i32(output, component);
    }
    put_u16(output, value.daylight_milli);
}

fn encode_policy(value: PresentationVisualPolicyV1, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.policy_digest);
    put_u16(output, value.terrain_view_distance);
    put_u16(output, value.entity_view_distance);
    put_u16(output, value.figure_lod_distance);
    put_u16(output, value.sprite_distance);
    output.push(u8::from(value.particles_enabled));
    output.push(u8::from(value.weapon_trails_enabled));
    output.push(u8::from(value.flashing_lights_enabled));
}

fn decode_entity(reader: &mut Reader<'_>) -> Result<PresentationEntityV1, PresentationErrorV1> {
    let semantic_id = reader.digest()?;
    let figure_resource = reader.digest()?;
    let group_tag = reader.u8()?;
    let group = reader.digest()?;
    let group_id = match group_tag {
        0 if is_zero(&group) => None,
        1 if !is_zero(&group) => Some(group),
        0 | 1 => return Err(PresentationErrorV1::InvalidEntity),
        other => return Err(PresentationErrorV1::MalformedOptionalTag(other)),
    };
    let mut position_mm = [0; 3];
    for component in &mut position_mm {
        *component = reader.i64()?;
    }
    let mut orientation_q30 = [0; 4];
    for component in &mut orientation_q30 {
        *component = reader.i32()?;
    }
    Ok(PresentationEntityV1 {
        semantic_id,
        figure_resource,
        group_id,
        position_mm,
        orientation_q30,
        scale_milli: reader.u32()?,
        state_tag: reader.u16()?,
        state_digest: reader.digest()?,
    })
}

fn decode_group(reader: &mut Reader<'_>) -> Result<PresentationGroupV1, PresentationErrorV1> {
    let semantic_id = reader.digest()?;
    let kind_tag = reader.u16()?;
    let member_count = reader.count(MAX_PRESENTATION_GROUP_MEMBERS_V1)?;
    let mut member_ids = try_vec(member_count)?;
    for _ in 0..member_count {
        member_ids.push(reader.digest()?);
    }
    Ok(PresentationGroupV1 {
        semantic_id,
        kind_tag,
        member_ids,
        state_digest: reader.digest()?,
    })
}

fn decode_event(reader: &mut Reader<'_>) -> Result<PresentationEventV1, PresentationErrorV1> {
    Ok(PresentationEventV1 {
        semantic_id: reader.digest()?,
        kind_tag: reader.u16()?,
        source_id: reader.digest()?,
        target_id: reader.digest()?,
        payload_digest: reader.digest()?,
    })
}

fn decode_environment(
    reader: &mut Reader<'_>,
) -> Result<PresentationEnvironmentV1, PresentationErrorV1> {
    Ok(PresentationEnvironmentV1 {
        terrain_root: reader.digest()?,
        environment_digest: reader.digest()?,
        cloud_milli: reader.u16()?,
        rain_milli: reader.u16()?,
        wind_mm_s: [reader.i32()?, reader.i32()?],
        daylight_milli: reader.u16()?,
    })
}

fn decode_policy(
    reader: &mut Reader<'_>,
) -> Result<PresentationVisualPolicyV1, PresentationErrorV1> {
    Ok(PresentationVisualPolicyV1 {
        policy_digest: reader.digest()?,
        terrain_view_distance: reader.u16()?,
        entity_view_distance: reader.u16()?,
        figure_lod_distance: reader.u16()?,
        sprite_distance: reader.u16()?,
        particles_enabled: reader.boolean()?,
        weapon_trails_enabled: reader.boolean()?,
        flashing_lights_enabled: reader.boolean()?,
    })
}

fn try_vec<T>(count: usize) -> Result<Vec<T>, PresentationErrorV1> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| PresentationErrorV1::AllocationFailure)?;
    Ok(values)
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], PresentationErrorV1> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(PresentationErrorV1::SizeOverflow)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(PresentationErrorV1::Truncated)?;
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, PresentationErrorV1> {
        Ok(*self
            .take(1)?
            .first()
            .ok_or(PresentationErrorV1::Truncated)?)
    }

    fn u16(&mut self) -> Result<u16, PresentationErrorV1> {
        let mut value = [0; 2];
        value.copy_from_slice(self.take(2)?);
        Ok(u16::from_le_bytes(value))
    }

    fn u32(&mut self) -> Result<u32, PresentationErrorV1> {
        let mut value = [0; 4];
        value.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(value))
    }

    fn u64(&mut self) -> Result<u64, PresentationErrorV1> {
        let mut value = [0; 8];
        value.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(value))
    }

    fn i32(&mut self) -> Result<i32, PresentationErrorV1> {
        let mut value = [0; 4];
        value.copy_from_slice(self.take(4)?);
        Ok(i32::from_le_bytes(value))
    }

    fn i64(&mut self) -> Result<i64, PresentationErrorV1> {
        let mut value = [0; 8];
        value.copy_from_slice(self.take(8)?);
        Ok(i64::from_le_bytes(value))
    }

    fn digest(&mut self) -> Result<PresentationDigestV1, PresentationErrorV1> {
        let mut value = [0; 32];
        value.copy_from_slice(self.take(32)?);
        Ok(value)
    }

    fn boolean(&mut self) -> Result<bool, PresentationErrorV1> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(PresentationErrorV1::MalformedBoolean(other)),
        }
    }

    fn count(&mut self, cap: usize) -> Result<usize, PresentationErrorV1> {
        let value = usize::try_from(self.u32()?).map_err(|_| PresentationErrorV1::SizeOverflow)?;
        if value > cap {
            return Err(PresentationErrorV1::EncodedSizeExceeded(value));
        }
        Ok(value)
    }

    fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.position) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn entity(byte: u8, group: Option<u8>) -> PresentationEntityV1 {
        PresentationEntityV1 {
            semantic_id: digest(byte),
            figure_resource: digest(byte + 10),
            group_id: group.map(digest),
            position_mm: [i64::from(byte), -i64::from(byte), 0],
            orientation_q30: [0, 0, 0, 1 << 30],
            scale_milli: 1_000,
            state_tag: 1,
            state_digest: digest(byte + 20),
        }
    }

    fn draft(generation: u64, reverse: bool) -> PresentationFrameDraftV1 {
        let mut entities = vec![entity(2, Some(9)), entity(1, Some(9))];
        let mut resources = vec![digest(12), digest(11), digest(4)];
        if !reverse {
            entities.reverse();
            resources.reverse();
        }
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 7,
                client_applied_generation: generation,
                simulation_tick: 300,
                coherent_snapshot_root: digest(30),
            },
            entities,
            groups: vec![PresentationGroupV1 {
                semantic_id: digest(9),
                kind_tag: 1,
                member_ids: vec![digest(2), digest(1)],
                state_digest: digest(31),
            }],
            events: vec![PresentationEventV1 {
                semantic_id: digest(40),
                kind_tag: 1,
                source_id: digest(1),
                target_id: digest(2),
                payload_digest: digest(41),
            }],
            environment: PresentationEnvironmentV1 {
                terrain_root: digest(4),
                environment_digest: digest(5),
                cloud_milli: 100,
                rain_milli: 20,
                wind_mm_s: [-30, 40],
                daylight_milli: 700,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: digest(6),
                terrain_view_distance: 16,
                entity_view_distance: 12,
                figure_lod_distance: 350,
                sprite_distance: 250,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: false,
            },
            renderer_required_resources: resources,
            complete: true,
        }
    }

    #[test]
    fn canonical_order_encoding_round_trip_and_frozen_digest() {
        let a = draft(1, false).seal().unwrap();
        let b = draft(1, true).seal().unwrap();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.frame_digest(), b.frame_digest());
        assert_eq!(
            hex_bytes(&a.frame_digest()),
            "3d0ee9418cbf09b40e9fa5e1ad913d7048fb67ef909e85dd1b31453dcd138528"
        );
        assert_eq!(
            PresentationFrameV1::decode_exact(a.canonical_bytes()).unwrap(),
            a
        );
    }

    #[test]
    fn every_section_changes_full_digest() {
        let base = draft(1, false).seal().unwrap();
        let mut cases = Vec::new();
        let mut entity_case = draft(1, false);
        entity_case.entities[0].position_mm[0] += 1;
        cases.push(entity_case);
        let mut group_case = draft(1, false);
        group_case.groups[0].state_digest = digest(55);
        cases.push(group_case);
        let mut event_case = draft(1, false);
        event_case.events[0].payload_digest = digest(56);
        cases.push(event_case);
        let mut environment_case = draft(1, false);
        environment_case.environment.daylight_milli += 1;
        cases.push(environment_case);
        let mut policy_case = draft(1, false);
        policy_case.visual_policy.sprite_distance += 1;
        cases.push(policy_case);
        for changed in cases {
            assert_ne!(changed.seal().unwrap().frame_digest(), base.frame_digest());
        }
    }

    #[test]
    fn partial_duplicate_oversize_and_group_mismatch_fail_closed() {
        let mut partial = draft(1, false);
        partial.complete = false;
        assert_eq!(partial.seal(), Err(PresentationErrorV1::UnsealedOrPartial));

        let mut duplicate = draft(1, false);
        duplicate.entities.push(duplicate.entities[0].clone());
        assert!(matches!(
            duplicate.seal(),
            Err(PresentationErrorV1::DuplicateEntity(_))
        ));

        let mut oversize = draft(1, false);
        oversize.entities = vec![entity(1, None); MAX_PRESENTATION_ENTITIES_V1 + 1];
        assert_eq!(
            oversize.seal(),
            Err(PresentationErrorV1::TooManyEntities(
                MAX_PRESENTATION_ENTITIES_V1 + 1
            ))
        );

        let mut mismatch = draft(1, false);
        mismatch.groups[0].member_ids.pop();
        assert!(matches!(
            mismatch.seal(),
            Err(PresentationErrorV1::EntityGroupMismatch(_))
        ));
    }

    #[test]
    fn malformed_truncated_and_trailing_bytes_are_typed() {
        let frame = draft(1, false).seal().unwrap();
        let mut magic = frame.canonical_bytes().to_vec();
        magic[0] ^= 1;
        assert_eq!(
            PresentationFrameV1::decode_exact(&magic),
            Err(PresentationErrorV1::InvalidMagic)
        );
        let truncated = &frame.canonical_bytes()[..frame.canonical_bytes().len() - 1];
        assert_eq!(
            PresentationFrameV1::decode_exact(truncated),
            Err(PresentationErrorV1::Truncated)
        );
        let mut trailing = frame.canonical_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            PresentationFrameV1::decode_exact(&trailing),
            Err(PresentationErrorV1::TrailingBytes(1))
        );
    }

    fn completion(frame: &PresentationFrameV1) -> RendererUploadCompletionV1 {
        RendererUploadCompletionV1 {
            client_applied_generation: frame.generation.client_applied_generation,
            frame_digest: frame.frame_digest,
            resource_set_digest: frame.resource_set_digest,
            completed_resources: frame.renderer_required_resources.clone(),
        }
    }

    #[test]
    fn handoff_rejects_partial_mismatch_stale_and_superseded_generations() {
        let first = draft(1, false).seal().unwrap();
        let second = draft(2, false).seal().unwrap();
        let mut handoff = PresentationHandoffV1::default();
        handoff.stage(first.clone()).unwrap();
        let mut partial = completion(&first);
        partial.completed_resources.pop();
        assert!(matches!(
            handoff.acknowledge_uploads(partial),
            Err(PresentationHandoffErrorV1::PartialResourceCompletion { .. })
        ));
        handoff.stage(second.clone()).unwrap();
        assert!(matches!(
            handoff.acknowledge_uploads(completion(&first)),
            Err(PresentationHandoffErrorV1::SupersededGeneration { .. })
        ));
        let mut mismatch = completion(&second);
        mismatch.frame_digest = digest(99);
        assert_eq!(
            handoff.acknowledge_uploads(mismatch),
            Err(PresentationHandoffErrorV1::AcknowledgementFrameMismatch)
        );
        let token = handoff.acknowledge_uploads(completion(&second)).unwrap();
        assert_eq!(token.client_applied_generation, 2);
        assert!(matches!(
            handoff.stage(first),
            Err(PresentationHandoffErrorV1::StaleOrEqualGeneration { .. })
        ));
    }

    #[test]
    fn held_reader_is_immutable_and_consumers_require_exact_ready_generation() {
        let first = draft(1, false).seal().unwrap();
        let second = draft(2, false).seal().unwrap();
        let mut handoff = PresentationHandoffV1::default();
        handoff.stage(first.clone()).unwrap();
        handoff.acknowledge_uploads(completion(&first)).unwrap();
        let held = handoff.acquire_visible().unwrap();
        handoff.stage(second.clone()).unwrap();
        handoff.acknowledge_uploads(completion(&second)).unwrap();
        assert_eq!(held.generation().client_applied_generation, 1);
        assert_eq!(
            handoff.authorize_consumer(1),
            Err(PresentationHandoffErrorV1::ConsumerGenerationMismatch {
                visible: 2,
                requested: 1
            })
        );
        assert!(handoff.authorize_consumer(2).is_ok());
    }

    #[test]
    fn semantic_tape_and_replay_are_identical_for_accepted_frame() {
        let frame = draft(10, false).seal().unwrap();
        let replay = PresentationFrameV1::decode_exact(frame.canonical_bytes()).unwrap();
        assert_eq!(
            frame.semantic_tape().unwrap().final_root(),
            replay.semantic_tape().unwrap().final_root()
        );
        assert_eq!(frame.frame_digest(), replay.frame_digest());
    }
}
