//! BUILD-007A10.5 (part 3) — D3 deterministic render-selection state (design
//! §13). The CPU-visible selection authority: conservative integer frustum
//! culling, integer screen-space-error LOD with hysteresis, deterministic
//! animation-sample selection, and the byte-exact GPU indirect-command ABI.
//!
//! All arithmetic is checked integer / fixed-point; there is no float and no
//! unordered container in the canonical output. `ObservedGpuVisibilityV1` (the
//! GPU-side corroboration path, §13.6) is environment-scoped and never changes
//! this canonical order — it lives in the GPU-evidence integration packet
//! (BUILD-007A10.14), not here.

use sha2::{Digest, Sha256};

use crate::camera::PlaneQ24_40V1;

/// Conservative frustum classification of an AABB (§13.2). Borderline equality
/// resolves toward visible to avoid unstable disappearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cull {
    Outside,
    Intersect,
    Inside,
}

impl Cull {
    #[must_use]
    pub fn is_visible(self) -> bool {
        !matches!(self, Cull::Outside)
    }
}

/// Integer AABB in millimeters (§13.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AabbMm {
    pub min: [i64; 3],
    pub max: [i64; 3],
}

/// Signed distance of a point along a plane, exact `i128` (§13.2). Scale mixes
/// the Q24.40 normal with the mm point, but the SIGN — all culling needs — is
/// scale-invariant per plane.
fn signed_dist(plane: &PlaneQ24_40V1, p: [i64; 3]) -> i128 {
    let mut s = i128::from(plane.distance);
    for i in 0..3 {
        s += i128::from(plane.normal[i]) * i128::from(p[i]);
    }
    s
}

/// Classify an AABB against one plane using its p-vertex / n-vertex (§13.2).
fn classify_plane(plane: &PlaneQ24_40V1, aabb: &AabbMm) -> Cull {
    // p-vertex: corner farthest along +normal; n-vertex: opposite.
    let mut p = [0i64; 3];
    let mut n = [0i64; 3];
    for i in 0..3 {
        if plane.normal[i] >= 0 {
            p[i] = aabb.max[i];
            n[i] = aabb.min[i];
        } else {
            p[i] = aabb.min[i];
            n[i] = aabb.max[i];
        }
    }
    if signed_dist(plane, p) < 0 {
        Cull::Outside // even the most-positive corner is behind the plane
    } else if signed_dist(plane, n) >= 0 {
        Cull::Inside // even the most-negative corner is in front (>=0 => visible)
    } else {
        Cull::Intersect
    }
}

/// Classify an AABB against a 6-plane frustum (§13.2): outside if ANY plane
/// excludes it, inside if ALL planes contain it, else intersect. Conservative.
#[must_use]
pub fn cull_aabb(planes: &[PlaneQ24_40V1; 6], aabb: &AabbMm) -> Cull {
    let mut all_inside = true;
    for plane in planes {
        match classify_plane(plane, aabb) {
            Cull::Outside => return Cull::Outside,
            Cull::Intersect => all_inside = false,
            Cull::Inside => {}
        }
    }
    if all_inside {
        Cull::Inside
    } else {
        Cull::Intersect
    }
}

/// A visible candidate for canonical draw ordering (§13.2). Accepted entities
/// sort by full semantic digest, then compact id — no unordered container ever
/// controls output order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleEntityV1 {
    pub entity_digest: [u8; 32],
    pub compact_id: u32,
}

/// Sort accepted candidates by `(entity_digest, compact_id)` (§13.2/§13.5 tie
/// rule).
pub fn sort_visible(mut v: Vec<VisibleEntityV1>) -> Vec<VisibleEntityV1> {
    v.sort_by(|a, b| a.entity_digest.cmp(&b.entity_digest).then(a.compact_id.cmp(&b.compact_id)));
    v
}

/// Integer screen-space error in Q16 (§13.3):
/// `ceil(geometric_error_um * projection_scale_q16 / max(distance_um, 1))`.
#[must_use]
pub fn sse_q16(geometric_error_um: u64, projection_scale_q16: u64, distance_um: u64) -> u64 {
    let denom = distance_um.max(1);
    let num = geometric_error_um.saturating_mul(projection_scale_q16);
    num.div_ceil(denom)
}

/// A single LOD transition with integer hysteresis thresholds (§13.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodThresholdV1 {
    pub enter_threshold_q16: u64,
    pub exit_threshold_q16: u64,
}

/// Select a LOD level with hysteresis (§13.3). Higher SSE => higher detail. On
/// the exact threshold the retained current LOD is kept; the first frame (no
/// previous) uses `default_lod`. `thresholds[i]` gates entry to LOD `i+1`.
#[must_use]
pub fn select_lod_hysteresis(
    sse: u64,
    thresholds: &[LodThresholdV1],
    previous: Option<u32>,
    default_lod: u32,
) -> u32 {
    let base = previous.unwrap_or(default_lod);
    // Compute the band the SSE falls into using enter (rising) / exit (falling)
    // thresholds relative to the retained level, resolving equality to retained.
    let mut lod = base;
    // Rise: promote while the next level's enter threshold is strictly exceeded.
    while (lod as usize) < thresholds.len() && sse > thresholds[lod as usize].enter_threshold_q16 {
        lod += 1;
    }
    // Fall: demote while below the current level's exit threshold.
    while lod > 0 && sse < thresholds[(lod - 1) as usize].exit_threshold_q16 {
        lod -= 1;
    }
    lod
}

/// Deterministic animation sample selection (§13.4). Missed wall frames never
/// change the selected sample — it is a pure function of the simulation tick.
#[must_use]
pub fn animation_sample_index(
    simulation_tick: u64,
    clip_sample_rate_num: u64,
    clip_sample_rate_den: u64,
    sample_count: u64,
    entity_digest: &[u8; 32],
    clip_digest: &[u8; 32],
) -> u64 {
    debug_assert!(sample_count > 0 && clip_sample_rate_den > 0);
    let mut h = Sha256::new();
    h.update(entity_digest);
    h.update(clip_digest);
    let d: [u8; 32] = h.finalize().into();
    let stable_phase_offset = u64::from(u32::from_le_bytes([d[0], d[1], d[2], d[3]])) % sample_count;
    let base = simulation_tick.saturating_mul(clip_sample_rate_num) / clip_sample_rate_den;
    (base + stable_phase_offset) % sample_count
}

/// Reduced-cadence update gate (§13.4): `first_u32(entity_digest) % cadence == tick % cadence`.
#[must_use]
pub fn animation_update_due(entity_digest: &[u8; 32], cadence: u64, tick: u64) -> bool {
    if cadence == 0 {
        return true;
    }
    let first = u64::from(u32::from_le_bytes([
        entity_digest[0],
        entity_digest[1],
        entity_digest[2],
        entity_digest[3],
    ]));
    first % cadence == tick % cadence
}

/// The canonical draw sort key (§13.5): `(pass_tag, pipeline_tag, package_digest,
/// material_tag, lod, entity_digest)`. All ties resolve by full digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DrawKeyV1 {
    pub pass_tag: u16,
    pub pipeline_tag: u16,
    pub package_digest: [u8; 32],
    pub material_tag: u16,
    pub lod: u32,
    pub entity_digest: [u8; 32],
}

/// Sort draws by the frozen total key (§13.5).
pub fn sort_draws(mut d: Vec<DrawKeyV1>) -> Vec<DrawKeyV1> {
    d.sort();
    d
}

/// WebGPU `DrawIndirectArgsV1` — exact 16-byte little-endian encoding (§13.6A).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawIndirectArgsV1 {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

impl DrawIndirectArgsV1 {
    /// Exact 16-byte encoding in WebGPU field order (§13.6A) — never a hashed
    /// arbitrary struct.
    #[must_use]
    pub fn encode(&self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[0..4].copy_from_slice(&self.vertex_count.to_le_bytes());
        b[4..8].copy_from_slice(&self.instance_count.to_le_bytes());
        b[8..12].copy_from_slice(&self.first_vertex.to_le_bytes());
        b[12..16].copy_from_slice(&self.first_instance.to_le_bytes());
        b
    }
}

/// WebGPU `DrawIndexedIndirectArgsV1` — exact 20-byte little-endian encoding
/// (§13.6A). `base_vertex` is a defined signed 32-bit field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawIndexedIndirectArgsV1 {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgsV1 {
    /// Exact 20-byte encoding in WebGPU field order (§13.6A).
    #[must_use]
    pub fn encode(&self) -> [u8; 20] {
        let mut b = [0u8; 20];
        b[0..4].copy_from_slice(&self.index_count.to_le_bytes());
        b[4..8].copy_from_slice(&self.instance_count.to_le_bytes());
        b[8..12].copy_from_slice(&self.first_index.to_le_bytes());
        b[12..16].copy_from_slice(&self.base_vertex.to_le_bytes());
        b[16..20].copy_from_slice(&self.first_instance.to_le_bytes());
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    // A generous axis-aligned frustum: 6 planes each admitting a slab around the
    // origin at +/- 1,000,000 mm. normal in Q24.40, distance in the same scaled
    // space so signed_dist sign is the cull decision.
    fn box_frustum(half_mm: i64) -> [PlaneQ24_40V1; 6] {
        let k = 1i64 << 40;
        let d = k.saturating_mul(half_mm); // distance offset at the slab face
        [
            PlaneQ24_40V1 { normal: [k, 0, 0], distance: d },   // x >= -half
            PlaneQ24_40V1 { normal: [-k, 0, 0], distance: d },  // x <=  half
            PlaneQ24_40V1 { normal: [0, k, 0], distance: d },
            PlaneQ24_40V1 { normal: [0, -k, 0], distance: d },
            PlaneQ24_40V1 { normal: [0, 0, k], distance: d },
            PlaneQ24_40V1 { normal: [0, 0, -k], distance: d },
        ]
    }

    #[test]
    fn aabb_fully_inside_is_inside() {
        let f = box_frustum(1000);
        let a = AabbMm { min: [-100, -100, -100], max: [100, 100, 100] };
        assert_eq!(cull_aabb(&f, &a), Cull::Inside);
    }

    #[test]
    fn aabb_fully_outside_is_outside() {
        let f = box_frustum(1000);
        // Far beyond +x slab.
        let a = AabbMm { min: [5000, 0, 0], max: [6000, 100, 100] };
        assert_eq!(cull_aabb(&f, &a), Cull::Outside);
    }

    #[test]
    fn aabb_straddling_face_is_intersect() {
        let f = box_frustum(1000);
        // Straddles the +x slab face at 1000.
        let a = AabbMm { min: [900, 0, 0], max: [1100, 100, 100] };
        assert_eq!(cull_aabb(&f, &a), Cull::Intersect);
        assert!(cull_aabb(&f, &a).is_visible());
    }

    #[test]
    fn visible_sort_is_digest_then_id() {
        let v = |d: u8, id: u32| VisibleEntityV1 { entity_digest: [d; 32], compact_id: id };
        let out = sort_visible(vec![v(2, 1), v(1, 9), v(1, 3)]);
        let got: Vec<_> = out.iter().map(|x| (x.entity_digest[0], x.compact_id)).collect();
        assert_eq!(got, vec![(1, 3), (1, 9), (2, 1)]);
    }

    #[test]
    fn sse_is_integer_ceil() {
        // 500um error * 65536 scale / 1000um distance = 32768 exactly.
        assert_eq!(sse_q16(500, 65536, 1000), 32768);
        // Ceil rounding: 1*10/3 = 3.33 -> 4.
        assert_eq!(sse_q16(1, 10, 3), 4);
        // Zero distance clamps to 1 (no divide-by-zero).
        assert_eq!(sse_q16(5, 2, 0), 10);
    }

    #[test]
    fn lod_hysteresis_holds_and_switches() {
        let t = [
            LodThresholdV1 { enter_threshold_q16: 100, exit_threshold_q16: 80 },
            LodThresholdV1 { enter_threshold_q16: 200, exit_threshold_q16: 180 },
        ];
        // First frame uses default.
        assert_eq!(select_lod_hysteresis(150, &t, None, 0), 1);
        // Rising past enter promotes.
        assert_eq!(select_lod_hysteresis(250, &t, Some(1), 0), 2);
        // In the hysteresis band (between exit and enter), retained level holds.
        assert_eq!(select_lod_hysteresis(90, &t, Some(1), 0), 1); // 80<90<=100 => hold 1
        // Falling below exit demotes.
        assert_eq!(select_lod_hysteresis(70, &t, Some(1), 0), 0);
    }

    #[test]
    fn animation_sample_is_tick_pure() {
        let e = [0x11; 32];
        let c = [0x22; 32];
        // Same tick => same sample regardless of "wall frames".
        let s1 = animation_sample_index(120, 30, 1, 60, &e, &c);
        let s2 = animation_sample_index(120, 30, 1, 60, &e, &c);
        assert_eq!(s1, s2);
        assert!(s1 < 60);
    }

    #[test]
    fn frozen_animation_sample() {
        let e = [0x11; 32];
        let c = [0x22; 32];
        assert_eq!(
            animation_sample_index(120, 30, 1, 60, &e, &c),
            17,
            "frozen animation sample drift",
        );
    }

    #[test]
    fn draw_key_orders_by_frozen_tuple() {
        let k = |pass: u16, pipe: u16, pkg: u8, mat: u16, lod: u32, ent: u8| DrawKeyV1 {
            pass_tag: pass,
            pipeline_tag: pipe,
            package_digest: [pkg; 32],
            material_tag: mat,
            lod,
            entity_digest: [ent; 32],
        };
        let out = sort_draws(vec![k(1, 0, 0, 0, 0, 0), k(0, 5, 0, 0, 0, 0), k(0, 5, 0, 0, 0, 1)]);
        assert_eq!(out[0].pass_tag, 0);
        assert_eq!(out[0].entity_digest[0], 0);
        assert_eq!(out[1].entity_digest[0], 1);
        assert_eq!(out[2].pass_tag, 1);
    }

    #[test]
    fn indirect_args_exact_bytes() {
        let a = DrawIndirectArgsV1 { vertex_count: 6, instance_count: 2, first_vertex: 0, first_instance: 1 };
        assert_eq!(hex_bytes(&a.encode()), "06000000020000000000000001000000");
    }

    #[test]
    fn indexed_indirect_args_exact_bytes_with_signed_base_vertex() {
        let a = DrawIndexedIndirectArgsV1 {
            index_count: 36,
            instance_count: 1,
            first_index: 0,
            base_vertex: -4,
            first_instance: 0,
        };
        // 36=0x24, then instance=1, first_index=0, base_vertex=-4 (fcffffff two's complement), first_instance=0.
        assert_eq!(hex_bytes(&a.encode()), "240000000100000000000000fcffffff00000000");
    }
}
