use std::{path::PathBuf, sync::RwLock};

use super::{ASSETS_PATH, Concatenate, fs::FileSystem};
use assets_manager::{
    Asset, AssetCache, BoxedError, Storable,
    asset::DirLoadable,
    hot_reloading::EventSender,
    source::{FileContent, Source, Tar},
};

struct PluginEntry {
    path: PathBuf,
    cache: AssetCache,
}

/// APEX-T2.1.08 — a privately-prepared, not-yet-published tar asset source.
/// Holding one of these has NO effect on any `CombinedSource`; only
/// `CombinedCache::commit_prepared_tars` publishes it.
pub struct PreparedPluginAssetSource {
    path: PathBuf,
    cache: AssetCache,
}

impl PreparedPluginAssetSource {
    pub fn path(&self) -> &std::path::Path { &self.path }
}

/// APEX-T2.1.09 — typed commit failure: the registry write lock was poisoned;
/// nothing was appended.
#[derive(Debug)]
pub struct CommitLockPoisoned;

/// The location of this asset
enum AssetSource {
    FileSystem,
    Plugin { index: usize },
}

struct SourceAndContents<'a>(AssetSource, FileContent<'a>);

/// This source combines assets loaded from the filesystem and from plugins.
/// It is typically used via the CombinedCache type.
///
/// A load will search through all sources and warn about unhandled duplicates.
pub struct CombinedSource {
    fs: FileSystem,
    plugin_list: RwLock<Vec<PluginEntry>>,
}

impl CombinedSource {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            fs: FileSystem::new()?,
            plugin_list: RwLock::new(Vec::new()),
        })
    }
}

impl CombinedSource {
    /// Look for an asset in all known sources
    fn read_multiple(&self, id: &str, ext: &str) -> Vec<SourceAndContents<'_>> {
        let mut result = Vec::new();
        if let Ok(file_entry) = self.fs.read(id, ext) {
            result.push(SourceAndContents(AssetSource::FileSystem, file_entry));
        }
        for (n, p) in self.plugin_list.read().unwrap().iter().enumerate() {
            if let Ok(entry) = p.cache.source().read(id, ext) {
                // the data is behind an RwLockReadGuard, so own it for returning
                result.push(SourceAndContents(
                    AssetSource::Plugin { index: n },
                    match entry {
                        FileContent::Slice(s) => FileContent::Buffer(Vec::from(s)),
                        FileContent::Buffer(b) => FileContent::Buffer(b),
                        FileContent::Owned(s) => {
                            FileContent::Buffer(Vec::from(s.as_ref().as_ref()))
                        },
                    },
                ));
            }
        }
        result
    }

    /// Return the path of a source
    fn plugin_path(&self, index: &AssetSource) -> Option<PathBuf> {
        match index {
            AssetSource::FileSystem => Some(ASSETS_PATH.clone()),
            AssetSource::Plugin { index } => self.plugin_list
                .read()
                .unwrap()
                .get(*index)
                // We don't want to keep the lock, so we clone
                .map(|plugin| plugin.path.clone()),
        }
    }
}

impl Source for CombinedSource {
    fn read(&self, id: &str, ext: &str) -> std::io::Result<FileContent<'_>> {
        // We could shortcut on fs if we dont check for conflicts
        let mut entries = self.read_multiple(id, ext);
        if entries.is_empty() {
            Err(std::io::ErrorKind::NotFound.into())
        } else {
            if entries.len() > 1 {
                let patha = self.plugin_path(&entries[0].0);
                let pathb = self.plugin_path(&entries[1].0);
                tracing::error!("Duplicate asset {id} in {patha:?} and {pathb:?}");
            }
            // unconditionally return the first asset found
            Ok(entries.swap_remove(0).1)
        }
    }

    fn read_dir(
        &self,
        id: &str,
        f: &mut dyn FnMut(assets_manager::source::DirEntry),
    ) -> std::io::Result<()> {
        // TODO: We should combine the sources, but this isn't used in veloren
        self.fs.read_dir(id, f)
    }

    fn exists(&self, entry: assets_manager::source::DirEntry) -> bool {
        self.fs.exists(entry)
            || self
                .plugin_list
                .read()
                .unwrap()
                .iter()
                .any(|plugin| plugin.cache.source().exists(entry))
    }

    // TODO: Enable hot reloading for plugins
    fn configure_hot_reloading(&self, events: EventSender) -> Result<(), BoxedError> {
        self.fs.configure_hot_reloading(events)
    }
}

/// A cache combining filesystem and plugin assets
pub struct CombinedCache(AssetCache);

impl CombinedCache {
    pub fn new() -> std::io::Result<Self> {
        CombinedSource::new().map(|combined_source| Self(AssetCache::with_source(combined_source)))
    }

    pub fn as_cache(&self) -> &AssetCache { &self.0 }

    /// Combine objects from filesystem and plugins
    pub fn combine<T: Concatenate>(
        &self,
        // this cache registers with hot reloading
        cache: &AssetCache,
        mut load_from: impl FnMut(&AssetCache) -> Result<T, assets_manager::Error>,
    ) -> Result<T, assets_manager::Error> {
        let mut result = load_from(cache);
        // Report a severe error from the filesystem asset even if later overwritten by
        // an Ok value from a plugin
        if let Err(ref fs_error) = result {
            match fs_error
                .reason()
                .downcast_ref::<std::io::Error>()
                .map(|io_error| io_error.kind())
            {
                Some(std::io::ErrorKind::NotFound) => (),
                _ => tracing::error!("Filesystem asset load {fs_error:?}"),
            }
        }
        for plugin in self
            .0
            .downcast_raw_source::<CombinedSource>()
            .unwrap()
            .plugin_list
            .read()
            .unwrap()
            .iter()
        {
            match load_from(&plugin.cache) {
                Ok(b) => {
                    result = if let Ok(a) = result {
                        Ok(a.concatenate(b))
                    } else {
                        Ok(b)
                    };
                },
                // Report any error other than NotFound
                Err(plugin_error) => {
                    match plugin_error
                        .reason()
                        .downcast_ref::<std::io::Error>()
                        .map(|io_error| io_error.kind())
                    {
                        Some(std::io::ErrorKind::NotFound) => (),
                        _ => tracing::error!(
                            "Loading from {:?} failed {plugin_error:?}",
                            plugin.path
                        ),
                    }
                },
            }
        }
        result
    }

    /// Add a tar archive (a plugin) to the system.
    /// All files in that tar file become potential assets.
    ///
    /// APEX-T2.1.08: now a thin wrapper over the split prepare/commit path —
    /// retained for API compatibility; batch loaders should prepare ALL sources
    /// privately and commit once (`commit_prepared_tars`).
    pub fn register_tar(&self, path: PathBuf) -> std::io::Result<()> {
        let prepared = Self::prepare_tar(path)?;
        self.commit_prepared_tars(vec![prepared])
            .map_err(|CommitLockPoisoned| {
                std::io::Error::other("plugin asset registry lock poisoned")
            })
    }

    /// APEX-T2.1.08 — construct a tar-backed asset source PRIVATELY: this can
    /// fail (missing/invalid tar) without any effect on the published
    /// `plugin_list`. Publication is a separate, deliberate step
    /// (`commit_prepared_tars`). An associated fn on purpose: preparation has
    /// no access to `self`, so it CANNOT touch the registry by construction.
    pub fn prepare_tar(path: PathBuf) -> std::io::Result<PreparedPluginAssetSource> {
        let tar_source = Tar::open(&path)?;
        let cache = AssetCache::with_source(tar_source);
        Ok(PreparedPluginAssetSource { path, cache })
    }

    /// APEX-T2.1.09 — publish a fully-prepared batch under ONE write-lock
    /// acquisition. All fallible work (tar open, cache construction) happened
    /// in `prepare_tar`; this is extend-only, so a batch either publishes
    /// completely or (on a poisoned lock) not at all — no partial registration.
    pub fn commit_prepared_tars(
        &self,
        prepared: Vec<PreparedPluginAssetSource>,
    ) -> Result<(), CommitLockPoisoned> {
        let mut plugin_list = self
            .0
            .downcast_raw_source::<CombinedSource>()
            .unwrap()
            .plugin_list
            .write()
            .map_err(|_| CommitLockPoisoned)?;
        plugin_list.extend(
            prepared
                .into_iter()
                .map(|PreparedPluginAssetSource { path, cache }| PluginEntry { path, cache }),
        );
        // DET-AST-034 (v6 deep-pass, Critical): `combine` folds every plugin's
        // asset into the base LAST-WRITER-WINS in `plugin_list` order (see
        // `Concatenate`, DET-AST-014). Sources arrive in `fs::read_dir`
        // (OS directory) order for filesystem plugins and network-arrival order
        // for server-delivered ones — so authoritative combined RON tables
        // (recipes, abilities, …) inherited a non-canonical merge order. Keep
        // `plugin_list` sorted by tar path: all entries share the same
        // plugins-root prefix, so the ordering key reduces to the per-plugin
        // suffix and is identical across machines for a fixed plugin set,
        // making the fold a pure function of that set. (Preserved through the
        // T2.1 batch split; T2.4 owns any future canonical activation order.)
        plugin_list.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(())
    }

    /// APEX-T2.1.15 — read-only test seam: the number of published plugin
    /// asset sources. Lets tests assert ZERO source delta after a rejected
    /// batch on a LOCAL `CombinedCache` (never the process-global one).
    pub fn plugin_source_count(&self) -> usize {
        self.0
            .downcast_raw_source::<CombinedSource>()
            .unwrap()
            .plugin_list
            .read()
            .map(|l| l.len())
            .unwrap_or(0)
    }

    pub fn no_record<T>(&self, f: impl FnOnce() -> T) -> T { self.0.no_record(f) }

    // Just forward these methods to the cache
    #[inline]
    pub fn load_rec_dir<A: DirLoadable + Asset>(
        &self,
        id: &str,
    ) -> Result<&assets_manager::Handle<assets_manager::RecursiveDirectory<A>>, assets_manager::Error>
    {
        self.0.load_rec_dir(id)
    }

    #[inline]
    pub fn load<A: Asset>(
        &self,
        id: &str,
    ) -> Result<&assets_manager::Handle<A>, assets_manager::Error> {
        self.0.load(id)
    }

    #[inline]
    pub fn get_or_insert<A: Storable>(&self, id: &str, a: A) -> &assets_manager::Handle<A> {
        self.0.get_or_insert(id, a)
    }

    #[inline]
    pub fn load_owned<A: Asset>(&self, id: &str) -> Result<A, assets_manager::Error> {
        self.0.load_owned(id)
    }
}

/// APEX-T2.1.16 — prepare/commit canaries on a LOCAL `CombinedCache` (never
/// the process-global one — PLG2P canary rule: local caches + explicit source
/// counts, no global-pollution false greens). Case IDs map to
/// PROJECT-BASTION-APEX-T2.1-TWO-PHASE-PLUGIN-CANARIES-v1.json.
#[cfg(test)]
mod two_phase_asset_tests {
    use super::*;

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

    fn temp_tar(name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "apex-t21-{}-{}-{name}.plugin.tar",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&p, tar_bytes(entries)).unwrap();
        p
    }

    /// PLG2P-017: preparation touches no plugin_list; PLG2P-019/022: batch
    /// commit publishes all sources under one lock, count delta exact.
    #[test]
    fn plg2p_017_019_prepare_is_private_commit_is_batch() {
        let cache = CombinedCache::new().unwrap();
        let a = temp_tar("a", &[("plugin.toml", b"x = 1\n".as_slice())]);
        let b = temp_tar("b", &[("plugin.toml", b"x = 2\n".as_slice())]);
        let pa = CombinedCache::prepare_tar(a).unwrap();
        let pb = CombinedCache::prepare_tar(b).unwrap();
        // PLG2P-017: zero delta while prepared-but-uncommitted.
        assert_eq!(cache.plugin_source_count(), 0, "prepare must not publish");
        cache.commit_prepared_tars(vec![pa, pb]).unwrap();
        // PLG2P-019: exact batch delta.
        assert_eq!(cache.plugin_source_count(), 2);
    }

    /// PLG2P-013-class: a failed preparation (missing tar) publishes nothing.
    #[test]
    fn plg2p_013_failed_prepare_zero_delta() {
        let cache = CombinedCache::new().unwrap();
        let missing = std::env::temp_dir().join("apex-t21-definitely-missing.plugin.tar");
        assert!(CombinedCache::prepare_tar(missing).is_err());
        assert_eq!(cache.plugin_source_count(), 0);
    }

    /// PLG2P-020-class: an asset unreadable before commit becomes readable
    /// after (publication is the visibility boundary).
    #[test]
    fn plg2p_020_visibility_flips_at_commit() {
        let cache = CombinedCache::new().unwrap();
        let a = temp_tar(
            "vis",
            &[("test_asset.ron", b"(x: 1)".as_slice())],
        );
        let prepared = CombinedCache::prepare_tar(a).unwrap();
        let source = cache.as_cache().downcast_raw_source::<CombinedSource>().unwrap();
        assert!(
            source.read("test_asset", "ron").is_err(),
            "asset must be invisible before commit"
        );
        cache.commit_prepared_tars(vec![prepared]).unwrap();
        assert!(
            source.read("test_asset", "ron").is_ok(),
            "asset must be visible after commit"
        );
    }

    /// DET-AST-034 (preserved through the split): committed entries stay
    /// path-sorted regardless of commit input order.
    #[test]
    fn det_ast_034_commit_preserves_canonical_path_sort() {
        let cache = CombinedCache::new().unwrap();
        let a = temp_tar("z-last", &[("plugin.toml", b"x = 1\n".as_slice())]);
        let b = temp_tar("a-first", &[("plugin.toml", b"x = 2\n".as_slice())]);
        // Commit in "wrong" (z before a) order…
        let pa = CombinedCache::prepare_tar(a.clone()).unwrap();
        let pb = CombinedCache::prepare_tar(b.clone()).unwrap();
        cache.commit_prepared_tars(vec![pa, pb]).unwrap();
        let source = cache.as_cache().downcast_raw_source::<CombinedSource>().unwrap();
        let paths: Vec<PathBuf> = source
            .plugin_list
            .read()
            .unwrap()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted, "plugin_list must stay path-sorted (DET-AST-034)");
    }
}
