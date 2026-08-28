//! Capability-gated production indexed-indirect submission.
//!
//! The canonical CPU batch remains the direct reference.  The runtime writes
//! exact frozen arguments, reconciles them from their structured fields, and
//! retains a typed direct fallback whenever indirect execution is unavailable
//! or the same-frame structural contract is not satisfied.

use std::sync::{Mutex, OnceLock};

use bastion_renderer_r0d::{
    draw_submission::{
        CanonicalSubmissionPlanV1, DirectDrawReferenceV1, SubmissionDigestV1, SubmissionErrorV1,
        SubmissionFallbackV1, SubmissionParityV1, SubmissionTerminalV1,
    },
    figure_batch::DRAW_INDEXED_INDIRECT_BYTES_V1,
};

const MAX_INDIRECT_DRAWS_PER_FRAME_V1: u32 = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionSubmissionEvidenceV1 {
    pub generation: u64,
    pub culling_result_digest: SubmissionDigestV1,
    pub plan_digest: SubmissionDigestV1,
    pub reference_digest: SubmissionDigestV1,
    pub indirect_digest: SubmissionDigestV1,
    pub reference_draw_count: u32,
    pub indirect_draw_count: u32,
    pub indirect_supported: bool,
    pub same_frame_parity: bool,
    pub terminal: SubmissionTerminalV1,
}

impl Default for ProductionSubmissionEvidenceV1 {
    fn default() -> Self {
        Self {
            generation: 0,
            culling_result_digest: [0; 32],
            plan_digest: [0; 32],
            reference_digest: [0; 32],
            indirect_digest: [0; 32],
            reference_draw_count: 0,
            indirect_draw_count: 0,
            indirect_supported: false,
            same_frame_parity: false,
            terminal: SubmissionTerminalV1::DirectFallback(
                SubmissionFallbackV1::UnsupportedCapability,
            ),
        }
    }
}

static LATEST: OnceLock<Mutex<ProductionSubmissionEvidenceV1>> = OnceLock::new();

fn evidence() -> &'static Mutex<ProductionSubmissionEvidenceV1> {
    LATEST.get_or_init(|| Mutex::new(ProductionSubmissionEvidenceV1::default()))
}

pub(crate) fn latest_evidence() -> ProductionSubmissionEvidenceV1 {
    evidence().lock().map_or_else(
        |_| ProductionSubmissionEvidenceV1::default(),
        |value| value.clone(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionSubmissionErrorV1 {
    ExplicitDirectReference,
    UnsupportedCapability,
    Capacity,
    Core(SubmissionErrorV1),
}

pub(crate) struct IndirectDrawRuntimeV1 {
    buffer: wgpu::Buffer,
    supported: bool,
    enabled: bool,
    generation: Option<u64>,
    culling_result_digest: SubmissionDigestV1,
    references: Vec<DirectDrawReferenceV1>,
    observed: Vec<[u8; DRAW_INDEXED_INDIRECT_BYTES_V1]>,
}

impl IndirectDrawRuntimeV1 {
    pub(crate) fn new(
        device: &wgpu::Device,
        supported: bool,
        enabled: bool,
    ) -> Result<Self, ProductionSubmissionErrorV1> {
        let size = u64::from(MAX_INDIRECT_DRAWS_PER_FRAME_V1)
            .checked_mul(
                u64::try_from(DRAW_INDEXED_INDIRECT_BYTES_V1)
                    .map_err(|_| ProductionSubmissionErrorV1::Capacity)?,
            )
            .ok_or(ProductionSubmissionErrorV1::Capacity)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r2-indexed-indirect-v1"),
            size,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Ok(Self {
            buffer,
            supported,
            enabled,
            generation: None,
            culling_result_digest: [0; 32],
            references: Vec::new(),
            observed: Vec::new(),
        })
    }

    pub(crate) fn begin_frame(&mut self) {
        self.generation = None;
        self.culling_result_digest = [0; 32];
        self.references.clear();
        self.observed.clear();
        if let Ok(mut current) = evidence().lock() {
            *current = ProductionSubmissionEvidenceV1 {
                indirect_supported: self.supported,
                ..ProductionSubmissionEvidenceV1::default()
            };
        }
    }

    pub(crate) fn stage(
        &mut self,
        queue: &wgpu::Queue,
        generation: u64,
        culling_result_digest: SubmissionDigestV1,
        reference: DirectDrawReferenceV1,
    ) -> Result<u64, ProductionSubmissionErrorV1> {
        if !self.supported {
            self.record_fallback(
                generation,
                culling_result_digest,
                SubmissionFallbackV1::UnsupportedCapability,
            );
            return Err(ProductionSubmissionErrorV1::UnsupportedCapability);
        }
        if !self.enabled {
            self.record_fallback(
                generation,
                culling_result_digest,
                SubmissionFallbackV1::ExplicitDirectReference,
            );
            return Err(ProductionSubmissionErrorV1::ExplicitDirectReference);
        }
        if self.references.len()
            >= usize::try_from(MAX_INDIRECT_DRAWS_PER_FRAME_V1)
                .map_err(|_| ProductionSubmissionErrorV1::Capacity)?
        {
            self.record_fallback(
                generation,
                culling_result_digest,
                SubmissionFallbackV1::Overflow,
            );
            return Err(ProductionSubmissionErrorV1::Capacity);
        }
        if let Some(existing) = self.generation {
            if existing != generation {
                let error = SubmissionErrorV1::StaleGeneration {
                    expected: existing,
                    actual: generation,
                };
                self.record_fallback(
                    generation,
                    culling_result_digest,
                    SubmissionFallbackV1::StaleGeneration,
                );
                return Err(ProductionSubmissionErrorV1::Core(error));
            }
            if self.culling_result_digest != culling_result_digest {
                self.record_fallback(
                    generation,
                    culling_result_digest,
                    SubmissionFallbackV1::StaleGeneration,
                );
                return Err(ProductionSubmissionErrorV1::Core(
                    SubmissionErrorV1::StaleCullingDigest,
                ));
            }
        }
        if self.references.last().is_some_and(|last| {
            (last.pass, last.batch_identity) >= (reference.pass, reference.batch_identity)
        }) {
            self.record_fallback(
                generation,
                culling_result_digest,
                SubmissionFallbackV1::Parity,
            );
            return Err(ProductionSubmissionErrorV1::Core(
                SubmissionErrorV1::DuplicateBatchIdentity,
            ));
        }

        let mut references = self.references.clone();
        references.push(reference);
        let plan =
            CanonicalSubmissionPlanV1::build(generation, culling_result_digest, references.clone())
                .map_err(ProductionSubmissionErrorV1::Core)?;
        let bytes = reference.indirect_bytes();
        let mut observed = self.observed.clone();
        observed.push(bytes);
        let parity = plan
            .reconcile_same_frame(generation, culling_result_digest, &observed)
            .map_err(ProductionSubmissionErrorV1::Core)?;
        let index = u64::try_from(self.references.len())
            .map_err(|_| ProductionSubmissionErrorV1::Capacity)?;
        let stride = u64::try_from(DRAW_INDEXED_INDIRECT_BYTES_V1)
            .map_err(|_| ProductionSubmissionErrorV1::Capacity)?;
        let offset = index
            .checked_mul(stride)
            .ok_or(ProductionSubmissionErrorV1::Capacity)?;
        queue.write_buffer(&self.buffer, offset, &bytes);

        self.generation = Some(generation);
        self.culling_result_digest = culling_result_digest;
        self.references = references;
        self.observed = observed;
        self.record_accepted(&plan, &parity);
        Ok(offset)
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer { &self.buffer }

    pub(crate) fn record_submission_failure(&self, fallback: SubmissionFallbackV1) {
        self.record_fallback(
            self.generation.unwrap_or(0),
            self.culling_result_digest,
            fallback,
        );
    }

    fn record_accepted(&self, plan: &CanonicalSubmissionPlanV1, parity: &SubmissionParityV1) {
        if let Ok(mut current) = evidence().lock() {
            *current = ProductionSubmissionEvidenceV1 {
                generation: plan.generation,
                culling_result_digest: plan.culling_result_digest,
                plan_digest: plan.plan_digest,
                reference_digest: parity.reference_digest,
                indirect_digest: parity.indirect_digest,
                reference_draw_count: parity.reference_draw_count,
                indirect_draw_count: parity.indirect_draw_count,
                indirect_supported: self.supported,
                same_frame_parity: parity.same_frame_parity,
                terminal: SubmissionTerminalV1::IndirectAccepted,
            };
        }
    }

    fn record_fallback(
        &self,
        generation: u64,
        culling_result_digest: SubmissionDigestV1,
        fallback: SubmissionFallbackV1,
    ) {
        if let Ok(mut current) = evidence().lock() {
            *current = ProductionSubmissionEvidenceV1 {
                generation,
                culling_result_digest,
                indirect_supported: self.supported,
                terminal: SubmissionTerminalV1::DirectFallback(fallback),
                ..ProductionSubmissionEvidenceV1::default()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bastion_renderer_r0d::{
        draw_submission::{CanonicalSubmissionPlanV1, DirectDrawReferenceV1},
        figure_batch::FigurePassV1,
    };
    use wgpu::util::DeviceExt;

    use super::*;

    const READBACK_TIMEOUT: Duration = Duration::from_secs(20);
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn reference(first_instance: u32) -> DirectDrawReferenceV1 {
        DirectDrawReferenceV1::new(FigurePassV1::Main, digest(2), 3, 1, 0, 0, first_instance)
            .unwrap()
    }

    #[test]
    fn actual_wgpu_indexed_indirect_executes_and_mismatch_fails_closed() {
        let _guard = test_guard();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = runtime
            .block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .expect("an actual wgpu adapter is required for the R2 indirect canary");
        assert!(
            adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION),
            "selected adapter does not support indirect execution"
        );
        assert!(
            adapter
                .features()
                .contains(wgpu::Features::INDIRECT_FIRST_INSTANCE),
            "selected adapter does not support nonzero indirect first_instance"
        );
        let requested_features = wgpu::Features::INDIRECT_FIRST_INSTANCE;
        let (device, queue) = runtime
            .block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bastion-r2-indirect-test"),
                required_features: requested_features,
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            }))
            .unwrap();
        let mut indirect = IndirectDrawRuntimeV1::new(&device, true, true).unwrap();
        indirect.begin_frame();
        let offset = indirect.stage(&queue, 7, digest(9), reference(1)).unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bastion-r2-indirect-test-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#
                .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bastion-r2-indirect-test-layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bastion-r2-indirect-test-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bastion-r2-indirect-test-index"),
            contents: bytemuck::cast_slice(&[0_u16, 1, 2]),
            usage: wgpu::BufferUsages::INDEX,
        });
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bastion-r2-indirect-test-target"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r2-indirect-test-readback"),
            size: 256 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bastion-r2-indirect-test-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bastion-r2-indirect-test-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed_indirect(indirect.buffer(), offset);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        );
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(READBACK_TIMEOUT),
            })
            .unwrap();
        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(READBACK_TIMEOUT),
            })
            .unwrap();
        receiver.recv_timeout(READBACK_TIMEOUT).unwrap().unwrap();
        let pixels = slice.get_mapped_range();
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel[0] > 0),
            "real indexed-indirect draw produced no red pixels"
        );
        drop(pixels);
        readback.unmap();

        let persisted = latest_evidence();
        assert_eq!(persisted.terminal, SubmissionTerminalV1::IndirectAccepted);
        assert_eq!(persisted.reference_draw_count, 1);
        assert_eq!(persisted.indirect_draw_count, 1);
        assert!(persisted.same_frame_parity);

        let plan = CanonicalSubmissionPlanV1::build(7, digest(9), vec![reference(1)]).unwrap();
        let mut injected = [plan.records[0].reference.indirect_bytes()];
        injected[0][0] ^= 1;
        assert_eq!(
            plan.reconcile_same_frame(7, digest(9), &injected),
            Err(SubmissionErrorV1::StructuralParity { index: 0 })
        );
        indirect.record_submission_failure(SubmissionFallbackV1::Parity);
        let rejected = latest_evidence();
        assert_eq!(
            rejected.terminal,
            SubmissionTerminalV1::DirectFallback(SubmissionFallbackV1::Parity)
        );
        assert!(!rejected.same_frame_parity);
        for fallback in [
            SubmissionFallbackV1::DeviceLoss,
            SubmissionFallbackV1::SubmissionFailure,
        ] {
            indirect.record_submission_failure(fallback);
            let fault = latest_evidence();
            assert_eq!(
                fault.terminal,
                SubmissionTerminalV1::DirectFallback(fallback)
            );
            assert!(!fault.same_frame_parity);
        }
    }

    #[test]
    fn unsupported_and_stale_paths_are_typed_fallbacks() {
        let _guard = test_guard();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = runtime
            .block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .unwrap();
        let (device, queue) = runtime
            .block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bastion-r2-indirect-fallback-test"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            }))
            .unwrap();
        let mut unsupported = IndirectDrawRuntimeV1::new(&device, false, true).unwrap();
        unsupported.begin_frame();
        assert_eq!(
            unsupported.stage(&queue, 7, digest(9), reference(0)),
            Err(ProductionSubmissionErrorV1::UnsupportedCapability)
        );
        assert_eq!(
            latest_evidence().terminal,
            SubmissionTerminalV1::DirectFallback(SubmissionFallbackV1::UnsupportedCapability)
        );
        let mut direct = IndirectDrawRuntimeV1::new(&device, true, false).unwrap();
        direct.begin_frame();
        assert_eq!(
            direct.stage(&queue, 7, digest(9), reference(0)),
            Err(ProductionSubmissionErrorV1::ExplicitDirectReference)
        );
        let direct_evidence = latest_evidence();
        assert!(direct_evidence.indirect_supported);
        assert_eq!(
            direct_evidence.terminal,
            SubmissionTerminalV1::DirectFallback(SubmissionFallbackV1::ExplicitDirectReference)
        );

        let mut supported = IndirectDrawRuntimeV1::new(&device, true, true).unwrap();
        supported.begin_frame();
        supported.stage(&queue, 7, digest(9), reference(0)).unwrap();
        assert!(matches!(
            supported.stage(
                &queue,
                8,
                digest(9),
                DirectDrawReferenceV1::new(FigurePassV1::Main, digest(3), 3, 1, 0, 0, 1).unwrap()
            ),
            Err(ProductionSubmissionErrorV1::Core(
                SubmissionErrorV1::StaleGeneration { .. }
            ))
        ));
    }
}
