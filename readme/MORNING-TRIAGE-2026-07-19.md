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
| C1 | **B54 metrics wall-clock chip** — `server/src/metrics.rs:404-406` `.expect("Time went backwards")` → fall back instead of panic (the `bastion_flight_recorder.rs:637-640` `.ok()` idiom). LOW/vanilla/startup-only. | chip | P3 | `BASTION_COMMON_ISSUES.md` row B54 (CASE-009 sweep) |
| C2 | **F2/F3 fixture-hardening chips** (stimulus-window family) — Fable's capstone-review dispositions, ruled chips by the architect. | chips | P3 | `readme/BUILD_REVIEW_LOG.md` §FABLE-004 (dispositions ¶) |
| C3 | **The metrics nit** — flagged tonight alongside the chip list; compile-time pointer uncertain (distinct from C1 per the architect's own listing). Confirm with architect what it names, then chip or drop. | architect confirm → chip | P3 | (no doc pointer at compile time — HONEST-PARTIAL) |

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

*HONEST-PARTIAL disclosures: C3 (the "metrics nit") could not be resolved to a pointer before the cutoff;
A2's ultra-review had not returned by compile time. Everything else is pointer-verified against the docs/code
as of ~23:50.*
