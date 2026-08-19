# TICK-LOADING CERTIFICATION — DISPOSITION

The three registered bars, each scored on the axis it names, with the
compressed-mode decision stated.

---

## BAR 1 — N=8 promotion-distribution re-run shows capped/uncapped OVERLAP
### **PASS** (refutation target was *zero overlap*)

| | |
|---|---|
| runs | **62** (30 matched twin pairs) — well past N=8 |
| capped promoted key set | `d597163d6340`, 242 keys |
| uncapped promoted key set | **`d597163d6340`, the same hash** |
| cap=8 planted arm | **`d597163d6340`, the same hash** |

**Not overlap — SET IDENTITY.** Capped, uncapped and the planted arm promote the
*identical* 242 chunks. The original finding (zero overlap, 6× promotion shift
under compression) is refuted in the strongest available form.

---

## BAR 2 — determinism fingerprint holds with loading inside it, **including chunk timing**
### **FAIL** — on the timing clause, and it propagates

| clause | result |
|---|---|
| membership (which chunks) | **IDENTICAL, 30/30 matched-input pairs** |
| **schedule (which tick)** | **DIFFERS, 31/31 pairs**, both anchor classes |

Controlled for the harness defect found this session: 20/20 origin-anchored pairs
and 10/10 real-anchored pairs both diverge, so the defect is excluded as cause.

★ **And the divergence reaches game state.** On the first arm ever carrying the
barrier *and* live colony observables (`endurseed`, name-hash matched):

| observable | differing tick-aligned samples |
|---|---|
| `preempt_attempts` | **816 / 905**, to the **final tick** |
| `designated_sweep_reaps` | 291 / 905 |
| `crop MATURE` | **8 vs 26** |

Across 14 identically-seeded runs the colony outcome is **bimodal**: 7 THRIVE
(936–2,015 maturations) / 7 COLLAPSE (8–46), **registered gap 50–500 empty**.

---

## BAR 3 — planted control: re-introduce one wall-coupled read → fingerprint red **by name**
### **PASS**

Plant: `BASTION_A3_PLANT_WALLCLOCK` — a `SystemTime::now().subsec_nanos()` read
re-introduced **at the stage this row disables** (the due-set bound), panicking
on a mis-specified modulus so an unlabelled arm cannot produce a number.

| arm | plant emits | promoted-per-tick support |
|---|---|---|
| **P0 control** | 0 | **max 4** — 92% of promoting ticks sit at the budget |
| **P1 modulus=8** | 11,422 | **1 … 8** |
| **P2 modulus=2** | 11,407 | **1 … 2 only** |

★ **Red by name in the strongest sense: `max = modulus`, exactly.** The
instrument does not merely report "something changed" — the planted parameter is
**recoverable from the distribution's support**, and the half-strength arm ranks
strictly between control and full strength, which is what the A3 registration
demanded of a severity metric.

★ Membership stays IDENTICAL under the plant. That is coherent, not a miss: the
wall-coupled read perturbs **how many chunks promote per tick**, not **which**
chunks promote — the same membership/schedule split bar 2 measures unplanted.

---

# DISPOSITION

| bar | verdict |
|---|---|
| 1 — capped/uncapped overlap | **PASS** (set identity, n=62) |
| 2 — fingerprint incl. chunk timing | **FAIL** |
| 3 — planted control red by name | **PASS** |

## ★ COMPRESSED MODE IS **NOT** DECLARED DEFAULT

The standing law is *on green*. **Bar 2 fails**, so the trigger does not fire and
**the runners are not updated.** This is a withheld default, deliberately, not an
oversight — and it is now backed by a measured consequence rather than by a
tripped clause: the residual nondeterminism selects between two colony outcomes
**20× apart**, so an unattended compressed run could not be trusted to reproduce
the behaviour it was launched to observe.

**What would change it.** Bar 2 is the only failing bar and it fails on the
*schedule* clause alone. If schedule-identity is judged out of scope for this
row, bars 1 and 3 already pass and the row lands — **but that scoping call is
Ben's, and the evidence now argues against it**, because the schedule difference
demonstrably reaches game state.

## What the row DID achieve

The wall-clock coupling it existed to remove **is removed**: promotion no longer
depends on the capped/uncapped axis (bar 1, set identity). That was the row's own
roadmap criterion and it passes. What remains is a platform-level scheduling
residual that #89 excluded ten candidates against.

---

# ★★★ FINAL DISPOSITION — all bars scored, and bar 2's failure is ORTHOGONAL TO COMPRESSION

## The measurement that decides it

| population | diverges |
|---|---|
| **CAPPED** twin pairs (compression **OFF**) | **35 of 35 — 100%** |
| **UNCAPPED** twin pairs (compression **ON**) | **4 of 4 — 100%** |
| **HEADLESS** (no client) | **0 of 6 — 0%** |

**The divergence is 100% whenever a client is attached, with compression on or
off, and 0% without one.** Compression does not cause it, does not worsen it,
and turning compression off does not avoid it.

★ So **withholding compressed mode on account of bar 2 protects against
nothing.** The failure it names is a property of running a networked client at
all, and it is already present in every capped run the program has ever made.

## The three bars, final

| bar | verdict | evidence |
|---|---|---|
| **1 — capped/uncapped OVERLAP** (refutation target: zero overlap) | **PASS**, strongest form | capped, uncapped **and** the plant land on the **identical** 242-key set; n=62 runs, registered N=8 |
| **2 — fingerprint holds with loading inside** | **ENGINE: PASS** / **WITH CLIENT: FAIL** | headless **6/6 identical across hosts**, 165 chunks, 71 active ticks; driven **38/38 diverge**, always first at request arrival |
| **3 — planted control red by name** | **PASS**, strongest form | `max = modulus` **exactly** (P0 max 4, P1 1–8, P2 1–2); the planted parameter is recoverable from the data |

## What bar 2 actually measures

Every server-side candidate was eliminated **by measurement**: #89's ten, the
chunk-send ordering fix (ran 11,400×, changed nothing), the request-side barrier
(engaged 226×, moved the divergence onto a boundary but did not remove it). The
engine-only arm then came back **tick-exact across two physical machines**.

**Bar 2 is measuring the client's arrival timing, not the engine.** Client and
server are separate processes with independent tick loops; no server-side change
can align them, and the row never claimed to.

## ★ RECOMMENDATION — and why it is a recommendation

**The evidence supports making compressed mode the default:** bar 1 (the actual
compression-safety bar) passes in its strongest form, bar 3 passes, the engine is
deterministic with loading inside it, and bar 2's failure is measurably
independent of compression.

**I am not flipping it unilaterally.** The standing law says *on green*, bar 2 is
not green by its letter, and changing the default for **every unattended run** is
a standing, consequential change — the class Ben reserved for himself. Redefining
"green" to fit the evidence is exactly the move a builder should not make alone,
however good the evidence.

**What is needed is one ruling:** does bar 2 certify *the engine* (**PASS**) or
*the engine plus a networked client* (**FAIL, and unfixable server-side**)? On
the first reading the trigger fires and the runners flip; on the second the row
closes as measured-and-bounded rather than open.
