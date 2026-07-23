//! Deterministic readiness budgets and content-derived asset metrics.
//!
//! Readiness contains semantic counts only. Wall time and completion timing are
//! intentionally absent and cannot turn a key ready.

use crate::{DomainHashErrorV1, domain_hash_v1, extract::MAX_ABS_POSITION_MM_V1};

pub const MAX_ASSET_METRIC_RECORDS_V1: usize = 65_536;
pub const MAX_RAW_SECTIONS_V1: usize = 256;
pub const MAX_ASSET_COUNT_FIELD_V1: u64 = 1_u64 << 48;
pub const MAX_RAW_SECTION_BYTES_V1: u64 = 1_u64 << 40;
pub const MAX_BONE_COUNT_V1: u32 = 65_535;
pub const MAX_MATERIAL_COUNT_V1: u32 = 65_535;
pub const MAX_PALETTE_COUNT_V1: u32 = 65_535;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RendererReadinessBudgetV1 {
    pub max_requests: u64,
    pub max_accepted_results: u64,
    pub max_render_frames: u64,
    pub max_capture_requests: u64,
    pub max_owner_generations: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RendererReadinessCountsV1 {
    pub requests: u64,
    pub accepted_results: u64,
    pub render_frames: u64,
    pub capture_requests: u64,
    pub owner_generations: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessFieldV1 {
    Requests,
    AcceptedResults,
    RenderFrames,
    CaptureRequests,
    OwnerGenerations,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessErrorV1 {
    BudgetExceeded {
        field: ReadinessFieldV1,
        limit: u64,
        actual: u64,
    },
    MetricCountOutOfRange,
    InvalidIndexWidth(u8),
    BoneCountOutOfRange(u32),
    MaterialCountOutOfRange(u32),
    PaletteCountOutOfRange(u32),
    InvalidAabb,
    AabbOutOfRange,
    TooManyRawSections {
        actual: usize,
        maximum: usize,
    },
    RawSectionTooLarge(u64),
    TooManyAssets {
        actual: usize,
        maximum: usize,
    },
    DuplicateContentDigest([u8; 32]),
    SizeOverflow,
    AllocationFailure,
    HashFailure(DomainHashErrorV1),
}

impl RendererReadinessBudgetV1 {
    pub fn check(self, counts: RendererReadinessCountsV1) -> Result<(), ReadinessErrorV1> {
        let fields = [
            (
                ReadinessFieldV1::Requests,
                self.max_requests,
                counts.requests,
            ),
            (
                ReadinessFieldV1::AcceptedResults,
                self.max_accepted_results,
                counts.accepted_results,
            ),
            (
                ReadinessFieldV1::RenderFrames,
                self.max_render_frames,
                counts.render_frames,
            ),
            (
                ReadinessFieldV1::CaptureRequests,
                self.max_capture_requests,
                counts.capture_requests,
            ),
            (
                ReadinessFieldV1::OwnerGenerations,
                self.max_owner_generations,
                counts.owner_generations,
            ),
        ];
        for (field, limit, actual) in fields {
            if actual > limit {
                return Err(ReadinessErrorV1::BudgetExceeded {
                    field,
                    limit,
                    actual,
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererAssetMetricV1 {
    content_digest: [u8; 32],
    source_voxel_count: u64,
    nonempty_voxel_count: u64,
    vertex_count: u64,
    index_count: u64,
    triangle_count: u64,
    index_width_bits: u8,
    bone_count: u32,
    material_count: u32,
    palette_count: u32,
    aabb_min: [i64; 3],
    aabb_max: [i64; 3],
    raw_section_byte_lengths: Vec<u64>,
    geometric_error_micrometers: u64,
    meshlet_count: u64,
}

impl RendererAssetMetricV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        content_digest: [u8; 32],
        source_voxel_count: u64,
        nonempty_voxel_count: u64,
        vertex_count: u64,
        index_count: u64,
        triangle_count: u64,
        index_width_bits: u8,
        bone_count: u32,
        material_count: u32,
        palette_count: u32,
        aabb_min: [i64; 3],
        aabb_max: [i64; 3],
        raw_section_byte_lengths: Vec<u64>,
        geometric_error_micrometers: u64,
        meshlet_count: u64,
    ) -> Result<Self, ReadinessErrorV1> {
        for value in [
            source_voxel_count,
            nonempty_voxel_count,
            vertex_count,
            index_count,
            triangle_count,
            geometric_error_micrometers,
            meshlet_count,
        ] {
            if value > MAX_ASSET_COUNT_FIELD_V1 {
                return Err(ReadinessErrorV1::MetricCountOutOfRange);
            }
        }
        if nonempty_voxel_count > source_voxel_count {
            return Err(ReadinessErrorV1::MetricCountOutOfRange);
        }
        if !matches!(index_width_bits, 16 | 32) {
            return Err(ReadinessErrorV1::InvalidIndexWidth(index_width_bits));
        }
        if bone_count > MAX_BONE_COUNT_V1 {
            return Err(ReadinessErrorV1::BoneCountOutOfRange(bone_count));
        }
        if material_count > MAX_MATERIAL_COUNT_V1 {
            return Err(ReadinessErrorV1::MaterialCountOutOfRange(material_count));
        }
        if palette_count > MAX_PALETTE_COUNT_V1 {
            return Err(ReadinessErrorV1::PaletteCountOutOfRange(palette_count));
        }
        for axis in 0..3 {
            if aabb_min[axis] > aabb_max[axis] {
                return Err(ReadinessErrorV1::InvalidAabb);
            }
            if aabb_min[axis] < -MAX_ABS_POSITION_MM_V1 || aabb_max[axis] > MAX_ABS_POSITION_MM_V1 {
                return Err(ReadinessErrorV1::AabbOutOfRange);
            }
        }
        if raw_section_byte_lengths.len() > MAX_RAW_SECTIONS_V1 {
            return Err(ReadinessErrorV1::TooManyRawSections {
                actual: raw_section_byte_lengths.len(),
                maximum: MAX_RAW_SECTIONS_V1,
            });
        }
        if let Some(length) = raw_section_byte_lengths
            .iter()
            .copied()
            .find(|length| *length > MAX_RAW_SECTION_BYTES_V1)
        {
            return Err(ReadinessErrorV1::RawSectionTooLarge(length));
        }
        Ok(Self {
            content_digest,
            source_voxel_count,
            nonempty_voxel_count,
            vertex_count,
            index_count,
            triangle_count,
            index_width_bits,
            bone_count,
            material_count,
            palette_count,
            aabb_min,
            aabb_max,
            raw_section_byte_lengths,
            geometric_error_micrometers,
            meshlet_count,
        })
    }

    #[must_use]
    pub const fn content_digest(&self) -> [u8; 32] { self.content_digest }

    fn encode(&self, output: &mut Vec<u8>) -> Result<(), ReadinessErrorV1> {
        output.extend_from_slice(&self.content_digest);
        for value in [
            self.source_voxel_count,
            self.nonempty_voxel_count,
            self.vertex_count,
            self.index_count,
            self.triangle_count,
        ] {
            output.extend_from_slice(&value.to_le_bytes());
        }
        output.push(self.index_width_bits);
        output.extend_from_slice(&self.bone_count.to_le_bytes());
        output.extend_from_slice(&self.material_count.to_le_bytes());
        output.extend_from_slice(&self.palette_count.to_le_bytes());
        for component in self.aabb_min.into_iter().chain(self.aabb_max) {
            output.extend_from_slice(&component.to_le_bytes());
        }
        output.extend_from_slice(
            &u16::try_from(self.raw_section_byte_lengths.len())
                .map_err(|_| ReadinessErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
        for length in &self.raw_section_byte_lengths {
            output.extend_from_slice(&length.to_le_bytes());
        }
        output.extend_from_slice(&self.geometric_error_micrometers.to_le_bytes());
        output.extend_from_slice(&self.meshlet_count.to_le_bytes());
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererAssetMetricSetV1 {
    records: Vec<RendererAssetMetricV1>,
    digest: [u8; 32],
}

impl RendererAssetMetricSetV1 {
    pub fn build(mut records: Vec<RendererAssetMetricV1>) -> Result<Self, ReadinessErrorV1> {
        if records.len() > MAX_ASSET_METRIC_RECORDS_V1 {
            return Err(ReadinessErrorV1::TooManyAssets {
                actual: records.len(),
                maximum: MAX_ASSET_METRIC_RECORDS_V1,
            });
        }
        records.sort_unstable_by_key(RendererAssetMetricV1::content_digest);
        if let Some(duplicate) = records
            .windows(2)
            .find(|pair| pair[0].content_digest == pair[1].content_digest)
        {
            return Err(ReadinessErrorV1::DuplicateContentDigest(
                duplicate[0].content_digest,
            ));
        }
        let capacity = records
            .len()
            .checked_mul(2_256)
            .and_then(|value| value.checked_add(8))
            .ok_or(ReadinessErrorV1::SizeOverflow)?;
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(capacity)
            .map_err(|_| ReadinessErrorV1::AllocationFailure)?;
        payload.extend_from_slice(
            &u64::try_from(records.len())
                .map_err(|_| ReadinessErrorV1::SizeOverflow)?
                .to_le_bytes(),
        );
        for record in &records {
            record.encode(&mut payload)?;
        }
        let digest = domain_hash_v1("bastion/r0d/asset-metric-set", 1, 0, &payload)
            .map_err(ReadinessErrorV1::HashFailure)?;
        Ok(Self { records, digest })
    }

    #[must_use]
    pub fn records(&self) -> &[RendererAssetMetricV1] { &self.records }

    #[must_use]
    pub const fn digest(&self) -> [u8; 32] { self.digest }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn budget() -> RendererReadinessBudgetV1 {
        RendererReadinessBudgetV1 {
            max_requests: 10,
            max_accepted_results: 11,
            max_render_frames: 12,
            max_capture_requests: 13,
            max_owner_generations: 14,
        }
    }

    fn counts() -> RendererReadinessCountsV1 {
        RendererReadinessCountsV1 {
            requests: 10,
            accepted_results: 11,
            render_frames: 12,
            capture_requests: 13,
            owner_generations: 14,
        }
    }

    #[test]
    fn every_budget_field_has_exact_pass_and_fail_boundary() {
        assert_eq!(budget().check(counts()), Ok(()));
        let cases = [
            (ReadinessFieldV1::Requests, RendererReadinessCountsV1 {
                requests: 11,
                ..counts()
            }),
            (
                ReadinessFieldV1::AcceptedResults,
                RendererReadinessCountsV1 {
                    accepted_results: 12,
                    ..counts()
                },
            ),
            (ReadinessFieldV1::RenderFrames, RendererReadinessCountsV1 {
                render_frames: 13,
                ..counts()
            }),
            (
                ReadinessFieldV1::CaptureRequests,
                RendererReadinessCountsV1 {
                    capture_requests: 14,
                    ..counts()
                },
            ),
            (
                ReadinessFieldV1::OwnerGenerations,
                RendererReadinessCountsV1 {
                    owner_generations: 15,
                    ..counts()
                },
            ),
        ];
        for (field, actual) in cases {
            assert!(matches!(
                budget().check(actual),
                Err(ReadinessErrorV1::BudgetExceeded { field: got, .. }) if got == field
            ));
        }
    }

    fn metric(digest: u8) -> RendererAssetMetricV1 {
        RendererAssetMetricV1::new(
            [digest; 32],
            4_096,
            1_200,
            480,
            720,
            240,
            16,
            18,
            6,
            32,
            [0, 0, 0],
            [16, 16, 16],
            vec![1_024, 2_048],
            500,
            3,
        )
        .unwrap()
    }

    #[test]
    fn metric_set_order_and_digest_are_content_deterministic() {
        let left = RendererAssetMetricSetV1::build(vec![metric(2), metric(1)]).unwrap();
        let right = RendererAssetMetricSetV1::build(vec![metric(1), metric(2)]).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.records()[0].content_digest(), [1; 32]);
        assert_eq!(
            hex_bytes(&left.digest()),
            "307baca620243b540002a28a7c04d65da74143812b2911101b8cb27153601eab"
        );
    }

    #[test]
    fn metric_bounds_and_duplicate_content_fail_closed() {
        assert_eq!(
            RendererAssetMetricSetV1::build(vec![metric(1), metric(1)]),
            Err(ReadinessErrorV1::DuplicateContentDigest([1; 32]))
        );
        assert!(matches!(
            RendererAssetMetricV1::new(
                [1; 32],
                1,
                2,
                0,
                0,
                0,
                16,
                0,
                0,
                0,
                [0; 3],
                [0; 3],
                vec![],
                0,
                0
            ),
            Err(ReadinessErrorV1::MetricCountOutOfRange)
        ));
        assert!(matches!(
            RendererAssetMetricV1::new(
                [1; 32],
                1,
                1,
                0,
                0,
                0,
                8,
                0,
                0,
                0,
                [0; 3],
                [0; 3],
                vec![],
                0,
                0
            ),
            Err(ReadinessErrorV1::InvalidIndexWidth(8))
        ));
        assert!(matches!(
            RendererAssetMetricV1::new(
                [1; 32],
                1,
                1,
                0,
                0,
                0,
                16,
                0,
                0,
                0,
                [1, 0, 0],
                [0; 3],
                vec![],
                0,
                0
            ),
            Err(ReadinessErrorV1::InvalidAabb)
        ));
        assert!(matches!(
            RendererAssetMetricV1::new(
                [1; 32],
                1,
                1,
                0,
                0,
                0,
                16,
                0,
                0,
                0,
                [0; 3],
                [0; 3],
                vec![0; MAX_RAW_SECTIONS_V1 + 1],
                0,
                0
            ),
            Err(ReadinessErrorV1::TooManyRawSections { .. })
        ));
        assert!(matches!(
            RendererAssetMetricV1::new(
                [1; 32],
                1,
                1,
                0,
                0,
                0,
                16,
                MAX_BONE_COUNT_V1 + 1,
                0,
                0,
                [0; 3],
                [0; 3],
                vec![],
                0,
                0
            ),
            Err(ReadinessErrorV1::BoneCountOutOfRange(_))
        ));
        assert!(matches!(
            RendererAssetMetricV1::new(
                [1; 32],
                1,
                1,
                0,
                0,
                0,
                16,
                0,
                0,
                0,
                [0; 3],
                [0; 3],
                vec![MAX_RAW_SECTION_BYTES_V1 + 1],
                0,
                0
            ),
            Err(ReadinessErrorV1::RawSectionTooLarge(_))
        ));
    }
}
