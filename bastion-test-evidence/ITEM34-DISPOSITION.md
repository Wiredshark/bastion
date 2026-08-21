# ITEM 34 (raids scale with wealth) — DISPOSITION: **PASS, 4/4 bars**

Scored against `ITEM34-PREREGISTRATION-V2.md`, registered before any leg.
Arms `injury` (band 0) and `raidrich` (band 1+), both attested fresh,
`dirty .rs 0`.

## Bar 1 — raids FIRE when there is something worth taking: **PASS**

```
ITEM 34 RAID — pressure scaled to what the colony is worth taking wealth=135 raiders=1
ITEM 34 RAID — …                                                  wealth=136 raiders=1
ITEM 34 RAID — …                                                  wealth=142 raiders=1
```
Six raids, wealth 135–162, `raiders=1` — the correct count for band 1
(64..=255). The band arithmetic is read from the emit, not assumed.

## Bar 2 — SILENCE is the null, and it was OBSERVED: **PASS**

Seven raid opportunities on the low-wealth arm, every one `raiders=0`, wealth
0–56 — all below the 64 floor. A colony with nothing worth stealing is never
raided, and this is the row's own couldn't-happen witness rather than an
absence of evidence.

**This bar is why the row is trustworthy.** Bar 1 alone would pass just as
happily if the wealth gate were decorative and raids fired always.

## Bar 3 — raiders arrive from OUTSIDE: **PASS**, and it clears a false defect

`dist_from_origin=48.0` on every placement. Raiders land exactly 48 blocks from
the colony centre.

An adversarial play session had reported raiders *"spawning on top of the
colony's own stockpile"* — reading the emitted `origin` (the stockpile centre)
as the spawn point, because **the line never printed `pos`**. The system was
correct; only the log was ambiguous. That ambiguity cost a good report its
accuracy on this point, and a correct system that reads as broken costs exactly
as much investigator time as a broken one.

## Bar 4 — DETERMINISM: **PASS**

Twin `raidrich` runs, four raider placements each, compared as exact position
strings:

```
IDENTICAL — determinism holds
```

**This bar exists because the spawner was drawing OS entropy earlier the same
day** (self-caught, `09970131ae`). Two same-seed runs would have placed raiders
differently — silently, because a raid still fires and still logs. The twin
comparison is what turns "I fixed it" into evidence.

## Instrument work this row required

Both nulls in this row were originally unreadable, and both were fixed before
scoring rather than reasoned around:

- The raid tick **returned silently at band 0**, so "the cadence never met
  sufficient wealth", "the band arithmetic is wrong" and "something refused to
  spawn" rendered identically. Every opportunity now emits wealth/raiders.
- The raid emit **printed only `origin`**, never the spawn position — the
  source of the false defect above.

The first of those immediately paid: it showed `matbeds` could not test bar 1
at all (7 opportunities, wealth 0–56), which is why the `raidrich` arm exists.

## Known residual

Wealth counts stockpiled units only. A colony holding its goods in colonists'
bags reads as poor and is never raided — arguably correct (nothing to steal
from a stockpile), arguably a loophole. Recorded, not chased.
