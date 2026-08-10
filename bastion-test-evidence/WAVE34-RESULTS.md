# WAVE 34 — item-6 witness + item-2 threshold batch (ARM 1, defaults)

**Binary:** `7f20a18438` · **Pin:** `bastion/pin-wave34-item6witness`
(`7f20a184382ac3cbd697bf668ff0142b3d994b12`)
**Invocation** *(recorded here because wave33's was not, which cost this wave a
detour — see CORRECTION)*:

```
ZONE=us-east1-b BRANCH=bastion/pin-wave34-item6witness \
  bash vm-pool.sh 4 e2-standard-8 12 49 "--b5-rowb-paired" 25 90
```

**Wave JSON:** `corpus-waves/wave34_ITEM6WITNESS_7f20a18438_FULL.json` (paired)
· `corpus-waves/wave34_BASE_7f20a18438_FULL.json` (base arm lifted, comparable)

---

## ★ CORRECTION — I RAN THE WRONG INVOCATION, AND SAID IT WAS MATCHED

**I told both the architect and the builder this fan used "wave33's exact
parameters." That was false.** I copied the command from `WIP-STATE.md`, where it
belongs to a **different campaign** (`wave2N-ROWBPRIME`, branch
`bastion/wip-batch-verify`, `7590dfa962`) — the output filename says so on the
line above the command and I read past it.

**wave33 used `--b5-scenario`** (flat record, 107 keys). **I ran
`--b5-rowb-paired`** (two legs wrapped as `paired_base`/`paired_variant`).

**Root cause is a gap in my own practice, not just a slip: wave33's invocation
was never recorded anywhere.** I ran that wave and did not write down how, so
when I needed it I reconstructed it from an adjacent document. *A number must
carry its producer — and so must a run.*

**Two consequences, one benign and one not:**

1. **Benign:** `b5_rowb_paired` **spawns `--b5-scenario` as a subprocess**
   (`main.rs:23459`), so each arm *is* the wave33 scenario. `paired_base` is
   Jaccard **0.922** against wave33 and wave33's key set is a **strict subset** —
   the 9 extra keys are exactly this batch's new fields. **Lifting `paired_base`
   recovers a genuine matched control**, which is what the results below use.
2. **Not benign:** my validators flatten nested JSON, and `paired_base` /
   `paired_variant` share **all 116 field names**. So the first validation pass
   (`48/48`) was reading a **phantom merged record** — whichever arm flattened
   last. *It passed because the two arms happened to be identical here.* **A
   validator that silently merges two runs is worse than one that refuses.**

---

## §1 ATTESTATION

4/4 VMs `COMMIT=7f20a184`, `DONE=12` each, **48 `@@@SEED` blocks**, no
`CREATE_FAIL` / `BUILD_FAIL` / `BINARY_STALE` / `STALE`.

*Two earlier launches produced **0/48** and both reported `FAN_EXIT=0`
(`ZONE_RESOURCE_POOL_EXHAUSTED`, `us-central1-a` then `-b`). Logs preserved as
`wave34-fanlog-…-ZONEFAIL-*.txt`. See #83.*

## §3 CLAUSE DELTA — **THE HEADLINE: NOTHING MOVED, AT ALL**

Read first, per protocol, across **every** seed rather than the mover list.

| | wave33 | wave34 base |
|---|---|---|
| failing seeds | 12 | 12 |
| **seeds that GAINED a clause** | — | **0** |
| **seeds that LOST a clause** | — | **0** |
| **swaps (invisible to any count)** | — | **0** |

> **The batch is inert on outcomes.** Every one of the 48 seeds carries an
> identical failed-clause set to wave33. That is the designed result — the new
> fields are diagnostics that gate nothing — and it is now measured rather than
> assumed.

**This also settles the parked-mover unlock as the architect framed it:** the
build/mine families **PERSIST**, unchanged, seed-for-seed. They are not
bundle-transient.

## §6 THE REGISTERED PREDICTION — **VOID, NOT REFUTED**

**All six refusal counters are 0 across all 48 seeds, in BOTH arms:**

    b5_pickup_refused_pile_protected          0/48
    b5_pickup_refused_ambient_disabled        0/48   <- the precondition
    b5_pickup_refused_ambient_uids_distinct   0/48
    b5_pickup_refused_ambient_later_colonist  0/48   <- the discriminator
    b5_pickup_refused_loot_owned_colonist     0/48
    b5_pickup_refused_loot_owned_ambient      0/48
    b5_pile_pickup_by_member                  0/48   <- and B2's control
    b5_pile_pickup_by_nonmember               0/48

★ **`ambient_disabled == 0` means the refusal path NEVER FIRED**, so
`later_colonist == 0` carries no information about the colonist-timing race.
**The prediction is UNTESTED, not killed** — exactly the caveat the checker was
built to print, firing on its first real data.

★★ **And B2's control is the decisive part: `pile_pickup_by_member` is ALSO 0.**
*Nobody picked anything up from a pile — not ambient NPCs, not colonists.* **The
entire pile-pickup subsystem is inert in this scenario**, so the zero is not
"the gate permitted everything" and not "the gate refused everything" — it is
**the mechanism never ran**.

> **The witness is CORRECT and the SCENARIO is wrong.** This needs a different
> scenario, not a different instrument — and B2's member/non-member pair is what
> proved that, which is precisely why a refusal count alone was never enough.

### ★★ PREMISE-CHECK RESOLVED (5b, at source) — **IT IS STRUCTURAL, NOT A SCENARIO CHOICE**

**Zero occurrences of `create_item_drop` or `dropall` anywhere in
`bastion-harness/src/main.rs`, across all ~40 scenario functions.**

> ## **NO HARNESS SCENARIO PROVISIONS ANY ITEM DROP AT ALL — persistent or loose.**
> *Not "no scenario drives colonists to a pile." **No scenario ever creates a
> pile, or any pickup-able item, in the first place.***

★★★ **So the corpus fan STRUCTURALLY CANNOT exercise item 6**, at any seed count,
under any existing scenario. **This was never a re-run question and no number of
fans would have found it** — the premise-check answered in one read what a second
48-seed fan would have reproduced as another six zeros.

**Disposition (architect-ruled, premise now answered "no"): the nine fields stay
as DORMANT SENTINELS** — they cost nothing, and they self-report the day any
scenario engages the subsystem in-corpus. **A bespoke ambient-NPC fixture is NOT
built on spec**; the det-fixture feasibility taxonomy gates that and this is its
first clause failing.

**Item 6's verification therefore rests on its LIVE acceptance, which stands on
its own evidence** (5b, script-12 driver, not the harness): colonists ate from a
persistent pile repeatedly across a 25-minute run while it survived untouched by
ambient pickers, against an equivalent loose drop taken by an ambient NPC in
**37 s** pre-fix. **That was always item 6's real arena.** What the corpus lacks
is not confidence in the mechanism but a fan-visible witness of it.

### What this does and does not license about item 6 and the movers

**It does NOT clear or implicate item 6 for the movers.** A tempting reading —
*"the gate never refuses, so it cannot be starving the build/mine jobs"* — is
**not available**, because the upstream AI gate (`ambient_item_looting_enabled()
→ false`) can prevent an attempt from ever reaching the server counter (task
#78's shape). **Zero refusals is consistent with both a permissive gate and a
fully-suppressed one.** The member/non-member zeros say the subsystem is inert
here; they do not say why.

## §5 CENSORING — UNCHANGED, AS EXPECTED AT DEFAULTS

`stalled_peak` still tops out at exactly **120.0** (31/48 nonzero) — arm 1 ran at
default thresholds by design. **`stalled_final` now exists**: nonzero in **6/48**,
max **91.0** — i.e. 6 seeds were **still stalling at run end**. The calibration
remains arm 2's job.

## ★ SPECIMEN-SEED FIELD DIFF — **THE MOVERS' MECHANISM IS ACCESS-PLAN, NOT ITEM 6**

*Architect-directed, free, from data already on disk: diff the FULL records of
one build-family and one mine-family specimen and let the clause-adjacent fields
name the candidates before anything runs.*

### SEED 51 (mine family) — 22 scalar fields moved wave32→wave33

**The failure is exactly ONE BLOCK:**

    b5_mine_blocks_mined      27 -> 26        b5_mine_jobs_remaining   0 -> 1
    b5_stone_sum              27 -> 26        b5_mine_cleared       True -> False

**And the access plan collapsed around it:**

    b5_live_is_access_count            37 -> 0     <- ZERO access jobs
    b5_access_pending_true_ticks     4459 -> 1470
    b5_access_plan_emergency_calls     23 -> 14
    b5_blocked_regions_count_at_settle  0 -> 1     <- a region went blocked
    b5_timeouts_on_never_completed_jobs 0 -> 2

### SEED 50 (build family) — 14 moved, access plan moved the OTHER way

    b5_any_needs_materials   True -> False    b5_build_placed  True -> False
    b5_live_is_access_count    15 -> 27       <- MORE, not zero
    b5_access_pending_true_ticks 7879 -> 9049
    b5_access_plan_emergency_calls  3 -> 12   <- 4x

> ## **BOTH SPECIMENS MOVE HARD ON ACCESS-PLAN FIELDS, IN OPPOSITE DIRECTIONS —
> one to zero access jobs, one to nearly double.** *That is the signature of a
> changed access-planning regime, not of a material-protection gate.*

**Candidate mechanisms therefore live in the wave32→wave33 access-plan work** —
#70's F3 accumulators, #68's F3-BRANCH port, item 2's stall sweep, and the B17
DPA sweep fix that routed pruning through `remove_job`. **Item 6 is not a
candidate on this evidence**: its own fields are inert (§6) and nothing in the
specimen diff touches material protection.

★ **This is a candidate set, not a conclusion.** The next step is to bisect
within the access-plan commits, not to run another fan.

### ★★ FREE BONUS — **DETERMINISM HELD ACROSS A ZONE AND MACHINE CHANGE**

wave34 ran in `us-east1-b` on different physical instances from wave33's
`us-central1-a`. **Across all 48 seeds, every scalar field is IDENTICAL between
wave33 and wave34-base except two:** `b5_build_stamp` (expected — different
commit) and `b5_soak_avg_tick_ms` (wall-clock timing).

> **The harness is bit-reproducible across zones, hosts, and machine instances.**
> *That was assumed by the determinism-by-construction programme and is now
> measured, for free, as a side effect of an infrastructure failure forcing the
> region change.* **It also retroactively licenses the machine-type fallback
> (8 × `e2-standard-4`) that was held as a changed variable — on this evidence it
> is free.**

## STANDING

- **Batch is inert on clause outcomes** — measured, 0/0/0 across 48 seeds.
- **Movers PERSIST** unchanged; still unattributed.
- **Item-6 witness VOID in this scenario** — needs a scenario that exercises pile
  pickup. The instrument is not at fault.
- **Arm 2 (calibration) unaffected** by any of this; still blocked on #83's
  env-forwarding fix.
