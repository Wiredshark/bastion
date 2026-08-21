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

## W5 honest state
Schema/oracle/readback-registry/shutdown/artifact-transaction substance
is on the fork (r0d modules, 292/292; exactly-once registry; atomic
tmp→rename tape). The automated CAPTURE leg is blocked at a NAMED gate:
in pause-mode the design waits for the OPERATOR's pause (works by
design — launch the freeze leg, press ESC at tick 300); in absolute
mode (`BASTION_R0D_CAPTURE_AT=1`) the SETTLED_TRACE_GATE never reached
`Open{advanced}` under the automated spectator flow — that gate's
observability (state witnesses inside `observe()`) is the single named
follow-up. A ready_token witness was added this session (the first
silent gate found by the same hunt).
