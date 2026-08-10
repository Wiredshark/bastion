# BUILD ROADMAP — the living list

**Maintained by Fable (orchestrator) — updated at EVERY item close, finding, or Ben directive.
Ben's standing order (2026-08-10): "you need to be constantly thinking about this stuff as we
finish features, you need to continually update this build list."**
Rulings live in `DECISIONS-FOR-BEN.md` (rows cited); this file is the current-state view.
Format: append changelog at bottom; statuses edited in place. Newest ruling wins.

## Standing design riders (apply to every item below)
- **Plot/plan data model** (#102): anything placing structures/zones designs against plots, never raw positions.
- **Live-emit declaration** (#88): every new harness accessor declares ported / harness-only-with-reason.
- **Diagnostics never verdict terms** (#70 pattern); **event-driven emits only** (diag-density budget).
- **Instrument + registry row land in the same commit**; **asset tuning touches compiled default same commit**.
- **Ruling acknowledged = packet edited, one operation** (#101). **Claims are not artifacts.**
- **A WITHDRAWAL is a ledger event too**: retraction and roadmap/packet edit are ONE operation — withdrawing has no natural trigger, which is exactly why stale entries outlive dead claims.
- **Every bar names its window AND its arena (who else can act)** (#95); success signature in every observable.

## ARC 1 — Close the survival loop (items 1–8)
| # | item | status |
|---|------|--------|
| 1 | Name the food thief | **DONE** (#95 — ambient NPCs; attribution complete, zero residual) |
| 2 | Stall counter (rescue cleanup counts progress, not claims) | **BUILT + corpus-exercised (wave33: prunes fire, both paths, attribution partitions perfectly).** ACCESS_STALL_SECS=120 PROVISIONAL — right-censored by its own reset (8 seeds at exactly 120; non-pruned 119 argues raise-never-lower); unblock row approved: env-tunable thresholds (+manifest rows same commit) + final-beside-peak emit, lands WITH item-6 witness, one fan scores both. Branch A retired (0/48 — option 3 dead by corpus, never built: measured-first paid). #38 CLOSED (prune fires, 14 events/11 seeds). |
| 3 | THE ARRIVAL FIX (sit trap) | **LANDED** 6797b5c409 + witness d0ce0a58e1; proof pair score PENDING |
| 4 | Egress planner (verdicts-without-plans) | PARKED — likely symptom of 3; re-scores on own signature post-3; VOID-on-zero recorded |
| 5 | Server-authoritative test timing (Wait(n)) | **DONE** a35a98aaf7 — A/B PASS: sim-target matched (20.02 vs 20.01s) across unthrottled/contended with different spin counts (609/495), zero errors; failsafe corrected pre-land (mute wall-clock check → engine ServerTimeout signal, Opus catch); spin-direction surprise traced to producer (starved client's wall-dt grows) before reporting. Scope limit honestly held: whole-box contention arm, not isolated server-slowness. |
| 6 | Protected persistent provisioning + stockpile eating | **DONE — live acceptance PASSED** (field-placed pile protected the full 25-min run vs a loose drop taken in 37 s pre-fix; positive controls live). Corpus visibility: STRUCTURALLY IMPOSSIBLE under existing scenarios (5b premise-check at source: zero item-drop creation anywhere in ~40 harness scenarios) — witness fields stay as DORMANT SENTINELS with the premise documented; no bespoke fixture on spec. **CORPUS-INVISIBLE (wave33 gap): no field matches pile/protect/ambient/loot — witness row APPROVED (refusals-by-reason, pile-pickup attribution, reserved-units), lands with item-2's threshold changes, one fan scores both; WITNESS RANKS FIRST within the batch (wave33 addendum: build/material signature widened to ≥3 seeds incl. a clause-SWAP invisible to the fail count — the gate is the strongest unexplained-pattern candidate). REGISTERED PREDICTION for the witness fan: if "ambient-loot-disabled" refusals fire on COLONIST pickers, the movers are explained and the gate has a membership/timing bug — candidate, not claim.** |
| 7 | Farm-to-table self-sufficiency | **REAL ROOT CAUSE: SEED-SUPPLY DEADLOCK (structural, two code sites — 5b addendum 812757982c).** `FARM_SEED_ITEM`'s only producer anywhere is a successful HARVEST (bastion_jobs ~13692); harvest needs a mature crop needs a SOW needs a fetched stockpiled seed; worldgen volunteers explicitly unharvested ("a later nicety"). A fresh colony has NO possible first seed — sow jobs sit forever unclaimable. The earlier "phantom-retire = root cause" claim is WITHDRAWN in location/timing: terrain.get's ONLY failure is NoSuchChunk, (x,y)-dependent — z cannot discriminate (Chonk::get_unchecked infallible per z, read at source); all 48 retirements fired in ONE 1.1ms window <1s after driver disconnect = end-of-run chunk-unload sweep, not a during-run defect (job 703 survived ~4800 sweeps first). The sow/till asymmetry explained: till jobs complete and leave the board; seed-blocked sow jobs alone survive to be swept at disconnect. TWO INDEPENDENT FIXES: (1) seed bootstrap source (cheap confirm first: /give_item seeds run); (2) task #57 sweep splits read-failure from genuine mismatch (disconnect unload must not masquerade as designation change) — still worthwhile, no longer the cause. |
| 8 | Multi-day endurance run (TIER-1 AUTONOMY GATE) | waits on 2,3,6,7 — the "colony runs itself" milestone; Ben gets observer seat |

## ARC 1.5 — Infrastructure (runs alongside)
- **Entity event log** (#99/#100): designed (DESIGN-ENTITY-EVENT-LOG.md, premise-checked chassis, per-entity rings, promotion persistence). **PROMOTED — builds NEXT in 5b's queue.** Pilot vocabulary includes Released{job, reason, tick, actor}; **first acceptance customer = the mover investigation's registered claim-overlap prediction** (orphanings cluster near removals vs uniform — undecidable by run-total counts, decidable by the stream). The Voonoo query stays as the second acceptance.
- Reasoning dossiers (#98): FOOD pilot written; travel dossier at item 3's close; dossier per arc-close is ritual.
- Self-terminating run harness (#73 — server+driver teardown); collect_wave --baseline DONE (#67). Graduation note (Fable, 2026-08-10): when #73 lands, `stale_binary_preflight.sh` (scratch tooling, 5b) becomes its FIRST STEP, invoked by the run script itself rather than remembered by whoever launches a run — a tool someone must remember to run is a habit wearing a filename; wired into the runner it becomes law.
- Farming writeup (5b, DONE 2026-08-10): FARM-SOW-PHANTOM-RETIRE-FINDING.md + addendum -- root cause is a seed-supply chicken-and-egg deadlock (structural, code-verified), not the phantom-retire mechanism the original body centered on (that's real but incidental, a disconnect-triggered chunk-unload sweep). .engine-integration-wt target scoped cleanup (5b, DONE 2026-08-10): target/debug/incremental removed (54G, confirmed 2-days stale), target/no_overflow untouched.

## ARC 2 — Legibility (9–12)
9 Colonist inspector HUD · 10 Colony dashboard · 11 Recreation/idle life · 12 Chronicle UI (becomes the entity-log's player view).

## ARC 3 — Threats (13–15)
13 Hostiles near colony (Flee drive's live test) · 14 Guard job · 15 Fortification designations.

## ARC 4 — Economy & persistence (16–18)
16 Haul priorities + hauler-vs-eater contention (**plot-model rider applies**) · 17 Skills visible+felt · 18 Save/load integrity (entity-log promoted-set migration rides this).

## Capstones (19–20)
19 Renderer horizon retest (one-constant fixture fix — arena radius ≥ tested horizon) · 20 **Milestone playthrough v2** — LLM-player session scoring everything (#62 recurring gate).

## Band 21–40 (directional; design docs exist where noted)
21 Personalities visible · 22 Relationships · 23 Morale events · 24 Seasons that bite · 25 Shelter quality ·
26 Crafting chains · 27 Cooking · 28 Tool quality/wear · 29 Trade with vanilla world · 30 Stockpile zones by type (**plot rider**) ·
31 God-hand v1 (founding docs exist) · 32 Faith economy · 33 Miracles ·
34 Raids scaling with wealth + **THIEVERY as designed feature** (#97 — gate lift is line one; chronicle wiring pre-built per #100) ·
35 Injuries & medicine · 36 Death that matters ·
37 LLM-player harness v2 · 38 RL-tuning groundwork · 39 Performance row (sub-threshold tick degradation) · 40 Colony scale 16–32.
**SETTLEMENT PLAN LAYER (#102)** slots as its own arc in this band, before 31: plot grammar reuse, district zoning, cost-driven form, desire-line roads, layout-quality corpus metrics.
KNOWN INHERITANCE (charter line, Opus 08-10): today's `stockpiles: Vec<(ZoneId, Region)>` painted-bounding-box zones ARE the raw-position model — the arc's FIRST MIGRATION TARGET, not a green field. Item 16 is stockpile-adjacent and carries the plot rider pre-spec.

## Changelog
- 2026-08-10 (14): Seed-69 collision SUPPORTED (1 prune ↔ +1 orphan at exactly 120.0; access→0) — supported-not-proven, no queue-position field exists; the pilot gains its SECOND customer (seed-69 position + mover clustering). Mover population cleans: 5 unexplained orphan-movers, no prunes involved. #103's derived-in-code threshold and re-scope stand. Entity-log stage 1 = the arc's critical path.
- 2026-08-10 (13): ARM 2 SCORED — prediction CONFIRMED (max self-resolved stall 141 ≥ 120): the pruner was cutting recoverable work. RULED: default → 180 (a bound, not a calibration; 287 = the genuine customer), ships with effective-config-to-stdout (third stderr casualty). UNREGISTERED PRIZE: six seeds peak at exactly 120.0 with the pruner OFF — a hidden ~120s releaser exists; open row (producer hunt, moderate priority) — may reframe item 2's value onto the >180 tail. "A data-side config proof only works where the config would have mattered" banked (24 proven / 24 unproven census).
- 2026-08-10 (12): Mover lead's meaning corrected at producer (orphaned-claim detector, not removal witness) — lead sharpened to a COMPOSITION fact (removal ∧ live claimant); #94 reopened on claim-timing footing; prediction registered with can-I-evaluate asked = NO ⇒ ENTITY EVENT LOG PROMOTED to next build, mover prediction = its first acceptance customer.
- 2026-08-10 (11): ITEM 6 DONE — live acceptance passed (pile held 25 min vs 37s pre-fix theft); corpus witness structurally impossible (no scenario creates item drops), dormant sentinels documented. #83 closed (count gate 43, ENVPREFIX, zone probe, effective-config banner — all rehearsed). ARM 2 RUNNING at 9999 with the effective-value-on-VMs check owed before any field is read; registered prior on record (max(R) ≥ 119 expected, over-read owned if wrong).
- 2026-08-10 (10): Specimen diffs name the mover mechanism as an ACCESS-PLAN REGIME change (seed 51: access planning stopped, 37→0 + region blocked; seed 50: doubled + 4x emergency calls — opposite directions = regime, not gate). Item 6 EXITS the candidate set. Next: bisect access-plan commits (free). PRE-REGISTERED reading: if the bisect names item 2's sweep, the movers ARE arm 2's calibration question (does pruning at 120s cost outcomes stalls used to salvage?). FREE HEADLINE: bit-reproducibility measured across zones/hosts/instances — 48 seeds, every scalar identical except stamp+clock; machine-type fallback licensed by data.
- 2026-08-10 (9): Wave34 arm-1 landed via rescue (mislabeled invocation retracted; paired_base = true matched control). Batch INERT on outcomes (12→12, zero clause drift) ⇒ movers are STABLE-TIP properties, not bundle-transient — next step = specimen-seed field diffs (50/51), free. Item-6 witness: correct but scenario-INERT (all counters 0 incl. member pickups) — prediction VOID not refuted; "gate never refuses ⇒ gate innocent" explicitly NOT licensed (upstream suppression); premise-check question to 5b, else fields stay dormant sentinels + live-path verification. stalled_final earned immediately (6 still-stalling seeds, invisible before). Run-invocation-in-header now a required template field. Arm 2 blocked only on #83.
- 2026-08-10 (8): Wave34 arm 1 RUNNING (defaults, pin 7f20a184, witness+stalled_final landed, checker rehearsed pre-data). TWO-ARM ruling: arm 1 = movers/witness/prediction (instrument-only delta); arm 2 = raised BASTION_ACCESS_STALL_SECS on the same pin for the uncensored calibration — corrects the one-fan conflation of landing-vs-answering.
- 2026-08-10 (7): Wave33 addendum — seed-66 tool refusal resolved (REG-1 vindicated on its own named specimen); build signature widened to ≥3 seeds via clause-SWAP; item-6 witness ranked first in batch; colonist-refusal prediction registered for the witness fan.
- 2026-08-10 (6): Item 5 DONE (A/B pass, a35a98aaf7) — driver timing is server-authoritative; the under-run class that voided driver-12 is closed.
- 2026-08-10 (5): Wave33 folded — #38 closed, branch A/option-3 retired by corpus, ACCESS_STALL_SECS provisional (self-censoring instrument), item-6 corpus witness + threshold row approved as one batch/one fan; 5 movers (11→12) PARKED-tracked pending the witness fan (bundle diff refused attribution).
- 2026-08-10 (4): 5b closed both owed items (farming writeup; 54G stale incremental cleanup — E: +54GB) and placed the #73 preflight-graduation note; item 5 in build.
- 2026-08-10 (3): Item 7 re-rooted — seed-supply deadlock (structural); phantom-retire demoted to end-of-run sweep noise (refuted at source + 1.1ms timing window); two independent fixes chartered.
- 2026-08-10 (2): Opus's #102 in-flight audit folded in — nothing exposed; item 16 flagged pre-spec; settlement arc's known inheritance (Region zones) chartered.
- 2026-08-10: Created (rows #93–#102 folded in). Items 1 done, 3 landed-pending-score, 6 in acceptance, 7 blocked-with-coordinate; settlement layer added with immediate rider; thievery moved to 34's family; entity log designed.
