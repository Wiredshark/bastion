//! Production adapter for the renderer-owned environment projection.
//!
//! One coherent client-applied sample is staged with its presentation frame.
//! Publication occurs only after the exact generation's figure material table
//! exists, so the projection cannot claim a guessed or prior material
//! authority.

use std::sync::{Arc, Mutex, OnceLock};

use bastion_renderer_r0d::{
    domain_hash_v1,
    environment::{
        EnvironmentAvailabilityV1, EnvironmentProjectionInputV1, EnvironmentProjectionPublisherV1,
        EnvironmentProjectionV1, GameplayVisibilityV1, SeasonV1, WeatherKindV1,
    },
    material::MaterialTableV1,
    presentation::PresentationFrameV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionEnvironmentSampleV1 {
    pub simulation_tick: u64,
    pub renderer_environment_identity: [u8; 32],
    pub time_of_day_millis: u64,
    pub season: SeasonV1,
    pub weather: WeatherKindV1,
    pub availability: EnvironmentAvailabilityV1,
    pub cloud_milli: u16,
    pub rain_milli: u16,
    pub wind_mm_s: [i32; 2],
    pub precipitation_milli: u16,
    pub temperature_milli: i32,
    pub visibility: GameplayVisibilityV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvironmentProductionEvidenceV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub frame_digest: [u8; 32],
    pub material_table_digest: [u8; 32],
    pub environment_identity: [u8; 32],
    pub projection_digest: [u8; 32],
    pub availability_bits: u16,
    pub client_interpolation_is_diagnostic: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvironmentAdapterErrorV1 {
    NonFinite(&'static str),
    OutOfRange(&'static str),
    InvalidIdentity,
    FrameGenerationMismatch,
    FrameTickMismatch,
    MaterialGenerationMismatch,
    NoPendingSample,
    StalePendingSample,
    Hash,
    Core(String),
    StatePoisoned,
}

#[derive(Clone, Debug)]
struct PendingSampleV1 {
    generation: u64,
    frame_digest: [u8; 32],
    sample: ProductionEnvironmentSampleV1,
}

#[derive(Debug, Default)]
struct EnvironmentAdapterStateV1 {
    pending: Option<PendingSampleV1>,
    publisher: EnvironmentProjectionPublisherV1,
    latest: Option<EnvironmentProductionEvidenceV1>,
}

static STATE: OnceLock<Mutex<EnvironmentAdapterStateV1>> = OnceLock::new();

fn state() -> &'static Mutex<EnvironmentAdapterStateV1> {
    STATE.get_or_init(|| Mutex::new(EnvironmentAdapterStateV1::default()))
}

pub(crate) fn reset() {
    if let Ok(mut state) = state().lock() {
        *state = EnvironmentAdapterStateV1::default();
    }
}

#[must_use]
pub(crate) fn latest_projection() -> Option<Arc<EnvironmentProjectionV1>> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.publisher.current())
}

#[must_use]
pub(crate) fn latest_evidence() -> Option<EnvironmentProductionEvidenceV1> {
    state().lock().ok().and_then(|state| state.latest)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sample_from_production(
    simulation_tick: u64,
    time_of_day_seconds: f64,
    season_days_in_year: f64,
    weather_kind: common::weather::WeatherKind,
    cloud: f32,
    rain: f32,
    wind: [f32; 2],
    temperature: f32,
    terrain_visibility: u16,
    entity_visibility: u16,
) -> Result<ProductionEnvironmentSampleV1, EnvironmentAdapterErrorV1> {
    let time_of_day_millis = finite_scaled_u64(
        "time_of_day",
        time_of_day_seconds,
        1_000.0,
        bastion_renderer_r0d::environment::MAX_ENVIRONMENT_TIME_MILLIS_V1,
    )?;
    let days_milli = finite_scaled_u64(
        "season_days_in_year",
        season_days_in_year,
        1_000.0,
        1_000_000,
    )?;
    if days_milli == 0 {
        return Err(EnvironmentAdapterErrorV1::OutOfRange("season_days_in_year"));
    }
    let cloud_milli = finite_unit_milli("cloud", f64::from(cloud))?;
    let rain_milli = finite_unit_milli("rain", f64::from(rain))?;
    let wind_mm_s = [
        finite_scaled_i32(
            "wind_x",
            f64::from(wind[0]),
            1_000.0,
            bastion_renderer_r0d::environment::MAX_ENVIRONMENT_WIND_MM_S_V1,
        )?,
        finite_scaled_i32(
            "wind_y",
            f64::from(wind[1]),
            1_000.0,
            bastion_renderer_r0d::environment::MAX_ENVIRONMENT_WIND_MM_S_V1,
        )?,
    ];
    let temperature_milli =
        finite_scaled_i32("temperature", f64::from(temperature), 1_000.0, 1_000)?;
    if terrain_visibility == 0
        || entity_visibility == 0
        || entity_visibility > terrain_visibility
        || terrain_visibility
            > bastion_renderer_r0d::environment::MAX_ENVIRONMENT_VISIBILITY_BLOCKS_V1
    {
        return Err(EnvironmentAdapterErrorV1::OutOfRange("visibility"));
    }
    let season = match common::time::Season::at(time_of_day_seconds, season_days_in_year) {
        common::time::Season::Spring => SeasonV1::Spring,
        common::time::Season::Summer => SeasonV1::Summer,
        common::time::Season::Autumn => SeasonV1::Autumn,
        common::time::Season::Winter => SeasonV1::Winter,
    };
    let weather = match weather_kind {
        common::weather::WeatherKind::Clear => WeatherKindV1::Clear,
        common::weather::WeatherKind::Cloudy => WeatherKindV1::Cloudy,
        common::weather::WeatherKind::Rain => WeatherKindV1::Rain,
        common::weather::WeatherKind::Storm => WeatherKindV1::Storm,
    };
    let mut identity = Vec::with_capacity(64);
    identity.extend_from_slice(b"client-applied-weather-grid+terrain-meta+master-time-v1");
    identity.extend_from_slice(&simulation_tick.to_le_bytes());
    identity.extend_from_slice(&time_of_day_millis.to_le_bytes());
    identity.extend_from_slice(&days_milli.to_le_bytes());
    identity.push(season as u8);
    identity.push(weather as u8);
    identity.extend_from_slice(&cloud_milli.to_le_bytes());
    identity.extend_from_slice(&rain_milli.to_le_bytes());
    for value in wind_mm_s {
        identity.extend_from_slice(&value.to_le_bytes());
    }
    identity.extend_from_slice(&temperature_milli.to_le_bytes());
    identity.extend_from_slice(&terrain_visibility.to_le_bytes());
    identity.extend_from_slice(&entity_visibility.to_le_bytes());
    let renderer_environment_identity =
        domain_hash_v1("bastion/r1f/production-environment-source", 1, 0, &identity)
            .map_err(|_| EnvironmentAdapterErrorV1::Hash)?;
    Ok(ProductionEnvironmentSampleV1 {
        simulation_tick,
        renderer_environment_identity,
        time_of_day_millis,
        season,
        weather,
        availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
        cloud_milli,
        rain_milli,
        wind_mm_s,
        precipitation_milli: rain_milli,
        temperature_milli,
        visibility: GameplayVisibilityV1 {
            terrain_blocks: terrain_visibility,
            entity_blocks: entity_visibility,
        },
    })
}

pub(crate) fn stage(
    frame: &PresentationFrameV1,
    sample: ProductionEnvironmentSampleV1,
) -> Result<(), EnvironmentAdapterErrorV1> {
    if frame.generation().simulation_tick != sample.simulation_tick {
        return Err(EnvironmentAdapterErrorV1::FrameTickMismatch);
    }
    if sample.renderer_environment_identity == [0; 32] {
        return Err(EnvironmentAdapterErrorV1::InvalidIdentity);
    }
    let generation = frame.generation().client_applied_generation;
    let mut state = state()
        .lock()
        .map_err(|_| EnvironmentAdapterErrorV1::StatePoisoned)?;
    if let Some(current) = state.publisher.current() {
        if current.presentation_generation() == generation
            && current.presentation_frame_digest() == frame.frame_digest()
        {
            return Ok(());
        }
        if current.presentation_generation() >= generation {
            return Err(EnvironmentAdapterErrorV1::StalePendingSample);
        }
    }
    state.pending = Some(PendingSampleV1 {
        generation,
        frame_digest: frame.frame_digest(),
        sample,
    });
    Ok(())
}

pub(crate) fn bind_material_and_publish(
    frame: &PresentationFrameV1,
    material_table: &MaterialTableV1,
) -> Result<EnvironmentProductionEvidenceV1, EnvironmentAdapterErrorV1> {
    let generation = frame.generation().client_applied_generation;
    if material_table.generation() != generation {
        return Err(EnvironmentAdapterErrorV1::MaterialGenerationMismatch);
    }
    let mut state = state()
        .lock()
        .map_err(|_| EnvironmentAdapterErrorV1::StatePoisoned)?;
    let pending = state
        .pending
        .clone()
        .ok_or(EnvironmentAdapterErrorV1::NoPendingSample)?;
    if pending.generation != generation || pending.frame_digest != frame.frame_digest() {
        return Err(EnvironmentAdapterErrorV1::FrameGenerationMismatch);
    }
    let projection = EnvironmentProjectionV1::new(EnvironmentProjectionInputV1 {
        presentation_generation: generation,
        simulation_tick: pending.sample.simulation_tick,
        presentation_frame_digest: pending.frame_digest,
        material_table_digest: material_table.table_digest(),
        renderer_environment_identity: pending.sample.renderer_environment_identity,
        time_of_day_millis: pending.sample.time_of_day_millis,
        season: pending.sample.season,
        weather: pending.sample.weather,
        availability: pending.sample.availability,
        cloud_milli: pending.sample.cloud_milli,
        rain_milli: pending.sample.rain_milli,
        wind_mm_s: pending.sample.wind_mm_s,
        precipitation_milli: pending.sample.precipitation_milli,
        temperature_milli: pending.sample.temperature_milli,
        wetness_milli: 0,
        snow_milli: 0,
        frost_milli: 0,
        visibility: pending.sample.visibility,
        events: Vec::new(),
        complete: true,
    })
    .map_err(|error| EnvironmentAdapterErrorV1::Core(format!("{error:?}")))?;
    let published = state
        .publisher
        .publish(projection)
        .map_err(|error| EnvironmentAdapterErrorV1::Core(format!("{error:?}")))?;
    let evidence = EnvironmentProductionEvidenceV1 {
        presentation_generation: generation,
        simulation_tick: pending.sample.simulation_tick,
        frame_digest: frame.frame_digest(),
        material_table_digest: material_table.table_digest(),
        environment_identity: pending.sample.renderer_environment_identity,
        projection_digest: published.projection_digest(),
        availability_bits: published.availability().0,
        client_interpolation_is_diagnostic: true,
    };
    state.latest = Some(evidence);
    state.pending = None;
    Ok(evidence)
}

pub(crate) fn bind_material_if_staged(
    frame: &PresentationFrameV1,
    material_table: &MaterialTableV1,
) -> Result<Option<EnvironmentProductionEvidenceV1>, EnvironmentAdapterErrorV1> {
    let is_staged = state()
        .lock()
        .map_err(|_| EnvironmentAdapterErrorV1::StatePoisoned)?
        .pending
        .as_ref()
        .is_some_and(|pending| {
            pending.generation == frame.generation().client_applied_generation
                && pending.frame_digest == frame.frame_digest()
        });
    if !is_staged {
        return Ok(None);
    }
    bind_material_and_publish(frame, material_table).map(Some)
}

fn finite_unit_milli(field: &'static str, value: f64) -> Result<u16, EnvironmentAdapterErrorV1> {
    if !value.is_finite() {
        return Err(EnvironmentAdapterErrorV1::NonFinite(field));
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(EnvironmentAdapterErrorV1::OutOfRange(field));
    }
    Ok((value * 1_000.0).round() as u16)
}

fn finite_scaled_u64(
    field: &'static str,
    value: f64,
    scale: f64,
    max: u64,
) -> Result<u64, EnvironmentAdapterErrorV1> {
    if !value.is_finite() {
        return Err(EnvironmentAdapterErrorV1::NonFinite(field));
    }
    let scaled = value * scale;
    if scaled < 0.0 || scaled > max as f64 {
        return Err(EnvironmentAdapterErrorV1::OutOfRange(field));
    }
    Ok(scaled.round() as u64)
}

fn finite_scaled_i32(
    field: &'static str,
    value: f64,
    scale: f64,
    max_abs: i32,
) -> Result<i32, EnvironmentAdapterErrorV1> {
    if !value.is_finite() {
        return Err(EnvironmentAdapterErrorV1::NonFinite(field));
    }
    let scaled = value * scale;
    if scaled < -f64::from(max_abs) || scaled > f64::from(max_abs) {
        return Err(EnvironmentAdapterErrorV1::OutOfRange(field));
    }
    Ok(scaled.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bastion_renderer_r0d::{
        figure_asset::{
            CompiledFigurePackageV1, FigureAssetRoleV1, FigurePackageTargetV1, FigureSourceInputV1,
            MaterialBindingV1, MaterialKindV1,
        },
        presentation::{
            PresentationEnvironmentV1, PresentationFrameDraftV1, PresentationGenerationV1,
            PresentationVisualPolicyV1,
        },
    };

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn frame(generation: u64, tick: u64) -> PresentationFrameV1 {
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: generation,
                simulation_tick: tick,
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
                terrain_view_distance: 512,
                entity_view_distance: 256,
                figure_lod_distance: 128,
                sprite_distance: 128,
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

    fn package() -> CompiledFigurePackageV1 {
        CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            digest(20),
            digest(21),
            vec![MaterialBindingV1 {
                slot: 1,
                kind: MaterialKindV1::OpaqueVoxel,
                base_color_rgba: [1, 2, 3, 255],
                flags: 1,
            }],
            vec![FigureSourceInputV1 {
                logical_path: "fixture/body.vox".to_owned(),
                role: FigureAssetRoleV1::CoreBody,
                material_slot: 1,
                bytes: b"body".to_vec(),
                deterministic_fixture: false,
            }],
        )
        .unwrap()
    }

    fn sample(tick: u64) -> ProductionEnvironmentSampleV1 {
        sample_from_production(
            tick,
            100_000.0,
            160.0,
            common::weather::WeatherKind::Rain,
            0.7,
            0.4,
            [2.0, -1.0],
            0.25,
            512,
            256,
        )
        .unwrap()
    }

    #[test]
    fn real_source_sample_is_bounded_and_unavailable_fields_stay_explicit() {
        let value = sample(9);
        assert_eq!(value.cloud_milli, 700);
        assert_eq!(value.rain_milli, 400);
        assert_eq!(value.wind_mm_s, [2_000, -1_000]);
        assert_eq!(value.temperature_milli, 250);
        assert!(
            !value
                .availability
                .contains(EnvironmentAvailabilityV1::WETNESS)
        );
        assert!(
            !value
                .availability
                .contains(EnvironmentAvailabilityV1::SMOKE_REGIONS)
        );
    }

    #[test]
    fn nan_infinity_and_out_of_range_inputs_reject() {
        assert!(matches!(
            sample_from_production(
                1,
                f64::NAN,
                160.0,
                common::weather::WeatherKind::Clear,
                0.0,
                0.0,
                [0.0, 0.0],
                0.0,
                512,
                256,
            ),
            Err(EnvironmentAdapterErrorV1::NonFinite("time_of_day"))
        ));
        assert!(
            sample_from_production(
                1,
                1.0,
                160.0,
                common::weather::WeatherKind::Clear,
                f32::INFINITY,
                0.0,
                [0.0, 0.0],
                0.0,
                512,
                256,
            )
            .is_err()
        );
        assert!(
            sample_from_production(
                1,
                1.0,
                160.0,
                common::weather::WeatherKind::Clear,
                0.0,
                0.0,
                [0.0, 0.0],
                2.0,
                512,
                256,
            )
            .is_err()
        );
    }

    #[test]
    fn exact_frame_material_projection_chain_publishes_and_mismatch_rejects() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset();
        let frame = frame(5, 9);
        stage(&frame, sample(9)).unwrap();
        let materials = crate::r1f_materials::compile_figure_material_table(5, &package()).unwrap();
        let evidence = bind_material_and_publish(&frame, &materials).unwrap();
        assert_eq!(evidence.presentation_generation, 5);
        assert_eq!(evidence.frame_digest, frame.frame_digest());
        assert_eq!(evidence.material_table_digest, materials.table_digest());
        assert!(evidence.client_interpolation_is_diagnostic);
        let projection = latest_projection().unwrap();
        assert_eq!(projection.presentation_generation(), 5);
        assert_eq!(projection.material_table_digest(), materials.table_digest());

        reset();
        stage(&frame, sample(9)).unwrap();
        let wrong = crate::r1f_materials::compile_figure_material_table(6, &package()).unwrap();
        assert_eq!(
            bind_material_and_publish(&frame, &wrong),
            Err(EnvironmentAdapterErrorV1::MaterialGenerationMismatch)
        );
    }

    #[test]
    fn stale_generation_and_tick_mismatch_fail_closed() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset();
        let current = frame(2, 10);
        assert_eq!(
            stage(&current, sample(9)),
            Err(EnvironmentAdapterErrorV1::FrameTickMismatch)
        );
        stage(&current, sample(10)).unwrap();
        let materials = crate::r1f_materials::compile_figure_material_table(2, &package()).unwrap();
        bind_material_and_publish(&current, &materials).unwrap();
        assert_eq!(stage(&current, sample(10)), Ok(()));
        let stale = frame(1, 10);
        assert_eq!(
            stage(&stale, sample(10)),
            Err(EnvironmentAdapterErrorV1::StalePendingSample)
        );
    }

    #[test]
    fn diagnostic_interpolation_timing_cannot_change_projection() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        reset();
        let frame = frame(3, 11);
        let sample = sample(11);
        stage(&frame, sample.clone()).unwrap();
        let materials = crate::r1f_materials::compile_figure_material_table(3, &package()).unwrap();
        let first = bind_material_and_publish(&frame, &materials).unwrap();
        reset();
        // No elapsed wall-time or worker-order input exists in this API.
        stage(&frame, sample).unwrap();
        let second = bind_material_and_publish(&frame, &materials).unwrap();
        assert_eq!(first.projection_digest, second.projection_digest);
    }
}
