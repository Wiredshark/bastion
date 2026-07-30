//! Diagnostic-only multi-camera authority for
//! `POST-R2-HORIZON-MULTICAMERA-001`.
//!
//! Nothing in this module is selected unless both exact environment
//! declarations are present. The paths observe the authoritative client terrain
//! surface and own presentation camera state only; they never change streaming,
//! terrain, gameplay, or simulation policy.

use crate::{
    bastion,
    scene::camera::{Camera, CameraMode},
};
use common::terrain::TerrainGrid;
use vek::{Vec2, Vec3};

pub const MULTICAMERA_FIXTURE_V1: &str = "flat-arena-multicamera-horizon-v1";
pub const CAMERA_PATH_ENV_V1: &str = "BASTION_POST_R2_HORIZON_CAMERA_PATH";

const FIXED_SCALE: f32 = 1_000_000.0;
const MILLIMETRES_PER_BLOCK: f32 = 1_000.0;
const RADIANS_PER_DEGREE: f32 = 0.01745329;
const MIN_CAMERA_CLEARANCE_MM_V1: i64 = 2_000;
const FOCUS_CLEARANCE_MM_V1: i64 = MIN_CAMERA_CLEARANCE_MM_V1;
const MOVING_PERIOD_TICKS_V1: u64 = 3_600;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CameraPathV1 {
    #[default]
    GroundForwardOpen = 1,
    ElevatedObliqueOpen = 2,
    DenseForestForward = 3,
    RidgeHighGroundForward = 4,
    MovingForwardTurn = 5,
}

impl CameraPathV1 {
    #[must_use]
    pub const fn declaration(self) -> &'static str {
        match self {
            Self::GroundForwardOpen => "ground-forward-open",
            Self::ElevatedObliqueOpen => "elevated-oblique-open",
            Self::DenseForestForward => "dense-forest-forward",
            Self::RidgeHighGroundForward => "ridge-or-high-ground-forward",
            Self::MovingForwardTurn => "moving-forward-turn",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CameraPathSampleV1 {
    pub path: CameraPathV1,
    pub path_ordinal: u64,
    pub origin_mm: [i64; 2],
    pub focus_mm: [i64; 3],
    pub position_mm: [i64; 3],
    pub focus_surface_mm: i64,
    pub camera_surface_mm: i64,
    pub minimum_clearance_mm: i64,
    pub yaw_microradians: i64,
    pub pitch_microradians: i64,
    pub distance_mm: u64,
    pub path_token: [u8; 32],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CameraPathEvidenceV1 {
    pub selected: bool,
    pub path: CameraPathV1,
    pub path_ordinal: u64,
    pub camera_valid: bool,
    pub surface_authority_available: bool,
    pub cutaway_solid: bool,
    pub underworld_rejected: bool,
    pub sky_ground_expected: bool,
    pub focus_surface_mm: i64,
    pub camera_surface_mm: i64,
    pub minimum_clearance_mm: i64,
    pub path_token: [u8; 32],
    pub camera_token: [u8; 32],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CameraPathStateV1 {
    origin_mm: Option<[i64; 2]>,
    last_sample: Option<CameraPathSampleV1>,
    last_failure: Option<&'static str>,
}

impl CameraPathStateV1 {
    pub fn initialize(&mut self, spawn: Vec3<f32>) -> Result<(), &'static str> {
        let origin = fixed_vec2_i64(spawn.xy(), MILLIMETRES_PER_BLOCK)
            .ok_or("POST_R2_HORIZON_CAMERA_ORIGIN_INVALID")?;
        self.origin_mm = Some(origin);
        self.last_sample = None;
        self.last_failure = None;
        Ok(())
    }
}

pub fn parse_camera_path_v1(declaration: Option<&str>) -> Result<CameraPathV1, &'static str> {
    match declaration {
        Some("ground-forward-open") => Ok(CameraPathV1::GroundForwardOpen),
        Some("elevated-oblique-open") => Ok(CameraPathV1::ElevatedObliqueOpen),
        Some("dense-forest-forward") => Ok(CameraPathV1::DenseForestForward),
        Some("ridge-or-high-ground-forward") => Ok(CameraPathV1::RidgeHighGroundForward),
        Some("moving-forward-turn") => Ok(CameraPathV1::MovingForwardTurn),
        None => Err("POST_R2_HORIZON_CAMERA_PATH_REQUIRED"),
        Some(_) => Err("POST_R2_HORIZON_CAMERA_PATH_INVALID"),
    }
}

pub fn selected_camera_path_v1() -> Result<Option<CameraPathV1>, &'static str> {
    let fixture = match std::env::var("BASTION_POST_R2_VISIBLE_HORIZON") {
        Err(std::env::VarError::NotPresent) => None,
        Ok(value) => Some(value),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err("POST_R2_HORIZON_MULTICAMERA_INVALID_DECLARATION");
        },
    };
    let path = match std::env::var(CAMERA_PATH_ENV_V1) {
        Err(std::env::VarError::NotPresent) => None,
        Ok(value) => Some(value),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err("POST_R2_HORIZON_CAMERA_PATH_INVALID");
        },
    };

    match (fixture.as_deref(), path.as_deref()) {
        (Some(MULTICAMERA_FIXTURE_V1), declaration) => parse_camera_path_v1(declaration).map(Some),
        (None, None) | (Some(crate::post_r2_visible_horizon::VISIBLE_HORIZON_FIXTURE_V1), None) => {
            Ok(None)
        },
        _ => Err("POST_R2_HORIZON_MULTICAMERA_DECLARATION_MISMATCH"),
    }
}

pub fn apply_post_maintenance_path_v1(
    camera: &mut Camera,
    terrain: &TerrainGrid,
    state: &mut CameraPathStateV1,
    path: CameraPathV1,
    server_tick: u64,
    configured_fov_degrees: u16,
) -> Result<CameraPathSampleV1, &'static str> {
    let origin_mm = state
        .origin_mm
        .ok_or("POST_R2_HORIZON_CAMERA_ORIGIN_UNAVAILABLE")?;
    let sample = sample_camera_path_v1(path, origin_mm, server_tick, |xy_mm| {
        let xy = Vec2::new(
            xy_mm[0] as f32 / MILLIMETRES_PER_BLOCK,
            xy_mm[1] as f32 / MILLIMETRES_PER_BLOCK,
        );
        bastion::ground_z(terrain, xy, 1.0).and_then(|z| fixed_i64(z, MILLIMETRES_PER_BLOCK))
    });

    let sample = match sample {
        Ok(sample) => sample,
        Err(error) => {
            state.last_failure = Some(error);
            state.last_sample = None;
            return Err(error);
        },
    };
    camera.set_mode(CameraMode::Overseer);
    camera.set_orientation_instant(Vec3::new(
        sample.yaw_microradians as f32 / FIXED_SCALE,
        sample.pitch_microradians as f32 / FIXED_SCALE,
        0.0,
    ));
    camera.set_distance_instant(sample.distance_mm as f32 / MILLIMETRES_PER_BLOCK);
    camera.set_fov_instant(f32::from(configured_fov_degrees) * RADIANS_PER_DEGREE);
    camera.set_fixate_instant(1.0);
    camera.force_focus_pos(Vec3::new(
        sample.focus_mm[0] as f32 / MILLIMETRES_PER_BLOCK,
        sample.focus_mm[1] as f32 / MILLIMETRES_PER_BLOCK,
        sample.focus_mm[2] as f32 / MILLIMETRES_PER_BLOCK,
    ));
    state.last_sample = Some(sample);
    state.last_failure = None;
    Ok(sample)
}

pub fn camera_path_evidence_v1(
    camera: &Camera,
    state: &CameraPathStateV1,
    selected_path: Option<CameraPathV1>,
    configured_fov_degrees: u16,
    cutaway_solid: bool,
) -> CameraPathEvidenceV1 {
    let Some(path) = selected_path else {
        return CameraPathEvidenceV1::default();
    };
    let Some(sample) = state.last_sample.filter(|sample| sample.path == path) else {
        return CameraPathEvidenceV1 {
            selected: true,
            path,
            ..CameraPathEvidenceV1::default()
        };
    };

    let focus_mm = fixed_vec3_i64(camera.get_focus_pos(), MILLIMETRES_PER_BLOCK);
    let orientation = camera.get_orientation();
    let yaw = fixed_i64(orientation.x, FIXED_SCALE);
    let pitch = fixed_i64(orientation.y, FIXED_SCALE);
    let distance = fixed_u64(camera.get_distance(), MILLIMETRES_PER_BLOCK);
    let (base_fov, target_fov) = camera.fov_state_v1();
    let configured_fov = fixed_u64(
        f32::from(configured_fov_degrees) * RADIANS_PER_DEGREE,
        FIXED_SCALE,
    );
    let base_fov = fixed_u64(base_fov, FIXED_SCALE);
    let target_fov = fixed_u64(target_fov, FIXED_SCALE);
    let effective_fov = fixed_u64(camera.get_effective_fov(), FIXED_SCALE);
    let (fixation, target_fixation) = camera.fixation_state_v1();
    let fixation = fixed_u64(fixation, FIXED_SCALE);
    let target_fixation = fixed_u64(target_fixation, FIXED_SCALE);
    let aspect = fixed_u64(camera.get_aspect_ratio(), FIXED_SCALE);
    let position = camera.get_focus_pos() - camera.forward() * camera.get_distance();
    let position_mm = fixed_vec3_i64(position, MILLIMETRES_PER_BLOCK);
    let half_height = camera.get_distance() * (camera.get_effective_fov() / 2.0).tan();
    let ground_sine = orientation.y.sin().abs();
    let frustum_ground_width_mm = (half_height.is_finite() && aspect.is_some())
        .then(|| {
            fixed_u64(
                2.0 * half_height * camera.get_aspect_ratio(),
                MILLIMETRES_PER_BLOCK,
            )
        })
        .flatten();
    let frustum_ground_depth_mm = (half_height.is_finite() && ground_sine >= 0.001)
        .then(|| fixed_u64(2.0 * half_height / ground_sine, MILLIMETRES_PER_BLOCK))
        .flatten();
    let mode_tag = camera.get_mode() as u8;
    let projection_tag = u8::from(camera.get_mode() == CameraMode::Overseer);

    let exact_camera = focus_mm == Some(sample.focus_mm)
        && position_mm == Some(sample.position_mm)
        && yaw == Some(sample.yaw_microradians)
        && pitch == Some(sample.pitch_microradians)
        && distance == Some(sample.distance_mm)
        && configured_fov.is_some()
        && base_fov == configured_fov
        && target_fov == configured_fov
        && effective_fov == configured_fov
        && fixation == Some(1_000_000)
        && target_fixation == Some(1_000_000)
        && aspect.is_some()
        && frustum_ground_width_mm.is_some()
        && frustum_ground_depth_mm.is_some()
        && camera.get_mode() == CameraMode::Overseer;
    let underworld_rejected = sample.minimum_clearance_mm < MIN_CAMERA_CLEARANCE_MM_V1;
    let camera_valid = exact_camera && cutaway_solid && !underworld_rejected;

    let mut payload = Vec::with_capacity(224);
    payload.extend_from_slice(MULTICAMERA_FIXTURE_V1.as_bytes());
    payload.extend_from_slice(&[path as u8, mode_tag, projection_tag]);
    payload.extend_from_slice(&sample.path_token);
    for value in focus_mm.unwrap_or([0; 3]) {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in position_mm.unwrap_or([0; 3]) {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&yaw.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&pitch.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&distance.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&configured_fov.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&base_fov.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&target_fov.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&effective_fov.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&fixation.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&target_fixation.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&aspect.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&frustum_ground_width_mm.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&frustum_ground_depth_mm.unwrap_or(0).to_le_bytes());
    payload.extend_from_slice(&sample.minimum_clearance_mm.to_le_bytes());
    payload.push(u8::from(cutaway_solid));
    let camera_token = bastion_renderer_r0d::domain_hash_v1(
        "bastion/post-r2/horizon-multicamera-camera",
        1,
        0,
        &payload,
    )
    .unwrap_or([0; 32]);

    CameraPathEvidenceV1 {
        selected: true,
        path,
        path_ordinal: sample.path_ordinal,
        camera_valid,
        surface_authority_available: true,
        cutaway_solid,
        underworld_rejected,
        sky_ground_expected: matches!(
            path,
            CameraPathV1::GroundForwardOpen
                | CameraPathV1::ElevatedObliqueOpen
                | CameraPathV1::RidgeHighGroundForward
                | CameraPathV1::MovingForwardTurn
        ),
        focus_surface_mm: sample.focus_surface_mm,
        camera_surface_mm: sample.camera_surface_mm,
        minimum_clearance_mm: sample.minimum_clearance_mm,
        path_token: sample.path_token,
        camera_token,
    }
}

fn sample_camera_path_v1(
    path: CameraPathV1,
    origin_mm: [i64; 2],
    path_ordinal: u64,
    mut surface_at: impl FnMut([i64; 2]) -> Option<i64>,
) -> Result<CameraPathSampleV1, &'static str> {
    let (focus_xy, yaw, pitch, distance) =
        path_parameters_v1(path, origin_mm, path_ordinal, &mut surface_at)?;
    let focus_surface = surface_at(focus_xy).ok_or("POST_R2_HORIZON_FOCUS_SURFACE_UNAVAILABLE")?;
    let mut focus = [
        focus_xy[0],
        focus_xy[1],
        focus_surface + FOCUS_CLEARANCE_MM_V1,
    ];
    let forward = forward_fixed_v1(yaw, pitch);
    let nominal_camera = camera_position_v1(focus, forward, distance)?;
    let camera_xy = [nominal_camera[0], nominal_camera[1]];
    let camera_surface =
        surface_at(camera_xy).ok_or("POST_R2_HORIZON_CAMERA_SURFACE_UNAVAILABLE")?;

    let mut lift = (camera_surface + MIN_CAMERA_CLEARANCE_MM_V1 - nominal_camera[2]).max(0);
    for numerator in [1_i64, 2, 3] {
        let point = [
            focus[0] + (nominal_camera[0] - focus[0]) * numerator / 4,
            focus[1] + (nominal_camera[1] - focus[1]) * numerator / 4,
            focus[2] + (nominal_camera[2] - focus[2]) * numerator / 4,
        ];
        let surface = surface_at([point[0], point[1]])
            .ok_or("POST_R2_HORIZON_SIGHTLINE_SURFACE_UNAVAILABLE")?;
        lift = lift.max(surface + MIN_CAMERA_CLEARANCE_MM_V1 - point[2]);
    }
    focus[2] = focus[2]
        .checked_add(lift)
        .ok_or("POST_R2_HORIZON_CAMERA_CLEARANCE_OVERFLOW")?;
    let position = camera_position_v1(focus, forward, distance)?;

    let mut minimum_clearance = position[2] - camera_surface;
    for numerator in [0_i64, 1, 2, 3, 4] {
        let point = [
            focus[0] + (position[0] - focus[0]) * numerator / 4,
            focus[1] + (position[1] - focus[1]) * numerator / 4,
            focus[2] + (position[2] - focus[2]) * numerator / 4,
        ];
        let surface = surface_at([point[0], point[1]])
            .ok_or("POST_R2_HORIZON_SIGHTLINE_SURFACE_UNAVAILABLE")?;
        minimum_clearance = minimum_clearance.min(point[2] - surface);
    }
    if minimum_clearance < MIN_CAMERA_CLEARANCE_MM_V1 {
        return Err("POST_R2_HORIZON_CAMERA_INTERSECTS_TERRAIN");
    }

    let mut token_payload = Vec::with_capacity(96);
    token_payload.extend_from_slice(MULTICAMERA_FIXTURE_V1.as_bytes());
    token_payload.push(path as u8);
    token_payload.extend_from_slice(path.declaration().as_bytes());
    for value in origin_mm {
        token_payload.extend_from_slice(&value.to_le_bytes());
    }
    token_payload.extend_from_slice(&1_u32.to_le_bytes());
    let path_token = bastion_renderer_r0d::domain_hash_v1(
        "bastion/post-r2/horizon-camera-path",
        1,
        0,
        &token_payload,
    )
    .map_err(|_| "POST_R2_HORIZON_CAMERA_PATH_TOKEN_FAILED")?;

    Ok(CameraPathSampleV1 {
        path,
        path_ordinal,
        origin_mm,
        focus_mm: focus,
        position_mm: position,
        focus_surface_mm: focus_surface,
        camera_surface_mm: camera_surface,
        minimum_clearance_mm: minimum_clearance,
        yaw_microradians: yaw,
        pitch_microradians: pitch,
        distance_mm: distance,
        path_token,
    })
}

fn path_parameters_v1(
    path: CameraPathV1,
    origin: [i64; 2],
    ordinal: u64,
    surface_at: &mut impl FnMut([i64; 2]) -> Option<i64>,
) -> Result<([i64; 2], i64, i64, u64), &'static str> {
    let result = match path {
        CameraPathV1::GroundForwardOpen => (origin, 0, 8_727, 64_000),
        CameraPathV1::ElevatedObliqueOpen => {
            ([origin[0] - 128_000, origin[1]], 0, 436_332, 256_000)
        },
        CameraPathV1::DenseForestForward => (
            [origin[0] + 128_000, origin[1] + 128_000],
            0,
            17_453,
            64_000,
        ),
        CameraPathV1::RidgeHighGroundForward => {
            let offsets = [
                [-256_000, -256_000],
                [-256_000, 0],
                [-256_000, 256_000],
                [0, -256_000],
                [0, 0],
                [0, 256_000],
                [256_000, -256_000],
                [256_000, 0],
                [256_000, 256_000],
            ];
            let mut selected = None;
            for offset in offsets {
                let xy = [origin[0] + offset[0], origin[1] + offset[1]];
                let height = surface_at(xy).ok_or("POST_R2_HORIZON_RIDGE_SURFACE_UNAVAILABLE")?;
                let candidate = (height, -xy[0], -xy[1], xy);
                if selected.is_none_or(|best| candidate > best) {
                    selected = Some(candidate);
                }
            }
            let xy = selected
                .ok_or("POST_R2_HORIZON_RIDGE_SURFACE_UNAVAILABLE")?
                .3;
            (xy, 0, 87_266, 128_000)
        },
        CameraPathV1::MovingForwardTurn => {
            let phase = ordinal % MOVING_PERIOD_TICKS_V1;
            let half = MOVING_PERIOD_TICKS_V1 / 2;
            let triangle = if phase <= half {
                phase as i64
            } else {
                (MOVING_PERIOD_TICKS_V1 - phase) as i64
            };
            let centered = triangle * 2 - half as i64;
            let x = origin[0] + centered * 128_000 / half as i64;
            let y = origin[1] + centered * 192_000 / half as i64;
            let yaw = centered * 349_066 / half as i64;
            ([x, y], yaw, 87_266, 96_000)
        },
    };
    Ok(result)
}

fn forward_fixed_v1(yaw_microradians: i64, pitch_microradians: i64) -> [f32; 3] {
    let yaw = yaw_microradians as f32 / FIXED_SCALE;
    let pitch = pitch_microradians as f32 / FIXED_SCALE;
    [
        yaw.sin() * pitch.cos(),
        yaw.cos() * pitch.cos(),
        -pitch.sin(),
    ]
}

fn camera_position_v1(
    focus_mm: [i64; 3],
    forward: [f32; 3],
    distance_mm: u64,
) -> Result<[i64; 3], &'static str> {
    let distance = distance_mm as f32 / MILLIMETRES_PER_BLOCK;
    let mut result = [0_i64; 3];
    for index in 0..3 {
        let focus_blocks = focus_mm[index] as f32 / MILLIMETRES_PER_BLOCK;
        result[index] = fixed_i64(
            focus_blocks - forward[index] * distance,
            MILLIMETRES_PER_BLOCK,
        )
        .ok_or("POST_R2_HORIZON_CAMERA_POSITION_OVERFLOW")?;
    }
    Ok(result)
}

fn fixed_vec2_i64(value: Vec2<f32>, scale: f32) -> Option<[i64; 2]> {
    Some([fixed_i64(value.x, scale)?, fixed_i64(value.y, scale)?])
}

fn fixed_vec3_i64(value: Vec3<f32>, scale: f32) -> Option<[i64; 3]> {
    Some([
        fixed_i64(value.x, scale)?,
        fixed_i64(value.y, scale)?,
        fixed_i64(value.z, scale)?,
    ])
}

fn fixed_i64(value: f32, scale: f32) -> Option<i64> {
    let scaled = f64::from(value) * f64::from(scale);
    (scaled.is_finite() && scaled >= i64::MIN as f64 && scaled <= i64::MAX as f64)
        .then(|| scaled.round() as i64)
}

fn fixed_u64(value: f32, scale: f32) -> Option<u64> {
    let scaled = f64::from(value) * f64::from(scale);
    (scaled.is_finite() && scaled >= 0.0 && scaled <= u64::MAX as f64)
        .then(|| scaled.round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_surface(_: [i64; 2]) -> Option<i64> { Some(40_000) }

    fn camera_and_state(sample: CameraPathSampleV1) -> (Camera, CameraPathStateV1) {
        let mut camera = Camera::new(16.0 / 9.0, CameraMode::Overseer);
        camera.set_orientation_instant(Vec3::new(
            sample.yaw_microradians as f32 / FIXED_SCALE,
            sample.pitch_microradians as f32 / FIXED_SCALE,
            0.0,
        ));
        camera.set_distance_instant(sample.distance_mm as f32 / MILLIMETRES_PER_BLOCK);
        camera.set_fov_instant(70.0 * RADIANS_PER_DEGREE);
        camera.set_fixate_instant(1.0);
        camera.force_focus_pos(Vec3::new(
            sample.focus_mm[0] as f32 / MILLIMETRES_PER_BLOCK,
            sample.focus_mm[1] as f32 / MILLIMETRES_PER_BLOCK,
            sample.focus_mm[2] as f32 / MILLIMETRES_PER_BLOCK,
        ));
        let state = CameraPathStateV1 {
            origin_mm: Some(sample.origin_mm),
            last_sample: Some(sample),
            last_failure: None,
        };
        (camera, state)
    }

    #[test]
    fn declarations_are_exact_and_bounded() {
        for path in [
            CameraPathV1::GroundForwardOpen,
            CameraPathV1::ElevatedObliqueOpen,
            CameraPathV1::DenseForestForward,
            CameraPathV1::RidgeHighGroundForward,
            CameraPathV1::MovingForwardTurn,
        ] {
            assert_eq!(parse_camera_path_v1(Some(path.declaration())), Ok(path));
        }
        assert_eq!(
            parse_camera_path_v1(Some("open")),
            Err("POST_R2_HORIZON_CAMERA_PATH_INVALID")
        );
        assert_eq!(
            parse_camera_path_v1(None),
            Err("POST_R2_HORIZON_CAMERA_PATH_REQUIRED")
        );
    }

    #[test]
    fn all_paths_are_surface_cleared_and_deterministic() {
        let origin = [16_384_500, 16_384_500];
        for path in [
            CameraPathV1::GroundForwardOpen,
            CameraPathV1::ElevatedObliqueOpen,
            CameraPathV1::DenseForestForward,
            CameraPathV1::RidgeHighGroundForward,
            CameraPathV1::MovingForwardTurn,
        ] {
            let first =
                sample_camera_path_v1(path, origin, 1_234, flat_surface).expect("valid sample");
            let replay = sample_camera_path_v1(path, origin, 1_234, flat_surface).expect("replay");
            assert_eq!(first, replay);
            assert!(first.minimum_clearance_mm >= MIN_CAMERA_CLEARANCE_MM_V1);
            assert!(first.position_mm[2] > first.camera_surface_mm);
        }
    }

    #[test]
    fn pair_identity_is_independent_of_view_distance() {
        let origin = [16_384_500, 16_384_500];
        let reference = sample_camera_path_v1(
            CameraPathV1::ElevatedObliqueOpen,
            origin,
            9_000,
            flat_surface,
        )
        .expect("reference");
        let far = sample_camera_path_v1(
            CameraPathV1::ElevatedObliqueOpen,
            origin,
            9_000,
            flat_surface,
        )
        .expect("far");
        assert_eq!(reference, far);
    }

    #[test]
    fn ground_and_forest_paths_are_forward_facing_near_surface() {
        let origin = [16_384_500, 16_384_500];
        for path in [
            CameraPathV1::GroundForwardOpen,
            CameraPathV1::DenseForestForward,
        ] {
            let sample = sample_camera_path_v1(path, origin, 7_000, flat_surface).expect("sample");
            assert!(sample.pitch_microradians > 0);
            assert!(sample.pitch_microradians <= 20_000);
            assert!(sample.position_mm[2] - sample.camera_surface_mm <= 4_000);
        }
    }

    #[test]
    fn complete_camera_token_rejects_later_aspect_drift() {
        let sample = sample_camera_path_v1(
            CameraPathV1::ElevatedObliqueOpen,
            [16_384_500, 16_384_500],
            9_000,
            flat_surface,
        )
        .expect("sample");
        let (mut camera, state) = camera_and_state(sample);
        let accepted = camera_path_evidence_v1(&camera, &state, Some(sample.path), 70, true);
        assert!(accepted.camera_valid);

        camera.set_aspect_ratio(4.0 / 3.0);
        let drifted = camera_path_evidence_v1(&camera, &state, Some(sample.path), 70, true);
        assert!(drifted.camera_valid);
        assert_ne!(accepted.camera_token, drifted.camera_token);
    }

    #[test]
    fn unavailable_surface_and_underworld_evidence_fail_closed() {
        let origin = [0, 0];
        assert_eq!(
            sample_camera_path_v1(CameraPathV1::GroundForwardOpen, origin, 1, |_| None),
            Err("POST_R2_HORIZON_FOCUS_SURFACE_UNAVAILABLE")
        );
        let discontinuous = |xy: [i64; 2]| {
            if xy[1] < -20_000 {
                Some(1_000_000)
            } else {
                Some(0)
            }
        };
        assert!(
            sample_camera_path_v1(CameraPathV1::GroundForwardOpen, origin, 1, discontinuous)
                .is_ok()
        );

        let mut underworld =
            sample_camera_path_v1(CameraPathV1::GroundForwardOpen, origin, 1, flat_surface)
                .expect("sample");
        underworld.minimum_clearance_mm = MIN_CAMERA_CLEARANCE_MM_V1 - 1;
        let (camera, state) = camera_and_state(underworld);
        let evidence = camera_path_evidence_v1(&camera, &state, Some(underworld.path), 70, true);
        assert!(evidence.underworld_rejected);
        assert!(!evidence.camera_valid);
    }

    #[test]
    fn moving_path_is_continuous_and_replayable_at_cycle_boundary() {
        let origin = [16_384_500, 16_384_500];
        let mut prior = sample_camera_path_v1(
            CameraPathV1::MovingForwardTurn,
            origin,
            MOVING_PERIOD_TICKS_V1 - 1,
            flat_surface,
        )
        .expect("prior");
        for ordinal in [MOVING_PERIOD_TICKS_V1, MOVING_PERIOD_TICKS_V1 + 1] {
            let current = sample_camera_path_v1(
                CameraPathV1::MovingForwardTurn,
                origin,
                ordinal,
                flat_surface,
            )
            .expect("current");
            let dx = (current.focus_mm[0] - prior.focus_mm[0]).abs();
            let dy = (current.focus_mm[1] - prior.focus_mm[1]).abs();
            assert!(dx <= 200);
            assert!(dy <= 300);
            prior = current;
        }
    }

    #[test]
    fn default_state_has_no_camera_authority() {
        let camera = Camera::new(16.0 / 9.0, CameraMode::ThirdPerson);
        let evidence =
            camera_path_evidence_v1(&camera, &CameraPathStateV1::default(), None, 70, false);
        assert_eq!(evidence, CameraPathEvidenceV1::default());
    }
}
