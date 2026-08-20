# Item 31 (God-hand v1, SERVER half) — PRE-REGISTRATION

**Authoritative design exists and is reviewer-approved**: `readme/GOD-POWERS-
DISPATCH-SPEC.md` (POWER-0/1 "FEASIBLE-WITH-CHANGES", no blockers) + CATALOG
+ GOD-HAND-design. The hand/UI half is the renderer lane's; THIS row is the
server-authoritative cast pipeline the spec's FR5 revision demands.

## Build shape (v1 = POWER-0: the pipeline + ONE real power)

1. **Cast request → server**: reuse the designation client→server message
   pattern (the spec names it); v1 driver entry = a Bastion chat command
   (the BastionPriority pattern) so the pit can cast headlessly.
2. **Favor gate server-side**: a colony `favor` meter (placeholder trickle
   regen per the spec's (d)); a cast refuses loudly when unaffordable —
   refusal-aware consumer, witnessed.
3. **Dispatch applies the REAL effect + the VFX** (spec ★(f)): v1 power =
   **Smite** — lethal/health damage via the health system + the
   already-server-emittable `Outcome::Lightning`. No command smuggling
   (★(b)): Smite touches health only, never activity/jobs.
4. **Witness + chronicle**: cast (power, target, cost, favor-after) and
   effect (damage applied) — treatment beside outcome; a chronicle record
   so mood/thoughts can weigh it later (the item-23 table takes it as data).

## BARS

1. A cast with sufficient favor applies the real effect (target health
   drops / dies) AND the witness carries cost + favor-after.
2. A cast WITHOUT favor refuses loudly and applies NOTHING (the null's
   couldn't-happen witness = the refusal line itself).
3. Determinism: same-seed twin legs, same favor trajectory, same outcome.
4. No-command invariant: the target's jobs/arbiter state untouched by the
   cast (assert: ActiveJob unchanged through a non-lethal cast).

VOID branches: no colonist in range (fixture); the Outcome emits but health
does not move (the ★(f) light-show failure — the exact defect the spec
names); favor never regenerates (report the meter, do not hand-set it).
