//! Canonical CPU reference and reconciliation contract for GPU draw culling.
//!
//! GPU acceleration is permitted to consume this exact ordered input and to
//! remove candidates only when the declared capability permits conservative
//! occlusion. Frustum acceleration must match the CPU reference bit-for-bit;
//! it can never invent or reorder a draw.

use crate::{DomainHashErrorV1, domain_hash_v1};

pub const GPU_CULL_SCHEMA_VERSION_V1: u16 = 1;
pub const MAX_GPU_CULL_CANDIDATES_V1: usize = 4_096;
pub const FRUSTUM_PLANE_COUNT_V1: usize = 6;
pub const FRUSTUM_POINT_COUNT_V1: usize = 8;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CullPassV1 {
    Main = 1,
    Shadow = 2,
    Rain = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OcclusionCapabilityV1 {
    UnsupportedNoDepthPyramid,
    ConservativeDepthPyramid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceleratorTerminalV1 {
    CpuReference,
    GpuFrustumParity,
    GpuFrustumAndConservativeOcclusion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuCullErrorV1 {
    UnsupportedVersion(u16),
    EmptyCandidates,
    TooManyCandidates { actual: usize, maximum: usize },
    InvalidGeneration,
    InvalidFloat,
    InvalidRadius,
    DuplicateCandidate,
    MalformedGpuFlag(u32),
    GpuResultLength { actual: usize, expected: usize },
    GpuInventedCandidate,
    GpuFrustumParity,
    StaleGeneration { expected: u64, actual: u64 },
    UnsupportedCapability,
    DeviceLoss,
    Readback,
    ArithmeticOverflow,
    TrailingBytes,
    HashFailure(DomainHashErrorV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrustumSnapshotV1 {
    plane_bits: [[u32; 4]; FRUSTUM_PLANE_COUNT_V1],
    point_bits: [[u32; 3]; FRUSTUM_POINT_COUNT_V1],
}

impl FrustumSnapshotV1 {
    pub fn new(
        planes: [[f32; 4]; FRUSTUM_PLANE_COUNT_V1],
        points: [[f32; 3]; FRUSTUM_POINT_COUNT_V1],
    ) -> Result<Self, GpuCullErrorV1> {
        if planes
            .iter()
            .flatten()
            .chain(points.iter().flatten())
            .any(|value| !value.is_finite())
        {
            return Err(GpuCullErrorV1::InvalidFloat);
        }
        Ok(Self {
            plane_bits: planes.map(|plane| plane.map(f32::to_bits)),
            point_bits: points.map(|point| point.map(f32::to_bits)),
        })
    }

    pub fn planes(self) -> [[f32; 4]; FRUSTUM_PLANE_COUNT_V1] {
        self.plane_bits.map(|plane| plane.map(f32::from_bits))
    }

    pub fn points(self) -> [[f32; 3]; FRUSTUM_POINT_COUNT_V1] {
        self.point_bits.map(|point| point.map(f32::from_bits))
    }

    pub fn canonical_bytes(self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity((FRUSTUM_PLANE_COUNT_V1 * 4 + FRUSTUM_POINT_COUNT_V1 * 3) * 4);
        for plane in self.plane_bits {
            for value in plane {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for point in self.point_bits {
            for value in point {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DrawCandidateV1 {
    pub semantic_entity: [u8; 32],
    pub pass: CullPassV1,
    center_radius_bits: [u32; 4],
    pub force_visible: bool,
}

impl DrawCandidateV1 {
    pub fn new(
        semantic_entity: [u8; 32],
        pass: CullPassV1,
        center: [f32; 3],
        radius: f32,
        force_visible: bool,
    ) -> Result<Self, GpuCullErrorV1> {
        if semantic_entity == [0; 32] {
            return Err(GpuCullErrorV1::DuplicateCandidate);
        }
        if center.into_iter().any(|value| !value.is_finite()) || !radius.is_finite() {
            return Err(GpuCullErrorV1::InvalidFloat);
        }
        if radius < 0.0 {
            return Err(GpuCullErrorV1::InvalidRadius);
        }
        Ok(Self {
            semantic_entity,
            pass,
            center_radius_bits: [
                center[0].to_bits(),
                center[1].to_bits(),
                center[2].to_bits(),
                radius.to_bits(),
            ],
            force_visible,
        })
    }

    pub fn center_radius(self) -> [f32; 4] { self.center_radius_bits.map(f32::from_bits) }

    fn canonical_bytes(self) -> [u8; 52] {
        let mut bytes = [0_u8; 52];
        bytes[..32].copy_from_slice(&self.semantic_entity);
        bytes[32] = self.pass as u8;
        bytes[33] = u8::from(self.force_visible);
        for (index, value) in self.center_radius_bits.into_iter().enumerate() {
            let start = 36 + index * 4;
            bytes[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalCullBatchV1 {
    schema_version: u16,
    generation: u64,
    frustum: FrustumSnapshotV1,
    candidates: Vec<DrawCandidateV1>,
    input_digest: [u8; 32],
}

impl CanonicalCullBatchV1 {
    pub fn new(
        generation: u64,
        frustum: FrustumSnapshotV1,
        mut candidates: Vec<DrawCandidateV1>,
    ) -> Result<Self, GpuCullErrorV1> {
        if generation == 0 {
            return Err(GpuCullErrorV1::InvalidGeneration);
        }
        if candidates.is_empty() {
            return Err(GpuCullErrorV1::EmptyCandidates);
        }
        if candidates.len() > MAX_GPU_CULL_CANDIDATES_V1 {
            return Err(GpuCullErrorV1::TooManyCandidates {
                actual: candidates.len(),
                maximum: MAX_GPU_CULL_CANDIDATES_V1,
            });
        }
        candidates.sort_by_key(|candidate| (candidate.pass, candidate.semantic_entity));
        if candidates.windows(2).any(|pair| {
            pair[0].pass == pair[1].pass && pair[0].semantic_entity == pair[1].semantic_entity
        }) {
            return Err(GpuCullErrorV1::DuplicateCandidate);
        }
        let mut payload =
            Vec::with_capacity(16 + frustum.canonical_bytes().len() + candidates.len() * 52);
        payload.extend_from_slice(&GPU_CULL_SCHEMA_VERSION_V1.to_le_bytes());
        payload.extend_from_slice(&generation.to_le_bytes());
        payload.extend_from_slice(
            &u32::try_from(candidates.len())
                .map_err(|_| GpuCullErrorV1::ArithmeticOverflow)?
                .to_le_bytes(),
        );
        payload.extend_from_slice(&frustum.canonical_bytes());
        for candidate in &candidates {
            payload.extend_from_slice(&candidate.canonical_bytes());
        }
        let input_digest = domain_hash_v1("bastion/r2/cull-input", 1, 0, &payload)
            .map_err(GpuCullErrorV1::HashFailure)?;
        Ok(Self {
            schema_version: GPU_CULL_SCHEMA_VERSION_V1,
            generation,
            frustum,
            candidates,
            input_digest,
        })
    }

    pub fn generation(&self) -> u64 { self.generation }

    pub fn frustum(&self) -> FrustumSnapshotV1 { self.frustum }

    pub fn candidates(&self) -> &[DrawCandidateV1] { &self.candidates }

    pub fn input_digest(&self) -> [u8; 32] { self.input_digest }

    pub fn cpu_reference_flags(&self) -> Vec<u32> {
        self.candidates
            .iter()
            .map(|candidate| {
                u32::from(candidate.force_visible || sphere_intersects(*candidate, self.frustum))
            })
            .collect()
    }

    pub fn cpu_reference_result(&self) -> Result<AcceleratorResultV1, GpuCullErrorV1> {
        self.reconcile(
            self.generation,
            &self.cpu_reference_flags(),
            AcceleratorTerminalV1::CpuReference,
            OcclusionCapabilityV1::UnsupportedNoDepthPyramid,
        )
    }

    pub fn reconcile(
        &self,
        generation: u64,
        gpu_flags: &[u32],
        terminal: AcceleratorTerminalV1,
        occlusion: OcclusionCapabilityV1,
    ) -> Result<AcceleratorResultV1, GpuCullErrorV1> {
        if generation != self.generation {
            return Err(GpuCullErrorV1::StaleGeneration {
                expected: self.generation,
                actual: generation,
            });
        }
        if gpu_flags.len() != self.candidates.len() {
            return Err(GpuCullErrorV1::GpuResultLength {
                actual: gpu_flags.len(),
                expected: self.candidates.len(),
            });
        }
        if let Some(flag) = gpu_flags.iter().copied().find(|flag| *flag > 1) {
            return Err(GpuCullErrorV1::MalformedGpuFlag(flag));
        }
        let cpu_flags = self.cpu_reference_flags();
        if gpu_flags
            .iter()
            .zip(&cpu_flags)
            .any(|(gpu, cpu)| *gpu == 1 && *cpu == 0)
        {
            return Err(GpuCullErrorV1::GpuInventedCandidate);
        }
        match terminal {
            AcceleratorTerminalV1::CpuReference => {
                if gpu_flags != cpu_flags {
                    return Err(GpuCullErrorV1::GpuFrustumParity);
                }
            },
            AcceleratorTerminalV1::GpuFrustumParity => {
                if gpu_flags != cpu_flags {
                    return Err(GpuCullErrorV1::GpuFrustumParity);
                }
                if occlusion != OcclusionCapabilityV1::UnsupportedNoDepthPyramid {
                    return Err(GpuCullErrorV1::UnsupportedCapability);
                }
            },
            AcceleratorTerminalV1::GpuFrustumAndConservativeOcclusion => {
                if occlusion != OcclusionCapabilityV1::ConservativeDepthPyramid {
                    return Err(GpuCullErrorV1::UnsupportedCapability);
                }
            },
        }
        let admitted = self
            .candidates
            .iter()
            .zip(gpu_flags)
            .filter_map(|(candidate, flag)| (*flag == 1).then_some(candidate.semantic_entity))
            .collect::<Vec<_>>();
        let mut payload = Vec::with_capacity(48 + admitted.len() * 32);
        payload.extend_from_slice(&self.input_digest);
        payload.extend_from_slice(&self.generation.to_le_bytes());
        payload.extend_from_slice(
            &u32::try_from(admitted.len())
                .map_err(|_| GpuCullErrorV1::ArithmeticOverflow)?
                .to_le_bytes(),
        );
        for semantic_entity in &admitted {
            payload.extend_from_slice(semantic_entity);
        }
        let result_digest = domain_hash_v1("bastion/r2/cull-result", 1, 0, &payload)
            .map_err(GpuCullErrorV1::HashFailure)?;
        Ok(AcceleratorResultV1 {
            generation: self.generation,
            input_digest: self.input_digest,
            result_digest,
            terminal,
            occlusion,
            candidate_count: u32::try_from(self.candidates.len())
                .map_err(|_| GpuCullErrorV1::ArithmeticOverflow)?,
            admitted,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceleratorResultV1 {
    pub generation: u64,
    pub input_digest: [u8; 32],
    pub result_digest: [u8; 32],
    pub terminal: AcceleratorTerminalV1,
    pub occlusion: OcclusionCapabilityV1,
    pub candidate_count: u32,
    admitted: Vec<[u8; 32]>,
}

impl AcceleratorResultV1 {
    pub fn admitted(&self) -> &[[u8; 32]] { &self.admitted }
}

fn sphere_intersects(candidate: DrawCandidateV1, frustum: FrustumSnapshotV1) -> bool {
    let [x, y, z, radius] = candidate.center_radius();
    for [a, b, c, d] in frustum.planes() {
        let distance = a * x + b * y + c * z + d;
        if distance < -radius {
            return false;
        }
    }
    let min = [x - radius, y - radius, z - radius];
    let max = [x + radius, y + radius, z + radius];
    let points = frustum.points();
    for axis in 0..3 {
        if points.iter().all(|point| point[axis] < min[axis])
            || points.iter().all(|point| point[axis] > max[axis])
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> [u8; 32] { [value; 32] }

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

    fn candidate(id: u8, x: f32) -> DrawCandidateV1 {
        DrawCandidateV1::new(digest(id), CullPassV1::Main, [x, 0.0, 0.0], 1.0, false).unwrap()
    }

    #[test]
    fn canonical_order_and_reference_are_permutation_independent() {
        let a = CanonicalCullBatchV1::new(7, cube_frustum(), vec![
            candidate(3, 20.0),
            candidate(1, 0.0),
            candidate(2, 9.0),
        ])
        .unwrap();
        let b = CanonicalCullBatchV1::new(7, cube_frustum(), vec![
            candidate(2, 9.0),
            candidate(3, 20.0),
            candidate(1, 0.0),
        ])
        .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.cpu_reference_flags(), vec![1, 1, 0]);
        assert_eq!(
            a.cpu_reference_result().unwrap(),
            b.cpu_reference_result().unwrap()
        );
    }

    #[test]
    fn boundary_empty_full_and_force_visible_vectors() {
        let boundary =
            DrawCandidateV1::new(digest(1), CullPassV1::Main, [11.0, 0.0, 0.0], 1.0, false)
                .unwrap();
        let forced =
            DrawCandidateV1::new(digest(2), CullPassV1::Main, [100.0, 0.0, 0.0], 1.0, true)
                .unwrap();
        let batch = CanonicalCullBatchV1::new(1, cube_frustum(), vec![forced, boundary]).unwrap();
        assert_eq!(batch.cpu_reference_flags(), vec![1, 1]);
        assert_eq!(
            CanonicalCullBatchV1::new(1, cube_frustum(), Vec::new()),
            Err(GpuCullErrorV1::EmptyCandidates)
        );
    }

    #[test]
    fn rejects_duplicate_overflow_stale_malformed_and_invented_results() {
        let one = candidate(1, 20.0);
        assert_eq!(
            CanonicalCullBatchV1::new(1, cube_frustum(), vec![one, one]),
            Err(GpuCullErrorV1::DuplicateCandidate)
        );
        let mut oversized = Vec::new();
        for id in 0..=MAX_GPU_CULL_CANDIDATES_V1 {
            let mut semantic = [0_u8; 32];
            semantic[..8].copy_from_slice(&(id as u64 + 1).to_le_bytes());
            oversized.push(
                DrawCandidateV1::new(semantic, CullPassV1::Main, [0.0, 0.0, 0.0], 1.0, false)
                    .unwrap(),
            );
        }
        assert!(matches!(
            CanonicalCullBatchV1::new(1, cube_frustum(), oversized),
            Err(GpuCullErrorV1::TooManyCandidates { .. })
        ));
        let batch = CanonicalCullBatchV1::new(9, cube_frustum(), vec![one]).unwrap();
        assert!(matches!(
            batch.reconcile(
                8,
                &[0],
                AcceleratorTerminalV1::GpuFrustumParity,
                OcclusionCapabilityV1::UnsupportedNoDepthPyramid
            ),
            Err(GpuCullErrorV1::StaleGeneration { .. })
        ));
        assert_eq!(
            batch.reconcile(
                9,
                &[2],
                AcceleratorTerminalV1::GpuFrustumParity,
                OcclusionCapabilityV1::UnsupportedNoDepthPyramid
            ),
            Err(GpuCullErrorV1::MalformedGpuFlag(2))
        );
        assert_eq!(
            batch.reconcile(
                9,
                &[1],
                AcceleratorTerminalV1::GpuFrustumParity,
                OcclusionCapabilityV1::UnsupportedNoDepthPyramid
            ),
            Err(GpuCullErrorV1::GpuInventedCandidate)
        );
    }

    #[test]
    fn conservative_occlusion_may_remove_but_never_invent_or_reorder() {
        let batch = CanonicalCullBatchV1::new(2, cube_frustum(), vec![
            candidate(2, 0.0),
            candidate(1, 0.0),
        ])
        .unwrap();
        let result = batch
            .reconcile(
                2,
                &[1, 0],
                AcceleratorTerminalV1::GpuFrustumAndConservativeOcclusion,
                OcclusionCapabilityV1::ConservativeDepthPyramid,
            )
            .unwrap();
        assert_eq!(result.admitted(), &[digest(1)]);
        assert_eq!(
            batch.reconcile(
                2,
                &[1, 0],
                AcceleratorTerminalV1::GpuFrustumParity,
                OcclusionCapabilityV1::UnsupportedNoDepthPyramid,
            ),
            Err(GpuCullErrorV1::GpuFrustumParity)
        );
    }

    #[test]
    fn gpu_frustum_terminal_preserves_cpu_structural_digest() {
        let batch = CanonicalCullBatchV1::new(5, cube_frustum(), vec![
            candidate(2, 20.0),
            candidate(1, 0.0),
        ])
        .unwrap();
        let cpu = batch.cpu_reference_result().unwrap();
        let gpu = batch
            .reconcile(
                5,
                &batch.cpu_reference_flags(),
                AcceleratorTerminalV1::GpuFrustumParity,
                OcclusionCapabilityV1::UnsupportedNoDepthPyramid,
            )
            .unwrap();
        assert_eq!(cpu.input_digest, gpu.input_digest);
        assert_eq!(cpu.result_digest, gpu.result_digest);
        assert_eq!(cpu.admitted(), gpu.admitted());
        assert_ne!(cpu.terminal, gpu.terminal);
    }

    #[test]
    fn invalid_floats_and_radii_fail_closed() {
        assert_eq!(
            DrawCandidateV1::new(
                digest(1),
                CullPassV1::Main,
                [f32::NAN, 0.0, 0.0],
                1.0,
                false
            ),
            Err(GpuCullErrorV1::InvalidFloat)
        );
        assert_eq!(
            DrawCandidateV1::new(digest(1), CullPassV1::Main, [0.0, 0.0, 0.0], -1.0, false),
            Err(GpuCullErrorV1::InvalidRadius)
        );
    }
}
