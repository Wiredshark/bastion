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
    /// APEX-T2.5.14: the one shared runtime failed to construct; the
    /// original failure is replayed for every later module (no module
    /// ever falls back to a private engine).
    RuntimeUnavailable { detail: String },
    /// APEX-T2.5.15: the module failed component preflight.
    Preflight(PluginPreflightErrorV1),
    /// APEX-T2.5.16: the component does not export the interface its
    /// manifest DECLARED (`PLUGIN-WORLD-MISMATCH`). Probing other worlds
    /// is not attempted for declared modules.
    WorldMismatch { module: String, declared: &'static str, detail: String },
}

/// APEX-T2.5.17 — deployment-manager build refusals: the manager is
/// ordinal-OWNED (missing/gapped/extra ordinals typed) and every
/// archive's recomputed identity must be the plan's artifact for its
/// ordinal (WrongOwner). The manager exists complete or not at all.
#[derive(Debug)]
pub enum PreparedManagerErrorV1 {
    OrdinalSetMismatch { missing: Vec<u32>, unexpected: Vec<u32> },
    WrongOwner { ordinal: u32 },
    Plugin(PluginError),
}

/// APEX-T2.5.15 — per-module preflight terminals, classified BY STAGE
/// (compile → host-linker setup → import resolution/typecheck). Engine
/// mismatch is unrepresentable since .14's single shared engine.
#[derive(Debug)]
pub enum PluginPreflightErrorV1 {
    /// The shared runtime itself is unavailable (replayed .14 failure).
    RuntimeUnavailable { detail: String },
    /// `Component::from_binary` rejected the bytes
    /// (`PLUGIN-COMPILE-FAILED`).
    CompileFailed { module: String, detail: String },
    /// Host API registration on the linker failed
    /// (`PLUGIN-LINKER-CONFLICT`).
    LinkerSetupFailed { module: String, detail: String },
    /// `instantiate_pre` could not resolve/typecheck the component's
    /// imports (`PLUGIN-IMPORT-FAILED`).
    ImportResolutionFailed { module: String, detail: String },
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
