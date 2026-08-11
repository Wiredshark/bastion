# ITEM 8 (ENDURANCE RUN) — pre-registered failing-observation criteria

Written before launch, per Opus's explicit pre-flight requirement 5:
*"Write the failing observation for each of the six bar rows before launch. A
check that cannot go red is not a check."* Also closes pre-flight requirement 6
(zero cases = VOID, never PASS) and the packet's own §5.3 (every bounded/capped
field carries a truncation flag, read before any distribution).

**Boot stamp:** to be filled from the actual scored run's boot log — `commit`,
`branch`, binary mtime vs latest source edit. Not backfilled from memory (per
the standing "never type hashes/pins from memory" rule).

**Instrumentation this run depends on** (all landed and compiling clean on
`bastion/wip-batch-verify` as of this doc):

| producer | fires at | commit |
|---|---|---|
| `ate`/`slept` completion logs, now `uid`-tagged | `EatFrom`/`RestAt` completion | `0e58e08a1c` |
| food-stock periodic sampler (`"bastion food stock sample"`) | every 300 ticks, unconditional | `0e58e08a1c` |
| `NeedCrossed{need, dir}` entity events | rest/hunger interrupt-band edge, need-order loop | `cad8d9e1a6` |
| effective mood config (decay/interrupt/comfort) | boot, unconditional | prior session |
| `ULTIMATE FAIL-SAFE` teleport log | egress fail-safe fire | prior session |
| `stalled_final` / `access_stalled_secs` | F3 pruner pass | prior session |

## 1 · THE CYCLE, NAMED FROM LIVE CONFIG — filled at run start, not from memory

The packet requires the actual crossing times be read from the live config at
run start and put in the results header. **Planning estimate only** (from a
prior throwaway boot's config read, to size the run — the scored run re-reads
its own boot log for the real numbers):

    rest:    decay_per_sec=0.000444, interrupt=0.2  -> full-to-interrupt ≈ 0.8/0.000444 ≈ 1802s ≈ 30.0 min
    hunger:  decay_per_sec=0.000889, interrupt=0.2  -> full-to-interrupt ≈ 0.8/0.000889 ≈  900s ≈ 15.0 min

**Cycle = the REST need's full-to-interrupt period (~30 sim-min), not the
vanilla day/night `DAY_LENGTH_DEFAULT`.** Rationale (unchanged from pre-flight
derivation): rest is the slower-crossing need, so a rest-keyed window gives
measure 3 (sleep) one expected crossing per cycle — the cleanest mapping — while
hunger crosses roughly twice as often inside the same window, which measure 2's
per-cycle accounting handles by counting distinct eaters within the window, not
by expecting exactly one eat. **5 cycles × ~30 min ≈ 2.5 hours**, matching
Opus's stated wall-clock estimate at the stated 1:1 ratio for this run mode
(NOT the ~9× headless divergence noted elsewhere — that note applies to a
different run mode and must not be silently reused here).

These are PLANNING numbers, trait-stagger-free. **The scored run's actual
per-colonist crossing times (which vary with the Conscientious/Neurotic
stagger) are read from that run's own `NeedCrossed` records and stated in the
results, not assumed from this table.**

## 2 · THE ZERO-WINDOW (packet §3, restated as this run's own check)

    cycle window          hunger        rest       REQUIRED
    0 -> first crossing   above         above      ZERO eats, ZERO sleeps
    after crossing        below         --         the event, EVERY cycle, EVERY colonist

A satisfying event before its need crosses does not just fail to help — it
VOIDS the leg (the outcome arrived by a path that is not the mechanism). The
pre-crossing window is checked first, before any pass/fail scoring below.

## 3 · THE SIX MEASURES

### Measure 1 (REVISED per the packet's own amendment — no death path exists)

Grepped fresh for this doc: zero hunger/hunger-need-to-Health/death coupling
anywhere in `bastion-server`, `common`, or `server`. Confirmed still true at
time of writing. **Operative wording (Fable's amendment, not the original bar
row):**

> No colonist's need stays in the interrupt band across a full cycle without a
> satisfying `NeedCrossed{OutOf}`.

- **Witness:** entity event log, `ColonistEventKind::NeedCrossed{need, dir}`
  records (subject `Uid`, `tick`).
- **PASS expression:** every `Into` record for a given `(uid, need)` has a
  matching `OutOf` for the same `(uid, need)` within one cycle-length (≈1802
  sim-sec) of ticks.
- **FAIL expression (concrete):** exists at least one `(uid, need)` `Into`
  record with no `OutOf` for that same pair within one cycle-length — either a
  late `OutOf` beyond the window, or none before run end.
- **VOID:** zero `NeedCrossed` records exist for the whole run despite the
  food-stock sampler (an independent, unconditional producer) having fired —
  this is the "zero from a dead counter" trap this arc has hit three times
  already; a truly empty needs-crossing log with a live sampler means the
  producer is dead, not that no colonist ever got hungry.
- **This branch is stated as a finding about the sim in the results, not a
  footnote** — with no death path, item 8 asks "does the colony stay
  RECOVERABLE," not "does it survive," per the packet's own honesty
  requirement.

### Measure 2 — every colonist eats every cycle

- **Witness:** `"bastion: ate — hunger restored"` log lines, now `uid`-tagged
  (`0e58e08a1c`), partitioned into cycle windows by `tick`.
- **PASS expression:** for every cycle window after the zero-window, distinct
  `uid` count in that window's `ate` lines == colony size (8).
- **FAIL expression:** any cycle window (post-zero-window) with distinct-eater
  count < 8.
- **VOID:** zero `ate` lines logged across the entire run despite hunger
  crossing `Into` at least once (per measure 1's own witness) — dead instrument,
  not a starving-but-unfed colony (that would be measure-1/5 territory with a
  live instrument).

### Measure 3 — every colonist sleeps every cycle

Same shape as measure 2, over `"bastion: slept — rest restored"` lines
(`uid`-tagged, `0e58e08a1c`). Same PASS/FAIL/VOID structure, colony size 8.

### Measure 4 — food stock does not trend down (★ the duration-only measure)

- **Witness:** `"bastion food stock sample"` log lines (tick, food_stock),
  unconditional 300-tick cadence (`0e58e08a1c`).
- **Baseline: cycle 2, not cycle 1** (cycle 1 is bootstrap, not steady-state —
  packet's explicit instruction).
- **PASS expression:** `food_stock` at cycle N ≥ `food_stock` at cycle 2, for
  every N ≥ 2, i.e. the per-cycle minimum sample never dips below the cycle-2
  sample.
- **FAIL expression:** monotone decline in the per-cycle sample across ≥3
  consecutive cycles (the packet's stated FAIL shape — a single-cycle dip that
  recovers is noise, a 3-cycle decline is the slow leak this row exists to
  catch).
- **VOID:** not expected (sampler is unconditional, tick-driven, no
  reachability dependency) — if it fires anyway, the sampler itself is dead and
  the whole run is VOID, not just this measure.

### Measure 5 — no permanent stall

★ **Interpretive choice, stated plainly, not hidden:** the packet's phrase
"idle-with-unmet-need" has no separate witness of its own — building one (a
new "is this colonist currently pursuing any job" classifier keyed against
need state) would be a second producer for data the `NeedCrossed` edge-detector
already carries. **This measure reuses measure 1's same witness with the same
window**, on the reasoning that a colonist actively being served (walking to a
bed, queued at a stove) crosses back `OutOf` inside the window in every run
observed so far this arc; a crossing that *doesn't* resolve inside one cycle is
this project's operational definition of "idle," not merely "slow." If this
interpretation is wrong it will show up as measures 1 and 5 always agreeing
100% of the time in the results — which itself is the falsifiable prediction
this framing makes.

- **Witness:** same `NeedCrossed` records as measure 1.
- **FAIL expression:** identical to measure 1's FAIL — any `(uid, need)` `Into`
  with no `OutOf` inside one cycle-length.
- **Secondary corroborating signal (not the primary witness):**
  `board.b5_f3_stalled_peak` / `access_stalled_secs` — the B6 access-economy
  stall counter. This measures a DIFFERENT population (claimed access jobs
  making no progress, not need-driven idleness) and is reported alongside, not
  substituted in, per the named-failure-mode table's own separate "Stall
  accumulation" row.

### Measure 6 — fail-safe rate does not climb

- **Witness:** `ULTIMATE FAIL-SAFE` teleport log line count, partitioned into
  cycle windows.
- **PASS expression:** teleports-per-cycle flat or falling across N.
- **FAIL expression:** teleports-per-cycle rising across cycles (any
  cycle-over-cycle increase counts; the trend, not a single spike, is what
  climbs).
- **VOID:** does not apply — zero teleports in every cycle is a legitimate
  (good) value here, unlike measures 1–5's zero-instrument trap.

## 4 · TRUNCATION / CAPPED-FIELD FLAGS (packet §5.3)

No field this run reads is silently capped: `NeedCrossed` records are
individual entity events (ring-buffered per the existing entity-event-log
capacity/truncation-flag machinery already built for `released_records`), the
food-stock sampler is a plain accumulate-and-log with no cap, and the
`ate`/`slept` lines are per-event, uncapped. **If the entity-event-log ring
reports its truncation flag as set at any read during results compilation,
that is read and reported BEFORE any distribution is computed from it** — per
the standing "instrument changes what it sees" / "field cannot calibrate its
own bound" rules.

## 5 · WHAT REMAINS BEFORE LAUNCH

- Colony size confirmed at 8 (founding default) from the scored run's own boot.
- Endurance driver script: found, designate, then **disconnect** for the scored
  unattended window (a connected driver is a client, per the ruling — presence
  is a variable).
- Liveness protocol: name a releasing event + producer-alive ping interval
  (Opus's ask) before launch, not during.
- Fresh binary rebuild against `cad8d9e1a6` before launch.
