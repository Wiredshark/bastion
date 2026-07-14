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
    vol::RectVolSize,
};
use std::sync::OnceLock;
use vek::*;
#[cfg(feature = "worldgen")]
use world::World;

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
    *ON.get_or_init(|| {
        std::env::var("BASTION_FLAT_ARENA").is_ok_and(|v| v != "0")
    })
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
    let center_key = center_wpos
        .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
    if (key - center_key).map(|e| e.abs()).reduce_max()
        > FLAT_ARENA_RADIUS_CHUNKS
    {
        return None;
    }
    Some((
        TerrainChunk::new(
            FLAT_ARENA_Z,
            Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
            Block::air(SpriteKind::Empty),
            TerrainChunkMeta::void(),
        ),
        // Empty on purpose: no worldgen entities — the arena spawns
        // nothing; Ben founds his colony through the normal command.
        ChunkSupplement::default(),
    ))
}
