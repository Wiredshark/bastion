# APEX-T3.3.13 — Row status (elevated gate)

**Ruling:** Fable, full elevated gate. Landed `acb2bbdb06`, pre-merge
review owned by Opus per the ruling. Scope: parallel region-worker
block only (Create/Delete/paired EntitySync+CompSync/throttle
CompSync) — the sequential own-entity/spectator CompSync sends outside
the parallel block are T3.3.14 scope, confirmed by Fable.

**Interleaving wart (record only, not solved here):** once the parallel
block routes through the outbox and the two sequential CompSync sites
stay direct-send, a V1-attached client would receive CompSync frames
via two paths whose relative order within one tick is unspecified — the
outbox's canonical total order governs only outbox traffic. Unreachable
live today (V1 dormant, T3.3.05). **T3.3.15's egress-drain placement
must answer this explicitly** before both paths can be live
simultaneously.
