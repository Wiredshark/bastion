# Project Bastion — The Divine Politics Bible v0.1

**The design corpus for world politics: trade, diplomacy, and war — as one connected system.**
Companion to the main build report, the Agency Bible, and the DF Gap Ledger. This is Bastion's **original**
contribution — not ported from DF or RimWorld (where the player *is* the political actor), but built from
the premise those games don't have: **you are a god, one of several, and mortals play the board while gods
tilt it.**

**The thesis (one sentence):** a grand-strategy world of autonomous factions that trade, ally, and war for
mortal reasons — with a theological layer on top where you and rival deities contend for their souls, and
faith is the force that ultimately bends the geopolitics.

**Scope honesty:** this is **Tier-3 epic** — the hardest, most-entangled layer in the project, dependent on
mature faction/rtsim systems. Designed now to clarify the north star; **built late**, after a single colony
reliably survives on its own. Do not build from this before the colony core (B3–B7) and agency (B-AG*) are
proven.

**Obeys the project laws:** Pillar §1a (you influence, never command — *especially* here: you never order a
faction), the loaded↔simulated LOD law (world politics runs at the cheap rtsim tier; manifests concretely
only where loaded), and rtsim's "assume nothing, tend toward equilibrium" law (faction/faith state is a
tendency, never a guarantee).

---

## 1. The two-layer model

### Layer 1 — Grand Strategy (mortal, rational). Runs even with no gods.
Factions are autonomous rtsim actors (Veloren already tracks ~16) with **interests**:
- **Resources** they need/lack (food, ore, luxury goods, land).
- **Territory** they hold and covet; borders and chokepoints.
- **Relationships** with other factions (rival/neutral/ally), shaped by proximity, history, and grudges
  (builds on Veloren's sentiment system).
From interests emerge the three verbs, as facets of one engine:
- **Trade** — supply/demand between factions creates **caravans and trade routes**; wealth flows; shortages
  drive deals or raids.
- **Diplomacy** — interest + proximity + history → alliances, tributes, rivalries, non-aggression, betrayal.
- **War** — interest + grudge → conflict over resources/territory/revenge; armies muster, march, besiege
  (scales up B8's raid system to faction-vs-faction).
This layer is **realpolitik**: cold, interest-driven, and it would tick along in a godless world.

### Layer 2 — Theology (divine, passionate). Sits on top and modulates everything below.
Every faction has a **faith state**: devotion to **you**, to a **rival deity**, **divided**, or **heretical/
faithless**. Faith is not flavor — it is the **modulator** on Layer 1:
- Shared worship makes **alliance easy and war repugnant** (co-religionists).
- Divergent worship makes **rivalry bitter and trade grudging**; a faction that turns to a rival god becomes
  an enemy no treaty fully soothes.
- **Heresy** can split a faction from within (civil strife, schism, breakaway sects).
- Devotion level sets how strongly a faction **responds to divine acts** (a fervent faction rallies to an
  omen; a faithless one shrugs).

### The feedback loop (what makes it ONE system, not two)
- **Mortal events change faith:** a faction you save from famine/raid **converts** toward you; one you let
  burn **abandons** you; a rival god's miracle can **poach** your followers.
- **Faith changes mortal politics:** shared faith → alliance & trade; divine favor → battlefield victory →
  emboldened conquest; hardened faith lines → **holy war**.
Round and round. The gods never issue orders; they **inflame and cool** mortal politics through faith, and
mortal politics feeds back into who gets worshipped.

---

## 2. Competing gods — the engine of drama

You are **one of several deities**. Rival gods are AI actors doing to *their* followers what you do to
yours: blessing armies, converting the wavering, punishing apostates, answering prayers.
- **The contest is for followers.** Your **power scales with worship** — more faithful = more **favor** to
  spend (the B13 economy); losing a flagship faction to a rival is a real, weakening blow.
- **The world map becomes a religious contest map** layered over the political one. A war between two nations
  is also a **proxy war between two gods**. Converting a rival's core faction is a **spiritual coup**.
- **Peers, not props.** Rival gods give you antagonists *at your own scale* — not bandits to smite, but
  opposing wills contesting the same souls. They can counter your miracles, race you to convert a wavering
  people, or punish a faction for heeding you.
- **God personalities:** rival deities have dispositions (a war-god who rewards conquest; a trickster who
  sows heresy; a nature-god who blesses the untamed) so their play *feels* different and the contest has
  texture.

---

## 3. The three verbs, reinterpreted through divinity

You never *do* trade/diplomacy/war. You **tilt** them. Each verb below lists the mortal mechanic (Layer 1)
and the divine levers (Layer 2, via the B13 favor economy).

### 3.1 Trade
- **Mortal:** factions trade to cover shortages; caravans travel faith-warmed routes between allied/
  co-religionist sites; wealth accrues to well-connected factions. (Reuse Veloren merchants/economy.)
- **Divine levers:** bless a caravan (safe passage, profit), curse a rival's roads (banditry, loss), make
  the faithful **prosperous** (their goods coveted), sanctify a market, or send famine to force a rival to
  the table. Trade with *you*-worshippers flourishes; heretics find their caravans cursed.

### 3.2 Diplomacy
- **Mortal:** alliances/rivalries from interest + proximity + history; tributes, marriages (ties to B-AG6
  genealogy — royal bloodlines!), non-aggression, betrayal.
- **Divine levers:** **conversion is the master diplomatic verb** — bring a faction into your faith and it
  aligns with your other faithful. Send **omens** that push two factions together or apart; sanctify a
  marriage-alliance; declare a people **chosen** (elevating them) or **cursed** (isolating them). Broker
  peace with a shared miracle; poison a peace with a portent of betrayal.

### 3.3 War
- **Mortal:** wars ignite over resources/territory/grudge; armies muster at sites, march, besiege, take or
  lose ground; outcomes shift the map. (Scales B8 raids → faction armies.)
- **Divine levers:** you are **prayed to for victory** — answer with battlefield miracles (smite, storms,
  courage/terror — reuse Explosion/Lightning/WeatherZone/Buff, §2a), or **withhold favor** to doom the
  faithless. **Holy war** erupts when faith lines harden: your faithful crusade against a rival god's people.
  Victory won by *your* favor deepens devotion (feedback loop); a defeat despite prayer can breed doubt or
  heresy.

---

## 4. Your colony in the geopolitics
Your player-colony is a faction *in* this system, not outside it: it trades, is courted or threatened,
worships you natively (your home flock), and can be drawn into faction wars. The scale ladder:
**your colonists (full ECS) → your colony as a faction → the faction/faith world (rtsim).** Everything you
learned tending one colony now plays out at world scale, with you as its god.

---

## 5. LOD & the god's-eye interface
- **Simulation LOD:** all of Layer 1 + Layer 2 runs at the **cheap rtsim tier** (throttled ticks; abstract
  faction/faith/army records). It manifests **concretely only where loaded** — you see an actual caravan, an
  actual besieging army, actual worshippers at a shrine — but the *system* is abstract, per the boundary law.
- **The interfaces the god needs (later HUD work):**
  - A **religious contest map** (faith overlay on the world — who worships whom, conversion pressure).
  - A **diplomacy/relations view** (faction alliances, wars, trade routes).
  - A **pantheon panel** (you vs. rival gods: followers, favor, recent divine acts).
  - **Prayer feed** — the faithful petitioning you (answerable → converts/favor; ignored → doubt).
- **God-powers (B13) become the political toolset:** bless/curse/omen/convert/smite/sanctify are the verbs;
  favor (scaling with worship) is the currency; the contest with rival gods is the game.

---

## 6. Build blocks (all Tier-3, LATE — after colony core + agency are proven)
Designed to Done-when here so the Mega-Prompt can build them *when the queue reaches them*, not before.

- **DP1 — Faction interest & grand-strategy substrate:** factions gain interests (resources/territory/
  relations) and autonomously form trade/rivalry/alliance from them, at the rtsim tier. *Done-when:* over a
  soak, factions trade, ally, and feud plausibly with zero gods, no runaway, no extinction-to-zero.
- **DP2 — Faith system:** faction faith states (you / rival / divided / heretical), devotion levels,
  conversion/abandonment driven by events. *Done-when:* factions' faith shifts from mortal events (rescue →
  convert; abandonment → apostasy); heresy can split a faction; state persists LOD-safely.
- **DP3 — Faith modulates politics (the feedback loop):** faith warms/chills trade, alliance, and war.
  *Done-when:* co-religionists ally & trade preferentially; divergent faith breeds rivalry/holy war; the
  loop (mortal→faith→politics→mortal) is observable over a soak.
- **DP4 — Competing gods:** AI rival deities with personalities, favor economies, and divine acts contesting
  followers. *Done-when:* a rival god converts, blesses, and punishes autonomously; you can contest a faction
  with it; losing/gaining followers shifts each god's power.
- **DP5 — The three verbs as god-powers + interfaces:** wire bless-caravan / curse-roads / convert / omen /
  sanctify-marriage / battlefield-miracle to the B13 favor economy; build the contest map, diplomacy view,
  pantheon panel, prayer feed. *Done-when:* the god can tilt trade/diplomacy/war through favor-costed acts,
  see the state, and contest rivals — all indirect (no faction commands).

**Dependencies:** rtsim factions (exists) + B8 (raids→armies) + B13 (favor/powers) + B-AG6 (genealogy for
royal marriage-alliances) + B-AG3 (faith as a value/belief in the mind). Build order DP1→DP5.

---

## 7. Why this is Bastion and not DF/RimWorld/Civ
- In DF/RimWorld/Civ, **the player is the political actor** — you rule, you negotiate, you command armies.
- In Bastion, **the player is a god above the actors** — mortals rule/negotiate/fight; you contend with
  *other gods* for their faith, and faith bends their politics. You never command a faction; you convert,
  bless, curse, and answer prayers.
- The originality isn't a novel trade screen — it's the **inversion of the player's relationship to
  politics**, and the **theology-as-diplomacy** layer that inversion makes possible. That's the seam where
  Bastion stops being "voxel DF" and becomes its own game.

*End of Divine Politics Bible v0.1 — the capstone the whole world is quietly building toward.*
