# FOUNDING PRESET v1 — **SMOKE TEST RESULT (first live run, ever)**

**2026-08-12. Binary `6c2991eb` (= HEAD of `bastion/wip-batch-verify`), read from the
server log.** *Resourced flat arena, throwaway userdata
`.engine-integration-wt/userdata-preset-smoke`, `--no-auth`.*

**Evidence:** `server-founding-preset-smoke.log` (289 lines) ·
`driver-founding-preset-smoke.log` · `script-founding-preset-smoke.txt`.

> ## ✅ **THE PLAYER PATH WORKS. ALL FOUR NAMED CHECKS PASS ON THEIR FIRST LIVE RUN.**

---

## 1 · THE FOUR CHECKS (handoff §3)

| # | check | result |
|---|---|---|
| **1** | `colony founded ... complete=true elements=stockpile,farm,bed` | ✅ |
| **2** | three `founding preset plot placed` lines | ✅ **exactly 3** |
| **3** | `farm plot registered ... unresolved=0` | ✅ **resolved=30 unresolved=0** |
| **4** | second spawn → `founding refused reason=colony_exists` | ✅ |

```
bastion: colony founded preset="v1" pos=Vec3 { x: 16384.5, y: 16384.5, z: 400.0 }
    datum=400 colonists=8 elements=stockpile,farm,bed complete=true
    jobs=8 designated_regions=3
bastion: founding refused reason="colony_exists" pos=... column=None
```

**Also observed:** 8 colonists spawned AND promoted to loaded entities; farm jobs
created per column (job=8…); the player-visible refusal message reached the client
(*"Your colony already lives in this world…"*). **0 panics.** *(The single ERROR-shaped
line is the driver's own disconnect — `BrokenPipe` at channel shutdown, logged INFO.)*

### GEOMETRY — the offsets reproduce exactly against `datum=400`

| element | region | expected from §1 offsets |
|---|---|---|
| stockpile | `(16382,16380,399)..(16386,16385,400)` | `(-2,-4,-1)..(+2,+1,0)` ✅ |
| farm | `(16377,16380,399)..(16381,16385,400)` | `(-7,-4,-1)..(-3,+1,0)` ✅ |
| bed | `(16381,16381,400)..(16382,16382,401)` | `(-3,-3,0)..(-2,-2,+1)` ✅ |

---

## 2 · ⚠ THREE FINDINGS — **the smoke test's real yield**

### ★★★★★★ F-1 · THE Z-DATUM TEST DID **NOT** FIRE — the handoff's "free" check is false as run

**Handoff §3 claims: *"the driver founds at z=401 while the datum is z=400 … if the
plots come out one block low, the datum derivation broke."***

    player pos at script start:  z = 400.0
    datum:                       400

> ## **THE PLAYER'S z EQUALS THE DATUM. "DERIVED FROM TERRAIN" AND "TAKEN FROM
> `pos.z`" PRODUCE THE SAME ANSWER HERE — SO B1's REGRESSION IS NOT EXERCISED.**

★★★ **Mechanism:** *the arena spawns at 401 (`+1` for landing jitter) but the player
FALLS to 400 and has settled long before the script's first verb runs.* **The +1 is
transient; the founding never sees it.**

**B1 remains covered at UNIT tier only** (`datum_is_derived_from_terrain_not_from_the_reported_z`).
⚠ **It is NOT live-exercised, and this run must not be cited as if it were.**

**To exercise it live, the founding must happen from a z ≠ the column's first air
cell.** *The driver has no move/teleport verb (`anchor`/`spawn`/`designate`/`wait`/
`list_designations`/`note`), so on a flat arena this is **not reachable with the
current driver** — it needs either a driver verb or a site with relief (the resourced
arena's stone outcrop).* **Named as a gap, not faked.**

### ⚠ F-2 · THE SEED DROP HAS NO WITNESS EMIT

*Handoff §1 behaviour 4: founding drops `FOUNDING_SEED_STOCK` (8 seeds).* **A grep for
`seed|FOUNDING_SEED|drop` across the whole server log returns NOTHING.**

★★★ **So the seed drop is UNCONFIRMABLE from the log.** *Not a claim that it failed —
a claim that it has no name-the-line witness, which the program's own law requires for
anything a bar reads.* **A5/A3 will need it before either can be scored on stock.**

### ⚠ F-3 · THE DRIVER'S `list_designations` REPORTED `rev=0 []` — AND IT NEARLY MISLED ME

**Three regions were placed and the server log proves it; the driver's client-side
view showed none.** ★★★ *Recorded loudly because the first read of this run looked like
a total failure of the feature.* **The driver's designation mirror is not a valid
witness for placement — the SERVER log is.** *Handoff §3 already says "read the SERVER
log"; this is why.*

---

## 3 · WHAT THIS RUN DOES **NOT** SHOW

- **A2 (colonists stay)** — not measured; no R defined yet (§8 N4).
- **A3 (till→sow→eat)** — not exercised; and per §8 B3 the first eat waits on a harvest.
- **B1** — see F-1.
- **F8-INCLUSION** — no chop/mine designations were placed by the preset, so no real
  Mine/Chop completion occurred. *The resourced arena's tree/stone were never
  designated: the preset places stockpile/farm/bed only.* ★★ **F8's inclusion evidence
  needs its own designate step in the acceptance script.**

## 4 · VERDICT

> ## **SMOKE PASSES. The player path founds a full preset, refuses the second founding
> by name, and resolves every farm column — on its first live run, having never run
> before.**

★★ **Three gaps are now named that were not named before, and one of them (F-1)
retires a claim the handoff made about this very test.**
