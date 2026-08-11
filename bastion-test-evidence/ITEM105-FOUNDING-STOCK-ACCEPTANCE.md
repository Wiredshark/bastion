# #105 (FOUNDING SEED STOCK) — acceptance: the loop is self-sustaining

**Boot stamp:** commit `593bd66afe3a634ad1f1615189b5edc81f5cfb57`, branch `bastion/wip-batch-verify`.

**Status: ACCEPTED, on the second attempt.** The first attempt (commit
`ab51e57949`) shipped a real bug, caught live before being believed done.

## The bug the first acceptance run caught

`ab51e57949` hooked `Server::bastion_spawn_colony` (a Rust-level wrapper)
to spawn the founding stock. The LIVE `BastionSpawnColony` client message
never calls that wrapper — `server/src/sys/msg/in_game.rs:1279` calls
`rtsim.bastion_spawn_colony(pos, count)` directly from inside a system,
bypassing the Server method entirely. The fix only ever fired for callers
going through the Rust API (the harness); a real in-game founding got
nothing.

The first acceptance run (script-14, ~670s, same script as this one) came
back **0 sown / 0 matured / 0 harvested for the entire window** — not a
timing-margin issue, a real wiring bug. This is exactly the "gate must
test the live path" law: a green harness call would have hidden this
completely; the honest zero from testing the real client message is what
caught it. Fixed at `593bd66afe`: the founding drop now emits from the
same system, same call site as `rtsim.bastion_spawn_colony` itself, so the
two can no longer drift apart.

**Two producers by design, not a double-fire risk.** The Server-level
wrapper (`bastion_found_colony_seed_stock`, `ab51e57949`) and the
live-path emission at the `in_game.rs` call site above are disjoint by
construction: a founding reaches exactly one of the two. The live
`BastionSpawnColony` client message is handled entirely inside that
system and never calls `Server::bastion_spawn_colony`/`_seeded`; every
other caller of THOSE methods (the harness's ~60 scenario call sites,
`bastion_arena.rs`'s "fixture" staging spawn -- a live but non-client-
message admin path, any determinism-capture code) is a direct Rust call
that never routes through the client-message system. One founding call,
one path, one producer -- documented at both sites in the code so the
next reader who greps two emit sites for the same drop doesn't have to
re-derive it.

## The re-run: the loop is not just closed, it's self-sustaining

Same script (`script-14-founding-stock-acceptance.txt`), same verified
z=418 coordinates, fresh userdata dir, **zero `/give_item` this run** —
the founding stock (`FOUNDING_SEED_STOCK=8`) is the only seed source from
start to finish.

    checkpoint    elapsed   sown   crop MATURE   harvested
    2 (till done)   ~40s      8         -             -
    3               ~100s     8         8             8      <- first wave: 8/8/8, closed
    4               ~190s     24        8             8      <- second wave sown (16 more)
    5               ~310s     54        24            24     <- second wave maturing/harvesting
    6-7             ~430-550s 56        56            56     <- fully saturated, third-generation activity
    8 (run end)     ~670s     56        56            56     <- FINAL

**8 founding seeds -> 56 sown-and-harvested over the run, with every sown
crop reaching harvest (0 stalled).** The plot (30 cells) cycled through
multiple generations within the ~670s window: harvest yields seeds ->
hauled to the stockpile -> a fresh SOW wave claims them -> grows -> more
harvest. This is not merely "the deadlock doesn't recur" — it's a
demonstrably compounding, self-sustaining farm economy from a single
one-time founding grant.

## Item 4 rider (this run)

`ULTIMATE FAIL-SAFE`: **2** events. `GOTO-STAND-RESCUE` (sit witness, not
item 4's population): 239 events. Combined across all item 7/#105 runs to
date, item 4's population is now 3 (1 + 0 + 2) — still Opus's to score,
not re-scored here.

## Evidence

    bastion-test-evidence/live-playthrough/script-14-founding-stock-acceptance.txt
    bastion-test-evidence/live-playthrough/driver-founding105.log        (first attempt, bug caught: 0/0/0)
    bastion-test-evidence/live-playthrough/server-stdout-founding105.log (first attempt)
    bastion-test-evidence/live-playthrough/driver-founding105b.log       (second attempt, accepted: 56/56/56)
    bastion-test-evidence/live-playthrough/server-stdout-founding105b.log (second attempt)
