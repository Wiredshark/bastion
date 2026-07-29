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
///
/// `pub`, and `baseline_file` added, for `E14-4`: the regen tool
/// (`src/bin/determinism_scan_regen.rs`) needs exactly this metadata
/// (which patterns, which self-exemption, which file to write) to stay
/// in sync with the live scan by construction, rather than hand-
/// duplicating a second table that could drift from this one.
pub struct FamilyV1 {
    pub name: &'static str,
    pub patterns: &'static [&'static str],
    pub self_exempt: &'static str,
    pub baseline: &'static [(&'static str, &'static str, u32)],
    /// A note on any specific sites within this family that were
    /// actually read (not just found), for the honesty split every
    /// registry this session uses. May be empty -- most families have
    /// no individually-read sites.
    pub verified_notes: &'static [(&'static str, &'static str)],
    /// The `determinism_scan_baseline_*.rs` file this family's baseline
    /// lives in, relative to `common/src` -- `E14-4`'s own field, unused
    /// by the live scan itself (which reads `baseline` above, already
    /// compiled in), read only by the regen tool.
    pub baseline_file: &'static str,
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

pub const FAMILIES: &[FamilyV1] = &[
    FamilyV1 {
        name: "InstantSystemTimeInAuthoritativeCode",
        patterns: INSTANT_SYSTEMTIME_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &INSTANT_SYSTEMTIME_BASELINE,
        verified_notes: &[],
        baseline_file: "determinism_scan_baseline_instant.rs",
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
        baseline_file: "determinism_scan_baseline_default_hasher.rs",
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
            ("common/state/src/plugin/mod.rs", "SAFE, read directly (E13 chunk 3): PluginMgr::from_dir's read_dir feeds `discovered`, and the OS order is captured as `discovery_ordinal` -- explicitly labelled `ordinal = legacy provenance, never priority` (APEX-T2.1.05), carried with a DiscoveryOrderIsLegacy warning on every record, and never used as a sort key or tiebreak anywhere in the crate (checked). Every ordering runs through canonical_plugin_order(.., |p| p.hash) by CONTENT HASH at commit_new_manager (DET-AST-024/025). Filesystem order is recorded as provenance and then superseded, which is the right shape: it neither trusts nor discards what the OS said"),
            ("server/src/semantic_net/receive_inventory.rs", "SAFE: same scanner convention"),
            ("server/src/semantic_net/send_inventory.rs", "SAFE: same scanner convention"),
        ],
        baseline_file: "determinism_scan_baseline_read_dir.rs",
    },
    FamilyV1 {
        name: "HashMapIteration",
        patterns: HASHMAP_ITERATION_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &HASHMAP_ITERATION_BASELINE,
        verified_notes: &[],
        baseline_file: "determinism_scan_baseline_hashmap_iteration.rs",
    },
    FamilyV1 {
        name: "RawEcsEntityIdAsIdentity",
        patterns: RAW_ENTITY_ID_PATTERNS,
        self_exempt: "determinism_scan",
        baseline: &RAW_ENTITY_ID_BASELINE,
        verified_notes: &[],
        baseline_file: "determinism_scan_baseline_raw_entity_id.rs",
    },
];

// The pinned baselines are declared as separate `const`s (rather than
// inlined into `FAMILIES` above) purely so each can carry its own doc
// comment explaining what was found -- Rust doesn't allow a doc comment
// on an array literal used inline in another const's initializer.

/// 45 sites at pin time (44 + 1, `E11-2b`-era pin; `APEX-T4.3`'s
/// `detected_at_unix_seconds` field is a diagnostic timestamp written
/// into a sidecar file for OPERATOR information only -- never read back
/// into the simulation, never feeds RNG or any deterministic decision --
/// the "metrics/logging timestamp" case this family's own doc names as
/// fine, not the `WeatherLerp`-class hazard). Not individually classified
/// this pass otherwise -- every other site needs its OWN read to tell
/// "this is a metrics/logging timestamp" (fine) from "this feeds an
/// authoritative eligibility decision" (the hazard this family exists to
/// catch).
///
/// **E13 chunk 2 (2026-07-29) -- this family's chunk-1 result was
/// VACUOUS and is now re-established.** Chunk 1 reported instant,
/// default_hasher and read_dir "unchanged, itself evidence the wire
/// crate added no such surface". For default_hasher and read_dir that
/// was true. For THIS family it was not evidence of anything: the
/// regeneration wrote to `..._baseline_instant_systemtime.rs`, an
/// orphan no `include!` names (see `no_baseline_file_is_an_orphan`), so
/// the comparison was a file against itself while the real baseline sat
/// untouched. The conclusion happens to hold -- this baseline contains
/// zero `common/net/src` entries, checked directly -- but it holds by
/// luck, not by the evidence given for it. Re-verified here against the
/// file that actually compiles.
///
/// **E14-4 (2026-07-29): +23, all self-catches, all benign.** Committing
/// `baseline_regen.rs`'s own test suite into `common/src` (an
/// authoritative scan root) put the literal strings `"Instant::now()"`
/// and `"SystemTime::now()"` into 23 test-fixture string literals --
/// same self-catching pattern as every prior instance in this file, not
/// a new live site. Regenerated with the row's own new tool
/// (`determinism_scan_regen`), which is itself the first real proof the
/// tool works: a pure addition, applied automatically, that touched no
/// existing line (verified via `git diff --stat` showing insertions
/// only). Landed on top of `E13` chunk 5's own +4 (`query_server`
/// root), so the array grows 50 -> 54 -> 77.
const INSTANT_SYSTEMTIME_BASELINE: [(&str, &str, u32); 77] = instant_systemtime_baseline();

/// 3 sites at pin time, all verified: comments naming a hasher this code
/// already avoids, zero live usage. A clean family is still worth
/// pinning -- the completeness/staleness tests keep it clean.
const DEFAULT_HASHER_BASELINE: [(&str, &str, u32); 3] = default_hasher_baseline();

/// 16 sites at pin time. 7 verified safe (6 are this program's own
/// scanner-file convention, sort-immediately-after; 1 is
/// save_inventory.rs, sorts before return but not adjacently). 9 not yet
/// reviewed.
const READ_DIR_BASELINE: [(&str, &str, u32); 17] = read_dir_baseline();

/// 184 sites at pin time -- the largest family by far, and per the module
/// doc's standing limit, the least trustworthy one without a type-aware
/// pass: an unknown fraction of these are `BTreeMap`/`BTreeSet` (already
/// canonically ordered) rather than `HashMap`/`HashSet`. Pinned as a
/// growth-detector, not a verdict.
/// **E13 chunk 1 (2026-07-29): `common/net/src` added to the scan roots.**
/// The original five roots omitted the WIRE crate -- where WSG's 88
/// goldens live and every message the server decodes is defined. Same
/// class of gap as `T6.1c`'s root-set finding: no amount of pattern
/// widening reveals a file that is never walked.
///
/// Six new live sites, all classified:
///
/// - `msg/checkpoint.rs` x2 (`staged.keys()`, `staged.values()`) and
///   `msg/command.rs` x2 (`active.keys().next_back()`,
///   `active.values().any(..)`): **BENIGN -- the receivers are
///   `BTreeMap`** (`checkpoint.rs:1252`, `command.rs:1219`), which is
///   canonically ordered. Exactly the false-positive class this family's
///   own doc warns it cannot distinguish without a type-aware pass.
/// - `msg/compression.rs` `.and_then(|h| h.keys().next())`: a REAL
///   `hashbrown::HashMap`, and `.next()` takes an ARBITRARY key. **Benign
///   today only because it is unreachable**: it sits behind
///   `if AVERAGE_PALETTE`, and the sole instantiation in the tree is
///   `TriPngEncoding<false>` (`msg/server.rs:154`). Dead-by-const-generic,
///   not dead-by-correctness -- flipping that bool would make terrain
///   palette colour depend on hash order. **The one to re-examine first.**
/// - `sync/track.rs:143` `let id = entity.id();`: **BENIGN-LOCAL**, a
///   `BitSet` index into specs' own `modified`/`inserted`/`removed`
///   change-sets -- the same storage-slot semantics classified in
///   `E11-6a`, not an identity.
///
/// **E14-1 (2026-07-29) -- the chunk-4 fire-spread hazard FIXED (net -1
/// +3, 193 -> 195).** `buff.rs`'s `touch_entities.keys()` walk (the site
/// classified NOT BENIGN above) is replaced by a collect-then-
/// `sort_unstable()` over `Uid` immediately before the loop -- the same
/// SAFE shape `module.rs` already established for this family (`.keys()`
/// collected and sorted the very next line). The per-target
/// `rng.random_bool` draw that made the old walk's non-determinism
/// OBSERVABLE (E13 chunk 4's "different SET catches fire" finding) is a
/// separate hazard in a different scanner (`rng_source_registry`'s
/// `UnmitigatedAuthoritativeEntropy`) and is fixed in the same commit --
/// see that registry's own note. +3 here: the fixed line itself (now
/// SAFE, matching the family's own sort-immediately convention) plus 2
/// self-catching comments in this fix's own prose.
const HASHMAP_ITERATION_BASELINE: [(&str, &str, u32); 195] = hashmap_iteration_baseline();

/// 19 sites (15 at E11-6a pin time, 2026-07-28; +4 net after E11-6b's
/// fix -- the one real misuse below was corrected in place, its old line
/// replaced by the fixed `.map(...)` call, plus 4 new sites purely from
/// prose comments that name `entity.id()` while explaining the fix).
/// Eighteen are benign for three distinct reasons; the historical misuse
/// is now fixed.
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
/// **Fixed misuse, now benign (1 site + 4 prose mentions).**
/// `server/src/lib.rs` used a raw `entity.id()` as the SORT KEY of a
/// returned `Vec` — exactly this family's hazard, the same class as the
/// stable-`Uid` canonicalisation `DET-PHY-005` applied to spatial-grid
/// cells. `E11-6b` fixed it: the join now also carries `Uid`, and the
/// sort key moved to `Uid` via a pure, independently-tested
/// `sort_persistent_item_snapshots_by_uid_v1`. The `.map(...)` call still
/// reads `entity.id()` into the tuple (matching this family's textual
/// pattern), but that field is now discarded by the sort helper and never
/// orders anything — the other 4 new sites are doc/inline comments
/// explaining the fix, prose only.
///
/// **E13 chunk 4 -- A SECOND INSTANCE OF THE MISUSE `E11-6b` FIXED (2
/// sites, `common/systems/src/phys/mod.rs`).** The crate was outside the
/// root set until this chunk, which is the only reason it survived
/// `E11-6b`'s sweep.
///
/// `land_on_grounds.sort_unstable_by_key(|(entity, ..)| entity.id())`
/// canonicalises the AUTHORITATIVE fall-damage payload before emission,
/// and the sibling line keys outcome batches by the same raw id. The
/// INTENT is right and already documented (`T0.28`/`DET-EVT-010`): the
/// rayon fold/reduce above concatenates per-split vecs, so without a
/// sort the emission order is thread partitioning. The sort fixes that.
///
/// The KEY is the weak one. `Entity::id()` is an allocator slot, and
/// this program's canonical ordering identity is `Uid` (`DET-PHY-005`
/// canonicalises spatial cells by it; `T6.3`'s tape keys entities by
/// it). Two runs agree only while entity ALLOCATION order agrees --
/// which is a stronger assumption than `Uid` needs, and it is precisely
/// the assumption a save/load cycle breaks, since slots are recycled
/// while `Uid`s are not. Fall-damage emission order decides who dies
/// first when two entities land on the same tick.
///
/// **The fix is cheap and its idiom is already in this file**:
/// `read.uids` is in scope (`phys/mod.rs:167`) and lines 389 and 716
/// already sort by `read.uids.get(e).map(|u| u.0.get()).unwrap_or(u64::
/// MAX)`. Classified here, not fixed -- `E11-6a` classified and `E11-6b`
/// fixed, and this chunk is the classification half of that same split.
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
///
/// **E14-2 (2026-07-29) -- the chunk-4 second instance FIXED (net -2 +3,
/// 22 -> 23).** Both sort sites now consume one Uid-derived sort key
/// (`read.uids.get(entity).map(|u| u.0.get()).unwrap_or(u64::MAX)`,
/// the same idiom `DET-PHY-005` already established in this file)
/// captured ONCE at the fold's map stage and threaded to both
/// consumers, rather than each independently re-deriving `entity.id()`
/// -- exactly the fix this doc block above already named as cheap and
/// already-idiomatic. The 2 real call sites are gone; +3 are this
/// fix's own explanatory prose mentioning `entity.id()` as literal
/// text, same self-catching pattern as every prior instance in this
/// file.
const RAW_ENTITY_ID_BASELINE: [(&str, &str, u32); 23] = raw_entity_id_baseline();

// Baseline data lives in generated `const fn`s below purely to keep the
// (very long) tuple literals out of the doc-commented declarations above.

const fn instant_systemtime_baseline() -> [(&'static str, &'static str, u32); 77] {
    include!("determinism_scan_baseline_instant.rs")
}
const fn default_hasher_baseline() -> [(&'static str, &'static str, u32); 3] {
    include!("determinism_scan_baseline_default_hasher.rs")
}
// The baseline files these pull in are NOT purely machine-owned. Builders
// annotate individual entries in place with `//` notes (`T4.6` chunk 3b
// did, explaining a test-only poll loop), and a blind full-file
// regeneration deletes those notes while the entry set stays identical --
// so the diff reads as a baseline SHRINK and the classification work is
// gone. Any regeneration must preserve `//` lines, or stop and hand the
// merge back to a human. Caught in `E13` chunk 2, on a rebase, by six
// deleted comment lines that no scan result explained.
const fn read_dir_baseline() -> [(&'static str, &'static str, u32); 17] {
    include!("determinism_scan_baseline_read_dir.rs")
}
const fn hashmap_iteration_baseline() -> [(&'static str, &'static str, u32); 195] {
    include!("determinism_scan_baseline_hashmap_iteration.rs")
}
const fn raw_entity_id_baseline() -> [(&'static str, &'static str, u32); 23] {
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

    /// No baseline file may sit in `common/src` without an `include!`
    /// naming it.
    ///
    /// **E13 chunk 2, written because chunk 1 created exactly such a
    /// file.** The regeneration script wrote
    /// `determinism_scan_baseline_instant_systemtime.rs` -- named after
    /// the FAMILY (`InstantSystemTimeInAuthoritativeCode`) rather than
    /// after the live `include!`, which is
    /// `determinism_scan_baseline_instant.rs`. Nothing compiled the
    /// orphan, so nothing could fail on it: the script reported five
    /// baselines regenerated, the suite stayed green, and the family's
    /// REAL baseline went untouched for a whole chunk. A pin that
    /// nothing reads is indistinguishable from a pin that holds, which
    /// is the precise failure this program exists to make impossible.
    ///
    /// The self-exemption makes it worse rather than better: every
    /// scanner skips paths containing `determinism_scan`, so an orphan
    /// baseline is invisible to the scanners too. It is dead data that
    /// looks exactly like live data from every direction except this
    /// test.
    #[test]
    fn no_baseline_file_is_an_orphan() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let this_file = include_str!("determinism_scan.rs");

        let mut orphans: Vec<String> = std::fs::read_dir(&src)
            .expect("common/src must be readable")
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().to_string_lossy().into_owned();
                let is_baseline =
                    name.starts_with("determinism_scan_baseline_") && name.ends_with(".rs");
                let included = this_file.contains(&format!("include!(\"{name}\")"));
                (is_baseline && !included).then_some(name)
            })
            .collect();
        orphans.sort();

        assert!(
            orphans.is_empty(),
            "baseline file(s) that no include! names -- dead data no build reads:\n{orphans:#?}"
        );
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
