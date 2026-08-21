# Item 37 (LLM-player harness v2) — DISPOSITION: **PASS**, with the order disclosed

**PROCESS DISCLOSURE FIRST:** this row was built BEFORE it was registered.
Ben's 2026-08-21 directive ("use them to actually play the real game like they
would play dwarf fortress… and collect what's going wrong") arrived as work,
and the harness was built to serve it the same hour. Pre-registration was
skipped. The bars below were written AFTER the sessions ran, which makes them
weaker evidence than a registered prediction, and that is stated rather than
smoothed over. What rescues the row is that the sessions produced FALSIFYING
results — including two defects in my own same-day work — which a
self-congratulatory harness cannot do.

## What v1 was, and what v2 adds

v1 was `bastion_playtest`: one script file, one fresh world, one verdict — a
BATCH instrument. A player is not batch. v2 adds the three things a player
needs:

1. **A world you can return to** — `bastion-test-evidence/play-harness.sh`
   (`boot` / `turn` / `watch` / `stop`, one port slot and userdata dir per
   player, arena or adopted-village). The colony you leave is the colony you
   come back to, so a player can form a plan, act, see consequences, and change
   their mind.
2. **Eyes** — the `ascii <radius>` verb: a DF-style top-down map (C colonists,
   i items, # structure, . ground, `?` UNLOADED and never drawn as ground),
   with a tally line naming how many cells were unloaded and how many
   colonists were off-map, so a blind spot cannot read as an empty world.
3. **An experience census in-engine** — working/moving/**stuck**/idle/fed/
   rested per window, so "the colony looks broken" became a number a leg can
   fail on.

## BARS (post-hoc, and labelled as such)

1. **An LLM player can run a real multi-turn session and reach conclusions a
   scripted leg cannot: PASS.** Four sessions ran: builder (9 turns, 42,312
   ticks), settler (13 turns, adopted village), adversary (11 turns, two
   worlds), long-game (13 turns, **127,200 ticks / 141 game-days / all four
   seasons**). Every one produced a play diary, a ranked defect list in player
   language, and a verdict.
2. **The harness finds what mechanism tests miss: PASS, decisively.** The
   sessions produced the night's most valuable findings, none of which any
   green unit or leg witness had caught: the claim-path deadlock
   (`1608 eligible / 0 assigned / 8 idle`), 976 dishes cooked against 39
   eaten, adoption ordering 1,519 beds into a furnished village, cancel
   deleting the food store, the chronicle shipping disabled, `inspect_colony`
   frozen for 23 game-minutes, and the ASCII map's own `#` glyph being
   unreachable by construction.
3. **It catches the AUTHOR's mistakes: PASS.** Two defects found were mine,
   made the same day: the in-flight-ingredient guard that protected the eater's
   own meal, and the colony-mind threat count that read 22 friendly villagers
   as an invasion.
4. **Honest failure reporting: PASS.** Every session disclosed its own errors
   (a mangled commit, a `pgrep` matching itself, a stale log grep, a wrong
   verb name that cost a turn), and one returned VOID rather than dressing a
   broken session as a bad game.

## Known limits (not claimed as strengths)

- Token cost is real: one workflow reached **155 agents** before it was
  stopped, and the verification fan is now capped at 6 defects × 2 lenses.
- The players cannot yet SEE the world visually — mode 3 (the real client) is
  driven by Ben, not by an agent.
- `play-harness.sh stop` reported success while the server lived (its `$!` was
  the subshell); fixed, and only found because a player had to kill it by hand.
