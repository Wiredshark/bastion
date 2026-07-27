# SESSION HANDOFF — Sonnet 5 (builder), 2026-07-27 (T3.3 row closed, pivoting off APEX)

Opus continues APEX alone from here (Ben-direct program split: Sonnet → engine-improvement
volume lane, Opus → APEX T3.4/T3.5/the frontier). Everything below is committed + pushed to
`bastion-origin bastion/apex-t0`; tip at handoff = `0064056dc9` (T3.3.20). Branch:
`bastion/apex-t0`, worktree `E:\veloren-master\.apex-t0-wt`.

## What's DONE (T3.3.01–.20, full mechanism spine + builder-lane construction)

All committed, all green locally, all reported to Fable at each step:
- `5170568a22` T3.3.15 — `SemanticEgressSysV1`, the single canonical egress owner.
- `6410d300bf` T3.3.16 — Envelope GameSync as Bootstrap sequence 1.
- `c000dffafa` T3.3.17 — causality/snapshot-domain profiles (categories 5/6 in the frozen
  `NET_ENVELOPE_PROFILE_V1` table, profile_root bumped deliberately).
- `6c32b625f3` T3.3.18 — typed terminal/reject codes, protocol-disconnect mapping, redacted
  `(code, stream)`-keyed ingress counters.
- `9584a5ec9d` T3.3.19 (builder half) — `--net-envelope-scenario` (delay/duplicate/gap/reconnect
  injection against the real `validate_semantic_frame_v1`, widened `pub`).
- `0064056dc9` T3.3.20 — receive-side scan/catalog (`receive_inventory.rs`, zero bypass sites,
  pinned), 160-case canary coverage runner (`canary_coverage.rs`).

## What's OPEN — this is what you inherit

**1. The 133 `PostAuthCandidate` send sites (`server/src/semantic_net/send_inventory_catalog.rs`)
are STILL UNMIGRATED.** Only the "replication family" (entity_sync.rs + subscription.rs, T3.3.13/
14/14a) is done. Fable's own sequencing ruling (still standing, never revoked): finish the
mechanism spine first (.15→.20, now done), THEN the adoption sub-blocks in this exact order:
- **.14b** — ChatMsg family, `state_ext.rs` (23 sites).
- **.14c** — sys/msg request/response family: `in_game.rs` (16) + `character_screen.rs` (9) +
  general + terrain.
- **.14d** — events family: `group_manip` (18) + `invite` (12) + tail events.
- **.14e** — the 1-4-site tail.
Each gets its own SHORT chat-only design note first (Fable's own words), middle-tier discipline
(not elevated — T3.3.13's own live-parallel-region-worker gate was the only elevated one; these
are sequential, no new race class).

**2. Opus's own T3.3.19 execution leg** (assigned by Fable, arming condition = my `9584a5ec9d`
landing): full 160-companion-case × 1/2/8-worker × schedule-seed × compression-mode campaign via
`bastion-harness --net-envelope-scenario`. I only ran it twice at pin scale (fresh build both
times, PASS both). This is genuinely new coverage, not a rerun.

**3. `canary_coverage.rs`'s own honesty gaps, worth a look when you're in the neighborhood:**
- **ENV-152** (certified-mode config surface) is marked `GAP` — no config surface exists yet
  (T3.3.05's own row-status doc already deferred it to T4.1). Not falsely claimed; will need a
  real claim once that surface lands.
- A handful of the 160 claims are `"structural: <reasoning>"` rather than a pinpoint unit test —
  honest but worth tightening into real tests opportunistically if T3.4 work touches that code
  anyway (I did NOT go add tests purely to convert a structural claim into a literal one; that
  would have been scope creep on an already-marathon row).

**4. One real wrinkle, documented not solved** (`server/src/sys/semantic_egress.rs`'s own module
doc): `server/src/lib.rs`'s tick loop conditionally re-runs `terrain::Sys` AFTER
`run_sync_systems` (the `DisconnectType::WithoutPersistence` cleanup path). Inert today
(`terrain.rs` is still `PostAuthCandidate`, unmigrated) — but when `.14b-e` eventually migrates
terrain.rs, that path will enqueue an intent egress already flushed this tick, delaying it one
tick. Needs a decision when you get there: re-invoke egress after that re-run, or accept the
one-tick delay for that narrow disconnect path.

## T3.4/T3.5/T3.6 boundary — held throughout, keep holding it

Every row this session was built with an explicit non-overclaim discipline: T3.3 declares
vocabulary/mechanism only, never production/watermark/exactly-once/physics-rollback semantics.
`canary_coverage.rs`'s own ENV-155/156/157/158 claims name exactly where this boundary is proven
(structural: no code path in T3.3 claims any of those). When you start T3.4, the snapshot-domain
profile mechanism (`NetEnvelopeCausalityProfileV1`, `production_causality_profile_v1`) is where
you'll extend the declared-domain set and per-schema requirements — it's a live, parameterized
mechanism now, not a stub; the production instance is just deliberately empty/all-optional today.

## Small cautions that cost real time this session (worth banking)

- **cwd does not persist across separate Bash tool calls in this environment, full stop** —
  every single call that needs a specific directory needs `cd <dir> &&` embedded in THAT call,
  never relying on a previous call's `cd`. A `run_in_background: true` call is not special-cased;
  ALL calls reset. Always print `pwd && git branch --show-current &&` first and read them before
  trusting anything else in a result — caught two false-green runs this session that way (one
  "0 tests" tell, one silently-wrong-branch build).
- The frozen `send_inventory`/`receive_inventory` catalogs self-flag on their OWN new code more
  often than you'd expect — any new line containing `.send(`/`.recv(`/`send_fallible`/etc.
  (including inside a DOC COMMENT'S PROSE, not just real code) trips the scanner. Happened 4
  times this session. Reword the comment rather than fight the test, or add a real catalog entry
  if it's a genuine new site.
- `cargo check -p X -p Y` (narrow multi-package scope) can silently select a DIFFERENT feature
  set than `cargo check --workspace` (hit this with the `plugins` feature / `PluginMgr` on
  `client`) — false `E0061` from feature-unification, not a real bug. Prefer the full workspace
  check as the actual verification gate; use narrow scopes only for fast iteration.

Go build. Ping me at any cross-review boundary — the split doesn't end that.
