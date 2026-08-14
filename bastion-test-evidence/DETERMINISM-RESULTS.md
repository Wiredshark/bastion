# DETERMINISM FOR SCORED LIVE MAGNITUDES — **RESULTS & ROW DISPOSITION**

Scored against `DETERMINISM-PREREG.md` (`81a8ae0158`). Engine tip `9332a553c8`, attested.

## THE SCORE

| bar | verdict | evidence |
|---|---|---|
| **D1** two deterministic runs identical | ⚠ **PARTIAL — and the second attempt was VACUOUS** | colonist identity reproducible; scored magnitudes untested |
| **D2** the control can disagree | ✅ PASS *(already measured)* | h8a vs h8b: 14 vs 8 harvested, 5 vs 0 hauls, 4 vs 0 stock |

## ATTEMPT 1 — VOID, NOT FAIL

Two runs with `BASTION_DETERMINISTIC=1`, driver-founded:

| | harvested | hauls | peak stock | colonist names |
|---|---|---|---|---|
| det1 | 28 | 15 | 18 | *(8 names)* |
| det2 | 40 | 14 | 32 | **entirely different** |

My first reading was *"the flag doesn't work."* **Wrong — the bar was mis-specified.**

Colonist generation seeds on `tick_rng(world_seed, seed_tick, key)` with
`seed_tick = state.tick` **at the moment the founding message arrives**. The driver
connects at a wall-clock-dependent moment, so **the two runs had different inputs.** The
code says so itself, at `bastion_spawn_colony_seeded`:

> *"the live `data.tick` is NOT deterministic at boot in a real server (rtsim generation
> advances it a variable amount before the colony is founded), so a fixed `seed_tick`
> pins colonist identities and spawn positions across runs."*

**Two runs are only comparable if their inputs match, and I let the founding tick float.**
Declaring "determinism is broken" from that would have been confident, wrong, and
entirely publishable.

## ATTEMPT 2 — THE POSITIVE RESULT, AND THE VACUITY UNDER IT

Re-run through the supported path: `BASTION_DETERMINISTIC=1` **+
`BASTION_AUTOFOUND_COLONY=8`** (the seeded founding).

**✅ Colonist identity is reproducible.** Both runs produced the *same eight names* —
`Awen Longstride · Doran the Stout · Hesta the Steady · Lira Ironhand · Nia the Quiet ·
Osric of the Vale · Wynn Brighteye · Wynn the Wary`. Given a pinned `seed_tick`, the RNG
authority delivers.

**⛔ But every scored magnitude was 0 in both runs** — harvested, tilled, hauls, stock.
Checked rather than celebrated: the autofound path emits `spawned starting colony` and
promotes 8 colonists, but **`colony founded: 0`, `plot placed: 0`, `designation placed:
0`.** It is a colonist-spawn shortcut, **not a founding** — the preset placement lives in
the message handler it bypasses.

> **So "the two runs matched exactly" is `0 == 0`.** Vacuous on precisely the counters
> that vary. Reporting it as D1 PASS would have been the cleanest false green of the
> session.

## ATTEMPT 3 — THE VACUITY FIXED, AND THE REAL ANSWER

`place_preset` extracted so the autofound path founds a **real** colony (engine
`abc725800b`). Both legs then had work to compare, with the founding **byte-identical** —
same eight colonists, same order — and both ran to **tick 9000**:

| | plots | tilled | harvested | sown | hauls | peak stock |
|---|---|---|---|---|---|---|
| d3a | 3 | 30 | 10 | **12** | **7** | **20** |
| d3b | 3 | 30 | 10 | **10** | **3** | **12** |

> ## **IDENTITY AND GEOMETRY REPRODUCE. THROUGHPUT DOES NOT.**

- **Reproduced:** colonist names, `plots=3`, `tilled=30`, `harvested=10` — all geometric
  or capped by the plot.
- **Diverged:** `sown` 12/10, `hauls` 7/3, `peak_stock` 20/12 — all throughput.

### ⚠ AND THAT CONCLUSION IS ITSELF CONFOUNDED — corrected on the same day I wrote it

I first wrote: *"`BASTION_DETERMINISTIC` does not make the work simulation
reproducible."* **That claim is not supported, and I am withdrawing it.**

Diffing the two logs with timestamps and the userdata tag normalised out: they agree for
**574 lines**, and the divergences are

1. the **boot UUID** — documented as *"excluded from authoritative simulation state/RNG
   keys, never persisted"*, so not a simulation input; and
2. **`Accepting Tcp` at line 262 vs 246** — the client connects at a **different point in
   the server's own tick sequence**.

`autofound` sits at line **198 in both**. The founding is pinned. But **the client is what
loads the chunks**, and work cannot proceed on unloaded terrain — so a connect that lands
16 lines apart starts the whole work trajectory from a different tick offset.

> **The client-connect tick is an uncontrolled input, exactly like the founding tick in
> attempt 1.** Same error class, third occurrence in one row, caught each time only by
> checking the premise before reporting the number.

**D1 verdict: PASS on identity and geometry; throughput is UNRESOLVED — not FAIL.** What
is measured is that the **end-to-end live pipeline** is not reproducible. Whether the
*simulation* is deterministic remains untested, because no run has yet held the client
arrival fixed.

## ✅ ATTEMPT 4 — THE ANSWER. THE SIMULATION **IS** DETERMINISTIC.

Attempt 3's correction named the last input: the client. Without a colony presence entity
the only thing keeping the colony's chunks loaded is a connected client, and work cannot
proceed on unloaded terrain — so a connect landing at a different point in the tick
sequence moved everything downstream.

`bastion_found_colony_presence` already existed (`COLONY_PRESENCE_VIEW_DISTANCE = 1`) with
two call sites; the deterministic autofound path was not one of them. It is now
(`8173de1bfa`), so the colony loads its own chunks and a capture needs **no client at
all**.

**Two headless legs, `clients=0` in both:**

| | plots | tilled | sown | harvested | hauls |
|---|---|---|---|---|---|
| hd1 | 3 | 30 | **50** | **22** | **24** |
| hd2 | 3 | 30 | **50** | **22** | **24** |

**Identical — including every throughput counter that diverged in all three earlier
attempts.** And stripped of timestamps, the userdata tag and the boot UUID, the two logs
are **1035 lines and identical line for line.**

### THE ISOLATING CONTROL — because "headless matches" could have been the wrong reason

Same headless setup, **`BASTION_DETERMINISTIC` unset**:

| | tilled | sown | harvested | hauls |
|---|---|---|---|---|
| nd1 | 30 | **12** | 12 | **3** |
| nd2 | 30 | **36** | 22 | **14** |

Without the flag the same configuration spreads `sown` 12→36 and `hauls` 3→14. **The
match is the flag's doing, not an artefact of removing the client.** Without this control,
"two headless runs agree" would have been consistent with *nothing varying headlessly at
all* — a false green wearing the right numbers.

**D1 verdict, final: PASS.** `BASTION_DETERMINISTIC` delivers a bit-identical simulation
when — and only when — every input is pinned: the founding tick (seeded autofound), the
work (shared `place_preset`), and the chunk-loading trigger (server-owned presence).

## ★ WHAT THIS ROW ACTUALLY ESTABLISHES

**Live scored magnitudes cannot currently be made reproducible**, and the reason is
structural, not a bug:

1. The **seeded** founding pins colonist identity but places **no work**, so nothing
   varies *and* nothing is measured.
2. Any work introduced through the **live driver** arrives at a wall-clock-dependent
   tick, which is exactly the input attempt 1 failed to control.

**The practical rule, which is this row's deliverable:** score live bars on
**separations** (A2-B: 47.6% vs 0.0%) or **geometric invariants** (chop yield 15, split
5/6/4; XP 120 = 40/48/32; `tilled = 30`, the farm's 5×6 columns) — **never on a single
stochastic magnitude**. Every bar this program has closed already satisfies that; the
haul row's H3 did not, and it is the one that got refuted.

## ⚠ HALF MY COUNTERS CANNOT VARY — worth knowing before choosing a metric

`tilled = 30` matched across **every** run, deterministic and not: it is the farm plot's
5 × 6 columns. The chop yield's 15 and its 5/6/4 split are likewise geometry. Those make
excellent bars and **useless determinism probes** — a fact I nearly learned the expensive
way, having originally proposed the chop script for this row and rejected it in the
prereg for exactly this reason.

## WHAT I DECLINE TO CLAIM

- **Not** that `BASTION_DETERMINISTIC` is broken. Attempt 1 was **void by an uncontrolled
  input**; attempt 2 showed the RNG authority reproducing colonist identity exactly.
- **Not** that determinism covers the work simulation. **It is untested** — no run has yet
  produced non-zero work under a pinned seed, and that is the honest state.
- **Not** that closed rows need re-running. Their separations and integers are immune to
  this spread, as argued in the prereg **before** these results existed.

## SESSION QUEUE STATE — sixteen rows closed

…14. The XP witness · 15. Haul throughput · 16. **Determinism**, this document.

**Next:** the gap this row names — **an autofound path that also places the preset**,
which would make live work-magnitudes reproducible for the first time. That is a real
row, not a tidy-up: it is the precondition for ever scoring a live magnitude again.
