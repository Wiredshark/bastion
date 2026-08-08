# CONDEMNED CELL — the rescue is gated on PLAYER INTENT

**Blob for every cite: `5f8cdf1392`.** Line numbers move — re-locate by symbol.
**Status: STRUCTURAL READ, confirmed at both call sites. NOT yet observed live.**

## §1 — THE BANKED HYPOTHESIS, AND IT HOLDS

Banked candidate was: *"`carve_ramp`'s `allowed = in_access_mask(mask, p)` means
the planner may only dig **inside a designation**, so a colonist stranded outside
every painted region cannot be rescued by carving."* **Read; it holds.**

```rust
// plan_access (859), the planner's own predicate
let allowed = |p: Vec3<i32>| {
    in_access_mask(mask, p)
        && emergency_owner.is_none_or(|_| !in_access_mask(&protected_designations, p))
};
```

**And `mask` is the player's paint — identical at both call sites:**

| call site | context | mask |
|---|---|---|
| **13142** | `"self_rescue"` | `let mask = board.designated.clone();` |
| **16096** | `"proactive_descent"` | `let mask = board.designated.clone();` |

## §2 — THE PREDICATE'S ACTUAL SHAPE (176) — asymmetric, and that matters

```rust
fn in_access_mask(designated: &[Region], p: Vec3<i32>) -> bool {
    designated.iter().any(|r| {
        p.x >= r.min.x - 1 && p.x <= r.max.x + 1
            && p.y >= r.min.y - 1 && p.y <= r.max.y + 1
            && p.z >= r.min.z - 1                      // ← NO max.z BOUND
    })
}
```

- **XY: dilated by exactly 1.** So the tolerance for being "outside" is **one
  block**. Stranded **≥2 blocks** beyond every painted region ⇒ **no cell the
  planner is permitted to dig** ⇒ `carve_ramp` yields nothing.
- **Z: unbounded ABOVE, hard-floored BELOW at `min.z - 1`.** A colonist who ends
  up *below* the bottom of every designation is excluded — ★ **which is the
  direction mining moves.**

## §3 — ★★★★★ WHY THIS IS THE INTERESTING PART

> **`board.designated` answers *"where does the player want digging?"* The rescue
> asks *"where may I dig to save a colonist's life?"* THE CODE USES ONE MASK FOR
> BOTH QUESTIONS.**

A self-rescue carve is **humanitarian** — it exists precisely for the case where a
colonist is somewhere it should not be. **Gating it on player intent means the
rescue is available exactly where it is least needed** (inside the area the player
is actively working) **and unavailable where a colonist is most likely stranded**
(one step past the edge, or one level below the floor).

★ **This is not a bug at any single site.** The mask is the correct input for
`proactive_descent` — that one *should* respect player intent, and shares the
call by design. **The defect is that one predicate serves two purposes with
opposite requirements.** Same shape as AUTON-2 §4c: every site correct, the
failure only visible across the composition.

## §4 — HOW IT PRESENTS: the strong verdict, silently

`plan_access`'s `None` is the **strong verdict** — immediate `unreachable`,
blocked message, bypasses the churn counter. So a stranded colonist outside the
mask does not churn, does not retry, does not escalate: **it is condemned in one
tick, and the reason recorded is "unreachable," not "I was not allowed to dig."**

> **A REFUSAL AND AN IMPOSSIBILITY RENDER IDENTICALLY.** The player sees a
> colonist that cannot be reached; the truth is a colonist the rescuer was
> forbidden to reach.

## §5 — THE ONE-LINE CHECK, still the right next step

**Is the stranded colonist's own position inside `in_access_mask(board.designated,
feet)`?** Log it at the `plan_access → None` site. **One boolean separates
"geometrically impossible" from "administratively forbidden,"** and those are
different rows with different fixes.

★ **Do not fix before measuring.** The obvious fix — dilate the mask, or exempt
self-rescue from it — **widens where colonists may dig without the player asking**,
which is a design change, not a repair. It needs Fable/Ben, and it needs the
boolean above first: if stranded colonists are *inside* the mask, this whole read
is a live-fire miss and the cause is elsewhere.

## §6 — RELATION TO THE OPEN AUTON-2 WORK

AUTON-2 §4c's still-open question is **why travel to a bed stalls** — the fifth
appearance of *displaced colonists failing to arrive*. **If a stalled colonist is
outside the mask, these are the same row**, and the bed case is simply the first
instance where the destination is not a dig site. **Check the mask boolean on the
AUTON-2 trace too** — it is one read and it either merges two rows or splits them.
