# THE CHOP YIELD — **RESULTS & ROW DISPOSITION**

Scored against `CHOP-YIELD-PREREG.md` (`1b08c80864`). Engine tip `ece698af80`.
Attested before the leg.

## THE SCORE — **3 PASS, 0 FAIL** *(and one plant reported non-constructible)*

| bar | verdict | evidence |
|---|---|---|
| **Y1** the witness exists and reads back off the item | ✅ PASS | 15 drops, **all `amount=1`**; and see the structural argument below |
| **Y2** the yield is 15 | ✅ PASS | **15** — exactly the number registered before the fell-sets ever resolved |
| **Y3** per-tree attribution 5 / 6 / 4 | ✅ PASS | **5** @ (16402,16386) · **6** @ (16406,16381) · **4** @ (16409,16388) |

| plant | required red | observed |
|---|---|---|
| leaves yield too | Y2 red at 42 | **42** across **27** columns — per-tree grouping shatters |
| emit the placement tally instead | Y1 red | **NOT CONSTRUCTIBLE** — see below |

Control restored: **15**, split **5/6/4**. Unit **126/126**.

## ★ READING THE CODE MOVED THE INSTRUMENT

Chop drops **do not happen at completion.** The base cut only converts the tree into a
`FellingTree`; the drops rain down **per Wood cell during the stagger** that follows
(the code says so: *"Drops happen per-band at removal time, per Wood cell in place"*).

**A witness on the completion path would have counted TREES — three — and called it
yield.** The number would have been wrong by a factor of five and looked perfectly
reasonable. This is the second time this session that reading the producer before
instrumenting changed where the instrument belongs.

## ★ Y1 IS STRUCTURAL, NOT MERELY DISCIPLINED — and the second plant is therefore VOID

The prereg demanded the amount be read **off the produced item**, never from the
placement-time `wood_count`, because that tally sets the work threshold and an emit
carrying it would report the *prediction* — a witness that cannot disagree with what it
checks.

It turns out the site **cannot** report it:

```rust
pub struct FellingTree { pub cells: Vec<Vec3<i32>>, pub cursor: usize }
```

`wood_count` is **not a field**. It does not survive the base cut. So the witness is
incapable of reporting the prediction *even if I wanted it to* — stronger than the
comparison test I registered, and it makes the registered plant 2 **not constructible at
that site**. Reported as such rather than quietly dropped or claimed as run.

Corroborated in the data anyway: all 15 drops read `amount=1`. Had the tally leaked in,
amounts would have read 5/6/4 and summed to 5² + 6² + 4² = **77**.

## THE NUMBER HELD FROM DERIVATION TO MEASUREMENT

`RESOURCED_TREES` → heights **5, 6, 4** → **15 Wood**, with the 27 leaf cells cleared and
yielding nothing. That figure was written down in the arena-trees prereg **before the
fell-sets had ever been shown to resolve**, and it has not moved since. The live split
lands on the three trunk columns individually — a total of 15 could have hidden 7+4+4,
which is precisely why Y3 exists.

## WHAT I DECLINE TO CLAIM

- **Not** that this holds on real worldgen. Real trees saturate `TREE_FELL_CELL_CAP` (all
  four measured **exactly 2048**), so their fell-sets are truncated and the yield is *not*
  the whole tree. The arena's small trunks are the only place 15 is well-defined —
  inherited from the arena-trees row, not rediscovered.
- **Not** that XP is witnessed. F8-C3 named **drop AND XP**; this row closes the drop half.
  `grant_xp` fires at the base cut with `COMPLETION_XP × wood_count` and still has **no
  emit** — registered open.
- **Not** that the mine path is covered by this bar. A yield witness was added there too
  (same read-back discipline), but **no bar in this row scores it**.

## SESSION QUEUE STATE — thirteen rows closed

1–7 as recorded · 8. Cancel across restart · 9. Run attestation · 10. Founding colonist
count · 11. Population sensitivity · 12. A3 at n=4 · 13. **The chop yield**, this document.

**Next:** the **XP half of F8-C3** (no emit on `grant_xp`), then the **haul-throughput
question** A3-at-n=4 opened (peak stock 6 → 2 on identical harvests).
