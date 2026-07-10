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

/// The test cast declared per asset in catalog.json (drives scenario
/// derivation — ASSET_DYNAMIC_TEST_SPEC's per-category minimal cast).
#[derive(Clone, Debug, serde::Deserialize)]
pub struct CastSpec {
    #[serde(default)]
    pub colonists: u8,
    #[serde(default)]
    pub hostiles: u8,
    pub target: String,
}

/// One scannable asset-lab entry. From `vox/real/catalog.json` when present
/// (the pilot's machine-readable contract: category + cast + authored marker
/// cells + optional `<id>.ron` custom_indices sidecar); legacy `vox/*.vox`
/// prefix-inference otherwise.
#[derive(Clone, Debug)]
pub struct AssetLabEntry {
    pub id: String,
    pub vox_path: PathBuf,
    pub category: AssetCategory,
    /// Raw catalog category string ("housing"/"production"/…); empty on the
    /// legacy path.
    pub category_raw: String,
    pub cast: Option<CastSpec>,
    /// Model dims from the catalog (scale sanity: world-layer is 1 vox = 1
    /// block; figure-layer props are 11 vox/block).
    pub dims: Option<Vec3<i32>>,
    /// Authored marker cells (MODEL space, from catalog.json) — the exact-cell
    /// fidelity input.
    pub authored_markers: HashMap<u8, Vec<Vec3<i32>>>,
    /// Parsed `<id>.ron` custom_indices sidecar (overrides the built-in
    /// registry where present).
    pub ron_indices: HashMap<u8, StructureBlock>,
    /// A sidecar existed but failed to parse — reported as a fidelity finding
    /// (the pilot fixes the RON; loading falls back to the registry).
    pub ron_error: Option<String>,
}

#[derive(serde::Deserialize)]
struct CatalogFile {
    assets: HashMap<String, CatalogAsset>,
}

#[derive(serde::Deserialize)]
struct CatalogAsset {
    vox: String,
    category: String,
    #[serde(default)]
    dims: Option<[i32; 3]>,
    #[serde(default)]
    cast: Option<CastSpec>,
    #[serde(default)]
    markers: Option<HashMap<String, Vec<[i32; 3]>>>,
}

#[derive(serde::Deserialize)]
struct RonSidecar {
    custom_indices: HashMap<u8, StructureBlock>,
}

fn category_from_str(s: &str) -> AssetCategory {
    match s {
        "defense" => AssetCategory::Defense,
        "flora" => AssetCategory::Flora,
        "prop" => AssetCategory::Prop,
        "item" => AssetCategory::Item,
        "housing" | "production" | "social" | "storage" | "dungeon-room" => {
            AssetCategory::Structure
        },
        _ => AssetCategory::Other,
    }
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
        let catalog_path = root.join("vox").join("real").join("catalog.json");
        let mut entries = Vec::new();
        if catalog_path.is_file() {
            match std::fs::read_to_string(&catalog_path)
                .map_err(|e| e.to_string())
                .and_then(|s| serde_json::from_str::<CatalogFile>(&s).map_err(|e| e.to_string()))
            {
                Ok(cat) => {
                    for (id, a) in cat.assets {
                        // Catalog vox paths are repo-root-relative
                        // ("asset-lab/vox/real/x.vox") — resolve against the
                        // asset-lab root by stripping its leading component.
                        let rel = a.vox.strip_prefix("asset-lab/").unwrap_or(&a.vox);
                        let vox_path = root.join(rel);
                        let ron_path = vox_path.with_extension("ron");
                        let (ron_indices, ron_error) = if ron_path.is_file() {
                            match std::fs::read_to_string(&ron_path)
                                .map_err(|e| e.to_string())
                                .and_then(|s| {
                                    ron::from_str::<RonSidecar>(&s).map_err(|e| e.to_string())
                                }) {
                                Ok(sc) => (sc.custom_indices, None),
                                Err(e) => {
                                    warn!(id, e, "bastion asset sidecar RON parse failed");
                                    (HashMap::default(), Some(e))
                                },
                            }
                        } else {
                            (HashMap::default(), None)
                        };
                        let authored_markers = a
                            .markers
                            .unwrap_or_default()
                            .into_iter()
                            .filter_map(|(k, cells)| {
                                let byte: u8 = k.parse().ok()?;
                                Some((
                                    byte,
                                    cells
                                        .into_iter()
                                        .map(|c| Vec3::new(c[0], c[1], c[2]))
                                        .collect(),
                                ))
                            })
                            .collect();
                        entries.push(AssetLabEntry {
                            category: category_from_str(&a.category),
                            category_raw: a.category,
                            cast: a.cast,
                            dims: a.dims.map(|d| Vec3::new(d[0], d[1], d[2])),
                            authored_markers,
                            ron_indices,
                            ron_error,
                            id,
                            vox_path,
                        });
                    }
                    entries.sort_by(|a, b| a.id.cmp(&b.id));
                    info!(
                        count = entries.len(),
                        ?catalog_path,
                        "bastion asset-lab catalog loaded (contract v2)"
                    );
                },
                Err(e) => warn!(?catalog_path, e, "catalog.json unreadable — falling back"),
            }
        }
        if entries.is_empty() {
            // Legacy path: flattened .vox directly under vox/, prefix-inferred.
            let vox_dir = root.join("vox");
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
                                category_raw: String::new(),
                                cast: None,
                                dims: None,
                                authored_markers: HashMap::default(),
                                ron_indices: HashMap::default(),
                                ron_error: None,
                                vox_path: path,
                            });
                        }
                    }
                    entries.sort_by(|a, b| a.id.cmp(&b.id));
                    info!(count = entries.len(), ?vox_dir, "bastion asset-lab catalog scanned (legacy)");
                },
                Err(e) => {
                    warn!(?vox_dir, ?e, "bastion asset-lab root not readable — empty catalog")
                },
            }
        }
        Self { root: root.to_path_buf(), entries }
    }

    pub fn get(&self, id: &str) -> Option<&AssetLabEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

/// The bastion marker registry: gameplay-marker band byte → intended engine
/// `StructureBlock`. This MIRRORS `readme/ASSET_MARKER_REGISTRY.md` (the ONE
/// authority, bytes 200–219; append new rows there first, then here). Kept as
/// RON strings so the table reads like the doc and parses through the exact
/// codepath asset `.ron` sidecars use — per-asset sidecars override this
/// fallback where present.
///
/// `open_variant` swaps the operable-closed gate byte (200) to carved air —
/// poses as mappings until the operable-state machine block (DF-MECH) lands.
pub fn marker_registry(open_variant: bool) -> HashMap<u8, StructureBlock> {
    const TABLE: &[(u8, &str)] = &[
        (200, "Sprite(DoorBars())"), // operable closed gate leaf (solid to A*)
        (201, "Sprite(Anvil())"),    // smithy anvil work point
        (202, "Sprite(Forge())"),    // smithy forge/hearth work point
        (204, "Sprite(FireBowlGround())"), // brazier / fire light point
        (206, "Filled(Rock, (r: 90, g: 90, b: 70))"), // trap trigger plate (inert until DF-MECH)
        (207, "Sprite(DiningtableWoodWoodlandSquare())"), // desk / study point
        (208, "Sprite(BenchWoodEnd())"), // bench / sit point
        (209, "Sprite(Bedroll())"),  // bed / sleep point
        (210, "Sprite(CraftingBench())"), // carpenter work point
        (211, "Sprite(CraftingBench())"), // mason work point
        (212, "Sprite(Forge())"),    // smelter furnace work point
        (213, "Sprite(TanningRack())"), // tannery work point
        (214, "Filled(Rock, (r: 60, g: 60, b: 50))"), // tripwire line (inert until DF-MECH)
        (215, "Sprite(CookingPot())"), // cooking/kitchen work point
        (216, "Sprite(Loom())"),     // loom/weaving work point
        (217, "Filled(GlowingRock, (r: 0, g: 201, b: 177))"), // glow crystal (gnarling convention)
        (218, "Filled(Rock, (r: 200, g: 190, b: 160))"), // worship point (REL-0 upgrades)
        (219, "Sprite(CraftingBench())"), // trade depot drop/work point
    ];
    let mut m = HashMap::default();
    for (b, s) in TABLE {
        match ron::from_str::<StructureBlock>(s) {
            Ok(sb) => {
                m.insert(*b, sb);
            },
            Err(e) => warn!(
                byte = b,
                target = s,
                ?e,
                "marker registry entry failed to parse — fix the TABLE to match \
                 readme/ASSET_MARKER_REGISTRY.md"
            ),
        }
    }
    if open_variant {
        m.insert(200, StructureBlock::Hollow);
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

    // Effective custom-index table: built-in registry ← per-asset RON sidecar
    // (sidecar wins), then the open-variant pose override on the gate byte.
    let mut custom = marker_registry(open_variant);
    for (b, sb) in entry.ron_indices.iter() {
        custom.insert(*b, sb.clone());
    }
    if open_variant {
        custom.insert(200, StructureBlock::Hollow);
    }

    // Center: None → footprint center at z=0 (ground-contact plane — the
    // asset-lab placement-anchor convention), computed common-side.
    let (structure, census) = Structure::bastion_from_vox_bytes(&bytes, None, &custom)?;

    // Marker fidelity: every world-band byte and every marker-band byte
    // present in the file must resolve to the intended StructureBlock through
    // the REAL lookup (guards the index-shift/welded-gate class).
    let mut checks = Vec::new();
    let mut fidelity_ok = true;

    // A sidecar that exists but does not parse is itself a fidelity finding
    // (the pilot fixes the RON; loading proceeded on the registry).
    if let Some(e) = &entry.ron_error {
        checks.push(MarkerCheck {
            byte: 0,
            count: 0,
            expected: "parseable .ron custom_indices sidecar".into(),
            resolved: format!("PARSE ERROR: {e}"),
            ok: false,
        });
        fidelity_ok = false;
    }

    for (&byte, &(sample, count)) in census.by_byte.iter() {
        let expected = if byte >= BASTION_MARKER_BAND_START {
            match custom.get(&byte) {
                Some(sb) => sb_name(sb),
                None => {
                    checks.push(MarkerCheck {
                        byte,
                        count,
                        expected: "UNKNOWN-MARKER (declare in sidecar .ron or marker_registry)"
                            .into(),
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
            // A sidecar may override world-band bytes too (e.g. 8 → carve).
            match (byte, entry.ron_indices.get(&byte)) {
                (_, Some(sb)) => sb_name(sb),
                (3 | 13, None) => continue,
                _ => String::new(), // resolved-variant check below via registry-less compare
            }
        } else if let Some(sb) = entry.ron_indices.get(&byte) {
            // Sidecar-declared literal-band byte (e.g. quarry hall 136 →
            // Sprite(Lantern)) — assert like a marker.
            sb_name(sb)
        } else {
            continue; // literal band — no semantic intent to assert
        };

        let resolved = structure.get(sample).unwrap_or(&StructureBlock::None);
        let resolved_name = sb_name(resolved);
        let ok = if expected.is_empty() {
            // World band: the intent is "not a literal" — a Filled(Misc, …)
            // here means the default mapping did not apply (index shift).
            !matches!(resolved, StructureBlock::Filled(kind, _) if *kind == common::terrain::BlockKind::Misc)
        } else {
            resolved_name == expected
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

    // Exact-cell fidelity: authored marker cells (catalog.json, MODEL space)
    // must match the census cells (structure space = model − center) — the
    // full welded-gate guard: not just "the byte resolves right" but "every
    // authored cell is where the author put it".
    if !entry.authored_markers.is_empty() {
        let center = -structure.get_bounds().min; // model→structure offset
        for (byte, authored) in entry.authored_markers.iter() {
            if census.cells_truncated.contains(byte) {
                // Too many cells to compare exactly — count check only.
                let count = census.by_byte.get(byte).map_or(0, |&(_, c)| c);
                let ok = count == authored.len();
                fidelity_ok &= ok;
                checks.push(MarkerCheck {
                    byte: *byte,
                    count,
                    expected: format!("{} authored cells (count-only; truncated)", authored.len()),
                    resolved: format!("{count} cells"),
                    ok,
                });
                continue;
            }
            let mut want: Vec<Vec3<i32>> = authored.iter().map(|c| *c - center).collect();
            let mut have: Vec<Vec3<i32>> =
                census.marker_cells.get(byte).cloned().unwrap_or_default();
            want.sort_by_key(|v| (v.z, v.y, v.x));
            have.sort_by_key(|v| (v.z, v.y, v.x));
            let ok = want == have;
            fidelity_ok &= ok;
            checks.push(MarkerCheck {
                byte: *byte,
                count: have.len(),
                expected: format!("{} authored cells (exact)", want.len()),
                resolved: if ok {
                    "all cells match".into()
                } else {
                    format!(
                        "{} cells present, {} missing, {} unexpected",
                        have.len(),
                        want.iter().filter(|c| !have.contains(c)).count(),
                        have.iter().filter(|c| !want.contains(c)).count()
                    )
                },
                ok,
            });
        }
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
