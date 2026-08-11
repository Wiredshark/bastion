# ROW: THE COLONY PRESENCE — **(a), RULED DECISIONS #106**

**Unblocks item 8.** *Until this lands, the endurance gate cannot certify
anything: every measure in its bar assumes colonists tick, and they do not.*

**The finding this answers:** founding colonists freeze into
`SimulationMode::Simulated` ~20 s post-founding, and **need-preemption has been
inert for every founding ever run** — invisible because every prior live leg had
a client holding the world loaded.

---

## ★ WHAT DOES NOT EXIST — READ FIRST, THE NAMES MISLEAD

**`Presence::bastion_terrain_anchor` is NOT the mechanism.** *All seven consumers
are in `client/src/lib.rs`; there are none server-side.* **It tells a CONNECTED
CLIENT where to keep terrain loaded. With no client, it does nothing.**

**And `PresenceKind` has no playerless variant:**

    Spectator · LoadingCharacter(CharacterId) · Character(CharacterId) · Possessor

★★ **So this row ADDS a concept. It does not wire an existing one.** *Budget it
that way; the anchor's name cost an hour of assuming otherwise.*

---

## THE SHAPE

**A server-owned presence that holds the colony's footprint loaded with no client
attached.**

1. **A new `PresenceKind` variant** — e.g. `Colony { … }` — carrying no
   `CharacterId`. *Every existing variant maps to a connected human; this one
   must not.*
2. **Minted when a colony is founded, dropped when it is disbanded.** *Lifetime
   tied to the colony, not to any session.*
3. **A view distance sized to the colony footprint**, not a player's. ★ **State
   the chosen number and its justification in the commit** — *it is the row's
   entire cost knob and the thing item 40 will revisit.*
4. **Colonists in that footprint stay `SimulationMode::Loaded`** — which is the
   observable that decides the row.

## ★★★ THE ACCEPTANCE — **THE PROMISE'S OWN CONDITIONS**

> **This row exists because we certified under conditions the promise excluded.
> Its own bar must not repeat that.**

**PASS requires, with NO client connected at any point:**

| # | measure | PASS | FAIL |
|---|---|---|---|
| 1 | colonists stay loaded | ★ `SimulationMode::Loaded` for all colonists at T+20s, T+2min, T+10min | any flip to `Simulated` |
| 2 | ★★ **needs actually tick** | need values CHANGE between samples | frozen values |
| 3 | preemption fires | ≥1 `NeedCrossed` (or the existing preempt witness) during a run long enough to cross | zero crossings across a full need period |
| 4 | no client required | the run has **zero** client connections in its log | any connection |

★★ **Measure 2 is the one that matters and the one most likely to be skipped.**
*"Still `Loaded`" is not "needs are ticking" — a loaded entity whose needs are
frozen would pass measure 1 and fail the row's actual purpose.* **Sample the same
colonist's hunger twice and require it to have MOVED.**

★ **Measure 4 is the anti-regression for the defect's own cause.** *The bug was
invisible because a client was present; the acceptance must prove none was.*

## NAMED FAILURE MODES

| mode | witness |
|---|---|
| presence minted but terrain system ignores it | chunk stays unloaded → colonists flip anyway |
| colonists load but needs still frozen | measure 2 — **the silent pass** |
| presence leaks after disband | held chunks with no colony |
| ★ **cost blowup** | held-chunk count per colony — **record it, it is the debt** |

## INHERITED PROCESS RULES

**§1a code identity at boot · effective config emitted (including the chosen view
distance) · attestation by count · every measure able to FAIL · zero cases VOID.**

## THE DEBT, WRITTEN DOWN AT BIRTH

> ★★ **Held chunks per colony grow with colony COUNT. This is bounded for one
> colony and unbounded across many.**

**Revisit trigger: item 40 / multi-colony**, where **(b) — needs as rtsim-native,
ticking without an ECS body — sits chartered as the eventual answer.** *(b) is a
MODEL change, not a wiring task: the `rtsim` crate has no needs concept at all
today.*

★ **Recorded now so it is known debt rather than a discovery.** *The reason (b)
was not chosen today is that it risks two implementations of needs — the defect
class this programme audited out of counters at retail price this week, and
electing it into the needs system would be the same disease at organ scale.*
