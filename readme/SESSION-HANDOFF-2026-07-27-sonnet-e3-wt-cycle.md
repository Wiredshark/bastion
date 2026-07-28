# Handoff: E3-WT / E3-W / E3-W2 (Sonnet 5, engine-improvement lane)

Branch: `bastion/engine2`, worktree `E:\veloren-master\.engine2-wt`. Landed
through `e58852ad7b` (T3.53 deterministic-reentry fix), pushed, verified.
E2 (T3.54/T3.58/T1.114/T1.107) and E3 behavior rows (T3.34/T3.27/T3.35+39/
T3.52/T3.52b/T3.53+fix) all landed, tested (real exit codes, correct
worktree each time), pushed. Opus cross-review of this batch is queued by
Fable — no action needed unless findings return.

Verify floor: `cargo check -p <crate> --all-targets` or a real `cargo test`
minimum after any merge/batch — plain `cargo check` never compiles
`#[cfg(test)]`. `DigestDomainIdV1` block allocation: 21-39 = engine lane
(mine), 40-99 = APEX lane (Opus's), ≤20 frozen.

## E3-WT (next, dedicated block — ratified)

Wire `common::threat_policy::{ThreatClassV1, ThreatCandidateV1, arbitrate}`
(already built+tested, `8293d2d2f0`) into live target selection at two
sites. Non-vacuity test per site + existing-suite rail both crates required.
Disclose any old-vs-new pick delta the ruling didn't anticipate.

**Server-agent half — START HERE (data-rich):**
- Site: `server/agent/src/action_nodes.rs::choose_target` +
  `server/src/sys/agent/behavior_tree/mod.rs::target_if_attacked`.
- `target_if_attacked` already reads `health.last_change` (exact attacker
  uid + exact recency, `DAMAGE_MEMORY_DURATION`-gated) and calls an
  EXISTING comparator `is_more_dangerous_than_target` — **read that
  function's body FIRST** (not yet read). Decide absorb-vs-replace: if it
  already encodes part of the ruled comparison (proximity/capability/
  recency), `threat_policy::arbitrate` should ABSORB that logic and the old
  comparator should delegate to it or retire — disclose the decision either
  way, don't duplicate judgment across two comparators.

**rtsim half — CONDITIONAL, check before building:**
- Site: `rtsim/src/rule/npc_ai/mod.rs::check_for_enemies` (current: `.min()`
  over nearby `Sentiment::ENEMY` actors — DET-AIT-004 canonical-Actor-order
  tiebreak, **not proximity**, confirmed by reading the function directly).
- rtsim's `NpcCtx` here only has a static `Sentiment::ENEMY` relationship +
  position — no per-actor engagement/recency tracking, so
  AttackingMe/AttackingAlly can't be honestly discriminated. Ruling:
  AttackingMe/AttackingAlly tiers live ONLY where the data lives
  (server-agent) until rtsim grows engagement tracking — not a compromise.
  - If `.min()`'s current key is confirmed NOT proximity (already true per
    above): wire the HONEST degraded projection — fixed class
    `HostileNearby`, real proximity+capability score — and disclose the
    collapse from 3 classes to 1.
  - If on closer look current selection is ALREADY effectively proximity
    and the projection would add nothing real: DEFER the rtsim half with a
    one-line note instead of shipping a fake upgrade.

## E3-W (after E3-WT)

T3.27-only: migrate `rtsim::ai::{Consider, Tree}`'s internal storage (today
a bare `u32` priority, shared by every `.urgent()`/`.important()`/
`.casual()` call site across `villager`/`humanoid`) to carry
`action_policy::{ActionClassV1, ActionCandidateV1}` scoring. Live-path exit
test required: comparator provably drives a real NPC decision, not just
unit tests on the comparator (the exact gate-must-test-live-path bar this
whole batch has been held to). Confirmed via E3-WT recon: threat-policy
wiring does NOT touch `Consider`/`Tree` at all (`check_for_enemies`/
`choose_target` are plain functions, not `Consider` closures) — E3-W is
purely T3.27's concern, no shared migration with E3-WT.

## E3-W2 (E3 tail, after E3-W)

Full `villager()` (rtsim/src/rule/npc_ai/mod.rs:885+) migration off the
"sticky first-wins" `Consider::action` pattern. Confirmed live bug: multiple
`consider.important(...)` calls fire in sequence at the same tier
(migrate-home, dark→seek-house, rain→seek-shelter, more below) — first
whose condition is true wins by DECLARATION ORDER, not actual urgency (e.g.
today, dark+raining always picks "go home" over "seek shelter" regardless
of which is worse). **CHARACTERIZATION-FIRST, mandatory**: land tests
capturing `villager()`'s CURRENT emergent decisions across the interesting
condition matrix (dark/rain/hungry/etc. combos) BEFORE migrating anything,
so the migration's behavior diff is explicit and reviewable, never blind.
`humanoid()` is structurally immune (a single exhaustive if/else chain,
never registers competing candidates) — not part of this row.

## Standing token protocol (Ben-directed, effective this cycle)

- Tag every report FYI or RULING-NEEDED. FYI: no reply expected, assume
  accepted unless objected to by next report — don't wait, don't re-state.
- RULING-NEEDED: one focused question + your recommended default. Silence
  >10min → proceed on stated default, mark PROCEEDED-ON-DEFAULT.
- Never re-state prior context after a message-cross; one line max to
  disambiguate.
- Report format cap: outcome, commit, disclosures, next — no process
  narrative.
