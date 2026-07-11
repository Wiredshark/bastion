# BASTION COMMON ISSUES — the SHARED game-side registry + pre-flight checklist

**Ben's mandate (2026-07-10):** keep ONE common list of the issues that recur — **bugs, design issues,
performance issues, gameplay-loop issues** — plus the **common questions that should always be asked** — curated
by the **Build-Reviewer** and contributed to by **every agent**. This is the game-code/design counterpart to the
asset team's `ASSET_COMMON_MISTAKES.md` (which stays the authority for asset/voxel issues). A living checklist so
the same class of problem stops recurring.

## Protocol (all agents)
- **CONSULT before/while working** — builder pre-implementation + at gate; designer while spec'ing; reviewer as
  the code-review AND feasibility checklist; play-tester while driving.
- **APPEND when a new class appears or an old one recurs** — with HOW TO DETECT it and the FIX/RULE. Every real
  bug Ben, the play-tester, or the reviewer finds should leave a class here so it can't hide next time.
- **REVIEWER curates** (dedupes, keeps it scannable, promotes recurring one-offs into classes). Architect folds
  in Ben's findings + cross-agent patterns.
- Companion: `ASSET_COMMON_MISTAKES.md` (assets), `BASTION_ARCHITECTURE.md` (system map), `BUILD_REVIEW_LOG.md`
  (per-review findings), `RESEARCH-IMPROVEMENT-LOG.md` (method upgrades).

---
## A. BUGS (correctness) — seeded from real finds
| # | Class | How to detect | Fix / rule |
|---|---|---|---|
| B1 | **Off-by-one in a reach/boundary predicate** (annulus "standable = rise ≤ reach" was really ≤ reach−1 — entombed a novice for WEEKS as a "flake") | boundary unit test on the PURE function at exactly reach==rise; scenario flakes that come/go | Pin every ±1 predicate with a direct `#[cfg(test)]` boundary test; a pure function must not be tested only through a flaky scenario |
| B2 | **Guard short-circuits BEFORE the real check** (churn "anchored" guard used pure PROXIMITY ≤8, skipped egress_scan → colonist walled off from a near-but-unreachable anchor never rescued) | ask "does this guard run the authoritative check, or a cheap proxy that can lie?" | Run the authoritative check (egress_scan/reachability) before trusting a proximity/count proxy |
| B3 | **One stuck item freezes a global one-at-a-time economy** (a single unreachable is_access job froze colony-wide rescue; unreachable jobs never pruned) | any global `any(...)`/one-plan gate + no staleness prune; two simultaneous instances compound | Staleness timeout (abandon+clear if no progress in N) or region-scope the gate; never let one failure wedge the whole economy |
| B4 | **Admin/debug-gated command inert in singleplayer** (time controls wired to admin `/time_scale` → buttons did nothing) | test the feature as a NON-admin singleplayer player, not just via the console | Wire player-facing verbs to the real setter/resource, not an admin command path |
| B5 | **Sprite-adjacency violation → silent one-tick vacate** (placed sprite vanishes; engine runs an adjacency sweep every terrain change) | consult `common/src/terrain/sprite/mod.rs::adjacency_requirement`; runtime persistence assert | Respect each sprite's attachment rule (standing needs solid below, wall/ceiling variants, ±z) — also fires on LATER mining near the sprite |
| B6 | **A*-walkable but not capsule-passable** (2-high opening → colonist wedges in window) | pathing succeeds but the agent physically stalls at the gap | Openings must fit the colonist capsule (height + width), not just the path graph; bar 2-high slits |
| B7 | **Serde-default / wire / save-compat break on a new field** | load an OLD save / cross-version; missing `#[serde(default)]` | New fields serde-default + note the wire/save caveat in the ledger; ship the pair |
| B8 | **Determinism break** (timing-dependent assert; A* reset-on-move; RNG order) | run the gate on a QUIET machine + repeat; flake under load | Assert on deterministic state, not wall-clock; seed + fix iteration order |
| B9 | **Panic/unwrap/overflow path** on unexpected state | grep unwrap/expect/indexing on runtime-derived data | Handle the None/err/empty + bounds; a colony sim runs millions of ticks — rare = certain |
| B15 | **Claimability/exposure gate admits UNSTANDABLE work** (Mine exposure = "any non-filled neighbor" but NOT "a colonist can STAND adjacent" → floating blocks + hillside `+1`-arrival-gap cells get claimed, never workable → cycle claim→unreachable; slope-mining "gives up" with blocks left, historically 8/200) | count completed vs `unreachable`-marked vs claimed-never-worked on a NATURAL slope (not a flattened fixture — fixtures terraform this away, so gates miss it); floating/arrival-gap remnants = unreachable cluster | Gate claimability on STANDABILITY (a reachable standable adjacent cell), not just a non-filled neighbor; for produced floaters, mine-in-place-from-adjacent or collapse/fall. Sibling of B6 (A*-walkable ≠ capsule-passable). Natural-slope geometry must be in the test matrix, not flattened away |
| B14 | **Reset-prone accumulator — the reset races/starves the threshold** (an accumulator that gates an action at threshold T is zeroed by SUB-threshold events, so it never reaches T and the action never fires: `stuck_time` zeroed by sub-block jitter → the watchdog never fires and every net below it goes blind [R3, confirmed THE root cause of the 20+-iter hover]; `churn.1` reset to 0 at cycle 8 → the teleport at cycle 16 is UNREACHABLE dead code [R5-F5]) | for any accumulator gating an action at a threshold: what RESETS it, and can the reset fire faster than it accumulates below T (or between a lower and a higher threshold)? trace the sawtooth | Reset ONLY on genuine progress/success (hysteresis: `stuck_time` zeros on ≥1 block NET progress, not any wiggle; a lower-threshold action resets the counter ONLY when it actually dispatched, so a higher threshold stays reachable). A dead net that comments present as active is worse than none — it reads as covered |
| B13 | **Implicit loaded-chunk assumption broken by a top-down/remote trigger** (an edit/op that's safe when a PRESENT player triggers it — player presence guarantees the target chunk is resident — silently drops when a top-down god-place / remote command / LOD op targets an UNLOADED chunk: `BlockChange::try_set` on a non-resident chunk is discarded on apply, no error) | who triggers this — a present actor (chunk loaded) or a top-down/remote one (chunk maybe unloaded)? does the handler verify the target chunk is loaded? | Any top-down/remote/god op on world state must verify the target chunk is LOADED (or load-then-edit), not inherit the native path's implicit "the player is standing there" locality. Ties the LOD seam (only loaded regions are editable/pathable; unloaded → the abstract tier) |
| B12 | **Real-time-anchored rate silently coupled to the timescale/day-length knob; TimeOfDay↔DeltaTime desync under the frame clamp** (a `dt.0`-based rate like `WORK_DURATION_BASE` is anchored to REAL seconds, so tuning `day_length` silently retunes "per game-day"; and `MAX_DELTA_TIME` clamps DeltaTime but NOT TimeOfDay, so at high TimeScale + low fps the calendar leads the sim → day-anchored vs dt-anchored rates diverge) | grep `dt.0 *`/`* dt`/`_SECS`/`_BASE` for rates; ask "is this SIM-PACING (per game-day) or ENGINE-timer (responsiveness)?"; does it stay put when `day_length` changes? | SIM-PACING rates: derive the dt-constant from a game-time spec × `day_cycle_coefficient` at load (stays physics-lockstep AND day-length-invariant) — do NOT rekey the mechanism to TimeOfDay-delta (it then leads physics under the clamp). ENGINE/watchdog timers (STUCK_TIMEOUT/grace): leave dt-based — they already scale with TimeScale and must not be day-length-anchored. Both clocks already ×time_scale (state.rs:884/892) |
| B11 | **Partial save-back loses loaded-tier mutations at the LOD seam** (the classic colony-sim persistence death: state mutated while an entity is Loaded is never written to its persistent home before unload/save — the rtsim sync writes only `npc.wpos`, so skill-XP/soft_until gained while loaded is lost on unload/save; promote restores from the now-stale record) | for EVERY field mutated while loaded: is there a save-back path to the persistent home before demote/save? unload→re-promote (or save→reload) and diff the state | Every loaded-tier mutation needs a save-back to the authoritative persistent record; drive it off change-tracking (FlaggedStorage) so it's cheap+complete, and flush before-save + on-demote. Conservation across the seam is the hard invariant (no dupe/loss) — distinct from B7 (that's serde/wire-compat; this is the sync being incomplete) |
| B10 | **Shared mutable state added under a PARALLEL join → silent determinism break** (a determinism that survives today ONLY because per-agent work is independent — e.g. the agent system's `.par_join()` at `server/src/sys/agent/mod.rs:76`; a future shared PathBudget/cache/counter the agents COMPETE for makes the outcome order-dependent under a nondeterministic join → two runs diverge) | ask "is new shared read/written INSIDE a par_join? was the parallelism safe only because entities were independent?" | Keep parallel work independent; if agents must share a budget/resource, allocate it order-INDEPENDENTLY — a sequential deterministic pre-pass (entity-id order) or lift the contended work OUT of the par_join into a sequential scheduler. Distinct from B8 (that's timing/RNG/assert-order; this is shared-state-under-parallelism) |

## B. DESIGN issues
| # | Class | How to detect | Fix / rule |
|---|---|---|---|
| D1 | **Re-implements a shared primitive** (a new dig/build/fell path that should call `carve_ramp`; a new search that should reuse `astar.rs`/`Chaser`) | code survey FIRST — does a primitive already do this? | Reuse-first; the fleet's best fixes were "it's carve_ramp again." One decomposer, N callers |
| D2 | **Two authorities write the same state / same id** (asset19 tool-gen clobbered asset36; duplicated invariant logic drifts) | two code paths mutating one resource/id | One authoritative owner per piece of state/id |
| D3 | **Mechanism designed too far ahead of its substrate** (speculative epics) | is the substrate at the frontier? | FRONTIER+1 — design just-ahead; feasibility-gate the hard ones |
| D4 | **Mechanic with no player-legible UI / feedback** (a verb the player can't see or trigger) | can a player SEE it work + invoke it? | Every mechanic needs a clear UI/feedback surface |
| D5 | **Escape-hatch / power with no cost** (emergency run with no penalty; a god-power with no favor cost) | is there a trade-off, or is it strictly-dominant? | Every strong option pays a cost; no free strictly-dominant verbs |
| D6 | **Drift from the architecture doc** (built ≠ documented invariant) | diff against BASTION_ARCHITECTURE + ledger consistency notes | Update the doc or fix the drift; flag consistency notes per tag |
| D7 | **Shared VOCABULARY not locked before emitters harden** (ChronicleEvent enum had ~10 kinds, corpus emits ~35; god-power catalog missing ~10 verbs — found in the gap-audit) | is this an enum/field that MANY systems write into? are all intended values enumerated? | Lock the FULL vocabulary early (like `Quality`/`Need`); a glyph/handler set follows the ONE lock, never forks per-system. Distinct from D1 (D1=reuse a primitive; D7=lock a schema) |
| D8 | **Asset-request skipped** (a design pass "done" without filing its assets → ~12 systems needed a gap-audit backfill) | does the pass have an ASSET_REQUESTS entry OR an explicit "asset-free" note? | Assets are a MANDATORY pass step — a pass isn't DONE until filed-or-noted-asset-free (and placed in the prioritized queue by build-proximity) |
| D9 | **Unbuilt dependency: crash vs graceful-degrade** (a system references a dep not yet built — DF-WOUND/DF-TAVERN — consumers must no-op gracefully, not crash/softlock) | what does this depend on that isn't built? do consumers degrade to a bounded self-resolving state? | DESIGNED-downstream tag + a graceful-degradation note for every unbuilt dep (a stub resolves the dangling ref); frontier+1 says write the full dep when ITS frontier nears |
| D10 | **4X / management-drift** (a system becomes MANDATORY management, or lets the god RUN the colony instead of influence it) | is zero-input / zero-policy STILL a complete healthy game? does the god TILT or COMMAND? | Autonomous-not-4X (the optional-thermostat guard) + influence-not-command (god tilts/blesses, never runs trades/moves units); the pillar guardrail |
| D11 | **OVERSTATED REUSE — a cited "reuse" symbol is a bare stub / unused / has no machinery** (feasibility class; the inverse of D1: a spec leans on `SharedChaser` for path-sharing, but it's `{nodes, goal}` with zero call sites, not even a registered Component — the NAME fits, the SUBSTANCE isn't there) | for every "reuse X" claim: grep X's call sites + read its body — does it have the machinery the plan needs, or just a fitting name? | Verify claimed reuse has real substance (call sites, logic, wiring) before pricing it as reuse; a fitting-named stub = build-from-scratch, re-rate the effort. The reviewer's job: confirm the reuse dividend is real |
| D12 | **Bastion feature MODIFIES a shared/vanilla struct instead of gating on `Colonist`** (adding a `portal_route` field to the shared `comp::Agent.chaser`, or any change to a struct all NPCs use, silently touches vanilla movement + wire/save for every entity) | does this edit a struct/system vanilla NPCs also use? is the new behavior gated on `comp::Colonist`? | Bastion behavior composes as Colonist-GATED wrappers/parallel components, never by mutating shared/vanilla structs — the ladder-waiver, soft-collision, tool_factor all gate on Colonist; vanilla stays byte-identical (the isolation invariant) |
| D15 | **Overloaded-BlockKind conflation — a naive kind-match hits semantically-different structures sharing the kind** (`BlockKind::Wood` is BOTH trees AND worldgen buildings/gnarling, so "chop every Wood" fells house walls; `Leaves` is mostly trees but also gnarling-moss/giant_tree — a kind-only selection can't tell them apart) | does the selection key off a BlockKind that worldgen reuses for unrelated structures? grep worldgen for the kind | Discriminate STRUCTURALLY, not by kind: seed from an authoritative oracle (`tree_valid_at`), connected-component with a must-contain-companion-kind gate + size-cap, and/or exclude cells inside a site's building PLOT. The colony's own builds are a distinct kind (Rock) — lean on that. Ties material-semantics (ASSET #16) |
| D14 | **Type-level "impossible by construction" invariant with a LEAKY variant** (the guarantee rests on the enum SHAPE — e.g. `PowerEffect` has no `Command` variant so a god-power "can't command a unit" — but another variant can express the forbidden semantics: `MindWrite`/`RtsimAct` writing `rtsim_controller.activity`/an ActiveJob IS a command in influence's clothing) | for every "X is impossible because the type has no Y variant" claim: audit each REMAINING variant's latitude — can any of them express Y's forbidden semantics? | The shape-invariant is necessary but not sufficient; enforce the semantic constraint INSIDE the permissive variants (MindWrite writes only mood/urgency the arbiter weighs, never activity/destination/job). Impossible-by-construction must cover every branch, not just the absent one |
| D13 | **Abstraction/LOD/culling FLATTENS the vertical deep** (our defining axis — a chunk-portal hierarchy that models only horizontal chunk-border portals mis-abstracts a ladder/shaft INSIDE one column; occlusion/camera/pathfinding all must respect z) | does this abstraction preserve intra-column vertical connectivity (ladder z10↔z20 in ONE chunk = cluster-internal, not a border portal)? does the deep survive it? | The vertical deep is load-bearing for a mining colony — every abstraction (nav hierarchy, occlusion, camera slice, LOD) must model vertical connectivity as a first-class edge, not collapse it to the surface plane |
| D16 | **Disabling/removing an access-PROVIDER silently starves a GATE that depended on it** (flagging `AUTO_LADDER_ACCESS` off removed the auto-access that ACCESS-BEFORE-DESCENT waits for → the gate then held every depth>2 cell UNMINEABLE wherever stairs don't fit → tight deep digs stalled at depth 2, 75/150 blocks; the provider and its consumer-gate were coupled but the flag touched only the provider) | when you flag-off / remove a PROVIDER (an access builder, a resource source, a job emitter), grep its CONSUMERS — who waits on / gates behind what it produced? does any gate now wait forever for something that can no longer be produced? | Audit a provider's consumers when you disable it: co-disable the dependent gate (tie both to the same flag) OR give the gate a fallback. Here: when no access is buildable AND the auto-provider is off, RELEASE the descent gate and rely on the universal teleport for egress (entombment stays impossible by construction — the redundant protective purpose is covered by a stronger backstop). A flag that silently half-breaks a downstream mechanism is worse than the bug it fixed |

## C. PERFORMANCE issues
| # | Class | How to detect | Fix / rule |
|---|---|---|---|
| P1 | **Unbounded per-agent compute, no per-tick budget** (per-agent A* that chokes as population grows) | cost scales super-linearly in N; frame time grows with colonists | Global per-tick budget + scheduler; reuse the incremental poll (PATH-0) |
| P2 | **Full recompute where incremental repair suffices** (re-search a whole path on any terrain change) | recompute cost on a small local mutation | Dirty-cluster + re-refine only the broken leg (hierarchy localizes the damage) |
| P3 | **Many-to-one hot goal not shared** (every colonist paths to the depot/dig-face independently) | N searches to 1 goal | Shared route / flow-field for hot common goals |
| P4 | **Watchdog/timeout burns cycles on a no-benefit path** (STUCK_TIMEOUT grace spent on TERRAIN-blocked stalls where soft-collision can't help; re-arms per re-claim) | timeout fires where the mechanism can't act; latency compounds | Gate the grace to cases it can actually resolve (density/cause-gated) |
| P5 | **Build/iteration cost** (28-min cold build; exe-relink lock on Windows) | RUSTC_WRAPPER unset; relinking a running voxygen | sccache wired; never relink the running exe; play-tester owns builds |
| P6 | **Wide regression window hides a perf regression** (500-tick part-(e) window would pass a 2× rescue-latency regression silently) | a lenient budget/window that a slowdown fits inside | Tighten windows to the real budget so a 2× regression fails loudly |

## D. GAMEPLAY-LOOP issues
| # | Class | How to detect | Fix / rule |
|---|---|---|---|
| G1 | **Loop doesn't close** (a verb with no payoff; a resource with no sink; produce with nothing to consume it) | trace the loop end-to-end — does output feed back into input? | Every loop closes: action → consequence → new choice |
| G2 | **Tedious friction vs fun friction** (micromanagement the autonomy should handle) | while playing, is the friction interesting or busywork? | Automate tedious friction; keep the interesting decisions |
| G3 | **Pace wrong** (instant dig felt cheap → needed deliberate pace + tool progression) | does the action feel weighty/rewarding, or trivial? | Deliberate pacing + progression (tools/skill) gives weight |
| G4 | **Autonomy-breaking stall** (a stuck/entombed colonist shatters the "it plays itself" fantasy — Ben's #1) | watch colonists over a long run; any stuck/idle-forever | The autonomy fantasy is sacred — a colonist must never be permanently stuck (ties B2/B3/G4) |
| G5 | **No feedback that an action worked** (time controls that don't visibly change anything) | does the player get immediate legible confirmation? | Visible, immediate feedback for every player action |
| G6 | **Missing emergent verb** ("I wish I could…" moments the rendered game begs for) | play-tester creative pass — what does the game WANT? | Surface as a suggestion → design lane; don't force, but notice |
| G7 | **Re-target churn without commitment hysteresis** (an agent re-selects its best target every arbitration cycle before completing the current one → visible in/out bob; mine-oscillation: 219 claims/150 jobs, claim cadence ~0.4s ≪ completion ~1.9s, reads as "running in and out of the mine") | count claims-vs-completions in a work window (ratio ≫1 = churn); claim cadence == arbitration interval; agent never commits long enough to finish | COMMITMENT HYSTERESIS — once claimed + making progress, don't re-select until Arrive+act or genuine fail; make staged access-anchor STICKY; dispersion/scoring biases INITIAL pick only, never re-targets a committed worker (the R3 hover-tail hysteresis applied to CLAIMS; ties the pathfinding-bob B-class) |
| G7 | **Write-only sink / no read-back** (a system RECORDS/emits but nothing CONSUMES the record — the chronicle before the reputation read-back; memory that affects nothing) | is the recorded/emitted data ever READ BACK into behaviour, or is it a dead log? | Every record needs a reader; close the write→read loop (deeds→standing→treatment — the 4-faces lesson). The sharpest, most common variant of G1 |
| G8 | **Keystone-dark** (a designed system can't close its loop because its keystone half is unbuilt — B7 needs gates the payoff of ~6 systems; the producer ships but nothing consumes) | what keystone (B7 needs / B8 defense) must exist for THIS loop to actually DO something? | Sequence the keystone first, or flag DARK-until-keystone; don't ship the producer half alone expecting a live loop |
| G9 | **Unbounded pressure / soak-law break** (a threat/night/climate/curse not scaled-to-colony + capped → attrition, not drama; a tended colony can't survive it) | does a TENDED, defended colony survive this UNTOUCHED (the Tier-1b soak law)? is it capped? | Bound every pressure to colony scale + cap it (prestige threat-pairing); drama not a nightly tax; the zero-input colony must endure |
| G10 | **Permanent softlock / unrecoverable loss** (a knowledge lost forever, a curse unliftable, a colony-death dead-end, a colonist un-stickable) | is there a REACHABLE recovery/relief for every loss/pressure? | Every loss is recoverable (re-learn / lift / appease / reclaim / un-stick); no permanent softlock. Composes with G4 (stuck) — the same rule at the loss layer |
| G11 | **Earned vs granted** (standing/alignment/reputation/knowledge set by a TOGGLE instead of earned by recorded deeds → kills the emergence + the B&W soul) | is this earned by PLAY (recorded deeds) or set by a switch? | Earned-by-deeds, never chosen; the drift IS the story (the hand's alignment, the epithet, reputation — all earned, reversible, legible) |
| G12 | **Self-correcting feedback that CONSUMES its own recovery labour** (the death-spiral trap: food-shortage raises Survive-urgency so colonists go EAT — but past a depth, EVERYONE spikes and nobody works the FARM jobs, so the "labour auto-shifts to food" recovery starves itself) | does the corrective feedback depend on a resource (labour/attention) the crisis itself drains? at what depth does recovery stop being possible? | Stagger the response (trait-modulation makes the hungriest/greediest spike at different thresholds so SOME keep producing) + gate must assert recovery in the RECOVERABLE band and graceful-degrade (not freeze) only PAST it — test a DEEP shortage, not a mild one. The E1 death-spiral criterion made concrete (composes with E1/G4/G10) |

---
## E. GENRE LESSONS — our genre = COLONY SIM × GOD GAME (from studying the field, 2026-07-10)
Reference titles: Dwarf Fortress, RimWorld, Oxygen Not Included, Going Medieval, Songs of Syx, Timberborn
(colony sim); Populous, Black & White, From Dust, Reus, WorldBox (god game).
| # | Lesson | Source | Apply to us |
|---|---|---|---|
| E1 | **Prevent DEATH SPIRALS — the player PLANS, doesn't FIREFIGHT** (autonomy that lets a colony recover; forced micromanagement kills the fantasy) | libcolony; DF/RimWorld | Autonomy-arbitration (drive scoring) must avoid unrecoverable spirals + minimize forced micro — the "it plays itself" promise IS this lesson. Ties G4/G10/D10 |
| E2 | **Emergent story via APOPHENIA — let the player's brain generate the narrative** (agents get freedom in HOW they execute; legible traits/moods/relations let players read stories in) | DF vs RimWorld storytelling | Agent-culture (stats/behaviour/history/relations/language) surfaces LEGIBLE traits + lets agents choose HOW, doesn't over-script outcomes — feeds object-inspection/dialogue UI + G11-earned drift |
| E3 | **INDIRECT CONTROL is the god-game pillar** (influence via environment/incentives/miracles; never command individual units) | Populous, B&W, From Dust | Never add a select-unit-and-order crutch; god-verbs shape terrain/incentives (hand, powers). Reinforces D10 (tilt-not-command) |
| E4 | **"Good enough" pathing is CORRECT, not a bug** (RimWorld deliberately picks good-enough paths to save CPU) | RimWorld routing | Validates PATH-0's budget: don't chase optimal; the invariant is "never stuck/entombed" (G4/B1-B3), NOT "shortest path." A suboptimal route is not a defect |
| E5 | **FLUID is a GOD VERB, not just a sim** (From Dust: manipulate water/lava/soil FLOW to guide people + avert disaster — the fluid IS the gameplay) | From Dust | DF-FLUID = a god-power (channel water to fields, divert lava, flood attackers), not just terrain flavor. Design the sim + the god-verb together |
| E6 | **CA fluid: choose the pressure model deliberately** (ONI/DF use cellular automata — Navier-Stokes too costly; one-material-per-tile; DF adds pressure so water climbs, DwarfCorp omits it so water only falls) | ONI, DF, DwarfCorp | The DF-FLUID spike's real fork = CA-WITH-pressure (richer/costlier, water equalizes+rises) vs WITHOUT (cheaper, falls/spreads only). Decide by the god-verb we want — confirms the designer's CA rec |
| E7 | **UI legibility is a first-class FEATURE, not polish** (DF's opacity vs RimWorld's streamlined UI = the accessibility gap that gates reach) | DF vs RimWorld | Object-inspection + dialogue + action-bar are genre-critical: sim depth is worthless if unreadable. Ties D4/G5 |

---
## COMMON QUESTIONS TO ALWAYS ASK (the pre-flight / review checklist)
**Correctness** — What's the boundary/±1 case (reach == rise)? Does this guard run the AUTHORITATIVE check or a
cheap proxy that can lie? Can this safety net WEDGE ITSELF? What happens with TWO simultaneous instances? Any
unwrap/panic/overflow on runtime state? Is it deterministic under load — and does any NEW shared state sit inside a
`par_join` whose determinism only held because the work was independent (B10)?
**Feasibility (spec pre-flight, before it reaches the builder)** — Are the spec's CODE claims actually true (verify
against source, don't trust the survey)? Does each claimed "reuse" have real machinery + call sites, or is it a
fitting-named stub (D11)? Does the approach touch a shared/vanilla struct instead of gating on Colonist (D12)? Does
the abstraction preserve the vertical deep (D13)? Is the effort estimate honest once the hard seam (determinism,
sync, decoupling) is priced in? What's the cheapest 80%-value alternative? Verdict: FEASIBLE / -WITH-CHANGES /
NOT-AS-SPECCED.
**Reuse / design** — Does a shared primitive already do this (`carve_ramp`, `astar.rs`, `Chaser`, `BlockChange`)?
One authority or duplicated? Does it drift from BASTION_ARCHITECTURE? Is this frontier+1 or speculative? Is a
shared VOCABULARY (enum/field N systems emit into) locked to its FULL range before emitters harden? Are its
ASSETS filed (or noted asset-free)? For every unbuilt dependency, do consumers DEGRADE gracefully? Is zero-input
still a complete healthy game (autonomous-not-4X; god tilts, never runs)?
**Testing** — Is there a UNIT test on the pure boundary function? A regression scenario for THIS exact bug? A
metamorphic property (more access ≤ entrapment; better tool ≤ dig time)? Did the fix ship WITH its test? Does the
scenario prove the MECHANISM, or does it pass via a scenario-specific motivator/workaround (continuously-fed jobs,
teleports, hand-placed anchors) that wouldn't hold in real play — and if so, is the masked gap logged + tracked?
**Performance** — Is there a per-tick budget? Full recompute or incremental repair? Does cost stay ~linear in N?
Is a hot many-to-one goal shareable? Does the regression window fail on a 2× slowdown?
**Persistence** — serde-default safe? wire/save-compat noted? rtsim promote/demote with no dupe/loss/desync?
**Gameplay** — Does the loop CLOSE? Is the recorded/emitted data READ BACK (no write-only sink)? Is the loop
DARK until a keystone (B7/B8) is built? Is the pace deliberate? Is there player-legible feedback? Does a stuck
agent break the autonomy fantasy? Does the strong option pay a cost? Is every pressure bounded + soak-survivable?
Is every loss RECOVERABLE (no permanent softlock)? Is standing/alignment EARNED by deeds, not granted by a toggle?
**Player legibility (UI)** — Can the player SEE it work and INVOKE it? Is there an action-bar/HUD surface for it?

*Seeded 2026-07-10 from real finds (annulus off-by-one, F2/F3 guard-wedges, time-controls-admin-gate,
window-wedge, pathfinding-scale, watchdog-grace). Append-only — every new class earns its row.*

*Curator appends — 2026-07-10 (Build-Reviewer): +B10 (shared-state-under-par_join determinism), +D11 (overstated
reuse / stub), +D12 (Bastion touches vanilla vs Colonist-gated), +D13 (abstraction flattens the vertical deep),
+Feasibility checklist row — all from the FR1 pathfinding + FR2 fluid feasibility reviews (BUILD_REVIEW_LOG §FR1/FR2).
+G12 (self-correcting feedback consumes its own recovery labour — the deep-shortage death-spiral trap) from FR3
autonomy-arbitration. +B11 (partial save-back loses loaded-tier mutations — the LOD-seam persistence death) from
FR4 lod-persistence. +D14 (type-level invariant with a leaky variant — the "impossible by construction" claim must
cover every branch) from FR5 god-powers-dispatch. +B12 (real-time-anchored rate coupled to the timescale/day-length knob + TimeOfDay/DeltaTime
clamp-desync) from FR6 timescale-design. +B13 (implicit loaded-chunk assumption broken by a top-down/remote
trigger) from FR7 build-mode. +B14 (reset-prone accumulator — the reset races/starves the threshold; R3 stuck_time +
R5-F5 churn.1, confirmed recurring) from the B6 SOFT-0 reviews. +D15 (overloaded-BlockKind conflation — Wood = trees
AND buildings; discriminate structurally) from FR10 chop-redesign.*

*Curator dedup — 2026-07-11 (Build-Reviewer): the architect-added "claimability/exposure gate admits UNSTANDABLE
work" class collided on the B10 number (B10 is already shared-state-under-par_join, referenced across the log/FR3/R7b)
→ renumbered the new class to B15, kept both. Numbers are stable once referenced; new classes take the next free slot.*

*Builder append — 2026-07-11 (LADDEROFF / Build 1): +D16 (disabling an access-PROVIDER silently starves a GATE
that depended on it) — surfaced while measuring the auto-ladder-off deep-dig throughput: the ACCESS-BEFORE-DESCENT
gate held depth>2 cells unmineable once the auto-ladder fallback was off (tight 5×5 stairs don't fit → plan_access
None → gate never releases → 75/150). Fix shipped in the LADDEROFF tag: release the descent gate when
AUTO_LADDER_ACCESS is off AND no access is buildable; the universal teleport is the egress. Architect-endorsed;
fully reversible with the flag. (Reviewer: curate/renumber as needed.)*

*Builder append — 2026-07-11 (SLOPE / Build 2): **B15 CLOSED** — bastion-block-SLOPE
gates Mine claimability on a TERRAIN-ONLY once-per-cycle STANDABLE stance
(has_standable_stance): prefers on-top, routes a wedged +1-slot to its reachable
downhill adjacent stance, clean-skips an isolated 1-wide floater (no reachable
stance → no churn). The exposed→standable swap is regression-safe (b58 150/150;
a first cut that preferred adjacent regressed to 87/150 — on-top-preferred + the
≥3-solid-side wedge check is the fix). Play-tester confirmed the class on a real
2:1 slope (65/75 give up pre-fix, 44 exposed-unstandable = the target; a solid
slab yields 0 floaters — floaters need undercut/overhang). B15's "natural-slope
in the test matrix, not flattened" rule now has b5 phase 7.9 (claim-level unit)
+ the play-tester's --slope-mine-scenario/--floating-block-scenario (SET-A/SET-B)
as the regression. (Reviewer: curate.)*
