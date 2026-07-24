//! Production compatibility bridge for the R1BC persistent figure GPU pool.
//!
//! The legacy figure draw buffers remain mirrors until R1BC-DRAW-003. This
//! bridge is nevertheless the sole resource-completion authority: a package
//! becomes ready only after its canonical instance/pose bytes have been
//! written to bounded persistent buffers and the exact backend submission has
//! completed.

use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

use bastion_renderer_r0d::{
    domain_hash_v1,
    figure_asset::{CompiledFigurePackageV1, PackageReceiptV1},
    figure_gpu::{
        BackendCompletionV1, FIGURE_GPU_INSTANCE_STRIDE_V1, FIGURE_GPU_POSE_PAGE_BYTES_V1,
        FigureGpuBoneV1, FigureGpuBufferKindV1, FigureGpuEntityInputV1, FigureGpuPoolConfigV1,
        FigureGpuPoolV1, SubmissionIdentityV1, UploadReceiptV1,
    },
    presentation::PresentationFrameV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FigureGpuProductionEvidenceV1 {
    pub generation: u64,
    pub package_digest: [u8; 32],
    pub frame_digest: [u8; 32],
    pub resource_set_digest: [u8; 32],
    pub assignment_digest: [u8; 32],
    pub staged_digest: [u8; 32],
    pub plan_digest: [u8; 32],
    pub submission_sequence: u64,
    pub completion_sequence: u64,
    pub instance_count: u32,
    pub pose_page_count: u32,
    pub upload_windows: u16,
    pub upload_operations: u32,
    pub upload_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FigureGpuRuntimeErrorV1 {
    InvalidPresentation,
    Hash,
    Core(String),
    Backend(String),
    CounterOverflow,
}

static LATEST_EVIDENCE: OnceLock<Mutex<Option<FigureGpuProductionEvidenceV1>>> = OnceLock::new();
static DRAW_AUTHORITY: OnceLock<Mutex<Option<FigureGpuDrawAuthorityV1>>> = OnceLock::new();

#[derive(Clone, Debug)]
pub(crate) struct FigureGpuDrawAuthorityV1 {
    pub plan: bastion_renderer_r0d::figure_gpu::FigureGpuUploadPlanV1,
    pub receipt: UploadReceiptV1,
}

fn evidence_state() -> &'static Mutex<Option<FigureGpuProductionEvidenceV1>> {
    LATEST_EVIDENCE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn latest_evidence() -> Option<FigureGpuProductionEvidenceV1> {
    evidence_state().lock().ok().and_then(|value| *value)
}

pub(crate) fn latest_draw_authority() -> Option<FigureGpuDrawAuthorityV1> {
    DRAW_AUTHORITY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|value| value.clone())
}

pub(super) struct FigureGpuRuntimeV1 {
    pool: FigureGpuPoolV1,
    instance_buffer: wgpu::Buffer,
    pose_buffer: wgpu::Buffer,
    next_submission_sequence: u64,
    current: Option<(UploadReceiptV1, SubmissionIdentityV1)>,
}

impl FigureGpuRuntimeV1 {
    pub(super) fn new(device: &wgpu::Device) -> Result<Self, FigureGpuRuntimeErrorV1> {
        let config = FigureGpuPoolConfigV1::default();
        let instance_bytes = u64::from(config.instance_capacity)
            .checked_mul(
                u64::try_from(FIGURE_GPU_INSTANCE_STRIDE_V1)
                    .map_err(|_| FigureGpuRuntimeErrorV1::CounterOverflow)?,
            )
            .ok_or(FigureGpuRuntimeErrorV1::CounterOverflow)?;
        let pose_bytes = u64::from(config.pose_page_capacity)
            .checked_mul(
                u64::try_from(FIGURE_GPU_POSE_PAGE_BYTES_V1)
                    .map_err(|_| FigureGpuRuntimeErrorV1::CounterOverflow)?,
            )
            .ok_or(FigureGpuRuntimeErrorV1::CounterOverflow)?;
        let usage = wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r1bc-figure-instance-pool-v1"),
            size: instance_bytes,
            usage,
            mapped_at_creation: false,
        });
        let pose_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bastion-r1bc-figure-pose-pool-v1"),
            size: pose_bytes,
            usage,
            mapped_at_creation: false,
        });
        if let Ok(mut latest) = evidence_state().lock() {
            *latest = None;
        }
        if let Ok(mut authority) = DRAW_AUTHORITY.get_or_init(|| Mutex::new(None)).lock() {
            *authority = None;
        }
        Ok(Self {
            pool: FigureGpuPoolV1::new(config)
                .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?,
            instance_buffer,
            pose_buffer,
            next_submission_sequence: 1,
            current: None,
        })
    }

    pub(super) fn upload_generation(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
        package_receipt: &PackageReceiptV1,
    ) -> Result<UploadReceiptV1, FigureGpuRuntimeErrorV1> {
        let generation = frame.generation().client_applied_generation;
        if let Some((receipt, _)) = &self.current
            && receipt.generation == generation
            && receipt.frame_digest == frame.frame_digest()
            && receipt.resource_set_digest == frame.resource_set_digest()
            && receipt.package_digest == package.package_digest()
        {
            return Ok(receipt.clone());
        }

        let inputs = entity_inputs(frame, package)?;
        let staged = self
            .pool
            .begin_generation(frame, package, package_receipt, inputs)
            .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?;
        staged
            .plan
            .validate()
            .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?;

        let mut upload_operations = 0_u32;
        let mut upload_bytes = 0_u64;
        let mut final_backend_submission = None;
        for window in &staged.plan.windows {
            for range in &window.ranges {
                let target = match range.buffer_kind {
                    FigureGpuBufferKindV1::Instances => &self.instance_buffer,
                    FigureGpuBufferKindV1::Poses => &self.pose_buffer,
                };
                queue.write_buffer(target, range.offset, &range.bytes);
                crate::r0p_observer::record_buffer_upload(range.bytes.len());
                upload_operations = upload_operations
                    .checked_add(1)
                    .ok_or(FigureGpuRuntimeErrorV1::CounterOverflow)?;
                upload_bytes = upload_bytes
                    .checked_add(
                        u64::try_from(range.bytes.len())
                            .map_err(|_| FigureGpuRuntimeErrorV1::CounterOverflow)?,
                    )
                    .ok_or(FigureGpuRuntimeErrorV1::CounterOverflow)?;
            }
            final_backend_submission = Some(queue.submit(std::iter::empty()));
            crate::r0p_observer::record_submission();
        }
        let Some(final_backend_submission) = final_backend_submission else {
            let _ = self.pool.rollback_pending(staged.plan.plan_digest);
            return Err(FigureGpuRuntimeErrorV1::InvalidPresentation);
        };
        if let Err(error) = device.poll(wgpu::PollType::Wait {
            submission_index: Some(final_backend_submission),
            timeout: Some(Duration::from_secs(60)),
        }) {
            let _ = self.pool.rollback_pending(staged.plan.plan_digest);
            return Err(FigureGpuRuntimeErrorV1::Backend(format!("{error:?}")));
        }

        let sequence = self.next_submission_sequence;
        self.next_submission_sequence = self
            .next_submission_sequence
            .checked_add(1)
            .ok_or(FigureGpuRuntimeErrorV1::CounterOverflow)?;
        let submission = SubmissionIdentityV1::for_plan(sequence, &staged.plan)
            .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?;
        let receipt = UploadReceiptV1::from_backend_completion(
            frame,
            package,
            package_receipt,
            &staged.plan,
            submission,
            BackendCompletionV1::Completed(submission),
        )
        .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?;

        let previous = self.current.clone();
        let committed = self
            .pool
            .commit_pending(receipt.clone())
            .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?;
        let instance_count = u32::try_from(staged.plan.assignments.len())
            .map_err(|_| FigureGpuRuntimeErrorV1::CounterOverflow)?;
        let pose_page_count = match staged
            .plan
            .assignments
            .iter()
            .map(|assignment| assignment.slot.pose_page)
            .max()
        {
            Some(page) => page
                .checked_add(1)
                .ok_or(FigureGpuRuntimeErrorV1::CounterOverflow)?,
            None => 0,
        };
        let upload_windows = u16::try_from(staged.plan.windows.len())
            .map_err(|_| FigureGpuRuntimeErrorV1::CounterOverflow)?;
        let evidence = FigureGpuProductionEvidenceV1 {
            generation,
            package_digest: receipt.package_digest,
            frame_digest: receipt.frame_digest,
            resource_set_digest: receipt.resource_set_digest,
            assignment_digest: receipt.assignment_digest,
            staged_digest: receipt.staged_digest,
            plan_digest: receipt.plan_digest,
            submission_sequence: receipt.submission_identity.sequence,
            completion_sequence: receipt.completion_identity.sequence,
            instance_count,
            pose_page_count,
            upload_windows,
            upload_operations,
            upload_bytes,
        };
        let draw_plan = staged.plan.clone();
        drop(staged);
        drop(committed);
        if let Some((old_receipt, old_completion)) = previous {
            self.pool
                .retire_generation(
                    old_receipt.generation,
                    &BackendCompletionV1::Completed(old_completion),
                )
                .map_err(|error| FigureGpuRuntimeErrorV1::Core(format!("{error:?}")))?;
        }
        self.current = Some((receipt.clone(), submission));
        if let Ok(mut latest) = evidence_state().lock() {
            *latest = Some(evidence);
        } else {
            return Err(FigureGpuRuntimeErrorV1::Backend(
                "figure GPU evidence lock poisoned".to_owned(),
            ));
        }
        if let Ok(mut authority) = DRAW_AUTHORITY.get_or_init(|| Mutex::new(None)).lock() {
            *authority = Some(FigureGpuDrawAuthorityV1 {
                plan: draw_plan,
                receipt: receipt.clone(),
            });
        } else {
            return Err(FigureGpuRuntimeErrorV1::Backend(
                "figure GPU draw authority lock poisoned".to_owned(),
            ));
        }
        tracing::info!(
            target: "bastion_r1bc_gpu",
            generation,
            instance_count,
            pose_page_count,
            upload_windows,
            upload_operations,
            upload_bytes,
            submission_sequence = submission.sequence,
            package_sha256 = %hex(&receipt.package_digest),
            staged_sha256 = %hex(&receipt.staged_digest),
            "persistent figure GPU generation completed"
        );
        Ok(receipt)
    }
}

fn entity_inputs(
    frame: &PresentationFrameV1,
    package: &CompiledFigurePackageV1,
) -> Result<Vec<FigureGpuEntityInputV1>, FigureGpuRuntimeErrorV1> {
    if frame.entities().is_empty() {
        return Err(FigureGpuRuntimeErrorV1::InvalidPresentation);
    }
    if frame.renderer_required_resources() != [package.package_digest()]
        || frame
            .entities()
            .iter()
            .any(|entity| entity.figure_resource != package.package_digest())
    {
        return Err(FigureGpuRuntimeErrorV1::InvalidPresentation);
    }
    let palette_digest = hash("bastion/r1bc/gpu-palette", &package.authority_digest())?;
    frame
        .entities()
        .iter()
        .map(|entity| {
            let mut composition = Vec::with_capacity(64);
            composition.extend_from_slice(&entity.semantic_id);
            composition.extend_from_slice(&package.package_digest());
            let composition_digest = hash("bastion/r1bc/gpu-composition", &composition)?;
            let mut transform = Vec::with_capacity(24 + 16 + 2);
            for component in entity.position_mm {
                transform.extend_from_slice(&component.to_le_bytes());
            }
            for component in entity.orientation_q30 {
                transform.extend_from_slice(&component.to_le_bytes());
            }
            transform.extend_from_slice(&entity.scale_milli.to_le_bytes());
            let transform_digest = hash("bastion/r1bc/gpu-transform", &transform)?;
            let mut pose = Vec::with_capacity(40);
            pose.extend_from_slice(&entity.state_digest);
            pose.extend_from_slice(&frame.generation().simulation_tick.to_le_bytes());
            let pose_digest = hash("bastion/r1bc/gpu-pose", &pose)?;
            Ok(FigureGpuEntityInputV1 {
                generation: frame.generation().client_applied_generation,
                semantic_entity: entity.semantic_id,
                package_digest: package.package_digest(),
                authority_digest: package.authority_digest(),
                composition_digest,
                palette_digest,
                transform_digest,
                pose_digest,
                lod_level: 0,
                section_id: 1,
                material_id: 1,
                flags: 0,
                bones: vec![FigureGpuBoneV1 {
                    matrix_q20: [1 << 20, 0, 0, 0, 0, 1 << 20, 0, 0, 0, 0, 1 << 20, 0],
                }],
            })
        })
        .collect()
}

fn hash(domain: &str, bytes: &[u8]) -> Result<[u8; 32], FigureGpuRuntimeErrorV1> {
    domain_hash_v1(domain, 1, 0, bytes).map_err(|_| FigureGpuRuntimeErrorV1::Hash)
}

fn hex(bytes: &[u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        figure_asset::{
            FigureAssetRoleV1, FigurePackageTargetV1, FigureSourceInputV1, MaterialBindingV1,
            MaterialKindV1,
        },
        presentation::{
            PresentationEntityV1, PresentationEnvironmentV1, PresentationFrameDraftV1,
            PresentationGenerationV1, PresentationVisualPolicyV1,
        },
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn package() -> CompiledFigurePackageV1 {
        CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            digest(1),
            digest(2),
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

    fn frame(package: &CompiledFigurePackageV1, x: i64) -> PresentationFrameV1 {
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: 1,
                simulation_tick: 300,
                coherent_snapshot_root: digest(3),
            },
            entities: vec![PresentationEntityV1 {
                semantic_id: digest(4),
                figure_resource: package.package_digest(),
                group_id: None,
                position_mm: [x, 2, 3],
                orientation_q30: [0, 0, 0, 1 << 30],
                scale_milli: 1_000,
                state_tag: 1,
                state_digest: digest(5),
            }],
            groups: Vec::new(),
            events: Vec::new(),
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

    #[test]
    fn production_entity_record_is_stable_and_every_transform_change_is_bound() {
        let package = package();
        let first = entity_inputs(&frame(&package, 1), &package).unwrap();
        let repeated = entity_inputs(&frame(&package, 1), &package).unwrap();
        let changed = entity_inputs(&frame(&package, 2), &package).unwrap();
        assert_eq!(first, repeated);
        assert_eq!(first.len(), 1);
        assert_eq!(changed.len(), 1);
        assert_ne!(first[0].transform_digest, changed[0].transform_digest);
        assert_eq!(first[0].package_digest, package.package_digest());
        assert_eq!(first[0].bones.len(), 1);
    }

    #[test]
    fn missing_or_wrong_package_entity_fails_closed() {
        let package = package();
        let empty = PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: 1,
                simulation_tick: 300,
                coherent_snapshot_root: digest(3),
            },
            entities: Vec::new(),
            groups: Vec::new(),
            events: Vec::new(),
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
        .unwrap();
        assert_eq!(
            entity_inputs(&empty, &package),
            Err(FigureGpuRuntimeErrorV1::InvalidPresentation)
        );
    }
}
