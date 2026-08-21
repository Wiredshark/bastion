# Renderer architecture selection — the program's endpoint (2026-08-21)

The Codex program's final artifact: a selection made ON the bench's
evidence, not on taste. Everything cited here is on this fork and was
measured this session (RTX 5070 / Vulkan; limits named at the bottom).

## The selection
**Keep Veloren's renderer architecture; adopt the ported Codex ladder as
its instrumented evolution path; ship the far-band terrain plan as the
first user-visible win; hold the R2 accelerators at landed-but-dormant
until their own A/B.**

1. **No architecture swap.** The program's own evidence removes the case
   for one: the wgpu-based pipeline, instrumented, holds 60 FPS
   (interval p50 16.6ms) with busy p99 13.3ms under the DOUBLED far-band
   load. The bottleneck story that motivated exploring alternatives died
   by measurement — capacity was there; visibility wasn't.
2. **The instrument layer IS the architecture decision.** What we
   actually lacked (and now have, proven live): the semantic tape with
   its triple verdict (determinism + replication neutrality), the r0p
   observatory (durable per-frame records: timing phases, draws,
   geometry, uploads, residency, horizon census), and the certified
   substrate (r0d). Every future renderer change lands against these.
3. **Far band (16→24): ADOPT.** Evidence: 189 chunks visible in the
   17–24 band, visible radius 23 chunks / 736 blocks (stock: 0 / 10 /
   320). Cost: busy p50 4.2→9.3ms, p99 6.3→13.3ms — inside the 60 FPS
   budget with ~3ms headroom on this GPU. Ship as the opt-in profile it
   already is; promotion to default rides a mid-tier-GPU rerun of the
   same two-arm leg (the runner is one command).
4. **R2 accelerators (GPU cull parity, indirect draw): HOLD, next to
   measure.** They are exactly the headroom lever if mid-tier GPUs eat
   the far band's 2× busy cost poorly — and they carry their own parity
   instruments. Their A/B is the same two-arm shape with the accelerator
   env toggled.

## Evidence base (all on-disk, reproducible)
- `smoke/gpu/r0p-{FAR,REF}.json/` — full frame distributions, both arms.
- `HORIZON-RETEST-RESULTS.md` — the item-19 verdict table.
- `LW-PORT-LEDGER.md` — the ladder port + its 292/292 suite.
- W1–W3 bench: triple-verdict attested (W3-LAUNCH-PACKET.md ledger).

## Named limits (what this selection does NOT claim)
- One GPU (RTX 5070), one scene (static flat-arena spectator). The
  adopt-as-default call for the far band explicitly waits on a mid-tier
  rerun; the selection itself (no swap + instrument-first) does not.
- The certified camera-pose path and PNG capture legs are unfinished
  (W5-class polish) — the census stands without them.
- PassDraw/VisualStructure tape domains (W0 ids 12/13) remain unbound to
  the SEMANTIC tape; r0p records draw counts already, and binding the
  domains is the named next wave if per-domain visual-neutrality proofs
  are wanted before renderer refactors begin.
