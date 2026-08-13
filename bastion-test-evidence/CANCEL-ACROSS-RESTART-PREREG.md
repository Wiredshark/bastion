# CANCEL ACROSS RESTART — **PRE-REGISTRATION**

Written before any run. Closes the item `COLONY-PERSISTENCE-RESULTS.md` registered as
**open**: *"Not that a cancelled designation stays cancelled across a restart BY TEST. It
follows from the store being the live set, but no bar exercised it."*

## 1 · WHY THIS IS NOT ALREADY COVERED

Persistence passed 4/4 by showing orders **survive**. The mirror case — an order the
player **erased** must not come back — is a different claim, and it is the one a resurrection
bug would hide behind. A save that appended history rather than snapshotting the live set
would pass every persistence bar and still resurrect cancelled work on every restart.

`cancel_region` removes from `designated` (by intersection, subtracting AABBs) and the
save writes `designated ∪ pending_restore`, so the erasure *should* be carried. **Should
is not tested.**

## 2 · THE WITNESS — a kind-specific named emit, not a count

The founding preset's three plots are **adjacent by construction** (farm x −7..−3,
stockpile −2..+2, bed −3..−2), so any full-cover cancel of one clips a neighbour and
leaves remainder pieces. **A count of orders is therefore the wrong instrument** — it
would move for reasons that have nothing to do with the bar.

The right witness is the farm's own registration line, which fires only when a Farm
designation is placed:

```
bastion: farm plot registered, per-column surface resolved … unresolved=0
```

## 3 · THE BARS

### C1 · **A CANCELLED ORDER DOES NOT COME BACK**
- Boot 1: found (3 plots, farm registered), then **cancel the farm region**, then hold
  past the 60 s save boundary.
- **PASS:** boot 2 replays orders and emits **no** `farm plot registered`.
- **FAIL:** the farm returns — the save carried an erased order.

### C2 · **THE OTHER ORDERS STILL SURVIVE** — the matched control, in the same run
- **PASS:** boot 2 still emits `stockpile zone registered` and still replays a non-zero
  order count.
- Without C2, C1 would pass on a build where **nothing** restored — which is exactly the
  wrong reason to be green, and precisely how a "fix" could sail through.

### C3 · **THE UNCANCELLED CONTROL** — already recorded
- The persistence row's control run on the golden save emitted `farm plot registered`
  and `jobs=8`. C1's red side is therefore **already measured** on a matched build.

### PLANT
- Make the save write an **append-only history** (union with the already-saved orders
  instead of snapshotting the live set) ⇒ **C1 red**: the cancelled farm returns.
- This is the realistic defect, not an invented one: it is what "just accumulate the
  orders" would look like if written without thinking about erasure.

## 4 · WHAT I WILL **NOT** DO

1. **I will not score C1 on an order count.** Adjacent plots make counts move for
   irrelevant reasons; the bar reads a kind-specific emit.
2. **I will not accept C1 without C2 in the same run.** "Nothing came back" satisfies C1
   trivially and is a worse outcome than the bug.
3. **I will not skip the 60 s save precondition.** Asserted by content, printed above the
   result, as with every restart test in this session — three of which were VOID before
   they were anything.
