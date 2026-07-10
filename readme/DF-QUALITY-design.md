# Project Bastion — DF-QUALITY + DF-ARTIFACT Design v0.1 (the craftsmanship ladder + strange moods)

**One interlocking pass for the quality ladder and its apex, the artifact.** DF-QUALITY = per-instance
craftsmanship (skill → quality tier → value + mood); DF-ARTIFACT = the strange-mood event that produces a
named legendary artifact (or a dead colonist). Companion to the main build report (B-AG3 minds, B-AG4
inspector), the DF gap ledger (§D DF-QUALITY, DF-ARTIFACT), **DF-PRODUCTION** (which stamps quality at craft
time — this pass fills the `S6` stub it left), and the agency bible (the mind the moods act on).

**Which wall:** **SIMULATION** (craft-time quality computation; the emergent mood event) with the **CONTENT +
LEGIBILITY walls already fallen** — Veloren *ships a `Quality` enum with colors and UI wiring*, so quality is
half-legible for free.

**Fit-check verdict: PASS (exemplary god-game fit).** Quality is **autonomous** — colonists produce at their
own skill; you never set an item's quality, you influence *who crafts* and *bless skill*. The artifact is
**pure emergence** — you don't trigger a strange mood, it *happens*, and you experience the drama (a legendary
work is born, or a colonist goes insane and dies). The failure mode is a *situation you react to* — optionally
with a god-power (calm the fey mind, conjure the demanded material). This is the god watching creation, not
managing a bill. Textbook Pillar §1a.

**Ledger/corpus entries this consolidates:** `df-feature-gap-ledger.md` §D **DF-QUALITY** ("quality tier per
item, tied to skill; feeds value + mood") and **DF-ARTIFACT** (the fact-checked strange-mood mechanics). It
seams into DF-PRODUCTION (S6), B-AG3 (mood/thoughts), and DF-ROOMS/DF-PREF (quality→thought). Appends to the
corpus; rewrites nothing.

---

## 0. LOCKED: the canonical Quality vocabulary is Veloren's `item::Quality` — do NOT fork

**Architect directive (2026-07-09): lock the shared Quality enum canonically, purpose-enum style, one
authoritative location.** The reuse survey answers it cleanly: **that authoritative location already exists in
the engine, and Bastion defers to it** — exactly as every zoned system defers to the frameworks §2 `purpose`
enum.

```rust
// common/src/comp/inventory/item/mod.rs:72  — THE canonical quality vocabulary. Bastion reuses; never forks.
pub enum Quality { Low, Common, Moderate, High, Epic, Legendary, Artifact, Debug }
//                 (Ord + PartialOrd, color-coded for UI, already computed for modular items)
```

**The lock, stated as law (put this in `BASTION-SYSTEM-FRAMEWORKS.md` §2-adjacent):**
> The **one canonical quality tier enum is `common::comp::item::Quality`.** Any Bastion system that speaks of
> "quality" (craft quality, meal quality, room quality, artifact tier) uses **this** enum's variants. No
> parallel Bastion quality enum is created. Where a system needs finer craftsmanship gradations than the 8
> variants express, it maps onto them, it does not fork them. `Quality::Artifact` is the reserved apex
> (DF-ARTIFACT); `Quality::Debug` stays out of gameplay.

**Why this is the right lock (not a compromise):** the enum is already `Ord` (tiers compare), already
color-coded (Grey→…→Orange — the legibility ships), already used by the item value/UI path, and already
computed per-instance for modular items (`Item::quality()`, `mod_base.compute_quality`). Forking a Bastion
enum would duplicate all four and immediately drift. **The DF craftsmanship ladder maps onto it** (§2). One
enum, every quality-speaking system — the same "build once, many uses" discipline as the purpose enum.

---

## 1. The reuse split — the de-risk table

### SUBSTRATE — exists, needs wiring

| Piece | Real symbol / location | What it gives us |
|---|---|---|
| **The Quality enum** | `common/src/comp/inventory/item/mod.rs:72` — `Quality { Low..Artifact, Debug }`, `Ord`, colored | The canonical tier vocabulary + its UI legibility (color per tier). |
| **Per-instance quality already exists (modular)** | `Item::quality()` (`:1385`); `ItemBase::Modular(mod_base).compute_quality(components)` | Modular items **already compute quality per-instance from components** — the machinery for "this specific item's quality ≠ its type's base" exists; we extend it to a craft-quality stamp. |
| **Quality → value/UI** | item value path + quality colors (used in HUD tooltips) | Higher quality already reads as more valuable + shows its color. Trade value (DF-TRADE) consumes it. |
| **Crafter skill** | `ColonistSkills` / `ColonistSkills::grant_xp` (`common/src/comp/bastion.rs`), B5 `work_rate(skill)` | The skill input DF quality is computed from — already per-colonist, per-work-type. |
| **The mind (thoughts/mood)** | **B-AG3 (DONE)** — event×personality×values×memory→emotion→mood; the agency bible §5b | The consumer of quality→thought ("I own a masterwork" = good thought). Owning/using fine goods feeds this pipeline. |
| **Preferences** | `df-feature-gap-ledger.md` §B **DF-PREF** (extends B-AG3 values) | DF: item quality depends partly on the crafter's *preference* for the item; the artifact's *type* honors preferences. Ties this pass to DF-PREF. |
| **Produce-job (craft hook)** | **DF-PRODUCTION S1** (`WorkType::Produce`, `craft_simple`) | The exact moment to stamp quality — at produce-job completion. This pass fills DF-PRODUCTION's S6 stub. |
| **rtsim history / naming** | rtsim accrues history (ledger §C); **DF-HIST** (Legends/Chronicle — CLAIMED, parallel) | The named legendary artifact + its legend entry ride the chronicle. Seam with DF-HIST. |

### BUILD — genuinely net-new

| Piece | Why it's new | Folds into |
|---|---|---|
| **Craft-quality stamp** | Simple items read static `item_def.quality`; nothing computes a **per-instance craft quality from crafter skill** at craft time. | DF-PRODUCTION S1 (produce-job completion) |
| **Skill→quality curve** | The RON-tunable mapping (skill band → quality tier, with mood/focus/preference modifiers). | Tuning-data |
| **Quality→thought hook** | Owning/using/seeing a fine (or shoddy) item generates a B-AG3 thought. | B-AG3 (the mind — DONE) |
| **The strange-mood event (DF-ARTIFACT)** | An emergent colonist state: a counter+chance clock strikes a colonist, who claims a workshop, demands materials in order, works to exclusion, and produces a named artifact — or goes insane and dies. | B-AG3 (mood) + DF-PRODUCTION (workshop) + hazard-event shape |
| **Named-unique item** | A specific one-off item with a generated name + legend + `Quality::Artifact`. | Item instance + DF-HIST (legend) |

**The collapse:** DF-QUALITY is **a craft-time stamp + a skill curve + a thought hook** — the enum, the
per-instance machinery (modular), the skill, the value path, and the mind all exist. DF-ARTIFACT is **an
emergent event** wiring together B-AG3 (mood), DF-PRODUCTION (workshop), and this pass's Artifact tier — its
scariness (a colonist can die) is *content*, not new engine.

---

## 2. The DF craftsmanship ladder → the canonical enum (the mapping)

DF's per-instance quality ([DF Item quality](https://dwarffortresswiki.org/index.php/DF2014:Item_quality)) is
**craftsmanship**, orthogonal to Veloren's item **rarity** — but both are "quality tiers," so they share the
one enum. The map (DF ladder → `Quality` variant):

| DF craft quality (symbol) | ~Value mult | Skill band (cited) | → Bastion `Quality` |
|---|---|---|---|
| standard | ×1 | 0–21 | `Low` / `Common` |
| -well-crafted- | ×2 | 22–29 | `Moderate` |
| +finely-crafted+ | ×3 | 30–34 | `High` |
| \*superior\* | ×4 | 35–44 | `High` (upper) |
| =exceptional= | ×5 | 45–54 | `Epic` |
| ☼masterful☼ (masterwork) | ×12 | 55+ (≤1/3 chance) | `Legendary` |
| artifact | ×120 | strange mood only | `Artifact` |

(Skill bands + multipliers per the DF wiki; Bastion's exact bands are **RON-tunable** — the *shape* is the
design, the numbers are data.) The key DF truths to keep: **quality is per-instance and skill-driven**,
**masterwork is capped-rare** (not guaranteed at max skill — [cited](https://dwarffortresswiki.org/index.php/DF2014:Item_quality)),
and **artifact is unreachable by normal crafting** — it comes *only* from the strange mood (DF-ARTIFACT). That
last rule is what makes an artifact special: you cannot grind for it.

---

## 3. Systems needed

### S1 — The craft-quality stamp (fills DF-PRODUCTION S6)
At produce-job completion (DF-PRODUCTION S1), compute a `Quality` from `crafter_skill` via the RON curve (§2),
modified by the crafter's mood/focus and preference for the item, roll against the capped-masterwork chance,
and **stamp it onto the specific output item instance**. **Where:** the produce-job completion arm
(`bastion_jobs.rs`) + a per-instance craft-quality field on the item (the schema decision — see §7 Q1). **Deps:**
DF-PRODUCTION S1. **Folds into:** the produce-job.

### S2 — Quality → value → mood
Quality already scales value (Veloren path); the net-new is the **thought hook**: owning/using/being-in-a-room-
with a fine or shoddy item emits a B-AG3 thought (good for masterwork, bad for shoddy/worn). Ties **DF-ROOMS**
(room value from furniture quality) and **DF-PREF** (a colonist who *likes* the item feels it more).
**Where:** a B-AG3 thought source keyed on item quality (loaded) + an rtsim aggregate ("colony wealth/comfort"
raises baseline mood, LOD). **Deps:** B-AG3 (DONE); DF-ROOMS/DF-PREF (seam). **Folds into:** the mind.

### S3 — The strange-mood event (DF-ARTIFACT) — the emergent drama
The fact-checked mechanic, kept faithful (the ledger's spec is the source):
- **Trigger clock:** once the colony passes a citizen threshold (~20, RON), a counter+chance clock periodically
  strikes an eligible colonist with a **strange mood**. Mood *type* (fey / possessed / macabre / fell / secretive)
  keys off the colonist's B-AG3 mind-state — **reuse the mood axis, don't invent one.**
- **Claim + demand:** the moody colonist **claims a workshop** (locks it) and **demands specific materials in a
  specific order** (the item type + materials honor the colonist's **preferences**, DF-PREF).
- **Obsessive work:** they work **to the exclusion of eating/sleeping** (a special produce-job that ignores
  Needs) until done.
- **Two outcomes (keep BOTH — the failure IS the drama):** demands met → a **named artifact** (`Quality::
  Artifact`) + the colonist gains **legendary skill** in that craft; demands *un*met within the window → the
  colonist **goes insane and dies** (a berserk/melancholy/stark-raving end — reuse B-AG3 breakdown states).
**Where:** new `server/src/bastion_moods.rs` (the clock + the moody-colonist state machine) + a special produce-
job + the named-item generator. **Deps:** B-AG3 (mood/breakdown — DONE), DF-PRODUCTION (workshop/craft),
DF-PREF (demands/type), DF-HIST (the legend — CLAIMED parallel). **Folds into:** the mind + the produce-job;
shaped like a **hazard-event** (a triggered situation the colony/god reacts to) per frameworks §1a.

### S4 — The named-unique artifact + its legend
Generate a one-off item: a name (procedural, DF-style "The X of Y"), `Quality::Artifact`, a legend entry
(image/history in DF; a Chronicle entry here), and permanent world-memory. **Where:** item instance + DF-HIST
seam. **Deps:** S3, DF-HIST. **Folds into:** DF-HIST (the world's memory).

---

## 4. Assets & animations

**Assets:** DF-QUALITY needs **none** (quality is a tag + a color that ships). DF-ARTIFACT wants **NEEDS:DF-
ARTIFACT** decorative treatment for artifact-tier items (a glow/ornateness pass so a masterwork *reads* as
special) → READY when S3 lands; and the named artifact reuses the produced-good model with an ornate variant.
Mostly a shader/tint concern, not new geometry.

**Animations:** the moody colonist's obsessive crafting reuses the **craft-at-station** animation (DF-PRODUCTION
§5, `anim::craft_*`) — **NATIVE-to-that-pass**, no new debt here. The **breakdown/insanity** end reuses B-AG3's
breakdown behavior (tantrum/berserk/melancholy) — existing. A "moody" idle pose (staring, gathering) is a
**minor NEEDS:animation-code** nicety, not required for v1 (reuse an existing agitated/idle state).

---

## 5. Legibility · Control-spectrum · LOD

**Legibility:**
- **Quality colors ship** (Grey→Orange per tier) — an item's quality reads at a glance in tooltips/inspector.
- **B-AG4 inspector:** a colonist's produced-item quality distribution + their skill → "why their work is fine."
- **The strange mood is LOUD by design** (it must be — it's the drama): an **alert** when a colonist enters a
  mood ("Urist has been taken by a fey mood!"), a visible claimed-workshop marker, the demanded-materials list
  (so the player can *react* — feed the demand or watch the tragedy), and a **Chronicle** entry at both
  outcomes (artifact born / colonist lost). The named artifact gets a legend page (DF-HIST).
- This is the payoff moment of the whole colony sim — its legibility is first-class, not a readout.

**Control-spectrum:**
- **Autonomous:** quality emerges from skill; artifacts emerge from the clock. Zero required input.
- **Manage:** influence *who* crafts what (assign your best smith via work priorities) — indirect quality
  control, never setting a number.
- **God layer (B13 / God-Powers):** the strange mood is a rich divine-intervention hook — **① Miracle:**
  *conjure the demanded material* (save the colonist) or *still the fevered mind* (end a doomed mood gently);
  **② Blessing:** *bless a crafter's hands* (raise effective skill → quality) as a standing enchantment;
  **③ Passive:** worship/favor nudges the masterwork chance. All are *causes the colony reacts to*, never
  "make item X" — you bless the smith, you don't craft the sword.

**LOD:**
- **Loaded:** per-item quality stamp; the full strange-mood state machine.
- **Unloaded (rtsim):** colonies still *produce quality goods* as an aggregate (a fraction masterwork by mean
  skill) and can *still throw a strange mood* resolved abstractly (artifact added to the colony's legend, or a
  death logged) — **tendency-first**: an unwatched colony's master smith can still make history. Reconciles on
  load (the artifact exists as a real item; no dupe). No accumulation here (quality is a stamp, not a stock).

---

## 6. Sequenced sub-blocks, each with a concrete Done-when

Dependency-ordered. **v1 = QUAL-0..QUAL-1; enrichment/drama = QUAL-2..QUAL-3.** **Hard dep: DF-PRODUCTION S1
(produce-job) must exist** — quality is stamped at craft time.

### QUAL-0 — Lock the enum + the craft-quality stamp · [DF-QUALITY core]
**Depends:** DF-PRODUCTION S1. Builds S1 + the §0 lock.
**Scope:** Bastion adopts `item::Quality` as canonical (documented in frameworks §2); produce-job completion
computes a per-instance craft quality from crafter skill (RON curve, capped masterwork) and stamps it on the
output.
**Done-when (`--quality-scenario`):** two colonists of different skill craft the same recipe; the outputs carry
**different, skill-ordered `Quality` tiers** (higher skill ⇒ ≥ tier, monotonic over many samples); masterwork
appears only at high skill and **at a capped rate** (≤ the RON cap over N samples, asserted statistically);
`Quality::Artifact` **never** appears from normal crafting. No dupe/loss; the stamp persists through save/load.

### QUAL-1 — Quality → value → thought · [the payoff seam]
**Depends:** QUAL-0, B-AG3. Builds S2.
**Scope:** quality scales trade value (reuse Veloren path); owning/using a fine or shoddy item emits a B-AG3
thought.
**Done-when (`--quality-mood-scenario`):** a colonist given a masterwork item they use/own gains a positive
thought (mood delta > 0 via the B-AG3 pipeline); a shoddy/worn item gives a negative thought; the delta scales
with tier and is **larger for a colonist who prefers the item** (DF-PREF hook, if present) — asserted through
the existing B-AG3 thought API, not a bespoke path.

### QUAL-2 — The strange mood (fey → artifact) · [DF-ARTIFACT, the happy path]
**Depends:** QUAL-0, B-AG3, DF-PRODUCTION. Builds S3 (trigger + claim + obsessive craft + artifact outcome).
**Scope:** the counter+chance clock (≥ threshold pop), moody-colonist state machine, workshop claim, demand
list (honoring preferences), obsessive produce-job, artifact + legendary-skill outcome.
**Done-when (`--strange-mood-scenario`):** in a ≥threshold colony, the clock strikes a colonist within a
bounded window; they claim a workshop (marked, locked to others), demand materials, and — **when the demands
are supplied** — produce a **named `Quality::Artifact` item** and gain legendary skill in that craft;
conservation holds (demanded materials consumed, one artifact produced); an alert + Chronicle entry fire. The
mood *type* matches the colonist's B-AG3 mind-state (reused, not invented).

### QUAL-3 — The tragic outcome + the legend · [DF-ARTIFACT, the drama completed]
**Depends:** QUAL-2, B-AG3 breakdown, DF-HIST (seam). Builds the failure branch + S4.
**Scope:** unmet-demand path → insanity/death (reuse B-AG3 breakdown states); the named artifact's legend
entry; the god-intervention hooks (conjure material / still the mind).
**Done-when (`--strange-mood-fail-scenario`):** a moody colonist whose demands are **not** met within the RON
window enters a B-AG3 breakdown and dies (entity removed cleanly, no leaked claim on the workshop, Chronicle
logs the loss); and, separately, a god-power that supplies the demanded material **before** the window flips
the outcome to success (artifact) — proving the drama is a *situation the god can react to*, not a scripted
result.

---

## 7. Dependencies · open questions · tuning-data · corpus notes

### Dependencies
- **DF-PRODUCTION S1 (produce-job) — HARD.** Quality is stamped at craft completion; without the produce-job
  there's no craft moment. This pass **completes DF-PRODUCTION's S6 stub** (bidirectional seam).
- **B-AG3 (minds/mood) — DONE.** The thought hook (QUAL-1) and the strange-mood type + breakdown (QUAL-2/3)
  reuse it directly.
- **DF-PREF — soft.** Preferences modulate quality + the artifact's demanded type; degrade gracefully if PREF
  isn't built (skip the preference modifier).
- **DF-HIST — seam (CLAIMED parallel).** The named artifact's legend rides the Chronicle. Coordinate the
  legend-entry schema with the DF-HIST designer (don't fork the chronicle event type).
- **DF-ROOMS / DF-TRADE — downstream consumers** of quality (room value, trade value). Not blockers.

### Open questions (flagged for the architect)
1. **Per-instance craft-quality field — the schema call.** Simple items read static `item_def.quality`; a craft
   stamp needs a per-instance override. Add a `craft_quality: Option<Quality>` on the item instance (clean,
   explicit), or model craft quality as a synthetic "component" (reuses the modular `compute_quality` fold)?
   *Rec:* an explicit per-instance `Option<Quality>` override on the item instance — modular's fold is a
   different axis (material components), and overloading it will confuse. **This is the load-bearing schema
   piece to lock alongside the enum.**
2. **Masterwork cap semantics** — a hard ≤1/3 roll (DF-faithful) or a smoother skill curve? *Rec:* keep the
   DF-faithful capped roll (masterwork must feel *earned/lucky*, not inevitable) — it's what makes a masterwork
   special; expose the cap in RON.
3. **Strange-mood eligibility** — any citizen, or gated by skill/personality (DF gates by "moodable" labor)?
   *Rec:* gate by a moodable-labor + a personality lean (reuse B-AG3 facets) so the artifact reflects a *real*
   crafter — richer and less random.
4. **rtsim artifacts** — can an *unloaded* colony throw a strange mood and mint an artifact abstractly? *Rec:*
   yes (tendency-first — an unwatched master can still make history), resolved as a low-rate aggregate event;
   the artifact materializes as a real item on load. Flag the numeric model as its own tuning task.

### Tuning-data (RON)
Skill→quality bands + modifiers (mood/focus/preference); masterwork cap; strange-mood clock (threshold pop,
period, per-tick chance); mood-type weights; demand-window length; quality→value multipliers; quality→thought
deltas per tier. **All data; the systems read it.**

### Corpus notes
- **Strengthens the corpus, no contradiction:** the §0 lock resolves the DF-PRODUCTION S6 "Quality schema seam"
  the architect flagged — one enum (Veloren's), Bastion defers. Recommend adding the lock text to
  `BASTION-SYSTEM-FRAMEWORKS.md` §2 (next to the purpose-enum lock) so it's canonical, not buried here.
- **DF-ARTIFACT kept faithful:** the ledger's fact-checked spec (threshold, demand-order, work-to-exclusion,
  artifact-or-death) is reproduced, not softened — "the failure mode IS the drama; keep it." This design keeps
  it and adds the god-intervention layer as the *only* new twist (fits the pillar).

## 8. Honest limits
- **Inert without DF-PRODUCTION.** No craft moment ⇒ nothing to stamp ⇒ no artifacts. This pass is the
  *quality half* of the production loop and is honest that it rides DF-PRODUCTION (which rides B6).
- **The strange mood is the highest-drama, highest-risk sub-block** — a system that *kills a colonist* must be
  legible and fair (the demand must be *meetable* and *readable*), or it feels arbitrary rather than tragic.
  QUAL-3's Done-when deliberately tests the god-intervention save path to prove the drama is reactable.
- **rtsim artifacts (Q4) are a principle, not a spec** — flagged for its own numeric model, not claimed done.
- **DF-PREF is a soft dep designed to degrade** — quality works without preferences (skill-only), but the
  *richness* (an artifact that honors what a colonist loves) waits on DF-PREF.

*End of DF-QUALITY + DF-ARTIFACT design. The canonical quality vocabulary was already in the engine; the pass
locks Bastion to it, adds a skill-driven craft stamp + a thought hook, and reproduces DF's strange mood — the
colony sim's signature drama — as an emergent event the god can witness and, at most, intervene in.**
