# THE FOUNDING COLONIST COUNT — **RESULTS & ROW DISPOSITION**

Scored against `FOUNDING-COUNT-PREREG.md` (`37ae4fb0af`). Engine tip `f6e1707988`.

## THE SCORE — **3 PASS, 0 FAIL. Both plants fire, on different axes.**

| bar | verdict | evidence |
|---|---|---|
| **N1** one constant, the widget reads it | ✅ PASS | `common::bastion::FOUNDING_COLONIST_COUNT`; `session/mod.rs` passes it, not `6` |
| **N2** bed capacity derived, not coincidental | ✅ PASS | `bed_capacity()` from `FOUNDING_PRESET_V1` ⇒ **8 ≥ 8** |
| **N3** the live path still founds | ✅ PASS | `colonists=8 elements=stockpile,farm,bed complete=true`, bed `jobs=8` |

| plant | required red | observed |
|---|---|---|
| bed shrunk one row | N2 red | capacity **4** — relation fails *and* the pinning test fails |
| count raised to 9 | N2 red **from the other side** | capacity still 8, **pinning test GREEN**, only the relation fails |

Unit **126/126**; voxygen checks clean. Attested before the live leg.

## ★ §8 N2 ANSWERED BY READING, NOT BY DRIVING A GPU

The widget path calls the **same** `Client::bastion_spawn_colony` the driver calls
(`voxygen/src/session/mod.rs` → `client/src/lib.rs:2844`). Message, server handler and
preset placement are therefore shared **by construction**. The tier question that had
been open all program needed a read, not a mouse.

## ⛔ AND THE ARGUMENT DIVERGED — the actual finding

| where | value | provenance |
|---|---|---|
| shipped widget | **6** | a bare literal |
| every acceptance script | **8** | 47 of 47 |
| the bed plot | **8 cells** | 2×2×2, matching `kind=Bed jobs=8` in every live log |

**Every scored bar in this program ran at a population the shipped action never
produces.** Three numbers with nothing relating them — so nothing could notice them
drifting apart. *A number must carry its producer;* these carried nothing.

The fix is one constant in `common::bastion` (both ends see it) plus a **relation** the
code must keep: `bed_capacity()` derives capacity from the preset table, and the bar
requires `capacity ≥ count`. Asserting `8 >= 8` between two literals would have
re-encoded the bug.

**The value is inherited from the scripts (8, the saturated case), not from the widget's
6.** Changing it is a design call, not a refactor — which is precisely why it now sits
where such a call is visible.

## ⚠ A PROCESS ERROR I MADE MID-ROW

Reverting plant 1, I ran `git checkout -- bastion-server/src/bastion_founding_preset.rs`
on a file holding **uncommitted work** — destroying `bed_capacity()` and both new tests.
The tell was plant 2 reporting *"0 passed, 124 filtered out"*: the filter matched nothing
because the tests no longer existed. Had I not read the count, I would have scored plant
2 as "did not fire".

**Revert a plant with a targeted edit. Never `git checkout` a file that holds uncommitted
work.** Rebuilt by hand and re-verified before continuing.

## WHAT I DECLINE TO CLAIM

- **Not** that the widget tier now has an automated bar. Convergence is established **by
  reading**; the mouse/render path above the call is still unexercised, and no bar here
  touches it.
- **Not** that N3 proves the widget passes the constant. The driver sends 8 and the
  constant *is* 8, so the live emit cannot distinguish them. That the widget passes the
  constant is a **code fact**, not a live measurement.
- **Not** that earlier rows' conclusions hold at 6. They ran at 8, are reported at 8, and
  whether they survive at a smaller population is **registered open**.

## SESSION QUEUE STATE — ten rows closed

1–7 as recorded · 8. Cancel across restart (`71d06226a4`) · 9. Run attestation
(`003d583f96`) · 10. **The founding colonist count**, this document.

**Next:** the open question this row created — do A2-B (work pull, 47.6% vs 0.0%) and
A3 (the eat loop) still hold at a population other than 8? The bars were all scored at
the saturated case.
