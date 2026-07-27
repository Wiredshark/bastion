# APEX-T3.3.13 — Row status (elevated gate)

**Ruling:** Fable, full elevated gate. Landed `acb2bbdb06`. Opus's
independent pre-merge re-verification (recompute-don't-trust, given the
cwd disclosure below) = **PASS**; cleared to merge. Scope: parallel
region-worker block only (Create/Delete/paired EntitySync+CompSync/
throttle CompSync) — the sequential own-entity/spectator CompSync sends
outside the parallel block are T3.3.14 scope, confirmed by Fable.

**Force-update-counter delta, forward-looking note:** V1's CompSync
gives each subscriber its own correct force-update counter; Legacy's
prepare/send_prepared optimization bakes only the first subscriber's
counter into the shared prepared bytes reused for the rest of the
region (a pre-existing quirk, untouched). Inert today (V1 unreachable).
When V1 eventually activates this becomes a real, BENEFICIAL behavioral
difference from Legacy, not a bug — record it now so it isn't
rediscovered as a surprise diff later. Intersects `T3.6`'s
`PhysicsGeneration` work.

**Advisory from Opus's pre-merge review (non-blocking, folded into
T3.3.14):** the real call sites' subject/local_ordinal conventions
(`0`/`1` for the paired vs. throttle CompSync, `for_uid`/`for_region`
choices) are currently MIRRORED in `semantic_intents`/
`semantic_intents_parallel`'s own test fixtures, not pinned against the
real sites — `Sys::run` is never invoked by any test, so a drive-by edit
of the real ordinal or subject-key choice fails no test today. Closed in
T3.3.14 (see that row's own status doc for which resolution was chosen).

**Interleaving wart (record only, not solved here):** once the parallel
block routes through the outbox and the two sequential CompSync sites
stay direct-send, a V1-attached client would receive CompSync frames
via two paths whose relative order within one tick is unspecified — the
outbox's canonical total order governs only outbox traffic. Unreachable
live today (V1 dormant, T3.3.05). **T3.3.15's egress-drain placement
must answer this explicitly** before both paths can be live
simultaneously.
