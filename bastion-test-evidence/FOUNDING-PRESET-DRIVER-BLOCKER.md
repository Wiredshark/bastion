# BLOCKER — **the driver can only found where the player stands**

**Found 2026-08-12 while preparing the A1–A5 scripts. Blocks TWO acceptances with one
missing primitive.**

---

## THE EXACT SYMBOL

**`client/src/bin/bastion_playtest.rs`**

    :175   "spawn" => ScriptCmd::Spawn(rest[0].parse().expect("bad spawn count")),

    :452   ScriptCmd::Spawn(count) => {
    :453       client.bastion_spawn_colony(current_pos, count);

**Six verbs exist** *(`wait` :173, `anchor` :174, `spawn` :175, `designate` :176,
`list_designations` :189, `note` :202)* — **none of them moves the player, and `spawn`
parses a COUNT only.**

> ## **THE FOUNDING POSITION IS ALWAYS `current_pos`. ON THE FLAT ARENA THAT IS ALWAYS
> FLAT AND ALWAYS z=400.**

---

## WHAT IT BLOCKS

| | needs | why blocked |
|---|---|---|
| **A5** *(terrain refusal)* | a site with ≥±1 z variation in the preset footprint | *the spawn point is flat; the resourced arena's stone outcrop is unreachable without moving* |
| **B1** *(z-datum derivation)* | founding from a z ≠ the column's first air cell | *smoke F-1 — the player settles to 400, which IS the datum, so the discriminator never fires* |

★★★ **One missing primitive, two uncovered bars.** *Both were reported as gaps in the
smoke result and the pre-registration BEFORE this cause was identified; this names the
cause.*

---

## ★★★★★ THE FIX IS PLUMBING — **the API ALREADY TAKES A POSITION**

    client.bastion_spawn_colony(current_pos, count)
                                ^^^^^^^^^^^ the driver hardcodes it

**`ClientGeneral::BastionSpawnColony` carries a position** *(the driver's own log line
prints `pos=…`)*. **The message supports founding anywhere; only the script verb
doesn't.**

**PROPOSED: `spawn <n> [x y z]`** — *optional explicit position, defaulting to
`current_pos` so every existing script is unchanged.*

### ★★★★★★ AND IT IS CLOSER TO THE REAL UI, NOT A TEST HACK

**Packet §3.1: *"God targets F via the overseer founding action."*** *The god AIMS at a
position — they do not have to stand on it.*

> ## **"FOUND WHERE THE PLAYER STANDS" IS THE DRIVER'S SIMPLIFICATION, NOT THE UI's
> SEMANTICS. THE OPTIONAL POSITION MAKES THE DRIVER *MORE* FAITHFUL TO THE PATH IT IS
> STANDING IN FOR.**

★★ *That is the argument for doing it here rather than deferring: it removes a
divergence between the message tier and the widget tier that N2 depends on.*

---

## STATUS

⚠ **NOT APPLIED YET — a `cargo build` is running against this working tree** *(voxygen +
server-cli, for B7)*. **Editing `veloren-client` mid-build would corrupt it**
(shared-checkout collision protocol: cargo builds the WORKING TREE).

**Order: build completes → apply the verb → rebuild `bastion_playtest` →
red-demonstrate → then A5 and B1 become runnable.**

★ *A1/A2/A4 and the F8-inclusion leg are NOT blocked by this and can run first.*
