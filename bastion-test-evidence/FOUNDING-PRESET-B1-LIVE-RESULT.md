# §8 B1 — **THE Z-DATUM, EXERCISED LIVE FOR THE FIRST TIME**

**2026-08-12. Binary `6c2991eb` + the targeted-spawn driver (`8932f91f31`).**
*Resourced flat arena, fresh throwaway userdata per leg, `--no-auth`.*

> ## **B1 PASSES LIVE — AND IT IS FALSIFIER-BACKED, NOT MERELY OBSERVED.**

---

## 1 · WHY THIS RUN EXISTS

**Smoke finding F-1: the arena does NOT test the datum "for free".** *The handoff
claimed the driver founds at z=401 while the datum is 400; in fact the player settles
to **400**, which IS the datum, so "derived from terrain" and "took `pos.z`" produce
identical output.* ★★★ **The discriminator could never fire.**

**Fixed by `spawn <n> [x y z]`** — *the founding TARGET is decoupled from the player's
body, which is also what packet §3.1 describes ("the god TARGETS F").*

---

## 2 · ★★★★★★ THE THREE-STATE DEMONSTRATION

**Same script, same target, three builds.** *Target `(16400.5, 16400.5, 405.0)` over
arena ground whose first air cell is **400** — a **5-block** discriminator.*

| # | build | emitted |
|---|---|---|
| **1** | **REAL** | ✅ `colony founded … datum=400 complete=true` · plots `399..400 / 399..400 / 400..401` |
| **2** | ★★★★★ **MUTANT** *(`resolve_datum` returns `hint_z`)* | ⛔ **`founding refused reason="terrain" column=Some(Vec2 { x: 16398, y: 16396 })`** |
| **3** | **REVERTED + REBUILT** | ✅ `colony founded … datum=400` |

**The player stood at `(16384.5, 16384.5, 400.0)` throughout** — *16 blocks away and 5
below the target, confirming the driver targeted rather than followed
(`targeted=true` in its log).*

### ★★★★ WHAT THE RED PROVES

> ## **THE MUTATION DID NOT PRODUCE FLOATING PLOTS — IT PRODUCED A REFUSAL.**

*With the datum taken from the reported z (405), every preset column's real surface
(400) falls outside the ±1 standability window, so `validate_site` rejected the site
and named the first failing column.* ★★★★★ **The two mechanisms COMPOSE: a datum error
cannot silently place a hanging colony, because validation catches it first.**

★★ **That is a stronger result than the bar asked for.** *B1's failure mode is
self-limiting rather than silent.*

---

## 3 · WHAT THIS DOES **NOT** ESTABLISH

- ⚠ **This is NOT A5.** *A5 wants a refusal on a genuinely UNEVEN SITE. This refusal
  came from a planted datum defect on flat ground — same emit, different cause.*
  **A5 still needs a real bad site** *(now reachable: target the resourced arena's
  stone outcrop).*
- **The revert was verified two ways** — *`git status` clean in the engine worktree
  (checked with explicit `-C`, because the session cwd had moved), and zero occurrences
  of the mutation marker in the file.*

---

## 4 · EVIDENCE

    server-founding-b1-datum.log     GREEN  (pre-mutation build)
    server-founding-b1-MUTANT.log    RED    (planted mutation)
    server-founding-b1-REVERT.log    GREEN  (reverted + rebuilt)
    driver-founding-b1-*.log         the targeted sends
    script-founding-b1-datum.txt     one script, all three legs

★★★ **One script drove all three states.** *No per-leg tailoring — the discriminator is
in the code under test, not in the harness.*
