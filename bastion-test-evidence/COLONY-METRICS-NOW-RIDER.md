# COLONY-LEVEL METRICS — THE NOW-RIDER (DECISIONS #107)

**The charter's rider: the colony mind is born with SENSES, not blind.** *These
accumulate as DIAGNOSTICS in current work, riding whatever touches the board
next — long before any arbiter consumes them.*

★ **Written after checking the inputs exist, because I specified item 8's bar
without doing that and four of six measures had no witness.** *The availability
column below is a read, not an assumption.*

---

## INPUT AVAILABILITY — READ AT `34003b74f8`

| metric | inputs | status |
|---|---|---|
| **food-days remaining** | stockpile food sums (63 refs), colonist roster, eat rate | ★ **computable** — the rate must be read from live config, not assumed |
| **housing ratio** | bed machinery (38 refs), roster | ★ **computable** |
| **material buffers** | stockpile sums per item class | ★ **computable** |
| **threat pressure** | `pub hostiles: u8` + per-agent `target.hostile` | ⚠ **PRIMITIVE — a COUNT exists, a PRESSURE model does not** |

> ★★ **Threat is the honest gap.** *`hostiles: u8` answers "how many," not "how
> dangerous, how close, how sustained." **Ship the count, name it a count, and do
> NOT let a future arbiter read it as pressure** — that would be the
> aggregate-late error committed at design time.*

---

## THE FOUR METRICS — **DIAGNOSTICS ONLY, NEVER CLAUSE TERMS**

    b5_colony_food_days          f32   stock / (roster x eat_rate) — SAMPLE, not a max
    b5_colony_housing_ratio      f32   beds / roster
    b5_colony_buffer_<class>     u32   per material class, SEPARATE, never summed
    b5_colony_hostiles_seen      u32   COUNT of hostiles observed — NOT a pressure

### ★★★ DESIGN RULES, EACH EARNED THIS WEEK

1. **KEEP THEM SEPARATE. No composite "colony health" score.** *A single number
   answers one question and no adjacent one — and the arbiter's whole job is to
   weigh these against each other, which it cannot do if they arrive pre-collapsed.*
2. ★ **Each metric names its producer in its doc comment.** *Two writers on one
   colony metric would be the union-counter defect at colony scale; this week
   audited that class out of `bastion_jobs` at retail price.*
3. **Sample-with-timestamp, not run-max.** *`food_days` at cycle 3 and at cycle 5
   is a TREND; `max(food_days)` is a number that cannot fail. The endurance row's
   measure 4 already established that trends are the only readable shape for a
   slow leak.*
4. ★★ **Each must be able to take more than one value in a real run.** *A metric
   that is structurally constant is a dead sentinel — three of those turned up
   this week (item 6's refusals, `instance_id`, the significance criterion).
   **Before shipping each one, name the run condition that moves it.***
5. **Emit under the existing diagnostics gate. Never a clause term, never a gate
   input** — *until an arbiter exists and is itself gated.*

---

## WHAT THESE CANNOT SEE — **STATED NOW SO THE ARBITER INHERITS THE LIMIT**

- ★ **No production RATES, only stocks.** *`food_days` divides a stock by an
  assumed rate; it will not notice a farm loop that has stopped until the stock
  drains. **The rate itself is the better metric and does not exist yet.***
- **No spatial structure.** *Housing ratio says 8 beds / 8 colonists; it cannot
  say they are a 400-block round trip from the work face.*
- **Threat is a count with no decay, distance, or severity.**

> ★★★ **The arbiter must therefore never be given authority its senses cannot
> support.** *Sustain/Grow/Defend/Expand weighted by these four is a colony that
> can tell it is HUNGRY but not that it is SLOWING — and "Grow while the harvest
> rate silently falls" is the exact failure this instrumentation gap would
> produce.*

★★ **RECOMMENDATION FOR THE DESIGN DOC: the first arbiter ships with SUSTAIN and
GROW only.** *Defend needs a threat model that does not exist; Expand needs
spatial structure that does not exist. **Two drives with real senses beat four
with two imaginary ones** — and the charter's "weights on existing generators,
never orders" makes a two-drive arbiter genuinely useful on day one.*

## RIDING ORDER

**First candidates ride the COLONY PRESENCE row** — it already touches colony
lifetime (founding → disband), which is exactly where a colony-scoped metric
block belongs. *`housing_ratio` and `buffer_<class>` are the cheapest and need no
new rate reads.*
