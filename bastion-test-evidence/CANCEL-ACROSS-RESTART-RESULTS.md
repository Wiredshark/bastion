# CANCEL ACROSS RESTART — **RESULTS & ROW DISPOSITION**

Scored against `CANCEL-ACROSS-RESTART-PREREG.md` (`792cd2c326`). Engine tip `9a83221505`
(no code change — this row is a bar, not a feature).

## THE SCORE — **2 PASS, 0 FAIL. The plant fires.**

| bar | verdict | evidence |
|---|---|---|
| **C1** a cancelled order does not come back | ✅ PASS | boot 2: `farm plot registered` = **0** |
| **C2** the other orders still survive | ✅ PASS | `replayed=3`, stockpile zone registered, Bed+Bed returned |

| plant | required red | observed |
|---|---|---|
| append-only save instead of a live-set snapshot | C1 red | **`replayed=9`, farm registered = 1** — the erased farm **resurrected**, and history accumulated three snapshots |

Control reproduced on **two** scripts (fast and slow) plus a third run on a
verified-fresh binary. Unit: **124/124**.

## ★ THE GEOMETRY THE PREREG PREDICTED, CONFIRMED TO THE CELL

The preset's plots are adjacent by construction, so cancelling the farm's AABB necessarily
clips the bed's shared `x = 16381` column. That is why the bar reads a **kind-specific
emit** and not an order count.

The restore reproduces the subtraction **exactly**:

- bed = `x 16381..16382, y 16381..16382, z 400..401` = **8 cells**
- ∩ farm-cancel = `x 16381, y 16381..16382, z 400` = **2 cells**
- boot 2 replays the bed as **two pieces, `jobs=4` + `jobs=2` = 6 = 8 − 2**

An AABB subtraction survived a save/load round trip cell-for-cell. Had I scored on a
count, "3 orders" would have looked unchanged while carrying entirely different geometry.

## ⛔ THE VOID THAT NEARLY BECAME A FINDING — and what it exposes

The plant's **first** run reported `FARM=0`: a clean, publishable negative result reading
*"the append-only defect does not actually resurrect anything."*

It was void. The binary was **15 minutes older than the source** — a `cargo build`
bundled with a ~7-minute run inside one 10-minute foreground call had been **killed by
the timeout**, so the background pair ran the **control** binary against a
plant-labelled userdata. Rebuilt with `binary mtime > source mtime` asserted first, the
plant fired hard: `replayed` 3 → **9**, farm back.

**Fourth void of the session** (A4's restart, W4's water radius, S1-D's tick count, this).
The first three were caught by incidental signals. This one was caught only because the
law had been written down.

### ⚠ AND IT NAMES F3's MISSING HALF

The driver-freshness row (`03d36e10f1`) made `bastion_playtest` declare its commit and
verb table. **`veloren-server-cli` declares nothing.** Every bar in this program reads
*server* emits — `founding site relief`, `colony orders replayed`, `COLONY TERMINAL` —
and not one of them can be attributed to a build. A stale server produces confident,
internally consistent evidence with nothing in the log to betray it, which is precisely
what happened here.

**This is a registered successor row, not a tidy-up.** Until it lands, `ls -la` the server
binary against its source before any scored run.

## WHAT I DECLINE TO CLAIM

- **Not** that this row changed any code. It did not; it is the bar the persistence row
  registered as owed. The implementation was already correct — the bar now proves it
  rather than assuming it.
- **Not** that cancellation is durable for kinds other than Farm. One kind was erased and
  witnessed by its own emit; the mechanism (`designated` is the live set, the save is a
  snapshot of it) is kind-agnostic, but only Farm was exercised.

## SESSION QUEUE STATE — eight rows closed

1. ✅ Founding preset on real worldgen (`f51213cc4c`)
2. ✅ Arena trees / F8-C1 (`793df9401a`)
3. ✅ S1 sentinel scored-bar (`dcc0b950e9`)
4. ✅ Water gate / F1 (`95a597ec5a`)
5. ✅ Relief-emit blind spot / F2 (`5801770cb5`)
6. ✅ Driver freshness / F3 (`03d36e10f1`)
7. ✅ Colony-state persistence (`94e9711405`)
8. ✅ **Cancel across restart**, this document

**Next:** **server-binary fingerprint (F3's missing half)** — promoted to the front of the
queue by the void above, ahead of §8 N2's widget tier.
