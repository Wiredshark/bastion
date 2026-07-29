//! T0.83: the shared scanner framework -- the walk/match/occurrence-index
//! primitive that `host_input_manifest`, `selection_registry`, and (from
//! this row on) the new determinism-scan families all delegate to,
//! instead of each hand-rolling the same directory walk.
//!
//! **Migration scope, disclosed rather than forced.** Five scanner
//! precedents exist going into this row: `rng_source_registry`,
//! `numeric_surface` (`common/src/apex/`), `host_input_manifest`,
//! `selection_registry`, and `semantic_net`'s receive/send catalogs
//! (`server/src/semantic_net/`). Only `selection_registry` is migrated
//! onto this framework this row -- its walk/match/occurrence-index code
//! was a byte-for-byte duplicate of what this framework now holds once,
//! a genuinely mechanical lift.
//!
//! The other four are NOT migrated, for two different reasons:
//! - `numeric_surface.rs` and the `semantic_net` catalogs: collision
//!   risk, not migration cost. `numeric_surface.rs` is under active edit
//!   on this same integration tip this session (T6.1c, T6.1d landed
//!   during this very batch); the `semantic_net` catalogs are an
//!   established, working convention neither built nor reviewed by this
//!   lane. Touching either risks a real collision with in-flight work.
//! - `host_input_manifest`: attempted, and found NOT mechanical on
//!   inspection, which is itself the finding worth recording. Its scan
//!   does two things `selection_registry`'s doesn't: it EXTRACTS the
//!   quoted variable name out of each matching line (not just the whole
//!   trimmed line), and it DEDUPLICATES to a `HashSet<(file, var)>` (one
//!   row per variable per file, however many lines mention it) rather
//!   than keeping every occurrence with a distinct index. Those are two
//!   different output contracts, not one algorithm with two callers --
//!   forcing it onto `scan_lines_matching` would mean re-deriving the
//!   extraction-and-dedup layer on top anyway, which is the whole
//!   scanner, not a mechanical delegation. Left in place, disclosed.
//! - `rng_source_registry`: pre-existing, independently evolving, no
//!   current collision risk migrating it would reduce -- left in place
//!   for the same reason as `numeric_surface`/`semantic_net`, just
//!   without the active-edit urgency.
//!
//! Per this row's own instruction ("migrate ONLY if mechanical... if
//! migration balloons, leave them in place and share only the exemption
//! format + root list, disclosed") -- this is that disclosure, for four
//! different reasons across four files rather than one blanket excuse.
//!
//! What IS shared going forward, even for the four un-migrated scanners:
//! the [`AUTHORITATIVE_SCAN_ROOTS`] root list (which `E13` is widening
//! one crate at a time -- see the constant) and the [`ExemptionEntryV1`]
//! shape for recording a deliberate bypass (site + reason + owner +
//! revisit condition) -- a convention, not a shared type those files are
//! forced to import.

use std::{collections::HashMap, fs, path::Path};

/// Every directory an authoritative-crate scanner walks, named once so a
/// new scanner doesn't have to rediscover the list. The first five are
/// what `rng_source_registry`, `numeric_surface`, `host_input_manifest`
/// and `selection_registry` were built against; the rest are `E13`'s
/// widening, each recorded with what it added.
pub const AUTHORITATIVE_SCAN_ROOTS: [&str; 9] = [
    "common/src",
    "server/src",
    "rtsim/src",
    "bastion-server/src",
    "world/src",
    // E13 chunk 1: `common/net/src` was NOT in the original five, which
    // meant the wire crate -- where WSG's own goldens live, and every
    // message the server decodes -- was outside every determinism scan
    // this program built. Same class of gap as T6.1c's root-set finding:
    // widening patterns would never have revealed it, because the files
    // were never walked.
    "common/net/src",
    // E13 chunk 2: `server/agent/src` -- 13,974 lines of NPC decision
    // logic, previously unwalked. It adds ZERO sites to all five
    // families, and that zero is a claim rather than an absence: the
    // crate contains no hash-container type at all, no wall-clock
    // reads, and no raw-entity-index calls. It reads the world through
    // ECS storages and its own `AgentData`, so the container-ordering
    // and wall-clock families have nothing to bite on here.
    //
    // (Named indirectly on purpose: spelling those tokens literally
    // made THIS COMMENT a scan hit on the first regeneration. A
    // scanner whose root list is prose is a scanner that indexes its
    // own documentation -- `rtsim/data/mod.rs`'s DET-RNG-009 gate hit
    // the same wall and answered it by building needles at runtime.)
    //
    // Because a zero-delta root leaves no trace in any baseline file,
    // the baselines cannot evidence that this root is walked at all.
    // Five plant-falsifications did -- one per family, each in this
    // root -- and that is the only reason the zero is trustworthy.
    "server/agent/src",
    // E13 chunk 3: `common/state/src` -- the ECS state container and the
    // whole plugin subsystem. Unlike chunk 2 this root is NOT empty: it
    // adds 6 family sites and 1 selection site, every one classified on
    // the baselines below.
    //
    // Worth knowing for anyone reading a hit here: most of this crate's
    // surface is behind the non-default `plugins` feature, and these
    // scans are TEXT-based over whole files, not cfg-aware. That is a
    // feature, not a limitation -- a hazard that only compiles under an
    // opt-in feature is still a hazard, and a cfg-aware scan would go
    // blind exactly where the coverage is thinnest.
    "common/state/src",
    // E13 chunk 4: `common/systems/src` -- the authoritative simulation
    // systems themselves (physics, buffs, melee, projectiles). Five
    // family sites, and the richest chunk of the campaign by findings
    // per site: see `HASHMAP_ITERATION_BASELINE` and
    // `RAW_ENTITY_ID_BASELINE` for two live hazards this root exposed on
    // its first scan.
    //
    // It also exposed a SECOND scanner's root-set gap. This list is
    // shared, but `rng_source_registry` keeps its own hardcoded four
    // (`common/src`, `server/src`, `server/agent/src`, `rtsim/src`) --
    // so the RNG audit has never walked this crate either, and seven
    // live `rand::rng()` call sites sit here unaudited while a
    // migration onto `combat::seed_ability_rng` is visibly half-done
    // (`beam.rs` converted, its six siblings not). Two scanners with
    // two root lists is how a crate stays invisible to both.
    "common/systems/src",
];

/// A deliberate, reviewed bypass of a scanner's own rule -- the shared
/// shape for what every scanner's exemption list should record, whether
/// or not the scanner's own code imports this type. Four fields, every
/// one required: WHERE (a scanner needs this to know what to skip), WHY
/// (a reviewer needs this to judge whether the bypass still makes sense),
/// WHO owns the judgment call, and WHEN it should be revisited -- an
/// exemption with no revisit condition is a permanent one wearing a
/// temporary label.
#[derive(Copy, Clone, Debug)]
pub struct ExemptionEntryV1 {
    pub file: &'static str,
    pub reason: &'static str,
    pub owner: &'static str,
    pub revisit_condition: &'static str,
}

/// Re-scans the given directories (each some `<crate>/src`, resolved
/// against `workspace_root`) right now for every line containing ANY of
/// `patterns`, returning `(file relative to workspace_root, trimmed line
/// text, 0-based occurrence index within that file)` triples -- the same
/// shape `selection_registry`, `host_input_manifest`, and
/// `rng_source_registry` each derived independently before this row.
///
/// `self_exempt_suffix` is a substring of the scanning module's own
/// file(s) -- typically its filename (e.g. `"my_registry.rs"`), or a
/// shared prefix (e.g. `"determinism_scan"`) when a scanner splits its
/// baseline data across several sibling files: a scanner whose own
/// source (or generated baseline data) quotes its patterns as string
/// literals would otherwise flag itself. Matched with `contains`, not
/// `ends_with`, so one substring can cover a whole family of files.
pub fn scan_lines_matching(
    workspace_root: &Path,
    dirs: &[&Path],
    patterns: &[&str],
    self_exempt_suffix: &str,
) -> Vec<(String, String, u32)> {
    let mut out = Vec::new();
    for dir in dirs {
        scan_dir(workspace_root, dir, patterns, self_exempt_suffix, &mut out);
    }
    out.sort();
    out
}

fn scan_dir(
    base: &Path,
    dir: &Path,
    patterns: &[&str],
    self_exempt_suffix: &str,
    out: &mut Vec<(String, String, u32)>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            scan_dir(base, &path, patterns, self_exempt_suffix, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            scan_file(base, &path, patterns, self_exempt_suffix, out);
        }
    }
}

fn scan_file(
    base: &Path,
    path: &Path,
    patterns: &[&str],
    self_exempt_suffix: &str,
    out: &mut Vec<(String, String, u32)>,
) {
    let Ok(contents) = fs::read_to_string(path) else { return };
    let rel = path.strip_prefix(base).unwrap_or(path).to_string_lossy().replace('\\', "/");
    // `contains`, not `ends_with`: a scanner that splits its own baseline
    // data across sibling files (e.g. `determinism_scan`'s per-family
    // `determinism_scan_baseline_*.rs` includes) needs one distinguishing
    // substring to exempt the whole family, not just its own exact
    // filename -- those baseline files quote every pattern as string
    // data, which `ends_with`-only exemption would flag as fresh hits.
    if rel.contains(self_exempt_suffix) {
        return;
    }
    let mut occurrence: HashMap<String, u32> = HashMap::new();
    for line in contents.lines() {
        if patterns.iter().any(|p| line.contains(p)) {
            let snippet = line.trim().to_string();
            let idx = occurrence.entry(snippet.clone()).or_insert(0);
            out.push((rel.clone(), snippet, *idx));
            *idx += 1;
        }
    }
}

/// The workspace root, resolved from any crate whose `Cargo.toml` sits
/// one directory below it -- every scanner's own test module needs this,
/// so it lives here once rather than five times.
pub fn workspace_root_from_manifest_dir(manifest_dir: &str) -> std::path::PathBuf {
    Path::new(manifest_dir).join("..").canonicalize().expect("workspace root must resolve")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_a_planted_matching_line() {
        let dir = std::env::temp_dir()
            .join(format!("scanner_framework_falsifier_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("planted.rs"), "fn f() { NEEDLE_PATTERN; }\n").unwrap();

        let found = scan_lines_matching(dir.as_path(), &[dir.as_path()], &["NEEDLE_PATTERN"], "nope.rs");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, "fn f() { NEEDLE_PATTERN; }");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_does_not_find_an_absent_pattern() {
        let dir = std::env::temp_dir()
            .join(format!("scanner_framework_negative_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("planted.rs"), "fn f() { other_thing(); }\n").unwrap();

        let found =
            scan_lines_matching(dir.as_path(), &[dir.as_path()], &["NEEDLE_PATTERN"], "nope.rs");
        assert!(found.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn repeated_occurrences_in_one_file_get_distinct_indices() {
        let dir =
            std::env::temp_dir().join(format!("scanner_framework_occurrence_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("planted.rs"), "NEEDLE;\nNEEDLE;\nNEEDLE;\n").unwrap();

        let mut found =
            scan_lines_matching(dir.as_path(), &[dir.as_path()], &["NEEDLE"], "nope.rs");
        found.sort_by_key(|(_, _, i)| *i);
        assert_eq!(found.iter().map(|(_, _, i)| *i).collect::<Vec<_>>(), vec![0, 1, 2]);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// `E12-a-fix`: `AUTHORITATIVE_SCAN_ROOTS` (and whatever narrower root
    /// list any given scanner passes in) functions as a declared-list
    /// exemption -- anything outside the roots actually handed to
    /// `scan_lines_matching` is invisible to a scan by construction,
    /// whether or not that code is itself authoritative. Falsifier: plant
    /// a hazard pattern in an "intruder" directory that sits right next
    /// to a declared root on disk but was never named as one, and confirm
    /// it is NOT found -- proving the boundary actually discriminates
    /// rather than the walk accidentally reaching everywhere (e.g. via a
    /// parent-directory escape).
    #[test]
    fn an_intruder_directory_outside_the_declared_roots_is_invisible_to_the_scan() {
        let base = std::env::temp_dir()
            .join(format!("scanner_framework_intruder_falsifier_{}", std::process::id()));
        let scanned_root = base.join("scanned");
        let intruder_root = base.join("intruder");
        std::fs::create_dir_all(&scanned_root).unwrap();
        std::fs::create_dir_all(&intruder_root).unwrap();
        std::fs::write(scanned_root.join("in_scope.rs"), "fn f() { NEEDLE_PATTERN; }\n").unwrap();
        std::fs::write(intruder_root.join("out_of_scope.rs"), "fn g() { NEEDLE_PATTERN; }\n").unwrap();

        // Only `scanned_root` is declared -- `intruder_root` sits right
        // next to it on disk but was never named as a root.
        let found = scan_lines_matching(
            base.as_path(),
            &[scanned_root.as_path()],
            &["NEEDLE_PATTERN"],
            "nope.rs",
        );

        assert_eq!(found.len(), 1, "expected exactly the declared root's hit, found: {:?}", found);
        assert_eq!(found[0].0, "scanned/in_scope.rs");
        assert!(
            !found.iter().any(|(f, _, _)| f.contains("intruder")),
            "the intruder directory outside the declared roots must not be scanned"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_scanner_naming_its_own_file_is_exempted() {
        let dir =
            std::env::temp_dir().join(format!("scanner_framework_self_exempt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("self_scanner.rs"), "const P: &str = \"NEEDLE\";\n").unwrap();

        let found = scan_lines_matching(
            dir.as_path(),
            &[dir.as_path()],
            &["NEEDLE"],
            "self_scanner.rs",
        );
        assert!(found.is_empty(), "the scanner's own file must be exempted, not flagged");

        std::fs::remove_dir_all(&dir).ok();
    }
}
