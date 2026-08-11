# BASTION — the game, when the board is complete

**Written 2026-08-11 at Ben's request: "once this is all complete, give me a detailed feature
list of how the game will be, point by point." Compiled from the 60-item board
(BUILD-ROADMAP.md), the DECISIONS ledger, and the design driver. This is the north star;
the roadmap is the route.**

## 1 · The world
- A full open world (the Veloren engine's terrain, biomes, weather, day/night, seasons) with
  living ambient civilization — villages, guards, merchants, travelers — that existed before
  your colony and continues around it.
- **The world lives unwatched**: your colony exists, works, eats, sleeps, and grows whether or
  not anyone — including you — is looking at it. (Proven by the presence system; certified by
  the unattended endurance gate.)
- Deterministic by construction: the same founding seed produces the same world and the same
  history — replayable, testable, fair.

## 2 · Founding
- You found a colony as its god: choose the site, and the colony begins with a founding kit
  (seeds, basic food, tools) — viable by construction, no babysitting required.
- From founding onward the colony is a first-class entity in the world: it holds its ground
  loaded, owns its stockpiles, and accrues its own history.

## 3 · The colonists — the ants
- Each colonist is an individual: named, persistent, with personality traits that visibly
  drive behavior (who's brave, who's lazy, who works nights), skills that grow with use and
  visibly matter, and personal needs — hunger, rest, recreation — on day-aligned rhythms.
- They run their own lives: choose work from the colony's job board, eat from shared piles
  and stockpiles (portion-aware, no hoarding), sleep in beds they walk to and arrive at,
  and fall through to their next-best need when the first can't be met — a colonist who
  can't eat still sleeps instead of breaking down.
- They fail like people: stall, sit, get stuck, despair — and recover: a layered rescue
  economy (self-rescue, escape planning, queue-fair ladders, last-resort fail-safes) keeps
  individual failure from ever becoming colony failure.
- Relationships form from shared work, meals, and rescues; moods swing with events —
  celebrations after wins, funks after losses, breakdowns under sustained deprivation.

## 4 · The colony — the superorganism
- The colony itself thinks, on the ant model made deliberate: no commander orders individuals;
  colonists coordinate through the shared job board (stigmergy), and above it the **Colony
  Mind** — a colony-scale drive arbiter (launching with Sustain and Grow; Defend and Expand
  as their senses arrive) — reads colony vitals (food-days remaining, housing per colonist,
  threat pressure, buffers) and re-weights the work generators. The ants stay ants; the
  pheromone gradients gain a brain.
- Colony metabolism: deficits create work automatically — unfilled construction pulls mining,
  hunger-pressure pulls farming, threats pull fortification.

## 5 · Survival — a closed, compounding loop
- Farm-to-table self-sufficiency: till → sow → grow → harvest → re-seed, compounding from
  the founding seeds (demonstrated: 8 seeds → 56 crops in three generations, no help).
- Cooking turns harvests into meals worth more than raw food; stockpile zones organize
  food, materials, and goods; colony food is colony property — protected from the world's
  sticky fingers.
- The colony passes the endurance standard: multiple sim-days of unattended life with food
  stocks trending level-or-up — no slow leaks.

## 6 · Work & economy
- Full job economy: mining, chopping, hauling with priorities, building, farming, crafting
  chains (raw → workshop → goods), tool quality and wear (better tools, faster work, and
  they break).
- Reservation and claim systems that never double-spend and never let one stuck colonist
  starve the rest (measured, audited, conserved by tested invariants).
- Trade with the outside world: caravans to ambient villages, buying and selling against
  the world's own economy.

## 7 · Settlement — colonies that look like places
- Colonies grow like real settlements, not junk piles: roads first, buildings claim
  frontage, districts emerge from adjacency (workshops near stockpiles, homes away from
  noise) because sensible layout is literally cheaper for the ants to live in.
- You paint intent — zones and districts — and the colony fills in the detail with
  coherent, palette-consistent architecture; desire-lines harden into roads; layout quality
  is measured (commute, connectivity) so sprawl-into-spaghetti fails a gate before it
  offends the eye.

## 8 · Threats — something to survive
- Hostiles pressure the colony; raids scale with your wealth; **thievery arrives as a
  designed feature** (the gate is built; lifting it is line one) — with intent, counters,
  and consequences, not as a bug.
- Defense becomes a way of life: guard duty as a job, fortification designations, walls
  that matter, and the Flee drive tested by real danger.
- Injury and medicine: wounded colonists, a medic job, rest-to-heal — and death that
  matters: graves, mourning, and a colony that remembers.

## 9 · You — the god
- Your interface is the Colony Mind, not the ants: powers that inspire Growth, harden
  Defense, steady a shaken colony — modulating drives rather than micromanaging colonists.
- Faith as your resource: colonists' values generate favor; your miracles spend it —
  heal an injury, conjure a feast, steel a coward at the wall.
- The god-hand for direct intervention when you choose it: bless, smite, place, provision.
- The overseer camera, minimap, time controls (pause/1×/2×/4×) — the world at your pace.

## 10 · Stories & memory — the world remembers
- **Everything notable carries its history**: items log who made, carried, stole, and used
  them ("forged by Awen, stolen by Voonoo, recovered at the mushroom incident"); colonists
  carry biographies; the retained set persists across saves forever.
- The Chronicle is the colony's own saga — deaths, thefts, rescues, firsts — browsable
  in-game, built as the player-facing view of the same event stream that powers debugging.
- Emergence is content: the world's ambient characters collide with your colony into
  unscripted stories (a merchant named Voonoo once stole an entire harvest in one grab —
  found, named, and legislated into a future feature).

## 11 · Legibility — seeing your colony
- Click any colonist: needs, mood, current job, what they're stuck on, their story.
- The colony dashboard: hunger/rest/mood/jobs/food-days at a glance; the Colony Mind's
  current drives visible — you can see what your colony is worried about.
- Recreation and idle life visible in the world: colonists who relax, socialize, and live
  between tasks instead of standing frozen.

## 12 · Persistence & scale
- Save/load with nothing lost: colony state, histories, skills, the retained event set —
  a colony survives a restart with its soul intact.
- Colonies of 16–32 running stably; the architecture (colony presence now, needs-in-rtsim
  at the horizon) scales toward multiple colonies in one living world.

## 13 · Under the hood — the promises the player never sees but always feels
- Every feature certified under the conditions its promise names — including the flagship:
  unattended life, tested with nobody watching.
- Deterministic, bit-reproducible simulation (proven across machines and datacenters);
  every claim in development measured, every zero proven able to speak.
- The endgame pipeline: LLM players stress-test the colony as players; their sessions
  score the experience; reinforcement learning then tunes the deterministic Colony Mind —
  the game that playtests itself and learns.

**One sentence: Bastion is an ant-farm god-game inside a living open world — a colony of
genuine individuals who run their own lives, a superorganism that thinks in drives you can
touch, settlements that look like places, a world that remembers every story it generates,
and all of it alive whether or not anyone is watching.**
