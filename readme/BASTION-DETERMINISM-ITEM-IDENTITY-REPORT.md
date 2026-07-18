# Registry class 7 determinism report

## Measured first divergence

The preserved ARCH-003/class-7 evidence showed byte-identical runs producing
different inventory definitions, `item_hash` values, and `inv_slot` values.
The first gameplay-visible consequence was a different healing `UseItem`
selection; later colonist trajectories then forked.

Source tracing corrected the original identity hypothesis:

- `Item::item_hash` is a deterministic content/definition hash. Different
  hashes meant different items had been generated.
- `Npc::rng(PERM_ENTITY_CONFIG)` was stable, but the lazy profession-loadout
  callback did not receive that or another stable RNG.
- `trader_loadout` created a thread RNG.
- `TradePricing::random_items_impl` used global random draws while indexing a
  candidate list sourced from hash-based storage.

Thus content, insertion order, slot assignment, and the chosen consumable all
varied before trajectory simulation.

## Fix

- Add a distinct stable per-NPC lazy-loadout RNG stream. It is deliberately
  separate from entity/body configuration so added loadout draws cannot shift
  those results.
- Carry that RNG through lazy loadout evaluation and trader sampling.
- Sort trade candidates by the full item-definition identity before sampling.
- Reuse the production healing selector in the focused fixture so the gate
  measures the exact `UseItem` decision, not a proxy.

This does not change the sampling distribution or healing policy. It does not
change `ItemId`, `item_hash`, persisted inventory schemas, global inventory
ordering, or `DETERMINISTIC_RTSIM`/ARCH-003 gating.

## Standing harness

Run the fast closure gate:

```powershell
target\debug\bastion-harness.exe `
  --determinism-regression class7-item-identity `
  --seed 21 `
  --determinism-output E:\evidence\class7-seed21
```

The parent launches two isolated child processes from the same executable and
seed, verifies executable/seed provenance, and compares the complete ordered
`(slot, definition_id, item_hash, amount)` inventory plus selected `UseItem`.
The machine verdict is `verdict.json`; `summary.txt` is the human result.

For flight-recorder integration scenarios, use the same subcommand with
`b55-deep` or `b58-ladder-integration-fixture`. See
`readme/BASTION-DETERMINISM-REGRESSION.md` for evidence validation, exit codes,
normalization policy, and known limits.

## Proof boundary

The class-7 fixture proves the earliest behavioral item-selection seam. It is
not a substitute for a full later colony trajectory or save/reload integration
run. Those remain separate recorder-backed gates. No behavioral field is
normalized.

## Highest-value missing tools

1. A save/reload fuzz corpus across repeated promote/demote cycles.
2. Coverage-guided seed/timing/job-interleaving scenario minimization.
3. Metamorphic conservation and storage-permutation properties.
4. Headless client-render receipts for camera/session provenance.

These are proposals only; they are outside this bounded change.
