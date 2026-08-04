# LIVE LLM-PLAYER PLAYTHROUGH — prep notes (READ-ONLY, slack-time)

**Milestone (Ben, relayed via Fable):** after Row A/B land and before AUTON-2,
run a **live LLM-player playthrough** exercising every shipped feature and
producing a **scored checklist — each feature pass/fail AS EXPERIENCED BY A
PLAYER**, not as measured by the harness.

> His concern, verbatim: *"if an LLM played it live, could they do all our
> implemented features successfully?"*

**This is `gate-must-test-live-path` at the product level.** Every green we have
is a harness green. The playthrough asks the one question no fan can: **does a
player, given the game, actually get there?**

*Status: PREP ONLY. Nothing here displaces the fans, the hold-check, or Row B.*

## §0 — SEQUENCING (settled by Ben, recorded once)

> **hold-check → Row A closes the window → Row B (built, gated by FR15 A/B) →
> LIVE PLAYTHROUGH scored → AUTON-2**

An intermediate proposal to run the playthrough **before** Row B was raised and
**countermanded by Ben**. This is the final ordering; it is written here so
nobody re-litigates it from a stale message.

**And the override gives the playthrough a better job.** Running it after Row B
makes it **Row B's player-level acceptance test** — one run answering *"did the
fix make the game better to play?"* — instead of two runs bracketing a known
bug. The stuck-job prediction below therefore flips from *"expect to observe
it"* to **"expect it to be FIXED, and this is where that gets confirmed in
player terms."**

**5b drives the live session** (server + client) through the **real player
surface** — designations painted, zones drawn, the actual command path — **never
harness scenario injection.** Injecting would re-create the exact defect this
run exists to detect. **I design the scorecard; they run it.**

**Run 1 is scored RECONNAISSANCE, not a gate.** A single live run is allowed:
the seed is recorded so anything interesting becomes a repro candidate, but the
playthrough is **not** held to corpus determinism standards on its first outing.

---

## §1 — ★ THE FIRST TRAP IS IN THE CHECKLIST ITSELF: AN ARM IS NOT A FEATURE

The harness has **74 dispatch arms** at `d5b56d1c79`. It would be easy — and
wrong — to call that the feature list.

| population | count | what it is |
|---|---|---|
| total `else if args.*` arms | **74** | every runnable mode |
| named `_fixture` / `_probe` / `_model` / `_sentinel` / `_paired` | **12** | determinism fixtures and geometry probes — **no player ever meets these** |
| remainder | 62 | *still not features* — includes `terrain_ground_dump`, `verify`, `net_envelope`, `phy`, `shd`, `lod0`/`lod1`, `ter`, `per`, `values`, `derive`, `esim`, `evt` — **tooling and engine diagnostics** |

> **A scenario is a TEST of a feature, sometimes a test of an instrument, and
> occasionally just a dump tool. Counting arms and calling the total "features"
> is `read-the-content-not-the-label` applied to our own roadmap.**

**So the checklist must be built from the player's side, not the harness's**:
enumerate what a player can *do* and *see*, then find which arm (if any) covers
it. **Any feature with no arm is exactly the interesting case** — shipped and
never harness-tested — and the playthrough is the only thing that will find it.

## §2 — HOW A LIVE SESSION IS ACTUALLY DRIVEN (read, not assumed)

**The harness cannot do it.** Its non-scenario flags are `--seed`, `--ticks`,
`--tps`, `--schedule-seed`, `--deterministic-parallel`, `--ladder-episode` —
a scenario runner, with no session to join and no way to act as a player.

**The live path is two crates:**

- **`server-cli/`** — hosts the world. Save isolation is `VELOREN_USERDATA`
  (**not** `VELOREN_SERVER_DATADIR` — that mistake is already recorded).
- **`client/`** — the programmatic client. **This is the LLM player's hands**;
  `voxygen/` is the human GUI and is the wrong surface for an agent.

**Prior art exists and should be read before anything is built:**
`bastion-test-evidence/live-voxygen-observer-20260716/` holds
`server-stdout*.log` / `server-stderr*.log` from a real live session — so a
server has been driven and observed before. **Read those logs first**; they will
show what a live session already emits and what it doesn't.

**Two live-path facts already paid for, do not re-derive:**
- The live game was **non-deterministic** until `tick_rng` was stopped from
  falling back to OS entropy — a fixed finding, and the reason a live
  playthrough is now reproducible enough to score at all.
- `--endurance-scenario` ran 10k ticks on real terrain and **held cross-run**,
  but **must run isolated**.

## §3 — WHAT A CHECKLIST ROW LOOKS LIKE

The point is that success is stated **the way a player would state it**, in
observable terms, with no harness field in the sentence.

| ✗ harness phrasing | ✓ player phrasing |
|---|---|
| `b5_mine_blocks_mined == 27` | *"I marked a wall to be mined and the colony finished it without me intervening."* |
| `farm_growth_rose == true` | *"I planted a field and later harvested food from it."* |
| `blocked_regions` non-empty | *"When something couldn't be reached, the game TOLD me, and what it told me was true."* |
| `b5_release_timed_out` | *"Colonists didn't visibly give up and re-try the same job forever."* |

**Each row carries:** the player action, the observable success, the observable
FAILURE (what going wrong looks like), and — where one exists — the harness
field that claims to cover it. **Rows whose harness column is empty are the
row's real value.**

## §4 — PRE-REGISTERED PREDICTIONS (Fable's, recorded BEFORE the run)

Registering these now is the whole point: a prediction written after the
playthrough is a rationalisation.

| # | predicted rough edge | status going in |
|---|---|---|
| 1 | **the stuck-job mystery** | Row A/B is the fix — playthrough is the live confirmation |
| 2 | **no needs behavior** | AUTON-2's job; expected to fail, and that failure is *scheduled*, not a surprise |
| 3 | **run-speed feel** | open — the 1.14-vs-1.25 question the corpus could not settle |
| 4 | **zone's red** | known-red going in |
| 5 | **message overclaims** | ★ **already confirmed by this session** — *"A designation is blocked — obstruction at (x,y,z) can't be reached"* asserts terrain its guard cannot establish, and seeds 80/90 proved the guard wrong. **A player will read that sentence and believe it.** |

> **★ Anything the playthrough finds that the corpus never saw is a finding about
> our INSTRUMENTS, not only about the game.** That is the same corollary as
> tonight's `blocked_regions` result: the store was reported at the wrong
> coordinates, so the corpus was blind to precisely the case the row exists for.
> **Expect the playthrough to expose more of those, and log them as instrument
> gaps rather than as bugs.**

## §4b — THE SCORECARD (v1 — 13 features, player language)

**Each row: the player ACTION, what SUCCESS looks like to a player, what FAILURE
looks like, and the metric to capture.** Failure is stated explicitly because
*"it didn't work"* is not a finding — **what it looked like when it didn't** is.

| # | feature | player action → SUCCESS | what FAILURE looks like | metric to capture |
|---|---|---|---|---|
| 1 | **mine** | *"I painted a 5×5 mine face; it fully cleared, and I didn't have to do anything else."* | cells left undug with nobody working them | blocks mined / designated; time-to-clear; timeout + release counts |
| 2 | **chop** | *"I marked trees; they came down and the logs ended up somewhere useful."* | tree stands; or falls and logs are never collected | trees felled; logs collected vs dropped |
| 3 | **build** | *"I placed a blueprint and it got built without me feeding it."* | blueprint sits unbuilt, or stalls waiting on materials that exist | placed vs completed; material-wait duration |
| 4 | **farm, full cycle** | *"I drew a field; it was tilled, sown, grew, and I got food out of it."* | any stage silently stops — esp. tilled-but-never-sown | per-stage counts (till/sow/grow/harvest); `farm_g1_baseline` |
| 5 | **haul / stockpile** | *"I drew a stockpile and loose items ended up in it."* | items sit on the ground next to an empty stockpile | items hauled; haul drops (`stuck_strikes >= cap`) |
| 6 | **bed / sleep** | *"Colonists went to bed and woke up rested."* | colonist stands next to a bed, or sleeps on the floor | bed claims; sleep completions |
| 7 | **storm flee** | *"A storm came and they took shelter instead of standing in it."* | colonists keep working outdoors through the storm | flee events; casualties |
| 8 | **pit rescue** | *"Someone got stuck in a hole and the colony got them out."* | colonist remains in the pit indefinitely | rescue fires; access plans emitted |
| 9 | **cave-in survival + conservation** | *"A tunnel collapsed, nobody was buried alive, and the blocks went somewhere sensible."* | buried colonist; or blocks vanish/duplicate | survival count; conservation delta (**must be 0**) |
| 10 | **coordination barks** | *"They said something that matched what they were actually doing."* | bark fires when nothing is crowded — the guard/claim mismatch | bark count vs measured saturation delta |
| 11 | **zones** | *"I drew a zone and it did what the zone is for."* | ★ **known-red going in** | zone-specific counters |
| 12 | **run-speed feel** | *"When they hurry, they visibly move faster."* | running looks like walking | observed ratio vs the 1.14 / 1.25 question |
| 13 | **blocked-designation messaging** | *"When something couldn't be reached the game told me — and what it told me was TRUE."* | ★ silence, **or a message naming an obstruction that isn't there** | messages emitted; `blocked_regions` count; **`source` per entry** |

> **★ ROW 13 IS THE ONE WITH A KNOWN DEFECT ALREADY PROVEN.** *"A designation is
> blocked — obstruction at (x, y, z) can't be reached"* asserts a **terrain
> cause**; its guard only establishes that the **planner found no route**, and
> seeds 80 and 90 both came back multi-layer, meaning the guard was wrong at the
> only two sites we checked. **A player reads that sentence and believes it.**
> Score this row on *truth*, not on presence.

**Rows 1–9 have harness coverage. Rows 10–13 are where the playthrough earns its
cost** — 12 in particular has never been settled by any fan.

## §5 — WHAT'S NOT DECIDED YET (open, for after Row B)

- **Who plays.** An LLM agent driving `client/` directly, vs a scripted harness
  that replays a fixed action list. *Ben's framing says LLM.* The scripted
  variant is a control, not a substitute.
- **How scoring is recorded** so it is comparable across runs — the same
  enumerate-the-delta problem, at the level of a subjective checklist. A score
  that cannot be re-derived is a verdict without a body, which is the failure
  this session already fixed once.
- **Whether the session is deterministic enough to re-run.** The `tick_rng` fix
  says probably; an actual paired live run says definitely. **Do that check
  before trusting any score.**

## §6 — ★ READ-BUDGET CHECK ON THE METRICS PLAN (added after the observer-effect bisection)

**New standing requirement (acceptance framework, ruled in 2026-08-04): every
observability addition states its READ BUDGET — cells × reads × cadence.**

It exists because a measured bisection showed two per-cell diagnostic reads
turned a bit-reproducible run into a varying one, while the same build with those
reads removed was 0-diff. **Reading is not passive**; see
`the-instrument-changes-what-it-sees`.

**So this plan's own metrics need the check before the run, not after.**

| §3 metric source | shape | budget | verdict |
|---|---|---|---|
| completion %, time-to-complete, timeout/release counts | **server-side aggregates**, already maintained | ~0 extra reads | **expected clean** |
| `blocked_regions` count | one accessor, colony-wide | ~0 | **expected clean** |
| wall-time | ambient | 0 | clean |
| player-visible message log | already emitted | 0 | clean |
| ★ **per-cell diag** (`mine_cell_diag` / `farm_cell_diag`) | **cells × reads × every capture** | **NOT free — this is the exact shape the bisection indicted** | **verify before relying on it** |

> **★ THE PLAYTHROUGH IS THE ONE PLACE THIS MATTERS MOST AND IS EASIEST TO MISS.**
> A live scored session is *precisely* where someone reaches for more per-cell
> detail to explain what they just watched — and that reach is now known to
> perturb the thing being watched. **Pull aggregates; add per-cell reads only
> deliberately, with the budget stated.**

**"Should be clean" is not the standard we now hold.** The metrics plan pulls
aggregates, so it *should* be unaffected — but that sentence is exactly the
untested-premise shape this campaign keeps finding. **One verification run before
the playthrough: same seed, two parallel legs, metrics collection on, confirm
0-diff.** If the collection itself perturbs, the scorecard measures a game
slightly different from the one a player would get.

*Method is canonical: parallel legs, separate `--data-dir`s.*

## §4c — ★ ROW B′'s ACCEPTANCE ROWS (added at launch; #61 shipped it)

**The playthrough is Row B′'s acceptance test.** Its corpus aggregate is a proven
null — so **these rows are where its claimed value either exists or doesn't.**
Score them honestly; a null here means the fix bought nothing anywhere.

| # | claim | player-language success | what FAILURE looks like | metric |
|---|---|---|---|---|
| **14** | **stuck jobs are noticed** | *"A cell nobody could reach stopped being retried over and over."* | colonists visibly cycle on the same unreachable cell all session | benched jobs (count, which cells) |
| **15** | **the colony stops paying** | *"When something was unreachable, the others got on with useful work instead of queuing for it."* | idle colonists, or the whole crew orbiting one bad cell | freed-colonist activity; other designations' completion |
| **16** | **benching RELEASES** | *"A job that got shelved came back and was tried again later."* | ★ shelved forever — **a bench that never releases is a drop in disguise** | graduations (count); bench→graduate→re-bench cycles |
| **17** | **the message is TRUE** | *"I was told colonists kept failing to reach it — and that's exactly what was happening."* | ★ a message asserting an **obstruction** that isn't there | message text vs observed cause |

> **★ ROW 17 IS THE REWORDED MESSAGE'S TRIAL.** The old text —
> *"obstruction at (x,y,z) can't be reached"* — asserted a **terrain cause** its
> guard could not establish, and seeds 80/90 proved the guard wrong at both sites
> checked. The new text — *"colonists have repeatedly failed to reach…"* — claims
> only what is measured. **Score it on truth, not on presence.** A player reading
> the old sentence would have believed a false thing.

**Prediction #1 UPGRADES** (was: *"stuck jobs occur and are now visible"*):

> **Stuck jobs occur, are visible, the colony stops wasting effort on them, and
> the player is told honestly.**

Four claims, four rows, each refutable. **If rows 14–17 score well and the corpus
still says nothing, that is the finding** — it would mean the harness cannot see a
class of value the player can, which is the wrong-coordinates lesson one level up.

## §7 — LAUNCH CHECKLIST

- **Shipped tip**, flag at its shipping default. Attest the binary hash.
- **Real player surface only** — designations painted, zones drawn, commands
  issued. **Never harness scenario injection**; injecting re-creates the exact
  defect this run exists to detect.
- **Seed recorded.** One session, reconnaissance-grade — no corpus determinism bar
  on run one, but the seed makes anything interesting a repro candidate.
- **Capture law applies:** full logs + counters persisted to
  `bastion-test-evidence/`, not just a verdict. *A run that persists only its
  conclusion is not evidence.*
- **Metrics are server-side aggregates.** Per-cell diag reads are the shape the
  observer-effect bisection indicted — **if you reach for more per-cell detail
  mid-session to explain something, note that you did**, because it perturbs what
  you're watching.
- **Score as experienced.** No harness field in the success sentence.
- **Anything the corpus never saw ⇒ log it as an INSTRUMENT lesson**, not only a
  game bug.
