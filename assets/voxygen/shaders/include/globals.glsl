#ifndef GLOBALS_GLSL
#define GLOBALS_GLSL

layout(std140, set = 0, binding = 0) uniform u_globals {
    mat4 view_mat;
    mat4 proj_mat;
    mat4 all_mat;
    vec4 cam_pos;
    vec4 focus_off;
    vec4 focus_pos;
    vec4 view_distance;
    // .x = time of day, repeats every day.
    // .y = a continuous value for what day it is. Repeats every `tick_overflow` for precisions sake.
    vec4 time_of_day;
    vec4 sun_dir;
    vec4 moon_dir;
    // .x = The `Time` resource, repeated every `tick_overflow`
    // .y = a floored (`Time` / `tick_overflow`)
    // .z = Time local to client, not synced between clients.
    vec4 tick;
    vec4 screen_res;
    uvec4 light_shadow_count;
    vec4 shadow_proj_factors;
    uvec4 medium;
    ivec4 select_pos;
    vec4 gamma_exposure;
    vec4 last_lightning;
    vec2 wind_vel;
    float ambiance;
    // 0 - FirstPerson
    // 1 - ThirdPerson
    uint cam_mode;
    float sprite_render_distance;
    float u_rotation;
    float screen_fade;
    // bastion (B1.6): pad where B1's bastion_slice_z lived; keeps the vec4
    // aligned. The unified overseer occlusion block follows on fresh 16-byte
    // rows — see bastion_occlusion.glsl for the field meanings. Mode 0 = off
    // (vanilla look), so vanilla/char-select are untouched.
    float bastion_pad;
    uvec4 bastion_occ_mode;      // .x mode bitmask, .y target_count
    vec4  bastion_occ_a;         // .x slice_z, .y fade_band, .z focus_z, .w prox_strength
    vec4  bastion_occ_b;         // .x height_start, .y height_end, .z dist_start, .w dist_end
    vec4  bastion_occ_c;         // .x cutaway_radius, .y roof_low, .z roof_high, .w relight_strength
    vec4  bastion_occ_d;         // .x roof_radius (near-look gate for roof reveal)
    vec4  bastion_occ_targets[4];// .xyz target in f_pos space, .w enabled
    uvec4 bastion_fog_mode;      // .x 0 legacy, 1 outdoor, 2 water, 3 underground; .y quality
    vec4  bastion_fog_distances; // .xy visual near/far; .zw gameplay terrain/entity bounds
    vec4  bastion_fog_color;     // linear RGB fog response; .w reserved
};

float distance_divider = 2.0;
float shadow_dithering = 0.5;

float tick_overflow = 300000.0;

// Get a scaled time with an offset that loops at a period.
float tick_loop(float period, float scale, float offset) {
    float loop = tick_overflow * scale;
    float rem = mod(loop, period);
    float rest = rem * tick.y;

    return mod(rest + tick.x * scale + offset, period);
}

float tick_loop(float period) {
    return tick_loop(period, 1.0, 0.0);
}

vec3 tick_loop(float period, vec3 scale, vec3 offset) {
    vec3 loop = tick_overflow * scale;
    vec3 rem = mod(loop, period);
    vec3 rest = rem * tick.y;

    return mod(rest + tick.x * scale + offset, period);
}

// Only works if t happened within tick_overflow
float time_since(float t) {
    return tick.x < t ? (tick_overflow - t + tick.x) : (tick.x - t);
}

#endif
