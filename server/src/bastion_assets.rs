//! bastion (B-ASSET1): the asset-lab runtime loader + placement seam.
//!
//! Loads generated `.vox` assets from the asset-lab workspace (OUTSIDE the
//! game asset tree — vanilla assets stay byte-identical) through the real
//! `Structure`/`custom_indices` machinery, asserts marker fidelity (the
//! welded-gate-class guard, engine side), and stamps structures into live
//! terrain through the authoritative `BlockChange` path (`State::set_block`,
//! the same path B5 work-execution uses — mesh/rtsim hooks fire, never raw
//! chonk writes).
//!
//! Everything here is inert unless explicitly invoked (harness `--asset-test`
//! or the `--asset-arena` client mode); no vanilla path calls into this
//! module. Format contract with the asset session (it reads this back via
//! `readme/ASSET_INTEGRATION_LOG.md`):
//! - input = flattened `.vox` in `<asset-lab>/vox/` (compose.py pre-flattens
//!   compositions; per-component placement deliberately not supported),
//! - byte bands per ASSET_LESSONS L3: 1–16 world-reserved (engine defaults),
//!   32–199 literal colors, 200–255 gameplay markers resolved through
//!   [`marker_registry`],
//! - unknown marker-band bytes load with a literal fallback and FAIL the
//!   fidelity gate (declare new markers in the registry, engine-side, first).

use common::{
    terrain::{
        Block, Structure,
        structure::{BASTION_MARKER_BAND_START, BastionVoxCensus, StructureBlock},
    },
    vol::ReadVol,
};
use common_state::State;
use hashbrown::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use vek::*;
use world::{IndexRef, World, block_from_structure, util::Sampler};

/// Coarse asset category inferred from the asset-lab id prefix; picks the
/// dynamic-test cast (ASSET_DYNAMIC_TEST_SPEC §per-asset-type).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetCategory {
    /// Structures with interiors (housing/production/social/dungeon…).
    Structure,
    /// Defense pieces (wall lines, gates) — the blocked/unblocked matrix.
    Defense,
    /// Props/sprites — path-around, never path-through.
    Prop,
    /// Held items / armor — world-layer load check only (no arena cast).
    Item,
    /// Flora — load + path-around.
    Flora,
    /// Test fixtures (`test_*`) — used by the FAIL-pair; treated as Structure.
    TestFixture,
    /// Creature part sets — a LATER integration rung (Body/skeleton Rust work);
    /// cataloged so `--asset-test all` can report SKIP rather than silence.
    Creature,
    Other,
}

impl AssetCategory {
    fn infer(id: &str) -> Self {
        let id = id.to_ascii_lowercase();
        if id.starts_with("creature_") || id.starts_with("ref_") {
            Self::Creature
        } else if id.starts_with("test_") {
            Self::TestFixture
        } else if id.starts_with("structure_")
            || id.starts_with("castle_")
            || id.starts_with("monastery_")
            || id.starts_with("godspire_")
            || id.starts_with("temple_")
        {
            Self::Structure
        } else if id.starts_with("defense_") || id.starts_with("wall_") || id.starts_with("gate_") {
            Self::Defense
        } else if id.starts_with("prop_") || id.starts_with("sprite_") {
            Self::Prop
        } else if id.starts_with("item_") || id.starts_with("armor_") {
            Self::Item
        } else if id.starts_with("flora_") {
            Self::Flora
        } else {
            Self::Other
        }
    }
}

/// One scannable asset-lab entry (a flattened `.vox` in `<root>/vox/`).
#[derive(Clone, Debug)]
pub struct AssetLabEntry {
    pub id: String,
    pub vox_path: PathBuf,
    pub category: AssetCategory,
}

/// The scanned asset-lab catalog. Missing directories yield an empty catalog
/// (callers decide whether that is fatal); malformed files are skipped at
/// load, not scan.
pub struct AssetLabCatalog {
    pub root: PathBuf,
    pub entries: Vec<AssetLabEntry>,
}

impl AssetLabCatalog {
    pub fn scan(root: &Path) -> Self {
        let vox_dir = root.join("vox");
        let mut entries = Vec::new();
        match std::fs::read_dir(&vox_dir) {
            Ok(dir) => {
                for e in dir.flatten() {
                    let path = e.path();
                    if path.extension().is_some_and(|x| x == "vox")
                        && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    {
                        entries.push(AssetLabEntry {
                            id: stem.to_string(),
                            category: AssetCategory::infer(stem),
                            vox_path: path,
                        });
                    }
                }
                entries.sort_by(|a, b| a.id.cmp(&b.id));
                info!(count = entries.len(), ?vox_dir, "bastion asset-lab catalog scanned");
            },
            Err(e) => warn!(?vox_dir, ?e, "bastion asset-lab root not readable — empty catalog"),
        }
        Self { root: root.to_path_buf(), entries }
    }

    pub fn get(&self, id: &str) -> Option<&AssetLabEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

/// The bastion marker registry: gameplay-marker band byte → intended engine
/// `StructureBlock`. This table is the engine half of the asset session's
/// marker contract (known assignments from the asset-lab generators:
/// 200 = gate bars, 206 = pressure plate, 207/208/209 = desk/bench/bed).
///
/// `open_variant` swaps operable-closed markers to their open pose (byte 200
/// gate: `KeyholeBars` ↔ carved air — the dwarves/entrance.ron precedent).
/// Function-point markers (206–209) carve to air: the dynamic tests need the
/// CELLS to be walkable targets; visible furniture mapping is backlog.
pub fn marker_registry(open_variant: bool) -> HashMap<u8, StructureBlock> {
    let mut m = HashMap::default();
    m.insert(
        200u8,
        if open_variant {
            StructureBlock::Hollow
        } else {
            // The consumes string only feeds the (dropped) unlock SpriteCfg;
            // pathing solidity comes from the KeyholeBars sprite itself.
            StructureBlock::KeyholeBars("common.items.utility.lockpick_0".to_string())
        },
    );
    for b in [206u8, 207, 208, 209] {
        m.insert(b, StructureBlock::Hollow);
    }
    m
}

/// One marker-fidelity check result (per distinct byte present in the vox).
#[derive(Clone, Debug)]
pub struct MarkerCheck {
    pub byte: u8,
    pub count: usize,
    pub expected: String,
    pub resolved: String,
    pub ok: bool,
}

/// A loaded (not yet placed) asset-lab asset.
pub struct LoadedAsset {
    pub id: String,
    pub category: AssetCategory,
    pub structure: Structure,
    pub census: BastionVoxCensus,
    pub checks: Vec<MarkerCheck>,
    /// True iff every world-band and marker-band byte resolved to its intended
    /// StructureBlock through the real `Structure::get` path.
    pub fidelity_ok: bool,
}

fn sb_name(sb: &StructureBlock) -> String {
    // Variant-name-only debug (payloads vary per placement context).
    let full = format!("{sb:?}");
    full.split(['(', ' ']).next().unwrap_or(&full).to_string()
}

/// Load one asset through the real Structure path and run the marker-fidelity
/// gate. Never panics on malformed input — `Err` means log + skip (+ FAIL that
/// asset id in the harness).
pub fn load_asset(entry: &AssetLabEntry, open_variant: bool) -> Result<LoadedAsset, String> {
    let bytes = std::fs::read(&entry.vox_path)
        .map_err(|e| format!("read {}: {e}", entry.vox_path.display()))?;
    let registry = marker_registry(open_variant);

    // Center: None → footprint center at z=0 (ground-contact plane — the
    // asset-lab placement-anchor convention), computed common-side.
    let (structure, census) = Structure::bastion_from_vox_bytes(&bytes, None, &registry)?;

    // Marker fidelity: every world-band byte and every marker-band byte
    // present in the file must resolve to the intended StructureBlock through
    // the REAL lookup (guards the index-shift/welded-gate class).
    let mut checks = Vec::new();
    let mut fidelity_ok = true;
    for (&byte, &(sample, count)) in census.by_byte.iter() {
        let expected = if byte >= BASTION_MARKER_BAND_START {
            match registry.get(&byte) {
                Some(sb) => sb_name(sb),
                None => {
                    checks.push(MarkerCheck {
                        byte,
                        count,
                        expected: "UNKNOWN-MARKER (declare in marker_registry)".into(),
                        resolved: sb_name(structure.get(sample).unwrap_or(&StructureBlock::None)),
                        ok: false,
                    });
                    fidelity_ok = false;
                    continue;
                },
            }
        } else if (1..=16).contains(&byte) {
            // World-reserved band: defaults from default_custom_indices();
            // bytes 3/13 are unmapped there and fall through to literals.
            match byte {
                3 | 13 => continue,
                _ => String::new(), // resolved-variant check below via registry-less compare
            }
        } else {
            continue; // literal band — no semantic intent to assert
        };

        let resolved = structure.get(sample).unwrap_or(&StructureBlock::None);
        let resolved_name = sb_name(resolved);
        let ok = if byte >= BASTION_MARKER_BAND_START {
            resolved_name == expected
        } else {
            // World band: the intent is "not a literal" — a Filled(Misc, …)
            // here means the default mapping did not apply (index shift).
            !matches!(resolved, StructureBlock::Filled(kind, _) if *kind == common::terrain::BlockKind::Misc)
        };
        fidelity_ok &= ok;
        checks.push(MarkerCheck {
            byte,
            count,
            expected: if expected.is_empty() { "world-band default".into() } else { expected },
            resolved: resolved_name,
            ok,
        });
    }
    checks.sort_by_key(|c| c.byte);

    Ok(LoadedAsset {
        id: entry.id.clone(),
        category: entry.category,
        structure,
        census,
        checks,
        fidelity_ok,
    })
}

/// What actually got stamped into the world.
#[derive(Clone, Debug, Default)]
pub struct PlacementReport {
    pub blocks_placed: usize,
    /// SpriteCfg-carrying blocks placed WITHOUT their cfg (worldgen stores cfg
    /// in chunk meta; no runtime write path exists — see findings §1d/§7.2).
    pub sprite_cfgs_dropped: usize,
    /// EntitySpawner voxels skipped (world-layer scope; creatures are a later
    /// integration rung).
    pub entity_spawners_skipped: usize,
    /// World positions of gameplay-marker cells (byte → cells).
    pub marker_cells: HashMap<u8, Vec<Vec3<i32>>>,
    /// World-space bounds of the stamped structure.
    pub bounds: Aabb<i32>,
}

/// Stamp a loaded structure into live terrain at `origin` (world position of
/// the structure's center/ground anchor) through `block_from_structure` +
/// `State::set_block`. Identity rotation only (v1 — assets are tested
/// unrotated; the `units` rotation basis is a documented follow-up).
pub fn place_structure(
    state: &mut State,
    world: &World,
    index: IndexRef,
    loaded: &LoadedAsset,
    origin: Vec3<i32>,
    seed: u32,
) -> PlacementReport {
    let units: Vec2<Vec2<i32>> = Vec2::new(Vec2::unit_x(), Vec2::unit_y());
    let sampler = world.sample_columns();
    let bounds = loaded.structure.get_bounds();
    let mut report = PlacementReport {
        bounds: Aabb { min: origin + bounds.min, max: origin + bounds.max },
        ..Default::default()
    };

    for x in bounds.min.x..bounds.max.x {
        for y in bounds.min.y..bounds.max.y {
            let wpos2d = origin.xy() + Vec2::new(x, y);
            let Some(col) = sampler.get((wpos2d, index, None)) else {
                continue; // off-map column: nothing sensible to place against
            };
            for z in bounds.min.z..bounds.max.z {
                let spos = Vec3::new(x, y, z);
                let Ok(sblock) = loaded.structure.get(spos) else { continue };
                if matches!(sblock, StructureBlock::None) {
                    continue;
                }
                let wpos = origin + spos;
                let existing = state
                    .terrain()
                    .get(wpos)
                    .ok()
                    .copied()
                    .unwrap_or_else(Block::empty);
                if let Some((block, sprite_cfg, entity_path)) = block_from_structure(
                    index,
                    sblock,
                    wpos,
                    origin.xy(),
                    seed,
                    &col,
                    || existing.into_vacant(),
                    None,
                    &units,
                ) {
                    if entity_path.is_some() {
                        report.entity_spawners_skipped += 1;
                        // The spawner voxel itself still places (air), matching
                        // worldgen semantics minus the NPC.
                    }
                    if sprite_cfg.is_some() {
                        report.sprite_cfgs_dropped += 1;
                    }
                    state.set_block(wpos, block);
                    report.blocks_placed += 1;
                }
            }
        }
    }

    for (&byte, cells) in loaded.census.marker_cells.iter() {
        report
            .marker_cells
            .insert(byte, cells.iter().map(|c| origin + *c).collect());
    }

    if report.sprite_cfgs_dropped > 0 {
        warn!(
            asset = loaded.id,
            dropped = report.sprite_cfgs_dropped,
            "bastion asset placement: sprite cfgs dropped (no runtime chunk-meta write path)"
        );
    }
    info!(
        asset = loaded.id,
        blocks = report.blocks_placed,
        spawners_skipped = report.entity_spawners_skipped,
        "bastion asset placed"
    );
    report
}

/// Real-terrain-kind ground scan (the B5 canopy lesson — never `is_filled`,
/// which counts tree Wood/Leaves and returns canopy heights).
pub fn ground_z(state: &State, x: i32, y: i32) -> Option<i32> {
    use common::terrain::BlockKind;
    let terrain = state.terrain();
    (0..2048).rev().find(|z| {
        terrain.get(Vec3::new(x, y, *z)).is_ok_and(|b| {
            matches!(
                b.kind(),
                BlockKind::Rock
                    | BlockKind::WeakRock
                    | BlockKind::GlowingRock
                    | BlockKind::GlowingWeakRock
                    | BlockKind::Grass
                    | BlockKind::Snow
                    | BlockKind::ArtSnow
                    | BlockKind::Earth
                    | BlockKind::Sand
                    | BlockKind::Ice
            )
        })
    })
}

/// Guaranteed-flat rock slab + clear air above, centered on (cx, cy) at
/// `pad_z`. Writes are buffered `BlockChange`s (applied at tick end).
/// Returns the write count.
pub fn flatten_pad(state: &mut State, cx: i32, cy: i32, pad_z: i32, half: i32, clear_h: i32) -> usize {
    use common::terrain::BlockKind;
    let mut writes = 0usize;
    for x in (cx - half)..=(cx + half) {
        for y in (cy - half)..=(cy + half) {
            for dz in -1..=0 {
                state.set_block(
                    Vec3::new(x, y, pad_z + dz),
                    Block::new(BlockKind::Rock, Rgb::new(120, 120, 120)),
                );
                writes += 1;
            }
            for dz in 1..=clear_h {
                state.set_block(Vec3::new(x, y, pad_z + dz), Block::empty());
                writes += 1;
            }
        }
    }
    writes
}

/// Pick the flattest, driest candidate anchor on rings around `around`,
/// scored by interpolated-altitude range over a coarse footprint grid using
/// worldgen sim data (no chunks needed — probing real terrain would require
/// force-loading every candidate). Returns the best candidate center.
pub fn pick_flat_anchor(world: &World, around: Vec2<f32>) -> Vec2<f32> {
    let sim = world.sim();
    let mut best: Option<(f32, Vec2<f32>)> = None;
    for r in [160.0f32, 224.0, 288.0, 352.0] {
        for i in 0..8 {
            let ang = std::f32::consts::TAU * i as f32 / 8.0;
            let cand = around + Vec2::new(ang.cos(), ang.sin()) * r;
            let mut min_alt = f32::INFINITY;
            let mut max_alt = f32::NEG_INFINITY;
            let mut wet = false;
            for dx in -2..=2 {
                for dy in -2..=2 {
                    let p = (cand + Vec2::new(dx as f32, dy as f32) * 22.0).map(|e| e as i32);
                    match (
                        sim.get_interpolated(p, |c| c.alt),
                        sim.get_interpolated(p, |c| c.water_alt),
                    ) {
                        (Some(alt), Some(water_alt)) => {
                            min_alt = min_alt.min(alt);
                            max_alt = max_alt.max(alt);
                            if water_alt > alt {
                                wet = true;
                            }
                        },
                        _ => wet = true, // off-map: disqualify
                    }
                }
            }
            if wet {
                continue;
            }
            let range = max_alt - min_alt;
            if best.is_none_or(|(b, _)| range < b) {
                best = Some((range, cand));
            }
        }
    }
    let (range, cand) = best.unwrap_or((f32::INFINITY, around));
    info!(?cand, range, "bastion arena anchor picked (flattest dry candidate)");
    cand
}

/// Survey REAL terrain over the pad footprint after force-load: returns
/// (min, max) ground z across a sample grid, for adaptive pad sizing.
pub fn survey_pad(state: &State, cx: i32, cy: i32, half: i32) -> Option<(i32, i32)> {
    let mut min_gz = i32::MAX;
    let mut max_gz = i32::MIN;
    let step = (half / 4).max(1);
    for dx in (-half..=half).step_by(step as usize) {
        for dy in (-half..=half).step_by(step as usize) {
            let gz = ground_z(state, cx + dx, cy + dy)?;
            min_gz = min_gz.min(gz);
            max_gz = max_gz.max(gz);
        }
    }
    (min_gz <= max_gz).then_some((min_gz, max_gz))
}

/// Geometric interior target: walkable cell (solid below, non-solid feet +
/// head) inside `bounds`, maximizing distance from the bounds edge (≥ 3 so
/// ARRIVE_DIST 2.5 can't false-arrive through a wall). Scans z from `base_z`
/// to `base_z + 8` (raised floors / sills).
pub fn interior_target(state: &State, bounds: Aabb<i32>, base_z: i32) -> Option<Vec3<f32>> {
    let terrain = state.terrain();
    let b = bounds;
    let mut best: Option<(i32, Vec3<i32>)> = None;
    for x in b.min.x..b.max.x {
        for y in b.min.y..b.max.y {
            let edge_dist = (x - b.min.x)
                .min(b.max.x - 1 - x)
                .min(y - b.min.y)
                .min(b.max.y - 1 - y);
            if edge_dist < 3 {
                continue;
            }
            for z in base_z..(base_z + 8) {
                let below = terrain.get(Vec3::new(x, y, z)).ok().copied();
                let feet = terrain.get(Vec3::new(x, y, z + 1)).ok().copied();
                let head = terrain.get(Vec3::new(x, y, z + 2)).ok().copied();
                let walkable = below.is_some_and(|b| b.is_filled())
                    && feet.is_some_and(|b| !b.is_solid())
                    && head.is_some_and(|b| !b.is_solid());
                if walkable && best.is_none_or(|(d, _)| edge_dist > d) {
                    best = Some((edge_dist, Vec3::new(x, y, z + 1)));
                }
            }
        }
    }
    best.map(|(_, p)| p.map(|e| e as f32) + Vec3::new(0.5, 0.5, 0.0))
}
