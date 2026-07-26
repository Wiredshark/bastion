# APEX-T3.1 — Server Boot-Scoped Authority: status

MVP (packet section 9, 10 items) status: **8/10 complete, 2/10 partial.**
No item silently skipped — every gap below is named.

## Complete

1. **Production system-random adapter.** T0.4's existing `OsRandomBytesSourceV1`
   already satisfies T3.1.02 (see `readme/apex/APEX-T3.1-T0.4-ABI-REVALIDATION.md`)
   — no new code needed.
2. **Generate one boot ID before startup side effects.** `Server::new`
   (`server/src/lib.rs`) generates `server_boot_id` as the first fallible
   operation, before `ServerInitStage::DbMigrations`.
3. **Store in `Server` and ECS.** `Server::server_boot_id` field +
   `state.ecs_mut().insert(server_boot_id)`, both the same `Copy` value.
4. **Distribute in `ServerInfo`.** `ServerInfo.server_boot_id`
   (`common/net/src/msg/server.rs`), sent before authentication.
5. **Echo in `ClientRegister`, reject before auth.** `ClientRegister.
   expected_server_boot_id`; `server/src/sys/msg/register.rs` compares
   against `ReadExpect<ServerBootId>` before calling `login_provider.verify`
   — a mismatch never reaches auth (`PendingLogin::new_failure`, zero
   `verify` calls).
6. **Repeat in `GameSync`, reject before client `State`.** `ServerInit::
   GameSync.server_boot_id`; `client/src/lib.rs` compares against the
   `ServerInfo` observation before `State::client` construction.
7. **Typed mismatch errors.** `RegisterError::ServerBootMismatch{current,
   received}` (wire), `server::Error::BootIdentity` (startup),
   `client::Error::ServerBootMismatch{server_info, game_sync}` (both
   client-side checkpoints, one shared variant per the packet's own
   section 7.5 sketch).
8. **Network minor-version bump.** `VELOREN_NETWORK_VERSION` `[0,6,0]` ->
   `[0,7,0]`, confirmed no other pending change claims minor 7.

## Partial

9. **Restart/cross-transport tests.** Done: handshake version-gate tests
   (`network/protocol/src/handshake.rs` —
   `handshake_old_minor_version_rejected`,
   `handshake_patch_difference_still_accepted`) and wire round-trip tests
   for every new field (`common/net/src/msg/{client,server}.rs`), all under
   the real `bincode::config::legacy()` `network/src/message.rs` uses.
   **Not done:** the full T3.1.16 transport matrix (equivalent fixtures
   across MPSC/TCP/QUIC specifically for the boot-ID lifecycle) and the
   T3.1.17 process-restart integration fixture (capture ServerInfo/
   Register/GameSync under boot A, inject under boot B, assert rejection)
   — both need more integration-test scaffolding than this pass built.
10. **Exclusion from simulation roots/RNG.** Verified **by construction**:
    `server_boot_id`/`ServerBootId` do not appear anywhere in
    `common/src/state_hash.rs`, `bastion-harness/src/determinism_regression.rs`,
    `world/src/`, or `rtsim/src/` (grep-confirmed, zero matches) — there is
    no code path for it to leak into a state hash or RNG seed, because
    nothing built this pass reads `ServerBootId` from any of those places.
    **Not done:** T3.1.18's own mutation-canary test (deliberately wire it
    into a state hash and confirm the canary fails) — the stronger,
    belt-and-suspenders proof the packet asks for, beyond absence-by-
    construction.

## Not attempted this pass

- **T3.1.13/.14** (restart-invalidation-semantics documentation, cache
  survival matrix): design/doc-only rows: no code impact, deferred rather
  than rushed.
- **T3.1.19** (startup receipt/diagnostics logging, `ServerBootReceiptV1`):
  the boot ID is already logged (`info!("Server boot ID: {}", ...)` in
  `Server::new`); the structured `ServerBootReceiptV1` external-evidence
  record is not built.
- **T3.1.20** (full 64-canary catalog replay against
  `PROJECT-BASTION-APEX-T3.1-SERVER-BOOT-AUTHORITY-CANARIES-v1.json`): not
  run — the canary file's exact consumption format wasn't built out this
  pass. SHA-256 of the canary file independently verified against Fable's
  cited pin (exact match) before starting this row.

## What was actually tested, for real, this pass

- `common::apex::identity` (T0.4): 25 unit tests, including 3 new ones for
  T3.1's wire Serde (compact 16-byte encoding, invalid-bytes rejection).
- `common::apex::manifest`/`digest` (T0.2/T0.3): unaffected, still 94/94
  green combined with T0.1/T0.4.
- `common-net` wire structs: 3 new bincode-legacy round-trip tests
  (`ServerInfo`, `RegisterError::ServerBootMismatch`, `ClientRegister`).
- `network-protocol` handshake: 2 new tests (old-minor rejected,
  patch-only accepted), both driving the real handshake state machine, not
  a synthetic comparison.
- **Full workspace `cargo check --workspace`**: clean, zero errors, across
  every crate including `bastion-harness`, `server-cli`, `voxygen` — every
  call site that constructs `ServerInfo`/`ClientRegister`/`ServerInit::
  GameSync`/`RegisterError`/the two `Error` enums was found and fixed by
  the compiler, not by manual grep alone.
