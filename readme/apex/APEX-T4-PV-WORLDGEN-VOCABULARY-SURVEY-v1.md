# APEX-T4-PV — worldgen vocabulary: SURVEY (v1)

**Status: SURVEY, orchestrator-reviewed 2026-07-29 — Q1 answered, Q4
ruled, Q2/Q3 open (Q3 promoted to its own survey). Not a vocabulary, not
a version, and not approvable by its author.** `T4.3` ruled that `WorldgenProtocolVersion` must be a
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
| **The loaded map file's CONTENT DIGEST** (when a load variant is in play) | the file `FileOpts::Load`/`LoadLegacy`/`LoadOrGenerate` reads | **Added by orchestrator ruling** (§5 Q4). "A map was loaded" is not an identity; the map's bytes are. Without this the vocabulary records that the seed→world derivation was broken without recording what replaced it. |

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

1. ~~**The `noise` crate's own version.**~~ **ANSWERED — ALREADY-COVERED,
   and pinned twice over.** `SuperSimplex`, `Perlin` and `Fbm` come from
   a dependency, so an algorithm change alters every world without
   touching `world/src` or `assets/`. `Cargo.lock` is inside `T1.2`'s
   closure by two independent mechanisms
   (`bastion-harness/src/bin/apex_source_closure.rs`): it is walked by
   `git ls-tree -r --full-tree` like every tracked file, AND it is a
   named fixed pin whose bytes are retained and hashed into its own
   `ArtifactIdentityV1` field (`cargo_lock: pin_artifact(..)`, line
   651). A `noise` bump moves both.

   Worth carrying into the derivation for a reason beyond this
   question: the same fixed-pin list covers **`rust-toolchain`**,
   `.cargo/config.toml`, `flake.nix`, `flake.lock`, every `build.rs`
   and every `Cargo.toml`. The COMPILER is pinned — which matters here
   specifically, because worldgen is float-heavy and codegen differences
   across rustc versions are exactly the kind of thing that would
   change generated terrain while every source file stayed byte-
   identical. That hazard is covered; it did not need to be, and it is
   worth knowing that it is.
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
4. ~~**`LoadLegacy`/`Load` content identity.**~~ **RULED by the
   orchestrator, 2026-07-29: YES — the loaded map's CONTENT DIGEST
   belongs in the root, not merely the fact that loading happened.**

   The reasoning, recorded because a ruling without it is an
   instruction nobody can re-derive: for the same reason the manifest
   digests payload bytes rather than trusting the writer, **"a map was
   loaded" is not an identity — the map's bytes are.** Two servers that
   both report "loaded a map" have agreed on nothing. This is the
   `FileOpts` finding taken to its conclusion: if the variant is in the
   vocabulary because loading breaks the seed→world derivation, then
   what was loaded has to be in it too, or the vocabulary records that
   the derivation was broken without recording what replaced it.

## 6. Recommendation

Derive `WorldgenProtocolVersion` from a frozen vocabulary of the **§3
MUST-BE set** — `FileOpts`' variant plus the loaded map's content
digest, the five `GenOpts` fields, `seed_elements`, and the seed — and
record beside it that everything else is covered by the source and
content roots, naming which. That keeps the vocabulary small enough to
be reviewable and avoids duplicating two roots that already exist.

**Gate status after the orchestrator's pass (2026-07-29).** Q1 answered:
the dependency pin AND the compiler pin are already covered, so nothing
is added on that account. Q4 ruled: the map digest is IN. **Q2 and Q3
remain open, and Q3 gates CONFIDENCE rather than derivation** — a
builder can derive the version today, but if any generation stage turns
out to be call-order-sensitive then thread count is a world-identity
input, which is a finding well beyond this row. Q3 is now its own
survey; Q2 (`Calendar`) rides that read, because both are answered by
the same walk of what generation stages actually consume.
