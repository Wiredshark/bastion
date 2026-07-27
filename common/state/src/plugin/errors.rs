use bincode::error::DecodeError;
use std::path::PathBuf;

#[derive(Debug)]
pub enum PluginError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    NoConfig,
    NoSuchModule,
    Encoding(Box<DecodeError>),
    PluginModuleError(String, String, PluginModuleError),
    FromDirDoesNotExist,
    // APEX-T2.1.01: phase-typed wrappers. Public callers keep matching on
    // `PluginError`; the phase variants make batch orchestration typed (every
    // failure names its phase + plugin/path identity, so an all-or-none batch
    // can prove WHERE it stopped and that nothing later ran).
    Inspection(PluginInspectionError),
    Instantiation(PluginInstantiationError),
    AssetPreparation(PluginAssetPreparationError),
    AssetCommit(PluginAssetCommitError),
}

#[derive(Debug)]
pub enum PluginModuleError {
    Wasmtime(wasmtime::Error),
}

/// APEX-T2.1.01 — inspection-phase failures (archive/config/manifest reading;
/// NO Wasmtime, NO ECS, NO global assets are reachable from this phase).
#[derive(Debug)]
pub enum PluginInspectionError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    ArchiveEntries {
        path: PathBuf,
        source: std::io::Error,
    },
    /// DET-AST-019 (CLOSED on this line, preserved): duplicate exact archive
    /// paths are rejected fail-closed — the occurrences remain visible in the
    /// inspection inventory for T2.2's canonical-profile decision.
    DuplicateArchivePath {
        path: PathBuf,
        duplicate: PathBuf,
    },
    NoConfig {
        path: PathBuf,
    },
    ConfigEncoding {
        path: PathBuf,
        source: Box<DecodeError>,
    },
    ConfigToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    MissingDeclaredModule {
        plugin: String,
        module: PathBuf,
    },
    DirectoryEntry {
        directory: PathBuf,
        source: std::io::Error,
    },
}

/// APEX-T2.1.01 — private guest-preparation failures (Wasmtime compile /
/// instantiate). A failure here must leave manager + global assets untouched.
#[derive(Debug)]
pub enum PluginInstantiationError {
    Module {
        plugin: String,
        module: PathBuf,
        source: PluginModuleError,
    },
}

/// APEX-T2.1.01 — private asset-source preparation failures (tar → cache,
/// digest revalidation). No `plugin_list` mutation is reachable from here.
#[derive(Debug)]
pub enum PluginAssetPreparationError {
    ArchiveChangedAfterInspection {
        path: PathBuf,
        expected: [u8; 32],
        observed: [u8; 32],
    },
    TarSource {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// APEX-T2.1.01 — batch-commit failures (the single write-lock acquisition).
#[derive(Debug)]
pub enum PluginAssetCommitError {
    RegistryLockPoisoned,
    /// APEX-T2.5.12: the incremental commit was refused because a content
    /// generation seals the registry, or the one-time generation install
    /// itself was refused (already installed / legacy publication present).
    GenerationRefused { detail: &'static str },
}

impl From<PluginInspectionError> for PluginError {
    fn from(e: PluginInspectionError) -> Self { PluginError::Inspection(e) }
}
impl From<PluginInstantiationError> for PluginError {
    fn from(e: PluginInstantiationError) -> Self { PluginError::Instantiation(e) }
}
impl From<PluginAssetPreparationError> for PluginError {
    fn from(e: PluginAssetPreparationError) -> Self { PluginError::AssetPreparation(e) }
}
impl From<PluginAssetCommitError> for PluginError {
    fn from(e: PluginAssetCommitError) -> Self { PluginError::AssetCommit(e) }
}
