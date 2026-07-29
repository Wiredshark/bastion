//! Canonical, bounded indexed-indirect submission plans.
//!
//! The CPU direct loop remains the reference authority.  This module only
//! freezes that authority into exact indexed-indirect records and reconciles
//! independently observed records from the same presentation frame.

use crate::{
    domain_hash_v1,
    figure_batch::{DRAW_INDEXED_INDIRECT_BYTES_V1, FigurePassV1, indexed_indirect_bytes},
};

pub const DRAW_SUBMISSION_SCHEMA_VERSION_V1: u16 = 1;
pub const DRAW_SUBMISSION_MAX_DRAWS_V1: usize = 4_096;
const DRAW_SUBMISSION_MAGIC_V1: &[u8; 8] = b"BSTRIDP1";
const DRAW_SUBMISSION_HEADER_BYTES_V1: usize = 8 + 2 + 8 + 32 + 4;
const DRAW_SUBMISSION_RECORD_BYTES_V1: usize = 1 + 32 + DRAW_INDEXED_INDIRECT_BYTES_V1;

pub type SubmissionDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionTerminalV1 {
    IndirectAccepted,
    DirectFallback(SubmissionFallbackV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmissionFallbackV1 {
    ExplicitDirectReference,
    UnsupportedCapability,
    Overflow,
    StaleGeneration,
    InvalidRange,
    DeviceLoss,
    SubmissionFailure,
    Parity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmissionErrorV1 {
    InvalidGeneration,
    InvalidCullingDigest,
    DrawCountOutOfRange(usize),
    ZeroBatchIdentity,
    DuplicateBatchIdentity,
    EmptyRange,
    RangeOverflow,
    UnsupportedPass(u8),
    UnsupportedVersion(u16),
    Malformed,
    TrailingBytes,
    StaleGeneration { expected: u64, actual: u64 },
    StaleCullingDigest,
    ObservedCountMismatch { expected: usize, actual: usize },
    StructuralParity { index: usize },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DirectDrawReferenceV1 {
    pub pass: FigurePassV1,
    pub batch_identity: SubmissionDigestV1,
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DirectDrawReferenceV1 {
    pub fn new(
        pass: FigurePassV1,
        batch_identity: SubmissionDigestV1,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        base_vertex: i32,
        first_instance: u32,
    ) -> Result<Self, SubmissionErrorV1> {
        let value = Self {
            pass,
            batch_identity,
            index_count,
            instance_count,
            first_index,
            base_vertex,
            first_instance,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), SubmissionErrorV1> {
        if self.batch_identity.iter().all(|byte| *byte == 0) {
            return Err(SubmissionErrorV1::ZeroBatchIdentity);
        }
        if self.index_count == 0 || self.instance_count == 0 {
            return Err(SubmissionErrorV1::EmptyRange);
        }
        self.first_index
            .checked_add(self.index_count)
            .ok_or(SubmissionErrorV1::RangeOverflow)?;
        self.first_instance
            .checked_add(self.instance_count)
            .ok_or(SubmissionErrorV1::RangeOverflow)?;
        Ok(())
    }

    #[must_use]
    pub fn indirect_bytes(&self) -> [u8; DRAW_INDEXED_INDIRECT_BYTES_V1] {
        indexed_indirect_bytes(
            self.index_count,
            self.instance_count,
            self.first_index,
            self.base_vertex,
            self.first_instance,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndirectDrawObservedV1 {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl IndirectDrawObservedV1 {
    #[must_use]
    pub fn decode(bytes: &[u8; DRAW_INDEXED_INDIRECT_BYTES_V1]) -> Self {
        Self {
            index_count: u32::from_le_bytes(bytes[0..4].try_into().unwrap_or([0; 4])),
            instance_count: u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4])),
            first_index: u32::from_le_bytes(bytes[8..12].try_into().unwrap_or([0; 4])),
            base_vertex: i32::from_le_bytes(bytes[12..16].try_into().unwrap_or([0; 4])),
            first_instance: u32::from_le_bytes(bytes[16..20].try_into().unwrap_or([0; 4])),
        }
    }

    fn matches(self, reference: DirectDrawReferenceV1) -> bool {
        self.index_count == reference.index_count
            && self.instance_count == reference.instance_count
            && self.first_index == reference.first_index
            && self.base_vertex == reference.base_vertex
            && self.first_instance == reference.first_instance
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubmissionRecordV1 {
    pub reference: DirectDrawReferenceV1,
    pub direct_digest: SubmissionDigestV1,
    pub indirect_digest: SubmissionDigestV1,
    pub record_digest: SubmissionDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalSubmissionPlanV1 {
    pub generation: u64,
    pub culling_result_digest: SubmissionDigestV1,
    pub records: Vec<SubmissionRecordV1>,
    pub plan_digest: SubmissionDigestV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmissionParityV1 {
    pub generation: u64,
    pub culling_result_digest: SubmissionDigestV1,
    pub reference_draw_count: u32,
    pub indirect_draw_count: u32,
    pub reference_digest: SubmissionDigestV1,
    pub indirect_digest: SubmissionDigestV1,
    pub same_frame_parity: bool,
}

impl CanonicalSubmissionPlanV1 {
    pub fn build(
        generation: u64,
        culling_result_digest: SubmissionDigestV1,
        mut references: Vec<DirectDrawReferenceV1>,
    ) -> Result<Self, SubmissionErrorV1> {
        validate_identity(generation, culling_result_digest)?;
        if references.len() > DRAW_SUBMISSION_MAX_DRAWS_V1 {
            return Err(SubmissionErrorV1::DrawCountOutOfRange(references.len()));
        }
        for reference in &references {
            reference.validate()?;
        }
        references.sort_by_key(|reference| (reference.pass, reference.batch_identity));
        if references.windows(2).any(|pair| {
            pair[0].pass == pair[1].pass && pair[0].batch_identity == pair[1].batch_identity
        }) {
            return Err(SubmissionErrorV1::DuplicateBatchIdentity);
        }
        let records = references
            .into_iter()
            .map(record_for_reference)
            .collect::<Result<Vec<_>, _>>()?;
        let plan_digest = plan_digest(generation, culling_result_digest, &records)?;
        Ok(Self {
            generation,
            culling_result_digest,
            records,
            plan_digest,
        })
    }

    pub fn reconcile_same_frame(
        &self,
        expected_generation: u64,
        expected_culling_result_digest: SubmissionDigestV1,
        observed: &[[u8; DRAW_INDEXED_INDIRECT_BYTES_V1]],
    ) -> Result<SubmissionParityV1, SubmissionErrorV1> {
        if self.generation != expected_generation {
            return Err(SubmissionErrorV1::StaleGeneration {
                expected: expected_generation,
                actual: self.generation,
            });
        }
        if self.culling_result_digest != expected_culling_result_digest {
            return Err(SubmissionErrorV1::StaleCullingDigest);
        }
        if observed.len() != self.records.len() {
            return Err(SubmissionErrorV1::ObservedCountMismatch {
                expected: self.records.len(),
                actual: observed.len(),
            });
        }
        let mut observed_payload = Vec::with_capacity(
            observed
                .len()
                .checked_mul(DRAW_INDEXED_INDIRECT_BYTES_V1)
                .ok_or(SubmissionErrorV1::RangeOverflow)?,
        );
        for (index, (record, bytes)) in self.records.iter().zip(observed).enumerate() {
            if !IndirectDrawObservedV1::decode(bytes).matches(record.reference) {
                return Err(SubmissionErrorV1::StructuralParity { index });
            }
            observed_payload.extend_from_slice(bytes);
        }
        let indirect_digest = domain_hash_v1(
            "bastion/r2/draw-observed",
            DRAW_SUBMISSION_SCHEMA_VERSION_V1,
            0,
            &observed_payload,
        )
        .map_err(|_| SubmissionErrorV1::Malformed)?;
        let reference_digest = aggregate_reference_digest(&self.records)?;
        Ok(SubmissionParityV1 {
            generation: self.generation,
            culling_result_digest: self.culling_result_digest,
            reference_draw_count: u32::try_from(self.records.len())
                .map_err(|_| SubmissionErrorV1::RangeOverflow)?,
            indirect_draw_count: u32::try_from(observed.len())
                .map_err(|_| SubmissionErrorV1::RangeOverflow)?,
            reference_digest,
            indirect_digest,
            same_frame_parity: true,
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, SubmissionErrorV1> {
        let record_bytes = self
            .records
            .len()
            .checked_mul(DRAW_SUBMISSION_RECORD_BYTES_V1)
            .ok_or(SubmissionErrorV1::RangeOverflow)?;
        let mut bytes = Vec::with_capacity(
            DRAW_SUBMISSION_HEADER_BYTES_V1
                .checked_add(record_bytes)
                .ok_or(SubmissionErrorV1::RangeOverflow)?,
        );
        bytes.extend_from_slice(DRAW_SUBMISSION_MAGIC_V1);
        bytes.extend_from_slice(&DRAW_SUBMISSION_SCHEMA_VERSION_V1.to_le_bytes());
        bytes.extend_from_slice(&self.generation.to_le_bytes());
        bytes.extend_from_slice(&self.culling_result_digest);
        bytes.extend_from_slice(
            &u32::try_from(self.records.len())
                .map_err(|_| SubmissionErrorV1::RangeOverflow)?
                .to_le_bytes(),
        );
        for record in &self.records {
            bytes.push(pass_tag(record.reference.pass));
            bytes.extend_from_slice(&record.reference.batch_identity);
            bytes.extend_from_slice(&record.reference.indirect_bytes());
        }
        Ok(bytes)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, SubmissionErrorV1> {
        if bytes.len() < DRAW_SUBMISSION_HEADER_BYTES_V1
            || bytes.get(0..8) != Some(DRAW_SUBMISSION_MAGIC_V1.as_slice())
        {
            return Err(SubmissionErrorV1::Malformed);
        }
        let version = read_u16(bytes, 8)?;
        if version != DRAW_SUBMISSION_SCHEMA_VERSION_V1 {
            return Err(SubmissionErrorV1::UnsupportedVersion(version));
        }
        let generation = read_u64(bytes, 10)?;
        let culling_result_digest = read_digest(bytes, 18)?;
        let count =
            usize::try_from(read_u32(bytes, 50)?).map_err(|_| SubmissionErrorV1::RangeOverflow)?;
        if count > DRAW_SUBMISSION_MAX_DRAWS_V1 {
            return Err(SubmissionErrorV1::DrawCountOutOfRange(count));
        }
        let expected = DRAW_SUBMISSION_HEADER_BYTES_V1
            .checked_add(
                count
                    .checked_mul(DRAW_SUBMISSION_RECORD_BYTES_V1)
                    .ok_or(SubmissionErrorV1::RangeOverflow)?,
            )
            .ok_or(SubmissionErrorV1::RangeOverflow)?;
        if bytes.len() < expected {
            return Err(SubmissionErrorV1::Malformed);
        }
        if bytes.len() != expected {
            return Err(SubmissionErrorV1::TrailingBytes);
        }
        let mut references = Vec::with_capacity(count);
        let mut cursor = DRAW_SUBMISSION_HEADER_BYTES_V1;
        for _ in 0..count {
            let pass = pass_from_tag(*bytes.get(cursor).ok_or(SubmissionErrorV1::Malformed)?)?;
            cursor += 1;
            let batch_identity = read_digest(bytes, cursor)?;
            cursor += 32;
            let end = cursor
                .checked_add(DRAW_INDEXED_INDIRECT_BYTES_V1)
                .ok_or(SubmissionErrorV1::RangeOverflow)?;
            let indirect: [u8; DRAW_INDEXED_INDIRECT_BYTES_V1] = bytes
                .get(cursor..end)
                .ok_or(SubmissionErrorV1::Malformed)?
                .try_into()
                .map_err(|_| SubmissionErrorV1::Malformed)?;
            cursor = end;
            let observed = IndirectDrawObservedV1::decode(&indirect);
            references.push(DirectDrawReferenceV1::new(
                pass,
                batch_identity,
                observed.index_count,
                observed.instance_count,
                observed.first_index,
                observed.base_vertex,
                observed.first_instance,
            )?);
        }
        Self::build(generation, culling_result_digest, references)
    }
}

fn validate_identity(
    generation: u64,
    culling_result_digest: SubmissionDigestV1,
) -> Result<(), SubmissionErrorV1> {
    if generation == 0 {
        return Err(SubmissionErrorV1::InvalidGeneration);
    }
    if culling_result_digest.iter().all(|byte| *byte == 0) {
        return Err(SubmissionErrorV1::InvalidCullingDigest);
    }
    Ok(())
}

fn record_for_reference(
    reference: DirectDrawReferenceV1,
) -> Result<SubmissionRecordV1, SubmissionErrorV1> {
    let direct_payload = direct_payload(reference);
    let indirect_bytes = reference.indirect_bytes();
    let direct_digest = domain_hash_v1(
        "bastion/r2/draw-direct",
        DRAW_SUBMISSION_SCHEMA_VERSION_V1,
        0,
        &direct_payload,
    )
    .map_err(|_| SubmissionErrorV1::Malformed)?;
    let indirect_digest = domain_hash_v1(
        "bastion/r2/draw-indirect",
        DRAW_SUBMISSION_SCHEMA_VERSION_V1,
        0,
        &indirect_bytes,
    )
    .map_err(|_| SubmissionErrorV1::Malformed)?;
    let mut record_payload = Vec::with_capacity(32 + 32 + 32 + 1);
    record_payload.push(pass_tag(reference.pass));
    record_payload.extend_from_slice(&reference.batch_identity);
    record_payload.extend_from_slice(&direct_digest);
    record_payload.extend_from_slice(&indirect_digest);
    let record_digest = domain_hash_v1(
        "bastion/r2/draw-record",
        DRAW_SUBMISSION_SCHEMA_VERSION_V1,
        0,
        &record_payload,
    )
    .map_err(|_| SubmissionErrorV1::Malformed)?;
    Ok(SubmissionRecordV1 {
        reference,
        direct_digest,
        indirect_digest,
        record_digest,
    })
}

fn direct_payload(reference: DirectDrawReferenceV1) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1 + 32 + DRAW_INDEXED_INDIRECT_BYTES_V1);
    bytes.push(pass_tag(reference.pass));
    bytes.extend_from_slice(&reference.batch_identity);
    bytes.extend_from_slice(&reference.index_count.to_le_bytes());
    bytes.extend_from_slice(&reference.instance_count.to_le_bytes());
    bytes.extend_from_slice(&reference.first_index.to_le_bytes());
    bytes.extend_from_slice(&reference.base_vertex.to_le_bytes());
    bytes.extend_from_slice(&reference.first_instance.to_le_bytes());
    bytes
}

fn aggregate_reference_digest(
    records: &[SubmissionRecordV1],
) -> Result<SubmissionDigestV1, SubmissionErrorV1> {
    let mut payload = Vec::with_capacity(
        records
            .len()
            .checked_mul(32)
            .ok_or(SubmissionErrorV1::RangeOverflow)?,
    );
    for record in records {
        payload.extend_from_slice(&record.record_digest);
    }
    domain_hash_v1(
        "bastion/r2/draw-reference-set",
        DRAW_SUBMISSION_SCHEMA_VERSION_V1,
        0,
        &payload,
    )
    .map_err(|_| SubmissionErrorV1::Malformed)
}

fn plan_digest(
    generation: u64,
    culling_result_digest: SubmissionDigestV1,
    records: &[SubmissionRecordV1],
) -> Result<SubmissionDigestV1, SubmissionErrorV1> {
    let mut payload = Vec::with_capacity(
        8_usize
            .checked_add(32)
            .and_then(|value| value.checked_add(records.len().saturating_mul(32)))
            .ok_or(SubmissionErrorV1::RangeOverflow)?,
    );
    payload.extend_from_slice(&generation.to_le_bytes());
    payload.extend_from_slice(&culling_result_digest);
    for record in records {
        payload.extend_from_slice(&record.record_digest);
    }
    domain_hash_v1(
        "bastion/r2/draw-plan",
        DRAW_SUBMISSION_SCHEMA_VERSION_V1,
        0,
        &payload,
    )
    .map_err(|_| SubmissionErrorV1::Malformed)
}

const fn pass_tag(pass: FigurePassV1) -> u8 {
    match pass {
        FigurePassV1::Main => 1,
        FigurePassV1::Shadow => 2,
        FigurePassV1::Rain => 3,
    }
}

fn pass_from_tag(tag: u8) -> Result<FigurePassV1, SubmissionErrorV1> {
    match tag {
        1 => Ok(FigurePassV1::Main),
        2 => Ok(FigurePassV1::Shadow),
        3 => Ok(FigurePassV1::Rain),
        _ => Err(SubmissionErrorV1::UnsupportedPass(tag)),
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, SubmissionErrorV1> {
    let end = offset
        .checked_add(2)
        .ok_or(SubmissionErrorV1::RangeOverflow)?;
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(SubmissionErrorV1::Malformed)?
            .try_into()
            .map_err(|_| SubmissionErrorV1::Malformed)?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, SubmissionErrorV1> {
    let end = offset
        .checked_add(4)
        .ok_or(SubmissionErrorV1::RangeOverflow)?;
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(SubmissionErrorV1::Malformed)?
            .try_into()
            .map_err(|_| SubmissionErrorV1::Malformed)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, SubmissionErrorV1> {
    let end = offset
        .checked_add(8)
        .ok_or(SubmissionErrorV1::RangeOverflow)?;
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(SubmissionErrorV1::Malformed)?
            .try_into()
            .map_err(|_| SubmissionErrorV1::Malformed)?,
    ))
}

fn read_digest(bytes: &[u8], offset: usize) -> Result<SubmissionDigestV1, SubmissionErrorV1> {
    let end = offset
        .checked_add(32)
        .ok_or(SubmissionErrorV1::RangeOverflow)?;
    bytes
        .get(offset..end)
        .ok_or(SubmissionErrorV1::Malformed)?
        .try_into()
        .map_err(|_| SubmissionErrorV1::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] { [byte; 32] }

    fn reference(byte: u8, first_instance: u32) -> DirectDrawReferenceV1 {
        DirectDrawReferenceV1::new(
            FigurePassV1::Main,
            digest(byte),
            36,
            4,
            2,
            -3,
            first_instance,
        )
        .unwrap()
    }

    #[test]
    fn canonical_plan_is_permutation_invariant_and_exact_eof() {
        let expected =
            CanonicalSubmissionPlanV1::build(7, digest(9), vec![reference(2, 4), reference(1, 0)])
                .unwrap();
        let permuted =
            CanonicalSubmissionPlanV1::build(7, digest(9), vec![reference(1, 0), reference(2, 4)])
                .unwrap();
        assert_eq!(expected, permuted);
        let bytes = expected.canonical_bytes().unwrap();
        assert_eq!(
            CanonicalSubmissionPlanV1::decode_exact(&bytes).unwrap(),
            expected
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            CanonicalSubmissionPlanV1::decode_exact(&trailing),
            Err(SubmissionErrorV1::TrailingBytes)
        );
    }

    #[test]
    fn same_frame_parity_is_independently_decoded_and_catches_mismatch() {
        let plan = CanonicalSubmissionPlanV1::build(7, digest(9), vec![reference(1, 0)]).unwrap();
        let observed = [plan.records[0].reference.indirect_bytes()];
        let parity = plan.reconcile_same_frame(7, digest(9), &observed).unwrap();
        assert!(parity.same_frame_parity);
        assert_eq!(parity.reference_draw_count, 1);
        assert_eq!(parity.indirect_draw_count, 1);
        let mut corrupt = observed;
        corrupt[0][0] ^= 1;
        assert_eq!(
            plan.reconcile_same_frame(7, digest(9), &corrupt),
            Err(SubmissionErrorV1::StructuralParity { index: 0 })
        );
    }

    #[test]
    fn empty_full_bounds_stale_and_duplicate_fail_closed() {
        let empty = CanonicalSubmissionPlanV1::build(7, digest(9), Vec::new()).unwrap();
        assert!(empty.records.is_empty());
        assert_eq!(
            empty.reconcile_same_frame(8, digest(9), &[]),
            Err(SubmissionErrorV1::StaleGeneration {
                expected: 8,
                actual: 7
            })
        );
        assert_eq!(
            empty.reconcile_same_frame(7, digest(8), &[]),
            Err(SubmissionErrorV1::StaleCullingDigest)
        );
        assert_eq!(
            CanonicalSubmissionPlanV1::build(7, digest(9), vec![reference(1, 0), reference(1, 4)]),
            Err(SubmissionErrorV1::DuplicateBatchIdentity)
        );
        assert_eq!(
            DirectDrawReferenceV1::new(FigurePassV1::Main, digest(1), u32::MAX, 1, 1, 0, 0),
            Err(SubmissionErrorV1::RangeOverflow)
        );
    }
}
