# ROW — **DEFECT 1: THE BLOCK THAT DOESN'T LEAVE**

**Successor to `ROW-INDESTRUCTIBLE-MINE-CELL.md`. Opened 2026-08-12 with its evidence
already in hand** — *the staging worked: this row starts with a measurement, not a
hypothesis.*

---

## 0 · ★★★★★★ THE EVIDENCE

**Both v5 completions at v4's trap cell `(15212, 16043, 425)`, capture `8dd2f9463b`:**

    00:21:19   tick  45,039   uid=131   completed_kind=Some(Leaves)
    01:59:32   tick 220,336   uid=129   completed_kind=Some(Leaves)

> ## **DIFFERENT COLONISTS. 98 MINUTES AND 175,297 TICKS APART. THE FIRST MINED IT; THE
> SECOND FOUND IT FILLED.**

★★★★★ **`still_valid` requires `is_filled()` and it passed the second time.** *The air
write issued at 00:21 (`completion_block(Mine) → Block::empty()`) was not in effect at
01:59.*

**Established, across two runs:**

1. **The subject is FOLIAGE, not rock** — *the "indestructible mine cell" was leaves.*
2. **The behaviour is PERSISTENT and REPRODUCIBLE** — *v4 (281 completions) and v5 (2),
   same cell, same kind.*
3. ★★★ **It is NOT the loop.** *Defect 2's fix removed the loop entirely; this survived
   it untouched.* **The two defects were genuinely separable, as registered.**

---

## 1 · TWO CANDIDATES — **and neither is promoted**

| | candidate |
|---|---|
| **(a)** | **the air write never lands at this cell** |
| **(b)** | **it lands, and the leaves REGROW within 98 minutes** |

★★★ **(b) is the more attractive story and is therefore the one to distrust** —
[[stop-proposing-and-instrument]]. *Neither is promoted; the read below decides.*

### ⚠ ONE ZERO THAT DOES **NOT** DISCRIMINATE

**`bastion: job moot` fires ZERO times in v5 — as in v4. Two runs, ~1.1M lines, never
once.**

*Tempting as support for (a).* ★★★★ **But the moot path may be DORMANT BY PREMISE: if
job creation already requires a filled cell, moot is structurally hard to reach.** *A
zero on a possibly-unreachable path is not evidence — refusal #2, applied to my own
hypothesis.*

**⇒ Before this zero is ever cited, prove the moot path REACHABLE** *(a fixture that
clears a cell under a live Mine job must produce the line).*

---

## 2 · ★★★★★★ THE DISCRIMINATING READ — **sharper than "is the cell filled"**

    last trap-cell completion   01:59:32
    teardown                    02:30:00        -> 31 minutes elapsed

> ## **THE QUESTION IS: IS THE CELL FILLED **31 MINUTES AFTER ITS LAST MINING**?**

| save shows | consequence |
|---|---|
| **Leaves** | *(b) needs regrowth faster than 31 min; **(a) becomes the simpler explanation*** |
| **air** | ***(b) CONFIRMED**, and the regrowth window is bounded between 31 and 98 minutes* |

★★★★★ **The instrument exists and is preserved: the untouched 790 MB `userdata` in
capture `8dd2f9463b`.** *This is why "preserve `userdata` untouched" was the one
irreversible item on the teardown checklist — the answer was written down before anyone
knew the question's final form.*

★★ **It is a HARNESS READ, not a grep.** *Deserialise the save, query the block at
`(15212,16043,425)`. That is a fixture, and it is this row's first build step.*

---

## 3 · THE WORK

1. **Build the save-query fixture** *(deserialise `userdata`, read one block by
   position)* — **the read above.**
2. **Prove the moot path reachable** *(else its zero can never be cited)*.
3. **Then, and only then, the fix** — *with planted tests both polarities per
   [[a-falsifier-needs-its-own-control]]: a foliage cell under a Mine job must end as
   air and STAY air; a normal mine must still complete.*

### ⚠ SCOPE NOTE

★★★ **Defect 1 caused no observable harm in v5** *(2 completions, no loop, F7 = 3.98%)*.
**It is a latent correctness defect whose blast radius was the LOOP — and the loop is
fixed.** *Priority accordingly: real, reproducible, not urgent.*

★★ **But it is exactly the class that waits.** *v4's 281-completion trap needed BOTH
this and an unbounded replan; one of the two is still there.*
