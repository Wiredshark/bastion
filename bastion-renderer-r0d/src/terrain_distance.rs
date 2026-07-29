//! Deterministic renderer-owned far-terrain residency policy.

use crate::{DomainHashErrorV1, domain_hash_v1};

pub const TERRAIN_DISTANCE_SCHEMA_V1: u16 = 1;
pub const REFERENCE_TERRAIN_RADIUS_CHUNKS_V1: u16 = 16;
pub const FAR_BAND_TERRAIN_RADIUS_CHUNKS_V1: u16 = 24;
pub const FAR_BAND_LOD_DISTANCE_BLOCKS_V1: u32 = 675;
pub const FAR_BAND_MAX_RESIDENT_CHUNKS_V1: u32 = 2_500;
pub const FAR_BAND_MAX_MESH_QUEUE_V1: u32 = FAR_BAND_MAX_RESIDENT_CHUNKS_V1;
pub const FAR_BAND_MAX_RESIDENT_BYTES_V1: u64 = 768 * 1024 * 1024;
pub const FAR_BAND_MAX_UPLOAD_BYTES_PER_FRAME_V1: u64 = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TerrainDistanceModeV1 {
    Reference = 0,
    FarBand = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerrainDistancePlanV1 {
    pub schema: u16,
    pub generation: u64,
    pub mode: TerrainDistanceModeV1,
    pub near_radius_chunks: u16,
    pub horizon_radius_chunks: u16,
    pub lod_distance_blocks: u32,
    pub max_resident_chunks: u32,
    pub max_mesh_queue: u32,
    pub max_resident_bytes: u64,
    pub max_upload_bytes_per_frame: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerrainDistanceErrorV1 {
    WrongSchema,
    StaleGeneration,
    InvalidRadius,
    InvalidBudget,
    CoordinateOverflow,
    DuplicateCoordinate,
    Digest(DomainHashErrorV1),
}

impl TerrainDistancePlanV1 {
    #[must_use]
    pub const fn reference(generation: u64) -> Self {
        Self {
            schema: TERRAIN_DISTANCE_SCHEMA_V1,
            generation,
            mode: TerrainDistanceModeV1::Reference,
            near_radius_chunks: REFERENCE_TERRAIN_RADIUS_CHUNKS_V1,
            horizon_radius_chunks: REFERENCE_TERRAIN_RADIUS_CHUNKS_V1,
            lod_distance_blocks: 450,
            max_resident_chunks: 1_100,
            max_mesh_queue: 256,
            max_resident_bytes: 512 * 1024 * 1024,
            max_upload_bytes_per_frame: 16 * 1024 * 1024,
        }
    }

    #[must_use]
    pub const fn far_band(generation: u64) -> Self {
        Self {
            schema: TERRAIN_DISTANCE_SCHEMA_V1,
            generation,
            mode: TerrainDistanceModeV1::FarBand,
            near_radius_chunks: REFERENCE_TERRAIN_RADIUS_CHUNKS_V1,
            horizon_radius_chunks: FAR_BAND_TERRAIN_RADIUS_CHUNKS_V1,
            lod_distance_blocks: FAR_BAND_LOD_DISTANCE_BLOCKS_V1,
            max_resident_chunks: FAR_BAND_MAX_RESIDENT_CHUNKS_V1,
            max_mesh_queue: FAR_BAND_MAX_MESH_QUEUE_V1,
            max_resident_bytes: FAR_BAND_MAX_RESIDENT_BYTES_V1,
            max_upload_bytes_per_frame: FAR_BAND_MAX_UPLOAD_BYTES_PER_FRAME_V1,
        }
    }

    pub fn validate(self, current_generation: u64) -> Result<(), TerrainDistanceErrorV1> {
        if self.schema != TERRAIN_DISTANCE_SCHEMA_V1 {
            return Err(TerrainDistanceErrorV1::WrongSchema);
        }
        if self.generation == 0 || self.generation != current_generation {
            return Err(TerrainDistanceErrorV1::StaleGeneration);
        }
        if self.near_radius_chunks == 0
            || self.horizon_radius_chunks < self.near_radius_chunks
            || self.horizon_radius_chunks > FAR_BAND_TERRAIN_RADIUS_CHUNKS_V1
        {
            return Err(TerrainDistanceErrorV1::InvalidRadius);
        }
        if self.max_resident_chunks == 0
            || self.max_mesh_queue == 0
            || self.max_resident_bytes == 0
            || self.max_upload_bytes_per_frame == 0
            || self.max_upload_bytes_per_frame > self.max_resident_bytes
            || self.max_mesh_queue > self.max_resident_chunks
        {
            return Err(TerrainDistanceErrorV1::InvalidBudget);
        }
        Ok(())
    }

    pub fn canonical_bytes(self) -> Result<Vec<u8>, TerrainDistanceErrorV1> {
        self.validate(self.generation)?;
        let mut bytes = Vec::with_capacity(43);
        bytes.extend_from_slice(&self.schema.to_le_bytes());
        bytes.extend_from_slice(&self.generation.to_le_bytes());
        bytes.push(self.mode as u8);
        bytes.extend_from_slice(&self.near_radius_chunks.to_le_bytes());
        bytes.extend_from_slice(&self.horizon_radius_chunks.to_le_bytes());
        bytes.extend_from_slice(&self.lod_distance_blocks.to_le_bytes());
        bytes.extend_from_slice(&self.max_resident_chunks.to_le_bytes());
        bytes.extend_from_slice(&self.max_mesh_queue.to_le_bytes());
        bytes.extend_from_slice(&self.max_resident_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.max_upload_bytes_per_frame.to_le_bytes());
        Ok(bytes)
    }

    pub fn digest(self) -> Result<[u8; 32], TerrainDistanceErrorV1> {
        domain_hash_v1(
            "bastion/post-r2/terrain-distance",
            TERRAIN_DISTANCE_SCHEMA_V1,
            0,
            &self.canonical_bytes()?,
        )
        .map_err(TerrainDistanceErrorV1::Digest)
    }

    pub fn canonical_requests(
        self,
        center: [i32; 2],
    ) -> Result<Vec<[i32; 2]>, TerrainDistanceErrorV1> {
        self.validate(self.generation)?;
        let radius = i32::from(self.horizon_radius_chunks);
        let radius_sq = i64::from(radius) * i64::from(radius);
        let mut requests = Vec::new();
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let distance_sq = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
                if distance_sq > radius_sq {
                    continue;
                }
                let x = center[0]
                    .checked_add(dx)
                    .ok_or(TerrainDistanceErrorV1::CoordinateOverflow)?;
                let y = center[1]
                    .checked_add(dy)
                    .ok_or(TerrainDistanceErrorV1::CoordinateOverflow)?;
                requests.push([x, y]);
            }
        }
        requests.sort_unstable_by_key(|position| {
            let dx = i64::from(position[0]) - i64::from(center[0]);
            let dy = i64::from(position[1]) - i64::from(center[1]);
            (dx * dx + dy * dy, position[0], position[1])
        });
        let limit = usize::try_from(self.max_resident_chunks).unwrap_or(usize::MAX);
        requests.truncate(limit);
        Ok(requests)
    }

    pub fn canonical_evictions(
        self,
        center: [i32; 2],
        mut resident: Vec<[i32; 2]>,
    ) -> Result<Vec<[i32; 2]>, TerrainDistanceErrorV1> {
        self.validate(self.generation)?;
        resident.sort_unstable();
        if resident.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(TerrainDistanceErrorV1::DuplicateCoordinate);
        }
        let radius = i64::from(self.horizon_radius_chunks);
        let radius_sq = radius * radius;
        resident.sort_unstable_by_key(|position| {
            let dx = i64::from(position[0]) - i64::from(center[0]);
            let dy = i64::from(position[1]) - i64::from(center[1]);
            (dx * dx + dy * dy, position[0], position[1])
        });
        let limit = usize::try_from(self.max_resident_chunks).unwrap_or(usize::MAX);
        let mut evictions = resident
            .into_iter()
            .enumerate()
            .filter_map(|(ordinal, position)| {
                let dx = i64::from(position[0]) - i64::from(center[0]);
                let dy = i64::from(position[1]) - i64::from(center[1]);
                (ordinal >= limit || dx * dx + dy * dy > radius_sq).then_some(position)
            })
            .collect::<Vec<_>>();
        evictions.sort_unstable_by_key(|position| {
            let dx = i64::from(position[0]) - i64::from(center[0]);
            let dy = i64::from(position[1]) - i64::from(center[1]);
            (
                core::cmp::Reverse(dx * dx + dy * dy),
                position[0],
                position[1],
            )
        });
        Ok(evictions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_band_is_exactly_fifty_percent_farther_and_bounded() {
        let plan = TerrainDistancePlanV1::far_band(7);
        plan.validate(7).unwrap();
        assert_eq!(plan.near_radius_chunks, 16);
        assert_eq!(plan.horizon_radius_chunks, 24);
        assert_eq!(
            u32::from(plan.horizon_radius_chunks) * 100,
            u32::from(plan.near_radius_chunks) * 150
        );
        assert!(plan.canonical_requests([0, 0]).unwrap().len() <= 2_500);
    }

    #[test]
    fn request_and_eviction_order_ignore_input_order() {
        let plan = TerrainDistancePlanV1::far_band(1);
        let requests = plan.canonical_requests([12, -4]).unwrap();
        assert_eq!(requests.first(), Some(&[12, -4]));
        let mut resident = requests;
        resident.extend([[100, 100], [-100, -100]]);
        let mut reversed = resident.clone();
        reversed.reverse();
        assert_eq!(
            plan.canonical_evictions([12, -4], resident).unwrap(),
            plan.canonical_evictions([12, -4], reversed).unwrap()
        );
        assert_eq!(
            plan.canonical_evictions([12, -4], vec![[0, 0], [0, 0]]),
            Err(TerrainDistanceErrorV1::DuplicateCoordinate)
        );
    }

    #[test]
    fn stale_invalid_and_overflow_fail_closed() {
        let plan = TerrainDistancePlanV1::far_band(2);
        assert_eq!(
            plan.validate(3),
            Err(TerrainDistanceErrorV1::StaleGeneration)
        );
        let mut invalid = plan;
        invalid.max_resident_bytes = 0;
        assert_eq!(
            invalid.validate(2),
            Err(TerrainDistanceErrorV1::InvalidBudget)
        );
        assert_eq!(
            plan.canonical_requests([i32::MAX, i32::MAX]),
            Err(TerrainDistanceErrorV1::CoordinateOverflow)
        );
    }

    #[test]
    fn canonical_digest_and_rollback_are_stable() {
        let far = TerrainDistancePlanV1::far_band(9);
        assert_eq!(far.canonical_bytes().unwrap().len(), 43);
        assert_eq!(
            crate::hex_bytes(&far.digest().unwrap()),
            "74fc0e0f1b645feb552fc5d88485c557593100352db72dbe45844e90db144d15"
        );
        assert_eq!(
            far.digest().unwrap(),
            TerrainDistancePlanV1::far_band(9).digest().unwrap()
        );
        let reference = TerrainDistancePlanV1::reference(10);
        reference.validate(10).unwrap();
        assert_eq!(reference.horizon_radius_chunks, 16);
        assert_ne!(far.digest().unwrap(), reference.digest().unwrap());
    }
}
