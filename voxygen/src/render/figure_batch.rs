//! Production storage-instanced figure draw buffer.
//!
//! Each pass owns a disjoint fixed region, so queue writes for later passes
//! cannot alter earlier encoded draws. Allocation order comes from the
//! canonical CPU batch plan, never callback or ECS order.

use std::{
    ops::Range,
    sync::{Mutex, OnceLock},
};

use super::pipelines::figure::{FigureBatchInstance, FigureLayout};

const MAX_INSTANCES_PER_PASS: u32 = 4_096;
const PASS_COUNT: u64 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeFigurePassV1 {
    Main,
    Shadow,
    Rain,
}

impl RuntimeFigurePassV1 {
    const fn index(self) -> usize {
        match self {
            Self::Main => 0,
            Self::Shadow => 1,
            Self::Rain => 2,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FigureBatchProductionEvidenceV1 {
    pub visible_figures: u32,
    pub batch_count: u32,
    pub fallback_count: u32,
    pub legacy_draw_equivalent: u32,
    pub actual_draw_count: u32,
    pub main_batches: u32,
    pub shadow_batches: u32,
    pub rain_batches: u32,
    pub capacity_fallbacks: u32,
    pub incompatible_resource_fallbacks: u32,
}

static LATEST: OnceLock<Mutex<FigureBatchProductionEvidenceV1>> = OnceLock::new();

fn evidence() -> &'static Mutex<FigureBatchProductionEvidenceV1> {
    LATEST.get_or_init(|| Mutex::new(FigureBatchProductionEvidenceV1::default()))
}

pub(super) fn latest_evidence() -> FigureBatchProductionEvidenceV1 {
    evidence().lock().map_or_else(
        |_| FigureBatchProductionEvidenceV1::default(),
        |value| value.clone(),
    )
}

#[derive(Debug)]
pub enum FigureBatchRuntimeErrorV1 {
    Capacity,
    LengthOverflow,
    Cull(super::gpu_cull::ProductionCullFallbackV1),
    Indirect(super::indirect_draw::ProductionSubmissionErrorV1),
}

pub struct FigureBatchRuntimeV1 {
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    cursors: [u32; 3],
    gpu_cull: super::gpu_cull::GpuCullRuntimeV1,
    indirect_draw: super::indirect_draw::IndirectDrawRuntimeV1,
}

impl FigureBatchRuntimeV1 {
    pub fn new(
        device: &wgpu::Device,
        layout: &FigureLayout,
        compute_supported: bool,
        indirect_supported: bool,
        indirect_enabled: bool,
    ) -> Result<Self, FigureBatchRuntimeErrorV1> {
        let stride = u64::try_from(core::mem::size_of::<FigureBatchInstance>())
            .map_err(|_| FigureBatchRuntimeErrorV1::LengthOverflow)?;
        let size = stride
            .checked_mul(u64::from(MAX_INSTANCES_PER_PASS))
            .and_then(|value| value.checked_mul(PASS_COUNT))
            .ok_or(FigureBatchRuntimeErrorV1::LengthOverflow)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r1bc-figure-batch-instances-v1"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bastion-r1bc-figure-batch-bind-v1"),
            layout: &layout.batch_instances,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Ok(Self {
            buffer,
            bind_group,
            cursors: [0; 3],
            gpu_cull: super::gpu_cull::GpuCullRuntimeV1::new(device, compute_supported),
            indirect_draw: super::indirect_draw::IndirectDrawRuntimeV1::new(
                device,
                indirect_supported,
                indirect_enabled,
            )
            .map_err(FigureBatchRuntimeErrorV1::Indirect)?,
        })
    }

    pub fn begin_frame(&mut self) {
        self.cursors = [0; 3];
        self.indirect_draw.begin_frame();
        if let Ok(mut current) = evidence().lock() {
            *current = FigureBatchProductionEvidenceV1::default();
        }
    }

    pub fn stage(
        &mut self,
        queue: &wgpu::Queue,
        pass: RuntimeFigurePassV1,
        instances: &[FigureBatchInstance],
    ) -> Result<Range<u32>, FigureBatchRuntimeErrorV1> {
        if instances.is_empty() {
            return Err(FigureBatchRuntimeErrorV1::Capacity);
        }
        let count = u32::try_from(instances.len())
            .map_err(|_| FigureBatchRuntimeErrorV1::LengthOverflow)?;
        let index = pass.index();
        let start = self.cursors[index];
        let end = start
            .checked_add(count)
            .ok_or(FigureBatchRuntimeErrorV1::LengthOverflow)?;
        if end > MAX_INSTANCES_PER_PASS {
            if let Ok(mut current) = evidence().lock() {
                current.capacity_fallbacks = current.capacity_fallbacks.saturating_add(count);
            }
            return Err(FigureBatchRuntimeErrorV1::Capacity);
        }
        let global_start = u32::try_from(index)
            .ok()
            .and_then(|pass_index| pass_index.checked_mul(MAX_INSTANCES_PER_PASS))
            .and_then(|base| base.checked_add(start))
            .ok_or(FigureBatchRuntimeErrorV1::LengthOverflow)?;
        let offset = u64::from(global_start)
            .checked_mul(
                u64::try_from(core::mem::size_of::<FigureBatchInstance>())
                    .map_err(|_| FigureBatchRuntimeErrorV1::LengthOverflow)?,
            )
            .ok_or(FigureBatchRuntimeErrorV1::LengthOverflow)?;
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(instances));
        self.cursors[index] = end;
        if let Ok(mut current) = evidence().lock() {
            current.batch_count = current.batch_count.saturating_add(1);
            current.legacy_draw_equivalent = current.legacy_draw_equivalent.saturating_add(count);
            current.actual_draw_count = current.actual_draw_count.saturating_add(1);
            match pass {
                RuntimeFigurePassV1::Main => {
                    current.visible_figures = current.visible_figures.saturating_add(count);
                    current.main_batches = current.main_batches.saturating_add(1);
                },
                RuntimeFigurePassV1::Shadow => {
                    current.shadow_batches = current.shadow_batches.saturating_add(1);
                },
                RuntimeFigurePassV1::Rain => {
                    current.rain_batches = current.rain_batches.saturating_add(1);
                },
            }
        }
        Ok(global_start..global_start + count)
    }

    pub fn reconcile_cull(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &bastion_renderer_r0d::gpu_cull::CanonicalCullBatchV1,
    ) -> Result<bastion_renderer_r0d::gpu_cull::AcceleratorResultV1, FigureBatchRuntimeErrorV1>
    {
        match self.gpu_cull.reconcile(device, queue, batch) {
            Ok(result) => Ok(result),
            Err(error) => Err(FigureBatchRuntimeErrorV1::Cull(
                super::gpu_cull::record_error(batch, &error),
            )),
        }
    }

    pub fn stage_indirect(
        &mut self,
        queue: &wgpu::Queue,
        generation: u64,
        culling_result_digest: [u8; 32],
        reference: bastion_renderer_r0d::draw_submission::DirectDrawReferenceV1,
    ) -> Result<u64, FigureBatchRuntimeErrorV1> {
        self.indirect_draw
            .stage(queue, generation, culling_result_digest, reference)
            .map_err(FigureBatchRuntimeErrorV1::Indirect)
    }

    pub fn indirect_buffer(&self) -> &wgpu::Buffer { self.indirect_draw.buffer() }

    pub fn record_indirect_submission_failure(
        &self,
        fallback: bastion_renderer_r0d::draw_submission::SubmissionFallbackV1,
    ) {
        self.indirect_draw.record_submission_failure(fallback);
    }

    pub fn record_fallback(&self, incompatible_resource: bool) {
        if let Ok(mut current) = evidence().lock() {
            current.fallback_count = current.fallback_count.saturating_add(1);
            current.legacy_draw_equivalent = current.legacy_draw_equivalent.saturating_add(1);
            current.actual_draw_count = current.actual_draw_count.saturating_add(1);
            if incompatible_resource {
                current.incompatible_resource_fallbacks =
                    current.incompatible_resource_fallbacks.saturating_add(1);
            }
        }
    }
}
