//! Bounded admission record for the production-proven R2 accelerator slice.
//!
//! This record admits only accelerators with independently bound structural
//! parity evidence. Optional mechanisms remain explicitly unavailable or
//! skipped until a valid production-world measurement and source seam exist.

use crate::{DomainHashErrorV1, domain_hash_v1};

pub const R2_ADMISSION_SCHEMA_VERSION_V1: u16 = 1;
pub const R2_SELECTED_ACCELERATOR_COUNT_V1: usize = 2;
pub const R2_OPTIONAL_LANE_COUNT_V1: usize = 5;
pub const R2_EVIDENCE_BINDING_COUNT_V1: usize = 2;
const MAGIC_V1: &[u8; 8] = b"BSTR2AD1";
const COMMIT_BYTES: usize = 20;
const DIGEST_BYTES: usize = 32;
const HEADER_BYTES: usize = 8 + 2 + COMMIT_BYTES + COMMIT_BYTES;
const ACCELERATOR_BYTES: usize = 1 + 1;
const OPTIONAL_BYTES: usize = 1 + 1 + 1 + 1 + 32 + 1;
const EVIDENCE_BYTES: usize = 1 + COMMIT_BYTES + 32 + 32;
const CANONICAL_BYTES: usize = HEADER_BYTES
    + R2_SELECTED_ACCELERATOR_COUNT_V1 * ACCELERATOR_BYTES
    + R2_OPTIONAL_LANE_COUNT_V1 * OPTIONAL_BYTES
    + R2_EVIDENCE_BINDING_COUNT_V1 * EVIDENCE_BYTES;

pub type R2AdmissionDigestV1 = [u8; DIGEST_BYTES];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum SelectedAcceleratorV1 {
    GpuFrustumCulling = 1,
    IndexedIndirectSubmission = 2,
}

impl SelectedAcceleratorV1 {
    fn from_tag(tag: u8) -> Result<Self, R2AdmissionErrorV1> {
        match tag {
            1 => Ok(Self::GpuFrustumCulling),
            2 => Ok(Self::IndexedIndirectSubmission),
            other => Err(R2AdmissionErrorV1::UnknownAccelerator(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum OptionalLaneV1 {
    ComputePose = 1,
    Meshlets = 2,
    GeometryStreaming = 3,
    SpecializedHeroGeometry = 4,
    AdvancedVolumetrics = 5,
}

impl OptionalLaneV1 {
    fn from_tag(tag: u8) -> Result<Self, R2AdmissionErrorV1> {
        match tag {
            1 => Ok(Self::ComputePose),
            2 => Ok(Self::Meshlets),
            3 => Ok(Self::GeometryStreaming),
            4 => Ok(Self::SpecializedHeroGeometry),
            5 => Ok(Self::AdvancedVolumetrics),
            other => Err(R2AdmissionErrorV1::UnknownOptionalLane(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum OptionalLaneDispositionV1 {
    Selected = 1,
    SkippedNotMeasured = 2,
    Unavailable = 3,
}

impl OptionalLaneDispositionV1 {
    fn from_tag(tag: u8) -> Result<Self, R2AdmissionErrorV1> {
        match tag {
            1 => Ok(Self::Selected),
            2 => Ok(Self::SkippedNotMeasured),
            3 => Ok(Self::Unavailable),
            other => Err(R2AdmissionErrorV1::UnknownDisposition(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ReconsiderationTriggerV1 {
    ValidComputePoseBottleneck = 1,
    MeshletPipelineAndMeasuredGeometryBottleneck = 2,
    ValidGeometryStreamingStall = 3,
    AuthoritativeHeroGeometryAndMeasuredBottleneck = 4,
    ValidVolumetricBottleneck = 5,
}

impl ReconsiderationTriggerV1 {
    fn from_tag(tag: u8) -> Result<Self, R2AdmissionErrorV1> {
        match tag {
            1 => Ok(Self::ValidComputePoseBottleneck),
            2 => Ok(Self::MeshletPipelineAndMeasuredGeometryBottleneck),
            3 => Ok(Self::ValidGeometryStreamingStall),
            4 => Ok(Self::AuthoritativeHeroGeometryAndMeasuredBottleneck),
            5 => Ok(Self::ValidVolumetricBottleneck),
            other => Err(R2AdmissionErrorV1::UnknownTrigger(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EvidenceKindV1 {
    GpuCullStructuralParity = 1,
    IndexedIndirectStructuralParity = 2,
}

impl EvidenceKindV1 {
    fn from_tag(tag: u8) -> Result<Self, R2AdmissionErrorV1> {
        match tag {
            1 => Ok(Self::GpuCullStructuralParity),
            2 => Ok(Self::IndexedIndirectStructuralParity),
            other => Err(R2AdmissionErrorV1::UnknownEvidenceKind(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceleratorAdmissionV1 {
    pub accelerator: SelectedAcceleratorV1,
    pub reference_path_callable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptionalDispositionRecordV1 {
    pub lane: OptionalLaneV1,
    pub disposition: OptionalLaneDispositionV1,
    pub source_seam_available: bool,
    pub valid_bottleneck_evidence: bool,
    pub source_evidence_digest: R2AdmissionDigestV1,
    pub reconsideration_trigger: ReconsiderationTriggerV1,
}

impl OptionalDispositionRecordV1 {
    pub fn new(
        lane: OptionalLaneV1,
        disposition: OptionalLaneDispositionV1,
        source_seam_available: bool,
        valid_bottleneck_evidence: bool,
        source_evidence_digest: R2AdmissionDigestV1,
        reconsideration_trigger: ReconsiderationTriggerV1,
    ) -> Result<Self, R2AdmissionErrorV1> {
        let record = Self {
            lane,
            disposition,
            source_seam_available,
            valid_bottleneck_evidence,
            source_evidence_digest,
            reconsideration_trigger,
        };
        record.validate()?;
        Ok(record)
    }

    fn validate(&self) -> Result<(), R2AdmissionErrorV1> {
        if self.source_evidence_digest == [0; DIGEST_BYTES] {
            return Err(R2AdmissionErrorV1::ZeroDigest);
        }
        let valid = match self.disposition {
            OptionalLaneDispositionV1::Selected => {
                self.source_seam_available && self.valid_bottleneck_evidence
            },
            OptionalLaneDispositionV1::SkippedNotMeasured => {
                self.source_seam_available && !self.valid_bottleneck_evidence
            },
            OptionalLaneDispositionV1::Unavailable => {
                !self.source_seam_available && !self.valid_bottleneck_evidence
            },
        };
        if !valid {
            return Err(R2AdmissionErrorV1::DispositionContradiction(self.lane));
        }
        if self.reconsideration_trigger as u8 != self.lane as u8 {
            return Err(R2AdmissionErrorV1::TriggerMismatch(self.lane));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceleratorEvidenceBindingV1 {
    pub kind: EvidenceKindV1,
    pub source_commit: [u8; COMMIT_BYTES],
    pub analysis_sha256: R2AdmissionDigestV1,
    pub archive_sha256: R2AdmissionDigestV1,
}

impl AcceleratorEvidenceBindingV1 {
    pub fn new(
        kind: EvidenceKindV1,
        source_commit: [u8; COMMIT_BYTES],
        analysis_sha256: R2AdmissionDigestV1,
        archive_sha256: R2AdmissionDigestV1,
    ) -> Result<Self, R2AdmissionErrorV1> {
        if source_commit == [0; COMMIT_BYTES]
            || analysis_sha256 == [0; DIGEST_BYTES]
            || archive_sha256 == [0; DIGEST_BYTES]
        {
            return Err(R2AdmissionErrorV1::ZeroDigest);
        }
        Ok(Self {
            kind,
            source_commit,
            analysis_sha256,
            archive_sha256,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvancedRendererAdmissionV1 {
    pub source_commit: [u8; COMMIT_BYTES],
    pub source_tree: [u8; COMMIT_BYTES],
    pub selected: [AcceleratorAdmissionV1; R2_SELECTED_ACCELERATOR_COUNT_V1],
    pub optional: [OptionalDispositionRecordV1; R2_OPTIONAL_LANE_COUNT_V1],
    pub evidence: [AcceleratorEvidenceBindingV1; R2_EVIDENCE_BINDING_COUNT_V1],
    pub admission_digest: R2AdmissionDigestV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceFallbackV1 {
    ExplicitReference,
    UnsupportedCapability,
    ParityFailure,
    DeviceLoss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeAdmissionTerminalV1 {
    Accelerated {
        gpu_cull: bool,
        indexed_indirect: bool,
    },
    ReferenceFallback(ReferenceFallbackV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeAdmissionInputV1 {
    pub request_acceleration: bool,
    pub gpu_cull_capability: bool,
    pub indirect_capability: bool,
    pub gpu_cull_parity: bool,
    pub indirect_parity: bool,
    pub device_healthy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum R2AdmissionErrorV1 {
    UnsupportedVersion(u16),
    Malformed,
    TrailingBytes(usize),
    ZeroSourceIdentity,
    ZeroDigest,
    DuplicateAccelerator,
    MissingAccelerator(SelectedAcceleratorV1),
    ReferencePathNotCallable(SelectedAcceleratorV1),
    DuplicateOptionalLane,
    MissingOptionalLane(OptionalLaneV1),
    DispositionContradiction(OptionalLaneV1),
    TriggerMismatch(OptionalLaneV1),
    DuplicateEvidence,
    MissingEvidence(EvidenceKindV1),
    UnknownAccelerator(u8),
    UnknownOptionalLane(u8),
    UnknownDisposition(u8),
    UnknownTrigger(u8),
    UnknownEvidenceKind(u8),
    InvalidBool(u8),
    StaleSource,
    HashFailure(DomainHashErrorV1),
}

impl AdvancedRendererAdmissionV1 {
    pub fn new(
        source_commit: [u8; COMMIT_BYTES],
        source_tree: [u8; COMMIT_BYTES],
        mut selected: [AcceleratorAdmissionV1; R2_SELECTED_ACCELERATOR_COUNT_V1],
        mut optional: [OptionalDispositionRecordV1; R2_OPTIONAL_LANE_COUNT_V1],
        mut evidence: [AcceleratorEvidenceBindingV1; R2_EVIDENCE_BINDING_COUNT_V1],
    ) -> Result<Self, R2AdmissionErrorV1> {
        if source_commit == [0; COMMIT_BYTES] || source_tree == [0; COMMIT_BYTES] {
            return Err(R2AdmissionErrorV1::ZeroSourceIdentity);
        }
        selected.sort_by_key(|record| record.accelerator);
        optional.sort_by_key(|record| record.lane);
        evidence.sort_by_key(|record| record.kind as u8);
        validate_selected(&selected)?;
        validate_optional(&optional)?;
        validate_evidence(&evidence)?;
        let mut value = Self {
            source_commit,
            source_tree,
            selected,
            optional,
            evidence,
            admission_digest: [0; DIGEST_BYTES],
        };
        value.admission_digest = domain_hash_v1(
            "bastion/r2/advanced-renderer-admission",
            1,
            0,
            &value.encode_without_digest(),
        )
        .map_err(R2AdmissionErrorV1::HashFailure)?;
        Ok(value)
    }

    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = self.encode_without_digest();
        bytes.extend_from_slice(&self.admission_digest);
        bytes
    }

    pub fn decode_exact(input: &[u8]) -> Result<Self, R2AdmissionErrorV1> {
        let expected = CANONICAL_BYTES + DIGEST_BYTES;
        if input.len() < expected {
            return Err(R2AdmissionErrorV1::Malformed);
        }
        if input.len() > expected {
            return Err(R2AdmissionErrorV1::TrailingBytes(input.len() - expected));
        }
        if &input[..8] != MAGIC_V1 {
            return Err(R2AdmissionErrorV1::Malformed);
        }
        let version = u16::from_le_bytes([input[8], input[9]]);
        if version != R2_ADMISSION_SCHEMA_VERSION_V1 {
            return Err(R2AdmissionErrorV1::UnsupportedVersion(version));
        }
        let mut cursor = 10;
        let source_commit = take_array::<COMMIT_BYTES>(input, &mut cursor)?;
        let source_tree = take_array::<COMMIT_BYTES>(input, &mut cursor)?;
        let mut selected = [AcceleratorAdmissionV1 {
            accelerator: SelectedAcceleratorV1::GpuFrustumCulling,
            reference_path_callable: false,
        }; R2_SELECTED_ACCELERATOR_COUNT_V1];
        for record in &mut selected {
            record.accelerator = SelectedAcceleratorV1::from_tag(take_u8(input, &mut cursor)?)?;
            record.reference_path_callable = take_bool(input, &mut cursor)?;
        }
        let placeholder_optional = OptionalDispositionRecordV1 {
            lane: OptionalLaneV1::ComputePose,
            disposition: OptionalLaneDispositionV1::Unavailable,
            source_seam_available: false,
            valid_bottleneck_evidence: false,
            source_evidence_digest: [1; DIGEST_BYTES],
            reconsideration_trigger: ReconsiderationTriggerV1::ValidComputePoseBottleneck,
        };
        let mut optional = [placeholder_optional; R2_OPTIONAL_LANE_COUNT_V1];
        for record in &mut optional {
            record.lane = OptionalLaneV1::from_tag(take_u8(input, &mut cursor)?)?;
            record.disposition = OptionalLaneDispositionV1::from_tag(take_u8(input, &mut cursor)?)?;
            record.source_seam_available = take_bool(input, &mut cursor)?;
            record.valid_bottleneck_evidence = take_bool(input, &mut cursor)?;
            record.source_evidence_digest = take_array::<DIGEST_BYTES>(input, &mut cursor)?;
            record.reconsideration_trigger =
                ReconsiderationTriggerV1::from_tag(take_u8(input, &mut cursor)?)?;
        }
        let placeholder_evidence = AcceleratorEvidenceBindingV1 {
            kind: EvidenceKindV1::GpuCullStructuralParity,
            source_commit: [1; COMMIT_BYTES],
            analysis_sha256: [1; DIGEST_BYTES],
            archive_sha256: [1; DIGEST_BYTES],
        };
        let mut evidence = [placeholder_evidence; R2_EVIDENCE_BINDING_COUNT_V1];
        for record in &mut evidence {
            record.kind = EvidenceKindV1::from_tag(take_u8(input, &mut cursor)?)?;
            record.source_commit = take_array::<COMMIT_BYTES>(input, &mut cursor)?;
            record.analysis_sha256 = take_array::<DIGEST_BYTES>(input, &mut cursor)?;
            record.archive_sha256 = take_array::<DIGEST_BYTES>(input, &mut cursor)?;
        }
        let declared_digest = take_array::<DIGEST_BYTES>(input, &mut cursor)?;
        let decoded = Self::new(source_commit, source_tree, selected, optional, evidence)?;
        if decoded.admission_digest != declared_digest {
            return Err(R2AdmissionErrorV1::Malformed);
        }
        Ok(decoded)
    }

    pub fn runtime_terminal(
        &self,
        expected_source_commit: [u8; COMMIT_BYTES],
        input: RuntimeAdmissionInputV1,
    ) -> Result<RuntimeAdmissionTerminalV1, R2AdmissionErrorV1> {
        if self.source_commit != expected_source_commit {
            return Err(R2AdmissionErrorV1::StaleSource);
        }
        if !input.request_acceleration {
            return Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::ExplicitReference,
            ));
        }
        if !input.device_healthy {
            return Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::DeviceLoss,
            ));
        }
        if !input.gpu_cull_capability || !input.indirect_capability {
            return Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::UnsupportedCapability,
            ));
        }
        if !input.gpu_cull_parity || !input.indirect_parity {
            return Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::ParityFailure,
            ));
        }
        Ok(RuntimeAdmissionTerminalV1::Accelerated {
            gpu_cull: true,
            indexed_indirect: true,
        })
    }

    fn encode_without_digest(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(CANONICAL_BYTES);
        bytes.extend_from_slice(MAGIC_V1);
        bytes.extend_from_slice(&R2_ADMISSION_SCHEMA_VERSION_V1.to_le_bytes());
        bytes.extend_from_slice(&self.source_commit);
        bytes.extend_from_slice(&self.source_tree);
        for record in &self.selected {
            bytes.push(record.accelerator as u8);
            bytes.push(u8::from(record.reference_path_callable));
        }
        for record in &self.optional {
            bytes.push(record.lane as u8);
            bytes.push(record.disposition as u8);
            bytes.push(u8::from(record.source_seam_available));
            bytes.push(u8::from(record.valid_bottleneck_evidence));
            bytes.extend_from_slice(&record.source_evidence_digest);
            bytes.push(record.reconsideration_trigger as u8);
        }
        for record in &self.evidence {
            bytes.push(record.kind as u8);
            bytes.extend_from_slice(&record.source_commit);
            bytes.extend_from_slice(&record.analysis_sha256);
            bytes.extend_from_slice(&record.archive_sha256);
        }
        bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct R2AdmissionSlotV1 {
    current: AdvancedRendererAdmissionV1,
}

impl R2AdmissionSlotV1 {
    pub fn new(current: AdvancedRendererAdmissionV1) -> Self { Self { current } }

    pub fn current(&self) -> &AdvancedRendererAdmissionV1 { &self.current }

    pub fn publish(
        &mut self,
        expected_source_commit: [u8; COMMIT_BYTES],
        candidate: AdvancedRendererAdmissionV1,
    ) -> Result<(), R2AdmissionErrorV1> {
        if candidate.source_commit != expected_source_commit {
            return Err(R2AdmissionErrorV1::StaleSource);
        }
        self.current = candidate;
        Ok(())
    }
}

fn validate_selected(
    selected: &[AcceleratorAdmissionV1; R2_SELECTED_ACCELERATOR_COUNT_V1],
) -> Result<(), R2AdmissionErrorV1> {
    for required in [
        SelectedAcceleratorV1::GpuFrustumCulling,
        SelectedAcceleratorV1::IndexedIndirectSubmission,
    ] {
        let records: Vec<_> = selected
            .iter()
            .filter(|record| record.accelerator == required)
            .collect();
        match records.as_slice() {
            [] => return Err(R2AdmissionErrorV1::MissingAccelerator(required)),
            [record] if !record.reference_path_callable => {
                return Err(R2AdmissionErrorV1::ReferencePathNotCallable(required));
            },
            [_] => {},
            _ => return Err(R2AdmissionErrorV1::DuplicateAccelerator),
        }
    }
    Ok(())
}

fn validate_optional(
    optional: &[OptionalDispositionRecordV1; R2_OPTIONAL_LANE_COUNT_V1],
) -> Result<(), R2AdmissionErrorV1> {
    for required in [
        OptionalLaneV1::ComputePose,
        OptionalLaneV1::Meshlets,
        OptionalLaneV1::GeometryStreaming,
        OptionalLaneV1::SpecializedHeroGeometry,
        OptionalLaneV1::AdvancedVolumetrics,
    ] {
        let records: Vec<_> = optional
            .iter()
            .filter(|record| record.lane == required)
            .collect();
        match records.as_slice() {
            [] => return Err(R2AdmissionErrorV1::MissingOptionalLane(required)),
            [record] => record.validate()?,
            _ => return Err(R2AdmissionErrorV1::DuplicateOptionalLane),
        }
    }
    Ok(())
}

fn validate_evidence(
    evidence: &[AcceleratorEvidenceBindingV1; R2_EVIDENCE_BINDING_COUNT_V1],
) -> Result<(), R2AdmissionErrorV1> {
    for required in [
        EvidenceKindV1::GpuCullStructuralParity,
        EvidenceKindV1::IndexedIndirectStructuralParity,
    ] {
        let records: Vec<_> = evidence
            .iter()
            .filter(|record| record.kind == required)
            .collect();
        match records.as_slice() {
            [] => return Err(R2AdmissionErrorV1::MissingEvidence(required)),
            [record] => {
                if record.source_commit == [0; COMMIT_BYTES]
                    || record.analysis_sha256 == [0; DIGEST_BYTES]
                    || record.archive_sha256 == [0; DIGEST_BYTES]
                {
                    return Err(R2AdmissionErrorV1::ZeroDigest);
                }
            },
            _ => return Err(R2AdmissionErrorV1::DuplicateEvidence),
        }
    }
    Ok(())
}

fn take_u8(input: &[u8], cursor: &mut usize) -> Result<u8, R2AdmissionErrorV1> {
    let byte = *input.get(*cursor).ok_or(R2AdmissionErrorV1::Malformed)?;
    *cursor += 1;
    Ok(byte)
}

fn take_bool(input: &[u8], cursor: &mut usize) -> Result<bool, R2AdmissionErrorV1> {
    match take_u8(input, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(R2AdmissionErrorV1::InvalidBool(other)),
    }
}

fn take_array<const N: usize>(
    input: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], R2AdmissionErrorV1> {
    let end = cursor.checked_add(N).ok_or(R2AdmissionErrorV1::Malformed)?;
    let slice = input
        .get(*cursor..end)
        .ok_or(R2AdmissionErrorV1::Malformed)?;
    let value = slice
        .try_into()
        .map_err(|_| R2AdmissionErrorV1::Malformed)?;
    *cursor = end;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(byte: u8) -> [u8; COMMIT_BYTES] { [byte; COMMIT_BYTES] }

    fn digest(byte: u8) -> [u8; DIGEST_BYTES] { [byte; DIGEST_BYTES] }

    fn hex<const N: usize>(value: &str) -> [u8; N] {
        assert_eq!(value.len(), N * 2);
        let mut bytes = [0_u8; N];
        for (output, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            let nibble = |byte| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => panic!("test vector is lowercase ASCII hex"),
            };
            *output = (nibble(pair[0]) << 4) | nibble(pair[1]);
        }
        bytes
    }

    fn optional(
        lane: OptionalLaneV1,
        disposition: OptionalLaneDispositionV1,
        source: bool,
        measured: bool,
    ) -> OptionalDispositionRecordV1 {
        OptionalDispositionRecordV1::new(
            lane,
            disposition,
            source,
            measured,
            digest(20 + lane as u8),
            match lane {
                OptionalLaneV1::ComputePose => ReconsiderationTriggerV1::ValidComputePoseBottleneck,
                OptionalLaneV1::Meshlets => {
                    ReconsiderationTriggerV1::MeshletPipelineAndMeasuredGeometryBottleneck
                },
                OptionalLaneV1::GeometryStreaming => {
                    ReconsiderationTriggerV1::ValidGeometryStreamingStall
                },
                OptionalLaneV1::SpecializedHeroGeometry => {
                    ReconsiderationTriggerV1::AuthoritativeHeroGeometryAndMeasuredBottleneck
                },
                OptionalLaneV1::AdvancedVolumetrics => {
                    ReconsiderationTriggerV1::ValidVolumetricBottleneck
                },
            },
        )
        .unwrap()
    }

    fn admission() -> AdvancedRendererAdmissionV1 {
        AdvancedRendererAdmissionV1::new(
            commit(1),
            commit(2),
            [
                AcceleratorAdmissionV1 {
                    accelerator: SelectedAcceleratorV1::IndexedIndirectSubmission,
                    reference_path_callable: true,
                },
                AcceleratorAdmissionV1 {
                    accelerator: SelectedAcceleratorV1::GpuFrustumCulling,
                    reference_path_callable: true,
                },
            ],
            [
                optional(
                    OptionalLaneV1::AdvancedVolumetrics,
                    OptionalLaneDispositionV1::SkippedNotMeasured,
                    true,
                    false,
                ),
                optional(
                    OptionalLaneV1::SpecializedHeroGeometry,
                    OptionalLaneDispositionV1::Unavailable,
                    false,
                    false,
                ),
                optional(
                    OptionalLaneV1::GeometryStreaming,
                    OptionalLaneDispositionV1::Unavailable,
                    false,
                    false,
                ),
                optional(
                    OptionalLaneV1::Meshlets,
                    OptionalLaneDispositionV1::Unavailable,
                    false,
                    false,
                ),
                optional(
                    OptionalLaneV1::ComputePose,
                    OptionalLaneDispositionV1::SkippedNotMeasured,
                    true,
                    false,
                ),
            ],
            [
                AcceleratorEvidenceBindingV1::new(
                    EvidenceKindV1::IndexedIndirectStructuralParity,
                    commit(1),
                    digest(4),
                    digest(5),
                )
                .unwrap(),
                AcceleratorEvidenceBindingV1::new(
                    EvidenceKindV1::GpuCullStructuralParity,
                    commit(1),
                    digest(6),
                    digest(7),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    #[test]
    fn canonical_record_orders_entries_round_trips_and_rejects_trailing_bytes() {
        let value = admission();
        assert_eq!(
            value.selected[0].accelerator,
            SelectedAcceleratorV1::GpuFrustumCulling
        );
        assert_eq!(value.optional[0].lane, OptionalLaneV1::ComputePose);
        let bytes = value.canonical_bytes();
        assert_eq!(
            AdvancedRendererAdmissionV1::decode_exact(&bytes).unwrap(),
            value
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            AdvancedRendererAdmissionV1::decode_exact(&trailing),
            Err(R2AdmissionErrorV1::TrailingBytes(1))
        );
    }

    #[test]
    fn optional_dispositions_require_real_seam_and_valid_measurement() {
        assert_eq!(
            OptionalDispositionRecordV1::new(
                OptionalLaneV1::Meshlets,
                OptionalLaneDispositionV1::Selected,
                false,
                false,
                digest(1),
                ReconsiderationTriggerV1::MeshletPipelineAndMeasuredGeometryBottleneck,
            ),
            Err(R2AdmissionErrorV1::DispositionContradiction(
                OptionalLaneV1::Meshlets
            ))
        );
        assert_eq!(
            OptionalDispositionRecordV1::new(
                OptionalLaneV1::ComputePose,
                OptionalLaneDispositionV1::SkippedNotMeasured,
                true,
                false,
                digest(1),
                ReconsiderationTriggerV1::ValidComputePoseBottleneck,
            )
            .unwrap()
            .disposition,
            OptionalLaneDispositionV1::SkippedNotMeasured
        );
    }

    #[test]
    fn runtime_default_accelerated_faults_and_stale_source_are_typed() {
        let value = admission();
        let base = RuntimeAdmissionInputV1 {
            request_acceleration: true,
            gpu_cull_capability: true,
            indirect_capability: true,
            gpu_cull_parity: true,
            indirect_parity: true,
            device_healthy: true,
        };
        assert_eq!(
            value.runtime_terminal(commit(1), RuntimeAdmissionInputV1 {
                request_acceleration: false,
                ..base
            }),
            Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::ExplicitReference
            ))
        );
        assert_eq!(
            value.runtime_terminal(commit(1), base),
            Ok(RuntimeAdmissionTerminalV1::Accelerated {
                gpu_cull: true,
                indexed_indirect: true
            })
        );
        assert_eq!(
            value.runtime_terminal(commit(1), RuntimeAdmissionInputV1 {
                indirect_capability: false,
                ..base
            }),
            Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::UnsupportedCapability
            ))
        );
        assert_eq!(
            value.runtime_terminal(commit(1), RuntimeAdmissionInputV1 {
                gpu_cull_parity: false,
                ..base
            }),
            Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::ParityFailure
            ))
        );
        assert_eq!(
            value.runtime_terminal(commit(1), RuntimeAdmissionInputV1 {
                device_healthy: false,
                ..base
            }),
            Ok(RuntimeAdmissionTerminalV1::ReferenceFallback(
                ReferenceFallbackV1::DeviceLoss
            ))
        );
        assert_eq!(
            value.runtime_terminal(commit(9), base),
            Err(R2AdmissionErrorV1::StaleSource)
        );
    }

    #[test]
    fn rejected_publication_preserves_previous_admission() {
        let current = admission();
        let mut slot = R2AdmissionSlotV1::new(current.clone());
        let mut stale = current.clone();
        stale.source_commit = commit(9);
        assert_eq!(
            slot.publish(commit(1), stale),
            Err(R2AdmissionErrorV1::StaleSource)
        );
        assert_eq!(slot.current(), &current);
    }

    #[test]
    fn corrupted_canonical_digest_and_invalid_bool_fail_closed() {
        let value = admission();
        let mut corrupt = value.canonical_bytes();
        let final_byte = corrupt.len() - 1;
        corrupt[final_byte] ^= 1;
        assert_eq!(
            AdvancedRendererAdmissionV1::decode_exact(&corrupt),
            Err(R2AdmissionErrorV1::Malformed)
        );
        let mut invalid_bool = value.canonical_bytes();
        invalid_bool[HEADER_BYTES + 1] = 2;
        assert_eq!(
            AdvancedRendererAdmissionV1::decode_exact(&invalid_bool),
            Err(R2AdmissionErrorV1::InvalidBool(2))
        );
    }

    #[test]
    fn production_admission_binds_exact_accepted_accelerator_evidence() {
        let source_commit = hex::<20>("25bd1a4d206281884a6035fb357d3e026b0e4f28");
        let value = AdvancedRendererAdmissionV1::new(
            source_commit,
            hex::<20>("7635f246c495ffa88f3440ed70e6d40c50e3c34e"),
            [
                AcceleratorAdmissionV1 {
                    accelerator: SelectedAcceleratorV1::GpuFrustumCulling,
                    reference_path_callable: true,
                },
                AcceleratorAdmissionV1 {
                    accelerator: SelectedAcceleratorV1::IndexedIndirectSubmission,
                    reference_path_callable: true,
                },
            ],
            [
                OptionalDispositionRecordV1::new(
                    OptionalLaneV1::ComputePose,
                    OptionalLaneDispositionV1::SkippedNotMeasured,
                    true,
                    false,
                    hex::<32>("3e707382ce1fc5a7466a4a17d2847ea942af2e3ffe222340a23473cb691605c5"),
                    ReconsiderationTriggerV1::ValidComputePoseBottleneck,
                )
                .unwrap(),
                OptionalDispositionRecordV1::new(
                    OptionalLaneV1::Meshlets,
                    OptionalLaneDispositionV1::Unavailable,
                    false,
                    false,
                    hex::<32>("b9d759571b6197ac4648f30fae89dfe46b9a50c6929373d1fcb188d441cc8127"),
                    ReconsiderationTriggerV1::MeshletPipelineAndMeasuredGeometryBottleneck,
                )
                .unwrap(),
                OptionalDispositionRecordV1::new(
                    OptionalLaneV1::GeometryStreaming,
                    OptionalLaneDispositionV1::SkippedNotMeasured,
                    true,
                    false,
                    hex::<32>("4ab3d76d42c6fe70f45ae3e90046f0f541d24887e6c02062035b4f350d9409b8"),
                    ReconsiderationTriggerV1::ValidGeometryStreamingStall,
                )
                .unwrap(),
                OptionalDispositionRecordV1::new(
                    OptionalLaneV1::SpecializedHeroGeometry,
                    OptionalLaneDispositionV1::Unavailable,
                    false,
                    false,
                    hex::<32>("3e707382ce1fc5a7466a4a17d2847ea942af2e3ffe222340a23473cb691605c5"),
                    ReconsiderationTriggerV1::AuthoritativeHeroGeometryAndMeasuredBottleneck,
                )
                .unwrap(),
                OptionalDispositionRecordV1::new(
                    OptionalLaneV1::AdvancedVolumetrics,
                    OptionalLaneDispositionV1::SkippedNotMeasured,
                    true,
                    false,
                    hex::<32>("58651f666769ca63ef66de829fe6fda62bd6dac40a157833884edd2dac534900"),
                    ReconsiderationTriggerV1::ValidVolumetricBottleneck,
                )
                .unwrap(),
            ],
            [
                AcceleratorEvidenceBindingV1::new(
                    EvidenceKindV1::GpuCullStructuralParity,
                    hex::<20>("d18141418230e15153f6524a5e0ae79f9e8ebb8b"),
                    hex::<32>("acf8c146b630b50e6b5aaa81c43cf1db75bb7724bc9067fa993c74a16ff09881"),
                    hex::<32>("1dcb461a59829ee42b63ddd0e8dc5a91750f9ebd995157a05292a16a7a19632f"),
                )
                .unwrap(),
                AcceleratorEvidenceBindingV1::new(
                    EvidenceKindV1::IndexedIndirectStructuralParity,
                    source_commit,
                    hex::<32>("9f96282d9e9f779d50f3f9dbd88742b56d17f5189dd25067068b22c2cf97baea"),
                    hex::<32>("08f4dfc461d342a3f035b246199deca55c29a200aabc6755f016b7adde86f072"),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(value.source_commit, source_commit);
        assert_eq!(
            value.evidence[0].source_commit,
            hex::<20>("d18141418230e15153f6524a5e0ae79f9e8ebb8b")
        );
        assert_eq!(
            AdvancedRendererAdmissionV1::decode_exact(&value.canonical_bytes()).unwrap(),
            value
        );
    }
}
