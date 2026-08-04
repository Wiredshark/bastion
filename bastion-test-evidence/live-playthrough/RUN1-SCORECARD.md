# Live Playthrough — Run 1 (reconnaissance-grade)

Driven by `client/src/bin/bastion_playtest.rs`, a real programmatic client (not
harness injection) against a real hosted `server-cli` instance. Scripts:
`script-01-recon.txt`, `script-02-playthrough.txt`. Raw logs: `driver-3.log`,
`driver-playthrough.log`, `server-stdout-3.log` (+ `.clean.log` with ANSI
stripped), `server-stderr-3.log`.

## Setup

- Engine tip: `7590dfa962` (ROW B-PRIME), on `bastion/wip-batch-verify`, in
  `.engine-integration-wt`. Attested by grepping the compiled
  `veloren-server-cli.exe` for the `ROWB-DIAG` string (2 hits, matching the
  two diag call sites in `bastion_jobs.rs`) — the server-startup version
  banner prints a stale `f7072cd3` hash, a separate cosmetic build-stamp bug
  (noted below), not evidence of what's compiled in.
- World seed: `130626853` (`world::sim::DEFAULT_WORLD_SEED`, unmodified — a
  fresh `VELOREN_USERDATA` always takes the default on first boot).
- Flags: `--no-auth` (no admin-role gate exists on `BastionSpawnColony`
  server-side despite its error text saying "need god mode" — the actual
  check is just `presence.bastion_terrain_anchor.is_some()`, so no admin
  seeding was needed). `BASTION_ROWB_BENCH=1` — a deliberate deviation from
  "shipped default" (OFF) specifically to give rows 14-17 a chance to fire;
  see the ruling below.
- Player spawn: `(15216.5, 16016.5, 419.0)`, deterministic across two
  fresh-userdata boots (confirmed byte-identical on reconnect).

## P0 finding, found and fixed before the scored run

**Every client disconnect crashed the server**, 100% reproducible,
independent of Bastion — a double-borrow panic in
`server/src/events/player.rs:handle_client_disconnect`. An `if let Some(uuid)
= ... && let Some(session_id) = server.state().ecs().read_resource::<SessionRegistry>()...`
chain keeps its `read_resource` `Ref` alive for the whole `if` body (Rust's
temporary-lifetime-extension rule for let-chains), and the body immediately
takes `write_resource::<SessionRegistry>()` on the same resource — an
overlapping borrow that `atomic_refcell` catches and panics on. This is
APEX-T3.2 session-registry code, unrelated to Row B′ or anything I built —
harness testing never triggers a real network disconnect this way, so
nothing in the corpus could have found it. Fixed by hoisting the read into
its own statement (so the `Ref` drops before the write), verified by
reproducing the crash, applying the fix, rebuilding, and reproducing the
*absence* of the crash on the identical disconnect. This is the single
highest-value thing this run found — every future live-playthrough attempt
would have hit it on the very first disconnect.

## Row B′ acceptance rows 14-17 (bench mechanism)

`ROWB-DIAG` count over the whole run: **0**. The bench/graduate branch never
fired — no job accumulated `PERSIST_ESCALATE_STRIKES` (3) while unreachable
in this run; colonists resolved every designation fast enough that nothing
ever needed benching. Rows 14-17 are therefore **not exercised this run** —
not a failure, a precondition that never arose. This is consistent with
Opus's corpus finding ("harmlessness proven, benefit not measurable") and
extends it: even with the flag deliberately ON and a live colony working a
423-cell mine footprint, the mechanism stayed dormant the whole run. A
future run that deliberately traps a colonist's only reachable job behind a
sealed volume (to force real strikes) would be needed to see rows 14-17
actually exercise; this run didn't attempt that.

## Scorecard (13 original rows + 4 Row B′ rows)

Player-language, no harness fields in the verdict sentence. "Observed" =
saw the colonist do it and the designation resolve; counts are from the
server's own log line, cross-checked for internal consistency
(placed = completed + phantom-retired, where phantom-retire means a
neighbor's completion already cleared the same cell).

1. **Mine** — OBSERVED, worked well. 423 cells designated, 108 mined by a
   colonist, 315 retired because a neighbor had already cleared the same
   cell first (dedup working as intended) — 423/423 accounted for, zero
   left stuck. 234 total job-claim events across all designations; only 5
   `job unreachable` releases all run, all outside the mine footprint.
2. **Chop** — NOT EXERCISED. My chosen footprint had no trees ("No trees
   rooted in the marked area" — accurate, honest player-facing message);
   this is a site-selection miss on my part, not a defect.
3. **Build** — OBSERVED, worked. 71 cells designated, 15 completed
   (a colonist placed a block), 56 phantom-retired — 71/71 accounted for.
4. **Farm full cycle** — NOT OBSERVED progressing. Plot registered (49
   cells), 0 jobs generated the entire ~4.5 minutes, `growth: None`
   unchanged at every checkpoint. Matches the pre-existing known defect
   (task #60, farm_tilled/farm_sown always false) — not a new finding.
5. **Haul / stockpile** — OBSERVED. Stockpile zone registered; at least one
   `haul delivered` event seen directly in the client's chat/job trace, 87
   total haul-delivered events in the server log across the run.
6. **Bed / sleep (build side)** — OBSERVED for the build half: 8/8 bed jobs
   completed ("bed registered (built)" x8, matching all 8 colonists). Did
   not observe a colonist actually sleeping in one this run (run too short
   to catch a fatigue cycle) — build succeeded, use-cycle not exercised.
7. **Storm-flee** — NOT EXERCISED (no storm occurred in this window).
8. **Pit-rescue** — NOT EXERCISED (no pit hazard encountered).
9. **Cave-in survival + conservation** — NOT EXERCISED (no cave-in
   triggered; mine footprint was a clean surface dig, not deep enough to
   risk one).
10. **Coordination barks** — OBSERVED. Repeated "Crowded here — I'll work
    where they're short-handed." lines from multiple colonists early in the
    run, consistent with the anti-crowding/load-balancing bark firing for
    real under real crowding (8 colonists, one small mine footprint).
11. **Zones** — NOT EXERCISED (no zone designation painted this run;
    already flagged known-red in the prep doc, not re-tested here).
12. **Run-speed feel** — NOT ASSESSABLE from logs alone (a felt-experience
    row; this run had no interactive human observer, only a scripted
    driver — genuinely not scoreable from this instrument).
13. **Blocked-designation messaging** — PARTIALLY OBSERVED. The chop
    "No trees rooted in the marked area" message fired correctly and was
    true. Did not encounter the previously-proven-defective
    plan_access/route_exhausted over-claiming message this run (no
    designation actually went unreachable-and-stayed-that-way long enough
    to trigger it) — the known defect (task #55 lineage) was not
    re-exercised either way.
14. **Row B′: stuck jobs noticed** — NOT EXERCISED (see above; 0
    `ROWB-DIAG` events).
15. **Row B′: colony stops paying attention to a stuck job** — NOT
    EXERCISED (same reason).
16. **Row B′: benching releases** — NOT EXERCISED (same reason).
17. **Row B′: the message is true** — NOT EXERCISED (same reason).

## Instrument lessons (not game bugs)

- **Driver's `inspect_cell` has a one-request read lag.** The script sends
  `BastionInspect`, ticks once, then reads `client.bastion_inspect()` — but
  the reply for that exact request doesn't reliably land within a single
  tick, so the logged value is frequently the *previous* request's reply,
  one step behind. This was caught by comparing the `Cell(...)` echoed in
  the reply against the coordinate the log line claims to have just
  requested — they didn't always match. Server-side log lines (job
  claimed/completed/etc.) were used as the actual source of truth for this
  scorecard instead of the client-side inspect trace, which is only
  corroborating color. A future run should wait for the reply's own
  `target` to match the request before trusting it, not a fixed tick count.
- **The server's version banner is stale** (`f7072cd3` printed at boot,
  while the actual compiled tip was `7590dfa962` — 1 commit later). Not
  investigated further (likely a build.rs re-run-if-changed scoping gap);
  attested the real tip via a binary string search instead of trusting the
  banner. Anyone using the printed version to attest a live server's code
  should not.

## Deviation from the launch checklist, and why

The checklist says "shipped tip, flag at its shipping default" (OFF for
`BASTION_ROWB_BENCH`). I ran Run 1 with it ON instead, specifically to give
rows 14-17 a chance to fire. It didn't change anything else observable —
rows 1-13 are unaffected by the flag either way.

## Run 2 — same script, true shipping default (`BASTION_ROWB_BENCH` unset)

Confirming run: identical `script-02-playthrough.txt`, fresh `VELOREN_USERDATA`,
default flags. Raw logs: `driver-default.log`, `server-stdout-4.log(.clean)`,
`server-stderr-4.log`.

- `ROWB-DIAG` count: 0 (expected — flag unset, matches Run 1's own 0).
- Designation placement identical to Run 1 (same deterministic world/seed,
  same footprints): Mine 423, Build 71, Bed 8, Stockpile/Farm/Ladder 0 jobs
  at placement.
- Full accounting held again: Mine 129 completed + 294 phantom-retired =
  423/423; Build 16 + 55 = 71/71; Bed 6 + 2(phantom) = 8/8. Per-run
  completed-vs-phantom-retired split shifts slightly run to run (real-time
  colonist scheduling isn't lockstep-deterministic across two live wall-clock
  runs), but the qualitative outcome — everything in every footprint gets
  resolved, nothing left permanently stuck — matches Run 1 exactly.
- `job claimed`: 252 (Run 1: 234). `job unreachable`: 5 (Run 1: 5, same).
- No WARN/ERROR lines beyond the same boot-time noise as Run 1. `stderr`
  empty — clean disconnect, no crash (confirms the P0 fix holds under the
  shipping-default flag combination too, not just the BENCH=1 combination
  it was originally caught under).

Live confirmation: rows 1-13 behave the same live under both flag settings,
matching what the harness-level 48-seed corpus already established. This
closes the checklist deviation from Run 1.
