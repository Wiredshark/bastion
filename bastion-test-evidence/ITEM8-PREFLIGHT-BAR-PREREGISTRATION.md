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

### Measure 1 (REPLACED — Opus's ruling, superseding this doc's earlier draft)

Grepped fresh for this doc: zero hunger/hunger-need-to-Health/death coupling
anywhere in `bastion-server`, `common`, or `server`. Confirmed still true at
time of writing — "no deaths" against an engine with no death path cannot go
red, so it is not a check (Opus's vacuity ruling).

★ **Correction to this doc's own earlier draft:** an earlier revision of this
section used the `NeedCrossed{OutOf}` interrupt-band framing as measure 1's
replacement. **Opus explicitly ruled against that** in favor of a better
analogue found mid-thread: `mood_below_since` / the existing despondency
breakdown, "the engine's own notion of a colonist going under" rather than an
inference about needs. **Operative measure 1:**

> Despondency events per cycle: FLAT OR FALLING = pass, RISING ACROSS CYCLES =
> fail — the same trend shape as measure 4, for the same reason (the failure
> this run exists to catch is cumulative, not a single bad moment).

- **Witness:** the EXISTING `"bastion: BREAKDOWN — despondent (mood sustained
  below the break threshold)"` log line (`bastion_jobs.rs:10462`), already
  `uid`-tagged (`colonist = %uid`) — **no new producer**, per the same
  reuse-over-invent law this arc keeps re-applying.
- **PASS expression:** despondency-event count per cycle window is flat or
  falling across N.
- **FAIL expression:** despondency-event count rises cycle-over-cycle.
- **Reachability — the vacuity check applied to the REPLACEMENT too** (Opus's
  own instruction: "apply the vacuity check to the replacement, don't repeat
  the mistake one message later"). **Confirmed empirically, not just from the
  mood formula**, by grepping this arc's own prior live-run logs for
  `BREAKDOWN`:

      server-stdout-16.log (script-09-milestone.txt, NO food provisioning
        -- the closest prior analogue to item 8's founding-stock-only
        scenario)                                          16 BREAKDOWN events
      server-stdout-51.log (script-10-milestone-food.txt, provisioned)  13
      server-stdout-15.log (continuous-supply variant)                  14

  **Despondency is reachable, and fires MORE under the less-provisioned
  scenario** (16 vs 13–14) — consistent with need pressure (not food-search
  specifically) driving it. Measure 1 is not vacuous; the ZERO-WINDOW fallback
  is not needed.
- **VOID:** zero `BREAKDOWN` lines across the whole run despite mood-tracking
  being unconditional and this exact scenario shape reliably producing
  double-digit counts in every prior comparable run — would mean the log
  pipe itself is dead, not that the colony never stressed.
- **The mood formula, for the record** (`mood_formula`,
  `common/src/comp/bastion.rs:273`): `mood = clamp(mood_base(0.6) +
  hunger.weight(-0.5)·shortfall(hunger,0.5) + rest.weight(-0.4)·
  shortfall(rest,0.5) + recreation.weight(-0.15)·shortfall(recreation,0.4) +
  thought_sum, 0, 1)`, `break_minor = 0.25`. Worst-case BOTH hunger and rest
  sitting exactly at their 0.2 interrupt thresholds simultaneously yields
  mood ≈ 0.33 before any `thought_sum` contribution — margin is thin enough
  that real travel/queue delay past the interrupt edge (not just the
  instantaneous crossing) plausibly closes it, matching the empirical counts
  above.
- **Scenario fact for the results, not a bar row:** *"No deaths occurred.
  Starvation does not reduce Health anywhere in this codebase — hunger feeds
  mood only. This is a fact about the sim, not evidence of colony health, and
  the endurance question is therefore RECOVERABILITY, not survival."*

### Measure 2 — every colonist eats every cycle

- **Witness:** `"bastion: ate — hunger restored"` log lines, now `uid`-tagged
  (`0e58e08a1c`), partitioned into cycle windows by `tick`.
- **PASS expression:** for every cycle window after the zero-window, distinct
  `uid` count in that window's `ate` lines == colony size (8).
- **FAIL expression:** any cycle window (post-zero-window) with distinct-eater
  count < 8.
- **VOID:** zero `ate` lines logged across the entire run despite hunger
  crossing `Into` at least once (per measure 5's `NeedCrossed` witness) — dead
  instrument, not a starving-but-unfed colony (that would be measure 5's own
  territory with a live instrument).

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
already carries (`cad8d9e1a6`; ratified by Opus for measures 2/3's
per-cycle distinctness, and reused here on the same reasoning). A colonist
actively being served (walking to a bed, queued at a stove) crosses back
`OutOf` inside the window in every run observed so far this arc; a crossing
that *doesn't* resolve inside one cycle is this project's operational
definition of "idle," not merely "slow." **Unlike measure 1 (now the
despondency trend, above), this measure's witness is need-crossing duration
directly** — the two measures are no longer the same instrument, so no
agreement prediction between them applies; this is a standalone reading of
`NeedCrossed`, not a reuse of measure 1's data.

- **Witness:** `NeedCrossed{need, dir}` records (`cad8d9e1a6`).
- **FAIL expression:** any `(uid, need)` `Into` with no matching `OutOf`
  inside one cycle-length.
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

## 5 · THE LIVENESS PROTOCOL (Opus's pre-flight requirement)

**Releasing event:** the driver script (`script-15-item8-endurance.txt`) ends
after founding + designating + a 3-checkpoint confirm window (~60s), then the
process exits — dropping the client connection. That exit is the release into
the scored unattended window; the server keeps running alone from there.

**Producer-alive ping:** the food-stock sampler (`"bastion food stock sample"`,
`0e58e08a1c`) is unconditional and tick-driven — no reachability, no client, no
colonist-state dependency. It cannot go silent unless the server process itself
has died. **Ping interval: ~20–25 minutes wall** (well inside one ~30-min
cycle) — read the server log's tail, confirm (a) the process is still alive,
(b) the most recent sample's `tick` value is strictly greater than the prior
check's, (c) the byte count has grown. Three consecutive identical ticks across
checks is treated as "the run died at cycle N" — **which the packet's own
words make a RESULT, not a failed run** ("if it dies at cycle 3, that is a
RESULT, not a failed run").

## 6 · WHAT REMAINS BEFORE LAUNCH

- Colony size confirmed at 8 (founding default) from the scored run's own boot.
- ~~Endurance driver script~~ — done: `script-15-item8-endurance.txt`.
- ~~Liveness protocol~~ — done, §5 above.
- Fresh binary rebuild against `cad8d9e1a6`/`d73f4d7ebb` before launch.
- `BastionSpawnColony` itself is gated only on `bastion_terrain_anchor` +
  count bounds, not admin (checked `server/src/sys/msg/in_game.rs`) — this
  script needs no `server-cli admin add` step, since it uses neither
  `give_item` nor `dropall` (the scenario explicitly forbids both).
