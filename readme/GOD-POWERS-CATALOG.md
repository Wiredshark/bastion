# Project Bastion — God-Powers / Divine-Acts Catalog v0.1

**The enumerated verb-set of divinity: what a god can actually DO, costed and build-ordered.**
Feeds **B13** (divine-influence layer) and the **faith layer** (divine-politics-bible / DP1–DP5). Companion to
the main build report (B13, §1a), the divine-politics bible, the agency bible (minds/mood the powers act on),
and §3y (the weather/nature substrate powers ride).

**Isolation note:** NEW design doc; appends to the corpus, rewrites nothing. Where it names costs/tiers not in
B13's build-report sketch, treat those as this catalog's proposed model — B13's palette (the reuse-verified list)
is the ground truth for *what exists*; this doc organizes it into a *verb menu* and extends it with the divine-
politics levers and the faith layer.

---

## 0. The three laws every verb obeys (Pillar §1a)

1. **Act on the world/conditions, never command a unit.** Every power targets **land, resource, weather, or a
   colonist's *condition*** — never "unit, do X." The colony *responds*; it is not ordered. (The only direct
   control is Embody/B12.) A power that reads as a unit order violates the pillar and does not ship.
2. **Costs favor; the god has limits.** Powers spend **favor/faith** (B13's economy, name TBD), which accrues
   from a thriving/worshipping colony and regenerates over time. Zero favor = powers unavailable, colony still
   runs fully (Tier-1b soak must pass with no god input, ever).
3. **Ripples through the autonomous sim.** A power produces a *situation the colony reacts to*, and its
   consequences propagate (a diverted river changes downstream farms; a blessed harvest shifts the economy; a
   forced action leaves a resentful thought, agency-bible §5b.4). Powers are *causes*, not *scripted outcomes*.

---

## 1. The control-spectrum model of a power (miracle / blessing / passive)

Every power sits at one of three **control-spectrum tiers** (§3q applied to divinity) — the same Autonomous /
Manage / Command axis the rest of the game uses, here as *how much the god reaches in*:

| Tier | Analogy | Nature | Favor shape | Legibility default |
|---|---|---|---|---|
| **① Miracle (cast)** | RTS-command / direct | One-shot, targeted, immediate. The god reaches in and *acts now*. | High, spent up-front | Loud — usually attributed to a god |
| **② Blessing (set)** | DF-manage / policy | A standing enchantment on a target that persists and ticks. Set-and-forget. | Medium up-front + optional upkeep drip | Medium — felt over time, attributed on reflection |
| **③ Passive (ambient)** | Autonomous / influence | Always-on tendency shaping from your mere divinity/worship. No targeting. | Free or a background drift | Quiet — read as fortune/nature, rarely pinned on you |

This is the god-side of the control spectrum: **miracles are the "command" mode of godhood** (spend big, act
directly, but only occasionally — the disciplined heavy lever), **blessings are the "manage" mode** (set standing
divine policy over a site/people), **passives are the "autonomous" mode** (your godhood ambiently tilts the world
even when you do nothing — the divine analog of the self-running colony). Autonomous-by-default holds even here:
a god who never casts still *matters* through passives and the faith they accrue.

### 1.1 Favor cost model (relative — B13 sets the real units)
A notional 5-step scale, since favor units are B13-defined:
**Free** (passive) · **Low** · **Med** · **High** · **Epic**. Cost scales with **area × duration × permanence ×
intervention-in-mortal-agency**, and is **discounted by devotion at the target** (blessing your fervent faithful
is cheap; smiting near heretics who'll credit a rival is dear — divine-politics feedback). Permanent world-edits
(terrain) and forcing a mortal's will (curse a person) cost most; ambient nudges cost least.

### 1.2 Legibility model (how the colony perceives the act) — the attribution axis
From the divine-politics-bible ("a rival god's storm is indistinguishable from nature until the chronicle
attributes it") and the legibility principle. Three attribution levels, orthogonal to tier:
- **Attributed** — the colony *knows* it was divine (an omen, a smite from a clear sky, a voice). Builds/spends
  faith directly; the faithful rally, the faithless doubt. Miracles trend here.
- **Ambiguous** — felt as fortune or nature (good weather, a lucky vein, a calm mood). Builds faith slowly and
  deniably; a rival god's act is indistinguishable from yours until the **chronicle** attributes it. Blessings/
  nature-nudges trend here.
- **Invisible** — never consciously perceived (a passive focus tilt). Shapes behavior without perception.
Legibility is a **design lever, not just flavor**: an *attributed* miracle is a faith transaction (rally or
doubt); an *ambiguous* one is safe but slow-converting. The god chooses loudness.

---

## 2. The verb catalog (grouped by aspect)

Columns: **Verb** · **Effect** · **Tier** (①miracle/②blessing/③passive) · **Target** · **Favor** · **Rides
(existing system)** · **Legibility** · **Status** (🟢 cheap-now = rides existing / 🟡 needs targeting+economy
polish only / 🔴 needs a new system).

Systems referenced (all confirmed to exist unless marked *new*): `MakeBlock`/`MakeVolume`/terrain-edit path (B5),
`Spawn`/`GiveItem`/`MakeSprite`, `Explosion`/`Lightning`, `WeatherGrid`/`WeatherZone` (§3y), `Buff` (blessings/
curses), `Time`/`TimeScale`, the **Hazard-Events engine** (location+radius+effect+NPC-reaction — backlog §1a;
some of it new), rtsim faction/faith records (*new: DP2*), the **Mind/mood/focus** model (*new: B-AG3*), the
**fluid-flow solver** (*new: the one real B13 build*), a **quest system** (*new: §3h*).

### 2.1 CREATION (land, resource, growth, sanctification — building the world up)

| Verb | Effect | Tier | Target | Favor | Rides | Legibility | Status |
|---|---|---|---|---|---|---|---|
| **Raise / lower land** | Sculpt terrain up/down — build pad, hill, dam | ① | Land | High | `MakeBlock`/`MakeVolume` terrain edit (B5) | Attributed | 🟢 |
| **Carve channel** | Cut a trench/canal; routes water when fluid lands | ① | Land | High | terrain edit; fluid *new* for flow | Attributed | 🟢 edit / 🔴 flow |
| **Material flow** (From Dust) | Trigger lava/water/sand to *flow* and settle | ① | Land | Epic | **fluid-flow solver *(new — the one real build)*** | Attributed | 🔴 |
| **Surface an ore vein** | Reveal/seed an ore deposit the colony then mines | ① | Land | Med | `MakeSprite`/`MakeBlock` + resource tracking | Ambiguous | 🟢 |
| **Bless a field (fertility)** | Standing fertility buff — crops grow faster/richer | ② | Land tile | Med + drip | `Buff`-analog on tile / sprite growth (§3y crops) | Ambiguous | 🟡 |
| **Spawn game** | Make food-animals appear for hunters | ① | Land | Med | `Spawn` (temperature-aware, §3y) | Ambiguous | 🟢 |
| **Grow / seed forest** | Accelerate flora growth / seed trees (§3y flora) | ② | Land region | Med | flora growth-stage ticks *(new, §3y)* | Ambiguous | 🔴 |
| **Sanctify ground / found shrine** | Mark a site holy — a dominion anchor; boosts nearby devotion/favor-regen | ② | Site | High | rtsim site record + `MakeBlock` (structure) + faith *(new DP2)* | Attributed | 🟡→🔴 |
| **Bless a harvest** | One-shot: this harvest is bountiful | ① | Colony/field | Med | resource/item emit | Ambiguous | 🟢 |

### 2.2 WRATH (destruction, calamity, punishment — tearing down)

| Verb | Effect | Tier | Target | Favor | Rides | Legibility | Status |
|---|---|---|---|---|---|---|---|
| **Smite** | Lightning-strike a target (raiders, a spot) | ① | Enemy/land | Med | **`Lightning`** (implemented) | Attributed | 🟢 |
| **Meteor / blast** | Explosion at a point — calamity/clearing | ① | Enemy/land | High | **`Explosion`** (implemented) | Attributed | 🟢 |
| **Panic an enemy party** | Flip a hostile band's AI to flee | ① | Enemy | Med | agent AI flee-flip (B8) | Attributed | 🟡 |
| **Earthquake / fissure** | Rip terrain, radius damage + NPC reaction | ① | Land/enemy | High | **Hazard-Events engine** (location+radius+effect) *(partly new)* | Attributed | 🔴 |
| **Flood** | Flash-flood a region (hazard) | ① | Land/enemy | High | Hazard engine + fluid *(new)* | Attributed/Ambiguous | 🔴 |
| **Wildfire** | Ignite a region; spreads, dampened by rain (§3y) | ① | Land/enemy | Med | Hazard engine (fire) *(partly new)* | Ambiguous | 🔴 |
| **Blight a field** | Standing curse — crops wither, soil sours | ② | Land tile | Med + drip | `Buff`-analog / sprite decay | Ambiguous | 🟡 |
| **Curse a colonist** | Affliction: weakness, ill-fortune, dark thoughts | ② | Colonist | Med | **`Buff`** + mind thought (§5b) | Attributed | 🟢 buff / 🟡 thought |
| **Drought** | Withhold rain from a region (§3y) | ② | Weather region | Med + drip | **`WeatherGrid`** write | Ambiguous | 🟢 |
| **Curse the roads** | A rival's caravans/trade routes beset (banditry, loss) | ② | Faction routes | High | rtsim trade/route records *(new DP)* | Ambiguous | 🔴 |
| **Curse a people** | Isolate/afflict a whole faction (divine-politics) | ② | Faction | Epic | rtsim faction + faith *(new DP2)* | Attributed | 🔴 |

### 2.3 SUCCOR (protection, aid, blessing the faithful — holding up)

| Verb | Effect | Tier | Target | Favor | Rides | Legibility | Status |
|---|---|---|---|---|---|---|---|
| **Bless a colonist** (vigor/courage/inspiration) | Buff → faster work / holds instead of flees / better mood | ② | Colonist | Low | **`Buff`** (implemented) + mood/focus (§5b) | Ambiguous | 🟢 |
| **Heal / mend** | Restore health to the hurt | ① | Colonist | Low | `Buff`/health op | Attributed | 🟢 |
| **Soothe / calm** | Lower stress; pull a breaking mind back (agency §5b) | ① | Colonist | Low | mind/mood write *(new B-AG3)* | Ambiguous | 🔴 |
| **Ward / shield a site** | Standing protection — dampen incoming harm/hazard | ② | Site | Med + drip | `Buff` region / hazard-resist | Attributed | 🟡 |
| **Raise a defensive wall** | Instantly wall a threatened spot | ① | Land | Med | **`MakeVolume`** (B5 path) | Attributed | 🟢 |
| **Embolden an army** (battlefield miracle) | Courage/terror at a battle — tilt a fight (divine-politics war) | ① | Colonist group / enemy | High | `Buff` (courage) + panic (terror) | Attributed | 🟡 |
| **Answer a prayer** | Respond to a petitioning colonist's need → convert/deepen faith | ① | Colonist | varies | prayer feed *(new DP)* + the granted effect | Attributed | 🔴 |
| **Bless a caravan** | Safe passage + profit for a trade run (divine-politics trade) | ② | Faction caravan | Med | rtsim trade record *(new DP)* | Ambiguous | 🔴 |
| **Grant insight / knowledge** | Accelerate a people's advancement (§3f tech) | ② | Colony/faction | High | tech/knowledge model *(new §3f)* | Ambiguous | 🔴 |

### 2.4 NATURE (weather, season, ecology — the ambient world)

| Verb | Effect | Tier | Target | Favor | Rides | Legibility | Status |
|---|---|---|---|---|---|---|---|
| **Call rain** | Rain onto a drought — waters crops, dampens fire (§3y) | ① | Weather region | Low | **`WeatherGrid`**/`WeatherZone` write | Ambiguous | 🟢 |
| **Clear skies** | Disperse a storm | ① | Weather region | Low | `WeatherGrid` write | Ambiguous | 🟢 |
| **Summon storm** | Bring a storm (rain/wind/lightning) — succor or wrath | ① | Weather region | Med | `WeatherGrid` + `Lightning` | Ambiguous | 🟢 |
| **Early frost / thaw** | Nudge the season locally (§3y freezing) | ① | Weather region | Med | temp field + `WeatherGrid` *(partly new)* | Ambiguous | 🟡 |
| **Fog / clear** | Raise or lift fog (visibility → raid tactics, §3y) | ① | Weather region | Low | `WeatherGrid` write | Ambiguous | 🟢 |
| **Still / rouse the winds** | Calm or raise wind (ranged combat, sails, §3y) | ② | Weather region | Low | `WeatherGrid` write | Ambiguous | 🟢 |
| **Bless a herd** | Wildlife thrive/breed (ecology, §3y + agency §5c) | ② | Wildlife region | Med | rtsim wildlife population *(new §3y)* | Ambiguous | 🔴 |
| **Calm wild beasts** | Soothe predators/monsters — de-aggress a region | ① | Wildlife | Med | agent AI aggression write | Ambiguous | 🟡 |
| **Guide migration** | Turn a herd's/monster's movement (§3y migration) | ② | Wildlife | Med | rtsim movement *(new §3y)* | Ambiguous | 🔴 |
| **Hasten / slow time** | Speed or slow the world (pacing) | ① | World | — | **`Time`/`TimeScale`** (implemented) | Invisible | 🟢 (pacing, not favor) |

### 2.5 Cross-aspect — DOMINION & FAITH (the Divine-Politics verbs — Tier-3, LATE)

These are the **social/theological verbs** the divine-politics-bible names (DP5). They don't fit the four
elemental aspects because their *target is faith itself*. All are 🔴 (need the faith system, DP1–DP4) and belong
to the LATE world-tier, but they're catalogued here so B13's menu and the faith layer share one list.

| Verb | Effect | Tier | Target | Favor | Rides | Legibility | Status |
|---|---|---|---|---|---|---|---|
| **Convert** *(the master verb)* | Bring a faction into your faith (divine-politics §3.2) | ② | Faction | Epic | faith system *(new DP2)* | Attributed | 🔴 |
| **Send an omen** | Push two factions together/apart | ① | Faction pair | High | faith + diplomacy *(new DP)* | Attributed | 🔴 |
| **Declare chosen / cursed** | Elevate or isolate a whole people | ② | Faction | Epic | faith + rtsim faction *(new DP2)* | Attributed | 🔴 |
| **Sanctify a marriage-alliance** | Bless a royal union (ties B-AG6 genealogy) | ① | Faction pair | High | genealogy *(new B-AG6)* + faith | Attributed | 🔴 |
| **Incite holy war** | Harden faith lines → your faithful crusade | ② | Factions | Epic | faith modulates war *(new DP3)* | Attributed | 🔴 |
| **Divine quest-giving** | Charge a hero with a task (§3h Mode A) | ① | Colonist | High | quest system *(new §3h)* | Attributed | 🔴 |

### 2.6 The passive tier (③ — always-on, free, aspect-spanning)
Passives have no per-cast entry because they don't get *cast* — they're the ambient tilt of your godhood, tuned
by worship. They're the divine analog of the autonomous colony (something always happening without input):
- **Favor accrual** — a thriving/worshipping colony drips favor to you (B13 core; the supply side of every verb).
- **Devout focus/mood tilt** — the faithful, knowing their god watches, run at slightly higher focus/mood
  (agency §5b focus system); the faithless don't. Invisible legibility; free.
- **Dominion ambience** — within sanctified ground (§2.1), your blessings are cheaper and rival acts costlier
  (the faith-discount of §1.1 made spatial). Rides the "my people's land" spatial object (§3q/dominion).
- **Presence deters** — near a shrine/holy site, minor threats are marginally less likely to press (ambient,
  not a shield). Rides threat-cadence weighting.

---

## 3. The build-ordered menu (what B13 builds, in order)

B13's own guidance: **build the framework + 3–4 powers first, then expand.** Ordered by "rides existing systems"
→ "needs polish only" → "needs a new system." A god game exists the moment the 🟢 tier ships.

### Wave 0 — The framework (B13 core, genuinely new, build first)
The **favor economy** (accrual from worship/thriving, regen, spend, zero-favor-safe), the **overseer targeting
UI** (pick land/colonist/region/enemy), and the **`ApplyInfluence` routing** (B2's reserved channel → spend
favor → apply effect → HUD cost/cooldown). Nothing below works without this. *This is the design core of B13.*

### Wave 1 — 🟢 Cheap-now (ride implemented systems — the first playable god)
Pick the **3–4** B13 build-report names, one per aspect so the god feels whole immediately:
1. **Smite** (Wrath, `Lightning`) — the iconic act; proves calamity + targeting.
2. **Bless a colonist** (Succor, `Buff`) — proves a power *measurably changes autonomous behavior* (courage buff
   → holds instead of flees, B13 Done-when).
3. **Call rain / clear skies** (Nature, `WeatherGrid`) — proves world-condition powers + the weather substrate.
4. **Surface ore / spawn game** (Creation, `Spawn`/`MakeSprite`) — proves *resource-seed → autonomous harvest*
   (the B13 Done-when that ties god-power back into autonomy).
Plus free: **raise/lower land** & **raise a wall** (`MakeVolume`, B5 path) round out Creation/Succor cheaply.

### Wave 2 — 🟡 Polish-only (exist, need targeting/economy/mind wiring)
Panic-an-enemy, curse-a-colonist (buff + thought), embolden-an-army, ward-a-site, bless/blight-a-field, early
frost, calm-wild-beasts. All ride existing ops but need a bit more than a straight call (a thought write, a
region buff, a hazard-resist). Second expansion pass.

### Wave 3 — 🔴 New-system (build the system first, then the power slots in)
Ordered by which new system unblocks the most verbs:
1. **Fluid-flow solver** (the one real B13 build) → material-flow, flood, canal-routing. *Prove standalone first
   (you have a From Dust reference), then port* (B13 build-report).
2. **Hazard-Events engine** (backlog §1a — one radius-effect + NPC-reaction engine) → earthquake, wildfire,
   flood-as-hazard. Build once, many wrath verbs plug in.
3. **Mind/mood write access** (B-AG3) → soothe/calm, curse-thought, the *resentful-thought-from-a-forced-act*
   loop (agency §5b.4).
4. **§3y nature upgrades** (wildlife population, flora growth, migration) → bless-herd, grow-forest,
   guide-migration.
5. **Faith system (DP1–DP4)** → the entire §2.5 Dominion & Faith cluster (convert, omen, chosen/cursed, holy
   war). The LATE capstone; build only after the colony core + agency are proven (divine-politics-bible §6).
6. **Quest system (§3h)** → divine quest-giving (also gates god-embodied Mode A).

---

## 4. Guardrails & cross-cutting notes

- **Every power routes through the same authoritative edit/spawn paths as B5/B6** and respects conservation
  invariants — misused powers must not dupe items, strand colonists (carve ground from under them), or break
  pathing/persistence (build-report §7-point-14, "god-power griefing the sim"). Fuzz-test in Tier-2.
- **Attribution is a mechanic, not flavor** (§1.2). Wiring the faith layer, an *attributed* act is a faith
  transaction (rally the faithful / doubt on failure); an *ambiguous* one converts slowly and deniably. Rival
  gods use the identical catalog — a rival's storm is indistinguishable from yours until the chronicle names it
  (divine-politics-bible). Build the catalog **symmetric**: whatever verbs you have, rival gods have.
- **Forcing a mortal's will is the most expensive and most legible thing a god does.** Curses, forced actions,
  and compulsions cost most favor *and* leave the target a resentful thought (agency §5b.4) — the god game's
  built-in cost for heavy-handedness. Cheap, gentle, ambiguous influence is the intended default; loud coercion
  is a lever you *pay* for, in favor and in faith.
- **The autonomy pillar is the final test for every verb (§0 law 2 + build-report §7-point-13).** If a proposed
  power reads as "tell this unit to do this now," it is **not a god-power** — it must become terrain, resource,
  weather, condition, or faith. The only direct control is Embody (B12). Guard this: the RTS temptation will try
  to sneak in through the power palette.

## 5. Open questions

- **Favor units & per-verb costs** are B13's to set; §1.1's relative scale is the proposed shape (cost ∝ area ×
  duration × permanence × agency-intrusion, discounted by devotion). Make it **tunable data**, not code
  (§7-point-12), so the god-economy is balanceable.
- **Blessing upkeep model:** do standing blessings (②) drip favor while active, or pay once? Proposed: a small
  upkeep drip so the god can't blanket the world in permanent blessings — a standing act is a standing cost.
- **Cooldown vs favor limiter:** B2b's favor⇄cooldown toggle (build-report) applies here — some powers may gate
  on cooldown instead of/alongside favor. Resolve per-verb at B13.
- **Legibility surfacing:** the chronicle (§3t) is where ambiguous acts get *attributed* after the fact — the
  faith payoff of a quiet miracle arrives when the chronicle names it. Depends on the chronicle system.
- **No contradictions found** with B13 (build-report §6), the divine-politics bible (DP1–DP5), the agency bible
  (minds), or §3y (weather/nature). This catalog organizes B13's reuse-verified palette into a build-ordered verb
  menu and extends it with the divine-politics levers; it supersedes nothing.

*End of God-Powers / Divine-Acts Catalog v0.1 — the verbs of a god, costed, and ordered so B13 has a menu.*

---

## Schema addition — ALIGNMENT WEIGHT + CAST-VFX PRESET per power (architect-approved 2026-07-10)

From UI-5 / GOD-HAND (the good/evil hand): **the powers ARE the deeds that move the god's alignment.** Every
catalog power gains TWO per-power fields (a schema lock — the god-hand + alignment systems read them):

- **`alignment_weight`** (−1 cruel … 0 neutral … +1 benevolent): how casting this power drifts the god's
  alignment (UI-5/GOD-HAND §0 — alignment is EARNED by deeds, not chosen). E.g. *Bless a Harvest* +, *Heal* +,
  *Call to Shelter* +, *Answer a Prayer* +; *Smite/Wrath* −, *Plague/Curse* −, a careless *Throw* −; most
  *Blessings* mildly +, most terrain/neutral acts ~0. Tunable in RON.
- **`cast_vfx`** (a preset id on Veloren's `outcome.rs` Outcome bus): the power's cast effect, with a **GOOD-tint
  and an EVIL-tint variant** (reagent/ParticleMode preset — the same power wears the god's face; UI-5 §divine-
  effects / GOD-HAND §4). NO new particle system — reuse the Outcome/ParticleMode/glow bus.

**Also (GOD-HAND §1):** the god-hand's physical verbs (grab/lift/carry/drop/throw/stroke/slap/tap/sculpt/paint)
join the catalog as the god's *physical* repertoire — each with its `alignment_weight` (stroke +, slap/throw −,
gentle-set-down +, careless-drop −) + its `anim::hand_*` (GOD-HAND §2) + Outcome VFX. The catalog is thus the
single source of the god's whole verb set (miracles + blessings + passives + hand-verbs), each carrying its
alignment weight + cast VFX. See `GOD-HAND-design.md` (the map) + `UI-HAND-ALIGNMENT-DIVINE-EFFECTS-design.md`
(the alignment/effect detail).

---

## GAP-AUDIT ADDENDUM — new divine verbs from the overnight design run (2026-07-10, architect-approved lock)

*(The overnight passes (DIVINE-CHAMPION/DF-CURSE/SACRED-SITES/DF-ANCESTORS/DF-BEAST/DF-FESTIVAL/DF-OMEN/DF-KNOWLEDGE/
COLONIST-EMERGENCY-RUN) added divine acts not yet in the catalog. Locked here with `alignment_weight` (−1..+1) +
`cast_vfx` per the schema section above, so the god-power schema covers the full repertoire before it hardens.
Reconcile-note: **Curse a colonist** already exists above (Buff+thought) — extend it with DF-CURSE's LIFT condition
(atonement/quest/mercy), don't duplicate.)*

| New power | Source pass | Shape | `alignment_weight` | `cast_vfx` (good↔evil tint on outcome.rs) | Reuse / mechanism |
|---|---|---|---|---|---|
| **Anoint a champion** | DIVINE-CHAMPION | ② Blessing | **+0.7** | radiant STROKE-glow (gold) | the hand STROKE + a blessed champion-state (favor) |
| **Lay a geas** (bind a vow) | DF-CURSE | ② | **~0** (lawful) | a binding-rune mark | a DF-SYNDROME tripwire binding (break→curse) |
| **Consecrate a site** | SACRED-SITES | ② | **+0.5** | warm hallow-glow | extends DF-ROOMS ROOM-3 aura to any ground |
| **Desecrate a site** | SACRED-SITES | ② | **−0.6** | dark blight-aura | a cursed-site aura on ground |
| **Cleanse a haunt** | SACRED-SITES / DF-ANCESTORS | ① Miracle | **+0.5** | a clearing light | lift a cursed-site / quiet a haunting |
| **Lay a soul to rest** | DF-ANCESTORS | ① Miracle | **+0.6** (mercy) | a gentle release-light | quiet a restless dead |
| **Raise the dead** | DF-ANCESTORS | ② | **−0.9** (necromancy) | dark reanimation | undead body-swap — the dread power |
| **Bless the hunt** | DF-BEAST | ② Blessing | **+0.4** | a buff-glow on the party | a Buff on the hunt party |
| **Tame a beast** | DF-BEAST | ① Miracle | **~0** (awe) | an awe-light on the beast | DF-LIVESTOCK Pet, god-gated (rare + costed) |
| **Loose a beast** | DF-BEAST | ② | **−0.8** | a goad/rage VFX | set a titan on a rival (rtsim) |
| **Bless a feast** | DF-FESTIVAL | ② Blessing | **+0.5** | warm feast-glow | a stronger festival mood-lift |
| **Blight a feast** | DF-FESTIVAL | ② | **−0.5** | a curdling VFX | the feast that sours |
| **Manifest a sign** | DF-OMEN | ① Miracle | **~0** | the cast-VFX aimed at the sky (alignment-tinted) | a portent — the MEANING stays the colony's |
| **Inspire a revelation** | DF-KNOWLEDGE / DF-OMEN | ② Blessing | **+0.4** | a dream/insight glow | a discovery or a dream-secret whispered |
| **Command an emergency-run** | COLONIST-EMERGENCY-RUN | ② directive | **~0** | a rally-pulse | the colony runs, at a collective `comp::Energy` cost |

**These join the alignment-weight + cast-VFX schema.** Notably: the god's TWO HANDS are now both in the catalog —
**Anoint a champion (+0.7)** ↔ **Lay a curse / Raise the dead (−0.9)** — the bless/afflict poles that drive the
GOD-EPITHET drift. `cast_vfx` reuses the GOD-HAND alignment aura + `outcome.rs`/ParticleMode presets (NO new
particle system).
