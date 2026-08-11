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
