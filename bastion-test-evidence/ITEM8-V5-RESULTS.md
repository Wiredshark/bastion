# ITEM 8 v5 — **RESULTS**

> ⚠ **SKELETON WRITTEN BEFORE TEARDOWN. EVERY `___` IS AN UNFILLED MEASUREMENT.**
> *Structure committed before the data exists, for the same reason the bar was:
> a results layout authored after seeing the numbers gets shaped to flatter them.*

**Scored against `ITEM8-V5-PACKET.md` · commands from `ITEM8-V5-SCORING-SHEET.md`.**
*Binary `4d918025` · scored window opened 2026-08-11T23:56:43Z.*

---

## 0 · PREFLIGHT — **GATE 0**

    gate-0 #1:  Server version: 4d918025 [2026-08-11]     PASS (read from log)
    gate-0 #2:  Server version: 4d918025 [2026-08-11]     PASS (v5's real launch env)

★★ *Both grep-confirmed against `git rev-parse --short=8 HEAD`, not assumed. Effective
config identical to v3/v4 — no drift. Label still reads "ITEM8-V4 config" (cosmetic).*

---

## 1 · THE BAR

| | bar | outcome |
|---|---|---|
| **F1** | **generation-2 completions > 0** — *any `sown` later than the FIRST `harvested`* | `___` |
| **F2** | no immortal jobs | `___` |
| **F3** | cells recycle *(and the reap count is not itself the defect)* | `___` |
| **F5** | targeted release fires | `___` |
| **F6** | leak backstop silent | `___` |
| **F7 / F10** | **no single position > 10% of completions** — *UNION of all three channels* | `___` |
| **F8** | `job completed` fires ONLY for completions with a world-effect | `___` |
| **S1** | sentinel: log-only *(promotion to a scored bar is its own post-v5 row)* | `___` |

★ **F9 is a MEASURE, not a bar** *(demoted pre-data: no observation was named that
would make it red).*

---

## 2 · THE REGISTERED COMPARISONS — **v4 numbers already measured and pinned**

| | v4 (measured) | v5 |
|---|---|---|
| **farm** | 19 tilled · 20 sown · 20 harvested, **then dead** | `___` |
| **`food_stock` peak** | **18** | `___` |
| ★★★ **terminal zero-streak** | **517 consecutive samples** | `___` |
| **`designated_sweep_reaps`** | **468,323** | `___` |
| **completions at one position** | **143/145 = 98.6%** | `___` |
| **eats** | 11 | `___` |
| **breakdowns** | 897 | `___` |

---

## 3 · THE TRIANGULATING READS

| read | result |
|---|---|
| **food SHAPE** — *sawtooth vs terminal, against v4's 517* | `___` |
| **rescue uid DISTRIBUTION** + `terminal_cause` — *traffic vs a rescue loop* | `___` |
| **`completed_kind` at repeat positions** — *did defect 1 recur?* | `___` |

★★ **Any two agreeing constrains the third.**

---

## 4 · THE FLAT-REAP FALSIFIER — **its registered form**

**Registered pre-build: *"reap count FLAT as claim rate varies."*** *Not "low reaps."*

    F5 claim_expiry_releases:  11 -> 20 -> 33 -> 50  (mid-run)   final: ___
    designated_sweep_reaps:    34 -> 34 -> 34 -> 34  (mid-run)   final: ___

★★★★★ **If it holds at teardown: not an improvement in a number — the mechanism
behaving as predicted before the fix existed.**

---

## 5 · CAVEATS CARRIED — **registered pre-data**

- ⚠ **DIAG DENSITY: v5 runs ~114× v3's byte rate.** ★★★ **REGISTERED · REAL ·
  ESTIMATED SMALL** *(absolute ≈ 14 KB/s, ~34 lines/s — not plausibly material against
  CPU-bound worldgen).* **Wall-clock cross-run figures carry the note. It may NOT be
  used to explain the rescue-rate gap.**
- ⚠ **The rescue RATE-PROFILE channel was WITHDRAWN at n=4** *(0.39 → 0.50/min is a
  plateau, not the decline I registered at n=3).* **The uid split is the discriminator.**
- ⚠ **Sample A of the N=8 is unscoreable** *(legs 1–2 pre-interleaving)*; **the 6×
  result stands independently on the capped arm's own 26% spread.**

---

## 6 · KNOWN-OPEN AT SCORING — **declared at launch, not discovered here**

- **DEFECT 1 — STAGED.** *Subject known: v4's trap cell is FOLIAGE
  (`completed_kind=Some(Leaves)`). Mechanism UNREAD. The save answers it; the fix window
  is the next row.*
- **The material-deadlock story stays PARKED and INFERRED** *(refusal #7)*.
- **The five-site arrival concentration** — *`kind`-on-arrival is the read.*

---

## 7 · VERDICT

`___`
