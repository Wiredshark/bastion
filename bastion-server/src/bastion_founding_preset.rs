//! bastion (FOUNDING PRESET v1, ITEM-FOUNDING-PRESET-PACKET.md): the
//! survival starter kit the PLAYER path places, so a UI founding gets the
//! same certified kit `script-15` gave every scored run.
//!
//! WHY THIS EXISTS: the harness path works and the player path was never
//! exercised. Ben founded a colony through the in-game action and the
//! colonists marched cross-country — because a founding placed no
//! designations, and what holds a colonist near F is THE WORK BEING AT F
//! (there is no colonist anchor primitive: the spawn sets only
//! `npc.home = nearest site`, and activity zones register for
//! `DesignationKind::Zone` alone). This module is that fix's data half.
//!
//! PLOT TEMPLATE, NOT FREE POSITIONS (PACKET-CRAFT-CHECKLIST entry 2 /
//! DECISIONS #102): the preset is a table of ROLE-TAGGED elements with
//! offsets as their data — not positions computed at a call site. The
//! placement code below reads this table and nothing else, so the first
//! plot template and the first plot CONSUMER land together.
//!
//! THE DATUM (packet §8 B1 — the one-block error that sinks the preset):
//! every offset is relative to the FIRST AIR CELL at F's own column,
//! DERIVED from terrain via [`column_surface_z`] — never taken from the
//! reported founding z. The god's reported z is legitimately ±1 (the flat
//! arena's slab has its first air at `FLAT_ARENA_Z` while the spawn point
//! is `FLAT_ARENA_Z + 1`, "+1 clears any landing jitter"), and one block
//! either way puts the stockpile floor inside solid ground or hanging.
//!
//! THE ARITHMETIC SELF-CHECK (packet §8, carried as a test below):
//! against script-15's anchor `F = (15216.5, 16016.5, 419.0)` — verified
//! from six driver logs — this table reproduces all twelve of that
//! script's designation numbers exactly. If a change to this file breaks
//! `preset_reproduces_script_15_absolute_numbers`, the change is wrong,
//! not the test.
//!
//! NO FOUNDING FOOD (packet §8 B3): the founding stock is seeds only
//! (`FOUNDING_SEED_STOCK`, the fixed live path from #105) — script-15's
//! own header says "NO give_item" verbatim, and every certified run ate
//! off the farm it planted. A food grant would be new work with an
//! underived quantity; it is deliberately absent here.

use crate::bastion_jobs::column_surface_z;
use common::{
    bastion::{DesignationKind, Region},
    terrain::TerrainGrid,
    // `submerged` is the first terrain read this module makes on its own —
    // every other lookup goes through `column_surface_z`.
    vol::ReadVol,
};
use vek::*;

/// The preset's version string — carried on the live witness line so a
/// scored run names the kit it actually got.
pub const PRESET_VERSION: &str = "v1";

/// How far a preset column's own first-air cell may deviate from F's datum
/// before the site is refused as uneven (packet §3: "standable within ±1
/// z"). A column outside this shifts nothing silently — the whole founding
/// refuses, by name.
pub const MAX_DATUM_DEVIATION: i32 = 1;

/// bastion (FOUNDING PRESET v1): the role each preset element plays in the
/// survival loop. Roles — not indices — are what the witness line reports
/// and what the completeness check counts, so a PARTIAL preset is visible
/// as a missing NAME rather than a smaller number (packet §8 B5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetRole {
    /// The haul destination — makes the founding seed drop reachable.
    Stockpile,
    /// The food engine. Without it the colony has seeds and no plot.
    Farm,
    /// Rest service (proven in script-10's own leg).
    Bed,
}

impl PresetRole {
    pub fn name(self) -> &'static str {
        match self {
            PresetRole::Stockpile => "stockpile",
            PresetRole::Farm => "farm",
            PresetRole::Bed => "bed",
        }
    }
}

/// One plot in the founding template: a role, the designation kind that
/// implements it, and its offsets from the founding origin.
#[derive(Debug, Clone, Copy)]
pub struct PresetElement {
    pub role: PresetRole,
    pub kind: DesignationKind,
    /// Offset of the region's min corner from `(floor F.x, floor F.y,
    /// datum)` — see the module doc on the datum.
    pub min_off: Vec3<i32>,
    /// Offset of the region's max corner, same origin.
    pub max_off: Vec3<i32>,
}

/// THE PRESET (packet §1): script-15's proven kit, made relative.
///
/// Source of truth is `script-15-item8-endurance.txt`, whose absolute
/// coordinates were proven across v3/v4/v5. The stockpile/farm footprints
/// match script-14 exactly (proven self-sustaining there) and the bed
/// footprint matches script-10 exactly (proven rest interrupt/service).
///
/// The farm's z offsets are a PAINT-PLANE HINT, not an extent (packet §8
/// B2): `DesignationKind::Farm` is `Area2D` — it never carries a `ZExtent`,
/// and its `region.min.z` is consumed as the hint into each column's own
/// surface resolution at registration. It is kept here at the same
/// `datum - 1` the other elements use so the hint lands ON the resolved
/// surface, which is exactly what script-15 sent.
pub const FOUNDING_PRESET_V1: &[PresetElement] = &[
    PresetElement {
        role: PresetRole::Stockpile,
        kind: DesignationKind::Stockpile,
        min_off: Vec3::new(-2, -4, -1),
        max_off: Vec3::new(2, 1, 0),
    },
    PresetElement {
        role: PresetRole::Farm,
        kind: DesignationKind::Farm,
        min_off: Vec3::new(-7, -4, -1),
        max_off: Vec3::new(-3, 1, 0),
    },
    PresetElement {
        role: PresetRole::Bed,
        kind: DesignationKind::Bed,
        min_off: Vec3::new(-3, -3, 0),
        max_off: Vec3::new(-2, -2, 1),
    },
];

/// The bed plot's sleeping capacity, DERIVED from the preset table rather
/// than restated. Consumed by the bar that keeps
/// [`common::bastion::FOUNDING_COLONIST_COUNT`] honest: asserting two
/// literals against each other would merely re-encode the drift it exists to
/// catch.
pub fn bed_capacity() -> usize {
    FOUNDING_PRESET_V1
        .iter()
        .filter(|element| element.role == PresetRole::Bed)
        .map(|element| {
            let span = |min: i32, max: i32| (max - min + 1).max(0) as usize;
            span(element.min_off.x, element.max_off.x)
                * span(element.min_off.y, element.max_off.y)
                * span(element.min_off.z, element.max_off.z)
        })
        .sum()
}

/// Why a founding was refused. A refusal is a FIRST-CLASS OUTCOME with its
/// own name on both channels (packet §3.2 / §4): the log carries
/// `reason()`, the player sees [`Self::player_message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundingRefusal {
    /// v1 is ONE COLONY PER WORLD (packet §4). Relocation and multiple
    /// colonies are deferred WITH rows (settlement arc / multi-colony at
    /// the needs-in-rtsim horizon).
    ColonyExists,
    /// The site cannot carry the preset (packet §3.2): some column's
    /// standable surface deviates by more than [`MAX_DATUM_DEVIATION`], or
    /// has no resolvable surface at all (open water, void, unloaded).
    Terrain,
    /// A plot column stands under WATER. Its own refusal because
    /// `reason="terrain"` cannot be told from slope in a log, and because
    /// the two have nothing to do with each other: a lakebed can be
    /// perfectly flat and still be the wrong place to put a farm.
    ///
    /// The gap this closes was masked by a correlate — a lake is usually a
    /// depression, so the deviation test refused the site for its SHAPE and
    /// the missing water test never showed.
    Submerged,
}

impl FoundingRefusal {
    /// The log-side name — `bastion: founding refused reason=<name>`.
    /// Refusal-needs-refusal-aware-consumers: this string IS the bar A4/A5
    /// read, so it is a stable identifier, not prose.
    pub fn reason(self) -> &'static str {
        match self {
            FoundingRefusal::ColonyExists => "colony_exists",
            FoundingRefusal::Terrain => "terrain",
            FoundingRefusal::Submerged => "submerged",
        }
    }

    /// The player-side message. Names the reason in the owner's own terms
    /// and, for the boundary, says what IS coming.
    pub fn player_message(self) -> &'static str {
        match self {
            FoundingRefusal::ColonyExists => {
                "Your colony already lives in this world — relocation and multiple colonies are \
                 future features."
            },
            FoundingRefusal::Terrain => {
                "Uneven ground — the founding kit needs a flatter site (every plot column must sit \
                 within one block of where you stand)."
            },
            FoundingRefusal::Submerged => {
                "Underwater — part of the founding kit would stand in water. Move to dry ground."
            },
        }
    }
}

/// The founding origin's XY: the block column the god targeted.
pub fn origin_xy(founding_pos: Vec3<f32>) -> Vec2<i32> {
    Vec2::new(
        founding_pos.x.floor() as i32,
        founding_pos.y.floor() as i32,
    )
}

/// THE DATUM (packet §8 B1): the first air cell above F's own resolved
/// surface. `column_surface_z` returns the topmost REAL TERRAIN block, so
/// the standable cell — the one the god's feet occupy — is that `+ 1`.
///
/// `hint_z` is the reported founding z, used ONLY to centre the resolver's
/// ±window; it never becomes the datum itself. That distinction is the
/// whole of B1: on the flat arena the reported z is 401 while the datum is
/// 400, and taking the report would sink every plot one block.
pub fn resolve_datum(terrain: &TerrainGrid, origin: Vec2<i32>, hint_z: i32) -> Option<i32> {
    column_surface_z(terrain, origin.x, origin.y, hint_z).map(|surface| surface + 1)
}

/// The absolute region for one element, given the resolved origin
/// `(x, y, datum)`.
pub fn element_region(element: &PresetElement, origin: Vec3<i32>) -> Region {
    Region {
        min: origin + element.min_off,
        max: origin + element.max_off,
    }
}

/// The whole template, resolved to absolute regions. This is what the
/// placement site iterates — it never computes a position of its own.
pub fn preset_regions(origin: Vec3<i32>) -> Vec<(PresetRole, DesignationKind, Region)> {
    FOUNDING_PRESET_V1
        .iter()
        .map(|element| {
            (
                element.role,
                element.kind,
                element_region(element, origin),
            )
        })
        .collect()
}

/// Every column the preset touches, deduped — the site-validation
/// footprint. Union across elements: a founding is refused if ANY plot
/// column is unstandable, not merely if the centre is.
pub fn footprint_columns(origin: Vec3<i32>) -> Vec<Vec2<i32>> {
    let mut columns = Vec::new();
    for (_, _, region) in preset_regions(origin) {
        for y in region.min.y..=region.max.y {
            for x in region.min.x..=region.max.x {
                let column = Vec2::new(x, y);
                if !columns.contains(&column) {
                    columns.push(column);
                }
            }
        }
    }
    columns
}

/// TERRAIN VALIDATION (packet §3.2, and the reuse N6 requires): every
/// preset column must resolve a surface whose own first-air cell sits
/// within [`MAX_DATUM_DEVIATION`] of the datum. Resolution goes through
/// [`column_surface_z`] — the ±window authority Farm registration and
/// relative-mode designations already use — rather than a second
/// standability rule written for this feature.
///
/// Returns the offending column on refusal so the emit can name WHERE, not
/// merely that something was uneven.
/// WHICH TEST refused a site. `reason()` is `"terrain"` for both, so the
/// log alone cannot separate "the ground is the wrong HEIGHT" from "there is
/// no ground here at all" — and the worldgen row's water prediction is
/// precisely a claim about which of the two fires. This names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliefBranch {
    /// Every column resolved and every deviation was within bounds.
    Ok,
    /// A column resolved a surface, but too far from the datum
    /// ([`MAX_DATUM_DEVIATION`]). Water sites reach here via the LAKEBED:
    /// `is_surface_terrain` does not match `Water`, and the scan reaches
    /// `SURFACE_SCAN_DOWN` blocks below the hint.
    Deviation,
    /// A column resolved NO surface in the scan window at all — open void,
    /// an unloaded chunk, or water deeper than the downward scan.
    Absence,
    /// Every column resolved and sat within bounds, but at least one stands
    /// under water. The flat-lakebed case: nothing about the SHAPE refuses
    /// it, which is exactly why it needs its own name.
    Submerged,
}

impl ReliefBranch {
    /// The log-side name. A stable identifier, not prose — the worldgen
    /// bars read this string.
    pub fn name(self) -> &'static str {
        match self {
            ReliefBranch::Ok => "ok",
            ReliefBranch::Deviation => "deviation",
            ReliefBranch::Absence => "absence",
            ReliefBranch::Submerged => "submerged",
        }
    }
}

/// THE MEASURED SHAPE of a candidate site, over every footprint column.
///
/// This is the SINGLE PRODUCER: [`validate_site`] does not re-derive any of
/// it, it delegates to [`survey_site`] and reads the verdict off this value.
/// The diagnostic emit reads the SAME value from the SAME call. Writing a
/// second function that recomputes relief alongside the real one is the F8
/// defect — a test that re-implements its subject cannot witness its
/// subject's failure — so there is deliberately only one.
#[derive(Debug, Clone)]
pub struct SiteRelief {
    /// Datum the site was measured against (`origin.z`).
    pub datum: i32,
    /// How many columns the preset occupies. Derived, not asserted: the
    /// union of the element footprints. For v1 this is 60.
    pub columns: usize,
    /// How many of those resolved a surface at all. `resolved < columns`
    /// means some column hit [`ReliefBranch::Absence`] — which is how a
    /// chunk-boundary hole would announce itself.
    pub resolved: usize,
    /// Smallest and largest signed deviation `surface + 1 - datum` over the
    /// RESOLVED columns. `None` when nothing resolved.
    pub min_dev: Option<i32>,
    pub max_dev: Option<i32>,
    /// The column with the largest absolute deviation, and its deviation.
    pub worst: Option<(Vec2<i32>, i32)>,
    /// How many resolved columns carry LIQUID directly above their surface.
    ///
    /// Without this the relief emit cannot see water at all: `is_surface_terrain`
    /// skips `Water`, so a lake column resolves its LAKEBED and reports as a
    /// large deviation — indistinguishable from a cliff by every other field
    /// here. The worldgen row's water bar is a claim about which branch
    /// refuses a submerged site, and that claim is only checkable if
    /// "submerged" is observable separately from "far below the datum".
    pub submerged: usize,
    /// The first submerged column in footprint order — the one the refusal
    /// names. A count alone cannot tell the owner WHERE the water is.
    pub first_submerged: Option<Vec2<i32>>,
    /// Per-column results in `footprint_columns` order, so the verdict can
    /// reproduce the original early-return EXACTLY: the refusing column is
    /// the first offender in iteration order, not merely some offender.
    columns_scanned: Vec<(Vec2<i32>, Option<i32>)>,
}

impl SiteRelief {
    /// The verdict, read off the measurement. Semantically identical to the
    /// original early-returning loop: it walks columns in the same order and
    /// reports the FIRST offender, so the named column does not move.
    pub fn verdict(&self) -> Result<(), (FoundingRefusal, Vec2<i32>)> {
        for (column, surface) in &self.columns_scanned {
            match surface {
                Some(surface) => {
                    if (surface + 1 - self.datum).abs() > MAX_DATUM_DEVIATION {
                        return Err((FoundingRefusal::Terrain, *column));
                    }
                },
                None => return Err((FoundingRefusal::Terrain, *column)),
            }
        }
        // THE WATER GATE, deliberately AFTER the deviation test. A site that
        // is both sloped and submerged still reports `terrain`: that refusal
        // is older, cheaper and more common, and changing an existing
        // refusal's reason would break the bars already reading it.
        //
        // Any single submerged column refuses. The preset's three elements
        // are SURFACE structures — there is no partially-submerged design —
        // so 1 is the only threshold derivable from "these plots sit on the
        // ground"; anything between 2 and 60 would be invented.
        if let Some(column) = self.first_submerged {
            return Err((FoundingRefusal::Submerged, column));
        }
        Ok(())
    }

    /// Which test decided it — see [`ReliefBranch`].
    pub fn branch(&self) -> ReliefBranch {
        for (_, surface) in &self.columns_scanned {
            match surface {
                Some(surface) => {
                    if (surface + 1 - self.datum).abs() > MAX_DATUM_DEVIATION {
                        return ReliefBranch::Deviation;
                    }
                },
                None => return ReliefBranch::Absence,
            }
        }
        if self.first_submerged.is_some() {
            return ReliefBranch::Submerged;
        }
        ReliefBranch::Ok
    }
}

/// MEASURE a candidate site over every footprint column — the producer both
/// [`validate_site`] and the founding diagnostic consume.
///
/// Unlike the decision it feeds, this does NOT stop at the first offending
/// column: a refusal that says only "terrain" and names one column cannot
/// distinguish a 2-block deviation from a 90-block one, nor slope from open
/// water. Scanning all of them costs 60 column lookups on a founding attempt,
/// which happens once per world.
pub fn survey_site(terrain: &TerrainGrid, origin: Vec3<i32>) -> SiteRelief {
    let mut columns_scanned = Vec::new();
    for column in footprint_columns(origin) {
        // Hint at the datum's own surface (datum - 1): the window is
        // centred where the preset EXPECTS terrain, so a column that
        // deviates reads as a deviation rather than as a miss.
        let surface = column_surface_z(terrain, column.x, column.y, origin.z - 1);
        columns_scanned.push((column, surface));
    }

    let mut resolved = 0;
    let mut submerged = 0;
    let mut first_submerged: Option<Vec2<i32>> = None;
    let mut min_dev: Option<i32> = None;
    let mut max_dev: Option<i32> = None;
    let mut worst: Option<(Vec2<i32>, i32)> = None;
    for (column, surface) in &columns_scanned {
        let Some(surface) = surface else { continue };
        resolved += 1;
        // The cell directly above the resolved surface. On a lake column
        // that surface is the BED, so this is the water sitting on it.
        if terrain
            .get(Vec3::new(column.x, column.y, surface + 1))
            .is_ok_and(|block| block.is_liquid())
        {
            submerged += 1;
            if first_submerged.is_none() {
                first_submerged = Some(*column);
            }
        }
        let dev = surface + 1 - origin.z;
        min_dev = Some(min_dev.map_or(dev, |m: i32| m.min(dev)));
        max_dev = Some(max_dev.map_or(dev, |m: i32| m.max(dev)));
        if worst.is_none_or(|(_, w)| dev.abs() > w.abs()) {
            worst = Some((*column, dev));
        }
    }

    SiteRelief {
        datum: origin.z,
        columns: columns_scanned.len(),
        resolved,
        min_dev,
        max_dev,
        worst,
        submerged,
        first_submerged,
        columns_scanned,
    }
}

pub fn validate_site(
    terrain: &TerrainGrid,
    origin: Vec3<i32>,
) -> Result<(), (FoundingRefusal, Vec2<i32>)> {
    survey_site(terrain, origin).verdict()
}

/// A1's DISCRIMINATOR (packet §8 B5): does a founding's placed-role set
/// carry the WHOLE preset?
///
/// The planted failure for A1 is a PARTIAL preset — not a disabled one —
/// so the witness must separate "full" from "farm missing". Counting is
/// not enough on its own; this checks every template role is present, so
/// dropping any single element turns the acceptance red by name.
pub fn preset_is_complete(placed: &[PresetRole]) -> bool {
    FOUNDING_PRESET_V1
        .iter()
        .all(|element| placed.contains(&element.role))
}

/// The roles, in template order, as a log-ready list — the witness line's
/// completeness evidence (`elements=stockpile,farm,bed`).
pub fn roles_summary(placed: &[PresetRole]) -> String {
    placed
        .iter()
        .map(|role| role.name())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// script-15's anchor, verified from six driver logs (`sent
    /// BastionSpawnColony pos=Vec3 { x: 15216.5, y: 16016.5, z: 419.0 }`).
    const SCRIPT_15_F: Vec3<f32> = Vec3::new(15216.5, 16016.5, 419.0);
    /// The datum that world resolves to: surface 418 (solid), first air 419.
    const SCRIPT_15_DATUM: i32 = 419;

    fn script_15_origin() -> Vec3<i32> {
        let xy = origin_xy(SCRIPT_15_F);
        Vec3::new(xy.x, xy.y, SCRIPT_15_DATUM)
    }

    /// THE ARITHMETIC SELF-CHECK the packet's §8 review demanded be carried
    /// in the build: the relative table must reproduce script-15's ABSOLUTE
    /// numbers exactly — all twelve of them, the coordinates proven across
    /// v3/v4/v5.
    ///
    ///     designate stockpile 15214 16012 418 15218 16017 419
    ///     designate farm      15209 16012 418 15213 16017 419
    ///     designate bed       15213 16013 419 15214 16014 420
    #[test]
    fn preset_reproduces_script_15_absolute_numbers() {
        let regions = preset_regions(script_15_origin());
        assert_eq!(regions.len(), 3, "the template is three plots");

        let stockpile = regions
            .iter()
            .find(|(role, ..)| *role == PresetRole::Stockpile)
            .expect("stockpile is in the template");
        assert_eq!(stockpile.2.min, Vec3::new(15214, 16012, 418));
        assert_eq!(stockpile.2.max, Vec3::new(15218, 16017, 419));

        let farm = regions
            .iter()
            .find(|(role, ..)| *role == PresetRole::Farm)
            .expect("farm is in the template");
        assert_eq!(farm.2.min, Vec3::new(15209, 16012, 418));
        assert_eq!(farm.2.max, Vec3::new(15213, 16017, 419));

        let bed = regions
            .iter()
            .find(|(role, ..)| *role == PresetRole::Bed)
            .expect("bed is in the template");
        assert_eq!(bed.2.min, Vec3::new(15213, 16013, 419));
        assert_eq!(bed.2.max, Vec3::new(15214, 16014, 420));
    }

    /// A flat test world whose first air cell is `first_air` — the flat
    /// arena's own construction (`TerrainChunk::new` makes everything BELOW
    /// `z` solid), built here so the datum tests drive the REAL resolver
    /// over REAL terrain instead of restating arithmetic.
    fn flat_world(first_air: i32, keys: &[Vec2<i32>]) -> TerrainGrid {
        use common::{
            terrain::{
                Block, BlockKind, MapSizeLg, SpriteKind, TerrainChunk, TerrainChunkMeta,
            },
            volumes::vol_grid_2d::VolGrid2d,
        };
        use std::sync::Arc;

        let chunk = || {
            Arc::new(TerrainChunk::new(
                first_air,
                Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
                Block::air(SpriteKind::Empty),
                TerrainChunkMeta::void(),
            ))
        };
        let mut grid = VolGrid2d::new(
            MapSizeLg::new(Vec2::new(14, 14)).expect("a valid test map size"),
            chunk(),
        )
        .expect("the grid must build");
        for key in keys {
            grid.insert(*key, chunk());
        }
        grid
    }

    fn chunk_key(column: Vec2<i32>) -> Vec2<i32> {
        use common::terrain::TerrainChunkSize;
        use common::vol::RectVolSize;
        column.map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
            e.div_euclid(sz as i32)
        })
    }

    /// B1, DRIVEN THROUGH THE RESOLVER: the datum is the FIRST AIR CELL
    /// derived from terrain — not the reported z. Both z-reports the flat
    /// arena can legitimately produce (400 = the slab's first air, 401 =
    /// the spawn point's "+1 clears landing jitter") must resolve to the
    /// SAME datum, because it is the same ground.
    ///
    /// This is the test that fails if anyone "simplifies" `resolve_datum`
    /// to `founding_pos.z as i32` — the one-block error that puts the
    /// stockpile floor inside the slab.
    #[test]
    fn datum_is_derived_from_terrain_not_from_the_reported_z() {
        let origin = Vec2::new(15216, 16016);
        let world = flat_world(400, &[chunk_key(origin)]);

        assert_eq!(
            resolve_datum(&world, origin, 401),
            Some(400),
            "the god standing at the spawn z (401) must still found on the datum (400)"
        );
        assert_eq!(
            resolve_datum(&world, origin, 400),
            Some(400),
            "and reporting the datum itself resolves identically -- same ground, same answer"
        );
    }

    /// A5's instrument: an unstandable site refuses BY NAME, and the same
    /// site validates clean when the ground is flat. The refusal names the
    /// offending column, so the emit can say WHERE rather than merely that
    /// something was uneven.
    #[test]
    fn a5_site_validation_refuses_uneven_ground_and_passes_flat_ground() {
        let origin_xy = Vec2::new(15216, 16016);
        let datum = 400;
        let origin = Vec3::new(origin_xy.x, origin_xy.y, datum);

        // Every column the preset touches must be loaded for the flat case
        // to pass -- an unresolvable column is itself a terrain refusal.
        let mut keys: Vec<Vec2<i32>> = footprint_columns(origin)
            .iter()
            .map(|c| chunk_key(*c))
            .collect();
        keys.sort_unstable_by_key(|k| (k.x, k.y));
        keys.dedup();

        let flat = flat_world(datum, &keys);
        assert_eq!(validate_site(&flat, origin), Ok(()), "flat ground must found");

        // The same preset, two blocks up: every column now deviates beyond
        // MAX_DATUM_DEVIATION and the founding must refuse by name.
        let stepped = Vec3::new(origin.x, origin.y, datum + 2);
        match validate_site(&flat, stepped) {
            Err((refusal, _column)) => {
                assert_eq!(refusal.reason(), "terrain");
            },
            Ok(()) => panic!("a site two blocks off its datum must be REFUSED, not shifted"),
        }
    }

    /// A1's control (packet §8 B5): the planted failure is a PARTIAL
    /// preset, so the completeness check must go RED when any single
    /// element is missing — not merely when placement is disabled
    /// wholesale (that removes subject and witness together: vacuity
    /// costume #3).
    #[test]
    fn a1_partial_preset_is_not_complete() {
        let full = vec![PresetRole::Stockpile, PresetRole::Farm, PresetRole::Bed];
        assert!(preset_is_complete(&full), "the full kit must read complete");

        // The PLANTED failure: found with the farm dropped.
        let no_farm = vec![PresetRole::Stockpile, PresetRole::Bed];
        assert!(
            !preset_is_complete(&no_farm),
            "A1 must go RED on a partial preset -- a founding without the farm is the colony \
             starving with a witness that said it was fed"
        );
        assert!(!preset_is_complete(&[PresetRole::Stockpile, PresetRole::Farm]));
        assert!(!preset_is_complete(&[PresetRole::Farm, PresetRole::Bed]));
        assert!(!preset_is_complete(&[]), "and the disabled case still reds");

        assert_eq!(roles_summary(&full), "stockpile,farm,bed");
        assert_eq!(roles_summary(&no_farm), "stockpile,bed");
    }

    /// A2's control (packet §8 B4): what holds colonists at F is THE WORK
    /// BEING AT F. So the property A2 actually rests on is that every plot
    /// the preset places is within the retention radius of the founding
    /// point — if the kit were placed far away, "colonists stay near F"
    /// would be measuring nothing.
    ///
    /// The planted failure A2 runs live is "found WITHOUT designations":
    /// that arm places zero regions, and this test pins the difference
    /// between the two arms at the data tier.
    #[test]
    fn a2_every_plot_sits_within_the_retention_radius_of_f() {
        // The generator scan half-width (`MINE_GEN_RADIUS = 12`) is the
        // colony's own working radius; the preset must fit well inside it,
        // or "stayed near F" and "went to work" stop being the same claim.
        const RETENTION_RADIUS: i32 = 12;
        let origin = script_15_origin();
        for (role, _, region) in preset_regions(origin) {
            for corner in [region.min, region.max] {
                let dx = (corner.x - origin.x).abs();
                let dy = (corner.y - origin.y).abs();
                assert!(
                    dx <= RETENTION_RADIUS && dy <= RETENTION_RADIUS,
                    "{} plot corner {:?} is outside the retention radius of F",
                    role.name(),
                    corner
                );
            }
        }

        // The control arm: no designations placed at all.
        assert!(
            !preset_is_complete(&[]),
            "the no-designation arm must be distinguishable from a founding"
        );
    }

    /// A4/A5: refusals are first-class outcomes and their log names are the
    /// bar. These strings are read by the acceptance scoring, so they are
    /// pinned here — a rename is a bar change and must fail a test first.
    #[test]
    fn a4_refusals_name_themselves_stably() {
        assert_eq!(FoundingRefusal::ColonyExists.reason(), "colony_exists");
        assert_eq!(FoundingRefusal::Terrain.reason(), "terrain");
        assert_ne!(
            FoundingRefusal::ColonyExists.reason(),
            FoundingRefusal::Terrain.reason(),
            "two refusals that log the same name are one refusal with a bug"
        );
        // The player must be TOLD, not merely refused (§3.2).
        assert!(
            FoundingRefusal::ColonyExists
                .player_message()
                .contains("already lives")
        );
        assert!(FoundingRefusal::Terrain.player_message().contains("flatter site"));
    }

    /// The footprint the site validation walks is the UNION of the plots,
    /// deduped — a founding is refused if ANY plot column is unstandable,
    /// not merely if the centre column is.
    #[test]
    fn footprint_covers_every_plot_column_once() {
        let origin = script_15_origin();
        let columns = footprint_columns(origin);

        // Stockpile 5x6 + farm 5x6 + bed 2x2, with the bed sitting inside
        // the stockpile's own x-band -- so the dedupe must actually fire.
        let naive: usize = preset_regions(origin)
            .iter()
            .map(|(_, _, r)| {
                ((r.max.x - r.min.x + 1) * (r.max.y - r.min.y + 1)) as usize
            })
            .sum();
        assert!(
            columns.len() < naive,
            "overlapping plots must dedupe: {} unique vs {} naive",
            columns.len(),
            naive
        );

        for (_, _, region) in preset_regions(origin) {
            for y in region.min.y..=region.max.y {
                for x in region.min.x..=region.max.x {
                    assert!(
                        columns.contains(&Vec2::new(x, y)),
                        "column ({x}, {y}) is in a plot but not in the validation footprint"
                    );
                }
            }
        }
    }

    /// Every chunk the preset's own footprint touches, DERIVED from
    /// `footprint_columns` rather than listed. A hand-written key list is a
    /// claim about geometry that silently rots when the element table moves;
    /// this one cannot disagree with the code under test.
    fn footprint_keys(origin: Vec3<i32>) -> Vec<Vec2<i32>> {
        let mut keys = Vec::new();
        for column in footprint_columns(origin) {
            let key = chunk_key(column);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    /// THE ORIGIN THAT STRADDLES. `TERRAIN_CHUNK_BLOCKS_LG = 5` ⇒ chunks are
    /// 32 blocks, so an origin on a multiple of 32 puts `ox−7` and `oy−4` in
    /// the previous chunks. This is the same rule the live W5 bar uses to
    /// pick its site — stated once, here, and asserted below.
    const STRADDLE_ORIGIN: Vec3<i32> = Vec3::new(15200, 16000, 419);

    /// A world whose chunks sit at DIFFERENT heights, so a footprint that
    /// spans a chunk boundary spans a real STEP. `flat_world` cannot express
    /// slope at all — every chunk it makes is identical — which is exactly
    /// why every prior bar ran blind to it.
    fn stepped_world(base_air: i32, origin: Vec3<i32>, raised: &[(Vec2<i32>, i32)]) -> TerrainGrid {
        use common::{
            terrain::{Block, BlockKind, MapSizeLg, SpriteKind, TerrainChunk, TerrainChunkMeta},
            volumes::vol_grid_2d::VolGrid2d,
        };
        use std::sync::Arc;

        let chunk_at = |first_air: i32| {
            Arc::new(TerrainChunk::new(
                first_air,
                Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
                Block::air(SpriteKind::Empty),
                TerrainChunkMeta::void(),
            ))
        };
        let mut grid = VolGrid2d::new(
            MapSizeLg::new(Vec2::new(14, 14)).expect("a valid test map size"),
            chunk_at(base_air),
        )
        .expect("the grid must build");
        for key in footprint_keys(origin) {
            grid.insert(key, chunk_at(base_air));
        }
        for (key, first_air) in raised {
            grid.insert(*key, chunk_at(*first_air));
        }
        grid
    }

    /// **THE FOUNDING COUNT AND THE BED PLOT MUST NOT DRIFT APART.**
    ///
    /// Capacity is DERIVED from `FOUNDING_PRESET_V1`, never restated —
    /// asserting `8 >= 8` between two literals would re-encode the very bug
    /// this row closes. BOTH sides move the bar: shrink the bed and it
    /// fails; raise `FOUNDING_COLONIST_COUNT` past capacity and it fails.
    /// That is what makes it a test of the RELATION rather than of a number.
    ///
    /// Why the bed is the binding resource: it is the only preset element
    /// sized per-colonist, and a colonist with no bed has no rest service.
    #[test]
    fn bed_capacity_covers_the_founding_count() {
        let capacity = bed_capacity();
        let count = common::bastion::FOUNDING_COLONIST_COUNT as usize;
        assert!(
            capacity > 0,
            "the preset must contain a Bed element for this bar to mean anything"
        );
        assert!(
            capacity >= count,
            "the founding brings {count} colonists but the preset's bed plot sleeps only \
             {capacity} -- either the bed shrank or the count grew"
        );
    }

    /// And the specimen is pinned, so a bed reshaped to the same total by
    /// accident still has to be looked at: 2 x 2 x 2 = 8.
    #[test]
    fn bed_capacity_is_the_two_by_two_by_two_the_table_describes() {
        assert_eq!(bed_capacity(), 8);
    }

    /// THE DENOMINATOR the worldgen row's every count is reported against.
    /// The pre-registration DERIVES 60 from the element table (farm −7..−3
    /// and stockpile −2..+2 are contiguous, bed is strictly inside); this
    /// makes that derivation fail loudly if the table ever changes, instead
    /// of silently re-basing every percentage that cites it.
    #[test]
    fn survey_reports_sixty_columns() {
        let origin = Vec3::new(15216, 16016, 419);
        let world = flat_world(419, &footprint_keys(origin));
        let relief = survey_site(&world, origin);
        assert_eq!(relief.columns, 60, "footprint column count");
        assert_eq!(relief.resolved, 60, "flat ground resolves every column");
    }

    /// W5's GEOMETRY CLAIM, at unit level: an origin on the 32-block chunk
    /// pitch really does spread the footprint over four chunks. The live bar
    /// asserts that every one of those columns still resolves; this asserts
    /// that the site it picks is actually the hard case it claims to be —
    /// otherwise W5 could "pass" on a footprint that never left one chunk.
    #[test]
    fn the_straddling_origin_really_spans_four_chunks() {
        let keys = footprint_keys(STRADDLE_ORIGIN);
        assert_eq!(
            keys.len(),
            4,
            "an origin on the chunk pitch must straddle in BOTH axes, got {keys:?}"
        );
        let centre = footprint_keys(Vec3::new(15216, 16016, 419));
        assert_eq!(
            centre.len(),
            1,
            "and the arena-style origin must NOT straddle, or the contrast is empty"
        );
    }

    /// A flat site is `Ok` on every field, and its deviations are ZERO —
    /// not merely "within bounds". A relief instrument that reported a
    /// constant would also pass a bounds check, so the bound is not enough.
    #[test]
    fn survey_of_flat_ground_is_zero_relief() {
        let origin = Vec3::new(15216, 16016, 419);
        let world = flat_world(419, &footprint_keys(origin));
        let relief = survey_site(&world, origin);
        assert_eq!(relief.min_dev, Some(0));
        assert_eq!(relief.max_dev, Some(0));
        assert_eq!(relief.branch(), ReliefBranch::Ok);
        assert!(relief.verdict().is_ok());
    }

    /// A world whose ground is flat and whose every cell ABOVE that ground is
    /// water — a lakebed, not a shore. `TerrainChunk::new`'s third argument
    /// is the block used above `first_air`, so this floods without touching
    /// the terrain profile at all, which is exactly the separation the test
    /// below needs.
    fn flooded_world(first_air: i32, keys: &[Vec2<i32>]) -> TerrainGrid {
        use common::{
            terrain::{Block, BlockKind, MapSizeLg, TerrainChunk, TerrainChunkMeta},
            volumes::vol_grid_2d::VolGrid2d,
        };
        use std::sync::Arc;

        let chunk = || {
            Arc::new(TerrainChunk::new(
                first_air,
                Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
                Block::new(BlockKind::Water, Rgb::new(0, 24, 255)),
                TerrainChunkMeta::void(),
            ))
        };
        let mut grid = VolGrid2d::new(
            MapSizeLg::new(Vec2::new(14, 14)).expect("a valid test map size"),
            chunk(),
        )
        .expect("the grid must build");
        for key in keys {
            grid.insert(*key, chunk());
        }
        grid
    }

    /// G1 · **A FLAT LAKEBED IS REFUSED.**
    ///
    /// This test is the INVERSION of one that previously asserted the
    /// opposite. That version pinned the gap — the preset founding
    /// underwater — precisely so that adding a gate would fail it loudly
    /// rather than let the change pass unnoticed. It did exactly that, and
    /// this is the deliberate update.
    ///
    /// Note what does NOT refuse it: `max_dev` is 0 and every column
    /// resolves, so nothing about the site's SHAPE is wrong. Only the water
    /// is, which is why it needed a refusal of its own rather than being
    /// folded into `terrain`.
    #[test]
    fn a_flat_lakebed_is_refused_by_the_water_gate() {
        let origin = Vec3::new(15216, 16016, 419);
        let world = flooded_world(419, &footprint_keys(origin));
        let relief = survey_site(&world, origin);

        assert_eq!(relief.submerged, 60, "every column carries water above it");
        assert_eq!(relief.resolved, 60, "the lakebed still resolves");
        assert_eq!(relief.max_dev, Some(0), "a flat bed deviates by nothing");
        assert_eq!(relief.branch(), ReliefBranch::Submerged);
        assert_eq!(
            relief.verdict(),
            Err((
                FoundingRefusal::Submerged,
                relief.first_submerged.expect("a flooded site names a column")
            )),
            "a flat lakebed must be refused BY NAME, not accepted"
        );
    }

    /// G1b · **ONE submerged column is enough.** The threshold is 1 because
    /// the preset's elements are surface structures; this drives the
    /// boundary rather than restating it, so a gate that only refused
    /// *fully* flooded sites would fail here.
    #[test]
    fn a_single_submerged_column_refuses_the_site() {
        // MUST be the straddling origin: a footprint inside a single chunk
        // cannot express a PARTIAL flood at chunk granularity — flooding
        // "one chunk" would flood all 60 columns and prove nothing about
        // the threshold.
        let origin = STRADDLE_ORIGIN;
        // Dry everywhere except the one chunk holding the farm's far corner.
        let flooded_key = chunk_key(Vec2::new(origin.x - 7, origin.y - 4));
        let mut world = flat_world(419, &footprint_keys(origin));
        {
            use common::{
                terrain::{Block, BlockKind, TerrainChunk, TerrainChunkMeta},
                vol::RectRasterableVol,
            };
            use std::sync::Arc;
            let _ = <TerrainChunk as RectRasterableVol>::RECT_SIZE;
            world.insert(
                flooded_key,
                Arc::new(TerrainChunk::new(
                    419,
                    Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
                    Block::new(BlockKind::Water, Rgb::new(0, 24, 255)),
                    TerrainChunkMeta::void(),
                )),
            );
        }
        let relief = survey_site(&world, origin);
        assert!(
            relief.submerged > 0 && relief.submerged < 60,
            "the specimen must be PARTLY flooded, got {} of 60",
            relief.submerged
        );
        assert_eq!(relief.max_dev, Some(0), "and still perfectly flat");
        assert!(
            matches!(relief.verdict(), Err((FoundingRefusal::Submerged, _))),
            "one submerged column is enough to refuse"
        );
    }

    /// G3 · **THE ORDERING HOLDS.** A site that is both sloped and submerged
    /// reports `terrain`, because the deviation test runs first and its
    /// reason string is already read by existing bars.
    #[test]
    fn a_sloped_submerged_site_still_reports_terrain() {
        let origin = STRADDLE_ORIGIN;
        let stepped = chunk_key(Vec2::new(origin.x - 7, origin.y - 4));
        // Flood everything, then step one chunk so it also deviates.
        let mut world = flooded_world(419, &footprint_keys(origin));
        {
            use common::terrain::{Block, BlockKind, TerrainChunk, TerrainChunkMeta};
            use std::sync::Arc;
            world.insert(
                stepped,
                Arc::new(TerrainChunk::new(
                    425,
                    Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
                    Block::new(BlockKind::Water, Rgb::new(0, 24, 255)),
                    TerrainChunkMeta::void(),
                )),
            );
        }
        let relief = survey_site(&world, origin);
        assert!(relief.submerged > 0, "the site really is submerged too");
        assert_eq!(
            relief.branch(),
            ReliefBranch::Deviation,
            "slope is reported first"
        );
        assert!(matches!(
            relief.verdict(),
            Err((FoundingRefusal::Terrain, _))
        ));
    }

    /// And the counterpart: dry ground must report ZERO submerged, or the
    /// field would be a constant and the test above would prove nothing.
    #[test]
    fn dry_ground_reports_no_submerged_columns() {
        let origin = Vec3::new(15216, 16016, 419);
        let world = flat_world(419, &footprint_keys(origin));
        assert_eq!(survey_site(&world, origin).submerged, 0);
    }

    /// THE ABSENCE BRANCH, REACHED ON PURPOSE. The worldgen row predicts
    /// water refuses through `Deviation` and NOT through `Absence`; that
    /// prediction is only falsifiable if `Absence` is reachable at all and
    /// reports itself distinctly. An unloaded chunk is the honest way to
    /// reach it, and it is also the failure mode a chunk-straddling
    /// footprint would exhibit if boundaries did leak.
    #[test]
    fn an_unloaded_chunk_reports_absence_not_deviation() {
        let world = flat_world(419, &[]);
        let relief = survey_site(&world, Vec3::new(15216, 16016, 419));
        assert_eq!(relief.columns, 60, "the footprint is still 60 columns");
        assert_eq!(relief.resolved, 0, "no chunk is loaded, so nothing resolves");
        assert_eq!(relief.min_dev, None);
        assert_eq!(relief.branch(), ReliefBranch::Absence);
        assert!(relief.verdict().is_err());
    }

    /// THE ACCEPTANCE CONDITION, AT ITS EXACT EDGE. `MAX_DATUM_DEVIATION`
    /// is 1 and the comparison is `> 1` on integers, so the
    /// pre-registration derives that a 1-block step is ACCEPTED and a
    /// 2-block step is REFUSED. Both directions are asserted: a bound
    /// tested only from the failing side cannot catch a bound that refuses
    /// everything.
    #[test]
    fn deviation_bound_accepts_one_block_and_refuses_two() {
        let origin = STRADDLE_ORIGIN;
        let stepped = chunk_key(Vec2::new(origin.x - 7, origin.y - 4));

        let one = stepped_world(419, origin, &[(stepped, 420)]);
        let relief = survey_site(&one, origin);
        assert_eq!(relief.resolved, 60, "a step is not an absence");
        assert_eq!(relief.max_dev, Some(1), "a one-block step deviates by one");
        assert!(
            relief.verdict().is_ok(),
            "MAX_DATUM_DEVIATION = 1 accepts a deviation OF one"
        );

        let two = stepped_world(419, origin, &[(stepped, 421)]);
        let relief = survey_site(&two, origin);
        assert_eq!(relief.max_dev, Some(2));
        assert_eq!(relief.branch(), ReliefBranch::Deviation);
        assert!(
            relief.verdict().is_err(),
            "`> MAX_DATUM_DEVIATION` on integers means two is refused"
        );
    }

    /// The refusal must be attributed to the DEVIATION test, and the worst
    /// column must be one that actually stepped — the water prediction in
    /// the worldgen pre-registration is a claim about exactly this field,
    /// so a `branch` that were hardcoded would make that claim unfalsifiable.
    #[test]
    fn deviation_branch_names_a_column_that_actually_moved() {
        let origin = STRADDLE_ORIGIN;
        let stepped = chunk_key(Vec2::new(origin.x - 7, origin.y - 4));
        let world = stepped_world(419, origin, &[(stepped, 423)]);
        let relief = survey_site(&world, origin);

        assert_eq!(relief.branch(), ReliefBranch::Deviation);
        assert_eq!(relief.resolved, 60, "a step is not an absence");
        assert_eq!(relief.min_dev, Some(0), "the unstepped chunks stay at datum");
        let (column, dev) = relief.worst.expect("a stepped world has a worst column");
        assert_eq!(dev, 4, "raising first_air by 4 deviates by 4");
        assert_eq!(
            chunk_key(column),
            stepped,
            "the worst column must lie in the chunk that moved"
        );
    }

    /// DELEGATION, NOT DUPLICATION. `validate_site` must return exactly what
    /// the survey's own verdict returns — including WHICH column is named,
    /// since the original early-returned at the first offender in iteration
    /// order. If these two ever disagree, the emit is describing a different
    /// computation from the one that decided, which is the whole failure
    /// mode this instrument was built to avoid.
    #[test]
    fn validate_site_agrees_with_the_survey_it_delegates_to() {
        let origin = STRADDLE_ORIGIN;
        let stepped = chunk_key(Vec2::new(origin.x - 7, origin.y - 4));
        for world in [
            flat_world(419, &footprint_keys(origin)),
            stepped_world(419, origin, &[(stepped, 420)]),
            stepped_world(419, origin, &[(stepped, 425)]),
            flat_world(419, &[]),
        ] {
            assert_eq!(
                validate_site(&world, origin),
                survey_site(&world, origin).verdict(),
                "validate_site must BE the survey's verdict, not a second opinion"
            );
        }
    }
}
