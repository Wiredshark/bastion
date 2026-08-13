# COLONY-STATE PERSISTENCE — **PRE-REGISTRATION**

Written before any code change.

## 1 · THE GAP, MEASURED — not hypothesised

From this session's own A4 restart evidence, same world, same userdata:

| | boot 1 (founded) | boot 2 (**restart**) |
|---|---|---|
| `founding preset plot placed` | **3** | **0** |
| `plot registered` | **1** | **0** |
| jobs | **8** | — |
| colonists promoted | **8** | **8** |

**Eight colonists come back to a world with nothing to do.**

This is not a new discovery — it is documented at `server/src/rtsim/mod.rs`
(`bastion_colony_exists`), found **live by Ben restarting the celebration world**:
*"colonists came back, the zones did not."* The one-colony predicate was built to read
rtsim records **precisely because** the board does not survive. This row closes the other
half of that observation.

## 2 · WHY IT MATTERS, IN THE PACKET'S OWN TERMS

§8 B4: **there is no colonist anchor — work at F is the retention mechanism.** A restart
deletes every designation, so it deletes the work, so it deletes the only thing holding
the colony together. The colonists then wander, which is the exact cross-country
leash-march §4 exists to prevent — arriving by a different door.

## 3 · SCOPE — designations, NOT the whole JobBoard

`JobBoard` is `#[derive(Default)]` and its own doc says it is *"not serialized and not
recorder-sampled"*. Persisting all of it — the command-admission ledger, claims, the
monotonic id counter — would be wrong, not merely large: those are **legitimately
runtime-only**.

The principled line is the domain's own:

> **A designation is a standing ORDER. A job is transient WORK derived from it.**

So: **persist the designations; let the jobs regenerate.** That is one save-side concern,
and it restores the retention mechanism without pretending transient scheduler state is
durable.

## 4 · THE BARS

### P1 · **DESIGNATIONS SURVIVE A RESTART**
- **PASS:** found on boot 1, restart, and boot 2 reports the **same designation count and
  the same regions**, without any founding being re-issued.
- **FAIL:** today's behaviour — 3 → 0.
- **Precondition, printed above the result** (this session's own law, learned three times):
  boot 1 must be held past the **60 s** rtsim save boundary, and the save asserted **by
  content**, or the run is VOID and not RED.

### P2 · **WORK RETURNS, NOT JUST DATA**
- **PASS:** boot 2 regenerates jobs from the restored designations — `jobs=N` with
  **N > 0** — and colonists claim them.
- A designation restored into a store nothing reads is a **vacuous** pass; P1 alone
  cannot tell "persisted" from "persisted and inert". *(The gate-must-test-live-path law,
  applied to a save file.)*

### P3 · **A WORLD WITH NO COLONY RESTORES NOTHING**
- **PASS:** boot 2 on a world that never founded reports **0** designations and does not
  fabricate any.
- The matched control: without it, a bug that always emits the founding preset on load
  would pass P1 and P2.

### P4 · **THE ONE-COLONY BOUNDARY IS UNAFFECTED**
- **PASS:** A4's restart bar still passes — second founding after restart still refuses
  `reason="colony_exists"`.
- Persistence must not become a second, competing source of truth for "does a colony
  exist". The predicate keeps reading rtsim records.

### PLANTS
1. **Save path disabled** ⇒ P1 red (3 → 0, today's behaviour returns).
2. **Load path disabled** ⇒ P1 red while the save file still contains the designations —
   separating *"we never wrote it"* from *"we never read it"*, which a single plant
   cannot distinguish.
3. **Restore designations but skip job regeneration** ⇒ **P2 red, P1 green** — proving P2
   measures work and not merely data.

## 5 · WHAT I WILL **NOT** DO

1. **I will not persist the whole `JobBoard`.** Its own documentation names the parts that
   are runtime-only; serialising them would freeze scheduler state that is meant to be
   rebuilt, and would make every future scheduler change a save-compatibility problem.
2. **I will not let persistence answer "does a colony exist".** §8 B6 settled that on
   rtsim records, and P4 guards it. Two sources of truth for one predicate is the defect
   that row already avoided once.
3. **I will not score P1 without the 60 s save precondition asserted by content.** A hard
   kill silently voids exactly this test — it did so earlier this session, and the void
   read as a clean red.
4. **I will not accept P1 alone as the row.** Data without work is a green harness over an
   inert feature.
5. **If the save format cannot carry designations without a migration**, I stop and report
   that as the blocker, naming the file and symbol, rather than inventing a format
   sideways.
