# bastion-model — R12 traversal-contract model-checker: results

Standalone exhaustive model checker over the ladder-traversal ownership contract
(R9 fair-queue/link + R10 epoch fencing as amended + the d84005dc89 release-decision
machine + the always-armed no-progress watchdog + the net floor). Zero non-std deps.

Run: `cd bastion-model && cargo test` (the checker's own falsifiers) or
`cargo run -- [--members 2|3] [--max-depth N] [--break-fence|--break-queue|--break-bound|--break-revision|--break-death]`.

## Headline

**The faithful contract-as-modeled is CLEAN: all safety (S1–S6) and liveness (L1–L2)
properties hold over the full reachable state space at both configs.**

| config | states | edges | verdict |
|---|---|---|---|
| faithful, 2 members | 5,849 | 31,518 | PASS (S1–S6, L1–L2) |
| faithful, 3 members | 165,950 | 1,018,128 | PASS (S1–S6, L1–L2) |

No contract hole found. The mechanisms shipped this week — the R10 fence, the R9
fair queue, the release-decision reengage bound, the watchdog releasing into the
bounded machinery — are each individually load-bearing: removing ANY one of them
produces a property violation with a minimal counterexample trace (below). That is
the checker detecting by construction the exact bug classes the corpus caught by
luck.

## Falsifier results (each knob removes ONE shipped mechanism)

| broken mechanism | fires | minimal witness (from `cargo run`) |
|---|---|---|
| `--break-fence` (R10 epoch fence off) | **S3** + **S2** + S6 cascade | S3 in 4 steps: `Enqueue(0), Reserve(0), WatchdogAbort(0), StaleWrite` — the released owner's delayed old-epoch write shoves them back into Traversing. S2 in 6: same prefix + `Enqueue(1), Reserve(1), StaleWrite` — **two owned-moving members, one link** (the double-owner class). |
| `--break-queue` (min-UID selection, the live anti-pattern R9 killed) | **L1** | member 1 queued forever; displacement cycle `Reacquire(0), Reserve(0), ApproachStep(0), ContactAcquire(0), EnterLink(0), FrontierComplete(0), WatchdogAbort(0)` — member 0's abort/reacquire always outruns member 1's turn. |
| `--break-bound` (reengage bound off) | **L2** | livelock cycle `Reacquire(1), Reserve(1), WatchdogAbort(1)` — abort→reacquire forever, no progress, net never reached (the registry class-12 shape). |
| `--break-revision` (terrain validation off) | **S4** | traversal progresses across a revision mismatch. |
| `--break-death` (despawn advance-site off) | **S5** | 3-step witness: dead member holds the reservation (the stale-map-entry class from the N6 analysis). |

All eight unit tests green (`cargo test`, ~4 s): 2- and 3-member faithful PASS +
the six falsifiers fire + minimal-trace check.

## Model insight worth carrying (from the broken-fence world)

With the fence off, the checker ALSO finds S6 stranding: the stale write creates a
"zombie owner" — Traversing with no reservation — whom the modeled watchdog cannot
abort (it guards reservation holders only) and who therefore can never be
delivered. Scope caveat: the REAL engine's ultimate failsafe is
reservation-independent (positional stuck-watch), so live stranding would still be
netted; but the insight stands that **the fence is protecting deliverability, not
just single-ownership** — a delayed write that lands creates a member the ownership
machinery no longer tracks.

## Properties (exact formulations)

- **S1** a reservation is never held by a non-owned-phase live member (capacity=1
  is structural: the reservation is a single `Option`).
- **S2** at most one owned-moving member per link, and it must be the reservation
  holder.
- **S3** a stale-epoch write mutates nothing (checked structurally on every
  `StaleWrite` edge: post-state == pre-state minus the consumed token).
- **S4** no `EnterLink`/`FrontierComplete`/`TopExit` under a revision mismatch
  (checked on every edge).
- **S5** a dead member holds no reservation.
- **S6** never-stranded skeleton: from every reachable state, every live
  non-terminal member can still reach Delivered or Netted — via backward
  reachability; death paths do NOT count as delivery.
- **L1** no starvation: no fair-closed cycling SCC keeps a live member Queued in
  every state (weak fairness on system actions; `Reserve` is intermittently
  enabled, so fairness cannot rescue — the fair queue must).
- **L2** no livelock: in the no-progress subgraph (progress edges removed), no
  fair-closed cycling SCC contains a `Reacquire` edge — the reengage bound must
  terminate abort→reacquire into net-delivery (a progress action).

Weak fairness is applied to SYSTEM actions only; environment actions (terrain
mutation, contact loss, interruptions, death, delayed stale packets, and a
member's own choice to Enqueue) are never owed fairness. Fairness handling is the
standard finite-state justice approximation: an SCC is a valid counterexample only
if it is *fair-closed* (no system action continuously enabled across the whole SCC
has all its edges exiting it).

## Honest gaps (what is NOT modeled)

1. `REENGAGE_BOUND` modeled as 2 (engine: `EMERGENCY_REENGAGE_BOUND` = 5); the
   property proven is bound-existence/termination, which is value-independent.
2. Terrain revision is one validity bit on the active reservation; a fresh
   reservation always re-validates. **Permanent-seal geometry (the N1C class —
   replan can never validate) is not modeled**, so S6 here does not cover the
   sealed-vault case; in the engine that case is covered by the
   reservation-independent positional net, which is also not modeled (the model's
   net fires only on reengage exhaustion).
3. R9 `generation` and R10 `epoch` are collapsed into one fencing abstraction
   (single in-flight stale token; multiple concurrent stale packets not modeled).
4. Untimed: watchdogs/budgets are nondeterministic actions + weak fairness, not
   timers — no wall-clock bounds are proven, only eventuality.
5. No physics: contact is a free environment bit; no falling/entombment geometry;
   ExitConfirm collapses the 5-stable-sample window to one action.
6. Single link; multi-link/portal composition out of scope.
7. L1 is proven for members who have enqueued (Enqueue is member volition,
   classified environment).
8. Model tick is not in the state (traces order actions); queue Vec order IS the
   `(enqueue_tick, uid)` order because interleaving assigns distinct ticks.

## Fidelity sources

- `readme/RESEARCH-TRIAGE-R9-R12.md` §R9 (TraversalLink, fair key
  `(enqueue_tick, uid)`, capacity-one) and §R10 as amended (fence at the
  owned-write sites; advance-on-release / adopt-on-acquire).
- Commit `d84005dc89`: `release_decision` (keep-driving / exhausted-replan /
  stable-exit-candidate), `reengage_exhausted` (the bound), abort-side counting.
- `readme/M3-BUILDER-PACKET-FINAL.md`: fair-queue semantics + M3 acceptance
  properties (single-owner-per-tick, release-frees-slot, no deadlock/starvation,
  never-stranded under contention) — S1/S2/L1/S6 are their model forms.
- The always-armed no-progress watchdog mirrors `MOUNT_NO_PROGRESS_TICKS` + the
  positional watch (M2/BACKSTOPOPT), releasing into the bounded reengage
  machinery, never replacing the net.
