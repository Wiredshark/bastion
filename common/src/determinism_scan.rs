//! T0.83(b): five detection families the existing scanners (T0.6, T0.79's
//! `rng_source_registry`, `numeric_surface`, `host_input_manifest`,
//! `selection_registry`) don't cover, built on [`crate::scanner_framework`].
//!
//! Scope discipline, matching this row's own instruction: each family is
//! PINNED (the current live set is frozen as a baseline; a completeness
//! test fails on anything new, a staleness test fails on anything that
//! moved or vanished) rather than individually classified site-by-site.
//! Five families times dozens-to-hundreds of sites each is not a budget
//! this row has for prose-per-site the way `selection_registry` got for
//! 58 sites alone; a handful of sites this pass genuinely read (not
//! guessed at) are called out below, but most entries are `NotReviewed`
//! by design, same honesty convention as every registry this session.
//!
//! **Standing limit, stated once here rather than five times below**:
//! these are TEXT scanners, not type-aware ones. `HashMapIteration` in
//! particular cannot distinguish a `HashMap` from a `BTreeMap` (whose
//! `.values()`/`.keys()` iteration IS already canonically ordered) --
//! every entry in that family needs its receiver's actual type checked
//! before it means anything, which this scan does not do. This is the
//! same class of limit `numeric_surface.rs` names for its own pattern
//! list: growth of a KNOWN surface is caught; a genuinely unknown one
//! (a new hazardous pattern nobody named yet) is not.

use crate::scanner_framework::{self, AUTHORITATIVE_SCAN_ROOTS};
use std::path::Path;

/// One family's frozen baseline: the exact set of `(file, snippet,
/// occurrence)` triples this row's scan found, keyed off a pattern list.
struct FamilyV1 {
    name: &'static str,
    patterns: &'static [&'static str],
    self_exempt: &'static str,
    baseline: &'static [(&'static str, &'static str, u32)],
    /// A note on any specific sites within this family that were
    /// actually read (not just found), for the honesty split every
    /// registry this session uses. May be empty -- most families have
    /// no individually-read sites.
    verified_notes: &'static [(&'static str, &'static str)],
}

/// Family 1: `Instant`/`SystemTime::now()` reached from an authoritative
/// crate. The `WeatherLerp` precedent this family generalizes: wall-clock
/// reads inside a decision that two runs must agree on are exactly the
/// class of bug `BASTION_DETERMINISTIC`'s whole rtsim-RNG story exists to
/// close for randomness -- wall-clock time is the same hazard shape.
const INSTANT_SYSTEMTIME_PATTERNS: &[&str] = &["Instant::now()", "SystemTime::now()"];

/// Family 2: `std::collections::hash_map::DefaultHasher` (SipHash,
/// explicitly NOT stable across Rust versions per `std`'s own docs) used
/// as if it produced a portable, reproducible digest.
const DEFAULT_HASHER_PATTERNS: &[&str] = &["DefaultHasher"];

/// Family 3: `fs::read_dir` calls -- directory iteration order is
/// filesystem/OS-dependent, not something Rust guarantees, so a raw
/// `read_dir` result feeding anything order-sensitive needs an explicit
/// sort somewhere before it's used, not an assumption that the OS
/// happens to return entries alphabetically.
const READ_DIR_PATTERNS: &[&str] = &["fs::read_dir("];

/// Family 4: `HashMap`/`HashSet` iteration via `.values()`/`.values_mut()`/
/// `.keys()` -- Rust's default hasher is randomly seeded per process, so
/// iteration order over these containers is NOT stable even within one
/// build, let alone across two runs. See the module doc's standing limit:
/// this pattern also matches `BTreeMap`/`BTreeSet`, whose iteration IS
/// canonically ordered -- every hit needs its receiver's real type
/// checked, which this scan does not do.
const HASHMAP_ITERATION_PATTERNS: &[&str] = &[".values()", ".values_mut()", ".keys()"];

/// Family 5: a raw specs `Entity`'s ECS-assigned id (`.id()`) used as if
/// it were a stable identity -- unlike `Uid` (the game's own stable,
/// synced identity), an `Entity`'s raw id is an allocator slot that can
/// be recycled across a save/load or even within one run as entities are
/// created and destroyed, so it must never be compared, sorted, hashed,
/// or serialized as if it named a persistent thing.
const RAW_ENTITY_ID_PATTERNS: &[&str] = &["entity.id()", ".entity.id()"];

const FAMILIES: &[FamilyV1] = &[
    FamilyV1 {
        name: "InstantSystemTimeInAuthoritativeCode",
        patterns: INSTANT_SYSTEMTIME_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &INSTANT_SYSTEMTIME_BASELINE,
        verified_notes: &[],
    },
    FamilyV1 {
        name: "DefaultHasherInAuthoritativeCode",
        patterns: DEFAULT_HASHER_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &DEFAULT_HASHER_BASELINE,
        verified_notes: &[
            ("common/src/comp/inventory/item/mod.rs", "comment only, naming a hasher this code explicitly moved AWAY from -- zero live usage"),
            ("common/src/state_hash.rs", "comment only, documenting WHY DomainHasher exists instead of DefaultHasher -- zero live usage"),
            ("bastion-server/src/bastion_jobs.rs", "comment only, documenting a past fix -- zero live usage"),
        ],
    },
    FamilyV1 {
        name: "ReadDirWithoutVerifiedSort",
        patterns: READ_DIR_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &READ_DIR_BASELINE,
        verified_notes: &[
            ("server/src/save_inventory.rs", "SAFE, read directly: inventory_save_dir_v1's two read_dir loops both feed one `artifacts` Vec that is sorted by (kind, relative_path) at line 315, before the function returns -- the sort just isn't textually adjacent to either read_dir call, which a narrower scanner would have missed"),
            ("common/src/apex/numeric_surface.rs", "SAFE: this crate's own scanner convention, sorts paths immediately after read_dir (same pattern as scanner_framework's own scan_dir)"),
            ("common/src/host_input_manifest.rs", "SAFE: same scanner convention"),
            ("common/src/rng_source_registry.rs", "SAFE: same scanner convention"),
            ("common/src/scanner_framework.rs", "SAFE: this is the convention itself"),
            ("server/src/semantic_net/receive_inventory.rs", "SAFE: same scanner convention"),
            ("server/src/semantic_net/send_inventory.rs", "SAFE: same scanner convention"),
        ],
    },
    FamilyV1 {
        name: "HashMapIteration",
        patterns: HASHMAP_ITERATION_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &HASHMAP_ITERATION_BASELINE,
        verified_notes: &[],
    },
    FamilyV1 {
        name: "RawEcsEntityIdAsIdentity",
        patterns: RAW_ENTITY_ID_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &RAW_ENTITY_ID_BASELINE,
        verified_notes: &[],
    },
];

// The pinned baselines are declared as separate `const`s (rather than
// inlined into `FAMILIES` above) purely so each can carry its own doc
// comment explaining what was found -- Rust doesn't allow a doc comment
// on an array literal used inline in another const's initializer.

/// 44 sites at pin time. Not individually classified this pass -- every
/// one needs its OWN read to tell "this is a metrics/logging timestamp"
/// (fine) from "this feeds an authoritative eligibility decision" (the
/// `WeatherLerp`-class hazard this family exists to catch).
const INSTANT_SYSTEMTIME_BASELINE: [(&str, &str, u32); 44] = instant_systemtime_baseline();

/// 3 sites at pin time, all verified: comments naming a hasher this code
/// already avoids, zero live usage. A clean family is still worth
/// pinning -- the completeness/staleness tests keep it clean.
const DEFAULT_HASHER_BASELINE: [(&str, &str, u32); 3] = default_hasher_baseline();

/// 16 sites at pin time. 7 verified safe (6 are this program's own
/// scanner-file convention, sort-immediately-after; 1 is
/// save_inventory.rs, sorts before return but not adjacently). 9 not yet
/// reviewed.
const READ_DIR_BASELINE: [(&str, &str, u32); 16] = read_dir_baseline();

/// 184 sites at pin time -- the largest family by far, and per the module
/// doc's standing limit, the least trustworthy one without a type-aware
/// pass: an unknown fraction of these are `BTreeMap`/`BTreeSet` (already
/// canonically ordered) rather than `HashMap`/`HashSet`. Pinned as a
/// growth-detector, not a verdict.
const HASHMAP_ITERATION_BASELINE: [(&str, &str, u32); 184] = hashmap_iteration_baseline();

/// 15 sites, INDIVIDUALLY CLASSIFIED (E11-6a, 2026-07-28). Fourteen are
/// benign for two distinct reasons; one is a real misuse.
///
/// **Benign — storage-slot semantics (4).** `region.rs:220/258` and
/// `sentinel.rs:282/283` use `Entity::id()` as a `BitSet` index — into
/// `tracked_entities` and into specs' own change-sets. That is what an
/// `Entity`'s id IS: a same-run storage slot. It never outlives the run
/// and is never compared across runs.
///
/// **Benign — diagnostic strings (8).** `bastion_jobs.rs`,
/// `entity_manipulation.rs`, `inventory_manip.rs` (×2), `lib.rs`,
/// `state_ext.rs`, `in_game.rs`, `object.rs` — all of the shape
/// `entity = entity.id(),` in a tracing field. A log naming a transient
/// handle is correct; swapping these to `Uid` would cost a lookup to make
/// logs prettier.
///
/// **Benign BY JUDGEMENT, and the two to re-examine first (2).**
/// `entity_sync.rs:468` (`tick + entity.id()`, then `is_multiple_of(32)`)
/// and `weather/tick.rs:206` (`entity.id() % 30 == tick % 30`) stagger
/// work across ticks by allocator slot. This does not reach authoritative
/// state — it changes update/send CADENCE, on the sync path rather than
/// the tick path — but it does make network timing allocation-order
/// dependent. The verdict here is "it doesn't reach state", not "it
/// can't", and that distinction is why these two are named.
///
/// **The one real misuse (1).** `server/src/lib.rs:3318` uses a raw
/// `entity.id()` as the SORT KEY of a returned `Vec`. That is exactly
/// this family's hazard — an allocator slot ordering something — and the
/// same class as the stable-`Uid` canonicalisation `DET-PHY-005` applied
/// to spatial-grid cells. Fix: key the tuple and the sort by `Uid`.
///
/// ---
///
/// **The assumption this family sits on top of, stated here because
/// nowhere else states it (`T0.69`).** `UidAllocator` (`common/src/uid.rs`)
/// is a monotone counter: a new entity's `Uid` is a pure function of how
/// many allocations preceded it. The program's determinism machinery is
/// built ON `Uid` ordering — `DET-PHY-005` canonicalises spatial cells by
/// it, `T6.3`'s tape keys entities by it — so `Uid` stability is a
/// CONSEQUENCE of upstream ordering rows holding (command journal,
/// checkpoint barriers, sorted drains), not a property with its own
/// guard. It holds today. If an upstream ordering ever regresses, the
/// failure presents as permuted `Uid`s and simultaneous divergence in
/// every consumer, with the true cause several rows away. `T0.69` is the
/// row that would make it derived rather than assumed; it is parked
/// behind the buildables with that trigger named.
const RAW_ENTITY_ID_BASELINE: [(&str, &str, u32); 15] = raw_entity_id_baseline();

// Baseline data lives in generated `const fn`s below purely to keep the
// (very long) tuple literals out of the doc-commented declarations above.

const fn instant_systemtime_baseline() -> [(&'static str, &'static str, u32); 44] {
    include!("determinism_scan_baseline_instant.rs")
}
const fn default_hasher_baseline() -> [(&'static str, &'static str, u32); 3] {
    include!("determinism_scan_baseline_default_hasher.rs")
}
const fn read_dir_baseline() -> [(&'static str, &'static str, u32); 16] {
    include!("determinism_scan_baseline_read_dir.rs")
}
const fn hashmap_iteration_baseline() -> [(&'static str, &'static str, u32); 184] {
    include!("determinism_scan_baseline_hashmap_iteration.rs")
}
const fn raw_entity_id_baseline() -> [(&'static str, &'static str, u32); 15] {
    include!("determinism_scan_baseline_raw_entity_id.rs")
}

fn scan_family(workspace_root: &Path, roots: &[&Path], family: &FamilyV1) -> Vec<(String, String, u32)> {
    scanner_framework::scan_lines_matching(workspace_root, roots, family.patterns, family.self_exempt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> std::path::PathBuf {
        scanner_framework::workspace_root_from_manifest_dir(env!("CARGO_MANIFEST_DIR"))
    }

    fn scan_roots() -> Vec<std::path::PathBuf> {
        let root = workspace_root();
        AUTHORITATIVE_SCAN_ROOTS.iter().map(|p| root.join(p)).collect()
    }

    /// One completeness + one staleness check per family -- every live
    /// site is in the baseline, every baseline entry still has a live
    /// site. Runs all 5 families so a single test failure names which
    /// one drifted.
    #[test]
    fn every_family_baseline_is_complete_and_not_stale() {
        let root_bufs = scan_roots();
        let roots: Vec<&Path> = root_bufs.iter().map(|p| p.as_path()).collect();
        let ws_root = workspace_root();

        for family in FAMILIES {
            let live = scan_family(&ws_root, &roots, family);
            let live_set: std::collections::HashSet<(&str, &str, u32)> =
                live.iter().map(|(f, s, i)| (f.as_str(), s.as_str(), *i)).collect();
            let baseline_set: std::collections::HashSet<(&str, &str, u32)> =
                family.baseline.iter().copied().collect();

            let new_sites: Vec<_> = live_set.difference(&baseline_set).collect();
            assert!(
                new_sites.is_empty(),
                "{}: new sites not in the pinned baseline:\n{:#?}",
                family.name,
                new_sites
            );

            let stale: Vec<_> = baseline_set.difference(&live_set).collect();
            assert!(
                stale.is_empty(),
                "{}: baseline entries with no live site:\n{:#?}",
                family.name,
                stale
            );
        }
    }

    /// Falsifier, positive direction: a planted, unbaselined instance of
    /// each family's pattern must be caught.
    #[test]
    fn falsifier_a_planted_site_is_caught_per_family() {
        for family in FAMILIES {
            let Some(pattern) = family.patterns.first() else { continue };
            let dir = std::env::temp_dir().join(format!(
                "determinism_scan_falsifier_{}_{}",
                family.name,
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("planted.rs"), format!("fn f() {{ {pattern}x; }}\n")).unwrap();

            let live = scanner_framework::scan_lines_matching(
                dir.as_path(),
                &[dir.as_path()],
                family.patterns,
                family.self_exempt,
            );
            assert!(!live.is_empty(), "{}: failed to find its own planted pattern", family.name);

            std::fs::remove_dir_all(&dir).ok();
        }
    }

    /// Falsifier, negative direction: a line that does NOT contain any
    /// family's pattern must not be flagged -- proves the scan isn't
    /// vacuously matching everything.
    #[test]
    fn falsifier_an_unrelated_line_is_not_flagged() {
        let dir = std::env::temp_dir()
            .join(format!("determinism_scan_negative_falsifier_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("planted.rs"), "fn totally_unrelated() { 1 + 1; }\n").unwrap();

        for family in FAMILIES {
            let live = scanner_framework::scan_lines_matching(
                dir.as_path(),
                &[dir.as_path()],
                family.patterns,
                family.self_exempt,
            );
            assert!(live.is_empty(), "{}: false-positived on an unrelated line", family.name);
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Every family with a `Complete`-equivalent claim (a verified-safe
    /// note) must still exist in its own baseline -- catches a note that
    /// silently stopped referring to anything real.
    #[test]
    fn every_verified_note_names_a_real_baseline_file() {
        for family in FAMILIES {
            for (file, _note) in family.verified_notes {
                assert!(
                    family.baseline.iter().any(|(f, _, _)| f == file),
                    "{}: verified note for {} names a file not in this family's own baseline",
                    family.name,
                    file
                );
            }
        }
    }

    #[test]
    fn default_hasher_family_is_genuinely_clean() {
        let family = FAMILIES.iter().find(|f| f.name == "DefaultHasherInAuthoritativeCode").unwrap();
        assert_eq!(
            family.baseline.len(),
            family.verified_notes.len(),
            "every DefaultHasher site found is expected to be a verified-safe comment-only \
             mention -- if this fails, a REAL DefaultHasher usage was added and needs its own \
             review, not just a baseline bump"
        );
    }
}
