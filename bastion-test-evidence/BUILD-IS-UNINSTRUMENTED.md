# BUILD IS THE MOST COMMON FAILURE AND NOT ONE OF ITS TWELVE FIELDS DISCRIMINATES

**wave25, read from disk.** `build_placed` fails in **6 of 11** failing seeds —
**the most frequent clause in the corpus.** Fable's record has it **unclassified.**

## §1 — ★★★★★★★★ EVERY BUILD FIELD IS IDENTICAL ON FAILING AND PASSING SEEDS

| field | 61 | 62 | 71 | 80 | 85 | 92 | **passing (49/50/51/53)** |
|---|---|---|---|---|---|---|---|
| `build_ok_jobs` | 1 | 1 | 1 | 1 | 1 | 1 | ★ **1** |
| `build_stall_jobs` | 1 | 1 | 1 | 1 | 1 | 1 | ★ **1** |
| `build_stall_untouched` | T | T | T | T | T | T | ★ **T** |
| `b15_floater_skipped` | T | T | T | T | T | T | T |
| `b15_adjacent_claimed` | T | T | **F** | T | T | T | T |
| `b15_ontop_claimed` | T | T | **F** | T | T | T | T |
| `tool_stone` | 1.5 | 1.5 | 1.5 | 1.5 | 1.5 | 1.5 | 1.5 |
| `stone_sum` | 26 | 27 | **5** | 27 | 27 | 27 | 27 |
| **`build_placed`** | **F** | **F** | **F** | **F** | **F** | **F** | **T** |
| `any_needs_materials` | F | F | F | F | F | F | T |

> ★★★ **Only the two fields that ARE the verdict differ.** *(And per the Aug-4
> Mode-A refutation, `any_needs_materials` is a **definitional consequent** of
> `build_placed` — one fact reported twice.)*

★ **`build_stall_untouched` is TRUE on PASSING seeds.** Whatever it means — ★ *I
have not read its producer and will not characterise it from its name* — **it
cannot discriminate**, because it holds the same value whether build succeeds or
fails.

## §2 — ★★★★★ AND THE MATERIALS EXPLANATION IS DEAD ON ITS FACE

**Seeds 80, 85, 92 have `stone_sum = 27` — identical to every passing seed.**

> **The stones were there. Build failed anyway.**

★ **Seed 71 is the ONE exception and it is fully explained:** `stone_sum = 5`
(not 27) and **both `b15_*claimed` flags FALSE** — its mine failed, so the stones
never existed. ★ **71's build failure is a downstream consequence with a known
cause and should be scored separately from the other five.**

> ★★★ **So: 61, 62, 80, 85, 92 — materials present, every diagnostic identical to
> a passing seed, build failed. FIVE SEEDS, ZERO SIGNAL.**

## §3 — ★★★★★★ THIS IS A STRONGER INSTRUMENT CASE THAN CHOP

**Chop at least has reachability** — for seeds 80/92 it says *"no path exists,"*
which is a story even if incomplete. ★ **Build has twelve fields and produces the
SAME READING for success and failure.**

> **The chop gap is a MISSING dimension. The build gap is TWELVE fields that
> measure something other than what determines the outcome.**

★ **`build_ok_jobs = 1` and `build_stall_jobs = 1` on every seed in the corpus**,
passing and failing alike — **two counters that never vary at all.** *(Constant
fields are not evidence; they are decoration. And they have been carried in every
wave.)*

## §4 — WHAT I DO **NOT** CLAIM

- ★ **Not** that these fields are wrong. **I have not read their producers**, and
  today has punished me three times for reasoning from a field's shape or name
  instead of what writes it. **They may measure exactly what they were built to
  measure — that thing is simply not what decides `build_placed`.**
- **Not** that the five share a mechanism. **Five seeds, one shared symptom, no
  discriminating data** — that is the definition of *unclassified*, and it is why
  Fable's record already says so.
- **Not** a new fix direction. ★ **This argues for an INSTRUMENT, and it argues
  more strongly than the chop spec did.**

## §5 — CONSEQUENCE

★★★ **I'd re-rank the chop instrument BEHIND a build instrument**, on three
counts: build is the **more frequent** clause (6 vs 4), its gap is **total** (chop
has partial coverage), and **one of its six seeds is already explained** (71),
which means a build instrument starts with a **known-good control inside its own
population.**

★ **The chop spec stands as written and needs no change** — this is a **ranking**
argument, not a correction to it. **Fable's call.**

★★ **And the same acceptance criterion transfers verbatim:** *"never claimed" and
"claimed and failed" must render differently.* **Right now build cannot even say
which of those five seeds ever had a colonist walk to the site.**
