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

## Gates
- [ ] Independent Python producer for both leaf/owner/domain shapes;
      Rust reproduces byte-exact.
- [ ] Wire golden for the extended ack recomputed (deliberate shape
      change, fork-internal message pair — noted, not hidden).
- [ ] Red-demo: mutate the PassDraw leaf id → exactly its consumers fail.
- [ ] Headless three-leg suite still green (acks carry visual: None;
      run_root untouched).
- [ ] One voxygen ack leg: acks carry Some(visual) with stable roots on
      the static spectator scene.
