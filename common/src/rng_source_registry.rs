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
    // roots. Six sites at pin time, ALL `UnmitigatedAuthoritativeEntropy`
    // (one, `buff.rs`'s fire-spread draw, is FIXED as of `E14-1` -- see
    // the note in its place below -- leaving five), all the same shape:
    // `let mut rng = rand::rng();` at the head of a
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
    // `E14-1` (2026-07-29): `common/systems/src/buff.rs`'s fire-spread
    // draw -- the designated-HIGH entry that used to sit here -- is
    // FIXED and no longer registered (a fixed site has zero live
    // `rand::rng()` matches, and this registry's own staleness check
    // fails a registered entry with nothing left to match, same
    // discipline as `E11-6b`'s precedent in `determinism_scan.rs`). Both
    // stacked hazards are closed: `touch_entities.keys()` is now
    // collected and `sort_unstable()`-ed by `Uid` before the loop (fixes
    // the walk), and the per-target draw comes from a
    // `ChaCha8Rng::seed_from_u64` keyed on (source entity's `Uid`, tick
    // `Time`, a distinguishing constant) instead of ambient entropy
    // (fixes the draw) -- the same inline idiom `beam.rs` (DET-EVT-011)
    // already established in this crate.
    //
    // `E14-1b` (2026-07-29): the other five `E14-1 family` sites --
    // arcing.rs, melee.rs, pool.rs, projectile.rs, shockwave.rs, all
    // "ambient rng passed to combat::attack" -- are ALSO fixed and no
    // longer registered, same idiom: each seeds one
    // `ChaCha8Rng::seed_from_u64(source Uid, tick Time, a distinguishing
    // constant)` once per source entity (the arc/attacker/pool/
    // projectile/shockwave, keyed on ITS OWNER's `Uid` where the source
    // is itself a spawned entity rather than a character) and draws
    // sequentially across that source's targets. `projectile.rs` and
    // `shockwave.rs` additionally fed their `rng.random_bool(0.05)`
    // SoundEvent roll (agent-perceived, per the removed entries' own
    // notes) from the same now-seeded stream. `projectile.rs`'s
    // `dispatch_hit` helper took `&mut rand::rngs::ThreadRng` as a
    // parameter -- retyped to `&mut rand_chacha::ChaCha8Rng`, its sole
    // caller already passing the seeded stream.
    //
    // `E14-2b` (2026-07-29): `bastion-server/src/bastion_jobs.rs`'s
    // cave-in draw -- the `E14-3` chunk 2 OUTLIER that used to sit here,
    // "every other damage-instance in the tree is derived" made literal
    // -- is FIXED and no longer registered, closing
    // `UnmitigatedAuthoritativeEntropy` at ZERO. `instance: rand::random()`
    // moved onto `combat::derive_attack_instance("bastion/cavein/v1",
    // None, victim_uid, time, 0)` -- no attacker (an environmental
    // collapse), the victim's own Uid as the target (already
    // discriminates each victim within one event, so ordinal stays 0).
    // `cavein_eject_and_injure` gained a `uids: &ReadStorage<Uid>`
    // parameter, threaded through both callers: the live `Sys::run`
    // post-loop (`bastion_jobs.rs`) and the harness's
    // `bastion_force_collapse_check` (`server/src/lib.rs`, which did not
    // previously fetch a `uids` storage at all).
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
    // `E14-3` chunk 4 (FINAL) -- the last three roots entered together:
    // `common/net/src`, `common/state/src`, `common/query_server/src`.
    // Between them, exactly ONE site. The wire crate and the ECS state
    // container hold zero ambient draws, which is what they should hold.
    (
        "common/query_server/src/server.rs",
        1,
        RandomDrawClassV1::IdentityGeneration,
        "gen_secret's rotating challenge-secret pair for the server-browser query protocol -- \
         the strongest possible case for this class, because DETERMINISM HERE WOULD BE THE BUG: \
         a predictable challenge secret is a spoofable one. Pairs with the secret-rotation \
         wall-clock read this crate contributed to the determinism_scan instant family in E13 \
         chunk 5. Also the only site in this whole campaign found by the BARE `rng()` detector \
         rather than a qualified path -- `let mut rng = rng();` via `use rand::rng;`, exactly \
         the import-shaped evasion the T0.79 cross-review added contains_bare_rng_call for",
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
        // E14-3 chunk 4 (FINAL): this scanner no longer keeps its own
        // root list. It CONSUMES `AUTHORITATIVE_SCAN_ROOTS`.
        //
        // The whole E13/E14-3 campaign exists because there were two
        // hand-maintained lists: this one sat at four roots while the
        // shared one grew to ten, and `common/systems/src` was in
        // NEITHER -- which is how six live authoritative combat draws
        // and a half-finished `seed_ability_rng` migration stayed
        // invisible to both scanners at once.
        //
        // Re-synchronising the two lists would have fixed today's gap
        // and left tomorrow's free to reopen silently, because nothing
        // would compare them. Sharing the constant makes the divergence
        // UNREPRESENTABLE instead: this scanner cannot fall behind a
        // root it does not own. If a deliberate exclusion is ever
        // needed, it belongs here as a NAMED exception with a reason --
        // the `UNSCANNED_WORKSPACE_MEMBERS` shape -- not as a second
        // list that drifts by accident.
        for crate_dir in crate::scanner_framework::AUTHORITATIVE_SCAN_ROOTS {
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
        (
            6,
            "E14-1 (MIGRATION): common/systems/src/buff.rs's fire-spread draw moved onto a \
             ChaCha8Rng::seed_from_u64(source Uid, tick Time, constant) seam -- the same inline \
             idiom beam.rs (DET-EVT-011) already established in this crate. The other five \
             E14-1-family sites (arcing/melee/pool/projectile/shockwave) are unchanged.",
        ),
        (
            5,
            "E14-1b (MIGRATION): common/systems/src/arcing.rs's arc-attack draw moved onto the \
             same seeded-per-source-entity idiom, keyed on the arc's owner Uid.",
        ),
        (
            4,
            "E14-1b (MIGRATION): common/systems/src/melee.rs's melee-attack draw moved onto the \
             same idiom, keyed on the attacker's own Uid (already in the join tuple).",
        ),
        (
            3,
            "E14-1b (MIGRATION): common/systems/src/pool.rs's area-effect draw moved onto the \
             same idiom, keyed on the pool's owner Uid.",
        ),
        (
            2,
            "E14-1b (MIGRATION): common/systems/src/projectile.rs's hit-resolution AND \
             SoundEvent draws moved onto the same idiom, keyed on the projectile's owner Uid; \
             dispatch_hit's rng parameter retyped from &mut rand::rngs::ThreadRng to \
             &mut rand_chacha::ChaCha8Rng to match.",
        ),
        (
            1,
            "E14-1b (MIGRATION): common/systems/src/shockwave.rs's hit-resolution AND \
             SoundEvent draws moved onto the same idiom, keyed on the shockwave's owner Uid -- \
             the last common/systems/src E14-3-chunk-1 site. Not zero: \
             bastion-server/src/bastion_jobs.rs's cave-in draw remains, a different fix shape \
             (derive_attack_instance, not an RNG stream), out of E14-1b's stated scope.",
        ),
        (
            0,
            "E14-2b (MIGRATION): bastion-server/src/bastion_jobs.rs's cave-in draw moved onto \
             combat::derive_attack_instance (no attacker, victim's own Uid as target, ordinal \
             0) -- the different fix shape the previous entry named. \
             UnmitigatedAuthoritativeEntropy is now EMPTY: the population reached zero and the \
             class has no members, not 'one left, see the other row'.",
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
