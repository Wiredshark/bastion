# ROW — **THE INDESTRUCTIBLE MINE CELL**

**Item 8's famine has a first cause, and it is not seeds, not claim expiry, and not
the sweep churn.** *Found 2026-08-11 in the committed v4 capture `1bedd79602`; no
rerun required, every read below reproduces from that pin.*

---

## 0 · ★★★★★★ THE FINDING

> ## **EVERY `bastion: job completed` IN 945,462 LINES IS `Designated(Mine)`, AND 328
> OF 361 ARE THE SAME TWO ADJACENT CELLS.**

| completed job | part-000 | part-001 | part-002 |
|---|---|---|---|
| **`Vec3 { x: 15212, y: 16043, z: 425 }`** | **47** | 19 | 0 |
| **`Vec3 { x: 15211, y: 16044, z: 425 }`** | 1 | **138** | **143** |
| *all other positions, all kinds* | 3 | 6 | 2 |

**A mine job completes without consuming its cell.** *The cell is re-designated,
re-claimed and re-completed — 281 times on one block — and the trap **walks to its
diagonal neighbour** when the first cell stops yielding.*

★★★★★ **This is TASK #61, filed as "do not run now — mine cell, progress 0.0,
claimed."** *It ran itself, live, for two and a half hours.*

---

## 1 · ★★★★★★ THE PRODUCTION COLLAPSE, ALONGSIDE IT

| | part-000 | part-001 | part-002 |
|---|---|---|---|
| **tilled · sown · harvested · crop** | 19 · 20 · 20 · 20 | ⛔ **0 · 0 · 0 · 0** | ⛔ **0 · 0 · 0 · 0** |
| **ate** | 11 | ⛔ **0** | ⛔ **0** |
| **haul delivered** | 11 | 2 | ⛔ **0** |
| ★★ **`job completed`** | 50 | **166** | **145** |

> ## **COMPLETIONS ROSE WHILE EVERY FORM OF PRODUCTION WENT TO ZERO.**

---

## 2 · WHAT IT OVERTURNS

**1 — THE COLONY WAS NEVER PARALYSED.** ★★★★ *This kills churn-starvation as the
farm's cause: if the 468K-cycle sweep were consuming colony capacity, non-farm work
would fall too. It rose — because the work was a phantom.*

**2 — ★★★★★ `job completed` IS A COUNT A BROKEN SYSTEM RELIABLY PRODUCES.** *It is
the colony's own health metric and it pointed UP while the colony starved to death.*
**The identical error to F1's, one level down — and it needs the same treatment.**

**3 — THE SEED HYPOTHESIS IS REFUTED AS A CAUSE.** *The complete message vocabulary
is 22 distinct lines; there is no seed line, no fetch line, no refusal line anywhere.*
★★★ **And it does not matter: the farm stopped because every colonist was permanently
employed on an indestructible block.** *19·20·20 is the founding wave finishing before
the trap captured the whole labour force.*

**4 — ROUTES 2 AND 3 ARE CORRECT FIXES FOR REAL DEFECTS THAT WERE NEVER WHAT STOPPED
THE FARM.** *Predicted from F1's identity with v3, before this read existed.*

---

## 3 · ⚠ TWO METHOD DEFECTS THIS READ EXPOSED — **both silent, both zero-shaped**

1. ★★★★ **A WRONG PATH AND AN EMPTY FILE RENDER IDENTICALLY.** *The first search ran
   against `bastion-test-evidence/…-v4-split/`; the capture is at
   `bastion-test-evidence/**live-playthrough**/…-v4-split/`.* **Zero matches was
   reported as an unresolved naming question. It was an unread directory.**
2. ★★★★★ **`kind=` MATCHES NOTHING IN THESE LOGS.** *ANSI escape codes sit between key
   and `=`:* `kind[2m=[0mDesignated(Mine)`. **Every past `key=value` grep of this
   corpus returned a silent zero.** *Strip with `sed 's/\x1b\[[0-9;]*m//g'` first.*

★★ **Both are the same class: a zero that means "the instrument was not pointed at
the data," presented in the shape of a result.**

---

## 4 · THE ROW

**The mine-completion path completes a job without verifying the block was removed.**

**Two candidate seams, both already on our books:**

- **#61** — progress 0.0 on a claimed mine cell
- **#52** — `blocked_regions` entry keyed on region alone

★★★ **The read is narrow now: the cell, the timestamps, and the handover to the
diagonal neighbour are all in hand.**

### CONSEQUENCE FOR v5

**F1 (generation-2 completions) survives unchanged and is better motivated.** ★★ *But
the fix under test should be THIS, not another claim-lifetime tweak — and the
`job completed` metric itself needs a kind/position breakdown before it is trusted as
a health signal again.*
