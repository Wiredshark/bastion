# WAVE 35 — ARM 2, THE CALIBRATION FAN (PRE-REGISTERED)

**Written BEFORE arm 1 (wave34) reported**, so nothing here can be retrofitted to
its data. Arm 1 = movers + item-6 witness at default thresholds. **Arm 2 = the
uncensored stall distribution, and nothing else.**

**Pin:** reuse `bastion/pin-wave34-item6witness` (`7f20a184382ac3cbd697bf668ff0142b3d994b12`).
**No rebuild** — the threshold is env-tunable as of that commit, which is the
whole point of A1.

---

## ★ THE DESIGN CORRECTION — "RAISE IT TO 600" WOULD NOT HAVE WORKED

**A run is 342–535 sim-seconds of F3 passes** (wave33 branch-C counts, ~1 pass ≈
1 s). **So any threshold at or above ~540 never fires inside a run.**

> **Raising the threshold does not un-censor the distribution. It DISABLES THE
> PRUNER.** *A "raised" threshold of 600 and one of 9999 are the same experiment.*

**That is still the right arm — but for a reason worth stating, because it changes
how the result reads:**

### ★★ WITH THE PRUNER DISABLED, THE EARNED RESET IS THE **ONLY** RESET

`access_stalled_secs` has exactly two reset paths: the threshold sweep, and the
earned reset when progress occurs. **Remove the first and every observed
peak-without-final is unambiguously a SELF-RESOLVED STALL.**

**Set `BASTION_ACCESS_STALL_SECS=9999`** — comfortably above any run — and read
`stalled_peak` against `stalled_final`:

| reading | meaning | value for calibration |
|---|---|---|
| `peak = N`, **`final = 0`** | a stall of **N seconds RESOLVED BY EARNED PROGRESS** | ★★★ **THE GOLD DATA.** The threshold must exceed every such N, or it kills work that would have recovered |
| `peak = N`, `final ≈ N`, `N ≈ run length` | **still stalling when the run ended** | censored by RUN LENGTH, not by the threshold. Reads as *"at least N"* — a lower bound only |
| `peak = 0` | never stalled | no information about the threshold |

**This is exactly the peak-vs-final discrimination that `stalled_final` was built
for**, and it only becomes readable once the pruner stops manufacturing resets.

---

## THE REGISTERED PREDICTION — BOTH BRANCHES AS FIELD EXPRESSIONS

*Per the standing law: if the refutation has no expression, the instrument is not
ready. And the placement check — these fields are not inside any branch that
fixes their value, because the pruner that would have is switched off.*

Let **R** = `{ stalled_peak : final == 0 and peak > 0 }` — the self-resolved stalls.

| | expression | consequence |
|---|---|---|
| **`ACCESS_STALL_SECS = 120` IS TOO LOW** | `max(R) >= 120` | ★★ Stalls that recover on their own take longer than the current threshold. **120 has been killing recoverable work** — raise to `max(R)` plus margin |
| **`120` IS DEFENSIBLE** | `max(R) < 120` | No self-resolving stall ever needed that long. The 8 wave33 seeds censored at 120 were **genuine deadlocks**, and the current value stands |
| **UNDECIDABLE** | `R` is empty | No stall ever self-resolved. **VOID — not a pass for either side.** The scenario cannot calibrate this constant and we say so |

★ **My prior, registered so it can be wrong:** wave33 showed a **non-pruned peak
of 119.0** (seed 59) — one second under the wire, and at that time indistinguishable
between "recovered" and "run ended." **I expect `max(R) >= 119`, i.e. the first
row: 120 is too low and sits inside the recoverable range rather than above it.**
*If `R` turns out empty or small, my raise-never-lower argument from wave33 was
over-read from a single censored seed and I will say so.*

---

## ★ SCOPING — WHAT ARM 2 MUST NOT BE USED FOR

> ## **ARM 2's CLAUSE OUTCOMES ARE NOT A MOVER READ. DO NOT COMPARE THEM TO wave33 OR wave34.**

**The pruner is OFF.** Stalled access jobs are never swept, so they accumulate for
the whole run — a **real behavioural change**, deliberately introduced. Build and
mine clauses may move for that reason alone.

- **Arm 1 (wave34)** answers: do the movers persist, and does the colonist-timing
  prediction fire.
- **Arm 2 (wave35)** answers: what is the true self-resolving stall duration.
- **Neither answers the other's question**, and a result read across them is the
  bundle-diff error in a new coat — *controlling the commit is not controlling the
  mechanism.*

**Report arm 2's clause failures only as a sanity note** ("the pruner being off
did / did not visibly change outcomes"), **never as a regression or a fix.**

---

## PROCEDURE

1. Confirm arm 1 scored and its pin is unchanged.
2. ## ★★★ BLOCKER, VERIFIED — **`vm-pool.sh` DOES NOT FORWARD ENV TO THE HARNESS. ARM 2 CANNOT RUN UNTIL IT DOES.**

   **Checked at `vm-pool.sh:42`, the only harness invocation:**

   ```sh
   ./target/verify/bastion-harness $ARGS --seed \$s --data-dir /tmp/mf-\$s >/tmp/mf-\$s.json 2>/dev/null &
   ```

   **No env prefix. No `SendEnv`/`AcceptEnv`. No `export` anywhere in the remote
   block** (grep returns nothing). `BRANCH` is consumed by the LOCAL shell for the
   `git reset`; it never crosses the ssh boundary as environment.

   > **So `BASTION_ACCESS_STALL_SECS=9999 bash vm-pool.sh ...` would run all 48
   > seeds at the DEFAULT 120.0** — and the result (*no stalls beyond 120*) is
   > **indistinguishable from the interesting negative finding.** A clean-looking
   > fan, a confident wrong calibration, ~$0.5 and 25 minutes spent proving
   > nothing.

   **This is the exact failure this pre-registration was written to catch, caught
   before the spend rather than after** — and it is why step 2's verification was
   never optional.

   **REQUIRED CHANGE (deferred — see below):** add an optional env-prefix
   parameter, e.g. `ENVPREFIX="${8:-}"`, applied at the invocation:

   ```sh
   \$ENVPREFIX ./target/verify/bastion-harness $ARGS --seed \$s ...
   ```

   ★ **DO NOT MAKE THIS EDIT WHILE A FAN IS RUNNING.** `sh` reads a script
   incrementally, so editing `vm-pool.sh` in place can corrupt the executing run —
   wave34 is using it right now. **Land the change after arm 1 completes**, then
   arm 2 becomes the one-liner it was supposed to be.

3. **THEN verify the value actually arrived**: the effective-value line from A1's
   unconditional startup log must read **9999**, on the VMs, before any field is
   read. *Two independent ways this arm can silently run at 120 — the missing
   forward and a malformed value — and both look like "no long stalls."*
3. Validate every seed (`wave34_validate.py` — the `final <= peak` invariant still
   holds and still discriminates).
4. Compute **R**, read the table above, and report `max(R)` with its seed.
5. Recommend the constant as `max(R)` + margin, **or** report VOID if `R` is empty.

★ **Step 2's verification is not optional.** The single most likely failure of
this arm is the env var not arriving — and its signature (*no long stalls
observed*) is identical to the interesting negative result. **Read the startup log
before reading anything else.**
