# B2a self-test results — overseer interaction surface

Run: 2026-07-08/09, branch `bastion/block-B2a` (`db41b4c..e7e9801`), gate per
design doc §B2a Done-when + standing invariants. Result: **PASS**.

## Compiles

- `cargo check -p veloren-voxygen -p veloren-server-cli` — green.
- `cargo build -p veloren-voxygen` — green.
- NOTE: `cargo check -p veloren-client` *standalone* fails pre-existing
  (feature unification: `State::client`'s `plugins`-gated arg) — this is why
  B0's `bastion-check` omits the client crate; unchanged by this block.

## Headless harness (server-side invariants)

`cargo run -p bastion-harness -- --seed 1337 --ticks 1000`:

```json
{"seed":1337,"tick_count":1000,"rtsim_tick":1000,"rtsim_npc_count":2355,
 "rtsim_site_count":204,"rtsim_faction_count":16,"rtsim_report_count":0,
 "loaded_entity_count":0,"sim_time":33.333,"time_of_day":33999.99}
```

No panics; aggregates healthy and consistent with the B0 baseline shape;
zero loaded-entity leak. The new server message arms are validate+echo only —
no sim interaction, as expected.

## Vanilla regression

Booted `veloren-voxygen` with **no** flag: main menu renders normally
(screenshot `b2a-vanilla-boot.png`). All B2a input/UI is Overseer-context- and
`bastion`-gated.

## In-game acceptance (scripted + live QA, spectate overseer)

Evidence screenshots in the session scratchpad (`b2a-qa-*.png`).

- **Left-drag pans** — intact (grab-drag used throughout the run). ✓
- **Left-click selects/inspects** — chat + info line `Selected: entity 81 —
  health 100%` on clicking a spawned deer; `BastionSelected` marker drives
  the info line and feeds the B1.6 cutaway targets. ✓
- **Designate-paint** — Mine-tool drag painted a live preview (yellow
  outline), release sent `BastionPlaceDesignation`, server validated and
  echoed, overlay rendered as an orange debug-line rectangle. ✓
- **Radial menu** — right-click opened a context menu: title "Ground" on
  terrain with pie [Build/Stockpile/Mine/Chop/Bless] + **More…** overflow
  wedge (Rain); entity context proven by the pick ack `[bastion stub] Inspect
  on entity 81`. Influence pick ack: `[bastion stub] Influence Rain at
  (9520, 10702, 1004)` — correct world coords. Radial closes on pick and on
  world-click. ✓
- **Tool palette** — top-center conrod strip (Pan/Inspect/Mine/Chop/Build/
  Stockpile + God-Free button); `T` cycles (chat-confirmed each step), `G`
  toggles ruleset (`Overseer ruleset: Free mode (enforced from B2b)`);
  palette highlight follows. ✓
- **Reclaimed slots** — Primary/Secondary now owned by the Overseer scheme
  and drive select/radial/paint. Deviation: `Interact` stays suppressed (its
  physical key `E` is owned by rotate-right; per-context overrides are B9) —
  covered by new `BastionCycleTool`(T)/`BastionToggleGodMode`(G) inputs. ✓
- **Perf** — 59–60 fps at 4K in overseer with the palette + overlays. ✓

## Bug found & fixed during the gate

**Entity pick-ray range** (`e7e9801`): the cursor-ray origin (NDC z=1) sits
`OVERSEER_BEHIND` (768 blocks) *behind* the camera plane since the B1.7 ortho
near extension, so the pick's `t ≤ 600` cap rejected every entity — clicks
"missed" everything and the radial always resolved to Ground. Plane-picking
(pan/zoom/paint) was immune (plane intersection is origin-offset-invariant).
Cap raised to 2000 with a NOTE. Lesson recorded for future ray users in
`BASTION_B2a_FINDINGS.md`.

## Watch items

- During scripted QA with the user simultaneously at the machine, radial
  opens were observed that scripts didn't issue — attributed to live human
  right-clicks (the only code path is RMB-release within 6px slop). Watch in
  future runs; no code path found for spurious opens.
- The selection info line was initially drawn under the chat box; moved up +
  select now also emits a chat line (`80fd1ba`).
