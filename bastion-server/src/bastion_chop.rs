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
        // CANDIDATE COLUMNS: `(trunk column, ground z)`. Only the SOURCE of
        // candidates varies — everything after this list (the seed search
        // and the bounded flood-fill) is shared and reads real blocks.
        let mut candidates: Vec<(Vec2<i32>, i32)> = Vec::new();

        for attr in sim.get_area_trees(min_xy, max_xy) {
            // ★ FOUND BY AN ADVERSARIAL PLAY SESSION (2026-08-21): the oracle
            // is a rough SUPERSET by its own admission, and returns trees from
            // any grid cell that merely OVERLAPS the query — but nothing here
            // ever re-checked that the tree it handed back was inside the box
            // the player actually painted. `tree_valid_at` below is an
            // ENVIRONMENT filter (alt/water/spawn-rate); it has no idea what
            // region was requested, so it cannot catch this and never did.
            //
            // Measured: a brush over bare walkable ground with no tree in it
            // produced two chop designations 20–46 blocks away with ZERO
            // x-overlap with the brush. A player paints a chop order on empty
            // grass and lumberjacks walk off to fell a tree half a screen away
            // that was never selected.
            //
            // A superset must be intersected with the request. Cheap, and it
            // runs before the sampler so it also saves the column resolve.
            if attr.pos.x < min_xy.x
                || attr.pos.x > max_xy.x
                || attr.pos.y < min_xy.y
                || attr.pos.y > max_xy.y
            {
                continue;
            }
            // Confirm with the engine's own env-filter, the same resolve the
            // LOD tree layer uses (world/src/lib.rs).
            let Some(col) = sampler.get((attr.pos, index_ref, calendar)) else {
                continue;
            };
            if !world::layer::tree::tree_valid_at(attr.pos, &col, None, attr.seed) {
                continue;
            }
            candidates.push((attr.pos, col.alt as i32));
        }

        // F8-C1: THE ARENA IS ITS OWN ORACLE.
        //
        // `get_area_trees` is generative — `structure_gen` plus a climate
        // lottery — so it can never propose a HAND-PLACED trunk, and
        // `tree_valid_at` would filter a synthetic flat column even if it
        // did. That is why the arena's trees refused with
        // `no_trees_rooted` BEFORE any block was examined, and it is why
        // this fix belongs at the candidate source rather than downstream:
        // loosening the shared path to admit them would break the
        // real-worldgen guarantee that attributed the defect to the arena
        // in the first place.
        //
        // Gated on `resourced()`, so a normal world is bit-for-bit
        // unaffected. The trunk's ground is `FLAT_ARENA_Z` by construction
        // (`resourced_feature_cells` paints from the slab's first air cell
        // upward), which is the same fact the seed search below would have
        // learned from `col.alt` on real terrain.
        if crate::bastion_flat_arena::resourced() {
            let centre = crate::bastion_flat_arena::world_center_wpos(world).map(|e| e as i32);
            for (offset, _height) in crate::bastion_flat_arena::RESOURCED_TREES {
                let pos = centre + *offset;
                let inside = pos.x >= min_xy.x
                    && pos.x <= max_xy.x
                    && pos.y >= min_xy.y
                    && pos.y <= max_xy.y;
                // ROOTED-IN-THE-PAINT is still the rule: a trunk outside
                // the designated footprint is not felled just because the
                // arena knows where it is.
                if inside && !candidates.iter().any(|(c, _)| *c == pos) {
                    candidates.push((pos, crate::bastion_flat_arena::FLAT_ARENA_Z));
                }
            }
        }

        for (pos, base_z) in candidates {
            // Seed at the first tree block near the column's ground — a
            // felled/ungenerated tree yields nothing (the chunk must be
            // loaded for its blocks to exist).
            let Some(seed_z) =
                (base_z - 2..=base_z + 8).find(|&z| is_tree(Vec3::new(pos.x, pos.y, z)))
            else {
                continue;
            };
            let cells = tree_fell_set(
                &is_tree,
                Vec3::new(pos.x, pos.y, seed_z),
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
                Vec3::new(pos.x, pos.y, seed_z),
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

/// Ground-truth scan outcome: three states, not two (Opus adversarial
/// review, 2026-07-30). `Found` and `NotFound` both mean the scan actually
/// ran to completion; `ScanIncomplete` means some columns could not be
/// examined at all (no altitude sample, or terrain never loaded there) --
/// "couldn't look" must never silently read the same as "looked and found
/// nothing," or an unloaded chunk inflates `precondition_unmet` while
/// looking like a clean result.
#[derive(Debug, Clone, Copy)]
pub enum TreeGroundTruthOutcome {
    Found(TreeGroundTruthWitness),
    NotFound,
    ScanIncomplete {
        unreachable_columns: u32,
        total_columns: u32,
    },
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
/// judgment. NOTE this is a shared-dependency asymmetry, not a shared
/// oracle: the SUBJECT (`detect_trees`) never consults `col.alt` at all
/// (it goes through `get_area_trees`/`tree_valid_at` instead), so a bug in
/// altitude sampling could blind only this auditor while leaving the
/// subject working -- exactly why `ScanIncomplete` below exists, so that
/// failure mode reports itself instead of masquerading as a clean null.
///
/// RULING (Fable, 2026-07-30): a bare Wood-or-Leaves block scan is NOT a
/// tree predicate -- worldgen wooden STRUCTURES (houses, bridges, fences,
/// dungeon timbers) are Wood blocks that are correctly not trees; scanning
/// for either kind alone would fabricate "real_detection_miss" verdicts at
/// any site near one. The tightened predicate requires a Leaves block
/// ABOVE a Wood block in the SAME (x, y) column, within a plausible canopy
/// span (`TREE_FELL_HEIGHT_CAP`) -- a flat Wood floor/fence/deck has
/// nothing above it and will not satisfy this; a trunk-plus-canopy will.
/// KNOWN LIMITATION (disclosed, not fixed): a canopy taller than
/// `TREE_FELL_HEIGHT_CAP`(40) above its own trunk is invisible to this
/// scan -- accepted, since `detect_trees`'s own fell-set walk shares the
/// same cap, so this predicate's blind spot matches the subject's.
/// `ArtLeaves` is deliberately unmatched -- cave-only, not a live miss on
/// surface search sites.
///
/// FIX (Opus adversarial review, 2026-07-30): the wood tracked here is the
/// NEAREST Wood block AT OR BELOW the current scan position, re-latched on
/// every Wood block seen -- not the LOWEST Wood in the whole window
/// (that earlier version could report a witness pairing wood and leaves
/// from two unrelated objects, and could miss a real tree whenever any
/// wood sat lower in the same column, e.g. a buried log).
///
/// Returns the witness block pair on a hit, so a miss is checkable, not
/// just counted. Short-circuits on the first hit.
pub fn detect_trees_ground_truth(
    world: &World,
    index: &IndexOwned,
    terrain: &TerrainGrid,
    min_xy: Vec2<i32>,
    max_xy: Vec2<i32>,
) -> TreeGroundTruthOutcome {
    #[cfg(feature = "worldgen")]
    {
        let calendar = None;
        let index_ref = index.as_index_ref();
        let sampler = world.sample_columns();
        let mut unreachable_columns: u32 = 0;
        let mut total_columns: u32 = 0;
        for x in min_xy.x..=max_xy.x {
            for y in min_xy.y..=max_xy.y {
                total_columns += 1;
                let Some(col) = sampler.get((Vec2::new(x, y), index_ref, calendar)) else {
                    unreachable_columns += 1;
                    continue;
                };
                let base_z = col.alt as i32;
                // Margin either side of detect_trees's own seed window
                // (alt-2..=alt+8) plus the fell-set height cap, so a real
                // tree can't be missed by an under-sized window.
                let lo = base_z - 10;
                let hi = base_z + TREE_FELL_HEIGHT_CAP + 10;
                let mut wood_z: Option<i32> = None;
                let mut column_reachable = false;
                for z in lo..=hi {
                    let Ok(block) = terrain.get(Vec3::new(x, y, z)) else {
                        continue;
                    };
                    column_reachable = true;
                    match block.kind() {
                        BlockKind::Wood => wood_z = Some(z),
                        BlockKind::Leaves => {
                            if let Some(wz) = wood_z {
                                if z > wz && z - wz <= TREE_FELL_HEIGHT_CAP {
                                    return TreeGroundTruthOutcome::Found(TreeGroundTruthWitness {
                                        wood_pos: Vec3::new(x, y, wz),
                                        leaves_pos: Vec3::new(x, y, z),
                                    });
                                }
                            }
                        },
                        _ => {},
                    }
                }
                if !column_reachable {
                    unreachable_columns += 1;
                }
            }
        }
        if unreachable_columns > 0 {
            TreeGroundTruthOutcome::ScanIncomplete {
                unreachable_columns,
                total_columns,
            }
        } else {
            TreeGroundTruthOutcome::NotFound
        }
    }
    #[cfg(not(feature = "worldgen"))]
    {
        let _ = (world, index, terrain, min_xy, max_xy);
        TreeGroundTruthOutcome::NotFound
    }
}
