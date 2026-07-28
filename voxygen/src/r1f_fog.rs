//! Production adapter for the renderer-owned fog/visibility policy.
//!
//! The adapter consumes the already-published coherent environment
//! projection. It never expands gameplay visibility and leaves the legacy
//! renderer unchanged when no coherent projection is available.

use std::sync::{Arc, Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    environment::EnvironmentProjectionV1,
    fog::{
        FogModeV1, FogPolicyInputV1, FogPolicyPublisherV1, FogPolicyV1, FogQualityV1,
        ShelterStateV1,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProductionMediumV1 {
    Air,
    Water,
    Solid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FogProductionInputV1 {
    pub medium: ProductionMediumV1,
    pub underground: bool,
    pub camera_mode_tag: u8,
    pub low_quality: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FogProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub environment_projection_digest: [u8; 32],
    pub camera_token_digest: [u8; 32],
    pub policy_digest: [u8; 32],
    pub mode: FogModeV1,
    pub quality: FogQualityV1,
    pub near_blocks: u16,
    pub far_blocks: u16,
    pub color_milli: [u16; 3],
    pub gameplay_terrain_visibility_blocks: u16,
    pub gameplay_entity_visibility_blocks: u16,
    pub shelter_authority_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FogUniformV1 {
    pub mode: [u32; 4],
    pub distances: [f32; 4],
    pub color: [f32; 4],
}

impl FogUniformV1 {
    pub(crate) const fn legacy_disabled() -> Self {
        Self {
            mode: [0; 4],
            distances: [0.0; 4],
            color: [0.0; 4],
        }
    }

    pub(crate) const fn fail_closed() -> Self {
        Self {
            mode: [3, 1, 1, 0],
            distances: [0.0, 1.0, 1.0, 1.0],
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FogAdapterErrorV1 {
    InvalidCameraMode,
    InvalidIdentity,
    GenerationConflict,
    Core(String),
    Hash,
    StatePoisoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FogCertificationFixtureV1 {
    LegacyRollback,
    Outdoor,
    Underwater,
    Underground,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FogCertificationDeclarationV1 {
    Disabled,
    Requested(FogCertificationFixtureV1),
    Invalid,
}

#[derive(Debug, Default)]
struct FogAdapterStateV1 {
    publisher: FogPolicyPublisherV1,
    latest: Option<FogProductionEvidenceV1>,
}

static STATE: OnceLock<Mutex<FogAdapterStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<FogAdapterStateV1> {
    STATE.get_or_init(|| Mutex::new(FogAdapterStateV1::default()))
}

pub(crate) fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = FogAdapterStateV1::default();
    }
}

#[must_use]
pub(crate) fn latest_evidence() -> Option<FogProductionEvidenceV1> {
    state().lock().ok().and_then(|state| state.latest)
}

fn certification_declaration_from(
    flat_arena: bool,
    value: Option<&str>,
) -> FogCertificationDeclarationV1 {
    if !flat_arena {
        return FogCertificationDeclarationV1::Disabled;
    }
    match value {
        Some("legacy") => {
            FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::LegacyRollback)
        },
        Some("outdoor") => {
            FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::Outdoor)
        },
        Some("underwater") => {
            FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::Underwater)
        },
        Some("underground") => {
            FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::Underground)
        },
        Some(_) => FogCertificationDeclarationV1::Invalid,
        None => FogCertificationDeclarationV1::Disabled,
    }
}

#[must_use]
pub(crate) fn certification_fixture_declaration() -> FogCertificationDeclarationV1 {
    certification_declaration_from(
        std::env::var_os("BASTION_FLAT_ARENA").is_some(),
        std::env::var("BASTION_R1F_FOG_FIXTURE").ok().as_deref(),
    )
}

#[must_use]
pub(crate) fn certification_legacy_rollback_requested() -> bool {
    certification_fixture_declaration()
        == FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::LegacyRollback)
}

#[must_use]
pub(crate) fn certification_fixture_ready_for_capture() -> bool {
    match certification_fixture_declaration() {
        FogCertificationDeclarationV1::Disabled => true,
        FogCertificationDeclarationV1::Invalid => false,
        FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::LegacyRollback) => {
            latest_evidence().is_none()
        },
        FogCertificationDeclarationV1::Requested(requested) => {
            let expected = match requested {
                FogCertificationFixtureV1::Outdoor => FogModeV1::Outdoor,
                FogCertificationFixtureV1::Underwater => FogModeV1::Underwater,
                FogCertificationFixtureV1::Underground => FogModeV1::Underground,
                FogCertificationFixtureV1::LegacyRollback => return false,
            };
            latest_evidence().is_some_and(|evidence| evidence.mode == expected)
        },
    }
}

pub(crate) fn update(
    environment: &EnvironmentProjectionV1,
    input: FogProductionInputV1,
) -> Result<(Arc<FogPolicyV1>, FogUniformV1), FogAdapterErrorV1> {
    if input.camera_mode_tag > 3 {
        return Err(FogAdapterErrorV1::InvalidCameraMode);
    }
    let mode = if input.medium == ProductionMediumV1::Water {
        FogModeV1::Underwater
    } else if input.underground {
        FogModeV1::Underground
    } else {
        FogModeV1::Outdoor
    };
    let quality = if input.low_quality {
        FogQualityV1::Low
    } else {
        FogQualityV1::Full
    };
    let mut camera_bytes = Vec::with_capacity(48);
    camera_bytes.extend_from_slice(&environment.presentation_generation().to_le_bytes());
    camera_bytes.extend_from_slice(&environment.simulation_tick().to_le_bytes());
    camera_bytes.extend_from_slice(&environment.presentation_frame_digest());
    camera_bytes.push(input.camera_mode_tag);
    camera_bytes.push(match input.medium {
        ProductionMediumV1::Air => 0,
        ProductionMediumV1::Water => 1,
        ProductionMediumV1::Solid => 2,
    });
    camera_bytes.push(u8::from(input.underground));
    camera_bytes.push(u8::from(input.low_quality));
    let camera_token_digest = domain_hash_v1("bastion/r1f/fog-camera-token", 1, 0, &camera_bytes)
        .map_err(|_| FogAdapterErrorV1::Hash)?;
    if camera_token_digest == [0; 32] {
        return Err(FogAdapterErrorV1::InvalidIdentity);
    }
    let policy = FogPolicyV1::new(FogPolicyInputV1 {
        presentation_generation: environment.presentation_generation(),
        simulation_tick: environment.simulation_tick(),
        environment_projection_digest: environment.projection_digest(),
        camera_token_digest,
        visibility: environment.visibility(),
        // Veloren's production view distance is expressed in horizontal
        // terrain chunks; the renderer policy operates in world blocks.
        visibility_unit_blocks: 32,
        mode,
        quality,
        shelter: ShelterStateV1::Unavailable,
        complete: true,
    })
    .map_err(|error| FogAdapterErrorV1::Core(format!("{error:?}")))?;

    let mut state = state()
        .lock()
        .map_err(|_| FogAdapterErrorV1::StatePoisoned)?;
    if let Some(current) = state.publisher.current() {
        if current.presentation_generation() == policy.presentation_generation() {
            if current.as_ref() == &policy {
                return Ok((Arc::clone(&current), to_uniform(&current)));
            }
            return Err(FogAdapterErrorV1::GenerationConflict);
        }
    }
    let published = state
        .publisher
        .publish(policy)
        .map_err(|error| FogAdapterErrorV1::Core(format!("{error:?}")))?;
    state.latest = Some(FogProductionEvidenceV1 {
        presentation_generation: published.presentation_generation(),
        simulation_tick: published.simulation_tick(),
        environment_projection_digest: published.environment_projection_digest(),
        camera_token_digest: published.camera_token_digest(),
        policy_digest: published.policy_digest(),
        mode: published.mode(),
        quality: published.quality(),
        near_blocks: published.near_blocks(),
        far_blocks: published.far_blocks(),
        color_milli: published.color_milli(),
        gameplay_terrain_visibility_blocks: published.visibility().terrain_blocks,
        gameplay_entity_visibility_blocks: published.visibility().entity_blocks,
        shelter_authority_available: false,
    });
    Ok((Arc::clone(&published), to_uniform(&published)))
}

fn to_uniform(policy: &FogPolicyV1) -> FogUniformV1 {
    let mode = match policy.mode() {
        FogModeV1::Outdoor => 1,
        FogModeV1::Underwater => 2,
        FogModeV1::Underground => 3,
    };
    let quality = match policy.quality() {
        FogQualityV1::Low => 1,
        FogQualityV1::Full => 2,
    };
    let color = policy.color_milli();
    FogUniformV1 {
        mode: [mode, quality, 0, 0],
        distances: [
            f32::from(policy.near_blocks()),
            f32::from(policy.far_blocks()),
            f32::from(policy.visibility().terrain_blocks),
            f32::from(policy.visibility().entity_blocks),
        ],
        color: [
            f32::from(color[0]) / 1_000.0,
            f32::from(color[1]) / 1_000.0,
            f32::from(color[2]) / 1_000.0,
            1.0,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::environment::{
        EnvironmentAvailabilityV1, EnvironmentProjectionInputV1, GameplayVisibilityV1, SeasonV1,
        WeatherKindV1,
    };

    static TEST_SERIAL: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_SERIAL
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("fog adapter test lock poisoned")
    }

    fn environment(generation: u64) -> EnvironmentProjectionV1 {
        EnvironmentProjectionV1::new(EnvironmentProjectionInputV1 {
            presentation_generation: generation,
            simulation_tick: 300 + generation,
            presentation_frame_digest: [1; 32],
            material_table_digest: [2; 32],
            renderer_environment_identity: [3; 32],
            time_of_day_millis: 12_000,
            season: SeasonV1::Summer,
            weather: WeatherKindV1::Clear,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: 0,
            rain_milli: 0,
            wind_mm_s: [0, 0],
            precipitation_milli: 0,
            temperature_milli: 20,
            wetness_milli: 0,
            snow_milli: 0,
            frost_milli: 0,
            visibility: GameplayVisibilityV1 {
                terrain_blocks: 512,
                entity_blocks: 256,
            },
            events: Vec::new(),
            complete: true,
        })
        .unwrap()
    }

    fn input() -> FogProductionInputV1 {
        FogProductionInputV1 {
            medium: ProductionMediumV1::Air,
            underground: false,
            camera_mode_tag: 1,
            low_quality: false,
        }
    }

    #[test]
    fn production_modes_consume_real_medium_and_terrain_classification() {
        let _guard = test_guard();
        reset();
        let (_, outdoor) = update(&environment(1), input()).unwrap();
        assert_eq!(outdoor.mode[0], 1);
        reset();
        let mut underwater = input();
        underwater.medium = ProductionMediumV1::Water;
        let (_, underwater) = update(&environment(1), underwater).unwrap();
        assert_eq!(underwater.mode[0], 2);
        reset();
        let mut underground = input();
        underground.underground = true;
        let (_, underground) = update(&environment(1), underground).unwrap();
        assert_eq!(underground.mode[0], 3);
    }

    #[test]
    fn exact_generation_is_idempotent_but_conflicts_fail_closed() {
        let _guard = test_guard();
        reset();
        let environment = environment(2);
        let (first, _) = update(&environment, input()).unwrap();
        let (second, _) = update(&environment, input()).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let mut changed = input();
        changed.underground = true;
        assert_eq!(
            update(&environment, changed),
            Err(FogAdapterErrorV1::GenerationConflict)
        );
    }

    #[test]
    fn low_tier_preserves_visibility_and_disables_no_authority() {
        let _guard = test_guard();
        reset();
        let mut low = input();
        low.low_quality = true;
        let (policy, uniform) = update(&environment(3), low).unwrap();
        assert_eq!(policy.visibility().terrain_blocks, 512);
        assert_eq!(policy.visibility().entity_blocks, 256);
        assert_eq!(uniform.mode[1], 1);
        assert!(!latest_evidence().unwrap().shelter_authority_available);
    }

    #[test]
    fn malformed_camera_mode_and_stale_generation_reject() {
        let _guard = test_guard();
        reset();
        let mut malformed = input();
        malformed.camera_mode_tag = 9;
        assert_eq!(
            update(&environment(4), malformed),
            Err(FogAdapterErrorV1::InvalidCameraMode)
        );
        update(&environment(5), input()).unwrap();
        assert!(matches!(
            update(&environment(4), input()),
            Err(FogAdapterErrorV1::Core(_))
        ));
    }

    #[test]
    fn certification_declaration_and_capture_gate_fail_closed() {
        let _guard = test_guard();
        reset();
        assert_eq!(
            certification_declaration_from(false, Some("outdoor")),
            FogCertificationDeclarationV1::Disabled
        );
        assert_eq!(
            certification_declaration_from(true, Some("legacy")),
            FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::LegacyRollback)
        );
        assert_eq!(
            certification_declaration_from(true, Some("underwater")),
            FogCertificationDeclarationV1::Requested(FogCertificationFixtureV1::Underwater)
        );
        assert_eq!(
            certification_declaration_from(true, Some("unknown")),
            FogCertificationDeclarationV1::Invalid
        );
    }
}
