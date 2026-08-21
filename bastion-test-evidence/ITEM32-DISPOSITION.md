# ITEM 32 (faith economy) — DISPOSITION: **PASS**, and the index was stale

The arc index recorded *"the SPEND half is not built."* Checked against disk:
**both halves are built and both are live.** Scored from arms already run and
attested this session (`smite`, `scale32diag`, `injury`), because re-running a
leg to observe what three attested legs already recorded would be ceremony, not
evidence.

## Bar 1 — favor is EARNED: **PASS**

```
ITEM 32 favor favor=0.0533  earning=true  drive=Grow
ITEM 32 favor favor=12.05   earning=true  drive=Expand
ITEM 32 favor favor=20.0    earning=true  drive=Expand   ← reaches FAVOR_CAP
```
A trickle that accrues and caps, rather than a field that exists.

## Bar 2 — favor is SPENT, with a loud refusal: **PASS**

Both branches of the gate observed in one arm:
- refused when poor — `smite refused — favor 2 < cost 5`
- cast when rich, and the pool visibly drops: `favor=20.0` → `favor=4.05`

The refusal branch matters more than the cast: a cost that can never bind is
not a cost. This is item 31's POWER-0 machinery, and it is exactly what makes
item 32 an *economy* rather than a counter.

## Bar 3 — the CRISIS GATE holds, and its falsifier is clean: **PASS**

Favor stops accruing while the colony is in a food crisis:
```
ITEM 32 favor favor=1.19  earning=false  drive=Sustain
```

**The exclusivity is the evidence.** Across three attested arms, every single
`earning=false` sample carries `drive=Sustain` — and `earning=false` appears
with **no other drive**:

```
$ grep earning=false … | sort -u
earning=false drive=Sustain
```

`earning=true` meanwhile appears under Grow, Expand *and* Defend. So this is an
exclusive relationship, not a correlation that happened to hold: a god's favor
does not grow while the colony starves, and it does grow while the colony
merely fights.

## What is NOT claimed

**Spend VARIETY.** Smite is the only sink. A faith economy with one thing to
buy is a working economy with a thin catalogue — that is item 33 (miracles),
already chartered as the successor, and this row does not borrow its credit.

## Index correction

The "SPEND half is not built" line was wrong, not merely out of date: spending
has been live since item 31 landed. Two rows were describing one mechanism from
opposite ends and neither page noticed. Corrected in `ARC-PROGRESS-INDEX.md`.
