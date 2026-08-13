# FOUNDING PRESET v1 — **ACCEPTANCE RESULTS** *(live, in progress)*

**Scored against `FOUNDING-PRESET-ACCEPTANCE-PREREG.md` (`3d1f74b4e0`), written before
any of this data existed.** *Binary `6c2991eb` + targeted-spawn driver `8932f91f31`.*
**Message tier (N2), resourced flat arena, fresh userdata per leg, `--no-auth`.**

| bar | state |
|---|---|
| **A1** full preset | ✅ **PASS, falsifier-backed** *(planted PARTIAL goes RED on all 3 fields)* |
| **A2** colonists stay | ⏳ **not yet run** |
| **A3** till→sow→eat | ⚠ **VOID by refusal #4** — *seed drop has no witness (F-2)* |
| **A4** second founding refuses | ✅ **PASS** |
| **A5** terrain refusal | ✅ **PASS — with its N5 control** |
| **B1** z-datum *(§8, not an A-bar)* | ✅ **PASS, falsifier-backed** |
| **B7** binary provenance | ✅ **PASS** — *both packages in the Compiling list; voxygen clean* |
| **F8-INCL** | ⏳ **not yet run** — *needs its own `designate` step* |

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

## NEXT

1. **A2** *(§8 B4 — found WITHOUT designations; colonists must leave R=16)*.
3. **F8-inclusion** *(designate the arena's tree and outcrop; observe a real
   `job completed` with drop+XP)*.
