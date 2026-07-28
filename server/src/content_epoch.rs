//! T0.72: content/config epoch + hot-reload admission barrier.
//!
//! `common-assets/hot-reloading` cascades (Cargo feature unification) into
//! any build that depends on a crate enabling it -- today that's voxygen's
//! default features, which transitively turns it on for the `server` crate
//! embedded in singleplayer, even though a standalone `server-cli` default
//! build never enables it. Nothing on the authoritative (server/common/
//! rtsim) side currently checks for a reload at all: a background config
//! edit can swap live data transparently, mid-tick, with no ordering
//! guarantee. This module is the fix: ONE named barrier, called once at
//! the very start of a tick (`Server::tick`, before the ECS dispatcher
//! runs), that detects reloads on the fixed set of boot-known content
//! manifests and advances a typed [`ContentEpoch`] exactly once per tick.
//!
//! This is the THIRD time this codebase has hit the same feature-
//! unification class (one crate's enabled feature silently applies to a
//! shared dependency for every crate in the build graph, not just the
//! crate that asked for it): (1) a false-positive `E0061` from `plugins`/
//! `PluginMgr` was already logged as a known trap in
//! `readme/SESSION-HANDOFF-2026-07-27-sonnet-apex-pivot.md` ("a narrow
//! `cargo check -p X -p Y` can silently select a DIFFERENT feature set
//! than `--workspace`"); (2) a REAL instance of that same PluginMgr arity
//! break was found this row (`client/src/lib.rs:941` calling
//! `State::client()` with the pre-T2.5.18/.19 arity, on a branch that
//! never got that feature merged -- left as a known-red, see
//! `docs/BASTION_RUN_LOG.md`); (3) this module's own motivating finding
//! (voxygen's default `hot-reloading` feature transitively arming
//! `common-assets/hot-reloading` on its embedded singleplayer `server`).
//!
//! Scope fence (disclosed, not silently narrowed): assets loaded by a
//! runtime-supplied id -- `EntityConfig` at dynamic entity-spawn sites,
//! the hardcoded potion `ItemDef` in `events::trade` -- have no fixed
//! handle to watch ahead of time (`assets_manager::ReloadWatcher` is
//! per-asset-entry, not cache-wide), so detecting THEIR changes isn't
//! covered here. The barrier still makes `ContentEpoch`'s current value
//! well-defined and stable for the whole tick, so those sites can stamp
//! it onto whatever they create even without a watcher of their own.

use common::{
    assets::{AssetExt, ReloadWatcher},
    comp,
    comp::inventory::item::MaterialStatManifest,
    recipe::RecipeBookManifest,
    resources::ContentEpoch,
};

/// The fixed set of boot-known content manifests the admission barrier
/// watches, named for disclosure/debugging (not currently read besides
/// that).
pub struct ContentWatchers {
    watchers: Vec<(&'static str, ReloadWatcher)>,
}

impl ContentWatchers {
    pub fn new() -> Self {
        Self {
            watchers: vec![
                (
                    "common.recipe_book_manifest",
                    RecipeBookManifest::load().reload_watcher(),
                ),
                (
                    "common.material_stats_manifest",
                    MaterialStatManifest::load().reload_watcher(),
                ),
                (
                    "common.abilities.ability_set_manifest",
                    comp::item::tool::AbilityMap::<comp::AbilityItem>::load_expect(
                        "common.abilities.ability_set_manifest",
                    )
                    .reload_watcher(),
                ),
            ],
        }
    }

    /// The ONE named admission barrier. Call once, at the very start of a
    /// tick, before anything reads content. Increments `epoch` exactly
    /// once for this call even if several watched manifests changed
    /// together, and returns their names for logging/disclosure.
    ///
    /// Every watcher is always polled (never short-circuits): `reloaded`
    /// is edge-triggered per watcher, so skipping one would leave its
    /// pending-change flag set and wrongly fire on some later, unrelated
    /// tick.
    pub fn poll_and_admit(&mut self, epoch: &mut ContentEpoch) -> Vec<&'static str> {
        let changed: Vec<&'static str> = self
            .watchers
            .iter_mut()
            .filter_map(|(name, watcher)| watcher.reloaded().then_some(*name))
            .collect();
        if !changed.is_empty() {
            epoch.0 += 1;
        }
        changed
    }
}

impl Default for ContentWatchers {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reload_leaves_the_epoch_unchanged() {
        let mut watchers = ContentWatchers::new();
        let mut epoch = ContentEpoch::default();
        let changed = watchers.poll_and_admit(&mut epoch);
        // Fresh watchers have nothing pending: construction itself isn't a
        // reload.
        assert!(changed.is_empty());
        assert_eq!(epoch, ContentEpoch(0));
    }

    #[test]
    fn repeated_polls_with_no_change_never_advance_the_epoch() {
        let mut watchers = ContentWatchers::new();
        let mut epoch = ContentEpoch::default();
        for _ in 0..5 {
            watchers.poll_and_admit(&mut epoch);
        }
        assert_eq!(epoch, ContentEpoch(0));
    }

    #[test]
    fn watches_every_boot_known_manifest_by_name() {
        let watchers = ContentWatchers::new();
        let names: Vec<&'static str> = watchers.watchers.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec![
            "common.recipe_book_manifest",
            "common.material_stats_manifest",
            "common.abilities.ability_set_manifest",
        ]);
    }
}
