//! Renderer-owned, versioned admission parsing and validation primitives.

use core::fmt;

/// Admission envelope marker and protocol version.
pub const RENDERER_ADMISSION_V1_VERSION: u16 = 1;
/// Source-epoch protocol marker and version.
pub const RENDERER_SOURCE_EPOCH_V1_VERSION: u16 = 1;
/// Maximum allowed bytes for a serialized corpus input digest payload.
pub const MAX_CORPUS_INPUT_BYTES_V1: u64 = 1_048_576;
/// Maximum number of corpus inputs in a V1 admission blob.
pub const MAX_CORPUS_INPUTS_V1: usize = 16;

const MAGIC: [u8; 8] = *b"R0D-ADM1";

const COMMIT_BYTES_LEN: usize = 20;
const DIGEST_BYTES_LEN: usize = 32;
const DIGESTS_PER_EPOCH: usize = 5;
const DIGEST_COUNT: usize = DIGESTS_PER_EPOCH;
const SOURCE_EPOCH_BYTES: usize = COMMIT_BYTES_LEN + DIGEST_BYTES_LEN * DIGEST_COUNT;

/// Admission serialization layout:
///
/// * bytes 0..8: magic
/// * bytes 8..10: admission schema version (u16 LE)
/// * bytes 10..12: source epoch schema version (u16 LE)
/// * bytes 12..192: source epoch payload
/// * bytes 192..194: corpus input count (u16 LE)
/// * bytes 194..: N x corpus entries
const R0D_FIXED_HEADER_BYTES: usize = 8 + 2 + 2 + SOURCE_EPOCH_BYTES + 2;
const R0D_INPUT_COUNT_OFFSET: usize = R0D_FIXED_HEADER_BYTES - 2;
const R0D_CORPUS_ENTRY_BYTES: usize = 2 + DIGEST_BYTES_LEN + 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RendererCorpusRoleV1 {
    CanonicalRendererCorpus = 0,
    LivingWorldRedesign = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionErrorV1 {
    UnsupportedAdmissionVersion(u16),
    UnsupportedSourceEpochVersion(u16),
    InvalidSourceCommitLength(usize),
    InvalidHex,
    InvalidDigestLength(usize),
    InvalidCorpusSize(u64),
    TooManyCorpusInputs(usize),
    DuplicateRole(RendererCorpusRoleV1),
    MissingRequiredRole(RendererCorpusRoleV1),
    UnknownRole(u16),
    NonCanonicalRoleOrder,
    Truncated,
    TrailingBytes(usize),
    SourceEpochMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererSourceEpochV1 {
    version: u16,
    source_commit: [u8; COMMIT_BYTES_LEN],
    source_asset_digest_a: [u8; DIGEST_BYTES_LEN],
    source_asset_digest_b: [u8; DIGEST_BYTES_LEN],
    source_asset_digest_c: [u8; DIGEST_BYTES_LEN],
    source_asset_digest_d: [u8; DIGEST_BYTES_LEN],
    source_asset_digest_e: [u8; DIGEST_BYTES_LEN],
}

impl RendererSourceEpochV1 {
    pub const VERSION: u16 = RENDERER_SOURCE_EPOCH_V1_VERSION;

    pub fn from_hex(
        source_commit_hex: &str,
        source_asset_digest_a: [u8; DIGEST_BYTES_LEN],
        source_asset_digest_b: [u8; DIGEST_BYTES_LEN],
        source_asset_digest_c: [u8; DIGEST_BYTES_LEN],
        source_asset_digest_d: [u8; DIGEST_BYTES_LEN],
        source_asset_digest_e: [u8; DIGEST_BYTES_LEN],
    ) -> Result<Self, AdmissionErrorV1> {
        if source_commit_hex.len() != COMMIT_BYTES_LEN * 2 {
            return Err(AdmissionErrorV1::InvalidSourceCommitLength(
                source_commit_hex.len(),
            ));
        }

        let mut source_commit = [0_u8; COMMIT_BYTES_LEN];
        for i in 0..COMMIT_BYTES_LEN {
            let chunk = &source_commit_hex[i * 2..i * 2 + 2];
            let decoded =
                u8::from_str_radix(chunk, 16).map_err(|_| AdmissionErrorV1::InvalidHex)?;
            source_commit[i] = decoded;
        }

        Ok(Self {
            version: Self::VERSION,
            source_commit,
            source_asset_digest_a,
            source_asset_digest_b,
            source_asset_digest_c,
            source_asset_digest_d,
            source_asset_digest_e,
        })
    }

    pub fn encode_exact(&self) -> [u8; SOURCE_EPOCH_BYTES] {
        let mut out = [0_u8; SOURCE_EPOCH_BYTES];
        let mut start = 0;
        out[start..start + COMMIT_BYTES_LEN].copy_from_slice(&self.source_commit);
        start += COMMIT_BYTES_LEN;
        out[start..start + DIGEST_BYTES_LEN].copy_from_slice(&self.source_asset_digest_a);
        start += DIGEST_BYTES_LEN;
        out[start..start + DIGEST_BYTES_LEN].copy_from_slice(&self.source_asset_digest_b);
        start += DIGEST_BYTES_LEN;
        out[start..start + DIGEST_BYTES_LEN].copy_from_slice(&self.source_asset_digest_c);
        start += DIGEST_BYTES_LEN;
        out[start..start + DIGEST_BYTES_LEN].copy_from_slice(&self.source_asset_digest_d);
        start += DIGEST_BYTES_LEN;
        out[start..start + DIGEST_BYTES_LEN].copy_from_slice(&self.source_asset_digest_e);
        out
    }

    fn decode_exact(payload: &[u8]) -> Result<Self, AdmissionErrorV1> {
        if payload.len() < SOURCE_EPOCH_BYTES {
            return Err(AdmissionErrorV1::Truncated);
        }

        let mut start = 0;
        let mut source_commit = [0_u8; COMMIT_BYTES_LEN];
        source_commit.copy_from_slice(&payload[start..start + COMMIT_BYTES_LEN]);
        start += COMMIT_BYTES_LEN;

        let mut source_asset_digest_a = [0_u8; DIGEST_BYTES_LEN];
        source_asset_digest_a.copy_from_slice(&payload[start..start + DIGEST_BYTES_LEN]);
        start += DIGEST_BYTES_LEN;

        let mut source_asset_digest_b = [0_u8; DIGEST_BYTES_LEN];
        source_asset_digest_b.copy_from_slice(&payload[start..start + DIGEST_BYTES_LEN]);
        start += DIGEST_BYTES_LEN;

        let mut source_asset_digest_c = [0_u8; DIGEST_BYTES_LEN];
        source_asset_digest_c.copy_from_slice(&payload[start..start + DIGEST_BYTES_LEN]);
        start += DIGEST_BYTES_LEN;

        let mut source_asset_digest_d = [0_u8; DIGEST_BYTES_LEN];
        source_asset_digest_d.copy_from_slice(&payload[start..start + DIGEST_BYTES_LEN]);
        start += DIGEST_BYTES_LEN;

        let mut source_asset_digest_e = [0_u8; DIGEST_BYTES_LEN];
        source_asset_digest_e.copy_from_slice(&payload[start..start + DIGEST_BYTES_LEN]);

        Ok(Self {
            version: Self::VERSION,
            source_commit,
            source_asset_digest_a,
            source_asset_digest_b,
            source_asset_digest_c,
            source_asset_digest_d,
            source_asset_digest_e,
        })
    }
}

impl fmt::Display for RendererCorpusRoleV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::CanonicalRendererCorpus => "CanonicalRendererCorpus",
            Self::LivingWorldRedesign => "LivingWorldRedesign",
        };
        write!(f, "{text}")
    }
}

impl RendererCorpusRoleV1 {
    const REQUIRED: [Self; 2] = [Self::CanonicalRendererCorpus, Self::LivingWorldRedesign];

    const fn to_u16(self) -> u16 { self as u16 }

    fn from_u16(raw: u16) -> Result<Self, AdmissionErrorV1> {
        match raw {
            0 => Ok(Self::CanonicalRendererCorpus),
            1 => Ok(Self::LivingWorldRedesign),
            other => Err(AdmissionErrorV1::UnknownRole(other)),
        }
    }

    fn required_roles() -> &'static [Self] { &Self::REQUIRED }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererCorpusInputV1 {
    role: RendererCorpusRoleV1,
    digest: [u8; DIGEST_BYTES_LEN],
    corpus_size_bytes: u64,
}

impl RendererCorpusInputV1 {
    pub fn from_digest_slice(
        role: RendererCorpusRoleV1,
        digest: &[u8],
        corpus_size_bytes: u64,
    ) -> Result<Self, AdmissionErrorV1> {
        if digest.len() != DIGEST_BYTES_LEN {
            return Err(AdmissionErrorV1::InvalidDigestLength(digest.len()));
        }

        if corpus_size_bytes == 0 || corpus_size_bytes > MAX_CORPUS_INPUT_BYTES_V1 {
            return Err(AdmissionErrorV1::InvalidCorpusSize(corpus_size_bytes));
        }

        let mut encoded_digest = [0_u8; DIGEST_BYTES_LEN];
        encoded_digest.copy_from_slice(digest);

        Ok(Self {
            role,
            digest: encoded_digest,
            corpus_size_bytes,
        })
    }

    pub const fn role(&self) -> RendererCorpusRoleV1 { self.role }

    fn encode_exact(&self) -> [u8; R0D_CORPUS_ENTRY_BYTES] {
        let mut out = [0_u8; R0D_CORPUS_ENTRY_BYTES];
        out[0..2].copy_from_slice(&self.role.to_u16().to_le_bytes());
        out[2..34].copy_from_slice(&self.digest);
        out[34..42].copy_from_slice(&self.corpus_size_bytes.to_le_bytes());
        out
    }

    fn decode_exact(payload: &[u8]) -> Result<Self, AdmissionErrorV1> {
        if payload.len() != R0D_CORPUS_ENTRY_BYTES {
            return Err(AdmissionErrorV1::Truncated);
        }

        let role = RendererCorpusRoleV1::from_u16(u16::from_le_bytes([payload[0], payload[1]]))?;
        let mut digest = [0_u8; DIGEST_BYTES_LEN];
        digest.copy_from_slice(&payload[2..34]);
        let corpus_size_bytes =
            u64::from_le_bytes(payload[34..42].try_into().expect("fixed width"));

        if corpus_size_bytes == 0 || corpus_size_bytes > MAX_CORPUS_INPUT_BYTES_V1 {
            return Err(AdmissionErrorV1::InvalidCorpusSize(corpus_size_bytes));
        }

        Ok(Self {
            role,
            digest,
            corpus_size_bytes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererAdmissionV1 {
    source_epoch: RendererSourceEpochV1,
    corpus_inputs: Vec<RendererCorpusInputV1>,
}

impl RendererAdmissionV1 {
    pub const CORPUS_ENTRY_BYTES: usize = R0D_CORPUS_ENTRY_BYTES;
    pub const FIXED_HEADER_BYTES: usize = R0D_FIXED_HEADER_BYTES;
    pub const INPUT_COUNT_OFFSET: usize = R0D_INPUT_COUNT_OFFSET;

    fn required_roles_present(inputs: &[RendererCorpusInputV1]) -> Result<(), AdmissionErrorV1> {
        for required in RendererCorpusRoleV1::required_roles() {
            if !inputs.iter().any(|input| input.role == *required) {
                return Err(AdmissionErrorV1::MissingRequiredRole(*required));
            }
        }
        Ok(())
    }

    fn canonicalize_inputs(
        mut inputs: Vec<RendererCorpusInputV1>,
    ) -> Result<Vec<RendererCorpusInputV1>, AdmissionErrorV1> {
        inputs.sort_by_key(|a| a.role as u16);
        let mut seen = [false; 2];

        for input in &inputs {
            let idx = input.role as usize;
            if seen[idx] {
                return Err(AdmissionErrorV1::DuplicateRole(input.role));
            }
            seen[idx] = true;
        }

        Self::required_roles_present(&inputs)?;
        Ok(inputs)
    }

    pub fn new(
        source_epoch: RendererSourceEpochV1,
        corpus_inputs: Vec<RendererCorpusInputV1>,
    ) -> Result<Self, AdmissionErrorV1> {
        if corpus_inputs.len() > MAX_CORPUS_INPUTS_V1 {
            return Err(AdmissionErrorV1::TooManyCorpusInputs(corpus_inputs.len()));
        }

        let canonical_inputs = Self::canonicalize_inputs(corpus_inputs)?;

        Ok(Self {
            source_epoch,
            corpus_inputs: canonical_inputs,
        })
    }

    pub fn decode_exact(input: &[u8]) -> Result<Self, AdmissionErrorV1> {
        if input.len() < R0D_FIXED_HEADER_BYTES {
            return Err(AdmissionErrorV1::Truncated);
        }

        if &input[0..8] != MAGIC {
            return Err(AdmissionErrorV1::UnsupportedAdmissionVersion(0));
        }

        let admission_version = u16::from_le_bytes([input[8], input[9]]);
        if admission_version != RENDERER_ADMISSION_V1_VERSION {
            return Err(AdmissionErrorV1::UnsupportedAdmissionVersion(
                admission_version,
            ));
        }

        let source_epoch_version = u16::from_le_bytes([input[10], input[11]]);
        if source_epoch_version != RENDERER_SOURCE_EPOCH_V1_VERSION {
            return Err(AdmissionErrorV1::UnsupportedSourceEpochVersion(
                source_epoch_version,
            ));
        }

        let source_epoch = RendererSourceEpochV1::decode_exact(&input[12..192])?;
        let input_count = u16::from_le_bytes(
            input[R0D_INPUT_COUNT_OFFSET..R0D_INPUT_COUNT_OFFSET + 2]
                .try_into()
                .expect("fixed width"),
        ) as usize;

        if input_count > MAX_CORPUS_INPUTS_V1 {
            return Err(AdmissionErrorV1::TooManyCorpusInputs(input_count));
        }

        let expected_len = R0D_FIXED_HEADER_BYTES + input_count * R0D_CORPUS_ENTRY_BYTES;
        if input.len() < expected_len {
            return Err(AdmissionErrorV1::Truncated);
        }
        if input.len() > expected_len {
            return Err(AdmissionErrorV1::TrailingBytes(input.len() - expected_len));
        }

        let mut inputs = Vec::with_capacity(input_count);
        let mut i = 0;
        let mut previous_role: Option<u16> = None;

        while i < input_count {
            let start = R0D_FIXED_HEADER_BYTES + (i * R0D_CORPUS_ENTRY_BYTES);
            let end = start + R0D_CORPUS_ENTRY_BYTES;
            let bytes = &input[start..end];
            let role_raw = u16::from_le_bytes([bytes[0], bytes[1]]);
            let role = RendererCorpusRoleV1::from_u16(role_raw)?;

            if previous_role.is_some() && role_raw <= previous_role.expect("set") {
                return Err(AdmissionErrorV1::NonCanonicalRoleOrder);
            }
            previous_role = Some(role as u16);

            let input = RendererCorpusInputV1::decode_exact(bytes)?;
            if inputs
                .iter()
                .any(|existing: &RendererCorpusInputV1| existing.role == input.role)
            {
                return Err(AdmissionErrorV1::DuplicateRole(input.role));
            }
            inputs.push(input);
            i += 1;
        }

        Self::required_roles_present(&inputs)?;

        Ok(Self {
            source_epoch,
            corpus_inputs: inputs,
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, AdmissionErrorV1> {
        let mut out = Vec::with_capacity(
            R0D_FIXED_HEADER_BYTES + self.corpus_inputs.len() * R0D_CORPUS_ENTRY_BYTES,
        );
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&RENDERER_ADMISSION_V1_VERSION.to_le_bytes());
        out.extend_from_slice(&RENDERER_SOURCE_EPOCH_V1_VERSION.to_le_bytes());
        out.extend_from_slice(&self.source_epoch.encode_exact());
        out.extend_from_slice(&(self.corpus_inputs.len() as u16).to_le_bytes());

        for entry in &self.corpus_inputs {
            out.extend_from_slice(&entry.encode_exact());
        }

        Ok(out)
    }

    pub fn validate_against(
        &self,
        expected_source_epoch: &RendererSourceEpochV1,
    ) -> Result<(), AdmissionErrorV1> {
        if self.source_epoch != *expected_source_epoch {
            return Err(AdmissionErrorV1::SourceEpochMismatch);
        }
        Ok(())
    }

    pub fn corpus_inputs(&self) -> &[RendererCorpusInputV1] { &self.corpus_inputs }

    pub fn source_epoch(&self) -> &RendererSourceEpochV1 { &self.source_epoch }
}
