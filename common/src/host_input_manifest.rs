//! T0.89: the host-input manifest -- every environment variable an
//! authoritative crate reads, classified, cited by site, and (for the
//! classes that can change what a run actually does or protects) captured
//! once at boot into a resource so a run's attestation can say which of
//! these were live, not just that the binary COULD read them.
//!
//! No such registry existed before this: 38 direct `std::env::var[_os]`
//! call sites were found scattered across common/rtsim/server/bastion-
//! server/world with zero central catalog (contrast T0.79's
//! `rng_source_registry` for RNG draws, which this module's shape
//! deliberately mirrors).
//!
//! Two disclosed scanner limitations, both real and neither silently
//! absorbed:
//! 1. `world/src/site/genstat.rs` reads its var name through a `&str`
//!    parameter (`get_bool_env_var(var_name)`), not a literal -- the
//!    scanner cannot see through that. The two ACTUAL resolved names
//!    (`SITE_GENERATION_STATS_VERBOSE`, `SITE_GENERATION_STATS_LOG`) are
//!    registered directly against that file instead; the indirection
//!    site itself is scanner-exempted (see `INDIRECTED_EXEMPT_LINES`).
//! 2. `bastion-server/src/bastion_flight_recorder.rs`'s `RecorderConfig::
//!    from_env` reads FOUR more variables (`BASTION_FLIGHT_RECORDER_UID`/
//!    `_SAMPLE_EVERY`/`_MAX_SAMPLES`/`_MAX_EVENTS`) through a passed-in
//!    `lookup: impl FnMut(&str) -> ...` closure -- a deeper indirection a
//!    literal-string scanner cannot see AT ALL (not even as a dynamic-
//!    name miss; the call site doesn't mention `std::env` at all). Found
//!    only by reading the file, not by widening the grep pattern -- the
//!    T6.1c/T6.1d lesson (pattern-widening has a ceiling; root-set/call-
//!    graph reading is what finds indirection) applies here too. These 4
//!    are registered against their real names, matching the T6.1c/.1d
//!    convention that a manually-verified indirected site outranks an
//!    unfound literal one.

use std::{collections::HashSet, fs, path::Path};

/// What kind of thing a variable controls, which is what decides whether
/// the boot-time manifest resource needs to remember its VALUE (not just
/// that it was read).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EnvVarClassV1 {
    /// Affects only what gets logged, traced, or recorded as provenance
    /// (build/source identity, sample filters). Zero effect on
    /// simulated state or control flow -- two runs with this set
    /// differently produce byte-identical simulation output, differing
    /// only in what got written to a side-channel log/artifact.
    Diagnostic,
    /// Changes what this run actually simulates: a different world/save
    /// data source, a different tuning constant, a different bootstrap
    /// path. Two runs differing only here can legitimately diverge --
    /// captured in the manifest so that divergence is EXPLAINED, not
    /// mysterious.
    GameplayVariant,
    /// Selects or relaxes the execution-scheduling policy itself
    /// (serial vs parallel, or a guard against that invariant) rather
    /// than changing what is simulated.
    DeterminismMode,
    /// Governs save/load version-mismatch or missing-data handling --
    /// the `ExplicitRecoveryOnly` class of override (see
    /// `server/src/save_migration.rs`).
    Recovery,
}

/// One registered environment-variable read site.
struct EnvVarSiteV1 {
    file: &'static str,
    var: &'static str,
    class: EnvVarClassV1,
    note: &'static str,
}

const fn site(
    file: &'static str,
    var: &'static str,
    class: EnvVarClassV1,
    note: &'static str,
) -> EnvVarSiteV1 {
    EnvVarSiteV1 { file, var, class, note }
}

use EnvVarClassV1::{Diagnostic, DeterminismMode, GameplayVariant, Recovery};

/// The full catalog. One entry per (file, variable) pair -- NOT one per
/// textual occurrence: `BASTION_EGRESS_DIAG` alone has ~40 call sites in
/// one file, all the same diagnostic toggle, and repeating the row 40
/// times would catalog nothing extra.
const CATALOG: &[EnvVarSiteV1] = &[
    site(
        "common/src/util/mod.rs",
        "VELOREN_GIT_VERSION",
        Diagnostic,
        "build-info string echoed in version handshakes; not a behavior toggle",
    ),
    site(
        "server/src/bastion_arena.rs",
        "BASTION_ASSET_ARENA",
        GameplayVariant,
        "enables the dev asset-arena bootstrap path instead of normal worldgen",
    ),
    site(
        "server/src/bastion_arena.rs",
        "BASTION_ASSET_LAB_DIR",
        GameplayVariant,
        "data-location paired with BASTION_ASSET_ARENA -- which arena assets load",
    ),
    site(
        "server/src/events/inventory_manip.rs",
        "BASTION_B55_TRACE_DELETES",
        Diagnostic,
        "B55 delete-tracing log toggle",
    ),
    site(
        "server/src/lib.rs",
        "BASTION_DETERMINISTIC",
        DeterminismMode,
        "the core opt-in: deterministic rtsim RNG + serial execution",
    ),
    site(
        "server/src/lib.rs",
        "BASTION_DETERMINISTIC_PARALLEL",
        DeterminismMode,
        "T0.52 probe-only: relaxes the one-worker-pool assertion under \
         DeterministicSerial, does not create a separate execution mode",
    ),
    site(
        "server/src/lib.rs",
        "BASTION_AUTOFOUND_COLONY",
        GameplayVariant,
        "headless auto-founds a colony for non-interactive determinism captures",
    ),
    site(
        "server/src/lib.rs",
        "BASTION_AUTH_POS_LOG",
        Diagnostic,
        "authoritative-position probe log path",
    ),
    site(
        "server/src/rtsim/mod.rs",
        "RTSIM_NOLOAD",
        Recovery,
        "skips loading rtsim save data entirely, forcing a fresh start",
    ),
    site(
        "server/src/rtsim/mod.rs",
        "RTSIM_IGNORE_VERSION",
        Recovery,
        "loads a version-mismatched rtsim save unmigrated instead of purging -- \
         the ExplicitRecoveryOnly mechanism (server/src/save_migration.rs)",
    ),
    site(
        "server/src/rtsim/mod.rs",
        "RTSIM_IGNORE_WORLD_BASELINE",
        Recovery,
        "loads a world-baseline-mismatched rtsim save unmigrated instead of purging -- \
         the ExplicitRecoveryOnly mechanism, world resolution policy \
         (server/src/save_migration.rs, APEX-T4.3)",
    ),
    site(
        "server/src/rtsim/mod.rs",
        "VELOREN_RTSIM",
        GameplayVariant,
        "data-location: which rtsim save directory this run reads/writes",
    ),
    site(
        "server/src/state_ext.rs",
        "BASTION_B55_TRACE_DELETES",
        Diagnostic,
        "B55 delete-tracing log toggle (second site, same variable)",
    ),
    site(
        "server/src/sys/agent/mod.rs",
        "BASTION_GOTO_WRITER_DIAG_UID",
        Diagnostic,
        "filters goto-writer diagnostic logging to one uid",
    ),
    site(
        "server/src/sys/item.rs",
        "BASTION_B55_TRACE_MERGES",
        Diagnostic,
        "B55 merge-tracing log toggle",
    ),
    site(
        "server/src/sys/object.rs",
        "BASTION_B55_TRACE_DELETES",
        Diagnostic,
        "B55 delete-tracing log toggle (third site, same variable)",
    ),
    site(
        "server/src/sys/sentinel.rs",
        "PLOT_UPDATE_COUNTS",
        Diagnostic,
        "tracy-plot toggle, additionally gated behind TRACY_ENABLED",
    ),
    site(
        "server/src/terrain_persistence.rs",
        "VELOREN_TERRAIN",
        GameplayVariant,
        "data-location: which terrain save directory this run reads/writes",
    ),
    site(
        "rtsim/src/rule/npc_ai/airship_logger.rs",
        "AIRSHIP_LOGGER_TGT_NPC_ID",
        Diagnostic,
        "filters airship diagnostic logging to one npc id",
    ),
    site(
        "rtsim/src/rule/npc_ai/airship_logger.rs",
        "AIRSHIP_LOGGER_OUTPUT_PATH",
        Diagnostic,
        "airship diagnostic log output path",
    ),
    site(
        "bastion-server/src/bastion_flat_arena.rs",
        "BASTION_FLAT_ARENA",
        GameplayVariant,
        "the standing flat-arena testbed toggle -- a different world entirely",
    ),
    site(
        "bastion-server/src/bastion_flat_arena.rs",
        "BASTION_FLAT_ARENA_RESOURCED",
        GameplayVariant,
        "the RESOURCED flat-arena variant (FOUNDING PRESET v1, packet section 2): \
         adds the deterministic tree cluster + stone outcrop at generation. \
         A different world again -- a run's attestation must say whether the \
         chop/mine work was there",
    ),
    site(
        "bastion-server/src/bastion_entity_event_log.rs",
        "BASTION_ENTITY_EVENT_LOG",
        Diagnostic,
        "the entity event log's master gate. Its own module doc states the \
         posture verbatim -- 'no init, no allocation beyond the (empty) \
         process-global slot, no ECS mutation, no scheduling change when off' \
         -- so two runs differing only in this produce identical simulation \
         and differ only in the recorded ring",
    ),
    site(
        "bastion-server/src/bastion_entity_event_log.rs",
        "BASTION_ENTITY_EVENT_LOG_RING_SIZE",
        Diagnostic,
        "ring capacity for the log above (DEFAULT_RING_SIZE, floored at 1). \
         Changes how much history is RETAINED for reading, never what the \
         simulation does -- a pure side-channel size knob",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG",
        Diagnostic,
        "per-colonist presence sample (tick, uid, loaded, hunger, rest, pos). \
         Read at its site: a bare `if var_os(..).is_some()` around an emit \
         loop over a join the block already pays for -- no mutation, no \
         control flow beyond the emit. FOUNDING PRESET A2 reads its `pos` \
         field, which is why the class matters: calling a Diagnostic a \
         GameplayVariant would make every A2 run attest as a different world",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_FINGERPRINT",
        Diagnostic,
        "per-tick state digest for the time-compression equivalence proof. \
         Reads existing JobBoard counters and job populations and hashes them \
         in deterministic (BTreeMap/sorted) order so the hook cannot itself \
         become a divergence source; it writes no simulated state. NOTE, \
         recorded rather than left implicit: it is NOT free in WALL time, and \
         this program has measured that wall cost is a real axis for \
         chunk-gen-coupled behaviour (colonist promotion timing) even when \
         simulation output is byte-identical. Diagnostic by this class's own \
         definition -- but a run comparing WALL-COUPLED quantities should \
         record whether it was on",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_DIR",
        Diagnostic,
        "enables the flight recorder feature (adds observation, does not \
         change simulated state)",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_ARTIFACT_SHA256",
        Diagnostic,
        "provenance metadata recorded verbatim into the recorder's own \
         attestation header (RecorderMetadata)",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_SOURCE_HEAD",
        Diagnostic,
        "provenance metadata, same class as ARTIFACT_SHA256 above",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_SOURCE_BRANCH",
        Diagnostic,
        "provenance metadata, same class as ARTIFACT_SHA256 above",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_SOURCE_DIRTY",
        Diagnostic,
        "provenance metadata, same class as ARTIFACT_SHA256 above",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_SEED",
        Diagnostic,
        "provenance metadata (the seed string), recorded not consumed here",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_COMMAND",
        Diagnostic,
        "provenance metadata, same class as ARTIFACT_SHA256 above",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_SESSION_ID",
        Diagnostic,
        "provenance metadata, same class as ARTIFACT_SHA256 above",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_UID",
        Diagnostic,
        "read via RecorderConfig::from_lookup's closure indirection, not a \
         literal std::env call the scanner could find unaided (see module \
         doc) -- filters recorded samples to one uid",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_SAMPLE_EVERY",
        Diagnostic,
        "same closure indirection as UID above -- sampling cadence for the \
         recorder, not the simulation",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_MAX_SAMPLES",
        Diagnostic,
        "same closure indirection as UID above -- recorder buffer cap",
    ),
    site(
        "bastion-server/src/bastion_flight_recorder.rs",
        "BASTION_FLIGHT_RECORDER_MAX_EVENTS",
        Diagnostic,
        "same closure indirection as UID above -- recorder buffer cap",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_EGRESS_DIAG",
        Diagnostic,
        "egress diagnostic logging toggle (~40 call sites, one variable)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ACCESS_CLAIM_DIAG",
        Diagnostic,
        "F3-BRANCH / CLAIM-RELEASE event-driven diagnostic toggle -- pre-existing \
         gap found and closed while adding ROW-ITEM6-WITNESS-PACKET's own two \
         vars, unrelated to that row otherwise (this variable predates it)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_MINE_READBACK_DIAG",
        Diagnostic,
        "next-tick mine readback (READBACK-PREREG.md) -- logs terrain.get at the \
         completion site and again one tick later, so 'the air write landed' is a \
         fact instead of an inference",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_STATUS_STAMP_DIAG",
        Diagnostic,
        "edge-triggered status-stamp emit (STAMP-EMIT-PREREG.md) -- both stamp \
         sites re-stamp every tick they hold, so this fires on the EPISODE edge, \
         never per tick",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_RECREATION",
        GameplayVariant,
        "ITEM 11's recreation break -- a REAL gameplay variant, off by default: \
         colonists stopping work to relax changes colony throughput, so it earns \
         its way on through a measured A/B rather than shipping enabled",
    ),
    site(
        "bastion-server/src/bastion_flat_arena.rs",
        "BASTION_FLAT_ARENA_SHAFT",
        GameplayVariant,
        "the SHAFT fixture (SHAFT-FIXTURE-PREREG.md) -- 8 deep, 3 across, the \
         geometry egress_scan's own arithmetic requires for a trap; the 4-deep \
         10-across pit cannot trap and its constants say so",
    ),
    site(
        "bastion-server/src/bastion_flat_arena.rs",
        "BASTION_FLAT_ARENA_PIT",
        GameplayVariant,
        "the PIT fixture's depth gate -- a PRE-EXISTING registration gap, not \
         from the rows that surround it here: this variable predates them and \
         was already unregistered, which means this manifest test was RED before \
         those rows and nobody had run the full suite to see it",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_TIGHTDIG",
        GameplayVariant,
        "the FR15-TIGHTDIG alternate stuck-economy metric -- a real gameplay \
         variant, off by default (see docs/BASTION_RUN_LOG.md)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_GOTO_WRITER_DIAG_UID",
        Diagnostic,
        "goto-writer diagnostic filter (second site, same variable as \
         server/src/sys/agent/mod.rs)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_M3_QUEUE_WAIT_BUDGET_TICKS",
        GameplayVariant,
        "test-only queue-wait budget override, documented \"never set by \
         live binaries\" -- test-only usage does not change what the \
         variable itself does if set",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_LEGC_DIAG",
        Diagnostic,
        "LEG-C target-stability diagnostic logging toggle",
    ),
    site(
        "bastion-server/src/bastion_path.rs",
        "BASTION_LEGC_DIAG",
        Diagnostic,
        "same variable as bastion_jobs.rs above, second site",
    ),
    site(
        "world/src/civ/airship_route_map.rs",
        "AIRSHIP_ROUTES_LOG_FOLDER",
        Diagnostic,
        "airship route diagnostic log folder (3 call sites, one variable)",
    ),
    site(
        "world/src/site/genstat.rs",
        "SITE_GENERATION_STATS_LOG",
        Diagnostic,
        "worldgen site-stats log path",
    ),
    site(
        "world/src/site/genstat.rs",
        "SITE_GENERATION_STATS_VERBOSE",
        Diagnostic,
        "worldgen site-stats verbosity toggle -- resolved name behind \
         get_bool_env_var's &str-parameter indirection (see module doc)",
    ),
    // Class-1 catalog batch (2026-08-09): 12 AUTON-2/bastion diagnostic
    // toggles + 1 gameplay-variant override landed across several
    // sessions without a same-commit registry row -- exactly the debt
    // this catalog exists to prevent. Each verified individually against
    // its actual gated block (not assumed from the "_DIAG" naming
    // convention): every one below gates ONLY an `info!` call; any
    // state mutation at the same site happens unconditionally outside
    // the env-var check.
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ARB_PERSONAL_DIAG",
        Diagnostic,
        "per-tick personal-drive arbitration snapshot (uid/severity/active job)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ARB_SWITCH_DIAG",
        Diagnostic,
        "drive-switch event snapshot, gated to a single call site",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_DECAY_JOIN_DIAG",
        Diagnostic,
        "decay-needs join population + effective mood-config rates, every 300 ticks",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_NEED_LOAD_FILTER_DIAG",
        Diagnostic,
        "is_loaded-filter A/B/C counters (pre/post-filter population, dropped count)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_NEED_SKIP_DIAG",
        Diagnostic,
        "per-colonist need-check skip reason (no_food_found, preempt_cooldown_active, etc.)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_RELEASE_DIAG",
        Diagnostic,
        "to_release site-scan trace (source line only, ~40 call sites, one variable)",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ROWB_BENCH",
        GameplayVariant,
        "Row B amnesty-bench escalation toggle -- off = today's behavior bit-for-bit \
         (benched_until_tick never populated); on = the escalation path is live. Not \
         diagnostic: gates a real state write (job.benched_until_tick), read by the \
         amnesty sweep. Paired-A/B only (--b5-rowb-paired), never a silent default-on.",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ROWB_DIAG",
        Diagnostic,
        "Row B bench-set event snapshot -- gates only the info! at the write site; \
         the write itself (job.benched_until_tick) is gated by BASTION_ROWB_BENCH above, \
         not this variable",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_SDIST_TRACE_JOB",
        Diagnostic,
        "one-off per-tick surface-distance trace for a single job id \
         (BASTION_SDIST_TRACE_JOB=<id>), gated to prevent corpus-wide unbounded logging",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_SELFJOB_COMPLETION_DIAG",
        Diagnostic,
        "self-job completion event snapshot (RestAt sleep-restored, etc.), 3 call sites",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_SETTLE_VIOLATION_DIAG",
        Diagnostic,
        "pre-sweep snapshot of labor-hold self-jobs about to be orphan-swept -- the \
         settle_invariant_violations counter itself increments unconditionally outside \
         this gate",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_STUCK_TERRAIN_DIAG",
        Diagnostic,
        "terrain-column snapshot around a stuck colonist's feet",
    ),
    site(
        "common/src/bastion.rs",
        "BASTION_AUTON2_MOOD_OVERRIDE",
        GameplayVariant,
        "test-only MoodConfig override, env-gated, off by default -- REPLACES the \
         config wholesale when set (never merges/shadows the shipped asset). Not \
         diagnostic: changes actual decay rates/tuning, not just what gets logged. \
         Fail-loud identity-or-loud refactor for the shipped asset path itself is \
         separately tracked design debt, not part of this registration.",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ACCESS_STALE_SECS",
        GameplayVariant,
        "F3 pruner's idle-plan threshold (default 20.0s) -- ITEM 2's threshold-\
         unblock row, 2026-08-10. Not diagnostic: changes when an abandoned \
         access plan actually gets removed. Malformed value REFUSES rather than \
         silently defaulting -- see access_stale_secs()'s own doc.",
    ),
    site(
        "bastion-server/src/bastion_jobs.rs",
        "BASTION_ACCESS_STALL_SECS",
        GameplayVariant,
        "F3 pruner's claimed-no-progress threshold (default 120.0s, PROVISIONAL \
         pending #70 corpus fan). Not diagnostic: changes when a stalled-but-\
         claimed access plan actually gets removed. Malformed value REFUSES \
         rather than silently defaulting -- see access_stall_secs()'s own doc.",
    ),
];

/// Lines the scanner must not treat as an unregistered literal site: the
/// indirection wrapper's OWN internal call, whose argument is a runtime
/// `&str` parameter, not a variable name. The two real names it resolves
/// to (`SITE_GENERATION_STATS_LOG`/`_VERBOSE`) are registered above
/// directly against their actual call sites.
const INDIRECTED_EXEMPT_LINES: &[(&str, &str)] =
    &[("world/src/site/genstat.rs", "match env::var(var_name).ok().as_deref() {")];

/// Catalog entries that are real but structurally invisible to this
/// scanner -- manually verified by reading the source (see module doc),
/// not by the literal-string scan the staleness check otherwise relies
/// on. Exempted from `every_catalog_entry_still_matches_a_live_site` for
/// that reason: a scanner miss here is EXPECTED, not evidence of a
/// rename/removal.
const MANUALLY_VERIFIED_INDIRECTED: &[(&str, &str)] = &[
    ("bastion-server/src/bastion_flight_recorder.rs", "BASTION_FLIGHT_RECORDER_UID"),
    ("bastion-server/src/bastion_flight_recorder.rs", "BASTION_FLIGHT_RECORDER_SAMPLE_EVERY"),
    ("bastion-server/src/bastion_flight_recorder.rs", "BASTION_FLIGHT_RECORDER_MAX_SAMPLES"),
    ("bastion-server/src/bastion_flight_recorder.rs", "BASTION_FLIGHT_RECORDER_MAX_EVENTS"),
    ("world/src/site/genstat.rs", "SITE_GENERATION_STATS_VERBOSE"),
    // ROW-ITEM6-WITNESS-PACKET part A1: `access_stale_secs()`/
    // `access_stall_secs()` both route their actual `std::env::var(var)`
    // call through the shared `env_threshold_secs_or_refuse(var, ..)`
    // helper, so the literal name only appears at the CALLER (as an
    // argument), never on the same line as `env::var(` itself -- same
    // indirection shape as the flight-recorder entries above, verified by
    // reading the source, not by the scanner.
    ("bastion-server/src/bastion_jobs.rs", "BASTION_ACCESS_STALE_SECS"),
    ("bastion-server/src/bastion_jobs.rs", "BASTION_ACCESS_STALL_SECS"),
];

/// Re-scans the given directories (each some `<crate>/src`) right now for
/// `std::env::var[_os](` and `env::var[_os](` call sites with a literal
/// string-argument, returning `(file relative to `workspace_root`,
/// variable name)` pairs -- collapsed to unique pairs, matching the
/// catalog's own granularity (a variable read 40 times in one file is
/// one row, not forty).
///
/// Takes `workspace_root` SEPARATELY from `dirs`: stripping each
/// directory against ITSELF (as a naive per-crate walk would) loses the
/// `server/src/`-style prefix the catalog above is keyed on, producing
/// `sys/item.rs` instead of `server/src/sys/item.rs`.
pub fn scan_live_env_reads(workspace_root: &Path, dirs: &[&Path]) -> Vec<(String, String)> {
    let mut out = HashSet::new();
    for dir in dirs {
        scan_dir(workspace_root, dir, &mut out);
    }
    let mut out: Vec<_> = out.into_iter().collect();
    out.sort();
    out
}

fn scan_dir(base: &Path, dir: &Path, out: &mut HashSet<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            scan_dir(base, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            scan_file(base, &path, out);
        }
    }
}

fn scan_file(base: &Path, path: &Path, out: &mut HashSet<(String, String)>) {
    let Ok(contents) = fs::read_to_string(path) else { return };
    let rel = path.strip_prefix(base).unwrap_or(path).to_string_lossy().replace('\\', "/");
    // The registry's own file is exempt: it quotes every variable name as
    // a string literal in the catalog above, which would otherwise look
    // like 38 fresh call sites to itself.
    if rel.ends_with("host_input_manifest.rs") {
        return;
    }
    for line in contents.lines() {
        let trimmed = line.trim();
        if INDIRECTED_EXEMPT_LINES.iter().any(|(f, l)| rel.ends_with(*f) && trimmed == *l) {
            continue;
        }
        if let Some(var) = extract_literal_env_var(line) {
            out.insert((rel.clone(), var));
        }
    }
}

/// Extracts the variable name from a `std::env::var("X")`,
/// `env::var("X")`, `std::env::var_os("X")`, or `env::var_os("X")` call
/// on this line. Returns `None` for a dynamic (non-literal) argument --
/// deliberately: those are a different, harder problem (see module doc).
fn extract_literal_env_var(line: &str) -> Option<String> {
    for marker in ["env::var_os(\"", "env::var(\""] {
        if let Some(start) = line.find(marker) {
            let after = &line[start + marker.len()..];
            if let Some(end) = after.find('"') {
                return Some(after[..end].to_string());
            }
        }
    }
    None
}

/// The subset of the catalog whose VALUE (not just presence) is worth
/// recording into a run's attestation header -- `Diagnostic` sites don't
/// change simulated behavior, so remembering whether they were set adds
/// provenance noise without explaining a divergence.
pub fn attestation_relevant_vars() -> impl Iterator<Item = &'static str> {
    CATALOG
        .iter()
        .filter(|s| !matches!(s.class, Diagnostic))
        .map(|s| s.var)
}

/// Read once at boot: the ACTUAL values (present/absent, not full string
/// contents for anything that might carry a path) of every
/// attestation-relevant variable. Cheap (a handful of env reads, once),
/// and its own presence in a run's attestation header answers "was this
/// run's behavior variant-affected" without re-deriving it from scratch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostInputManifestV1 {
    /// (variable name, was it set) for every attestation-relevant
    /// variable, in the catalog's own declared order.
    pub present: Vec<(&'static str, bool)>,
}

impl HostInputManifestV1 {
    pub fn capture() -> Self {
        Self {
            present: attestation_relevant_vars()
                .map(|var| (var, std::env::var_os(var).is_some()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .expect("workspace root must resolve")
    }

    fn scan_roots() -> Vec<std::path::PathBuf> {
        let root = workspace_root();
        ["common/src", "server/src", "rtsim/src", "bastion-server/src", "world/src"]
            .iter()
            .map(|p| root.join(p))
            .collect()
    }

    /// The completeness gate: every live literal env-read site found by a
    /// fresh scan must already be in `CATALOG`.
    #[test]
    fn every_live_env_read_is_classified() {
        let root_bufs = scan_roots();
        let roots: Vec<&Path> = root_bufs.iter().map(|p| p.as_path()).collect();
        let live = scan_live_env_reads(&workspace_root(), &roots);
        let catalog: HashSet<(&str, &str)> =
            CATALOG.iter().map(|s| (s.file, s.var)).collect();

        let unregistered: Vec<_> = live
            .iter()
            .filter(|(file, var)| !catalog.contains(&(file.as_str(), var.as_str())))
            .collect();
        assert!(
            unregistered.is_empty(),
            "unregistered env-var read sites found:\n{:#?}",
            unregistered
        );
    }

    /// Falsifier: every catalog entry must still correspond to a real,
    /// live site -- catches a rename/removal leaving a stale row behind.
    #[test]
    fn every_catalog_entry_still_matches_a_live_site() {
        let root_bufs = scan_roots();
        let roots: Vec<&Path> = root_bufs.iter().map(|p| p.as_path()).collect();
        let live: HashSet<(String, String)> =
            scan_live_env_reads(&workspace_root(), &roots).into_iter().collect();

        let stale: Vec<_> = CATALOG
            .iter()
            .filter(|s| !live.contains(&(s.file.to_string(), s.var.to_string())))
            .map(|s| (s.file, s.var))
            .filter(|entry| !MANUALLY_VERIFIED_INDIRECTED.contains(entry))
            .collect();
        assert!(stale.is_empty(), "catalog entries with no live site:\n{:#?}", stale);
    }

    /// The exemption list itself must name entries that actually exist in
    /// `CATALOG` -- otherwise it's silently exempting nothing and the
    /// test above isn't testing what this comment claims.
    #[test]
    fn manually_verified_indirected_entries_are_real_catalog_rows() {
        let catalog: HashSet<(&str, &str)> = CATALOG.iter().map(|s| (s.file, s.var)).collect();
        for entry in MANUALLY_VERIFIED_INDIRECTED {
            assert!(catalog.contains(entry), "{entry:?} is not in CATALOG at all");
        }
    }

    /// Falsifier, the required direction: a planted, unregistered
    /// `std::env::var` read in an authoritative crate must be CAUGHT, not
    /// silently absorbed.
    #[test]
    fn falsifier_a_planted_unregistered_read_is_flagged() {
        let dir = std::env::temp_dir().join(format!(
            "host_input_manifest_falsifier_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("planted.rs"),
            "fn planted() { let _ = std::env::var(\"BASTION_TOTALLY_UNREGISTERED_VAR\"); }\n",
        )
        .unwrap();

        let live = scan_live_env_reads(dir.as_path(), &[dir.as_path()]);
        assert!(
            live.iter().any(|(_, var)| var == "BASTION_TOTALLY_UNREGISTERED_VAR"),
            "the scanner failed to find the planted read at all"
        );
        let catalog: HashSet<&str> = CATALOG.iter().map(|s| s.var).collect();
        assert!(
            !catalog.contains("BASTION_TOTALLY_UNREGISTERED_VAR"),
            "the falsifier's own planted name accidentally collides with a real entry"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn capture_reports_absence_for_an_unset_attestation_relevant_var() {
        // BASTION_TIGHTDIG is attestation-relevant (GameplayVariant) and
        // not set in the test environment.
        let manifest = HostInputManifestV1::capture();
        assert!(
            manifest.present.iter().any(|&(var, set)| var == "BASTION_TIGHTDIG" && !set),
            "expected BASTION_TIGHTDIG present-but-unset in the captured manifest"
        );
    }

    #[test]
    fn diagnostic_class_vars_are_excluded_from_attestation_relevance() {
        let relevant: HashSet<&str> = attestation_relevant_vars().collect();
        assert!(
            !relevant.contains("BASTION_EGRESS_DIAG"),
            "a Diagnostic-class var should not be attestation-relevant"
        );
    }

    /// Every entry's `note` must say something, not just exist -- an
    /// empty or placeholder note defeats the point of a classification
    /// registry (a future reader needs the reasoning, not just the tag).
    #[test]
    fn every_entry_has_a_substantive_note() {
        for entry in CATALOG {
            assert!(
                entry.note.len() > 10,
                "{}::{} has no substantive note",
                entry.file,
                entry.var
            );
        }
    }
}
