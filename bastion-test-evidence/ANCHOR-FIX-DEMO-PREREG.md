# ANCHOR FIX — RED DEMONSTRATION, pre-registered

Fix under test: `bastion_playtest.rs` now waits on `Pos` as a CONDITION, refuses
(exit 3) rather than anchoring at the world origin, and logs a self-scoring
witness naming what the OLD code would have done.

Banked control: `provtravcap` 49/85 = 58% origin-anchored, `provtravuncap`
6/6 = 100%, every non-terrain arm 0/586.

## Bars, registered before launch

| bar | arm | PASS | FAIL |
|---|---|---|---|
| **A — the fix works** | all provtrav runs | **zero** runs report a start pos of `(0.0, 0.0, 0.0)` | any run anchors at origin |
| **B — the fix was NEEDED** ★ | all provtrav runs | **≥1** run logs `OLD BEHAVIOUR WOULD HAVE ANCHORED AT THE WORLD ORIGIN` | zero such lines ⇒ **the fan is VOID for demonstrating the fix** |
| **C — no regression** | provtravcap twins | promoted key sets IDENTICAL within each twin pair, and in the 304 (real-anchor) class | sets differ, or a new class appears |
| **D — #114 replication** | endurseed twins | both reach target; maturations ≫ 32 and `blocked_materials` collapses to 0 in BOTH | either twin fails to reach target ⇒ that twin VOID, not FAIL |

## ★ Why bar B is not optional

"Zero origin anchors" has two readings that render identically: **the fix
worked**, or **the condition never arose on these hosts**. Fresh VMs, a re-baked
image, and a faster server would all suppress the race without the fix doing
anything. The witness discriminates them because it reports the counterfactual
per run — `warmed > TPS*2` is exactly the set of runs the old driver got wrong.

Without a bar-B line, bar A is VACUOUS and I will report it as such rather than
as a pass. [[null-needs-a-couldnt-happen-witness]]

## Preconditions printed above every verdict

1. Both twins booted (`ready to accept` = 1) and carry a terminator.
2. The driver log exists and carries an `anchor precondition:` line — its absence
   means the binary predates the fix, and the arm is VOID, not green.
3. `provtravuncap` is included deliberately: it was 6/6 origin-anchored, so it is
   the arm most likely to satisfy bar B.
