//! Bounded renderer-owned world-lens snapshots.
//!
//! A lens visualizes facts already present in an immutable presentation
//! generation. It never discovers, infers, or mutates gameplay authority.

use std::sync::Arc;

use crate::{DomainHashErrorV1, domain_hash_v1};

pub const LENS_FRAME_VERSION_V1: u16 = 1;
pub const MAX_LENS_FRAME_BYTES_V1: usize = 64 * 1024;
pub const MAX_LENS_DATUMS_V1: usize = 64;
pub const MAX_VISIBLE_LENS_DATUMS_V1: u16 = 16;
pub const MAX_LENS_LABEL_BYTES_V1: usize = 48;
pub const MAX_LENS_PRIORITY_V1: u16 = 1_000;
pub const MAX_LENS_VALUE_V1: i32 = 1_000_000_000;
const MAGIC_V1: &[u8; 8] = b"BASR1LN1";
const SEALED_V1: u8 = 1;

pub type LensDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LensModeV1 {
    Off = 0,
    Weather = 1,
}

impl TryFrom<u8> for LensModeV1 {
    type Error = LensErrorV1;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Off),
            1 => Ok(Self::Weather),
            other => Err(LensErrorV1::UnknownMode(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LensKindV1 {
    Weather = 1,
}

impl TryFrom<u8> for LensKindV1 {
    type Error = LensErrorV1;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Weather),
            other => Err(LensErrorV1::UnknownKind(other)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LensDatumV1 {
    pub semantic_id: LensDigestV1,
    pub kind: LensKindV1,
    pub authority_digest: LensDigestV1,
    pub authority_generation: u64,
    pub priority: u16,
    /// Kind-defined bounded integer payload. Weather uses
    /// `[kind_tag, cloud_milli, rain_milli, wind_speed_mm_s]`.
    pub values: [i32; 4],
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LensFrameInputV1 {
    pub presentation_generation: u64,
    pub publication_sequence: u64,
    pub simulation_tick: u64,
    pub presentation_frame_digest: LensDigestV1,
    pub camera_token: LensDigestV1,
    pub selection_digest: LensDigestV1,
    pub mode: LensModeV1,
    pub max_visible_datums: u16,
    pub datums: Vec<LensDatumV1>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LensFrameV1 {
    presentation_generation: u64,
    publication_sequence: u64,
    simulation_tick: u64,
    presentation_frame_digest: LensDigestV1,
    camera_token: LensDigestV1,
    selection_digest: LensDigestV1,
    mode: LensModeV1,
    max_visible_datums: u16,
    datums: Vec<LensDatumV1>,
    canonical_bytes: Vec<u8>,
    frame_digest: LensDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LensErrorV1 {
    InvalidMagic,
    UnsupportedVersion(u16),
    UnknownMode(u8),
    UnknownKind(u8),
    UnsealedOrPartial,
    InvalidGeneration,
    InvalidBinding,
    InvalidDensity(u16),
    InvalidDatum,
    InvalidLabel,
    DuplicateDatum(LensDigestV1),
    ModeDatumMismatch,
    TooManyDatums(usize),
    EncodedSizeExceeded(usize),
    AllocationFailure,
    Truncated,
    TrailingBytes(usize),
    NonCanonicalOrder,
    DigestMismatch,
    StaleOrEqualPublication {
        current_generation: u64,
        current_sequence: u64,
        offered_generation: u64,
        offered_sequence: u64,
    },
    Hash(DomainHashErrorV1),
}

#[derive(Clone, Debug, Default)]
pub struct LensPublicationV1 {
    current: Option<Arc<LensFrameV1>>,
    previous: Option<Arc<LensFrameV1>>,
}

impl LensFrameV1 {
    pub fn seal(mut input: LensFrameInputV1) -> Result<Self, LensErrorV1> {
        validate_header(&input)?;
        validate_datums(&input.datums, input.presentation_generation)?;
        if input.mode == LensModeV1::Off && !input.datums.is_empty() {
            return Err(LensErrorV1::ModeDatumMismatch);
        }
        if input.mode == LensModeV1::Weather
            && (input.datums.is_empty()
                || input
                    .datums
                    .iter()
                    .any(|datum| datum.kind != LensKindV1::Weather))
        {
            return Err(LensErrorV1::ModeDatumMismatch);
        }

        // Density selection is semantic: highest priority wins, ties use full
        // semantic identity. Final wire order remains kind/id canonical.
        input.datums.sort_unstable_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.semantic_id.cmp(&right.semantic_id))
        });
        input.datums.truncate(usize::from(input.max_visible_datums));
        input
            .datums
            .sort_unstable_by_key(|datum| (datum.kind, datum.semantic_id));

        let mut frame = Self {
            presentation_generation: input.presentation_generation,
            publication_sequence: input.publication_sequence,
            simulation_tick: input.simulation_tick,
            presentation_frame_digest: input.presentation_frame_digest,
            camera_token: input.camera_token,
            selection_digest: input.selection_digest,
            mode: input.mode,
            max_visible_datums: input.max_visible_datums,
            datums: input.datums,
            canonical_bytes: Vec::new(),
            frame_digest: [0; 32],
        };
        let prefix = frame.encode_prefix()?;
        frame.frame_digest =
            domain_hash_v1("bastion/r1g/lens-frame", 1, 0, &prefix).map_err(LensErrorV1::Hash)?;
        frame.canonical_bytes = prefix;
        frame.canonical_bytes.extend_from_slice(&frame.frame_digest);
        if frame.canonical_bytes.len() > MAX_LENS_FRAME_BYTES_V1 {
            return Err(LensErrorV1::EncodedSizeExceeded(
                frame.canonical_bytes.len(),
            ));
        }
        Ok(frame)
    }

    #[must_use]
    pub const fn presentation_generation(&self) -> u64 { self.presentation_generation }

    #[must_use]
    pub const fn publication_sequence(&self) -> u64 { self.publication_sequence }

    #[must_use]
    pub const fn simulation_tick(&self) -> u64 { self.simulation_tick }

    #[must_use]
    pub const fn presentation_frame_digest(&self) -> LensDigestV1 { self.presentation_frame_digest }

    #[must_use]
    pub const fn camera_token(&self) -> LensDigestV1 { self.camera_token }

    #[must_use]
    pub const fn selection_digest(&self) -> LensDigestV1 { self.selection_digest }

    #[must_use]
    pub const fn mode(&self) -> LensModeV1 { self.mode }

    #[must_use]
    pub const fn max_visible_datums(&self) -> u16 { self.max_visible_datums }

    #[must_use]
    pub fn datums(&self) -> &[LensDatumV1] { &self.datums }

    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] { &self.canonical_bytes }

    #[must_use]
    pub const fn frame_digest(&self) -> LensDigestV1 { self.frame_digest }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, LensErrorV1> {
        if bytes.len() > MAX_LENS_FRAME_BYTES_V1 {
            return Err(LensErrorV1::EncodedSizeExceeded(bytes.len()));
        }
        let mut reader = ReaderV1::new(bytes);
        if reader.take(8)? != MAGIC_V1 {
            return Err(LensErrorV1::InvalidMagic);
        }
        let version = reader.u16()?;
        if version != LENS_FRAME_VERSION_V1 {
            return Err(LensErrorV1::UnsupportedVersion(version));
        }
        if reader.u8()? != SEALED_V1 {
            return Err(LensErrorV1::UnsealedOrPartial);
        }
        let mode = LensModeV1::try_from(reader.u8()?)?;
        let presentation_generation = reader.u64()?;
        let publication_sequence = reader.u64()?;
        let simulation_tick = reader.u64()?;
        let presentation_frame_digest = reader.digest()?;
        let camera_token = reader.digest()?;
        let selection_digest = reader.digest()?;
        let max_visible_datums = reader.u16()?;
        let count = usize::from(reader.u16()?);
        if count > MAX_LENS_DATUMS_V1 || count > usize::from(max_visible_datums) {
            return Err(LensErrorV1::TooManyDatums(count));
        }
        let mut datums = Vec::new();
        datums
            .try_reserve_exact(count)
            .map_err(|_| LensErrorV1::AllocationFailure)?;
        for _ in 0..count {
            let semantic_id = reader.digest()?;
            let kind = LensKindV1::try_from(reader.u8()?)?;
            let authority_digest = reader.digest()?;
            let authority_generation = reader.u64()?;
            let priority = reader.u16()?;
            let values = [reader.i32()?, reader.i32()?, reader.i32()?, reader.i32()?];
            let label_len = usize::from(reader.u8()?);
            let label = std::str::from_utf8(reader.take(label_len)?)
                .map_err(|_| LensErrorV1::InvalidLabel)?
                .to_owned();
            datums.push(LensDatumV1 {
                semantic_id,
                kind,
                authority_digest,
                authority_generation,
                priority,
                values,
                label,
            });
        }
        let encoded_digest = reader.digest()?;
        reader.finish()?;
        let rebuilt = Self::seal(LensFrameInputV1 {
            presentation_generation,
            publication_sequence,
            simulation_tick,
            presentation_frame_digest,
            camera_token,
            selection_digest,
            mode,
            max_visible_datums,
            datums,
            complete: true,
        })?;
        if rebuilt.frame_digest != encoded_digest {
            return Err(LensErrorV1::DigestMismatch);
        }
        if rebuilt.canonical_bytes != bytes {
            return Err(LensErrorV1::NonCanonicalOrder);
        }
        Ok(rebuilt)
    }

    fn encode_prefix(&self) -> Result<Vec<u8>, LensErrorV1> {
        let mut output = Vec::new();
        output
            .try_reserve(192)
            .map_err(|_| LensErrorV1::AllocationFailure)?;
        output.extend_from_slice(MAGIC_V1);
        output.extend_from_slice(&LENS_FRAME_VERSION_V1.to_le_bytes());
        output.push(SEALED_V1);
        output.push(self.mode as u8);
        output.extend_from_slice(&self.presentation_generation.to_le_bytes());
        output.extend_from_slice(&self.publication_sequence.to_le_bytes());
        output.extend_from_slice(&self.simulation_tick.to_le_bytes());
        output.extend_from_slice(&self.presentation_frame_digest);
        output.extend_from_slice(&self.camera_token);
        output.extend_from_slice(&self.selection_digest);
        output.extend_from_slice(&self.max_visible_datums.to_le_bytes());
        let count = u16::try_from(self.datums.len())
            .map_err(|_| LensErrorV1::TooManyDatums(self.datums.len()))?;
        output.extend_from_slice(&count.to_le_bytes());
        for datum in &self.datums {
            output.extend_from_slice(&datum.semantic_id);
            output.push(datum.kind as u8);
            output.extend_from_slice(&datum.authority_digest);
            output.extend_from_slice(&datum.authority_generation.to_le_bytes());
            output.extend_from_slice(&datum.priority.to_le_bytes());
            for value in datum.values {
                output.extend_from_slice(&value.to_le_bytes());
            }
            let label_len =
                u8::try_from(datum.label.len()).map_err(|_| LensErrorV1::InvalidLabel)?;
            output.push(label_len);
            output.extend_from_slice(datum.label.as_bytes());
            if output.len() > MAX_LENS_FRAME_BYTES_V1.saturating_sub(32) {
                return Err(LensErrorV1::EncodedSizeExceeded(output.len()));
            }
        }
        Ok(output)
    }
}

impl LensPublicationV1 {
    pub fn publish(&mut self, frame: LensFrameV1) -> Result<Arc<LensFrameV1>, LensErrorV1> {
        if let Some(current) = &self.current {
            if current.presentation_generation == frame.presentation_generation
                && current.publication_sequence == frame.publication_sequence
                && current.frame_digest == frame.frame_digest
            {
                return Ok(Arc::clone(current));
            }
            if (frame.presentation_generation, frame.publication_sequence)
                <= (
                    current.presentation_generation,
                    current.publication_sequence,
                )
            {
                return Err(LensErrorV1::StaleOrEqualPublication {
                    current_generation: current.presentation_generation,
                    current_sequence: current.publication_sequence,
                    offered_generation: frame.presentation_generation,
                    offered_sequence: frame.publication_sequence,
                });
            }
        }
        let frame = Arc::new(frame);
        self.previous = self.current.replace(Arc::clone(&frame));
        Ok(frame)
    }

    #[must_use]
    pub fn current(&self) -> Option<Arc<LensFrameV1>> { self.current.clone() }

    pub fn rollback(&mut self, failed_generation: u64) -> Result<(), LensErrorV1> {
        if self
            .current
            .as_ref()
            .is_none_or(|current| current.presentation_generation != failed_generation)
        {
            return Err(LensErrorV1::InvalidGeneration);
        }
        self.current = self.previous.take();
        Ok(())
    }
}

fn validate_header(input: &LensFrameInputV1) -> Result<(), LensErrorV1> {
    if !input.complete {
        return Err(LensErrorV1::UnsealedOrPartial);
    }
    if input.presentation_generation == 0 || input.publication_sequence == 0 {
        return Err(LensErrorV1::InvalidGeneration);
    }
    if [
        input.presentation_frame_digest,
        input.camera_token,
        input.selection_digest,
    ]
    .iter()
    .any(|digest| *digest == [0; 32])
    {
        return Err(LensErrorV1::InvalidBinding);
    }
    if input.max_visible_datums == 0 || input.max_visible_datums > MAX_VISIBLE_LENS_DATUMS_V1 {
        return Err(LensErrorV1::InvalidDensity(input.max_visible_datums));
    }
    if input.datums.len() > MAX_LENS_DATUMS_V1 {
        return Err(LensErrorV1::TooManyDatums(input.datums.len()));
    }
    Ok(())
}

fn validate_datums(datums: &[LensDatumV1], generation: u64) -> Result<(), LensErrorV1> {
    let mut ids = datums
        .iter()
        .map(|datum| datum.semantic_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    if let Some(pair) = ids.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(LensErrorV1::DuplicateDatum(pair[0]));
    }
    for datum in datums {
        if datum.semantic_id == [0; 32]
            || datum.authority_digest == [0; 32]
            || datum.authority_generation != generation
            || datum.priority > MAX_LENS_PRIORITY_V1
            || datum.values.iter().any(|value| {
                value
                    .checked_abs()
                    .is_none_or(|value| value > MAX_LENS_VALUE_V1)
            })
        {
            return Err(LensErrorV1::InvalidDatum);
        }
        if datum.label.is_empty()
            || datum.label.len() > MAX_LENS_LABEL_BYTES_V1
            || !datum
                .label
                .bytes()
                .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        {
            return Err(LensErrorV1::InvalidLabel);
        }
    }
    Ok(())
}

struct ReaderV1<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ReaderV1<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, offset: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], LensErrorV1> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(LensErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(LensErrorV1::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, LensErrorV1> { Ok(self.take(1)?[0]) }

    fn u16(&mut self) -> Result<u16, LensErrorV1> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| LensErrorV1::Truncated)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, LensErrorV1> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| LensErrorV1::Truncated)?,
        ))
    }

    fn i32(&mut self) -> Result<i32, LensErrorV1> {
        Ok(i32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| LensErrorV1::Truncated)?,
        ))
    }

    fn digest(&mut self) -> Result<LensDigestV1, LensErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| LensErrorV1::Truncated)
    }

    fn finish(self) -> Result<(), LensErrorV1> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(LensErrorV1::TrailingBytes(self.bytes.len() - self.offset))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> LensDigestV1 { [value; 32] }

    fn datum(id: u8, priority: u16) -> LensDatumV1 {
        LensDatumV1 {
            semantic_id: digest(id),
            kind: LensKindV1::Weather,
            authority_digest: digest(9),
            authority_generation: 7,
            priority,
            values: [3, 600, 400, 2_500],
            label: format!("RAIN {id}"),
        }
    }

    fn input(datums: Vec<LensDatumV1>) -> LensFrameInputV1 {
        LensFrameInputV1 {
            presentation_generation: 7,
            publication_sequence: 1,
            simulation_tick: 300,
            presentation_frame_digest: digest(1),
            camera_token: digest(2),
            selection_digest: digest(3),
            mode: LensModeV1::Weather,
            max_visible_datums: 4,
            datums,
            complete: true,
        }
    }

    #[test]
    fn canonical_round_trip_and_exact_eof() {
        let frame = LensFrameV1::seal(input(vec![datum(2, 4), datum(1, 5)])).unwrap();
        let decoded = LensFrameV1::decode_exact(frame.canonical_bytes()).unwrap();
        assert_eq!(decoded, frame);
        let mut trailing = frame.canonical_bytes().to_vec();
        trailing.push(0);
        assert_eq!(
            LensFrameV1::decode_exact(&trailing),
            Err(LensErrorV1::TrailingBytes(1))
        );
    }

    #[test]
    fn input_order_is_nonauthoritative_and_density_is_stable() {
        let a = LensFrameV1::seal(input(vec![
            datum(1, 1),
            datum(2, 4),
            datum(3, 3),
            datum(4, 2),
            datum(5, 5),
        ]))
        .unwrap();
        let b = LensFrameV1::seal(input(vec![
            datum(5, 5),
            datum(4, 2),
            datum(3, 3),
            datum(2, 4),
            datum(1, 1),
        ]))
        .unwrap();
        assert_eq!(a.frame_digest(), b.frame_digest());
        assert_eq!(a.datums().len(), 4);
        assert!(
            a.datums()
                .iter()
                .all(|datum| datum.semantic_id != digest(1))
        );
    }

    #[test]
    fn duplicate_malformed_stale_and_partial_inputs_fail_closed() {
        assert!(matches!(
            LensFrameV1::seal(input(vec![datum(1, 1), datum(1, 2)])),
            Err(LensErrorV1::DuplicateDatum(_))
        ));
        let mut malformed = datum(1, 1);
        malformed.label = "\n".to_owned();
        assert_eq!(
            LensFrameV1::seal(input(vec![malformed])),
            Err(LensErrorV1::InvalidLabel)
        );
        let mut stale = datum(1, 1);
        stale.authority_generation = 6;
        assert_eq!(
            LensFrameV1::seal(input(vec![stale])),
            Err(LensErrorV1::InvalidDatum)
        );
        let mut partial = input(vec![datum(1, 1)]);
        partial.complete = false;
        assert_eq!(
            LensFrameV1::seal(partial),
            Err(LensErrorV1::UnsealedOrPartial)
        );
    }

    #[test]
    fn off_is_explicit_and_preserves_empty_legacy_path() {
        let mut off = input(Vec::new());
        off.mode = LensModeV1::Off;
        let frame = LensFrameV1::seal(off).unwrap();
        assert_eq!(frame.mode(), LensModeV1::Off);
        assert!(frame.datums().is_empty());
        let mut invalid = input(vec![datum(1, 1)]);
        invalid.mode = LensModeV1::Off;
        assert_eq!(
            LensFrameV1::seal(invalid),
            Err(LensErrorV1::ModeDatumMismatch)
        );
    }

    #[test]
    fn camera_selection_and_values_bind_identity() {
        let base = LensFrameV1::seal(input(vec![datum(1, 1)])).unwrap();
        let mut changed_camera = input(vec![datum(1, 1)]);
        changed_camera.camera_token = digest(4);
        let mut changed_selection = input(vec![datum(1, 1)]);
        changed_selection.selection_digest = digest(5);
        let mut changed_value = input(vec![datum(1, 1)]);
        changed_value.datums[0].values[2] += 1;
        for changed in [changed_camera, changed_selection, changed_value] {
            assert_ne!(
                LensFrameV1::seal(changed).unwrap().frame_digest(),
                base.frame_digest()
            );
        }
    }

    #[test]
    fn publication_is_idempotent_monotonic_and_rollback_capable() {
        let first = LensFrameV1::seal(input(vec![datum(1, 1)])).unwrap();
        let mut publication = LensPublicationV1::default();
        let held = publication.publish(first.clone()).unwrap();
        assert_eq!(
            publication.publish(first).unwrap().frame_digest(),
            held.frame_digest()
        );

        let mut next_input = input(vec![datum(2, 1)]);
        next_input.presentation_generation = 8;
        next_input.publication_sequence = 2;
        next_input.datums[0].authority_generation = 8;
        let next = LensFrameV1::seal(next_input).unwrap();
        publication.publish(next).unwrap();
        publication.rollback(8).unwrap();
        assert_eq!(
            publication.current().unwrap().frame_digest(),
            held.frame_digest()
        );
    }
}
