# M3 — Ladder Contention (FINAL builder packet)

Status: READY TO FIRE the instant R10 (fencing token) lands and tags. Translates + tightens
`M3-CONTENTION-BUILD-PACKET.md` (architect draft) with R9 folded in per `RESEARCH-TRIAGE-R9-R12.md`
(ChatGPT-web, architect-accepted) — R9 is MORE RIGOROUS than the original M3 draft and supersedes its
reservation/queue design. `MINE-LOGISTICS-DESIGN.md` is context only (where M3 leads); nothing from it
is build scope here.

## Goal (one hard thing: contention)
M2 proved ONE colonist builds + climbs a ladder under the single-owner contract. M3 proves it holds
under **multiple trapped colonists needing the same egress**: they reserve/queue, wait SAFELY, and ALL
get out in a bounded, fair order — no deadlock, no starvation, no fall-off, no double-ownership.

## Capacity: ONE (confirmed, do not sneak capacity-N)
One owned entry→top-exit transaction per ladder for M3. Capacity-N (multiple same-direction climbers on
different rungs) is `MINE-LOGISTICS-DESIGN.md`'s ML-2/3 tier — out of scope here, but the `TraversalLink`
type below should carry a `capacity` field now (set to 1) so ML-2/3 raises it later without a type change.

## ★ VERIFIED CURRENT STATE (read directly, 2026-07-19 — don't trust the draft's symbol names blind)
- `server/src/bastion_traversal.rs` (231 lines, Stage-1 extraction, already exists) defines
  `BastionTraversalTask` — ONE struct doing double duty as both link-state and session-state. It already
  has a `link_id: u64` field, but link identity has NO independent lifecycle: it only exists for the
  duration of one task. `reserved_member: Uid` is a single field, not a queue.
- **`traversal_queue_head` (`bastion_jobs.rs:3327-3341`) is the LIVE anti-pattern R9 explicitly warns
  about.** Read it: when no task exists yet, the "queue head" is computed as
  `.min_by_key(|member| member.0.get())` — literally lowest-UID-alone. This is not a hypothetical risk;
  it is the exact starvation-after-cancel/reacquire bug R9's routing note names. M3 replaces this
  call site's fallback logic with the real fair queue.
- `common/src/comp/bastion.rs:64-110` defines `BastionTraversalMode` (6 variants) and
  `BastionTraversalOwnership` (the read-only inspection struct: `link_id`, `route_owner`,
  `reserved_member`, `mode`, `terrain_revision`) — this is what UI-5/inspection reads today; it gains
  queue-position/ticket fields, read-only, no behavior change.
- Confirms the M3 draft's own claim ("R9 already reconciled into M3-CONTENTION-BUILD-PACKET.md") is
  **stale** — the draft still describes the old single-`reserved_member` design, not R9's
  `TraversalLink`/generation/queue-ticket structure. This packet is what actually does that folding.

## Design (R9, superseding the draft's simpler reservation/queue shape)
Prior art: Cooperative A* / WHCA* (space-time reservations), narrowed to treat each traversal LINK as a
capacity-limited shared resource (NOT a replacement for global pathfinding).

- **NEW: a persistent `TraversalLink`** (factor OUT of `BastionTraversalTask`, `bastion_traversal.rs`) —
  stable link identity that survives across sessions/tasks: `link_id`, `entry`, `exit`,
  `terrain_revision`, `capacity` (=1 for M3), `reservation_generation`, and the queue itself. Today's
  `link_id` field on the task dies with the task; M3's link must outlive it so a queue can exist before
  any task does.
- **Reservation tuple:** `link_id` / `generation` / `member_uid` / `enqueue_tick` / `direction` /
  `phase` — replaces the bare `reserved_member: Uid` field.
- **Fair queue key: `(enqueue_tick, colonist_uid)`, NOT UID-alone.** UID is the tiebreak only. This is
  the direct fix for the confirmed live anti-pattern above.
- **Queue ticket:** emitted per member on enqueue, carried as a new recorder-v2 field (additive,
  v1-compatible — `link_id`, `queue_ticket` (enqueue_tick+UID), `reservation_generation`, `direction`,
  `phase`).
- **R10 note (informational, not this packet's scope):** the architect's R10 fencing-token work
  (`ownership_epoch`, a validate-then-write helper at the bastion owned-write sites — NOT at
  `sys/agent/mod.rs`, per the builder-review amendment) sequences at-or-before M3 and will add an
  `ownership_epoch` field on top of this same reservation tuple. Don't build epoch/fencing here; do
  leave the reservation tuple's shape open to that addition (it composes cleanly — `generation` here
  and R10's `epoch` are complementary, not the same field).

## Acceptance (un-fakeable, on the recorder tape — these are the gate)
- **Single-owner-per-link at every tick:** at most one member in an owned traversal mode for a given
  link (the link's reservation is sole authority; the queue head is decided by the fair key). Prove on
  tape: zero ticks with two owned-mode members on the same link.
- **Safe waiting:** queued colonists never inside the body lane during the owner's traversal (SOFT-0
  exclusion — XY deviation from lane center held; never on the rungs; never fall back into the pit;
  never cap-wedge waiting).
- **Release frees the slot deterministically:** after the owner's Complete+release, the next queued
  member reserves and traverses within budget — proves release actually frees the link (the N2 property,
  at N>2 scale, now driven by the fair queue not the min-UID fallback).
- **No deadlock:** every trapped colonist eventually exits (bounded per-member wait). **No starvation:**
  queue order is fair/deterministic by `(enqueue_tick, uid)` — no member waits behind others
  indefinitely, and no member starves after a cancel/reacquire re-enqueue (the specific failure mode
  UID-alone ordering permits).
- **Never-stranded holds** under contention (the M2/2-opt safety floor is inviolable — a queued member
  whose turn never comes must still be caught by the net; the queue must not livelock the watch).

## WHERE TO LOOK (START HERE → THEN → REFERENCE-ONLY)

**START HERE:**
- `server/src/bastion_traversal.rs` — add the `TraversalLink` type (persistent identity, `capacity`,
  `reservation_generation`, the queue) alongside the existing `BastionTraversalTask`. First edit: factor
  `link_id`'s lifecycle out of the task so a link can exist (and hold a queue) with zero active task.
- `bastion_jobs.rs:3327-3341` (`traversal_queue_head`) — replace the `.min_by_key(uid)` fallback with a
  real `(enqueue_tick, uid)`-ordered queue lookup against the new `TraversalLink`. This is the exact bug
  fix at the exact confirmed site.
- The N2 fixture episode (`bastion-harness`, `--ladder-episode N2`) — two colonists, one ladder,
  reservation exclusion + release-frees-slot. **Generalize N2 → N-body** (N=3, N=5) with the new fair
  queue + the acceptance predicates above.

**THEN (build order):**
- Wire reservation-tuple creation/acquire/release/cancel/lifecycle at the existing task-construction call
  sites in `bastion_jobs.rs` (e.g. the `reserved_member: *uid` task-construction sites — grep
  `reserved_member:` for the full list) to read/write through the new `TraversalLink` instead of the bare
  field.
- Wire the queued-member SAFE-WAIT (SOFT-0 exclusion + no cap-wedge + no fall-back) — reuse the M2
  owned-approach machinery; a queued member holds at a safe staging cell, not in the body lane.
- Add the read-only queue-position/ticket fields to `BastionTraversalOwnership`
  (`common/src/comp/bastion.rs:102-110`) for inspection — no behavior change, additive only.

**REFERENCE-ONLY (do not modify):**
- The M2 owned-traversal phase machinery (`BastionTraversalPhase`'s `transition()` state table,
  `bastion_traversal.rs:122-176`) — M3 is the queue in FRONT of this contract, not a change to it.
- ★★★ **THE KEY INTERACTION — read this before touching the watchdog:** the never-stranded net + the
  progress-discrimination watch (from the 2/6-optimization / BACKSTOPOPT) must NOT livelock on a
  legitimately-queued member. A member waiting its turn IS making no positional progress — if the watch
  treats that as a hopeless cycle, it will wrongly teleport a colonist who is correctly, safely waiting
  in a fair queue. The queue-wait MUST be a NAMED, bounded, progress-exempt state — the same shape as the
  existing energy-wait hold, not a new mechanism. This is the single most important correctness
  interaction in this packet; get the acceptance-episode proof (M3-D below) right before considering the
  block done.

## Fixture episodes (M3 matrix)
- **M3-A (N=3, one ladder):** all three exit in fair order; single-owner every tick; safe-wait;
  release-frees.
- **M3-B (N=5, one 1-wide shaft):** the chokepoint case (cf. `--chokepoint-scenario`) — no deadlock,
  bounded per-member wait, never-stranded.
- **M3-C (contention + a mid-queue abort):** the owner aborts mid-traversal → the queue must re-elect
  the next head cleanly by `(enqueue_tick, uid)`, no double-ownership, no orphaned reservation.
- **M3-D (queued-member never-stranded — THE SAFETY-PROOF EPISODE, prove BOTH arms):** a member whose
  turn is delayed past its budget → the net catches it (never-stranded); AND a member legitimately
  waiting its turn within budget → the net does NOT fire (progress-exemption holds). Both arms must be
  demonstrated on tape, à la BACKSTOPOPT's N7/N7B pair.

## Corpus
N ∈ {3, 5} × 6 seeds × 3 reps × both scenarios; escape-time-under-contention report (total time for
all-N-out; the last colonist's exit is the metric). Deterministic ×3.

## Discipline
Corpus-first; un-fakeable tape predicates (single-owner, safe-wait, all-out, fair-order); no
reintroducing the mount/approach flake; capacity-one (do not sneak capacity-N — leave the `capacity`
field at 1); the never-stranded floor is inviolable; the queue-wait must be a named progress-exempt
bounded state (★ the key interaction above). Self-verify + package for the architect's inline
Opus-depth gate (safety focus: no double-ownership, no starvation, never-stranded under contention,
queue-wait progress-exemption correct).

## Sequencing note
M3 is gated on the 2/6-optimization having cleared (done — BACKSTOPOPT tagged) AND on R10 (fencing
token) landing/tagging first, per the architect's explicit fire condition. Do not start until R10 tags.
