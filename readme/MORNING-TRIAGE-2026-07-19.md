# MORNING TRIAGE — 2026-07-19 (compiled ~23:50 by Sonnet, architect-directed)

One page; the morning fires off this. Format: **item → owner-lane → priority → pointer**.
Priority: P1 = fire first / blocking something, P2 = same-day, P3 = slot when convenient.

## A. In-flight lanes to check FIRST (state may have moved overnight)
| # | Item | Lane | Pri | Pointer |
|---|---|---|---|---|
| A1 | **R10 fencing-token build** — fresh Fable session charter ready to spawn (or already spawned late tonight; check for the session + its scratchpad plan). Gates M3 + STATUS-SURFACE slotting. | fresh Fable builder | P1 | `readme/R10-FRESH-SESSION-CHARTER.md` (+ main builder's scratchpad `r10-implementation-plan.md`) |
| A2 | **Ultra-review findings** — was PENDING tonight; check whether the cloud review returned + triage its findings. | architect → Sonnet curation | P1 | check the ultra-review session/output (no doc pointer existed at compile time) |
| A3 | **Codex DET-0004** — pending closure + review (DET-0002/0003 already reviewed PASS; merge queued as one stacked merge). | Codex lane → reviewer | P2 | `readme/CODEX-DETERMINISM-HUNT-2-TASK.md` (esp. "Notes for the fleet" tail) |
| A4 | **5 bug-hunt chips** (B49 tps-guard / B50 seed-stamp / B51 suite-reset / B52 velorite path / B53 aura-reader) — ALL 5 were STARTED as independent local sessions last night; check each for completion → review → merge. NOTE: B53's chip was architect-REDIRECTED to the reader-side `try_from` fix per the sweep (see D1). | 5 spawned sessions → reviewer merge | P2 | chips task_d09995ee / task_d277c9a0 / task_a65ea53a / task_8d916978 / task_be051dc5; registry rows `BASTION_COMMON_ISSUES.md` B49–B53 |
| A5 | **Builder-2 IDLE-HOME-LEASH** — pending merge into the fleet line. | builder-2 → reviewer merge | P2 | `readme/IDLE-HOME-LEASH-design.md` + builder-2's session/branch |

## B. Queued main-lane builds (order after R10 lands)
| # | Item | Lane | Pri | Pointer |
|---|---|---|---|---|
| B1 | **M3 contention** — FINAL packet ready, fires the instant R10 tags. R9 folded (verified against live code; the min-UID anti-pattern is live at `bastion_jobs.rs:3327`). | main builder | P1 (post-R10) | `readme/M3-BUILDER-PACKET-FINAL.md` |
| B2 | **STATUS-SURFACE (LEGIBILITY-1)** — the "four indistinguishable motionless states" fix + energy meter; small, CHOP-PROGRESS shape. Slot: first fill/warm-down after R10. | main builder (fill) | P2 | `readme/STATUS-SURFACE-BLOCK.md` |
| B3 | **Tier-2 `checked_tick` B16 fix** — `common/src/states/utils.rs:1746` + buff-multiplied speed modifiers with no floor (~15+ character states); PROTECTED main-lane file, so it's a routed main-lane task, NOT a chip. | main builder | P2 | `readme/BUG-INVESTIGATION-LOG.md` §CASE-009 primary sweep (line ~707); Tier-3 is separately routed via `readme/OPUS-B16-TIER3-PROMPT.md` |
| B4 | **DPA (dig-provisioned access)** — packet COMPLETE with Qs resolved (headline: re-enable + re-scope the dormant `AUTO_LADDER_ACCESS` gate, material-cost the rungs, classified block reason). Sequenced per `MINE-LOGISTICS-DESIGN.md` dependency chain (after M3/R10/fork-#15). | main builder (later) | P3 | `readme/DIG-PROVISIONED-ACCESS-PACKET.md` (§7 = resolved Qs + rulings) |

## C. Chips to SPAWN in the morning (not yet spawned)
| # | Item | Lane | Pri | Pointer |
|---|---|---|---|---|
| C1 | **B54 metrics wall-clock chip** — `server/src/metrics.rs:404-406` `.expect("Time went backwards")` → fall back instead of panic (the `bastion_flight_recorder.rs:637-640` `.ok()` idiom). LOW/vanilla/startup-only. *(Resolved: "the metrics nit" in the architect's sprint list was THIS same item, listed twice by error — one entry, not two.)* | chip | P3 | `BASTION_COMMON_ISSUES.md` row B54 (CASE-009 sweep) |
| C2 | **F2/F3 fixture-hardening chips** (stimulus-window family) — Fable's capstone-review dispositions, ruled chips by the architect. | chips | P3 | `readme/BUILD_REVIEW_LOG.md` §FABLE-004 (dispositions ¶) |

## D. Bookkeeping / curation (Sonnet lane, morning first-pass)
| # | Item | Lane | Pri | Pointer |
|---|---|---|---|---|
| D1 | **FABLE-004 F4/F5/F6 bookkeeping items** — ruled "morning bookkeeping" in the closure note; F1 is already folded into the R10 charter (recorder-v2 token witness) — do NOT re-work F1. | Sonnet | P2 | `readme/BUILD_REVIEW_LOG.md` §FABLE-004 (lines ~1205-1281, closure note at the end) |
| D2 | **DPA-3 known-class registry curation** — the designer's raw append (lateral-gallery re-gating, deliberately deferred to Part-B/M5) sits uncurated at `BASTION_COMMON_ISSUES.md:382`; fold into the numbered table (next free class number) per the B44-46/B49-53 precedent. | Sonnet | P2 | `BASTION_COMMON_ISSUES.md:382` + `DIG-PROVISIONED-ACCESS-PACKET.md` §DPA-3 |
| D3 | **Stale-triage-doc correction line** — `RESEARCH-TRIAGE-R9-R12.md` still says "R9 already reconciled into M3-CONTENTION-BUILD-PACKET.md"; false — the folding actually happened in `M3-BUILDER-PACKET-FINAL.md`. Add a one-line correction so the next reader isn't misled. | Sonnet | P3 | `readme/RESEARCH-TRIAGE-R9-R12.md` (tail) |
| D4 | **Bug-hunt chips' registry rows close-out** — as each of A4's 5 chips merges, flip its B49-53 row to fix-verified (the B37 precedent: preserve the original finding text, prepend the verified note). | Sonnet (per-merge) | P2 | `BASTION_COMMON_ISSUES.md` B49-53 |

## E. Standing context (no action, orientation only)
- **Automation-map ranked build-list** — the Play-Tester's complete 29-item mapping; its §3 ranks the
  build-needing pieces (bot-client first green compile is rank 1). Feeds whatever test-infra slot opens.
  → `readme/CHECKLIST-AUTOMATION-MAP.md` §3.
- **Text-sim priority line stands** (Ben): Grok's text-view → interactive-play → recorded-playback →
  live-watch outranks everything Phase-3/3D-render. → `TEST-INFRASTRUCTURE-AUDIT.md` §Phase-2 backlog.
- **Seed-8 through-wall breach follow-up** (CLIMBCAP residual) — committed follow-up: xy on the cap tape +
  one rerun to confirm the class-6/B46 hypothesis; any result implicating the cap/A2 RETROACTIVELY REOPENS
  the CLIMBCAP tag. → run-log §bastion-block-CLIMBCAP.
- **Morning sanity-check**: `git status` in the main checkout before ANY new work — tonight ended with many
  agents' uncommitted source edits in the shared tree (see the gitStatus snapshot; the 5 chip sessions ran in
  their own worktrees, but the main tree carries in-flight edits from the B16 sweep + others).

*HONEST-PARTIAL disclosure: A2's ultra-review had not returned by compile time. (The original second
honest-partial — "the metrics nit" — was resolved post-delivery as the architect's own accidental duplicate of
B54/C1; the duplicate line is removed.) Everything else is pointer-verified against the docs/code as of ~23:50.*

## Post-compile additions (architect, ~00:05)
- [C+] B16 TIER-3 guards NOT YET WRITTEN — Opus's sprint was interrupted pre-edit (tree clean of it). Re-fire readme/OPUS-B16-TIER3-PROMPT.md next window (any capable session). Priority: HIGH (open crash class).
- [D+] LOW/efficiency (Opus ultra-review, verified): BASTION_B55_TRACE_DELETES env::var_os re-read per entity-deletion in state_ext.rs delete_entity_common (:1413) + sys/object.rs (:85) + inventory_manip.rs (:229) — hot path; sys/item.rs (:46) already shows the hoist pattern. Fix = LazyLock cache one-liner, or rides until B55 traces are removed. Owner: Bug-Tester's B55 instrumentation lane.
- [A+] Opus ultra-review CLEAN BILL on builder-2's uncommitted leash selector (determinism/stranding/vanilla-identical all verified) — credit as pre-review at the leash merge gate.

## Night-close additions (architect, ~00:25)
- [A++] R12 MODEL-CHECKER DELIVERED + COMMITTED (f146327668, bastion-model/**): contract CLEAN over 165,950 states (S1-S6+L1-L2), all 5 falsifier knobs fire with minimal traces. MORNING ITEM: the independent fidelity audit (model-vs-real semantics; author=Fable so auditor=architect/Opus) — focus: the documented watchdog-addition, the two honest gaps (permanent-seal geometry + positional net unmodeled; untimed). Its S6 INSIGHT (broken fence → zombie owner → stranding: the fence protects DELIVERABILITY) → banked into R10's rationale.
- [C++] B16 TIER-3: recon COMPLETE (all 4 sites read; projectile new_lifetime sign-provenance ANSWERED = no range check, neg/NaN/inf reachable → guard confirmed needed; no shared duration helper exists → create one on Secs in veloren-common). Full resume packet: BUILD_REVIEW_LOG.md §B16-TIER3 RESUME NOTE. Re-fire next window.
- [D++] Architect ultra-review findings (2, committed in this file's history): bastion_jobs.rs:11506 missing-position⇒verified-exit default (PLAUSIBLE latent, fail-closed follow-on chip + corpus re-proof); action_nodes.rs:51 per-tick env read (LOW, LazyLock one-liner, rides with the B55-family cleanup).
- Ultra-review NET for the branch: 2 findings total across two independent max-effort passes over the full delta — the night's work stands.

## FABLE-005 findings (final wrap, ~00:40 — none tag-blocking; full record BUILD_REVIEW_LOG §FABLE-005)
- [C-PRIORITY] common/src/rtsim.rs:283 — stored path endpoint matched by EXACT float equality: any transformed goto target silently disables the M2 endpoint-tolerance min-clamp → reverts to WIDE tolerance, zero diagnostic (the smoke-77 inversion class, one transform away from re-opening). CHIP with fixture/corpus verification (tolerance-adjacent — not a blind one-liner). Fix property: epsilon match OR an unmatched-endpoint diag.
- [C] trade_pricing.rs:1047 — legacy thread-RNG random_items: AUDIT all callers (no deterministic path may call it) → deprecate/rename. Fold into Codex DET-0005 as an audit item.
- [C] server/src/lib.rs:3208 — class7 healing hook hardcodes (true, 1.0) context vs its "production-exact" doc claim — align the doc or thread the real params. Small chip.
- [low] bastion-model efficiency nits ×2 — Fable queues its own follow-up commit (bastion-model/** only).
- NOTE: the REAL cloud swarm = `claude ultrareview` run from a TERMINAL (not inside a session) — available to Ben any time; both local max-effort passes are done and the delta HOLDS.
