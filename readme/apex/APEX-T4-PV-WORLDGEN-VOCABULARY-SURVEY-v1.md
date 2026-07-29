# APEX-T4-PV — worldgen vocabulary: SURVEY (v1)

**Status: SURVEY. Not a vocabulary, not a version, and not approvable by
its author.** `T4.3` ruled that `WorldgenProtocolVersion` must be a
frozen-vocabulary content-root derivation "never arbitrary integers".
This document is the internals read that lets someone derive it without
fabricating it. It proposes no values and changes no code.

Surveyed from live reads of `world/src` at `45aa651680`. Every claim
below was read, not recalled; where I could not establish something, it
is in §5 as an open question rather than smoothed over.

---

## 1. The question the vocabulary has to answer

> What must two servers agree on before they may claim they generate the
> SAME WORLD?

Not "what does worldgen read" — that is all of `world/src`. The
vocabulary is the set of inputs whose disagreement produces a different
world **and which are not already pinned by something else**. Getting
that second clause wrong in either direction is the failure mode:
duplicate what is already covered and the version churns for no reason;
miss what is not and it certifies agreement that does not exist.

## 2. Step zero — what is ALREADY covered (and must NOT be re-enumerated)

This is the survey's main structural finding, and it shrinks the row
substantially.

**(a) Everything compiled is covered by `T1.2`'s source closure.**
Worldgen is overwhelmingly CODE, not data: 48 files under
`world/src/site/plot/` alone, and only 12 asset loads in all of
`world/src`. `world/src/sim/erosion.rs` declares no top-level constants
at all — its parameters are inline literals and function arguments.
`CONFIG` (`world/src/config.rs:60`) is a compiled `const` with 17
numeric fields, read at 268 sites across `scatter`, `wildlife`,
`column`, `sim`, `civ`, and `erosion`.

A vocabulary that enumerated `CONFIG.sea_level`, `mountain_scale`, the
erosion coefficients, and the `diffuse` multiplier
(`util/seed_expan.rs`) would be **restating the source closure in a
second, hand-maintained list** — the exact two-lists-that-drift failure
`E13`/`E14-3` spent five chunks eliminating. Editing any of them edits
`world/src`, which moves the source root already.

**(b) Everything in `assets/` is covered by the content root.** The
asset-root recompute walks the whole tree and digests every file as
`assets/<rel>` (`bastion-harness/src/main.rs:1240-1296`). So
`world.style.colors`, `world.features` (the `Features` toggles —
`caves`, `trees`, `scatter`, `wildlife_density`, …) and the wildlife
spawn manifests, all loaded in `Index::new`
(`world/src/index.rs:67-85`), ride the content identity. Flipping
`caves: false` changes generated worlds and already moves that root.

**So the vocabulary is not "the constants of worldgen". It is the
inputs that can differ between two servers running the SAME BINARY over
the SAME ASSETS.** Those are runtime inputs, and there are few of them.

## 3. Candidates, with verdicts

### MUST-BE-IN-THE-VOCABULARY

| Candidate | Where | Why it qualifies |
|---|---|---|
| `FileOpts` variant | `sim/mod.rs:168` | **The sharpest item, and the one a code-only derivation misses entirely.** `Generate`/`Save`/`LoadOrGenerate`/`LoadLegacy`/`Load` — under the load variants the world is **not derived from the seed at all**; it is the bytes of a map file. Two servers on identical code, assets and seed generate different worlds if one loaded a map. A version that omits this certifies an agreement that does not exist. |
| `GenOpts.x_lg`, `y_lg` | `sim/mod.rs:147-153` | Map dimensions (log₂). Different size, different world, same seed. Runtime-settable, default 10/10. |
| `GenOpts.scale` | same | Horizontal scale, default 2.0. |
| `GenOpts.map_kind` | same | `Square`/`Circle` — changes the domain shape. |
| `GenOpts.erosion_quality` | same | Erosion is the dominant terrain-shaping pass; quality changes its result, not merely its cost. |
| `WorldOpts.seed_elements` | `sim/mod.rs:458-463` | Documented as "disable seeding elements during worldgen" — a boolean that changes what gets placed. |
| The world seed itself | `Index::new(seed)`, `WorldSim::generate(seed, ..)` | Already the acknowledged input; named for completeness because the vocabulary is a *set*, and a set that omits the obvious member invites someone to assume other obvious members are omitted too. |

### ALREADY-COVERED (do not enumerate — say what covers them)

| Candidate | Covered by |
|---|---|
| `CONFIG`'s 17 fields | source closure (compiled `const`) |
| erosion parameters | source closure (inline literals, no constants to name) |
| `diffuse` multiplier + seed expansion | source closure. Note its endianness hazard was already found and fixed in place (`RNG-DEEP-001`, `to_le_bytes` with the reasoning recorded at the call site) |
| `Noise`'s octave count (5) and the `seed+0/+1/+2` derivation | source closure (`index.rs:173-187`) |
| `Colors`, `Features`, wildlife spawn manifests | content/asset root |
| all 48 site-plot generators | source closure |

### IRRELEVANT

| Candidate | Why |
|---|---|
| `Index.time` | mutated at runtime, not a generation input |
| `Index.trade`, `Index.sites` | populated BY generation, not inputs to it — derived state |
| `stage_report` callback | progress reporting |
| `threadpool` | see §5 — an open question, not an irrelevance |
| `Calendar` (`WorldOpts.calendar`) | **provisional** — see §5 |

## 4. What a change to each does to a live save (T4.3 comparison)

Under `T4.3`, `RtSim::new` compares the stored `world_baseline_root`
against the freshly computed one and, on mismatch, records the loss to
`world_baseline_mismatch.json` and purges-and-regenerates unless
`RTSIM_IGNORE_WORLD_BASELINE` is set (`save_migration.rs`'s
`rtsim_baseline_support_v1`, `ExplicitRecoveryOnly`).

So for **every** MUST-BE row above, the consequence of a change is the
same and is already built: the root moves, the save is refused as
world-incompatible, the mismatch is recorded, and the operator has a
blunt escape hatch. The vocabulary does not need new save machinery.

The consequence of getting the vocabulary WRONG differs by direction,
and that asymmetry is the argument for erring wide:

- **Too wide** (includes something that does not change worlds): saves
  are purged that did not need to be. Loud, recoverable, annoying.
- **Too narrow** (omits something that does): the root matches, RTSim
  adopts a save whose world no longer exists, and the divergence appears
  later as unexplained NPC/site incoherence with the true cause several
  rows away. **Silent, and the exact failure `T4.3` was built to
  prevent.**

## 5. What this survey CANNOT settle (for the deriving builder)

Recorded as questions rather than guesses, because a survey that
answers these on vibes is worse than one that flags them.

1. **The `noise` crate's own version.** `SuperSimplex`, `Perlin` and
   `Fbm` come from a dependency. A change to their algorithm changes
   every world without touching `world/src` or `assets/`. Whether
   `Cargo.lock` is inside `T1.2`'s source closure decides if this is
   ALREADY-COVERED or a genuine vocabulary member — I did not verify
   the closure's file set, so I will not assert it.
2. **`Calendar`.** It reaches generation via `WorldOpts`, but whether it
   perturbs terrain/site output or only seasonal presentation needs a
   read of its use sites. Provisionally IRRELEVANT, explicitly unproven.
3. **Threadpool determinism.** Generation is threaded. `ARCH-003`
   already establishes that the deterministic-worldgen path seeds
   per-chunk from `(world seed, chunk pos)` precisely so it is
   call-order-independent — but whether ALL of generation has that
   property, or only the chunk-level dynamic RNG, is unestablished. If
   any generation stage is order-sensitive, thread count is a
   vocabulary member, which would be an unwelcome result worth knowing
   early.
4. **`LoadLegacy`/`Load` content identity.** If the vocabulary includes
   the `FileOpts` variant, a loaded map arguably needs the map FILE's
   digest in the root, not merely the fact that loading happened.
   That is a design call, not a survey finding.

## 6. Recommendation

Derive `WorldgenProtocolVersion` from a frozen vocabulary of the **§3
MUST-BE set** — which is `FileOpts`' variant, the five `GenOpts` fields,
`seed_elements`, and the seed — and record beside it that everything
else is covered by the source and content roots, naming which. That
keeps the vocabulary small enough to be reviewable, avoids duplicating
two roots that already exist, and leaves §5's four questions as named
gates rather than silent assumptions.
