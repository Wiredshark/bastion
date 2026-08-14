# THE CHOP YIELD — **PRE-REGISTRATION**

Written before any code change, and before the instrument it needs exists. Successor
registered by `ARENA-TREES-RESULTS.md`: *"the 15-Wood yield prediction becomes testable
end to end."*

## 1 · THE PREDICTION, DERIVED AND ALREADY ON RECORD

`RESOURCED_TREES` gives trunk heights **5, 6, 4**. Only Wood yields — leaves clear free —
so:

> **A full chop of the arena's cluster yields exactly 15 Wood.**

That number was derived in the arena-trees prereg **before** the fell-sets ever resolved,
and it has not moved since. The fell-set row proved the trees *resolve* (`trees=3`,
`cells=[14,15,13]` = trunk + a 9-cell leaf crown each). **It did not prove they yield.**

## 2 · THE INSTRUMENT MUST COME FIRST — F8-C3 IS STILL OPEN

F8-C3 recorded that **drop and XP have no witness**. Re-verified now, before building:
grepping the completion path for a yield emit returns only *"access anchors dropped
(ladder erased)"* and *"WINDED — run dropped to walk"* — **neither is a yield witness.**

So the yield is currently **unmeasurable**, exactly as the founding stock was before A3's
seed witness was built. *(A3 travelled VOID → PARTIAL → PASS on that same shape, and the
first step was building the instrument.)*

**This row's first deliverable is the witness, not the number.**

## 3 · THE BARS

### Y1 · **THE WITNESS EXISTS AND NAMES ITS PRODUCER**
- A completion emit carrying the **item, the amount, and the job** — read back off the
  produced item, not from the placement-time tally.
- **Why read-back:** `wood_count` is frozen at placement (it sets the work threshold). An
  emit that printed *that* would report the PREDICTION, not the yield — a witness that
  cannot disagree with the thing it is checking. The seed witness had the same trap and
  the same fix.

### Y2 · **THE YIELD IS 15** — the registered number, live on the arena
- **PASS:** summed Wood across the cluster's three completions = **15**.
- **FINDING (not failure):** any other total. 5+6+4 is arithmetic; a different number
  means the yield rule is not "one Wood per Wood cell", and that is worth more than the
  pass.

### Y3 · **PER-TREE ATTRIBUTION**
- **PASS:** the three completions yield **5, 6 and 4** individually, in some order.
- A total of 15 could hide 7+4+4. The per-tree split is what makes Y2 non-vacuous — the
  same reason the fell-set row counted cells per tree instead of trusting `trees=3`.

### PLANTS
1. **Yield the leaf cells too** ⇒ Y2 red at 15 + 27 = 42, and Y3 red per-tree. Confirms
   the bar reads Wood specifically and not "everything felled".
2. **Emit the placement-time `wood_count` instead of the read-back amount** ⇒ Y1's
   *claim* survives while the witness becomes incapable of disagreeing. **This plant must
   redden a test that compares the two**, or Y1 is decoration.

## 4 · WHAT I WILL **NOT** DO

1. **I will not report the yield from `wood_count`.** That is the threshold's input, not
   the drop's output; using it would make Y2 a tautology.
2. **I will not score Y2 without Y3.** A matching total with the wrong split is a false
   green, and I know the shape well enough now to expect it.
3. **I will not run this on real worldgen.** Real trees saturate `TREE_FELL_CELL_CAP`
   (all four measured exactly 2048), so their fell-sets are truncated and the yield is
   *not* the whole tree. The arena's small trunks are the only place 15 is even
   well-defined — a constraint the arena-trees row measured and this row inherits.
