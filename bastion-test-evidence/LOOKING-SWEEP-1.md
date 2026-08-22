# LOOKING SWEEP 1 — a real village, 46 houses, 23 fields

First sweep where observation actually worked. The world spawn now follows the
colony, so `inspect_colonists` returned **8 payloads** instead of 0, and the
player stands in the town at `(27440, 18352, 408)`.

Scored against Ben's six acceptance criteria, worst-first, with what is
**not** verified said plainly.

---

## 2. They work real farms — **YES, VERIFIED**

The clearest pass. Following two colonists across four samples:

| | sample 1 | 2 | 3 | 4 |
|---|---|---|---|---|
| **Lowly Adow** | `Farm 0.208` | walking | `Farm 0.344` | walking |
| **Oafish Sheca** | walking | walking | `Farm 0.747` | walking |

Progress **advances between visits** (0.208 → 0.344), so these are real field
plots being worked, not a painted rectangle. Across the sweep: `Farm ×8`,
`Cook ×1`, and `jobs_total` grew **3,721 → 16,075** as the field plots streamed
in. `designations` 29 → 71 — the adopt drain placing the village's own plots.

## 6. Do they read as functioning humans? — **PARTIALLY**

The arc above **is narratable**: *she's working a field, walks off, comes back
and works it again.* That is what Ben asked for and it did not exist before.

`drive` across 32 colonist-samples: **Work ×31, Flee ×1.** Against an earlier
session's **339 flee preempts**, colonists are no longer running from
everything.

**But 38% of them are frozen** — see criterion 4.

## 4. Not a mad scramble — **MIXED, and this is the next row**

27 census samples:

```
mean_engaged = 4.44 of 8   (55%)
mean_stuck   = 3.04 of 8   (38%)
idle         = 0–1
```

`idle` is near zero — there is plenty of work and everyone claims some
(`jobs_claimed=8` at both readings). But **three of eight are stuck at any
moment**, holding a job and not moving. That is not a scramble; it is worse in
one way — it's a third of the town standing still mid-errand.

**This is the single biggest remaining gap** and the next row.

## 5. Pathing is human — **IMPROVED, NOT MET**

| | before fixes | now |
|---|---|---|
| fail-safe teleports | 11 | **5** |
| wallrun mentions | 4 | **1** |

Both down with the scramble re-priced (8.0 → 30.0) and the unstuck-jump
withheld from colonists on walls. **Not zero.** Every remaining teleport is
still a colonist "moved by magic because nothing else could reach it".

## 1. Colonists occupy homes — **PLACED, NOT VERIFIED**

8 colonists settled into a 46-house village, 2 per house by `settle_plan`'s
wrap. **Not verified that they sleep there or return to it** — `rested=8`
throughout, so nobody got tired enough to test it. Ben has ruled sharing is
fine for now (#114).

## 3. Stations in the right rooms — **NOT VERIFIED**

The cook-station granularity fix is merged (one station per painted region,
adjacent hearths absorbed) but nothing in this sweep looked at where stations
landed relative to rooms.

---

## What this sweep could not test

- **Sleep.** Nobody's rest dropped; a full day is 54,000 ticks and this ran
  ~8,000.
- **Any visual judgement.** `ascii` shows `#` for "solid above surface", which
  is walls, roofs and cliffs alike — it cannot answer "does it look like a
  town". A screenshot still needs a human at the client.
- **Whether the buildings are used.** I know colonists were *settled into*
  houses; I did not observe anyone inside one.
