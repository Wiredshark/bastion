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

/// APEX-T2.5.16 — declared module worlds from a V1 manifest's raw bytes
/// (`None` = legacy manifest / unparseable = the probing path). Pure and
/// derivable by EVERY consumer from the verified archive bytes — world
/// enforcement needs no wire field and no side channel. Full validation
/// stays T2.3's job; this reads exactly the `[[modules]]` world tags.
pub fn extract_declared_worlds_v1(
    manifest_bytes: &[u8],
) -> Option<HashMap<PathBuf, manifest::PluginModuleWorldV1>> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    let raw: toml::Value = toml::de::from_str(text).ok()?;
    if !matches!(raw.get("manifest_version"), Some(toml::Value::Integer(_))) {
        return None; // legacy manifest: no declaration, probing preserved
    }
    let mut worlds = HashMap::new();
    for m in raw.get("modules")?.as_array()? {
        let path = m.get("path")?.as_str()?;
        let world = match m.get("world")?.as_str()? {
            "plugin" => manifest::PluginModuleWorldV1::Plugin,
            "server-plugin" => manifest::PluginModuleWorldV1::ServerPlugin,
            "animation-plugin" => manifest::PluginModuleWorldV1::AnimationPlugin,
            _ => return None, // unknown world: T2.3 validation owns the refusal
        };
        worlds.insert(PathBuf::from(path), world);
    }
    Some(worlds)
}

/// APEX-T2.5.19 — declared COMMAND claims from a V1 manifest's raw bytes
/// (union across runtime modes; `None` = legacy manifest = observation
/// only, no enforcement). Same recompute-from-verified-bytes discipline
/// as `extract_declared_worlds_v1`.
pub fn extract_declared_commands_v1(manifest_bytes: &[u8]) -> Option<std::collections::BTreeSet<String>> {
    let text = std::str::from_utf8(manifest_bytes).ok()?;
    let raw: toml::Value = toml::de::from_str(text).ok()?;
    if !matches!(raw.get("manifest_version"), Some(toml::Value::Integer(_))) {
        return None;
    }
    let mut commands = std::collections::BTreeSet::new();
    if let Some(claims) = raw.get("claims") {
        if let Some(runtime) = claims.get("runtime").and_then(|r| r.as_array()) {
            for mode in runtime {
                if let Some(cmds) = mode.get("commands").and_then(|c| c.as_array()) {
                    for c in cmds {
                        commands.insert(c.as_str()?.to_owned());
                    }
                }
            }
        }
    }
    Some(commands)
}

/// APEX-T2.5.19 — a registration outside the manifest's declared claims:
/// the ceiling violation that ABORTS initialization on governed sessions.
#[derive(Debug)]
pub struct UndeclaredRegistrationV1 {
    pub plugin: String,
    pub command: String,
}

/// APEX-T2.5.19 — pure subset validation: every ACTUAL command
/// registration must appear in the DECLARED claim set. Bodies/skeletons
/// have no claim vocabulary in the V1 manifest (packet: asset/animation
/// claims are per-mode animation lists — enforcement joins when .20/.21
/// ownership lands); commands are the enforced family here.
pub fn validate_registrations_v1(
    plugin: &str,
    actual_commands: &[String],
    declared: Option<&std::collections::BTreeSet<String>>,
) -> Result<(), UndeclaredRegistrationV1> {
    let Some(declared) = declared else {
        return Ok(()); // legacy manifest: observation only
    };
    for command in actual_commands {
        if !declared.contains(command) {
            return Err(UndeclaredRegistrationV1 { plugin: plugin.to_owned(), command: command.clone() });
        }
    }
    Ok(())
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
    /// APEX-T2.5.16 consumes this for declared-world extraction (was
    /// retained since T2.1 for exactly this class of revalidation).
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
    fn instantiate(
        mut inspected: InspectedPluginArchive,
        limits: Option<module::PluginStoreLimitsV1>,
    ) -> Result<Self, PluginInstantiationError> {
        let data = inspected.manifest;
        // APEX-T2.5.16: declared worlds from the archive's OWN manifest
        // (V1 manifests only; legacy manifests yield None = probing).
        // Derived identically on every consumer from the verified bytes —
        // no side channel, no wire field.
        let declared_worlds = extract_declared_worlds_v1(&inspected.manifest_bytes);
        // APEX-T2.5.15: COMPLETE preflight (compile + import
        // resolution/typecheck) of EVERY module before ANY instantiation
        // — a failure in the last module surfaces before the first
        // module instantiates; only private objects exist until the
        // whole set is through.
        let preflighted = data
            .modules
            .iter()
            .map(|path| {
                let wasm_data = inspected
                    .legacy_files
                    .remove(path)
                    .expect("inspection verified every declared module has bytes");
                module::preflight_component_v1(&data.name, &wasm_data)
                    .map(|prepared| (path.clone(), prepared))
                    .map_err(|source| PluginInstantiationError::Module {
                        plugin: data.name.to_owned(),
                        module: path.clone(),
                        source: PluginModuleError::Preflight(source),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let modules = preflighted
            .iter()
            .map(|(path, prepared)| {
                let expected_world = declared_worlds.as_ref().and_then(|m| m.get(path).copied());
                PluginModule::new_from_prepared(prepared, limits, expected_world).map_err(|source| {
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
    fn prepare(
        inspected: Vec<InspectedPluginArchive>,
        limits: Option<module::PluginStoreLimitsV1>,
    ) -> Result<Self, PluginError> {
        // Digest + path pairs survive instantiation (which consumes the recs).
        let sources: Vec<(PathBuf, PluginHash)> = inspected
            .iter()
            .map(|i| (i.source_path.clone(), i.artifact_hash))
            .collect();
        let plugins = inspected
            .into_iter()
            .map(|i| Plugin::instantiate(i, limits))
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
        Ok(PluginMgr { plugins, governed: false, lifecycle: PluginLifecycleStateV1::NotActivated, command_owners: None })
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
        Ok(PluginMgr { plugins, governed: true, lifecycle: PluginLifecycleStateV1::NotActivated, command_owners: None })
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

/// APEX-T2.5.18 — the manager's exactly-once lifecycle state. `Failed`
/// poisons the manager: partial-hook state is never reusable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PluginLifecycleStateV1 {
    #[default]
    NotActivated,
    Activated,
    Failed,
}

#[derive(Default)]
pub struct PluginMgr {
    plugins: Vec<Plugin>,
    /// APEX-T2.5.17: built from a verified deployment (fail-closed
    /// lifecycle) vs legacy discovery (byte-preserved fallback behavior).
    governed: bool,
    /// APEX-T2.5.18: exactly-once activation tracking.
    lifecycle: PluginLifecycleStateV1,
    /// APEX-T2.5.20: the compiled command→owner map (governed
    /// deployments): dispatch is ONE lookup, never a provider scan.
    /// `None` = legacy manager = the old scan.
    command_owners: Option<HashMap<String, PluginHash>>,
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
    /// APEX-T2.5.11/.17 — construct the COMPLETE, ordinal-owned manager
    /// from a deployment: every expected ordinal present exactly once,
    /// every archive's RECOMPUTED identity equal to the plan's artifact
    /// for that ordinal (a file that changed between deployment compile
    /// and manager build is a typed WrongOwner refusal, not a silent
    /// swap), full private batch (T2.1) — the public manager exists only
    /// complete or not at all.
    pub fn from_deployment_paths_v1(
        paths: Vec<(u32, PathBuf)>,
        expected_artifacts: &[(u32, [u8; 32])],
        generation_token: [u8; 32],
        limits: Option<module::PluginStoreLimitsV1>,
        max_instances: Option<u32>,
        command_owners: Option<HashMap<String, PluginHash>>,
    ) -> Result<Self, errors::PreparedManagerErrorV1> {
        let batch = Self::verified_deployment_batch_v1(paths, expected_artifacts, limits, max_instances)?;
        let mut mgr = batch
            .commit_new_manager_as_generation(generation_token)
            .map_err(errors::PreparedManagerErrorV1::Plugin)?;
        mgr.command_owners = command_owners;
        Ok(mgr)
    }

    /// The commit-free half of `from_deployment_paths_v1` (ordinal
    /// ownership + identity verification + full private prepare) — split
    /// out so it is testable without touching the PROCESS-GLOBAL asset
    /// registry (whose one-time generation seal makes global-commit tests
    /// mutually exclusive within one test process).
    fn verified_deployment_batch_v1(
        paths: Vec<(u32, PathBuf)>,
        expected_artifacts: &[(u32, [u8; 32])],
        limits: Option<module::PluginStoreLimitsV1>,
        max_instances: Option<u32>,
    ) -> Result<PreparedPluginBatch, errors::PreparedManagerErrorV1> {
        use errors::PreparedManagerErrorV1 as E;
        let mut have: Vec<u32> = paths.iter().map(|(o, _)| *o).collect();
        have.sort_unstable();
        let mut want: Vec<u32> = expected_artifacts.iter().map(|(o, _)| *o).collect();
        want.sort_unstable();
        if have != want {
            return Err(E::OrdinalSetMismatch {
                missing: want.iter().filter(|o| !have.contains(o)).copied().collect(),
                unexpected: have.iter().filter(|o| !want.contains(o)).copied().collect(),
            });
        }
        let inspected = paths
            .into_iter()
            .map(|(ordinal, path)| {
                info!("Inspecting deployment plugin ordinal {ordinal} at {:?}", path);
                let rec = InspectedPluginArchive::inspect_path(path, ordinal)
                    .map_err(|e| E::Plugin(PluginError::Inspection(e)))?;
                let expected = expected_artifacts
                    .iter()
                    .find(|(o, _)| *o == ordinal)
                    .map(|(_, d)| d)
                    .expect("set equality checked above");
                // Ownership: the RECOMPUTED archive identity must be the
                // plan's artifact for this ordinal.
                if rec.artifact_hash != *expected {
                    return Err(E::WrongOwner { ordinal });
                }
                Ok(rec)
            })
            .collect::<Result<Vec<_>, E>>()
            .inspect_err(|e| error!(?e, "deployment manager build refused"))?;
        // APEX-T2.5.12: governed deployments publish as THE one-time
        // content generation (token = the deployment root) — the
        // incremental path stays sealed off for the rest of the process.
        let batch = PreparedPluginBatch::prepare(inspected, limits)
            .map_err(errors::PreparedManagerErrorV1::Plugin)
            .inspect_err(|e| error!(?e, "Failed to prepare deployment plugin batch"))?;
        // APEX-T2.5.18: the policy's per-mode instance ceiling is a
        // MANAGER-level count (total instantiated modules).
        if let Some(ceiling) = max_instances {
            let instances: usize = batch.plugins.iter().map(|p| p.modules.len()).sum();
            if instances > ceiling as usize {
                return Err(E::InstanceCeilingExceeded { instances, max_instances: ceiling });
            }
        }
        for plugin in &batch.plugins {
            info!("Prepared deployment plugin '{}' with {} module(s)", plugin.data.name, plugin.modules.len());
        }
        Ok(batch)
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
        let mgr = PreparedPluginBatch::prepare(inspected, None)
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
        let mut batch = PreparedPluginBatch::prepare(vec![inspected], None)?;
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

    pub fn is_governed(&self) -> bool { self.governed }

    pub fn lifecycle(&self) -> PluginLifecycleStateV1 { self.lifecycle }

    /// APEX-T2.5.18 — THE exactly-once ordered lifecycle: every plugin's
    /// load hooks run once, in canonical (content-hash) order; the FIRST
    /// failure aborts the sequence, names the plugin, and POISONS the
    /// manager (partial hook state is never reusable — a later retry is
    /// its own typed refusal, not a rerun).
    pub fn activate_v1(
        &mut self,
        ecs: &EcsWorld,
        mode: common::resources::GameMode,
    ) -> Result<(), errors::PluginLifecycleErrorV1> {
        use errors::PluginLifecycleErrorV1 as E;
        match self.lifecycle {
            PluginLifecycleStateV1::Activated => return Err(E::DuplicateActivation),
            PluginLifecycleStateV1::Failed => return Err(E::ActivationAfterFailure),
            PluginLifecycleStateV1::NotActivated => {},
        }
        for plugin in self.plugins.iter_mut() {
            if let Err(source) = plugin.load_event(ecs, mode) {
                self.lifecycle = PluginLifecycleStateV1::Failed;
                return Err(E::HookFailed { plugin: plugin.data.name.clone(), source: Box::new(source) });
            }
        }
        self.lifecycle = PluginLifecycleStateV1::Activated;
        Ok(())
    }

    /// APEX-T2.5.19 — collect every plugin's ACTUAL registrations
    /// (canonical order, deduped across its modules) and validate the
    /// command set against the plugin's OWN manifest claims (recomputed
    /// from the verified archive bytes; legacy manifests = observation
    /// only). Returns the ordered receipt input; the first undeclared
    /// registration is the typed violation.
    pub fn registration_receipt_input_v1(
        &mut self,
    ) -> Result<Vec<(String, Vec<String>, Vec<String>)>, UndeclaredRegistrationV1> {
        let mut out = Vec::with_capacity(self.plugins.len());
        for plugin in self.plugins.iter_mut() {
            // plugin.toml bytes re-read from the retained archive buffer.
            let manifest_bytes = {
                let mut found = None;
                let mut ar = tar::Archive::new(plugin.data_buf.as_slice());
                if let Ok(entries) = ar.entries() {
                    for entry in entries.flatten() {
                        if entry.path().ok().as_deref() == Some(Path::new("plugin.toml")) {
                            let mut bytes = Vec::new();
                            let mut entry = entry;
                            if entry.read_to_end(&mut bytes).is_ok() {
                                found = Some(bytes);
                            }
                            break;
                        }
                    }
                }
                found
            };
            let declared = manifest_bytes.as_deref().and_then(extract_declared_commands_v1);
            let mut commands = Vec::new();
            let mut bodies = Vec::new();
            for module in plugin.modules.iter_mut() {
                let (c, b) = module.actual_registrations_v1();
                commands.extend(c);
                bodies.extend(b);
            }
            commands.sort_unstable();
            commands.dedup();
            bodies.sort_unstable();
            bodies.dedup();
            validate_registrations_v1(&plugin.data.name, &commands, declared.as_ref())?;
            out.push((plugin.data.name.clone(), commands, bodies));
        }
        Ok(out)
    }

    pub fn command_event(
        &mut self,
        ecs: &EcsWorld,
        name: &str,
        args: &[String],
        player: Uid,
    ) -> Result<Vec<String>, CommandResults> {
        // APEX-T2.5.20 — GOVERNED dispatch: ONE owner lookup, no provider
        // scan. The owner map is compiled from claims + operator decisions
        // at deployment compile; a command outside the map is a
        // deterministic UnknownCommand (unused-ceiling rule), never a
        // probe of every plugin.
        if let Some(owners) = &self.command_owners {
            let Some(owner_hash) = owners.get(name).copied() else {
                return Err(CommandResults::UnknownCommand);
            };
            let Some(plugin) = self.plugins.iter_mut().find(|p| p.hash == owner_hash) else {
                // Missing/inactive owner: typed, deterministic.
                return Err(CommandResults::UnknownCommand);
            };
            return plugin.command_event(ecs, name, args, player);
        }
        // DET-AST-023 (v6 deep-pass, declared policy): LAST-registered
        // handler wins, in the canonical plugin order. That order is a pure
        // function of the plugin set because `self.plugins` is kept sorted by
        // content hash (DET-AST-024/025 at the `from_dir` / `load_server_plugin`
        // write sites). Multiple handlers for one command are an AMBIGUITY —
        // witnessed loudly below rather than silent. (LEGACY managers only
        // — governed managers take the owner-map path above.)
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
        match Plugin::instantiate(i, None) {
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
        let plugin = Plugin::instantiate(i, None).unwrap();
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
        match PreparedPluginBatch::prepare(vec![good, bad], None) {
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
        match PreparedPluginBatch::prepare(vec![inspected], None) {
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
        ], None)
        .unwrap()
        .commit_new_manager()
        .unwrap();
        let m2 = PreparedPluginBatch::prepare(vec![
            InspectedPluginArchive::inspect_path(pb.clone(), 0).unwrap(),
            InspectedPluginArchive::inspect_path(pa.clone(), 1).unwrap(),
        ], None)
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

/// `APEX-T2.5.17` — ordinal-owned deployment-manager canaries.
#[cfg(test)]
mod prepared_plugin_manager_v1 {
    use super::*;

    fn temp_tar(name: &str) -> (PathBuf, [u8; 32]) {
        let mut b = tar::Builder::new(Vec::new());
        let toml = format!("name = \"{name}\"\nmodules = []\ndependencies = []\n");
        let mut h = tar::Header::new_gnu();
        h.set_size(toml.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        b.append_data(&mut h, "plugin.toml", toml.as_bytes()).unwrap();
        let bytes = b.into_inner().unwrap();
        let digest = compute_hash(&bytes);
        let p = std::env::temp_dir().join(format!(
            "apex-t2517-{}-{}-{name}.plugin.tar",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::write(&p, bytes).unwrap();
        (p, digest)
    }

    #[test]
    fn manager_is_ordinal_owned_and_input_order_free() {
        let (pa, da) = temp_tar("a");
        let (pb, db) = temp_tar("b");
        let expected = [(0u32, da), (1u32, db)];

        // All assertions drive the COMMIT-FREE seam — the process-global
        // registry (one-time generation seal) is never touched, so this
        // test cannot fight the legacy-commit tests over global state.
        // Install semantics live on a LOCAL cache in
        // common-assets::plugin_content_generation_v1.

        // Wrong owner: swap the digests.
        let swapped = [(0u32, db), (1u32, da)];
        assert!(matches!(
            PluginMgr::verified_deployment_batch_v1(
                vec![(0, pa.clone()), (1, pb.clone())],
                &swapped,
                None,
                None
            ),
            Err(errors::PreparedManagerErrorV1::WrongOwner { ordinal: 0 })
        ));
        // Gapped/extra ordinals.
        assert!(matches!(
            PluginMgr::verified_deployment_batch_v1(vec![(0, pa.clone())], &expected, None, None),
            Err(errors::PreparedManagerErrorV1::OrdinalSetMismatch { missing, .. }) if missing == vec![1]
        ));
        assert!(matches!(
            PluginMgr::verified_deployment_batch_v1(
                vec![(0, pa.clone()), (1, pb.clone()), (7, pa.clone())],
                &expected,
                None,
                None
            ),
            Err(errors::PreparedManagerErrorV1::OrdinalSetMismatch { unexpected, .. }) if unexpected == vec![7]
        ));

        // Happy path, permuted input: the private batch carries exactly
        // the expected plugin set (canonical hash order applies at
        // manager construction; the batch content is what .17 owns).
        let batch = PluginMgr::verified_deployment_batch_v1(
            vec![(1, pb.clone()), (0, pa.clone())], // permuted
            &expected,
            None,
            None,
        )
        .unwrap();
        let mut hashes: Vec<_> = batch.plugins.iter().map(|p| p.hash).collect();
        hashes.sort_unstable();
        assert_eq!(hashes, {
            let mut h = vec![da, db];
            h.sort_unstable();
            h
        });
    }
}

/// `APEX-T2.5.18` — exactly-once lifecycle canaries (vacuous-hook
/// managers: ordering/trap fixtures with real trapping wasm live in the
/// VM lane; the STATE MACHINE is fully provable here).
#[cfg(test)]
mod plugin_lifecycle_activation_v1 {
    use super::*;
    use specs::WorldExt;

    fn with_ecs(f: impl FnOnce(&EcsWorld)) {
        let mut world = specs::World::new();
        world.register::<common::comp::Health>();
        world.register::<common::comp::Player>();
        world.register::<Uid>();
        world.insert(common::uid::IdMaps::default());
        let entities = world.entities();
        let id_maps: specs::Read<common::uid::IdMaps> =
            world.read_resource::<common::uid::IdMaps>().into();
        let ecs = EcsWorld {
            entities: &entities,
            health: world.read_component::<common::comp::Health>().into(),
            uid: world.read_component::<Uid>().into(),
            player: world.read_component::<common::comp::Player>().into(),
            id_maps: &id_maps,
        };
        f(&ecs);
    }

    #[test]
    fn activation_is_exactly_once_and_poisons_on_failure() {
        with_ecs(|ecs| {
            // Empty manager: hooks vacuous, activation succeeds ONCE.
            let mut mgr = PluginMgr::default();
            assert_eq!(mgr.lifecycle(), PluginLifecycleStateV1::NotActivated);
            assert!(!mgr.is_governed());
            mgr.activate_v1(ecs, common::resources::GameMode::Server).unwrap();
            assert_eq!(mgr.lifecycle(), PluginLifecycleStateV1::Activated);
            // Second activation: typed duplicate, state unchanged.
            assert!(matches!(
                mgr.activate_v1(ecs, common::resources::GameMode::Server),
                Err(errors::PluginLifecycleErrorV1::DuplicateActivation)
            ));
            assert_eq!(mgr.lifecycle(), PluginLifecycleStateV1::Activated);

            // Poisoned manager: retry is its own typed refusal, not a rerun.
            let mut failed = PluginMgr::default();
            failed.lifecycle = PluginLifecycleStateV1::Failed;
            assert!(matches!(
                failed.activate_v1(ecs, common::resources::GameMode::Server),
                Err(errors::PluginLifecycleErrorV1::ActivationAfterFailure)
            ));
            assert_eq!(failed.lifecycle(), PluginLifecycleStateV1::Failed);
        });
    }
}

/// `APEX-T2.5.19` — registration-receipt canaries (pure validation +
/// claim extraction; live wasm-registration fixtures live in the VM
/// lane on top of this mechanism).
#[cfg(test)]
mod plugin_registration_receipt_v1 {
    use super::*;

    #[test]
    fn subset_validation_and_claim_extraction() {
        let declared: std::collections::BTreeSet<String> =
            ["hello".to_owned(), "wave".to_owned()].into_iter().collect();
        let ok = |cmds: &[&str]| {
            validate_registrations_v1("p", &cmds.iter().map(|s| s.to_string()).collect::<Vec<_>>(), Some(&declared))
        };
        // Exact and strict-subset registrations pass.
        assert!(ok(&["hello", "wave"]).is_ok());
        assert!(ok(&["hello"]).is_ok());
        assert!(ok(&[]).is_ok());
        // Outside the claim set: typed violation naming plugin+command.
        let err = ok(&["hello", "sudo"]).unwrap_err();
        assert_eq!(err.plugin, "p");
        assert_eq!(err.command, "sudo");
        // Legacy manifest (no declaration): observation only.
        assert!(
            validate_registrations_v1("p", &["anything".to_owned()], None).is_ok()
        );

        // Extraction: union across runtime modes; legacy => None.
        let v1 = b"manifest_version = 1\n[claims]\nasset_roots = []\n\n\
                   [[claims.runtime]]\nmode = \"server\"\ncommands = [\"a\", \"b\"]\nanimations = []\n\n\
                   [[claims.runtime]]\nmode = \"client\"\ncommands = [\"c\"]\nanimations = []\n";
        let cmds = extract_declared_commands_v1(v1).unwrap();
        assert_eq!(cmds.into_iter().collect::<Vec<_>>(), vec!["a", "b", "c"]);
        assert!(extract_declared_commands_v1(b"name = \"old\"\n").is_none());
    }
}
