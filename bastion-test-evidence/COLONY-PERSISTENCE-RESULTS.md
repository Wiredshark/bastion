# COLONY-STATE PERSISTENCE — **RESULTS & ROW DISPOSITION**

Scored against `COLONY-PERSISTENCE-PREREG.md` (`e1120a9284`) and
`COLONY-PERSISTENCE-DESIGN.md` (`4fb7a73aee`). Engine tips `31b5928dcb` (stage 1),
`9a83221505` (stages 2–3).

## THE SCORE — **4 PASS, 0 FAIL. Every plant fired.**

| bar | verdict | evidence |
|---|---|---|
| **P1** orders survive a restart | ✅ PASS | `orders=3` read, `replayed=3 still_waiting=0` — was **3 → 0** |
| **P2** work returns, not just data | ✅ PASS | 3 designations placed, **bed `jobs=8`**, stockpile zone + farm re-registered |
| **P3** a fresh world restores nothing | ✅ PASS | 0 read, 0 replayed, 0 designations — nothing fabricated |
| **P4** one-colony boundary unaffected | ✅ PASS | second founding still refuses `colony_exists`, with `replayed=3` |

| plant | required red | observed |
|---|---|---|
| save path disabled **at the write** | P1 red | **0 read / 0 replayed / 0 designations — and 8 colonists back** |
| load path disabled | P1 red, different cause | 0/0/0 on a copy of the **same** golden save |
| restore data, skip job regeneration | **P2 red, P1 green** | `replayed=3` ✅ but **0 designations, no bed jobs, no zone** |

Control on the **same golden save file**: `orders=3`, `replayed=3`, 3 designations,
`jobs=8`. Unit: **124/124**.

**Precondition, printed above the result** (this session's thrice-learned law): boot 1 was
held **170 s** past its founding and `data.dat`'s mtime (16:20:52) is after the founding
(16:18:05 local; the log stamps UTC). The 60 s rtsim save fired. The run was **valid**,
not merely green.

## WHAT WAS ACTUALLY BROKEN

Ben found it live restarting the celebration world — *"colonists came back, the zones did
not."* §8 B4 says there is **no colonist anchor**: work at F is the only retention
mechanism. So a restart deleted every designation, deleted the work, and deleted the one
thing holding the colony together — the colonists then wander, which is the
cross-country leash-march §4 exists to prevent, arriving through a different door.

## ★ THREE DESIGN DECISIONS THE PLANTS EARNED

**1 · Replay through `place_designation`, not into the store.** Plant 3 is why this
matters: pushing straight into `designated` leaves P1 perfectly green at `replayed=3`
while P2 collapses to zero. Jobs, zone registration and the claim mask all come from the
call a founding already makes, so the restore path cannot drift away from the real one.

**2 · The kind rides with the region (stage 1).** `cancel_region` removes by
**intersection** and subtracts AABBs, so a parallel order log would have had to replicate
that predicate forever. One store instead —
`a_partially_cancelled_order_keeps_its_kind` red-demonstrated by stamping `Build` on a
Mine's remainder (*"left: Build, right: Mine"*).

**3 · ⚠ The save writes a UNION, and this is load-bearing, not tidy.** rtsim saves
**immediately on its first tick** (`last_saved.is_none_or(..)`), and after a restart the
board is empty until the replay finds terrain. Saving `designated` alone would have
overwritten the very orders being restored **with nothing, on the first tick, every
time** — the feature would have eaten its own save. An order awaiting replay is still an
order. The seed is placed before the save block for the same reason.

## ⚠ A METHODOLOGY CATCH — my first save-plant proved nothing

I ran the save-disabled plant against a userdata copy that **already held the saved
orders**, so disabling the save only blocked *future* writes and the restore succeeded
anyway. The plant looked green and meant nothing.

**A plant must be present at the stage it disables.** Redone as a full boot 1 with the
save disabled, it produced the original defect verbatim: 0 orders, 0 designations, **8
colonists**.

## WHAT I DECLINE TO CLAIM

- **Not** that the whole `JobBoard` persists. Deliberately scoped out: its own doc calls
  the command ledger, claims and id counter runtime-only, and freezing them would make
  every scheduler change a save-compatibility problem.
- **Not** that replay is instant. Orders **wait** for their chunks; on a real-worldgen
  world that means until a player brings the colony's terrain in. `still_waiting=0` here
  is an arena result, where the colony sits at spawn.
- **Not** that a cancelled designation stays cancelled across a restart *by test*. It
  follows from the store being the live set (a cancel removes it before any save), but
  **no bar exercised it** — registered as open rather than assumed.

## SESSION QUEUE STATE — seven rows closed

1. ✅ Founding preset on real worldgen (`f51213cc4c`)
2. ✅ Arena trees / F8-C1 (`793df9401a`)
3. ✅ S1 sentinel scored-bar (`dcc0b950e9`)
4. ✅ Water gate / F1 (`95a597ec5a`)
5. ✅ Relief-emit blind spot / F2 (`5801770cb5`)
6. ✅ Driver freshness / F3 (`03d36e10f1`)
7. ✅ **Colony-state persistence**, this document

**Next:** the cancel-across-restart bar named above, then §8 N2's widget tier — still the
one acceptance tier no bar has ever run at.
