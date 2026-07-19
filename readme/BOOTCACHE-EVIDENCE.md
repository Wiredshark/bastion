# Boot-cache prototype evidence and roadblock

Status: **FAIL / DO NOT SHIP**. This packet preserves the requested design,
opt-in prototype, and x2 evidence for architect review. It does not authorize a
merge into `bastion/builder`.

## Provenance and isolation

- worktree: `E:\bastion-bootcache`
- branch: `codex/boot-cache`
- anchor: `96bbf1d2bb9d38857b97a0a28d687952e1707ed2`
- separate target: `E:\bastion-bootcache-target`
- diagnostic artifact:
  `E:\bastion-bootcache-target\verify\bastion-harness.exe`
- diagnostic artifact SHA-256:
  `481BCCD8913090E675725663C7E39FF514E3372B334692B45B8D5B2419F7CD36`
- artifact size: `338233856` bytes
- artifact mtime UTC: `2026-07-19T23:35:04.3915688Z`
- protected traversal/path/physics files were not changed.

The executable identifies the dirty source anchor as `96bbf1d2bb+dirty`.
Artifact identity is therefore the executable SHA-256 plus this branch's
scoped diff, not the cached build timestamp.

## Focused compile and fail-closed tests

```text
CARGO_TARGET_DIR=E:\bastion-bootcache-target \
  cargo check -p bastion-harness --message-format short
```

Result: exit `0` (pre-existing anchor warnings only).

```text
CARGO_TARGET_DIR=E:\bastion-bootcache-target \
  cargo test -p veloren-server bastion_boot_cache::tests --lib -- --nocapture
```

Result: exit `0`, `3 passed; 0 failed`. The tests cover invalid executable
digest, default-disabled behavior, parallel/calendar refusal, seed-key
separation, and pre-existing RTSim persistence refusal.

```text
CARGO_TARGET_DIR=E:\bastion-bootcache-target \
  cargo build --profile verify -p bastion-harness --message-format short
```

Result: exit `0`.

## Mandatory x2 gate

Exact diagnostic command (one process, new evidence directory):

```text
E:\bastion-bootcache-target\verify\bastion-harness.exe \
  --boot-cache-proof \
  E:\bastion-bootcache-evidence\diagnostic-rtsim-seed21-x2-20260719\proof \
  --boot-cache-proof-seeds 21 --boot-cache-proof-ticks 30
```

Result: child exit `1`, wall `72.531s`, `deterministic=false`,
`gate_pass=false`.

Both x2 pairs correctly proved `fresh` then `restored`, used the same exact
cache key, produced 31 nonempty samples, and kept the pristine cached
block/resource fingerprint unchanged across a deliberate post-capture terrain
mutation. The restored boot was materially faster:

| pair | fresh boot | restored boot | speedup | trajectory |
|---|---:|---:|---:|---|
| seed-21-x1 | 33019 ms | 500 ms | 66.04x | byte-identical |
| seed-21-x2 | 33812 ms | 564 ms | 59.95x | byte-identical |

All four trajectory files have the same SHA-256:
`F0E69EAF0502DCE87F5CA31C47129E7EF04280B0DBEF996E7368B4BC93239443`.
Only `wall_unix_millis` is an allowed normalization; the dedicated proof tape
contains no wall field.

The authoritative RTSim persistence bytes failed:

| leg | RTSim RON SHA-256 |
|---|---|
| x1 fresh | `32D682D8FD4723A284607B2D5AC33CF98D8B1B45AEE06BEE8B53D4BF668DD507` |
| x1 restored | `532CCAB1ABBF8285301AF4EF15419540292CA6E4A2B497928FC0AE21D3C93C70` |
| x2 fresh | `51B0636FB40C35ABFCAB1D725730E8FB676C045D9C6F0E1B4D091B427D962880` |
| x2 restored | `B036A0313B7E1A51EC3CBCB0E7C7F14E92717E15C28626E203F8AF022F6DE2D2` |

The two fresh hashes also differ, proving that raw RTSim persistence ordering
is not currently byte-stable even without restore.

## First measured split

In x1 the RON files have equal length (`7468904` bytes). Their first differing
character is offset `7426152`, inside a persisted Bastion colonist's
`values: HashMap<Value, i8>`:

```text
fresh:    values:{Tradition:5,Freedom:-4,Wealth:-50,Kin:-2,Piety:-4,Craft:9,Glory:15,Nature:35}
restored: values:{Kin:-2,Glory:15,Tradition:5,Craft:9,Piety:-4,Freedom:-4,Wealth:-50,Nature:35}
```

The key/value content shown is equal, but the raw byte order is not. The cache
prototype clones `rtsim::Data`; standard `HashMap` cloning/reconstruction does
not provide the byte-order invariant required by this task. Accepting a
content-sorted representation here would add a normalization beyond the
permitted wall timestamp and would not prove that later iteration order cannot
affect behavior.

## Raw evidence

- diagnostic root:
  `E:\bastion-bootcache-evidence\diagnostic-rtsim-seed21-x2-20260719`
- diagnostic `verdict.json` SHA-256:
  `4211DBAAE934317E9ED4ED075F0D343CD5A4FA6C0F603A9833449E42EE01B6D9`
- diagnostic `stdout.log` SHA-256:
  `D5BAF4F6F3DAD93D55614217B9AF3A5112962186AE5C48B0690BF4E26519E844`
- diagnostic `stderr.log` SHA-256:
  `57F17075FCB0055EF4C46A0E1447482297ACE9C150564B3A17DA5FD9D1AECCCC`
- initial rejected root:
  `E:\bastion-bootcache-evidence\smoke-seed21-x2-20260719`
- initial `verdict.json` SHA-256:
  `790DF5D09CB8412F4937C58F0B5CE1B0720962C0DFBD1068503D6ACEA636E320`

The diagnostic root also contains both complete RON files, outcomes, and
trajectory tapes for each pair. Evidence directories were created once and
were not overwritten.

## Decision and next required scope

The multi-seed corpus was intentionally not run because the mandatory primary
x2 gate was already red. This prototype **does not ship**.

The architect must decide whether to authorize a separate RTSim persistence
determinism task: replace colonist persistence maps (and any other unordered
state reachable from `rtsim::Data`) with stable-order storage or a
source-reviewed deterministic hasher/iteration contract, then rerun this exact
x2 gate before any corpus. That is a broader state-model change and was not
silently folded into the boot-cache task.

Rollback is the isolated cache module, the harness flag/proof module, the
narrow `RtSim` constructor split, `Lod: Clone`, and the two server hashing
dependencies. No normal game/server path enables the prototype by default.
