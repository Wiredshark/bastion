# renderer-bench W1 — fork re-admission + build record (2026-08-20, Fable)

**Ben directive: build the renderer build list in full on the merge-later
fork `bastion/renderer-integration`.** The W0 handoff's freshness rule says
any base mismatch "requires a new W0 admission" — this file IS that
re-admission, on the fork's own base.

## Base attestation
- Branch: `bastion/renderer-integration`, forked from
  `bastion/wip-batch-verify @ 424ba82009` (the tick-loading-fix tip).
- W0's `source_commit 5de5361bc5` (2026-07-21) is ~1 month behind this base;
  the four allowed-existing files have all moved since. DELTA DISPOSITION:
  every W0 `source_fact` was RE-VERIFIED against the new base by read —
  `comp/mod.rs` still exports `pub mod bastion` (R0D-CONTRA-003 holds);
  `bastion-harness` is still a binary package with independent `tests/`
  compilation (R0D-CONTRA-004 holds); the synced-components x-macro and
  non-root re-export precedent (`Heads`) still hold. No contradiction with
  the W0 bundle survives on the new base.
- The W0 artifact pins are ENFORCED AS A TEST
  (`bastion-harness/tests/renderer_bench_reference.rs::mirrored_artifacts_match_handoff_pins`)
  — drifted vector files fail the suite, so freshness stops being a ritual.

## What W1 built (all inside the handoff's allowed-files list)
- `common/src/renderer_bench.rs` — the canonical contract: LE writer/strict
  reader; W0 tag tables (TYPE/DOMAIN/OWNER, numeric values contractual);
  fixture manifest RBDM v1 encode + fail-closed decode (unknown tags,
  trailing bytes, zero/dup ids, unsorted order all refuse); frame token
  RBFT v1 (164B) encode/decode; the leaf→owner→domain→frame→run hash
  hierarchy (incl. the W0 double-length-prefix run-id quirk, reproduced and
  documented); character presentation state; figure-key projection; the
  exactly-once readback registry.
- `common/src/comp/bastion.rs` — `RendererBenchEntityId` (synced, flagged
  storage). `common/net/src/synced_components.rs` — x-macro entry +
  explicit non-root re-export + `NetSync` (AnyEntity).
  `common/state/src/state.rs` — component registration + readback-registry
  resource. `common/src/lib.rs` — module export, no feature gate.
- Tests: `common/tests/renderer_bench_vectors.rs` (every canonical + every
  reviewed vector, byte-exact from the checked-in JSON, plus refusal cases:
  NaN, -0.0 normalization, dup slot/id, trailing bytes, unknown tag, author-
  order normalization, component-order preservation, exactly-once registry);
  `common/net/tests/renderer_bench_sync.rs` (registration as compile-time
  witness + wire-size pin); `bastion-harness/tests/renderer_bench_reference.rs`
  (artifact-pin enforcement + harness-side recomputation).
- Mirrored contract artifacts: the four W0 JSONs under this directory.

## Honest state / owed at the merge gate
- **COMPILE-DEFERRED**: no cargo has run on this branch (the mandate's
  builds own the machine; project hard rule). The merge gate owes:
  `cargo test -p veloren-common --test renderer_bench_vectors`,
  `-p veloren-common-net --test renderer_bench_sync`,
  `-p bastion-harness --test renderer_bench_reference`, plus a planted
  red-demonstration (flip one tag-table value → the vector suite must go
  red by name).
- Frame-token trailing field NAMES are W1-provisional (bytes contractual);
  W2 binds semantics. Downstream stays per the handoff:
  ARCHITECTURE_SELECTION / GOLDEN_PROMOTION / R0P / W2+ remain BLOCKED until
  the W1 suite passes at the merge gate.
