//! R1BC reproducible modular figure assets.
//!
//! This layer deliberately builds on
//! [`crate::figure_package::FigurePackageV1`]. It adds source authority,
//! material/section semantics, immutable cache publication, and
//! presentation-generation receipts without introducing a second package
//! container or any GPU policy.

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    figure_package::{FigurePackageErrorV1, FigurePackageV1, SectionV1, normalize_machine_path},
    presentation::{PresentationFrameV1, PresentationGenerationV1, RendererUploadCompletionV1},
};

pub const FIGURE_ASSET_SCHEMA_VERSION_V1: u16 = 1;
pub const FIGURE_COMPILER_VERSION_V1: u16 = 1;
pub const MAX_FIGURE_SOURCES_V1: usize = 48;
pub const MAX_FIGURE_SOURCE_PATH_BYTES_V1: usize = 240;
pub const MAX_FIGURE_SOURCE_BYTES_V1: usize = 8 * 1024 * 1024;
pub const MAX_FIGURE_MATERIALS_V1: usize = 64;
pub const MAX_FIGURE_COMPOSITION_COMPONENTS_V1: usize = 24;

const AUTHORITY_TAG: u16 = 1;
const MATERIAL_TAG: u16 = 2;
const SOURCE_TAG_BASE: u16 = 100;
const AUTHORITY_MAGIC: &[u8; 8] = b"BSTRFSA1";
const MATERIAL_MAGIC: &[u8; 8] = b"BSTRMAT1";
const SECTION_MAGIC: &[u8; 8] = b"BSTRSEC1";
const INDEX_MAGIC: &[u8; 8] = b"BSTRIDX1";

pub type FigureDigestV1 = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FigureAssetRoleV1 {
    CoreBody = 1,
    Equipment = 2,
    Attachment = 3,
    Lod = 4,
    ShadowProxy = 5,
    Impostor = 6,
}

impl FigureAssetRoleV1 {
    fn from_u8(value: u8) -> Result<Self, FigureAssetErrorV1> {
        match value {
            1 => Ok(Self::CoreBody),
            2 => Ok(Self::Equipment),
            3 => Ok(Self::Attachment),
            4 => Ok(Self::Lod),
            5 => Ok(Self::ShadowProxy),
            6 => Ok(Self::Impostor),
            _ => Err(FigureAssetErrorV1::UnknownRole(value)),
        }
    }

    const fn fixture_allowed(self) -> bool {
        matches!(self, Self::Lod | Self::ShadowProxy | Self::Impostor)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum FigurePackageTargetV1 {
    Body = 1,
    Equipment = 2,
    Attachment = 3,
    Composite = 4,
}

impl FigurePackageTargetV1 {
    fn from_u8(value: u8) -> Result<Self, FigureAssetErrorV1> {
        match value {
            1 => Ok(Self::Body),
            2 => Ok(Self::Equipment),
            3 => Ok(Self::Attachment),
            4 => Ok(Self::Composite),
            _ => Err(FigureAssetErrorV1::UnknownTarget(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u16)]
pub enum MaterialKindV1 {
    OpaqueVoxel = 1,
    CutoutVoxel = 2,
    EmissiveVoxel = 3,
    MetallicVoxel = 4,
}

impl MaterialKindV1 {
    fn from_u16(value: u16) -> Result<Self, FigureAssetErrorV1> {
        match value {
            1 => Ok(Self::OpaqueVoxel),
            2 => Ok(Self::CutoutVoxel),
            3 => Ok(Self::EmissiveVoxel),
            4 => Ok(Self::MetallicVoxel),
            _ => Err(FigureAssetErrorV1::UnknownMaterial(value)),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaterialBindingV1 {
    pub slot: u16,
    pub kind: MaterialKindV1,
    pub base_color_rgba: [u8; 4],
    pub flags: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigureSourceInputV1 {
    pub logical_path: String,
    pub role: FigureAssetRoleV1,
    pub material_slot: u16,
    pub bytes: Vec<u8>,
    /// True only for deterministic R1BC placeholders where the current corpus
    /// has no authored LOD, shadow proxy, or impostor source yet.
    pub deterministic_fixture: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FigureSourceAuthorityV1 {
    pub logical_path: String,
    pub role: FigureAssetRoleV1,
    pub material_slot: u16,
    pub content_sha256: FigureDigestV1,
    pub size: u64,
    pub deterministic_fixture: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FigurePackageAuthorityV1 {
    pub schema_version: u16,
    pub compiler_version: u16,
    pub target: FigurePackageTargetV1,
    pub asset_epoch: FigureDigestV1,
    pub corpus_epoch: FigureDigestV1,
    pub sources: Vec<FigureSourceAuthorityV1>,
}

impl FigurePackageAuthorityV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, FigureAssetErrorV1> {
        validate_authority(self)?;
        let mut output = Vec::new();
        output.extend_from_slice(AUTHORITY_MAGIC);
        put_u16(&mut output, self.schema_version);
        put_u16(&mut output, self.compiler_version);
        output.push(self.target as u8);
        output.push(0);
        output.extend_from_slice(&self.asset_epoch);
        output.extend_from_slice(&self.corpus_epoch);
        put_count(&mut output, self.sources.len())?;
        for source in &self.sources {
            output.push(source.role as u8);
            output.push(u8::from(source.deterministic_fixture));
            put_u16(&mut output, source.material_slot);
            put_text(&mut output, &source.logical_path)?;
            output.extend_from_slice(&source.content_sha256);
            put_u64(&mut output, source.size);
        }
        Ok(output)
    }

    pub fn digest(&self) -> Result<FigureDigestV1, FigureAssetErrorV1> {
        Ok(Sha256::digest(self.canonical_bytes()?).into())
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, FigureAssetErrorV1> {
        let mut reader = AssetReaderV1::new(bytes);
        if reader.take(8)? != AUTHORITY_MAGIC {
            return Err(FigureAssetErrorV1::InvalidAuthorityMagic);
        }
        let schema_version = reader.u16()?;
        if schema_version != FIGURE_ASSET_SCHEMA_VERSION_V1 {
            return Err(FigureAssetErrorV1::UnsupportedSchema(schema_version));
        }
        let compiler_version = reader.u16()?;
        if compiler_version != FIGURE_COMPILER_VERSION_V1 {
            return Err(FigureAssetErrorV1::UnsupportedCompiler(compiler_version));
        }
        let target = FigurePackageTargetV1::from_u8(reader.u8()?)?;
        if reader.u8()? != 0 {
            return Err(FigureAssetErrorV1::NonCanonical);
        }
        let asset_epoch = reader.digest()?;
        let corpus_epoch = reader.digest()?;
        let count = reader.count(MAX_FIGURE_SOURCES_V1)?;
        let mut sources = try_vec(count)?;
        for _ in 0..count {
            let role = FigureAssetRoleV1::from_u8(reader.u8()?)?;
            let deterministic_fixture = match reader.u8()? {
                0 => false,
                1 => true,
                other => return Err(FigureAssetErrorV1::MalformedBoolean(other)),
            };
            let material_slot = reader.u16()?;
            let logical_path = reader.text(MAX_FIGURE_SOURCE_PATH_BYTES_V1)?;
            let content_sha256 = reader.digest()?;
            let size = reader.u64()?;
            sources.push(FigureSourceAuthorityV1 {
                logical_path,
                role,
                material_slot,
                content_sha256,
                size,
                deterministic_fixture,
            });
        }
        if reader.remaining() != 0 {
            return Err(FigureAssetErrorV1::TrailingBytes(reader.remaining()));
        }
        let authority = Self {
            schema_version,
            compiler_version,
            target,
            asset_epoch,
            corpus_epoch,
            sources,
        };
        validate_authority(&authority)?;
        if authority.canonical_bytes()?.as_slice() != bytes {
            return Err(FigureAssetErrorV1::NonCanonical);
        }
        Ok(authority)
    }
}

#[derive(Clone, Debug)]
pub struct CompiledFigurePackageV1 {
    package: FigurePackageV1,
    canonical_bytes: Vec<u8>,
    package_digest: FigureDigestV1,
    authority: FigurePackageAuthorityV1,
    authority_digest: FigureDigestV1,
    materials: Vec<MaterialBindingV1>,
    required_section_identities: Vec<FigureDigestV1>,
}

impl CompiledFigurePackageV1 {
    pub fn compile(
        target: FigurePackageTargetV1,
        asset_epoch: FigureDigestV1,
        corpus_epoch: FigureDigestV1,
        mut materials: Vec<MaterialBindingV1>,
        sources: Vec<FigureSourceInputV1>,
    ) -> Result<Self, FigureAssetErrorV1> {
        if is_zero(&asset_epoch) || is_zero(&corpus_epoch) {
            return Err(FigureAssetErrorV1::InvalidEpoch);
        }
        validate_materials(&mut materials)?;
        if sources.is_empty() {
            return Err(FigureAssetErrorV1::MissingSources);
        }
        if sources.len() > MAX_FIGURE_SOURCES_V1 {
            return Err(FigureAssetErrorV1::TooManySources(sources.len()));
        }
        let material_slots = materials
            .iter()
            .map(|material| material.slot)
            .collect::<BTreeSet<_>>();
        let mut normalized = Vec::new();
        for source in sources {
            if source.bytes.is_empty() {
                return Err(FigureAssetErrorV1::EmptySource);
            }
            if source.bytes.len() > MAX_FIGURE_SOURCE_BYTES_V1 {
                return Err(FigureAssetErrorV1::SourceTooLarge(source.bytes.len()));
            }
            if !material_slots.contains(&source.material_slot) {
                return Err(FigureAssetErrorV1::UnknownMaterialSlot(
                    source.material_slot,
                ));
            }
            if source.deterministic_fixture && !source.role.fixture_allowed() {
                return Err(FigureAssetErrorV1::FixtureForbiddenForRole(source.role));
            }
            let path =
                normalize_machine_path(&source.logical_path).map_err(FigureAssetErrorV1::Path)?;
            if path.len() > MAX_FIGURE_SOURCE_PATH_BYTES_V1 {
                return Err(FigureAssetErrorV1::SourcePathTooLong(path.len()));
            }
            let digest: FigureDigestV1 = Sha256::digest(&source.bytes).into();
            normalized.push((
                FigureSourceAuthorityV1 {
                    logical_path: path,
                    role: source.role,
                    material_slot: source.material_slot,
                    content_sha256: digest,
                    size: u64::try_from(source.bytes.len())
                        .map_err(|_| FigureAssetErrorV1::LengthOverflow)?,
                    deterministic_fixture: source.deterministic_fixture,
                },
                source.bytes,
            ));
        }
        normalized.sort_by(|(a, _), (b, _)| source_authority_cmp(a, b));
        for pair in normalized.windows(2) {
            if pair[0].0.logical_path == pair[1].0.logical_path {
                return Err(FigureAssetErrorV1::DuplicateSourcePath(
                    pair[0].0.logical_path.clone(),
                ));
            }
        }
        if !normalized
            .iter()
            .any(|(source, _)| source.role == FigureAssetRoleV1::CoreBody)
            && matches!(
                target,
                FigurePackageTargetV1::Body | FigurePackageTargetV1::Composite
            )
        {
            return Err(FigureAssetErrorV1::MissingCoreBody);
        }
        let authority = FigurePackageAuthorityV1 {
            schema_version: FIGURE_ASSET_SCHEMA_VERSION_V1,
            compiler_version: FIGURE_COMPILER_VERSION_V1,
            target,
            asset_epoch,
            corpus_epoch,
            sources: normalized
                .iter()
                .map(|(authority, _)| authority.clone())
                .collect(),
        };
        let authority_bytes = authority.canonical_bytes()?;
        let authority_digest: FigureDigestV1 = Sha256::digest(&authority_bytes).into();
        let material_bytes = encode_materials(&materials)?;
        let mut package_sections = vec![
            SectionV1 {
                tag: AUTHORITY_TAG,
                media_type: "application/vnd.bastion.figure-authority-v1".to_string(),
                bytes: authority_bytes,
            },
            SectionV1 {
                tag: MATERIAL_TAG,
                media_type: "application/vnd.bastion.figure-materials-v1".to_string(),
                bytes: material_bytes,
            },
        ];
        let mut required_section_identities = Vec::new();
        for (index, (source, payload)) in normalized.iter().enumerate() {
            let section_bytes = encode_source_section(source, payload)?;
            required_section_identities.push(Sha256::digest(&section_bytes).into());
            package_sections.push(SectionV1 {
                tag: SOURCE_TAG_BASE
                    .checked_add(
                        u16::try_from(index).map_err(|_| FigureAssetErrorV1::LengthOverflow)?,
                    )
                    .ok_or(FigureAssetErrorV1::LengthOverflow)?,
                media_type: "application/vnd.bastion.figure-source-v1".to_string(),
                bytes: section_bytes,
            });
        }
        required_section_identities.sort_unstable();
        let package = FigurePackageV1::try_from_sections(package_sections)?;
        let canonical_bytes = package.try_canonical_bytes()?;
        let package_digest = trailing_digest(&canonical_bytes)?;
        Ok(Self {
            package,
            canonical_bytes,
            package_digest,
            authority,
            authority_digest,
            materials,
            required_section_identities,
        })
    }

    pub fn decode_exact(bytes: &[u8]) -> Result<Self, FigureAssetErrorV1> {
        let package = FigurePackageV1::decode_exact(bytes)?;
        let authority_section = package
            .section(AUTHORITY_TAG)
            .ok_or(FigureAssetErrorV1::MissingAuthoritySection)?;
        let authority = FigurePackageAuthorityV1::decode_exact(&authority_section.bytes)?;
        let material_section = package
            .section(MATERIAL_TAG)
            .ok_or(FigureAssetErrorV1::MissingMaterialSection)?;
        let materials = decode_materials(&material_section.bytes)?;
        let mut inputs = Vec::new();
        let source_sections = package
            .sections()
            .iter()
            .filter(|section| section.tag >= SOURCE_TAG_BASE)
            .collect::<Vec<_>>();
        if source_sections.len() != authority.sources.len() {
            return Err(FigureAssetErrorV1::SourceCountMismatch);
        }
        for (expected, section) in authority.sources.iter().zip(source_sections) {
            let input = decode_source_section(&section.bytes)?;
            let actual_digest: FigureDigestV1 = Sha256::digest(&input.bytes).into();
            if input.logical_path != expected.logical_path
                || input.role != expected.role
                || input.material_slot != expected.material_slot
                || input.deterministic_fixture != expected.deterministic_fixture
                || actual_digest != expected.content_sha256
                || u64::try_from(input.bytes.len())
                    .map_err(|_| FigureAssetErrorV1::LengthOverflow)?
                    != expected.size
            {
                return Err(FigureAssetErrorV1::SourceProvenanceMismatch);
            }
            inputs.push(input);
        }
        let rebuilt = Self::compile(
            authority.target,
            authority.asset_epoch,
            authority.corpus_epoch,
            materials,
            inputs,
        )?;
        if rebuilt.canonical_bytes.as_slice() != bytes {
            return Err(FigureAssetErrorV1::PackageDigestMismatch);
        }
        Ok(rebuilt)
    }

    #[must_use]
    pub fn package(&self) -> &FigurePackageV1 { &self.package }

    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] { &self.canonical_bytes }

    #[must_use]
    pub const fn package_digest(&self) -> FigureDigestV1 { self.package_digest }

    #[must_use]
    pub const fn authority_digest(&self) -> FigureDigestV1 { self.authority_digest }

    #[must_use]
    pub fn authority(&self) -> &FigurePackageAuthorityV1 { &self.authority }

    #[must_use]
    pub fn materials(&self) -> &[MaterialBindingV1] { &self.materials }

    #[must_use]
    pub fn required_section_identities(&self) -> &[FigureDigestV1] {
        &self.required_section_identities
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachePublicationPolicyV1 {
    Commit,
    RollbackBeforePublish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachePublicationTerminalV1 {
    Published,
    ExistingIdentical,
    RolledBack,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachePublicationRecordV1 {
    pub authority_digest: FigureDigestV1,
    pub package_digest: FigureDigestV1,
    pub terminal: CachePublicationTerminalV1,
}

#[derive(Clone, Debug)]
pub struct FigurePackageCacheV1 {
    root: PathBuf,
}

impl FigurePackageCacheV1 {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self { Self { root: root.into() } }

    pub fn publish(
        &self,
        package: &CompiledFigurePackageV1,
        policy: CachePublicationPolicyV1,
    ) -> Result<CachePublicationRecordV1, FigureAssetErrorV1> {
        CompiledFigurePackageV1::decode_exact(package.canonical_bytes())?;
        let entries = self.root.join("entries");
        let staging = self.root.join("staging");
        create_dir_all(&entries)?;
        create_dir_all(&staging)?;
        let package_hex = hex_digest(&package.package_digest);
        let authority_hex = hex_digest(&package.authority_digest);
        let entry_path = entries.join(&authority_hex);
        let transaction = staging.join(format!("{authority_hex}-{package_hex}.txn"));
        let package_name = format!("{package_hex}.fpkg");
        let package_temp = transaction.join(&package_name);
        let index_temp = transaction.join("authority.idx");
        let index_bytes = encode_index(package.authority_digest, package.package_digest);

        if entry_path.exists() {
            validate_cache_entry(
                &entry_path,
                package.authority_digest,
                package.package_digest,
                package.canonical_bytes(),
            )?;
            return Ok(CachePublicationRecordV1 {
                authority_digest: package.authority_digest,
                package_digest: package.package_digest,
                terminal: if policy == CachePublicationPolicyV1::RollbackBeforePublish {
                    CachePublicationTerminalV1::RolledBack
                } else {
                    CachePublicationTerminalV1::ExistingIdentical
                },
            });
        }
        if transaction.exists() {
            validate_transaction(
                &transaction,
                &package_name,
                package.canonical_bytes(),
                &index_bytes,
            )?;
        } else {
            fs::create_dir(&transaction)
                .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))?;
        }
        if let Err(primary) = prepare_file(&package_temp, package.canonical_bytes()) {
            return cleanup_failed_transaction(&transaction, primary);
        }
        if let Err(primary) = prepare_file(&index_temp, &index_bytes) {
            return cleanup_failed_transaction(&transaction, primary);
        }
        validate_transaction(
            &transaction,
            &package_name,
            package.canonical_bytes(),
            &index_bytes,
        )?;
        if policy == CachePublicationPolicyV1::RollbackBeforePublish {
            remove_transaction(&transaction)?;
            return Ok(CachePublicationRecordV1 {
                authority_digest: package.authority_digest,
                package_digest: package.package_digest,
                terminal: CachePublicationTerminalV1::RolledBack,
            });
        }
        let existed = match fs::rename(&transaction, &entry_path) {
            Ok(()) => false,
            Err(_) if entry_path.exists() => {
                validate_cache_entry(
                    &entry_path,
                    package.authority_digest,
                    package.package_digest,
                    package.canonical_bytes(),
                )?;
                remove_transaction(&transaction)?;
                true
            },
            Err(error) => {
                return cleanup_failed_transaction(
                    &transaction,
                    FigureAssetErrorV1::Io(error.to_string()),
                );
            },
        };
        let loaded = self.load(package.authority_digest)?;
        if loaded.package_digest != package.package_digest {
            return Err(FigureAssetErrorV1::PackageDigestMismatch);
        }
        Ok(CachePublicationRecordV1 {
            authority_digest: package.authority_digest,
            package_digest: package.package_digest,
            terminal: if existed {
                CachePublicationTerminalV1::ExistingIdentical
            } else {
                CachePublicationTerminalV1::Published
            },
        })
    }

    pub fn load(
        &self,
        authority_digest: FigureDigestV1,
    ) -> Result<CompiledFigurePackageV1, FigureAssetErrorV1> {
        let authority_hex = hex_digest(&authority_digest);
        let entry = self.root.join("entries").join(authority_hex);
        let index_path = entry.join("authority.idx");
        let index = read(&index_path)?;
        let (indexed_authority, package_digest) = decode_index(&index)?;
        if indexed_authority != authority_digest {
            return Err(FigureAssetErrorV1::ProvenanceIndexMismatch);
        }
        let package_path = entry.join(format!("{}.fpkg", hex_digest(&package_digest)));
        let actual_names = fs::read_dir(&entry)
            .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))?
            .map(|value| {
                value
                    .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))
                    .and_then(|entry| {
                        entry
                            .file_name()
                            .into_string()
                            .map_err(|_| FigureAssetErrorV1::NonCanonical)
                    })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let expected_names = BTreeSet::from([
            "authority.idx".to_string(),
            package_path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or(FigureAssetErrorV1::NonCanonical)?
                .to_string(),
        ]);
        if actual_names != expected_names {
            return Err(FigureAssetErrorV1::CorruptPartialPublication);
        }
        let bytes = read(&package_path)?;
        let package = CompiledFigurePackageV1::decode_exact(&bytes)?;
        if package.package_digest != package_digest || package.authority_digest != authority_digest
        {
            return Err(FigureAssetErrorV1::ProvenanceIndexMismatch);
        }
        Ok(package)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageReceiptTerminalV1 {
    Accepted,
    RolledBack,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageReceiptV1 {
    pub generation: PresentationGenerationV1,
    pub frame_digest: FigureDigestV1,
    pub resource_set_digest: FigureDigestV1,
    pub package_digest: FigureDigestV1,
    pub authority_digest: FigureDigestV1,
    pub required_section_identities: Vec<FigureDigestV1>,
    pub cache_terminal: CachePublicationTerminalV1,
    pub terminal: PackageReceiptTerminalV1,
}

impl PackageReceiptV1 {
    pub fn from_publication(
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
        publication: &CachePublicationRecordV1,
    ) -> Result<Self, FigureAssetErrorV1> {
        if publication.authority_digest != package.authority_digest
            || publication.package_digest != package.package_digest
        {
            return Err(FigureAssetErrorV1::ReceiptPublicationMismatch);
        }
        if !frame
            .renderer_required_resources()
            .contains(&package.package_digest)
        {
            return Err(FigureAssetErrorV1::PackageNotRequired);
        }
        let terminal = match publication.terminal {
            CachePublicationTerminalV1::Published
            | CachePublicationTerminalV1::ExistingIdentical => PackageReceiptTerminalV1::Accepted,
            CachePublicationTerminalV1::RolledBack => PackageReceiptTerminalV1::RolledBack,
        };
        Ok(Self {
            generation: frame.generation(),
            frame_digest: frame.frame_digest(),
            resource_set_digest: frame.resource_set_digest(),
            package_digest: package.package_digest,
            authority_digest: package.authority_digest,
            required_section_identities: package.required_section_identities.clone(),
            cache_terminal: publication.terminal,
            terminal,
        })
    }

    pub fn validate(
        &self,
        frame: &PresentationFrameV1,
        package: &CompiledFigurePackageV1,
    ) -> Result<(), FigureAssetErrorV1> {
        if self.terminal != PackageReceiptTerminalV1::Accepted {
            return Err(FigureAssetErrorV1::ReceiptRolledBack);
        }
        if self.cache_terminal == CachePublicationTerminalV1::RolledBack {
            return Err(FigureAssetErrorV1::ReceiptRolledBack);
        }
        if self.generation != frame.generation() {
            return Err(FigureAssetErrorV1::ReceiptGenerationMismatch);
        }
        if self.frame_digest != frame.frame_digest()
            || self.resource_set_digest != frame.resource_set_digest()
        {
            return Err(FigureAssetErrorV1::ReceiptFrameMismatch);
        }
        if self.package_digest != package.package_digest
            || self.authority_digest != package.authority_digest
        {
            return Err(FigureAssetErrorV1::ReceiptPackageMismatch);
        }
        if self.required_section_identities != package.required_section_identities {
            return Err(FigureAssetErrorV1::ReceiptSectionMismatch);
        }
        if !frame
            .renderer_required_resources()
            .contains(&self.package_digest)
        {
            return Err(FigureAssetErrorV1::PackageNotRequired);
        }
        Ok(())
    }
}

pub fn completion_from_package_receipts(
    frame: &PresentationFrameV1,
    packages_and_receipts: &[(&CompiledFigurePackageV1, &PackageReceiptV1)],
) -> Result<RendererUploadCompletionV1, FigureAssetErrorV1> {
    if packages_and_receipts.len() != frame.renderer_required_resources().len() {
        return Err(FigureAssetErrorV1::IncompleteReceiptSet {
            required: frame.renderer_required_resources().len(),
            received: packages_and_receipts.len(),
        });
    }
    let mut completed = Vec::new();
    for (package, receipt) in packages_and_receipts {
        receipt.validate(frame, package)?;
        completed.push(package.package_digest);
    }
    completed.sort_unstable();
    if completed != frame.renderer_required_resources() {
        return Err(FigureAssetErrorV1::ReceiptResourceSetMismatch);
    }
    Ok(RendererUploadCompletionV1 {
        client_applied_generation: frame.generation().client_applied_generation,
        frame_digest: frame.frame_digest(),
        resource_set_digest: frame.resource_set_digest(),
        completed_resources: completed,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModularComponentV1 {
    pub slot: u16,
    pub role: FigureAssetRoleV1,
    pub package_digest: FigureDigestV1,
    pub authority_digest: FigureDigestV1,
    pub compatibility_digest: FigureDigestV1,
    pub required_section_identities: Vec<FigureDigestV1>,
    pub receipt_terminal: PackageReceiptTerminalV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModularCompositionKeyV1 {
    generation: u64,
    components: Vec<ModularComponentV1>,
    digest: FigureDigestV1,
}

impl ModularCompositionKeyV1 {
    pub fn new(
        generation: u64,
        mut components: Vec<ModularComponentV1>,
    ) -> Result<Self, FigureAssetErrorV1> {
        if generation == 0 {
            return Err(FigureAssetErrorV1::InvalidGeneration);
        }
        if components.is_empty() || components.len() > MAX_FIGURE_COMPOSITION_COMPONENTS_V1 {
            return Err(FigureAssetErrorV1::InvalidComponentCount(components.len()));
        }
        components.sort_by(|a, b| {
            a.slot
                .cmp(&b.slot)
                .then(a.role.cmp(&b.role))
                .then(a.package_digest.cmp(&b.package_digest))
        });
        let mut slots = BTreeSet::new();
        let mut body_count = 0_usize;
        let compatibility = components[0].compatibility_digest;
        if is_zero(&compatibility) {
            return Err(FigureAssetErrorV1::InvalidCompatibility);
        }
        for component in &components {
            if !slots.insert(component.slot) {
                return Err(FigureAssetErrorV1::DuplicateComponentSlot(component.slot));
            }
            if component.role == FigureAssetRoleV1::CoreBody {
                body_count += 1;
            }
            if component.receipt_terminal != PackageReceiptTerminalV1::Accepted {
                return Err(FigureAssetErrorV1::PartiallyPublishedComponent);
            }
            if is_zero(&component.package_digest)
                || is_zero(&component.authority_digest)
                || component.required_section_identities.is_empty()
                || component.compatibility_digest != compatibility
            {
                return Err(FigureAssetErrorV1::IncompatibleComponent);
            }
            if component
                .required_section_identities
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            {
                return Err(FigureAssetErrorV1::NonCanonical);
            }
        }
        if body_count != 1 {
            return Err(FigureAssetErrorV1::MissingOrDuplicateCoreBody(body_count));
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"BSTRCMP1");
        put_u64(&mut bytes, generation);
        put_count(&mut bytes, components.len())?;
        for component in &components {
            put_u16(&mut bytes, component.slot);
            bytes.push(component.role as u8);
            bytes.push(0);
            bytes.extend_from_slice(&component.package_digest);
            bytes.extend_from_slice(&component.authority_digest);
            bytes.extend_from_slice(&component.compatibility_digest);
            put_count(&mut bytes, component.required_section_identities.len())?;
            for section in &component.required_section_identities {
                bytes.extend_from_slice(section);
            }
        }
        let digest = Sha256::digest(&bytes).into();
        Ok(Self {
            generation,
            components,
            digest,
        })
    }

    pub fn authorize_generation(&self, generation: u64) -> Result<(), FigureAssetErrorV1> {
        if generation != self.generation {
            return Err(FigureAssetErrorV1::StaleComposition {
                expected: self.generation,
                actual: generation,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn components(&self) -> &[ModularComponentV1] { &self.components }

    #[must_use]
    pub const fn digest(&self) -> FigureDigestV1 { self.digest }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FigureAssetErrorV1 {
    Path(crate::figure_package::PathError),
    Package(FigurePackageErrorV1),
    Io(String),
    UnsupportedSchema(u16),
    UnsupportedCompiler(u16),
    UnknownRole(u8),
    UnknownTarget(u8),
    UnknownMaterial(u16),
    UnknownMaterialSlot(u16),
    InvalidAuthorityMagic,
    InvalidMaterialMagic,
    InvalidSectionMagic,
    InvalidIndexMagic,
    InvalidEpoch,
    MissingSources,
    MissingCoreBody,
    TooManySources(usize),
    TooManyMaterials(usize),
    EmptySource,
    SourceTooLarge(usize),
    SourcePathTooLong(usize),
    DuplicateSourcePath(String),
    DuplicateMaterialSlot(u16),
    InvalidMaterialSlot,
    FixtureForbiddenForRole(FigureAssetRoleV1),
    SourceCountMismatch,
    SourceProvenanceMismatch,
    MissingAuthoritySection,
    MissingMaterialSection,
    PackageDigestMismatch,
    CorruptCachePackage,
    CorruptPartialPublication,
    ProvenanceIndexMismatch,
    RollbackCleanup { primary: String, cleanup: String },
    ReceiptPublicationMismatch,
    PackageNotRequired,
    ReceiptRolledBack,
    ReceiptGenerationMismatch,
    ReceiptFrameMismatch,
    ReceiptPackageMismatch,
    ReceiptSectionMismatch,
    IncompleteReceiptSet { required: usize, received: usize },
    ReceiptResourceSetMismatch,
    InvalidGeneration,
    InvalidComponentCount(usize),
    DuplicateComponentSlot(u16),
    MissingOrDuplicateCoreBody(usize),
    InvalidCompatibility,
    IncompatibleComponent,
    PartiallyPublishedComponent,
    StaleComposition { expected: u64, actual: u64 },
    MalformedBoolean(u8),
    NonCanonical,
    Truncated,
    TrailingBytes(usize),
    LengthOverflow,
    AllocationFailure,
}

impl From<FigurePackageErrorV1> for FigureAssetErrorV1 {
    fn from(value: FigurePackageErrorV1) -> Self { Self::Package(value) }
}

fn validate_authority(authority: &FigurePackageAuthorityV1) -> Result<(), FigureAssetErrorV1> {
    if authority.schema_version != FIGURE_ASSET_SCHEMA_VERSION_V1 {
        return Err(FigureAssetErrorV1::UnsupportedSchema(
            authority.schema_version,
        ));
    }
    if authority.compiler_version != FIGURE_COMPILER_VERSION_V1 {
        return Err(FigureAssetErrorV1::UnsupportedCompiler(
            authority.compiler_version,
        ));
    }
    if is_zero(&authority.asset_epoch) || is_zero(&authority.corpus_epoch) {
        return Err(FigureAssetErrorV1::InvalidEpoch);
    }
    if authority.sources.is_empty() || authority.sources.len() > MAX_FIGURE_SOURCES_V1 {
        return Err(FigureAssetErrorV1::TooManySources(authority.sources.len()));
    }
    let mut previous: Option<&FigureSourceAuthorityV1> = None;
    let mut paths = BTreeSet::new();
    for source in &authority.sources {
        let path =
            normalize_machine_path(&source.logical_path).map_err(FigureAssetErrorV1::Path)?;
        if path.len() > MAX_FIGURE_SOURCE_PATH_BYTES_V1 {
            return Err(FigureAssetErrorV1::SourcePathTooLong(path.len()));
        }
        if !paths.insert(path.clone()) {
            return Err(FigureAssetErrorV1::DuplicateSourcePath(path));
        }
        if source.size == 0
            || source.size
                > u64::try_from(MAX_FIGURE_SOURCE_BYTES_V1)
                    .map_err(|_| FigureAssetErrorV1::LengthOverflow)?
            || is_zero(&source.content_sha256)
            || (source.deterministic_fixture && !source.role.fixture_allowed())
        {
            return Err(FigureAssetErrorV1::SourceProvenanceMismatch);
        }
        if previous.is_some_and(|value| !source_authority_cmp(value, source).is_lt()) {
            return Err(FigureAssetErrorV1::NonCanonical);
        }
        previous = Some(source);
    }
    Ok(())
}

fn validate_materials(materials: &mut Vec<MaterialBindingV1>) -> Result<(), FigureAssetErrorV1> {
    if materials.is_empty() || materials.len() > MAX_FIGURE_MATERIALS_V1 {
        return Err(FigureAssetErrorV1::TooManyMaterials(materials.len()));
    }
    materials.sort_unstable();
    let mut slots = BTreeSet::new();
    for material in materials {
        if material.slot == 0 {
            return Err(FigureAssetErrorV1::InvalidMaterialSlot);
        }
        if !slots.insert(material.slot) {
            return Err(FigureAssetErrorV1::DuplicateMaterialSlot(material.slot));
        }
    }
    Ok(())
}

fn source_authority_cmp(
    left: &FigureSourceAuthorityV1,
    right: &FigureSourceAuthorityV1,
) -> std::cmp::Ordering {
    left.role
        .cmp(&right.role)
        .then(left.logical_path.cmp(&right.logical_path))
        .then(left.content_sha256.cmp(&right.content_sha256))
        .then(left.material_slot.cmp(&right.material_slot))
        .then(left.deterministic_fixture.cmp(&right.deterministic_fixture))
}

fn encode_materials(materials: &[MaterialBindingV1]) -> Result<Vec<u8>, FigureAssetErrorV1> {
    let mut values = materials.to_vec();
    validate_materials(&mut values)?;
    let mut output = Vec::new();
    output.extend_from_slice(MATERIAL_MAGIC);
    put_u16(&mut output, FIGURE_ASSET_SCHEMA_VERSION_V1);
    put_count(&mut output, values.len())?;
    for value in values {
        put_u16(&mut output, value.slot);
        put_u16(&mut output, value.kind as u16);
        output.extend_from_slice(&value.base_color_rgba);
        put_u16(&mut output, value.flags);
    }
    Ok(output)
}

fn decode_materials(bytes: &[u8]) -> Result<Vec<MaterialBindingV1>, FigureAssetErrorV1> {
    let mut reader = AssetReaderV1::new(bytes);
    if reader.take(8)? != MATERIAL_MAGIC {
        return Err(FigureAssetErrorV1::InvalidMaterialMagic);
    }
    let version = reader.u16()?;
    if version != FIGURE_ASSET_SCHEMA_VERSION_V1 {
        return Err(FigureAssetErrorV1::UnsupportedSchema(version));
    }
    let count = reader.count(MAX_FIGURE_MATERIALS_V1)?;
    let mut materials = try_vec(count)?;
    for _ in 0..count {
        materials.push(MaterialBindingV1 {
            slot: reader.u16()?,
            kind: MaterialKindV1::from_u16(reader.u16()?)?,
            base_color_rgba: reader
                .take(4)?
                .try_into()
                .map_err(|_| FigureAssetErrorV1::Truncated)?,
            flags: reader.u16()?,
        });
    }
    if reader.remaining() != 0 {
        return Err(FigureAssetErrorV1::TrailingBytes(reader.remaining()));
    }
    validate_materials(&mut materials)?;
    if encode_materials(&materials)?.as_slice() != bytes {
        return Err(FigureAssetErrorV1::NonCanonical);
    }
    Ok(materials)
}

fn encode_source_section(
    source: &FigureSourceAuthorityV1,
    payload: &[u8],
) -> Result<Vec<u8>, FigureAssetErrorV1> {
    if Sha256::digest(payload).as_slice() != source.content_sha256 {
        return Err(FigureAssetErrorV1::SourceProvenanceMismatch);
    }
    let mut output = Vec::new();
    output.extend_from_slice(SECTION_MAGIC);
    put_u16(&mut output, FIGURE_ASSET_SCHEMA_VERSION_V1);
    output.push(source.role as u8);
    output.push(u8::from(source.deterministic_fixture));
    put_u16(&mut output, source.material_slot);
    put_text(&mut output, &source.logical_path)?;
    output.extend_from_slice(&source.content_sha256);
    put_u64(&mut output, source.size);
    output.extend_from_slice(payload);
    Ok(output)
}

fn decode_source_section(bytes: &[u8]) -> Result<FigureSourceInputV1, FigureAssetErrorV1> {
    let mut reader = AssetReaderV1::new(bytes);
    if reader.take(8)? != SECTION_MAGIC {
        return Err(FigureAssetErrorV1::InvalidSectionMagic);
    }
    let version = reader.u16()?;
    if version != FIGURE_ASSET_SCHEMA_VERSION_V1 {
        return Err(FigureAssetErrorV1::UnsupportedSchema(version));
    }
    let role = FigureAssetRoleV1::from_u8(reader.u8()?)?;
    let deterministic_fixture = match reader.u8()? {
        0 => false,
        1 => true,
        other => return Err(FigureAssetErrorV1::MalformedBoolean(other)),
    };
    let material_slot = reader.u16()?;
    let logical_path = reader.text(MAX_FIGURE_SOURCE_PATH_BYTES_V1)?;
    let expected_digest = reader.digest()?;
    let size = usize::try_from(reader.u64()?).map_err(|_| FigureAssetErrorV1::LengthOverflow)?;
    if size == 0 || size > MAX_FIGURE_SOURCE_BYTES_V1 {
        return Err(FigureAssetErrorV1::SourceTooLarge(size));
    }
    let payload = reader.take(size)?.to_vec();
    if reader.remaining() != 0 {
        return Err(FigureAssetErrorV1::TrailingBytes(reader.remaining()));
    }
    if Sha256::digest(&payload).as_slice() != expected_digest {
        return Err(FigureAssetErrorV1::SourceProvenanceMismatch);
    }
    Ok(FigureSourceInputV1 {
        logical_path,
        role,
        material_slot,
        bytes: payload,
        deterministic_fixture,
    })
}

fn encode_index(authority: FigureDigestV1, package: FigureDigestV1) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + 32 + 32 + 32);
    bytes.extend_from_slice(INDEX_MAGIC);
    bytes.extend_from_slice(&authority);
    bytes.extend_from_slice(&package);
    let digest = Sha256::digest(&bytes);
    bytes.extend_from_slice(&digest);
    bytes
}

fn decode_index(bytes: &[u8]) -> Result<(FigureDigestV1, FigureDigestV1), FigureAssetErrorV1> {
    if bytes.len() != 8 + 32 + 32 + 32 || &bytes[..8] != INDEX_MAGIC {
        return Err(FigureAssetErrorV1::InvalidIndexMagic);
    }
    let expected: FigureDigestV1 = Sha256::digest(&bytes[..72]).into();
    if bytes[72..] != expected {
        return Err(FigureAssetErrorV1::ProvenanceIndexMismatch);
    }
    let authority = bytes[8..40]
        .try_into()
        .map_err(|_| FigureAssetErrorV1::Truncated)?;
    let package = bytes[40..72]
        .try_into()
        .map_err(|_| FigureAssetErrorV1::Truncated)?;
    Ok((authority, package))
}

fn prepare_file(path: &Path, bytes: &[u8]) -> Result<(), FigureAssetErrorV1> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(bytes)
                .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))?;
            file.sync_all()
                .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))?;
            Ok(())
        },
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read(path)? == bytes {
                Ok(())
            } else {
                Err(FigureAssetErrorV1::CorruptCachePackage)
            }
        },
        Err(error) => Err(FigureAssetErrorV1::Io(error.to_string())),
    }
}

fn create_dir_all(path: &Path) -> Result<(), FigureAssetErrorV1> {
    fs::create_dir_all(path).map_err(|error| FigureAssetErrorV1::Io(error.to_string()))
}

fn read(path: &Path) -> Result<Vec<u8>, FigureAssetErrorV1> {
    fs::read(path).map_err(|error| FigureAssetErrorV1::Io(error.to_string()))
}

fn remove_transaction(path: &Path) -> Result<(), FigureAssetErrorV1> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FigureAssetErrorV1::Io(error.to_string())),
    }
}

fn cleanup_failed_transaction<T>(
    transaction: &Path,
    primary: FigureAssetErrorV1,
) -> Result<T, FigureAssetErrorV1> {
    match remove_transaction(transaction) {
        Ok(()) => Err(primary),
        Err(cleanup) => Err(FigureAssetErrorV1::RollbackCleanup {
            primary: format!("{primary:?}"),
            cleanup: format!("{cleanup:?}"),
        }),
    }
}

fn validate_transaction(
    transaction: &Path,
    package_name: &str,
    package_bytes: &[u8],
    index_bytes: &[u8],
) -> Result<(), FigureAssetErrorV1> {
    let actual = fs::read_dir(transaction)
        .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))?
        .map(|entry| {
            entry
                .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))
                .and_then(|entry| {
                    entry
                        .file_name()
                        .into_string()
                        .map_err(|_| FigureAssetErrorV1::NonCanonical)
                })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let allowed = BTreeSet::from([package_name.to_string(), "authority.idx".to_string()]);
    if !actual.is_subset(&allowed)
        || (actual.contains(package_name)
            && read(&transaction.join(package_name))? != package_bytes)
        || (actual.contains("authority.idx")
            && read(&transaction.join("authority.idx"))? != index_bytes)
    {
        return Err(FigureAssetErrorV1::CorruptPartialPublication);
    }
    Ok(())
}

fn validate_cache_entry(
    entry: &Path,
    authority_digest: FigureDigestV1,
    package_digest: FigureDigestV1,
    package_bytes: &[u8],
) -> Result<(), FigureAssetErrorV1> {
    let package_name = format!("{}.fpkg", hex_digest(&package_digest));
    let expected = BTreeSet::from([package_name.clone(), "authority.idx".to_string()]);
    let actual = fs::read_dir(entry)
        .map_err(|error| FigureAssetErrorV1::Io(error.to_string()))?
        .map(|item| {
            item.map_err(|error| FigureAssetErrorV1::Io(error.to_string()))
                .and_then(|item| {
                    item.file_name()
                        .into_string()
                        .map_err(|_| FigureAssetErrorV1::NonCanonical)
                })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if actual != expected {
        return Err(FigureAssetErrorV1::CorruptPartialPublication);
    }
    if read(&entry.join(&package_name))? != package_bytes {
        return Err(FigureAssetErrorV1::CorruptCachePackage);
    }
    let (indexed_authority, indexed_package) = decode_index(&read(&entry.join("authority.idx"))?)?;
    if indexed_authority != authority_digest || indexed_package != package_digest {
        return Err(FigureAssetErrorV1::ProvenanceIndexMismatch);
    }
    Ok(())
}

fn trailing_digest(bytes: &[u8]) -> Result<FigureDigestV1, FigureAssetErrorV1> {
    bytes
        .get(
            bytes
                .len()
                .checked_sub(32)
                .ok_or(FigureAssetErrorV1::Truncated)?..,
        )
        .ok_or(FigureAssetErrorV1::Truncated)?
        .try_into()
        .map_err(|_| FigureAssetErrorV1::Truncated)
}

fn hex_digest(digest: &FigureDigestV1) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn is_zero(value: &FigureDigestV1) -> bool { value.iter().all(|byte| *byte == 0) }

fn put_u16(output: &mut Vec<u8>, value: u16) { output.extend_from_slice(&value.to_le_bytes()); }

fn put_u64(output: &mut Vec<u8>, value: u64) { output.extend_from_slice(&value.to_le_bytes()); }

fn put_count(output: &mut Vec<u8>, count: usize) -> Result<(), FigureAssetErrorV1> {
    let count = u16::try_from(count).map_err(|_| FigureAssetErrorV1::LengthOverflow)?;
    put_u16(output, count);
    Ok(())
}

fn put_text(output: &mut Vec<u8>, text: &str) -> Result<(), FigureAssetErrorV1> {
    let length = u16::try_from(text.len()).map_err(|_| FigureAssetErrorV1::LengthOverflow)?;
    put_u16(output, length);
    output.extend_from_slice(text.as_bytes());
    Ok(())
}

fn try_vec<T>(count: usize) -> Result<Vec<T>, FigureAssetErrorV1> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| FigureAssetErrorV1::AllocationFailure)?;
    Ok(values)
}

struct AssetReaderV1<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> AssetReaderV1<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    fn take(&mut self, count: usize) -> Result<&'a [u8], FigureAssetErrorV1> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(FigureAssetErrorV1::LengthOverflow)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(FigureAssetErrorV1::Truncated)?;
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, FigureAssetErrorV1> {
        Ok(*self.take(1)?.first().ok_or(FigureAssetErrorV1::Truncated)?)
    }

    fn u16(&mut self) -> Result<u16, FigureAssetErrorV1> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| FigureAssetErrorV1::Truncated)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, FigureAssetErrorV1> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| FigureAssetErrorV1::Truncated)?,
        ))
    }

    fn digest(&mut self) -> Result<FigureDigestV1, FigureAssetErrorV1> {
        self.take(32)?
            .try_into()
            .map_err(|_| FigureAssetErrorV1::Truncated)
    }

    fn count(&mut self, maximum: usize) -> Result<usize, FigureAssetErrorV1> {
        let value = usize::from(self.u16()?);
        if value > maximum {
            return Err(FigureAssetErrorV1::LengthOverflow);
        }
        Ok(value)
    }

    fn text(&mut self, maximum: usize) -> Result<String, FigureAssetErrorV1> {
        let length = self.count(maximum)?;
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| FigureAssetErrorV1::NonCanonical)?;
        Ok(value.to_owned())
    }

    fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.position) }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::{
        hex_bytes,
        presentation::{
            PresentationEnvironmentV1, PresentationFrameDraftV1, PresentationVisualPolicyV1,
        },
    };

    static TEMP_ORDINAL: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let ordinal = TEMP_ORDINAL.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bastion-r1bc-{label}-{}-{ordinal}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    fn material(slot: u16) -> MaterialBindingV1 {
        MaterialBindingV1 {
            slot,
            kind: MaterialKindV1::OpaqueVoxel,
            base_color_rgba: [10, 20, 30, 255],
            flags: 0,
        }
    }

    fn source(path: &str, role: FigureAssetRoleV1, bytes: &[u8]) -> FigureSourceInputV1 {
        FigureSourceInputV1 {
            logical_path: path.to_string(),
            role,
            material_slot: 1,
            bytes: bytes.to_vec(),
            deterministic_fixture: role.fixture_allowed(),
        }
    }

    fn package_with_order(reverse: bool) -> CompiledFigurePackageV1 {
        let mut sources = vec![
            source(
                "voxygen/voxel/figure/head/dwarf/female.vox",
                FigureAssetRoleV1::CoreBody,
                b"body-source",
            ),
            source(
                "voxygen/voxel/armor/warlord/chest.vox",
                FigureAssetRoleV1::Equipment,
                b"equipment-source",
            ),
            source(
                "r1bc/fixtures/humanoid/lod0.bin",
                FigureAssetRoleV1::Lod,
                b"lod-fixture-v1",
            ),
        ];
        if reverse {
            sources.reverse();
        }
        CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            [7; 32],
            [8; 32],
            vec![material(1)],
            sources,
        )
        .unwrap()
    }

    fn frame(package: &CompiledFigurePackageV1, generation: u64) -> PresentationFrameV1 {
        PresentationFrameDraftV1 {
            generation: PresentationGenerationV1 {
                run_epoch: 1,
                client_applied_generation: generation,
                simulation_tick: 300,
                coherent_snapshot_root: [9; 32],
            },
            entities: Vec::new(),
            groups: Vec::new(),
            events: Vec::new(),
            environment: PresentationEnvironmentV1 {
                terrain_root: [1; 32],
                environment_digest: [2; 32],
                cloud_milli: 0,
                rain_milli: 0,
                wind_mm_s: [0, 0],
                daylight_milli: 500,
            },
            visual_policy: PresentationVisualPolicyV1 {
                policy_digest: [3; 32],
                terrain_view_distance: 4,
                entity_view_distance: 4,
                figure_lod_distance: 4,
                sprite_distance: 4,
                particles_enabled: false,
                weapon_trails_enabled: false,
                flashing_lights_enabled: false,
            },
            renderer_required_resources: vec![package.package_digest()],
            complete: true,
        }
        .seal()
        .unwrap()
    }

    #[test]
    fn frozen_package_vector_and_exact_decode() {
        let package = package_with_order(false);
        assert_eq!(
            hex_bytes(&package.package_digest()),
            "25d4a021d550d61da55690e367dd1ec6e3c8fe400d72bc40267014a30c3f37dc"
        );
        let decoded = CompiledFigurePackageV1::decode_exact(package.canonical_bytes()).unwrap();
        assert_eq!(decoded.canonical_bytes(), package.canonical_bytes());
        assert_eq!(decoded.package_digest(), package.package_digest());
    }

    #[test]
    fn source_enumeration_and_material_order_do_not_change_bytes() {
        let a = package_with_order(false);
        let b = package_with_order(true);
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.authority_digest(), b.authority_digest());

        let mut second = material(2);
        second.kind = MaterialKindV1::MetallicVoxel;
        let sources = vec![
            source(
                "voxygen/voxel/figure/head/dwarf/female.vox",
                FigureAssetRoleV1::CoreBody,
                b"body-source",
            ),
            FigureSourceInputV1 {
                logical_path: "voxygen/voxel/weapon/sword/starter.vox".to_string(),
                role: FigureAssetRoleV1::Attachment,
                material_slot: 2,
                bytes: b"attachment-source".to_vec(),
                deterministic_fixture: false,
            },
        ];
        let c = CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            [1; 32],
            [2; 32],
            vec![material(1), second.clone()],
            sources.clone(),
        )
        .unwrap();
        let d = CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            [1; 32],
            [2; 32],
            vec![second, material(1)],
            sources,
        )
        .unwrap();
        assert_eq!(c.canonical_bytes(), d.canonical_bytes());
    }

    #[test]
    fn every_source_field_is_provenance_sensitive() {
        let base = package_with_order(false);
        for mutation in [
            ("path", 0_u8),
            ("bytes", 1),
            ("epoch", 2),
            ("corpus", 3),
            ("material", 4),
        ] {
            let changed = match mutation.1 {
                0 => CompiledFigurePackageV1::compile(
                    FigurePackageTargetV1::Composite,
                    [7; 32],
                    [8; 32],
                    vec![material(1)],
                    vec![
                        source(
                            "voxygen/voxel/figure/head/dwarf/male.vox",
                            FigureAssetRoleV1::CoreBody,
                            b"body-source",
                        ),
                        source(
                            "voxygen/voxel/armor/warlord/chest.vox",
                            FigureAssetRoleV1::Equipment,
                            b"equipment-source",
                        ),
                    ],
                )
                .unwrap(),
                1 => CompiledFigurePackageV1::compile(
                    FigurePackageTargetV1::Composite,
                    [7; 32],
                    [8; 32],
                    vec![material(1)],
                    vec![
                        source(
                            "voxygen/voxel/figure/head/dwarf/female.vox",
                            FigureAssetRoleV1::CoreBody,
                            b"changed",
                        ),
                        source(
                            "voxygen/voxel/armor/warlord/chest.vox",
                            FigureAssetRoleV1::Equipment,
                            b"equipment-source",
                        ),
                    ],
                )
                .unwrap(),
                2 => {
                    let mut value = package_with_order(false);
                    value = CompiledFigurePackageV1::compile(
                        FigurePackageTargetV1::Composite,
                        [6; 32],
                        [8; 32],
                        value.materials().to_vec(),
                        vec![
                            source(
                                "voxygen/voxel/figure/head/dwarf/female.vox",
                                FigureAssetRoleV1::CoreBody,
                                b"body-source",
                            ),
                            source(
                                "voxygen/voxel/armor/warlord/chest.vox",
                                FigureAssetRoleV1::Equipment,
                                b"equipment-source",
                            ),
                            source(
                                "r1bc/fixtures/humanoid/lod0.bin",
                                FigureAssetRoleV1::Lod,
                                b"lod-fixture-v1",
                            ),
                        ],
                    )
                    .unwrap();
                    value
                },
                3 => CompiledFigurePackageV1::compile(
                    FigurePackageTargetV1::Composite,
                    [7; 32],
                    [6; 32],
                    vec![material(1)],
                    vec![
                        source(
                            "voxygen/voxel/figure/head/dwarf/female.vox",
                            FigureAssetRoleV1::CoreBody,
                            b"body-source",
                        ),
                        source(
                            "voxygen/voxel/armor/warlord/chest.vox",
                            FigureAssetRoleV1::Equipment,
                            b"equipment-source",
                        ),
                    ],
                )
                .unwrap(),
                _ => {
                    let mut changed_material = material(1);
                    changed_material.kind = MaterialKindV1::MetallicVoxel;
                    CompiledFigurePackageV1::compile(
                        FigurePackageTargetV1::Composite,
                        [7; 32],
                        [8; 32],
                        vec![changed_material],
                        vec![
                            source(
                                "voxygen/voxel/figure/head/dwarf/female.vox",
                                FigureAssetRoleV1::CoreBody,
                                b"body-source",
                            ),
                            source(
                                "voxygen/voxel/armor/warlord/chest.vox",
                                FigureAssetRoleV1::Equipment,
                                b"equipment-source",
                            ),
                        ],
                    )
                    .unwrap()
                },
            };
            assert_ne!(
                base.package_digest(),
                changed.package_digest(),
                "{}",
                mutation.0
            );
        }
    }

    #[test]
    fn malformed_partial_duplicate_and_oversize_inputs_reject() {
        let duplicate = CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Composite,
            [1; 32],
            [2; 32],
            vec![material(1)],
            vec![
                source("a/body.vox", FigureAssetRoleV1::CoreBody, b"a"),
                source("a/body.vox", FigureAssetRoleV1::Equipment, b"b"),
            ],
        );
        assert!(matches!(
            duplicate,
            Err(FigureAssetErrorV1::DuplicateSourcePath(_))
        ));
        let oversize = CompiledFigurePackageV1::compile(
            FigurePackageTargetV1::Body,
            [1; 32],
            [2; 32],
            vec![material(1)],
            vec![source(
                "a/body.vox",
                FigureAssetRoleV1::CoreBody,
                &vec![1; MAX_FIGURE_SOURCE_BYTES_V1 + 1],
            )],
        );
        assert!(matches!(
            oversize,
            Err(FigureAssetErrorV1::SourceTooLarge(_))
        ));
        let package = package_with_order(false);
        let mut corrupt = package.canonical_bytes().to_vec();
        corrupt[20] ^= 1;
        assert!(CompiledFigurePackageV1::decode_exact(&corrupt).is_err());
        let truncated = &package.canonical_bytes()[..package.canonical_bytes().len() - 1];
        assert!(CompiledFigurePackageV1::decode_exact(truncated).is_err());
        let mut trailing = package.canonical_bytes().to_vec();
        trailing.push(0);
        assert!(CompiledFigurePackageV1::decode_exact(&trailing).is_err());
        assert!(matches!(
            CompiledFigurePackageV1::compile(
                FigurePackageTargetV1::Body,
                [1; 32],
                [2; 32],
                vec![material(1), material(1)],
                vec![source("a/body.vox", FigureAssetRoleV1::CoreBody, b"a")],
            ),
            Err(FigureAssetErrorV1::DuplicateMaterialSlot(1))
        ));
    }

    #[test]
    fn fixture_policy_is_closed_to_future_forms() {
        let mut body = source("fixture/body.vox", FigureAssetRoleV1::CoreBody, b"x");
        body.deterministic_fixture = true;
        assert!(matches!(
            CompiledFigurePackageV1::compile(
                FigurePackageTargetV1::Body,
                [1; 32],
                [2; 32],
                vec![material(1)],
                vec![body],
            ),
            Err(FigureAssetErrorV1::FixtureForbiddenForRole(
                FigureAssetRoleV1::CoreBody
            ))
        ));
        for role in [
            FigureAssetRoleV1::Lod,
            FigureAssetRoleV1::ShadowProxy,
            FigureAssetRoleV1::Impostor,
        ] {
            let mut sources = vec![source(
                "real/body.vox",
                FigureAssetRoleV1::CoreBody,
                b"body",
            )];
            sources.push(source(
                &format!("r1bc/fixtures/{role:?}.bin").to_ascii_lowercase(),
                role,
                b"fixture",
            ));
            assert!(
                CompiledFigurePackageV1::compile(
                    FigurePackageTargetV1::Composite,
                    [1; 32],
                    [2; 32],
                    vec![material(1)],
                    sources,
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn cold_warm_and_duplicate_publication_converge() {
        let package = package_with_order(false);
        let root = TestDir::new("cache");
        let cache = FigurePackageCacheV1::new(&root.0);
        let cold = cache
            .publish(&package, CachePublicationPolicyV1::Commit)
            .unwrap();
        assert_eq!(cold.terminal, CachePublicationTerminalV1::Published);
        let warm = cache
            .publish(&package, CachePublicationPolicyV1::Commit)
            .unwrap();
        assert_eq!(warm.terminal, CachePublicationTerminalV1::ExistingIdentical);
        let loaded = cache.load(package.authority_digest()).unwrap();
        assert_eq!(loaded.canonical_bytes(), package.canonical_bytes());
    }

    #[test]
    fn rollback_preserves_prior_package_and_index() {
        let package = package_with_order(false);
        let root = TestDir::new("rollback");
        let cache = FigurePackageCacheV1::new(&root.0);
        let first_rollback = cache
            .publish(&package, CachePublicationPolicyV1::RollbackBeforePublish)
            .unwrap();
        assert_eq!(
            first_rollback.terminal,
            CachePublicationTerminalV1::RolledBack
        );
        assert_eq!(fs::read_dir(root.0.join("staging")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(root.0.join("entries")).unwrap().count(), 0);
        cache
            .publish(&package, CachePublicationPolicyV1::Commit)
            .unwrap();
        let before = cache
            .load(package.authority_digest())
            .unwrap()
            .canonical_bytes()
            .to_vec();
        let rolled = cache
            .publish(&package, CachePublicationPolicyV1::RollbackBeforePublish)
            .unwrap();
        assert_eq!(rolled.terminal, CachePublicationTerminalV1::RolledBack);
        assert_eq!(
            cache
                .load(package.authority_digest())
                .unwrap()
                .canonical_bytes(),
            before
        );
        assert_eq!(
            fs::read_dir(root.0.join("staging")).unwrap().count(),
            0,
            "rollback must leave no partial publication"
        );
    }

    #[test]
    fn corrupted_cache_and_wrong_provenance_reject() {
        let package = package_with_order(false);
        let root = TestDir::new("corrupt");
        let cache = FigurePackageCacheV1::new(&root.0);
        cache
            .publish(&package, CachePublicationPolicyV1::Commit)
            .unwrap();
        let entry_path = root
            .0
            .join("entries")
            .join(hex_digest(&package.authority_digest()));
        let package_path =
            entry_path.join(format!("{}.fpkg", hex_digest(&package.package_digest())));
        fs::write(&package_path, b"corrupt").unwrap();
        assert!(cache.load(package.authority_digest()).is_err());
        fs::write(&package_path, package.canonical_bytes()).unwrap();
        let index_path = entry_path.join("authority.idx");
        let mut index = fs::read(&index_path).unwrap();
        index[10] ^= 1;
        fs::write(&index_path, index).unwrap();
        assert!(matches!(
            cache.load(package.authority_digest()),
            Err(FigureAssetErrorV1::ProvenanceIndexMismatch)
        ));
        fs::write(
            &index_path,
            encode_index(package.authority_digest(), package.package_digest()),
        )
        .unwrap();
        fs::write(entry_path.join("unexpected.extra"), b"x").unwrap();
        assert!(matches!(
            cache.load(package.authority_digest()),
            Err(FigureAssetErrorV1::CorruptPartialPublication)
        ));
    }

    #[test]
    fn malformed_staging_transaction_rejects_without_publication() {
        let package = package_with_order(false);
        let root = TestDir::new("partial");
        let cache = FigurePackageCacheV1::new(&root.0);
        let transaction = root.0.join("staging").join(format!(
            "{}-{}.txn",
            hex_digest(&package.authority_digest()),
            hex_digest(&package.package_digest())
        ));
        fs::create_dir_all(&transaction).unwrap();
        fs::write(transaction.join("unexpected"), b"partial").unwrap();
        assert!(matches!(
            cache.publish(&package, CachePublicationPolicyV1::Commit),
            Err(FigureAssetErrorV1::CorruptPartialPublication)
        ));
        assert_eq!(
            fs::read_dir(root.0.join("entries")).unwrap().count(),
            0,
            "malformed staging must not publish a final entry"
        );
    }

    #[test]
    fn exact_receipts_complete_presentation_and_stale_or_rollback_reject() {
        let package = package_with_order(false);
        let root = TestDir::new("receipt");
        let cache = FigurePackageCacheV1::new(&root.0);
        let publication = cache
            .publish(&package, CachePublicationPolicyV1::Commit)
            .unwrap();
        let accepted_frame = frame(&package, 7);
        let receipt =
            PackageReceiptV1::from_publication(&accepted_frame, &package, &publication).unwrap();
        let completion =
            completion_from_package_receipts(&accepted_frame, &[(&package, &receipt)]).unwrap();
        assert_eq!(completion.client_applied_generation, 7);
        assert_eq!(completion.completed_resources, vec![
            package.package_digest()
        ]);

        let stale_frame = frame(&package, 8);
        assert_eq!(
            receipt.validate(&stale_frame, &package),
            Err(FigureAssetErrorV1::ReceiptGenerationMismatch)
        );
        let rolled_publication = cache
            .publish(&package, CachePublicationPolicyV1::RollbackBeforePublish)
            .unwrap();
        let rolled =
            PackageReceiptV1::from_publication(&accepted_frame, &package, &rolled_publication)
                .unwrap();
        assert_eq!(
            rolled.validate(&accepted_frame, &package),
            Err(FigureAssetErrorV1::ReceiptRolledBack)
        );
    }

    #[test]
    fn partial_mismatched_and_duplicate_receipts_reject() {
        let package = package_with_order(false);
        let frame = frame(&package, 3);
        assert!(matches!(
            completion_from_package_receipts(&frame, &[]),
            Err(FigureAssetErrorV1::IncompleteReceiptSet {
                required: 1,
                received: 0
            })
        ));
        let publication = CachePublicationRecordV1 {
            authority_digest: package.authority_digest(),
            package_digest: package.package_digest(),
            terminal: CachePublicationTerminalV1::Published,
        };
        let mut receipt =
            PackageReceiptV1::from_publication(&frame, &package, &publication).unwrap();
        receipt.required_section_identities.pop();
        assert_eq!(
            receipt.validate(&frame, &package),
            Err(FigureAssetErrorV1::ReceiptSectionMismatch)
        );
    }

    #[test]
    fn modular_composition_is_order_independent_and_fail_closed() {
        let package = package_with_order(false);
        let sections = package.required_section_identities().to_vec();
        let body = ModularComponentV1 {
            slot: 1,
            role: FigureAssetRoleV1::CoreBody,
            package_digest: package.package_digest(),
            authority_digest: package.authority_digest(),
            compatibility_digest: [4; 32],
            required_section_identities: sections.clone(),
            receipt_terminal: PackageReceiptTerminalV1::Accepted,
        };
        let equipment = ModularComponentV1 {
            slot: 2,
            role: FigureAssetRoleV1::Equipment,
            package_digest: [6; 32],
            authority_digest: [8; 32],
            compatibility_digest: [4; 32],
            required_section_identities: vec![[7; 32]],
            receipt_terminal: PackageReceiptTerminalV1::Accepted,
        };
        let a = ModularCompositionKeyV1::new(9, vec![body.clone(), equipment.clone()]).unwrap();
        let b = ModularCompositionKeyV1::new(9, vec![equipment.clone(), body.clone()]).unwrap();
        assert_eq!(a.digest(), b.digest());
        assert_eq!(
            a.authorize_generation(8),
            Err(FigureAssetErrorV1::StaleComposition {
                expected: 9,
                actual: 8
            })
        );
        assert!(matches!(
            ModularCompositionKeyV1::new(9, vec![equipment]),
            Err(FigureAssetErrorV1::MissingOrDuplicateCoreBody(0))
        ));
        let mut rolled = body.clone();
        rolled.receipt_terminal = PackageReceiptTerminalV1::RolledBack;
        assert_eq!(
            ModularCompositionKeyV1::new(9, vec![rolled]),
            Err(FigureAssetErrorV1::PartiallyPublishedComponent)
        );
        let mut incompatible = body.clone();
        incompatible.slot = 3;
        incompatible.role = FigureAssetRoleV1::Attachment;
        incompatible.compatibility_digest = [5; 32];
        assert_eq!(
            ModularCompositionKeyV1::new(9, vec![body.clone(), incompatible]),
            Err(FigureAssetErrorV1::IncompatibleComponent)
        );
        let mut duplicate = body.clone();
        duplicate.role = FigureAssetRoleV1::Equipment;
        assert_eq!(
            ModularCompositionKeyV1::new(9, vec![body, duplicate]),
            Err(FigureAssetErrorV1::DuplicateComponentSlot(1))
        );
    }
}
