# Fable Handoff Index — Opus-reviewed code blocks, indexed for the apex tier

**Owner:** Sonnet reviewer (this file), updated in the same pass as `docs/BASTION_RUN_LOG.md` /
`readme/BASTION_RESTORE_LEDGER.md` bookkeeping, every time the Opus Build-Reviewer reports a verdict.
**Purpose:** `readme/BUILD_REVIEW_LOG.md` is the narrative log — full reasoning, prose, evidence-as-written.
This file is the LOOKUP LAYER on top of it: one row per Opus-reviewed block, so the Fable Reviewer
(`readme/FABLE-REVIEWER-prompt.md`) can find, for any block, (a) exactly which code it covers, (b) what
Opus already verified and how, and (c) the literal pins/fixtures/commands that re-prove it — without
re-deriving any of it from scratch. Fable still verifies from actual code per its own charter (adversarial,
proof-first); this index exists so that verification starts from a precise map, not a cold read of 1400+
lines of narrative log.

**How to use this file (Fable):** find the block → read the "Code blocks" + "Verdict" columns → if you need
the full reasoning, jump to the cited `§`-anchor in `BUILD_REVIEW_LOG.md` → to re-verify, run the commands
in "Test/fixture coverage" directly. The "Open/tracked" column is where a fresh adversarial pass has the
best chance of finding something real — those are the residuals Opus itself flagged as not fully closed.

**Scope of this seed pass (2026-07-21):** rows go back to R10 (the first full GATE-tier review, not the
routine per-commit R1-R9 passes — those stay in `BUILD_REVIEW_LOG.md` only, pull them directly if a
pre-R10 block is ever in question). Everything from ENGOPT4 onward is listed in the PENDING section below
until Opus's own attested backfill lands (see that section for why).

---

## Reviewed & logged (Opus verdict on record in `BUILD_REVIEW_LOG.md`)

| Block (tag) | Commit | Code blocks reviewed | Opus verdict | Test/fixture coverage (run these) | Open/tracked | Full entry |
|---|---|---|---|---|---|---|
| R10 | `508c8136cb` | Fence mechanism: `authority_valid` exact-match fence, `fenced_movement_write`, `advance_epoch` coupled to `retire_traversal_task` | PASS w/ 1 condition (corpus x2 same-seed byte-consistency before M3 — condition cleared, see M3 row) | N-FENCE proof (local+VM); regression baseline 14/14 consistent (no-op property); RECs 1-5 | Condition satisfied by M3's own gate | `§R10` |
| M3 | `8c4543094a` | `bastion_jobs.rs`: `advance_epoch` call sites (`:3693`,`:4213`), sole removal site (`:4211`), 13 fenced movement writers (`5580-6175`), corridor pre-reservation writers (`5001/5133/6620/6668/8839`); `bastion_traversal.rs:59` `fenced_movement_write` | PASS (2 tracked follow-ups, none blocking) | Pin `r10_retirement_is_sole_removal` (runtime-built needles via `include_str!`: `.remove`==1, `retain/drain/clear/swap_remove`==0, `advance_epoch`==2); `fenced_movement_write` count pin ==13; `emergency_route_members.insert`==3; N2 green 13x; M3A/M3D 24-run matrix (owned-conflicts==0/24, SOFT-0==0); x3 byte-identical | (1) corridor task-less invariant was behavioral not by-construction — **closed by B58's debug_assert rider**. (2) frontier-approach corridor-unification — **closed by B58**. (3) M3B 2x2-shaft-vs-1-wide geometry deviation — documented, not urgent, still open | `§M3` |
| CRATE-SPLIT | `6357c35d23` | `server/src/bastion_*.rs` → new `bastion-server` leaf crate (8 files pure move; `bastion_jobs.rs` 74 `pub(crate)`→`pub` visibility edits; `sys/agent`'s `traversal_config_for` moved) | PASS | `cargo test -p bastion-server --lib` 35/35 (includes the R10 retirement/fence-exhaustiveness pins + M3 queue pins, now living in the new crate); `veloren-server --lib` 11/11; byte-identity @1337 pre/post ×2 (dig-access identical; mine-fidelity canonicalized-identical) | `mf_per_colonist` array-order variance — pre-existing HashMap-iteration artifact, NOT a split regression, already queued in the determinism sweep (**since resolved, see ENGOPT6 row in PENDING**) | `§CRATE-SPLIT` |
| B58 | `8e0e3bc03d` | `bastion-server/src/bastion_jobs.rs` (+140, corridor-unification logic) + `bastion-harness/src/main.rs` (+215, a2/a3 fixture hardening); `bastion_traversal.rs` (fence file) UNTOUCHED | PASS | Pins unchanged (`remove`==1, `advance_epoch`==2, `insert`==3, `fenced_movement_write`==13); new debug_assert rider (task-less corridor drive, by-construction now); self-gate M3A/M3D/N2 @1337 ALL PASS w/ rider live; 35/35 lib tests; attest clean @`8e0e3bc03d` | Residual M3A@21/42 red RE-CLASSIFIED (not B58's bug) → filed as **ORGANIC-CLIMB-BOUNCE ESCALATION STARVATION** (stuck-economy/R11/FR15 territory, routed to architect, needs a designed FR15 paired-A/B, not an overnight fix) — **still open as of this seed pass, worth an adversarial look if idle** | `§B58`, `§FILED (B58-derived)` |
| ENGINE-OPT-1 | `115cd34e54` | `common/src/astar.rs` (FxHasher64 deterministic tie-break key + `closest_node`/empty-path fallback fixes) + harness fixture diag | PASS (M3A red classified, leak flagged as immediate next block) | Falsifier: shuffle-neighbor-order determinism test, RED on seq-only tie-break → GREEN on the shipped key (real falsifier, precondition asserted); self-repro byte-identical pairs; M3A@1337 rc=1 classified — teleports 0, deliveries improved, sole red = fork-15 vanilla-climb leak (pre-existing, exposed not caused) | fork-15 leak fix — **closed, see ENGOPT1's own next-block landing** (not separately logged in BUILD_REVIEW_LOG; verify via run-log if Fable needs it) | `§ENGINE-OPT-1` |
| ENGINE-OPT-2 | `623fc58f01` | `common/src/astar.rs` (decrease-key/reopen via lazy deletion) + harness A/B diag | PASS (SHIP classified; retune companion tracked as `DECISIONS-FOR-BEN.md` #23) | FR15 paired A/B; both falsifiers RED on emulated-old-mechanism → GREEN; optimality vs Bellman-Ford; M3/N2 safety untouched; ENGOPT1's FxHash frontier key confirmed intact (reopen didn't reintroduce non-determinism) | **POST-SHIP CORRECTION (same day):** the "cross-machine determinism preserved" claim was WITHDRAWN — mine-fidelity cross-machine pair diverged (20 fields) despite within-machine byte-identical. Within-machine/unit determinism, optimality, and M3/N2 safety all stood; ship was not reversed. **This exact seam is the one ENGOPT6 later root-caused and closed (merge-topology UID-sort) — treat as RESOLVED, see PENDING section** | `§ENGINE-OPT-2`, `§ENGINE-OPT-2 — POST-SHIP CORRECTION` |
| ENGINE-OPT-3 | `695bbb0172` | `server/agent/src/action_nodes.rs` (`choose_target` loot arm) + `common/src/comp/loot_owner.rs` (`can_pickup` + first-ever tests for it) | PASS | Truth-table + verbatim-old-predicate falsifier 2/2; `can_pickup` 1/1; commit-gate scan 1/1 (rc=0, local); VM fan `FAN=eo3` 3 jobs ALL ATTESTED @`695bbb01`; mf undisturbed; M3A classified red preserved to the byte; N2 PASS | None blocking | `§ENGINE-OPT-3` |
| §FABLE-005 *(worked example, not a Sonnet/Opus entry — kept for format reference)* | delta `2880f341d6..HEAD` (pre-tag) | Row-51.7 extraction, instrumentation commit, Codex determinism merge (4 commits), `bastion-model` crate (deepest pass — never independently reviewed before) | Post-tag delta HOLDS (ReportFindings: 5 findings, none tag-blocking, none safety-path — 2 PLAUSIBLE correctness/determinism-fragility notes worth a look, 3 CONFIRMED-low efficiency/test-fidelity nits) | Local max-effort ultra fallback (cloud swarm unavailable) — adversarial-verified 4 candidate claims down to refuted-with-line-proof; corroborated by the existing 8/8 tape verification | 2 PLAUSIBLE findings still open: `common/src/rtsim.rs:283` exact-float endpoint match (tolerance-inversion risk), `trade_pricing.rs:1047` legacy `random_items` still thread-RNG | `§FABLE-005` |

---

## PENDING — awaiting Opus's attested backfill (architect directive, 2026-07-21)

The architect made **VM-every-change** a hard requirement with **mandatory ATTEST lines** (SHA-matched, not
a "re-ran, looks fine" claim) on 2026-07-21, and the Opus Build-Reviewer is currently backfilling attested
verification across every block landed since ENGINE-OPT-3 — this is in progress as of this seed pass
(`local_7e72649b`, session running). **Do not treat the absence of a row below as "unreviewed forever" — it
means the Opus pass hasn't reported back yet.** I will add a full index row (same columns as above) for each
of these the moment Opus reports a verdict, in the same pass as the run-log/ledger bookkeeping.

Until then, the best available source for each is my own bookkeeping (mechanical status + self-gate results,
NOT an independent code review) in `docs/BASTION_RUN_LOG.md` and the corresponding row in
`readme/BASTION_RESTORE_LEDGER.md`:

| Block (tag) | Commit | What it is | Where to read the interim (Sonnet-only) record |
|---|---|---|---|
| ENGOPT4 | (SlowJobPool/ARCH-003 — merge-topology groundwork) | Async job-pool determinism substrate | run-log / ledger, ENGOPT4-era entries |
| ENGOPT6 | `781a553eb71e` | Agent-layer determinism residual, ROOT-CAUSED + CLOSED — the actual ENGOPT2 cross-machine seam (entity-ID join order vs stable UIDs during a mass-merge burst), cured via ENGOPT4's sorted-apply pattern. END-PROOF: tapes8 byte-identical across 2 VMs (36,059 trajectory + 24,726 event blocks, zero divergence) | `BASTION_RESTORE_LEDGER.md` row `bastion-block-ENGOPT6`; run-log ENGOPT6 hunt narrative |
| ENGOPT7-REVERT-183 | `daaf8aba45` | Pure revert of ledger #183 (a stuck-economy-constraint-class regression caught by `floor6`) — not itself reviewed as new logic, it's a revert | `BASTION_RESTORE_LEDGER.md` row `bastion-block-ENGOPT7-REVERT-183` |
| CTRLFRAME | `9b3c6850ac3e` | T0-002 group: declarative phase manifest, order contracts, Controller frames + tagged envelope (the namesake), all-builds topology check | `BASTION_RESTORE_LEDGER.md` row `bastion-block-CTRLFRAME` |
| T0DET3 | `96315c8fbf85` | T0-003 group: stamped event bus, keyed-RNG family (ChaCha8, 2 latent OS-entropy seams closed → registry B75), claim/need-target total order, `ItemInstanceId` | `BASTION_RESTORE_LEDGER.md` row `bastion-block-T0DET3` |
| T0DET4 | `a1130b1c5793` | T0-004 group (Tier 0 CLOSE): async ownership substrate, `--deterministic-parallel` serial-vs-parallel proof, canonical domain-hash/Merkle/`FinalStateCertificate`, causal recorder, Bastion schedule fuzzer. **Capstone: 3-machine × multi-schedule byte-identical `FinalStateCertificate`** | `BASTION_RESTORE_LEDGER.md` row `bastion-block-T0DET4` |
| T1CMD | `d319508dacb6` | T1-001 group: command/commit/capability protocols, `CommandReceipt` admission, `effect_journal`, `conservation_saga`. **T1.6 = a real live bug fix** (`execute_character_edit` was committing on failed edits). Live wire-in through the haul path proven pure-refactor (certificate byte-identical pre/post) | `BASTION_RESTORE_LEDGER.md` row `bastion-block-T1CMD` |
| T2 substrate cluster | `b901288b..926d5db3..90fb70e6` (T2.1/4/7/8/9/13/16/17/19/21) + `t2_2` pin (T2.2, 4-state `SimulationMode` cycle) | Reuse-first RTSim↔ECS lifecycle formalization; T2.2 confirmed pure-refactor on 3 axes (recorder mode-blind, single exhaustive match site, no live-transition activation yet) | `docs/BASTION_RUN_LOG.md` T2 narrative (extensive — includes the T2.2 ruling, the self-caught Tier-1-skip process error, and the architect's endorsed correction plan) |

**Standing note for whoever reviews next:** per the architect's 2026-07-21 directive, any block above that
turns up a real red on the attested VM re-run is a **safety-red**, not routine — it gets flagged immediately,
not batched.

---

## Maintenance

- Add a row to the top table the same pass I bookkeep a tag whose Opus verdict has landed (run-log + ledger +
  this file, one pass).
- Move a row from PENDING → the top table the moment Opus's attested backfill reports on it.
- Keep rows LEAN — this is a lookup index, not a duplicate of the narrative log. If a cell would need more
  than ~3 lines, summarize and point at the `§`-anchor instead.
- If a "PASS" row's own follow-up later gets closed or superseded (like M3's follow-ups closing via B58, or
  ENGOPT2's cross-machine gap closing via ENGOPT6), update the cell in place rather than leaving it stale —
  a follow-up marked open after it's actually closed is exactly the kind of noise that wastes Fable's budget.
