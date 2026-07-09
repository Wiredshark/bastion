# B-TESTBED — Reference Colony Test Environment (integrated-dynamic tier, observable by Claude)

> **For Ben:** paste into a game-build session at `E:\veloren-master` (or queue it — independent block,
> best after B6 lands so the economy loop exists to observe; extends B0 harness + B-ASSET1 arena). Standard
> block protocol: branch → build → self-test → merge → tag `bastion-block-BTESTBED`. Concurrent asset
> session's files are read-only inputs.

## WHY
The test ladder's top rung is only spec'd, never built: STATIC (geometry) → ISOLATED-DYNAMIC (one asset,
flat arena, B-ASSET1) → **INTEGRATED-DYNAMIC (this block)** → SOAK. The flat arena proves an asset is
internally sound; it cannot prove a building gets *built by real colonists, in a real colony, and then
used* — or that monsters *behave* — or that systems compose. This block builds the **reference colony**: a
seeded, reproducible, fully-running colony scenario that boots on demand, instrumented so BOTH Ben (eyes)
and CLAUDE (telemetry + captured frames) can observe and judge full system lifecycles.

## PART 1 — The reference colony scenario (the fixture, colony-scale)
- A **scripted, seeded setup** bootable headless or with client: fixed world seed + region, a founded
  colony (N colonists via B3 spawn-at-site), pre-placed stockpiles, starting resources, a road stub, and a
  surrounding cast (deer herd, a wolf pack, a lair) — all placed deterministically. One command:
  `--testbed [scenario]`.
- **Scenario library (data-driven — a scenario = setup + script + assertions + camera track):**
  - `construction`: a building blueprint is designated at t=0 → observe the FULL lifecycle: material
    hauling → site-prep (if built) → construction progress → completion → a colonist pathing in and USING
    it (the interior function point). Asserts each stage transition within tick budgets.
  - `economy`: mine designation + stockpile → paint→claim→mine→pile→haul→store, end to end, conservation
    asserted at every hop.
  - `wildlife`: the deer herd grazes/flees; the wolf pack hunts from it (population + drive behavior,
    loaded-tier). Asserts: predation occurs, herd responds, no stuck agents.
  - `monster`: a lair-dwelling hostile with drives near the boundary → territory behavior, incursion,
    colony threat-response (grows teeth with B8; ships with what exists).
  - `defense`, `operables`, `weather` — stubs added as their systems land. New scenarios are DATA, not
    code rewrites.
- Deterministic-enough: same seed → same setup; behavior asserted invariant-style (stages complete, nothing
  stuck), never trace-exact.

## PART 2 — The event timeline (Claude's structured eyes)
- Instrument the testbed to emit a **structured event log** (`testbed_timeline.jsonl` + human-readable
  summary): tick-stamped entries for every meaningful transition — job claimed/started/completed/failed,
  construction stage changes, item created/merged/hauled/stored, agent stuck-warnings (watchdog),
  creature drive events (hunt started/kill/flee), boundary incursions, panics/errors.
- Post-run, a **verdict summary**: per-scenario assertions PASS/FAIL with the timeline evidence attached.
  Appended to `readme/TESTBED_LOG.md` (append-only). A Claude session (either agent) reads the timeline and
  can diagnose WITHOUT running the game — the timeline is the testbed's transcript.

## PART 3 — Visual capture (Claude's actual eyes — §3p render→view, in-engine)
- **Scripted camera + automated screenshot capture:** each scenario defines a camera track (godcam
  positions/targets — B1 machinery) and capture triggers (every N ticks AND on key events from Part 2:
  construction-stage-complete, first-use, predation kill). Frames saved to `testbed_captures/<run>/` with
  timeline-linked filenames (`t01234_construction_complete.png`).
- Headless-compatible path: if the full client is needed for rendering, run it windowed with input scripted;
  if an offscreen render target is feasible, prefer it. Document which was achievable.
- **The Claude review loop:** after a run, a Claude session views the captured frames alongside the
  timeline — "construction completed at t=1234, here is what it LOOKS like" — and judges visually (misplaced
  geometry, floating buildings, T-posing, visual wrongness the assertions can't see). This closes the
  §3p loop against the REAL game: harness = floor, Claude's eye = ceiling-check, Ben's eye = final.
- For Ben: the same scenarios boot with free camera for live watching (`--testbed construction --watch`) —
  the colony-scale version of the asset arena.

## PART 4 — Wiring into both workstreams
- **Build queue:** each future block adds/extends a testbed scenario as part of its Done-when (B8 fills
  `defense`; operable-engine fills `operables`) — the testbed grows with the game and becomes the standing
  integration regression suite (run key scenarios before every merge, like the B4/B5 scenarios today).
- **Asset pipeline:** an asset's INTEGRATED-DYNAMIC validation = appearing in a testbed scenario (a new
  building type gets validated by being the `construction` scenario's subject). The asset session reads
  `TESTBED_LOG.md` + captures to upgrade catalog status to FULLY-VALIDATED.

## DONE-WHEN
- `--testbed construction` and `--testbed economy` run headless end-to-end with timeline + verdicts, green.
- `wildlife` scenario runs; predation observed in the timeline.
- Visual capture produces timeline-linked frames for at least the construction scenario; a sample
  Claude-review of frames+timeline is performed and logged (prove the loop, not just the plumbing).
- `--watch` mode boots for Ben. Vanilla flagless boot clean. Tag + bookkeeping + findings doc.

## WATCH-ITEMS
- Screenshot capture from the client may be the fiddly part (windowing/offscreen) — timebox it; the
  timeline alone is most of the value, frames are the multiplier.
- Scenario runtime budget: each core scenario should complete in minutes, not hours — testbed speed is what
  makes it a regression suite instead of a chore (use TimeScale where behavior allows).
- The construction scenario depends on how much of build/site-prep exists — script to what's real, extend
  as blocks land. Never assert against unbuilt systems.
