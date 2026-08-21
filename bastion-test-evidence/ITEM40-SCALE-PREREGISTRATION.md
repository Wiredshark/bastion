# ITEM 40 (colony scale) — pre-registration, 2026-08-21

The arc index records: *"arms exist; first measurement VOID (co-load), re-run
owed."* The arms did **not** exist in `run-pit.sh` — they are built by this
row. The VOID stands and is the reason this is being redone properly.

## Why the first attempt was VOID, and why now is different

Scale is a COST measurement, and cost is wall-coupled. The first triplet ran
while other work shared the machine, so the three arms were not comparable to
each other — a bigger colony on a quieter host can measure *faster* than a
small one on a loaded host, and nothing in the numbers would say so.

**This project's own law:** local pins and VM fixtures never run in parallel.
Tonight the machine is quiet: no play agents, no concurrent builds, no other
legs. The triplet runs **sequentially on slot 0**, which is the arm the
contention caveat in `run-pit.sh` reserves for timing-sensitive rows.

## The instrument now exists

Item 39 landed a tick-cost ring (p50/p95/max, microseconds) that did not exist
when scale was first attempted. That is the measurement this row needs, and it
is deliberately wall-clock — sim time is a fixed step and would report the same
number on a host ten times slower. It lives outside the labor paths for exactly
that reason.

## BARS

**Bar 1 — the triplet is COMPARABLE.** All three arms run sequentially, on the
same host, same binary, same script, differing only in
`BASTION_AUTOFOUND_COLONY`. Each attestation is quoted. Without this the other
bars are meaningless, which is precisely how the first attempt died.

**Bar 2 — cost is MEASURED at each size**, not asserted: p50/p95 tick cost
reported for 8, 16 and 32 colonists.

**Bar 3 — the colony still WORKS at 32.** Scale that degrades the colony into
uselessness is a failure even if the tick cost is flat. The EXPERIENCE census
(working / idle / stuck) must show a colony doing work at 32, not merely a
server that keeps ticking. This is the bar that stops "it didn't crash" from
being read as "it scales".

## PREDICTION

Cost rises with population — the job board, claim scan and census are all
per-colonist. The row is not "cost must stay flat"; it is **"cost must be
measured and the colony must still function."**

## FALSIFIERS

- Any arm's attestation STALE or dirty ⇒ that arm is VOID, not slow.
- Census shows `working≈0` at 32 with work available ⇒ bar 3 FAILS: the colony
  scaled into paralysis, and the tick cost is irrelevant.
- Wildly non-monotone cost across the triplet ⇒ contention leaked in after all;
  say so and re-run rather than reporting the numbers.
