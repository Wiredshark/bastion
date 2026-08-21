# FUNCTIONALITY FIRST — the working list (Ben direct, 2026-08-21)

> *"before we return to the build list we need to get all the existing pieces
> functional. the llm playtest will diagnose and fix."*

The arc order is PAUSED. This file is the working list until the game holds up
under play. Every row is something that EXISTS but does not behave the way a
player would expect. A row closes only when a live play session SHOWS it
working — the EXPERIENCE census is the floor, a played session is the bar.

**The floor, measured 2026-08-21 (adopted village, 8 colonists, ~1,500 jobs
available):** `total=8 working=0 moving=2 stuck=1 idle=6 fed=8 rested=8`.
Zero working. That number is the headline this list exists to move.

**MOVED (2026-08-21, arm `injury`, founded colony, nothing handed to it):**
`mean working 1.30 → 2.88`, `mean idle 6.00 → 3.70`, `beds built 0 → 8`,
`rested` recovering instead of pinning at 0, six completed sleeps. The
autonomous colony now beats the arm that was handed its materials by a test
env var. See `F13-DISPOSITION.md` — four stacked blockers, each sufficient on
its own, each hiding the next behind the identical symptom.

---

## P0 — the colony does not look alive

| # | Broken thing (player language) | Root cause (measured) | State |
|---|---|---|---|
| F1 | Colonists walk into a wall and stall there | A* starts at `PathLength::Small` (500 iters); escalation requires standing near the search START, which a partial route already broke — so it resets to Small forever, and the give-up route aims at the explored node nearest the goal: the wall face | design panel + fix in flight |
| F2 | Colonists starve next to food | Their meal is a loose ground item the colony's own haulers merge away mid-walk; the eat job then DIED (635 preempts → 135 jobs → **33 meals**) | FIXED — re-aims at the nearest food (`5bb790c99c`), needs a played confirmation |
| F3 | Nobody eats at all when any job wants that food | SELF-CATCH: my in-flight-ingredient guard protected the eater's own meal (an EatFrom job carries food as its `required_item`) | FIXED (`5bb790c99c`), needs a played confirmation |
| F4 | 6 of 8 colonists idle in a town full of work | Partly F1; the remainder is unproven — the claim path must be measured per colonist, not per job | OPEN — needs a per-colonist claim census |
| F13 | A colony founded on open ground can never sleep, and finishes nothing it was founded with | **CONFIRMED — see `F13-DISPOSITION.md`.** Matched arms differing in materials alone: `bed registered (built)` 0 vs 8, and rest tracks identically for 7,800 ticks then falls to 0/8 and never recovers, against a twin that recovers to 7/8. Repair is Ben's (decision 112) — pre-stocking proves the machinery, not autonomy | **CLOSED, PASS** — the colony now mines its own stone, builds its own 8 beds and sleeps in them with nothing handed to it; it beats the hand-fed control on working and idle |
| ~~F13 (as first written)~~ | A colony founded on open ground never sleeps, and never finishes anything it was founded with | The founding designates 8 beds, but a Bed job requires `BUILD_MATERIAL_ITEM` and the founding provides **no material and no way to get one**: the preset places stockpile/farm/bed and no Mine or Chop. Materials only ever arrive from `BASTION_SEED_MATERIALS`, a *test* env var — so an unseeded founding is materially inert by construction. Measured on the attested injury leg: `beds=0` and `rested=0/8` for the whole run, `idle=7/8`, and 24 of 27 claim refusals reason `materials` | OPEN — pre-registered below |

**F13, PRE-REGISTERED before its leg runs.** The chain is proven at three of
four links and assumed at the fourth, which is exactly the link the leg tests.

- Proven by reading the code: a Bed job's `required_item` is
  `BUILD_MATERIAL_ITEM`; `completion_block(Bed)` returns `Some(Bedroll)`, so
  registration is *reachable*; `board.beds` has exactly one writer on the
  founding path — a completed Bed build.
- Proven by the attested log: `beds=0`, `rested=0/8`, `materials=24`.
- ASSUMED, and under test: that materials are the ONLY thing missing.

**Prediction.** Re-run the identical arm with `BASTION_SEED_MATERIALS=64` added
and nothing else changed. If materials are the whole story: beds get built
(`bed registered (built)` appears), `beds` rises to 8, and `rested` leaves 0.
If beds still do not appear, materials were a *sufficient-looking* blocker
hiding a second one, and F13 is bigger than this row claims — that is the
result I would rather find early than assume away.

**The control that makes it a comparison and not a demo:** the injury leg
already run is the matched arm — same seed, same colony size, same decay
multiplier, differing in one declared variable. Both attestations get quoted.

**What this row does NOT decide.** Whether the fix is "the founding ships a
starter cache" or "a materials-short colony generates its own Mine/Chop work"
is a design fork with gameplay consequences, and the second is the one that
makes the colony autonomous rather than pre-stocked. It is banked for Ben in
`readme/DECISIONS-FOR-BEN.md`, not chosen here. This row establishes the
DEFECT; the ruling picks the repair.

## P1 — the adopted village is scenery, not a home

| # | Broken thing | Root cause | State |
|---|---|---|---|
| F5 | Colonists stand in a ripe field and do nothing | Harvest fires only on `WheatYellow` at `Growth >= FARM_GROWTH_MAX` in a registered column; village fields are `WheatGreen`/`Flax`/`Corn`/`Tomato` at Growth 0, explicitly reserved by a code comment | BUILT (`3e28e7f71d`) — volunteer arm mints harvest jobs for WheatGreen/Tomato/Carrot/Lettuce and the yields joined FOOD_DEFS; **awaiting live confirmation** |
| F6 | A furnished kitchen is invisible | `cook_stations` has exactly ONE writer: a completed CookStation *build* | BUILT (`623bf76b3f`) — and my FIRST version was on a path houses never take, with a vocabulary (`CookingPot`) worldgen never places in a house; now one shared scan keyed on `Ember`; **awaiting live confirmation** |
| F7 | An adopted barn is a decorative rectangle | `stockpiles` only ever holds painted regions; no chest/container sprite is read | BUILT (`623bf76b3f`) — 1-block stockpile per container, vocabulary corrected to what house.rs actually places (`Crate`/`Drawer*`/`Wardrobe*`); **awaiting live confirmation** |
| F8 | Colonists sleep on the ground beside made beds | *(was broken; the plot scan now registers village beds)* | FIXED — `adopted_beds=2`/house live |

## P2 — systems that exist but cannot be believed

| # | Broken thing | Root cause | State |
|---|---|---|---|
| F9 | Every village pays exactly the same for wood | `ratio_min=1.0 ratio_max=1.0 ratio_distinct=1` across all 194 sites — a constant wearing a site's clothes; item 29 bar 2 FAILS | OPEN |
| F10 | Starving has no consequence | Nothing reads hunger=0 except mood; no health pressure, no death | OPEN — needs a design ruling on lethality, banked |
| F11 | Colonists cannot die | Health reached 0.0 under repeated smite: no death event, no despawn, population unchanged | OPEN — item 36's true first step |
| F12 | Colony scale is unmeasured | The 8/16/32 legs ran under different co-load; VOID for the guard question | OPEN — re-run on the VM fleet, which is now quiet |

## P3 — found by an adversarial play session (12 turns, town arm, 2026-08-21)

An agent played the game specifically trying to break it, the way a real
player does: legitimate actions in awkward orders. Every row below is backed
by a log line or number it read directly. Two are already fixed (`11577a48d8`).

| # | Broken thing (player language) | Root cause (measured) | State |
|---|---|---|---|
| F14 | Cancelling near your barn kills hauling forever; the food store reads 0 while the food lies visibly on the ground | The orphaned-haul sweep was gated on a stockpile piece COUNT, and delete-one (−1) + split-another (+1) leaves the count identical. Logged its own proof: `zones_before=2 zones_after=2` with a zone destroyed | **FIXED** — compares zone identity; regression test asserts the trap arithmetic occurs before asserting the sweep |
| F15 | Paint a chop order on bare grass, lumberjacks fell a tree half a screen away | `get_area_trees` is a rough superset by its own comment; only `tree_valid_at` (an *environment* gate that cannot know the requested region) filtered it. Three paints, trees 20–46 blocks outside, zero x-overlap | **FIXED** — intersect the superset with the request |
| F16 | A starving colony cannot defend itself | Drive ladder is `food_per_cap < 2.0 ⇒ Sustain` **before** `threats > 0 ⇒ Defend`, so food strictly dominates. Five wolves, population 8→4, drive pinned `Sustain` throughout, no `Defend` transition. Not a broken sensor — a single animal DID flip the drive earlier the same session while food was healthy | OPEN — **the two failure modes compose**: any food loss disarms defence. Ladder order is a gameplay ruling |
| F17 | The colonist panel shows "Mining, 4%" for a colonist fleeing wolves | `activity` never cleared: byte-identical progress floats 3,828 ticks apart across two reconnects, while the colony reported `jobs_claimed=1` and *two* colonists displayed a mining job. The source comment claims the release drain clears it; it does not | OPEN |
| F18 | Rejoin your fort and every order you painted is invisible | `list_designations` returns `rev=0 []` on every fresh connection while the server holds 11; only designations made *this session* ever appear | OPEN |
| F19 | Eight colonists idle beside four jobs, and the dashboard explains nothing | All 32 considerations refused on `affordance` (no standable stance), but the player-facing counters read `jobs_unreachable=0 blocked_materials=0` — there is no bucket for this state | OPEN — a refusal reason with no dashboard category is invisible by construction |
| F20 | Cancelling your own mine order mutilates a town plot and *raises* the designation count | The brush clipped an adopted house plot, splitting one region into two: `designations 10 → 11` after a cancel | OPEN |
| F21 | Half a freshly painted mine is dead on arrival | 153 of 311 jobs flagged unreachable within 500 ticks of painting an ordinary 12×12×3 region, with the same job ids churning claim/release | OPEN — related to F1 |
| F22 | Colonists freeze and get teleported by magic | 7 stranding teleports in ~25 min, several with `character_state=Idle`, velocity ≈ 0, `on_ground=true` after a full 60s. The engine's own message calls it a design violation | OPEN — known, but the rate is the news |

**What survived deliberate attack** (recorded so the fix list is not
distorted): zero claim leaks across the whole session, including a mass cancel
of 291 in-flight jobs; 312 jobs minted in one tick without a stall; the
phantom-job and unclaimed-designation backstops never had to fire;
partial-overlap zone cancel genuinely works (pieces keep their ZoneId); the
per-colonist flee response and its bravery thresholds are sound; the
starvation *signal* is correct and even minted a trade mission to sell wood
for food. F16 is a priority defect, not a sensing one.

---

## The loop this list is worked with

1. PLAY the world (persistent, via `play-harness.sh`), see the failure as a player.
2. NAME it in player language, with the log line that shows it.
3. ROOT-CAUSE it in code — the enclosing symbol and the predicate, never a guess.
4. FIX it, disclosing anything the fix reveals about an earlier claim.
5. REPLAY the same world and show the census move.

A row that cannot be shown moving in play does not close, however green its unit
evidence.
