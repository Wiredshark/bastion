//! Production adapter for the renderer-owned world lens.
//!
//! The first supported slice is a weather lens sourced exclusively from the
//! accepted `EnvironmentProjectionV1`. Threat, underground, and site-activity
//! lenses remain unavailable until a coherent source is bound.

use std::sync::{Arc, Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    environment::{EnvironmentProjectionV1, WeatherKindV1},
    lens::{
        LensDatumV1, LensErrorV1, LensFrameInputV1, LensFrameV1, LensKindV1, LensModeV1,
        LensPublicationV1,
    },
    presentation::PresentationFrameV1,
};

pub const WEATHER_SOURCE_CAPABILITY_V1: &str = "ENVIRONMENT_PROJECTION_V1";
pub const THREAT_SOURCE_CAPABILITY_V1: &str = "UNAVAILABLE_NO_COHERENT_THREAT_SNAPSHOT";
pub const UNDERGROUND_SOURCE_CAPABILITY_V1: &str =
    "UNAVAILABLE_Z_LEVEL_ONLY_NO_SEMANTIC_UNDERGROUND_AUTHORITY";
pub const SITE_ACTIVITY_SOURCE_CAPABILITY_V1: &str =
    "UNAVAILABLE_NO_COHERENT_SITE_ACTIVITY_GENERATION";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LensProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub publication_sequence: u64,
    pub simulation_tick: u64,
    pub frame_digest: [u8; 32],
    pub presentation_frame_digest: [u8; 32],
    pub environment_projection_digest: [u8; 32],
    pub camera_token: [u8; 32],
    pub selection_digest: [u8; 32],
    pub mode: LensModeV1,
    pub datum_count: u16,
    pub weather_kind: u8,
    pub weather_cloud_milli: u16,
    pub weather_rain_milli: u16,
    pub weather_wind_speed_mm_s: u32,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LensAdapterErrorV1 {
    InvalidDeclaration,
    GenerationMismatch,
    InvalidCamera,
    InvalidSelection,
    SizeOverflow,
    Hash,
    Core(LensErrorV1),
    StatePoisoned,
}

#[derive(Debug, Default)]
struct LensAdapterStateV1 {
    publication: LensPublicationV1,
    next_sequence: u64,
    last_source_digest: Option<[u8; 32]>,
    latest: Option<LensProductionEvidenceV1>,
}

static STATE: OnceLock<Mutex<LensAdapterStateV1>> = OnceLock::new();

#[cfg(test)]
static TEST_LOCK_V1: Mutex<()> = Mutex::new(());

fn state() -> &'static Mutex<LensAdapterStateV1> {
    STATE.get_or_init(|| Mutex::new(LensAdapterStateV1::default()))
}

pub(crate) fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = LensAdapterStateV1::default();
    }
}

#[must_use]
pub(crate) fn latest_frame() -> Option<Arc<LensFrameV1>> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.publication.current())
}

#[must_use]
pub(crate) fn latest_evidence() -> Option<LensProductionEvidenceV1> {
    state().lock().ok().and_then(|state| state.latest.clone())
}

#[must_use]
pub(crate) fn certification_fixture_ready_for_capture() -> bool {
    let Ok(declaration) = std::env::var("BASTION_R1G_LENS") else {
        return true;
    };
    let expected = if declaration.eq_ignore_ascii_case("off") {
        LensModeV1::Off
    } else if declaration.eq_ignore_ascii_case("weather") {
        LensModeV1::Weather
    } else {
        return false;
    };
    latest_evidence().is_some_and(|evidence| {
        evidence.mode == expected
            && match expected {
                LensModeV1::Off => evidence.datum_count == 0 && evidence.label.is_empty(),
                LensModeV1::Weather => {
                    evidence.datum_count == 1
                        && !evidence.label.is_empty()
                        && evidence.weather_kind != 0
                },
            }
    })
}

pub(crate) fn requested_mode(ui_weather_enabled: bool) -> Result<LensModeV1, LensAdapterErrorV1> {
    match std::env::var("BASTION_R1G_LENS") {
        Ok(value) => parse_requested_mode(Some(&value), ui_weather_enabled),
        Err(std::env::VarError::NotPresent) => parse_requested_mode(None, ui_weather_enabled),
        Err(std::env::VarError::NotUnicode(_)) => Err(LensAdapterErrorV1::InvalidDeclaration),
    }
}

fn parse_requested_mode(
    declaration: Option<&str>,
    ui_weather_enabled: bool,
) -> Result<LensModeV1, LensAdapterErrorV1> {
    match declaration {
        Some(value) if value.eq_ignore_ascii_case("off") => Ok(LensModeV1::Off),
        Some(value) if value.eq_ignore_ascii_case("weather") => Ok(LensModeV1::Weather),
        Some(_) => Err(LensAdapterErrorV1::InvalidDeclaration),
        None => Ok(if ui_weather_enabled {
            LensModeV1::Weather
        } else {
            LensModeV1::Off
        }),
    }
}

pub(crate) fn update(
    frame: &PresentationFrameV1,
    environment: &EnvironmentProjectionV1,
    camera_position_mm: [i64; 3],
    mut selected_semantic_ids: Vec<[u8; 32]>,
    mode: LensModeV1,
) -> Result<Arc<LensFrameV1>, LensAdapterErrorV1> {
    let generation = frame.generation().client_applied_generation;
    if environment.presentation_generation() != generation
        || environment.presentation_frame_digest() != frame.frame_digest()
        || environment.simulation_tick() != frame.generation().simulation_tick
    {
        return Err(LensAdapterErrorV1::GenerationMismatch);
    }
    let camera_token = camera_token(camera_position_mm)?;
    selected_semantic_ids.sort_unstable();
    if selected_semantic_ids
        .windows(2)
        .any(|pair| pair[0] == pair[1])
        || selected_semantic_ids
            .iter()
            .any(|identity| *identity == [0; 32])
    {
        return Err(LensAdapterErrorV1::InvalidSelection);
    }
    let selection_digest = selection_digest(&selected_semantic_ids)?;
    let source_digest = source_digest(frame, environment, camera_token, selection_digest, mode)?;
    let mut state = state()
        .lock()
        .map_err(|_| LensAdapterErrorV1::StatePoisoned)?;
    if state.last_source_digest == Some(source_digest)
        && let Some(current) = state.publication.current()
    {
        return Ok(current);
    }
    state.next_sequence = state
        .next_sequence
        .checked_add(1)
        .ok_or(LensAdapterErrorV1::SizeOverflow)?;
    let publication_sequence = state.next_sequence;
    let (datums, label, wind_speed_mm_s) = match mode {
        LensModeV1::Off => (Vec::new(), String::new(), 0),
        LensModeV1::Weather => {
            let wind = environment.wind_mm_s();
            let wind_speed_mm_s = wind[0].unsigned_abs().max(wind[1].unsigned_abs());
            let label = weather_label(
                environment.weather(),
                environment.rain_milli(),
                wind_speed_mm_s,
            );
            let semantic_id = domain_hash_v1(
                "bastion/r1g/weather-lens-datum",
                1,
                0,
                &environment.renderer_environment_identity(),
            )
            .map_err(|_| LensAdapterErrorV1::Hash)?;
            (
                vec![LensDatumV1 {
                    semantic_id,
                    kind: LensKindV1::Weather,
                    authority_digest: environment.projection_digest(),
                    authority_generation: generation,
                    priority: 1_000,
                    values: [
                        environment.weather() as i32,
                        i32::from(environment.cloud_milli()),
                        i32::from(environment.rain_milli()),
                        i32::try_from(wind_speed_mm_s)
                            .map_err(|_| LensAdapterErrorV1::SizeOverflow)?,
                    ],
                    label: label.clone(),
                }],
                label,
                wind_speed_mm_s,
            )
        },
    };
    let lens = LensFrameV1::seal(LensFrameInputV1 {
        presentation_generation: generation,
        publication_sequence,
        simulation_tick: frame.generation().simulation_tick,
        presentation_frame_digest: frame.frame_digest(),
        camera_token,
        selection_digest,
        mode,
        max_visible_datums: 8,
        datums,
        complete: true,
    })
    .map_err(LensAdapterErrorV1::Core)?;

    let evidence = LensProductionEvidenceV1 {
        presentation_generation: generation,
        publication_sequence,
        simulation_tick: frame.generation().simulation_tick,
        frame_digest: lens.frame_digest(),
        presentation_frame_digest: frame.frame_digest(),
        environment_projection_digest: environment.projection_digest(),
        camera_token,
        selection_digest,
        mode,
        datum_count: u16::try_from(lens.datums().len())
            .map_err(|_| LensAdapterErrorV1::SizeOverflow)?,
        weather_kind: environment.weather() as u8,
        weather_cloud_milli: environment.cloud_milli(),
        weather_rain_milli: environment.rain_milli(),
        weather_wind_speed_mm_s: wind_speed_mm_s,
        label,
    };
    let published = state
        .publication
        .publish(lens)
        .map_err(LensAdapterErrorV1::Core)?;
    state.last_source_digest = Some(source_digest);
    state.latest = Some(evidence);
    Ok(published)
}

fn camera_token(camera_position_mm: [i64; 3]) -> Result<[u8; 32], LensAdapterErrorV1> {
    if camera_position_mm.iter().any(|value| {
        value
            .checked_abs()
            .is_none_or(|value| value > 9_000_000_000_000)
    }) {
        return Err(LensAdapterErrorV1::InvalidCamera);
    }
    let mut bytes = Vec::with_capacity(24);
    for value in camera_position_mm {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    domain_hash_v1("bastion/r1g/lens-camera", 1, 0, &bytes).map_err(|_| LensAdapterErrorV1::Hash)
}

fn selection_digest(selected: &[[u8; 32]]) -> Result<[u8; 32], LensAdapterErrorV1> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        &u64::try_from(selected.len())
            .map_err(|_| LensAdapterErrorV1::SizeOverflow)?
            .to_le_bytes(),
    );
    for identity in selected {
        bytes.extend_from_slice(identity);
    }
    domain_hash_v1("bastion/r1g/lens-selection", 1, 0, &bytes).map_err(|_| LensAdapterErrorV1::Hash)
}

fn source_digest(
    frame: &PresentationFrameV1,
    environment: &EnvironmentProjectionV1,
    camera_token: [u8; 32],
    selection_digest: [u8; 32],
    mode: LensModeV1,
) -> Result<[u8; 32], LensAdapterErrorV1> {
    let mut bytes = Vec::with_capacity(129);
    bytes.extend_from_slice(&frame.frame_digest());
    bytes.extend_from_slice(&environment.projection_digest());
    bytes.extend_from_slice(&camera_token);
    bytes.extend_from_slice(&selection_digest);
    bytes.push(mode as u8);
    domain_hash_v1("bastion/r1g/lens-source", 1, 0, &bytes).map_err(|_| LensAdapterErrorV1::Hash)
}

fn weather_label(kind: WeatherKindV1, rain_milli: u16, wind_speed_mm_s: u32) -> String {
    let kind = match kind {
        WeatherKindV1::Clear => "CLEAR",
        WeatherKindV1::Cloudy => "CLOUDY",
        WeatherKindV1::Rain => "RAIN",
        WeatherKindV1::Storm => "STORM",
    };
    format!(
        "{kind} {}% WIND {}.{:01}m/s",
        rain_milli / 10,
        wind_speed_mm_s / 1_000,
        (wind_speed_mm_s % 1_000) / 100
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        environment::{
            EnvironmentAvailabilityV1, EnvironmentProjectionInputV1, GameplayVisibilityV1,
        },
        presentation::{
            PresentationEnvironmentV1, PresentationFrameDraftV1, PresentationGenerationV1,
            PresentationVisualPolicyV1,
        },
    };

    fn digest(value: u8) -> [u8; 32] { [value; 32] }

    fn frame(generation: u64) -> PresentationFrameV1 {
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                simulation_tick: 300,
                client_applied_generation: generation,
                coherent_snapshot_root: digest(1),
            },
            entities: Vec::new(),
            groups: Vec::new(),
            events: Vec::new(),
            environment: PresentationEnvironmentV1 {
                terrain_root: digest(2),
                environment_digest: digest(3),
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 500,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: digest(4),
                terrain_view_distance: 100,
                entity_view_distance: 80,
                figure_lod_distance: 60,
                sprite_distance: 40,
                particles_enabled: true,
                weapon_trails_enabled: true,
                flashing_lights_enabled: true,
            },
            renderer_required_resources: vec![digest(5)],
            complete: true,
        }
        .seal()
        .unwrap()
    }

    fn environment(frame: &PresentationFrameV1, weather: WeatherKindV1) -> EnvironmentProjectionV1 {
        EnvironmentProjectionV1::new(EnvironmentProjectionInputV1 {
            presentation_generation: frame.generation().client_applied_generation,
            simulation_tick: frame.generation().simulation_tick,
            presentation_frame_digest: frame.frame_digest(),
            material_table_digest: digest(7),
            renderer_environment_identity: digest(8),
            time_of_day_millis: 10,
            season: bastion_renderer_r0d::environment::SeasonV1::Summer,
            weather,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: if weather == WeatherKindV1::Clear {
                0
            } else {
                500
            },
            rain_milli: if weather == WeatherKindV1::Rain {
                400
            } else {
                0
            },
            wind_mm_s: [2_500, -1_000],
            precipitation_milli: if weather == WeatherKindV1::Rain {
                400
            } else {
                0
            },
            temperature_milli: 200,
            wetness_milli: 0,
            snow_milli: 0,
            frost_milli: 0,
            visibility: GameplayVisibilityV1 {
                terrain_blocks: 100,
                entity_blocks: 80,
            },
            events: Vec::new(),
            complete: true,
        })
        .unwrap()
    }

    #[test]
    fn weather_projection_publishes_one_canonical_visible_datum() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        reset();
        let frame = frame(1);
        let environment = environment(&frame, WeatherKindV1::Rain);
        let lens = update(
            &frame,
            &environment,
            [1_000, 2_000, 3_000],
            vec![digest(21), digest(20)],
            LensModeV1::Weather,
        )
        .unwrap();
        assert_eq!(lens.datums().len(), 1);
        assert_eq!(lens.datums()[0].values, [3, 500, 400, 2_500]);
        assert_eq!(latest_evidence().unwrap().label, "RAIN 40% WIND 2.5m/s");
        assert_eq!(
            LensFrameV1::decode_exact(lens.canonical_bytes()).unwrap(),
            *lens
        );
    }

    #[test]
    fn input_permutation_is_equal_and_camera_selection_bind_identity() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        reset();
        let frame = frame(2);
        let environment = environment(&frame, WeatherKindV1::Clear);
        let a = update(
            &frame,
            &environment,
            [1, 2, 3],
            vec![digest(2), digest(1)],
            LensModeV1::Weather,
        )
        .unwrap();
        reset();
        let b = update(
            &frame,
            &environment,
            [1, 2, 3],
            vec![digest(1), digest(2)],
            LensModeV1::Weather,
        )
        .unwrap();
        assert_eq!(a.frame_digest(), b.frame_digest());
        let changed = update(
            &frame,
            &environment,
            [1, 2, 4],
            vec![digest(1), digest(2)],
            LensModeV1::Weather,
        )
        .unwrap();
        assert_ne!(a.frame_digest(), changed.frame_digest());
        assert_eq!(changed.publication_sequence(), 2);
    }

    #[test]
    fn off_is_empty_and_generation_mismatch_fails_closed() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        reset();
        let current = frame(3);
        let environment = environment(&current, WeatherKindV1::Clear);
        let off = update(&current, &environment, [0; 3], Vec::new(), LensModeV1::Off).unwrap();
        assert_eq!(off.mode(), LensModeV1::Off);
        assert!(off.datums().is_empty());
        let other = frame(4);
        assert_eq!(
            update(&other, &environment, [0; 3], Vec::new(), LensModeV1::Off),
            Err(LensAdapterErrorV1::GenerationMismatch)
        );
    }

    #[test]
    fn duplicate_selection_fails_closed() {
        let _guard = TEST_LOCK_V1.lock().unwrap();
        reset();
        let frame = frame(5);
        let environment = environment(&frame, WeatherKindV1::Clear);
        assert_eq!(
            update(
                &frame,
                &environment,
                [0; 3],
                vec![digest(1), digest(1)],
                LensModeV1::Weather
            ),
            Err(LensAdapterErrorV1::InvalidSelection)
        );
    }

    #[test]
    fn declaration_parser_has_explicit_off_weather_and_invalid_outcomes() {
        assert_eq!(parse_requested_mode(Some("off"), true), Ok(LensModeV1::Off));
        assert_eq!(
            parse_requested_mode(Some("WEATHER"), false),
            Ok(LensModeV1::Weather)
        );
        assert_eq!(
            parse_requested_mode(Some("threat"), true),
            Err(LensAdapterErrorV1::InvalidDeclaration)
        );
        assert_eq!(parse_requested_mode(None, false), Ok(LensModeV1::Off));
        assert_eq!(parse_requested_mode(None, true), Ok(LensModeV1::Weather));
    }
}
