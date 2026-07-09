//! Project Bastion voxygen-side systems (see `BASTION.md` at the repo root).
//!
//! Everything here is additive and namespaced: vanilla voxygen must behave
//! bit-identically when the `--bastion-overseer` launch flag is off.

pub mod input;
pub mod occlusion;
pub mod tools;

use crate::scene::camera::Camera;
use common::{terrain::TerrainGrid, vol::ReadVol};
use vek::*;

/// bastion: approximate ground altitude under a world XY column — the
/// overseer camera's ground-glide reference (B&W2 style: the focus rides the
/// terrain surface and the camera never dips under it).
///
/// Block-accurate `try_find_ground` seeded from the chunk-meta altitude (so it
/// converges even when the previous focus is hundreds of blocks off — e.g. the
/// spectator spawn altitude), falling back to the coarse meta altitude, and
/// riding *water surfaces* rather than the seabed. `None` = chunk not loaded.
pub fn ground_z(terrain: &TerrainGrid, wpos2: Vec2<f32>, z_hint: f32) -> Option<f32> {
    let xy = wpos2.map(|e| e.floor() as i32);
    let coarse = terrain.get_interpolated(xy, |c| c.meta().alt());
    let seed = Vec3::new(xy.x, xy.y, coarse.unwrap_or(z_hint) as i32);
    let mut z = terrain
        .try_find_ground(seed)
        .map(|p| p.z as f32)
        .or(coarse)?;
    // Over water `try_find_ground` lands on the seabed; glide on the surface.
    for _ in 0..256 {
        if terrain
            .get(Vec3::new(xy.x, xy.y, z as i32))
            .is_ok_and(|b| b.is_liquid())
        {
            z += 1.0;
        } else {
            break;
        }
    }
    Some(z)
}

/// bastion (B5.6a): the *visible* terrain surface height at `xy` for overlay
/// draping. Normally `ground_z`; while a Z-slice is active the visible top is
/// the slice cut (terrain above `slice_z` is discarded by the occlusion
/// pass), so the draped overlay must clamp to it — otherwise the outline
/// floats above the sliced surface. `z_hint` seeds the ground search (the
/// caller's flat pick-plane height is a fine hint).
pub fn overlay_surface_z(
    terrain: &TerrainGrid,
    xy: Vec2<f32>,
    z_hint: f32,
    slice_z: Option<f32>,
) -> f32 {
    let g = ground_z(terrain, xy, z_hint).unwrap_or(z_hint);
    match slice_z {
        Some(s) => g.min(s),
        None => g,
    }
}

/// bastion (B5.6a): the reusable overlay-draping primitive the B5.6 scope
/// flag called for. Returns the perimeter of the axis-aligned `[min,max]` xy
/// rectangle as short world-space line segments conformed to the terrain
/// surface — each vertex sits `hover` blocks above the surface at that point
/// (fixes the flat-rectangle-floating-over-a-hill bug). `step` is the sample
/// stride in blocks (1.0 = per-cell; coarser is cheaper for live drag
/// previews). The caller turns each segment into a debug-line shape.
///
/// SEAM (B5.6b + §3w): the same surface sampling drives conformed *fills*
/// (emit draped quads over the interior grid) and the colony-boundary
/// overlay. Keep `overlay_surface_z` the single height authority so all
/// overlays agree.
pub fn draped_rect_outline(
    terrain: &TerrainGrid,
    min_xy: Vec2<f32>,
    max_xy: Vec2<f32>,
    z_hint: f32,
    slice_z: Option<f32>,
    hover: f32,
    step: f32,
) -> Vec<[Vec3<f32>; 2]> {
    let step = step.max(0.25);
    let sample = |xy: Vec2<f32>| -> Vec3<f32> {
        let z = overlay_surface_z(terrain, xy, z_hint, slice_z) + hover;
        Vec3::new(xy.x, xy.y, z)
    };
    let corners = [
        min_xy,
        Vec2::new(max_xy.x, min_xy.y),
        max_xy,
        Vec2::new(min_xy.x, max_xy.y),
    ];
    let mut segs = Vec::new();
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        let len = a.distance(b).max(0.0001);
        let n = ((len / step).ceil() as usize).max(1);
        let mut prev = sample(a);
        for k in 1..=n {
            let t = k as f32 / n as f32;
            let cur = sample(a + (b - a) * t);
            segs.push([prev, cur]);
            prev = cur;
        }
    }
    segs
}

/// bastion (B5.6b-1): a terrain-conformed FILL over the integer XY footprint
/// `[min_xy, max_xy]` (inclusive) as world-space triangles — two per cell,
/// each vertex draped onto the visible surface (`overlay_surface_z`, so it's
/// slice-aware, same height authority as the outline). Each grid corner is
/// sampled once (shared between adjacent cells). The caller feeds these to a
/// `DebugShape::ConformedTris` and colours it via `set_context` (the fill
/// colour's alpha is what makes it translucent). The reusable overlay-fill
/// half of the utility the §3w boundary + B5.6b-2 volumes reuse.
pub fn draped_fill_tris(
    terrain: &TerrainGrid,
    min_xy: Vec2<i32>,
    max_xy: Vec2<i32>,
    z_hint: f32,
    slice_z: Option<f32>,
    hover: f32,
) -> Vec<[Vec3<f32>; 3]> {
    let nx = max_xy.x - min_xy.x + 1;
    let ny = max_xy.y - min_xy.y + 1;
    if nx <= 0 || ny <= 0 {
        return Vec::new();
    }
    // Corner-height grid: (nx+1) × (ny+1) samples, one per cell corner.
    let cols = (nx + 1) as usize;
    let rows = (ny + 1) as usize;
    let mut h = vec![0.0f32; cols * rows];
    for j in 0..rows {
        for i in 0..cols {
            let wx = (min_xy.x + i as i32) as f32;
            let wy = (min_xy.y + j as i32) as f32;
            h[j * cols + i] =
                overlay_surface_z(terrain, Vec2::new(wx, wy), z_hint, slice_z) + hover;
        }
    }
    let vert = |i: usize, j: usize| {
        Vec3::new(
            (min_xy.x + i as i32) as f32,
            (min_xy.y + j as i32) as f32,
            h[j * cols + i],
        )
    };
    let mut tris = Vec::with_capacity((nx * ny * 2).max(0) as usize);
    for j in 0..(rows - 1) {
        for i in 0..(cols - 1) {
            let c00 = vert(i, j);
            let c10 = vert(i + 1, j);
            let c11 = vert(i + 1, j + 1);
            let c01 = vert(i, j + 1);
            tris.push([c00, c10, c11]);
            tris.push([c00, c11, c01]);
        }
    }
    tris
}

/// bastion: unproject a screen-space cursor position onto the horizontal
/// world plane `z = plane_z`, using the overseer camera's matrices.
///
/// Used by grab-drag panning and zoom-to-cursor (B1.5) and, from B2 on, by
/// designation painting. Two caveats verified in `BASTION_B1_FINDINGS.md`:
/// the camera's view matrix only subtracts `focus.fract()`, so unprojected
/// coordinates are relative to `focus.trunc()` (added back here); and
/// `Mat4::mul_point` performs no perspective divide, which is exact for the
/// overseer's *orthographic* projection (w stays 1) but would be wrong for
/// the perspective camera modes.
///
/// Returns `None` when the ray runs (near-)parallel to the plane or hits it
/// behind the camera — callers should just ignore the input in that case.
pub fn unproject_to_world_plane(
    camera: &Camera,
    cursor: Vec2<f32>,
    screen_res: Vec2<f32>,
    plane_z: f32,
) -> Option<Vec3<f32>> {
    let (near, dir) = cursor_ray(camera, cursor, screen_res)?;
    if dir.z.abs() < 1e-5 {
        return None;
    }
    let t = (plane_z - near.z) / dir.z;
    (t.is_finite() && t > 0.0).then(|| near + dir * t)
}

/// bastion (B2a): the world-space ray under a screen cursor — origin on the
/// near plane + unnormalized direction toward the far plane. Same
/// inverse-matrix path as [`unproject_to_world_plane`] (ortho-exact; see its
/// caveats). Entity picking walks this ray.
pub fn cursor_ray(
    camera: &Camera,
    cursor: Vec2<f32>,
    screen_res: Vec2<f32>,
) -> Option<(Vec3<f32>, Vec3<f32>)> {
    if !(screen_res.x > 0.0 && screen_res.y > 0.0) {
        return None;
    }
    let deps = camera.dependents();
    let focus_off = camera.get_focus_pos().map(|e| e.trunc());
    let inv = deps.view_mat_inv * deps.proj_mat_inv;
    let ndc = Vec2::new(
        cursor.x / screen_res.x * 2.0 - 1.0,
        1.0 - cursor.y / screen_res.y * 2.0,
    );
    // Reversed depth (B1): clip z=1 is the near plane, z=0 the far plane.
    let near = inv.mul_point(Vec3::new(ndc.x, ndc.y, 1.0)) + focus_off;
    let far = inv.mul_point(Vec3::new(ndc.x, ndc.y, 0.0)) + focus_off;
    Some((near, far - near))
}
