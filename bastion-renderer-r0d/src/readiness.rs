//! BUILD-007A10.5 (part 2) — deterministic readiness budgets and asset metrics
//! substrate (design §12).
//!
//! - §12.3 semantic operation-count budgets: a manifest declares count limits;
//!   an over-budget setup is rejected BEFORE the canonical run begins. Wall-clock
//!   timeout is infrastructure-only and never makes a key ready or advances a
//!   frame — that is not modeled here, only the count budgets are.
//! - §12.6 deterministic asset metrics: every field is integer/fixed-point. No
//!   timing, queue age, allocation address, worker ID, or wall timestamp may
//!   enter these metrics, so the digest is a pure function of content.
//!
//! The live async completion registry (§12.2) reuses the existing engine shared
//! async substrate (design Section 3A) and is wired in the integration surface,
//! not reimplemented here.

/// Declared semantic operation-count budgets (§12.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RendererBudgetsV1 {
    pub max_requests: u64,
    pub max_accepted_results: u64,
    pub max_render_frames: u64,
    pub max_capture_requests: u64,
    pub max_owner_generations: u64,
}

/// The actual declared counts of a candidate setup, checked against the budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct RendererSetupCountsV1 {
    pub requests: u64,
    pub accepted_results: u64,
    pub render_frames: u64,
    pub capture_requests: u64,
    pub owner_generations: u64,
}

/// Typed over-budget failure (§12.3): which budget field was exceeded and by
/// what, so the rejection is machine-stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetExceeded {
    pub field: BudgetField,
    pub limit: u64,
    pub actual: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetField {
    Requests,
    AcceptedResults,
    RenderFrames,
    CaptureRequests,
    OwnerGenerations,
}

impl RendererBudgetsV1 {
    /// Reject an over-budget setup before the canonical run (§12.3). The first
    /// exceeded field (in frozen order) is reported.
    pub fn check(&self, counts: &RendererSetupCountsV1) -> Result<(), BudgetExceeded> {
        let checks = [
            (BudgetField::Requests, self.max_requests, counts.requests),
            (BudgetField::AcceptedResults, self.max_accepted_results, counts.accepted_results),
            (BudgetField::RenderFrames, self.max_render_frames, counts.render_frames),
            (BudgetField::CaptureRequests, self.max_capture_requests, counts.capture_requests),
            (BudgetField::OwnerGenerations, self.max_owner_generations, counts.owner_generations),
        ];
        for (field, limit, actual) in checks {
            if actual > limit {
                return Err(BudgetExceeded { field, limit, actual });
            }
        }
        Ok(())
    }
}

/// Deterministic per-family/LOD/package asset metrics (§12.6). Every field is a
/// pure function of content — no timing/queue/address/worker/timestamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererAssetMetricsV1 {
    pub package_sha256: [u8; 32],
    pub source_voxel_count: u64,
    pub nonempty_voxel_count: u64,
    pub vertex_count: u64,
    pub index_count: u64,
    pub triangle_count: u64,
    pub index_width_bits: u8,
    pub bone_count: u32,
    pub material_count: u32,
    pub palette_count: u32,
    /// Integer AABB min/max in canonical units.
    pub aabb_min: [i64; 3],
    pub aabb_max: [i64; 3],
    pub raw_section_byte_lengths: Vec<u64>,
    pub geometric_error_micrometers: u64,
    pub meshlet_count: u64,
}

impl RendererAssetMetricsV1 {
    /// Domain-separated (§4.4) digest over every integer metric field in frozen
    /// order.
    #[must_use]
    pub fn metrics_digest(&self) -> [u8; 32] {
        let mut p = Vec::new();
        p.extend_from_slice(&self.package_sha256);
        for v in [
            self.source_voxel_count,
            self.nonempty_voxel_count,
            self.vertex_count,
            self.index_count,
            self.triangle_count,
        ] {
            p.extend_from_slice(&v.to_le_bytes());
        }
        p.push(self.index_width_bits);
        p.extend_from_slice(&self.bone_count.to_le_bytes());
        p.extend_from_slice(&self.material_count.to_le_bytes());
        p.extend_from_slice(&self.palette_count.to_le_bytes());
        for c in self.aabb_min.iter().chain(self.aabb_max.iter()) {
            p.extend_from_slice(&c.to_le_bytes());
        }
        p.extend_from_slice(&(self.raw_section_byte_lengths.len() as u64).to_le_bytes());
        for l in &self.raw_section_byte_lengths {
            p.extend_from_slice(&l.to_le_bytes());
        }
        p.extend_from_slice(&self.geometric_error_micrometers.to_le_bytes());
        p.extend_from_slice(&self.meshlet_count.to_le_bytes());
        crate::domain_hash("bastion/r0d/asset-metrics", 1, 0, &p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    fn budgets() -> RendererBudgetsV1 {
        RendererBudgetsV1 {
            max_requests: 100,
            max_accepted_results: 100,
            max_render_frames: 60,
            max_capture_requests: 10,
            max_owner_generations: 4,
        }
    }

    #[test]
    fn within_budget_admitted() {
        let counts = RendererSetupCountsV1 { requests: 50, accepted_results: 50, render_frames: 60, capture_requests: 10, owner_generations: 4 };
        assert!(budgets().check(&counts).is_ok());
    }

    #[test]
    fn over_budget_rejected_with_typed_field() {
        let counts = RendererSetupCountsV1 { render_frames: 61, ..Default::default() };
        assert_eq!(
            budgets().check(&counts),
            Err(BudgetExceeded { field: BudgetField::RenderFrames, limit: 60, actual: 61 })
        );
    }

    #[test]
    fn first_exceeded_field_wins_in_frozen_order() {
        // Both requests and render_frames exceed; requests is earlier in the
        // frozen check order, so it is reported.
        let counts = RendererSetupCountsV1 { requests: 200, render_frames: 200, ..Default::default() };
        assert_eq!(budgets().check(&counts).unwrap_err().field, BudgetField::Requests);
    }

    fn metrics() -> RendererAssetMetricsV1 {
        RendererAssetMetricsV1 {
            package_sha256: [0xab; 32],
            source_voxel_count: 4096,
            nonempty_voxel_count: 1200,
            vertex_count: 480,
            index_count: 720,
            triangle_count: 240,
            index_width_bits: 16,
            bone_count: 18,
            material_count: 6,
            palette_count: 32,
            aabb_min: [0, 0, 0],
            aabb_max: [16, 16, 16],
            raw_section_byte_lengths: vec![1024, 2048],
            geometric_error_micrometers: 500,
            meshlet_count: 0,
        }
    }

    #[test]
    fn metrics_digest_is_content_sensitive() {
        let a = metrics();
        let mut b = metrics();
        b.vertex_count += 1;
        assert_ne!(a.metrics_digest(), b.metrics_digest());
    }

    #[test]
    fn frozen_metrics_digest() {
        assert_eq!(
            hex_bytes(&metrics().metrics_digest()),
            "e83170f216a76e9dd7ea7b69be60092c6b87e76852fe8b743c2ac35b686bf324",
            "frozen asset-metrics digest drift",
        );
    }
}
