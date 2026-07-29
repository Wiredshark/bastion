//! T0.68: the selection-site registry -- every `min_by_key`/`max_by_key`/
//! `min_by`/`max_by` call in the authoritative crates, cataloged against
//! whether its key already ends in a stable tiebreak (see
//! [`crate::decision_key`] for the convention itself).
//!
//! Two of these were found genuinely missing a tiebreak and migrated onto
//! `DecisionKeyV1` this row: `world/src/civ/mod.rs`'s biome-center chunk
//! pick (an exact distance tie fell through to flood-fill insertion
//! order) and `server/src/lib.rs`'s spawn-point settlement pick (an exact
//! distance tie fell through to `Store::values()` iteration order). Both
//! are now `Complete`.
//!
//! Keyed by (file relative to workspace root, exact trimmed matched line
//! text, 0-based occurrence index) -- NOT by line number. An earlier
//! draft of this registry used line numbers and broke the moment either
//! migrated site's surrounding lines shifted; this is the same
//! line-number-drift lesson `semantic_net`'s catalogs and
//! `rng_source_registry` already learned, applied here after re-learning
//! it the hard way.
//!
//! Honesty note, matching the `numeric_surface`/`host_input_manifest`
//! sampled-vs-presumed precedent: every site the scanner finds is
//! enumerated here (the completeness gate below is real), but not every
//! site was read with the same depth. `Complete`/`IncompleteCosmetic`/
//! `IncompleteAuthoritativeMigrated` entries were read directly -- their
//! `note` says what was found. `NotReviewed` entries are the scanner's
//! raw catalog only; classifying them is future work, not a claim this
//! row makes.

use std::path::Path;

/// What's known about a selection site's tiebreak behavior.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SelectionStatusV1 {
    /// The key already ends in a stable, reviewable tiebreak (an id, a
    /// position's components, or the id itself) -- an exact-score tie is
    /// decided by that field, not by iteration order.
    Complete,
    /// No tiebreak, confirmed by reading the site -- but the selection
    /// feeds a read-only display/inspect/flavor-content path, not
    /// authoritative simulation state, so a tie's outcome doesn't affect
    /// what's simulated (only what a tooltip shows or which line of
    /// flavor dialogue is picked). Not migrated: `DecisionKeyV1` exists
    /// for decisions, and these aren't decisions.
    IncompleteCosmetic,
    /// No tiebreak, confirmed by reading the site, and the selection DOES
    /// feed authoritative state -- migrated onto `DecisionKeyV1` this row.
    IncompleteAuthoritativeMigrated,
    /// No tiebreak in the KEY, read directly, and authoritative -- but
    /// the selection is still deterministic because its INPUT SEQUENCE
    /// is provably ordered (a `Vec` in a fixed build order, or an
    /// explicit sort upstream), so `min_by*`-takes-first /
    /// `max_by*`-takes-last resolves every tie the same way each run.
    ///
    /// A separate status from [`Complete`](Self::Complete) on purpose:
    /// the guarantee is NON-LOCAL. Nothing at the selection expression
    /// says the input is ordered, so the site can be silently broken by
    /// an edit elsewhere -- dropping a sort, or swapping a `Vec` for a
    /// `HashMap` -- that never touches this line and looks unrelated in
    /// review. Calling that `Complete` would file a remote dependency as
    /// a local property, which is the overstatement this registry
    /// exists to prevent. Introduced `E13` chunk 2, when the widened
    /// root set brought `server/agent/src` in and all three of its
    /// selection sites turned out to be exactly this shape.
    CompleteByOrderedInput,
    /// Found by the scan, not yet individually read. Disclosed as such,
    /// not silently assumed either way.
    NotReviewed,
}

struct SelectionSiteV1 {
    file: &'static str,
    occurrence: u32,
    snippet: &'static str,
    status: SelectionStatusV1,
    note: &'static str,
}

const fn site(
    file: &'static str,
    occurrence: u32,
    snippet: &'static str,
    status: SelectionStatusV1,
    note: &'static str,
) -> SelectionSiteV1 {
    SelectionSiteV1 { file, occurrence, snippet, status, note }
}

use SelectionStatusV1::{
    Complete, CompleteByOrderedInput, IncompleteAuthoritativeMigrated, IncompleteCosmetic,
    NotReviewed,
};

const CATALOG: &[SelectionSiteV1] = &[
    site("common/src/async_work.rs", 0, ".min_by_key(|(_, p)| {", NotReviewed, "found by scan, not read this pass"),
    site("common/src/comp/body/parts.rs", 0, ".min_by(|a, b| match (a, b) {", NotReviewed, "found by scan, not read this pass"),
    site("common/src/comp/health.rs", 0, ".max_by_key(|(contrib, (_, time))| (time.0.to_bits(), contrib.uid().0))", Complete, "key is (time.0.to_bits(), contrib.uid().0) -- canonical score then stable uid"),
    site("common/src/path.rs", 0, ".min_by_key(|intersect: &Vec2<f32>| {", NotReviewed, "found by scan, not read this pass"),
    site("common/src/path.rs", 0, ".min_by_key(|(d2, _)| (d2 * 1000.0) as i32)", NotReviewed, "found by scan, not read this pass"),
    site("common/src/terrain/mod.rs", 0, ".min_by(|&(ap, _, a), &(bp, _, b)| {", NotReviewed, "found by scan, not read this pass"),
    site("common/src/threat_policy.rs", 0, "candidates.iter().enumerate().max_by(|(_, a), (_, b)| compare(a, b)).map(|(i, _)| i)", NotReviewed, "found by scan, not read this pass"),
    site("common/src/volumes/chunk.rs", 0, ".max_by_key(|(_, default_groups)| {", NotReviewed, "found by scan, not read this pass"),
    site("server/src/events/inventory_manip.rs", 0, ".min_by_key(|(_, _, wild_pos, _)| {", NotReviewed, "found by scan, not read this pass"),
    site("server/src/lib.rs", 0, ".min_by_key(|site_pos| {", IncompleteAuthoritativeMigrated, "spawn-point settlement pick: an exact distance tie previously fell through to Store::values() iteration order -- migrated onto DecisionKeyV1, tiebroken on the site center's own (x, y)"),
    site("server/src/lib.rs", 0, ".min_by_key(|j| (j.pos.z - cell.z).abs())", IncompleteCosmetic, "bastion_inspect_cell, explicitly documented READ-ONLY (a right-click inspect tooltip) -- a tie only changes which equidistant job's info is shown, not any simulated state"),
    site("server/src/rtsim/mod.rs", 0, ".min_by_key(|(_, site)| {", NotReviewed, "found by scan, not read this pass"),
    site("server/src/state_ext.rs", 0, "nearby_items.min_by_key(|(_, dist)| (dist * 1000.0) as i32)", NotReviewed, "found by scan, not read this pass"),
    site("server/src/sys/msg/in_game.rs", 0, ".min_by_key(|j| (j.pos.z - cell.z).abs())", NotReviewed, "found by scan, not read this pass"),
    // E13 chunk 2: server/agent/src entered the root set. All three of
    // its selection sites were read, and all three are ordered-input.
    site("server/agent/src/action_nodes.rs", 0, ".max_by_key(|(_, item)| {", CompleteByOrderedInput, "select_healing_item: which item an NPC drinks. Input is inventory.slots_with_id(), a Vec walked in slot-index order; max_by_key takes the LAST maximum, so equal-value items always resolve to the highest slot index"),
    site("server/agent/src/action_nodes.rs", 0, ".min_by(|a, b| {", CompleteByOrderedInput, "colonist zone-magnet wander pull. Input is ActivityZones(pub Vec<(ZoneKind, Region)>) -- a Vec, not a map (common/src/bastion.rs:298); equidistant zones resolve to the first in Vec order. NaN would compare Equal via unwrap_or, which also lands on the first"),
    site("server/agent/src/action_nodes.rs", 0, ".min_by_key(|(_, pos, _)| (pos.distance_squared(self_pos) * 100.0) as i32)", CompleteByOrderedInput, "pick_target_candidate's non-combat bucket -- WHO an NPC targets, fully authoritative. The *100-as-i32 quantization manufactures ties routinely, and the key carries no tiebreak; determinism comes from choose_target sorting entities_nearby by Uid ~150 lines upstream (action_nodes.rs:1345), so ties land on the lowest Uid. The clearest case for this status existing: delete that one sort and this line still reads correct"),
    // E13 chunk 4: common/systems/src entered the root set.
    site("common/systems/src/phys/collision.rs", 0, ".min_by_key(|(a, b)| ordered_float::OrderedFloat(a.distance_squared(*b)))", CompleteByOrderedInput, "parallel-line closest-approach fallback. No tiebreak in the key, but the input is a FOUR-ELEMENT ARRAY LITERAL in the same expression -- so unlike the E13-2 agent sites the ordering guarantee is LOCAL and cannot be broken by a remote edit. Filed under this status for accuracy about the key, not because it carries the same fragility"),
    site("common/systems/src/phys/mod.rs", 0, ".min_by_key(|line| (line.distance_to_point(tgt_pos) * 1000.0) as i32)", CompleteByOrderedInput, "rail/track snapping, which clamps velocity -- authoritative. The *1000-as-i32 quantization manufactures ties, and the key has no tiebreak; order comes from Spiral2d::new().take(9) (a deterministic generator) then TerrainChunkMeta::tracks() -> &[CubicBezier3<f32>] (a slice, checked at terrain/mod.rs:239) then a 0..32 subdivision range. Every stage is ordered, so ties land consistently on the first"),
    // E13 chunk 3: common/state/src entered the root set.
    site("common/state/src/plugin/resolver.rs", 0, ".min_by_key(|&(_, &n)| n)", Complete, "APEX-T2.4 dependency-cycle report canonicalization: rotates a detected cycle so its smallest node index leads. No tiebreak is needed because NO TIE IS POSSIBLE -- a cycle cannot contain the same node twice, so the keys are distinct by construction. This site exists precisely to make a cycle's reported form canonical rather than dependent on which node the DFS happened to enter from"),
    site("rtsim/src/ai/action_policy.rs", 0, ".max_by(|(_, a), (_, b)| compare(a, b))", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/generate/site.rs", 0, ".min_by_key(|(faction_wpos, _)| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/migrate.rs", 0, ".min_by_key(|(_, site)| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/airship_ai.rs", 0, "if let Some(my_spawn_loc) = my_route.spawning_locations.iter().min_by(|a, b| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/dialogue.rs", 0, "if let Some(p) = ws.plots().filter(f).min_by_key(|p| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/mod.rs", 0, ".min_by_key(|other| other.wpos.xy().distance(ctx.npc.wpos.xy()) as i32)", IncompleteCosmetic, "NPC flavor-dialogue nearest-monster mention, itself marked t0.6-exempt (one-shot content/decision draw) -- a tie only changes which monster gets named in speech text, not any simulated state"),
    site("rtsim/src/rule/npc_ai/mod.rs", 0, ".min_by_key(|(site_id, site)| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/mod.rs", 1, ".min_by_key(|(site_id, site)| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/mod.rs", 0, ".min_by_key(|(site_id, site, _)| {", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/mod.rs", 0, ".min_by_key(|(_, site)| site.wpos.as_().distance(npc_pos) as i32)*/", NotReviewed, "found by scan, not read this pass"),
    site("rtsim/src/rule/npc_ai/quest.rs", 0, ".min_by_key(|(_, npc)| npc.wpos.xy().distance_squared(ctx.npc.wpos.xy()) as i64)", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|candidate| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|(owner, (nearest, _, target))| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|step| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 1, ".min_by_key(|candidate| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|cell| ((cell.z - feet.z).abs(), cell.x, cell.y, cell.z));", Complete, "key is (z-delta, cell.x, cell.y, cell.z) -- full position, no tie possible"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by(|a, b| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|(_, r)| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 1, ".min_by_key(|(_, r)| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|(_, ipos, iuid)| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|(p, _)| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 1, ".min_by(|a, b| {", NotReviewed, "found by scan, not read this pass"),
    site("bastion-server/src/bastion_jobs.rs", 0, ".min_by_key(|(id, _)| *id)", Complete, "key is *id -- the selection key IS the stable id, no separate tiebreak needed"),
    site("world/src/civ/mod.rs", 0, ".min_by_key(|(id, s)| (s.origin.map(|e| e as i64).distance_squared(wpos), *id));", Complete, "DET-SITE-003: key is (distance_squared, *id) -- the in-repo precedent this whole row generalizes"),
    site("world/src/civ/mod.rs", 0, ".min_by_key(|&b| {", IncompleteAuthoritativeMigrated, "biome-center chunk pick: an exact distance tie previously fell through to the flood-fill's insertion order into biome.1 -- migrated onto DecisionKeyV1 via the new select_biome_center_chunk helper, tiebroken on the chunk index"),
    site("world/src/layer/cave.rs", 0, ".max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))", NotReviewed, "found by scan, not read this pass"),
    site("world/src/layer/cave.rs", 1, ".max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))", NotReviewed, "found by scan, not read this pass"),
    site("world/src/lib.rs", 0, ".min_by_key(|id| {", NotReviewed, "found by scan, not read this pass"),
    site("world/src/sim/erosion.rs", 0, ".max_by(|a, b| a.partial_cmp(b).unwrap())", NotReviewed, "found by scan, not read this pass"),
    site("world/src/sim/erosion.rs", 1, ".max_by(|a, b| a.partial_cmp(b).unwrap())", NotReviewed, "found by scan, not read this pass"),
    site("world/src/sim/mod.rs", 0, ".max_by_key(|rpos| {", NotReviewed, "found by scan, not read this pass"),
    site("world/src/sim/mod.rs", 0, ".min_by_key(|NearestWaysData { dist_sqrd, .. }| (dist_sqrd * 1024.0) as i32)", NotReviewed, "found by scan, not read this pass"),
    site("world/src/sim/mod.rs", 0, ".max_by_key(|fk| (fk.proclivity(&env) * 10000.0) as u32)", NotReviewed, "found by scan, not read this pass"),
    site("world/src/sim/mod.rs", 0, ".min_by_key(|id| index_sites[**id].origin.distance_squared(wpos2d))", IncompleteCosmetic, "worldgen location display-name lookup (feeds e.g. WaypointSaved-style chat notifications) -- a tie only changes which equidistant site's name is displayed, not any simulated state"),
    site("world/src/site/economy/mod.rs", 0, ".max_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap_or(Less))", NotReviewed, "found by scan, not read this pass"),
    site("world/src/site/economy/mod.rs", 0, ".min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Less))", NotReviewed, "found by scan, not read this pass"),
    site("world/src/site/generation.rs", 0, ".max_by_key(|(_, b)| (jaccard(*a, **b) * 1000.0) as usize);", NotReviewed, "found by scan, not read this pass"),
    site("world/src/site/mod.rs", 0, ".min_by_key(|d2| *d2 as i32)", NotReviewed, "found by scan, not read this pass"),
    site("world/src/site/mod.rs", 0, ".min_by_key(|&&p| (self.plot(p).root_tile.distance_squared(tpos), p))", Complete, "key is (distance_squared, p) where p is the plot's own stable handle -- score then stable id"),
    site("world/src/site/mod.rs", 0, ".min_by_key(|d| (*d * 100.0) as i32)", NotReviewed, "found by scan, not read this pass"),
    site("world/src/site/plot/adlet.rs", 0, ".max_by_key(|theta| {", NotReviewed, "found by scan, not read this pass"),
    site("world/src/site/plot/tavern.rs", 0, ".max_by_key(|bounds| bounds.size().product())?;", NotReviewed, "found by scan, not read this pass"),
    site("common/src/decision_key.rs", 0, "let winner_forward = candidates.iter().min_by_key(|c| key(c)).copied();", Complete, "decision_key's own permutation-invariance test -- key(c) returns a DecisionKeyV1, this IS the convention demonstrated, not a gap"),
    site("common/src/decision_key.rs", 0, "candidates.iter().rev().min_by_key(|c| key(c)).copied();", Complete, "same test, the reversed-order half of the permutation-invariance check"),
];

const PATTERNS: [&str; 4] = [".min_by_key(", ".max_by_key(", ".min_by(", ".max_by("];

/// T0.83: delegates to the shared [`crate::scanner_framework`] walk/
/// match/occurrence-index primitive rather than hand-rolling it a fourth
/// time (this module, `host_input_manifest`, and `rng_source_registry`
/// each wrote the same walk before this row unified it).
pub fn scan_live_selection_sites(workspace_root: &Path, dirs: &[&Path]) -> Vec<(String, String, u32)> {
    crate::scanner_framework::scan_lines_matching(workspace_root, dirs, &PATTERNS, "selection_registry.rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> std::path::PathBuf {
        crate::scanner_framework::workspace_root_from_manifest_dir(env!("CARGO_MANIFEST_DIR"))
    }

    fn scan_roots() -> Vec<std::path::PathBuf> {
        let root = workspace_root();
        crate::scanner_framework::AUTHORITATIVE_SCAN_ROOTS
            .iter()
            .map(|p| root.join(p))
            .collect()
    }

    #[test]
    fn every_live_selection_site_is_cataloged() {
        let root_bufs = scan_roots();
        let roots: Vec<&Path> = root_bufs.iter().map(|p| p.as_path()).collect();
        let live = scan_live_selection_sites(&workspace_root(), &roots);
        let catalog: std::collections::HashSet<(&str, &str, u32)> =
            CATALOG.iter().map(|s| (s.file, s.snippet, s.occurrence)).collect();

        let uncataloged: Vec<_> = live
            .iter()
            .filter(|(f, s, i)| !catalog.contains(&(f.as_str(), s.as_str(), *i)))
            .collect();
        assert!(
            uncataloged.is_empty(),
            "uncataloged min_by_key/max_by_key/min_by/max_by sites found:\n{:#?}",
            uncataloged
        );
    }

    #[test]
    fn every_catalog_entry_still_matches_a_live_site() {
        let root_bufs = scan_roots();
        let roots: Vec<&Path> = root_bufs.iter().map(|p| p.as_path()).collect();
        let live: std::collections::HashSet<(String, String, u32)> =
            scan_live_selection_sites(&workspace_root(), &roots).into_iter().collect();

        let stale: Vec<_> = CATALOG
            .iter()
            .filter(|s| !live.contains(&(s.file.to_string(), s.snippet.to_string(), s.occurrence)))
            .map(|s| (s.file, s.snippet, s.occurrence))
            .collect();
        assert!(
            stale.is_empty(),
            "catalog entries with no live site (rename/removal/edit):\n{:#?}",
            stale
        );
    }

    /// Falsifier: a planted, uncataloged selection site must be caught.
    #[test]
    fn falsifier_a_planted_uncataloged_site_is_flagged() {
        let dir = std::env::temp_dir().join(format!(
            "selection_registry_falsifier_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("planted.rs"),
            "fn planted(v: &[i32]) -> Option<&i32> { v.iter().min_by_key(|&&x| x) }\n",
        )
        .unwrap();

        let live = scan_live_selection_sites(dir.as_path(), &[dir.as_path()]);
        assert!(!live.is_empty(), "the scanner failed to find the planted site at all");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn every_entry_has_a_substantive_note() {
        for entry in CATALOG {
            assert!(
                entry.note.len() > 10,
                "{}::{} has no substantive note",
                entry.file,
                entry.snippet
            );
        }
    }

    /// Non-vacuity: the catalog must actually contain at least one entry
    /// of each status, or a whole class of this registry's own claims is
    /// untested.
    #[test]
    fn every_status_class_has_at_least_one_entry() {
        for status in [Complete, IncompleteCosmetic, IncompleteAuthoritativeMigrated, NotReviewed] {
            assert!(
                CATALOG.iter().any(|s| s.status == status),
                "no catalog entry has status {status:?}"
            );
        }
    }
}
