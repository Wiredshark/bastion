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

/// The largest client VIEW DISTANCE, in chunks, that any scored arm has used.
/// `HAUL-FIXTURE-RESULTS.md` / `VIEW-DISTANCE-RESULTS.md` ran VD **6** and VD
/// **25** as an A/B (the pair that refuted view distance as the haul-count
/// mechanism), so 25 is the horizon the fixture must actually cover.
///
/// ★ Named as a constant rather than written into the radius, so the arena's
/// own test can state the invariant instead of comparing two magic numbers.
pub const MAX_TESTED_VIEW_DISTANCE_CHUNKS: i32 = 25;

/// Arena half-width in CHUNKS (32-block chunks). Sized against observed colony
/// dynamics: generator scan radii (±12), the soft-magnet leash, and the
/// deep-wander excursions the AUTON-2 forensics measured (~150 blocks) all fit
/// with a wide margin, so a test colony's whole behavioral footprint stays on
/// the flat.
///
/// ★★ **AND — ITEM 19 — AGAINST THE CLIENT'S VIEW HORIZON, WHICH THE ORIGINAL
/// SIZING NEVER CONSIDERED.** The paragraph above answers *"where do colonists
/// walk"*. A renderer horizon test asks *"what does the client SEE"*, and those
/// are different requirements on the same number. At the old value of **16**, a
/// client at VD **25** rendered **9 chunks past the slab** into ordinary
/// worldgen — so any horizon measurement was contaminated by real terrain and
/// the retest was void by construction. **That is the one-constant fixture
/// defect the roadmap names, and the constant was never wrong for its stated
/// purpose — it was silently reused for a second one.**
///
/// 26 = [`MAX_TESTED_VIEW_DISTANCE_CHUNKS`] + 1 chunk of margin ⇒ a
/// 1696×1696-block slab. `arena_radius_covers_the_tested_view_horizon` holds
/// the invariant so a future VD bump fails the test instead of silently
/// re-voiding the row.
///
/// ★ COST — **MEASURED 2026-08-17, and the number is small.** The override area
/// grows 33×33 → 53×53 chunks (~2.6×), and server boot-to-port went
/// **63–75 s (six runs at radius 16) → 78 s (radius 26)**. A few seconds, near
/// noise — because each overridden chunk is a flat slab written directly,
/// which is cheaper than the real worldgen it replaces, so 2.6× the chunks is
/// nothing like 2.6× the time.
///
/// ⚠ *Recorded because I briefly believed the opposite.* Watching a log that
/// had not yet flushed its `port open after` line, I read my own polling
/// latency as server latency and started drafting a ">3× regression, make the
/// radius conditional" fix. **The absence of a line is not a measurement of
/// the thing the line reports** — the same law the evidence files are held to,
/// applied to a live log.
pub const FLAT_ARENA_RADIUS_CHUNKS: i32 = 26;

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

/// bastion (WALL-FIXTURE, ITEM 15a): half-extent of the perimeter WALL — a
/// hollow square ring centred on the arena, enclosing the colony.
///
/// ★ WHY A WORLD-GENERATED WALL AND NOT A `Build` DESIGNATION: item 15 splits
/// into a PHYSICS question ("does a wall stop a hostile") and an ECONOMY
/// question ("can the colony build one"). `Build` requires
/// `BUILD_MATERIAL_ITEM` and carries the blocked-materials machinery that has
/// stalled real arms, so a colony-built wall makes "the wolf got in"
/// ambiguous between *the wall failed* and *the wall was never finished*.
/// Generating it makes the wall's EXISTENCE a fixture fact, and leaves the
/// run measuring exactly one thing.
///
/// ★ THE NUMBER COMES FROM THE FEATURE SET, NOT FROM A GUESS. The furthest
/// resourced feature is the third tree at [`RESOURCED_TREES`]`(25, 4)`, and the
/// outcrop reaches `|dx| = 22` (`RESOURCED_OUTCROP_OFFSET` −20 ± half-width 2).
/// **My first value was 24 and `wall_ring_clears_the_work_area_and_the_features`
/// failed on that very tree** (`dx=25`) — the ring would have left one of the
/// colony's own chop targets OUTSIDE its wall, and the arm would have measured
/// a wall and a severed work area at once. 32 clears the furthest feature by 7
/// and is one chunk width, and it is still far inside the arena's own radius
/// ([`FLAT_ARENA_RADIUS_CHUNKS`] × 32), so the slab extends well past the wall
/// on every side — a hostile must have somewhere to stand OUTSIDE it, or the
/// fixture cannot pose its own question.
///
/// ★ This is a LAYOUT constraint, not a result-tuning one, and the distinction
/// matters: [`WALL_HEIGHT`] must never be raised to make a treatment pass
/// (the height that stops a hostile *is* the measurement), whereas the radius
/// is fixed by where the arena already puts its trees. **The test derives the
/// constraint from `resourced_feature_cells` itself rather than from a
/// remembered list, which is why it caught this.**
pub const WALL_RADIUS: i32 = 32;

/// bastion (WALL-FIXTURE, ITEM 15a): the wall's height in blocks.
///
/// ⚠ **THIS CONSTANT IS A CLAIM ABOUT TRAVERSAL AND IT IS NOT YET VERIFIED.**
/// The reachability model has STEP / JUMP / SCRAMBLE tiers, and 4 is chosen to
/// sit above a step and a plain jump — but *what a quadruped can clear is
/// unread*. A wall the hostile simply hops is a CONTROL FAILURE, not a
/// treatment failure, so any run using this fixture must print the
/// hostile's own approach against an unwalled control before reading the
/// treatment arm. **Do not raise this to "make the test pass": the number
/// that stops a wolf IS the measurement.**
pub const WALL_HEIGHT: i32 = 4;

/// bastion (WALL-FIXTURE, ITEM 15a): is the perimeter wall on?
///
/// A VARIANT OF A VARIANT, gated exactly like [`pit_depth`]/[`shaft_depth`]:
/// false unless the resourced arena is itself on, so the wall can never
/// half-apply to a normal world or the bare slab. ★ Unlike pit-vs-shaft it is
/// NOT mutually exclusive with them — the wall is at the PERIMETER and the
/// others are at the CENTRE, so they compose.
pub fn walled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        resourced() && std::env::var("BASTION_FLAT_ARENA_WALLED").is_ok_and(|v| v != "0")
    })
}

/// bastion (WALL-FIXTURE, ITEM 15a): the perimeter ring's cells, as absolute
/// world cells. Pure and radius/height-passed for the same reason as the pit
/// and shaft forms: assertable in one test binary with no env manipulation.
///
/// A HOLLOW square — only the four edges, never the interior — because the
/// point is an enclosure, not a plateau. Emitted from [`FLAT_ARENA_Z`] (the
/// slab's first air cell) upward, so the wall stands ON the ground exactly as
/// a tree trunk does.
pub fn wall_cells(center_wpos: Vec2<u32>, radius: i32, height: i32) -> Vec<(Vec3<i32>, Block)> {
    let centre = center_wpos.map(|e| e as i32);
    let mut cells = Vec::new();
    for dz in 0..height {
        for d in -radius..=radius {
            // The four edges. Corners are produced twice by construction (once
            // by the x-pair, once by the y-pair); `chunk.set` is idempotent for
            // an identical block, so the duplicate is harmless and dropping it
            // would cost a branch in every iteration to save nothing.
            for (dx, dy) in [(d, -radius), (d, radius), (-radius, d), (radius, d)] {
                cells.push((
                    Vec3::new(centre.x + dx, centre.y + dy, FLAT_ARENA_Z + dz),
                    Block::new(BlockKind::Rock, Rgb::new(94, 94, 98)),
                ));
            }
        }
    }
    cells
}

/// bastion (VERTICAL-FIXTURE): half-width of the MINE PIT — the
/// depression excavated around the outcrop when the pit variant is on.
/// Wider than [`RESOURCED_OUTCROP_HALF_WIDTH`] so the outcrop sits on the
/// pit FLOOR with clear ground around it, rather than wedged against a
/// wall where a miner could never stand beside it.
pub const PIT_HALF_WIDTH: i32 = 5;

/// bastion (VERTICAL-FIXTURE): the pit's depth in blocks when enabled.
///
/// WHY A PIT AND NOT A TALLER OUTCROP. `BastionColonistStatus::RestingToClimb`
/// is written inside the EMERGENCY EGRESS machinery, gated on
/// `grounded_clear && !route_energy_ready` — it is not "a colonist climbing
/// a hill", it is a colonist STUCK on an escape route and too tired to get
/// out. The status surface's own doc calls these "the four indistinguishable
/// PIT states". Height produces no escape route; a depression does.
/// ⚠ MEASURED 2026-08-16: THIS PIT NEVER TRAPS ANYONE, and the reasoning
/// above — sound about the MACHINERY — never checked the DETECTOR.
/// `egress_scan_with` accepts any ring surface with
/// `s >= feet.z - 4 && s <= feet.z + reach - 1`, so a 4-deep floor is inside
/// its own "level-or-lower surfaces COUNT as egress (hop down)" band; and at
/// [`PIT_HALF_WIDTH`] `= 5` the pit is 10 across, past the scan's documented
/// ">7 across evades this local test" limit. Four rows chased `status`
/// reading `None` before the read found this (EGRESS-ENTRY-READ.md). The
/// replacement geometry is [`SHAFT_DEPTH`]/[`SHAFT_HALF_WIDTH`] below; the
/// pit constants are LEFT UNCHANGED so every committed pit-arm result stays
/// reproducible.
pub const PIT_DEPTH: i32 = 4;

/// bastion (SHAFT-FIXTURE): the shaft variant's depth — the geometry the
/// egress detector's own arithmetic requires for a TRAP.
///
/// `reach = cap_for_skill(0) = 3` for a novice, so a rim is standable at
/// `feet.z + 2` or lower. At 8 deep the rim is 6 blocks above that, and 8
/// is also outside the `feet.z - 4` hop-down band.
pub const SHAFT_DEPTH: i32 = 8;

/// bastion (SHAFT-FIXTURE): half-width 1 — a 3-across shaft.
///
/// ★ NOT A SIZE PREFERENCE, AN ARITHMETIC REQUIREMENT. The scan's annulus
/// starts at `d >= 3`; if any FLOOR cell sits that far from a colonist, the
/// floor itself reads as egress (it is level with the feet) and the trap
/// never fires. Two floor cells are at most `2 * half_width` apart, so the
/// trap needs `2 * half_width <= 2`. A 7-across shaft — wide enough to ring
/// the normal 5-across outcrop — FAILS for exactly this reason, which is
/// pinned as an assertion in `bastion_jobs`'s
/// `egress_scan_traps_in_a_narrow_shaft_and_frees_in_the_wide_pit`.
pub const SHAFT_HALF_WIDTH: i32 = 1;

/// bastion (SHAFT-FIXTURE): the shaft's outcrop is a SINGLE COLUMN.
///
/// The normal outcrop is 5 across ([`RESOURCED_OUTCROP_HALF_WIDTH`] `= 2`)
/// and cannot fit in a 3-across shaft with anywhere to stand. One column
/// leaves the 8 surrounding floor cells free — and every one of them is
/// within `d < 3` of the others, so none of them is scanned as egress.
pub const SHAFT_OUTCROP_HALF_WIDTH: i32 = 0;

/// bastion (SHAFT-FIXTURE): the shaft variant's depth, or 0 when off.
/// Gated exactly like [`pit_depth`], and MUTUALLY EXCLUSIVE with it: a run
/// that asks for both gets the shaft, because the pit's own doc above
/// records that it cannot trap.
pub fn shaft_depth() -> i32 {
    static ON: OnceLock<bool> = OnceLock::new();
    let on = *ON.get_or_init(|| {
        resourced() && std::env::var("BASTION_FLAT_ARENA_SHAFT").is_ok_and(|v| v != "0")
    });
    if on { SHAFT_DEPTH } else { 0 }
}

/// bastion (VERTICAL-FIXTURE): the pit variant's depth, or 0 when off.
///
/// A VARIANT OF A VARIANT, gated exactly like [`resourced`]: false unless
/// the resourced arena is itself on, so the pit can never half-apply to a
/// normal world or to the bare slab.
pub fn pit_depth() -> i32 {
    static ON: OnceLock<bool> = OnceLock::new();
    let on = *ON.get_or_init(|| {
        resourced() && std::env::var("BASTION_FLAT_ARENA_PIT").is_ok_and(|v| v != "0")
    });
    if on { PIT_DEPTH } else { 0 }
}

/// THE FEATURE SET, as absolute world cells — pure, so the layout can be
/// asserted without generating a chunk.
///
/// With `pit_depth == 0` every cell sits at or above [`FLAT_ARENA_Z`] (the
/// slab's first air cell), i.e. ON the ground rather than in it: a tree's
/// trunk starts where a colonist's feet would.
pub fn resourced_feature_cells(center_wpos: Vec2<u32>) -> Vec<(Vec3<i32>, Block)> {
    // SHAFT-FIXTURE: the shaft WINS over the pit when both are asked for.
    // The pit's own doc records that it cannot trap anyone (the egress scan
    // reads a 4-deep, 10-across depression as escapable, correctly), so a
    // run that wants a trap gets the geometry that produces one.
    let shaft = shaft_depth();
    let mut cells = if shaft > 0 {
        resourced_feature_cells_shaft(center_wpos, shaft)
    } else {
        resourced_feature_cells_with_pit(center_wpos, pit_depth())
    };
    // WALL-FIXTURE (ITEM 15a): APPENDED, not substituted. The wall is at the
    // perimeter and the pit/shaft are at the centre, so it composes with
    // whichever centre geometry is active rather than replacing it. Appended
    // LAST so it is applied after any excavation, matching the ordering rule
    // the pit and shaft already rely on (`apply_resourced_features` applies
    // cells in order via `chunk.set`).
    if walled() {
        cells.extend(wall_cells(center_wpos, WALL_RADIUS, WALL_HEIGHT));
    }
    cells
}

/// SHAFT-FIXTURE: the TRAPPING geometry — a narrow deep shaft whose only
/// mine work is a single column on its floor.
///
/// Pure and depth-passed for the same reason as the pit form: assertable in
/// one test binary with no env manipulation.
///
/// WHY THIS SHAPE, from the detector's arithmetic (EGRESS-ENTRY-READ.md):
/// `egress_scan_with` accepts a ring surface iff
/// `s >= feet.z - 4 && s <= feet.z + reach - 1`, scanning only `d >= 3`.
/// [`SHAFT_DEPTH`] `= 8` puts the rim far above `feet + reach - 1` (= +2 for
/// a novice) and outside the −4 hop-down band; [`SHAFT_HALF_WIDTH`] `= 1`
/// guarantees no FLOOR cell is ever `d >= 3` from a colonist standing in it,
/// which is the failure a 7-across shaft would have had.
pub fn resourced_feature_cells_shaft(
    center_wpos: Vec2<u32>,
    shaft_depth: i32,
) -> Vec<(Vec3<i32>, Block)> {
    let centre = center_wpos.map(|e| e as i32);
    let mut cells = Vec::new();
    let outcrop = centre + RESOURCED_OUTCROP_OFFSET;

    // THE SHAFT, EXCAVATED FIRST — same ordering rule as the pit: air goes
    // down before the outcrop is carved back in, because
    // `apply_resourced_features` applies cells in order via `chunk.set`.
    for dz in -shaft_depth..0 {
        for dy in -SHAFT_HALF_WIDTH..=SHAFT_HALF_WIDTH {
            for dx in -SHAFT_HALF_WIDTH..=SHAFT_HALF_WIDTH {
                cells.push((
                    Vec3::new(outcrop.x + dx, outcrop.y + dy, FLAT_ARENA_Z + dz),
                    Block::air(SpriteKind::Empty),
                ));
            }
        }
    }

    // THE SINGLE-COLUMN OUTCROP on the shaft floor: the only mine work in
    // the arena, so a miner must descend to reach it. At
    // [`SHAFT_OUTCROP_HALF_WIDTH`] `= 0` this is one column, leaving the
    // eight surrounding floor cells free to stand on.
    for z in 0..RESOURCED_OUTCROP_HEIGHT {
        for dy in -SHAFT_OUTCROP_HALF_WIDTH..=SHAFT_OUTCROP_HALF_WIDTH {
            for dx in -SHAFT_OUTCROP_HALF_WIDTH..=SHAFT_OUTCROP_HALF_WIDTH {
                cells.push((
                    Vec3::new(outcrop.x + dx, outcrop.y + dy, FLAT_ARENA_Z - shaft_depth + z),
                    Block::new(BlockKind::Rock, Rgb::new(94, 94, 98)),
                ));
            }
        }
    }
    cells
}

/// The pure form, with the pit depth PASSED IN rather than read from the
/// environment — so both the flat invariant (`pit_depth == 0`, features sit
/// on the slab) and the pit invariant (`> 0`, the outcrop sits on the pit
/// floor) are assertable in one test binary, with no env manipulation and
/// no `OnceLock` that a second test would inherit.
pub fn resourced_feature_cells_with_pit(
    center_wpos: Vec2<u32>,
    pit_depth: i32,
) -> Vec<(Vec3<i32>, Block)> {
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

    // THE PIT, EXCAVATED FIRST. Emitted BEFORE the outcrop because
    // `apply_resourced_features` applies cells in order via `chunk.set`, so
    // a later solid cell overwrites this air — the outcrop is carved back
    // in below, and the ordering is what keeps it solid.
    //
    // THE OUTCROP GOES IN THE PIT, rather than the pit going somewhere
    // else, because a fixture only fires if the colony has a REASON to
    // enter it. A depression beside the work is a hole colonists path
    // around; a depression CONTAINING the only mine work is one a miner
    // must descend into and then escape — which is the emergency-egress
    // scenario `RestingToClimb` is written from.
    for dz in -pit_depth..0 {
        for dy in -PIT_HALF_WIDTH..=PIT_HALF_WIDTH {
            for dx in -PIT_HALF_WIDTH..=PIT_HALF_WIDTH {
                cells.push((
                    Vec3::new(outcrop.x + dx, outcrop.y + dy, FLAT_ARENA_Z + dz),
                    Block::air(SpriteKind::Empty),
                ));
            }
        }
    }

    // The outcrop stands on the pit FLOOR (`-pit_depth`), so it is still a
    // solid minable column and still 3 blocks tall — the mine work is
    // unchanged, only its elevation is. With `pit_depth == 0` this is
    // exactly the original expression.
    for z in 0..RESOURCED_OUTCROP_HEIGHT {
        for dy in -RESOURCED_OUTCROP_HALF_WIDTH..=RESOURCED_OUTCROP_HALF_WIDTH {
            for dx in -RESOURCED_OUTCROP_HALF_WIDTH..=RESOURCED_OUTCROP_HALF_WIDTH {
                cells.push((
                    Vec3::new(outcrop.x + dx, outcrop.y + dy, FLAT_ARENA_Z - pit_depth + z),
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
// post-r2 (lw port): the radius predicate named and testable — the horizon
// diagnostics reason about exactly this boundary.
fn within_flat_override_radius(center_key: Vec2<i32>, key: Vec2<i32>) -> bool {
    (key - center_key).map(i32::abs).reduce_max() <= FLAT_ARENA_RADIUS_CHUNKS
}

pub fn override_chunk(
    center_wpos: Vec2<u32>,
    key: Vec2<i32>,
) -> Option<(TerrainChunk, ChunkSupplement)> {
    if !enabled() {
        return None;
    }
    let center_key = center_wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
    if !within_flat_override_radius(center_key, key) {
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

    // post-r2 (lw port): the override boundary, asserted at the corner and
    // one past it.
    #[test]
    fn flat_override_radius_is_bounded_and_outside_is_not_overridden() {
        let center = Vec2::new(100, 100);
        assert!(within_flat_override_radius(center, center));
        assert!(within_flat_override_radius(
            center,
            center + Vec2::new(FLAT_ARENA_RADIUS_CHUNKS, -FLAT_ARENA_RADIUS_CHUNKS)
        ));
        assert!(!within_flat_override_radius(
            center,
            center + Vec2::new(FLAT_ARENA_RADIUS_CHUNKS + 1, 0)
        ));
    }

    /// The world centre used by the tests — an arbitrary but fixed
    /// stand-in for `world.get_center()`.
    const CENTRE: Vec2<u32> = Vec2::new(15216, 16016);

    /// ITEM 19: the arena must extend at least as far as the client can SEE,
    /// not merely as far as colonists WALK.
    ///
    /// ★ The radius was originally sized against colony dynamics alone, and was
    /// correct for that. Reusing it as the renderer-horizon fixture added a
    /// second requirement nobody wrote down, and at radius 16 vs VD 25 the
    /// client rendered 9 chunks of REAL worldgen past the slab — so the horizon
    /// retest could never have measured the flat arena.
    ///
    /// This test exists so the next VD increase FAILS HERE rather than silently
    /// voiding that row again: it is the invariant, stated once, in the place
    /// that owns the number.
    #[test]
    fn arena_radius_covers_the_tested_view_horizon() {
        assert!(
            FLAT_ARENA_RADIUS_CHUNKS > MAX_TESTED_VIEW_DISTANCE_CHUNKS,
            "arena radius {FLAT_ARENA_RADIUS_CHUNKS} chunks does not strictly cover the \
             largest tested view distance {MAX_TESTED_VIEW_DISTANCE_CHUNKS} — a client at \
             that VD renders {} chunk(s) of real worldgen beyond the slab, which voids any \
             renderer-horizon measurement",
            MAX_TESTED_VIEW_DISTANCE_CHUNKS - FLAT_ARENA_RADIUS_CHUNKS + 1,
        );
    }

    /// WALL-FIXTURE (ITEM 15a): the ring is HOLLOW, CLOSED, and stands ON the
    /// slab. Asserted without a chunk, a server, or an env var — the fixture's
    /// own design rule, and the reason `wall_cells` takes radius/height rather
    /// than reading them from the environment.
    ///
    /// ★ These are the three ways a perimeter can silently fail to be one:
    /// a GAP (a hostile walks through and the row blames the AI), a SOLID
    /// interior (the colony is entombed, not enclosed), and a wall floating
    /// above or buried below the ground plane.
    #[test]
    fn wall_ring_is_hollow_closed_and_on_the_slab() {
        let cells = wall_cells(CENTRE, WALL_RADIUS, WALL_HEIGHT);
        let c = CENTRE.map(|e| e as i32);
        let occupied: std::collections::HashSet<Vec3<i32>> =
            cells.iter().map(|(p, _)| *p).collect();

        // 1. CLOSED: every cell of every edge is present, at every height.
        for dz in 0..WALL_HEIGHT {
            for d in -WALL_RADIUS..=WALL_RADIUS {
                for (dx, dy) in [
                    (d, -WALL_RADIUS),
                    (d, WALL_RADIUS),
                    (-WALL_RADIUS, d),
                    (WALL_RADIUS, d),
                ] {
                    let p = Vec3::new(c.x + dx, c.y + dy, FLAT_ARENA_Z + dz);
                    assert!(occupied.contains(&p), "gap in the ring at {p:?}");
                }
            }
        }

        // 2. HOLLOW: nothing strictly inside the ring is filled. The colony
        //    has to live in there.
        for dy in -(WALL_RADIUS - 1)..=(WALL_RADIUS - 1) {
            for dx in -(WALL_RADIUS - 1)..=(WALL_RADIUS - 1) {
                for dz in 0..WALL_HEIGHT {
                    let p = Vec3::new(c.x + dx, c.y + dy, FLAT_ARENA_Z + dz);
                    assert!(!occupied.contains(&p), "interior filled at {p:?}");
                }
            }
        }

        // 3. ON THE SLAB: the lowest course sits at the first air cell, so the
        //    wall stands on the ground exactly as a tree trunk does — not
        //    hovering above it, not buried in it.
        let min_z = cells.iter().map(|(p, _)| p.z).min().unwrap();
        let max_z = cells.iter().map(|(p, _)| p.z).max().unwrap();
        assert_eq!(min_z, FLAT_ARENA_Z, "wall does not start at the slab top");
        assert_eq!(max_z, FLAT_ARENA_Z + WALL_HEIGHT - 1, "wall height wrong");
    }

    /// WALL-FIXTURE (ITEM 15a): the ring must not eat the work area. If the
    /// wall overlapped the cleared radius or the features, the fixture would
    /// be testing two changes at once and any result would be unattributable.
    #[test]
    fn wall_ring_clears_the_work_area_and_the_features() {
        assert!(
            WALL_RADIUS > RESOURCED_CLEAR_RADIUS,
            "wall at {WALL_RADIUS} would overlap the cleared work area ({RESOURCED_CLEAR_RADIUS})"
        );
        // Every feature the resourced arena places must fall strictly INSIDE
        // the ring -- checked against the real feature set, not against a
        // remembered list of what it contains.
        for (pos, _) in resourced_feature_cells(CENTRE) {
            let c = CENTRE.map(|e| e as i32);
            let (dx, dy) = ((pos.x - c.x).abs(), (pos.y - c.y).abs());
            assert!(
                dx < WALL_RADIUS && dy < WALL_RADIUS,
                "feature cell {pos:?} is on or outside the ring (dx={dx}, dy={dy})"
            );
        }
    }

    /// VERTICAL-FIXTURE: with the pit on, the outcrop sits on the pit
    /// FLOOR and the excavation reaches full depth — the two facts the
    /// fixture's whole premise rests on, asserted without a chunk, a
    /// server, or an env var.
    ///
    /// The pit must be DEEPER than the outcrop is tall, or a miner could
    /// simply walk up its own work and out; that is asserted here rather
    /// than left as a comment, because it is the difference between a
    /// depression a colonist must escape and a ramp.
    #[test]
    fn pit_variant_puts_the_outcrop_on_the_pit_floor() {
        let flat = resourced_feature_cells_with_pit(CENTRE, 0);
        let pit = resourced_feature_cells_with_pit(CENTRE, PIT_DEPTH);

        let lowest_rock = |cells: &[(Vec3<i32>, Block)]| {
            cells
                .iter()
                .filter(|(_, b)| b.kind() == BlockKind::Rock)
                .map(|(p, _)| p.z)
                .min()
                .expect("the arena must place mine work")
        };
        assert_eq!(lowest_rock(&flat), FLAT_ARENA_Z, "flat: outcrop on the slab");
        assert_eq!(
            lowest_rock(&pit),
            FLAT_ARENA_Z - PIT_DEPTH,
            "pit: the outcrop must stand on the pit floor, not float at slab level"
        );

        // The excavation reaches the floor and is wider than the outcrop.
        let deepest_air = pit
            .iter()
            .filter(|(_, b)| b.is_air())
            .map(|(p, _)| p.z)
            .min()
            .expect("the pit variant must excavate");
        assert_eq!(deepest_air, FLAT_ARENA_Z - PIT_DEPTH);
        assert!(
            PIT_HALF_WIDTH > RESOURCED_OUTCROP_HALF_WIDTH,
            "a miner needs floor to stand on beside the outcrop"
        );
        assert!(
            PIT_DEPTH > RESOURCED_OUTCROP_HEIGHT,
            "a pit no deeper than the outcrop is tall is a ramp, not a pit"
        );

        // And the flat arm excavates NOTHING — the control is a control.
        assert!(
            !flat.iter().any(|(_, b)| b.is_air()),
            "pit_depth = 0 must leave the slab untouched"
        );
    }

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
        // Pinned to the pure form at depth 0: the invariant is a property of
        // the UNPITTED arena, and reading it through the env-gated wrapper
        // would make this test's meaning depend on a process-wide `OnceLock`
        // that any other test could have set first.
        for (wpos, _) in resourced_feature_cells_with_pit(CENTRE, 0) {
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
