# ITEM 8 v5 — **TEARDOWN CHECKLIST**

**Written ~1.5 h before teardown, by the scorer who needs the artifact.**

★★★★★ **The capture is the only thing that survives this run. Everything else tonight
is recoverable; a botched teardown is not.**

---

## ⛔ 1 · THE ONE THAT CANNOT BE UNDONE — **PRESERVE `userdata` UNTOUCHED**

> ## **THE SAVE STATE CONTAINS THE ONLY ANSWER TO DEFECT 1's REMAINING QUESTION.**

**Defect 1's mechanism is unread, and its subject is now known: v4's trap cell
`(15212, 16043, 425)` is FOLIAGE (`completed_kind=Some(Leaves)`).**

★★★★★★ **The log cannot say whether that cell is filled or air at run's end. The SAVE
can.** *v4's capture preserved `userdata` untouched and that decision is what makes
this read possible at all.*

**⇒ Commit the full `userdata` tree exactly as v4's capture did. Do not clean, do not
prune, do not "tidy" the epochs.**

---

## 2 · PROCESS TEARDOWN — *ledger task #73*

- **KILL BOTH: the server AND the driver.** *#73 exists because one of them has been
  left behind before.*
- **The teardown script must SELF-TERMINATE at script end.**
- ★★ **Verify by PID, not by "the command returned"** — *the same concrete-verification
  standard applied all evening to PID 1455.*

---

## 3 · CAPTURE INTEGRITY

1. **Record the teardown timestamp** — *the scored window needs both ends.*
2. **Capture the FINAL heartbeat before the kill** — *the last counter values are the
   run's terminal state and cannot be recovered from a partial log.*
3. ★★★ **`md5sum` the raw log BEFORE splitting and confirm the parts reassemble to it**
   — *v4 did exactly this ("verified byte-for-byte lossless via matching md5sum") and
   it is why v4's log is trustworthy evidence.*
4. **Check the last line is COMPLETE** *(not truncated mid-line)* — *a half-written
   final line is the signature of a kill that raced the writer.*

---

## ⚠ 4 · DO NOT START THE N=8 IDLE LEGS UNTIL THE CAPTURE IS COMMITTED

★★★★ **The idle-box re-run is queued and cheap, and the temptation is to start it the
moment the server dies.** *Teardown, split, md5 verification and commit are the most
fragile minutes of the whole exercise.*

> ## **A CONTENDED BOX DURING CAPTURE-COMMIT RISKS THE ONE ARTIFACT THAT CANNOT BE
> RE-RUN, TO SAVE FIFTEEN MINUTES ON ONE THAT CAN.**

**Order: teardown → capture → md5 verify → commit → *then* the idle legs.**
*(Which also makes the idle legs genuinely idle — the condition the re-run exists to
create.)*

---

## 5 · THE TEARDOWN PULL — **what scoring needs in the same pass**

- **per-`uid` breakdown of `ULTIMATE FAIL-SAFE` firings**, ★★ **carrying
  `terminal_cause`** *(23 firings sharing one cause is a different finding from 23
  across several)*
- **the final farm counts** *(`tilled` · `sown` · `harvested`)*
- **the full `food_stock` series** — *for the terminal-streak shape test against v4's
  measured 517*

★ **Everything else I pull myself from the committed capture.**
