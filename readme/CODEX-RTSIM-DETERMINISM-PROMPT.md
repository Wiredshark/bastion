# Codex continuation — fix the RTSim persistence non-determinism you found

Your boot-cache FAIL was correct and stays correct: do not ship the cache until this gate is green.
Authorized by the architect: fix the root cause on your SAME isolated branch/worktree
(`codex/boot-cache`, `E:\bastion-bootcache`). Do NOT touch `bastion/builder` (a separate builder is
live on it). Land this as its OWN commit, separate from the cache prototype commit, so it can be
reviewed/cherry-picked on its own merit later (it's a genuine correctness bug independent of caching).

## Root cause (confirmed against the current tree)
`common/src/bastion.rs:1282` — `pub values: std::collections::HashMap<Value, i8>` on the persisted
colonist struct. Rust's `HashMap` randomizes iteration order per-process by construction — that's why
your evidence showed even two FRESH runs (no restore) producing different RON bytes; the cache exposed
a pre-existing bug, it didn't create one.

The `Value` enum (`common/src/bastion.rs:822`) is documented `/// append-only, never reorder — wire-
and save-stable` — its variant order is ALREADY a fixed contract. So giving it a total order is free:
it doesn't reorder anything, it just formalizes the order that already can't change.

## Fix shape (verify each step yourself — this is a starting hypothesis, not a mandate)
1. Add `PartialOrd, Ord` to `Value`'s derive list (alongside its existing `Copy, Clone, Debug,
   PartialEq, Eq, Hash, Serialize, Deserialize`). Safe: it derives from the existing never-reorder
   variant sequence.
2. Change the persisted field's type from `HashMap<Value, i8>` to `BTreeMap<Value, i8>` (deterministic
   iteration order under Ord). Audit ALL sites touching this exact field/type — at minimum
   `common/src/bastion.rs:1282,1480,1492` and `common/src/comp/bastion.rs:709,759` — check each is
   actually the same persisted field before changing it (don't assume; some may be an unrelated local).
3. `cargo check` across the workspace to confirm the API surface (insert/get/get_mut/entry/iter) still
   compiles at every call site — BTreeMap's API is compatible with the common HashMap operations but
   isn't identical; check `.entry()` usage in particular.
4. Per your evidence's own note: audit (don't necessarily fix yet — flag if found) whether any OTHER
   unordered collection is reachable from persisted `rtsim::Data`. Scope creep risk — if you find more,
   report them rather than silently expanding the fix.

## Mandatory re-gate (same rigor as before — do not weaken it)
Re-run your EXACT x2 gate command (same seed 21, same tool, new evidence dir). Required to pass:
- The two FRESH legs' RTSim RON SHA-256 must now be IDENTICAL to each other (this is the part that was
  broken even without the cache — this is the real proof the bug is fixed).
- fresh vs. restored RTSim RON SHA-256 must be IDENTICAL.
- Trajectory tapes stay byte-identical (already were).
If ANY of those aren't identical, report exactly like last time — do not normalize, do not force a pass.

## Only if the gate goes green: the multi-seed corpus
Your evidence correctly skipped this while the gate was red. Once green, this is a good candidate for
the VM infra (a "many seeds, one scenario" shape): from your worktree,
`BRANCH=codex/boot-cache bash /e/veloren-master/vm-scale.sh e2-standard-32 <N_seeds> <first-seed> "<your gate command>" <max_usd> <max_min>`
runs it on an ephemeral 32-core cloud VM (auto-deletes after; harmless to the M3 builder — separate
branch, separate VMs). Optional — local is fine too if the corpus is small.

## Report back exactly like before
State PASS/FAIL on the re-gate with the same evidence rigor (SHA-256s, exact commands, exit codes). If
FAIL, stop and report — do not expand scope. This still does not authorize a merge to `bastion/builder`;
that's a separate architect decision after M3 lands.
