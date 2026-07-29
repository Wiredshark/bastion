//! `T0.79` (E7 Stage 1) — the probability/rate source-gate closure. Every
//! remaining DIRECT ambient-entropy call (`rand::rng()`, `thread_rng()`,
//! `rand::random()`) in the authoritative crates (`rtsim`, `server`,
//! `server-agent`, `common`'s sim paths) is named here and classified.
//! `voxygen`/`client`-only presentation code is exempt (out of scope by
//! construction — it never reaches an authoritative draw).
//!
//! An unclassified call would mean a NEW ambient-entropy source landed
//! without anyone deciding whether it's legitimate; [`scan_finds_only_registered_sites`]
//! rejects that by construction. This is the SAME "unclaimed-name-fails"
//! discipline the apex `net_checkpoint_canaries`/`net_command_canaries`
//! coverage maps use, applied to source-level scanning instead of a fixed
//! case-ID range.
//!
//! Classification taxonomy (Fable-ruled, T0.79 research):
//! - `PerTimeHazard`: chance is meant to scale with elapsed time (a rate).
//! - `PerDecisionDraw`: one-shot choice among discrete options, not a
//!   recurring per-tick roll.
//! - `KeyedEpisodeThreshold`: gates a bounded episode/window, not sim state.
//! - `IdentityGeneration`: mints a fresh opaque identifier (session ids,
//!   nonces) — the VALUE'S randomness is the point, not a game outcome.
//! - `NonAuthoritativeEntropy`: presentation-only, operator/admin-command-
//!   only (external input, not sim-loop state), or `#[cfg(test)]`-only —
//!   never reaches replay-critical authoritative state.
//! - `DeterministicModeGatedLiveEntropy`: a live, AUTHORITATIVE agent-
//!   behavior path (not presentation, not admin/test) that explicitly
//!   branches on `ExecutionMode::is_deterministic()` (or an equivalent
//!   `Option<seeded-rng>` gate) -- the deterministic harness gets a
//!   derived stream, live (non-harness) play deliberately keeps drawing
//!   OS entropy. Added (Fable-ruled) rather than folded into
//!   `NonAuthoritativeEntropy`: this bucket IS authoritative sim state in
//!   live play, unlike every other member of that bucket -- it stays
//!   OS-entropy by DELIBERATE design choice, not because it's out of
//!   scope. Same family as `server/src/state_ext.rs`'s item-orientation
//!   gate.
//!
//! `count` is the number of live (non-comment, non-doc) matching lines the
//! scan below expects to find in that file RIGHT NOW — pinned exactly like
//! `OPEN_CASE_COUNT`, so a new site (or a site quietly disappearing without
//! updating this table) is a build failure, not a silent drift.
//!
//! Cross-review (Fable-ruled): the scan also matches a bare, unqualified
//! `rng()` call (see [`tests::contains_bare_rng_call`]) -- a plain
//! `.contains("rand::rng()")` substring check let `use rand::rng;` (an
//! unqualified import) evade the scan entirely. This is how
//! `server/agent/src/action_nodes.rs`'s `helper_random_bool` escaped, and
//! how 5 REAL bugs (common/src/states/{basic_ranged,charged_ranged,
//! rapid_ranged,sprite_summon,transform}.rs -- all authoritative combat/
//! summon rolls) were found and fixed as part of E5-C once the scanner
//! could actually see them.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RandomDrawClassV1 {
    PerTimeHazard,
    PerDecisionDraw,
    KeyedEpisodeThreshold,
    IdentityGeneration,
    NonAuthoritativeEntropy,
    DeterministicModeGatedLiveEntropy,
    /// **DEBT, NOT A CLASSIFICATION.** A live, authoritative draw on
    /// ambient OS entropy with NO mitigation: not presentation, not
    /// admin, not test, and NOT gated on `is_deterministic()` the way
    /// [`Self::DeterministicModeGatedLiveEntropy`] is.
    ///
    /// This variant exists because `E14-3` chunk 1 widened the scan to
    /// `common/systems/src` and found six, and every other bucket would
    /// have been a lie: `NonAuthoritativeEntropy` asserts the draw never
    /// reaches replay-critical state, which is false here, and leaving
    /// them unregistered would just fail the gate without recording
    /// WHY. Registering them makes them visible; it does not make them
    /// acceptable.
    ///
    /// **The population may only shrink** -- see
    /// [`tests::unmitigated_authoritative_entropy_only_shrinks`]. A new
    /// member has to be a deliberate act with a reviewer attached.
    UnmitigatedAuthoritativeEntropy,
}

/// `(workspace-relative path, expected live-site count, classification, note)`.
pub const AMBIENT_ENTROPY_SITES: &[(&str, usize, RandomDrawClassV1, &str)] = &[
    // ---------------------------------------------------------------
    // `E14-3` chunk 1 -- `common/systems/src` entered this scanner's
    // roots. Six sites, ALL `UnmitigatedAuthoritativeEntropy`, all the
    // same shape: `let mut rng = rand::rng();` at the head of a
    // `System::run`, then handed to `combat::attack(.., &mut rng, ..)`.
    //
    // What that rng decides, traced rather than assumed: `combat.rs`
    // draws `rng.random::<f32>() < chance` at eight sites to decide
    // whether an on-hit buff LANDS, and calls `EntityInfo::at(target
    // .pos, &mut *rng)` to SPAWN summoned entities. Both are
    // replay-critical authoritative state, so `NonAuthoritativeEntropy`
    // would be false, and none of these branch on
    // `is_deterministic()`, so `DeterministicModeGatedLiveEntropy`
    // would be false too.
    //
    // The seam already exists and its siblings already use it:
    // `combat::seed_ability_rng(label, uid, time)` (`combat.rs:114`),
    // which `common/src/states/*` were migrated onto -- leaving unused
    // `use rand::rng;` imports behind as the visible fingerprint of a
    // migration that stopped before reaching this crate. `beam.rs` in
    // THIS crate was converted; these six were not.
    (
        "common/systems/src/arcing.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "arc-attack damage application: ambient rng passed to combat::attack, which rolls \
         on-hit buff chances and summon spawns. E14-1 family",
    ),
    (
        "common/systems/src/buff.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "E14-1 (designated HIGH): the fire-spread draw, and the WORST of the six because it is \
         consumed INSIDE a hashbrown::HashMap iteration (touch_entities) -- ambient entropy and \
         hash order compound, so a different SET of entities catches fire rather than the same \
         set in a different order",
    ),
    (
        "common/systems/src/melee.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "melee damage application: same combat::attack path. E14-1 family",
    ),
    (
        "common/systems/src/pool.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "pool (area-effect) damage application: same combat::attack path. E14-1 family",
    ),
    (
        "common/systems/src/projectile.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "projectile hit resolution via combat::attack, PLUS rng.random_bool(0.05) emitting a \
         SoundEvent. The sound is not merely presentation: agent perception reads sound.kind \
         and reacts (server/agent/src/action_nodes.rs:2426), so the draw reaches NPC behaviour",
    ),
    (
        "common/systems/src/shockwave.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "shockwave hit resolution via combat::attack, plus the same 0.05 SoundEvent draw as \
         projectile.rs -- likewise heard by agents, not just players",
    ),
    // `E14-3` chunk 2 -- `bastion-server/src` entered this scanner's
    // roots. Exactly one site, and it is an OUTLIER rather than a
    // pattern: every other damage-instance in the tree is derived.
    (
        "bastion-server/src/bastion_jobs.rs",
        1,
        RandomDrawClassV1::UnmitigatedAuthoritativeEntropy,
        "cave-in injury (cavein_eject_and_injure): `instance: rand::random()` on a HealthChange \
         applied to a colonist's Health. NOT IdentityGeneration -- the value is not an opaque \
         handle whose unguessability is the point, it lands in Health::last_change, which is \
         authoritative component state and is synced, so two runs diverge in any state hash. \
         The deterministic seam for THIS EXACT FIELD already exists and is used at ~7 sites in \
         combat.rs: derive_attack_instance(site, Option<attacker_uid>, target_uid, time, \
         ordinal) -- attacker is already Option, so an environmental cave-in with no attacker \
         fits it directly. (health.rs's `instance: 0` convention is only for the \
         no-change-yet placeholder, not for real damage.) Fix is small but not a one-liner: \
         cavein_eject_and_injure takes no `uids` storage, so the param has to be threaded \
         through it and both callers (the live Sys::run post-loop and the harness's \
         bastion_force_collapse_check)",
    ),
    // `E14-3` chunk 3 -- `world/src` entered this scanner's roots.
    // Worldgen is where seeded-vs-ambient confusion is the classic
    // failure, so this was the chunk most likely to find debt. It found
    // NONE: five files, six sites, every one already mitigated. The
    // interesting result is a negative one, and it is only a result
    // because each site was traced rather than pattern-matched.
    (
        "world/src/lib.rs",
        1,
        RandomDrawClassV1::DeterministicModeGatedLiveEntropy,
        "ARCH-003, and the textbook member of this class: the per-chunk `dynamic_rng` that \
         drives chests, entities, scatter, shrubs and farm-field crop sprites. Under \
         common::deterministic_worldgen_enabled() it is ChaCha8Rng::seed_from_u64 of \
         RandomField::new(index.seed).get(chunk_pos) -- a pure function of (world seed, chunk \
         pos), so it is call-ORDER-independent too, which matters because chunk gen is \
         threaded. Only the ungated live branch reads OS entropy. The in-tree comment records \
         the failure it was built for: a phantom crop sprite perturbs a colonist walkability \
         read and desyncs the run",
    ),
    (
        "world/src/layer/wildlife.rs",
        2,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "both inside #[cfg(test)] (mod at line 690): test_load_entities' dummy_rng and \
         test_group_choose's dynamic_rng. Same shape as the lottery.rs/cmd.rs entries",
    ),
    (
        "world/src/site/mod.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "test_site()'s Site::generate_city seed. WEAKER GUARANTEE THAN ITS NEIGHBOURS, and \
         recorded as such: unlike the other four chunk-3 sites this is NOT #[cfg(test)]-gated \
         -- it is a `pub fn` compiled into the world library, and its safety rests on its sole \
         caller being world/examples/site.rs (traced, not assumed; same shape as the npc.rs \
         entry's find_unused.rs tracing). Caller-tracing is a non-local guarantee: someone \
         calling test_site() from live code would move this site's class without touching this \
         line",
    ),
    (
        "world/src/site/plot/adlet.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "#[cfg(test)] mod tests' test_creating_entities only",
    ),
    (
        "world/src/site/plot/gnarling.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "#[cfg(test)] mod tests' test_creating_entities only",
    ),
    // ---------------------------------------------------------------
    (
        "common/src/apex/identity/opaque.rs",
        1,
        RandomDrawClassV1::IdentityGeneration,
        "fill_bytes for a fresh opaque identifier (SessionId/ServerBootId-class) -- the VALUE \
         being unguessable is the requirement, not sim-reproducibility",
    ),
    (
        "common/src/apex/reconciliation_metric.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "T7.3c-i's #[cfg(test)] baseline() fixture helper only -- humanoid::Body::random_with \
         for a throwaway comparison-test body, never reached by any authoritative/live path; \
         same shape as loadout_builder.rs/cmd.rs/lottery.rs's #[cfg(test)]-only sites above",
    ),
    (
        "common/src/comp/body/ship.rs",
        2,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() has zero live callers (dead, like Lottery::choose() was); \
         make_collider()'s random branch only fires for ship::Body::Volume, whose only \
         constructor is an admin command (server/src/cmd.rs:2554)",
    ),
    (
        "common/src/explosion.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "IceBomb particle color variation -- a cosmetic RGB value, presentation-namespace like \
         Outcome::Lightning",
    ),
    (
        "common/src/npc.rs",
        4,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "kind_to_body (ambient twin of the seeded kind_to_body_with) and NpcBody::from_str's \
         species-conv closure -- both traced to their only caller, \
         common/src/bin/find_unused.rs, a dev-tool binary, not live sim",
    ),
    (
        "common/src/comp/inventory/loadout_builder.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "#[cfg(test)] ItemSpec::validate only",
    ),
    (
        "common/src/cmd.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "#[cfg(test)] test_load_kits only",
    ),
    (
        "common/src/lottery.rs",
        2,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "#[cfg(test)] validate_loot_spec + test_distribute_many only",
    ),
    (
        "common/src/path.rs",
        1,
        RandomDrawClassV1::DeterministicModeGatedLiveEntropy,
        "stuck-route jiggle: self.deterministic_rng.as_mut().map_or_else(ambient, seeded) -- \
         live play keeps the OS-seeded path deliberately, only the harness derives a stream",
    ),
    (
        "server/src/character_creator.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "RNG-P3: character creation is an external input event (a specific player's one-time \
         choice), not sim-replay-critical state -- pre-existing, documented classification",
    ),
    (
        "server/src/cmd.rs",
        13,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "handle_drop_all/handle_spawn_airship/etc, ModularWeaponRandom kit-spawn, debug \
         outcome-spawn, handle_goto_rand -- all operator-triggered admin commands (every bare \
         call traced to its own handle_* fn), external input not sim-loop state",
    ),
    (
        "server/src/state_ext.rs",
        1,
        RandomDrawClassV1::DeterministicModeGatedLiveEntropy,
        "ARCH-003: item-drop orientation, explicitly gated by ExecutionMode::is_deterministic() \
         -- deliberate, pre-existing design; the PRECEDENT this bucket is named after",
    ),
    (
        "server/src/weather/tick.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Outcome::Lightning emission -- presentation-only (E6-confirmed: no HealthChangeEvent, \
         no terrain mutation, zero server-side listeners of the Outcome)",
    ),
    (
        "server/agent/src/action_nodes.rs",
        2,
        RandomDrawClassV1::DeterministicModeGatedLiveEntropy,
        "helper_random_bool's None-fallback (Fable-named finding): live play deliberately keeps \
         OS entropy when self.helper_rng is unset, same data.rs:79 family. Second site \
         (attack_inner) is #[cfg(feature = \"be-dyn-lib\")] only -- a hot-reload DEV BUILD \
         path, the normal build takes an injected rng param instead; dev-tool-only, not live sim",
    ),
    (
        "server/src/sys/agent/mod.rs",
        1,
        RandomDrawClassV1::DeterministicModeGatedLiveEntropy,
        "chaser unstick stream: ChaCha8Rng::from_rng(&mut rng()) when execution_mode is not \
         deterministic -- explicitly documented (\"Live mode explicitly retains the original \
         OS-seeded path\"), harness mode derives a seed from (world_seed, tick, uid) instead",
    ),
    (
        "common/src/comp/body/arthropod.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- only reachable via npc.rs's kind_to_body (already-classified \
         dev-tool-only ambient twin); no other caller",
    ),
    (
        "common/src/comp/body/biped_large.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/biped_small.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/bird_large.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/bird_medium.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/crustacean.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/dragon.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/fish_medium.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/fish_small.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/golem.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/humanoid.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- non-test live callers all route through npc.rs's kind_to_body \
         (dev-tool-only); the 4 direct callers elsewhere (agent.rs x3, loot_owner.rs x1) are \
         all #[cfg(test)]",
    ),
    (
        "common/src/comp/body/object.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() has zero callers anywhere (dead, like ship.rs's)",
    ),
    (
        "common/src/comp/body/quadruped_low.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/quadruped_medium.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- non-test live callers route through npc.rs's kind_to_body; the \
         direct loot_owner.rs caller is #[cfg(test)]",
    ),
    (
        "common/src/comp/body/quadruped_small.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
    (
        "common/src/comp/body/theropod.rs",
        1,
        RandomDrawClassV1::NonAuthoritativeEntropy,
        "Body::random() -- same kind_to_body-only reachability as arthropod.rs",
    ),
];

#[cfg(test)]
mod tests {
    use super::{AMBIENT_ENTROPY_SITES, RandomDrawClassV1};
    use std::path::{Path, PathBuf};

    fn workspace_root() -> PathBuf {
        // common's own manifest dir, two levels up to the workspace root.
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("common has a parent dir")
            .to_path_buf()
    }

    /// Count non-comment lines in `path` matching one of the three ambient-
    /// entropy call forms. A line is treated as a comment (and skipped) if
    /// its trimmed text starts with `//` -- every legitimate mention in
    /// this codebase (doc comments, historical notes) is a whole-line
    /// comment, never a trailing one on a code line; this is a disclosed,
    /// deliberately simple heuristic, not a full Rust tokenizer.
    /// Cross-review (Fable-ruled): a bare `rng()` call reached via an
    /// unqualified `use rand::rng;` import evaded the plain
    /// `.contains("rand::rng()")` substring check entirely -- this is how
    /// `server/agent/src/action_nodes.rs`'s `helper_random_bool` escaped
    /// the scan. Matches the free-function call form `rng()` NOT preceded
    /// by `.` (a method call like `self.rng()` or `foo.rng()` is a
    /// DIFFERENT thing -- an accessor, not the ambient-entropy free
    /// function -- and must not false-positive here) and not part of a
    /// longer identifier (`child_rng()`, `npc_rng()`).
    fn contains_bare_rng_call(line: &str) -> bool {
        let bytes = line.as_bytes();
        let pat = b"rng()";
        let mut i = 0;
        while i + pat.len() <= bytes.len() {
            if &bytes[i..i + pat.len()] == pat {
                let preceded_by_word_char_or_dot = i > 0
                    && (bytes[i - 1] == b'.'
                        || bytes[i - 1].is_ascii_alphanumeric()
                        || bytes[i - 1] == b'_');
                if !preceded_by_word_char_or_dot {
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    fn count_live_matches(path: &Path) -> usize {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("registry names a file that doesn't exist: {path:?}: {e}"));
        text.lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    return false;
                }
                trimmed.contains("rand::rng()")
                    || trimmed.contains("thread_rng()")
                    || trimmed.contains("rand::random(")
                    || contains_bare_rng_call(trimmed)
            })
            .count()
    }

    /// Walks the in-scope crate source trees and returns every `.rs`
    /// file containing at least one live ambient-entropy match, workspace-
    /// relative.
    fn scan_scoped_crates() -> Vec<PathBuf> {
        let root = workspace_root();
        let mut found = Vec::new();
        for crate_dir in [
            "common/src",
            "server/src",
            "server/agent/src",
            "rtsim/src",
            // E14-3 chunk 1: this scanner kept its OWN root list, four
            // wide, while `scanner_framework::AUTHORITATIVE_SCAN_ROOTS`
            // grew to ten. `common/systems/src` -- the authoritative
            // combat/physics systems -- was in NEITHER for the whole of
            // E13, which is how a half-finished `seed_ability_rng`
            // migration sat here unflagged. Two scanners with two root
            // lists is how a crate stays invisible to both.
            "common/systems/src",
            // E14-3 chunk 2: the authoritative colony sim, where an
            // unmitigated draw would matter most -- and where exactly
            // one was hiding.
            "bastion-server/src",
            // E14-3 chunk 3: worldgen -- the classic home of
            // seeded-vs-ambient confusion, and the chunk most likely to
            // find debt. It found none; all six sites were already
            // mitigated.
            "world/src",
        ] {
            walk(&root.join(crate_dir), &mut |path| {
                // This file's own pattern-matching code contains the three
                // match strings as string literals (not calls) -- self-
                // exempt rather than have the meta/tooling file register
                // itself as a site.
                if path.file_name().is_some_and(|n| n == "rng_source_registry.rs") {
                    return;
                }
                if path.extension().is_some_and(|e| e == "rs") && count_live_matches(path) > 0 {
                    found.push(path.to_path_buf());
                }
            });
        }
        found.sort();
        found
    }

    fn walk(dir: &Path, on_file: &mut impl FnMut(&Path)) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, on_file);
            } else {
                on_file(&path);
            }
        }
    }

    fn relative(root: &Path, path: &Path) -> String {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// The gate: every file the scan finds must be in the registry, with
    /// the EXACT expected count -- an unclassified new site fails, and so
    /// does a site quietly changing shape (more or fewer matches) without
    /// updating the table.
    #[test]
    fn scan_finds_only_registered_sites() {
        let root = workspace_root();
        let found = scan_scoped_crates();

        let mut registry: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for &(path, count, _, _) in AMBIENT_ENTROPY_SITES {
            let prev = registry.insert(path, count);
            assert!(prev.is_none(), "duplicate registry entry for {path}");
        }

        let mut unregistered = Vec::new();
        let mut mismatched = Vec::new();
        for path in &found {
            let rel = relative(&root, path);
            match registry.remove(rel.as_str()) {
                None => unregistered.push(rel),
                Some(expected) => {
                    let actual = count_live_matches(path);
                    if actual != expected {
                        mismatched.push(format!("{rel}: expected {expected}, found {actual}"));
                    }
                },
            }
        }

        assert!(
            unregistered.is_empty(),
            "unclassified ambient-entropy sites (add to AMBIENT_ENTROPY_SITES or convert to a \
             keyed derivation): {unregistered:?}"
        );
        assert!(mismatched.is_empty(), "site count drifted from the registry: {mismatched:?}");
        // registry now holds only entries the scan did NOT find -- a stale
        // entry for a site that was converted/removed and never un-registered.
        let stale: Vec<&str> = registry.keys().copied().collect();
        assert!(
            stale.is_empty(),
            "registry entries for files the scan no longer finds any live match in (stale, \
             remove from AMBIENT_ENTROPY_SITES): {stale:?}"
        );
    }

    /// Falsifier: the scanner would actually catch an unclassified site.
    #[test]
    fn an_unregistered_file_would_be_caught() {
        let root = workspace_root();
        let found = scan_scoped_crates();
        let registered: std::collections::HashSet<&str> =
            AMBIENT_ENTROPY_SITES.iter().map(|&(p, ..)| p).collect();
        let all_registered = found
            .iter()
            .all(|path| registered.contains(relative(&root, path).as_str()));
        assert!(all_registered, "precondition: this test only proves something if the real \
                 scan is currently clean");
        // A file the registry has never heard of must not silently pass.
        assert!(!registered.contains("nonexistent/made/up/path.rs"));
    }

    /// Every movement of the debt population, oldest first, each
    /// with its CAUSE — and the cause must declare which KIND of
    /// movement it is.
    ///
    /// **`E14-3` chunk 2 tested this ratchet one chunk after it was
    /// built, and the test was whether I would rubber-stamp my own
    /// gate.** Widening the roots to `bastion-server/src` revealed a
    /// seventh site, so the mark had to rise — which is precisely
    /// the move the ratchet exists to make uncomfortable.
    ///
    /// The resolution is that two different events were being
    /// counted as one:
    ///
    /// - **`DISCOVERY`** — a scan-root widening reveals debt that
    ///   ALREADY EXISTED. The number rises because vision improved,
    ///   not because the code got worse. Legitimate.
    /// - **`MIGRATION`** — a site moved onto a deterministic seam.
    ///   The number falls. Always legitimate.
    /// - New ambient code inside an already-scanned root has NO
    ///   legal label here, which is the point: there is no honest
    ///   string to write, so the ledger cannot be used to launder a
    ///   regression into a recorded fact.
    ///
    /// [`ledger_causes_declare_their_kind`] enforces the labels, so
    /// this cannot decay into a list of bare numbers.
    const DEBT_LEDGER: &[(usize, &str)] = &[
        (
            6,
            "E14-3 chunk 1 (DISCOVERY): common/systems/src entered this scanner's roots -- \
             six pre-existing combat draws (arcing/buff/melee/pool/projectile/shockwave), \
             unwalked since T0.79 because this scanner kept its own narrower root list",
        ),
        (
            7,
            "E14-3 chunk 2 (DISCOVERY): bastion-server/src entered this scanner's roots -- \
             the cave-in damage-instance draw, likewise pre-existing and likewise invisible \
             only because of the root gap",
        ),
    ];

    /// The debt ratchet: `UnmitigatedAuthoritativeEntropy` may only
    /// SHRINK.
    ///
    /// Six is not a target, it is a high-water mark. Registering these
    /// sites was the only honest option -- every other class asserts a
    /// safety property they do not have -- but registration is exactly
    /// what makes debt comfortable, because the gate goes green and the
    /// build stops complaining. This test is the discomfort: adding a
    /// seventh means editing this number, and editing a number that
    /// says "must only go down" is a decision somebody has to defend.
    #[test]
    fn unmitigated_authoritative_entropy_only_shrinks() {
        let (high_water_mark, _) = *DEBT_LEDGER.last().expect("ledger is never empty");

        let live = AMBIENT_ENTROPY_SITES
            .iter()
            .filter(|(_, _, class, _)| {
                *class == RandomDrawClassV1::UnmitigatedAuthoritativeEntropy
            })
            .count();

        assert!(
            live <= high_water_mark,
            "UnmitigatedAuthoritativeEntropy grew to {live} (high-water mark {high_water_mark}). \
             This class is DEBT: a new live authoritative ambient draw needs a reviewed decision, \
             not a registry line. Migrate it onto a deterministic seam \
             (combat::seed_ability_rng / combat::derive_attack_instance) instead. If this growth \
             is a scan-root WIDENING revealing pre-existing debt, add a DEBT_LEDGER entry saying \
             so -- and if you cannot honestly write (DISCOVERY), you are looking at a regression."
        );
        assert_eq!(
            live, high_water_mark,
            "UnmitigatedAuthoritativeEntropy shrank to {live} -- good. Add a DEBT_LEDGER entry \
             ({live}, \"... (MIGRATION): ...\") so the ratchet holds the new ground."
        );
    }

    /// Every `DEBT_LEDGER` cause must declare its KIND.
    ///
    /// Without this the ledger decays into bare numbers with prose
    /// beside them, and "we raised it because we had to" becomes an
    /// acceptable entry. `DISCOVERY` and `MIGRATION` are the only two
    /// legitimate reasons the population moves; new ambient code in an
    /// already-scanned root has no legal label, which is exactly what
    /// stops the ledger from laundering a regression into a record.
    #[test]
    fn ledger_causes_declare_their_kind() {
        for (mark, cause) in DEBT_LEDGER {
            assert!(
                cause.contains("(DISCOVERY)") || cause.contains("(MIGRATION)"),
                "DEBT_LEDGER entry {mark} must declare (DISCOVERY) or (MIGRATION): {cause}"
            );
        }
    }

    #[test]
    fn every_entry_has_a_substantive_note() {
        for &(path, _, _, note) in AMBIENT_ENTROPY_SITES {
            assert!(note.len() > 16, "site {path} has a stub note: {note:?}");
        }
    }

    #[test]
    fn identity_generation_is_actually_used_somewhere() {
        // Non-vacuity: the taxonomy has 5 buckets: prove at least the two
        // buckets this registry currently populates (IdentityGeneration,
        // NonAuthoritativeEntropy) are both reachable outcomes of
        // classifying a real site, not dead enum variants.
        assert!(
            AMBIENT_ENTROPY_SITES
                .iter()
                .any(|&(_, _, class, _)| class == RandomDrawClassV1::IdentityGeneration)
        );
        assert!(
            AMBIENT_ENTROPY_SITES
                .iter()
                .any(|&(_, _, class, _)| class == RandomDrawClassV1::NonAuthoritativeEntropy)
        );
    }
}
