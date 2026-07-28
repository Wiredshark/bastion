//! Renderer-owned, immutable environment projection.
//!
//! The projection records only bounded values sampled from one coherent
//! client-applied presentation generation. Unsupported environment authority
//! remains explicit in `availability`; diagnostics such as interpolation
//! timing are deliberately outside the canonical bytes.

use std::sync::Arc;

use crate::domain_hash_v1;

pub const ENVIRONMENT_PROJECTION_SCHEMA_V1: u16 = 1;
pub const MAX_ENVIRONMENT_PROJECTION_BYTES_V1: usize = 64 * 1024;
pub const MAX_ENVIRONMENT_EVENTS_V1: usize = 256;
pub const MAX_ENVIRONMENT_TIME_MILLIS_V1: u64 = 10_000_000_000_000_000;
pub const MAX_ENVIRONMENT_WIND_MM_S_V1: i32 = 200_000;
pub const MAX_ENVIRONMENT_VISIBILITY_BLOCKS_V1: u16 = 8_192;

const MAGIC: &[u8; 8] = b"BSTENV01";
const DIGEST_BYTES: usize = 32;
const EVENT_BYTES: usize = 76;
const KNOWN_AVAILABILITY_BITS: u16 = 0x0fff;

pub type EnvironmentDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SeasonV1 {
    Spring = 1,
    Summer = 2,
    Autumn = 3,
    Winter = 4,
}

impl SeasonV1 {
    fn decode(tag: u8) -> Result<Self, EnvironmentErrorV1> {
        match tag {
            1 => Ok(Self::Spring),
            2 => Ok(Self::Summer),
            3 => Ok(Self::Autumn),
            4 => Ok(Self::Winter),
            _ => Err(EnvironmentErrorV1::UnknownSeason(tag)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum WeatherKindV1 {
    Clear = 1,
    Cloudy = 2,
    Rain = 3,
    Storm = 4,
}

impl WeatherKindV1 {
    fn decode(tag: u8) -> Result<Self, EnvironmentErrorV1> {
        match tag {
            1 => Ok(Self::Clear),
            2 => Ok(Self::Cloudy),
            3 => Ok(Self::Rain),
            4 => Ok(Self::Storm),
            _ => Err(EnvironmentErrorV1::UnknownWeather(tag)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvironmentAvailabilityV1(pub u16);

impl EnvironmentAvailabilityV1 {
    pub const FIRE_REGIONS: u16 = 1 << 10;
    pub const FROST: u16 = 1 << 8;
    pub const LIGHTNING_EVENTS: u16 = 1 << 11;
    pub const PRECIPITATION: u16 = 1 << 4;
    pub const PRODUCTION_V1: Self = Self(
        Self::TIME_OF_DAY
            | Self::SEASON
            | Self::WEATHER
            | Self::WIND
            | Self::PRECIPITATION
            | Self::TEMPERATURE,
    );
    pub const SEASON: u16 = 1 << 1;
    pub const SMOKE_REGIONS: u16 = 1 << 9;
    pub const SNOW: u16 = 1 << 7;
    pub const TEMPERATURE: u16 = 1 << 5;
    pub const TIME_OF_DAY: u16 = 1 << 0;
    pub const WEATHER: u16 = 1 << 2;
    pub const WETNESS: u16 = 1 << 6;
    pub const WIND: u16 = 1 << 3;

    #[must_use]
    pub const fn contains(self, capability: u16) -> bool { self.0 & capability == capability }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct GameplayVisibilityV1 {
    pub terrain_blocks: u16,
    pub entity_blocks: u16,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EnvironmentEventV1 {
    pub kind_tag: u16,
    pub semantic_id: EnvironmentDigestV1,
    pub source_id: EnvironmentDigestV1,
    pub simulation_tick: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentProjectionInputV1 {
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub presentation_frame_digest: EnvironmentDigestV1,
    pub material_table_digest: EnvironmentDigestV1,
    pub renderer_environment_identity: EnvironmentDigestV1,
    pub time_of_day_millis: u64,
    pub season: SeasonV1,
    pub weather: WeatherKindV1,
    pub availability: EnvironmentAvailabilityV1,
    pub cloud_milli: u16,
    pub rain_milli: u16,
    pub wind_mm_s: [i32; 2],
    pub precipitation_milli: u16,
    pub temperature_milli: i32,
    pub wetness_milli: u16,
    pub snow_milli: u16,
    pub frost_milli: u16,
    pub visibility: GameplayVisibilityV1,
    pub events: Vec<EnvironmentEventV1>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentProjectionV1 {
    input: EnvironmentProjectionInputV1,
    projection_digest: EnvironmentDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvironmentErrorV1 {
    UnsealedOrPartial,
    InvalidGeneration,
    InvalidIdentity,
    TimeOutOfRange(u64),
    UnknownSeason(u8),
    UnknownWeather(u8),
    UnknownAvailabilityBits(u16),
    CapabilityValueMismatch(&'static str),
    ScalarOutOfRange(&'static str, i64),
    VisibilityOutOfRange,
    EventCountOutOfRange(usize),
    InvalidEvent,
    DuplicateEvent(EnvironmentDigestV1),
    NoncanonicalOrder,
    EncodedSizeOutOfRange(usize),
    InvalidMagic,
    UnsupportedVersion(u16),
    NonzeroReserved,
    MalformedBoolean(u8),
    Truncated,
    TrailingBytes,
    DigestMismatch,
    HashFailure,
    AllocationFailure,
    StaleOrEqualGeneration { current: u64, offered: u64 },
    RollbackGenerationMismatch,
}

#[derive(Clone, Debug, Default)]
pub struct EnvironmentProjectionPublisherV1 {
    current: Option<Arc<EnvironmentProjectionV1>>,
    previous: Option<Arc<EnvironmentProjectionV1>>,
}

impl EnvironmentProjectionV1 {
    pub fn new(mut input: EnvironmentProjectionInputV1) -> Result<Self, EnvironmentErrorV1> {
        validate_input(&mut input)?;
        let payload = encode_payload(&input)?;
        let projection_digest = hash_projection(&payload)?;
        Ok(Self {
            input,
            projection_digest,
        })
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.input.presentation_generation }

    #[must_use]
    pub const fn simulation_tick(&self) -> u64 { self.input.simulation_tick }

    #[must_use]
    pub const fn presentation_frame_digest(&self) -> EnvironmentDigestV1 {
        self.input.presentation_frame_digest
    }

    #[must_use]
    pub const fn material_table_digest(&self) -> EnvironmentDigestV1 {
        self.input.material_table_digest
    }

    #[must_use]
    pub const fn renderer_environment_identity(&self) -> EnvironmentDigestV1 {
        self.input.renderer_environment_identity
    }

    #[must_use]
    pub const fn projection_digest(&self) -> EnvironmentDigestV1 { self.projection_digest }

    #[must_use]
    pub const fn availability(&self) -> EnvironmentAvailabilityV1 { self.input.availability }

    #[must_use]
    pub const fn visibility(&self) -> GameplayVisibilityV1 { self.input.visibility }

    #[must_use]
    pub fn events(&self) -> &[EnvironmentEventV1] { &self.input.events }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EnvironmentErrorV1> {
        let mut input = self.input.clone();
        validate_input(&mut input)?;
        if input != self.input {
            return Err(EnvironmentErrorV1::NoncanonicalOrder);
        }
        let mut bytes = encode_payload(&input)?;
        let digest = hash_projection(&bytes)?;
        if digest != self.projection_digest {
            return Err(EnvironmentErrorV1::DigestMismatch);
        }
        bytes.extend_from_slice(&digest);
        if bytes.len() > MAX_ENVIRONMENT_PROJECTION_BYTES_V1 {
            return Err(EnvironmentErrorV1::EncodedSizeOutOfRange(bytes.len()));
        }
        Ok(bytes)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, EnvironmentErrorV1> {
        if bytes.len() > MAX_ENVIRONMENT_PROJECTION_BYTES_V1 {
            return Err(EnvironmentErrorV1::EncodedSizeOutOfRange(bytes.len()));
        }
        if bytes.len() < MAGIC.len() + DIGEST_BYTES {
            return Err(EnvironmentErrorV1::Truncated);
        }
        let split = bytes
            .len()
            .checked_sub(DIGEST_BYTES)
            .ok_or(EnvironmentErrorV1::Truncated)?;
        let (payload, declared_digest) = bytes.split_at(split);
        let mut reader = Reader::new(payload);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(EnvironmentErrorV1::InvalidMagic);
        }
        let schema = reader.u16()?;
        if schema != ENVIRONMENT_PROJECTION_SCHEMA_V1 {
            return Err(EnvironmentErrorV1::UnsupportedVersion(schema));
        }
        if reader.u16()? != 0 {
            return Err(EnvironmentErrorV1::NonzeroReserved);
        }
        let presentation_generation = reader.u64()?;
        let simulation_tick = reader.u64()?;
        let presentation_frame_digest = reader.digest()?;
        let material_table_digest = reader.digest()?;
        let renderer_environment_identity = reader.digest()?;
        let time_of_day_millis = reader.u64()?;
        let season = SeasonV1::decode(reader.u8()?)?;
        let weather = WeatherKindV1::decode(reader.u8()?)?;
        let availability = EnvironmentAvailabilityV1(reader.u16()?);
        let cloud_milli = reader.u16()?;
        let rain_milli = reader.u16()?;
        let wind_mm_s = [reader.i32()?, reader.i32()?];
        let precipitation_milli = reader.u16()?;
        let temperature_milli = reader.i32()?;
        let wetness_milli = reader.u16()?;
        let snow_milli = reader.u16()?;
        let frost_milli = reader.u16()?;
        let visibility = GameplayVisibilityV1 {
            terrain_blocks: reader.u16()?,
            entity_blocks: reader.u16()?,
        };
        let event_count = usize::from(reader.u16()?);
        let complete = match reader.u8()? {
            0 => false,
            1 => true,
            value => return Err(EnvironmentErrorV1::MalformedBoolean(value)),
        };
        if reader.u8()? != 0 {
            return Err(EnvironmentErrorV1::NonzeroReserved);
        }
        if event_count > MAX_ENVIRONMENT_EVENTS_V1 {
            return Err(EnvironmentErrorV1::EventCountOutOfRange(event_count));
        }
        let remaining_events = event_count
            .checked_mul(EVENT_BYTES)
            .ok_or(EnvironmentErrorV1::EncodedSizeOutOfRange(bytes.len()))?;
        if reader.remaining() != remaining_events {
            return Err(if reader.remaining() < remaining_events {
                EnvironmentErrorV1::Truncated
            } else {
                EnvironmentErrorV1::TrailingBytes
            });
        }
        let mut events = Vec::new();
        events
            .try_reserve_exact(event_count)
            .map_err(|_| EnvironmentErrorV1::AllocationFailure)?;
        for _ in 0..event_count {
            let kind_tag = reader.u16()?;
            if reader.u16()? != 0 {
                return Err(EnvironmentErrorV1::NonzeroReserved);
            }
            events.push(EnvironmentEventV1 {
                kind_tag,
                semantic_id: reader.digest()?,
                source_id: reader.digest()?,
                simulation_tick: reader.u64()?,
            });
        }
        if !reader.is_empty() {
            return Err(EnvironmentErrorV1::TrailingBytes);
        }
        let projection = Self::new(EnvironmentProjectionInputV1 {
            presentation_generation,
            simulation_tick,
            presentation_frame_digest,
            material_table_digest,
            renderer_environment_identity,
            time_of_day_millis,
            season,
            weather,
            availability,
            cloud_milli,
            rain_milli,
            wind_mm_s,
            precipitation_milli,
            temperature_milli,
            wetness_milli,
            snow_milli,
            frost_milli,
            visibility,
            events,
            complete,
        })?;
        if projection.projection_digest.as_slice() != declared_digest {
            return Err(EnvironmentErrorV1::DigestMismatch);
        }
        if projection.canonical_bytes()?.as_slice() != bytes {
            return Err(EnvironmentErrorV1::NoncanonicalOrder);
        }
        Ok(projection)
    }
}

impl EnvironmentProjectionPublisherV1 {
    pub fn publish(
        &mut self,
        projection: EnvironmentProjectionV1,
    ) -> Result<Arc<EnvironmentProjectionV1>, EnvironmentErrorV1> {
        if let Some(current) = &self.current
            && projection.presentation_generation() <= current.presentation_generation()
        {
            return Err(EnvironmentErrorV1::StaleOrEqualGeneration {
                current: current.presentation_generation(),
                offered: projection.presentation_generation(),
            });
        }
        let projection = Arc::new(projection);
        self.previous = self.current.replace(Arc::clone(&projection));
        Ok(projection)
    }

    #[must_use]
    pub fn current(&self) -> Option<Arc<EnvironmentProjectionV1>> { self.current.clone() }

    pub fn rollback(&mut self, failed_generation: u64) -> Result<(), EnvironmentErrorV1> {
        match &self.current {
            Some(current) if current.presentation_generation() == failed_generation => {
                self.current = self.previous.take();
                Ok(())
            },
            _ => Err(EnvironmentErrorV1::RollbackGenerationMismatch),
        }
    }
}

fn validate_input(input: &mut EnvironmentProjectionInputV1) -> Result<(), EnvironmentErrorV1> {
    if !input.complete {
        return Err(EnvironmentErrorV1::UnsealedOrPartial);
    }
    if input.presentation_generation == 0 {
        return Err(EnvironmentErrorV1::InvalidGeneration);
    }
    if [
        input.presentation_frame_digest,
        input.material_table_digest,
        input.renderer_environment_identity,
    ]
    .iter()
    .any(|value| *value == [0; 32])
    {
        return Err(EnvironmentErrorV1::InvalidIdentity);
    }
    if input.time_of_day_millis > MAX_ENVIRONMENT_TIME_MILLIS_V1 {
        return Err(EnvironmentErrorV1::TimeOutOfRange(input.time_of_day_millis));
    }
    if input.availability.0 & !KNOWN_AVAILABILITY_BITS != 0 {
        return Err(EnvironmentErrorV1::UnknownAvailabilityBits(
            input.availability.0,
        ));
    }
    for (name, value) in [
        ("cloud_milli", input.cloud_milli),
        ("rain_milli", input.rain_milli),
        ("precipitation_milli", input.precipitation_milli),
        ("wetness_milli", input.wetness_milli),
        ("snow_milli", input.snow_milli),
        ("frost_milli", input.frost_milli),
    ] {
        if value > 1_000 {
            return Err(EnvironmentErrorV1::ScalarOutOfRange(name, i64::from(value)));
        }
    }
    for value in input.wind_mm_s {
        if value.unsigned_abs() > MAX_ENVIRONMENT_WIND_MM_S_V1 as u32 {
            return Err(EnvironmentErrorV1::ScalarOutOfRange(
                "wind_mm_s",
                i64::from(value),
            ));
        }
    }
    if !(-1_000..=1_000).contains(&input.temperature_milli) {
        return Err(EnvironmentErrorV1::ScalarOutOfRange(
            "temperature_milli",
            i64::from(input.temperature_milli),
        ));
    }
    validate_capability_value(
        input.availability,
        EnvironmentAvailabilityV1::TIME_OF_DAY,
        input.time_of_day_millis != 0,
        "time_of_day",
    )?;
    validate_capability_value(
        input.availability,
        EnvironmentAvailabilityV1::WEATHER,
        input.cloud_milli != 0 || input.rain_milli != 0 || input.weather == WeatherKindV1::Clear,
        "weather",
    )?;
    validate_capability_value(
        input.availability,
        EnvironmentAvailabilityV1::WIND,
        input.wind_mm_s != [0, 0],
        "wind",
    )?;
    validate_capability_value(
        input.availability,
        EnvironmentAvailabilityV1::PRECIPITATION,
        input.precipitation_milli != 0,
        "precipitation",
    )?;
    validate_capability_value(
        input.availability,
        EnvironmentAvailabilityV1::TEMPERATURE,
        input.temperature_milli != 0,
        "temperature",
    )?;
    for (capability, value, name) in [
        (
            EnvironmentAvailabilityV1::WETNESS,
            input.wetness_milli,
            "wetness",
        ),
        (EnvironmentAvailabilityV1::SNOW, input.snow_milli, "snow"),
        (EnvironmentAvailabilityV1::FROST, input.frost_milli, "frost"),
    ] {
        validate_capability_value(input.availability, capability, value != 0, name)?;
    }
    let smoke_available = input
        .availability
        .contains(EnvironmentAvailabilityV1::SMOKE_REGIONS);
    let fire_available = input
        .availability
        .contains(EnvironmentAvailabilityV1::FIRE_REGIONS);
    if smoke_available || fire_available {
        return Err(EnvironmentErrorV1::CapabilityValueMismatch(
            "region-authority-unimplemented",
        ));
    }
    let lightning_available = input
        .availability
        .contains(EnvironmentAvailabilityV1::LIGHTNING_EVENTS);
    if lightning_available != !input.events.is_empty() {
        return Err(EnvironmentErrorV1::CapabilityValueMismatch(
            "lightning_events",
        ));
    }
    if input.visibility.terrain_blocks == 0
        || input.visibility.entity_blocks == 0
        || input.visibility.terrain_blocks > MAX_ENVIRONMENT_VISIBILITY_BLOCKS_V1
        || input.visibility.entity_blocks > input.visibility.terrain_blocks
    {
        return Err(EnvironmentErrorV1::VisibilityOutOfRange);
    }
    if input.events.len() > MAX_ENVIRONMENT_EVENTS_V1 {
        return Err(EnvironmentErrorV1::EventCountOutOfRange(input.events.len()));
    }
    for event in &input.events {
        if event.kind_tag == 0
            || event.semantic_id == [0; 32]
            || event.source_id == [0; 32]
            || event.simulation_tick > input.simulation_tick
        {
            return Err(EnvironmentErrorV1::InvalidEvent);
        }
    }
    input.events.sort_unstable();
    for pair in input.events.windows(2) {
        if pair[0].semantic_id == pair[1].semantic_id {
            return Err(EnvironmentErrorV1::DuplicateEvent(pair[0].semantic_id));
        }
    }
    Ok(())
}

fn validate_capability_value(
    availability: EnvironmentAvailabilityV1,
    capability: u16,
    nondefault: bool,
    field: &'static str,
) -> Result<(), EnvironmentErrorV1> {
    if !availability.contains(capability) && nondefault {
        return Err(EnvironmentErrorV1::CapabilityValueMismatch(field));
    }
    Ok(())
}

fn encode_payload(input: &EnvironmentProjectionInputV1) -> Result<Vec<u8>, EnvironmentErrorV1> {
    let event_bytes = input
        .events
        .len()
        .checked_mul(EVENT_BYTES)
        .ok_or(EnvironmentErrorV1::EncodedSizeOutOfRange(usize::MAX))?;
    let capacity = 172_usize
        .checked_add(event_bytes)
        .ok_or(EnvironmentErrorV1::EncodedSizeOutOfRange(usize::MAX))?;
    if capacity + DIGEST_BYTES > MAX_ENVIRONMENT_PROJECTION_BYTES_V1 {
        return Err(EnvironmentErrorV1::EncodedSizeOutOfRange(
            capacity + DIGEST_BYTES,
        ));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| EnvironmentErrorV1::AllocationFailure)?;
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&ENVIRONMENT_PROJECTION_SCHEMA_V1.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&input.presentation_generation.to_le_bytes());
    output.extend_from_slice(&input.simulation_tick.to_le_bytes());
    output.extend_from_slice(&input.presentation_frame_digest);
    output.extend_from_slice(&input.material_table_digest);
    output.extend_from_slice(&input.renderer_environment_identity);
    output.extend_from_slice(&input.time_of_day_millis.to_le_bytes());
    output.push(input.season as u8);
    output.push(input.weather as u8);
    output.extend_from_slice(&input.availability.0.to_le_bytes());
    output.extend_from_slice(&input.cloud_milli.to_le_bytes());
    output.extend_from_slice(&input.rain_milli.to_le_bytes());
    output.extend_from_slice(&input.wind_mm_s[0].to_le_bytes());
    output.extend_from_slice(&input.wind_mm_s[1].to_le_bytes());
    output.extend_from_slice(&input.precipitation_milli.to_le_bytes());
    output.extend_from_slice(&input.temperature_milli.to_le_bytes());
    output.extend_from_slice(&input.wetness_milli.to_le_bytes());
    output.extend_from_slice(&input.snow_milli.to_le_bytes());
    output.extend_from_slice(&input.frost_milli.to_le_bytes());
    output.extend_from_slice(&input.visibility.terrain_blocks.to_le_bytes());
    output.extend_from_slice(&input.visibility.entity_blocks.to_le_bytes());
    output.extend_from_slice(
        &u16::try_from(input.events.len())
            .map_err(|_| EnvironmentErrorV1::EventCountOutOfRange(input.events.len()))?
            .to_le_bytes(),
    );
    output.push(u8::from(input.complete));
    output.push(0);
    for event in &input.events {
        output.extend_from_slice(&event.kind_tag.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&event.semantic_id);
        output.extend_from_slice(&event.source_id);
        output.extend_from_slice(&event.simulation_tick.to_le_bytes());
    }
    Ok(output)
}

fn hash_projection(bytes: &[u8]) -> Result<EnvironmentDigestV1, EnvironmentErrorV1> {
    domain_hash_v1("bastion/r1f/environment-projection", 1, 0, bytes)
        .map_err(|_| EnvironmentErrorV1::HashFailure)
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.cursor) }

    fn is_empty(&self) -> bool { self.cursor == self.bytes.len() }

    fn take(&mut self, count: usize) -> Result<&'a [u8], EnvironmentErrorV1> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or(EnvironmentErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(EnvironmentErrorV1::Truncated)?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, EnvironmentErrorV1> {
        Ok(*self.take(1)?.first().ok_or(EnvironmentErrorV1::Truncated)?)
    }

    fn u16(&mut self) -> Result<u16, EnvironmentErrorV1> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| EnvironmentErrorV1::Truncated)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, EnvironmentErrorV1> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| EnvironmentErrorV1::Truncated)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn i32(&mut self) -> Result<i32, EnvironmentErrorV1> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| EnvironmentErrorV1::Truncated)?;
        Ok(i32::from_le_bytes(bytes))
    }

    fn digest(&mut self) -> Result<EnvironmentDigestV1, EnvironmentErrorV1> {
        self.take(DIGEST_BYTES)?
            .try_into()
            .map_err(|_| EnvironmentErrorV1::Truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn input() -> EnvironmentProjectionInputV1 {
        EnvironmentProjectionInputV1 {
            presentation_generation: 7,
            simulation_tick: 99,
            presentation_frame_digest: digest(1),
            material_table_digest: digest(2),
            renderer_environment_identity: digest(3),
            time_of_day_millis: 123_456,
            season: SeasonV1::Summer,
            weather: WeatherKindV1::Rain,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: 700,
            rain_milli: 500,
            wind_mm_s: [2_000, -1_000],
            precipitation_milli: 500,
            temperature_milli: 250,
            wetness_milli: 0,
            snow_milli: 0,
            frost_milli: 0,
            visibility: GameplayVisibilityV1 {
                terrain_blocks: 512,
                entity_blocks: 256,
            },
            events: Vec::new(),
            complete: true,
        }
    }

    #[test]
    fn canonical_vector_and_exact_decode_are_frozen() {
        let projection = EnvironmentProjectionV1::new(input()).unwrap();
        let bytes = projection.canonical_bytes().unwrap();
        assert_eq!(
            EnvironmentProjectionV1::decode_exact(&bytes).unwrap(),
            projection
        );
        assert_eq!(
            crate::hex_bytes(&projection.projection_digest()),
            "dca183a27df80befeffc7c3227b57e69fc5d75af4028d854bac8d177e0f73e36"
        );
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            EnvironmentProjectionV1::decode_exact(&trailing),
            Err(EnvironmentErrorV1::TrailingBytes)
        );
        for cut in [0, 7, 64, bytes.len() - 1] {
            assert!(EnvironmentProjectionV1::decode_exact(&bytes[..cut]).is_err());
        }
    }

    #[test]
    fn weather_boundaries_and_every_semantic_field_change_digest() {
        let base = EnvironmentProjectionV1::new(input()).unwrap();
        for weather in [
            WeatherKindV1::Clear,
            WeatherKindV1::Cloudy,
            WeatherKindV1::Storm,
        ] {
            let mut changed = input();
            changed.weather = weather;
            assert_ne!(
                EnvironmentProjectionV1::new(changed)
                    .unwrap()
                    .projection_digest(),
                base.projection_digest()
            );
        }
        let mut bytes = base.canonical_bytes().unwrap();
        bytes[133] = 0xff;
        assert!(EnvironmentProjectionV1::decode_exact(&bytes).is_err());
    }

    #[test]
    fn event_input_permutation_is_canonical_and_duplicates_reject() {
        let event_a = EnvironmentEventV1 {
            kind_tag: 1,
            semantic_id: digest(10),
            source_id: digest(11),
            simulation_tick: 98,
        };
        let event_b = EnvironmentEventV1 {
            kind_tag: 2,
            semantic_id: digest(12),
            source_id: digest(13),
            simulation_tick: 99,
        };
        let mut a = input();
        a.availability.0 |= EnvironmentAvailabilityV1::LIGHTNING_EVENTS;
        a.events = vec![event_b.clone(), event_a.clone()];
        let mut b = input();
        b.availability.0 |= EnvironmentAvailabilityV1::LIGHTNING_EVENTS;
        b.events = vec![event_a.clone(), event_b];
        assert_eq!(
            EnvironmentProjectionV1::new(a).unwrap(),
            EnvironmentProjectionV1::new(b).unwrap()
        );
        let mut duplicate = input();
        duplicate.availability.0 |= EnvironmentAvailabilityV1::LIGHTNING_EVENTS;
        duplicate.events = vec![event_a.clone(), event_a];
        assert!(matches!(
            EnvironmentProjectionV1::new(duplicate),
            Err(EnvironmentErrorV1::DuplicateEvent(_))
        ));
    }

    #[test]
    fn unavailable_fields_are_explicit_and_cannot_carry_values() {
        let projection = EnvironmentProjectionV1::new(input()).unwrap();
        assert!(
            !projection
                .availability()
                .contains(EnvironmentAvailabilityV1::WETNESS)
        );
        assert!(
            !projection
                .availability()
                .contains(EnvironmentAvailabilityV1::SMOKE_REGIONS)
        );
        let mut wet = input();
        wet.wetness_milli = 1;
        assert_eq!(
            EnvironmentProjectionV1::new(wet),
            Err(EnvironmentErrorV1::CapabilityValueMismatch("wetness"))
        );
        let mut unsupported_regions = input();
        unsupported_regions.availability.0 |= EnvironmentAvailabilityV1::FIRE_REGIONS;
        assert_eq!(
            EnvironmentProjectionV1::new(unsupported_regions),
            Err(EnvironmentErrorV1::CapabilityValueMismatch(
                "region-authority-unimplemented"
            ))
        );
    }

    #[test]
    fn invalid_ranges_partial_and_unknown_bits_fail_closed() {
        let mut partial = input();
        partial.complete = false;
        assert_eq!(
            EnvironmentProjectionV1::new(partial),
            Err(EnvironmentErrorV1::UnsealedOrPartial)
        );
        let mut invalid = input();
        invalid.cloud_milli = 1_001;
        assert!(matches!(
            EnvironmentProjectionV1::new(invalid),
            Err(EnvironmentErrorV1::ScalarOutOfRange("cloud_milli", _))
        ));
        let mut invalid = input();
        invalid.wind_mm_s[0] = MAX_ENVIRONMENT_WIND_MM_S_V1 + 1;
        assert!(EnvironmentProjectionV1::new(invalid).is_err());
        let mut invalid = input();
        invalid.visibility.entity_blocks = 513;
        assert_eq!(
            EnvironmentProjectionV1::new(invalid),
            Err(EnvironmentErrorV1::VisibilityOutOfRange)
        );
        let mut invalid = input();
        invalid.availability.0 |= 1 << 15;
        assert!(matches!(
            EnvironmentProjectionV1::new(invalid),
            Err(EnvironmentErrorV1::UnknownAvailabilityBits(_))
        ));
    }

    #[test]
    fn exact_generation_material_and_visibility_bind_projection() {
        let base = EnvironmentProjectionV1::new(input()).unwrap();
        for changed in [
            {
                let mut value = input();
                value.presentation_generation += 1;
                value
            },
            {
                let mut value = input();
                value.material_table_digest = digest(9);
                value
            },
            {
                let mut value = input();
                value.visibility.entity_blocks -= 1;
                value
            },
        ] {
            assert_ne!(
                EnvironmentProjectionV1::new(changed)
                    .unwrap()
                    .projection_digest(),
                base.projection_digest()
            );
        }
    }

    #[test]
    fn publisher_is_monotonic_immutable_and_rolls_back() {
        let mut publisher = EnvironmentProjectionPublisherV1::default();
        let first = publisher
            .publish(EnvironmentProjectionV1::new(input()).unwrap())
            .unwrap();
        let held = Arc::clone(&first);
        let mut next = input();
        next.presentation_generation = 8;
        next.presentation_frame_digest = digest(8);
        let second = publisher
            .publish(EnvironmentProjectionV1::new(next).unwrap())
            .unwrap();
        assert_eq!(held.presentation_generation(), 7);
        assert_eq!(second.presentation_generation(), 8);
        assert!(matches!(
            publisher.publish(EnvironmentProjectionV1::new(input()).unwrap()),
            Err(EnvironmentErrorV1::StaleOrEqualGeneration { .. })
        ));
        publisher.rollback(8).unwrap();
        assert_eq!(
            publisher.current().unwrap().projection_digest(),
            held.projection_digest()
        );
    }

    #[test]
    fn diagnostic_timing_is_not_part_of_projection_identity() {
        #[derive(Clone, Copy)]
        struct Diagnostics {
            interpolation_elapsed_micros: u64,
            worker_sequence: u64,
        }
        let first = Diagnostics {
            interpolation_elapsed_micros: 1,
            worker_sequence: 9,
        };
        let second = Diagnostics {
            interpolation_elapsed_micros: 999_999,
            worker_sequence: 1,
        };
        let projection = EnvironmentProjectionV1::new(input()).unwrap();
        assert_ne!(
            first.interpolation_elapsed_micros,
            second.interpolation_elapsed_micros
        );
        assert_ne!(first.worker_sequence, second.worker_sequence);
        assert_eq!(
            projection.canonical_bytes().unwrap(),
            EnvironmentProjectionV1::new(input())
                .unwrap()
                .canonical_bytes()
                .unwrap()
        );
    }
}
