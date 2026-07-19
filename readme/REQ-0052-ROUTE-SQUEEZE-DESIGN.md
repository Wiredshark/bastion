# REQ-0052 — Route-squeeze collision-radius mechanism (contract doc)

Status: **Opus-cleared for commit** (Opus Reviewer session `local_7e72649b`, full trace
there if this doc's summary isn't enough). Landed as part of the B5.8 Stage-1
external-effort line, not a standalone numbered master-list row.

**Purpose of this doc:** this is a CONTRACT, not a feature writeup. The mechanism's
safety rests entirely on a narrow, server-validated gate — the point of writing this
down is so nobody later broadens the scope, radius, or expiry window without seeing
the envelope it was actually designed and cleared within.

## What it is

A per-colonist timestamp field, `route_squeeze_until: f64` (`common/src/bastion.rs:1238`,
default `0.0`). While `route_squeeze_until > read.time.0` is true for a colonist, that
colonist's horizontal collision cylinder radius shrinks from the normal `0.45` to `0.22`
(`common/systems/src/phys/mod.rs:1145-1156`), feeding directly into
`box_voxel_collision`. This is a **per-colonist, time-windowed** effect — it does not
touch any other colonist, and it does not touch vertical/voxel collision at all, only
the horizontal radius cap used by the phys sweep.

## The gating invariant (why it's safe)

**All 15 write sites live in `server/src/bastion_jobs.rs`**, all inside the server-side
emergency-route/traversal-task machinery (`BastionTraversalPhase` arms:
`LinkApproach`, `FrontierWork`, exit-stability completion, abort/reset paths). No writer
exists anywhere outside that machinery — grep for `route_squeeze_until\s*=` confirms
exactly these 15 sites, all under `bastion_jobs.rs`'s server-authoritative traversal
code, none in client code, none in general phys code.

Only a server-validated, **ADJACENT** emergency-route mount may set it — the mount
check (`dx.max(dy) == 1`, e.g. `bastion_jobs.rs:5287/5311/5465`) requires the colonist
be at most one cell away (cardinal-adjacent, not diagonal-2) from the traversal
mount point before a squeeze window opens; it is preflighted by the route's own
reservation/validation machinery (`transaction.reserve`, `transaction.
validate_terrain_revision`), not a bare distance check.

**200ms auto-expiry**: every write site sets `route_squeeze_until = time.0 + 0.2`
(seconds) — the effect reverts to the normal `0.45` radius automatically 200ms after
the last write, with no separate "turn it back off" path required. Several
completion/abort sites also explicitly zero it early (`= 0.0`) on transaction
completion or reset, rather than relying solely on the timeout.

## The bounded-safety argument

The intuitive worry is "a smaller collision radius sounds more dangerous, more likely
to let a colonist clip into geometry." **Opus's key correction: this is backwards.** A
*smaller* radius means the phys sweep needs *finer* steps to detect the same
penetration depth — smaller radius reduces tunneling risk relative to the normal
radius, it does not increase it. The mechanism narrows the collision envelope in a
way that is strictly more conservative for the sweep, not less.

Any persistent-embed edge case that could still occur is caught by the existing
`embed_watch`/teleport backstop (the R11/CASE-003 class, see
`BASTION_COMMON_ISSUES.md`) **independent of this mechanism** — route-squeeze does not
weaken or bypass that backstop in any way; it is a separate, already-relied-upon net.

## Diagnostic logging

`BASTION_EGRESS_DIAG` — an env-gated diagnostic log (`"bastion: authoritative route
squeeze active"` / `"...resolved"`, `common/systems/src/phys/mod.rs:1176/1214`).
**Observation-only, no state mutation** — safe to leave gated off in normal play,
useful for tracing a specific egress/traversal session when investigating a stuck or
embedded colonist.

## Open items (Opus flagged, did NOT block commit on either — track, don't drop)

1. **`FrontierWork`'s write site (`bastion_jobs.rs:5035`) is UNGATED on
   `traversal_kind == ConstructedLadder`,** unlike `LinkApproach`
   (`bastion_jobs.rs:4530-4536`), which explicitly only squeezes for
   `EmergencyTraversalKind::ConstructedLadder` and sets `0.0` (no squeeze) for every
   other kind. `FrontierWork` squeezes unconditionally for `time.0 + 0.2` regardless of
   traversal kind. **Unresolved: is this deliberate** (e.g. does `NaturalShaft`'s own
   `FrontierWork` phase also need the narrower radius for the same physical reason
   `ConstructedLadder` does?) **or should it be narrowed to match `LinkApproach`'s
   kind-gating?** Needs an explicit answer before this pattern is copied elsewhere.
2. **Interaction with the `has_live_job`/`rescue_pending` watchdog fixes (STUCKJOB,
   `bastion-block-CKSTAIR` → `9ad9d97808`) was not deep-traced.** The two mechanisms
   are believed complementary (route-squeeze narrows the collision envelope during an
   active traversal mount; the watchdog governs whether/when the teleport backstop
   fires for a stuck colonist) but this has NOT been independently verified — flagged
   as **complementary-not-verified-independent**, worth a joint look eventually,
   particularly once Phase 2 of `STAIR-LADDER-MINE-ACCESS-DESIGN.md` starts extending
   the same traversal machinery both mechanisms sit inside.

## Why this matters before Phase 2 (stair/ladder mine access)

`STAIR-LADDER-MINE-ACCESS-DESIGN.md`'s Phase 2 extends the same emergency-route/
traversal-task machinery this mechanism lives inside (new geometries, new traversal
phases). Anyone adding a new phase or a new traversal kind to that machinery should
read this doc FIRST and explicitly decide, for their new write site, whether it
belongs in the `LinkApproach`-style kind-gated camp or the `FrontierWork`-style
unconditional camp — not copy whichever site is nearest without deciding on purpose.
That is precisely the gap open item #1 above is flagging.
