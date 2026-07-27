pub mod archive_profile;
pub mod manifest;
pub mod activation_plan;
pub mod artifact_cache;
pub mod deployment;
pub mod resolver;
pub mod errors;
pub mod memory_manager;
pub mod module;

use bincode::error::DecodeError;
use common::{assets::ASSETS_PATH, event::PluginHash, uid::Uid};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tracing::{error, info};

use self::{
    errors::{
        PluginAssetCommitError, PluginAssetPreparationError, PluginError, PluginInspectionError,
        PluginInstantiationError, PluginModuleError,
    },
    memory_manager::EcsWorld,
    module::PluginModule,
};

use sha2::Digest;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginData {
    name: String,
    // DET-AST-017 (v6 deep-pass, Critical): BTreeSet, not HashSet — the
    // module list drives load/iteration order, and its serialized form is
    // part of plugin identity; both must be canonical.
    modules: std::collections::BTreeSet<PathBuf>,
    dependencies: std::collections::BTreeSet<String>,
}

fn compute_hash(data: &[u8]) -> PluginHash {
    let shasum = sha2::Sha256::digest(data);
    let mut shasum_iter = shasum.iter();
    // a newer generic-array supports into_array ...
    let shasum: PluginHash = std::array::from_fn(|_| *shasum_iter.next().unwrap());
    shasum
}

/// DET-AST-024/025: put a plugin set into its CANONICAL order — ascending
/// content hash (`PluginHash` = SHA-256). `fs::read_dir` yields OS-directory
/// order and `load_server_plugin` pushes in network-arrival order, both
/// non-canonical; `create_body` / `update_skeleton` / `command_event` are
/// LAST-WINS over the plugin Vec, so which provider wins a stable
/// body/skeleton/command name must be a pure function of the plugin SET, not
/// load order. Sorting by the content-derived, globally-unique hash makes it so.
/// Extracted from the two inline `sort_unstable_by_key(|p| p.hash)` sites so the
/// canonical-order contract is directly testable.
fn canonical_plugin_order<P>(plugins: &mut [P], hash: impl Fn(&P) -> PluginHash) {
    plugins.sort_unstable_by_key(hash);
}

fn cache_file_name(
    mut base_dir: PathBuf,
    hash: &PluginHash,
    create_dir: bool,
) -> Result<PathBuf, std::io::Error> {
    base_dir.push("server-plugins");
    if create_dir {
        std::fs::create_dir_all(base_dir.as_path())?;
    }
    let name = hex::encode(hash);
    base_dir.push(name);
    base_dir.set_extension("plugin.tar");
    Ok(base_dir)
}

// write received plugin to disk cache
pub fn store_server_plugin(base_dir: &Path, data: Vec<u8>) -> Result<PathBuf, std::io::Error> {
    let shasum = compute_hash(data.as_slice());
    let result = cache_file_name(base_dir.to_path_buf(), &shasum, true)?;
    // DET-AST-030 (v6 deep-pass, High): atomic cache admission. The old code
    // wrote the payload straight into the hash-named cache file, so a crash or
    // partial write left an invalid entry under a semantic-looking name that
    // `find_cached` would later trust as a valid plugin — a corrupt,
    // non-canonical plugin-load input. Write to a temp sibling, flush it
    // durably, then atomically rename into place: `find_cached` only ever
    // observes a complete file whose content hashes to its own name (the name
    // is `compute_hash(data)` and we wrote exactly `data`, so raw + semantic
    // identity are verified by construction).
    let tmp = result.with_extension("partial");
    {
        let mut file = std::fs::File::create(tmp.as_path())?;
        file.write_all(data.as_slice())?;
        file.sync_all()?;
    }
    std::fs::rename(tmp.as_path(), result.as_path())?;
    Ok(result)
}

pub fn find_cached(base_dir: &Path, hash: &PluginHash) -> Result<PathBuf, std::io::Error> {
    let local_path = cache_file_name(base_dir.to_path_buf(), hash, false)?;
    if local_path.as_path().exists() {
        Ok(local_path)
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    }
}

pub struct Plugin {
    data: PluginData,
    modules: Vec<PluginModule>,
    hash: PluginHash,
    path: PathBuf,
    data_buf: Vec<u8>,
}

/// APEX-T2.1.02 — one sequentially-observed tar entry (observation only; T2.2
/// decides canonical acceptance). Raw path bytes, type byte, size, and file
/// position are retained so T2.2 can apply canonical path/type policy without
/// trusting a prematurely decoded `PathBuf` or reparsing another file version.
pub(crate) struct LegacyArchiveEntryRecordV1 {
    pub archive_ordinal: u32,
    pub raw_path_bytes: Vec<u8>,
    pub decoded_path: Option<PathBuf>,
    pub entry_type_byte: u8,
    pub declared_size: u64,
    pub raw_file_position: u64,
}

/// APEX-T2.1.02 — inspection observations that are hazards-for-later-rows,
/// not rejections. DELTA vs the packet's §7.1: `DuplicateExactPath` and
/// `ModuleOrderIsLegacyHashSet` are omitted because this line already CLOSED
/// DET-AST-019 (duplicates reject fail-closed, occurrences stay visible in
/// `entry_inventory`) and DET-AST-017 (modules are a canonical `BTreeSet`).
#[derive(Debug)]
pub(crate) enum PluginInspectionWarningV1 {
    UnsupportedEntryTypeDeferred {
        archive_ordinal: u32,
        entry_type_byte: u8,
    },
    /// `fs::read_dir` discovery order is legacy provenance, not a canonical
    /// activation order (T2.4 owns that policy).
    DiscoveryOrderIsLegacy,
}

/// APEX-T2.1.02/.03 — a side-effect-free legacy-format archive inspection.
/// Construction reads bytes, hashes, sequentially inventories every tar entry,
/// parses the legacy TOML, and verifies every declared module has bytes.
/// Construction may NOT call Wasmtime, access ECS, invoke guest hooks, publish
/// a manager entry, or mutate global assets — the type contains no
/// `PluginModule`, `Engine`, `Store`, ECS reference, or global cache handle.
pub(crate) struct InspectedPluginArchive {
    pub source_path: PathBuf,
    /// Legacy discovery provenance (diagnostic; never an activation priority).
    #[expect(dead_code, reason = "diagnostic provenance until APEX-T2.4 consumes it")]
    pub discovery_ordinal: u32,
    pub artifact_hash: PluginHash,
    /// The single immutable buffer used for artifact hashing, sequential tar
    /// inspection, module-byte extraction, and later `Plugin.data_buf`.
    pub archive_bytes: Vec<u8>,
    pub manifest: PluginData,
    #[expect(dead_code, reason = "retained for APEX-T2.3's canonical-manifest revalidation")]
    pub manifest_bytes: Vec<u8>,
    #[expect(dead_code, reason = "retained for APEX-T2.2's canonical-profile revalidation")]
    pub entry_inventory: Vec<LegacyArchiveEntryRecordV1>,
    /// APEX-T2.2.08: the ObserveLegacy profile observation — a strict-
    /// pipeline PREVIEW recorded at inventory time. NEVER an admission
    /// input on this legacy path (spec policy 1: observation-only,
    /// byte-unchanged legacy behavior).
    #[expect(dead_code, reason = "evidence surface until APEX-T2.5's rollout consumes it")]
    pub profile_observation: archive_profile::ObserveSummaryV1,
    pub legacy_files: HashMap<PathBuf, Vec<u8>>,
    pub warnings: Vec<PluginInspectionWarningV1>,
}

impl InspectedPluginArchive {
    /// APEX-T2.1.03 — inspect an archive from a path (reads the file once; the
    /// same bytes serve hash, inventory, module extraction, and `data_buf`).
    pub(crate) fn inspect_path(
        path_buf: PathBuf,
        discovery_ordinal: u32,
    ) -> Result<Self, PluginInspectionError> {
        let mut reader =
            fs::File::open(path_buf.as_path()).map_err(|source| PluginInspectionError::Io {
                path: path_buf.clone(),
                source,
            })?;
        let mut buf = Vec::new();
        reader
            .read_to_end(&mut buf)
            .map_err(|source| PluginInspectionError::Io {
                path: path_buf.clone(),
                source,
            })?;
        Self::inspect_bytes(path_buf, discovery_ordinal, buf)
    }

    /// APEX-T2.1.03 — byte-oriented inspection core (no filesystem access).
    pub(crate) fn inspect_bytes(
        source_path: PathBuf,
        discovery_ordinal: u32,
        archive_bytes: Vec<u8>,
    ) -> Result<Self, PluginInspectionError> {
        let artifact_hash = compute_hash(archive_bytes.as_slice());
        let arch_err = |source| PluginInspectionError::ArchiveEntries {
            path: source_path.clone(),
            source,
        };

        // Sequential raw entry inventory BEFORE any reduction: every
        // occurrence (incl. would-be duplicates) is observed and retained.
        let mut entry_inventory = Vec::new();
        let mut warnings = Vec::new();
        let mut selected: Vec<(PathBuf, Vec<u8>)> = Vec::new();
        for (i, entry) in tar::Archive::new(archive_bytes.as_slice())
            .entries()
            .map_err(&arch_err)?
            .enumerate()
        {
            let entry = entry.map_err(&arch_err)?;
            let archive_ordinal = u32::try_from(i)
                .map_err(|_| arch_err(std::io::Error::other("archive ordinal overflow")))?;
            let entry_type_byte = entry.header().entry_type().as_byte();
            let record = LegacyArchiveEntryRecordV1 {
                archive_ordinal,
                raw_path_bytes: entry.path_bytes().into_owned(),
                decoded_path: entry.path().ok().map(|p| p.into_owned()),
                entry_type_byte,
                declared_size: entry.size(),
                raw_file_position: entry.raw_file_position(),
            };
            // Legacy selection (current behavior): decoded regular-path bytes
            // sliced out of the one immutable buffer.
            match &record.decoded_path {
                Some(path) => {
                    let offset = record.raw_file_position as usize;
                    let end = offset.saturating_add(record.declared_size as usize);
                    let bytes = archive_bytes
                        .get(offset..end)
                        .ok_or_else(|| {
                            arch_err(std::io::Error::other("entry data outside archive bounds"))
                        })?
                        .to_vec();
                    selected.push((path.clone(), bytes));
                },
                None => warnings.push(PluginInspectionWarningV1::UnsupportedEntryTypeDeferred {
                    archive_ordinal,
                    entry_type_byte,
                }),
            }
            entry_inventory.push(record);
        }

        // DET-AST-019 (v6 deep-pass, Critical — CLOSED, preserved): duplicate
        // archive paths were silent last-entry-wins by archive order — an
        // aliased/malformed archive could shadow content invisibly. Duplicates
        // are REJECTED fail-closed (now with a typed terminal); the raw
        // occurrences remain visible in `entry_inventory` for T2.2.
        let mut legacy_files = HashMap::new();
        for (path, bytes) in selected {
            if legacy_files.insert(path.clone(), bytes).is_some() {
                tracing::error!(
                    ?path,
                    "DET-AST-019: duplicate path inside plugin archive — rejected"
                );
                return Err(PluginInspectionError::DuplicateArchivePath {
                    path: source_path,
                    duplicate: path,
                });
            }
        }

        let manifest_bytes = legacy_files
            .get(Path::new("plugin.toml"))
            .ok_or(PluginInspectionError::NoConfig {
                path: source_path.clone(),
            })?
            .clone();
        let manifest = toml::de::from_str::<PluginData>(
            std::str::from_utf8(&manifest_bytes).map_err(|inner| {
                PluginInspectionError::ConfigEncoding {
                    path: source_path.clone(),
                    source: Box::new(DecodeError::Utf8 { inner }),
                }
            })?,
        )
        .map_err(|source| PluginInspectionError::ConfigToml {
            path: source_path.clone(),
            source,
        })?;

        // Every declared module must have selected bytes (moves the missing-
        // module failure to inspection, where it belongs: no Wasmtime yet).
        for module in manifest.modules.iter() {
            if !legacy_files.contains_key(module) {
                return Err(PluginInspectionError::MissingDeclaredModule {
                    plugin: manifest.name.clone(),
                    module: module.clone(),
                });
            }
        }

        // APEX-T2.2.08: profile observation over the SAME immutable buffer
        // — total, side-effect-free, never consulted for THIS (legacy)
        // admission decision.
        let profile_observation = archive_profile::observe_legacy(&archive_bytes);
        tracing::debug!(
            source = %source_path.display(),
            dialect = ?profile_observation.dialect,
            strict_preview = profile_observation.strict_preview_terminal,
            "APEX-T2.2 archive profile observation"
        );

        Ok(Self {
            source_path,
            discovery_ordinal,
            artifact_hash,
            archive_bytes,
            manifest,
            manifest_bytes,
            entry_inventory,
            legacy_files,
            warnings,
            profile_observation,
        })
    }
}

/// APEX-T2.1.05 — legacy discovery provenance. The ordinal records current
/// `read_dir` position for diagnostics only; it is NOT an activation priority
/// and must not enter a future canonical plan except as evidence (T2.4).
struct DiscoveredPluginPath {
    discovery_ordinal: u32,
    path: PathBuf,
}

impl Plugin {
    /// APEX-T2.1.06/.07 — the ONLY transition from inspected bytes to private
    /// live modules. Consumes the inspection record: iterates the canonical
    /// `BTreeSet` module order (DET-AST-017), removes module bytes from the
    /// legacy map, runs the existing Wasmtime-backed constructor, and moves
    /// `archive_bytes` into `Plugin.data_buf` (NO second `fs::read` — hash and
    /// stored bytes come from one buffer). Does NOT call `load_event`, does NOT
    /// register assets, does NOT insert into a manager.
    fn instantiate(mut inspected: InspectedPluginArchive) -> Result<Self, PluginInstantiationError> {
        let data = inspected.manifest;
        let modules = data
            .modules
            .iter()
            .map(|path| {
                let wasm_data = inspected
                    .legacy_files
                    .remove(path)
                    .expect("inspection verified every declared module has bytes");
                PluginModule::new(data.name.to_owned(), &wasm_data).map_err(|source| {
                    PluginInstantiationError::Module {
                        plugin: data.name.to_owned(),
                        module: path.clone(),
                        source,
                    }
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(Plugin {
            data,
            modules,
            hash: inspected.artifact_hash,
            path: inspected.source_path,
            data_buf: inspected.archive_bytes,
        })
    }

    pub fn load_event(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), PluginModuleError> {
        self.modules
            .iter_mut()
            .try_for_each(|module| module.load_event(ecs, mode))
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: common::uid::Uid,
    ) -> Result<Vec<String>, CommandResults> {
        let mut result = Err(CommandResults::UnknownCommand);
        self.modules.iter_mut().for_each(|module| {
            match module.command_event(ecs, name, args, player) {
                Ok(res) => result = Ok(res),
                Err(CommandResults::UnknownCommand) => (),
                Err(err) => {
                    if result.is_err() {
                        result = Err(err)
                    }
                },
            }
        });
        result
    }

    /// get the path to the plugin file
    pub fn path(&self) -> &Path { self.path.as_path() }

    /// Get the data of this plugin
    pub fn data_buf(&self) -> &[u8] { &self.data_buf }

    pub fn create_body(&mut self, name: &str) -> Option<module::Body> {
        let mut result = None;
        self.modules.iter_mut().for_each(|module| {
            if let Some(body) = module.create_body(name) {
                result = Some(body);
            }
        });
        result
    }

    pub fn update_skeleton(
        &mut self,
        body: &module::Body,
        dep: &module::Dependency,
        time: f32,
    ) -> Option<module::Skeleton> {
        let mut result = None;
        self.modules.iter_mut().for_each(|module| {
            if let Some(skel) = module.update_skeleton(body, dep, time) {
                result = Some(skel);
            }
        });
        result
    }
}

/// APEX-T2.1.10 — a fully-prepared, not-yet-published plugin batch. ALL
/// fallible work (inspection happened before `prepare`; Wasmtime instantiation
/// and asset-source preparation happen inside it) completes before any
/// publication: after `commit` succeeds on the asset side, manager
/// construction is extend/move-only and infallible under normal semantics.
struct PreparedPluginBatch {
    plugins: Vec<Plugin>,
    asset_sources: Vec<common::assets::PreparedPluginAssetSource>,
}

impl PreparedPluginBatch {
    /// Phase order (packet §7.5): ALL plugins instantiate privately, then ALL
    /// asset sources prepare privately (with digest revalidation against the
    /// inspected artifact hash — the MVP TOCTOU guard; the residual hostile
    /// path-swap between revalidation and `Tar::open` is NAMED and deferred,
    /// packet §4.1). Any failure drops the whole private batch: zero manager
    /// and zero global-asset delta.
    fn prepare(inspected: Vec<InspectedPluginArchive>) -> Result<Self, PluginError> {
        // Digest + path pairs survive instantiation (which consumes the recs).
        let sources: Vec<(PathBuf, PluginHash)> = inspected
            .iter()
            .map(|i| (i.source_path.clone(), i.artifact_hash))
            .collect();
        let plugins = inspected
            .into_iter()
            .map(Plugin::instantiate)
            .collect::<Result<Vec<_>, _>>()?;
        let asset_sources = sources
            .into_iter()
            .map(|(path, expected)| {
                // Revalidate the on-disk bytes against the inspected digest
                // before opening a path-backed Tar source.
                let observed = fs::read(&path)
                    .map(|b| compute_hash(&b))
                    .map_err(|source| PluginAssetPreparationError::TarSource {
                        path: path.clone(),
                        source,
                    })?;
                if observed != expected {
                    return Err(PluginAssetPreparationError::ArchiveChangedAfterInspection {
                        path,
                        expected,
                        observed,
                    });
                }
                common::assets::prepare_plugin_tar(path.clone()).map_err(|source| {
                    PluginAssetPreparationError::TarSource { path, source }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            plugins,
            asset_sources,
        })
    }

    /// APEX-T2.1.11 — publish the batch as a NEW manager: one asset-registry
    /// write lock, then manager construction (canonical DET-AST-024/025 hash
    /// order preserved). Nothing fallible runs after the asset commit.
    fn commit_new_manager(self) -> Result<PluginMgr, PluginError> {
        common::assets::commit_prepared_plugin_tars(self.asset_sources).map_err(|e| match e {
            common::assets::CommitRejectedV1::LockPoisoned => PluginAssetCommitError::RegistryLockPoisoned,
            common::assets::CommitRejectedV1::GenerationSealed => {
                PluginAssetCommitError::GenerationRefused { detail: "generation sealed: incremental forbidden" }
            },
        })?;
        let mut plugins = self.plugins;
        canonical_plugin_order(&mut plugins, |p| p.hash);
        Ok(PluginMgr { plugins })
    }

    /// APEX-T2.5.12 — publish the batch as THE one-time content generation
    /// (governed deployment paths): install-exactly-once instead of the
    /// incremental commit; everything else identical to
    /// `commit_new_manager`.
    fn commit_new_manager_as_generation(self, generation_token: [u8; 32]) -> Result<PluginMgr, PluginError> {
        common::assets::install_plugin_content_generation_v1(generation_token, self.asset_sources).map_err(
            |e| match e {
                common::assets::ContentGenerationErrorV1::LockPoisoned => {
                    PluginAssetCommitError::RegistryLockPoisoned
                },
                common::assets::ContentGenerationErrorV1::AlreadyInstalled { .. } => {
                    PluginAssetCommitError::GenerationRefused { detail: "generation already installed" }
                },
                common::assets::ContentGenerationErrorV1::LegacyPublicationPresent { .. } => {
                    PluginAssetCommitError::GenerationRefused { detail: "legacy publication present" }
                },
            },
        )?;
        let mut plugins = self.plugins;
        canonical_plugin_order(&mut plugins, |p| p.hash);
        Ok(PluginMgr { plugins })
    }

    /// APEX-T2.1.14 — publish the batch INTO an existing manager (late
    /// cached/downloaded path). Same commit discipline; returns the admitted
    /// hashes in input order.
    fn commit_into(self, manager: &mut PluginMgr) -> Result<Vec<PluginHash>, PluginError> {
        common::assets::commit_prepared_plugin_tars(self.asset_sources)
            .map_err(|_| PluginAssetCommitError::RegistryLockPoisoned)?;
        let hashes = self.plugins.iter().map(|p| p.hash).collect();
        manager.plugins.extend(self.plugins);
        // DET-AST-024/025: canonical content-hash order is a manager invariant,
        // re-established after every admission.
        canonical_plugin_order(&mut manager.plugins, |p| p.hash);
        Ok(hashes)
    }
}

#[derive(Default)]
pub struct PluginMgr {
    plugins: Vec<Plugin>,
}

impl PluginMgr {
    pub fn from_asset_or_default() -> Self {
        let mut path = (*ASSETS_PATH).clone();
        path.push("plugins");
        info!("Searching {:?} for plugins...", path);

        match Self::from_dir(&path) {
            Ok(plugin_mgr) => {
                info!("{} plugin(s) loaded", plugin_mgr.plugins.len());
                plugin_mgr
            },
            Err(e) => {
                if let PluginError::FromDirDoesNotExist = e {
                    info!("{:?} does not exist, no plugins loaded", path);
                } else {
                    error!(?e, "Failed to read plugins from assets");
                };
                PluginMgr::default()
            },
        }
    }

    /// APEX-T2.1.11 — batch path: discover ALL → inspect ALL → prepare batch
    /// (instantiate all, prepare all asset sources) → ONE commit. A failure at
    /// any pre-commit phase drops the whole private batch: the returned error
    /// leaves ZERO plugin asset sources committed by this batch (the old
    /// per-plugin `register_tar`-inside-the-loop could pollute the global
    /// combined source with earlier plugins after a later one failed).
    /// APEX-T2.5.11 — construct from an EXPLICIT verified path list (the
    /// client's deployment-acquisition cache): same inspect → prepare →
    /// one-commit batch as `from_dir`, but ordinals come from the given
    /// order (the deployment plan's canonical ordinals, not discovery),
    /// so no `DiscoveryOrderIsLegacy` warning is attached.
    pub fn from_paths_v1(paths: Vec<PathBuf>, generation_token: [u8; 32]) -> Result<Self, PluginError> {
        let inspected = paths
            .into_iter()
            .enumerate()
            .map(|(i, path)| {
                info!("Inspecting deployment plugin at {:?}", path);
                InspectedPluginArchive::inspect_path(path, i as u32)
            })
            .collect::<Result<Vec<_>, PluginInspectionError>>()
            .inspect_err(|e| error!(?e, "Failed to inspect deployment plugin"))?;
        // APEX-T2.5.12: governed deployments publish as THE one-time
        // content generation (token = the deployment root) — the
        // incremental path stays sealed off for the rest of the process.
        let mgr = PreparedPluginBatch::prepare(inspected)
            .inspect_err(|e| error!(?e, "Failed to prepare deployment plugin batch"))?
            .commit_new_manager_as_generation(generation_token)?;
        for plugin in &mgr.plugins {
            info!("Loaded deployment plugin '{}' with {} module(s)", plugin.data.name, plugin.modules.len());
        }
        Ok(mgr)
    }

    fn from_dir(path: &Path) -> Result<Self, PluginError> {
        // APEX-T2.1.04: no silent `filter_map(e.ok())` — every directory entry
        // yields a path or a typed `DirectoryEntry` terminal.
        let mut discovered = Vec::new();
        let mut ordinal: u32 = 0;
        for entry in fs::read_dir(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PluginError::FromDirDoesNotExist
            } else {
                PluginError::Io(e)
            }
        })? {
            let entry = entry.map_err(|source| PluginInspectionError::DirectoryEntry {
                directory: path.to_path_buf(),
                source,
            })?;
            let file_type =
                entry
                    .file_type()
                    .map_err(|source| PluginInspectionError::DirectoryEntry {
                        directory: path.to_path_buf(),
                        source,
                    })?;
            if file_type.is_file()
                && entry
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.ends_with(".plugin.tar"))
                    .unwrap_or(false)
            {
                // APEX-T2.1.05: ordinal = legacy provenance, never priority.
                discovered.push(DiscoveredPluginPath {
                    discovery_ordinal: ordinal,
                    path: entry.path(),
                });
                ordinal += 1;
            }
        }

        // Inspect ALL (side-effect-free) before any Wasmtime work.
        let inspected = discovered
            .into_iter()
            .map(|d| {
                info!("Inspecting plugin at {:?}", d.path);
                let mut rec = InspectedPluginArchive::inspect_path(d.path, d.discovery_ordinal)?;
                rec.warnings
                    .push(PluginInspectionWarningV1::DiscoveryOrderIsLegacy);
                Ok(rec)
            })
            .collect::<Result<Vec<_>, PluginInspectionError>>()
            .inspect_err(|e| error!(?e, "Failed to inspect plugin"))?;

        // Instantiate all + prepare all asset sources privately, then commit
        // once. Canonical DET-AST-024/025 hash order is applied at manager
        // construction (see `PreparedPluginBatch::commit_new_manager`).
        let mgr = PreparedPluginBatch::prepare(inspected)
            .inspect_err(|e| error!(?e, "Failed to prepare plugin batch"))?
            .commit_new_manager()?;

        for plugin in &mgr.plugins {
            info!(
                "Loaded plugin '{}' with {} module(s)",
                plugin.data.name,
                plugin.modules.len()
            );
        }

        Ok(mgr)
    }

    /// Add a plugin received from the server, running its load hook exactly
    /// once as part of admission.
    ///
    /// DET-PLG-003 (determinism audit): the raw push APIs used to register a
    /// late-arriving server plugin and make it visible *without* ever invoking
    /// its `load_event`. A plugin present in the initial asset directory ran
    /// its load hook (via `State::setup_ecs_world`), while the same plugin
    /// delivered from cache/network was published with no hook. Plugin
    /// activation therefore depended on the bootstrap path/timing, not solely
    /// on the declared plugin set. We now run the load hook here, against the
    /// current ECS view and game mode, so every admission path
    /// (preinstalled/cached/downloaded) shares one lifecycle.
    ///
    /// Admission is idempotent: a plugin whose hash is already present is a
    /// no-op, so duplicate delivery (cache + network, or a repeat) neither
    /// double-pushes nor double-activates. The hook runs *before* the push, so
    /// a hook failure leaves the manager unchanged (a small typed rollback).
    pub fn load_server_plugin(
        &mut self,
        path: PathBuf,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<PluginHash, PluginError> {
        // APEX-T2.1.14 — same inspect/prepare/commit substrate as the initial
        // batch, one-item edition. PLG-003's closure is PRESERVED (delta vs the
        // packet, which targets a tree where the late hook was still skipped):
        // the hook runs on the PRIVATE plugin BEFORE any publication, so a hook
        // failure now leaves BOTH the manager AND the global asset source
        // unchanged (the old path registered the tar before the hook ran).
        let inspected = InspectedPluginArchive::inspect_path(path, 0)?;
        let hash = inspected.artifact_hash;
        // Idempotent: an already-admitted plugin must not re-run its load
        // hook or be pushed a second time (checked BEFORE any Wasmtime work).
        if self.plugins.iter().any(|p| p.hash == hash) {
            return Ok(hash);
        }
        let mut batch = PreparedPluginBatch::prepare(vec![inspected])?;
        let plugin = batch
            .plugins
            .first_mut()
            .expect("one-item batch has one plugin");
        plugin.load_event(ecs, mode).map_err(|e| {
            PluginError::PluginModuleError(plugin.data.name.clone(), "<load>".to_owned(), e)
        })?;
        // Publication: one asset commit + manager insert + canonical
        // DET-AST-024/025 re-sort (inside commit_into).
        batch.commit_into(self)?;
        Ok(hash)
    }

    pub fn cache_server_plugin(
        &mut self,
        base_dir: &Path,
        data: Vec<u8>,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<PluginHash, PluginError> {
        let path = store_server_plugin(base_dir, data).map_err(PluginError::Io)?;
        self.load_server_plugin(path, ecs, mode)
    }

    /// list all registered plugins
    pub fn plugin_list(&self) -> Vec<PluginHash> {
        self.plugins.iter().map(|plugin| plugin.hash).collect()
    }

    /// retrieve a specific plugin
    pub fn find(&self, hash: &PluginHash) -> Option<&Plugin> {
        self.plugins.iter().find(|plugin| &plugin.hash == hash)
    }

    pub fn load_event(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), PluginModuleError> {
        self.plugins
            .iter_mut()
            .try_for_each(|plugin| plugin.load_event(ecs, mode))
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: Uid,
    ) -> Result<Vec<String>, CommandResults> {
        // DET-AST-023 (v6 deep-pass, declared policy): LAST-registered
        // handler wins, in the canonical plugin order. That order is a pure
        // function of the plugin set because `self.plugins` is kept sorted by
        // content hash (DET-AST-024/025 at the `from_dir` / `load_server_plugin`
        // write sites). Multiple handlers for one command are an AMBIGUITY —
        // witnessed loudly below rather than silent.
        let mut handlers = 0u32;
        let mut result = Err(CommandResults::UnknownCommand);
        self.plugins.iter_mut().for_each(|plugin| {
            match plugin.command_event(ecs, name, args, player) {
                Ok(val) => {
                    handlers += 1;
                    if handlers > 1 {
                        tracing::warn!(
                            command = name,
                            "DET-AST-023: multiple plugins handle this command —                              last in canonical order wins"
                        );
                    }
                    result = Ok(val)
                },
                Err(CommandResults::UnknownCommand) => (),
                Err(err) => {
                    if result.is_err() {
                        result = Err(err);
                    }
                },
            }
        });
        result
    }

    pub fn create_body(&mut self, name: &str) -> Option<module::Body> {
        let mut result = None;
        self.plugins.iter_mut().for_each(|plugin| {
            if let Some(body) = plugin.create_body(name) {
                result = Some(body);
            }
        });
        result
    }

    pub fn update_skeleton(
        &mut self,
        body: &module::Body,
        dep: &module::Dependency,
        time: f32,
    ) -> Option<module::Skeleton> {
        let mut result = None;
        self.plugins.iter_mut().for_each(|plugin| {
            if let Some(skeleton) = plugin.update_skeleton(body, dep, time) {
                result = Some(skeleton);
            }
        });
        result
    }
}

/// Error returned by plugin based server commands
pub enum CommandResults {
    UnknownCommand,
    HostError(wasmtime::Error),
    PluginError(String),
}

/// DET-AST-024/025 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof):
/// plugins are processed in a canonical order — ascending content hash — so the
/// LAST-WINS arbitration for a stable body/skeleton/command name is a pure
/// function of the plugin SET, never the OS-directory (`fs::read_dir`) or
/// network-arrival (`load_server_plugin`) load order. Guards the two
/// `canonical_plugin_order` sites; a revert to raw push/read-dir order would RED.
#[cfg(test)]
mod det_ast_order_tests {
    use super::*;

    // A distinct content hash (SHA-256 stand-in) in the high bytes.
    fn h(b: u8) -> PluginHash {
        let mut a = [0u8; 32];
        a[0] = b;
        a
    }

    #[test]
    fn det_ast_024_plugin_order_is_load_order_independent() {
        // The SAME plugin set, delivered in two DIFFERENT load orders (OS-dir
        // vs network-arrival). The payload rides along so a reorder is visible.
        let mut os_order = vec![(h(3), "gamma"), (h(1), "alpha"), (h(2), "beta")];
        let mut net_order = vec![(h(2), "beta"), (h(3), "gamma"), (h(1), "alpha")];
        canonical_plugin_order(&mut os_order, |p| p.0);
        canonical_plugin_order(&mut net_order, |p| p.0);
        assert_eq!(
            os_order, net_order,
            "canonical plugin order must not depend on load order"
        );
        // ...and it is ascending-by-content-hash.
        assert_eq!(os_order, vec![
            (h(1), "alpha"),
            (h(2), "beta"),
            (h(3), "gamma")
        ]);
    }

    #[test]
    fn det_ast_024_is_non_vacuous() {
        // A DIFFERENT set orders differently — the contract carries information.
        let mut set = vec![(h(9), "z"), (h(4), "d")];
        canonical_plugin_order(&mut set, |p| p.0);
        assert_eq!(set, vec![(h(4), "d"), (h(9), "z")]);
        // The lowest-hash provider wins last-wins arbitration regardless of who
        // was pushed last.
        assert_eq!(set.first().map(|p| p.1), Some("d"));
    }
}

/// APEX-T2.1.16 — two-phase canaries (case IDs from
/// PROJECT-BASTION-APEX-T2.1-TWO-PHASE-PLUGIN-CANARIES-v1.json, adapted where
/// this line is STRONGER than the packet's audited tree: AST-017 BTreeSet
/// canonical modules, AST-019 duplicate rejection, AST-024/025 canonical hash
/// order, AST-030 atomic cache store, PLG-003 late load hook present).
#[cfg(test)]
mod two_phase_tests {
    use super::*;

    const TOML_EMPTY: &[u8] = b"name = \"canary\"\nmodules = []\ndependencies = []\n";

    fn tar_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for (path, bytes) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(bytes.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, path, *bytes).unwrap();
        }
        b.into_inner().unwrap()
    }

    fn inspect(entries: &[(&str, &[u8])]) -> Result<InspectedPluginArchive, PluginInspectionError> {
        InspectedPluginArchive::inspect_bytes(
            PathBuf::from("test.plugin.tar"),
            0,
            tar_bytes(entries),
        )
    }

    fn temp_tar(tag: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "apex-t21-state-{}-{}-{tag}.plugin.tar",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// PLG2P-001: valid empty-module archive inspects READY.
    #[test]
    fn plg2p_001_valid_empty_module_inspects() {
        let i = inspect(&[("plugin.toml", TOML_EMPTY)]).unwrap();
        assert_eq!(i.manifest.name, "canary");
        assert!(i.manifest.modules.is_empty());
    }

    /// PLG2P-002/003/004/005: typed inspection rejections.
    #[test]
    fn plg2p_002_005_inspection_rejections_are_typed() {
        assert!(matches!(
            inspect(&[("other.txt", b"x".as_slice())]),
            Err(PluginInspectionError::NoConfig { .. })
        ));
        assert!(matches!(
            inspect(&[("plugin.toml", &[0xff, 0xfe, 0x00, 0x01][..])]),
            Err(PluginInspectionError::ConfigEncoding { .. })
        ));
        assert!(matches!(
            inspect(&[("plugin.toml", b"not = = toml".as_slice())]),
            Err(PluginInspectionError::ConfigToml { .. })
        ));
        let missing_mod = b"name = \"canary\"\nmodules = [\"missing.wasm\"]\ndependencies = []\n";
        assert!(matches!(
            inspect(&[("plugin.toml", missing_mod.as_slice())]),
            Err(PluginInspectionError::MissingDeclaredModule { .. })
        ));
    }

    /// PLG2P-006/007: invalid Wasm INSPECTS successfully (no compile during
    /// inspection) and fails only at private instantiation, typed.
    #[test]
    fn plg2p_006_007_invalid_wasm_inspects_then_fails_instantiation() {
        let toml = b"name = \"canary\"\nmodules = [\"bad.wasm\"]\ndependencies = []\n";
        let i = inspect(&[
            ("plugin.toml", toml.as_slice()),
            ("bad.wasm", b"definitely not wasm".as_slice()),
        ])
        .expect("PLG2P-006: invalid wasm must inspect successfully");
        match Plugin::instantiate(i) {
            Err(PluginInstantiationError::Module { plugin, module, .. }) => {
                assert_eq!(plugin, "canary");
                assert_eq!(module, PathBuf::from("bad.wasm"));
            },
            Ok(_) => panic!("PLG2P-007: invalid wasm must fail instantiation"),
        }
    }

    /// PLG2P-008: the exact inspected bytes become `Plugin.data_buf`.
    #[test]
    fn plg2p_008_data_buf_is_inspected_bytes() {
        let bytes = tar_bytes(&[("plugin.toml", TOML_EMPTY)]);
        let i = InspectedPluginArchive::inspect_bytes(
            PathBuf::from("test.plugin.tar"),
            0,
            bytes.clone(),
        )
        .unwrap();
        let hash = i.artifact_hash;
        let plugin = Plugin::instantiate(i).unwrap();
        assert_eq!(plugin.data_buf(), bytes.as_slice());
        assert_eq!(plugin.hash, hash);
        assert_eq!(hash, compute_hash(&bytes));
    }

    /// PLG2P-010 (STRONGER DELTA): duplicate exact paths REJECT fail-closed
    /// (DET-AST-019 closed on this line); occurrences observed before
    /// reduction per the sequential-inventory requirement.
    #[test]
    fn plg2p_010_duplicates_reject_typed() {
        let dup = tar_bytes(&[
            ("plugin.toml", TOML_EMPTY),
            ("a.txt", b"one".as_slice()),
            ("a.txt", b"two".as_slice()),
        ]);
        match InspectedPluginArchive::inspect_bytes(PathBuf::from("t.plugin.tar"), 0, dup) {
            Err(PluginInspectionError::DuplicateArchivePath { duplicate, .. }) => {
                assert_eq!(duplicate, PathBuf::from("a.txt"));
            },
            Err(other) => panic!("expected DuplicateArchivePath, got {other:?}"),
            Ok(_) => panic!("duplicate paths must reject"),
        }
    }

    /// PLG2P-023/024/025 (runtime half): inspection of a wasm-bearing archive
    /// constructs no module and holds raw bytes only.
    #[test]
    fn plg2p_023_025_inspection_constructs_no_modules() {
        let toml = b"name = \"canary\"\nmodules = [\"bad.wasm\"]\ndependencies = []\n";
        let i = inspect(&[
            ("plugin.toml", toml.as_slice()),
            ("bad.wasm", b"junk".as_slice()),
        ])
        .unwrap();
        assert_eq!(i.legacy_files.len(), 2);
        assert_eq!(i.entry_inventory.len(), 2);
    }

    /// PLG2P-013/014-class + PLG2P-040: second-item instantiation failure
    /// rejects the whole batch before any commit API is reachable.
    #[test]
    fn plg2p_013_014_batch_second_failure_rejects_whole_batch() {
        let good = InspectedPluginArchive::inspect_bytes(
            PathBuf::from("good.plugin.tar"),
            0,
            tar_bytes(&[("plugin.toml", TOML_EMPTY)]),
        )
        .unwrap();
        let toml = b"name = \"bad\"\nmodules = [\"bad.wasm\"]\ndependencies = []\n";
        let bad = InspectedPluginArchive::inspect_bytes(
            PathBuf::from("bad.plugin.tar"),
            1,
            tar_bytes(&[
                ("plugin.toml", toml.as_slice()),
                ("bad.wasm", b"junk".as_slice()),
            ]),
        )
        .unwrap();
        match PreparedPluginBatch::prepare(vec![good, bad]) {
            Err(PluginError::Instantiation(PluginInstantiationError::Module {
                plugin, ..
            })) => assert_eq!(plugin, "bad"),
            Err(other) => panic!("expected instantiation rejection, got {other:?}"),
            Ok(_) => panic!("batch with invalid second plugin must reject"),
        }
    }

    /// PLG2P-016: digest change between inspection and asset preparation is a
    /// typed rejection (MVP TOCTOU guard).
    #[test]
    fn plg2p_016_archive_changed_after_inspection_rejects() {
        let path = temp_tar("toctou", &tar_bytes(&[("plugin.toml", TOML_EMPTY)]));
        let inspected = InspectedPluginArchive::inspect_path(path.clone(), 0).unwrap();
        std::fs::write(
            &path,
            tar_bytes(&[("plugin.toml", TOML_EMPTY), ("x.txt", b"swap".as_slice())]),
        )
        .unwrap();
        match PreparedPluginBatch::prepare(vec![inspected]) {
            Err(PluginError::AssetPreparation(
                PluginAssetPreparationError::ArchiveChangedAfterInspection { .. },
            )) => (),
            Err(other) => panic!("expected ArchiveChangedAfterInspection, got {other:?}"),
            Ok(_) => panic!("changed archive must reject"),
        }
        let _ = std::fs::remove_file(&path);
    }

    /// DET-AST-024/025 (preserved): manager order is canonical content-hash
    /// order for BOTH batch input orders.
    #[test]
    fn det_ast_024_manager_order_is_canonical_hash_order() {
        let mk = |name: &str| {
            let toml = format!("name = \"{name}\"\nmodules = []\ndependencies = []\n");
            temp_tar(name, &tar_bytes(&[("plugin.toml", toml.as_bytes())]))
        };
        let (pa, pb) = (mk("aaa"), mk("bbb"));
        let m1 = PreparedPluginBatch::prepare(vec![
            InspectedPluginArchive::inspect_path(pa.clone(), 0).unwrap(),
            InspectedPluginArchive::inspect_path(pb.clone(), 1).unwrap(),
        ])
        .unwrap()
        .commit_new_manager()
        .unwrap();
        let m2 = PreparedPluginBatch::prepare(vec![
            InspectedPluginArchive::inspect_path(pb.clone(), 0).unwrap(),
            InspectedPluginArchive::inspect_path(pa.clone(), 1).unwrap(),
        ])
        .unwrap()
        .commit_new_manager()
        .unwrap();
        assert_eq!(m1.plugin_list(), m2.plugin_list());
        let mut sorted = m1.plugin_list();
        canonical_plugin_order(&mut sorted, |h| *h);
        assert_eq!(m1.plugin_list(), sorted);
        let _ = std::fs::remove_file(&pa);
        let _ = std::fs::remove_file(&pb);
    }
}
