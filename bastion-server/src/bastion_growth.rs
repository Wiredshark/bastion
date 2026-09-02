//! bastion (G1b): grow the colony's town by one plot, **in place, in the live
//! world index**.
//!
//! `world::site::bastion_layout` can lay a plot out given a `&mut Site`. On the
//! server there is no such `&mut`: the `Site` lives inside `IndexOwned`, behind
//! an `Arc<Index>` that the chunk generator clones into every chunk job it
//! dispatches (`server/src/chunk_generator.rs`). This module is the seam that
//! reconciles the two.
//!
//! # Why mutate the index at all
//!
//! Because every reader of the town reads it *through the index*: the next
//! plot's placement search, the tile grid the pathfinder walks, and the
//! engine's own `Site::render` when a chunk is (re)generated. A plot laid out
//! on a copy of the site would be invisible to all three — the colony would
//! build a house that worldgen did not believe in.
//!
//! # Why it can refuse
//!
//! `Arc::get_mut` succeeds only when this `IndexOwned` is the sole owner, i.e.
//! only on a tick with no chunk job in flight. That is not a limitation to be
//! worked around; it is the safety property. A `Some` is a *proof* that nobody
//! is reading the index right now, which is exactly the precondition for
//! mutating it without a lock. So `grow_plot` **refuses** rather than blocking
//! or cloning: [`GrowRefusal::IndexShared`] means "not this tick", and the
//! caller simply asks again next tick. On a 30 tps server that costs the colony
//! a few tens of milliseconds, not a building.
//!
//! The refusal is checked **before anything is spent** — before the site is
//! looked up, before `Land` is built, before any rng is drawn — so a refused
//! call is free and, more importantly, leaves the world byte-identical.
//!
//! # What this module deliberately does NOT do
//!
//! Nothing here is wired to the job board, the housing verdict, or block
//! placement, and nothing here is persisted. `grow_plot` is a pure "ask
//! worldgen for one more plot and tell me what it looks like" call. Deciding
//! *when* to call it, turning [`GrownPlot::blocks`] into build jobs, and
//! surviving a restart are separate pieces of work.

use common::{
    store::Id,
    terrain::Block,
};
use vek::*;
use world::{
    Land,
    site::{
        Site,
        bastion_layout::{LayoutKind, PlacedPlot, place_plot_for_colony, render_placed_plot},
    },
};

/// Why the colony did not get a plot this call.
///
/// Every variant leaves the world exactly as it was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowRefusal {
    /// The world index is shared right now — at least one chunk job holds a
    /// clone of it — so it cannot be mutated. **Retry next tick.** This is the
    /// expected, healthy refusal, not an error.
    ///
    /// `strong_count` is the number of owners of the index `Arc` at the moment
    /// of the refusal (`1` would have succeeded), so a caller that sees this
    /// constantly can tell a busy generation storm from a leaked clone.
    IndexShared { strong_count: usize },
    /// The site id does not name a site in this index. Stale id — a persisted
    /// colony pointing at a world that no longer has that site, most likely.
    ///
    /// This is checked rather than trusted because `common::store::Store`'s
    /// `get`/`get_mut` `unwrap()` an out-of-range index, i.e. a stale id would
    /// take the server down rather than return an error.
    NoSuchSite,
    /// Worldgen could not fit the plot anywhere on the site: the placement
    /// search (`find_roadside_aabr`/`find_rural_aabr`, 32 attempts) failed.
    ///
    /// **This does NOT mean the town is full.** The search picks ONE random
    /// plaza or road node to grow from and gives up after 32 attempts around
    /// it, so an unlucky seed refuses on a town with plenty of space. Measured
    /// on the fresh test town (`no_room_leaves_the_site_untouched`): of 200
    /// consecutive seeds, **11 grew a house and 189 refused** — and 9 of those
    /// 11 came *after* the first refusal. A caller that read one `NoRoom` as
    /// "the town is full" would have stopped at 2 houses instead of 11.
    ///
    /// So: retry with a different seed. Only a long run of refusals is
    /// evidence of saturation, and even then it is evidence about the roadside
    /// ring, not the site.
    ///
    /// The site is untouched: the search runs entirely before `create_plot`,
    /// so a failed search inserts nothing and blits nothing.
    NoRoom,
}

/// One plot the colony successfully grew, and everything a builder needs to
/// put it up.
///
/// The plot is **already in the world** by the time this exists — it is in
/// `site.plots` and its tiles are blitted — so worldgen, pathfinding and the
/// chunk renderer all already agree it is there. What is left is the physical
/// construction, which is what `blocks` describes.
pub struct GrownPlot {
    /// The site the plot was added to.
    pub site: Id<Site>,
    /// Where and what worldgen put down: plot id, footprint, door, altitude.
    pub placed: PlacedPlot,
    /// Every non-air block the plot is made of, sorted by (z, y, x) so a
    /// builder can lay courses bottom-up. May legitimately be EMPTY for plot
    /// kinds that paint themselves through the terrain-surface hook rather
    /// than a fill tree — a farm field, notably. See
    /// `world::site::bastion_layout::LaidOutPlot::blocks`.
    pub blocks: Vec<(Vec3<i32>, Block)>,
    /// The position of each bed's HEAD sprite — one entry per bed, so the
    /// housing verdict can count sleepers straight off this.
    pub beds: Vec<Vec3<i32>>,
    /// The seed the layout was driven by. Kept so the same plot can be
    /// re-derived: site state + seed determines the whole result.
    pub seed: u64,
}

/// Grow the town on `site` by one plot of `kind`, mutating the world index in
/// place.
///
/// On `Ok` the plot is already part of the world; the returned blocks are what
/// still has to be physically built.
///
/// On `Err` **nothing changed** — see [`GrowRefusal`] for which of the three
/// refusals happened and whether it is worth retrying (only `IndexShared` is).
///
/// `sim` must be the same `WorldSim` the world was generated from; it supplies
/// both the terrain altitudes the plot generator needs and the mock canvas the
/// plot is rendered against.
///
/// The order below is load-bearing: the sharing guard runs first, so a refused
/// call spends nothing and — crucially — cannot have half-mutated the index.
pub fn grow_plot(
    owned: &mut world::IndexOwned,
    sim: &world::sim::WorldSim,
    site: Id<Site>,
    kind: LayoutKind,
    seed: u64,
) -> Result<GrownPlot, GrowRefusal> {
    // Read the count BEFORE the `get_mut` attempt so the refusal can name it.
    // Cheap and read-only; this is not "spending".
    let strong_count = owned.index_strong_count();

    // --- Phase 1: mutate. Needs `&mut Index`, must NOT hold an `IndexRef`. --
    let placed = {
        // A guard must refuse before it spends: this is the very first thing
        // that can fail, and it fails having touched nothing.
        let Some(index) = owned.try_index_mut() else {
            return Err(refuse(GrowRefusal::IndexShared { strong_count }));
        };

        // `Store::get`/`get_mut` panic on an out-of-range id (they `unwrap()`
        // an `Option`), so the id has to be validated first. `recreate_id` is
        // the store's own bounds check and is the only non-panicking way to
        // ask "is this id live?".
        if index.sites.recreate_id(site.id()).is_none() {
            return Err(refuse(GrowRefusal::NoSuchSite));
        }

        // `Land` borrows `sim`, not the index, so it is free to coexist with
        // the `&mut Index` below.
        let land = Land::from_sim(sim);
        let site_mut = index.sites.get_mut(site);

        match place_plot_for_colony(site_mut, kind, &land, seed) {
            Some(placed) => placed,
            // The placement search failed before `create_plot`/`blit_aabr`, so
            // the site is untouched by construction.
            None => return Err(refuse(GrowRefusal::NoRoom)),
        }
    };

    // --- Phase 2: render. Needs `IndexRef`; the `&mut` above has ended. -----
    // `PlacedPlot` borrows nothing, which is the whole reason the layout is
    // split in two: this line would not compile if it did.
    let index_ref = owned.as_index_ref();
    let site_ref = index_ref.sites.get(site);
    let (blocks, beds) = render_placed_plot(site_ref, &placed, index_ref, sim);

    // ONE witness per grown plot. Everything this line prints is a number a
    // reader can go and check: the plot is in the site, the aabr is where the
    // building stands, `beds` is what the housing verdict will count.
    tracing::info!(
        kind = ?kind,
        plot = ?placed.plot,
        aabr_wpos = ?placed.aabr_wpos,
        door = ?placed.door_wpos,
        beds = beds.len(),
        blocks = blocks.len(),
        seed,
        "bastion: PLOT LAID OUT — worldgen placed a plot for the colony"
    );

    Ok(GrownPlot {
        site,
        placed,
        blocks,
        beds,
        seed,
    })
}

/// Log every refusal on the way out, so a colony that never grows leaves a
/// trail saying which of the three reasons it was.
fn refuse(refusal: GrowRefusal) -> GrowRefusal {
    tracing::info!(refusal = ?refusal, "bastion: PLOT LAYOUT REFUSED");
    refusal
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaChaRng;
    use rand::SeedableRng;
    use world::{
        IndexRef,
        index::{Index, IndexOwned},
        sim::{self, DEFAULT_WORLD_MAP, FileOpts, WorldOpts},
        site::SitesGenMeta,
    };

    const WORLD_SEED: u32 = 230;

    /// Install a subscriber so `grow_plot`'s `tracing::info!` witness actually
    /// reaches the test output under `--nocapture`.
    ///
    /// Without this every witness in this module is a no-op and these tests
    /// would prove the log line *compiles*, not that it *fires* with the fields
    /// it claims. Idempotent: only the first caller wins, which is what we
    /// want when the test harness runs these in parallel.
    fn show_witnesses() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            // No colour: an ANSI-escaped witness line cannot be grepped as
            // key=value by whoever reads this evidence later.
            .with_ansi(false)
            .with_test_writer()
            .try_init();
    }

    /// Same world the `bastion_layout` tests build: the shipped map asset (so
    /// no erosion pass) plus a fresh index. ~3 s.
    fn world() -> (sim::WorldSim, IndexOwned) {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let sim = sim::WorldSim::generate(
            WORLD_SEED,
            WorldOpts {
                seed_elements: true,
                world_file: FileOpts::LoadAsset(DEFAULT_WORLD_MAP.into()),
                calendar: None,
            },
            &pool,
            &|_| {},
        );
        (sim, IndexOwned::new(Index::new(WORLD_SEED)))
    }

    /// Deterministic buildable spot near the map centre (same helper as the
    /// layout tests: fixed spiral, first hit wins).
    fn buildable_origin(land: &Land) -> Vec2<i32> {
        use common::{terrain::TerrainChunkSize, vol::RectVolSize};
        let size = land.size();
        let centre_chunk = (size / 2).as_::<i32>();
        for offset in common::spiral::Spiral2d::new().take(4096) {
            let chunk = centre_chunk + offset * 4;
            let wpos = chunk * TerrainChunkSize::RECT_SIZE.as_::<i32>();
            let alt = land.get_alt_approx(wpos);
            let flat_enough = land.get_gradient_approx(wpos) < 0.35;
            if alt > world::CONFIG.sea_level + 12.0 && flat_enough {
                return wpos;
            }
        }
        panic!("no buildable origin found near the centre of the map");
    }

    /// Build a town and put it INTO the index, the way `world::civ` does
    /// (`Store::insert`), so the site under test is a real index resident and
    /// not a local that only this test can see.
    fn town_in_index(owned: &mut IndexOwned, sim: &sim::WorldSim, town_seed: u64) -> Id<Site> {
        let land = Land::from_sim(sim);
        let origin = buildable_origin(&land);
        let site = {
            let index_ref: IndexRef = owned.as_index_ref();
            let mut rng = ChaChaRng::seed_from_u64(town_seed);
            let mut meta = SitesGenMeta::new(WORLD_SEED);
            Site::generate_city(&land, index_ref, &mut rng, origin, 0.5, None, &mut meta)
        };
        owned
            .try_index_mut()
            .expect("a freshly built index is unshared, so this must succeed")
            .sites
            .insert(site)
    }

    /// Read the plot count back through the SHARED path, i.e. the way any
    /// other reader of the world would see it. Reading it off a local `Site`
    /// would prove nothing about the index.
    fn plot_count(owned: &IndexOwned, site: Id<Site>) -> usize {
        owned.as_index_ref().sites.get(site).plots().len()
    }

    /// THE refusal that makes the design safe: while a chunk job holds a clone
    /// of the index, the colony must not grow, and must not have started to.
    ///
    /// The "before anything is spent" half is the point. A guard that refused
    /// *after* placing the plot would leave a half-grown town behind on every
    /// busy tick, and the symptom (a plot that exists but was never reported)
    /// would be invisible from the return value.
    #[test]
    fn a_shared_index_is_refused_before_anything_is_spent() {
        show_witnesses();
        let (sim, mut owned) = world();
        let site = town_in_index(&mut owned, &sim, 0xB0_5E_ED);
        let before = plot_count(&owned, site);

        // Stand in for an in-flight chunk job.
        let held = owned.clone();

        let result = grow_plot(&mut owned, &sim, site, LayoutKind::House, 7);
        let after = plot_count(&owned, site);
        println!(
            "shared index: strong_count={} plots {} -> {} result={:?}",
            owned.index_strong_count(),
            before,
            after,
            result.as_ref().err(),
        );

        assert_eq!(
            result.err(),
            Some(GrowRefusal::IndexShared { strong_count: 2 }),
            "a shared index must be refused, and the refusal must name the count"
        );
        assert_eq!(
            after, before,
            "a refused grow must not have placed a plot; plots went {before} -> {after}"
        );

        // Hold the clone until here, or the compiler is free to drop it early
        // and the whole premise of the test evaporates.
        drop(held);
        // ...and once it is gone the very same call succeeds, which is the
        // control that proves the refusal above was caused by the sharing and
        // not by something else being wrong with this site or seed.
        let now = grow_plot(&mut owned, &sim, site, LayoutKind::House, 7);
        assert!(
            now.is_ok(),
            "control failed: with the clone dropped the same call must succeed, got {:?}",
            now.err()
        );
    }

    /// The happy path: with nothing else holding the index, the colony gets a
    /// house, and the house is IN THE INDEX — read back through
    /// `as_index_ref()`, not off a local copy.
    #[test]
    fn an_unshared_index_grows_a_house_in_place() {
        show_witnesses();
        let (sim, mut owned) = world();
        let site = town_in_index(&mut owned, &sim, 0xB0_5E_ED);
        let before = plot_count(&owned, site);

        assert_eq!(
            owned.index_strong_count(),
            1,
            "precondition: this test only means anything on an unshared index"
        );

        let grown = grow_plot(&mut owned, &sim, site, LayoutKind::House, 7)
            .expect("an unshared index must be able to grow a house on a fresh town");
        let after = plot_count(&owned, site);

        println!(
            "grown in place: plots {} -> {} | plot={:?} blocks={} beds={} door={:?} aabr={:?} \
             alt={} seed={}",
            before,
            after,
            grown.placed.plot,
            grown.blocks.len(),
            grown.beds.len(),
            grown.placed.door_wpos,
            grown.placed.aabr_wpos,
            grown.placed.alt,
            grown.seed,
        );

        assert_eq!(
            after,
            before + 1,
            "the plot must be in the INDEX's site, not just in the return value"
        );
        assert!(
            grown.blocks.len() >= 100,
            "a house is not 100 blocks big? got {}",
            grown.blocks.len()
        );
        assert!(
            !grown.beds.is_empty(),
            "a house with no bed is not a dwelling; got {} blocks but 0 bed heads",
            grown.blocks.len()
        );
        assert!(
            grown.placed.door_wpos.is_some(),
            "a roadside house must report the door colonists walk through"
        );
        assert_eq!(grown.site, site, "the grown plot must name the site it grew on");
        assert_eq!(grown.seed, 7, "the seed must come back with the plot");
    }

    /// A `NoRoom` refusal must leave the site exactly as it found it — the
    /// plot count on the refusing call must not move.
    ///
    /// This test also settles what `NoRoom` actually MEANS, which is not what
    /// the name suggests and which a caller has to know. The loop runs a
    /// different seed each iteration and keeps going *past* the first refusal.
    /// If houses still succeed afterwards, then `NoRoom` is a **per-seed
    /// placement-search failure**, not "the town is full": `find_roadside_aabr`
    /// picks one random plaza or road node and gives up after 32 attempts, so
    /// an unlucky seed refuses on a town with plenty of space left.
    ///
    /// That distinction is the difference between a colony that keeps growing
    /// and one that stops forever the first time a die roll goes badly, so the
    /// counts below are printed and asserted rather than left implied.
    #[test]
    fn no_room_leaves_the_site_untouched() {
        show_witnesses();
        let (sim, mut owned) = world();
        let site = town_in_index(&mut owned, &sim, 0xB0_5E_ED);

        let mut grown = 0usize;
        let mut refusals = 0usize;
        let mut first_refusal_at: Option<u64> = None;
        let mut grown_after_first_refusal = 0usize;

        for i in 0..200u64 {
            let before = plot_count(&owned, site);
            match grow_plot(&mut owned, &sim, site, LayoutKind::House, 1000 + i) {
                Ok(_) => {
                    let after = plot_count(&owned, site);
                    assert_eq!(
                        after,
                        before + 1,
                        "iteration {i}: a successful grow must add exactly one plot"
                    );
                    grown += 1;
                    if first_refusal_at.is_some() {
                        grown_after_first_refusal += 1;
                    }
                },
                Err(GrowRefusal::NoRoom) => {
                    let after = plot_count(&owned, site);
                    // THE assert of this test, applied to EVERY refusal rather
                    // than only the first: a refusal must spend nothing.
                    assert_eq!(
                        after, before,
                        "iteration {i}: the NoRoom call must leave the site untouched; \
                         plots {before} -> {after}"
                    );
                    refusals += 1;
                    if first_refusal_at.is_none() {
                        first_refusal_at = Some(i);
                        println!(
                            "first NoRoom at iteration {i} after {grown} houses: \
                             plots {before} -> {after} (unchanged)"
                        );
                    }
                },
                Err(other) => panic!("iteration {i}: unexpected refusal {other:?}"),
            }
        }

        println!(
            "200 seeds on one town: grown={grown} refusals={refusals} \
             first_refusal_at={first_refusal_at:?} grown_after_first_refusal={grown_after_first_refusal} \
             final_plots={}",
            plot_count(&owned, site)
        );

        // A null needs a witness: with no refusal at all this test asserted
        // nothing, and must say so rather than read green.
        assert!(
            first_refusal_at.is_some(),
            "no seed in 200 was ever refused ({grown} houses grown), so the NoRoom path \
             was never exercised — this test proved nothing"
        );
        assert!(
            grown > 0,
            "every seed was refused, so the refusals say nothing about a town with room"
        );
        // The finding G1c depends on: a refusal is not a terminal verdict.
        assert!(
            grown_after_first_refusal > 0,
            "NoRoom was never followed by a success in {} further tries, so on this town it \
             really does mean 'full' — G1c must NOT retry-on-NoRoom the way the doc says",
            200 - first_refusal_at.unwrap_or(0) - 1
        );
    }
}
