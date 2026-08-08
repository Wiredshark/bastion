# FARM-PAINT: per-column surface resolution, replacing the literal `plot.min.z` read

Fixes the live-observed defect (this session's own earlier playthrough,
`RUN1-SCORECARD.md`'s farm row): a Farm plot painted 1-3 blocks off real
ground silently produces zero jobs forever, indistinguishable from "the
game is doing nothing because there's nothing to do yet."

## Corrected target, twice, before writing code

Two layers of Opus's own spec were wrong and corrected in sequence before
this landed — recorded here so the final shape is legible against the
false starts, not just the result:

1. **First framing:** "make Farm accept `z_extent` like the kinds that
   already do." **Wrong** — `z_extent` is already kind-agnostic end to
   end (wire, client API, server handler all confirmed unconditional on
   `kind`). Non-fix; the plumbing already accepts it for Farm.
2. **Second framing (mine, verified before building):** `DesignationKind
   ::Farm` maps to `FootprintMode::Area2D` (`common/src/bastion.rs:491`),
   and voxygen's paint handler (`session/mod.rs:1117-1120`) sends `None`
   for `z_extent` whenever `footprint_mode() == Area2D`. **Farm never
   carries a z_extent on the wire, by design** — same shape as CHOP.
   Confirmed by reading `place_chop_fell`: it takes a pre-computed
   `cells: &[Vec3<i32>]` list, calling neither `column_surface_z` nor
   `column_flat_surface_z` itself — the "shared pattern" with CHOP is
   architectural (Area2D, server resolves from pure XY), not a literal
   shared resolver function to copy.

**The actual fix target:** the FARM JOB GENERATOR
(`bastion_jobs.rs`'s `bastion_place_designation` Farm branch and the
farm trigger pass), which collapsed a whole plot to `region.min.z`
regardless of what (if anything) the wire ever carried.

## Design

- New `JobBoard` field `farm_column_z: BTreeMap<(i32,i32), i32>` —
  resolved real ground z per (x,y) column, populated **once at
  registration**, not re-scanned per tick (cheap; avoids the surface
  moving under a growing crop or a nearby dig re-answering the question
  mid-season). Kept separate from `farms: Vec<(ZoneId, Region)>` rather
  than adding a third tuple field, because `farms.iter().chain(board.
  stockpiles.iter())` (two call sites) requires matching tuple arity.
- Resolver: `column_surface_z(terrain, x, y, region.min.z)` — the
  existing **bidirectional** (±96/+48 window) surface search, using the
  painted z as a hint, not a literal. **Not** `column_flat_surface_z`
  (upward-only from a floor, built for Mine's flatten-a-hill-down case):
  Farm's paint-height error can go either direction (this session's own
  live mistake was over-picking, painting ABOVE real ground), and an
  upward-only scan from an already-too-high hint would never find the
  true, lower surface. Verified empirically, not just by reading the
  function signatures — see below.
- Trigger pass: `let gz = plot.min.z;` (shared for the whole plot)
  replaced with a per-column `board.farm_column_z.get(&(x,y))` lookup
  inside the existing x/y loop. A column absent from the map (paint over
  open water / an unloaded chunk at registration time) is silently
  skipped — same treatment the pre-existing `!ground.is_filled()` "no
  field under a hole" arm already gives an unresolved column.

## Verification

New harness flag `--farm-paint-z-offset <i32>` (`farm_scenario`, additive,
absent/0 byte-identical to every prior invocation) offsets the plot paint
z before placement while the real terrain (the flush plateau fixture)
stays exactly where it was — planting the live mistake directly rather
than guessing at a synthetic repro.

| case | offset | result | `FARM-CERTIFICATE` hash |
|---|---|---|---|
| baseline (correct height) | 0 | PASS | `[144,114,4,20,...]` |
| over-picked (painted above ground) | +2 | PASS | **identical** to baseline |
| under-picked (painted below ground) | -2 | PASS | **identical** to baseline |

All three runs produce **byte-identical** `durable_composite` hashes —
not just "still passes," but **no player-visible seam**: a plot painted
2 blocks off resolves to the exact same tilled/sown/matured/harvested
outcome as one painted exactly right. `farm_tilled`/`farm_sown`/
`farm_matured`/`farm_harvested`/`farm_cycled` all `true` in every case.

This result also settles the resolver choice empirically: had
`column_flat_surface_z` been used instead, the +2 (over-picked) case
would be expected to fail to resolve (upward-only scan from an
already-too-high hint), based on a direct read of that function's own
window (`floor_z..=floor_z+128`, no downward half). Not run head-to-head
against the alternative — the bidirectional choice was made and verified
directly, not comparatively.

## Scope not covered in this pass

**Refusal-with-message** (Fable's other stated half: "if resolution
genuinely can't find a valid surface for some or all of the painted
footprint, say so, never just silently generate zero jobs" — Opus's §3,
"if ZERO columns resolve, REFUSE with a message; if SOME resolve,
register those and report the count") is **not implemented here**. The
registration log line (`resolved`/`unresolved` counts) exists for
diagnostics, but nothing surfaces to the player yet — `bastion_place_
designation` doesn't have `chat_emitter` in scope at paint time (same gap
`BlockedRegionInfo`'s own doc names: "deferred to the next arbitration
cycle, which has `chat_emitter` in scope"). Flagging explicitly rather
than silently dropping half the spec: the core correctness fix (silent
zero-jobs) is closed; the message-surfacing half is a natural, scoped
follow-up, not forgotten.
