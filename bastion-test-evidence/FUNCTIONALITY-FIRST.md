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

---

## P0 — the colony does not look alive

| # | Broken thing (player language) | Root cause (measured) | State |
|---|---|---|---|
| F1 | Colonists walk into a wall and stall there | A* starts at `PathLength::Small` (500 iters); escalation requires standing near the search START, which a partial route already broke — so it resets to Small forever, and the give-up route aims at the explored node nearest the goal: the wall face | design panel + fix in flight |
| F2 | Colonists starve next to food | Their meal is a loose ground item the colony's own haulers merge away mid-walk; the eat job then DIED (635 preempts → 135 jobs → **33 meals**) | FIXED — re-aims at the nearest food (`5bb790c99c`), needs a played confirmation |
| F3 | Nobody eats at all when any job wants that food | SELF-CATCH: my in-flight-ingredient guard protected the eater's own meal (an EatFrom job carries food as its `required_item`) | FIXED (`5bb790c99c`), needs a played confirmation |
| F4 | 6 of 8 colonists idle in a town full of work | Partly F1; the remainder is unproven — the claim path must be measured per colonist, not per job | OPEN — needs a per-colonist claim census |

## P1 — the adopted village is scenery, not a home

| # | Broken thing | Root cause | State |
|---|---|---|---|
| F5 | Colonists stand in a ripe field and do nothing | Harvest fires only on `WheatYellow` at `Growth >= FARM_GROWTH_MAX` in a registered column; village fields are `WheatGreen`/`Flax`/`Corn`/`Tomato` at Growth 0, explicitly reserved by a code comment | OPEN — Ben's own words authorise using existing farmland; policy nuance banked |
| F6 | A furnished kitchen is invisible | `cook_stations` has exactly ONE writer: a completed CookStation *build* | OPEN — two-line push in the bed-scan arm |
| F7 | An adopted barn is a decorative rectangle | `stockpiles` only ever holds painted regions; no chest/container sprite is read | OPEN — 1-block Region per chest |
| F8 | Colonists sleep on the ground beside made beds | *(was broken; the plot scan now registers village beds)* | FIXED — `adopted_beds=2`/house live |

## P2 — systems that exist but cannot be believed

| # | Broken thing | Root cause | State |
|---|---|---|---|
| F9 | Every village pays exactly the same for wood | `ratio_min=1.0 ratio_max=1.0 ratio_distinct=1` across all 194 sites — a constant wearing a site's clothes; item 29 bar 2 FAILS | OPEN |
| F10 | Starving has no consequence | Nothing reads hunger=0 except mood; no health pressure, no death | OPEN — needs a design ruling on lethality, banked |
| F11 | Colonists cannot die | Health reached 0.0 under repeated smite: no death event, no despawn, population unchanged | OPEN — item 36's true first step |
| F12 | Colony scale is unmeasured | The 8/16/32 legs ran under different co-load; VOID for the guard question | OPEN — re-run on the VM fleet, which is now quiet |

---

## The loop this list is worked with

1. PLAY the world (persistent, via `play-harness.sh`), see the failure as a player.
2. NAME it in player language, with the log line that shows it.
3. ROOT-CAUSE it in code — the enclosing symbol and the predicate, never a guess.
4. FIX it, disclosing anything the fix reveals about an earlier claim.
5. REPLAY the same world and show the census move.

A row that cannot be shown moving in play does not close, however green its unit
evidence.
