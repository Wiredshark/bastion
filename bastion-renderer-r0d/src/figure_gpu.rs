//! Deterministic R1BC figure GPU ABI, persistent allocation plan, bounded
//! upload schedule, completion receipt, and generation-safe retirement.
//!
//! This module owns semantic GPU resource truth. Backend callback order and
//! frame timing never choose slots, upload order, readiness, or retirement.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use sha2::{Digest, Sha256};

use crate::{
    figure_asset::{CompiledFigurePackageV1, PackageReceiptTerminalV1, PackageReceiptV1},
    presentation::{PresentationFrameV1, RendererUploadCompletionV1},
};

pub const FIGURE_GPU_ABI_VERSION_V1: u16 = 1;
pub const FIGURE_GPU_INSTANCE_STRIDE_V1: usize = 256;
pub const FIGURE_GPU_POSE_SLOT_BYTES_V1: usize = 1_024;
pub const FIGURE_GPU_POSE_PAGE_BYTES_V1: usize = 4_096;
pub const FIGURE_GPU_POSE_SLOTS_PER_PAGE_V1: usize =
    FIGURE_GPU_POSE_PAGE_BYTES_V1 / FIGURE_GPU_POSE_SLOT_BYTES_V1;
pub const FIGURE_GPU_MAX_BONES_V1: usize = 16;
pub const FIGURE_GPU_BONE_COMPONENTS_V1: usize = 12;
pub const FIGURE_GPU_MAX_INSTANCES_V1: usize = 4_096;
pub const FIGURE_GPU_MAX_POSE_PAGES_V1: usize = 1_024;
pub const FIGURE_GPU_MAX_UPLOAD_BYTES_V1: usize = 4 * 1024 * 1024;
pub const FIGURE_GPU_MAX_UPLOAD_OPS_V1: usize = 256;
pub const FIGURE_GPU_MAX_PACKAGES_PER_WINDOW_V1: usize = 64;
pub const FIGURE_GPU_MAX_PAGES_PER_WINDOW_V1: usize = 64;

const INSTANCE_MAGIC: &[u8; 8] = b"BSTRGI01";
const POSE_MAGIC: &[u8; 8] = b"BSTRGP01";
const PLAN_MAGIC: &[u8; 8] = b"BSTRGU01";
const RECEIPT_MAGIC: &[u8; 8] = b"BSTRGR01";

pub type FigureGpuDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FigureGpuBufferKindV1 {
    Instances = 1,
    Poses = 2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FigureGpuSlotV1 {
    pub instance_slot: u32,
    pub pose_page: u32,
    pub pose_offset: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FigureGpuBoneV1 {
    /// Row-major affine 3x4 matrix in signed Q20 fixed point.
    pub matrix_q20: [i32; FIGURE_GPU_BONE_COMPONENTS_V1],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuEntityInputV1 {
    pub generation: u64,
    pub semantic_entity: FigureGpuDigestV1,
    pub package_digest: FigureGpuDigestV1,
    pub authority_digest: FigureGpuDigestV1,
    pub composition_digest: FigureGpuDigestV1,
    pub palette_digest: FigureGpuDigestV1,
    pub transform_digest: FigureGpuDigestV1,
    pub pose_digest: FigureGpuDigestV1,
    pub lod_level: u16,
    pub section_id: u16,
    pub material_id: u16,
    pub flags: u16,
    pub bones: Vec<FigureGpuBoneV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuInstanceRecordV1 {
    pub input: FigureGpuEntityInputV1,
    pub slot: FigureGpuSlotV1,
}

impl FigureGpuInstanceRecordV1 {
    pub fn canonical_bytes(&self) -> Result<[u8; FIGURE_GPU_INSTANCE_STRIDE_V1], FigureGpuErrorV1> {
        validate_entity(&self.input)?;
        validate_slot(self.slot)?;
        let mut output = [0_u8; FIGURE_GPU_INSTANCE_STRIDE_V1];
        output[0..8].copy_from_slice(INSTANCE_MAGIC);
        output[8..10].copy_from_slice(&FIGURE_GPU_ABI_VERSION_V1.to_le_bytes());
        output[10..12].copy_from_slice(&1_u16.to_le_bytes());
        output[12..16].copy_from_slice(
            &u32::try_from(FIGURE_GPU_INSTANCE_STRIDE_V1)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                .to_le_bytes(),
        );
        output[16..24].copy_from_slice(&self.input.generation.to_le_bytes());
        output[24..56].copy_from_slice(&self.input.semantic_entity);
        output[56..88].copy_from_slice(&self.input.package_digest);
        output[88..120].copy_from_slice(&self.input.composition_digest);
        output[120..152].copy_from_slice(&self.input.palette_digest);
        output[152..184].copy_from_slice(&self.input.transform_digest);
        output[184..216].copy_from_slice(&self.input.pose_digest);
        output[216..220].copy_from_slice(&self.slot.instance_slot.to_le_bytes());
        output[220..224].copy_from_slice(&self.slot.pose_page.to_le_bytes());
        output[224..228].copy_from_slice(&self.slot.pose_offset.to_le_bytes());
        output[228..230].copy_from_slice(&self.input.lod_level.to_le_bytes());
        output[230..232].copy_from_slice(&self.input.section_id.to_le_bytes());
        output[232..234].copy_from_slice(&self.input.material_id.to_le_bytes());
        output[234..236].copy_from_slice(&self.input.flags.to_le_bytes());
        let provenance: FigureGpuDigestV1 = Sha256::digest(
            [
                self.input.authority_digest.as_slice(),
                self.input.package_digest.as_slice(),
            ]
            .concat(),
        )
        .into();
        output[236..256].copy_from_slice(&provenance[..20]);
        Ok(output)
    }

    pub fn decode_exact(
        bytes: &[u8],
        authority_digest: FigureGpuDigestV1,
        bones: Vec<FigureGpuBoneV1>,
    ) -> Result<Self, FigureGpuErrorV1> {
        if bytes.len() != FIGURE_GPU_INSTANCE_STRIDE_V1 {
            return Err(FigureGpuErrorV1::InvalidRecordLength(bytes.len()));
        }
        if &bytes[0..8] != INSTANCE_MAGIC {
            return Err(FigureGpuErrorV1::InvalidMagic);
        }
        let version = u16_at(bytes, 8)?;
        if version != FIGURE_GPU_ABI_VERSION_V1 {
            return Err(FigureGpuErrorV1::UnsupportedAbi(version));
        }
        if u16_at(bytes, 10)? != 1
            || usize::try_from(u32_at(bytes, 12)?).map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                != FIGURE_GPU_INSTANCE_STRIDE_V1
        {
            return Err(FigureGpuErrorV1::NonCanonical);
        }
        let input = FigureGpuEntityInputV1 {
            generation: u64_at(bytes, 16)?,
            semantic_entity: digest_at(bytes, 24)?,
            package_digest: digest_at(bytes, 56)?,
            authority_digest,
            composition_digest: digest_at(bytes, 88)?,
            palette_digest: digest_at(bytes, 120)?,
            transform_digest: digest_at(bytes, 152)?,
            pose_digest: digest_at(bytes, 184)?,
            lod_level: u16_at(bytes, 228)?,
            section_id: u16_at(bytes, 230)?,
            material_id: u16_at(bytes, 232)?,
            flags: u16_at(bytes, 234)?,
            bones,
        };
        let record = Self {
            input,
            slot: FigureGpuSlotV1 {
                instance_slot: u32_at(bytes, 216)?,
                pose_page: u32_at(bytes, 220)?,
                pose_offset: u32_at(bytes, 224)?,
            },
        };
        if record.canonical_bytes()?.as_slice() != bytes {
            return Err(FigureGpuErrorV1::DigestMismatch);
        }
        Ok(record)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuPoseRecordV1 {
    pub generation: u64,
    pub semantic_entity: FigureGpuDigestV1,
    pub pose_digest: FigureGpuDigestV1,
    pub slot: FigureGpuSlotV1,
    pub bones: Vec<FigureGpuBoneV1>,
}

impl FigureGpuPoseRecordV1 {
    pub fn canonical_bytes(&self) -> Result<[u8; FIGURE_GPU_POSE_SLOT_BYTES_V1], FigureGpuErrorV1> {
        if self.generation == 0
            || is_zero(&self.semantic_entity)
            || is_zero(&self.pose_digest)
            || self.bones.is_empty()
            || self.bones.len() > FIGURE_GPU_MAX_BONES_V1
        {
            return Err(FigureGpuErrorV1::InvalidPose);
        }
        validate_slot(self.slot)?;
        let mut output = [0_u8; FIGURE_GPU_POSE_SLOT_BYTES_V1];
        output[0..8].copy_from_slice(POSE_MAGIC);
        output[8..10].copy_from_slice(&FIGURE_GPU_ABI_VERSION_V1.to_le_bytes());
        output[10..12].copy_from_slice(
            &u16::try_from(self.bones.len())
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                .to_le_bytes(),
        );
        output[12..20].copy_from_slice(&self.generation.to_le_bytes());
        output[20..52].copy_from_slice(&self.semantic_entity);
        output[52..84].copy_from_slice(&self.pose_digest);
        output[84..88].copy_from_slice(&self.slot.pose_page.to_le_bytes());
        output[88..92].copy_from_slice(&self.slot.pose_offset.to_le_bytes());
        let payload_start = 128;
        let mut cursor = payload_start;
        for bone in &self.bones {
            for component in bone.matrix_q20 {
                output[cursor..cursor + 4].copy_from_slice(&component.to_le_bytes());
                cursor += 4;
            }
        }
        let payload_digest: FigureGpuDigestV1 =
            Sha256::digest(&output[payload_start..cursor]).into();
        output[92..124].copy_from_slice(&payload_digest);
        Ok(output)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, FigureGpuErrorV1> {
        if bytes.len() != FIGURE_GPU_POSE_SLOT_BYTES_V1 {
            return Err(FigureGpuErrorV1::InvalidRecordLength(bytes.len()));
        }
        if &bytes[0..8] != POSE_MAGIC {
            return Err(FigureGpuErrorV1::InvalidMagic);
        }
        let version = u16_at(bytes, 8)?;
        if version != FIGURE_GPU_ABI_VERSION_V1 {
            return Err(FigureGpuErrorV1::UnsupportedAbi(version));
        }
        let bone_count = usize::from(u16_at(bytes, 10)?);
        if bone_count == 0 || bone_count > FIGURE_GPU_MAX_BONES_V1 {
            return Err(FigureGpuErrorV1::InvalidPose);
        }
        let payload_end = 128_usize
            .checked_add(
                bone_count
                    .checked_mul(FIGURE_GPU_BONE_COMPONENTS_V1)
                    .and_then(|value| value.checked_mul(4))
                    .ok_or(FigureGpuErrorV1::LengthOverflow)?,
            )
            .ok_or(FigureGpuErrorV1::LengthOverflow)?;
        let expected_payload: FigureGpuDigestV1 = Sha256::digest(&bytes[128..payload_end]).into();
        if bytes[92..124] != expected_payload
            || bytes[124..128].iter().any(|byte| *byte != 0)
            || bytes[payload_end..].iter().any(|byte| *byte != 0)
        {
            return Err(FigureGpuErrorV1::DigestMismatch);
        }
        let mut bones = Vec::new();
        bones
            .try_reserve_exact(bone_count)
            .map_err(|_| FigureGpuErrorV1::AllocationFailure)?;
        let mut cursor = 128;
        for _ in 0..bone_count {
            let mut matrix_q20 = [0_i32; FIGURE_GPU_BONE_COMPONENTS_V1];
            for component in &mut matrix_q20 {
                *component = i32_at(bytes, cursor)?;
                cursor += 4;
            }
            bones.push(FigureGpuBoneV1 { matrix_q20 });
        }
        let record = Self {
            generation: u64_at(bytes, 12)?,
            semantic_entity: digest_at(bytes, 20)?,
            pose_digest: digest_at(bytes, 52)?,
            slot: FigureGpuSlotV1 {
                instance_slot: 0,
                pose_page: u32_at(bytes, 84)?,
                pose_offset: u32_at(bytes, 88)?,
            },
            bones,
        };
        if record.canonical_bytes()?.as_slice() != bytes {
            return Err(FigureGpuErrorV1::NonCanonical);
        }
        Ok(record)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FigureGpuPoolConfigV1 {
    pub instance_capacity: u32,
    pub pose_page_capacity: u32,
    pub max_upload_bytes: u32,
    pub max_upload_ops: u16,
    pub max_packages_per_window: u16,
    pub max_pages_per_window: u16,
}

impl Default for FigureGpuPoolConfigV1 {
    fn default() -> Self {
        Self {
            instance_capacity: FIGURE_GPU_MAX_INSTANCES_V1 as u32,
            pose_page_capacity: FIGURE_GPU_MAX_POSE_PAGES_V1 as u32,
            max_upload_bytes: FIGURE_GPU_MAX_UPLOAD_BYTES_V1 as u32,
            max_upload_ops: FIGURE_GPU_MAX_UPLOAD_OPS_V1 as u16,
            max_packages_per_window: FIGURE_GPU_MAX_PACKAGES_PER_WINDOW_V1 as u16,
            max_pages_per_window: FIGURE_GPU_MAX_PAGES_PER_WINDOW_V1 as u16,
        }
    }
}

impl FigureGpuPoolConfigV1 {
    pub fn validate(self) -> Result<Self, FigureGpuErrorV1> {
        if self.instance_capacity == 0
            || usize::try_from(self.instance_capacity)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                > FIGURE_GPU_MAX_INSTANCES_V1
            || self.pose_page_capacity == 0
            || usize::try_from(self.pose_page_capacity)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                > FIGURE_GPU_MAX_POSE_PAGES_V1
            || self.max_upload_bytes == 0
            || usize::try_from(self.max_upload_bytes)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                > FIGURE_GPU_MAX_UPLOAD_BYTES_V1
            || self.max_upload_ops == 0
            || usize::from(self.max_upload_ops) > FIGURE_GPU_MAX_UPLOAD_OPS_V1
            || self.max_packages_per_window == 0
            || usize::from(self.max_packages_per_window) > FIGURE_GPU_MAX_PACKAGES_PER_WINDOW_V1
            || self.max_pages_per_window == 0
            || usize::from(self.max_pages_per_window) > FIGURE_GPU_MAX_PAGES_PER_WINDOW_V1
        {
            return Err(FigureGpuErrorV1::InvalidConfig);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuAssignmentV1 {
    pub semantic_entity: FigureGpuDigestV1,
    pub package_digest: FigureGpuDigestV1,
    pub slot: FigureGpuSlotV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuUploadRangeV1 {
    pub buffer_kind: FigureGpuBufferKindV1,
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub bytes_digest: FigureGpuDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuUploadWindowV1 {
    pub ordinal: u16,
    pub ranges: Vec<FigureGpuUploadRangeV1>,
    pub total_bytes: u32,
    pub operation_count: u16,
    pub package_count: u16,
    pub pose_page_count: u16,
    pub staged_digest: FigureGpuDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureGpuUploadPlanV1 {
    pub config: FigureGpuPoolConfigV1,
    pub generation: u64,
    pub frame_digest: FigureGpuDigestV1,
    pub resource_set_digest: FigureGpuDigestV1,
    pub package_digest: FigureGpuDigestV1,
    pub package_receipt_digest: FigureGpuDigestV1,
    pub assignments: Vec<FigureGpuAssignmentV1>,
    pub windows: Vec<FigureGpuUploadWindowV1>,
    pub assignment_digest: FigureGpuDigestV1,
    pub staged_digest: FigureGpuDigestV1,
    pub plan_digest: FigureGpuDigestV1,
}

#[derive(Clone, Debug)]
pub struct FigureGpuGenerationV1 {
    pub plan: FigureGpuUploadPlanV1,
    records: Vec<(FigureGpuInstanceRecordV1, FigureGpuPoseRecordV1)>,
}

impl FigureGpuGenerationV1 {
    #[must_use]
    pub fn records(&self) -> &[(FigureGpuInstanceRecordV1, FigureGpuPoseRecordV1)] { &self.records }
}

#[derive(Clone, Debug)]
struct LiveGenerationV1 {
    generation: Arc<FigureGpuGenerationV1>,
    receipt: UploadReceiptV1,
}

#[derive(Clone, Debug)]
pub struct FigureGpuPoolV1 {
    config: FigureGpuPoolConfigV1,
    live: BTreeMap<u64, LiveGenerationV1>,
    pending: Option<Arc<FigureGpuGenerationV1>>,
}

impl FigureGpuPoolV1 {
    pub fn new(config: FigureGpuPoolConfigV1) -> Result<Self, FigureGpuErrorV1> {
        Ok(Self {
            config: config.validate()?,
            live: BTreeMap::new(),
            pending: None,
        })
    }

    pub fn begin_generation(
        &mut self,
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
        package_receipt: &PackageReceiptV1,
        mut entities: Vec<FigureGpuEntityInputV1>,
    ) -> Result<Arc<FigureGpuGenerationV1>, FigureGpuErrorV1> {
        if self.pending.is_some() {
            return Err(FigureGpuErrorV1::PendingGenerationExists);
        }
        package_receipt
            .validate(frame, package)
            .map_err(|_| FigureGpuErrorV1::PackageReceiptMismatch)?;
        if package_receipt.terminal != PackageReceiptTerminalV1::Accepted {
            return Err(FigureGpuErrorV1::PackageReceiptMismatch);
        }
        let generation = frame.generation().client_applied_generation;
        if entities.is_empty() {
            return Err(FigureGpuErrorV1::EmptyGeneration);
        }
        entities.sort_by(entity_order);
        if entities
            .windows(2)
            .any(|pair| pair[0].semantic_entity == pair[1].semantic_entity)
        {
            return Err(FigureGpuErrorV1::DuplicateSemanticEntity);
        }
        for entity in &entities {
            validate_entity(entity)?;
            if entity.generation != generation
                || entity.package_digest != package.package_digest()
                || entity.authority_digest != package.authority_digest()
            {
                return Err(FigureGpuErrorV1::GenerationOrPackageMismatch);
            }
        }

        let mut occupied_instances = BTreeSet::new();
        let mut occupied_pose_slots = BTreeSet::new();
        for live in self.live.values() {
            for assignment in &live.generation.plan.assignments {
                occupied_instances.insert(assignment.slot.instance_slot);
                occupied_pose_slots.insert(pose_linear_slot(assignment.slot)?);
            }
        }
        let free_instances = (0..self.config.instance_capacity)
            .filter(|slot| !occupied_instances.contains(slot))
            .take(entities.len())
            .collect::<Vec<_>>();
        let pose_capacity = self
            .config
            .pose_page_capacity
            .checked_mul(
                u32::try_from(FIGURE_GPU_POSE_SLOTS_PER_PAGE_V1)
                    .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
            )
            .ok_or(FigureGpuErrorV1::LengthOverflow)?;
        let free_pose_slots = (0..pose_capacity)
            .filter(|slot| !occupied_pose_slots.contains(slot))
            .take(entities.len())
            .collect::<Vec<_>>();
        if free_instances.len() != entities.len() || free_pose_slots.len() != entities.len() {
            return Err(FigureGpuErrorV1::PoolCapacity);
        }

        let mut assignments = Vec::new();
        let mut records = Vec::new();
        for ((entity, instance_slot), pose_linear) in entities
            .into_iter()
            .zip(free_instances)
            .zip(free_pose_slots)
        {
            let pose_slots_per_page = u32::try_from(FIGURE_GPU_POSE_SLOTS_PER_PAGE_V1)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
            let pose_slot_bytes = u32::try_from(FIGURE_GPU_POSE_SLOT_BYTES_V1)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
            let slot = FigureGpuSlotV1 {
                instance_slot,
                pose_page: pose_linear / pose_slots_per_page,
                pose_offset: (pose_linear % pose_slots_per_page)
                    .checked_mul(pose_slot_bytes)
                    .ok_or(FigureGpuErrorV1::LengthOverflow)?,
            };
            assignments.push(FigureGpuAssignmentV1 {
                semantic_entity: entity.semantic_entity,
                package_digest: entity.package_digest,
                slot,
            });
            records.push((
                FigureGpuInstanceRecordV1 {
                    input: entity.clone(),
                    slot,
                },
                FigureGpuPoseRecordV1 {
                    generation: entity.generation,
                    semantic_entity: entity.semantic_entity,
                    pose_digest: entity.pose_digest,
                    slot: FigureGpuSlotV1 {
                        instance_slot: 0,
                        ..slot
                    },
                    bones: entity.bones,
                },
            ));
        }
        let package_receipt_digest = package_receipt_digest(package_receipt)?;
        let (windows, assignment_digest, staged_digest) =
            build_upload_windows(self.config, &assignments, &records)?;
        let plan_digest = plan_digest(
            self.config,
            generation,
            frame.frame_digest(),
            frame.resource_set_digest(),
            package.package_digest(),
            package_receipt_digest,
            assignment_digest,
            staged_digest,
            &windows,
        )?;
        let staged = Arc::new(FigureGpuGenerationV1 {
            plan: FigureGpuUploadPlanV1 {
                config: self.config,
                generation,
                frame_digest: frame.frame_digest(),
                resource_set_digest: frame.resource_set_digest(),
                package_digest: package.package_digest(),
                package_receipt_digest,
                assignments,
                windows,
                assignment_digest,
                staged_digest,
                plan_digest,
            },
            records,
        });
        self.pending = Some(Arc::clone(&staged));
        Ok(staged)
    }

    pub fn commit_pending(
        &mut self,
        receipt: UploadReceiptV1,
    ) -> Result<Arc<FigureGpuGenerationV1>, FigureGpuErrorV1> {
        let pending = self
            .pending
            .take()
            .ok_or(FigureGpuErrorV1::NoPendingGeneration)?;
        if receipt.terminal != UploadReceiptTerminalV1::Accepted
            || receipt.plan_digest != pending.plan.plan_digest
            || receipt.generation != pending.plan.generation
        {
            self.pending = Some(pending);
            return Err(FigureGpuErrorV1::UploadReceiptMismatch);
        }
        if self.live.contains_key(&receipt.generation) {
            self.pending = Some(pending);
            return Err(FigureGpuErrorV1::StaleOrDuplicateGeneration);
        }
        self.live.insert(receipt.generation, LiveGenerationV1 {
            generation: Arc::clone(&pending),
            receipt,
        });
        Ok(pending)
    }

    pub fn rollback_pending(
        &mut self,
        plan_digest: FigureGpuDigestV1,
    ) -> Result<(), FigureGpuErrorV1> {
        let pending = self
            .pending
            .take()
            .ok_or(FigureGpuErrorV1::NoPendingGeneration)?;
        if pending.plan.plan_digest != plan_digest {
            self.pending = Some(pending);
            return Err(FigureGpuErrorV1::UploadPlanMismatch);
        }
        Ok(())
    }

    #[must_use]
    pub fn held_generation(&self, generation: u64) -> Option<Arc<FigureGpuGenerationV1>> {
        self.live
            .get(&generation)
            .map(|value| Arc::clone(&value.generation))
    }

    pub fn retire_generation(
        &mut self,
        generation: u64,
        completion: &BackendCompletionV1,
    ) -> Result<(), FigureGpuErrorV1> {
        let live = self
            .live
            .get(&generation)
            .ok_or(FigureGpuErrorV1::UnknownGeneration)?;
        if Arc::strong_count(&live.generation) != 1 {
            return Err(FigureGpuErrorV1::HeldGeneration);
        }
        match completion {
            BackendCompletionV1::Completed(identity)
                if *identity == live.receipt.completion_identity => {},
            BackendCompletionV1::Completed(_) => {
                return Err(FigureGpuErrorV1::CompletionIdentityMismatch);
            },
            BackendCompletionV1::Incomplete => {
                return Err(FigureGpuErrorV1::IncompleteSubmission);
            },
            BackendCompletionV1::DeviceLost => {
                return Err(FigureGpuErrorV1::DeviceLost);
            },
        }
        self.live.remove(&generation);
        Ok(())
    }

    #[must_use]
    pub fn live_generation_count(&self) -> usize { self.live.len() }

    #[must_use]
    pub fn pending(&self) -> Option<&Arc<FigureGpuGenerationV1>> { self.pending.as_ref() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubmissionIdentityV1 {
    pub sequence: u64,
    pub digest: FigureGpuDigestV1,
}

impl SubmissionIdentityV1 {
    pub fn for_plan(sequence: u64, plan: &FigureGpuUploadPlanV1) -> Result<Self, FigureGpuErrorV1> {
        if sequence == 0 {
            return Err(FigureGpuErrorV1::InvalidSubmission);
        }
        plan.validate()?;
        let mut bytes = Vec::with_capacity(8 + 32);
        bytes.extend_from_slice(&sequence.to_le_bytes());
        bytes.extend_from_slice(&plan.plan_digest);
        Ok(Self {
            sequence,
            digest: Sha256::digest(bytes).into(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendCompletionV1 {
    Completed(SubmissionIdentityV1),
    Incomplete,
    DeviceLost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploadReceiptTerminalV1 {
    Accepted,
    RolledBack,
    DeviceFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UploadReceiptV1 {
    pub generation: u64,
    pub frame_digest: FigureGpuDigestV1,
    pub resource_set_digest: FigureGpuDigestV1,
    pub package_digest: FigureGpuDigestV1,
    pub package_receipt_digest: FigureGpuDigestV1,
    pub abi_version: u16,
    pub assignment_digest: FigureGpuDigestV1,
    pub staged_digest: FigureGpuDigestV1,
    pub plan_digest: FigureGpuDigestV1,
    pub submission_identity: SubmissionIdentityV1,
    pub completion_identity: SubmissionIdentityV1,
    pub terminal: UploadReceiptTerminalV1,
}

impl UploadReceiptV1 {
    pub fn from_backend_completion(
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
        package_receipt: &PackageReceiptV1,
        plan: &FigureGpuUploadPlanV1,
        submission: SubmissionIdentityV1,
        completion: BackendCompletionV1,
    ) -> Result<Self, FigureGpuErrorV1> {
        package_receipt
            .validate(frame, package)
            .map_err(|_| FigureGpuErrorV1::PackageReceiptMismatch)?;
        plan.validate()?;
        if package_receipt.terminal != PackageReceiptTerminalV1::Accepted
            || plan.generation != frame.generation().client_applied_generation
            || plan.frame_digest != frame.frame_digest()
            || plan.resource_set_digest != frame.resource_set_digest()
            || plan.package_digest != package.package_digest()
            || plan.package_receipt_digest != package_receipt_digest(package_receipt)?
            || submission != SubmissionIdentityV1::for_plan(submission.sequence, plan)?
        {
            return Err(FigureGpuErrorV1::UploadPlanMismatch);
        }
        let completion_identity = match completion {
            BackendCompletionV1::Completed(identity) if identity == submission => identity,
            BackendCompletionV1::Completed(_) => {
                return Err(FigureGpuErrorV1::CompletionIdentityMismatch);
            },
            BackendCompletionV1::Incomplete => {
                return Err(FigureGpuErrorV1::IncompleteSubmission);
            },
            BackendCompletionV1::DeviceLost => return Err(FigureGpuErrorV1::DeviceLost),
        };
        Ok(Self {
            generation: plan.generation,
            frame_digest: plan.frame_digest,
            resource_set_digest: plan.resource_set_digest,
            package_digest: plan.package_digest,
            package_receipt_digest: plan.package_receipt_digest,
            abi_version: FIGURE_GPU_ABI_VERSION_V1,
            assignment_digest: plan.assignment_digest,
            staged_digest: plan.staged_digest,
            plan_digest: plan.plan_digest,
            submission_identity: submission,
            completion_identity,
            terminal: UploadReceiptTerminalV1::Accepted,
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FigureGpuErrorV1> {
        self.validate_shape()?;
        let mut output = Vec::with_capacity(356);
        output.extend_from_slice(RECEIPT_MAGIC);
        output.extend_from_slice(&FIGURE_GPU_ABI_VERSION_V1.to_le_bytes());
        output.push(match self.terminal {
            UploadReceiptTerminalV1::Accepted => 1,
            UploadReceiptTerminalV1::RolledBack => 2,
            UploadReceiptTerminalV1::DeviceFailed => 3,
        });
        output.push(0);
        output.extend_from_slice(&self.generation.to_le_bytes());
        for digest in [
            self.frame_digest,
            self.resource_set_digest,
            self.package_digest,
            self.package_receipt_digest,
            self.assignment_digest,
            self.staged_digest,
            self.plan_digest,
        ] {
            output.extend_from_slice(&digest);
        }
        output.extend_from_slice(&self.submission_identity.sequence.to_le_bytes());
        output.extend_from_slice(&self.submission_identity.digest);
        output.extend_from_slice(&self.completion_identity.sequence.to_le_bytes());
        output.extend_from_slice(&self.completion_identity.digest);
        let digest: FigureGpuDigestV1 = Sha256::digest(&output).into();
        output.extend_from_slice(&digest);
        Ok(output)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, FigureGpuErrorV1> {
        if bytes.len() != 356 {
            return Err(FigureGpuErrorV1::InvalidRecordLength(bytes.len()));
        }
        if &bytes[0..8] != RECEIPT_MAGIC {
            return Err(FigureGpuErrorV1::InvalidMagic);
        }
        let version = u16_at(bytes, 8)?;
        if version != FIGURE_GPU_ABI_VERSION_V1 || bytes[11] != 0 {
            return Err(FigureGpuErrorV1::UnsupportedAbi(version));
        }
        let terminal = match bytes[10] {
            1 => UploadReceiptTerminalV1::Accepted,
            2 => UploadReceiptTerminalV1::RolledBack,
            3 => UploadReceiptTerminalV1::DeviceFailed,
            _ => return Err(FigureGpuErrorV1::NonCanonical),
        };
        let receipt = Self {
            generation: u64_at(bytes, 12)?,
            frame_digest: digest_at(bytes, 20)?,
            resource_set_digest: digest_at(bytes, 52)?,
            package_digest: digest_at(bytes, 84)?,
            package_receipt_digest: digest_at(bytes, 116)?,
            abi_version: version,
            assignment_digest: digest_at(bytes, 148)?,
            staged_digest: digest_at(bytes, 180)?,
            plan_digest: digest_at(bytes, 212)?,
            submission_identity: SubmissionIdentityV1 {
                sequence: u64_at(bytes, 244)?,
                digest: digest_at(bytes, 252)?,
            },
            completion_identity: SubmissionIdentityV1 {
                sequence: u64_at(bytes, 284)?,
                digest: digest_at(bytes, 292)?,
            },
            terminal,
        };
        if receipt.canonical_bytes()?.as_slice() != bytes {
            return Err(FigureGpuErrorV1::DigestMismatch);
        }
        Ok(receipt)
    }

    pub fn to_renderer_completion(
        &self,
        frame: &PresentationFrameV1,
    ) -> Result<RendererUploadCompletionV1, FigureGpuErrorV1> {
        self.validate_shape()?;
        if self.terminal != UploadReceiptTerminalV1::Accepted
            || self.generation != frame.generation().client_applied_generation
            || self.frame_digest != frame.frame_digest()
            || self.resource_set_digest != frame.resource_set_digest()
            || frame.renderer_required_resources() != [self.package_digest]
        {
            return Err(FigureGpuErrorV1::UploadReceiptMismatch);
        }
        Ok(RendererUploadCompletionV1 {
            client_applied_generation: self.generation,
            frame_digest: self.frame_digest,
            resource_set_digest: self.resource_set_digest,
            completed_resources: vec![self.package_digest],
        })
    }

    fn validate_shape(&self) -> Result<(), FigureGpuErrorV1> {
        if self.abi_version != FIGURE_GPU_ABI_VERSION_V1
            || self.generation == 0
            || [
                self.frame_digest,
                self.resource_set_digest,
                self.package_digest,
                self.package_receipt_digest,
                self.assignment_digest,
                self.staged_digest,
                self.plan_digest,
                self.submission_identity.digest,
                self.completion_identity.digest,
            ]
            .iter()
            .any(is_zero)
            || self.submission_identity.sequence == 0
            || self.completion_identity.sequence == 0
        {
            return Err(FigureGpuErrorV1::InvalidReceipt);
        }
        Ok(())
    }
}

impl FigureGpuUploadPlanV1 {
    /// Recomputes every checksum-bound field and all declared budgets before a
    /// backend is allowed to consume this plan.
    pub fn validate(&self) -> Result<(), FigureGpuErrorV1> {
        self.config.validate()?;
        if self.generation == 0
            || self.assignments.is_empty()
            || self.windows.is_empty()
            || [
                self.frame_digest,
                self.resource_set_digest,
                self.package_digest,
                self.package_receipt_digest,
            ]
            .iter()
            .any(is_zero)
        {
            return Err(FigureGpuErrorV1::UploadPlanMismatch);
        }
        if self
            .assignments
            .windows(2)
            .any(|pair| pair[0].semantic_entity >= pair[1].semantic_entity)
        {
            return Err(FigureGpuErrorV1::DuplicateSemanticEntity);
        }
        let mut instance_slots = BTreeSet::new();
        let mut pose_slots = BTreeSet::new();
        for assignment in &self.assignments {
            validate_slot(assignment.slot)?;
            if assignment.package_digest != self.package_digest
                || !instance_slots.insert(assignment.slot.instance_slot)
                || !pose_slots.insert(pose_linear_slot(assignment.slot)?)
            {
                return Err(FigureGpuErrorV1::UploadPlanMismatch);
            }
        }
        if assignment_digest_of(&self.assignments)? != self.assignment_digest {
            return Err(FigureGpuErrorV1::DigestMismatch);
        }

        let mut previous_key: Option<(FigureGpuBufferKindV1, u64)> = None;
        let mut staged = Vec::new();
        for (index, window) in self.windows.iter().enumerate() {
            if usize::from(window.ordinal) != index
                || window.ranges.is_empty()
                || usize::from(window.operation_count) != window.ranges.len()
                || usize::from(window.operation_count) > usize::from(self.config.max_upload_ops)
                || window.total_bytes > self.config.max_upload_bytes
                || window.package_count != 1
                || window.package_count > self.config.max_packages_per_window
                || window.pose_page_count > self.config.max_pages_per_window
            {
                return Err(FigureGpuErrorV1::UploadPlanMismatch);
            }
            let rebuilt = finalize_window(
                window.ordinal,
                window.ranges.clone(),
                usize::from(window.package_count),
                self.config.max_pages_per_window,
            )?;
            if rebuilt != *window {
                return Err(FigureGpuErrorV1::DigestMismatch);
            }
            for range in &window.ranges {
                let actual_digest: FigureGpuDigestV1 = Sha256::digest(&range.bytes).into();
                if range.bytes.is_empty() || range.bytes_digest != actual_digest {
                    return Err(FigureGpuErrorV1::DigestMismatch);
                }
                let key = (range.buffer_kind, range.offset);
                if previous_key.is_some_and(|previous| previous >= key) {
                    return Err(FigureGpuErrorV1::NonCanonical);
                }
                previous_key = Some(key);
                let end = range
                    .offset
                    .checked_add(
                        u64::try_from(range.bytes.len())
                            .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                    )
                    .ok_or(FigureGpuErrorV1::LengthOverflow)?;
                let capacity = match range.buffer_kind {
                    FigureGpuBufferKindV1::Instances => u64::from(self.config.instance_capacity)
                        .checked_mul(
                            u64::try_from(FIGURE_GPU_INSTANCE_STRIDE_V1)
                                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                        )
                        .ok_or(FigureGpuErrorV1::LengthOverflow)?,
                    FigureGpuBufferKindV1::Poses => u64::from(self.config.pose_page_capacity)
                        .checked_mul(
                            u64::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
                                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                        )
                        .ok_or(FigureGpuErrorV1::LengthOverflow)?,
                };
                if end > capacity {
                    return Err(FigureGpuErrorV1::PoolCapacity);
                }
            }
            staged.extend_from_slice(&window.ordinal.to_le_bytes());
            staged.extend_from_slice(&window.staged_digest);
        }
        let staged_digest: FigureGpuDigestV1 = Sha256::digest(staged).into();
        if staged_digest != self.staged_digest
            || plan_digest(
                self.config,
                self.generation,
                self.frame_digest,
                self.resource_set_digest,
                self.package_digest,
                self.package_receipt_digest,
                self.assignment_digest,
                self.staged_digest,
                &self.windows,
            )? != self.plan_digest
        {
            return Err(FigureGpuErrorV1::DigestMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FigureGpuErrorV1 {
    UnsupportedAbi(u16),
    InvalidMagic,
    InvalidRecordLength(usize),
    InvalidConfig,
    InvalidEntity,
    InvalidPose,
    InvalidSlot,
    EmptyGeneration,
    DuplicateSemanticEntity,
    GenerationOrPackageMismatch,
    PackageReceiptMismatch,
    PendingGenerationExists,
    NoPendingGeneration,
    StaleOrDuplicateGeneration,
    PoolCapacity,
    UploadBudgetTooSmall,
    TooManyPackages,
    TooManyPages,
    UploadPlanMismatch,
    UploadReceiptMismatch,
    InvalidSubmission,
    CompletionIdentityMismatch,
    IncompleteSubmission,
    DeviceLost,
    UnknownGeneration,
    HeldGeneration,
    InvalidReceipt,
    DigestMismatch,
    NonCanonical,
    Truncated,
    LengthOverflow,
    AllocationFailure,
}

fn validate_entity(entity: &FigureGpuEntityInputV1) -> Result<(), FigureGpuErrorV1> {
    if entity.generation == 0
        || [
            entity.semantic_entity,
            entity.package_digest,
            entity.authority_digest,
            entity.composition_digest,
            entity.palette_digest,
            entity.transform_digest,
            entity.pose_digest,
        ]
        .iter()
        .any(is_zero)
        || entity.section_id == 0
        || entity.material_id == 0
        || entity.bones.is_empty()
        || entity.bones.len() > FIGURE_GPU_MAX_BONES_V1
    {
        return Err(FigureGpuErrorV1::InvalidEntity);
    }
    Ok(())
}

fn validate_slot(slot: FigureGpuSlotV1) -> Result<(), FigureGpuErrorV1> {
    let page_bytes = u32::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    let slot_bytes = u32::try_from(FIGURE_GPU_POSE_SLOT_BYTES_V1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    if slot.pose_offset >= page_bytes || slot.pose_offset % slot_bytes != 0 {
        return Err(FigureGpuErrorV1::InvalidSlot);
    }
    Ok(())
}

fn pose_linear_slot(slot: FigureGpuSlotV1) -> Result<u32, FigureGpuErrorV1> {
    validate_slot(slot)?;
    let slots_per_page = u32::try_from(FIGURE_GPU_POSE_SLOTS_PER_PAGE_V1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    let slot_bytes = u32::try_from(FIGURE_GPU_POSE_SLOT_BYTES_V1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    slot.pose_page
        .checked_mul(slots_per_page)
        .and_then(|value| value.checked_add(slot.pose_offset / slot_bytes))
        .ok_or(FigureGpuErrorV1::LengthOverflow)
}

fn entity_order(
    left: &FigureGpuEntityInputV1,
    right: &FigureGpuEntityInputV1,
) -> std::cmp::Ordering {
    left.semantic_entity
        .cmp(&right.semantic_entity)
        .then(left.package_digest.cmp(&right.package_digest))
        .then(left.composition_digest.cmp(&right.composition_digest))
        .then(left.pose_digest.cmp(&right.pose_digest))
}

fn build_upload_windows(
    config: FigureGpuPoolConfigV1,
    assignments: &[FigureGpuAssignmentV1],
    records: &[(FigureGpuInstanceRecordV1, FigureGpuPoseRecordV1)],
) -> Result<
    (
        Vec<FigureGpuUploadWindowV1>,
        FigureGpuDigestV1,
        FigureGpuDigestV1,
    ),
    FigureGpuErrorV1,
> {
    let assignment_digest = assignment_digest_of(assignments)?;

    let mut ranges = Vec::new();
    let mut packages = BTreeSet::new();
    for (instance, pose) in records {
        packages.insert(instance.input.package_digest);
        let instance_bytes = instance.canonical_bytes()?.to_vec();
        ranges.push(FigureGpuUploadRangeV1 {
            buffer_kind: FigureGpuBufferKindV1::Instances,
            offset: u64::from(instance.slot.instance_slot)
                .checked_mul(
                    u64::try_from(FIGURE_GPU_INSTANCE_STRIDE_V1)
                        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                )
                .ok_or(FigureGpuErrorV1::LengthOverflow)?,
            bytes_digest: Sha256::digest(&instance_bytes).into(),
            bytes: instance_bytes,
        });
        let pose_bytes = pose.canonical_bytes()?.to_vec();
        ranges.push(FigureGpuUploadRangeV1 {
            buffer_kind: FigureGpuBufferKindV1::Poses,
            offset: u64::from(pose.slot.pose_page)
                .checked_mul(
                    u64::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
                        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                )
                .and_then(|value| value.checked_add(u64::from(pose.slot.pose_offset)))
                .ok_or(FigureGpuErrorV1::LengthOverflow)?,
            bytes_digest: Sha256::digest(&pose_bytes).into(),
            bytes: pose_bytes,
        });
    }
    if packages.len() > usize::from(config.max_packages_per_window) {
        return Err(FigureGpuErrorV1::TooManyPackages);
    }
    ranges.sort_by(|left, right| {
        left.buffer_kind
            .cmp(&right.buffer_kind)
            .then(left.offset.cmp(&right.offset))
            .then(left.bytes_digest.cmp(&right.bytes_digest))
    });
    let mut coalesced: Vec<FigureGpuUploadRangeV1> = Vec::new();
    for range in ranges {
        if let Some(previous) = coalesced.last_mut() {
            let previous_end = previous
                .offset
                .checked_add(
                    u64::try_from(previous.bytes.len())
                        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                )
                .ok_or(FigureGpuErrorV1::LengthOverflow)?;
            let combined = previous
                .bytes
                .len()
                .checked_add(range.bytes.len())
                .ok_or(FigureGpuErrorV1::LengthOverflow)?;
            if previous.buffer_kind == range.buffer_kind
                && previous_end == range.offset
                && combined
                    <= usize::try_from(config.max_upload_bytes)
                        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                && pose_page_count_for_range(previous.buffer_kind, previous.offset, combined)?
                    <= usize::from(config.max_pages_per_window)
            {
                previous.bytes.extend_from_slice(&range.bytes);
                previous.bytes_digest = Sha256::digest(&previous.bytes).into();
                continue;
            }
        }
        coalesced.push(range);
    }

    let mut windows = Vec::new();
    let mut current = Vec::new();
    let mut current_bytes = 0_usize;
    let max_bytes =
        usize::try_from(config.max_upload_bytes).map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    let max_ops = usize::from(config.max_upload_ops);
    for range in coalesced {
        if range.bytes.len() > max_bytes {
            return Err(FigureGpuErrorV1::UploadBudgetTooSmall);
        }
        if pose_page_count_for_range(range.buffer_kind, range.offset, range.bytes.len())?
            > usize::from(config.max_pages_per_window)
        {
            return Err(FigureGpuErrorV1::TooManyPages);
        }
        let prospective_pages =
            pose_page_count_for_ranges(current.iter().chain(std::iter::once(&range)))?;
        if !current.is_empty()
            && (current_bytes
                .checked_add(range.bytes.len())
                .ok_or(FigureGpuErrorV1::LengthOverflow)?
                > max_bytes
                || current.len() == max_ops
                || prospective_pages > usize::from(config.max_pages_per_window))
        {
            windows.push(finalize_window(
                u16::try_from(windows.len()).map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                std::mem::take(&mut current),
                packages.len(),
                config.max_pages_per_window,
            )?);
            current_bytes = 0;
        }
        current_bytes = current_bytes
            .checked_add(range.bytes.len())
            .ok_or(FigureGpuErrorV1::LengthOverflow)?;
        current.push(range);
    }
    if !current.is_empty() {
        windows.push(finalize_window(
            u16::try_from(windows.len()).map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
            current,
            packages.len(),
            config.max_pages_per_window,
        )?);
    }
    let mut staged = Vec::new();
    for window in &windows {
        staged.extend_from_slice(&window.ordinal.to_le_bytes());
        staged.extend_from_slice(&window.staged_digest);
    }
    Ok((windows, assignment_digest, Sha256::digest(staged).into()))
}

fn pose_page_count_for_range(
    kind: FigureGpuBufferKindV1,
    offset: u64,
    byte_len: usize,
) -> Result<usize, FigureGpuErrorV1> {
    if kind != FigureGpuBufferKindV1::Poses || byte_len == 0 {
        return Ok(0);
    }
    let page_bytes = u64::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    let end = offset
        .checked_add(u64::try_from(byte_len).map_err(|_| FigureGpuErrorV1::LengthOverflow)?)
        .and_then(|value| value.checked_sub(1))
        .ok_or(FigureGpuErrorV1::LengthOverflow)?;
    usize::try_from(end / page_bytes - offset / page_bytes + 1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)
}

fn pose_page_count_for_ranges<'a>(
    ranges: impl IntoIterator<Item = &'a FigureGpuUploadRangeV1>,
) -> Result<usize, FigureGpuErrorV1> {
    let page_bytes = u64::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
    let mut pages = BTreeSet::new();
    for range in ranges {
        if range.buffer_kind != FigureGpuBufferKindV1::Poses || range.bytes.is_empty() {
            continue;
        }
        let first = range.offset / page_bytes;
        let last = range
            .offset
            .checked_add(
                u64::try_from(range.bytes.len()).map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
            )
            .and_then(|value| value.checked_sub(1))
            .ok_or(FigureGpuErrorV1::LengthOverflow)?
            / page_bytes;
        for page in first..=last {
            pages.insert(page);
        }
    }
    Ok(pages.len())
}

fn finalize_window(
    ordinal: u16,
    ranges: Vec<FigureGpuUploadRangeV1>,
    package_count: usize,
    max_pages: u16,
) -> Result<FigureGpuUploadWindowV1, FigureGpuErrorV1> {
    let mut staged = Vec::new();
    let mut total_bytes = 0_usize;
    let mut pages = BTreeSet::new();
    for range in &ranges {
        staged.push(range.buffer_kind as u8);
        staged.extend_from_slice(&range.offset.to_le_bytes());
        staged.extend_from_slice(
            &u64::try_from(range.bytes.len())
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
                .to_le_bytes(),
        );
        staged.extend_from_slice(&range.bytes_digest);
        total_bytes = total_bytes
            .checked_add(range.bytes.len())
            .ok_or(FigureGpuErrorV1::LengthOverflow)?;
        if range.buffer_kind == FigureGpuBufferKindV1::Poses {
            let page_bytes = u64::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
                .map_err(|_| FigureGpuErrorV1::LengthOverflow)?;
            let first = range.offset / page_bytes;
            let last = range
                .offset
                .checked_add(
                    u64::try_from(range.bytes.len())
                        .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
                )
                .and_then(|value| value.checked_sub(1))
                .ok_or(FigureGpuErrorV1::LengthOverflow)?
                / page_bytes;
            for page in first..=last {
                pages.insert(page);
            }
        }
    }
    if pages.len() > usize::from(max_pages) {
        return Err(FigureGpuErrorV1::TooManyPages);
    }
    Ok(FigureGpuUploadWindowV1 {
        ordinal,
        total_bytes: u32::try_from(total_bytes).map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
        operation_count: u16::try_from(ranges.len())
            .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
        package_count: u16::try_from(package_count)
            .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
        pose_page_count: u16::try_from(pages.len())
            .map_err(|_| FigureGpuErrorV1::LengthOverflow)?,
        staged_digest: Sha256::digest(staged).into(),
        ranges,
    })
}

#[allow(clippy::too_many_arguments)]
fn plan_digest(
    config: FigureGpuPoolConfigV1,
    generation: u64,
    frame_digest: FigureGpuDigestV1,
    resource_set_digest: FigureGpuDigestV1,
    package_digest: FigureGpuDigestV1,
    package_receipt_digest: FigureGpuDigestV1,
    assignment_digest: FigureGpuDigestV1,
    staged_digest: FigureGpuDigestV1,
    windows: &[FigureGpuUploadWindowV1],
) -> Result<FigureGpuDigestV1, FigureGpuErrorV1> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PLAN_MAGIC);
    bytes.extend_from_slice(&FIGURE_GPU_ABI_VERSION_V1.to_le_bytes());
    bytes.extend_from_slice(&config.instance_capacity.to_le_bytes());
    bytes.extend_from_slice(&config.pose_page_capacity.to_le_bytes());
    bytes.extend_from_slice(&config.max_upload_bytes.to_le_bytes());
    bytes.extend_from_slice(&config.max_upload_ops.to_le_bytes());
    bytes.extend_from_slice(&config.max_packages_per_window.to_le_bytes());
    bytes.extend_from_slice(&config.max_pages_per_window.to_le_bytes());
    bytes.extend_from_slice(&generation.to_le_bytes());
    for digest in [
        frame_digest,
        resource_set_digest,
        package_digest,
        package_receipt_digest,
        assignment_digest,
        staged_digest,
    ] {
        bytes.extend_from_slice(&digest);
    }
    bytes.extend_from_slice(
        &u16::try_from(windows.len())
            .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
            .to_le_bytes(),
    );
    for window in windows {
        bytes.extend_from_slice(&window.ordinal.to_le_bytes());
        bytes.extend_from_slice(&window.staged_digest);
    }
    Ok(Sha256::digest(bytes).into())
}

fn assignment_digest_of(
    assignments: &[FigureGpuAssignmentV1],
) -> Result<FigureGpuDigestV1, FigureGpuErrorV1> {
    let bytes_per_assignment = 32_usize
        .checked_add(32)
        .and_then(|value| value.checked_add(12))
        .ok_or(FigureGpuErrorV1::LengthOverflow)?;
    let capacity = assignments
        .len()
        .checked_mul(bytes_per_assignment)
        .ok_or(FigureGpuErrorV1::LengthOverflow)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| FigureGpuErrorV1::AllocationFailure)?;
    for assignment in assignments {
        bytes.extend_from_slice(&assignment.semantic_entity);
        bytes.extend_from_slice(&assignment.package_digest);
        bytes.extend_from_slice(&assignment.slot.instance_slot.to_le_bytes());
        bytes.extend_from_slice(&assignment.slot.pose_page.to_le_bytes());
        bytes.extend_from_slice(&assignment.slot.pose_offset.to_le_bytes());
    }
    Ok(Sha256::digest(bytes).into())
}

fn package_receipt_digest(
    receipt: &PackageReceiptV1,
) -> Result<FigureGpuDigestV1, FigureGpuErrorV1> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BSTRPR01");
    bytes.extend_from_slice(&receipt.generation.run_epoch.to_le_bytes());
    bytes.extend_from_slice(&receipt.generation.client_applied_generation.to_le_bytes());
    bytes.extend_from_slice(&receipt.generation.simulation_tick.to_le_bytes());
    bytes.extend_from_slice(&receipt.generation.coherent_snapshot_root);
    for digest in [
        receipt.frame_digest,
        receipt.resource_set_digest,
        receipt.package_digest,
        receipt.authority_digest,
    ] {
        bytes.extend_from_slice(&digest);
    }
    bytes.extend_from_slice(
        &u16::try_from(receipt.required_section_identities.len())
            .map_err(|_| FigureGpuErrorV1::LengthOverflow)?
            .to_le_bytes(),
    );
    for section in &receipt.required_section_identities {
        bytes.extend_from_slice(section);
    }
    bytes.push(match receipt.cache_terminal {
        crate::figure_asset::CachePublicationTerminalV1::Published => 1,
        crate::figure_asset::CachePublicationTerminalV1::ExistingIdentical => 2,
        crate::figure_asset::CachePublicationTerminalV1::RolledBack => 3,
    });
    bytes.push(match receipt.terminal {
        PackageReceiptTerminalV1::Accepted => 1,
        PackageReceiptTerminalV1::RolledBack => 2,
    });
    Ok(Sha256::digest(bytes).into())
}

fn is_zero(value: &FigureGpuDigestV1) -> bool { value.iter().all(|byte| *byte == 0) }

fn bytes_at(bytes: &[u8], offset: usize, count: usize) -> Result<&[u8], FigureGpuErrorV1> {
    let end = offset
        .checked_add(count)
        .ok_or(FigureGpuErrorV1::LengthOverflow)?;
    bytes.get(offset..end).ok_or(FigureGpuErrorV1::Truncated)
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, FigureGpuErrorV1> {
    Ok(u16::from_le_bytes(
        bytes_at(bytes, offset, 2)?
            .try_into()
            .map_err(|_| FigureGpuErrorV1::Truncated)?,
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, FigureGpuErrorV1> {
    Ok(u32::from_le_bytes(
        bytes_at(bytes, offset, 4)?
            .try_into()
            .map_err(|_| FigureGpuErrorV1::Truncated)?,
    ))
}

fn i32_at(bytes: &[u8], offset: usize) -> Result<i32, FigureGpuErrorV1> {
    Ok(i32::from_le_bytes(
        bytes_at(bytes, offset, 4)?
            .try_into()
            .map_err(|_| FigureGpuErrorV1::Truncated)?,
    ))
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64, FigureGpuErrorV1> {
    Ok(u64::from_le_bytes(
        bytes_at(bytes, offset, 8)?
            .try_into()
            .map_err(|_| FigureGpuErrorV1::Truncated)?,
    ))
}

fn digest_at(bytes: &[u8], offset: usize) -> Result<FigureGpuDigestV1, FigureGpuErrorV1> {
    bytes_at(bytes, offset, 32)?
        .try_into()
        .map_err(|_| FigureGpuErrorV1::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        figure_asset::{
            CachePublicationRecordV1, CachePublicationTerminalV1, FigureAssetRoleV1,
            FigurePackageTargetV1, FigureSourceInputV1, MaterialBindingV1, MaterialKindV1,
            PackageReceiptV1,
        },
        presentation::{
            PresentationEnvironmentV1, PresentationFrameDraftV1, PresentationGenerationV1,
            PresentationVisualPolicyV1,
        },
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

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

    fn frame(package: &CompiledFigurePackageV1, generation: u64) -> PresentationFrameV1 {
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: generation,
                simulation_tick: 300,
                coherent_snapshot_root: digest(12),
            },
            entities: Vec::new(),
            groups: Vec::new(),
            events: Vec::new(),
            environment: PresentationEnvironmentV1 {
                terrain_root: digest(13),
                environment_digest: digest(14),
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 500,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: digest(15),
                terrain_view_distance: 16,
                entity_view_distance: 16,
                figure_lod_distance: 16,
                sprite_distance: 16,
                particles_enabled: false,
                weapon_trails_enabled: false,
                flashing_lights_enabled: false,
            },
            renderer_required_resources: vec![package.package_digest()],
            complete: true,
        }
        .seal()
        .unwrap()
    }

    fn package_receipt(
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
    ) -> PackageReceiptV1 {
        PackageReceiptV1::from_publication(frame, package, &CachePublicationRecordV1 {
            authority_digest: package.authority_digest(),
            package_digest: package.package_digest(),
            terminal: CachePublicationTerminalV1::Published,
        })
        .unwrap()
    }

    fn bone(value: i32) -> FigureGpuBoneV1 {
        FigureGpuBoneV1 {
            matrix_q20: [value; FIGURE_GPU_BONE_COMPONENTS_V1],
        }
    }

    fn entity(
        package: &CompiledFigurePackageV1,
        generation: u64,
        identity: u8,
    ) -> FigureGpuEntityInputV1 {
        FigureGpuEntityInputV1 {
            generation,
            semantic_entity: digest(identity),
            package_digest: package.package_digest(),
            authority_digest: package.authority_digest(),
            composition_digest: digest(20),
            palette_digest: digest(21),
            transform_digest: digest(22),
            pose_digest: digest(identity.wrapping_add(1)),
            lod_level: 0,
            section_id: 1,
            material_id: 1,
            flags: 0,
            bones: vec![bone(i32::from(identity))],
        }
    }

    fn accepted(
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
        package_receipt: &PackageReceiptV1,
        plan: &FigureGpuUploadPlanV1,
        sequence: u64,
    ) -> UploadReceiptV1 {
        let submission = SubmissionIdentityV1::for_plan(sequence, plan).unwrap();
        UploadReceiptV1::from_backend_completion(
            frame,
            package,
            package_receipt,
            plan,
            submission,
            BackendCompletionV1::Completed(submission),
        )
        .unwrap()
    }

    #[test]
    fn frozen_instance_pose_and_receipt_vectors_round_trip() {
        let package = package();
        let frame = frame(&package, 1);
        let receipt = package_receipt(&frame, &package);
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let staged = pool
            .begin_generation(&frame, &package, &receipt, vec![entity(&package, 1, 30)])
            .unwrap();
        let (instance, pose) = &staged.records()[0];
        let instance_bytes = instance.canonical_bytes().unwrap();
        let pose_bytes = pose.canonical_bytes().unwrap();
        assert_eq!(
            FigureGpuInstanceRecordV1::decode_exact(
                &instance_bytes,
                package.authority_digest(),
                instance.input.bones.clone(),
            )
            .unwrap(),
            *instance
        );
        assert_eq!(
            FigureGpuPoseRecordV1::decode_exact(&pose_bytes).unwrap(),
            *pose
        );
        let upload = accepted(&frame, &package, &receipt, &staged.plan, 1);
        let bytes = upload.canonical_bytes().unwrap();
        assert_eq!(UploadReceiptV1::decode_exact(&bytes).unwrap(), upload);
        assert_eq!(
            hex(&Sha256::digest(instance_bytes)),
            "af78c3cc7356b694349f27df4623982fb57d47da2d610a3fc77323ece51120c2"
        );
        assert_eq!(
            hex(&Sha256::digest(pose_bytes)),
            "68a81da6a3df8988fcd4a8dfbe1e87e1b571f23e191b1868e801157c68f45d96"
        );
    }

    #[test]
    fn producer_order_cannot_change_slots_or_upload_plan() {
        let package = package();
        let frame = frame(&package, 1);
        let receipt = package_receipt(&frame, &package);
        let inputs = vec![entity(&package, 1, 40), entity(&package, 1, 30)];
        let mut a = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let mut b = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let plan_a = a
            .begin_generation(&frame, &package, &receipt, inputs.clone())
            .unwrap();
        let plan_b = b
            .begin_generation(
                &frame,
                &package,
                &receipt,
                inputs.into_iter().rev().collect(),
            )
            .unwrap();
        assert_eq!(plan_a.plan, plan_b.plan);
        assert_eq!(plan_a.records, plan_b.records);
    }

    #[test]
    fn generation_isolation_capacity_and_held_reader_are_fail_closed() {
        let config = FigureGpuPoolConfigV1 {
            instance_capacity: 2,
            pose_page_capacity: 1,
            ..FigureGpuPoolConfigV1::default()
        };
        let package = package();
        let frame1 = frame(&package, 1);
        let receipt1 = package_receipt(&frame1, &package);
        let mut pool = FigureGpuPoolV1::new(config).unwrap();
        let staged1 = pool
            .begin_generation(&frame1, &package, &receipt1, vec![entity(&package, 1, 30)])
            .unwrap();
        let upload1 = accepted(&frame1, &package, &receipt1, &staged1.plan, 1);
        pool.commit_pending(upload1.clone()).unwrap();
        let held = pool.held_generation(1).unwrap();

        let frame2 = frame(&package, 2);
        let receipt2 = package_receipt(&frame2, &package);
        let staged2 = pool
            .begin_generation(&frame2, &package, &receipt2, vec![entity(&package, 2, 31)])
            .unwrap();
        assert_ne!(
            staged1.plan.assignments[0].slot,
            staged2.plan.assignments[0].slot
        );
        let upload2 = accepted(&frame2, &package, &receipt2, &staged2.plan, 2);
        pool.commit_pending(upload2).unwrap();
        assert_eq!(
            pool.retire_generation(
                1,
                &BackendCompletionV1::Completed(upload1.completion_identity)
            ),
            Err(FigureGpuErrorV1::HeldGeneration)
        );
        drop(staged1);
        drop(held);
        pool.retire_generation(
            1,
            &BackendCompletionV1::Completed(upload1.completion_identity),
        )
        .unwrap();
    }

    #[test]
    fn deterministic_coalescing_and_budget_splits_are_explicit() {
        let package = package();
        let frame = frame(&package, 1);
        let receipt = package_receipt(&frame, &package);
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1 {
            max_upload_bytes: 2_048,
            max_upload_ops: 1,
            ..FigureGpuPoolConfigV1::default()
        })
        .unwrap();
        let staged = pool
            .begin_generation(&frame, &package, &receipt, vec![
                entity(&package, 1, 30),
                entity(&package, 1, 31),
            ])
            .unwrap();
        assert_eq!(staged.plan.windows.len(), 2);
        assert_eq!(staged.plan.windows[0].total_bytes, 512);
        assert_eq!(staged.plan.windows[1].total_bytes, 2_048);
    }

    #[test]
    fn partial_wrong_stale_device_loss_and_rollback_never_complete() {
        let package = package();
        let frame = frame(&package, 1);
        let receipt = package_receipt(&frame, &package);
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let staged = pool
            .begin_generation(&frame, &package, &receipt, vec![entity(&package, 1, 30)])
            .unwrap();
        let submission = SubmissionIdentityV1::for_plan(1, &staged.plan).unwrap();
        assert_eq!(
            UploadReceiptV1::from_backend_completion(
                &frame,
                &package,
                &receipt,
                &staged.plan,
                submission,
                BackendCompletionV1::Incomplete,
            ),
            Err(FigureGpuErrorV1::IncompleteSubmission)
        );
        assert_eq!(
            UploadReceiptV1::from_backend_completion(
                &frame,
                &package,
                &receipt,
                &staged.plan,
                submission,
                BackendCompletionV1::DeviceLost,
            ),
            Err(FigureGpuErrorV1::DeviceLost)
        );
        let wrong = SubmissionIdentityV1 {
            sequence: 2,
            digest: digest(99),
        };
        assert_eq!(
            UploadReceiptV1::from_backend_completion(
                &frame,
                &package,
                &receipt,
                &staged.plan,
                submission,
                BackendCompletionV1::Completed(wrong),
            ),
            Err(FigureGpuErrorV1::CompletionIdentityMismatch)
        );
        let plan_digest = staged.plan.plan_digest;
        pool.rollback_pending(plan_digest).unwrap();
        assert_eq!(pool.live_generation_count(), 0);
        assert!(pool.pending().is_none());
    }

    #[test]
    fn duplicate_capacity_and_mutated_upload_ranges_fail_closed() {
        let package = package();
        let frame1 = frame(&package, 1);
        let receipt = package_receipt(&frame1, &package);
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1 {
            instance_capacity: 1,
            pose_page_capacity: 1,
            ..FigureGpuPoolConfigV1::default()
        })
        .unwrap();
        assert_eq!(
            pool.begin_generation(&frame1, &package, &receipt, vec![
                entity(&package, 1, 30),
                entity(&package, 1, 30)
            ],)
                .unwrap_err(),
            FigureGpuErrorV1::DuplicateSemanticEntity
        );
        let staged = pool
            .begin_generation(&frame1, &package, &receipt, vec![entity(&package, 1, 30)])
            .unwrap();
        let mut mutated = staged.plan.clone();
        mutated.windows[0].ranges[0].bytes[0] ^= 1;
        assert_eq!(mutated.validate(), Err(FigureGpuErrorV1::DigestMismatch));
        let upload = accepted(&frame1, &package, &receipt, &staged.plan, 1);
        pool.commit_pending(upload).unwrap();
        drop(staged);

        let frame2 = frame(&package, 2);
        let receipt2 = package_receipt(&frame2, &package);
        assert!(matches!(
            pool.begin_generation(&frame2, &package, &receipt2, vec![entity(&package, 2, 31)]),
            Err(FigureGpuErrorV1::PoolCapacity)
        ));
    }

    #[test]
    fn rollback_of_new_upload_preserves_the_previous_admitted_generation() {
        let package = package();
        let frame1 = frame(&package, 1);
        let receipt1 = package_receipt(&frame1, &package);
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1 {
            instance_capacity: 2,
            pose_page_capacity: 1,
            ..FigureGpuPoolConfigV1::default()
        })
        .unwrap();
        let staged1 = pool
            .begin_generation(&frame1, &package, &receipt1, vec![entity(&package, 1, 30)])
            .unwrap();
        let upload1 = accepted(&frame1, &package, &receipt1, &staged1.plan, 1);
        pool.commit_pending(upload1).unwrap();
        drop(staged1);

        let frame2 = frame(&package, 2);
        let receipt2 = package_receipt(&frame2, &package);
        let staged2 = pool
            .begin_generation(&frame2, &package, &receipt2, vec![entity(&package, 2, 31)])
            .unwrap();
        let rejected_plan = staged2.plan.plan_digest;
        pool.rollback_pending(rejected_plan).unwrap();
        drop(staged2);
        assert!(pool.held_generation(1).is_some());
        assert!(pool.held_generation(2).is_none());
        assert_eq!(pool.live_generation_count(), 1);
    }

    #[test]
    fn no_premature_reuse_and_exact_receipt_chain() {
        let package = package();
        let frame = frame(&package, 1);
        let receipt = package_receipt(&frame, &package);
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let staged = pool
            .begin_generation(&frame, &package, &receipt, vec![entity(&package, 1, 30)])
            .unwrap();
        assert!(matches!(
            pool.begin_generation(&frame, &package, &receipt, vec![entity(&package, 1, 30)]),
            Err(FigureGpuErrorV1::PendingGenerationExists)
        ));
        let upload = accepted(&frame, &package, &receipt, &staged.plan, 1);
        assert_eq!(
            upload.to_renderer_completion(&frame).unwrap(),
            RendererUploadCompletionV1 {
                client_applied_generation: 1,
                frame_digest: frame.frame_digest(),
                resource_set_digest: frame.resource_set_digest(),
                completed_resources: vec![package.package_digest()],
            }
        );
        pool.commit_pending(upload.clone()).unwrap();
        drop(staged);
        assert_eq!(
            pool.retire_generation(1, &BackendCompletionV1::Incomplete),
            Err(FigureGpuErrorV1::IncompleteSubmission)
        );
        assert_eq!(
            pool.retire_generation(1, &BackendCompletionV1::DeviceLost),
            Err(FigureGpuErrorV1::DeviceLost)
        );
    }

    fn hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }
}
