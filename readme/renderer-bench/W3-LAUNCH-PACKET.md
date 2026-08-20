# W3 launch packet (A4) — replication_projection_ack v1 (self-authored)

Codex never issued the A4 packet; per the fork mandate this packet is
authored here, inside lease r0d-lease-03-w3's surfaces (common message
enums; client/server message handlers; session readiness hook) and its
forbidden-surface rules. Interface implemented: `replication_projection_ack`
(manifest v1). Domains used: the W0 table's pre-allocated
`ReplicationProjection = 4` and `ClientProjection = 5`.

## Semantics (registered before build)
1. **Announce** (server→client, in-game stream):
   `ServerGeneral::RendererBenchFrame(BenchFrameAnnounceV1)` — sent to all
   in-game clients (spectators included) at every cadence frame, carrying
   `run_id, frame_index, sim_tick, frame_root, cadence, run_ticks,
   arena_origin_mm, entity_count`.
2. **Readiness** (client→server): `ClientGeneral::RendererBenchReady`.
   Sent once by voxygen when the session becomes ready with
   `BASTION_RENDERER_BENCH_ACK=1`, and by the headless ackbot after
   spectate success. Server side: counts readiness; with
   `BASTION_RENDERER_BENCH_WAIT_CLIENT=1` the run does not START until
   ready_count ≥ 1 (default off — the headless twin legs are unchanged).
3. **Ack** (client→server): `ClientGeneral::RendererBenchProjectionAck(
   BenchProjectionAckV1)` — on each announce receipt the client computes,
   from ITS OWN replicated ECS view, the ClientProjection domain root over
   all entities carrying the synced `RendererBenchEntityId` (leaf
   `0x0500_0001`, mm position, FixedI32×3, StableEntity owner key =
   semantic id LE — the exact server shapes, computed by the SAME shared
   functions in `common::renderer_bench`), echoes the announced
   `frame_root` verbatim, and reports `entities_resolved`.
4. **Tape**: acks land in the artifact as a `client_acks` sidecar array
   (`frame_index, sim_tick, echo_match, client_projection_root,
   entities_resolved`). **run_root is computed over server frames ONLY —
   unchanged.** Client acks are wall-coupled observations (which sync
   snapshot the client held at receipt) and the project law says
   determinism and wall-coupling are mutually exclusive observables; so
   ack CONTENT is evidence, never identity.

## What the ack channel PROVES (claims kept honest)
- `echo_match=true` per ack → the announce/ack message channel carried the
  32-byte root faithfully (channel proof).
- `entities_resolved == manifest entity count` → the synced component
  arrived and the client can enumerate the population by SEMANTIC id, not
  runtime uid (replication proof — the forbidden-surface rule about
  runtime ids as identity is honored by construction).
- `client_projection_root` equal to the server's FigureSourceProjection
  root is NOT asserted (sync timing lags); it is recorded for study. What
  IS asserted by the integrated smoke: a run with a live acking client
  produces the IDENTICAL run_root to a clientless run (replication
  neutrality — observing through the net does not perturb the tape).

## Wire plumbing (surfaces touched)
- **Stream decision (revised during build):** all three messages ride the
  **General** stream, not InGame — they are out-of-band diagnostics
  (checkpoint class `OutOfBandDiagnostic`) and must never wait behind an
  in-game data fence; `CheckpointCommitAck` is the precedent. The verify
  gate still requires presence, so only in-session observers may speak.
- **Run identity made RUN-RELATIVE (the W2 doc reserved this revision):**
  the frame token's `sim_tick`, the announce's `sim_tick`, and the tape's
  frame `tick` are now ticks-since-run-start. With WAIT_CLIENT the run
  starts when the operator's client readies — an absolute boot tick would
  make identical runs differ by operator timing, which is wall-coupling
  inside deterministic identity.
- `common/net/src/msg/{server,client}.rs`: the three variants above;
  verify + checkpoint participation/refs + command admission + semantic
  routes all extended (every classification match is exhaustive — the
  compiler enumerated the sites); wire-shape goldens extended (the
  zero-length UNCOVERED lists force this at test time; counts pinned
  39/53).
- `common::renderer_bench`: `BenchFrameAnnounceV1`, `BenchProjectionAckV1`
  (serde), `CLIENT_PROJECTION_LEAF = 0x0500_0001`,
  `stable_entity_owner(schema, semantic_id, domain, leaf_id, wire_type,
  payload)` shared helper (client and server call ONE implementation —
  the "one shared type owner" invariant), plus resources
  `RendererBenchNetOutbox` / `RendererBenchClientSignals` (bastion-server
  cannot see `server::Client`, so announces drain through an outbox sys
  in the server crate and ready/acks flow back through signals).
- `server/src/sys/msg/general.rs`: Ready/Ack handlers → signals resource
  (interior mutability — that handler par_joins clients); in_game.rs
  refuses them as misrouted, same as the commit ack.
- `server/src/sys/renderer_bench_net.rs` (new, thin): outbox →
  `notify_in_game_clients` equivalent join (Client × Presence).
- `client/src/lib.rs`: announce handler (compute + ack under
  `BASTION_RENDERER_BENCH_ACK=1`), `renderer_bench_ready()`.
- `voxygen/src/session/mod.rs`: readiness hook beside the W2 capture hook.
- `client/src/bin/rbench_ackbot.rs`: headless SPECTATOR client (modeled
  on `bastion_playtest`; spectate is moderator-gated, so the runner's
  existing first-boot `admin add` covers it) — the integrated proof's
  second half.

## Gates for this wave (all must be green before W4)
- [ ] Python-independent vectors for the ClientProjection leaf/owner/domain
      shapes (new producer script + checked-in JSON; Rust reproduces
      byte-exact — the encoder never blesses its own vectors).
- [ ] Wire-shape goldens for all three new variants; coverage lists stay [].
- [ ] Planted red-demo: mutate `CLIENT_PROJECTION_LEAF` → exactly the
      client-projection consumers fail by name; restore; green.
- [ ] Integrated smoke (three legs): leg A server+ackbot (WAIT_CLIENT=1),
      leg B clientless, leg C clientless. Assert: A's tape has ≥2 acks,
      all `echo_match=true`, `entities_resolved=3`; run_root(A) ==
      run_root(B) == run_root(C) (replication neutrality + the existing
      twin determinism witness in one run).
- [ ] `cargo check -p veloren-common -p veloren-common-net -p veloren-client
      -p veloren-server -p bastion-server` clean + the three W1/W2 suites
      still green.
