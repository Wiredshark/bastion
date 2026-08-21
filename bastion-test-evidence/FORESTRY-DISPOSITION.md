# FORESTRY SELF-GENERATOR — DISPOSITION: **PASS**, first leg

Scored against `FORESTRY-PREREGISTRATION.md`, which was committed before a
line of the feature was written. Arm `pitnowood` (a pit needing ladder access,
with **no wood seeded**), attested fresh, `dirty .rs 0`.

## The three registered predictions

**1. Chop designations appear that no player painted — PASS.**

```
FORESTRY felled a tree for a job that billed wood  demand=16 supply=0 pending=0 trees_seen=2
FORESTRY felled a tree for a job that billed wood  demand=16 supply=1 pending=1 trees_seen=2
```

Sixteen units of ladder demand woke the generator; it found the two trees in
range and felled them. Nothing in the script asked for wood.

**2. Wood enters supply — PASS.** `supply` climbs `0 → 1 → 7` across the
firings, and `demand` falls `16 → 12 → 11` as the ladders get their timber.
The loop closes: demand pulls, felling supplies, demand drops.

**3. The generator goes quiet — PASS on the arithmetic, PARTIAL on the
demonstration, and the difference is stated rather than smoothed.** It felled
exactly one tree per firing and re-read the deficit each cadence, so it never
clear-cut. But demand was never fully *satisfied* in this leg — it stopped
because the arena ran out of trees, not because the colony ran out of need. So
the `demand <= supply + pending` early-return is proven by construction and by
the falling demand, **not** by an observed transition to silence-while-supplied.
A leg in a forested arena is owed for that branch.

## The falsifier fired, correctly, and named which null it was

```
FORESTRY wanted wood and felled NOTHING  demand=11 supply=7 pending=0 trees_seen=0 radius=24
```

`trees_seen=0` — the two trees in a 24-block radius had already been felled.
This is the **worldgen-fact** branch, not the refusal branch, and the witness
says so in one line without another leg.

That distinction is the entire reason this row cost one pass. F13 burned
**three legs** on a null that rendered "the resource is absent" and "the
resource was refused" identically, and only closed after a witness was added
retroactively. Here the witness shipped with the feature, so its first null
was already self-explaining.

## Why this took one leg and F13 took five

Every one of the mine's four blockers has a wood twin, and each was answered
in the design rather than discovered by a failed leg:

| Mine blocker (cost) | Wood twin | Answered by |
|---|---|---|
| demand over PLANS only (leg 1) | demand over nothing | `job_bills_wood_unsupplied` |
| radius too small (leg 2) | trees out of range | same 24, and `radius` printed in the witness |
| CLAIMED jobs excluded, so progress zeroed demand (leg 3) | identical | `unclaimed OR needs_materials` in the first commit, pinned |
| generator's test easier than the claim path's (leg 5) | unfellable cells | `place_chop_fell` re-validates; witness reports seen-vs-felled |

The third is the sharpest: the stone predicate shipped unclaimed-only, and the
moment colonists could reach their jobs, every claim flipped a job out of the
demand set and silenced the generator — **a fix that made the numbers go
down**. `a_claimed_but_unsupplied_wood_job_is_still_demand` pins that the wood
twin never had the hole.

## Known residuals, registered not hidden

- **Fixed radius, again.** 24 blocks. A colony founded on open plain with its
  nearest forest 60 blocks off will report `trees_seen=0` forever. Same
  residual the mine carries; the real answer is ranging, which is a design row.
- ~~**The standing par-stock floor is NOT built.**~~ **BUILT the same day** —
  see the disposition appended below. Struck rather than deleted: this row was
  genuinely demand-PULL only when it passed, and a reader should be able to see
  that the floor came after, not that this row silently included it.
- ~~**Quiescence-while-supplied is unobserved**~~ — **now observed**, by the
  floor's leg: `demand=8 supply=8 pending_mine=0 quota=0`, held. That is the
  transition to silence-while-supplied this row could not demonstrate, because
  its own arena ran out of trees before the colony ran out of need.

---

# PAR-STOCK FLOOR — DISPOSITION: **PASS**

Scored against the prediction in `FORESTRY-PREREGISTRATION.md`, registered
before the leg ran. Arm `injury`, attested fresh, `dirty .rs 0`.

```
mine generator STATE demand=8 supply=8 pending_mine=0 quota=0   (x4, held)
quota distribution across the run: quota=0 in 41 samples, quota=1 in 1
bed registered (built): 8
```

1. **Stockpiles with nothing asking — PASS.** `supply=8` with every bed job
   already satisfied. Before the floor the colony mined exactly what a job
   demanded and held nothing.
2. **IT STOPS — PASS, and this was the bar that mattered.** Demand and supply
   converge exactly at the par level and `quota` sits at 0 for 41 of 42
   samples. The single `quota=1` is the firing that ordered the last block. A
   floor that kept digging would have been *worse than no floor*; it doesn't.
3. **No F13 regression — PASS.** 8 beds still built.

## The self-catch this row is really about

The **first** version passed the safety bar too — `quota=0`, no runaway, beds
intact — while parked at `demand=3 supply=5` against a floor of 8. I had
written the floor as a *deficit* (`PAR_STOCK − on_hand`) when the quota
arithmetic downstream already subtracts supply, so supply came off twice and
the floor silently converged to about half its level.

**The error's direction was safe**, which is exactly why it survived: it
under-mined rather than strip-mined, so it sailed through the one failure mode
I had pre-registered. A registered bar protects against the failure you
predicted and is silent about the one you didn't. Nothing was red; the catch
came from reading the numbers *beside* the verdict and noticing that
`demand=3 supply=5` is not a sentence a floor of 8 can produce.

This is the mirror of the session's other lesson. Earlier a silent instrument
was mistaken for health; here a **green bar** was nearly mistaken for health.
Neither is evidence on its own unless the numbers underneath agree with what
the feature claims to do.
