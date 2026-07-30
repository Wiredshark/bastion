//! bastion (CHOP redesign, FR10): WHOLE-TREE detection — the ONE
//! implementation both the designation handler (the player paint path) and
//! the harness hook call, so the tested path IS the shipping path (registry
//! B17: identity by construction, no parallel copy).
//!
//! PRIMARY per FR10: seed only from the World's own tree oracle
//! (`get_area_trees` candidates, confirmed by the engine's `tree_valid_at`
//! env-filter — never a hand-rolled Wood probe, so detection can never START
//! from a building; registry D15), then bound-flood the connected
//! Wood+Leaves component for the tree's full block-set.

#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World, util::Sampler};

use crate::bastion_jobs::{
    TREE_FELL_CELL_CAP, TREE_FELL_HEIGHT_CAP, TREE_FELL_RADIUS, tree_fell_set,
};
use common::{
    bastion::Region,
    terrain::{BlockKind, TerrainGrid},
    vol::ReadVol,
};
use vek::*;

/// Every whole tree ROOTED in the XY footprint: `(tight AABB, base, fell
/// cells)` per tree. The AABB is what echoes to the client as the per-tree
/// outline box; the BASE (the ground-rooted trunk cell `seed_z` found) is
/// where the single base-cut job sits (CHOP-FELLING, row 51.6); the cells
/// are the stored fell-set. Under the non-worldgen stub World (no oracle)
/// this degrades to no trees.
pub fn detect_trees(
    world: &World,
    index: &IndexOwned,
    terrain: &TerrainGrid,
    min_xy: Vec2<i32>,
    max_xy: Vec2<i32>,
) -> Vec<(Region, Vec3<i32>, Vec<Vec3<i32>>)> {
    let mut trees: Vec<(Region, Vec3<i32>, Vec<Vec3<i32>>)> = Vec::new();
    #[cfg(feature = "worldgen")]
    {
        let sim = world.sim();
        // WorldSim's calendar is private; None only skips seasonal column
        // tinting (snow cover etc.) — irrelevant to tree_valid_at's
        // alt/water/spawn-rate/path gates.
        let calendar = None;
        let index_ref = index.as_index_ref();
        let sampler = world.sample_columns();
        let is_tree = |p: Vec3<i32>| {
            terrain
                .get(p)
                .map(|b| matches!(b.kind(), BlockKind::Wood | BlockKind::Leaves))
                .unwrap_or(false)
        };
        for attr in sim.get_area_trees(min_xy, max_xy) {
            // The oracle is a rough superset ("needs to be reworked",
            // all.rs:188) — confirm with the engine's own env-filter, the
            // same resolve the LOD tree layer uses (world/src/lib.rs).
            let Some(col) = sampler.get((attr.pos, index_ref, calendar)) else {
                continue;
            };
            if !world::layer::tree::tree_valid_at(attr.pos, &col, None, attr.seed) {
                continue;
            }
            // Seed at the first tree block near the column's ground — a
            // felled/ungenerated tree yields nothing (the chunk must be
            // loaded for its blocks to exist).
            let base_z = col.alt as i32;
            let Some(seed_z) =
                (base_z - 2..=base_z + 8).find(|&z| is_tree(Vec3::new(attr.pos.x, attr.pos.y, z)))
            else {
                continue;
            };
            let cells = tree_fell_set(
                &is_tree,
                Vec3::new(attr.pos.x, attr.pos.y, seed_z),
                TREE_FELL_CELL_CAP,
                TREE_FELL_HEIGHT_CAP,
                TREE_FELL_RADIUS,
            );
            if cells.is_empty() {
                continue;
            }
            let mut mn = cells[0];
            let mut mx = cells[0];
            for c in &cells {
                mn = Vec3::partial_min(mn, *c);
                mx = Vec3::partial_max(mx, *c);
            }
            trees.push((
                Region { min: mn, max: mx },
                Vec3::new(attr.pos.x, attr.pos.y, seed_z),
                cells,
            ));
        }
    }
    #[cfg(not(feature = "worldgen"))]
    let _ = (world, index, terrain, min_xy, max_xy);
    trees
}

/// A ground-truth tree witness: the Wood block's position and the Leaves
/// block found above it in the same column, checkable rather than merely
/// counted (Opus/Fable ruling, 2026-07-30 chop-oracle row).
#[derive(Debug, Clone, Copy)]
pub struct TreeGroundTruthWitness {
    pub wood_pos: Vec3<i32>,
    pub leaves_pos: Vec3<i32>,
}

/// bastion (chop-oracle ground-truth audit, 2026-07-30): an INDEPENDENT
/// answer to "is a REAL TREE physically materialized anywhere in this XY
/// footprint" -- deliberately NOT built from [`detect_trees`]'s own
/// machinery (`get_area_trees`'s candidate list, `tree_valid_at`'s
/// env-filter). Those are exactly the subject under test; using them here
/// would make a detection bug and "no tree exists" indistinguishable
/// again -- the same falsifier-canon mistake `b5_failed_clauses` and the
/// rescue-clause split already fixed twice today. The ONLY thing borrowed
/// from the World is the altitude sampler (`col.alt`), a basic geography
/// fact used solely to bound the Z search window per column -- not a tree
/// judgment.
///
/// RULING (Fable, 2026-07-30): a bare Wood-or-Leaves block scan is NOT a
/// tree predicate -- worldgen wooden STRUCTURES (houses, bridges, fences,
/// dungeon timbers) are Wood blocks that are correctly not trees; scanning
/// for either kind alone would fabricate "real_detection_miss" verdicts at
/// any site near one. The tightened predicate requires a Leaves block
/// ABOVE a Wood block in the SAME (x, y) column, within a plausible canopy
/// span (`TREE_FELL_HEIGHT_CAP`) -- a flat Wood floor/fence/deck has
/// nothing above it and will not satisfy this; a trunk-plus-canopy will.
/// Returns the witness block pair so a miss is checkable, not just
/// counted. Short-circuits on the first hit.
pub fn detect_trees_ground_truth(
    world: &World,
    index: &IndexOwned,
    terrain: &TerrainGrid,
    min_xy: Vec2<i32>,
    max_xy: Vec2<i32>,
) -> Option<TreeGroundTruthWitness> {
    #[cfg(feature = "worldgen")]
    {
        let calendar = None;
        let index_ref = index.as_index_ref();
        let sampler = world.sample_columns();
        for x in min_xy.x..=max_xy.x {
            for y in min_xy.y..=max_xy.y {
                let Some(col) = sampler.get((Vec2::new(x, y), index_ref, calendar)) else {
                    continue;
                };
                let base_z = col.alt as i32;
                // Margin either side of detect_trees's own seed window
                // (alt-2..=alt+8) plus the fell-set height cap, so a real
                // tree can't be missed by an under-sized window.
                let lo = base_z - 10;
                let hi = base_z + TREE_FELL_HEIGHT_CAP + 10;
                let mut wood_z: Option<i32> = None;
                for z in lo..=hi {
                    let Ok(block) = terrain.get(Vec3::new(x, y, z)) else {
                        continue;
                    };
                    match block.kind() {
                        BlockKind::Wood if wood_z.is_none() => wood_z = Some(z),
                        BlockKind::Leaves => {
                            if let Some(wz) = wood_z {
                                if z > wz && z - wz <= TREE_FELL_HEIGHT_CAP {
                                    return Some(TreeGroundTruthWitness {
                                        wood_pos: Vec3::new(x, y, wz),
                                        leaves_pos: Vec3::new(x, y, z),
                                    });
                                }
                            }
                        },
                        _ => {},
                    }
                }
            }
        }
        None
    }
    #[cfg(not(feature = "worldgen"))]
    {
        let _ = (world, index, terrain, min_xy, max_xy);
        None
    }
}
