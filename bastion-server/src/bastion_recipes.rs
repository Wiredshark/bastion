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
