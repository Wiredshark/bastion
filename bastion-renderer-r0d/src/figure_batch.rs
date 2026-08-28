//! Deterministic R1BC figure draw classification and batch planning.
//!
//! This module turns an accepted presentation/package/upload chain into a
//! canonical, pass-complete draw plan. Runtime handles, ECS enumeration,
//! worker completion, callback order, and GPU timing never enter a key.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::{
    figure_gpu::{
        FIGURE_GPU_ABI_VERSION_V1, FigureGpuAssignmentV1, FigureGpuErrorV1, FigureGpuUploadPlanV1,
        UploadReceiptTerminalV1, UploadReceiptV1,
    },
    group_representation::{FormationKindV1, GroupRepresentationTierV1},
    individual_tier::AnimationTierV1,
};

pub const FIGURE_BATCH_SCHEMA_VERSION_V1: u16 = 1;
pub const FIGURE_BATCH_MAX_FIGURES_V1: usize = 4_096;
pub const FIGURE_BATCH_MAX_BATCHES_V1: usize = 12_288;
pub const FIGURE_BATCH_KEY_BYTES_V1: usize = 420;
pub const DRAW_INDEXED_INDIRECT_BYTES_V1: usize = 20;

pub type FigureBatchDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FigurePassV1 {
    Main = 1,
    Shadow = 2,
    Rain = 3,
}

impl FigurePassV1 {
    fn tag(self) -> u8 { self as u8 }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FigureFormV1 {
    Full = 1,
    Lod = 2,
    ShadowProxy = 3,
    Impostor = 4,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CullModeV1 {
    None = 1,
    Front = 2,
    Back = 3,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum DepthModeV1 {
    ReadWrite = 1,
    ReadOnly = 2,
    ShadowWrite = 3,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum BlendModeV1 {
    Opaque = 1,
    Alpha = 2,
    Additive = 3,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum PrimitiveTopologyV1 {
    TriangleList = 1,
    TriangleStrip = 2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum IndexFormatV1 {
    U16 = 1,
    U32 = 2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ShadowTierV1 {
    None = 1,
    MainMesh = 2,
    Proxy = 3,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum RainModeV1 {
    None = 1,
    Occluder = 2,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CompatibilityClassV1 {
    StorageInstanced = 1,
    ValidatedLegacy = 2,
}

/// Complete semantic key. Every field is checksum-bound and participates in
/// total ordering. Runtime buffer/texture pointers are deliberately absent.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FigureBatchKeyV1 {
    pub pass: FigurePassV1,
    pub pipeline_digest: FigureBatchDigestV1,
    pub shader_digest: FigureBatchDigestV1,
    pub package_digest: FigureBatchDigestV1,
    pub section_digest: FigureBatchDigestV1,
    pub mesh_digest: FigureBatchDigestV1,
    pub material_digest: FigureBatchDigestV1,
    pub palette_digest: FigureBatchDigestV1,
    pub texture_or_atlas_digest: FigureBatchDigestV1,
    pub sampler_digest: FigureBatchDigestV1,
    /// Zero/None for ungrouped figures. Grouped figures bind the exact
    /// immutable group plan and semantic group identity into the complete key.
    pub group_plan_digest: FigureBatchDigestV1,
    pub group_id: FigureBatchDigestV1,
    pub group_tier: Option<GroupRepresentationTierV1>,
    pub formation: Option<FormationKindV1>,
    pub form: FigureFormV1,
    pub lod_level: u16,
    pub animation_tier: AnimationTierV1,
    pub fade_phase: u16,
    pub abi_version: u16,
    pub instance_stride: u32,
    pub pose_page_bytes: u32,
    pub cull: CullModeV1,
    pub depth: DepthModeV1,
    pub blend: BlendModeV1,
    pub topology: PrimitiveTopologyV1,
    pub index_format: IndexFormatV1,
    pub index_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub shadow_tier: ShadowTierV1,
    pub rain_mode: RainModeV1,
    pub compatibility: CompatibilityClassV1,
}

impl FigureBatchKeyV1 {
    pub fn canonical_bytes(&self) -> Result<[u8; FIGURE_BATCH_KEY_BYTES_V1], FigureBatchErrorV1> {
        self.validate()?;
        let mut output = [0_u8; FIGURE_BATCH_KEY_BYTES_V1];
        output[0..8].copy_from_slice(b"BSTRBK01");
        output[8..10].copy_from_slice(&FIGURE_BATCH_SCHEMA_VERSION_V1.to_le_bytes());
        output[10] = self.pass.tag();
        output[11] = self.form as u8;
        let mut cursor = 12;
        for digest in [
            self.pipeline_digest,
            self.shader_digest,
            self.package_digest,
            self.section_digest,
            self.mesh_digest,
            self.material_digest,
            self.palette_digest,
            self.texture_or_atlas_digest,
            self.sampler_digest,
            self.group_plan_digest,
            self.group_id,
        ] {
            output[cursor..cursor + 32].copy_from_slice(&digest);
            cursor += 32;
        }
        output[cursor] = self.group_tier.map_or(0, |value| value as u8);
        cursor += 1;
        output[cursor] = self.formation.map_or(0, |value| value as u8);
        cursor += 1;
        output[cursor..cursor + 2].copy_from_slice(&self.lod_level.to_le_bytes());
        cursor += 2;
        output[cursor] = self.animation_tier as u8;
        cursor += 1;
        output[cursor..cursor + 2].copy_from_slice(&self.fade_phase.to_le_bytes());
        cursor += 2;
        output[cursor..cursor + 2].copy_from_slice(&self.abi_version.to_le_bytes());
        cursor += 2;
        output[cursor..cursor + 4].copy_from_slice(&self.instance_stride.to_le_bytes());
        cursor += 4;
        output[cursor..cursor + 4].copy_from_slice(&self.pose_page_bytes.to_le_bytes());
        cursor += 4;
        for tag in [
            self.cull as u8,
            self.depth as u8,
            self.blend as u8,
            self.topology as u8,
            self.index_format as u8,
        ] {
            output[cursor] = tag;
            cursor += 1;
        }
        output[cursor..cursor + 4].copy_from_slice(&self.index_count.to_le_bytes());
        cursor += 4;
        output[cursor..cursor + 4].copy_from_slice(&self.first_index.to_le_bytes());
        cursor += 4;
        output[cursor..cursor + 4].copy_from_slice(&self.base_vertex.to_le_bytes());
        cursor += 4;
        output[cursor] = self.shadow_tier as u8;
        cursor += 1;
        output[cursor] = self.rain_mode as u8;
        cursor += 1;
        output[cursor] = self.compatibility as u8;
        cursor += 1;
        debug_assert!(cursor <= FIGURE_BATCH_KEY_BYTES_V1);
        Ok(output)
    }

    pub fn digest(&self) -> Result<FigureBatchDigestV1, FigureBatchErrorV1> {
        Ok(Sha256::digest(self.canonical_bytes()?).into())
    }

    fn validate(&self) -> Result<(), FigureBatchErrorV1> {
        if [
            self.pipeline_digest,
            self.shader_digest,
            self.package_digest,
            self.section_digest,
            self.mesh_digest,
            self.material_digest,
            self.palette_digest,
            self.texture_or_atlas_digest,
            self.sampler_digest,
        ]
        .iter()
        .any(|digest| digest.iter().all(|byte| *byte == 0))
            || self.abi_version != FIGURE_GPU_ABI_VERSION_V1
            || self.instance_stride == 0
            || self.pose_page_bytes == 0
            || self.index_count == 0
        {
            return Err(FigureBatchErrorV1::InvalidBatchKey);
        }
        let group_absent = self.group_plan_digest == [0; 32]
            && self.group_id == [0; 32]
            && self.group_tier.is_none()
            && self.formation.is_none();
        let group_present = self.group_plan_digest != [0; 32]
            && self.group_id != [0; 32]
            && self.group_tier.is_some()
            && self.formation.is_some();
        if !group_absent && !group_present {
            return Err(FigureBatchErrorV1::InvalidBatchKey);
        }
        match self.pass {
            FigurePassV1::Main
                if self.shadow_tier != ShadowTierV1::None || self.rain_mode != RainModeV1::None =>
            {
                Err(FigureBatchErrorV1::InvalidBatchKey)
            },
            FigurePassV1::Shadow
                if self.shadow_tier == ShadowTierV1::None || self.rain_mode != RainModeV1::None =>
            {
                Err(FigureBatchErrorV1::InvalidBatchKey)
            },
            FigurePassV1::Rain
                if self.rain_mode != RainModeV1::Occluder
                    || self.shadow_tier != ShadowTierV1::None =>
            {
                Err(FigureBatchErrorV1::InvalidBatchKey)
            },
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FigurePassMaskV1(u8);

impl FigurePassMaskV1 {
    pub const ALL: Self = Self(7);
    pub const MAIN: Self = Self(1);
    pub const RAIN: Self = Self(4);
    pub const SHADOW: Self = Self(2);

    pub fn new(bits: u8) -> Result<Self, FigureBatchErrorV1> {
        if bits == 0 || bits & !Self::ALL.0 != 0 {
            return Err(FigureBatchErrorV1::InvalidPassMask);
        }
        Ok(Self(bits))
    }

    #[must_use]
    pub fn contains(self, pass: FigurePassV1) -> bool {
        self.0
            & match pass {
                FigurePassV1::Main => Self::MAIN.0,
                FigurePassV1::Shadow => Self::SHADOW.0,
                FigurePassV1::Rain => Self::RAIN.0,
            }
            != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FigureFallbackReasonV1 {
    ValidatedLegacyForm,
    MissingShadowProxy,
    UnsupportedRainMaterial,
    CapacityExceeded,
    IncompatibleRuntimeResource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureDrawInputV1 {
    pub semantic_entity: FigureBatchDigestV1,
    pub key: FigureBatchKeyV1,
    pub instance_slot: u32,
    pub pose_page: u32,
    pub pose_offset: u32,
    pub required_passes: FigurePassMaskV1,
    pub fallback_reason: Option<FigureFallbackReasonV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureBatchRecordV1 {
    pub key: FigureBatchKeyV1,
    pub key_digest: FigureBatchDigestV1,
    pub semantic_entities: Vec<FigureBatchDigestV1>,
    pub instance_slots: Vec<u32>,
    pub pose_assignments: Vec<(u32, u32)>,
    pub first_instance: u32,
    pub instance_count: u32,
    pub indirect_bytes: [u8; DRAW_INDEXED_INDIRECT_BYTES_V1],
    pub record_digest: FigureBatchDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureFallbackRecordV1 {
    pub pass: FigurePassV1,
    pub semantic_entity: FigureBatchDigestV1,
    pub reason: FigureFallbackReasonV1,
    pub key_digest: FigureBatchDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureBatchPlanV1 {
    pub generation: u64,
    pub frame_digest: FigureBatchDigestV1,
    pub resource_set_digest: FigureBatchDigestV1,
    pub frame_token: FigureBatchDigestV1,
    pub package_digest: FigureBatchDigestV1,
    pub upload_plan_digest: FigureBatchDigestV1,
    pub batches: Vec<FigureBatchRecordV1>,
    pub fallbacks: Vec<FigureFallbackRecordV1>,
    pub input_count: u32,
    pub required_draw_count: u32,
    pub batched_draw_count: u32,
    pub fallback_draw_count: u32,
    pub plan_digest: FigureBatchDigestV1,
}

impl FigureBatchPlanV1 {
    pub fn build(
        frame_token: FigureBatchDigestV1,
        upload_plan: &FigureGpuUploadPlanV1,
        upload_receipt: &UploadReceiptV1,
        mut inputs: Vec<FigureDrawInputV1>,
    ) -> Result<Self, FigureBatchErrorV1> {
        upload_plan.validate().map_err(FigureBatchErrorV1::Gpu)?;
        validate_receipt(upload_plan, upload_receipt)?;
        if frame_token.iter().all(|byte| *byte == 0) {
            return Err(FigureBatchErrorV1::InvalidFrameToken);
        }
        if inputs.is_empty() || inputs.len() > FIGURE_BATCH_MAX_FIGURES_V1 {
            return Err(FigureBatchErrorV1::FigureCountOutOfRange(inputs.len()));
        }
        inputs.sort_by(|left, right| {
            (left.semantic_entity, left.key.pass).cmp(&(right.semantic_entity, right.key.pass))
        });
        if inputs.windows(2).any(|pair| {
            pair[0].semantic_entity == pair[1].semantic_entity
                && pair[0].key.pass == pair[1].key.pass
        }) {
            return Err(FigureBatchErrorV1::DuplicateSemanticEntity);
        }
        let assignments = upload_plan
            .assignments
            .iter()
            .map(|value| (value.semantic_entity, value))
            .collect::<BTreeMap<_, _>>();
        let mut groups: BTreeMap<FigureBatchKeyV1, Vec<FigureDrawInputV1>> = BTreeMap::new();
        let mut fallbacks = Vec::new();
        let mut required_draw_count = 0_u32;
        for input in inputs.iter().cloned() {
            input.key.validate()?;
            if !input.required_passes.contains(input.key.pass) {
                return Err(FigureBatchErrorV1::UnexpectedPass);
            }
            let assignment = assignments
                .get(&input.semantic_entity)
                .ok_or(FigureBatchErrorV1::MissingGpuAssignment)?;
            validate_assignment(&input, assignment)?;
            required_draw_count = required_draw_count
                .checked_add(1)
                .ok_or(FigureBatchErrorV1::LengthOverflow)?;
            let reason = input.fallback_reason.or_else(|| {
                (input.key.compatibility == CompatibilityClassV1::ValidatedLegacy)
                    .then_some(FigureFallbackReasonV1::ValidatedLegacyForm)
            });
            if let Some(reason) = reason {
                fallbacks.push(FigureFallbackRecordV1 {
                    pass: input.key.pass,
                    semantic_entity: input.semantic_entity,
                    reason,
                    key_digest: input.key.digest()?,
                });
            } else {
                groups.entry(input.key.clone()).or_default().push(input);
            }
        }
        if groups.len() > FIGURE_BATCH_MAX_BATCHES_V1 {
            return Err(FigureBatchErrorV1::BatchCountOutOfRange(groups.len()));
        }

        let mut batches = Vec::with_capacity(groups.len());
        let mut next_instance = 0_u32;
        for (key, mut members) in groups {
            members.sort_by(|left, right| left.semantic_entity.cmp(&right.semantic_entity));
            let instance_count =
                u32::try_from(members.len()).map_err(|_| FigureBatchErrorV1::LengthOverflow)?;
            let indirect_bytes = indexed_indirect_bytes(
                key.index_count,
                instance_count,
                key.first_index,
                key.base_vertex,
                next_instance,
            );
            let key_digest = key.digest()?;
            let semantic_entities = members
                .iter()
                .map(|value| value.semantic_entity)
                .collect::<Vec<_>>();
            let instance_slots = members
                .iter()
                .map(|value| value.instance_slot)
                .collect::<Vec<_>>();
            let pose_assignments = members
                .iter()
                .map(|value| (value.pose_page, value.pose_offset))
                .collect::<Vec<_>>();
            let record_digest = record_digest(
                key_digest,
                &semantic_entities,
                &instance_slots,
                &pose_assignments,
                &indirect_bytes,
            );
            batches.push(FigureBatchRecordV1 {
                key,
                key_digest,
                semantic_entities,
                instance_slots,
                pose_assignments,
                first_instance: next_instance,
                instance_count,
                indirect_bytes,
                record_digest,
            });
            next_instance = next_instance
                .checked_add(instance_count)
                .ok_or(FigureBatchErrorV1::LengthOverflow)?;
        }
        fallbacks.sort_by(|left, right| {
            (left.pass, left.semantic_entity, left.reason).cmp(&(
                right.pass,
                right.semantic_entity,
                right.reason,
            ))
        });
        let batched_draw_count =
            u32::try_from(batches.len()).map_err(|_| FigureBatchErrorV1::LengthOverflow)?;
        let fallback_draw_count =
            u32::try_from(fallbacks.len()).map_err(|_| FigureBatchErrorV1::LengthOverflow)?;
        let input_count =
            u32::try_from(inputs.len()).map_err(|_| FigureBatchErrorV1::LengthOverflow)?;
        let plan_digest = plan_digest(
            upload_plan,
            frame_token,
            &batches,
            &fallbacks,
            input_count,
            required_draw_count,
        );
        Ok(Self {
            generation: upload_plan.generation,
            frame_digest: upload_plan.frame_digest,
            resource_set_digest: upload_plan.resource_set_digest,
            frame_token,
            package_digest: upload_plan.package_digest,
            upload_plan_digest: upload_plan.plan_digest,
            batches,
            fallbacks,
            input_count,
            required_draw_count,
            batched_draw_count,
            fallback_draw_count,
            plan_digest,
        })
    }

    pub fn validate_complete_pass_coverage(
        &self,
        requirements: &[(FigureBatchDigestV1, FigurePassMaskV1)],
    ) -> Result<(), FigureBatchErrorV1> {
        let mut observed = BTreeSet::new();
        for batch in &self.batches {
            for entity in &batch.semantic_entities {
                if !observed.insert((*entity, batch.key.pass)) {
                    return Err(FigureBatchErrorV1::DuplicatePassDraw);
                }
            }
        }
        for fallback in &self.fallbacks {
            if !observed.insert((fallback.semantic_entity, fallback.pass)) {
                return Err(FigureBatchErrorV1::DuplicatePassDraw);
            }
        }
        for (entity, mask) in requirements {
            for pass in [FigurePassV1::Main, FigurePassV1::Shadow, FigurePassV1::Rain] {
                if mask.contains(pass) != observed.contains(&(*entity, pass)) {
                    return Err(FigureBatchErrorV1::IncompletePassCoverage);
                }
            }
        }
        if observed.len()
            != usize::try_from(self.required_draw_count)
                .map_err(|_| FigureBatchErrorV1::LengthOverflow)?
        {
            return Err(FigureBatchErrorV1::IncompletePassCoverage);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FigureBatchErrorV1 {
    Gpu(FigureGpuErrorV1),
    InvalidBatchKey,
    InvalidFrameToken,
    InvalidPassMask,
    FigureCountOutOfRange(usize),
    BatchCountOutOfRange(usize),
    DuplicateSemanticEntity,
    MissingGpuAssignment,
    AssignmentMismatch,
    ReceiptMismatch,
    UnexpectedPass,
    DuplicatePassDraw,
    IncompletePassCoverage,
    LengthOverflow,
}

fn validate_receipt(
    plan: &FigureGpuUploadPlanV1,
    receipt: &UploadReceiptV1,
) -> Result<(), FigureBatchErrorV1> {
    if receipt.terminal != UploadReceiptTerminalV1::Accepted
        || receipt.generation != plan.generation
        || receipt.frame_digest != plan.frame_digest
        || receipt.resource_set_digest != plan.resource_set_digest
        || receipt.package_digest != plan.package_digest
        || receipt.package_receipt_digest != plan.package_receipt_digest
        || receipt.abi_version != FIGURE_GPU_ABI_VERSION_V1
        || receipt.assignment_digest != plan.assignment_digest
        || receipt.staged_digest != plan.staged_digest
        || receipt.plan_digest != plan.plan_digest
        || receipt.submission_identity != receipt.completion_identity
    {
        return Err(FigureBatchErrorV1::ReceiptMismatch);
    }
    Ok(())
}

fn validate_assignment(
    input: &FigureDrawInputV1,
    assignment: &FigureGpuAssignmentV1,
) -> Result<(), FigureBatchErrorV1> {
    if assignment.package_digest != input.key.package_digest
        || assignment.slot.instance_slot != input.instance_slot
        || assignment.slot.pose_page != input.pose_page
        || assignment.slot.pose_offset != input.pose_offset
    {
        return Err(FigureBatchErrorV1::AssignmentMismatch);
    }
    Ok(())
}

#[must_use]
pub fn indexed_indirect_bytes(
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
) -> [u8; DRAW_INDEXED_INDIRECT_BYTES_V1] {
    let mut bytes = [0_u8; DRAW_INDEXED_INDIRECT_BYTES_V1];
    bytes[0..4].copy_from_slice(&index_count.to_le_bytes());
    bytes[4..8].copy_from_slice(&instance_count.to_le_bytes());
    bytes[8..12].copy_from_slice(&first_index.to_le_bytes());
    bytes[12..16].copy_from_slice(&base_vertex.to_le_bytes());
    bytes[16..20].copy_from_slice(&first_instance.to_le_bytes());
    bytes
}

fn record_digest(
    key_digest: FigureBatchDigestV1,
    semantic_entities: &[FigureBatchDigestV1],
    instance_slots: &[u32],
    pose_assignments: &[(u32, u32)],
    indirect_bytes: &[u8; DRAW_INDEXED_INDIRECT_BYTES_V1],
) -> FigureBatchDigestV1 {
    let mut hasher = Sha256::new();
    hasher.update(b"BSTRBR01");
    hasher.update(key_digest);
    for ((entity, instance), (page, offset)) in semantic_entities
        .iter()
        .zip(instance_slots)
        .zip(pose_assignments)
    {
        hasher.update(entity);
        hasher.update(instance.to_le_bytes());
        hasher.update(page.to_le_bytes());
        hasher.update(offset.to_le_bytes());
    }
    hasher.update(indirect_bytes);
    hasher.finalize().into()
}

fn plan_digest(
    plan: &FigureGpuUploadPlanV1,
    frame_token: FigureBatchDigestV1,
    batches: &[FigureBatchRecordV1],
    fallbacks: &[FigureFallbackRecordV1],
    input_count: u32,
    required_draw_count: u32,
) -> FigureBatchDigestV1 {
    let mut hasher = Sha256::new();
    hasher.update(b"BSTRBP01");
    hasher.update(FIGURE_BATCH_SCHEMA_VERSION_V1.to_le_bytes());
    hasher.update(plan.generation.to_le_bytes());
    hasher.update(plan.frame_digest);
    hasher.update(plan.resource_set_digest);
    hasher.update(frame_token);
    hasher.update(plan.package_digest);
    hasher.update(plan.plan_digest);
    hasher.update(input_count.to_le_bytes());
    hasher.update(required_draw_count.to_le_bytes());
    for batch in batches {
        hasher.update(batch.record_digest);
    }
    for fallback in fallbacks {
        hasher.update([fallback.pass.tag(), fallback.reason as u8]);
        hasher.update(fallback.semantic_entity);
        hasher.update(fallback.key_digest);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        figure_asset::{
            CachePublicationRecordV1, CachePublicationTerminalV1, CompiledFigurePackageV1,
            FigureAssetRoleV1, FigurePackageTargetV1, FigureSourceInputV1, MaterialBindingV1,
            MaterialKindV1, PackageReceiptV1,
        },
        figure_gpu::{
            BackendCompletionV1, FigureGpuBoneV1, FigureGpuEntityInputV1, FigureGpuPoolConfigV1,
            FigureGpuPoolV1, SubmissionIdentityV1, UploadReceiptV1,
        },
        presentation::{
            PresentationEntityV1, PresentationEnvironmentV1, PresentationFrameDraftV1,
            PresentationGenerationV1, PresentationVisualPolicyV1,
        },
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn fixture(
        count: u8,
    ) -> (
        FigureGpuUploadPlanV1,
        UploadReceiptV1,
        Vec<FigureDrawInputV1>,
    ) {
        let package = CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Body,
            digest(2),
            digest(3),
            vec![MaterialBindingV1 {
                slot: 1,
                kind: MaterialKindV1::OpaqueVoxel,
                base_color_rgba: [10, 20, 30, 255],
                flags: 0,
            }],
            vec![FigureSourceInputV1 {
                logical_path: "figure/core/body.vox".into(),
                role: FigureAssetRoleV1::CoreBody,
                material_slot: 1,
                bytes: b"vox".to_vec(),
                deterministic_fixture: false,
            }],
        )
        .unwrap();
        let generation = PresentationGenerationV1 {
            run_epoch: 1,
            client_applied_generation: 7,
            simulation_tick: 300,
            coherent_snapshot_root: digest(5),
        };
        let entities = (1..=count)
            .map(|index| PresentationEntityV1 {
                semantic_id: digest(20 + index),
                figure_resource: package.package_digest(),
                group_id: None,
                position_mm: [i64::from(index), 0, 0],
                orientation_q30: [0, 0, 0, 1 << 30],
                scale_milli: 1_000,
                state_tag: 1,
                state_digest: digest(40 + index),
            })
            .collect();
        let frame = PresentationFrameDraftV1 {
            generation,
            entities,
            groups: vec![],
            events: vec![],
            environment: PresentationEnvironmentV1 {
                terrain_root: digest(6),
                environment_digest: digest(7),
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 500,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: digest(8),
                terrain_view_distance: 100,
                entity_view_distance: 100,
                figure_lod_distance: 100,
                sprite_distance: 100,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: true,
            },
            renderer_required_resources: vec![package.package_digest()],
            complete: true,
        }
        .seal()
        .unwrap();
        let publication = CachePublicationRecordV1 {
            authority_digest: package.authority_digest(),
            package_digest: package.package_digest(),
            terminal: CachePublicationTerminalV1::Published,
        };
        let package_receipt =
            PackageReceiptV1::from_publication(&frame, &package, &publication).unwrap();
        let gpu_inputs = frame
            .entities()
            .iter()
            .map(|entity| FigureGpuEntityInputV1 {
                generation: 7,
                semantic_entity: entity.semantic_id,
                package_digest: package.package_digest(),
                authority_digest: package.authority_digest(),
                composition_digest: digest(9),
                palette_digest: digest(10),
                transform_digest: digest(11),
                pose_digest: digest(12),
                lod_level: 0,
                section_id: 1,
                material_id: 1,
                flags: 0,
                bones: vec![FigureGpuBoneV1 {
                    matrix_q20: [1 << 20, 0, 0, 0, 0, 1 << 20, 0, 0, 0, 0, 1 << 20, 0],
                }],
            })
            .collect();
        let mut pool = FigureGpuPoolV1::new(FigureGpuPoolConfigV1::default()).unwrap();
        let staged = pool
            .begin_generation(&frame, &package, &package_receipt, gpu_inputs)
            .unwrap();
        let submission = SubmissionIdentityV1::for_plan(1, &staged.plan).unwrap();
        let receipt = UploadReceiptV1::from_backend_completion(
            &frame,
            &package,
            &package_receipt,
            &staged.plan,
            submission,
            BackendCompletionV1::Completed(submission),
        )
        .unwrap();
        let key = key(package.package_digest(), FigurePassV1::Main);
        let inputs = staged
            .plan
            .assignments
            .iter()
            .map(|assignment| FigureDrawInputV1 {
                semantic_entity: assignment.semantic_entity,
                key: key.clone(),
                instance_slot: assignment.slot.instance_slot,
                pose_page: assignment.slot.pose_page,
                pose_offset: assignment.slot.pose_offset,
                required_passes: FigurePassMaskV1::MAIN,
                fallback_reason: None,
            })
            .collect();
        (staged.plan.clone(), receipt, inputs)
    }

    fn key(package: [u8; 32], pass: FigurePassV1) -> FigureBatchKeyV1 {
        FigureBatchKeyV1 {
            pass,
            pipeline_digest: digest(70),
            shader_digest: digest(71),
            package_digest: package,
            section_digest: digest(72),
            mesh_digest: digest(73),
            material_digest: digest(74),
            palette_digest: digest(75),
            texture_or_atlas_digest: digest(76),
            sampler_digest: digest(77),
            group_plan_digest: [0; 32],
            group_id: [0; 32],
            group_tier: None,
            formation: None,
            form: FigureFormV1::Full,
            lod_level: 0,
            animation_tier: AnimationTierV1::EveryTick,
            fade_phase: 0,
            abi_version: FIGURE_GPU_ABI_VERSION_V1,
            instance_stride: 256,
            pose_page_bytes: 4_096,
            cull: CullModeV1::Back,
            depth: DepthModeV1::ReadWrite,
            blend: BlendModeV1::Opaque,
            topology: PrimitiveTopologyV1::TriangleList,
            index_format: IndexFormatV1::U32,
            index_count: 36,
            first_index: 4,
            base_vertex: -2,
            shadow_tier: if pass == FigurePassV1::Shadow {
                ShadowTierV1::MainMesh
            } else {
                ShadowTierV1::None
            },
            rain_mode: if pass == FigurePassV1::Rain {
                RainModeV1::Occluder
            } else {
                RainModeV1::None
            },
            compatibility: CompatibilityClassV1::StorageInstanced,
        }
    }

    #[test]
    fn complete_key_frozen_vector_and_every_field_changes_identity() {
        let baseline_key = key(digest(1), FigurePassV1::Main);
        let baseline = baseline_key.digest().unwrap();
        assert_eq!(
            crate::hex32(&baseline),
            "3b5f844b261e376e7b91f8766bc987d7c16a9c8c08ffb71f6a4933ef78b09f3a"
        );
        let variants = [
            {
                let mut v = baseline_key.clone();
                v.pipeline_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.shader_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.package_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.section_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.mesh_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.material_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.palette_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.texture_or_atlas_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.sampler_digest = digest(2);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.group_plan_digest = digest(2);
                v.group_id = digest(3);
                v.group_tier = Some(GroupRepresentationTierV1::FormationMiddle);
                v.formation = Some(FormationKindV1::Wedge);
                v
            },
            {
                let mut v = baseline_key.clone();
                v.form = FigureFormV1::Lod;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.lod_level = 1;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.animation_tier = AnimationTierV1::EverySecondTick;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.fade_phase = 1;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.instance_stride = 512;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.pose_page_bytes = 8192;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.cull = CullModeV1::Front;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.depth = DepthModeV1::ReadOnly;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.blend = BlendModeV1::Alpha;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.topology = PrimitiveTopologyV1::TriangleStrip;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.index_format = IndexFormatV1::U16;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.index_count = 37;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.first_index = 5;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.base_vertex = -1;
                v
            },
            {
                let mut v = baseline_key.clone();
                v.compatibility = CompatibilityClassV1::ValidatedLegacy;
                v
            },
        ];
        for variant in variants {
            assert_ne!(variant.digest().unwrap(), baseline);
        }
        assert_ne!(
            key(digest(1), FigurePassV1::Shadow).digest().unwrap(),
            baseline
        );
        assert_ne!(
            key(digest(1), FigurePassV1::Rain).digest().unwrap(),
            baseline
        );
        let mut shadow_proxy = key(digest(1), FigurePassV1::Shadow);
        let shadow_main = shadow_proxy.digest().unwrap();
        shadow_proxy.shadow_tier = ShadowTierV1::Proxy;
        assert_ne!(shadow_proxy.digest().unwrap(), shadow_main);
        let mut illegal_rain = key(digest(1), FigurePassV1::Rain);
        illegal_rain.rain_mode = RainModeV1::None;
        assert_eq!(
            illegal_rain.digest(),
            Err(FigureBatchErrorV1::InvalidBatchKey)
        );
        let mut wrong_abi = baseline_key;
        wrong_abi.abi_version = 2;
        assert_eq!(wrong_abi.digest(), Err(FigureBatchErrorV1::InvalidBatchKey));
    }

    #[test]
    fn producer_and_worker_partition_order_do_not_change_plan() {
        let (plan, receipt, inputs) = fixture(8);
        let expected =
            FigureBatchPlanV1::build(digest(90), &plan, &receipt, inputs.clone()).unwrap();
        for order in [
            inputs.iter().rev().cloned().collect::<Vec<_>>(),
            inputs[0..3]
                .iter()
                .chain(&inputs[6..8])
                .chain(&inputs[3..6])
                .cloned()
                .collect(),
        ] {
            let actual = FigureBatchPlanV1::build(digest(90), &plan, &receipt, order).unwrap();
            assert_eq!(actual, expected);
        }
        assert_eq!(expected.batches.len(), 1);
        assert_eq!(expected.batches[0].instance_count, 8);
        assert_eq!(
            expected.batches[0].indirect_bytes,
            indexed_indirect_bytes(36, 8, 4, -2, 0)
        );
    }

    #[test]
    fn stable_groups_ranges_fallbacks_and_complete_pass_coverage() {
        let (plan, receipt, base) = fixture(2);
        let assignments = &plan.assignments;
        let mut inputs = Vec::new();
        for (index, _assignment) in assignments.iter().enumerate() {
            for pass in [FigurePassV1::Main, FigurePassV1::Shadow, FigurePassV1::Rain] {
                let mut value = base[index].clone();
                value.key = key(plan.package_digest, pass);
                value.required_passes = FigurePassMaskV1::ALL;
                if index == 1 && pass == FigurePassV1::Rain {
                    value.key.compatibility = CompatibilityClassV1::ValidatedLegacy;
                    value.fallback_reason = Some(FigureFallbackReasonV1::UnsupportedRainMaterial);
                }
                inputs.push(value);
            }
        }
        let result = FigureBatchPlanV1::build(digest(91), &plan, &receipt, inputs).unwrap();
        assert_eq!(result.batches.len(), 3);
        assert_eq!(result.fallbacks.len(), 1);
        assert_eq!(result.required_draw_count, 6);
        assert_eq!(
            result
                .batches
                .iter()
                .map(|batch| batch.instance_count)
                .sum::<u32>(),
            5
        );
        assert_eq!(result.fallback_draw_count, 1);
        let requirements = assignments
            .iter()
            .map(|assignment| (assignment.semantic_entity, FigurePassMaskV1::ALL))
            .collect::<Vec<_>>();
        result
            .validate_complete_pass_coverage(&requirements)
            .unwrap();
    }

    #[test]
    fn duplicate_mismatch_stale_and_missing_assignment_fail_closed() {
        let (plan, receipt, inputs) = fixture(2);
        let mut duplicate = inputs.clone();
        duplicate[1].semantic_entity = duplicate[0].semantic_entity;
        assert_eq!(
            FigureBatchPlanV1::build(digest(90), &plan, &receipt, duplicate),
            Err(FigureBatchErrorV1::DuplicateSemanticEntity)
        );
        let mut mismatch = inputs.clone();
        mismatch[0].instance_slot += 1;
        assert_eq!(
            FigureBatchPlanV1::build(digest(90), &plan, &receipt, mismatch),
            Err(FigureBatchErrorV1::AssignmentMismatch)
        );
        let mut stale = receipt.clone();
        stale.generation += 1;
        assert_eq!(
            FigureBatchPlanV1::build(digest(90), &plan, &stale, inputs.clone()),
            Err(FigureBatchErrorV1::ReceiptMismatch)
        );
        let mut missing = inputs;
        missing[0].semantic_entity = digest(255);
        assert_eq!(
            FigureBatchPlanV1::build(digest(90), &plan, &receipt, missing),
            Err(FigureBatchErrorV1::MissingGpuAssignment)
        );
    }

    #[test]
    fn invalid_key_mask_capacity_and_coverage_fail_closed() {
        let (plan, receipt, mut inputs) = fixture(1);
        inputs[0].key.index_count = 0;
        assert_eq!(
            FigureBatchPlanV1::build(digest(90), &plan, &receipt, inputs),
            Err(FigureBatchErrorV1::InvalidBatchKey)
        );
        assert_eq!(
            FigurePassMaskV1::new(0),
            Err(FigureBatchErrorV1::InvalidPassMask)
        );
        let (plan, receipt, inputs) = fixture(1);
        let result = FigureBatchPlanV1::build(digest(90), &plan, &receipt, inputs).unwrap();
        assert_eq!(
            result.validate_complete_pass_coverage(&[(
                plan.assignments[0].semantic_entity,
                FigurePassMaskV1::ALL
            )]),
            Err(FigureBatchErrorV1::IncompletePassCoverage)
        );
    }
}
