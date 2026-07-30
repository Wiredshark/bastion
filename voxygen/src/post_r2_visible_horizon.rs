//! Diagnostic-only horizon-facing camera authority for
//! `POST-R2-VISIBLE-HORIZON-DIAG-002`.
//!
//! The ordinary flat-arena path keeps the existing Overseer camera. This
//! module is active only for one exact declaration and changes presentation
//! camera state only; it never changes terrain policy, simulation, gameplay,
//! culling, or draw admission.

use crate::scene::camera::{Camera, CameraMode};
#[cfg(test)] use vek::Vec2;
use vek::Vec3;

pub const VISIBLE_HORIZON_FIXTURE_V1: &str = "flat-arena-oblique-horizon-v1";
pub const CAMERA_YAW_MICRORADIANS_V1: i64 = 0;
pub const CAMERA_PITCH_MICRORADIANS_V1: i64 = 349_066;
pub const CAMERA_DISTANCE_MM_V1: u64 = 384_000;
pub const CAMERA_FOCUS_Z_MM_V1: i64 = 1_000;

const FIXED_SCALE: f32 = 1_000_000.0;
const MILLIMETRES_PER_BLOCK: f32 = 1_000.0;
const RADIANS_PER_DEGREE: f32 = 0.01745329;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HorizonCameraEvidenceV1 {
    pub fixture_selected: bool,
    pub camera_valid: bool,
    pub mode_tag: u8,
    pub projection_tag: u8,
    pub focus_mm: [i64; 3],
    pub position_mm: [i64; 3],
    pub yaw_microradians: i64,
    pub pitch_microradians: i64,
    pub distance_mm: u64,
    pub configured_base_fov_microradians: u64,
    pub base_fov_microradians: u64,
    pub target_base_fov_microradians: u64,
    pub fov_microradians: u64,
    pub fixation_millionths: u64,
    pub target_fixation_millionths: u64,
    pub aspect_millionths: u64,
    pub frustum_ground_width_mm: u64,
    pub frustum_ground_depth_mm: u64,
    pub camera_token: [u8; 32],
}

pub fn parse_visible_horizon_fixture_v1(declaration: Option<&str>) -> Result<bool, &'static str> {
    match declaration {
        None => Ok(false),
        Some(VISIBLE_HORIZON_FIXTURE_V1) => Ok(true),
        Some(_) => Err("POST_R2_VISIBLE_HORIZON_INVALID_DECLARATION"),
    }
}

pub fn visible_horizon_fixture_selected_v1() -> Result<bool, &'static str> {
    match std::env::var("BASTION_POST_R2_VISIBLE_HORIZON") {
        Err(std::env::VarError::NotPresent) => parse_visible_horizon_fixture_v1(None),
        Ok(value) => parse_visible_horizon_fixture_v1(Some(&value)),
        Err(std::env::VarError::NotUnicode(_)) => parse_visible_horizon_fixture_v1(Some("")),
    }
}

pub fn configure_camera_v1(camera: &mut Camera, spawn: Vec3<f32>) {
    camera.set_mode(CameraMode::Overseer);
    camera.set_orientation_instant(Vec3::new(
        CAMERA_YAW_MICRORADIANS_V1 as f32 / FIXED_SCALE,
        CAMERA_PITCH_MICRORADIANS_V1 as f32 / FIXED_SCALE,
        0.0,
    ));
    camera.set_distance_instant(CAMERA_DISTANCE_MM_V1 as f32 / MILLIMETRES_PER_BLOCK);
    camera.set_fixate_instant(1.0);
    camera.force_focus_pos(Vec3::new(
        spawn.x,
        spawn.y,
        CAMERA_FOCUS_Z_MM_V1 as f32 / MILLIMETRES_PER_BLOCK,
    ));
}

/// Apply the exact diagnostic authority after ordinary camera maintenance.
///
/// The opt-in deliberately owns focus height, base FOV, and fixation.
/// Orientation, distance, and aspect remain ordinary camera state and drift
/// in any of them is still visible to the canonical evidence/token.
pub fn apply_post_maintenance_camera_v1(
    camera: &mut Camera,
    fixture_selected: bool,
    configured_fov_degrees: u16,
) {
    if !fixture_selected {
        return;
    }
    let focus = camera
        .get_focus_pos()
        .xy()
        .with_z(CAMERA_FOCUS_Z_MM_V1 as f32 / MILLIMETRES_PER_BLOCK);
    camera.force_focus_pos(focus);
    camera.set_fov_instant(f32::from(configured_fov_degrees) * RADIANS_PER_DEGREE);
    camera.set_fixate_instant(1.0);
}

pub fn camera_evidence_v1(
    camera: &Camera,
    fixture_selected: bool,
    configured_fov_degrees: u16,
) -> Option<HorizonCameraEvidenceV1> {
    let focus = camera.get_focus_pos();
    let orientation = camera.get_orientation();
    let distance = camera.get_distance();
    let configured_base_fov = f32::from(configured_fov_degrees) * RADIANS_PER_DEGREE;
    let (base_fov, target_base_fov) = camera.fov_state_v1();
    let fov = camera.get_effective_fov();
    let (fixation, target_fixation) = camera.fixation_state_v1();
    let aspect = camera.get_aspect_ratio();
    if !focus.x.is_finite()
        || !focus.y.is_finite()
        || !focus.z.is_finite()
        || !orientation.x.is_finite()
        || !orientation.y.is_finite()
        || !orientation.z.is_finite()
        || !distance.is_finite()
        || !configured_base_fov.is_finite()
        || !base_fov.is_finite()
        || !target_base_fov.is_finite()
        || !fov.is_finite()
        || !fixation.is_finite()
        || !target_fixation.is_finite()
        || !aspect.is_finite()
        || distance <= 0.0
        || configured_base_fov <= 0.0
        || base_fov <= 0.0
        || target_base_fov <= 0.0
        || fov <= 0.0
        || aspect <= 0.0
    {
        return None;
    }

    let focus_mm = fixed_vec3_i64(focus, MILLIMETRES_PER_BLOCK)?;
    let yaw_microradians = fixed_i64(orientation.x, FIXED_SCALE)?;
    let pitch_microradians = fixed_i64(orientation.y, FIXED_SCALE)?;
    let distance_mm = fixed_u64(distance, MILLIMETRES_PER_BLOCK)?;
    let configured_base_fov_microradians = fixed_u64(configured_base_fov, FIXED_SCALE)?;
    let base_fov_microradians = fixed_u64(base_fov, FIXED_SCALE)?;
    let target_base_fov_microradians = fixed_u64(target_base_fov, FIXED_SCALE)?;
    let fov_microradians = fixed_u64(fov, FIXED_SCALE)?;
    let fixation_millionths = fixed_u64(fixation, FIXED_SCALE)?;
    let target_fixation_millionths = fixed_u64(target_fixation, FIXED_SCALE)?;
    let aspect_millionths = fixed_u64(aspect, FIXED_SCALE)?;

    let forward = camera.forward();
    let position = focus - forward * distance;
    let position_mm = fixed_vec3_i64(position, MILLIMETRES_PER_BLOCK)?;
    let half_height = distance * (fov / 2.0).tan();
    let ground_sine = orientation.y.sin().abs();
    if !half_height.is_finite() || ground_sine < 0.001 {
        return None;
    }
    let frustum_ground_width_mm = fixed_u64(2.0 * half_height * aspect, MILLIMETRES_PER_BLOCK)?;
    let frustum_ground_depth_mm =
        fixed_u64(2.0 * half_height / ground_sine, MILLIMETRES_PER_BLOCK)?;

    let mode_tag = camera.get_mode() as u8;
    let projection_tag = u8::from(camera.get_mode() == CameraMode::Overseer);
    let camera_valid = fixture_selected
        && mode_tag == CameraMode::Overseer as u8
        && projection_tag == 1
        && yaw_microradians == CAMERA_YAW_MICRORADIANS_V1
        && pitch_microradians == CAMERA_PITCH_MICRORADIANS_V1
        && distance_mm == CAMERA_DISTANCE_MM_V1
        && focus_mm[2] == CAMERA_FOCUS_Z_MM_V1
        && base_fov_microradians == configured_base_fov_microradians
        && target_base_fov_microradians == configured_base_fov_microradians
        && fov_microradians == configured_base_fov_microradians
        && fixation_millionths == 1_000_000
        && target_fixation_millionths == 1_000_000
        && frustum_ground_depth_mm >= 1_000_000;

    let mut payload = Vec::with_capacity(192);
    payload.extend_from_slice(VISIBLE_HORIZON_FIXTURE_V1.as_bytes());
    payload.extend_from_slice(&[mode_tag, projection_tag]);
    for value in focus_mm {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in position_mm {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload.extend_from_slice(&yaw_microradians.to_le_bytes());
    payload.extend_from_slice(&pitch_microradians.to_le_bytes());
    payload.extend_from_slice(&distance_mm.to_le_bytes());
    payload.extend_from_slice(&configured_base_fov_microradians.to_le_bytes());
    payload.extend_from_slice(&base_fov_microradians.to_le_bytes());
    payload.extend_from_slice(&target_base_fov_microradians.to_le_bytes());
    payload.extend_from_slice(&fov_microradians.to_le_bytes());
    payload.extend_from_slice(&fixation_millionths.to_le_bytes());
    payload.extend_from_slice(&target_fixation_millionths.to_le_bytes());
    payload.extend_from_slice(&aspect_millionths.to_le_bytes());
    payload.extend_from_slice(&frustum_ground_width_mm.to_le_bytes());
    payload.extend_from_slice(&frustum_ground_depth_mm.to_le_bytes());
    let camera_token = bastion_renderer_r0d::domain_hash_v1(
        "bastion/post-r2/visible-horizon-camera",
        1,
        0,
        &payload,
    )
    .ok()?;

    Some(HorizonCameraEvidenceV1 {
        fixture_selected,
        camera_valid,
        mode_tag,
        projection_tag,
        focus_mm,
        position_mm,
        yaw_microradians,
        pitch_microradians,
        distance_mm,
        configured_base_fov_microradians,
        base_fov_microradians,
        target_base_fov_microradians,
        fov_microradians,
        fixation_millionths,
        target_fixation_millionths,
        aspect_millionths,
        frustum_ground_width_mm,
        frustum_ground_depth_mm,
        camera_token,
    })
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
pub fn declared_ground_footprint_blocks_v1() -> Option<Vec2<u64>> {
    let pitch = CAMERA_PITCH_MICRORADIANS_V1 as f32 / FIXED_SCALE;
    let distance = CAMERA_DISTANCE_MM_V1 as f32 / MILLIMETRES_PER_BLOCK;
    let configured_fov = 70.0 * RADIANS_PER_DEGREE;
    let half_height = distance * (configured_fov / 2.0).tan();
    let width = fixed_u64(2.0 * half_height * (16.0 / 9.0), 1.0)?;
    let depth = fixed_u64(2.0 * half_height / pitch.sin().abs(), 1.0)?;
    Some(Vec2::new(width, depth))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_is_exact_and_fail_closed() {
        assert_eq!(parse_visible_horizon_fixture_v1(None), Ok(false));
        assert_eq!(
            parse_visible_horizon_fixture_v1(Some(VISIBLE_HORIZON_FIXTURE_V1)),
            Ok(true)
        );
        assert_eq!(
            parse_visible_horizon_fixture_v1(Some("flat-arena-oblique-horizon")),
            Err("POST_R2_VISIBLE_HORIZON_INVALID_DECLARATION")
        );
    }

    #[test]
    fn declared_camera_footprint_reaches_beyond_twenty_four_chunks() {
        let footprint = declared_ground_footprint_blocks_v1().expect("bounded footprint");
        assert!(footprint.x >= 700);
        assert!(footprint.y >= 1_300);
        assert!(footprint.y > 24 * 32);
    }

    #[test]
    fn camera_evidence_binds_every_declared_field_and_rejects_drift() {
        let spawn = Vec3::new(10_000.5, 20_000.5, 1.0);
        let mut camera = Camera::new(16.0 / 9.0, CameraMode::ThirdPerson);
        configure_camera_v1(&mut camera, spawn);
        apply_post_maintenance_camera_v1(&mut camera, true, 70);
        let accepted = camera_evidence_v1(&camera, true, 70).expect("camera evidence");
        assert!(accepted.camera_valid);
        assert_eq!(accepted.mode_tag, CameraMode::Overseer as u8);
        assert_eq!(accepted.projection_tag, 1);
        assert_eq!(accepted.focus_mm[2], CAMERA_FOCUS_Z_MM_V1);
        assert_eq!(accepted.distance_mm, CAMERA_DISTANCE_MM_V1);
        assert_eq!(
            accepted.configured_base_fov_microradians,
            accepted.base_fov_microradians
        );
        assert_eq!(
            accepted.configured_base_fov_microradians,
            accepted.target_base_fov_microradians
        );
        assert_eq!(
            accepted.configured_base_fov_microradians,
            accepted.fov_microradians
        );
        assert_eq!(accepted.fixation_millionths, 1_000_000);
        assert_eq!(accepted.target_fixation_millionths, 1_000_000);
        assert!(accepted.frustum_ground_depth_mm > 24 * 32 * 1_000);

        camera.force_focus_pos(camera.get_focus_pos().xy().with_z(2.0));
        let drifted = camera_evidence_v1(&camera, true, 70).expect("drift evidence");
        assert!(!drifted.camera_valid);
        assert_ne!(accepted.camera_token, drifted.camera_token);

        apply_post_maintenance_camera_v1(&mut camera, true, 70);
        camera.set_fixate_instant(0.5);
        let fixate_drifted = camera_evidence_v1(&camera, true, 70).expect("fixate drift evidence");
        assert!(!fixate_drifted.camera_valid);
        assert_ne!(accepted.camera_token, fixate_drifted.camera_token);

        apply_post_maintenance_camera_v1(&mut camera, true, 70);
        camera.set_fov(0.9);
        let target_fov_drifted =
            camera_evidence_v1(&camera, true, 70).expect("target FOV drift evidence");
        assert!(!target_fov_drifted.camera_valid);
        assert_ne!(accepted.camera_token, target_fov_drifted.camera_token);

        camera.set_fov_instant(0.9);
        let base_fov_drifted =
            camera_evidence_v1(&camera, true, 70).expect("base FOV drift evidence");
        assert!(!base_fov_drifted.camera_valid);
        assert_ne!(accepted.camera_token, base_fov_drifted.camera_token);
    }

    #[test]
    fn post_maintenance_authority_is_exact_across_reference_and_far_profiles() {
        let spawn = Vec3::new(16_384.5, 16_384.5, 1.0);
        let mut reference = Camera::new(16.0 / 9.0, CameraMode::ThirdPerson);
        let mut far = Camera::new(16.0 / 9.0, CameraMode::ThirdPerson);
        configure_camera_v1(&mut reference, spawn);
        configure_camera_v1(&mut far, spawn);
        reference.set_fov_instant(0.8);
        far.set_fov_instant(1.4);
        reference.set_fixate(0.4);
        far.set_fixate(0.7);

        apply_post_maintenance_camera_v1(&mut reference, true, 70);
        apply_post_maintenance_camera_v1(&mut far, true, 70);
        let reference_evidence =
            camera_evidence_v1(&reference, true, 70).expect("reference camera evidence");
        let far_evidence = camera_evidence_v1(&far, true, 70).expect("far camera evidence");
        assert!(reference_evidence.camera_valid);
        assert_eq!(
            reference_evidence.base_fov_microradians,
            reference_evidence.target_base_fov_microradians
        );
        assert_eq!(
            reference_evidence.base_fov_microradians,
            reference_evidence.fov_microradians
        );
        assert_eq!(
            reference_evidence.frustum_ground_width_mm,
            far_evidence.frustum_ground_width_mm
        );
        assert_eq!(
            reference_evidence.frustum_ground_depth_mm,
            far_evidence.frustum_ground_depth_mm
        );
        assert_eq!(reference_evidence.camera_token, far_evidence.camera_token);
        assert_eq!(reference_evidence, far_evidence);
    }

    #[test]
    fn absent_opt_in_leaves_ordinary_focus_and_fixation_unchanged() {
        let mut camera = Camera::new(16.0 / 9.0, CameraMode::ThirdPerson);
        camera.force_focus_pos(Vec3::new(7.0, 8.0, 9.0));
        camera.set_fov(0.8);
        camera.set_fixate(0.4);
        let before_focus = camera.get_focus_pos();
        let before_fov = camera.fov_state_v1();
        let before_fixation = camera.fixation_state_v1();

        apply_post_maintenance_camera_v1(&mut camera, false, 70);
        assert_eq!(camera.get_focus_pos(), before_focus);
        assert_eq!(camera.fov_state_v1(), before_fov);
        assert_eq!(camera.fixation_state_v1(), before_fixation);
    }

    #[test]
    fn overhead_camera_cannot_satisfy_horizon_authority() {
        let mut camera = Camera::new(16.0 / 9.0, CameraMode::ThirdPerson);
        configure_camera_v1(&mut camera, Vec3::new(0.5, 0.5, 1.0));
        apply_post_maintenance_camera_v1(&mut camera, true, 70);
        camera.set_orientation_instant(Vec3::new(0.0, crate::scene::camera::OVERSEER_PITCH, 0.0));
        let overhead = camera_evidence_v1(&camera, true, 70).expect("bounded overhead evidence");
        assert!(!overhead.camera_valid);
        assert_ne!(overhead.pitch_microradians, CAMERA_PITCH_MICRORADIANS_V1);
    }
}
