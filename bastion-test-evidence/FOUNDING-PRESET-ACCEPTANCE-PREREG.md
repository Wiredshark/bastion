# FOUNDING PRESET v1 — **SCORED ACCEPTANCE, PRE-REGISTERED**

**Written 2026-08-12 BEFORE any acceptance data exists.** *Smoke result
(`6848c22d35`) is the only live evidence so far, and its three findings are inputs
here.* **Packet §5 A1–A5, as amended by §8 (binding).**

---

## 0 · ★★★★★★ WHAT I WILL NOT DO AT SCORING TIME

*Written by the scorer, before the run, naming the generous moves that will feel
reasonable at this gate's particular sunk cost.*

1. **I will not score a run whose binary stamps I have not read FROM THE LOG.** *Both
   of them. `--no-auth` and a fresh userdata do not exempt a run from gate 0.*
2. **I will not read the DRIVER's view as a witness for server state.** ★★★ *Smoke
   F-3: `list_designations` said `rev=0 []` while three regions were placed. The
   server log is the authority; the driver mirror is not evidence of absence.*
3. **I will not treat B1 as exercised because the plots landed correctly.** *Smoke F-1:
   the player's z equalled the datum, so a correct result is equally consistent with
   the bug. **A correct answer from an unexercised discriminator is not evidence.***
4. **I will not score A3 on "founding stock" while the seed drop has no witness
   line.** *Smoke F-2. If the drop is unwitnessed, A3's premise is unread and A3 is
   VOID, not failed.*
5. **I will not accept a zero on any channel I have not proven reachable.** *Refusal #2
   of the program standard, and the reason F8 was scored PARTIAL on v5.*
6. ★★★★ **I will not let a PASS on A1 carry A2/A4.** *Co-resident results do not
   carry each other in either direction — the fix can hold while something else is
   wrong.*
7. **I will not extend the run because it is going well.** *Window declared below;
   the asymmetry test applies (would I extend a failing run? no ⇒ the rule is
   one-directional ⇒ it is not a rule).*

---

## 1 · ★★★★★ §8 N2 — **THE TIER DECISION, MADE BEFORE RUNNING**

**"Via the ACTUAL UI" is two tiers and they are not conflated here.**

| tier | what it is | what runs there |
|---|---|---|
| **MESSAGE tier** *(driver)* | `bastion_playtest`'s `spawn` sends the LIVE `ClientGeneral::BastionSpawnColony` — **the same message the in-game action sends**, handled at `in_game.rs:1293` | ★★★ **A1 · A2 · A4 · A5 and the F8-inclusion leg.** *Deterministic, scriptable, repeatable — this is where regression value lives.* |
| **WIDGET tier** *(human)* | mouse/keyboard through voxygen's founding control | ★★ **ONE founding, by a human, as the widget-wiring witness.** *Ben's original failure was AT the widget; this leg is what covers it.* |

★★★★★ **I cannot run the widget tier myself.** *It is a human-in-the-loop leg (§8 N3 —
real-time by definition, and the standing law's own second exception).* **It is
requested, not assumed, and A1–A5 do not depend on it.**

> ## **A DRIVER PASS DOES NOT CERTIFY THE WIDGET. THE WIDGET LEG IS A SEPARATE
> WITNESS AND ITS ABSENCE WILL BE REPORTED AS AN ABSENCE.**

---

## 2 · B7 — **BINARY PROVENANCE, BOTH BINARIES**

**Procedure (checklist 5, and the `64ad49dc1e` lesson that voxygen breaks on its own):**

1. **Build with `-p veloren-voxygen -p veloren-server-cli`** — *no `--bin` filter; a
   scoped filter exits 0 while excluding the package that matters.*
2. **Read the OUTPUT's `Compiling` lines** and confirm BOTH packages appear.
3. **Record both stamps.** *server-cli's from its boot log (`Server version:`);
   voxygen's from its own startup line.*
4. **Compare to `git rev-parse --short=8 HEAD` — pasted, never typed from memory.**

⚠ **VOXYGEN HAS NOT BEEN BUILT ON THIS BRANCH SINCE `PresenceKind::Colony` LANDED.**
★★★ **If it fails to compile, that IS the finding** — it is precisely the failure B7
exists to catch, and it blocks the widget leg rather than the driver legs.

---

## 3 · THE BARS — **with §8's corrections applied**

| # | measure | witness (named emit) | planted failure |
|---|---|---|---|
| **A1** | founding places the FULL preset | `colony founded … complete=true elements=stockpile,farm,bed designated_regions=3` | ★★★★ **PARTIAL preset (place all but the farm) ⇒ the witness must go RED** — `complete=false`, `elements` short, `designated_regions=2`. *(§8 B5: the old "disable placement" plant deleted subject and witness together.)* |
| **A2** | colonists STAY | positions within **R** of F across the window | ★★★★ **found WITHOUT the designations (stock + colonists only) ⇒ colonists leave R.** *(§8 B4: there is no anchor to omit; **the work being at F is the retention mechanism**, which is Ben's observation stated correctly.)* |
| **A3** | till → sow → eat | farm/eat emits | ⚠ **VOID unless F-2 is closed** — see refusal #4 |
| **A4** | second founding refuses by name | `founding refused reason="colony_exists"` | boundary check disabled ⇒ silent second colony |
| **A5** | terrain refusal fires | `founding refused reason="terrain"` on a bad site | ★★ **control (§8 N5): A1 founds successfully on the same arena** — else the bar is satisfiable by a founder that refuses everything |
| **F8-INCL** | real chop/mine completion through the generic path | `bastion: job completed` with drop+XP | *the run is the witness* — ⚠ **needs its own `designate` step; the preset places no chop/mine** (smoke §3) |

### ★★★ R, DERIVED (§8 N4 — R was undefined)

    spawn scatter:  rng.random_range(-5.0..5.0) in x AND y   -> max radial 7.07
    legitimate work travel: preset spans -7..+2 x, -4..+1 y  -> ~8 blocks to a far corner
    R = 16 blocks

★★ **R clears scatter + legitimate work with margin, and the failure it must catch —
a cross-country leash-march to another colony's coordinates — is HUNDREDS of blocks.**
*R is not a close call in either direction, which is what makes it safe to fix now.*

### RUN LENGTH — declared

**Each driver leg: 900 ticks (~30 s sim) after founding.** *Enough for promotion,
job claim and first completions; short enough to run the full A1–A5 matrix.*
⚠ **Declared here; flown exactly; extension would be a second run with its own
window.**

---

## 4 · WHAT IS ALREADY KNOWN NOT TO BE COVERED

- **B1 (z-datum)** — *unit tier only; smoke F-1 shows the arena does NOT exercise it.
  **Not reachable with the current driver** (no move verb, flat arena). Reported as
  uncovered unless a driver verb or a relief site lands first.*
- **The seed drop** — *no witness line (F-2).*
- **The widget tier** — *human leg, not mine to run.*

★★★★ **All three are named BEFORE the run so none of them can be discovered at scoring
time and quietly reframed as out of scope.**
