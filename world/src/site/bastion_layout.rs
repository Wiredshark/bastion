//! Bastion: lay out ONE new plot on an already-generated `Site` and hand the
//! caller the blocks a colony builder would have to place.
//!
//! The point of this module is that a colony does NOT get to invent its own
//! architecture. It asks worldgen for a plot, using exactly the machinery the
//! town generator itself uses (`find_roadside_aabr` / `find_rural_aabr`,
//! `House::generate`, `create_plot`, `blit_aabr`), and then *renders* that plot
//! off-chunk into a plain list of `(wpos, Block)` so the server can place the
//! blocks over time instead of all at once.
//!
//! Two things make this different from normal worldgen:
//!
//! 1. **It runs after the site exists.** The tile grid already has plazas,
//!    roads and buildings in it, so the new plot is fitted into the gaps the
//!    real town left — it is not a free-standing structure dropped on a field.
//! 2. **It renders without a chunk.** `CanvasInfo::with_mock_canvas_info` gives
//!    a canvas that has an index and a `WorldSim` but no terrain chunk, so the
//!    primitive/fill tree can be sampled position-by-position with no terrain
//!    write. This is the same shape `world/benches/site.rs` uses.
//!
//! Known limit: only plots that draw themselves through a primitive/fill tree
//! (`Structure::render_inner`) produce blocks here. `House` and `Workshop` do.
//! `FarmField` does NOT — it is a per-column overlay on existing terrain — so
//! it lays out with a correct footprint and an empty block list. See
//! [`LayoutKind::FarmField`].
//!
//! Determinism: the whole layout is driven by one `ChaChaRng` seeded from the
//! `seed` argument, so the same site in the same state plus the same seed
//! always yields the same plot and the same block list. (The *site state* is
//! part of the input — laying out two houses in a row is not two identical
//! houses, because the first one's tiles are occupied by the time the second
//! one looks.)

use super::{
    Plot, PlotKind, Site, Structure, aabr_tiles, foreach_plot, plot, reseed,
    tile::{Tile, TileKind},
};
use crate::{CanvasInfo, IndexRef, Land, column::ColInfo, sim::WorldSim, util::attempt};
use common::{
    store::Id,
    terrain::{Block, SpriteKind},
};
use hashbrown::HashMap;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaChaRng;
use vek::*;

/// Which kind of plot the colony is asking worldgen to lay out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutKind {
    /// A dwelling. Roadside — it wants a door onto a road or plaza.
    House,
    /// A crop field. Rural — it is placed away from the plaza, not on a road.
    ///
    /// **A field lays out but renders NO blocks here, by construction.**
    /// `FarmField` is the one kind of the three with no `render_inner` at all
    /// (`world/src/site/plot/farm_field.rs` has only
    /// `Structure::terrain_surface_at_inner`, line 213) — it is painted as a
    /// per-column *overlay on existing ground*, not as a primitive/fill tree.
    /// That hook needs the block already in the terrain (`old`, whose
    /// `BlockKind` gates the soil branch) and a full `ColumnSample`, and an
    /// off-chunk render has neither. So `layout_plot_for_colony` returns a
    /// field with a correct `plot`/`aabr_wpos` and an EMPTY `blocks` — see the
    /// note on [`LaidOutPlot::blocks`] for what the colony must do instead.
    FarmField,
    /// A crafting building. Roadside, like a house.
    Workshop,
}

/// One plot that worldgen has laid out for the colony, reduced to what a
/// builder needs in order to actually construct it.
pub struct LaidOutPlot {
    /// The id of the plot that was inserted into `site.plots`. The plot IS in
    /// the site already — the tile grid has been blitted — so anything that
    /// reads the site (pathfinding, further plot placement) sees it.
    pub plot: Id<Plot>,
    /// The kind that was requested.
    pub kind: LayoutKind,
    /// World-space (block) bounds of the plot's tile footprint. `min` is
    /// inclusive, `max` exclusive, matching `Site::tile_wpos` on the tile aabr.
    pub aabr_wpos: Aabr<i32>,
    /// World position of the centre of the door tile, at the plot's altitude.
    /// `None` for kinds that have no door (a farm field).
    pub door_wpos: Option<Vec3<i32>>,
    /// One entry per bed: the position of the bed's HEAD sprite. A bed in
    /// Veloren is three sprites (head/middle/tail); the head is the single
    /// unambiguous "this is one bed" anchor, so a colony can count sleepers
    /// off this vector directly.
    pub beds: Vec<Vec3<i32>>,
    /// Every non-air block the plot places, sorted by (z, y, x) so a builder
    /// can walk it bottom-up. Positions are deduplicated: where several fills
    /// overlap, the later fill wins, exactly as it would when painted onto a
    /// canvas.
    ///
    /// This is the plot's FILL tree only. Plots that paint themselves through
    /// `Structure::terrain_surface_at_inner` instead — `FarmField`, and also
    /// `Road`, `Plaza` and `Barn` — contribute nothing here and come back
    /// empty. For those, the plot is still inserted into the site, so the
    /// engine's own chunk render (`Site::render`, `world/src/site/mod.rs:3300`)
    /// paints them the next time the chunk is generated; a colony that wants
    /// them in an ALREADY-loaded chunk has to force a regeneration rather than
    /// place blocks from this list.
    pub blocks: Vec<(Vec3<i32>, Block)>,
}

/// The sprite that anchors each bed family used by the house/hut plots.
/// `PainterSpriteExt::bed_*` in `world/src/site/util/sprites.rs` builds every
/// bed out of a Head/Middle/Tail triple; matching only the head gives exactly
/// one position per bed.
const BED_HEAD_SPRITES: [SpriteKind; 5] = [
    SpriteKind::BedWoodWoodlandHead,
    SpriteKind::BedCliffHead,
    SpriteKind::BedCoastalHead,
    SpriteKind::BedDesertHead,
    SpriteKind::BedSavannahHead,
];

fn is_bed_head(block: &Block) -> bool {
    block
        .get_sprite()
        .is_some_and(|s| BED_HEAD_SPRITES.contains(&s))
}

/// Lay out one new plot of `kind` on `site`, using worldgen's own placement
/// and generation code, and return the blocks that plot is made of.
///
/// Returns `None` if worldgen could not find anywhere to put the plot (the
/// same failure the town generator handles by making another plaza instead).
/// On `None` the site is left untouched.
///
/// `land`/`index`/`sim` must all describe the same world: `land` is used by the
/// plot generators for terrain altitude, and `index` + `sim` build the mock
/// canvas the plot is rendered against.
pub fn layout_plot_for_colony(
    site: &mut Site,
    kind: LayoutKind,
    land: &Land,
    index: IndexRef,
    sim: &WorldSim,
    seed: u64,
) -> Option<LaidOutPlot> {
    let mut rng = ChaChaRng::seed_from_u64(seed);

    // --- 1. Placement + generation, copied from the town generator ---------
    let (plot_id, tile_aabr, door_tile, alt) = match kind {
        LayoutKind::House => {
            // `Site::generate_city`'s house arm.
            let size = (1.5f32 + rng.random::<f32>().powf(5.0)).round() as u32;
            let (aabr, door_tile, door_dir, alt) = attempt(32, || {
                site.find_roadside_aabr(&mut rng, 4..(size + 1).pow(2), Extent2::broadcast(size))
            })?;
            let house = plot::House::generate(
                land,
                &mut reseed(&mut rng),
                site,
                door_tile,
                door_dir,
                aabr,
                // No calendar: a colony house must not sprout Christmas
                // decorations depending on the wall clock. See the
                // determinism-by-construction law.
                None,
                alt,
            );
            let house_alt = house.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::House(house),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
            });
            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(house_alt),
            });
            (plot, aabr, Some(door_tile), house_alt)
        },
        LayoutKind::Workshop => {
            // `Site::generate_city`'s workshop arm.
            let size = (3.0f32 + rng.random::<f32>().powf(5.0) * 1.5).round() as u32;
            let (aabr, door_tile, door_dir, alt) = attempt(32, || {
                site.find_roadside_aabr(&mut rng, 4..(size + 1).pow(2), Extent2::broadcast(size))
            })?;
            let workshop = plot::Workshop::generate(
                land,
                &mut reseed(&mut rng),
                site,
                door_tile,
                door_dir,
                aabr,
                alt,
            );
            let workshop_alt = workshop.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::Workshop(workshop),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
            });
            site.blit_aabr(aabr, Tile {
                kind: TileKind::Building,
                plot: Some(plot),
                hard_alt: Some(workshop_alt),
            });
            (plot, aabr, Some(door_tile), workshop_alt)
        },
        LayoutKind::FarmField => {
            // `Site::generate_farm`, with `is_desert` fixed to false: the
            // caller does not (yet) tell us the biome, and a colony that wants
            // a desert field should be asking for one explicitly rather than
            // having it inferred here.
            let size = (3.0f32 + rng.random::<f32>().powf(5.0) * 6.0).round() as u32;
            let (aabr, door_tile, door_dir, _alt) = attempt(32, || {
                site.find_rural_aabr(&mut rng, 6..(size + 1).pow(2), Extent2::broadcast(size))
            })?;
            let field = plot::FarmField::generate(
                land,
                &mut reseed(&mut rng),
                site,
                door_tile,
                door_dir,
                aabr,
                false,
            );
            let field_alt = field.alt;
            let plot = site.create_plot(Plot {
                kind: PlotKind::FarmField(field),
                root_tile: aabr.center(),
                tiles: aabr_tiles(aabr).collect(),
            });
            site.blit_aabr(aabr, Tile {
                kind: TileKind::Field,
                plot: Some(plot),
                hard_alt: Some(field_alt),
            });
            // A field has a "door tile" for orientation but no door.
            (plot, aabr, None, field_alt)
        },
    };

    let aabr_wpos = Aabr {
        min: site.tile_wpos(tile_aabr.min),
        max: site.tile_wpos(tile_aabr.max),
    };
    let door_wpos = door_tile.map(|t| site.tile_center_wpos(t).with_z(alt));

    // --- 2. Render the plot off-chunk into a block list -------------------
    let site_ref: &Site = site;
    let (blocks, beds) = CanvasInfo::with_mock_canvas_info(index, sim, |canvas| {
        let plot_ref = site_ref.plot(plot_id);
        let (prim_tree, fills, _entities) =
            foreach_plot!(&plot_ref.kind(), p => p.render_collect(site_ref, canvas));

        // Positions accumulate so that a later fill overwrites an earlier one
        // at the same position, and so that each fill sees the block already
        // written there as its `old_block` -- that is what makes furniture
        // sprites sit on the floor block instead of in bare air.
        let mut acc: HashMap<Vec3<i32>, Block> = HashMap::new();
        // `col()` on a mock canvas is always None, but the fills'
        // terrain-relative predicates read the column's altitude. Generate the
        // column from the sim instead of falling back to a zeroed default, and
        // cache it -- the same few hundred columns are revisited by every fill.
        let mut col_cache: HashMap<Vec2<i32>, ColInfo> = HashMap::new();

        for (prim, fill) in fills {
            // Disjoint bounds rather than the union: this is what the real
            // render loop in `Site::render` iterates, and it avoids sweeping
            // the empty volume between two far-apart sub-primitives.
            for aabb in super::Fill::get_bounds_disjoint(&prim_tree, prim) {
                for x in aabb.min.x..aabb.max.x {
                    for y in aabb.min.y..aabb.max.y {
                        let wpos2d = Vec2::new(x, y);
                        let col = col_cache
                            .entry(wpos2d)
                            .or_insert_with(|| {
                                canvas
                                    .col_or_gen(wpos2d)
                                    .map(|col| col.get_info())
                                    .unwrap_or_default()
                            })
                            .clone();
                        for z in aabb.min.z..aabb.max.z {
                            let pos = Vec3::new(x, y, z);
                            let old_block =
                                acc.get(&pos).copied().unwrap_or_else(Block::empty);
                            let mut sprite_cfg = None;
                            let (new_block, _sb, _entity_path) = fill.sample_at(
                                &prim_tree,
                                prim,
                                pos,
                                canvas,
                                old_block,
                                &mut sprite_cfg,
                                &col,
                            );
                            if let Some(block) = new_block {
                                acc.insert(pos, block);
                            }
                        }
                    }
                }
            }
        }

        let empty = Block::empty();
        let mut blocks: Vec<(Vec3<i32>, Block)> =
            acc.into_iter().filter(|(_, b)| *b != empty).collect();
        // HashMap order is not stable; sort so the output is deterministic and
        // a builder can lay courses bottom-up.
        blocks.sort_unstable_by_key(|(p, _)| (p.z, p.y, p.x));

        let beds = blocks
            .iter()
            .filter(|(_, b)| is_bed_head(b))
            .map(|(p, _)| *p)
            .collect::<Vec<_>>();

        (blocks, beds)
    });

    Some(LaidOutPlot {
        plot: plot_id,
        kind,
        aabr_wpos,
        door_wpos,
        beds,
        blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        index::{Index, IndexOwned},
        sim::{self, DEFAULT_WORLD_MAP, FileOpts, WorldOpts},
        site::SitesGenMeta,
    };

    const WORLD_SEED: u32 = 230;

    /// Build the world once: a `WorldSim` (loaded from the shipped map asset,
    /// which skips erosion) plus an index. No civs, no economy -- we only need
    /// terrain to place a site on and to render against.
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

    /// Find a buildable spot: dry land, comfortably above sea level, near the
    /// centre of the map. Deterministic (a fixed spiral, first hit wins).
    fn buildable_origin(land: &Land) -> Vec2<i32> {
        use common::{terrain::TerrainChunkSize, vol::RectVolSize};
        let size = land.size();
        let centre_chunk = (size / 2).as_::<i32>();
        for offset in common::spiral::Spiral2d::new().take(4096) {
            let chunk = centre_chunk + offset * 4;
            let wpos = chunk * TerrainChunkSize::RECT_SIZE.as_::<i32>();
            let alt = land.get_alt_approx(wpos);
            let flat_enough = land.get_gradient_approx(wpos) < 0.35;
            if alt > crate::config::CONFIG.sea_level + 12.0 && flat_enough {
                return wpos;
            }
        }
        panic!("no buildable origin found near the centre of the map");
    }

    /// Generate a small town at `origin`. Deterministic in `town_seed`.
    fn town(land: &Land, index: IndexRef, origin: Vec2<i32>, town_seed: u64) -> Site {
        let mut rng = ChaChaRng::seed_from_u64(town_seed);
        let mut meta = SitesGenMeta::new(WORLD_SEED);
        Site::generate_city(land, index, &mut rng, origin, 0.5, None, &mut meta)
    }

    /// The whole deliverable in one test: worldgen can be asked for a colony
    /// house on an existing town, and what comes back is buildable -- a real
    /// block list, a door to walk through, and a bed to sleep in -- and it
    /// comes back the same way every time for the same seed.
    ///
    /// The determinism half matters more than it looks: the colony persists
    /// plot layouts, so a layout that drifted between runs would resurrect a
    /// half-built house as a *different* house.
    #[test]
    fn a_colony_house_is_laid_out_with_blocks_a_door_and_a_bed() {
        let (sim, index) = world();
        let index_ref = index.as_index_ref();
        let land = Land::from_sim(&sim);
        let origin = buildable_origin(&land);

        let mut site = town(&land, index_ref, origin, 0xB0_5E_ED);
        let plots_before = site.plots().len();

        let laid = layout_plot_for_colony(
            &mut site,
            LayoutKind::House,
            &land,
            index_ref,
            &sim,
            7,
        )
        .expect("worldgen must be able to fit one more house into a fresh town");

        // Print the producers of every number this test asserts on, so a
        // future green bar can be read rather than trusted.
        println!(
            "origin={:?} plots {} -> {} | blocks={} beds={} door={:?} aabr={:?}",
            origin,
            plots_before,
            site.plots().len(),
            laid.blocks.len(),
            laid.beds.len(),
            laid.door_wpos,
            laid.aabr_wpos,
        );

        assert_eq!(
            site.plots().len(),
            plots_before + 1,
            "the plot must actually be inserted into the site, not just rendered"
        );
        assert!(
            laid.blocks.len() >= 100,
            "a house is not 100 blocks big? got {} blocks, aabr {:?}",
            laid.blocks.len(),
            laid.aabr_wpos
        );
        assert!(
            laid.door_wpos.is_some(),
            "a roadside house must report the door the colonists walk through"
        );
        assert!(
            !laid.beds.is_empty(),
            "a house with no bed is not a dwelling; got {} blocks but 0 bed heads",
            laid.blocks.len()
        );

        // Determinism: an identically-built site, laid out with the same seed,
        // must produce byte-identical geometry.
        let mut site2 = town(&land, index_ref, origin, 0xB0_5E_ED);
        let laid2 = layout_plot_for_colony(
            &mut site2,
            LayoutKind::House,
            &land,
            index_ref,
            &sim,
            7,
        )
        .expect("the second, identical layout must also succeed");

        assert_eq!(
            laid.blocks.len(),
            laid2.blocks.len(),
            "seed 7 twice gave different block counts"
        );
        assert!(
            laid.blocks == laid2.blocks,
            "seed 7 twice gave different block lists"
        );
        assert_eq!(laid.door_wpos, laid2.door_wpos, "door moved between runs");
        assert_eq!(laid.beds, laid2.beds, "beds moved between runs");
        assert_eq!(laid.aabr_wpos, laid2.aabr_wpos, "footprint moved between runs");
    }

    /// The other two kinds are not part of the house contract above, so they
    /// get their own bar -- and the two bars are DIFFERENT, which is the point
    /// of this test.
    ///
    /// A workshop is a fill-tree plot like a house: it must come back with
    /// blocks and a door. A farm field is not: it has no `render_inner` at all
    /// (`world/src/site/plot/farm_field.rs:213` is a
    /// `terrain_surface_at_inner`), so it lays out correctly and returns ZERO
    /// blocks. That zero is pinned here on purpose. A colony that reads it as
    /// a failure will refuse to ever plant a field; a future change that made
    /// fields render through fills would break this assert and force whoever
    /// makes it to update the colony side too.
    #[test]
    fn a_workshop_and_a_farm_field_also_lay_out() {
        let (sim, index) = world();
        let index_ref = index.as_index_ref();
        let land = Land::from_sim(&sim);
        let origin = buildable_origin(&land);
        let mut site = town(&land, index_ref, origin, 0xB0_5E_ED);

        let shop =
            layout_plot_for_colony(&mut site, LayoutKind::Workshop, &land, index_ref, &sim, 11)
                .expect("a workshop must fit beside a road in a fresh town");
        println!(
            "workshop: blocks={} door={:?} aabr={:?}",
            shop.blocks.len(),
            shop.door_wpos,
            shop.aabr_wpos
        );
        assert!(shop.blocks.len() >= 100, "workshop blocks={}", shop.blocks.len());
        assert!(shop.door_wpos.is_some(), "a workshop has a door");

        let plots_before_field = site.plots().len();
        let field =
            layout_plot_for_colony(&mut site, LayoutKind::FarmField, &land, index_ref, &sim, 13)
                .expect("a farm field must fit in the rural ring around the plaza");
        println!(
            "farm field: blocks={} door={:?} aabr={:?}",
            field.blocks.len(),
            field.door_wpos,
            field.aabr_wpos
        );
        // The field IS laid out -- the plot exists and has a real footprint...
        assert_eq!(
            site.plots().len(),
            plots_before_field + 1,
            "the field plot must be inserted into the site"
        );
        assert!(
            field.aabr_wpos.size().w > 0 && field.aabr_wpos.size().h > 0,
            "the field must have a real footprint, got {:?}",
            field.aabr_wpos
        );
        // ...but it renders through the terrain-surface hook, not fills, so
        // there is nothing for a block-placing builder to place.
        assert!(
            field.blocks.is_empty(),
            "a farm field has no fill tree, so this must be 0; got {} blocks -- if \
             FarmField gained a render_inner, the colony's field builder needs updating",
            field.blocks.len()
        );
        assert!(
            field.door_wpos.is_none(),
            "a farm field must report no door, not a fake one"
        );
        assert!(field.beds.is_empty(), "nobody sleeps in a field");
    }
}
