# ROW-COLONY-PRESENCE — ACCEPTANCE RESULTS: **PASS, all 4 measures**

**DECISIONS #106.** Packet: `bastion-test-evidence/ROW-COLONY-PRESENCE-PACKET.md`
(`583344e679`). Build: `7b00cce894` (row) + `fb9a740110` (acceptance diagnostic).
Boot stamp for the scored leg: commit `fb9a740110f60446722f3f3416f45aa6e2a68465`
(`git describe`: matches `git rev-parse HEAD` at build time, binary mtime
08:06/08:07 EDT, after every source edit).

## Boot config, read live

    hunger_decay_per_sec=0.000889  hunger_interrupt=0.2  hunger_comfort=0.5
    rest_decay_per_sec=0.000444    rest_interrupt=0.2    rest_comfort=0.5

## The scored leg (script-19, "v3")

Founded 8 colonists with `cmd give_item` + `cmd dropall true` (persistent food)
and a `designate bed` beforehand, then disconnected immediately — driver
connected 12:52:08–12:52:20 UTC (~12 seconds, all before founding settled),
zero reconnects for the remainder of the ~15-minute scored window.

**Two earlier attempts (v1, v2) preceded this leg and are part of the record,
not discarded:**
- **v1** (script-17): bare founding, no bed, no food. Measures 1/2/4 passed
  cleanly. Measure 3 never fired — **verified as inconclusive, not a pass**,
  once Opus asked whether that was "fired, found nothing" vs "never
  attempted": the disambiguating log line (`no_food_found`/`no_bed_found`,
  `bastion_jobs.rs` ~10907/~10986) is gated behind `BASTION_NEED_SKIP_DIAG`,
  which was not set. Correctly read as a script gap (no target existed to
  preempt into), not a code defect — but that specific claim was not
  independently confirmed from this leg's log, and is superseded by v3's
  clean positive result rather than relied on.
- **v2** (script-18): added a bed + `cmd dropall` (no `true`). Non-persistent
  drop despawns in ~5 sim-min; hunger's crossing at ~15 sim-min missed the
  window entirely. Self-diagnosed from `server/src/cmd.rs`'s
  `handle_drop_all` before relaunching.

## THE FOUR MEASURES — v3, unambiguous

| # | measure | PASS expression | result |
|---|---|---|---|
| 1 | colonists stay `Loaded` | `Loaded` at T+20s/2min/10min | **PASS** — 0 demotions across the entire ~15min run (`colonist demoted` count: 0); `COLONY-PRESENCE-ACCEPTANCE-DIAG` shows `loaded=true` on all 27,640 sampled lines |
| 2 | ★★ needs actually tick | same colonist's value MOVED between samples | **PASS** — uid 55 (one of the 8): `hunger 0.99985 → 0.769 → 0.191 → 0.0 → …cycles up to 0.233` after eating; not a frozen snapshot |
| 3 | preemption fires ≥1 | a real preempt event | **PASS, unambiguous** — 40 `"need preempt — hunger below interrupt"` lines (named colonist + target item + positions), `preempt_attempts` climbed 0→32 and held, **16 `"hunger restored"` completions** (colonists actually reached the food and ate, repeatedly — not just a preempt attempt) |
| 4 | zero client connections | none in the log | **PASS** — exactly one `Client connected!` / `Client disconnected!` pair, both at 12:52:08–12:52:20, before the scored window begins |

## Named failure modes, checked

- **Presence minted but terrain system ignores it (chunks stay unloaded,
  colonists flip anyway):** did not occur — 0 demotions.
- **Colonists load but needs stay frozen (the silent pass):** did not occur —
  measure 2's own value trajectory is the disproof.
- **Presence leaks after disband:** not exercised this leg (no disband path
  triggered); left as a follow-up for whenever disband is built.
- **Cost blowup:** view distance = 1 chunk per colony, as designed; not
  independently re-measured here (no second colony in this leg to compare
  against) — flagged, per the packet's own debt note, for item 40.

## Capture protocol

Server killed, log verified stable across two `wc -c` reads (8223805 →
8242867 → final 8252395 bytes after kill) before this doc was written.

## Evidence

    bastion-test-evidence/live-playthrough/script-17-colony-presence-acceptance.txt   (v1)
    bastion-test-evidence/live-playthrough/script-18-colony-presence-acceptance-v2.txt (v2)
    bastion-test-evidence/live-playthrough/script-19-colony-presence-acceptance-v3.txt (v3, scored)
    bastion-test-evidence/live-playthrough/server-stdout-colony-presence-acceptance.log    (v1)
    bastion-test-evidence/live-playthrough/server-stdout-colony-presence-acceptance-v2.log (v2)
    bastion-test-evidence/live-playthrough/server-stdout-colony-presence-acceptance-v3.log (v3, scored -- 8.25MB)
    bastion-test-evidence/live-playthrough/driver-colony-presence-acceptance-v3.log

## Verdict

**ROW-COLONY-PRESENCE PASSES its own acceptance bar.** The defect the
endurance run surfaced (founding colonists demote to `Simulated` and
need-preemption goes permanently inert, invisible only because a client was
always present) is fixed: a founded colony now ticks its own needs, feeds
itself, and stays `Loaded` with zero client connected. **Item 8's endurance
run is unblocked and can be relaunched against this pin.**

## ADDENDUM — the "40 vs 32" gap in measure 3, read (Fable's ask, parallel-fill
during item 8 v3): not a double-count and not a miss, two loosely-named
populations that were never the same set

**Re-grepped `server-stdout-colony-presence-acceptance-v3.log` directly rather
than trusting the summary numbers above.** The "40" cited in measure 3 is
itself two different messages counted together: 24 `"need preempt — hunger
below interrupt"` lines (`bastion_jobs.rs` ~10917-10927, a fresh preempt —
this site increments `board.preempt_attempts` in the same block, no
divergence possible there) **plus** 16 `"need preempt — reclaiming suspended
EatFrom"` lines (~10811, a *different* code path — resuming an already-
pending job, not a fresh preempt — which does **not** touch
`preempt_attempts` at all). Separately, `preempt_attempts` itself (0→32) is
the union of four increment sites, not one: the 24 hunger-preempts above,
0 rest-preempts (`"rest below interrupt"`, ~11062 — none fired this leg), 0
breakdown-preempts (`"BREAKDOWN — despondent"`, ~10535 — none fired), **plus
8 struck-out anti-wedge cooldown arms (~11079-11084) that increment the
counter with no named log line at all** — a colonist that exhausted every
candidate (no food, no bed) and armed its cooldown so it doesn't re-strike
the same dead end every tick. **The two numbers therefore share an overlap
of exactly 24 (the fresh hunger-preempts) and diverge on both sides**: "40"
over-counts relative to `preempt_attempts` by including 16 reclaim events
the counter was never designed to see; "32" over-counts relative to the
named-line total by including 8 silent struck-out arms neither log line
names. **Not a bug** — nothing is double-counted (each of the four
increment sites fires from a disjoint code branch) and nothing that should
increment the counter fails to — but `preempt_attempts` is not, and was
never meant to be, "count of `need preempt` log lines"; the label invites
that reading and the doc above (measure 3's row) states both numbers side
by side without saying they measure different sets. Left as a naming note,
not a fix: the counter's own doc comment (`bastion_jobs.rs` ~4841-4843)
already scopes it as "preempt attempts fired (telemetry)" without claiming
completeness over every preemption-adjacent event, and re-scoping it to
include reclaims or exclude struck-outs is a design call for whoever next
needs it to answer a specific question, not a correctness defect to patch
here.

**★ The 8 silent struck-out arms are a mute channel, worth naming as its
own hazard (Fable's addition):** every other increment site pairs its
`preempt_attempts` bump with a named log line in the same block, so the
counter's movement is auditable from the log alone for those three
sources — but the struck-out arm increments with nothing printed. A
future reader who diffs `preempt_attempts` against grepped log lines and
finds a residual has no way to attribute it to *this* source without
already knowing this addendum exists; the log undercounts this producer
by construction, not by omission of a case anyone forgot to add. Anyone
next touching this counter or building a reconciliation check on it
should know that gap going in.
