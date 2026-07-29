# APEX-T4-PV — worldgen vocabulary: SURVEY (v1)

**Status: SURVEY, orchestrator-reviewed 2026-07-29 — ALL FOUR OPEN
QUESTIONS NOW CLOSED (Q1 answered, Q2 answered and it CORRECTED this
document, Q3 answered per-stage, Q4 ruled). Not a vocabulary, not a
version, and not approvable by its author.** `T4.3` ruled that `WorldgenProtocolVersion` must be a
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
| **`Calendar`** (`WorldOpts.calendar`) | `sim/mod.rs:462` → `block.rs:317,325` | **Moved here from IRRELEVANT by the Q2 read — my provisional verdict was wrong.** It reaches block generation through `column_gen.get((wpos, index, calendar))` and branches on `CalendarEvent::Christmas`/`Halloween` to emit DIFFERENT BLOCKS. That is generated chunk content, not seasonal presentation. Two servers generating the same seed on different calendar dates do not produce the same world. |
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
| `threadpool` / thread count | **IRRELEVANT, established per-stage in §5 Q3 rather than assumed** — no generation stage's output depends on the order its parallel work completes |
| ~~`Calendar`~~ | **WRONG — see §5 Q2. It is MUST-BE, and the provisional verdict was mine.** |

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
2. ~~**`Calendar`.**~~ **ANSWERED — MUST-BE, and my provisional verdict
   was WRONG.** It reaches block generation via
   `column_gen.get((wpos, index, calendar))` and branches on
   `CalendarEvent::Christmas` / `Halloween` (`block.rs:317,325`) to emit
   different blocks. That is generated content. Recorded as a
   correction rather than a silent table edit, because the provisional
   verdict is exactly the kind of thing a later reader would otherwise
   inherit as settled: **"provisionally IRRELEVANT, explicitly
   unproven" was doing real work — it is why this got read at all.**
3. ~~**Threadpool determinism.**~~ **ANSWERED — NO generation stage's
   output depends on the order its parallel work completes. Thread
   count is NOT a world-identity input, and the MUST-BE set is
   unchanged by this question.** Per-stage evidence, since the answer
   is only worth what the evidence is:

   - **`civ` placement and `site` generation are SEQUENTIAL.** There is
     not one parallel construct anywhere under `world/src/civ/` or
     `world/src/site/` except `economy/context.rs`. The stages most
     likely to be order-sensitive — placing settlements, generating
     plots — never split work at all, so the question cannot arise.
   - **`economy` is per-element.** `index.sites.par_iter_mut()
     .for_each(|(site_id, site)| site.economy_mut().tick(..))` gives
     each closure a mutable borrow of its OWN site and nothing else.
     The part that WOULD be order-sensitive — `INTER_SITE_TRADE`'s
     order distribution, which drains every site into one shared
     `index.trade.orders` — is deliberately sequential
     (`for .. in index.sites.iter_mut()`) right below it.
   - **`erosion` is 27 parallel constructs and the only stage with real
     reductions.** 24 are indexed element-wise (`map`+`collect`,
     `par_iter_mut().for_each`, `zip_eq`), where each output depends
     only on its own index. Three are reductions, and the split between
     them is the finding:
     - `sum_uplift` is a **parallel `f64` sum**, whose value genuinely
       depends on rayon's reduction tree and therefore on thread count
       — and it is consumed **only** by `debug!("Sum uplifts: ..")`.
       Log-only; it reaches nothing.
     - `max_uplift` and `max_g` **do** feed generation (`dt`, `tol2`,
       and two `max_g` branches). They are parallel `max_by`, and
       **max is associative and commutative**, so the result is
       order-invariant by the operation's algebra rather than by luck.
       `partial_cmp().unwrap()` panics on NaN rather than silently
       varying, which is the right failure.
   - **`sim/mod.rs`, `sim/util.rs`, `sim/diffusion.rs`** are indexed
     `map`/`collect`/`for_each`; `lib.rs`'s two `threadpool.install()`
     calls only scope work onto the pool.

   **MEASURED 2026-07-29 (`T4-PV-EXP`) — THE EXPERIMENT AGREES WITH
   THE SURVEY.** `world/examples/t4pv_thread_count_experiment.rs`
   generates seed 1337 at 1, 2 and 8 threads and prints the canonical
   map-geometry root. All three are bit-identical:

   ```
   threads=1  f85c6ac2f01e0b6acbb9e6545203d2fb46ccd33c34abc8a51b89f73fcc51bc89
   threads=2  f85c6ac2f01e0b6acbb9e6545203d2fb46ccd33c34abc8a51b89f73fcc51bc89
   threads=8  f85c6ac2f01e0b6acbb9e6545203d2fb46ccd33c34abc8a51b89f73fcc51bc89
   ```

   What the measurement does and does not cover, so nobody inherits
   more than was measured:
   - It compares `world_map_geometry_root_v1` — `T4.3`'s OWN identity
     for a generated map, reused rather than a comparison invented for
     the occasion, so a difference here is one the rest of the program
     would also see. It covers map geometry and the site/POI listing.
     Per-chunk block generation sits downstream of it and is separately
     seeded per chunk by `ARCH-003`.
   - The world is GENERATED, not loaded: `FileOpts::LoadAsset` (what
     the default server and the existing timing example use) would read
     a prebuilt map off disk and measure nothing about generation.
   - One seed, three thread counts, one run each, at a 9/9 map rather
     than the 10/10 default. The size is a cost control, disclosed:
     order-sensitivity in a stage would show at any size, and all four
     stages under test still run. Single-threaded 9/9 generation is
     slow enough that a 10-minute wrapper cut the first sequence short,
     which is worth knowing for anyone repeating this at full size.

   **Bound, stated plainly: the survey was a reading, not an
   experiment.** It establishes that no stage's SHAPE admits
   order-dependence, which is a stronger claim than a passing test but
   a different one from a measurement. The confirming experiment is
   cheap and someone should still run it — generate one seed at
   several thread counts and compare the resulting map bytes — and if
   it ever disagrees with this survey, the survey is what is wrong.
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

---

## 7. PREMISE-CHECK FINDING (derivation row, 2026-07-29) — a fork above the builder

The derivation row was handed over with the vocabulary settled. The
premise-check found a hole in the row's own premise, before any code:

**`T4.3` ruled the derivation must be a "frozen-vocabulary content-root
derivation per `net_envelope_profile_root_v1`'s pattern, never arbitrary
integers". That precedent returns a `DigestBytes32V1` — 32 bytes. But
`WorldgenProtocolVersion` is `ProtocolVersion(u32)`
(`apex/scalar.rs:203`), and the baseline preimage absorbs it as a u32:**

```
push_option_u32(&mut buf, input.worldgen.map(|w| w.get().get()));
```
(`world_baseline.rs:118`, and the same for `content` and `numeric`.)

So a faithful vocabulary digest cannot reach the baseline root intact.
Narrowing 256 bits to 32 to fit the existing field would mean two
DIFFERENT worldgen vocabularies that collide in the truncated 32 bits
produce an IDENTICAL baseline root — a save adopted against a world that
no longer exists. That is precisely the **too-narrow, silent** failure
direction §4 argued to err against, and the one `T4.3` exists to
prevent. A 1-in-4-billion collision is not a hazard worth accepting
merely to avoid touching a type, because the whole point of the root is
that it is not allowed to be wrong quietly.

Worth noting the preimage builder's own doc, two lines above: "every
field length-prefixed or fixed-width so no two distinct inputs can ever
produce the same bytes". The encoding was built to make collisions
unrepresentable; feeding it a truncated digest would reintroduce at the
input what the encoding removed at the format.

**The three options, for the orchestrator:**

1. **Truncate to u32.** No type change, cheapest, reintroduces a silent
   collision path. Not recommended.
2. **Widen the three fields to carry the full digest**
   (`DigestBytes32V1`/`ProtocolDigestV1`). Faithful to the ruling and to
   the precedent. Changes an approved `T4.3` type and its manifest
   encoding, and touches `T4.1`'s Content slot — which is the same
   wiring the derivation row already has to do, so the cost may be
   smaller than it looks.
3. **Keep the u32 as a coarse schema version AND add a digest field.**
   Two fields, one meaning each; no lossy narrowing and no re-meaning of
   an existing field, at the cost of more surface.

**Recommendation: (2).** The `u32` has no independent meaning today —
every construction site in the tree passes a hand-written `1` or `2` in
tests. It was a placeholder for exactly the derivation that did not
exist yet, so widening it re-uses a slot rather than repurposing a
meaning. But this changes an approved boundary's shape, which is not a
builder's call to make mid-implementation — the same reason `T4.5`'s
resolution policies were carried as stated questions rather than
answered.
