#ifndef BASTION_OCCLUSION_GLSL
#define BASTION_OCCLUSION_GLSL

// bastion (B1.6): the unified overseer occlusion & transparency function.
//
// One shared alpha function drives four composable view behaviors — the same
// fragment operation, a dithered screen-door discard whose threshold is a
// function of position instead of the B1 constant. See
// docs/BASTION_B1_6_FINDINGS.md and voxygen/src/bastion/occlusion.rs.
//
// Coordinate spaces (precision-safe, no huge-float subtraction): fragment world
// z = f_pos.z + focus_off.z; fragment relative to focus in XY = f_pos.xy -
// focus_pos.xy (focus_pos = fract(focus)); camera in f_pos space = cam_pos.xyz +
// focus_pos.xyz; targets arrive pre-subtracted (world - focus_off).

#include <globals.glsl>

const uint BASTION_OCC_SLICE     = 1u;
const uint BASTION_OCC_PROXIMITY = 2u;
const uint BASTION_OCC_CUTAWAY   = 4u;
const uint BASTION_OCC_ROOF      = 8u;

// 4x4 ordered Bayer threshold in [0,1). Self-contained (Veloren's dither() is
// behind an experimental flag), so every pass can screen-door uniformly.
float bastion_bayer(vec2 frag_coord) {
    const float m[16] = float[16](
        0.0/16.0,  8.0/16.0,  2.0/16.0, 10.0/16.0,
        12.0/16.0, 4.0/16.0, 14.0/16.0,  6.0/16.0,
        3.0/16.0, 11.0/16.0,  1.0/16.0,  9.0/16.0,
        15.0/16.0, 7.0/16.0, 13.0/16.0,  5.0/16.0
    );
    ivec2 p = ivec2(mod(frag_coord, 4.0));
    return m[p.y * 4 + p.x];
}

// Fragment visibility, 1 = fully solid, 0 = hidden. min-composed across the
// active modes (most-hiding wins).
float bastion_occlusion_alpha(vec3 f_pos) {
    uint mode = bastion_occ_mode.x;
    if (mode == 0u) { return 1.0; }

    float world_z = f_pos.z + focus_off.z;
    float focus_z = bastion_occ_a.z + focus_off.z;
    float height_above = world_z - focus_z;
    float dist_xy = length(f_pos.xy - focus_pos.xy);

    float a = 1.0;

    // Soft slice: smooth fade band below the manual cut.
    if ((mode & BASTION_OCC_SLICE) != 0u) {
        float slice_z = bastion_occ_a.x;
        float band = max(bastion_occ_a.y, 0.001);
        a = min(a, 1.0 - smoothstep(slice_z - band, slice_z, world_z));
    }

    // Proximity / height: foreground floor near focus stays solid; tall or
    // background geometry fades, scaled by the strength slider.
    if ((mode & BASTION_OCC_PROXIMITY) != 0u) {
        float strength = clamp(bastion_occ_a.w, 0.0, 1.0);
        float hf = smoothstep(bastion_occ_b.x, bastion_occ_b.y, height_above);
        float df = smoothstep(bastion_occ_b.z, bastion_occ_b.w, dist_xy);
        a = min(a, 1.0 - max(hf, df) * strength);
    }

    // Roof/interior reveal: approximate mask — geometry in a slab above the
    // focus plane and near it in XY (so distant hills stay solid). B2/B3 will
    // refine the mask with real per-column/room coverage.
    if ((mode & BASTION_OCC_ROOF) != 0u) {
        float near = 1.0 - smoothstep(bastion_occ_b.z, bastion_occ_b.w, dist_xy);
        float slab = smoothstep(bastion_occ_c.y, bastion_occ_c.z, height_above);
        a = min(a, 1.0 - slab * near);
    }

    // Camera-to-target cutaway: fade geometry inside the cylinder between the
    // camera and each target, nearer the camera than the target.
    if ((mode & BASTION_OCC_CUTAWAY) != 0u) {
        vec3 cam_rel = cam_pos.xyz + focus_pos.xyz;
        float radius = max(bastion_occ_c.x, 0.001);
        uint n = min(bastion_occ_mode.y, 4u);
        for (uint i = 0u; i < n; i++) {
            if (bastion_occ_targets[i].w < 0.5) { continue; }
            vec3 ct = bastion_occ_targets[i].xyz - cam_rel;
            float ct_len = length(ct);
            if (ct_len < 0.001) { continue; }
            vec3 dir = ct / ct_len;
            vec3 cf = f_pos - cam_rel;
            float t = dot(cf, dir);
            if (t > 0.0 && t < ct_len) {
                float perp = length(cf - dir * t);
                // 1 inside the inner radius, easing to 0 at the edge.
                float radial = 1.0 - smoothstep(radius * 0.5, radius, perp);
                a = min(a, 1.0 - radial);
            }
        }
    }

    return a;
}

// Screen-door discard: true => discard this fragment.
bool bastion_occlusion_discard(vec3 f_pos, vec2 frag_coord) {
    if (bastion_occ_mode.x == 0u) { return false; }
    float a = bastion_occlusion_alpha(f_pos);
    // Near-zero always goes (the Bayer cell at 0 would otherwise leak 1/16).
    if (a < 1.0 / 32.0) { return true; }
    return a < bastion_bayer(frag_coord);
}

// Interior re-lighting: a soft top-down fill over exposed interior surfaces so
// revealed rooms read as lit-from-above rather than black. Returns an additive
// linear-light term for surf_color. Only active where a slice/reveal is on.
vec3 bastion_relight_add(vec3 f_pos, vec3 f_norm) {
    uint mode = bastion_occ_mode.x;
    float strength = bastion_occ_c.w;
    if (strength <= 0.0) { return vec3(0.0); }
    if ((mode & (BASTION_OCC_SLICE | BASTION_OCC_ROOF)) == 0u) { return vec3(0.0); }

    float world_z = f_pos.z + focus_off.z;
    float focus_z = bastion_occ_a.z + focus_off.z;

    // "Interior" = at/under the exposing plane, near the focus in XY. `below`
    // is 1 under the plane (lit interior floor/walls) easing to 0 above it.
    float plane_z = ((mode & BASTION_OCC_SLICE) != 0u) ? bastion_occ_a.x : focus_z;
    float band = max(bastion_occ_a.y, 0.001);
    float below = 1.0 - smoothstep(plane_z - band, plane_z, world_z);
    float near = 1.0 - smoothstep(bastion_occ_b.z, bastion_occ_b.w, length(f_pos.xy - focus_pos.xy));
    // Brightest on upward faces (floors) → reads as lit from above.
    float up = clamp(f_norm.z * 0.5 + 0.5, 0.0, 1.0);
    return vec3(strength * below * near * up);
}

#endif
