# WAVE 35 — ARM 2, THE UNCENSORED STALL DISTRIBUTION

**Binary:** `7f20a18438` · **Pin:** `bastion/pin-wave34-item6witness`
**Invocation** (recorded per protocol):

```
ZONES="us-east1-b us-east1-c us-central1-a" BRANCH=bastion/pin-wave34-item6witness \
  bash vm-pool.sh 4 e2-standard-8 12 49 "--b5-scenario" 25 90 "BASTION_ACCESS_STALL_SECS=9999"
```

**Wave JSON:** `corpus-waves/wave35_ARM2_UNCENSORED_7f20a18438_FULL.json`

## §1 ATTESTATION

4/4 VMs `COMMIT=7f20a184`, `DONE=12` each, **48 seed blocks**, no
`CREATE_FAIL` / `BUILD_FAIL` / `STALE`. Zone probe selected `us-east1-b` first try.

## ★ STEP 3 — **PROVEN ON 2 OF 4 VMs, AND THE GAP IS STRUCTURAL**

The prereg demanded the effective threshold be verified **on the VMs** before any
field was read. The effective-value log goes to **stderr, which the fan
discards** — so the log route is unavailable and the check had to be made from
data: **at the old threshold 120.0 was a hard ceiling; anything above it proves
the raised value took effect.**

| VM | seeds | max `stalled_peak` | verdict |
|---|---|---|---|
| VM0 | 49–60 | **120.0** (3 seeds pinned exactly) | ★ **UNPROVEN** — this is also the censoring signature |
| VM1 | 61–72 | **287.0** | **PROVEN uncensored** |
| VM2 | 73–84 | 91.0 | **UNPROVEN** — no seed approached the ceiling |
| VM3 | 85–96 | **141.0** | **PROVEN uncensored** |

> ## **A DATA-SIDE CONFIG PROOF ONLY WORKS WHERE THE CONFIG WOULD HAVE MATTERED.**
> *On an arm whose seeds never approach the old ceiling, the evidence that would
> distinguish "raised" from "not raised" cannot exist.* **VM2 is not suspicious —
> it is structurally unfalsifiable. VM0 is genuinely ambiguous: 3 seeds at exactly
> 120.0 and nothing above is what censoring looks like, and also what a real
> attractor at 120 looks like.**

**The `ENVPREFIX` is a single shared string in one ssh command, identical for all
four VMs, so partial arrival is unlikely on mechanism.** The honest statement is
that **24 seeds are proven uncensored and 24 are unproven**, not that half the fan
failed.

★ **INSTRUMENT GAP (files as a row): the effective config must reach STDOUT or the
JSON, not stderr.** *Then every seed carries its own configuration and no
inference from values is needed. This is the third time this session that stderr
being discarded has cost a verification route.*

## §6 THE REGISTERED QUESTION — **ANSWERED. 120 IS TOO LOW.**

**R** = self-resolved stalls (`peak > 0` and `final == 0`; with the pruner
disabled the **earned reset is the only reset**, so every member of R is a stall
that recovered through real progress).

    n(R) = 25        max(R) = 141.0
    R = 13, 38, 40, 40, 49, 52, 58, 60, 61x5, 62, 72, 74, 76, 83, 119, 120x5, 141

> ## **`max(R) = 141.0 >= 120` → THE REGISTERED PREDICTION IS CONFIRMED.**
> **A stall that recovered on its own ran 141 seconds. `ACCESS_STALL_SECS = 120`
> has been pruning access plans that would have recovered.**

**My prior was registered as `max(R) >= 119` and is confirmed at 141** — and
**141 comes from VM3, one of the two PROVEN-uncensored arms**, so the answer does
not depend on the ambiguous half of the wave.

**Also measured:** 6 seeds were **still stalling at run end** (`final > 0`), peaks
`24, 51, 61, 91, 120, 287`. **287 is a lower bound** — that stall never resolved
within the run.

### ★★ RECOMMENDATION — **WITHDRAWN. THE PRODUCER HUNT INVERTED IT.**

**What this section originally said, and what I told the architect (who ratified
it): raise to 180, "not above 287 — that seed never recovered, so it is exactly
the case the pruner should catch."** ★ **Both halves are now believed WRONG.**
See the producer hunt below. **The number is withdrawn before it shipped.**

**Superseding position: the threshold must be derived from the QUEUE BUDGET, not
from a corpus distribution** — because the distribution is *generated* by that
budget. See below.

## ★★ THE FINDING NOBODY REGISTERED — **120 WAS COINCIDING WITH SOMETHING**

wave35's distribution is **identical to wave33's** except at the top:

    wave33 (threshold 120):  ... 91, 119, 120 x8
    wave35 (threshold 9999): ... 91, 119, 120 x6, 141, 287

**Only 2 of the 8 censored seeds revealed a higher true value.** The other six
peaked at exactly 120.0 **with the pruner switched off** — so for those, 120 was
never the pruner's doing. **Three of those six are on VM1, whose env arrival is
PROVEN**, which rules out censoring as the explanation for at least half of them.

> **Something else releases these claims at ~120 s and earns the reset.** No other
> `120.0` constant exists in `bastion_jobs.rs` (only `ROWB_BENCH_TICKS: u64 = 120`,
> a tick count in a different scenario), so the coincidence is **unexplained and
> worth a row of its own** — and it means the old threshold was landing exactly
> where claims were already being released, making the stall prune largely
> redundant at that value.

## ★★★ THE PRODUCER HUNT — **ANSWERED AT SOURCE, AND IT RE-SCOPES ITEM 2**

*Architect-approved as "a producer hunt: WHAT releases an access claim at ~120 s
— no candidates named until read."* **One read answered it.**

`bastion_jobs.rs:6630`, in the emergency-route queue path:

```rust
const QUEUE_WAIT_BASE_TICKS: u32 = 120 * crate::SIM_TPS as u32;
const QUEUE_WAIT_PER_TURN_TICKS: u32 =  90 * crate::SIM_TPS as u32;
let budget = QUEUE_WAIT_BASE_TICKS + QUEUE_WAIT_PER_TURN_TICKS * position.min(8);
```

**A colonist queued for a COMPLETED access route — waiting its turn on the ladder
— is granted `120 + 90 × position` seconds.** While `within_budget` it is
`WaitingForLadder` and its `stuck_watch` is **wiped every pass**. When the budget
lapses **the suppression stops**, the stuck machinery engages, the claim is
released, and `access_stalled_secs` takes its earned reset.

> ## **AT QUEUE POSITION 0 THE BUDGET IS EXACTLY 120 SECONDS.**
> **That is the attractor: the six seeds peaking at exactly 120.0 with the pruner
> switched OFF are position-0 waiters whose lawful wait ran out.**

### ★★ WHY THIS BREAKS THE 180 RECOMMENDATION

**The legitimate wait is not a constant — it scales with queue position:**

    pos 0: 120s   pos 1: 210s   pos 2: 300s   pos 3: 390s   ...   pos 8: 840s

- **`ACCESS_STALL_SECS = 120` prunes a position-3 waiter at under a third of its lawful wait.**
- **180 still prunes every waiter at position ≥ 1.**
- ★ **`287` — which I called "exactly what the pruner should catch" — sits between
  210 (pos 1) and 300 (pos 2). It is plausibly a LAWFUL waiter, not a deadlock.**

### ★★★ WHAT IT MEANS FOR ITEM 2

**A correct threshold must exceed the maximum lawful budget — 840 s — or the
pruner kills colonists waiting their turn. Runs are 340–540 s.**

> **So a correctly-set threshold would essentially NEVER FIRE inside a run. Item 2's
> pruner has no demonstrable customer in this scenario, and every firing observed
> at 120 was landing on a waiter the queue mechanism was already managing.**

★ **And the calibration trap has a second coat:** the corpus distribution could
never have set this constant, because **the distribution is GENERATED by the queue
budget.** *A field cannot calibrate its own bound — nor a bound imposed by a
mechanism upstream of it.* **The constant is derivable from
`QUEUE_WAIT_BASE + QUEUE_WAIT_PER_TURN × 8` plus margin, and from nothing else.**

## STANDING

- **Prediction CONFIRMED; 120 is too low; recommend 180 as a BOUND, not a calibration.**
- **24 seeds proven uncensored, 24 unproven** — the distribution's upper tail is a
  lower bound on all counts.
- **Effective-config-to-stdout filed as an instrument row**; until then a raised
  threshold cannot be verified on arms where it would not have mattered.
- **The ~120 s claim-release coincidence is a new open finding.**
