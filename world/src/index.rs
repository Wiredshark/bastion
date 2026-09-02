use crate::{
    Colors, Features,
    layer::wildlife::{self, DensityFn, SpawnEntry},
    site::{Site, economy::TradeInformation},
};
use common::{
    assets::{AssetExt, AssetHandle, ReloadWatcher, Ron},
    store::Store,
    trade::{SiteId, SitePrices},
};
use core::ops::Deref;
use noise::{Fbm, MultiFractal, Perlin, SuperSimplex};
use std::sync::Arc;

const WORLD_COLORS_MANIFEST: &str = "world.style.colors";
const WORLD_FEATURES_MANIFEST: &str = "world.features";

pub struct Index {
    pub seed: u32,
    pub time: f32,
    pub noise: Noise,
    pub sites: Store<Site>,
    pub trade: TradeInformation,
    pub wildlife_spawns: Vec<(AssetHandle<Ron<SpawnEntry>>, DensityFn)>,
    colors: AssetHandle<Arc<Colors>>,
    features: AssetHandle<Arc<Features>>,
}

/// An owned reference to indexed data.
///
/// The data are split out so that we can replace the colors without disturbing
/// the rest of the index, while also keeping all the data within a single
/// indirection.
#[derive(Clone)]
pub struct IndexOwned {
    colors: Arc<Colors>,
    features: Arc<Features>,
    colors_reload_watcher: ReloadWatcher,
    features_reload_watcher: ReloadWatcher,
    index: Arc<Index>,
}

impl Deref for IndexOwned {
    type Target = Index;

    fn deref(&self) -> &Self::Target { &self.index }
}

/// A shared reference to indexed data.
///
/// This is copyable and can be used from either style of index.
#[derive(Clone, Copy)]
pub struct IndexRef<'a> {
    pub colors: &'a Colors,
    pub features: &'a Features,
    pub index: &'a Index,
}

impl Deref for IndexRef<'_> {
    type Target = Index;

    fn deref(&self) -> &Self::Target { self.index }
}

impl Index {
    /// NOTE: Panics if the color manifest cannot be loaded.
    pub fn new(seed: u32) -> Self {
        let colors = Arc::<Colors>::load_expect(WORLD_COLORS_MANIFEST);
        let features = Arc::<Features>::load_expect(WORLD_FEATURES_MANIFEST);
        let wildlife_spawns = wildlife::spawn_manifest()
            .into_iter()
            .map(|(e, f)| (Ron::<SpawnEntry>::load_expect(e), f))
            .collect();

        Self {
            seed,
            time: 0.0,
            noise: Noise::new(seed),
            sites: Store::default(),
            trade: Default::default(),
            wildlife_spawns,
            colors,
            features,
        }
    }

    pub fn colors(&self) -> impl Deref<Target = Arc<Colors>> + '_ { self.colors.read() }

    pub fn features(&self) -> impl Deref<Target = Arc<Features>> + '_ { self.features.read() }

    pub fn get_site_prices(&self, site_id: SiteId) -> Option<SitePrices> {
        self.sites
            .recreate_id(site_id)
            .map(|i| self.sites.get(i))
            .and_then(|s| s.economy.as_ref())
            .map(|econ| econ.get_site_prices())
    }

    /// `APEX-T4.3` chunk 2a helper, factored out for `T8.1`: every site's
    /// own `Economy::canonical_baseline_hash_v1`, sorted by site id
    /// (same canonicalize-before-hash discipline as
    /// `Civs::baseline_site_graph_v1`, `E11-3b`). A site with no
    /// `Economy` yet contributes nothing, same as an absent descriptor
    /// elsewhere in this program. `world_economy_root_v1` reduces this
    /// to one composite root; `T8.1`'s per-phase evidence collection
    /// (`world/src/site/economy/context.rs`) reuses the SAME per-site
    /// digests directly, rather than re-deriving them, so the two never
    /// drift.
    pub fn world_economy_per_site_v1(&self) -> Vec<(u64, common::apex::digest::ArtifactDigestV1)> {
        let mut per_site: Vec<(u64, common::apex::digest::ArtifactDigestV1)> = self
            .sites
            .ids()
            .filter_map(|id| {
                let site = self.sites.get(id);
                site.economy.as_ref().map(|economy| (id.id(), economy.canonical_baseline_hash_v1().digest))
            })
            .collect();
        per_site.sort_unstable_by_key(|(id, _)| *id);
        per_site
    }

    /// `APEX-T4.3`: the "economic baseline" component of
    /// `WorldBaselineManifestV1` -- one composite root over every site's
    /// own economic baseline, since the spec asks for ONE economic-
    /// baseline root and `Economy` is inherently per-site.
    pub fn world_economy_root_v1(&self) -> common::apex::digest::ArtifactIdentityV1 {
        Self::economy_root_from_per_site_v1(&self.world_economy_per_site_v1())
    }

    /// Pure reduction: per-site digests (already sorted by
    /// [`Self::world_economy_per_site_v1`]) to one composite root. Split
    /// out so `T8.1`'s per-phase evidence can compute the same root a
    /// live phase would produce without re-deriving the reduction.
    pub fn economy_root_from_per_site_v1(
        per_site: &[(u64, common::apex::digest::ArtifactDigestV1)],
    ) -> common::apex::digest::ArtifactIdentityV1 {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(per_site.len() as u64).to_be_bytes());
        for (id, digest) in per_site {
            buf.extend_from_slice(&id.to_be_bytes());
            buf.extend_from_slice(digest.bytes.as_array());
        }
        common::apex::digest::hash_artifact_bytes_v1(&buf)
    }
}

impl IndexOwned {
    pub fn new(index: Index) -> Self {
        let colors = index.colors.cloned();
        let features = index.features.cloned();
        let colors_reload_watcher = index.colors.reload_watcher();
        let features_reload_watcher = index.features.reload_watcher();

        Self {
            index: Arc::new(index),
            colors,
            features,
            colors_reload_watcher,
            features_reload_watcher,
        }
    }

    /// NOTE: Callback is called only when colors actually have to be reloaded.
    /// The server is responsible for making sure that all affected chunks are
    /// reloaded; a naive approach will just regenerate every chunk on the
    /// server, but it is possible that eventually we can find a better
    /// solution.
    ///
    /// Ideally, this should be called about once per tick.
    pub fn reload_if_changed<R>(&mut self, reload: impl FnOnce(&mut Self) -> R) -> Option<R> {
        let colors_reloaded = self.colors_reload_watcher.reloaded();
        let features_reloaded = self.features_reload_watcher.reloaded();
        let reloaded = colors_reloaded || features_reloaded;
        reloaded.then(move || {
            // Reload the fields from the asset handle, which is updated automatically
            self.colors = self.index.colors.cloned();
            self.features = self.index.features.cloned();
            // Update wildlife spawns which is based on base_density in features
            reload(self)
        })
    }

    pub fn as_index_ref(&self) -> IndexRef<'_> {
        IndexRef {
            colors: &self.colors,
            features: &self.features,
            index: &self.index,
        }
    }

    /// Mutable access to the index itself, if and only if this `IndexOwned` is
    /// the sole owner of it.
    ///
    /// Everything else here hands out `&Index`, because the index is
    /// conceptually immutable once the world is generated: it is `Arc`-shared
    /// with every in-flight chunk job precisely so those jobs can read it from
    /// worker threads without a lock.
    ///
    /// The Bastion colony breaks that assumption in one specific way: as the
    /// town grows, worldgen lays out a new plot *on the site that is already in
    /// the index*, so that every later reader — pathfinding, the next plot's
    /// placement search, the engine's own chunk render — sees the new building.
    /// Copying the site out, mutating it and putting it back is not an option:
    /// the copy would be stale the moment any other system touched the
    /// original, and there is nowhere to "put it back" that the readers share.
    ///
    /// So the mutation has to happen in place, and the only safe moment to do
    /// it is a tick when nothing else holds the `Arc`. `Arc::get_mut` is
    /// exactly that test, and it is a test rather than a wait *on purpose*: a
    /// `Some` here is a proof that no chunk job is reading the index right now
    /// (see `server/src/chunk_generator.rs`, which clones the `IndexOwned` into
    /// every job it dispatches), and a `None` is a proof that one is.
    ///
    /// A caller that gets `None` must **defer, not block and not clone** — the
    /// colony simply tries again next tick, and the town grows a fraction of a
    /// second later. Blocking here would stall the tick behind chunk
    /// generation; cloning would silently throw the mutation away.
    pub fn try_index_mut(&mut self) -> Option<&mut Index> { Arc::get_mut(&mut self.index) }

    /// How many owners the index `Arc` currently has. `1` means
    /// [`Self::try_index_mut`] will succeed.
    ///
    /// This exists so a refusal can carry its own producer: "the index was
    /// shared" is not actionable, but "the index was shared by 3" says how many
    /// chunk jobs were in flight, which is what tells a caller whether it is
    /// waiting on a momentary blip or on a sustained generation storm.
    pub fn index_strong_count(&self) -> usize { Arc::strong_count(&self.index) }
}

pub struct Noise {
    pub cave_nz: SuperSimplex,
    pub scatter_nz: SuperSimplex,
    pub cave_fbm_nz: Fbm<Perlin>,
}

impl Noise {
    fn new(seed: u32) -> Self {
        Self {
            cave_nz: SuperSimplex::new(seed + 0),
            scatter_nz: SuperSimplex::new(seed + 1),
            cave_fbm_nz: Fbm::new(seed + 2).set_octaves(5),
        }
    }
}
