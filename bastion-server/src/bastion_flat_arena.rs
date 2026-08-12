//! bastion (FLAT-TEST-ARENA, row 50.5): Ben's runtime flat playtest arena.
//!
//! With `BASTION_FLAT_ARENA=1` in the environment, chunk GENERATION is
//! overridden inside a bounded radius around the world-center spawn: those
//! chunks come out as a perfectly flat grass slab with empty supplements
//! (no worldgen entities), while everything beyond the radius generates
//! normally. Overriding at GENERATION — rather than post-hoc `set_block`
//! flattening — makes the arena stable across chunk unload/reload with no
//! persistence dependency: a flat chunk regenerates flat, deterministically,
//! every time. Without the env var the override is a single cold branch per
//! chunk generation — normal-world play is untouched.
//!
//! Runtime-selectable by design (the packet's architecture question): the
//! real worldgen stays fully active — this wraps the two `generate_chunk`
//! CALL SITES (the live slowjob + the harness force-load), so no
//! World-selection abstraction is needed and the compile-time
//! `test_world` duality is left exactly as it is.

#[cfg(not(feature = "worldgen"))]
use crate::test_world::World;
use common::{
    generation::ChunkSupplement,
    terrain::{Block, BlockKind, SpriteKind, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{RectVolSize, WriteVol},
};
use std::sync::OnceLock;
use vek::*;
#[cfg(feature = "worldgen")] use world::World;

/// Arena half-width in CHUNKS (32-block chunks → 16 ⇒ a 1056×1056-block
/// flat square). Sized against observed colony dynamics: generator scan
/// radii (±12), the soft-magnet leash, and the deep-wander excursions the
/// AUTON-2 forensics measured (~150 blocks) all fit with a wide margin, so
/// a test colony's whole behavioral footprint stays on the flat.
pub const FLAT_ARENA_RADIUS_CHUNKS: i32 = 16;

/// The arena's uniform surface height — comfortably above sea level so the
/// slab never floods, in the same z-band the site-area scenarios exercise.
/// The boundary meets real terrain as a cliff/step seam at the rim; the
/// radius keeps that seam far outside a test colony's footprint.
pub const FLAT_ARENA_Z: i32 = 400;

/// The env toggle, read once (the `BASTION_TIGHTDIG`-style dev-flag
/// pattern): `BASTION_FLAT_ARENA` set to anything but `0` enables the
/// arena for this server's lifetime.
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("BASTION_FLAT_ARENA").is_ok_and(|v| v != "0"))
}

/// bastion (FOUNDING PRESET v1, packet §2 — Ben's call): the RESOURCED
/// variant of the arena. `BASTION_FLAT_ARENA_RESOURCED` set to anything
/// but `0` adds the deterministic feature set below to the flat slab.
///
/// A VARIANT, not a second arena: this reads as false unless
/// [`enabled`] is also true, so the resourced gate can never half-apply
/// to a normal world.
pub fn resourced() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        enabled() && std::env::var("BASTION_FLAT_ARENA_RESOURCED").is_ok_and(|v| v != "0")
    })
}

/// Trunk columns of the CHOP cluster, as xy offsets from arena centre,
/// with each trunk's height in blocks. Fixed offsets, hand-placed once:
/// the arena's whole value is that the layout is IDENTICAL every run, so
/// a red means the feature broke and never the terrain (the matched
/// control built into the world).
pub const RESOURCED_TREES: &[(Vec2<i32>, i32)] = &[
    (Vec2::new(18, 2), 5),
    (Vec2::new(22, -3), 6),
    (Vec2::new(25, 4), 4),
];

/// The MINE outcrop's xy offset from arena centre, its half-width, and its
/// height. A solid rock mass sitting on the slab — real `Mine` work
/// through the generic path, which is what F8's inclusion evidence needs.
pub const RESOURCED_OUTCROP_OFFSET: Vec2<i32> = Vec2::new(-20, 0);
pub const RESOURCED_OUTCROP_HALF_WIDTH: i32 = 2;
pub const RESOURCED_OUTCROP_HEIGHT: i32 = 3;

/// How far from centre the founding ground stays CLEAR. The preset's own
/// footprint spans x −7..+2, y −4..+1 from the founding point; this radius
/// keeps every feature well outside it, so "found here" and "there is work
/// nearby" never contend for the same columns.
pub const RESOURCED_CLEAR_RADIUS: i32 = 12;

/// THE FEATURE SET, as absolute world cells — pure, so the layout can be
/// asserted without generating a chunk.
///
/// Every cell sits at or above [`FLAT_ARENA_Z`] (the slab's first air
/// cell), i.e. ON the ground rather than in it: a tree's trunk starts
/// where a colonist's feet would.
pub fn resourced_feature_cells(center_wpos: Vec2<u32>) -> Vec<(Vec3<i32>, Block)> {
    let centre = center_wpos.map(|e| e as i32);
    let mut cells = Vec::new();

    // CHOP: trunks of Wood with a small Leaves cap. Only Wood yields
    // (FR10 — leaves clear free), so the trunk height IS the yield.
    for (offset, height) in RESOURCED_TREES {
        let column = centre + *offset;
        for step in 0..*height {
            cells.push((
                Vec3::new(column.x, column.y, FLAT_ARENA_Z + step),
                Block::new(BlockKind::Wood, Rgb::new(88, 56, 34)),
            ));
        }
        // A one-block canopy ring at the crown: enough for the Leaves
        // arm to be exercised, not enough to turn the cluster into a
        // forest that changes pathing.
        let crown = FLAT_ARENA_Z + *height;
        for dy in -1..=1 {
            for dx in -1..=1 {
                cells.push((
                    Vec3::new(column.x + dx, column.y + dy, crown),
                    Block::new(BlockKind::Leaves, Rgb::new(42, 92, 34)),
                ));
            }
        }
    }

    // MINE: a solid rock outcrop.
    let outcrop = centre + RESOURCED_OUTCROP_OFFSET;
    for z in 0..RESOURCED_OUTCROP_HEIGHT {
        for dy in -RESOURCED_OUTCROP_HALF_WIDTH..=RESOURCED_OUTCROP_HALF_WIDTH {
            for dx in -RESOURCED_OUTCROP_HALF_WIDTH..=RESOURCED_OUTCROP_HALF_WIDTH {
                cells.push((
                    Vec3::new(outcrop.x + dx, outcrop.y + dy, FLAT_ARENA_Z + z),
                    Block::new(BlockKind::Rock, Rgb::new(94, 94, 98)),
                ));
            }
        }
    }

    cells
}

/// The arena anchor: the map-center wpos (the default spawn area), from
/// each World flavor's own size accessor — the one place the
/// worldgen/test_world duality is bridged.
pub fn world_center_wpos(world: &World) -> Vec2<u32> {
    #[cfg(feature = "worldgen")]
    {
        world.sim().get_size() / 2 * TerrainChunkSize::RECT_SIZE
    }
    #[cfg(not(feature = "worldgen"))]
    {
        world.get_center()
    }
}

/// The arena spawn point: dead center, feet on the slab. The normal spawn
/// calc aims at the nearest TOWN with a sim-derived altitude — both wrong
/// under the override (wrong place, and a z that may be inside or far
/// above the slab), so when the arena is on, spawn is owned here.
/// `TerrainChunk::new(z, below, above)` makes everything below `z` solid,
/// so the first air cell is `FLAT_ARENA_Z` itself; +1 clears any landing
/// jitter without fall damage.
pub fn spawn_wpos(center_wpos: Vec2<u32>) -> Vec3<f32> {
    Vec3::new(
        center_wpos.x as f32 + 0.5,
        center_wpos.y as f32 + 0.5,
        FLAT_ARENA_Z as f32 + 1.0,
    )
}

/// The override: `Some(flat chunk)` for in-arena keys when enabled, else
/// `None` (callers fall through to the real generator). `center_wpos` =
/// `world.get_center()` — the same anchor the default spawn waypoint uses,
/// so the arena is exactly where Ben lands.
pub fn override_chunk(
    center_wpos: Vec2<u32>,
    key: Vec2<i32>,
) -> Option<(TerrainChunk, ChunkSupplement)> {
    if !enabled() {
        return None;
    }
    let center_key = center_wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
    if (key - center_key).map(|e| e.abs()).reduce_max() > FLAT_ARENA_RADIUS_CHUNKS {
        return None;
    }
    let mut chunk = TerrainChunk::new(
        FLAT_ARENA_Z,
        Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
        Block::air(SpriteKind::Empty),
        TerrainChunkMeta::void(),
    );

    // FOUNDING PRESET v1, packet §2: the RESOURCED variant's features are
    // written AT GENERATION, into the chunk this call is producing — not
    // by a post-hoc `set_block` pass over a live world. Same reasoning as
    // the flat slab itself: a regenerated chunk comes back identical, so
    // the layout survives unload/reload with no persistence dependency.
    // A post-hoc pass would lose its trees the first time a chunk cycled,
    // and the arena's matched-control property ("a red means the FEATURE
    // broke, never the terrain") would go with them.
    if resourced() {
        apply_resourced_features(&mut chunk, center_wpos, key);
    }

    Some((
        chunk,
        // Empty on purpose: no worldgen entities — the arena spawns
        // nothing; Ben founds his colony through the normal command.
        ChunkSupplement::default(),
    ))
}

/// Write the feature cells that fall inside `key`'s chunk into `chunk`.
///
/// Extracted so the test asserts THE SAME CODE the generator runs, not a
/// parallel description of it (the F8 lesson: a fixture that re-implements
/// its subject certifies the fixture). The gate lives at the call site;
/// this function is unconditional and therefore directly testable.
///
/// A Chonk write takes LOCAL xy and an ABSOLUTE z — getting that wrong
/// writes the outcrop into a neighbouring z-band silently, which is why
/// the test drives this function rather than the cell list alone.
pub fn apply_resourced_features(chunk: &mut TerrainChunk, center_wpos: Vec2<u32>, key: Vec2<i32>) {
    let chunk_origin = key * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
    for (wpos, block) in resourced_feature_cells(center_wpos) {
        let local = wpos.xy() - chunk_origin;
        if (0..TerrainChunkSize::RECT_SIZE.x as i32).contains(&local.x)
            && (0..TerrainChunkSize::RECT_SIZE.y as i32).contains(&local.y)
        {
            let _ = chunk.set(Vec3::new(local.x, local.y, wpos.z), block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::vol::ReadVol;

    /// The world centre used by the tests — an arbitrary but fixed
    /// stand-in for `world.get_center()`.
    const CENTRE: Vec2<u32> = Vec2::new(15216, 16016);

    /// §2's core property: the SAME LAYOUT EVERY RUN. Nothing here is
    /// seeded, sampled, or ordered by a hash — so two calls are equal
    /// element-for-element, and a run's terrain can never be the
    /// explanation for a red.
    #[test]
    fn resourced_layout_is_identical_every_time() {
        let first = resourced_feature_cells(CENTRE);
        let second = resourced_feature_cells(CENTRE);
        assert_eq!(first, second, "the resourced arena must be deterministic");
        assert!(!first.is_empty(), "the variant must actually place something");
    }

    /// The arena has to give BOTH verbs real work: chop (Wood) and mine
    /// (Rock). This is the convergence the packet rests on — those
    /// completions are F8's missing inclusion evidence, so an arena
    /// carrying only one of them would quietly halve the run's value.
    #[test]
    fn resourced_arena_carries_both_chop_and_mine_work() {
        let cells = resourced_feature_cells(CENTRE);
        let wood = cells.iter().filter(|(_, b)| b.kind() == BlockKind::Wood).count();
        let rock = cells.iter().filter(|(_, b)| b.kind() == BlockKind::Rock).count();
        let leaves = cells.iter().filter(|(_, b)| b.kind() == BlockKind::Leaves).count();

        assert_eq!(
            wood,
            RESOURCED_TREES.iter().map(|(_, h)| *h as usize).sum::<usize>(),
            "every trunk block must be Wood -- only Wood yields (FR10)"
        );
        assert_eq!(
            rock,
            (RESOURCED_OUTCROP_HALF_WIDTH * 2 + 1).pow(2) as usize
                * RESOURCED_OUTCROP_HEIGHT as usize
        );
        assert!(leaves > 0, "the Leaves (clear-free) arm gets exercised too");
    }

    /// The founding ground stays CLEAR: no feature cell may land inside
    /// the radius the preset's own footprint occupies. If a tree grew
    /// where the farm goes, a terrain refusal would fire on the arena
    /// itself and the acceptance would be measuring the wrong thing.
    #[test]
    fn resourced_features_keep_the_founding_ground_clear() {
        let centre = CENTRE.map(|e| e as i32);
        for (wpos, block) in resourced_feature_cells(CENTRE) {
            let offset = wpos.xy() - centre;
            assert!(
                offset.x.abs() > RESOURCED_CLEAR_RADIUS || offset.y.abs() > RESOURCED_CLEAR_RADIUS,
                "{:?} at {:?} sits inside the founding clear radius",
                block.kind(),
                offset
            );
        }
    }

    /// Every feature sits ON the slab (at or above the first air cell),
    /// never buried in it: a trunk starts where a colonist's feet would.
    #[test]
    fn resourced_features_sit_on_the_slab_not_in_it() {
        for (wpos, _) in resourced_feature_cells(CENTRE) {
            assert!(
                wpos.z >= FLAT_ARENA_Z,
                "feature cell at z={} is inside the slab (first air is {})",
                wpos.z,
                FLAT_ARENA_Z
            );
        }
    }

    /// The write path itself: a generated chunk must actually CONTAIN the
    /// features whose cells fall inside it — the Chonk write takes local
    /// xy with an ABSOLUTE z, and getting that wrong writes the outcrop
    /// into a neighbouring column silently.
    #[test]
    fn generated_chunk_contains_the_features_that_fall_in_it() {
        use common::vol::RectVolSize;

        let centre = CENTRE.map(|e| e as i32);
        let outcrop = centre + RESOURCED_OUTCROP_OFFSET;
        let key = outcrop.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e.div_euclid(sz as i32));
        let chunk_origin = key * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);

        let mut chunk = TerrainChunk::new(
            FLAT_ARENA_Z,
            Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
            Block::air(SpriteKind::Empty),
            TerrainChunkMeta::void(),
        );
        // THE PRODUCTION PATH, called — not re-implemented here.
        apply_resourced_features(&mut chunk, CENTRE, key);

        let local_outcrop = outcrop - chunk_origin;
        assert_eq!(
            chunk
                .get(Vec3::new(local_outcrop.x, local_outcrop.y, FLAT_ARENA_Z))
                .map(|b| b.kind())
                .ok(),
            Some(BlockKind::Rock),
            "the outcrop's own column must be Rock at the slab's first air cell"
        );
        // And the slab under it is untouched.
        assert_eq!(
            chunk
                .get(Vec3::new(local_outcrop.x, local_outcrop.y, FLAT_ARENA_Z - 1))
                .map(|b| b.kind())
                .ok(),
            Some(BlockKind::Grass)
        );
    }
}
