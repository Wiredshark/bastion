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
