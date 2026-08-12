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

### 🛑 **THE SAVE CANNOT ANSWER IT. MY NAMED INSTRUMENT IS VOID BY PREMISE.**

**Checked BEFORE building the fixture — which is the only reason this cost three greps
instead of a build.**

    common/src/apex/save_universe.rs:
        pub enum SaveStoreIdV1 { CharacterDb = 1, RtsimData = 2 }

    server/src/persistence/:  character/ · character_loader · character_updater
                              diesel_to_rusqlite · error · json_models · models
                              -- NO terrain module

> ## **TERRAIN BLOCK STATE IS NOT PERSISTED. THE SAVE HOLDS CHARACTERS AND RTSIM DATA.
> THE BLOCK AT `(15212,16043,425)` IS NOT IN IT AND NEVER WAS.**

★★★★★★ **MY ERROR, and it is my own banked law:
[[enumerate-what-the-instrument-can-see]].** *I named an instrument repeatedly, made it
the one IRREVERSIBLE item on the teardown checklist, had it ratified and had 790 MB
committed on it — **without ever checking that it could see the thing**.*

### ★★★ THE ACTION WAS RIGHT; THE REASON WAS FALSE

**Preserving `userdata` untouched remains correct** — *it holds rtsim state (colonist and
NPC state) that is genuinely valuable and genuinely irreplaceable.* ★★ **But it was
defended on a claim that was never true, and a right action defended by a wrong reason
will be reused with the reason attached.** *Corrected here so it isn't.*

---

## 2b · ✅ THE REPLACEMENT INSTRUMENT — **cheaper AND sharper than the save**

**Terrain edits live only in memory, so the read must be LIVE. And the sharp version
needs no waiting at all:**

> ## **LOG `terrain.get(job.pos)` ON THE TICK *AFTER* A MINE COMPLETION APPLIES.**

| next-tick read | verdict |
|---|---|
| **air** | *the write LANDS* → **(b): something re-fills it later; the regrowth window is then the question** |
| **still filled** | *the write DOES NOT LAND* → **(a), and `block_change`'s apply path is the next read** |

★★★★★ **This discriminates (a) from (b) in ONE TICK instead of inferring it across 98
minutes — and it is a diagnostic line, not a fixture build.** *The 31-minute framing was
an artifact of the wrong instrument.*

★★ **Gate it behind the existing `BASTION_EGRESS_DIAG` or its own flag** *(diag density
is budgeted — [[the-instrument-changes-what-it-sees]])*, **and it rides the next scored
run that designates mining — the same run F8's registered prediction is waiting on.**

---

## 2c · THE FREE READ — **attempted on v5's existing log, and EXHAUSTED**

**Before speccing new instrumentation, I asked whether the committed log already
settles it. Every line mentioning the cell:**

    112  emergency access arrived ownership check tick
      5  emergency access job selected diagnostic
      5  emergency access job accepted remote arrival
      5  colonist arrived at job site
      3  emergency access plan ordered jobs owner
      2  emergency access job completed diagnostic
      2  emergency access job completed

★★★★ **NEW FACT: the cell was SELECTED as an egress mine target 5 times and ARRIVED at
5 times, but COMPLETED only twice.** *Three attempts reached the cell and produced no
completion.*

★★ *Also: **112 ownership-check lines at a single cell** — heavy egress churn for two
completions. Recorded as a measure; not interpreted.*

### ⛔ AND IT DOES NOT DISCRIMINATE (a) FROM (b)

*The tempting next step is to argue from the 3 non-completions — "the cell must have
been air, so the write landed."* ★★★★★ **That requires knowing whether SELECTION checks
`is_filled`, which I have not read — and inferring across unread code is exactly the
five-dead-mechanisms trap.**

> ## **THE EXISTING LOG CANNOT SETTLE IT. THE FREE READ IS EXHAUSTED, AND THAT
> STRENGTHENS THE CASE FOR THE NEXT-TICK DIAGNOSTIC RATHER THAN REPLACING IT.**

★★ **A cheap read attempted and reported as insufficient is worth more than a cheap read
not attempted** — *it converts "we should instrument" from a preference into a
demonstrated necessity.*

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
