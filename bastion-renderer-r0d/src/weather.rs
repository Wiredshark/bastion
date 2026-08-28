//! Deterministic renderer-owned weather presentation policy.
//!
//! This module consumes one accepted [`EnvironmentProjectionV1`] generation.
//! It does not simulate weather. It freezes the bounded precipitation and wind
//! decisions that the existing Voxygen rain path consumes, including semantic
//! seeds and a simulation-tick-derived phase.

use std::sync::Arc;

use crate::{
    domain_hash_v1,
    environment::{EnvironmentAvailabilityV1, WeatherKindV1},
};

pub const WEATHER_PRESENTATION_SCHEMA_V1: u16 = 1;
pub const MAX_WEATHER_EFFECT_RECORDS_V1: usize = 256;
pub const MAX_WEATHER_EFFECTS_PER_CELL_V1: u16 = 512;
pub const MAX_WEATHER_EFFECTS_TOTAL_V1: u32 = 65_536;
pub const MAX_WEATHER_PRESENTATION_BYTES_V1: usize = 64 * 1024;
pub const WEATHER_PHASE_MODULUS_V1: u64 = 1_000_000;

const MAGIC: &[u8; 8] = b"BSTWTH01";
const DIGEST_BYTES: usize = 32;
const RECORD_BYTES: usize = 100;
const MAX_WIND_MM_S: i32 = 200_000;

pub type WeatherDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum WeatherEffectKindV1 {
    Rain = 1,
}

impl WeatherEffectKindV1 {
    fn decode(tag: u16) -> Result<Self, WeatherErrorV1> {
        match tag {
            1 => Ok(Self::Rain),
            _ => Err(WeatherErrorV1::UnknownEffectKind(tag)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WeatherEffectInputV1 {
    pub cell_identity: WeatherDigestV1,
    pub effect_identity: WeatherDigestV1,
    pub kind: WeatherEffectKindV1,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WeatherEffectRecordV1 {
    pub cell_identity: WeatherDigestV1,
    pub effect_identity: WeatherDigestV1,
    pub kind: WeatherEffectKindV1,
    pub count: u16,
    pub seed: WeatherDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeatherPresentationInputV1 {
    pub run_identity: WeatherDigestV1,
    pub presentation_generation: u64,
    pub simulation_tick: u64,
    pub presentation_frame_digest: WeatherDigestV1,
    pub environment_projection_digest: WeatherDigestV1,
    pub environment_source_identity: WeatherDigestV1,
    pub weather: WeatherKindV1,
    pub availability: EnvironmentAvailabilityV1,
    pub cloud_milli: u16,
    pub rain_milli: u16,
    pub wind_mm_s: [i32; 2],
    pub precipitation_milli: u16,
    pub effect_inputs: Vec<WeatherEffectInputV1>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeatherPresentationV1 {
    input: WeatherPresentationInputV1,
    effect_records: Vec<WeatherEffectRecordV1>,
    phase_milli: u64,
    presentation_digest: WeatherDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeatherErrorV1 {
    UnsealedOrPartial,
    InvalidGeneration,
    InvalidIdentity(&'static str),
    ScalarOutOfRange(&'static str),
    CapabilityUnavailable(&'static str),
    WeatherValueMismatch,
    EffectCountOutOfRange(usize),
    InvalidEffectIdentity,
    DuplicateEffect,
    NoncanonicalOrder,
    TotalEffectCountOutOfRange(u32),
    UnknownWeather(u8),
    UnknownEffectKind(u16),
    InvalidMagic,
    UnsupportedVersion(u16),
    NonzeroReserved,
    MalformedBoolean(u8),
    Truncated,
    TrailingBytes,
    EncodedSizeOutOfRange(usize),
    DigestMismatch,
    HashFailure,
    StaleOrEqualGeneration { current: u64, offered: u64 },
    RollbackGenerationMismatch,
}

#[derive(Clone, Debug, Default)]
pub struct WeatherPresentationPublisherV1 {
    current: Option<Arc<WeatherPresentationV1>>,
    previous: Option<Arc<WeatherPresentationV1>>,
}

impl WeatherPresentationV1 {
    pub fn new(mut input: WeatherPresentationInputV1) -> Result<Self, WeatherErrorV1> {
        validate_input(&mut input)?;
        let effect_records = build_effect_records(&input)?;
        let phase_milli = deterministic_phase(&input)?;
        let payload = encode_payload(&input, &effect_records, phase_milli)?;
        let presentation_digest = hash_presentation(&payload)?;
        Ok(Self {
            input,
            effect_records,
            phase_milli,
            presentation_digest,
        })
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.input.presentation_generation }

    #[must_use]
    pub const fn simulation_tick(&self) -> u64 { self.input.simulation_tick }

    #[must_use]
    pub const fn presentation_frame_digest(&self) -> WeatherDigestV1 {
        self.input.presentation_frame_digest
    }

    #[must_use]
    pub const fn environment_projection_digest(&self) -> WeatherDigestV1 {
        self.input.environment_projection_digest
    }

    #[must_use]
    pub const fn environment_source_identity(&self) -> WeatherDigestV1 {
        self.input.environment_source_identity
    }

    #[must_use]
    pub const fn weather(&self) -> WeatherKindV1 { self.input.weather }

    #[must_use]
    pub const fn cloud_milli(&self) -> u16 { self.input.cloud_milli }

    #[must_use]
    pub const fn rain_milli(&self) -> u16 { self.input.rain_milli }

    #[must_use]
    pub const fn wind_mm_s(&self) -> [i32; 2] { self.input.wind_mm_s }

    #[must_use]
    pub const fn precipitation_milli(&self) -> u16 { self.input.precipitation_milli }

    #[must_use]
    pub const fn phase_milli(&self) -> u64 { self.phase_milli }

    #[must_use]
    pub const fn presentation_digest(&self) -> WeatherDigestV1 { self.presentation_digest }

    #[must_use]
    pub fn effect_records(&self) -> &[WeatherEffectRecordV1] { &self.effect_records }

    #[must_use]
    pub fn is_raining(&self) -> bool {
        matches!(
            self.input.weather,
            WeatherKindV1::Rain | WeatherKindV1::Storm
        ) && self.input.rain_milli > 0
            && self.input.precipitation_milli > 0
    }

    #[must_use]
    pub fn total_effect_count(&self) -> u32 {
        self.effect_records
            .iter()
            .map(|record| u32::from(record.count))
            .sum()
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, WeatherErrorV1> {
        let mut input = self.input.clone();
        validate_input(&mut input)?;
        if input != self.input {
            return Err(WeatherErrorV1::NoncanonicalOrder);
        }
        let records = build_effect_records(&input)?;
        let phase = deterministic_phase(&input)?;
        if records != self.effect_records || phase != self.phase_milli {
            return Err(WeatherErrorV1::DigestMismatch);
        }
        let mut bytes = encode_payload(&input, &records, phase)?;
        let digest = hash_presentation(&bytes)?;
        if digest != self.presentation_digest {
            return Err(WeatherErrorV1::DigestMismatch);
        }
        bytes.extend_from_slice(&digest);
        if bytes.len() > MAX_WEATHER_PRESENTATION_BYTES_V1 {
            return Err(WeatherErrorV1::EncodedSizeOutOfRange(bytes.len()));
        }
        Ok(bytes)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, WeatherErrorV1> {
        if bytes.len() > MAX_WEATHER_PRESENTATION_BYTES_V1 {
            return Err(WeatherErrorV1::EncodedSizeOutOfRange(bytes.len()));
        }
        if bytes.len() < MAGIC.len() + DIGEST_BYTES {
            return Err(WeatherErrorV1::Truncated);
        }
        let split = bytes
            .len()
            .checked_sub(DIGEST_BYTES)
            .ok_or(WeatherErrorV1::Truncated)?;
        let (payload, declared_digest) = bytes.split_at(split);
        let mut reader = Reader::new(payload);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(WeatherErrorV1::InvalidMagic);
        }
        let schema = reader.u16()?;
        if schema != WEATHER_PRESENTATION_SCHEMA_V1 {
            return Err(WeatherErrorV1::UnsupportedVersion(schema));
        }
        if reader.u16()? != 0 {
            return Err(WeatherErrorV1::NonzeroReserved);
        }
        let run_identity = reader.digest()?;
        let presentation_generation = reader.u64()?;
        let simulation_tick = reader.u64()?;
        let presentation_frame_digest = reader.digest()?;
        let environment_projection_digest = reader.digest()?;
        let environment_source_identity = reader.digest()?;
        let weather = match reader.u8()? {
            1 => WeatherKindV1::Clear,
            2 => WeatherKindV1::Cloudy,
            3 => WeatherKindV1::Rain,
            4 => WeatherKindV1::Storm,
            tag => return Err(WeatherErrorV1::UnknownWeather(tag)),
        };
        let complete = match reader.u8()? {
            0 => false,
            1 => true,
            value => return Err(WeatherErrorV1::MalformedBoolean(value)),
        };
        let availability = EnvironmentAvailabilityV1(reader.u16()?);
        let cloud_milli = reader.u16()?;
        let rain_milli = reader.u16()?;
        let wind_mm_s = [reader.i32()?, reader.i32()?];
        let precipitation_milli = reader.u16()?;
        if reader.u16()? != 0 {
            return Err(WeatherErrorV1::NonzeroReserved);
        }
        let phase_milli = reader.u64()?;
        let record_count = usize::try_from(reader.u32()?)
            .map_err(|_| WeatherErrorV1::EffectCountOutOfRange(usize::MAX))?;
        if record_count > MAX_WEATHER_EFFECT_RECORDS_V1 {
            return Err(WeatherErrorV1::EffectCountOutOfRange(record_count));
        }
        let mut effect_records = Vec::new();
        effect_records
            .try_reserve_exact(record_count)
            .map_err(|_| WeatherErrorV1::EffectCountOutOfRange(record_count))?;
        let mut effect_inputs = Vec::new();
        effect_inputs
            .try_reserve_exact(record_count)
            .map_err(|_| WeatherErrorV1::EffectCountOutOfRange(record_count))?;
        for _ in 0..record_count {
            let cell_identity = reader.digest()?;
            let effect_identity = reader.digest()?;
            let kind = WeatherEffectKindV1::decode(reader.u16()?)?;
            let count = reader.u16()?;
            let seed = reader.digest()?;
            effect_records.push(WeatherEffectRecordV1 {
                cell_identity,
                effect_identity,
                kind,
                count,
                seed,
            });
            effect_inputs.push(WeatherEffectInputV1 {
                cell_identity,
                effect_identity,
                kind,
            });
        }
        if !reader.is_eof() {
            return Err(WeatherErrorV1::TrailingBytes);
        }
        let input = WeatherPresentationInputV1 {
            run_identity,
            presentation_generation,
            simulation_tick,
            presentation_frame_digest,
            environment_projection_digest,
            environment_source_identity,
            weather,
            availability,
            cloud_milli,
            rain_milli,
            wind_mm_s,
            precipitation_milli,
            effect_inputs,
            complete,
        };
        let value = Self::new(input)?;
        if value.phase_milli != phase_milli || value.effect_records != effect_records {
            return Err(WeatherErrorV1::DigestMismatch);
        }
        let actual_digest = hash_presentation(payload)?;
        if actual_digest.as_slice() != declared_digest
            || value.presentation_digest != actual_digest
            || value.canonical_bytes()?.as_slice() != bytes
        {
            return Err(WeatherErrorV1::DigestMismatch);
        }
        Ok(value)
    }
}

impl WeatherPresentationPublisherV1 {
    pub fn publish(
        &mut self,
        value: WeatherPresentationV1,
    ) -> Result<Arc<WeatherPresentationV1>, WeatherErrorV1> {
        if let Some(current) = &self.current
            && value.presentation_generation() <= current.presentation_generation()
        {
            return Err(WeatherErrorV1::StaleOrEqualGeneration {
                current: current.presentation_generation(),
                offered: value.presentation_generation(),
            });
        }
        let value = Arc::new(value);
        self.previous = self.current.replace(Arc::clone(&value));
        Ok(value)
    }

    #[must_use]
    pub fn current(&self) -> Option<Arc<WeatherPresentationV1>> { self.current.clone() }

    pub fn rollback(
        &mut self,
        failed_generation: u64,
    ) -> Result<Option<Arc<WeatherPresentationV1>>, WeatherErrorV1> {
        match &self.current {
            Some(current) if current.presentation_generation() == failed_generation => {
                self.current = self.previous.take();
                Ok(self.current.clone())
            },
            _ => Err(WeatherErrorV1::RollbackGenerationMismatch),
        }
    }
}

fn validate_input(input: &mut WeatherPresentationInputV1) -> Result<(), WeatherErrorV1> {
    if !input.complete {
        return Err(WeatherErrorV1::UnsealedOrPartial);
    }
    if input.presentation_generation == 0 {
        return Err(WeatherErrorV1::InvalidGeneration);
    }
    for (field, digest) in [
        ("run_identity", input.run_identity),
        ("presentation_frame_digest", input.presentation_frame_digest),
        (
            "environment_projection_digest",
            input.environment_projection_digest,
        ),
        (
            "environment_source_identity",
            input.environment_source_identity,
        ),
    ] {
        if digest == [0; 32] {
            return Err(WeatherErrorV1::InvalidIdentity(field));
        }
    }
    if input.cloud_milli > 1_000 || input.rain_milli > 1_000 || input.precipitation_milli > 1_000 {
        return Err(WeatherErrorV1::ScalarOutOfRange("weather_milli"));
    }
    if input
        .wind_mm_s
        .iter()
        .any(|value| value.unsigned_abs() > MAX_WIND_MM_S.unsigned_abs())
    {
        return Err(WeatherErrorV1::ScalarOutOfRange("wind_mm_s"));
    }
    for (field, capability) in [
        ("weather", EnvironmentAvailabilityV1::WEATHER),
        ("wind", EnvironmentAvailabilityV1::WIND),
        ("precipitation", EnvironmentAvailabilityV1::PRECIPITATION),
    ] {
        if !input.availability.contains(capability) {
            return Err(WeatherErrorV1::CapabilityUnavailable(field));
        }
    }
    let raining = matches!(input.weather, WeatherKindV1::Rain | WeatherKindV1::Storm);
    if raining {
        if input.rain_milli == 0 || input.precipitation_milli == 0 || input.effect_inputs.is_empty()
        {
            return Err(WeatherErrorV1::WeatherValueMismatch);
        }
    } else if input.rain_milli != 0
        || input.precipitation_milli != 0
        || !input.effect_inputs.is_empty()
    {
        return Err(WeatherErrorV1::WeatherValueMismatch);
    }
    if input.effect_inputs.len() > MAX_WEATHER_EFFECT_RECORDS_V1 {
        return Err(WeatherErrorV1::EffectCountOutOfRange(
            input.effect_inputs.len(),
        ));
    }
    input.effect_inputs.sort_unstable();
    for (index, effect) in input.effect_inputs.iter().enumerate() {
        if effect.cell_identity == [0; 32] || effect.effect_identity == [0; 32] {
            return Err(WeatherErrorV1::InvalidEffectIdentity);
        }
        if index > 0 && input.effect_inputs[index - 1] == *effect {
            return Err(WeatherErrorV1::DuplicateEffect);
        }
    }
    Ok(())
}

fn build_effect_records(
    input: &WeatherPresentationInputV1,
) -> Result<Vec<WeatherEffectRecordV1>, WeatherErrorV1> {
    let numerator = u32::from(input.precipitation_milli)
        .checked_mul(u32::from(MAX_WEATHER_EFFECTS_PER_CELL_V1))
        .ok_or(WeatherErrorV1::TotalEffectCountOutOfRange(u32::MAX))?;
    let count = if numerator == 0 {
        0
    } else {
        u16::try_from((numerator + 999) / 1_000)
            .map_err(|_| WeatherErrorV1::TotalEffectCountOutOfRange(numerator))?
    };
    let total = u32::from(count)
        .checked_mul(
            u32::try_from(input.effect_inputs.len())
                .map_err(|_| WeatherErrorV1::EffectCountOutOfRange(input.effect_inputs.len()))?,
        )
        .ok_or(WeatherErrorV1::TotalEffectCountOutOfRange(u32::MAX))?;
    if total > MAX_WEATHER_EFFECTS_TOTAL_V1 {
        return Err(WeatherErrorV1::TotalEffectCountOutOfRange(total));
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(input.effect_inputs.len())
        .map_err(|_| WeatherErrorV1::EffectCountOutOfRange(input.effect_inputs.len()))?;
    for effect in &input.effect_inputs {
        let mut payload = Vec::with_capacity(115);
        payload.extend_from_slice(&input.run_identity);
        payload.extend_from_slice(&input.presentation_generation.to_le_bytes());
        payload.extend_from_slice(&input.simulation_tick.to_le_bytes());
        payload.push(input.weather as u8);
        payload.extend_from_slice(&effect.cell_identity);
        payload.extend_from_slice(&effect.effect_identity);
        payload.extend_from_slice(&(effect.kind as u16).to_le_bytes());
        let seed = domain_hash_v1("bastion/r1f/weather-effect-seed", 1, 0, &payload)
            .map_err(|_| WeatherErrorV1::HashFailure)?;
        records.push(WeatherEffectRecordV1 {
            cell_identity: effect.cell_identity,
            effect_identity: effect.effect_identity,
            kind: effect.kind,
            count,
            seed,
        });
    }
    Ok(records)
}

fn deterministic_phase(input: &WeatherPresentationInputV1) -> Result<u64, WeatherErrorV1> {
    let wind_magnitude = u64::from(input.wind_mm_s[0].unsigned_abs())
        .checked_add(u64::from(input.wind_mm_s[1].unsigned_abs()))
        .ok_or(WeatherErrorV1::ScalarOutOfRange("wind_phase"))?;
    let step = wind_magnitude
        .checked_add(u64::from(input.precipitation_milli))
        .ok_or(WeatherErrorV1::ScalarOutOfRange("weather_phase"))?;
    input
        .simulation_tick
        .checked_mul(step)
        .map(|value| value % WEATHER_PHASE_MODULUS_V1)
        .ok_or(WeatherErrorV1::ScalarOutOfRange("weather_phase"))
}

fn encode_payload(
    input: &WeatherPresentationInputV1,
    records: &[WeatherEffectRecordV1],
    phase_milli: u64,
) -> Result<Vec<u8>, WeatherErrorV1> {
    let record_bytes = records
        .len()
        .checked_mul(RECORD_BYTES)
        .ok_or(WeatherErrorV1::EncodedSizeOutOfRange(usize::MAX))?;
    let capacity = 188usize
        .checked_add(record_bytes)
        .ok_or(WeatherErrorV1::EncodedSizeOutOfRange(usize::MAX))?;
    if capacity
        .checked_add(DIGEST_BYTES)
        .is_none_or(|value| value > MAX_WEATHER_PRESENTATION_BYTES_V1)
    {
        return Err(WeatherErrorV1::EncodedSizeOutOfRange(capacity));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| WeatherErrorV1::EncodedSizeOutOfRange(capacity))?;
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&WEATHER_PRESENTATION_SCHEMA_V1.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&input.run_identity);
    output.extend_from_slice(&input.presentation_generation.to_le_bytes());
    output.extend_from_slice(&input.simulation_tick.to_le_bytes());
    output.extend_from_slice(&input.presentation_frame_digest);
    output.extend_from_slice(&input.environment_projection_digest);
    output.extend_from_slice(&input.environment_source_identity);
    output.push(input.weather as u8);
    output.push(u8::from(input.complete));
    output.extend_from_slice(&input.availability.0.to_le_bytes());
    output.extend_from_slice(&input.cloud_milli.to_le_bytes());
    output.extend_from_slice(&input.rain_milli.to_le_bytes());
    for value in input.wind_mm_s {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(&input.precipitation_milli.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&phase_milli.to_le_bytes());
    output.extend_from_slice(
        &u32::try_from(records.len())
            .map_err(|_| WeatherErrorV1::EffectCountOutOfRange(records.len()))?
            .to_le_bytes(),
    );
    for record in records {
        output.extend_from_slice(&record.cell_identity);
        output.extend_from_slice(&record.effect_identity);
        output.extend_from_slice(&(record.kind as u16).to_le_bytes());
        output.extend_from_slice(&record.count.to_le_bytes());
        output.extend_from_slice(&record.seed);
    }
    Ok(output)
}

fn hash_presentation(payload: &[u8]) -> Result<WeatherDigestV1, WeatherErrorV1> {
    domain_hash_v1("bastion/r1f/weather-presentation", 1, 0, payload)
        .map_err(|_| WeatherErrorV1::HashFailure)
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], WeatherErrorV1> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or(WeatherErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(WeatherErrorV1::Truncated)?;
        self.cursor = end;
        Ok(value)
    }

    fn digest(&mut self) -> Result<WeatherDigestV1, WeatherErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| WeatherErrorV1::Truncated)
    }

    fn u8(&mut self) -> Result<u8, WeatherErrorV1> {
        self.take(1)?
            .first()
            .copied()
            .ok_or(WeatherErrorV1::Truncated)
    }

    fn u16(&mut self) -> Result<u16, WeatherErrorV1> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| WeatherErrorV1::Truncated)?,
        ))
    }

    fn u32(&mut self) -> Result<u32, WeatherErrorV1> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| WeatherErrorV1::Truncated)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, WeatherErrorV1> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| WeatherErrorV1::Truncated)?,
        ))
    }

    fn i32(&mut self) -> Result<i32, WeatherErrorV1> {
        Ok(i32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| WeatherErrorV1::Truncated)?,
        ))
    }

    fn is_eof(&self) -> bool { self.cursor == self.bytes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> WeatherDigestV1 { [byte; 32] }

    fn base(weather: WeatherKindV1) -> WeatherPresentationInputV1 {
        let raining = matches!(weather, WeatherKindV1::Rain | WeatherKindV1::Storm);
        WeatherPresentationInputV1 {
            run_identity: digest(1),
            presentation_generation: 7,
            simulation_tick: 300,
            presentation_frame_digest: digest(2),
            environment_projection_digest: digest(3),
            environment_source_identity: digest(4),
            weather,
            availability: EnvironmentAvailabilityV1::PRODUCTION_V1,
            cloud_milli: if raining { 300 } else { 0 },
            rain_milli: if raining { 300 } else { 0 },
            wind_mm_s: if raining { [15_000, 20_000] } else { [0, 0] },
            precipitation_milli: if raining { 300 } else { 0 },
            effect_inputs: if raining {
                vec![
                    WeatherEffectInputV1 {
                        cell_identity: digest(6),
                        effect_identity: digest(8),
                        kind: WeatherEffectKindV1::Rain,
                    },
                    WeatherEffectInputV1 {
                        cell_identity: digest(5),
                        effect_identity: digest(7),
                        kind: WeatherEffectKindV1::Rain,
                    },
                ]
            } else {
                Vec::new()
            },
            complete: true,
        }
    }

    #[test]
    fn clear_rain_and_storm_boundaries_are_explicit() {
        let clear = WeatherPresentationV1::new(base(WeatherKindV1::Clear)).unwrap();
        assert!(!clear.is_raining());
        assert_eq!(clear.total_effect_count(), 0);
        let rain = WeatherPresentationV1::new(base(WeatherKindV1::Rain)).unwrap();
        let storm = WeatherPresentationV1::new(base(WeatherKindV1::Storm)).unwrap();
        assert!(rain.is_raining());
        assert!(storm.is_raining());
        assert_eq!(rain.effect_records().len(), 2);
        assert!(rain.total_effect_count() <= MAX_WEATHER_EFFECTS_TOTAL_V1);
    }

    #[test]
    fn producer_order_does_not_change_bytes_or_digest() {
        let a = WeatherPresentationV1::new(base(WeatherKindV1::Storm)).unwrap();
        let mut input = base(WeatherKindV1::Storm);
        input.effect_inputs.reverse();
        let b = WeatherPresentationV1::new(input).unwrap();
        assert_eq!(a.canonical_bytes().unwrap(), b.canonical_bytes().unwrap());
        assert_eq!(a.presentation_digest(), b.presentation_digest());
    }

    #[test]
    fn canonical_roundtrip_exact_eof_and_frozen_vector() {
        let value = WeatherPresentationV1::new(base(WeatherKindV1::Storm)).unwrap();
        let bytes = value.canonical_bytes().unwrap();
        assert_eq!(WeatherPresentationV1::decode_exact(&bytes).unwrap(), value);
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(WeatherPresentationV1::decode_exact(&trailing).is_err());
        assert_eq!(
            crate::hex_bytes(&value.presentation_digest()),
            "4f611ed4127a224a273548df107ddf43693688ac982dfcf62202ac3d892c0a88"
        );
    }

    #[test]
    fn seed_and_phase_bind_run_generation_tick_cell_and_effect() {
        let base_value = WeatherPresentationV1::new(base(WeatherKindV1::Storm)).unwrap();
        for mutate in [
            |input: &mut WeatherPresentationInputV1| input.run_identity = digest(9),
            |input: &mut WeatherPresentationInputV1| input.presentation_generation += 1,
            |input: &mut WeatherPresentationInputV1| input.simulation_tick += 1,
        ] {
            let mut input = base(WeatherKindV1::Storm);
            mutate(&mut input);
            let changed = WeatherPresentationV1::new(input).unwrap();
            assert_ne!(
                base_value.effect_records()[0].seed,
                changed.effect_records()[0].seed
            );
        }
        assert_eq!(
            base_value.phase_milli(),
            (300 * (15_000 + 20_000 + 300)) % WEATHER_PHASE_MODULUS_V1
        );
    }

    #[test]
    fn malformed_partial_duplicate_and_oversize_fail_closed() {
        let mut partial = base(WeatherKindV1::Storm);
        partial.complete = false;
        assert_eq!(
            WeatherPresentationV1::new(partial),
            Err(WeatherErrorV1::UnsealedOrPartial)
        );
        let mut duplicate = base(WeatherKindV1::Storm);
        duplicate.effect_inputs.push(duplicate.effect_inputs[0]);
        assert_eq!(
            WeatherPresentationV1::new(duplicate),
            Err(WeatherErrorV1::DuplicateEffect)
        );
        let mut oversize = base(WeatherKindV1::Storm);
        oversize.effect_inputs = (1..=MAX_WEATHER_EFFECT_RECORDS_V1 + 1)
            .map(|ordinal| WeatherEffectInputV1 {
                cell_identity: {
                    let mut value = digest(10);
                    value[..8].copy_from_slice(&(ordinal as u64).to_le_bytes());
                    value
                },
                effect_identity: digest(11),
                kind: WeatherEffectKindV1::Rain,
            })
            .collect();
        assert!(matches!(
            WeatherPresentationV1::new(oversize),
            Err(WeatherErrorV1::EffectCountOutOfRange(_))
        ));
    }

    #[test]
    fn unavailable_fields_and_weather_value_mismatch_fail_closed() {
        let mut unavailable = base(WeatherKindV1::Storm);
        unavailable.availability = EnvironmentAvailabilityV1(
            EnvironmentAvailabilityV1::WEATHER | EnvironmentAvailabilityV1::WIND,
        );
        assert_eq!(
            WeatherPresentationV1::new(unavailable),
            Err(WeatherErrorV1::CapabilityUnavailable("precipitation"))
        );
        let mut clear_with_rain = base(WeatherKindV1::Clear);
        clear_with_rain.rain_milli = 1;
        assert_eq!(
            WeatherPresentationV1::new(clear_with_rain),
            Err(WeatherErrorV1::WeatherValueMismatch)
        );
    }

    #[test]
    fn stale_generation_rejects_and_rollback_restores_held_reader() {
        let mut publisher = WeatherPresentationPublisherV1::default();
        let first = publisher
            .publish(WeatherPresentationV1::new(base(WeatherKindV1::Rain)).unwrap())
            .unwrap();
        assert!(matches!(
            publisher.publish(WeatherPresentationV1::new(base(WeatherKindV1::Storm)).unwrap()),
            Err(WeatherErrorV1::StaleOrEqualGeneration { .. })
        ));
        let mut next_input = base(WeatherKindV1::Storm);
        next_input.presentation_generation = 8;
        let second = publisher
            .publish(WeatherPresentationV1::new(next_input).unwrap())
            .unwrap();
        assert_eq!(publisher.rollback(8).unwrap().unwrap(), first);
        assert_eq!(second.presentation_generation(), 8);
        assert_eq!(first.presentation_generation(), 7);
    }

    #[test]
    fn bounded_counts_and_wind_response_are_stable() {
        let value = WeatherPresentationV1::new(base(WeatherKindV1::Storm)).unwrap();
        assert_eq!(value.wind_mm_s(), [15_000, 20_000]);
        assert_eq!(value.effect_records()[0].count, 154);
        assert_eq!(value.total_effect_count(), 308);
        assert!(
            value
                .effect_records()
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
    }
}
