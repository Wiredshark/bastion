[
    ("bastion-server/src/bastion_flight_recorder.rs", "SystemTime::now()", 0),
    // `E13` chunk 5, all four: the out-of-band server-browser query
    // protocol. `ratelimit.rs` shifts a token-bucket window,
    // `server.rs` rotates a challenge secret and stamps request
    // arrival, `client.rs` measures round-trip time. Wall-clock is the
    // CORRECT input for every one of them -- rate limiting against
    // simulation time would not rate-limit anything -- and none reaches
    // authoritative state. This crate is in the root set for
    // completeness, not because it carries authority.
    ("common/query_server/src/client.rs", "let query_sent = Instant::now();", 0),
    ("common/query_server/src/ratelimit.rs", "last_shift: Instant::now(),", 0),
    ("common/query_server/src/server.rs", "let mut last_secret_refresh = Instant::now();", 0),
    ("common/query_server/src/server.rs", "let now = Instant::now();", 0),
    ("common/src/baseline_regen.rs", "assert!(new_content.contains(\"(\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\"), \"the existing entry line must survive verbatim\");", 0),
    ("common/src/baseline_regen.rs", "assert!(new_content.contains(\"(\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\r\\n\"), \"the untouched existing line must keep its CRLF ending:\\n{new_content:?}\");", 0),
    ("common/src/baseline_regen.rs", "assert!(new_content.contains(\"(\\\"z/new.rs\\\", \\\"SystemTime::now()\\\", 0),\"), \"the new entry must be added\");", 0),
    ("common/src/baseline_regen.rs", "assert!(new_content.contains(\"(\\\"z/new.rs\\\", \\\"SystemTime::now()\\\", 0),\\r\\n\"), \"the new line must ALSO be CRLF, matching the file:\\n{new_content:?}\");", 0),
    ("common/src/baseline_regen.rs", "assert_eq!(added, vec![entry(\"z/new.rs\", \"SystemTime::now()\", 0)]);", 0),
    ("common/src/baseline_regen.rs", "assert_eq!(entries, vec![entry(\"a/b.rs\", \"Instant::now()\", 0), entry(\"c/d.rs\", \"SystemTime::now()\", 1)]);", 0),
    ("common/src/baseline_regen.rs", "assert_eq!(removed, vec![entry(\"a/b.rs\", \"Instant::now()\", 0)]);", 0),
    ("common/src/baseline_regen.rs", "let content = \"[\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\n    (\\\"c/d.rs\\\", \\\"SystemTime::now()\\\", 1),\\n]\\n\";", 0),
    ("common/src/baseline_regen.rs", "let content = \"[\\n    // a builder's own note\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\n]\\n\";", 0),
    ("common/src/baseline_regen.rs", "let crlf = \"[\\r\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\r\\n]\\r\\n\";", 0),
    ("common/src/baseline_regen.rs", "let crlf_existing = parse_baseline_file_v1(\"[\\r\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\r\\n]\\r\\n\");", 0),
    ("common/src/baseline_regen.rs", "let existing = parse_baseline_file_v1(\"[\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\n]\\n\");", 0),
    ("common/src/baseline_regen.rs", "let existing = parse_baseline_file_v1(\"[\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\n]\\n\");", 1),
    ("common/src/baseline_regen.rs", "let existing = parse_baseline_file_v1(\"[\\n    (\\\"a/first.rs\\\", \\\"Instant::now()\\\", 0),\\n    (\\\"z/last.rs\\\", \\\"Instant::now()\\\", 0),\\n]\\n\");", 0),
    ("common/src/baseline_regen.rs", "let existing_src = \"[\\n    // T4.6 chunk 3b's own annotation, must survive\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\n]\\n\";", 0),
    ("common/src/baseline_regen.rs", "let existing_src = \"[\\r\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\r\\n]\\r\\n\";", 0),
    ("common/src/baseline_regen.rs", "let lf = \"[\\n    (\\\"a/b.rs\\\", \\\"Instant::now()\\\", 0),\\n]\\n\";", 0),
    ("common/src/baseline_regen.rs", "let live = vec![entry(\"a/b.rs\", \"Instant::now()\", 0), entry(\"z/new.rs\", \"SystemTime::now()\", 0)];", 0),
    ("common/src/baseline_regen.rs", "let live = vec![entry(\"a/b.rs\", \"Instant::now()\", 0), entry(\"z/new.rs\", \"SystemTime::now()\", 0)];", 1),
    ("common/src/baseline_regen.rs", "let live = vec![entry(\"a/b.rs\", \"Instant::now()\", 0)];", 0),
    ("common/src/baseline_regen.rs", "let live = vec![entry(\"a/b.rs\", \"Instant::now()\", 0)];", 1),
    ("common/src/baseline_regen.rs", "let live = vec![entry(\"a/first.rs\", \"Instant::now()\", 0), entry(\"m/middle.rs\", \"Instant::now()\", 0), entry(\"z/last.rs\", \"Instant::now()\", 0)];", 0),
    ("common/src/baseline_regen.rs", "let live = vec![entry(\"z/new.rs\", \"SystemTime::now()\", 0)];", 0),
    ("common/src/clock.rs", "last_tick: Instant::now(),", 0),
    ("common/src/clock.rs", "last_work: Instant::now(),", 0),
    ("common/src/clock.rs", "let this_tick = Instant::now();", 0),
    ("common/src/clock.rs", "self.last_work = Instant::now();", 0),
    ("common/src/comp/chat.rs", "let timeout = Instant::now() + Duration::from_secs_f64(SpeechBubble::DEFAULT_DURATION);", 0),
    ("common/src/comp/presence.rs", "let now = Instant::now();", 0),
    ("common/src/slowjob.rs", "let execution_end = Instant::now();", 0),
    ("common/src/slowjob.rs", "let execution_start = Instant::now();", 0),
    ("common/src/slowjob.rs", "let queue_created = Instant::now();", 0),
    ("common/src/slowjob.rs", "let start = Instant::now();", 0),
    // `E13` chunk 3, all three: `common/state/src` entered the roots.
    // The two `plugin/mod.rs` hits are `#[cfg(test)]` helpers minting
    // unique temp-file names (`temp_tar`) -- not authoritative, same
    // class as the `T4.6` note below. `state.rs` is `MetricsGuard`, a
    // tick-duration metric: the "metrics/logging timestamp" case this
    // family's doc names as fine, not a decision input.
    ("common/state/src/plugin/mod.rs", "std::time::SystemTime::now()", 0),
    ("common/state/src/plugin/mod.rs", "std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()", 0),
    ("common/state/src/state.rs", "start: Instant::now(),", 0),
    ("server/src/chunk_generator.rs", "if std::time::Instant::now() > deadline {", 0),
    ("server/src/chunk_generator.rs", "let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);", 0),
    ("server/src/events/player.rs", "let now = std::time::Instant::now();", 0),
    ("server/src/events/player.rs", "std::time::Instant::now(),", 0),
    ("server/src/lib.rs", "let before_entity_cleanup = Instant::now();", 0),
    ("server/src/lib.rs", "let before_handle_events = Instant::now();", 0),
    ("server/src/lib.rs", "let before_new_connections = Instant::now();", 0),
    ("server/src/lib.rs", "let before_persistence_updates = Instant::now();", 0),
    ("server/src/lib.rs", "let before_state_tick = Instant::now();", 0),
    ("server/src/lib.rs", "let before_sync = Instant::now();", 0),
    ("server/src/lib.rs", "let before_update_terrain_and_regions = Instant::now();", 0),
    ("server/src/lib.rs", "let before_world_tick = Instant::now();", 0),
    ("server/src/lib.rs", "let end_of_server_tick = Instant::now();", 0),
    ("server/src/lib.rs", "self.state.ecs().write_resource::<TickStart>().0 = Instant::now();", 0),
    ("server/src/lib.rs", "state.ecs_mut().insert(TickStart(Instant::now()));", 0),
    ("server/src/metrics.rs", "let since_the_epoch = SystemTime::now()", 0),
    ("server/src/rtsim/mod.rs", "detected_at_unix_seconds: std::time::SystemTime::now()", 0),
    ("server/src/rtsim/mod.rs", "self.last_saved = Some(Instant::now());", 0),
    // `APEX-T4.6` chunk 3b: test-only (`#[cfg(test)]`), a bounded poll
    // loop de-flaking `vacuum_into_is_not_blocked_by_and_does_not_block_
    // concurrent_writer_commits` against real writer-thread scheduling
    // under a contended parallel test run -- not authoritative game
    // state, but the scan is text-based over the whole file, not
    // cfg-aware, so it self-catches here same as every prior instance.
    ("server/src/save_universe.rs", "let wait_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);", 0),
    ("server/src/save_universe.rs", "while rows_committed.load(Ordering::Relaxed) < 1 && std::time::Instant::now() < wait_deadline {", 0),
    ("server/src/session_registry.rs", "fn now() -> Instant { Instant::now() }", 0),
    ("server/src/state_ext.rs", "Instant::now(),", 0),
    ("server/src/sys/invite_timeout.rs", "// not wall-clock Instant::now().", 0),
    ("server/src/sys/metrics.rs", "let start = Instant::now();", 0),
    ("server/src/sys/mod.rs", "last_run: Instant::now(),", 0),
    ("server/src/sys/mod.rs", "last_run: Instant::now(),", 1),
    ("server/src/sys/mod.rs", "self.last_run = Instant::now();", 0),
    ("server/src/sys/msg/in_game.rs", "let time_for_vd_changes = Instant::now();", 0),
    ("server/src/sys/msg/register.rs", "let outcomes = session_registry.admit_sorted(intents, max_players, Instant::now(), DEFAULT_DETACHED_RETENTION_CAP, &mut random_source);", 0),
    ("server/src/sys/semantic_egress.rs", "let out = registry.admit_sorted(vec![((), intent)], 64, std::time::Instant::now(), 64, &mut src);", 0),
    ("server/src/sys/semantic_egress.rs", "registry.detach(recipient.session_id, std::time::Instant::now(), std::time::Duration::from_secs(60), 64);", 0),
    ("world/src/sim/erosion.rs", "let start_time = Instant::now();", 0),
    ("world/src/sim/erosion.rs", "let start_time = Instant::now();", 1),
    ("world/src/sim/erosion.rs", "let start_time = Instant::now();", 2),
    ("world/src/sim/erosion.rs", "let start_time = Instant::now();", 3),
    ("world/src/sim/mod.rs", "let now = std::time::Instant::now();", 0),
]
