# W4 launch packet (A5) — PassDraw + VisualStructure domains, semantic comparator (self-authored)

Lease r0d-lease-04-w5's surfaces: session/scene/FigureMgr semantic
observer hooks; harness semantic comparator. Domains used: the W0 table's
pre-allocated `PassDraw = 12` and `VisualStructure = 13`.

## Registered semantics
1. **Source of truth**: the RENDERING CLIENT. A server has no draw calls;
   these domains ride the W3 ack channel as observational sidecar data —
   NEVER in run_root (determinism ⊥ wall-coupling: draw counts vary by
   GPU/settings by design; the point is a fingerprint that names what
   moved when the SAME rig re-runs the same scene).
2. **Leaves (v1 = frame-aggregate granularity; per-pass detail is a
   future wave, named not implied):**
   - PassDraw `0x0C00_0001`, owner `Pass(6)`, owner_key = u32 ordinal LE
     (v1: single owner, ordinal 0 = the frame aggregate). Payload LE:
     u32 pass_count ‖ u32 draw_count ‖ u32 instances ‖ u64 geometry_units.
   - VisualStructure `0x0D00_0001`, owner `Frame(2)`, owner_key = u32
     frame_index LE. Payload LE: u32 terrain_chunks ‖ u32
     visible_terrain_chunks ‖ u32 shadow_terrain_chunks ‖ u32
     bind_group_sets.
3. **Plumbing**: voxygen's session feeds the client a per-tick
   `BenchSceneStatsV1` note (the semantic observer hook); the client's
   ack builder computes both domain roots from the latest note (staleness
   ≤ 1 tick, observational) and attaches them as
   `visual: Option<BenchVisualDomainsV1>` on the ack. The headless
   ackbot has no renderer → `None` — absence is honest, not zeros.
4. **Tape**: `client_acks` entries carry the two roots when present.
   Frames additionally expose the SERVER-side per-domain roots in the
   envelope (`domains` hex map) — run_root computation untouched.
5. **Comparator**: the golden CLI, on a frame mismatch, now names the
   first DIVERGENT DOMAIN (script/movement/identity) when both tapes
   carry the domain map — a renderer change that alters nothing semantic
   is provably visual-neutral per-domain, and one that does names what
   moved.

## Gates — ALL GREEN (2026-08-21)
- [x] Independent Python producer reproduced byte-exact, 16/16 FIRST RUN
      (`w4_visual_domain_vectors_v1.py` → checked-in JSON).
- [x] Wire golden recomputed deliberately (`cc14325a…`), noted in place.
- [x] Red-demo EXACT: PassDraw leaf id 0x…01→0x…99 → 1 failed by name,
      15 green; restored 16/16.
- [x] Headless three-leg suite GREEN on this tree (twins identical,
      neutrality identical, acks green with visual honestly absent).
- [x] Voxygen ack leg GREEN: **20/20 acks carry Some(visual), all echoes
      matched, both entities resolved from frame 1, PassDraw root STABLE
      across the settled tail** — a live renderer fingerprinting its own
      draw decisions through the bench channel.

## W6 (built + DOGFOODED the same session)
`--renderer-bench-promote <cand> <golden> --attest "<who/why>"`:
refuses unattested / malformed; every promotion appends an audit line
(PROMOTIONS.log: epoch, sha256, run_root, frames, replaced, attest).
First blessed golden: `goldens/walk-and-seek-v2.json` (the headless twin
witness), and the INDEPENDENT twin tape PASSES against it. Comparator
now names the divergent DOMAIN on mismatch (3/3 module tests).

## W5 closure statement (lease deliverables vs the ops flow)
Every W5A/W5B LEASE deliverable exists on the fork and is tested:
capture schema + visual comparator (`bastion-renderer-r0d` capture/
visual_oracle modules, in the 292/292), renderer pipeline/readback hooks
(`record_draw` + the r1bc GPU receipts logging live in every leg),
readback registry (exactly-once, W2, unit-tested), shutdown hooks
(r0d shutdown module + the singleplayer freeze/latch flow, exercised
live), artifact transaction (atomic tmp→rename, every tape).

What does NOT yet run unattended is the end-to-end CAPTURE FLOW — an
ops sequence, not a lease surface. Burned down this session, each gate
named by new telemetry (committed):
1. r1f weather absent≠invalid server-kill — FIXED.
2. freeze-vs-streaming interaction — understood, documented.
3. pause requirement — AUTOMATED (Space = BastionPauseToggle via
   SendKeys; telemetry shows pause_ok flip true).
4. RESIDUAL: `r1a_presentation::observe_visible_scene` stability —
   requires presentation-generation match + terrain AND figure draw
   coverage + upload_ready across N consecutive frames; never satisfied
   under the silent-spectator flow, horizon-fixture camera competition
   ruled out by a control leg. The next witness belongs INSIDE
   observe_visible_scene (log which conjunct fails, on change) — one
   edit + one leg for whoever picks this up (likely trivial: the
   spectator camera's default pose may simply not frame the colonist,
   in which case the anchor-camera engage order is the fix).
