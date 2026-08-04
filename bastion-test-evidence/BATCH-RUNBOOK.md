# BATCH RUNBOOK — single-seed diagnostics, 7 items

**Executable without re-deriving anything.** Engine tip **`a85dec2912`**.
All items are **local, single-seed, no fan** — zero VM minutes.

Interpretation rules are **frozen** in `SCENARIO-MAP.md`'s reconciliation table
(`5e18ded778`). **Do not amend them after seeing a result.**

---

## STEP 0 — attest the binary BEFORE anything else

```bash
RUSTC_WRAPPER="" cargo build -p bastion-harness --profile no_overflow -j 48
./target/no_overflow/bastion-harness.exe --print-git-hash     # must match HEAD
git rev-parse --short=10 HEAD
```

**`RUSTC_WRAPPER=""` is mandatory** — sccache is user-global and has served
stale objects across sessions on this machine. **A cross-crate field addition
is exactly the change that trap eats.**

**Compare sha parts only** (`${GH%%+*}`); a `+dirty` suffix is expected on the
VMs (LFS noise) and *not* expected in a clean local worktree.

## STEP 1 — field PRESENCE **and** AGREEMENT

**Presence catches a stale binary. AGREEMENT catches a MISWIRED counter.**
Presence alone is not sufficient.

Run seed 71 and check:

| check | expectation |
|---|---|
| the 9 new `b5_access_plan_*` fields appear | non-null in real output |
| `emergency_emissions` vs corpus `access_emissions_max: 3` | **equal — but only because `members_seen == 1`** |
| `starvation_cycles` re-read | **exactly 360** (deterministic) |

> **★ THE TRAP: the emissions check is an EQUALITY only where
> `members_seen == 1`.** Seeds 66 / 92 / 80 have `members_seen` 2 / 3 / 3, so
> `access_emissions_max` is a **per-member maximum** while the new counters are
> **totals**. **There the check is `total ≥ max`.** A legitimate `total > max`
> read as a miswired counter would be a false alarm *from the guard itself* —
> the most corrosive kind, because it teaches people to override the guard.

**Any `starvation_cycles` drift from the corpus indicts the BINARY or the SEED,
never the colony.** That makes the batch self-attesting.

## STEP 2 — the seven items

| # | seed | invocation | question | branch A | branch B |
|---|---|---|---|---|---|
| 1 | **71** | per-attempt trace | **shape A vs B** | attempts EXIST and fail ⇒ **A** (aging/cooldown) | **ZERO attempts** ⇒ **B** (cap/round-robin) |
| 2 | **66** | per-attempt trace | why contention resolves here | — | — |
| 3 | **61** | `--b5-settle-iters <N>` | slow vs stalled | completes late ⇒ **window artifact** | never completes ⇒ **real stall** |
| 4 | **90** | `--b5-settle-iters <N>` | same, claimed-and-stuck variant | as above | as above |
| 5 | **92** | raised-cap probe | UNKNOWN → known | probe completes ⇒ known | still incomplete ⇒ **cap not binding** |
| 6 | **80** | column scan `[24484,26192,153]` | is the site multi-layer? | single-surface ⇒ **negative STANDS** | multi-layer ⇒ **negative UNSOUND**, caveat everywhere |
| 7 | **run** | `--run-sample-ticks <N>` | overhead vs real gap | ratio → **1.25** ⇒ window overhead | stays **~1.14** ⇒ **real speed shortfall** |

> **★ No item can return "inconclusive."** Every null is named and **redirects**
> a row rather than failing it.

### ★ Item 7 hazard — `--run-sample-ticks 0`

**Never pass 0.** It divides by zero → `inf`, and `ran_faster` evaluates
`inf > inf * 1.15` = **false**: **a garbage result shaped like a clean fail**,
which is the taxonomy's worst object, inside the flag built to avoid producing
one. **A `max(1, ..)` or an assert is filed for whoever is next in that file.**

**Use a clearly larger window (e.g. 3–5× the 45 default), and run the default
once in the same session** — the default must reproduce `walk=0.263 run=0.300`
exactly, which attests the flag changed nothing when absent.

## STEP 3 — reading results

1. **Attestation before verdict, every time** — binary hash, exit code, **log
   size** (an exit-0 with a 12-byte log is the empty-log false green).
2. **Strip ANSI before any count-grep** — tracing's colour codes sit *inside*
   field values, so a literal `field=value` match returns 0 on a coloured log.
3. **`-C <repo>` on every git call** — a multi-worktree session will otherwise
   run the command in whichever tree the shell last entered.
4. **Assert types, never guard them away** — a filter that silently drops a
   class reports the remainder as the total.
5. **A null result is written up with the same rigour as a positive one**, and
   claims that survive checking are **recorded as having survived** — otherwise
   the ledger only ever carries bad news and people stop running the checks.
