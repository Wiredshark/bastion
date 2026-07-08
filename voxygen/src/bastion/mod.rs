//! Project Bastion voxygen-side systems (see `BASTION.md` at the repo root).
//!
//! Everything here is additive and namespaced: vanilla voxygen must behave
//! bit-identically when the `--bastion-overseer` launch flag is off.

pub mod input;
pub mod occlusion;

use crate::scene::camera::Camera;
use vek::*;

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
    let dir = far - near;
    if dir.z.abs() < 1e-5 {
        return None;
    }
    let t = (plane_z - near.z) / dir.z;
    (t.is_finite() && t > 0.0).then(|| near + dir * t)
}
