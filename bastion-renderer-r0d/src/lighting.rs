//! Canonical presentation-only lighting and exposure policy.

use std::sync::Arc;

use crate::{
    domain_hash_v1,
    environment::{EnvironmentProjectionV1, WeatherKindV1},
};

pub const LIGHTING_POLICY_SCHEMA_V1: u16 = 1;
pub const MAX_LIGHTING_POLICY_BYTES_V1: usize = 256;
const MAGIC: &[u8; 8] = b"BSTLGT01";
const DIGEST_BYTES: usize = 32;
const PAYLOAD_BYTES: usize = 141;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LightingModeV1 {
    Outdoor = 1,
    Underwater = 2,
    Underground = 3,
}

impl LightingModeV1 {
    fn decode(tag: u8) -> Result<Self, LightingErrorV1> {
        match tag {
            1 => Ok(Self::Outdoor),
            2 => Ok(Self::Underwater),
            3 => Ok(Self::Underground),
            _ => Err(LightingErrorV1::UnknownMode(tag)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightingPolicyInputV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub environment_projection_digest: [u8; 32],
    pub material_table_digest: [u8; 32],
    pub camera_token_digest: [u8; 32],
    pub time_of_day_millis: u64,
    pub weather: WeatherKindV1,
    pub cloud_milli: u16,
    pub rain_milli: u16,
    pub mode: LightingModeV1,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LightingPolicyV1 {
    input: LightingPolicyInputV1,
    sun_milli: u16,
    moon_milli: u16,
    weather_attenuation_milli: u16,
    exposure_scale_milli: u16,
    ambient_scale_milli: u16,
    policy_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LightingErrorV1 {
    UnsealedOrPartial,
    InvalidGeneration,
    InvalidIdentity,
    TimeOutOfRange(u64),
    ScalarOutOfRange(&'static str, u16),
    WeatherValueMismatch,
    UnknownWeather(u8),
    UnknownMode(u8),
    InvalidMagic,
    UnsupportedVersion(u16),
    NonzeroReserved,
    MalformedBoolean(u8),
    Truncated,
    TrailingBytes,
    DigestMismatch,
    HashFailure,
    EncodedSizeOutOfRange(usize),
    StaleOrEqualGeneration { current: u64, offered: u64 },
    RollbackGenerationMismatch,
}

#[derive(Clone, Debug, Default)]
pub struct LightingPolicyPublisherV1 {
    current: Option<Arc<LightingPolicyV1>>,
    previous: Option<Arc<LightingPolicyV1>>,
}

impl LightingPolicyV1 {
    pub fn from_environment(
        environment: &EnvironmentProjectionV1,
        camera_token_digest: [u8; 32],
        mode: LightingModeV1,
    ) -> Result<Self, LightingErrorV1> {
        Self::new(LightingPolicyInputV1 {
            presentation_generation: environment.presentation_generation(),
            simulation_tick: environment.simulation_tick(),
            environment_projection_digest: environment.projection_digest(),
            material_table_digest: environment.material_table_digest(),
            camera_token_digest,
            time_of_day_millis: environment.time_of_day_millis(),
            weather: environment.weather(),
            cloud_milli: environment.cloud_milli(),
            rain_milli: environment.rain_milli(),
            mode,
            complete: true,
        })
    }

    pub fn new(input: LightingPolicyInputV1) -> Result<Self, LightingErrorV1> {
        validate_input(&input)?;
        let (sun_milli, moon_milli) = celestial_levels(input.time_of_day_millis);
        let weather_attenuation_milli = match input.weather {
            WeatherKindV1::Clear => 1_000,
            WeatherKindV1::Cloudy => 900,
            WeatherKindV1::Rain => 820,
            WeatherKindV1::Storm => 700,
        };
        let mode_scale = match input.mode {
            LightingModeV1::Outdoor => 1_000_u32,
            LightingModeV1::Underwater => 720,
            LightingModeV1::Underground => 560,
        };
        let exposure_scale_milli =
            ((u32::from(weather_attenuation_milli) * mode_scale) / 1_000) as u16;
        let ambient_base = 400_u32 + (u32::from(sun_milli) * 600 / 1_000);
        let ambient_scale_milli =
            ((ambient_base * u32::from(weather_attenuation_milli) * mode_scale) / 1_000_000) as u16;
        let mut policy = Self {
            input,
            sun_milli,
            moon_milli,
            weather_attenuation_milli,
            exposure_scale_milli,
            ambient_scale_milli,
            policy_digest: [0; 32],
        };
        let payload = policy.encode_payload();
        policy.policy_digest = hash_payload(&payload)?;
        Ok(policy)
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.input.presentation_generation }

    #[must_use]
    pub const fn simulation_tick(&self) -> u64 { self.input.simulation_tick }

    #[must_use]
    pub const fn environment_projection_digest(&self) -> [u8; 32] {
        self.input.environment_projection_digest
    }

    #[must_use]
    pub const fn material_table_digest(&self) -> [u8; 32] { self.input.material_table_digest }

    #[must_use]
    pub const fn camera_token_digest(&self) -> [u8; 32] { self.input.camera_token_digest }

    #[must_use]
    pub const fn time_of_day_millis(&self) -> u64 { self.input.time_of_day_millis }

    #[must_use]
    pub const fn weather(&self) -> WeatherKindV1 { self.input.weather }

    #[must_use]
    pub const fn mode(&self) -> LightingModeV1 { self.input.mode }

    #[must_use]
    pub const fn sun_milli(&self) -> u16 { self.sun_milli }

    #[must_use]
    pub const fn moon_milli(&self) -> u16 { self.moon_milli }

    #[must_use]
    pub const fn weather_attenuation_milli(&self) -> u16 { self.weather_attenuation_milli }

    #[must_use]
    pub const fn exposure_scale_milli(&self) -> u16 { self.exposure_scale_milli }

    #[must_use]
    pub const fn ambient_scale_milli(&self) -> u16 { self.ambient_scale_milli }

    #[must_use]
    pub const fn policy_digest(&self) -> [u8; 32] { self.policy_digest }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LightingErrorV1> {
        validate_input(&self.input)?;
        let payload = self.encode_payload();
        if hash_payload(&payload)? != self.policy_digest {
            return Err(LightingErrorV1::DigestMismatch);
        }
        let mut bytes = payload;
        bytes.extend_from_slice(&self.policy_digest);
        Ok(bytes)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, LightingErrorV1> {
        if bytes.len() > MAX_LIGHTING_POLICY_BYTES_V1 {
            return Err(LightingErrorV1::EncodedSizeOutOfRange(bytes.len()));
        }
        if bytes.len() != PAYLOAD_BYTES + DIGEST_BYTES {
            return Err(if bytes.len() < PAYLOAD_BYTES + DIGEST_BYTES {
                LightingErrorV1::Truncated
            } else {
                LightingErrorV1::TrailingBytes
            });
        }
        let (payload, digest) = bytes.split_at(PAYLOAD_BYTES);
        if hash_payload(payload)?.as_slice() != digest {
            return Err(LightingErrorV1::DigestMismatch);
        }
        let mut reader = Reader::new(payload);
        if reader.take(8)? != MAGIC {
            return Err(LightingErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != LIGHTING_POLICY_SCHEMA_V1 {
            return Err(LightingErrorV1::UnsupportedVersion(version));
        }
        if reader.u16()? != 0 {
            return Err(LightingErrorV1::NonzeroReserved);
        }
        let input = LightingPolicyInputV1 {
            presentation_generation: reader.u64()?,
            simulation_tick: reader.u64()?,
            environment_projection_digest: reader.digest()?,
            material_table_digest: reader.digest()?,
            camera_token_digest: reader.digest()?,
            time_of_day_millis: reader.u64()?,
            weather: decode_weather(reader.u8()?)?,
            cloud_milli: reader.u16()?,
            rain_milli: reader.u16()?,
            mode: LightingModeV1::decode(reader.u8()?)?,
            complete: match reader.u8()? {
                0 => false,
                1 => true,
                value => return Err(LightingErrorV1::MalformedBoolean(value)),
            },
        };
        if reader.u16()? != 0 || !reader.is_empty() {
            return Err(LightingErrorV1::NonzeroReserved);
        }
        let decoded = Self::new(input)?;
        if decoded.policy_digest.as_slice() != digest {
            return Err(LightingErrorV1::DigestMismatch);
        }
        Ok(decoded)
    }

    fn encode_payload(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PAYLOAD_BYTES);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&LIGHTING_POLICY_SCHEMA_V1.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&self.input.presentation_generation.to_le_bytes());
        bytes.extend_from_slice(&self.input.simulation_tick.to_le_bytes());
        bytes.extend_from_slice(&self.input.environment_projection_digest);
        bytes.extend_from_slice(&self.input.material_table_digest);
        bytes.extend_from_slice(&self.input.camera_token_digest);
        bytes.extend_from_slice(&self.input.time_of_day_millis.to_le_bytes());
        bytes.push(self.input.weather as u8);
        bytes.extend_from_slice(&self.input.cloud_milli.to_le_bytes());
        bytes.extend_from_slice(&self.input.rain_milli.to_le_bytes());
        bytes.push(self.input.mode as u8);
        bytes.push(u8::from(self.input.complete));
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        debug_assert_eq!(bytes.len(), PAYLOAD_BYTES);
        bytes
    }
}

impl LightingPolicyPublisherV1 {
    #[must_use]
    pub fn current(&self) -> Option<Arc<LightingPolicyV1>> { self.current.clone() }

    pub fn publish(
        &mut self,
        policy: LightingPolicyV1,
    ) -> Result<Arc<LightingPolicyV1>, LightingErrorV1> {
        if let Some(current) = &self.current {
            if policy.presentation_generation() <= current.presentation_generation() {
                return Err(LightingErrorV1::StaleOrEqualGeneration {
                    current: current.presentation_generation(),
                    offered: policy.presentation_generation(),
                });
            }
        }
        let next = Arc::new(policy);
        self.previous = self.current.replace(Arc::clone(&next));
        Ok(next)
    }

    pub fn rollback(&mut self, generation: u64) -> Result<Arc<LightingPolicyV1>, LightingErrorV1> {
        let previous = self
            .previous
            .as_ref()
            .filter(|policy| policy.presentation_generation() == generation)
            .cloned()
            .ok_or(LightingErrorV1::RollbackGenerationMismatch)?;
        self.current = Some(Arc::clone(&previous));
        self.previous = None;
        Ok(previous)
    }
}

fn validate_input(input: &LightingPolicyInputV1) -> Result<(), LightingErrorV1> {
    if !input.complete {
        return Err(LightingErrorV1::UnsealedOrPartial);
    }
    if input.presentation_generation == 0 {
        return Err(LightingErrorV1::InvalidGeneration);
    }
    if input.environment_projection_digest == [0; 32]
        || input.material_table_digest == [0; 32]
        || input.camera_token_digest == [0; 32]
    {
        return Err(LightingErrorV1::InvalidIdentity);
    }
    if input.time_of_day_millis > crate::environment::MAX_ENVIRONMENT_TIME_MILLIS_V1 {
        return Err(LightingErrorV1::TimeOutOfRange(input.time_of_day_millis));
    }
    for (name, value) in [("cloud", input.cloud_milli), ("rain", input.rain_milli)] {
        if value > 1_000 {
            return Err(LightingErrorV1::ScalarOutOfRange(name, value));
        }
    }
    match input.weather {
        WeatherKindV1::Clear if input.rain_milli != 0 => {
            return Err(LightingErrorV1::WeatherValueMismatch);
        },
        WeatherKindV1::Rain | WeatherKindV1::Storm if input.rain_milli == 0 => {
            return Err(LightingErrorV1::WeatherValueMismatch);
        },
        _ => {},
    }
    Ok(())
}

fn celestial_levels(time_millis: u64) -> (u16, u16) {
    let local = time_millis % 86_400_000;
    let hour_milli = local.saturating_mul(24_000) / 86_400_000;
    let sun = if (6_000..=18_000).contains(&hour_milli) {
        let distance = hour_milli.abs_diff(12_000);
        1_000_u64.saturating_sub(distance.saturating_mul(600) / 6_000)
    } else {
        0
    };
    (sun as u16, (1_000_u64.saturating_sub(sun)) as u16)
}

fn decode_weather(tag: u8) -> Result<WeatherKindV1, LightingErrorV1> {
    match tag {
        1 => Ok(WeatherKindV1::Clear),
        2 => Ok(WeatherKindV1::Cloudy),
        3 => Ok(WeatherKindV1::Rain),
        4 => Ok(WeatherKindV1::Storm),
        _ => Err(LightingErrorV1::UnknownWeather(tag)),
    }
}

fn hash_payload(bytes: &[u8]) -> Result<[u8; 32], LightingErrorV1> {
    domain_hash_v1("bastion/r1f/lighting-policy", 1, 0, bytes)
        .map_err(|_| LightingErrorV1::HashFailure)
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], LightingErrorV1> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or(LightingErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(LightingErrorV1::Truncated)?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, LightingErrorV1> {
        Ok(*self.take(1)?.first().ok_or(LightingErrorV1::Truncated)?)
    }

    fn u16(&mut self) -> Result<u16, LightingErrorV1> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| LightingErrorV1::Truncated)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, LightingErrorV1> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| LightingErrorV1::Truncated)?,
        ))
    }

    fn digest(&mut self) -> Result<[u8; 32], LightingErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| LightingErrorV1::Truncated)
    }

    fn is_empty(&self) -> bool { self.cursor == self.bytes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{
        EnvironmentAvailabilityV1, EnvironmentProjectionInputV1, GameplayVisibilityV1, SeasonV1,
    };

    fn environment(generation: u64, weather: WeatherKindV1, hour: u64) -> EnvironmentProjectionV1 {
        let raining = matches!(weather, WeatherKindV1::Rain | WeatherKindV1::Storm);
        EnvironmentProjectionV1::new(EnvironmentProjectionInputV1 {
            presentation_generation: generation,
            simulation_tick: 300 + generation,
            presentation_frame_digest: [1; 32],
            material_table_digest: [2; 32],
            renderer_environment_identity: [3; 32],
            time_of_day_millis: hour * 3_600_000,
            season: SeasonV1::Summer,
            weather,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: if weather == WeatherKindV1::Clear {
                0
            } else {
                700
            },
            rain_milli: if raining { 600 } else { 0 },
            wind_mm_s: [0, 0],
            precipitation_milli: if raining { 600 } else { 0 },
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

    #[test]
    fn canonical_round_trip_binds_identity() {
        let policy = LightingPolicyV1::from_environment(
            &environment(1, WeatherKindV1::Clear, 12),
            [4; 32],
            LightingModeV1::Outdoor,
        )
        .unwrap();
        let bytes = policy.canonical_bytes().unwrap();
        assert_eq!(LightingPolicyV1::decode_exact(&bytes).unwrap(), policy);
        assert_ne!(policy.policy_digest(), [0; 32]);
    }

    #[test]
    fn real_time_and_weather_boundaries_are_ordered() {
        let dawn = LightingPolicyV1::from_environment(
            &environment(1, WeatherKindV1::Clear, 6),
            [4; 32],
            LightingModeV1::Outdoor,
        )
        .unwrap();
        let noon = LightingPolicyV1::from_environment(
            &environment(2, WeatherKindV1::Clear, 12),
            [4; 32],
            LightingModeV1::Outdoor,
        )
        .unwrap();
        let rain = LightingPolicyV1::from_environment(
            &environment(3, WeatherKindV1::Rain, 12),
            [4; 32],
            LightingModeV1::Outdoor,
        )
        .unwrap();
        assert!(noon.sun_milli() > dawn.sun_milli());
        assert!(rain.exposure_scale_milli() < noon.exposure_scale_milli());
    }

    #[test]
    fn medium_modes_are_bounded_presentation_only() {
        let env = environment(1, WeatherKindV1::Clear, 12);
        let outdoor =
            LightingPolicyV1::from_environment(&env, [4; 32], LightingModeV1::Outdoor).unwrap();
        let water =
            LightingPolicyV1::from_environment(&env, [4; 32], LightingModeV1::Underwater).unwrap();
        let underground =
            LightingPolicyV1::from_environment(&env, [4; 32], LightingModeV1::Underground).unwrap();
        assert!(outdoor.exposure_scale_milli() > water.exposure_scale_milli());
        assert!(water.exposure_scale_milli() > underground.exposure_scale_milli());
    }

    #[test]
    fn malformed_stale_and_rollback_fail_closed() {
        let mut policy = LightingPolicyV1::from_environment(
            &environment(1, WeatherKindV1::Clear, 12),
            [4; 32],
            LightingModeV1::Outdoor,
        )
        .unwrap();
        let mut bytes = policy.canonical_bytes().unwrap();
        bytes.push(0);
        assert_eq!(
            LightingPolicyV1::decode_exact(&bytes),
            Err(LightingErrorV1::TrailingBytes)
        );
        policy.policy_digest[0] ^= 1;
        assert_eq!(
            policy.canonical_bytes(),
            Err(LightingErrorV1::DigestMismatch)
        );

        let mut publisher = LightingPolicyPublisherV1::default();
        publisher
            .publish(
                LightingPolicyV1::from_environment(
                    &environment(1, WeatherKindV1::Clear, 12),
                    [4; 32],
                    LightingModeV1::Outdoor,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(matches!(
            publisher.publish(
                LightingPolicyV1::from_environment(
                    &environment(1, WeatherKindV1::Clear, 12),
                    [4; 32],
                    LightingModeV1::Outdoor,
                )
                .unwrap()
            ),
            Err(LightingErrorV1::StaleOrEqualGeneration { .. })
        ));
        publisher
            .publish(
                LightingPolicyV1::from_environment(
                    &environment(2, WeatherKindV1::Rain, 12),
                    [5; 32],
                    LightingModeV1::Underwater,
                )
                .unwrap(),
            )
            .unwrap();
        let restored = publisher.rollback(1).unwrap();
        assert_eq!(restored.presentation_generation(), 1);
        assert_eq!(restored.mode(), LightingModeV1::Outdoor);
        assert_eq!(
            publisher.rollback(99),
            Err(LightingErrorV1::RollbackGenerationMismatch)
        );
    }

    #[test]
    fn unavailable_producers_do_not_enter_policy() {
        let bytes = LightingPolicyV1::from_environment(
            &environment(1, WeatherKindV1::Clear, 12),
            [4; 32],
            LightingModeV1::Outdoor,
        )
        .unwrap()
        .canonical_bytes()
        .unwrap();
        assert!(!bytes.windows(6).any(|window| window == b"divine"));
        assert!(!bytes.windows(9).any(|window| window == b"corrupted"));
    }
}
