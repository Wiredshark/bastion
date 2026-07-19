//! bastion (B-AG2, row 40): ONE SHARED BRAIN, archetype-keyed DATA — the
//! Playbook's own scope. The brain historically scattered
//! `matches!(ctx.npc.profession(), Some(Profession::X)) &&
//! ctx.rng.random_bool(HARDCODED)` gates through its decision logic; the
//! hardcoded probability IS a scoring weight and the `matches!` IS an
//! allowed-activity list — so this module lifts both into a RON asset
//! keyed by ARCHETYPE, and the gates become ONE shared lookup. Identical
//! world-state then yields data-driven behavior differences with zero AI
//! forks (Agency Bible §5c.1's "one action library, two drivers",
//! generalized to "one brain, many archetype configs" — the same shape
//! B-AG5-CORE proved for verbs, applied to decision-weighting).
//!
//! SCOPE (proof-of-concept per the packet): three of the brain's gates
//! convert this block (herbalist / hunter / guard — see the call sites in
//! [`super`]); the §4 expansion pass sweeps the rest (farmer / merchant /
//! chef, then species keys for `Role::Wild`). No new verbs are invented
//! here — weights only ever gate the brain's EXISTING activity
//! vocabulary. Bastion colonists' `work_priorities` is a different
//! mechanism for a different population and stays untouched.

use common::assets::{self, AssetExt, BoxedError, FileAsset, load_ron};
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap};

/// One archetype's decision data: activity name → chance weight (the
/// probability the brain's consideration pass takes that activity when
/// its preconditions hold — exactly the semantics the old hardcoded
/// `random_bool` constants had). Key ABSENT = activity not allowed for
/// this archetype (the allowed-list and the weights are one map).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ArchetypeConfig {
    pub activities: HashMap<String, f32>,
}

/// The full archetype table (`assets/common/rtsim/archetypes.ron`).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ArchetypeConfigs {
    pub archetypes: HashMap<String, ArchetypeConfig>,
}

impl FileAsset for ArchetypeConfigs {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> { load_ron(&bytes) }
}

/// The archetype KEY for an NPC — professions map to keys this block
/// (the converted gates are all villager roles); the §4 expansion adds
/// the rest of the professions and species-class keys for wild bodies.
/// `None` = no archetype data applies → every converted gate stays
/// closed, exactly like a non-matching profession under the old
/// hardcoded `matches!` (the graceful default the invariants require).
pub fn archetype_key(profession: Option<common::rtsim::Profession>) -> Option<&'static str> {
    use common::rtsim::Profession;
    match profession? {
        Profession::Herbalist => Some("herbalist"),
        Profession::Hunter => Some("hunter"),
        Profession::Guard => Some("guard"),
        _ => None,
    }
}

/// The ONE shared lookup every converted gate calls: this archetype's
/// weight for this activity, `None` when the archetype doesn't list it
/// (not allowed), the archetype is unknown, or the asset failed to load
/// (graceful, never a crash — a broken table means role-flavor activities
/// simply don't fire, it never invents behavior).
pub fn archetype_chance(key: &str, activity: &str) -> Option<f32> {
    let handle = ArchetypeConfigs::load("common.rtsim.archetypes")
        .map_err(|e| {
            tracing::warn!(?e, "bastion (B-AG2): archetype table failed to load");
            e
        })
        .ok()?;
    let configs = handle.read();
    lookup(&configs, key, activity)
}

/// The pure core of [`archetype_chance`] — separated so tests and the
/// harness contrast probe exercise EXACTLY the code path the brain uses.
pub fn lookup(configs: &ArchetypeConfigs, key: &str, activity: &str) -> Option<f32> {
    configs
        .archetypes
        .get(key)?
        .activities
        .get(activity)
        .copied()
}

/// An archetype's full allowed set, name-sorted — the harness's
/// "same code, different data → different choices" contrast probe.
pub fn allowed_set(key: &str) -> Vec<(String, f32)> {
    let Ok(handle) = ArchetypeConfigs::load("common.rtsim.archetypes") else {
        return Vec::new();
    };
    let configs = handle.read();
    let mut set: Vec<(String, f32)> = configs
        .archetypes
        .get(key)
        .map(|c| c.activities.iter().map(|(a, w)| (a.clone(), *w)).collect())
        .unwrap_or_default();
    set.sort_by(|a, b| a.0.cmp(&b.0));
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ArchetypeConfigs {
        load_ron(
            br#"(
    archetypes: {
        "herbalist": (activities: {"gather_forest": 0.8}),
        "guard": (activities: {"patrol_plaza": 0.7}),
    },
)"#,
        )
        .expect("inline archetype RON parses")
    }

    /// The done-when in miniature: the SAME lookup code, two archetype
    /// keys, the same activity vocabulary → different data-driven
    /// outcomes; unknown keys/activities are a graceful None.
    #[test]
    fn one_lookup_many_archetypes() {
        let c = sample();
        assert_eq!(lookup(&c, "herbalist", "gather_forest"), Some(0.8));
        assert_eq!(lookup(&c, "guard", "gather_forest"), None);
        assert_eq!(lookup(&c, "guard", "patrol_plaza"), Some(0.7));
        assert_eq!(lookup(&c, "herbalist", "patrol_plaza"), None);
        // Unknown archetype / activity: closed gate, no panic.
        assert_eq!(lookup(&c, "farmer", "gather_forest"), None);
        assert_eq!(lookup(&c, "herbalist", "no_such_activity"), None);
    }

    /// Key derivation: converted professions map, everything else is the
    /// graceful None (old-hardcoded-equivalent) default.
    #[test]
    fn key_derivation_matches_converted_set() {
        use common::rtsim::Profession;
        assert_eq!(
            archetype_key(Some(Profession::Herbalist)),
            Some("herbalist")
        );
        assert_eq!(archetype_key(Some(Profession::Hunter)), Some("hunter"));
        assert_eq!(archetype_key(Some(Profession::Guard)), Some("guard"));
        assert_eq!(archetype_key(Some(Profession::Farmer)), None);
        assert_eq!(archetype_key(None), None);
    }
}
