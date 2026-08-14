# HAUL THROUGHPUT vs POPULATION — **PRE-REGISTRATION**

Written before any code change. Opened by `A3-N4-RESULTS.md`: peak `food_stock` **6 at
n=8, 2 at n=4, on identical harvest counts of 10**, with the mechanism recorded as
**untested**.

## 1 · WHAT THE CODE READ ALREADY SETTLES

`food_stock` is not a harvest counter. It is computed each sample as:

```rust
for (item, pos) in (&pickup_items, &positions).join() {
    …FOOD_DEFS.contains(&def)… if board.stockpile_at(cell).is_some() { food_stock += … }
}
```

> **`food_stock` counts food ITEMS LYING IN A STOCKPILE CELL.** Not inventories, not
> totals, not harvest.

So a harvested crop contributes **nothing** until something carries it to the stockpile.
With 10 harvested in both runs and 6 vs 2 arriving, the difference is in **delivery**.
That is now a code-supported deduction rather than a story — but *which* delivery
constraint binds is still unmeasured, and that is this row.

**It also clarifies what A3 has been measuring all along:** A3's "food_stock rose"
criterion is really *"food got hauled"*. Worth stating, because A3 is a scored bar and
its subject was subtly misdescribed.

## 2 · THE CANDIDATE MECHANISMS — and they are not the same claim

1. **Haul throughput scales with population.** Fewer colonists ⇒ fewer haul trips
   completed inside the window ⇒ less food in the stockpile.
2. **Haul PRIORITY loses to other work.** The mix shifts with population; hauling may
   simply be outcompeted, independent of how many bodies exist.

Both predict "less food at n=4". **A count of arrivals alone cannot separate them** — the
same trap A2's original bar fell into, where two mechanisms produced the same number.

## 3 · THE INSTRUMENT

There is **no haul witness** — grepping the completion paths for a haul emit returns
nothing. So, as with the seed, the yield and the XP: **the instrument is the first
deliverable.**

Emit on haul completion, carrying the **item**, the **destination cell**, and the
**colonist** — so arrivals can be counted *and* attributed.

## 4 · THE BARS

### H1 · **HAUL IS WITNESSED**
- **PASS:** a named emit fires on haul completion, at least once, in a run where
  `food_stock` rises.
- If `food_stock` rises with **zero** haul emits, the witness is on the wrong path —
  reported as a finding, not patched around. *(The chop row nearly made this exact
  mistake: the base cut is not where drops happen.)*

### H2 · **ARRIVALS EXPLAIN THE STOCK**
- **PASS:** haul-completion count ≥ peak `food_stock` in the same run, and the two move
  together across populations.
- **FINDING:** stock rising *without* matching arrivals would mean food enters the
  stockpile by some path other than hauling — which would refute §1's reading.

### H3 · **THE POPULATION COMPARISON** — the row's actual question
- Run n=8 and n=4, same script, same window.
- **PASS (mechanism 1 supported):** haul completions at n=4 are **materially fewer**,
  roughly tracking the population ratio.
- **REFUTED:** comparable haul counts at both populations ⇒ throughput is **not** the
  constraint, and mechanism 2 (priority/mix) moves to the front. **That is the more
  interesting outcome and it is registered as such.**

## 5 · WHAT I WILL **NOT** DO

1. **I will not conclude "hauling is the cause" from arrival counts alone.** §2 names two
   mechanisms with the same signature; H3 can support one and only *fail to exclude* the
   other. Any stronger claim needs a priority-mix measurement I am not making here.
2. **I will not re-derive `food_stock`.** It is read, quoted, and cited — the number stays
   the code's, not mine.
3. **I will not shorten A3's 28,000-tick window** for the comparison legs. The eat gate
   sets it; a shorter window measures a different question.
4. **I will not treat n=1 per population as a rate.** Two runs give a direction, not a
   coefficient, and the disposition will say so.
