//! Certification-only authoritative weather fixture for the flat arena.
//!
//! This is deliberately driven from the singleplayer server loop after the
//! weather job exists. Production behavior is unchanged unless the caller has
//! already selected the flat-arena certification path.

use common::weather::{CELL_SIZE, Weather, WeatherGrid, WeatherKind};
use specs::WorldExt;
use vek::Vec2;

use crate::{Server, weather::WeatherJob};

const FIXTURE_RADIUS_WORLD: f32 = 1_500.0;
const FIXTURE_LIFETIME_SECONDS: f32 = 3_600.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BastionWeatherFixtureKindV1 {
    Clear,
    Rain,
    Storm,
}

impl BastionWeatherFixtureKindV1 {
    fn weather(self) -> Weather {
        match self {
            Self::Clear => Weather {
                cloud: 0.0,
                rain: 0.0,
                wind: Vec2::zero(),
            },
            Self::Rain => Weather {
                cloud: 0.1,
                rain: 0.15,
                wind: Vec2::new(1.0, -1.0),
            },
            Self::Storm => Weather {
                cloud: 0.3,
                rain: 0.3,
                wind: Vec2::new(15.0, 20.0),
            },
        }
    }

    #[must_use]
    pub fn acknowledges(self, weather: Weather) -> bool {
        match self {
            Self::Clear => weather.get_kind() == WeatherKind::Clear && weather.rain == 0.0,
            Self::Rain => weather.get_kind() == WeatherKind::Rain && weather.rain > 0.0,
            Self::Storm => weather.get_kind() == WeatherKind::Storm && weather.rain > 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BastionWeatherFixtureStepV1 {
    WaitingForWeatherJob,
    Queued {
        zone_generation: u64,
    },
    QueueGenerationOverflow,
    WaitingForAuthoritativeSnapshot {
        requested_zone_generation: u64,
        completed_zone_generation: u64,
        observed_kind: WeatherKind,
        observed_rain: f32,
    },
    Acknowledged {
        observed_kind: WeatherKind,
        observed_rain: f32,
    },
}

fn authoritative_snapshot_acknowledges(
    kind: BastionWeatherFixtureKindV1,
    requested_zone_generation: u64,
    completed_zone_generation: u64,
    observed: Weather,
) -> bool {
    completed_zone_generation >= requested_zone_generation && kind.acknowledges(observed)
}

impl Server {
    /// Queue or observe the flat-arena weather fixture at the authoritative
    /// server boundary. A queue request is never reported as acknowledgement:
    /// acknowledgement requires the resulting `WeatherGrid` snapshot.
    pub fn bastion_weather_fixture_step_v1(
        &mut self,
        kind: BastionWeatherFixtureKindV1,
        requested_zone_generation: Option<u64>,
    ) -> BastionWeatherFixtureStepV1 {
        let center_world = crate::bastion_flat_arena::world_center_wpos(&self.world).as_::<f32>();
        if requested_zone_generation.is_none() {
            let mut weather_job = self.state.ecs_mut().write_resource::<Option<WeatherJob>>();
            let Some(weather_job) = weather_job.as_mut() else {
                return BastionWeatherFixtureStepV1::WaitingForWeatherJob;
            };
            let Some(zone_generation) = weather_job.queue_zone(
                kind.weather(),
                center_world / CELL_SIZE as f32,
                FIXTURE_RADIUS_WORLD / CELL_SIZE as f32,
                FIXTURE_LIFETIME_SECONDS,
            ) else {
                return BastionWeatherFixtureStepV1::QueueGenerationOverflow;
            };
            return BastionWeatherFixtureStepV1::Queued { zone_generation };
        }

        let Some(requested_zone_generation) = requested_zone_generation else {
            return BastionWeatherFixtureStepV1::QueueGenerationOverflow;
        };
        let completed_zone_generation = self
            .state
            .ecs()
            .read_resource::<Option<WeatherJob>>()
            .as_ref()
            .map_or(0, WeatherJob::completed_zone_generation);
        let observed = self
            .state
            .ecs()
            .read_resource::<WeatherGrid>()
            .get_interpolated(center_world);
        if authoritative_snapshot_acknowledges(
            kind,
            requested_zone_generation,
            completed_zone_generation,
            observed,
        ) {
            BastionWeatherFixtureStepV1::Acknowledged {
                observed_kind: observed.get_kind(),
                observed_rain: observed.rain,
            }
        } else {
            BastionWeatherFixtureStepV1::WaitingForAuthoritativeSnapshot {
                requested_zone_generation,
                completed_zone_generation,
                observed_kind: observed.get_kind(),
                observed_rain: observed.rain,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_rain_and_storm_require_matching_authoritative_weather() {
        let clear = BastionWeatherFixtureKindV1::Clear;
        let rain = BastionWeatherFixtureKindV1::Rain;
        let storm = BastionWeatherFixtureKindV1::Storm;
        assert!(clear.acknowledges(clear.weather()));
        assert!(rain.acknowledges(rain.weather()));
        assert!(storm.acknowledges(storm.weather()));
        assert!(!rain.acknowledges(clear.weather()));
        assert!(!storm.acknowledges(rain.weather()));
        assert!(!clear.acknowledges(rain.weather()));
    }

    #[test]
    fn matching_weather_before_requested_zone_completion_is_not_acknowledgement() {
        let clear = BastionWeatherFixtureKindV1::Clear;
        assert!(!authoritative_snapshot_acknowledges(
            clear,
            1,
            0,
            clear.weather()
        ));
        assert!(authoritative_snapshot_acknowledges(
            clear,
            1,
            1,
            clear.weather()
        ));
    }
}
