//! Production adapter from the accepted environment projection to the
//! renderer-owned deterministic weather presentation.

use std::sync::{Arc, Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    environment::WeatherKindV1,
    weather::{
        WeatherEffectInputV1, WeatherEffectKindV1, WeatherPresentationInputV1,
        WeatherPresentationPublisherV1, WeatherPresentationV1,
    },
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeatherProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub environment_projection_digest: [u8; 32],
    pub environment_source_identity: [u8; 32],
    pub weather_tag: u8,
    pub rain_milli: u16,
    pub precipitation_milli: u16,
    pub wind_mm_s: [i32; 2],
    pub phase_milli: u64,
    pub effect_record_count: u16,
    pub effect_instance_count: u32,
    pub presentation_digest: [u8; 32],
    pub legacy_rollback: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeatherAdapterErrorV1 {
    Hash,
    Core(String),
    StatePoisoned,
    EffectCountOverflow,
}

#[derive(Debug, Default)]
struct WeatherAdapterStateV1 {
    publisher: WeatherPresentationPublisherV1,
    latest: Option<WeatherProductionEvidenceV1>,
}

static STATE: OnceLock<Mutex<WeatherAdapterStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<WeatherAdapterStateV1> {
    STATE.get_or_init(|| Mutex::new(WeatherAdapterStateV1::default()))
}

pub(crate) fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = WeatherAdapterStateV1::default();
    }
}

#[must_use]
pub(crate) fn latest_presentation() -> Option<Arc<WeatherPresentationV1>> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.publisher.current())
}

#[must_use]
pub(crate) fn latest_evidence() -> Option<WeatherProductionEvidenceV1> {
    state().lock().ok().and_then(|state| state.latest)
}

pub(crate) fn refresh_from_environment()
-> Result<Option<Arc<WeatherPresentationV1>>, WeatherAdapterErrorV1> {
    let environment = crate::r1f_environment::latest_projection();
    refresh_from_projection(environment.as_deref())
}

fn refresh_from_projection(
    environment: Option<&bastion_renderer_r0d::environment::EnvironmentProjectionV1>,
) -> Result<Option<Arc<WeatherPresentationV1>>, WeatherAdapterErrorV1> {
    let mut state = state()
        .lock()
        .map_err(|_| WeatherAdapterErrorV1::StatePoisoned)?;
    let Some(environment) = environment else {
        *state = WeatherAdapterStateV1::default();
        return Ok(None);
    };
    if let Some(current) = state.publisher.current() {
        if current.presentation_generation() == environment.presentation_generation()
            && current.environment_projection_digest() == environment.projection_digest()
        {
            return Ok(Some(current));
        }
    }
    let value = build_from_environment(&environment)?;
    let published = state
        .publisher
        .publish(value)
        .map_err(|error| WeatherAdapterErrorV1::Core(format!("{error:?}")))?;
    let effect_record_count = u16::try_from(published.effect_records().len())
        .map_err(|_| WeatherAdapterErrorV1::EffectCountOverflow)?;
    state.latest = Some(WeatherProductionEvidenceV1 {
        presentation_generation: published.presentation_generation(),
        simulation_tick: published.simulation_tick(),
        environment_projection_digest: published.environment_projection_digest(),
        environment_source_identity: published.environment_source_identity(),
        weather_tag: published.weather() as u8,
        rain_milli: published.rain_milli(),
        precipitation_milli: published.precipitation_milli(),
        wind_mm_s: published.wind_mm_s(),
        phase_milli: published.phase_milli(),
        effect_record_count,
        effect_instance_count: published.total_effect_count(),
        presentation_digest: published.presentation_digest(),
        legacy_rollback: false,
    });
    Ok(Some(published))
}

fn build_from_environment(
    environment: &bastion_renderer_r0d::environment::EnvironmentProjectionV1,
) -> Result<WeatherPresentationV1, WeatherAdapterErrorV1> {
    let mut run_payload = Vec::with_capacity(64);
    run_payload.extend_from_slice(&environment.presentation_frame_digest());
    run_payload.extend_from_slice(&environment.renderer_environment_identity());
    let run_identity = domain_hash_v1("bastion/r1f/weather-run", 1, 0, &run_payload)
        .map_err(|_| WeatherAdapterErrorV1::Hash)?;
    let raining = matches!(
        environment.weather(),
        WeatherKindV1::Rain | WeatherKindV1::Storm
    ) && environment.rain_milli() > 0
        && environment.precipitation_milli() > 0;
    let effect_inputs = if raining {
        let cell_identity = domain_hash_v1(
            "bastion/r1f/weather-player-cell",
            1,
            0,
            &environment.renderer_environment_identity(),
        )
        .map_err(|_| WeatherAdapterErrorV1::Hash)?;
        let effect_identity = domain_hash_v1(
            "bastion/r1f/weather-rain-occlusion-cloud-shader",
            1,
            0,
            b"voxygen:rain-occlusion+clouds-frag-v1",
        )
        .map_err(|_| WeatherAdapterErrorV1::Hash)?;
        vec![WeatherEffectInputV1 {
            cell_identity,
            effect_identity,
            kind: WeatherEffectKindV1::Rain,
        }]
    } else {
        Vec::new()
    };
    WeatherPresentationV1::new(WeatherPresentationInputV1 {
        run_identity,
        presentation_generation: environment.presentation_generation(),
        simulation_tick: environment.simulation_tick(),
        presentation_frame_digest: environment.presentation_frame_digest(),
        environment_projection_digest: environment.projection_digest(),
        environment_source_identity: environment.renderer_environment_identity(),
        weather: environment.weather(),
        availability: environment.availability(),
        cloud_milli: environment.cloud_milli(),
        rain_milli: environment.rain_milli(),
        wind_mm_s: environment.wind_mm_s(),
        precipitation_milli: environment.precipitation_milli(),
        effect_inputs,
        complete: true,
    })
    .map_err(|error| WeatherAdapterErrorV1::Core(format!("{error:?}")))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CertificationFixtureKindV1 {
    Clear,
    Rain,
    Storm,
}

#[must_use]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CertificationFixtureDeclarationV1 {
    Disabled,
    Requested(CertificationFixtureKindV1),
    Invalid,
}

#[must_use]
pub(crate) fn certification_fixture_declaration() -> CertificationFixtureDeclarationV1 {
    if std::env::var_os("BASTION_FLAT_ARENA").is_none() {
        return CertificationFixtureDeclarationV1::Disabled;
    }
    match std::env::var("BASTION_R1F_WEATHER_FIXTURE").as_deref() {
        Ok("clear") => {
            CertificationFixtureDeclarationV1::Requested(CertificationFixtureKindV1::Clear)
        },
        Ok("rain") => {
            CertificationFixtureDeclarationV1::Requested(CertificationFixtureKindV1::Rain)
        },
        Ok("storm") => {
            CertificationFixtureDeclarationV1::Requested(CertificationFixtureKindV1::Storm)
        },
        // lw-port fix (found live, first GPU leg): an ABSENT declaration is
        // DISABLED, not invalid — the ported arm treated every flat-arena
        // run without a weather fixture as a fault and killed its own
        // embedded server two minutes in. Absent ≠ invalid.
        Err(std::env::VarError::NotPresent) => CertificationFixtureDeclarationV1::Disabled,
        Ok(_) | Err(_) => CertificationFixtureDeclarationV1::Invalid,
    }
}

fn fixture_matches_presentation(
    kind: CertificationFixtureKindV1,
    presentation: &WeatherPresentationV1,
) -> bool {
    match kind {
        CertificationFixtureKindV1::Clear => {
            presentation.weather() == WeatherKindV1::Clear
                && presentation.rain_milli() == 0
                && presentation.effect_records().is_empty()
                && presentation.total_effect_count() == 0
        },
        CertificationFixtureKindV1::Rain => {
            presentation.weather() == WeatherKindV1::Rain
                && presentation.rain_milli() > 0
                && presentation.is_raining()
                && !presentation.effect_records().is_empty()
                && presentation.total_effect_count() > 0
        },
        CertificationFixtureKindV1::Storm => {
            presentation.weather() == WeatherKindV1::Storm
                && presentation.rain_milli() > 0
                && presentation.is_raining()
                && !presentation.effect_records().is_empty()
                && presentation.total_effect_count() > 0
        },
    }
}

#[must_use]
pub(crate) fn certification_fixture_ready_for_capture() -> bool {
    match certification_fixture_declaration() {
        CertificationFixtureDeclarationV1::Disabled => true,
        CertificationFixtureDeclarationV1::Invalid => false,
        CertificationFixtureDeclarationV1::Requested(kind) => latest_presentation()
            .as_deref()
            .is_some_and(|presentation| fixture_matches_presentation(kind, presentation)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::environment::{
        EnvironmentAvailabilityV1, EnvironmentProjectionInputV1, EnvironmentProjectionV1,
        GameplayVisibilityV1, SeasonV1,
    };

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn projection(generation: u64, weather: WeatherKindV1) -> EnvironmentProjectionV1 {
        EnvironmentProjectionV1::new(EnvironmentProjectionInputV1 {
            presentation_generation: generation,
            simulation_tick: 300,
            presentation_frame_digest: digest(2),
            material_table_digest: digest(3),
            renderer_environment_identity: digest(4),
            time_of_day_millis: 1,
            season: SeasonV1::Summer,
            weather,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: if matches!(weather, WeatherKindV1::Rain | WeatherKindV1::Storm) {
                300
            } else {
                0
            },
            rain_milli: if matches!(weather, WeatherKindV1::Rain | WeatherKindV1::Storm) {
                300
            } else {
                0
            },
            wind_mm_s: if weather == WeatherKindV1::Storm {
                [15_000, 20_000]
            } else {
                [1_000, -1_000]
            },
            precipitation_milli: if matches!(weather, WeatherKindV1::Rain | WeatherKindV1::Storm) {
                300
            } else {
                0
            },
            temperature_milli: 10,
            wetness_milli: 0,
            snow_milli: 0,
            frost_milli: 0,
            visibility: GameplayVisibilityV1 {
                terrain_blocks: 64,
                entity_blocks: 32,
            },
            events: Vec::new(),
            complete: true,
        })
        .unwrap()
    }

    #[test]
    fn coherent_projection_maps_to_exact_generation_weather_and_wind() {
        let projection = projection(7, WeatherKindV1::Storm);
        let presentation = build_from_environment(&projection).unwrap();
        assert_eq!(presentation.presentation_generation(), 7);
        assert_eq!(presentation.simulation_tick(), 300);
        assert_eq!(presentation.wind_mm_s(), [15_000, 20_000]);
        assert!(presentation.is_raining());
        assert_eq!(presentation.effect_records().len(), 1);
    }

    #[test]
    fn legacy_fallback_is_explicit_when_projection_is_absent() {
        reset();
        assert_eq!(refresh_from_projection(None).unwrap(), None);
        assert_eq!(latest_evidence(), None);
    }

    #[test]
    fn clear_projection_has_no_precipitation_records() {
        let presentation = build_from_environment(&projection(9, WeatherKindV1::Clear)).unwrap();
        assert!(!presentation.is_raining());
        assert!(presentation.effect_records().is_empty());
        assert_eq!(presentation.total_effect_count(), 0);
    }

    #[test]
    fn certification_capture_gate_requires_matching_clear_rain_or_storm_acknowledgement() {
        let clear = build_from_environment(&projection(10, WeatherKindV1::Clear)).unwrap();
        let rain = build_from_environment(&projection(11, WeatherKindV1::Rain)).unwrap();
        let storm = build_from_environment(&projection(12, WeatherKindV1::Storm)).unwrap();
        assert!(fixture_matches_presentation(
            CertificationFixtureKindV1::Clear,
            &clear
        ));
        assert!(fixture_matches_presentation(
            CertificationFixtureKindV1::Rain,
            &rain
        ));
        assert!(fixture_matches_presentation(
            CertificationFixtureKindV1::Storm,
            &storm
        ));
        assert!(!fixture_matches_presentation(
            CertificationFixtureKindV1::Rain,
            &clear
        ));
        assert!(!fixture_matches_presentation(
            CertificationFixtureKindV1::Storm,
            &rain
        ));
        assert!(!fixture_matches_presentation(
            CertificationFixtureKindV1::Clear,
            &rain
        ));
    }
}
