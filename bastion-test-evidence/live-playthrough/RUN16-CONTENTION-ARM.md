# Run-16 — deliberate CPU-contention arm for `#63` (2026-08-09)

Same `script-09-milestone.txt`, same 72,300-tick budget, same footprint,
current tip, all diagnostics on. Independent variable: 8 sustained
CPU-bound busy-loop threads (bash, `while :; do :; done`, nohup-detached),
saturating all 8 physical cores of this box for the run's duration.
Pre-registered outcome bar (three-way) and scope limits written before
running — CPU-only proxy, not a driver-12 reproduction; kept in this
session's scratchpad.

## Non-vacuity — satisfied, run-16's own number stands alone

| | driver-12 | run-16 |
|---|---|---|
| `slow system execution` warnings | 22, up to 625ms | **5**, up to 875ms |
| wall-vs-nominal overage | ~324s / ~11.8% (driver-derived) | **~603s / ~29.3%** (server-derived) |

**Provenance flagged, not just measured**: driver-12's overage figure was
computed from the driver's own completion claim — exactly the measurement
this run's own divergence finding (below) shows can be unreliable under
contention. run-16's 29.3% comes from server-authoritative log lines
(tick field + timestamp on `IS-LOADED-FILTER-DIAG`). The two cells are
not directly comparable; **run-16's non-vacuity claim rests on its own
server-derived number being large on its own terms (29.3%, 5 real
warnings up to 875ms), not on "exceeding" driver-12's differently-sourced
figure.** Independently corroborated by Opus from the OS process table: 8
bash processes, ~1,065s CPU each over ~1,260s wall (~85% duty cycle per
thread), confirming the contention was real and sustained by a second
instrument (`Get-Counter` on my side, process-table CPU-time on Opus's).

**Corollary for driver-12 itself**: any estimate of driver-12's true tick
coverage built by scaling its driver-derived overage against run-16's
server-derived overage mixes provenance the same way the table above
originally did, and shouldn't be carried as a number. driver-12's actual
tick coverage is unknown and unknowable — the one instrument that could
have measured it directly (its own server-side tick-tagged log) is the
one the deletion incident destroyed.

Opus also disclosed a `find`-scan confound (09:11-09:14 local /
13:11-13:14 UTC, ~3min of disk I/O) run against this same worktree during
the window. Checked explicitly: none of the 5 `slow system execution`
warnings fall inside that window (nearest is 13:15:05, just after it
closes), and `dropped_by_is_loaded` is 0 both inside and outside it — the
confound doesn't touch either headline number.

## Result: dropped_by_is_loaded — zero, throughout

**All 4,234 samples, 0.** Population was a clean 8 (b_count=c_count=8)
for the entire healthy portion of the run, then a clean 0
(b_count=c_count=0) after the disconnect below — never a partial drop,
never `b_count > c_count`. The `is_loaded` filter did not selectively
drop any colonist at any point, under real, measured, non-vacuous
contention.

**Reading against the pre-registered table: overage climbed (29.3%,
comparable to or exceeding driver-12's 11.8%) AND `dropped_by_is_loaded`
stayed 0 throughout → the `is_loaded` hypothesis is REFUTED with its
precondition MET.** A real negative result, not a void one.

## Result: the fix, live, under sustained adverse contention — third independent confirmation

Rest crosses at tick 54,000 (1,800 sim-sec); run-16 reached tick 61,756
before the disconnect — 7,756 ticks of margin. **Did colonists sleep in
that window? Yes.** First `RestAt` job at tick 54,436 (13:31:09), first
`slept — rest restored` at 13:31:48 (job 699). **3 distinct sleep
completions** (jobs 699, 700, 711) before the disconnect truncated the
run. 16 `BREAKDOWN` events, 325 `RestAt` mentions, 2,416
`preempt_cooldown` mentions, 14,095 `no_food_found` (hunger correctly
dead-ending throughout — no food anywhere in this scenario).

This is the fix's third independent live confirmation — after the 10x
accelerated repro (script-14) and the shipped-rate clean run (run-15) —
and the strongest condition of the three, since it ran under real,
measured, non-vacuous sustained host contention. Only 3 of 8 colonists
completed a sleep cycle before truncation (run-15's clean run took ~9,000
ticks / 300 sim-sec from first `RestAt` to the 8th completion; run-16 only
had ~7,332 ticks of window after its first `RestAt` before the disconnect
cut it off — consistent with partial completion under a narrower window,
not with the fix failing). Scored via `logcount.py`-style byte-level
counting against the run-15 calibrator's method, not bare grep.

## The finding nobody was looking for: driver tick-tracking diverges from server truth under load

The driver's own log shows all 8 checkpoints and "script complete,
disconnecting" firing normally, at the expected ~40min mark. Taken alone,
that reads as a full, successful run. **It wasn't, on the server's own
clock.**

`ScriptCmd::Wait(n)` (`client/src/bin/bastion_playtest.rs:326-341`) calls
`client.tick()` exactly `n` times in a loop, paced by the *client's own*
`clock.tick()` — it does not poll or block on the server's authoritative
tick count reaching a target. Under sustained host contention (the driver
process competes for the same 8 physical cores as everything else), the
client's local tick loop can complete and disconnect on its own schedule
while the server has fallen significantly behind.

**Measured directly:** the server's `Client disconnected!` event fired at
`13:36:23.973330Z`, 52ms after the driver's own "script complete"
timestamp (`1786282583925` epoch-ms → `13:36:23.925Z`) — the two clocks
agree closely on *when* the disconnect happened. But the server's own
authoritative tick counter at that instant was **61,756 of 72,300 — 85.4%
of the intended budget, ~352 sim-seconds short.** The driver believed it
had run the full 40.2-minute script; the server had actually simulated
about 34.3 minutes of it in that same 40.2 minutes of wall time.

**This is a real, previously-unnoticed methodological gap**, not
specific to this arm: "the driver reported script complete" has been an
implicit proxy for "the server processed the full matched tick budget"
across every live run in this arc. Under contention, that proxy silently
breaks, and only comparing server-side tick+timestamp pairs directly (not
trusting driver-side checkpoint messages) reveals it.

## What this means for driver-12

**This is now a serious, testable alternative to the `is_loaded`
hypothesis for driver-12's own null.** If driver-12's driver similarly
disconnected "on schedule" by its own client-side clock while the server
was running significantly behind, the server may simply never have
reached the tick where hunger or rest cross their interrupt thresholds —
which would produce exactly the observed signature (zero preempts, zero
despondency, for the entire logged run) without needing any `is_loaded`
filter mechanism, colonist-count anomaly, or dropped-entity event at all.
driver-12's own raw log is gone, so this can't be checked directly against
that run — but the mechanism is now demonstrated to exist and to be
capable of silently truncating an apparently-complete run by double-digit
percentages.

## Population collapse at the tail — explained, not anomalous

`b_count`/`c_count`/`decay_join_count` all drop to 0 together at tick
61,768, immediately following the disconnect at tick ~61,756 — a
component-level despawn triggered by the client disconnecting, the same
mechanism already established for run-15's tail and the earlier ad hoc
4-colonist sanity check. Not an `is_loaded`-specific finding: all three
population instruments agree at every sample, before and after.

## Standing-protocol addition (proposed)

Score a live run's tick budget against the **server's own tick-tagged
log lines**, not the driver's "script complete" message — the two can
diverge substantially under host contention, and only the former is
authoritative.
