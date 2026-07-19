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
- [fidelity-audit addendum, author-disclosed]: 3rd focus item for the bastion-model audit — the model counts EVERY classified abort into the shared reengage counter; verify that matches the ENGINE's counter-sharing exactly (not just the route-exhausted-replan leg). (Plus the watchdog abstraction + the two honest gaps, per FABLE's stand-down note.)
- [A] Chip B50/C2 (--data-dir seed-stamp): COMPLETE-pending-merge-review (worktree clever-mclaren; 3 hunks ~35 lines, harness main.rs only; 3 verification legs green incl cross-seed pre-boot hard-error). Line-review at the morning merge sweep.
- [D] NEW REGISTRY CLASS candidate (Sonnet files): worktree-session ABSOLUTE-PATH edits landing in the SHARED checkout (B50 chip near-miss, self-caught pre-build via git diff --stat, surgically reverted zero-trace). Rule: worktree sessions use worktree-relative paths; tripwire = git diff --stat the shared tree before any build.

## Night-END state (architect, builder stood down)
- [DONE] B16 Tier-2 checked_tick VERIFIED + committed (bedb167540). tick_guard_tests 2/2; ~15 ticking-state callers closed at one reader site. Row can flip DONE.
- [★ MORNING BLOCKER] The SHARED main tree's working copy has uncommitted WIP that BREAKS `cargo test -p bastion-harness` (E0425 `MUSHROOM` not in scope, main.rs:~10192, inside ~399 lines of a session's in-flight edit). Chips in isolated worktrees are unaffected, but any SHARED-tree harness build/test fails until this is reconciled. FIRST morning action before the merge sweep: identify the owner + commit-or-stash the dirt (the shared-checkout-collision protocol). The determinism_regression suite property is already proven 4× by the isolated runner, so no correctness gap — this is a compile-hygiene blocker only.
- [BEN ACTION] R10 = HANDED OFF (builder's honest context call). Spawn the fresh session from readme/R10-FRESH-SESSION-CHARTER.md — this is the only path to overnight/critical-path build progress; without it, all building is a morning start.
- OVERNIGHT REALITY: main builder stood down after checked_tick; no active build lane remains. STATUS-SURFACE, the 5 chips, builder-2 leash, Codex DET-0004 all → morning merge sweep (serial, after the MUSHROOM dirt is cleared).

## MAIN BUILDER — overnight queue progress (as of ~00:45)
- **STEP 1 DONE (pre-queue; messages crossed):** B16 checked_tick reader guard COMMITTED
  `bedb167540` (utils.rs only; cargo check -p veloren-common rc=0; tick_guard_tests 2/2).
  Residual `determinism_regression::tests` line ANSWERED: rc=101 E0425 `MUSHROOM`
  bastion-harness/src/main.rs:10192 = an ADDED line inside ~399 lines of ANOTHER session's
  uncommitted main.rs WIP (file was clean at session start; HEAD's bin built+ran green in the
  merge gate) → NOT a merge regression; item-2 verdict stands FULLY GREEN on the direct
  process-isolated runner proof (4× DETERMINISM OK). ⚠ MORNING: whoever owns the main.rs WIP
  currently breaks `cargo test -p bastion-harness` for every lane.
- **STEP 2:** R10 = HAND OFF (call made + restated to architect; fresh-spawn charter stands:
  scratchpad r10-implementation-plan.md + readme/R10-FRESH-SESSION-CHARTER.md).
- **STEP 3 IN PROGRESS — STATUS-SURFACE:** all 6 files EDITED (comp/bastion.rs
  BastionColonistStatus enum + payload tail energy/status; bastion_jobs.rs display-only
  TTL-stamped status map [no clear sites — expiry IS the wait ending] + pure classifier
  colonist_status_display shared by wire fill AND harness probe + write sites at the
  QueuedForLink arm and the GRANTED energy-wait hold [denied hold falls through to
  Replanning — honest]; in_game.rs SystemData +Energy+Tick + fill; lib.rs
  bastion_colonist_status probe; session/mod.rs status line under the name + Energy in the
  meters row). Verification BOX-BLOCKED at write time:
- ⚠ **BOX CONTENTION 00:31:** Bug-Tester lane (session 0554684d…, bughunt/gate.ps1) launched
  `cargo build --release -p bastion-harness` against the MAIN TREE mid-my-edit-window. Their
  gate binary ingests my uncommitted edits + the broken main.rs dirt (E0425 → the build should
  fail there) — tonight's bughunt evidence is provenance-unclean in either outcome (class-8
  shape). Architect flagged [dedupe status-surface-box-contention]. Also: rustc now runs
  through a LIVE sccache server (up since 21:40) — #17's install happened somewhere.
- **NEXT on box-free:** check server→voxygen → unit test → boundary commit → isolated-worktree
  harness evidence (P0 tape byte-diff sim-inert + N1B probe RestingToClimb observation;
  worktree-local main.rs probe patch ONLY — committing shared main.rs under live third-party
  WIP invites the silent-drop hazard).

## INCIDENT (architect, ~04:40) — unsanctioned bug-hunt build + evidence quarantine
- The adversarial bug-hunt (local_c9064dd4) launched `cargo build --release -p bastion-harness` against the MAIN tree at 00:31 — violating its READ-ONLY/NO-COMPILE charter AND the overnight one-build-lane rule. Session went idle after; build ran ownerless (2 cargo procs, no OOM risk, not killable — no pattern-kill).
- CONSEQUENCE: it compiled a DIRTY tree (main builder's uncommitted STATUS-SURFACE + the third-party MUSHROOM break) → tonight's bug-hunt gate evidence is PROVENANCE-UNCLEAN (dirty-tree artifact trap). DISCARD any finding from it; re-run from a CLEAN pre-built binary if the bug-hunt is repeated. Correction messaged to the lane.
- REINFORCE at next bug-hunt spawn: read-only, no-compile, pre-built isolated binary only, never the shared tree.
- (Benign note: sccache is up since 21:40 — #17 install completed by some lane.)

## INCIDENT CORRECTION (architect, ~04:48 — retracting the over-correction above)
The prior INCIDENT note over-reached; the bug-hunt reconciled it factually. Corrected record:
- Part-1 70-run bug-hunt evidence = PROVENANCE-CLEAN (prebuilt exe 7f087da317, ZERO compiles, hunt closed before any cargo). NOT discarded — it STANDS. The "discard" applied ONLY to leash-gate evidence, now downgraded to PROVISIONAL (below).
- The E0425 MUSHROOM was the bug-hunt's OWN fixture miss, self-caught at its 00:20 typecheck (no binary) and fixed; the 00:31 build is check-green — NO compile-break carried. The main builder read stale pre-fix state.
- The 00:31 build was WITHIN a compile authority the ARCHITECT had granted (Part-2 leash-gate re-tasking + box-check + pileup exempt-lane ruling). NOT a charter breach. The real miss was the ARCHITECT's: declared the main builder the "single overnight lane" without accounting for the bug-hunt's already-authorized in-flight leash gate = a coordination error on my side, not the lane's.
- LEASH-GATE EVIDENCE DISPOSITION = provisional/directional (dirty-tree gate, the shared-checkout norm); re-gate on a clean committed tree for tag-grade once STATUS-SURFACE + the other WIP land. File flagged-provisional, do not discard.
- STANDING RULE (one): adversarial HUNT = read-only/no-compile/prebuilt-isolated; explicit GATE re-tasking = build lane under box-discipline. Different tasks.

## Leash re-gate DEFERRED to morning (torn-tree, ~04:52)
- The bug-hunt's 00:31 leash build FAILED (exit 101): `BastionColonistStatus` not found — STATUS-SURFACE's server half (bastion_jobs/lib.rs) was on disk before its common half (comp/bastion.rs enum). A mid-edit torn-tree snapshot, not a code bug. EMPIRICAL upshot: NO gate binary was producible tonight → there is NO leash evidence to adjudicate; the earlier provenance debate is moot.
- MORNING MERGE-SWEEP SEQUENCE (order matters): (1) main builder commits STATUS-SURFACE; (2) builder-2 leash commits (Opus clean-billed); (3) bug-hunt re-gates the leash on the clean committed tree = tag-grade in one pass. Bug-hunt stood down till then; nothing on the box from it.
- ROOT: two lanes' uncommitted work co-resident in one shared checkout kept tripping each other's builds all night (shared-checkout collision class). The morning sweep's job #0 is to serialize these commits before anything gates.
