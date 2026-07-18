# Bastion determinism regression gate

`bastion-harness --determinism-regression` is the standing process-isolated
determinism gate. It launches the exact current harness executable twice,
records artifact and input provenance, and reports the earliest observed
divergence.

## Usage

```powershell
# Fast registry-class-7 inventory/UseItem proof
target\debug\bastion-harness.exe `
  --determinism-regression class7-item-identity `
  --seed 21 `
  --determinism-output E:\evidence\class7-seed21

# Existing flight-recorder scenarios
target\debug\bastion-harness.exe `
  --determinism-regression b55-deep `
  --seed 21 `
  --determinism-output E:\evidence\b55-seed21

target\debug\bastion-harness.exe `
  --determinism-regression b58-ladder-integration-fixture `
  --ladder-episode P0G `
  --seed 21 `
  --determinism-output E:\evidence\b58-p0g-seed21
```

The parent creates `run-a` and `run-b`, never overwrites an existing output
directory, hashes its executable before and after both children, and checks
the recorder/observation artifact and seed metadata. B5.8 uses the fixture's
existing delayed `M2_RECORDER_DIR` carrier. Recorder sampling is lossless
(`sample_every=1`) within explicit bounds; any timeout, missing finalization,
missing stream, truncation, signal-style exit, metadata mismatch, or changing
artifact makes the verdict invalid.

The current B5.5 and B5.8 scenario implementations own internal temporary data
directories. `--determinism-save-tree` is therefore rejected for these named
mappings; it is not silently advertised as an input that the child ignores.

## Outputs and exit status

- `verdict.json`: machine-readable `bastion.determinism-regression.verdict/v1`.
- `summary.txt`: concise human result and exact first divergent values.
- Per child: command, numeric exit, stdout/stderr, recorder or authoritative
  observation, hashes, counts, and truncation/provenance checks.

Exit codes are:

- `0`: deterministic and both functional scenario assertions passed.
- `1`: a behavioral/tape/outcome divergence was measured.
- `2`: invalid evidence or infrastructure failure.
- `3`: deterministic evidence, but both children reported a structured
  functional scenario failure. This is never presented as an overall green.

Complete tapes from an explicit structured scenario `FAIL` remain comparable,
which is required to inspect a pre-fix failure. A panic is not comparable: a
nonzero child is accepted only when its exact scenario `...: FAIL` marker and
complete evidence are present.

Trajectory and writer-event streams have no shared total ordering. If both
first diverge at the same tick, the verdict reports all same-tick candidates
and sets `cross_stream_order_proven=false`. A writer is named as observed only
when both records identify the same writer and both carry proven dispatcher
ordering; this is evidence, not automatic causal attribution.

## Normalization policy

No normalization is enabled by default. The sole allow-listed normalization is:

```text
--determinism-normalize wall-unix-millis
```

It applies only to the exact top-level `$.wall_unix_millis` field. Position,
velocity, state, item definition, item hash, inventory slot, amount, target,
writer, and all nested fields are behavioral and cannot be normalized.

## Registry class 7

`class7-item-identity` uses authoritative state rather than a long trajectory:
the production lazy farmer loadout, `SpawnEntityData` inventory construction,
and the exact production healing-item selector. Each child emits the complete
ordered `(slot, definition_id, item_hash, amount)` inventory and selected
UseItem slot/hash. Task 1 compares that record byte-semantically across fresh
processes.

The measured root was not an unstable `Item::item_hash`: that hash is derived
from item definition/content. RTSim created a stable per-NPC RNG for entity
configuration, but lazy profession loadouts discarded it; `trader_loadout`
used `rand::rng()`, while `TradePricing` used `rand::random()` and sampled an
unordered candidate vector by index. The fix preserves laziness and gameplay
distribution while giving it a separate stable per-NPC RNG stream and sorting
candidates by full item-definition identity before sampling. Item hashes,
ItemIds, persistence, global inventory ordering, and ARCH-003 deterministic-mode
gating are unchanged.

The fast class-7 mapping closes the earliest measured behavioral split: both
processes must construct the same ordered inventory and issue the same concrete
`UseItem` slot/hash. It intentionally does not claim that a later full colony
trajectory has been exercised. Use `b55-deep` or another recorder-backed named
mapping when a full integration trajectory is required; that longer run is a
separate gate, not something normalized or inferred by this fixture.

## Follow-up test-tool proposals

Highest-value next additions, kept out of this bounded change:

1. Save/reload fuzz corpus that feeds identical snapshots through repeated
   promote/demote/save cycles and compares the same determinism verdict.
2. Coverage-guided scenario mutation over seed, timing, and job interleavings,
   minimizing any first-divergence tape into a small fixture.
3. Metamorphic invariants for conservation, permutation-independent candidate
   storage, and observer/non-observer equivalence.
4. Headless client-render receipt checks for camera/session provenance, to
   replace human-eye-only rendering gates without claiming pixel equivalence.
