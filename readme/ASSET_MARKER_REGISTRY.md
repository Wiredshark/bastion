# Bastion Asset Marker Registry (append-only) — the ONE authority for custom gameplay-marker bytes

**Why this exists:** B-ASSET1's integration sweep FAILED 8 assets because the asset pipeline allocated
gameplay-marker bytes **ad-hoc, undeclared** (bytes 201/202/210–213/217 from new content) plus a real
collision (a quarry hall used byte **8** for dwarven carve-air, but byte 8's DEFAULT maps to `Fruit`). Every
static check passed them; only engine integration caught it. Ad-hoc markers = broken integration. This
registry is the single place custom marker bytes are allocated, so generated content never collides with the
world defaults, the engine defaults, or each other.

**THE RULE (pilot + any asset agent MUST follow):**
1. **Use STANDARD bytes for standard meanings — never a custom byte for something a default already covers.**
   Carve-air = **byte 16 (`Hollow`)**, not a custom byte. Leaves/water/grass/chest/etc. use their reserved
   default bytes (see the table in `readme/ASSET_GAMEPLAY_MARKERS.md §1`). Do NOT repurpose a default byte
   (that's the byte-8→Fruit bug).
2. **Custom gameplay markers live ONLY in the 200–255 band, and every one MUST be declared here BEFORE use**,
   with the exact `StructureBlock` it maps to (`Filled(kind,color)` / `Sprite(..)` / `Keyhole*` / `EntitySpawner(..)`
   / etc.) — and the asset's own RON `custom_indices` must set it to that same target.
3. **No two custom bytes mean different things across assets** — one byte, one meaning, registry-wide. Check
   this table before allocating; reuse an existing declared byte if the meaning matches; append a new row if not.
4. **The function/marker harness gates against THIS table** — an asset using an undeclared or colliding byte
   FAILS. (B-ASSET1's marker-fidelity assert is the engine-side enforcement; the pilot's `function_check`
   byte-resolution check is the content-side one.)

## Reserved bands (from ASSET_GAMEPLAY_MARKERS.md — do not violate)
- **1–16** — world semantics (leaves/water/grass/`MaybeChest`/`Hollow`/etc.). Fixed defaults; overrides per
  asset RON only, never as a *literal color*.
- **32–199** — literal palette colors (`Filled(Misc, color)`). Not gameplay markers.
- **200–255** — **custom gameplay markers. Declared here.**

## Custom marker allocation table (200–255) — APPEND-ONLY
| byte | meaning | StructureBlock target | first used by | notes |
|---|---|---|---|---|
| _(seed the currently-undeclared bytes below — the pilot must BACKFILL what each was intended to mean, then set each asset's RON custom_indices to match, then re-verify)_ |
| 200 | _reserved — declare_ | ? | ? | pilot: fill or free |
| 201 | **UNDECLARED (B-ASSET1 fail)** | ? | new asset-session content | pilot: declare the intended meaning or remap to a declared byte |
| 202 | **UNDECLARED (B-ASSET1 fail)** | ? | new asset-session content | " |
| 210–213 | **UNDECLARED (B-ASSET1 fail)** | ? | new asset-session content | " |
| 217 | **UNDECLARED (B-ASSET1 fail)** | ? (gnarling totem used 217=`Filled(GlowingRock,…)` per ASSET_GAMEPLAY_MARKERS) | new asset-session content | pilot: confirm it matches the totem convention or remap |

## Known FIX (action for the pilot, from B-ASSET1 §9)
- **Quarry hall byte 8 → carve-air collision:** change carve-air cells from byte 8 to **byte 16 (`Hollow`)**
  (or add an explicit RON `custom_indices: {8: Filled(Air,…)}` override) — byte 8's default is `Fruit`.
  Regenerate + re-verify. Log the fix in `ASSET_REJECTION_LOG.md`.

*Source of truth for the default byte table: `readme/ASSET_GAMEPLAY_MARKERS.md`. This registry governs only
the CUSTOM 200–255 band + the "use standard bytes for standard meanings" rule. Append every new custom marker
here before shipping the asset that uses it.*

## BACKFILL 2026-07-09 (pilot) — every custom byte in use, declared

Content-side mirror: `asset-lab/gen/markers.py` (staging FAILS on any custom-band
byte absent from that table; generators import it to place markers). RON
`custom_indices` now emitted per staged asset: `asset-lab/vox/real/<id>.ron`.
Targets verified against `common/src/terrain/sprite/mod.rs` + `structure.rs`.
**Figure-layer exemption:** creature parts / armor / held items never pass through
Structure custom_indices — their palettes are colors + figure material bands
(9-13 shiny, 14-16 glow); bytes >=200 there are literal colors, not markers.

| byte | meaning | StructureBlock target | first used by | notes |
|---|---|---|---|---|
| 200 | welded/closed gate leaf (operable closed state) | `DoorBars(())` | palisade gate; defense kit C2 gates; castle portcullis | operable state machine = DF-MECH; engine loads closed state |
| 201 | smithy anvil work point | `Sprite(Anvil())` | structure_production_smithy (+ castle/monastery wraps) | |
| 202 | smithy forge/hearth work point | `Sprite(Forge())` | structure_production_smithy (+ wraps) | |
| 203 | — free (never shipped) | — | — | composer default passable list only; unallocated |
| 204 | brazier / fire light point | `Sprite(FireBowlGround())` | c_brazier (castle, monastery) | |
| 205 | — free | — | — | |
| 206 | trap/mechanism trigger plate | `Filled(Rock, (r:90,g:90,b:70))` | c_pit_housing, c_trap_housing, monastery | inert plate until DF-MECH trigger wiring |
| 207 | desk / study point | `Sprite(DiningtableWoodWoodlandSquare())` | c_marker_desk (monastery) | |
| 208 | bench / sit point | `Sprite(BenchWoodEnd())` | c_marker_bench (monastery) | |
| 209 | bed / sleep point | `Sprite(Bedroll())` | c_marker_bed (monastery) | single-cell Bedroll = the camps' own convention |
| 210 | carpenter work point | `Sprite(CraftingBench())` | workshop_carpenter | DF-WORKSHOP differentiates trade by zone, not sprite |
| 211 | mason work point | `Sprite(CraftingBench())` | workshop_mason | same note as 210 |
| 212 | smelter furnace work point | `Sprite(Forge())` | workshop_smelter | distinct meaning from 202 (zone trade), same target |
| 213 | tannery work point | `Sprite(TanningRack())` | workshop_tannery | |
| 214 | tripwire trigger line | `Filled(Rock, (r:60,g:60,b:50))` | c_tripwire | inert until DF-MECH |
| 215 | cooking/kitchen work point | `Sprite(CookingPot())` | workshop_kitchen (this session) | |
| 216 | loom/weaving work point | `Sprite(Loom())` | workshop_loomhouse (this session) | figure-layer 216-as-color exempt (header) |
| 217 | glow crystal (dungeon accent) | `Filled(GlowingRock, (r:0,g:201,b:177))` | terracotta set, quarry hall | matches gnarling-totem convention — CONFIRMED |
| 218 | worship point (altar-facing congregation spot) | `Filled(Rock, (r:200,g:190,b:160))` | shrine/temple (this session) | no altar sprite in vanilla; REL-0 upgrades the target |

**Native-adopted (literal band, meaning fixed by the native RON we mirror):**
byte 136 = lantern light point (`Sprite(Lantern())`, dwarves/entrance.ron convention) —
used by structure_dungeon_quarry_hall; its emitted RON mirrors the native mapping.

**Byte-8 fix executed:** quarry hall carve-air remapped 8 -> 16 (Hollow), regenerated,
style+function re-PASS; byte 8 remains ONLY where it means Fruit (rowan berries —
standard meaning, correct). Details in ASSET_REJECTION_LOG.md.

## Allocation 2026-07-09 (pilot, DF-TRADE board item)
| byte | meaning | StructureBlock target | first used by | notes |
|---|---|---|---|---|
| 219 | trade depot drop/work point (surplus pool + caravan unload) | `Sprite(CraftingBench())` | structure_trade_depot (this session) | TRADE-1 differentiates by zone kind; bench = neutral work surface |

## Allocation 2026-07-09 (pilot, naval directive)
| byte | meaning | StructureBlock target | first used by | notes |
|---|---|---|---|---|
| 223 | beacon light (lighthouse / warning fire) | `Filled(GlowingRock, (r:255,g:157,b:46))` | structure_lighthouse | warm glow |
| 224 | mooring / dock work point | `Sprite(CraftingBench())` | pier set + harbor crane | neutral until naval-movement verbs land |

**Vehicle namespace note:** ship-manifest RONs map LOW bytes 1-15 per vehicle to
`Air(Sprite...)` (see assets/common/manifests/ship_manifest.ron) — a per-vehicle
namespace, NOT the world band. Lab vessels mirror the airship convention
(1 = seat, 2 = helm, 6 = lantern) and are exempt from the 200-255 rule.

## CORRECTION 2026-07-09 (from B-ASSET1 graduation sweep, 61/64)
Byte 200 target was written `DoorBars(())` — **invalid**: `DoorBars` is a
`SpriteKind`, not a `StructureBlock` variant. Correct form: **`Sprite(DoorBars())`**
(the byte-200 row above reads with this correction). markers.py fixed, the 3
affected sidecars (defense_palisade_line_demo / gate_brick_line /
gate_dwarven_line) regenerated. Gate DYNAMICS had already verified via the
tester's registry-mirror fallback; this closes the parse-fidelity finding.

## GLOW BAND CONVENTION (pinned 2026-07-10, tester hazard closure)
- **Bytes 14-15 = the Bastion GLOW band.** custom_indices REQUIRED (markers.ron_custom_indices emits
  `Filled(GlowingRock, color)` — emissive). NO engine-default reliance, NO structural/walls-scale usage.
- **Byte 16 = `Hollow` (engine-reserved carve-air), a STANDARD byte** — OMITTED from custom_indices,
  resolved engine-side. NEVER glow. (Was mis-grouped into a 14-16 glow rule; a generic rule would have
  filled structure interiors' Hollow void with glowing rock — e.g. quarry_hall's 2226-cell interior.)
- Bytes 9-13 = "shiny" figure bands (material sheen, not emissive). Bytes 8/11 = Fruit/MaybeChest standards.
- ENFORCEMENT: markers.ron_custom_indices checks STANDARD first (omit), then emits 14-15 as GlowingRock.
  Any structural material must live in the 32-199 literal band, never 14-16.
