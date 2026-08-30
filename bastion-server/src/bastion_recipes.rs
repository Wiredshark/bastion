//! bastion (ARC 5 item 26): the crafting-chain RECIPE TABLE — the cook
//! pipeline's hardcoded mushroom→curry generalized to data. One loader
//! (the [`crate::bastion_mood::ThoughtTable`] pattern), one lookup the
//! generator/fetch/completion all share, so a chain cannot half-exist.
//!
//! `&'static str` defs: the Job's `required_item` field is `&'static`
//! (the B6 fetch contract rides it through claims and inventories), so
//! loaded defs are leaked ONCE at first access — bounded by the recipe
//! count, and the table is process-lifetime data anyway.

use common::assets::{AssetExt, BoxedError, FileAsset, load_ron};
use common::bastion::DesignationKind;
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize)]
pub struct RecipeV1 {
    pub station: DesignationKind,
    pub input: (String, u32),
    pub output: (String, u32),
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RecipeTable {
    pub recipes: Vec<RecipeV1>,
}

impl FileAsset for RecipeTable {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> { load_ron(&bytes) }
}

/// A recipe with its defs leaked to `&'static` — the shape the job board
/// consumes directly.
#[derive(Copy, Clone, Debug)]
pub struct StaticRecipe {
    pub station: DesignationKind,
    pub input_def: &'static str,
    pub input_n: u32,
    pub output_def: &'static str,
    pub output_n: u32,
}

/// The loaded, leaked table. Loaded ONCE (no hot reload: the leak is
/// per-load, and the consumers hold `&'static` — a reload would strand
/// jobs on old defs mid-flight). Empty on a missing/broken asset —
/// graceful: no recipes means no craft jobs generate, witnessed by the
/// generator's own idle path, and nothing panics.
pub fn recipes() -> &'static [StaticRecipe] {
    static TABLE: OnceLock<Vec<StaticRecipe>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let loaded = RecipeTable::load("common.bastion_recipes")
            .map(|h| h.read().clone())
            .unwrap_or_default();
        loaded
            .recipes
            .into_iter()
            .map(|r| StaticRecipe {
                station: r.station,
                input_def: Box::leak(r.input.0.into_boxed_str()),
                input_n: r.input.1,
                output_def: Box::leak(r.output.0.into_boxed_str()),
                output_n: r.output.1,
            })
            .collect()
    })
}

/// Every recipe registered for a station kind (multi-recipe dispatch is
/// data: first-with-available-input wins at the generator).
pub fn recipes_for(station: DesignationKind) -> impl Iterator<Item = &'static StaticRecipe> {
    recipes().iter().filter(move |r| r.station == station)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bastion (ITEM 29): the invariant `RAW_FOOD_DEFS` CLAIMED and nothing
    /// enforced.
    ///
    /// ITEM 27 kept a hand-written `RAW_FOOD_DEFS` allowlist beside
    /// `FOOD_DEFS` in `bastion_jobs`, doc'd "kept separate so the pot can
    /// never cook its own output (the dish matching a raw scan would loop
    /// curry→curry forever)". ITEM 26 moved the cook generator onto THIS
    /// table and the const stopped being read — a repo-wide grep found only
    /// its own declaration. The sentence outlived the code, describing data
    /// it no longer governed: the RON could grow a row whose input is another
    /// row's output and the station would cook its own product forever with
    /// nothing red. A comment cannot enforce; the property belongs where the
    /// data now lives.
    ///
    /// The loop is real, not theoretical: the completion `emit_drop`s the
    /// output as a ground item AT THE STATION, and the generator scans ground
    /// items lying in a stockpile for ANY registered input — so a station
    /// standing inside a stockpile footprint would re-pick its own dish on
    /// the next cadence. That it needs that layout to bite is exactly why it
    /// is pinned on the TABLE, where it holds regardless of where a colony
    /// happens to put its pot.
    ///
    /// DELIBERATELY STRICTER THAN A CYCLE CHECK: this also forbids a benign
    /// multi-step chain (wheat→flour, flour→bread), which cascades on its own
    /// for the same reason and which nothing has been designed for. It is also
    /// the only form that catches a cycle spanning two station kinds
    /// (Cook→Forge→Cook), which a per-station check would miss — the check is
    /// over the WHOLE table, not per station, for exactly that reason. When a
    /// real multi-step chain is genuinely wanted, relax this
    /// to cycle detection over the input→output graph — as a deliberate
    /// change, with the cascade designed, not by deleting the assertion.
    #[test]
    fn no_recipe_input_is_any_recipe_output() {
        let table = recipes();

        // The falsifier's own precondition. `recipes()` swallows a missing or
        // unparseable asset into an EMPTY table, and an empty table satisfies
        // the property below vacuously — a green bar that proves nothing.
        // Convict the load first, so a broken asset reads RED, not PASS.
        assert!(
            !table.is_empty(),
            "the recipe table loaded EMPTY — assets/common/bastion_recipes.ron is missing or \
             failed to parse. The cook-loop assertion below is vacuous on an empty table, so this \
             is a failure, not a pass."
        );
        assert!(
            table
                .iter()
                .any(|r| r.station == DesignationKind::CookStation),
            "no CookStation recipe loaded — the cook pipeline whose curry→curry loop this pins is \
             not in the table at all, so the assertion below would pass without ever seeing the \
             thing it guards"
        );

        let outputs: std::collections::BTreeSet<&'static str> =
            table.iter().map(|r| r.output_def).collect();
        let loops: Vec<(&'static str, &'static str)> = table
            .iter()
            .filter(|r| outputs.contains(r.input_def))
            .map(|r| (r.input_def, r.output_def))
            .collect();

        assert!(
            loops.is_empty(),
            "a recipe's INPUT is some recipe's OUTPUT — the station would cook its own product \
             forever (the curry→curry loop ITEM 27 kept RAW_FOOD_DEFS to prevent). Offending \
             (input, output) rows: {loops:?}"
        );
    }
}
