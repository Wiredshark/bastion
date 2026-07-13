//! bastion (B7-0, row 44): the server-side half of the mood pipeline —
//! the THOUGHT TABLE and the chronicle query that turns remembered
//! events into the formula's summed thought term.
//!
//! Lives server-side by dependency direction: the table keys on
//! [`rtsim::data::ChronicleKind`], which `common` cannot see — so
//! `common`'s [`common::comp::bastion::mood_formula`] takes the summed
//! term as a plain input, and this module owns producing it (the server
//! sees both crates). v1 ships the seed set the design names (a `Death`
//! thought; `CaveIn` pending the fork ruling); more thoughts are PURE
//! DATA added to the RON as their emitter events land (HIST-1) — the
//! formula never reshapes.

use common::assets::{self, AssetExt, BoxedError, FileAsset, load_ron};
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap};

/// Thought tuning: [`rtsim::data::ChronicleKind`] → (signed magnitude,
/// lifetime in game-seconds). An event only weighs on a colonist whose
/// [`common::rtsim::Actor`] appears in the entry's `actors`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ThoughtTable {
    pub thoughts: HashMap<rtsim::data::ChronicleKind, (f32, f64)>,
}

impl FileAsset for ThoughtTable {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        load_ron(&bytes)
    }
}

impl ThoughtTable {
    /// The loaded table (hot-reloadable); an EMPTY table on a
    /// missing/broken asset — graceful: no thoughts weigh, needs still
    /// drive mood, nothing panics.
    pub fn current() -> Self {
        Self::load("common.bastion_thoughts")
            .map(|h| h.read().clone())
            .unwrap_or_default()
    }
}

/// The summed thought term for one colonist: every chronicle entry whose
/// `actors` names them and whose kind the table maps, decayed by
/// [`common::comp::bastion::thought_decay`] (pure `(deposit, now)` — no
/// per-tick state). Order-free (addition commutes) — the determinism
/// house invariant, same as the formula it feeds.
pub fn thought_sum(
    chronicle: &rtsim::data::Chronicle,
    table: &ThoughtTable,
    actor: common::rtsim::Actor,
    now: f64,
) -> f32 {
    chronicle
        .events()
        .filter(|e| e.actors.contains(&actor))
        .filter_map(|e| {
            table.thoughts.get(&e.kind).map(|(mag, life)| {
                common::comp::bastion::thought_decay(*mag, e.at_tod.0, now, *life)
            })
        })
        .sum()
}
