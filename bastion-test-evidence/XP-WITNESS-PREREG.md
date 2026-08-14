# THE XP WITNESS (F8-C3's other half) — **PRE-REGISTRATION**

Written before any code change. `CHOP-YIELD-RESULTS.md` closed F8-C3's **drop** half and
registered this one open: *"F8-C3 named drop AND XP; `grant_xp` fires at the base cut and
still has no emit."*

## 1 · THE THREE GRANT SITES, READ

| line | site | amount |
|---|---|---|
| `:787` | the shared completion path | `COMPLETION_XP` |
| `:14816` | the per-completion arm | `COMPLETION_XP` |
| `:14931` | **the chop base cut** | `COMPLETION_XP × fell.wood_count` |

`COMPLETION_XP = 8.0`.

## 2 · THE DERIVED NUMBER

The arena cluster's trunks are **5, 6, 4** Wood. The chop grant is per-tree at the base
cut, scaled by the whole tree:

> **Cluster XP = 8 × (5 + 6 + 4) = 120**, split **40 / 48 / 32**.

## 3 · ⚠ THE SAME TALLY IS RIGHT HERE AND WAS WRONG NEXT DOOR

The yield row refused `fell.wood_count` as the drop amount — it is frozen at placement, so
a witness carrying it would report the prediction rather than the yield.

**For XP that same tally is the CORRECT source.** XP is granted once, at the base cut, for
the whole tree; there is no per-cell XP event to read back from. The number is not
"trusted" here for lack of an alternative — it *is* the quantity the game awards.

**Same value, opposite verdicts, for a reason that is about the QUANTITY BEING MEASURED
rather than about the value's provenance.** Naming it so the yield row's rule is not
cargo-culted into a place it does not apply.

## 4 · THE BARS

### X1 · **XP IS WITNESSED AT ALL THREE SITES**
- **PASS:** an emit carrying the **work/skill**, the **amount**, and the **colonist**,
  at each of the three grant sites.
- Three sites, one shape: an emit at only the chop site would leave mine/build XP as
  unwitnessed as before, which is the gap F8-C3 named.

### X2 · **THE CLUSTER'S CHOP XP IS 120**
- **PASS:** summed chop-grant XP over the felled cluster = **120**, split **40 / 48 / 32**
  per tree.
- The split matters for the same reason it did in the yield row: 120 could hide 56+32+32.

### PLANT
- **Drop the `× wood_count` scale** at the chop site ⇒ each tree grants a flat 8, total
  **24**, and the split collapses to 8/8/8. **X2 red on both the total and the split.**
- This is the realistic defect: `grant_xp(work, COMPLETION_XP)` is exactly what the other
  two sites say, so the scale is the easiest thing to lose in a refactor.

## 5 · WHAT I WILL **NOT** DO

1. **I will not read the XP total from a colonist's skill state.** That is the
   accumulator, not the event — it would fold in mine/build/haul XP and make the chop
   number unrecoverable. The bar reads the GRANT.
2. **I will not treat this as closing F8-C3's XP half for the mine path.** The witness
   lands at all three sites, but only the **chop** number is derived and scored here.
3. **I will not reuse the yield row's "never trust the tally" rule.** See §3 — it is the
   right source for this quantity, and applying a rule past its premise is how a good
   habit becomes a bug.
