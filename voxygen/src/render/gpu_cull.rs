//! Capability-gated wgpu frustum reconciliation for canonical figure draws.
//!
//! This first R2 seam deliberately reads structural flags back before they can
//! affect draw admission. A mismatch is a typed CPU fallback, never a silent
//! renderer change. There is no production depth pyramid yet, so occlusion is
//! explicitly reported as unsupported.

use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

use bastion_renderer_r0d::gpu_cull::{
    AcceleratorResultV1, AcceleratorTerminalV1, CanonicalCullBatchV1, GpuCullErrorV1,
    MAX_GPU_CULL_CANDIDATES_V1, OcclusionCapabilityV1,
};
use bytemuck::{Pod, Zeroable};

const WORKGROUP_SIZE: u32 = 64;
const READBACK_TIMEOUT: Duration = Duration::from_secs(30);

const SHADER: &str = r#"
struct Candidate {
    center_radius: vec4<f32>,
    flags: vec4<u32>,
};

struct Frustum {
    planes: array<vec4<f32>, 6>,
    points: array<vec4<f32>, 8>,
    header: vec4<u32>,
};

@group(0) @binding(0) var<storage, read> candidates: array<Candidate>;
@group(0) @binding(1) var<uniform> frustum: Frustum;
@group(0) @binding(2) var<storage, read_write> visible: array<u32>;

fn sphere_intersects(candidate: Candidate) -> bool {
    if candidate.flags.x != 0u {
        return true;
    }
    let center = candidate.center_radius.xyz;
    let radius = candidate.center_radius.w;
    for (var plane_index = 0u; plane_index < 6u; plane_index += 1u) {
        let plane = frustum.planes[plane_index];
        let distance =
            plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
        if distance < -radius {
            return false;
        }
    }
    let minimum = center - vec3<f32>(radius);
    let maximum = center + vec3<f32>(radius);
    for (var axis = 0u; axis < 3u; axis += 1u) {
        var below = 0u;
        var above = 0u;
        for (var point_index = 0u; point_index < 8u; point_index += 1u) {
            let value = frustum.points[point_index][axis];
            if value < minimum[axis] {
                below += 1u;
            }
            if value > maximum[axis] {
                above += 1u;
            }
        }
        if below == 8u || above == 8u {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    if index >= frustum.header.x {
        return;
    }
    visible[index] = select(0u, 1u, sphere_intersects(candidates[index]));
}
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionCullModeV1 {
    CpuReference,
    GpuFrustum,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionCullFallbackV1 {
    None,
    InvalidDeclaration,
    UnsupportedCompute,
    Overflow,
    Stale,
    Device,
    Readback,
    Parity,
    Core,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionCullEvidenceV1 {
    pub generation: u64,
    pub mode: ProductionCullModeV1,
    pub terminal: AcceleratorTerminalV1,
    pub fallback: ProductionCullFallbackV1,
    pub occlusion: OcclusionCapabilityV1,
    pub candidate_count: u32,
    pub admitted_count: u32,
    pub dispatch_count: u32,
    pub input_digest: [u8; 32],
    pub result_digest: [u8; 32],
    pub reference_candidate_count: u32,
    pub reference_admitted_count: u32,
    pub reference_input_digest: [u8; 32],
    pub reference_result_digest: [u8; 32],
    pub gpu_candidate_count: u32,
    pub gpu_admitted_count: u32,
    pub gpu_input_digest: [u8; 32],
    pub gpu_result_digest: [u8; 32],
    pub same_frame_parity: bool,
}

static LATEST: OnceLock<Mutex<Option<ProductionCullEvidenceV1>>> = OnceLock::new();

fn latest_state() -> &'static Mutex<Option<ProductionCullEvidenceV1>> {
    LATEST.get_or_init(|| Mutex::new(None))
}

pub(crate) fn latest_evidence() -> Option<ProductionCullEvidenceV1> {
    latest_state().lock().ok().and_then(|value| *value)
}

#[derive(Debug)]
pub enum ProductionCullErrorV1 {
    InvalidDeclaration,
    UnsupportedCompute,
    Core(GpuCullErrorV1),
    Device(String),
    Readback(String),
    LengthOverflow,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CandidateRaw {
    center_radius: [f32; 4],
    flags: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FrustumRaw {
    planes: [[f32; 4]; 6],
    points: [[f32; 4]; 8],
    header: [u32; 4],
}

enum RuntimeState {
    Cpu,
    Unsupported,
    Gpu {
        pipeline: wgpu::ComputePipeline,
        bind_group: wgpu::BindGroup,
        candidate_buffer: wgpu::Buffer,
        frustum_buffer: wgpu::Buffer,
        output_buffer: wgpu::Buffer,
        readback_buffer: wgpu::Buffer,
    },
    Invalid,
}

pub(super) struct GpuCullRuntimeV1 {
    state: RuntimeState,
}

impl GpuCullRuntimeV1 {
    pub(super) fn new(device: &wgpu::Device, compute_supported: bool) -> Self {
        let requested = std::env::var("BASTION_R2_CULL_MODE")
            .unwrap_or_else(|_| "cpu".to_owned())
            .to_ascii_lowercase();
        let state = match requested.as_str() {
            "cpu" => RuntimeState::Cpu,
            "gpu" if !compute_supported => RuntimeState::Unsupported,
            "gpu" => Self::create_gpu(device),
            _ => RuntimeState::Invalid,
        };
        if let Ok(mut latest) = latest_state().lock() {
            *latest = None;
        }
        Self { state }
    }

    #[cfg(test)]
    fn new_for_test(device: &wgpu::Device) -> Self {
        Self {
            state: Self::create_gpu(device),
        }
    }

    fn create_gpu(device: &wgpu::Device) -> RuntimeState {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bastion-r2-cull-v1"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bastion-r2-cull-v1"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let candidate_size = u64::try_from(
            MAX_GPU_CULL_CANDIDATES_V1.saturating_mul(core::mem::size_of::<CandidateRaw>()),
        )
        .unwrap_or(u64::MAX);
        let output_size =
            u64::try_from(MAX_GPU_CULL_CANDIDATES_V1.saturating_mul(core::mem::size_of::<u32>()))
                .unwrap_or(u64::MAX);
        let candidate_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r2-cull-candidates-v1"),
            size: candidate_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frustum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r2-cull-frustum-v1"),
            size: u64::try_from(core::mem::size_of::<FrustumRaw>()).unwrap_or(u64::MAX),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r2-cull-output-v1"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r2-cull-readback-v1"),
            size: output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bastion-r2-cull-bind-v1"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: candidate_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: frustum_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });
        RuntimeState::Gpu {
            pipeline,
            bind_group,
            candidate_buffer,
            frustum_buffer,
            output_buffer,
            readback_buffer,
        }
    }

    pub(super) fn reconcile(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &CanonicalCullBatchV1,
    ) -> Result<AcceleratorResultV1, ProductionCullErrorV1> {
        match &self.state {
            RuntimeState::Cpu => {
                let result = batch
                    .cpu_reference_result()
                    .map_err(ProductionCullErrorV1::Core)?;
                record_reference_evidence(ProductionCullFallbackV1::None, &result);
                Ok(result)
            },
            RuntimeState::Unsupported => {
                record_fallback(batch, ProductionCullFallbackV1::UnsupportedCompute);
                Err(ProductionCullErrorV1::UnsupportedCompute)
            },
            RuntimeState::Invalid => {
                record_fallback(batch, ProductionCullFallbackV1::InvalidDeclaration);
                Err(ProductionCullErrorV1::InvalidDeclaration)
            },
            RuntimeState::Gpu {
                pipeline,
                bind_group,
                candidate_buffer,
                frustum_buffer,
                output_buffer,
                readback_buffer,
            } => {
                let raw_candidates = batch
                    .candidates()
                    .iter()
                    .map(|candidate| CandidateRaw {
                        center_radius: candidate.center_radius(),
                        flags: [u32::from(candidate.force_visible), 0, 0, 0],
                    })
                    .collect::<Vec<_>>();
                let points = batch
                    .frustum()
                    .points()
                    .map(|point| [point[0], point[1], point[2], 0.0]);
                let frustum = FrustumRaw {
                    planes: batch.frustum().planes(),
                    points,
                    header: [
                        u32::try_from(raw_candidates.len())
                            .map_err(|_| ProductionCullErrorV1::LengthOverflow)?,
                        0,
                        0,
                        0,
                    ],
                };
                queue.write_buffer(candidate_buffer, 0, bytemuck::cast_slice(&raw_candidates));
                queue.write_buffer(frustum_buffer, 0, bytemuck::bytes_of(&frustum));
                let output_bytes = u64::try_from(
                    raw_candidates
                        .len()
                        .checked_mul(core::mem::size_of::<u32>())
                        .ok_or(ProductionCullErrorV1::LengthOverflow)?,
                )
                .map_err(|_| ProductionCullErrorV1::LengthOverflow)?;
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("bastion-r2-cull-v1"),
                });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("bastion-r2-cull-v1"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, bind_group, &[]);
                    let count = u32::try_from(raw_candidates.len())
                        .map_err(|_| ProductionCullErrorV1::LengthOverflow)?;
                    pass.dispatch_workgroups(count.div_ceil(WORKGROUP_SIZE), 1, 1);
                }
                encoder.copy_buffer_to_buffer(output_buffer, 0, readback_buffer, 0, output_bytes);
                let submission = queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: Some(READBACK_TIMEOUT),
                    })
                    .map_err(|error| ProductionCullErrorV1::Device(format!("{error:?}")))?;

                let slice = readback_buffer.slice(0..output_bytes);
                let (sender, receiver) = crossbeam_channel::bounded(1);
                slice.map_async(wgpu::MapMode::Read, move |result| {
                    let _ = sender.send(result);
                });
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(READBACK_TIMEOUT),
                    })
                    .map_err(|error| ProductionCullErrorV1::Device(format!("{error:?}")))?;
                receiver
                    .recv_timeout(READBACK_TIMEOUT)
                    .map_err(|error| ProductionCullErrorV1::Readback(error.to_string()))?
                    .map_err(|error| ProductionCullErrorV1::Readback(format!("{error:?}")))?;
                let mapped = slice.get_mapped_range();
                let flags = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
                drop(mapped);
                readback_buffer.unmap();
                let result = reconcile_same_frame_gpu_flags(batch, &flags)?;
                Ok(result)
            },
        }
    }
}

fn admitted_count(result: &AcceleratorResultV1) -> u32 {
    u32::try_from(result.admitted().len()).unwrap_or(u32::MAX)
}

fn record_reference_evidence(fallback: ProductionCullFallbackV1, result: &AcceleratorResultV1) {
    if let Ok(mut latest) = latest_state().lock() {
        *latest = Some(ProductionCullEvidenceV1 {
            generation: result.generation,
            mode: ProductionCullModeV1::CpuReference,
            terminal: result.terminal,
            fallback,
            occlusion: result.occlusion,
            candidate_count: result.candidate_count,
            admitted_count: admitted_count(result),
            dispatch_count: 0,
            input_digest: result.input_digest,
            result_digest: result.result_digest,
            reference_candidate_count: result.candidate_count,
            reference_admitted_count: admitted_count(result),
            reference_input_digest: result.input_digest,
            reference_result_digest: result.result_digest,
            gpu_candidate_count: 0,
            gpu_admitted_count: 0,
            gpu_input_digest: [0; 32],
            gpu_result_digest: [0; 32],
            same_frame_parity: false,
        });
    }
}

fn record_gpu_evidence(reference: &AcceleratorResultV1, result: &AcceleratorResultV1) {
    if let Ok(mut latest) = latest_state().lock() {
        *latest = Some(ProductionCullEvidenceV1 {
            generation: result.generation,
            mode: ProductionCullModeV1::GpuFrustum,
            terminal: result.terminal,
            fallback: ProductionCullFallbackV1::None,
            occlusion: result.occlusion,
            candidate_count: result.candidate_count,
            admitted_count: admitted_count(result),
            dispatch_count: 1,
            input_digest: result.input_digest,
            result_digest: result.result_digest,
            reference_candidate_count: reference.candidate_count,
            reference_admitted_count: admitted_count(reference),
            reference_input_digest: reference.input_digest,
            reference_result_digest: reference.result_digest,
            gpu_candidate_count: result.candidate_count,
            gpu_admitted_count: admitted_count(result),
            gpu_input_digest: result.input_digest,
            gpu_result_digest: result.result_digest,
            same_frame_parity: true,
        });
    }
}

fn reconcile_same_frame_gpu_flags(
    batch: &CanonicalCullBatchV1,
    flags: &[u32],
) -> Result<AcceleratorResultV1, ProductionCullErrorV1> {
    let reference = batch
        .cpu_reference_result()
        .map_err(ProductionCullErrorV1::Core)?;
    let result = batch
        .reconcile(
            batch.generation(),
            flags,
            AcceleratorTerminalV1::GpuFrustumParity,
            OcclusionCapabilityV1::UnsupportedNoDepthPyramid,
        )
        .map_err(ProductionCullErrorV1::Core)?;
    if reference.candidate_count != result.candidate_count
        || reference.input_digest != result.input_digest
        || reference.result_digest != result.result_digest
        || reference.admitted() != result.admitted()
    {
        return Err(ProductionCullErrorV1::Core(
            GpuCullErrorV1::GpuFrustumParity,
        ));
    }
    record_gpu_evidence(&reference, &result);
    Ok(result)
}

fn record_fallback(batch: &CanonicalCullBatchV1, fallback: ProductionCullFallbackV1) {
    if let Ok(result) = batch.cpu_reference_result() {
        record_reference_evidence(fallback, &result);
    }
}

pub(super) fn fallback_for_error(error: &ProductionCullErrorV1) -> ProductionCullFallbackV1 {
    match error {
        ProductionCullErrorV1::InvalidDeclaration => ProductionCullFallbackV1::InvalidDeclaration,
        ProductionCullErrorV1::UnsupportedCompute => ProductionCullFallbackV1::UnsupportedCompute,
        ProductionCullErrorV1::LengthOverflow => ProductionCullFallbackV1::Overflow,
        ProductionCullErrorV1::Device(message) => {
            let _ = message;
            ProductionCullFallbackV1::Device
        },
        ProductionCullErrorV1::Readback(message) => {
            let _ = message;
            ProductionCullFallbackV1::Readback
        },
        ProductionCullErrorV1::Core(GpuCullErrorV1::StaleGeneration { .. }) => {
            ProductionCullFallbackV1::Stale
        },
        ProductionCullErrorV1::Core(GpuCullErrorV1::GpuFrustumParity)
        | ProductionCullErrorV1::Core(GpuCullErrorV1::GpuInventedCandidate) => {
            ProductionCullFallbackV1::Parity
        },
        ProductionCullErrorV1::Core(_) => ProductionCullFallbackV1::Core,
    }
}

pub(super) fn record_error(
    batch: &CanonicalCullBatchV1,
    error: &ProductionCullErrorV1,
) -> ProductionCullFallbackV1 {
    let fallback = fallback_for_error(error);
    record_fallback(batch, fallback);
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::gpu_cull::{CullPassV1, DrawCandidateV1, FrustumSnapshotV1};

    fn cube_frustum() -> FrustumSnapshotV1 {
        FrustumSnapshotV1::new(
            [
                [1.0, 0.0, 0.0, 10.0],
                [-1.0, 0.0, 0.0, 10.0],
                [0.0, 1.0, 0.0, 10.0],
                [0.0, -1.0, 0.0, 10.0],
                [0.0, 0.0, 1.0, 10.0],
                [0.0, 0.0, -1.0, 10.0],
            ],
            [
                [-10.0, -10.0, -10.0],
                [-10.0, -10.0, 10.0],
                [-10.0, 10.0, -10.0],
                [-10.0, 10.0, 10.0],
                [10.0, -10.0, -10.0],
                [10.0, -10.0, 10.0],
                [10.0, 10.0, -10.0],
                [10.0, 10.0, 10.0],
            ],
        )
        .unwrap()
    }

    #[test]
    fn actual_wgpu_dispatch_matches_cpu_reference() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = runtime
            .block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .expect("an actual wgpu adapter is required for the R2 compute canary");
        assert!(
            adapter
                .get_downlevel_capabilities()
                .flags
                .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS),
            "selected adapter does not support compute shaders"
        );
        let (device, queue) = runtime
            .block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bastion-r2-cull-test"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            }))
            .unwrap();
        let candidates = vec![
            DrawCandidateV1::new([1; 32], CullPassV1::Main, [0.0, 0.0, 0.0], 1.0, false).unwrap(),
            DrawCandidateV1::new([2; 32], CullPassV1::Main, [20.0, 0.0, 0.0], 1.0, false).unwrap(),
            DrawCandidateV1::new([3; 32], CullPassV1::Main, [100.0, 0.0, 0.0], 1.0, true).unwrap(),
        ];
        let batch = CanonicalCullBatchV1::new(7, cube_frustum(), candidates).unwrap();
        let cpu = batch.cpu_reference_result().unwrap();
        let mut gpu = GpuCullRuntimeV1::new_for_test(&device);
        // Match the production call site: compute reconciliation runs while the
        // main render pass is being recorded, before that encoder is submitted.
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bastion-r2-cull-test-target"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bastion-r2-cull-test-render"),
        });
        let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bastion-r2-cull-test-render"),
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
        let actual = gpu.reconcile(&device, &queue, &batch).unwrap();
        drop(render_pass);
        let render_submission = queue.submit([render_encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(render_submission),
                timeout: Some(READBACK_TIMEOUT),
            })
            .unwrap();
        assert_eq!(actual.terminal, AcceleratorTerminalV1::GpuFrustumParity);
        assert_eq!(actual.input_digest, cpu.input_digest);
        assert_eq!(actual.result_digest, cpu.result_digest);
        assert_eq!(actual.admitted(), cpu.admitted());
        let persisted = latest_evidence().expect("same-frame evidence must be persisted");
        assert!(persisted.same_frame_parity);
        assert_eq!(persisted.reference_candidate_count, cpu.candidate_count);
        assert_eq!(
            persisted.reference_admitted_count,
            cpu.admitted().len() as u32
        );
        assert_eq!(persisted.reference_input_digest, cpu.input_digest);
        assert_eq!(persisted.reference_result_digest, cpu.result_digest);
        assert_eq!(persisted.gpu_candidate_count, actual.candidate_count);
        assert_eq!(persisted.gpu_admitted_count, actual.admitted().len() as u32);
        assert_eq!(persisted.gpu_input_digest, actual.input_digest);
        assert_eq!(persisted.gpu_result_digest, actual.result_digest);

        let mut injected = batch.cpu_reference_flags();
        injected[0] ^= 1;
        let injected_error = reconcile_same_frame_gpu_flags(&batch, &injected).unwrap_err();
        assert!(matches!(
            &injected_error,
            ProductionCullErrorV1::Core(
                GpuCullErrorV1::GpuFrustumParity | GpuCullErrorV1::GpuInventedCandidate
            )
        ));
        assert_eq!(
            record_error(&batch, &injected_error),
            ProductionCullFallbackV1::Parity
        );
        let rejected = latest_evidence().expect("typed fallback evidence must be persisted");
        assert_eq!(rejected.fallback, ProductionCullFallbackV1::Parity);
        assert!(!rejected.same_frame_parity);
        assert_eq!(rejected.gpu_candidate_count, 0);
    }

    #[test]
    fn typed_runtime_failures_never_look_like_success() {
        let cases = [
            (
                ProductionCullErrorV1::UnsupportedCompute,
                ProductionCullFallbackV1::UnsupportedCompute,
            ),
            (
                ProductionCullErrorV1::LengthOverflow,
                ProductionCullFallbackV1::Overflow,
            ),
            (
                ProductionCullErrorV1::Device("fault-injected-device-loss".to_owned()),
                ProductionCullFallbackV1::Device,
            ),
            (
                ProductionCullErrorV1::Readback("fault-injected-readback".to_owned()),
                ProductionCullFallbackV1::Readback,
            ),
            (
                ProductionCullErrorV1::Core(GpuCullErrorV1::StaleGeneration {
                    expected: 7,
                    actual: 6,
                }),
                ProductionCullFallbackV1::Stale,
            ),
            (
                ProductionCullErrorV1::Core(GpuCullErrorV1::GpuFrustumParity),
                ProductionCullFallbackV1::Parity,
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(fallback_for_error(&error), expected);
        }
    }
}
