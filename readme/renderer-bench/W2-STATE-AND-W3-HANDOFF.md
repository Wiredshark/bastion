# renderer-bench W2 state + W3 handoff (fork build, 2026-08-20)

**W2 is BUILT on `bastion/renderer-integration`.** This file is the wave's
own state record and the successor handoff the W0 process expected — so the
campaign's documentation chain stays unbroken even though the wave gates are
now Ben-owned rather than Codex-owned.

## What W2 delivers (all committed on the fork)
1. **Fixture loading** (`bastion-server/src/bastion_renderer_bench.rs`):
   RBDM manifest → live entities via the normal `CreateNpcEvent` path;
   fail-closed decode (a bad manifest REFUSES loudly, once); bodies mapped
   index-checked (out-of-range = entity skipped by name, never clamped);
   spawned WITHOUT Agents — the bench is the only driver.
2. **Script protocol v1**: `Steps` = commanded velocity (milli-blocks/sec,
   step holds until the next step's tick); `Target` = unit-speed seek with
   0.25-block arrival; ordinals recorded per frame. Animation actions parse
   (W1) but carry no live consumer yet — W3 binds them.
3. **Semantic tape**: every N ticks one frame — token (run-id = manifest
   domain sha; parent chains to previous frame root) + three domains
   (FigureIdentity, FigureSourceProjection mm, ServerScriptState ordinal) →
   frame root; at the terminal tick, run root + atomic (tmp→rename)
   artifact `renderer-bench-tape-v1` JSON.
4. **Golden policy** (`bastion-harness`): `--renderer-bench-golden
   <candidate> <golden>` — PASS / MISMATCH(first divergent frame named) /
   MALFORMED / NO-GOLDEN(exit 3; promotion is a HUMAN copy, deliberately
   unimplemented as code).
5. **Visual capture hook** (voxygen): `BASTION_RENDERER_BENCH_CAPTURE=N`
   screenshots every N client ticks through the existing capture path —
   the human-eyeball sidecar; NOT semantic authority (W0 invariant).
6. **Runner**: `run-renderer-bench.sh` — twin legs on the flat arena +
   golden compare of the two tapes = the stack's determinism witness.
7. Env rows registered in `host_input_manifest`; sample fixtures under
   `fixtures/` produced by the INDEPENDENT Python encoder (not the Rust
   production encoder — the no-self-blessing rule extended to fixtures).

## W2 semantics decisions (revisable by W3 without byte breakage)
run-id=manifest-domain-sha · script-sha=manifest-domain-sha (script lives in
the manifest) · parent chain = previous frame ROOT · ppm = milli-blocks/sec
· three-domain frame set (additive) · tape artifact is JSON (the canonical
BYTES live in token/root hex fields; the envelope is for humans and the
golden CLI).

## What W3+ owes (the remaining campaign, none of it startable earlier)
- **Client projection + replication ack**: the voxygen-side ClientProjection
  domain (what the CLIENT resolved for each synced semantic id) and an ack
  channel proving replication carried it — the client half of the tape.
- **PassDraw / VisualStructure domains**: real draw-call/mesh-structure
  leaves from the renderer — this is where actual RENDERER improvements
  become measurable (a renderer change that alters no PassDraw root is
  provably visual-neutral; one that does names exactly what moved).
- **Action binding**: script actions 0–8 semantics + live consumers.
- **Golden promotion operations**: a golden store layout per scenario +
  the human promotion ritual documented (candidate → golden copy + ledger
  line). The CLI already refuses to do it.
- **R0P (performance admission)**: per-frame wall duration DISTRIBUTIONS
  recorded beside (never inside) the semantic tape — wall-coupled numbers
  answer distributional questions only (project law: determinism and
  wall-coupling are mutually exclusive observables).
- **Architecture selection**: the decision artifact comparing renderer
  architectures ON the bench's evidence (tape stability + PassDraw deltas +
  R0P distributions). Cannot precede PassDraw.

## Merge-gate ledger for THIS fork (owed before merge to wip-batch-verify)
- [x] W1 vector suite GREEN (13/13, first run, 2026-08-20 02:59).
- [x] common-net sync suite GREEN (2/2, 2026-08-20 03:2x). Harness reference suite: in flight (first attempt died to DISK FULL — 37GB freed, deletion recorded in MAINTENANCE.md).
- [x] `cargo check -p bastion-server -p veloren-server` CLEAN (W2 compiles; 4 first-pass errors fixed: crate-name import, bincode-2 API, two data. leftovers, height_scale field).
- [x] Planted red-demo DONE (2026-08-20 03:5x): WireType::Enum 12→99 → EXACTLY
      the two hierarchy tests failed by name (canonical_schema_hash_and_hierarchy,
      hierarchy_mutation_changes_every_root), 11 non-consumers stayed green →
      restored → 13/13 green again. Harness reference suite also GREEN 2/2
      (pin enforcement + recomputation).
- [x] TWIN SMOKE GREEN (2026-08-20 13:1x): both legs' tapes arrived (~176s/
      ~178s), golden CLI **PASS — run_root identical** on independent boots.
      The stack's determinism witness. Two runner defects found+fixed on the
      way: WSL-bash resolution trap (PowerShell `bash`≠Git Bash) and Git
      Bash `kill` not terminating Windows servers (leg-1 squatting the port
      → AddrInUse; fixed with taskkill + identity-scoped cleanup).
- [ ] R2b container visuals: unit tests + one live look.
