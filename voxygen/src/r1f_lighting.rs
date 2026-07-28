//! Production adapter for coherent renderer lighting/exposure policy.

use std::sync::{Arc, Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    environment::{EnvironmentProjectionV1, WeatherKindV1},
    lighting::{LightingModeV1, LightingPolicyPublisherV1, LightingPolicyV1},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LightingMediumV1 {
    Air,
    Water,
    Solid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LightingProductionInputV1 {
    pub medium: LightingMediumV1,
    pub underground: bool,
    pub camera_mode_tag: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LightingProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub environment_projection_digest: [u8; 32],
    pub material_table_digest: [u8; 32],
    pub camera_token_digest: [u8; 32],
    pub policy_digest: [u8; 32],
    pub weather_tag: u8,
    pub mode: LightingModeV1,
    pub time_of_day_millis: u64,
    pub sun_milli: u16,
    pub moon_milli: u16,
    pub weather_attenuation_milli: u16,
    pub exposure_scale_milli: u16,
    pub ambient_scale_milli: u16,
    pub local_light_budget_is_legacy_diagnostic: bool,
    pub divine_corrupted_overgod_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LightingUniformV1 {
    pub policy: [f32; 4],
}

impl LightingUniformV1 {
    pub(crate) const fn legacy_disabled() -> Self {
        Self {
            policy: [0.0, 1.0, 1.0, 0.0],
        }
    }

    pub(crate) const fn fail_closed() -> Self {
        Self {
            policy: [1.0, 0.05, 0.0, 3.0],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LightingAdapterErrorV1 {
    InvalidCameraMode,
    GenerationConflict,
    Hash,
    Core(String),
    StatePoisoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LightingFixtureV1 {
    Clear,
    Rain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LightingFixtureDeclarationV1 {
    Disabled,
    Requested(LightingFixtureV1),
    Invalid,
}

#[derive(Debug, Default)]
struct LightingAdapterStateV1 {
    publisher: LightingPolicyPublisherV1,
    latest: Option<LightingProductionEvidenceV1>,
}

static STATE: OnceLock<Mutex<LightingAdapterStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<LightingAdapterStateV1> {
    STATE.get_or_init(|| Mutex::new(LightingAdapterStateV1::default()))
}

pub(crate) fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = LightingAdapterStateV1::default();
    }
}

#[must_use]
pub(crate) fn latest_evidence() -> Option<LightingProductionEvidenceV1> {
    state().lock().ok().and_then(|state| state.latest)
}

fn fixture_declaration_from(flat_arena: bool, value: Option<&str>) -> LightingFixtureDeclarationV1 {
    if !flat_arena {
        return LightingFixtureDeclarationV1::Disabled;
    }
    match value {
        Some("clear") => LightingFixtureDeclarationV1::Requested(LightingFixtureV1::Clear),
        Some("rain") => LightingFixtureDeclarationV1::Requested(LightingFixtureV1::Rain),
        Some(_) => LightingFixtureDeclarationV1::Invalid,
        None => LightingFixtureDeclarationV1::Disabled,
    }
}

fn fixture_declaration() -> LightingFixtureDeclarationV1 {
    fixture_declaration_from(
        std::env::var_os("BASTION_FLAT_ARENA").is_some(),
        std::env::var("BASTION_R1F_LIGHTING_FIXTURE")
            .ok()
            .as_deref(),
    )
}

#[must_use]
pub(crate) fn certification_fixture_ready_for_capture() -> bool {
    match fixture_declaration() {
        LightingFixtureDeclarationV1::Disabled => true,
        LightingFixtureDeclarationV1::Invalid => false,
        LightingFixtureDeclarationV1::Requested(requested) => {
            let expected = match requested {
                LightingFixtureV1::Clear => WeatherKindV1::Clear as u8,
                LightingFixtureV1::Rain => WeatherKindV1::Rain as u8,
            };
            latest_evidence().is_some_and(|evidence| {
                evidence.weather_tag == expected
                    && evidence.exposure_scale_milli > 0
                    && !evidence.divine_corrupted_overgod_available
            })
        },
    }
}

pub(crate) fn update(
    environment: &EnvironmentProjectionV1,
    input: LightingProductionInputV1,
) -> Result<(Arc<LightingPolicyV1>, LightingUniformV1), LightingAdapterErrorV1> {
    if input.camera_mode_tag > 3 {
        return Err(LightingAdapterErrorV1::InvalidCameraMode);
    }
    let mode = if input.medium == LightingMediumV1::Water {
        LightingModeV1::Underwater
    } else if input.underground {
        LightingModeV1::Underground
    } else {
        LightingModeV1::Outdoor
    };
    let mut camera_bytes = Vec::with_capacity(51);
    camera_bytes.extend_from_slice(&environment.presentation_generation().to_le_bytes());
    camera_bytes.extend_from_slice(&environment.simulation_tick().to_le_bytes());
    camera_bytes.extend_from_slice(&environment.presentation_frame_digest());
    camera_bytes.push(input.camera_mode_tag);
    camera_bytes.push(match input.medium {
        LightingMediumV1::Air => 0,
        LightingMediumV1::Water => 1,
        LightingMediumV1::Solid => 2,
    });
    camera_bytes.push(u8::from(input.underground));
    let camera_token_digest =
        domain_hash_v1("bastion/r1f/lighting-camera-token", 1, 0, &camera_bytes)
            .map_err(|_| LightingAdapterErrorV1::Hash)?;
    let policy = LightingPolicyV1::from_environment(environment, camera_token_digest, mode)
        .map_err(|error| LightingAdapterErrorV1::Core(format!("{error:?}")))?;

    let mut state = state()
        .lock()
        .map_err(|_| LightingAdapterErrorV1::StatePoisoned)?;
    if let Some(current) = state.publisher.current() {
        if current.presentation_generation() == policy.presentation_generation() {
            if current.as_ref() == &policy {
                return Ok((Arc::clone(&current), to_uniform(&current)));
            }
            return Err(LightingAdapterErrorV1::GenerationConflict);
        }
    }
    let published = state
        .publisher
        .publish(policy)
        .map_err(|error| LightingAdapterErrorV1::Core(format!("{error:?}")))?;
    state.latest = Some(LightingProductionEvidenceV1 {
        presentation_generation: published.presentation_generation(),
        simulation_tick: published.simulation_tick(),
        environment_projection_digest: published.environment_projection_digest(),
        material_table_digest: published.material_table_digest(),
        camera_token_digest: published.camera_token_digest(),
        policy_digest: published.policy_digest(),
        weather_tag: published.weather() as u8,
        mode: published.mode(),
        time_of_day_millis: published.time_of_day_millis(),
        sun_milli: published.sun_milli(),
        moon_milli: published.moon_milli(),
        weather_attenuation_milli: published.weather_attenuation_milli(),
        exposure_scale_milli: published.exposure_scale_milli(),
        ambient_scale_milli: published.ambient_scale_milli(),
        local_light_budget_is_legacy_diagnostic: true,
        divine_corrupted_overgod_available: false,
    });
    Ok((Arc::clone(&published), to_uniform(&published)))
}

fn to_uniform(policy: &LightingPolicyV1) -> LightingUniformV1 {
    LightingUniformV1 {
        policy: [
            1.0,
            f32::from(policy.exposure_scale_milli()) / 1_000.0,
            f32::from(policy.ambient_scale_milli()) / 1_000.0,
            policy.mode() as u8 as f32,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::environment::{
        EnvironmentAvailabilityV1, EnvironmentProjectionInputV1, GameplayVisibilityV1, SeasonV1,
    };

    static TEST_SERIAL: OnceLock<Mutex<()>> = OnceLock::new();
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_SERIAL
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lighting adapter test lock poisoned")
    }

    fn environment(generation: u64, weather: WeatherKindV1) -> EnvironmentProjectionV1 {
        let raining = weather == WeatherKindV1::Rain;
        EnvironmentProjectionV1::new(EnvironmentProjectionInputV1 {
            presentation_generation: generation,
            simulation_tick: generation + 300,
            presentation_frame_digest: [1; 32],
            material_table_digest: [2; 32],
            renderer_environment_identity: [3; 32],
            time_of_day_millis: 12 * 3_600_000,
            season: SeasonV1::Summer,
            weather,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: if raining { 700 } else { 0 },
            rain_milli: if raining { 500 } else { 0 },
            wind_mm_s: [0, 0],
            precipitation_milli: if raining { 500 } else { 0 },
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

    fn input() -> LightingProductionInputV1 {
        LightingProductionInputV1 {
            medium: LightingMediumV1::Air,
            underground: false,
            camera_mode_tag: 1,
        }
    }

    #[test]
    fn production_modes_and_weather_are_coherent() {
        let _guard = guard();
        reset();
        let (_, clear) = update(&environment(1, WeatherKindV1::Clear), input()).unwrap();
        reset();
        let (_, rain) = update(&environment(1, WeatherKindV1::Rain), input()).unwrap();
        assert!(clear.policy[1] > rain.policy[1]);
        reset();
        let mut underwater = input();
        underwater.medium = LightingMediumV1::Water;
        let (_, underwater) = update(&environment(1, WeatherKindV1::Clear), underwater).unwrap();
        assert_eq!(underwater.policy[3], 2.0);
    }

    #[test]
    fn same_generation_is_idempotent_and_conflict_rejects() {
        let _guard = guard();
        reset();
        let environment = environment(2, WeatherKindV1::Clear);
        let (first, _) = update(&environment, input()).unwrap();
        let (second, _) = update(&environment, input()).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let mut changed = input();
        changed.underground = true;
        assert_eq!(
            update(&environment, changed),
            Err(LightingAdapterErrorV1::GenerationConflict)
        );
    }

    #[test]
    fn malformed_and_stale_reject() {
        let _guard = guard();
        reset();
        let mut malformed = input();
        malformed.camera_mode_tag = 9;
        assert_eq!(
            update(&environment(3, WeatherKindV1::Clear), malformed),
            Err(LightingAdapterErrorV1::InvalidCameraMode)
        );
        update(&environment(4, WeatherKindV1::Clear), input()).unwrap();
        assert!(matches!(
            update(&environment(3, WeatherKindV1::Clear), input()),
            Err(LightingAdapterErrorV1::Core(_))
        ));
    }

    #[test]
    fn fixture_gate_requires_exact_coherent_weather() {
        let _guard = guard();
        reset();
        assert_eq!(
            fixture_declaration_from(false, Some("rain")),
            LightingFixtureDeclarationV1::Disabled
        );
        assert_eq!(
            fixture_declaration_from(true, Some("unknown")),
            LightingFixtureDeclarationV1::Invalid
        );
        assert_eq!(
            fixture_declaration_from(true, Some("clear")),
            LightingFixtureDeclarationV1::Requested(LightingFixtureV1::Clear)
        );
    }
}
