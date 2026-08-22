# LOOKING SWEEP 2 — the first one where the instrument could actually see

Run on the 46-house village with a readable map (`H` built, `T` canopy, `^` rock,
`:` high ground, `.` ground, `C` colonist, `i` item), `NEEDS_DECAY_MULT=3`.

## Does it look like a town?

**Yes.** This is the first sweep where that question could be answered at all —
the previous map drew walls, roofs, cliffs and hillsides as one `#`.

```
18344 |::::::::::::.HHHHHHHHHHHHHHH.........................
18343 |:::::::::::..HH^^^HHHHHHHHHH.........................
18342 |:::TTT:::::..HH^.^HHHHHHHHHH.........................
18341 |::TTTTT:::...HH^.^CHHHHHHHHH.........................
18340 |:TTTTTTT::...HH^^^HHHHHHHHHH.........................
...
18332 |H^i.^HHHHHHHHHHH.....................................
18331 |H^^^^HCHHHHHHHHH.....................................
```

Read it: large contiguous buildings, and **inside them stonework (`^`) around
open floor (`.`)** — rooms, with hearths or stone walls. A copse of trees to the
west. A slope rising away to the north. Flat ground where the colony works.

**`C` at 18331 and 18341 are colonists standing inside houses**, which answers
"are the buildings used, and who is inside them" directly.

| glyph | count | |
|---|---|---|
| `.` | 1171 | flat ground |
| `:` | 887 | high ground |
| `H` | 416 | **built** |
| `^` | 292 | rock / stonework |
| `T` | 37 | trees |
| `C` | 2 | colonists indoors |
| `i` | 3 | items on floors |

## Follow one colonist for two minutes

**Oafish Sheca**, four samples:

| | hunger | rest | drive | doing |
|---|---|---|---|---|
| 1 | 0.518 | 0.759 | Work | walking |
| 2 | 0.439 | 0.720 | Work | **Cook** |
| 3 | 0.349 | 0.675 | Work | walking |
| 4 | 0.270 | 0.635 | Work | walking |

**Narratable:** *she walks to a kitchen, cooks, and moves on to the next job,
getting steadily hungrier and more tired as the day goes.* You can tell what she
wanted. Nothing she did was something a person obviously wouldn't.

## The colony behind her

```
colonists=8   food_stock=678   jobs_claimed=8/8   jobs_unreachable=0   favor=20.0
```

Every colonist holds a job, nothing is unreachable, and the pantry is deep.

## What is still wrong

- **It does not last.** Earlier legs show this same colony at `fed=0 rested=2
  stuck=8` after ~1.5 game-days. Sleeping and eating now happen; they do not yet
  keep up.
- **Sleep is proof-of-life, not a pattern** — 2 events.
- **Teleports persist.** The above-grade rescue watch still records zero
  verdicts, so a colonist stranded on a roof still leaves by magic.
- **`H` for "wood" conflates house walls with any timber**, including fences and
  bridges. Good enough to see a town; not good enough to audit room layout.
