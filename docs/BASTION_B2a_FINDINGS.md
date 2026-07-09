# B2a findings — overseer interaction surface

Block spec: design doc v2.1 §B2a (no dedicated prompt file exists in-tree; the
doc entry is the working spec). Explored 2026-07-08 on `bastion/block-B2a`
(start `f456b08`).

## 1. Verified seams (all as handed over by B1.5/B1.6)

- `voxygen/src/bastion/input.rs` — `OVERSEER_SCHEME.suppressed` currently
  contains `Primary`, `Secondary`, `Interact` (among the avatar verbs). B2a
  reclaims **Primary/Secondary** into `owned`.
  **Deviation (conscious):** `Interact` stays suppressed — its physical key
  (`E`) is owned by `BastionRotateRight` in the overseer scheme, and
  per-context key *overrides* don't exist until B9. Un-suppressing it would
  fan `E` out to both rotate and interact — the exact bug class the context
  system exists to kill. The third "reclaimed slot" is instead covered by two
  new bastion GameInputs: `BastionCycleTool` (T) and `BastionToggleGodMode`
  (G).
- `bastion::unproject_to_world_plane` (`voxygen/src/bastion/mod.rs`) — ground/
  slice picking, ortho-exact. Reused for tile-picking and designate-paint.
- `Hud::bastion_cursor_over_widget` (`hud/mod.rs:5533`) — the HUD gate; the
  session's raw-mouse overseer arms already consult it before grabbing.
- Raw-mouse routing (`session/mod.rs` ~1617): overseer intercepts
  `winit::event::MouseButton` Left (grab-drag) / Right (orbit) directly.
  Click-vs-drag disambiguation does NOT exist yet — B2a adds it (cursor
  displacement threshold between press and release; small = click).

## 2. UI tech decision: conrod HUD, not egui

`egui` rendering is hard-gated on `settings.interface.toggle_egui_debug`
(`run.rs:44`, `interface.rs:121`) — a debug toggle the player may never press.
Gameplay UI cannot live there. The radial menu + tool palette therefore render
in the **conrod HUD** (`voxygen/src/hud/`), which always renders in session and
is where B9's colony HUD lands anyway. New module `hud/bastion_radial.rs`,
its own `widget_ids!` block, events surfaced through the existing
`hud::Event` enum (consumed by the session like every other HUD event).

## 3. Designation overlay: scene debug shapes

`voxygen/src/scene/debug.rs` — `Debug::add_shape(DebugShape) -> DebugShapeId`,
`set_context(id, pos, color, ori)`, `remove_shape(id)`; `DebugShape::Line` et
al. Echoed designations render as line-rectangle outlines at the region top —
plumbing-grade visuals (B4 gives designations real render treatment). The
debug pipeline renders independently of the egui/debug toggles (used for
hitboxes via a setting, but shapes we add draw unconditionally).

## 4. Message plumbing (mirrors the B1.6 BastionCameraAnchor precedent)

- New `common/src/bastion.rs`: `DesignationKind`, `InfluenceKind`,
  `ContextVerb`, `Region` (serde-ready per B10 ground rule).
- `ClientGeneral::{BastionPlaceDesignation, BastionApplyInfluence,
  BastionContextAction}` — in-game stream; added to `verify()` and the client
  stream-selection match.
- Server `sys/msg/in_game.rs`: validate (finite coords, region volume cap,
  kind known) then **echo**: designations back as a structured
  `ServerGeneral::BastionDesignation { region, kind }` (client stores + draws
  overlay); context-actions/influence as `ChatType::CommandInfo` chat lines
  (visible, testable, zero new render).
- Client (`client/src/lib.rs`): store echoed designations in
  `bastion_designations: Vec<(Region, DesignationKind)>` + accessor; voxygen
  session syncs them into `scene.debug` shapes.

## 5. Selection

- `common/src/comp/bastion.rs` → `BastionSelected` marker (NullStorage,
  serde-unit), registered alongside other comps in `common/state`. Inserted/
  removed client-side on click; scene reads the storage each maintain to feed
  **real** B1.6 cutaway targets (replaces the B1.6 focus+debug-marker stubs).
- Entity picking: `bastion::cursor_ray` (same inverse-matrix path as
  `unproject_to_world_plane`, returning origin+dir), nearest entity whose
  `Pos` lies within `body.max_radius()+0.5` of the ray, within 300 blocks.

## 6. God/Free + tools state

`voxygen/src/bastion/tools.rs`: `ToolMode { Pan, Inspect, Designate(kind) }`,
`GodMode { God, Free }`, `fn target_allowed(...) -> bool` — the restriction
hook, stubbed permissive with the B2b enforcement point documented. Lives on
the session; palette + radial + keys mutate it.

## 7. Interact-slot note for B9

When B9 builds per-context binding overrides, move overseer rotation off Q/E
(or rebind Interact per-context) and reclaim `Interact` properly.
