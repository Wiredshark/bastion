# FOUNDING PRESET v1 — **ACCEPTANCE RESULTS** *(live, in progress)*

**Scored against `FOUNDING-PRESET-ACCEPTANCE-PREREG.md` (`3d1f74b4e0`), written before
any of this data existed.** *Binary `6c2991eb` + targeted-spawn driver `8932f91f31`.*
**Message tier (N2), resourced flat arena, fresh userdata per leg, `--no-auth`.**

| bar | state |
|---|---|
| **A1** full preset | ✅ **PASS, falsifier-backed** *(planted PARTIAL goes RED on all 3 fields)* |
| **A2** colonists stay | ⛔ **bar UNSOUND** — ✅ **but A2-B (WORK PULL) replaces it and PASSES 47.6% vs 0.0%** |
| **A3** till→sow→eat | ⚠ **PARTIAL** *(was VOID)* — *stock now witnessed; till+sow observed; **eat** outstanding* |
| **A4** second founding refuses | ✅ **PASS** |
| **A5** terrain refusal | ✅ **PASS — with its N5 control** |
| **B1** z-datum *(§8, not an A-bar)* | ✅ **PASS, falsifier-backed** |
| **B7** binary provenance | ✅ **PASS** — *both packages in the Compiling list; voxygen clean* |
| **F8-INCL** | ✅ **MINE HALF PASSES** — ⛔ *chop half BLOCKED BY THE ARENA; drop+XP unwitnessed* |

---

## ★★★★★★ A5 — **TERRAIN REFUSAL, ON A GENUINELY UNEVEN SITE, WITH ITS CONTROL**

**One leg, one script, both polarities** *(`script-founding-a5-terrain.txt`)*.

### THE SITE

*Resourced arena outcrop: centre offset `(-20, 0)`, half-width 2, height 3 → spans
`x 16362..16366, y 16382..16386`, **top solid 402 ⇒ datum 403**. Flat slab **top solid
399 ⇒ datum 400**. A preset footprint straddling the edge varies by **3 blocks**,
against a ±1 tolerance.*

### THE RESULT

    REFUSED   reason="terrain" pos=(16364.5,16384.5,403.0)
              column=Some(Vec2 { x: 16362, y: 16380 })
              player message: "Uneven ground — the founding kit needs a flatter site
              (every plot column must sit within one block of where you stand)."

    CONTROL   colony founded preset="v1" pos=(16384.5,16384.5,400.0) datum=400
              complete=true jobs=8 designated_regions=3

★★★★ **The named column `(16362, 16380)` is exactly a straddling column** — *x on the
outcrop's west edge, y one block SOUTH of it, so that column is flat (400) while the
origin sits on rock (403).* **The refusal names a real offender, not a placeholder.**

### ★★★★★ N5's CONTROL IS SATISFIED — **the founder does not refuse everything**

*Same founder, same run, same binary: it refused the bad site and founded the good one
seconds later.* **A5's bar cannot be satisfied by a founding action that always
refuses.**

### ★★★★★★ AND "A REFUSED FOUNDING MUTATES NOTHING" IS OBSERVED, NOT ASSERTED

    plot placed lines in the whole run:  3
    all three regions:  x 16377..16386   (the CONTROL site)
    at the outcrop:     ZERO

> ## **THE CONTROL FOUNDING SUCCEEDING IS ITSELF THE PROOF: had the refused attempt
> created a colony, `colony_exists` would have blocked it.**

★★ *Two independent confirmations of the same property — the plot-line count and the
control's success.*

---

---

## ★★★★★★ A1 — **FULL PRESET, WITH §8 B5's CORRECTED PLANTED FAILURE**

**Three states, same script (`script-founding-preset-smoke.txt`), three builds.**

| # | build | `elements=` · `complete=` · `designated_regions=` | plot lines |
|---|---|---|---|
| **1** | **REAL** | `stockpile,farm,bed` · **true** · **3** | stockpile · farm · bed |
| **2** | ★★★★★ **MUTANT** *(skip `DesignationKind::Farm` at the placement loop)* | ⛔ `stockpile,bed` · **false** · **2** | stockpile · bed |
| **3** | **REVERTED + REBUILT** | `stockpile,farm,bed` · **true** · **3** | all three |

### ★★★★★★ WHY THIS PLANT IS THE ONE §8 B5 ASKED FOR

**The packet's ORIGINAL plant was "founding action with preset-placement disabled must
NOT emit the founded line."** *That removes the subject and the witness together — it
proves the emit is coupled to something, never that it means FULL preset.* **B5 called
it vacuity costume #3.**

> ## **THE CORRECTED PLANT KEEPS THE FOUNDING HAPPENING AND MAKES THE WITNESS TELL THE
> TRUTH ABOUT IT: the founded line STILL EMITS, carrying `complete=false`.**

★★★★★ **All three discriminating fields moved** — *`elements=` shortened, `complete=`
flipped, `designated_regions=` dropped 3→2.* **A1's witness discriminates full from
partial, which is exactly the property the bar claims and the old plant could not have
shown.**

★★ *A4's `reason="colony_exists"` refusal also fired in the reverted run — the second
founding is still refused, so this leg re-confirms A4 alongside A1.*

---

## ⛔★★★★★★ A2 — **FAILS AS REGISTERED, AND ITS PLANTED FAILURE DOES NOT DISCRIMINATE**

### FIRST, THE INSTRUMENT HAD TO BE BUILT

**Nothing in the tree emitted a colonist BODY position periodically.** *The only
position-bearing lines are job sites (where WORK is, not where bodies are) and the
fail-safe rescue.* ★★★★★ **Measuring A2 off job-site arrivals would have been unsound
in the exact direction that matters: the plant removes the designations, so there would
be NO arrivals at all, and "they left" would have been read from an ABSENCE.**

**Built: `pos` added to the existing `BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG`** —
*same env gate, same join, no new query.*

### THE MEASUREMENT

    GREEN  (full preset, designations placed):  max 22.96 blocks from F
           per-uid:  6.3 · 20.5 · 5.3 · 17.7 · 9.9 · 16.1 · 12.0 · 23.0

    MUTANT (NO designations at all, 0 plot lines): max 21.00 blocks from F
           per-uid:  5.3 ·  6.6 · 5.0 · 21.0 ·  3.5 ·  2.2 · 12.1 ·  3.8

    R (registered pre-data): 16

> ## **BOTH ARMS EXCEED R. AND REMOVING *EVERY* DESIGNATION MOVED THE MAXIMUM BY TWO
> BLOCKS — IN THE WRONG DIRECTION.**

### ★★★★★★ WHY THE PLANT CANNOT WORK — **B4's correction was still not enough**

**Ben's observed failure was colonists marching to THE OLD COLONY'S COORDINATES.** *That
requires an **ATTRACTOR** somewhere else in the world.*

★★★★★ **On a fresh one-colony world there is nowhere to march to.** *Removing the
designations removes the WORK, but it does not create a distant destination — so the
colonists just wander locally, which is what both arms show.*

> ## **§4's ONE-COLONY BOUNDARY MAY ALREADY HAVE MADE A2's FAILURE MODE STRUCTURALLY
> UNREACHABLE — WHICH IS EXACTLY WHAT §4 CLAIMED IT WOULD DO.**

*"The leash-march fallthrough becomes impossible: spawn-bind only happens through the
founding action."* ★★★ **If that holds, A2 cannot be demonstrated empirically on this
world, and the honest score is neither PASS nor "mechanism broken".**

### ⚠ AND ONE OBSERVATION THAT RUNS AGAINST THE BAR'S PREMISE

**WITH designations, FOUR colonists ranged past 16 (20.5, 17.7, 16.1, 23.0). WITHOUT
them, ONE did (21.0).** ★★★ *Directionally, the designations made colonists range
**further**, not closer — because work gives them places to go.*

⚠ **n=1 per arm and these numbers are dominated by ordinary wander, so this is
registered as an observation, not a finding.** *But it is the opposite of what "work at
F retains them" predicts, and it should be tested properly before that premise is
relied on again.*

### WHAT I AM **NOT** DOING

⛔ **NOT re-baselining R to make A2 green** *(scoring refusal #1).* **R was derived and
registered before the data; it failed; that is the result.**

★★ *And I note my derivation was itself loose — it ADDED spawn scatter (7.07) to work
travel (~8), but a colonist standing at the farm's far corner is ~8 from F regardless
of where it spawned. **The registered R was too generous by construction and STILL both
arms blew it.***

### THE ROW THIS OPENS

1. **A2 needs a bar that its plant can move** — *either an attractor-based reproduction
   (a second site to march to, which §4 forbids), or a different measure entirely
   (e.g. "colonists REACH the work", which the 36 `arrived at job site` lines already
   witness).*
2. **R must be derived from measured wander, not from a construction** — *and from more
   than one run per arm.*

---

## ★★★★★★ F8-INCLUSION — **THE MINE HALF CLOSES v5's REGISTERED PREDICTION**

**v5's results doc registered:** *"the next scored run that designates mining MUST show
`bastion: job completed` firing."* ★★★ **This is that run. It fires.**

    designation placed kind=Mine jobs=75
    bastion: job completed job=90 kind=Designated(Mine)
        pos=Vec3 { x: 16364, y: 16382, z: 402 } completed_kind=Some(Rock)   … ×37

| | |
|---|---|
| **Mine completions** | **37**, all `completed_kind=Some(Rock)` |
| **Bed completions** | **6**, `Some(Air)` |
| ★★★★ **total vs DISTINCT positions** | **43 / 43 — ZERO REPEATS** |
| haul delivered · job claimed · arrivals | 22 · 133 · 152 |

### ★★★★★★ THE ECONOMY CHAIN CLOSED ON ITS OWN

**The handoff warned: *"Bed jobs need `BUILD_MATERIAL_ITEM` (stone). The founding stock
is seeds only, so the bed stays unbuilt until something is mined."***

> ## **SOMETHING WAS MINED, AND SIX BEDS GOT BUILT. mine → stone → haul → build, END TO
> END, UNPROMPTED.**

★★★ **And that is the drop evidence** — *not a witness line, but a downstream
consequence that could not have occurred without drops.* ⚠ **Recorded as INFERRED, not
observed.**

### ★★★★ AND A DEFECT-1 DATA POINT, FREE

**43 real Mine completions on ROCK, 43 distinct positions, no cell completing twice.**
*v4's 281-completion trap cell was **`Leaves`**.* ★★★ **Rock consumes correctly; the
foliage hypothesis narrows further** *(task #86)*.

---

## ⛔★★★★★ TWO FINDINGS FROM THE CHOP HALF

### F8-C1 · **THE ARENA'S TREES CANNOT BE CHOPPED — §2's CLAIM IS FALSE FOR CHOP**

    driver:  sent BastionPlaceDesignation kind=Chop region=(16400,16384,400)..(16404,16388,405)
    client:  "No trees rooted in the marked area."      (68 ms later)
    server:  designation placed kind=Chop      -- NEVER APPEARS

**Cause, read in `in_game.rs`:** *the Chop branch resolves its fell-set through the
**World oracle** — `get_area_trees` candidates → `tree_valid_at` confirm → flood-fill.*

> ## **THE ARENA'S TREES ARE BLOCKS PAINTED AT CHUNK GENERATION BY
> `apply_resourced_features`. THE WORLD SIM HAS NO TREE RECORD THERE, SO THE ORACLE
> CORRECTLY REPORTS NONE.**

★★★★★ **§2 says the resourced arena provides "a tree cluster (chop)" and becomes "the
standing resourced proving ground".** *It does for **mine**. It cannot for **chop**,
by construction.* **The chop half of F8-inclusion is unreachable on this arena.**

### F8-C2 · ⚠ **THE REFUSAL HAS NO SERVER-LOG WITNESS** *(and I nearly misreported it)*

**The refusal IS delivered — to the CLIENT, as `CommandInfo`.** *It is absent from the
server log entirely.*

★★★ **My first read of this was "the designation vanished silently".** *That was wrong:
it vanished silently **from the server log**, which is the surface a scored run reads.*
**Corrected before it reached a finding** — *the driver's chat capture is what caught
it, which is the mirror image of smoke F-3 (where the driver's view was the misleading
one).*

> ## **NAME-THE-LINE APPLIES SERVER-SIDE: A REFUSAL A SCORED RUN CANNOT SEE IS NOT A
> WITNESSED REFUSAL.**

★★ *Same class as F-2 (seed drop) and the drop/XP gap below — three unwitnessed
outcomes in one feature.*

### ⚠ F8-C3 · **drop + XP HAVE NO WITNESS, AND MY OWN GREP LIED**

**§5's F8 row asks to "observe `bastion: job completed` with drop+XP".** *The
completion is observed; **drop and XP are not.***

★★★★★ **A grep for `emit_drop|item.*drop|xp` returned 8 — ALL FALSE POSITIVES.** *Two
were `food stock sample` lines; the rest matched **`xp`** inside the migration name
`V34__remove_immunemelee_be`**xp**`losion`.* **A two-letter pattern matches inside
words** — *the naming-claim trap at its most literal, caught only by reading the
matches instead of the count.*

---

## ★★★★★ A3 — **LIFTED FROM VOID TO PARTIAL (F-2 closed, `37132be674`)**

**Smoke F-2 held A3 VOID under scoring refusal #4:** *the founding dropped
`FOUNDING_SEED_STOCK` with **no emit**, so "founded WITH stock" was UNREAD rather than
true.* **Built the witness:**

    bastion: founding stock dropped item="common.items.bastion.wheat_seeds" amount=8
        pos=Vec3 { x: 16384.5, y: 16384.5, z: 400 }

★★★ **`amount` is read BACK OFF THE ITEM, never echoed from the constant** — *`set_amount`'s
Result is discarded at that site, so the constant is INTENT and only the item carries
EFFECT.* **Echoing the constant would have been the F8 defect in miniature.**

**RED-DEMONSTRATED (bogus asset ⇒ `new_from_asset` Err ⇒ no drop):**

| build | drop lines | `colony founded` |
|---|---|---|
| **GREEN** | **1**, `amount=8` | 1 |
| ★★★★ **MUTANT** | ⛔ **0** | ✅ **still 1** |
| **REVERTED** | **1**, `amount=8` | 1 |

> ## **THE PLANT REMOVED THE DROP WITHOUT DELETING THE SUBJECT — SO THE WITNESS
> DISCRIMINATES RATHER THAN BEING MERELY COUPLED TO SOMETHING.**

★★ **A second control came free:** *the script founds TWICE and the second is refused; the
drop line appears **exactly once**.* **The emit's position relative to the refusal path
is proven by the count, not asserted** — *and it re-confirms "a refused founding mutates
nothing" on a third independent channel.*

### A3's REMAINING GAP — **narrowed, not closed**

| stage | state |
|---|---|
| **stock** | ✅ **witnessed** — 8 seeds, live |
| **till** | ✅ observed — **30** in a 300-tick leg |
| **sow** | ✅ observed — **5** |
| ⚠ **eat** | ⛔ **still unexercised** |

★★★ **§8 B3's standing risk holds: with seeds only, the first EAT waits on a HARVEST.**
*That is a run-length question, not a defect — and §5's "minutes-scale" claim remains
unproven for A3 specifically, exactly as the handoff said.*

---

## ★★★★★★ A2-B · **THE SUCCESSOR BAR — "DOES WORK PULL?" — BUILT AND PASSING**

**A2's original bar could not work** *(no attractor on a one-colony world; both arms
wandered to ~21–23)*. **Its successor tests §8 B4's actual CLAIM rather than assuming
it:**

> ## **B4 SAYS "WHAT HOLDS COLONISTS AT F IS THE WORK BEING AT F." THAT PREDICTS A
> CONVERSE: PUT THE WORK 20 BLOCKS AWAY AND THEY SHOULD FOLLOW IT.**

**MEASURE: fraction of colonist position samples within 8 blocks of the OUTCROP
(20 blocks west of F).**

| arm | samples | **within 8 of OUTCROP** | within 8 of F |
|---|---|---|---|
| ★★★ **work designated at the outcrop** | 704 | ✅ **335 = 47.6%** | 314 = 44.6% |
| **no designations at all** *(the A2 mutant)* | 1008 | ⛔ **0 = 0.0%** | 568 = 56.3% |

★★★★★★ **ZERO OF 1008. Not "rarely" — colonists NEVER visit the outcrop when no work is
there.** *So the 21-block excursions in the no-work arm were wander in other directions,
not drift toward anything.*

### WHY THIS BAR IS SOUND WHERE A2's WAS NOT

| | A2 (max distance from F) | ★★★ A2-B (concentration at the work) |
|---|---|---|
| **with work** | max 22.96 | **47.6% present** |
| **without work** | max 21.00 | **0.0% present** |
| **separation** | ⛔ **2 blocks — indistinguishable** | ✅ **47.6 points — absolute** |

★★★★ **And I rejected the obvious alternative first:** *"colonists REACH the work,
planted by removing the designations" is TAUTOLOGICAL — no jobs means no job sites to
arrive at, so that plant cannot fail.* **Disguise #1/#4. The work-pull form avoids it
because BOTH arms have colonists, a world, and a window — only the work's LOCATION
changes.**

### ★★★★★ WHAT IT ESTABLISHES

1. **§8 B4's mechanism is CORRECT and now DEMONSTRATED, not assumed.** *Work is the
   retention mechanism; there is no anchor, exactly as B4 said.*
2. **Ben's observed failure is explained rather than merely prevented** — *his colonists
   marched because the WORK was at the old colony. The pull is real; §4's one-colony
   boundary removes the far attractor, and this run shows what the pull does when one
   exists.*
3. ⚠ **n=1 per arm.** *The separation is 47.6 vs 0.0, which no plausible wander model
   closes — but the number of runs is one, and that is stated rather than buried.*

**Evidence:** `script-founding-a2b-work-pull.txt` ·
`server-founding-a2b-workpull.log` · control arm re-read from
`server-founding-a2-MUTANT.log`.

---

## NEXT
3. **F8-inclusion** *(designate the arena's tree and outcrop; observe a real
   `job completed` with drop+XP)*.
