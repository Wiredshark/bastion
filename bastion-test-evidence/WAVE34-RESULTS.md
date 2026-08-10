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

## STANDING

- **Batch is inert on clause outcomes** — measured, 0/0/0 across 48 seeds.
- **Movers PERSIST** unchanged; still unattributed.
- **Item-6 witness VOID in this scenario** — needs a scenario that exercises pile
  pickup. The instrument is not at fault.
- **Arm 2 (calibration) unaffected** by any of this; still blocked on #83's
  env-forwarding fix.
