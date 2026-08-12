# FOUNDING PRESET v1 — BUILD HANDOFF (Opus 5c → next builder)

**Date:** 2026-08-12. **Spec:** `ITEM-FOUNDING-PRESET-PACKET.md` (§1-§7
Fable, §8 review amendments Opus 5c — §8 is binding, it corrects §1).
**Code branch:** `bastion/wip-batch-verify`, worktree
`E:\veloren-master\.engine-integration-wt`. **Docs branch:**
`bastion/block-B6HAUL`, checkout `E:\veloren-master`.

**State in one line: the feature is BUILT, unit-tested and
red-demonstrated; it has NEVER RUN LIVE — not once.**

---

## §1 WHAT LANDED (three commits, all pushed)

| commit | what |
|---|---|
| `45a8613c72` | `bastion-server/src/bastion_founding_preset.rs` (new) + `lib.rs` registration. The plot template + refusal types. 7 tests. |
| `e1ac693b71` | Live wiring: `server/src/sys/msg/in_game.rs`, `RtSim::bastion_colony_exists` in `server/src/rtsim/mod.rs`, re-export in `server/src/lib.rs`. |
| `6c2991eb47` | Resourced arena in `bastion-server/src/bastion_flat_arena.rs` + manifest entry in `common/src/host_input_manifest.rs`. 5 tests. |

`cargo test -p bastion-server --lib` = **108/108**. `cargo build -p
veloren-server-cli` and `cargo build -p veloren-client --bin
bastion_playtest` both clean (dev profile).

### The symbols you will need

- `bastion_founding_preset::FOUNDING_PRESET_V1` — the plot template:
  three `PresetElement { role, kind, min_off, max_off }` rows
  (Stockpile / Farm / Bed). **Offsets are the template's data — do not
  compute positions at a call site.**
- `resolve_datum(terrain, origin_xy, hint_z) -> Option<i32>` —
  `column_surface_z(..) + 1`, the FIRST AIR CELL. `hint_z` only centres
  the resolver window. This is §8 B1; the test
  `datum_is_derived_from_terrain_not_from_the_reported_z` kills the
  "just use `pos.z`" regression.
- `validate_site(terrain, origin) -> Result<(), (FoundingRefusal, Vec2<i32>)>`
  — every plot column within ±1 of the datum, via `column_surface_z`
  (reused, per §8 N6 — there is no second standability rule).
- `FoundingRefusal::{ColonyExists, Terrain}`, `.reason()` →
  `"colony_exists"` / `"terrain"`, `.player_message()`.
- `preset_is_complete(&[PresetRole])` — A1's full-vs-partial
  discriminator.
- `RtSim::bastion_colony_exists()` — the one-colony predicate. Reads
  **rtsim colonist records** (§8 B6: the only colony state that survives
  a restart). Declared SNAPSHOT.
- `bastion_flat_arena::{resourced, resourced_feature_cells,
  apply_resourced_features}` — the resourced variant, written **at chunk
  generation** (§8 N7).

### The emits (this is what every bar reads — name-the-line law)

```
bastion: colony founded          preset=v1 pos=.. datum=.. colonists=..
                                 elements=stockpile,farm,bed complete=true
                                 jobs=.. designated_regions=..
bastion: founding refused        reason=colony_exists|terrain pos=.. column=..
bastion: founding preset plot placed  role=.. kind=.. region=.. jobs=..
```

`elements=` / `complete=` exist specifically so A1's planted failure (a
PARTIAL preset) is visible in the witness rather than inferred.

### Behaviour, in order (a refused founding mutates NOTHING)

1. BOUNDARY: `rtsim.bastion_colony_exists()` → refuse `colony_exists`.
2. SITE: datum resolve + `validate_site` → refuse `terrain`.
3. Place the three plots via `job_board.place_designation` (the same
   placement authority the painted path uses).
4. `rtsim.bastion_spawn_colony` → seed drop (`FOUNDING_SEED_STOCK`, 8
   seeds, **no food** — §8 B3) → `CreateColonyPresenceEvent` (this is
   the "promote" half) → the witness emit.

---

## §2 WHAT HAS NOT HAPPENED (the honest half)

- **NO LIVE RUN.** The smoke test was set up and not started. A1/A2/A3/
  A5 have never been observed; the only evidence is unit-tier.
- **A3 (till → sow → eat) is entirely unexercised.** Note the standing
  risk from §8 B3: with seeds only, the first EAT waits on a harvest —
  so "minutes-scale" in packet §5 is unproven for A3 specifically.
- **The scored acceptance has not been pre-registered.** Packet §5
  requires the scorer's own "what I will not do" written BEFORE the
  data, and §8 B7 requires binary provenance for BOTH binaries
  (server-cli AND voxygen, per the `64ad49dc1e` lesson) — neither
  exists yet.
- **§8 N2 is undecided:** "via the ACTUAL UI" is two tiers. The driver's
  `spawn` reaches the same live `ClientGeneral::BastionSpawnColony`
  handler the in-game action does; the mouse/widget path is a separate
  witness. Pick per-acceptance before running, don't conflate.
- **No voxygen change was made.** The founding action's client side is
  untouched.

### Reported, not fixed (a row, not a park)

`host_input_manifest`'s completeness test
(`every_live_env_read_is_classified`) is **RED on the branch** for FOUR
pre-existing unregistered env sites — `BASTION_ENTITY_EVENT_LOG`,
`BASTION_ENTITY_EVENT_LOG_RING_SIZE`,
`BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG`, `BASTION_FINGERPRINT`.
Confirmed pre-existing by stash-and-rerun; none is mine, and
`6c2991eb47` reduces the set by one (its own). Each needs its class
(Diagnostic vs GameplayVariant) read from its own site — that is a row,
not a drive-by, because a misclassification silently corrupts a run's
attestation.

---

## §3 HOW TO RUN THE SMOKE TEST (the next concrete step)

Nothing here is certified — this is "does the path work at all".

1. **Check first: no `veloren-server-cli` / `veloren-voxygen` running
   (Ben may be in-game), and no other cargo running.**
2. Server, with a THROWAWAY userdata dir (never Ben's celebration
   world). Userdata location comes from the `VELOREN_USERDATA` env var
   (`common/base/src/userdata_dir.rs`):

```bash
VELOREN_USERDATA=E:/veloren-master/.engine-integration-wt/userdata-preset-smoke BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 ./target/debug/veloren-server-cli
```

3. Driver: `bastion_playtest <server> <username> <script> [log]`. Script
   verbs: `anchor`, `spawn <n>`, `designate <kind> x0 y0 z0 x1 y1 z1`,
   `wait <ticks>`, `list_designations`, `note <text>`.

```
note SMOKE: first founding must place the full kit
anchor
spawn 8
wait 300
list_designations
note SMOKE: second founding must REFUSE reason=colony_exists
spawn 8
wait 150
```

4. **What to read in the SERVER log** (not the driver log):
   - `bastion: colony founded ... complete=true elements=stockpile,farm,bed`
   - three `founding preset plot placed` lines
   - `bastion: farm plot registered, per-column surface resolved` with
     `unresolved=0`
   - the second spawn → `bastion: founding refused reason=colony_exists`

**The arena exercises §8 B1 for free:** the driver founds at the player's
position, which on the arena is z=**401**, while the datum is z=**400**
(`FLAT_ARENA_Z` is the first air cell; spawn is +1 for landing jitter).
If the plots come out one block low, the datum derivation broke.

### Things to watch that are NOT bugs

- **Stockpile generates zero jobs** — it registers a zone (B6 haul
  destination), by design.
- **Bed jobs need `BUILD_MATERIAL_ITEM`** (stone). The founding stock is
  seeds only, so the bed stays unbuilt until something is mined. Expected;
  worth an explicit observation rather than a surprise.
- The preset path calls `place_designation` directly and so bypasses the
  painted path's `MAX_DESIGNATION_VOLUME` gate. Volumes are ~60 blocks;
  noted, not a hazard.

---

## §4 IF YOU CHANGE THE PRESET

`preset_reproduces_script_15_absolute_numbers` reproduces all twelve of
script-15's absolute designation numbers from the relative table, against
the anchor `F = (15216.5, 16016.5, 419.0)` verified across six driver
logs. **If that test fails, the change is wrong, not the test.**

Red demonstration is required before any commit here. The eight
mutations already proven to fire (four preset, four arena) are listed in
the commit messages of `45a8613c72` and `6c2991eb47` — reuse them as the
regression set. One of them (`B`: the Chonk write given a local z) did
NOT fire on first attempt because the test had re-implemented the write
loop instead of calling it; the write is now extracted as
`apply_resourced_features` and the test drives the production path. That
is the F8 lesson applied in-flight — **if you add a test here, make it
call the code, not describe it.**
