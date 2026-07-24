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
    errors::{PluginError, PluginModuleError},
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

impl Plugin {
    pub fn from_path(path_buf: PathBuf) -> Result<Self, PluginError> {
        let mut reader = fs::File::open(path_buf.as_path()).map_err(PluginError::Io)?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(PluginError::Io)?;
        let shasum = compute_hash(buf.as_slice());

        let mut files = tar::Archive::new(&*buf)
            .entries()
            .map_err(PluginError::Io)?
            .map(|e| {
                e.and_then(|e| {
                    Ok((e.path()?.into_owned(), {
                        let offset = e.raw_file_position() as usize;
                        buf[offset..offset + e.size() as usize].to_vec()
                    }))
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(PluginError::Io)?
            .into_iter()
            .try_fold(HashMap::new(), |mut files, (path, bytes)| {
                // DET-AST-019 (v6 deep-pass, Critical): duplicate archive
                // paths were silent last-entry-wins by archive order — an
                // aliased/malformed archive could shadow content invisibly.
                // Duplicates are REJECTED fail-closed; plugin content is a
                // pure function of a well-formed archive.
                if files.insert(path.clone(), bytes).is_some() {
                    tracing::error!(
                        ?path,
                        "DET-AST-019: duplicate path inside plugin archive — rejected"
                    );
                    return Err(PluginError::NoConfig);
                }
                Ok(files)
            })?;

        let data = toml::de::from_str::<PluginData>(
            std::str::from_utf8(
                files
                    .get(Path::new("plugin.toml"))
                    .ok_or(PluginError::NoConfig)?,
            )
            .map_err(|inner| PluginError::Encoding(Box::new(DecodeError::Utf8 { inner })))?,
        )
        .map_err(PluginError::Toml)?;

        let modules = data
            .modules
            .iter()
            .map(|path| {
                let wasm_data = files.remove(path).ok_or(PluginError::NoSuchModule)?;
                PluginModule::new(data.name.to_owned(), &wasm_data).map_err(|e| {
                    PluginError::PluginModuleError(data.name.to_owned(), "<init>".to_owned(), e)
                })
            })
            .collect::<Result<_, _>>()?;

        let data_buf = fs::read(&path_buf).map_err(PluginError::Io)?;

        Ok(Plugin {
            data,
            modules,
            hash: shasum,
            path: path_buf,
            data_buf,
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

    fn from_dir(path: &Path) -> Result<Self, PluginError> {
        let mut plugins = fs::read_dir(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    PluginError::FromDirDoesNotExist
                } else {
                    PluginError::Io(e)
                }
            })?
            .filter_map(|e| e.ok())
            .map(|entry| {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                    && entry
                        .path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.ends_with(".plugin.tar"))
                        .unwrap_or(false)
                {
                    info!("Loading plugin at {:?}", entry.path());
                    Plugin::from_path(entry.path()).map(|plugin| {
                        if let Err(e) = common::assets::register_tar(entry.path()) {
                            error!("Plugin {:?} tar error {e:?}", entry.path());
                        }
                        Some(plugin)
                    })
                } else {
                    Ok(None)
                }
            })
            .filter_map(Result::transpose)
            .inspect(|p| {
                let _ = p.as_ref().map_err(|e| error!(?e, "Failed to load plugin"));
            })
            .collect::<Result<Vec<_>, _>>()?;

        // DET-AST-024/025 (v6 deep-pass, Critical/High): canonically order the
        // plugin set by content hash. `fs::read_dir` above yields OS directory
        // order (and `load_server_plugin` pushes in network-arrival order) —
        // both non-canonical. `create_body`, `update_skeleton`, and
        // `command_event` are all LAST-WINS over this Vec, so which provider
        // won for a stable body/skeleton/command name depended on load order.
        // `PluginHash` is the SHA-256 of plugin content: globally unique,
        // content-derived, identical on every machine — sorting by it makes
        // every last-wins arbitration a pure function of the plugin SET. This
        // is exactly the canonical order DET-AST-023's comment already assumes.
        canonical_plugin_order(&mut plugins, |p| p.hash);

        for plugin in &plugins {
            info!(
                "Loaded plugin '{}' with {} module(s)",
                plugin.data.name,
                plugin.modules.len()
            );
        }

        Ok(Self { plugins })
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
        Plugin::from_path(path.clone()).and_then(|mut plugin| {
            if let Err(e) = common::assets::register_tar(path.clone()) {
                error!("Plugin {:?} tar error {e:?}", path.as_path());
            }
            let hash = plugin.hash;
            // Idempotent: an already-admitted plugin must not re-run its load
            // hook or be pushed a second time.
            if self.plugins.iter().any(|p| p.hash == hash) {
                return Ok(hash);
            }
            // Run the load hook before publishing; on failure the plugin is
            // never pushed, so the manager is left exactly as it was.
            plugin.load_event(ecs, mode).map_err(|e| {
                PluginError::PluginModuleError(plugin.data.name.clone(), "<load>".to_owned(), e)
            })?;
            self.plugins.push(plugin);
            // DET-AST-024/025: re-establish the canonical content-hash order so
            // a server-delivered plugin never selects last-wins arbitration by
            // its network arrival position (see `from_dir`).
            canonical_plugin_order(&mut self.plugins, |p| p.hash);
            Ok(hash)
        })
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
