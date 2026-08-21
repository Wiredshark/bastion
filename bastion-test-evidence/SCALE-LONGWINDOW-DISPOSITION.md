# Scale: **not warm-up — a DECAY.** The colony starts at 21/32 working and settles to 4

Answers the question registered in `SCALE-CONSTRAINT-CORRECTED.md`: *is the food
shortage at 32 colonists a scaling property or a warm-up artifact?*

**It is neither.** It is a decay, and that is a third answer neither branch of
the registered question anticipated.

## The measurement

`scalelong` — 32 colonists, ~33,900 ticks (2.8× the original window), attested
fresh, `dirty .rs 0`.

```
tick=300     total=32  working=21  idle=10     ← 21 of 32 EMPLOYED
tick=7800    total=32  working=4   idle=24
tick=15300   total=32  working=3   idle=26
tick=22800   total=32  working=4   idle=22
tick=30300   total=32  working=4   idle=22     ← still 4, 30,000 ticks later

whole run: n=113  mean working=3.75  idle=23.60  share=12%
beds built: 32
wheat stockpile, entire run: units 2 … 5, reserved_units == units, always
```

## Why this settles it

**`working=21` at tick 300 is the load-bearing number.** It refutes both
branches of the original question at once:

- **Not warm-up.** A colony still spinning up would start LOW and climb. This
  one starts HIGH and falls — the opposite shape. Extending the window 2.8×
  changed the mean share from 9% to 12%, i.e. nothing.
- **Not a capability ceiling.** The machinery *can* employ 21 of 32
  simultaneously. It demonstrably did. Whatever caps it at 4 is not the job
  board's arbitration, the claim path, or the colonists.

## What actually distinguishes tick 300 from tick 7800

At tick 300 the colony is spending its **seeded** materials (256 units). By
tick 7800 those are consumed, and from then on it lives on what it *produces*.

**So: seeded material supports 21 workers; produced material supports 4.** The
gap between those two numbers is the colony's sustained-throughput deficit,
and it is now quantified rather than described.

The wheat stockpile never rises above 5 units across 33,900 ticks, with every
unit reserved — so production is consumed the instant it lands and no buffer
ever forms.

## Still not diagnosed, and deliberately

*Which* production step is the limiter — farming rate, harvest→stockpile
hauling, or the sow/harvest cycle time — is unresolved. The haul census already
refuted the obvious candidate (quota not binding, `pending=1 cap=64`), and I
have published one wrong mechanism tonight by outrunning the evidence. The next
instrument should measure the food *pipeline* per stage, not the symptom at
its end.

## Bearing on the NPC-conversion charter

Ben's charter takes a colony from 8 to ~30 in one step. This row says the extra
people would be employed *briefly*, on whatever stock the village already holds,
and then idle — because sustained throughput, not population, is the binding
constraint. Adopting a village's **stores and fields** matters as much as
adopting its people.
