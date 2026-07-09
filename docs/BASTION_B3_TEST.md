# B3 self-test results — colonist entity model & starting colony

Run: 2026-07-09, branch `bastion/block-B3` (`7f68814..1d7c07c` + gate), gate
per design doc §B3 Done-when + §4 anchor directive + standing invariants.
Result: **PASS**.

## Compiles

`cargo check`/`build -p veloren-voxygen -p bastion-harness -p veloren-server
-p veloren-rtsim -p veloren-common -p veloren-common-net` — green.

## Headless harness (Done-when a)

`cargo run -p bastion-harness -- --seed 1337 --ticks 500 --colony 6`:

- Summary: `rtsim_npc_count: 2361` = baseline (2355) + 6, `colonist_count: 6`,
  no panics across 500 ticks.
- Roster line: 6 colonists with randomized names/backstories/skills (e.g.
  "Gwil the Quiet"/disgraced guard/mining 3 construction 5 melee 4 …).
- Baseline run (no `--colony`): byte-identical shape to B0/B2a baselines,
  `colonist_count: 0` — zero impact on the vanilla sim.

## In-voxygen (Done-when b) — character presence (per §4 anchor directive)

- Entered with the character; overseer camera anchored to the invisible
  avatar (NOT spectator).
- Radial Ground context → **Found colony** → chat ack "Founding colony: 6
  settlers arriving."; server log: spawn + 6 named promote lines within 60ms.
- **Colonist names**: click-select shows the roster name — chat
  `Selected: Quill Brighteye — health 100%` (villager clicks show
  `Selected: entity N` — `Colonist` comp sync + `Stats::name` override both
  verified end-to-end).
- **Visibly distinct**: cyan overhead markers render above colonists (debug
  shape pipeline); villagers have none.
- **Box-select**: Inspect tool + drag → `Selected: 2 units` (chat + info
  line). Multi-select feeds the `BastionSelected` markers → cutaway targets.
- A second colony was founded live by the user — repeat founding works; 12
  colonists total.

## Loaded↔simulated boundary (Done-when c) — log-verified both ways

```
05:26:29 promoted ×6  (batch 1: Quill Brighteye, Lira of the Ford, …)
05:34:21 promoted ×6  (batch 2, second founding)
05:35:37 demoted  ×6  (batch 1 → SimulationMode::Simulated on chunk unload)
05:42:19 promoted     (batch 1 RE-promoted, same names — count=2 for
                       "Quill Brighteye" — after a FULL GAME RESTART)
```

The re-promote after restart additionally proves the colonist record
round-trips rtsim persistence (`serde(default)` field in data.dat) — B10
relevance noted.

## §4 anchor directive (inert + invulnerable god anchor)

- On anchor set (god mode), the server inserts `BastionGodAnchor` + a
  permanent vanilla `Invulnerability` buff (100% damage reduction); removed
  on F9/anchor clear. The buff icon is visible above the hotbar in-game
  (screenshots `b3-founded.png` et al.).
- No-aggro: the agent behavior tree already drops/never-pursues invulnerable
  targets (`is_invulnerable` checks at `behavior_tree/mod.rs:285,958`).
- **Residual (documented)**: a live hostile-aggro field test (spawned
  predator beside the anchor) was not run — the user was at the machine and
  closed the session; B8 (threats) exercises this hard. Greet/pushback
  filtering beyond the buff is best-effort per findings §6 and untested.

## Vanilla regression

Flagless boot to menu: alive and rendering at 15s, cleanly stopped, no
interference with the concurrently running flagged instance.

## Standing invariants

- No panics anywhere (harness 500 ticks; two live sessions).
- Entity counts: harness baseline+N exactly; demote path deletes loaded
  entities (vanilla delete on `Simulated`), re-promote restores — no leak
  observed across the restart.
- Perf: 60 fps at 4K with markers + selection active.

## Notes / watch items

- The user plays live during scripted QA; camera state in evidence
  screenshots drifts between steps.
- Colonists currently idle under vanilla civilised-agent AI (they wander,
  e.g. to the farm plot) — correct for B3; B4 gives them jobs.
- `readme/cross-genre-nice-to-haves.md` (architect input) rode along in
  commit `be1e9d9`.
