//! Deterministic renderer-owned material classification and response table.
//!
//! V1 deliberately describes only material semantics already declared by an
//! accepted package.  The table is canonical authority for renderer policy;
//! the legacy atlas/shader path remains the pixel-producing compatibility
//! path until a later environment packet intentionally changes visual output.

use std::{collections::BTreeSet, sync::Arc};

use crate::domain_hash_v1;

pub const MATERIAL_TABLE_SCHEMA_V1: u16 = 1;
pub const MATERIAL_COMPILER_V1: u16 = 1;
pub const MAX_MATERIAL_ENTRIES_V1: usize = 256;
pub const MAX_MATERIAL_TABLE_BYTES_V1: usize = 64 * 1024;
pub const MATERIAL_SHADER_RECORD_BYTES_V1: usize = 32;
pub const MATERIAL_RESPONSE_MAX_MILLI_V1: u16 = 1_000;

const MAGIC: &[u8; 8] = b"BSTMAT01";
const ENTRY_BYTES: usize = 52;
const HEADER_BYTES: usize = 88;
const DIGEST_BYTES: usize = 32;
const ALLOWED_RESPONSE_FLAGS: u16 = 0x000f;

pub type MaterialDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum MaterialClassV1 {
    LegacyFallback = 0,
    OpaqueVoxel = 1,
    CutoutVoxel = 2,
    EmissiveVoxel = 3,
    MetallicVoxel = 4,
}

impl MaterialClassV1 {
    /// Classification boundary for noncanonical producer tags. Unsupported
    /// inputs remain explicit legacy fallback and never acquire an invented
    /// semantic label.
    #[must_use]
    pub const fn classify_declared_tag(tag: u16) -> Self {
        match tag {
            1 => Self::OpaqueVoxel,
            2 => Self::CutoutVoxel,
            3 => Self::EmissiveVoxel,
            4 => Self::MetallicVoxel,
            _ => Self::LegacyFallback,
        }
    }

    fn decode_canonical(tag: u16) -> Result<Self, MaterialErrorV1> {
        match tag {
            0 => Ok(Self::LegacyFallback),
            1 => Ok(Self::OpaqueVoxel),
            2 => Ok(Self::CutoutVoxel),
            3 => Ok(Self::EmissiveVoxel),
            4 => Ok(Self::MetallicVoxel),
            _ => Err(MaterialErrorV1::UnknownClass(tag)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaterialResponseV1 {
    pub base_color_rgba: [u8; 4],
    pub roughness_milli: u16,
    pub metallic_milli: u16,
    pub emission_milli: u16,
    pub alpha_cutoff_milli: u16,
    pub flags: u16,
}

impl MaterialResponseV1 {
    fn validate(self) -> Result<(), MaterialErrorV1> {
        for (field, value) in [
            ("roughness_milli", self.roughness_milli),
            ("metallic_milli", self.metallic_milli),
            ("emission_milli", self.emission_milli),
            ("alpha_cutoff_milli", self.alpha_cutoff_milli),
        ] {
            if value > MATERIAL_RESPONSE_MAX_MILLI_V1 {
                return Err(MaterialErrorV1::ResponseOutOfRange { field, value });
            }
        }
        if self.flags & !ALLOWED_RESPONSE_FLAGS != 0 {
            return Err(MaterialErrorV1::ResponseFlagsOutOfRange(self.flags));
        }
        Ok(())
    }

    fn encode(self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.base_color_rgba);
        output.extend_from_slice(&self.roughness_milli.to_le_bytes());
        output.extend_from_slice(&self.metallic_milli.to_le_bytes());
        output.extend_from_slice(&self.emission_milli.to_le_bytes());
        output.extend_from_slice(&self.alpha_cutoff_milli.to_le_bytes());
        output.extend_from_slice(&self.flags.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaterialEntryV1 {
    pub slot: u16,
    pub source_identity: MaterialDigestV1,
    pub class: MaterialClassV1,
    pub response: MaterialResponseV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialTableInputV1 {
    pub generation: u64,
    pub package_digest: MaterialDigestV1,
    pub package_authority_digest: MaterialDigestV1,
    pub entries: Vec<MaterialEntryV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialTableV1 {
    generation: u64,
    package_digest: MaterialDigestV1,
    package_authority_digest: MaterialDigestV1,
    entries: Vec<MaterialEntryV1>,
    table_digest: MaterialDigestV1,
}

impl MaterialTableV1 {
    pub fn new(mut input: MaterialTableInputV1) -> Result<Self, MaterialErrorV1> {
        validate_identity(
            input.generation,
            input.package_digest,
            input.package_authority_digest,
        )?;
        validate_entries(&mut input.entries)?;
        let digest = table_digest(
            input.generation,
            &input.package_digest,
            &input.package_authority_digest,
            &input.entries,
        )?;
        Ok(Self {
            generation: input.generation,
            package_digest: input.package_digest,
            package_authority_digest: input.package_authority_digest,
            entries: input.entries,
            table_digest: digest,
        })
    }

    #[must_use]
    pub const fn generation(&self) -> u64 { self.generation }

    #[must_use]
    pub const fn package_digest(&self) -> MaterialDigestV1 { self.package_digest }

    #[must_use]
    pub const fn package_authority_digest(&self) -> MaterialDigestV1 {
        self.package_authority_digest
    }

    #[must_use]
    pub fn entries(&self) -> &[MaterialEntryV1] { &self.entries }

    #[must_use]
    pub const fn table_digest(&self) -> MaterialDigestV1 { self.table_digest }

    pub fn validate_package(
        &self,
        package_digest: MaterialDigestV1,
        package_authority_digest: MaterialDigestV1,
    ) -> Result<(), MaterialErrorV1> {
        if package_digest != self.package_digest {
            return Err(MaterialErrorV1::PackageDigestMismatch);
        }
        if package_authority_digest != self.package_authority_digest {
            return Err(MaterialErrorV1::PackageAuthorityMismatch);
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, MaterialErrorV1> {
        let mut entries = self.entries.clone();
        validate_identity(
            self.generation,
            self.package_digest,
            self.package_authority_digest,
        )?;
        validate_entries(&mut entries)?;
        if entries != self.entries {
            return Err(MaterialErrorV1::NoncanonicalOrder);
        }
        let mut output = encode_without_digest(
            self.generation,
            &self.package_digest,
            &self.package_authority_digest,
            &entries,
        )?;
        let digest = hash_table(&output)?;
        if digest != self.table_digest {
            return Err(MaterialErrorV1::DigestMismatch);
        }
        output.extend_from_slice(&digest);
        Ok(output)
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, MaterialErrorV1> {
        if bytes.len() > MAX_MATERIAL_TABLE_BYTES_V1 {
            return Err(MaterialErrorV1::EncodedBytesOutOfRange(bytes.len()));
        }
        if bytes.len() < HEADER_BYTES + DIGEST_BYTES {
            return Err(MaterialErrorV1::Truncated);
        }
        let payload_len = bytes
            .len()
            .checked_sub(DIGEST_BYTES)
            .ok_or(MaterialErrorV1::Truncated)?;
        let (payload, declared_digest) = bytes.split_at(payload_len);
        let mut reader = Reader::new(payload);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(MaterialErrorV1::InvalidMagic);
        }
        let schema = reader.u16()?;
        if schema != MATERIAL_TABLE_SCHEMA_V1 {
            return Err(MaterialErrorV1::UnsupportedVersion(schema));
        }
        let compiler = reader.u16()?;
        if compiler != MATERIAL_COMPILER_V1 {
            return Err(MaterialErrorV1::UnsupportedCompiler(compiler));
        }
        let generation = reader.u64()?;
        let package_digest = reader.digest()?;
        let package_authority_digest = reader.digest()?;
        let count = usize::from(reader.u16()?);
        if reader.u16()? != 0 {
            return Err(MaterialErrorV1::NonzeroReserved);
        }
        if count == 0 || count > MAX_MATERIAL_ENTRIES_V1 {
            return Err(MaterialErrorV1::EntryCountOutOfRange(count));
        }
        let expected_payload = HEADER_BYTES
            .checked_add(
                count
                    .checked_mul(ENTRY_BYTES)
                    .ok_or(MaterialErrorV1::EncodedBytesOutOfRange(bytes.len()))?,
            )
            .ok_or(MaterialErrorV1::EncodedBytesOutOfRange(bytes.len()))?;
        if expected_payload != payload.len() {
            return Err(if expected_payload > payload.len() {
                MaterialErrorV1::Truncated
            } else {
                MaterialErrorV1::TrailingBytes
            });
        }
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let slot = reader.u16()?;
            let class = MaterialClassV1::decode_canonical(reader.u16()?)?;
            let source_identity = reader.digest()?;
            let base_color_rgba = reader.array4()?;
            let response = MaterialResponseV1 {
                base_color_rgba,
                roughness_milli: reader.u16()?,
                metallic_milli: reader.u16()?,
                emission_milli: reader.u16()?,
                alpha_cutoff_milli: reader.u16()?,
                flags: reader.u16()?,
            };
            if reader.u16()? != 0 {
                return Err(MaterialErrorV1::NonzeroReserved);
            }
            entries.push(MaterialEntryV1 {
                slot,
                source_identity,
                class,
                response,
            });
        }
        if !reader.is_empty() {
            return Err(MaterialErrorV1::TrailingBytes);
        }
        let actual_digest = hash_table(payload)?;
        if declared_digest != actual_digest {
            return Err(MaterialErrorV1::DigestMismatch);
        }
        let value = Self::new(MaterialTableInputV1 {
            generation,
            package_digest,
            package_authority_digest,
            entries,
        })?;
        if value.table_digest != actual_digest || value.canonical_bytes()?.as_slice() != bytes {
            return Err(MaterialErrorV1::NoncanonicalEncoding);
        }
        Ok(value)
    }

    pub fn shader_records(&self) -> Result<Vec<MaterialShaderRecordV1>, MaterialErrorV1> {
        self.entries
            .iter()
            .copied()
            .map(MaterialShaderRecordV1::from_entry)
            .collect()
    }

    pub fn shader_interface_bytes(&self) -> Result<Vec<u8>, MaterialErrorV1> {
        let records = self.shader_records()?;
        let capacity = records
            .len()
            .checked_mul(MATERIAL_SHADER_RECORD_BYTES_V1)
            .ok_or(MaterialErrorV1::EncodedBytesOutOfRange(usize::MAX))?;
        let mut output = Vec::with_capacity(capacity);
        for record in records {
            output.extend_from_slice(&record.to_le_bytes());
        }
        Ok(output)
    }

    pub fn shader_interface_digest(&self) -> Result<MaterialDigestV1, MaterialErrorV1> {
        domain_hash_v1(
            "bastion/r1f/material-shader-interface",
            MATERIAL_TABLE_SCHEMA_V1,
            0,
            &self.shader_interface_bytes()?,
        )
        .map_err(|_| MaterialErrorV1::Hash)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaterialShaderRecordV1 {
    bytes: [u8; MATERIAL_SHADER_RECORD_BYTES_V1],
}

impl MaterialShaderRecordV1 {
    fn from_entry(entry: MaterialEntryV1) -> Result<Self, MaterialErrorV1> {
        entry.response.validate()?;
        let mut bytes = [0_u8; MATERIAL_SHADER_RECORD_BYTES_V1];
        bytes[0..2].copy_from_slice(&entry.slot.to_le_bytes());
        bytes[2..4].copy_from_slice(&(entry.class as u16).to_le_bytes());
        bytes[4..6].copy_from_slice(&entry.response.flags.to_le_bytes());
        bytes[8..12].copy_from_slice(&entry.response.base_color_rgba);
        bytes[12..14].copy_from_slice(&entry.response.roughness_milli.to_le_bytes());
        bytes[14..16].copy_from_slice(&entry.response.metallic_milli.to_le_bytes());
        bytes[16..18].copy_from_slice(&entry.response.emission_milli.to_le_bytes());
        bytes[18..20].copy_from_slice(&entry.response.alpha_cutoff_milli.to_le_bytes());
        bytes[20..32].copy_from_slice(&entry.source_identity[..12]);
        Ok(Self { bytes })
    }

    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; MATERIAL_SHADER_RECORD_BYTES_V1] { self.bytes }
}

#[derive(Clone, Debug, Default)]
pub struct MaterialTablePublisherV1 {
    current: Option<(u64, u64, Arc<MaterialTableV1>)>,
}

impl MaterialTablePublisherV1 {
    pub fn publish(
        &mut self,
        sequence: u64,
        table: MaterialTableV1,
    ) -> Result<Arc<MaterialTableV1>, MaterialErrorV1> {
        if sequence == 0 {
            return Err(MaterialErrorV1::InvalidPublicationSequence);
        }
        if let Some((generation, prior_sequence, _)) = &self.current
            && (table.generation(), sequence) <= (*generation, *prior_sequence)
        {
            return Err(MaterialErrorV1::StalePublication {
                attempted_generation: table.generation(),
                attempted_sequence: sequence,
                current_generation: *generation,
                current_sequence: *prior_sequence,
            });
        }
        let value = Arc::new(table);
        self.current = Some((value.generation(), sequence, Arc::clone(&value)));
        Ok(value)
    }

    #[must_use]
    pub fn current(&self) -> Option<Arc<MaterialTableV1>> {
        self.current.as_ref().map(|(_, _, value)| Arc::clone(value))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialErrorV1 {
    InvalidGeneration,
    InvalidPackageDigest,
    InvalidPackageAuthorityDigest,
    EntryCountOutOfRange(usize),
    InvalidSlot(u16),
    InvalidSourceIdentity,
    DuplicateSlot(u16),
    DuplicateSourceIdentity(MaterialDigestV1),
    ResponseOutOfRange {
        field: &'static str,
        value: u16,
    },
    ResponseFlagsOutOfRange(u16),
    UnknownClass(u16),
    InvalidMagic,
    UnsupportedVersion(u16),
    UnsupportedCompiler(u16),
    NonzeroReserved,
    Truncated,
    TrailingBytes,
    NoncanonicalOrder,
    NoncanonicalEncoding,
    DigestMismatch,
    PackageDigestMismatch,
    PackageAuthorityMismatch,
    EncodedBytesOutOfRange(usize),
    InvalidPublicationSequence,
    StalePublication {
        attempted_generation: u64,
        attempted_sequence: u64,
        current_generation: u64,
        current_sequence: u64,
    },
    Hash,
}

fn validate_identity(
    generation: u64,
    package_digest: MaterialDigestV1,
    package_authority_digest: MaterialDigestV1,
) -> Result<(), MaterialErrorV1> {
    if generation == 0 {
        return Err(MaterialErrorV1::InvalidGeneration);
    }
    if package_digest == [0; 32] {
        return Err(MaterialErrorV1::InvalidPackageDigest);
    }
    if package_authority_digest == [0; 32] {
        return Err(MaterialErrorV1::InvalidPackageAuthorityDigest);
    }
    Ok(())
}

fn validate_entries(entries: &mut Vec<MaterialEntryV1>) -> Result<(), MaterialErrorV1> {
    if entries.is_empty() || entries.len() > MAX_MATERIAL_ENTRIES_V1 {
        return Err(MaterialErrorV1::EntryCountOutOfRange(entries.len()));
    }
    entries.sort_unstable_by(|left, right| {
        left.slot
            .cmp(&right.slot)
            .then(left.source_identity.cmp(&right.source_identity))
    });
    let mut slots = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for entry in entries {
        if entry.slot == 0 {
            return Err(MaterialErrorV1::InvalidSlot(entry.slot));
        }
        if entry.source_identity == [0; 32] {
            return Err(MaterialErrorV1::InvalidSourceIdentity);
        }
        if !slots.insert(entry.slot) {
            return Err(MaterialErrorV1::DuplicateSlot(entry.slot));
        }
        if !sources.insert(entry.source_identity) {
            return Err(MaterialErrorV1::DuplicateSourceIdentity(
                entry.source_identity,
            ));
        }
        entry.response.validate()?;
    }
    Ok(())
}

fn table_digest(
    generation: u64,
    package_digest: &MaterialDigestV1,
    package_authority_digest: &MaterialDigestV1,
    entries: &[MaterialEntryV1],
) -> Result<MaterialDigestV1, MaterialErrorV1> {
    hash_table(&encode_without_digest(
        generation,
        package_digest,
        package_authority_digest,
        entries,
    )?)
}

fn hash_table(bytes: &[u8]) -> Result<MaterialDigestV1, MaterialErrorV1> {
    domain_hash_v1(
        "bastion/r1f/material-table",
        MATERIAL_TABLE_SCHEMA_V1,
        0,
        bytes,
    )
    .map_err(|_| MaterialErrorV1::Hash)
}

fn encode_without_digest(
    generation: u64,
    package_digest: &MaterialDigestV1,
    package_authority_digest: &MaterialDigestV1,
    entries: &[MaterialEntryV1],
) -> Result<Vec<u8>, MaterialErrorV1> {
    let count = u16::try_from(entries.len())
        .map_err(|_| MaterialErrorV1::EntryCountOutOfRange(entries.len()))?;
    let capacity = HEADER_BYTES
        .checked_add(
            entries
                .len()
                .checked_mul(ENTRY_BYTES)
                .ok_or(MaterialErrorV1::EncodedBytesOutOfRange(usize::MAX))?,
        )
        .ok_or(MaterialErrorV1::EncodedBytesOutOfRange(usize::MAX))?;
    if capacity + DIGEST_BYTES > MAX_MATERIAL_TABLE_BYTES_V1 {
        return Err(MaterialErrorV1::EncodedBytesOutOfRange(
            capacity + DIGEST_BYTES,
        ));
    }
    let mut output = Vec::with_capacity(capacity);
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&MATERIAL_TABLE_SCHEMA_V1.to_le_bytes());
    output.extend_from_slice(&MATERIAL_COMPILER_V1.to_le_bytes());
    output.extend_from_slice(&generation.to_le_bytes());
    output.extend_from_slice(package_digest);
    output.extend_from_slice(package_authority_digest);
    output.extend_from_slice(&count.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    for entry in entries {
        output.extend_from_slice(&entry.slot.to_le_bytes());
        output.extend_from_slice(&(entry.class as u16).to_le_bytes());
        output.extend_from_slice(&entry.source_identity);
        entry.response.encode(&mut output);
    }
    debug_assert_eq!(output.len(), capacity);
    Ok(output)
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, cursor: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], MaterialErrorV1> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or(MaterialErrorV1::Truncated)?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(MaterialErrorV1::Truncated)?;
        self.cursor = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, MaterialErrorV1> {
        let bytes: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| MaterialErrorV1::Truncated)?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, MaterialErrorV1> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| MaterialErrorV1::Truncated)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn digest(&mut self) -> Result<MaterialDigestV1, MaterialErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| MaterialErrorV1::Truncated)
    }

    fn array4(&mut self) -> Result<[u8; 4], MaterialErrorV1> {
        self.take(4)?
            .try_into()
            .map_err(|_| MaterialErrorV1::Truncated)
    }

    fn is_empty(&self) -> bool { self.cursor == self.bytes.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex32;

    fn digest(byte: u8) -> MaterialDigestV1 { [byte; 32] }

    fn response(class: MaterialClassV1, base: u8) -> MaterialResponseV1 {
        MaterialResponseV1 {
            base_color_rgba: [base, base + 1, base + 2, 255],
            roughness_milli: if class == MaterialClassV1::MetallicVoxel {
                300
            } else {
                800
            },
            metallic_milli: if class == MaterialClassV1::MetallicVoxel {
                1_000
            } else {
                0
            },
            emission_milli: if class == MaterialClassV1::EmissiveVoxel {
                1_000
            } else {
                0
            },
            alpha_cutoff_milli: if class == MaterialClassV1::CutoutVoxel {
                500
            } else {
                0
            },
            flags: 1,
        }
    }

    fn entry(slot: u16, source: u8, class: MaterialClassV1) -> MaterialEntryV1 {
        MaterialEntryV1 {
            slot,
            source_identity: digest(source),
            class,
            response: response(class, source),
        }
    }

    fn table(entries: Vec<MaterialEntryV1>) -> MaterialTableV1 {
        MaterialTableV1::new(MaterialTableInputV1 {
            generation: 7,
            package_digest: digest(90),
            package_authority_digest: digest(91),
            entries,
        })
        .unwrap()
    }

    #[test]
    fn canonical_permutation_round_trip_and_frozen_digest() {
        let a = table(vec![
            entry(2, 12, MaterialClassV1::MetallicVoxel),
            entry(1, 11, MaterialClassV1::OpaqueVoxel),
        ]);
        let b = table(vec![
            entry(1, 11, MaterialClassV1::OpaqueVoxel),
            entry(2, 12, MaterialClassV1::MetallicVoxel),
        ]);
        assert_eq!(a, b);
        let bytes = a.canonical_bytes().unwrap();
        assert_eq!(MaterialTableV1::decode_exact(&bytes).unwrap(), a);
        assert_eq!(
            hex32(&a.table_digest()),
            "88fd2c1d8ffeeaffce0b91635f9c7cb4fda2912e52b1e86aff11cee9e0462a3f"
        );
    }

    #[test]
    fn exact_eof_and_malformed_matrix_fail_closed() {
        let value = table(vec![entry(1, 11, MaterialClassV1::OpaqueVoxel)]);
        let bytes = value.canonical_bytes().unwrap();
        for cut in [0, 1, 7, 87, bytes.len() - 1] {
            assert!(MaterialTableV1::decode_exact(&bytes[..cut]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(MaterialTableV1::decode_exact(&trailing).is_err());
        let mut bad_magic = bytes.clone();
        bad_magic[0] ^= 1;
        assert_eq!(
            MaterialTableV1::decode_exact(&bad_magic),
            Err(MaterialErrorV1::InvalidMagic)
        );
        let mut bad_class = bytes.clone();
        bad_class[90..92].copy_from_slice(&99_u16.to_le_bytes());
        assert_eq!(
            MaterialTableV1::decode_exact(&bad_class),
            Err(MaterialErrorV1::UnknownClass(99))
        );
        let oversized = vec![0_u8; MAX_MATERIAL_TABLE_BYTES_V1 + 1];
        assert_eq!(
            MaterialTableV1::decode_exact(&oversized),
            Err(MaterialErrorV1::EncodedBytesOutOfRange(
                MAX_MATERIAL_TABLE_BYTES_V1 + 1
            ))
        );
    }

    #[test]
    fn duplicate_conflict_bounds_and_response_failures_are_typed() {
        assert_eq!(
            MaterialTableV1::new(MaterialTableInputV1 {
                generation: 7,
                package_digest: digest(90),
                package_authority_digest: digest(91),
                entries: vec![
                    entry(1, 11, MaterialClassV1::OpaqueVoxel),
                    entry(1, 12, MaterialClassV1::MetallicVoxel),
                ],
            }),
            Err(MaterialErrorV1::DuplicateSlot(1))
        );
        assert!(matches!(
            MaterialTableV1::new(MaterialTableInputV1 {
                generation: 7,
                package_digest: digest(90),
                package_authority_digest: digest(91),
                entries: vec![
                    entry(1, 11, MaterialClassV1::OpaqueVoxel),
                    entry(2, 11, MaterialClassV1::OpaqueVoxel),
                ],
            }),
            Err(MaterialErrorV1::DuplicateSourceIdentity(_))
        ));
        let mut invalid = entry(1, 11, MaterialClassV1::OpaqueVoxel);
        invalid.response.roughness_milli = 1_001;
        assert!(matches!(
            table_result(vec![invalid]),
            Err(MaterialErrorV1::ResponseOutOfRange { .. })
        ));
        assert_eq!(
            table_result(Vec::new()),
            Err(MaterialErrorV1::EntryCountOutOfRange(0))
        );
    }

    #[test]
    fn unknown_declared_inputs_use_explicit_legacy_fallback() {
        assert_eq!(
            MaterialClassV1::classify_declared_tag(1),
            MaterialClassV1::OpaqueVoxel
        );
        assert_eq!(
            MaterialClassV1::classify_declared_tag(999),
            MaterialClassV1::LegacyFallback
        );
    }

    #[test]
    fn package_and_provenance_mismatch_reject() {
        let value = table(vec![entry(1, 11, MaterialClassV1::OpaqueVoxel)]);
        assert_eq!(
            value.validate_package(digest(1), value.package_authority_digest()),
            Err(MaterialErrorV1::PackageDigestMismatch)
        );
        assert_eq!(
            value.validate_package(value.package_digest(), digest(1)),
            Err(MaterialErrorV1::PackageAuthorityMismatch)
        );
    }

    #[test]
    fn shader_interface_layout_and_frozen_bytes_are_stable() {
        let value = table(vec![entry(1, 11, MaterialClassV1::OpaqueVoxel)]);
        let bytes = value.shader_interface_bytes().unwrap();
        assert_eq!(bytes.len(), MATERIAL_SHADER_RECORD_BYTES_V1);
        assert_eq!(bytes, vec![
            1, 0, 1, 0, 1, 0, 0, 0, 11, 12, 13, 255, 32, 3, 0, 0, 0, 0, 0, 0, 11, 11, 11, 11, 11,
            11, 11, 11, 11, 11, 11, 11,
        ]);
    }

    #[test]
    fn every_response_field_changes_authority_and_replay_is_exact() {
        let value = table(vec![entry(1, 11, MaterialClassV1::OpaqueVoxel)]);
        let baseline = value.table_digest();
        for mutate in 0..6 {
            let mut changed = entry(1, 11, MaterialClassV1::OpaqueVoxel);
            match mutate {
                0 => changed.response.base_color_rgba[0] ^= 1,
                1 => changed.response.roughness_milli += 1,
                2 => changed.response.metallic_milli += 1,
                3 => changed.response.emission_milli += 1,
                4 => changed.response.alpha_cutoff_milli += 1,
                _ => changed.response.flags ^= 2,
            }
            assert_ne!(table(vec![changed]).table_digest(), baseline);
        }
        let replay = MaterialTableV1::decode_exact(&value.canonical_bytes().unwrap()).unwrap();
        assert_eq!(
            replay.canonical_bytes().unwrap(),
            value.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn publication_is_monotonic_and_failure_preserves_held_reader() {
        let first = table(vec![entry(1, 11, MaterialClassV1::OpaqueVoxel)]);
        let mut publisher = MaterialTablePublisherV1::default();
        let held = publisher.publish(1, first.clone()).unwrap();
        assert_eq!(
            publisher.publish(1, first.clone()),
            Err(MaterialErrorV1::StalePublication {
                attempted_generation: 7,
                attempted_sequence: 1,
                current_generation: 7,
                current_sequence: 1,
            })
        );
        assert_eq!(
            publisher.current().unwrap().table_digest(),
            held.table_digest()
        );
        let mut next_input = MaterialTableInputV1 {
            generation: 8,
            package_digest: digest(90),
            package_authority_digest: digest(91),
            entries: vec![entry(1, 12, MaterialClassV1::MetallicVoxel)],
        };
        let next = MaterialTableV1::new(next_input.clone()).unwrap();
        let current = publisher.publish(1, next).unwrap();
        assert_ne!(current.table_digest(), held.table_digest());
        next_input.generation = 7;
        assert!(
            publisher
                .publish(2, MaterialTableV1::new(next_input).unwrap())
                .is_err()
        );
        assert_eq!(held.table_digest(), first.table_digest());
    }

    fn table_result(entries: Vec<MaterialEntryV1>) -> Result<MaterialTableV1, MaterialErrorV1> {
        MaterialTableV1::new(MaterialTableInputV1 {
            generation: 7,
            package_digest: digest(90),
            package_authority_digest: digest(91),
            entries,
        })
    }
}
