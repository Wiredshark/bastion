# APEX-T6 — Cross-platform numeric evidence and selective kernels (fleet-authored spec v1)

Authored by Builder Opus 5 on `bastion/apex-t34` @ `fb0b00cd0e`, from the
master-order rows `APEX-T6.1`..`T6.5`, grounded in live code reads at that
tip. Symbols cited were read, not recalled.

**The tier's thesis.** T6 is where determinism stops being a property of
*our* code and starts being a property of *the machine our code runs on*.
Everything before this tier assumed that if two runs execute the same
operations in the same order they produce the same bits. That assumption
is false across targets the moment a `powf` result drives a branch. The
tier's job is therefore evidence first, kernels last — T6.5 is
explicitly `CONDITIONAL` because replacing a transcendental is only
justified once T6.1 has shown that a specific call site drives an
authoritative branch and T6.2 has shown it actually diverges.

**Read the ordering as a refusal.** The rows do not say "make floats
deterministic". They say: inventory the surface, prove divergence is
visible, freeze the *order* of contributions, pin the build tuple, and
only then — for named functions with measured input ranges — substitute a
certified kernel.

---

## Shared failure surface (verified)

**Authoritative transcendentals exist on live physics paths.** Read at
tip:

- `common/src/comp/fluid_dynamics.rs:201` — `1.78 * (1.0 - 0.045 * ar.powf(0.68)) - 0.64`
- `:210` — `planform_area * scale.powf(2.0) * (c_l * *lift_dir + c_d * *rel_flow_dir)`
- `:321` — `CD * (PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)`
- `common/src/states/utils.rs:1428`, `:1481` — `s.0.powf(13.0).powf(0.25)`
- `common/src/comp/buff.rs:500` — `f32::powf(1.0 - nn_scaling(data.strength), 1.1)`

These are not presentation paths: fluid dynamics feeds glider and
projectile motion, `states/utils` scaling feeds movement, and buff
strength feeds combat. `powf` is the exact function T6.5's acceptance
criterion names.

**Contribution ORDER is already partly frozen, and the remaining gap is
named in-tree.** `PhysicsData::construct_spatial_grid`
(`common/systems/src/phys/mod.rs:352-393`) inserts entities and then
calls `spatial_grid.canonicalize_cells(...)` keyed on stable `Uid`
(`:387-390`, comment `DET-PHY-005: canonical per-cell candidate order by
stable Uid`). So per-cell candidate order is deterministic today.
`apply_pushback` (`:395`) then runs under `ParMode::Rayon` with a
`par_join()`. That is the seam T6.3 addresses: candidates are ordered,
but the *application* of their contributions is parallel, so the
accumulation order — and therefore float summation order — is a function
of Rayon's partitioning.

Float addition is not associative. A parallel reduction over the same
multiset can produce different bits per partition layout. DET-PHY-005
fixed *which* candidates; T6.3 must fix *in what order their
contributions land*.

> **RETRACTED 2026-07-28. The paragraph above is wrong and is kept
> verbatim rather than quietly corrected.**
>
> There is no parallel reduction here. Read fully:
>
> 1. `par_join()` (`phys/mod.rs:420`) parallelises over **entities**, not
>    over contributions. Each task accumulates into a task-local
>    `vel_delta` (declared `:471`) and writes only its own entity's
>    velocity (`:655`). specs guarantees disjoint mutable component
>    access, so no accumulator is shared and no reduction crosses tasks.
>    Rayon chooses which *thread* handles which entity; it does not order
>    any accumulation.
> 2. Within a task the neighbour walk is deterministic:
>    `in_circle_aabr` → `in_aabr` (`util/spatial_grid.rs:56-76`) iterates
>    cells in a nested range order and fetches each cell with
>    `grid.get(&cell)` — a **lookup, not a hashbrown iteration**, so no
>    map ordering leaks — and each cell's contents were already
>    canonicalised to stable `Uid` order by DET-PHY-005.
> 3. The second grid query (`:1296`) is a `for_each` with no cross-entity
>    accumulation either.
>
> So DET-PHY-005 plus the deterministic traversal appear to have closed
> this seam completely, and the "half-fixed file" reading was a mis-read.
>
> **The method error, recorded because it is the transferable part:** the
> claim came from seeing `par_join` and `ParMode::Rayon` sitting next to
> a float accumulation. The shape looked like the bug. The correct
> question is what the parallelism is **over** — parallel-over-entities
> with task-local accumulation is safe; parallel-over-contributions is
> not. Proximity is not evidence.
>
> *Pending independent re-derivation* (assigned to the reviewer at the
> next boundary): the four sub-claims above are to be re-derived from
> code by a second reader before this correction is treated as settled.

---

## T6.1 — Numeric attack-surface inventory

**Objective.** Every branch-driving or persistent authoritative
transcendental has an owner and a protocol status.

**Selected architecture.** A source-scanned inventory in the shape T3.5's
bypass scanner already proved (`server/src/net_command_bypass.rs`):
classify every site, fail the build on an unclassified one. Classes:

- **finite ops** (add/mul/div) — recorded, not a hazard by themselves
  except through order (T6.3's domain)
- **reductions** — the order-sensitive class
- **normalization / root / power / noise / transcendentals** — the
  cross-target class
- **thresholds** — where a numeric difference becomes a *branch*, which is
  the only place a small difference becomes a large one

Mark each site: persistent or next-tick state? crosses a network, save or
hash boundary? Presentation-only exclusions require **evidence**, not
assertion — the T5.4 finding is the cautionary case, where a value that
looked presentational (`local_wind`) reached glider steering.

Record compiler and native-library implementation dependency per site,
because `powf` is libm's, not ours.

**Migration steps.** (1) Scanner + classification table. (2) Owner and
protocol status per site. (3) Source canary that fails on a *new*
authoritative transcendental call site, so the surface cannot grow
silently.

**Required tests.** Unclassified-site build failure; a presentation-only
exclusion without evidence fails; the scanner sees all five `powf` sites
listed above (a fixed lower bound, so a regexp regression is visible).

---

## T6.2 — Raw and semantic numeric probes

**Objective.** Hidden raw divergence is visible even when gameplay
tolerance masks it.

**Selected architecture.** Structurally the twin of T5.3, and it should
share its types rather than re-derive them:

- **Raw probe** — hash of canonical `to_bits()` values with a stable
  component and entity order. `to_bits()` because `==` on floats hides
  exactly the differences this probe exists to find (`-0.0`, NaN
  payloads).
- **Semantic probe** — a *separately reviewed* quantization policy:
  scale, rounding mode, saturation, and an explicit non-finite policy.
  "Separately reviewed" is a process requirement in the row and should
  stay one: the quantization is a gameplay-tolerance judgement, not a
  numerics detail.

Store both digests and the first differing field. **Do not feed semantic
probe values back into simulation** — the probe observes, it does not
participate. That prohibition is what keeps the probe from becoming the
thing it measures.

Exact component phase boundaries must be defined first: a probe taken at
a different phase is a different measurement, and comparing across phases
is a false divergence.

**Required tests.** The row's step 5 is the non-vacuity case and must be
built as a canary, not an aspiration: a fixture where the **semantic
probes match and the raw probes differ**. If that case cannot be
constructed, the two probes are not independent and the semantic one is
decorative.

---

## T6.3 — Ordered PHY-008 contributions

**Objective.** An identical candidate multiset yields an identical raw
contribution tape, *before* any numeric-kernel change.

**Verified failure surface — RECAST after the retraction above.** The
seam this row was written for appears already closed: per-cell candidate
order is canonical (`phys/mod.rs:387-390`) and the accumulation is
task-local under a parallelism that is over entities, not contributions.
The row is therefore **a pinning test, not a change**. That is not a
downgrade: nothing currently prevents a future edit from reopening the
seam silently, and the existing x2 harness never varies worker count, so
this is new coverage rather than duplication.

**Selected architecture.** (1) Stable body and pair identity — `Uid` is
already the tie-break key DET-PHY-005 uses, so extend rather than replace
it. (2) **Materialize** collision candidates before solving; a candidate
set that is discovered lazily during solving cannot be sorted. (3) Sort
by a frozen pair/contact key. (4) Fixed solver iteration count and fixed
contribution application order. (5) Remove semantic dependence on
spatial-grid traversal *and* on Rayon partitioning — parallelism may
compute contributions, but must not decide the order they are applied in.

The separation to hold onto: parallel *computation* is fine, parallel
*accumulation* is not. The standard shape is compute-in-parallel into an
indexed buffer, then reduce serially in key order.

**Required tests — now the row's whole content.** All insertion
permutations of a candidate set produce one tape; worker counts 1, 2, 8
and 48 produce one tape over the REAL path. Worker-count invariance is
the test that actually catches a Rayon dependence — permutation alone can
pass while partitioning still leaks — and it is precisely the axis the x2
harness does not vary.

**Canary sketch.** `PHY-008-001..` — candidate discovered mid-solve;
tie-break key absent for a pair; solver iteration count varying with
load; contribution applied in completion order; worker count changing the
tape.

---

## T6.4 — `NumericProfileV1`

**Objective.** A numeric profile is a precise tested tuple, not "uses
IEEE floats".

**Selected architecture.** Record: rustc and LLVM version, dependency
set, target triple, CPU and feature flags, profile, codegen flags, LTO
and codegen-units, native libraries, and rounding/subnormal assumptions.

Two prohibitions with teeth: **no `target-cpu=native`** (it makes the
binary's numerics a function of the build machine, which destroys the
tuple's meaning) and **no undeclared features**.

The row's step 4 is the honest part and must survive into the
implementation: **golden conformance vectors are the authority**. Do not
claim that stable Rust's flags enforce complete strict floating
semantics — they do not, and a profile that rests on that claim is
asserting something the toolchain does not promise. The vectors are what
is actually tested; the tuple is what is recorded.

Separate **target artifact reproducibility** (same inputs → same binary)
from **execution vector equality** (different binaries → same numeric
results). They are different properties and a profile that conflates them
will certify the wrong one. Bind the profile and its evidence into the
T4.1 bootstrap manifest and the T4.6 save manifest, as provenance for the
former and equality-critical for the numeric protocol.

**Required tests.** A profile differing in exactly one recorded field is
a different profile; `target-cpu=native` is rejected at build; the golden
vectors fail on a deliberately perturbed kernel.

---

## T6.5 — Selective deterministic transcendental kernels

**Objective.** Certified branch-driving `powf` no longer reaches the
path.

**Why this row is `CONDITIONAL`, restated so no one skips it.** A
replacement kernel is a gameplay change. It is justified only for call
sites T6.1 marked branch-driving or persistent AND T6.2 showed actually
diverge. Replacing the others is cost without evidence.

**Selected architecture.** Per selected function: collect the *actual*
input range and distribution from live play, not the mathematical domain
— a kernel certified over `[0, ∞)` when the game only ever passes
`[0.5, 4.0]` is a much harder problem solved for nothing. State the
required decision tolerance: how close to a threshold does the result
get, and how much error can it absorb before a branch flips.

Then choose **one** explicit algorithm — table, polynomial, rational, or
decimal — and freeze coefficients or table bytes, evaluation order,
rounding, saturation, and error bounds. Evaluation order is part of the
artifact, not an implementation detail: reassociation changes results.

Version weather humidity/rain, train friction, and economy price
functions **separately** (row step 5). They have different tolerances and
different review audiences; one shared version number would force them to
migrate together for no reason.

Migrate and tune only after gameplay review — the kernel changes what the
game does, so the review is a gameplay review, not a numerics one.

**Required tests.** Exhaustive vectors where the domain permits,
domain-edge vectors where it does not (zero, one, subnormal, the
saturation boundary, non-finite), compared across **all** supported
targets. The cross-target comparison is the whole point; a kernel tested
on one target has proven nothing this tier cares about.

---

## Cross-tier notes

**Ordering — REVISED with the retraction.** T6.1 is `READY after T0.5`
and starts immediately; its second half (`T6.1b`, per-site owner and
protocol status) is its own row. T6.3 follows T6.1b as a pinning-test
row. The original text called T6.3 "the highest-value row in the tier,
because it is a pure ordering fix" — that rested on the retracted claim,
and a row that pins an existing property is valuable but not that. T6.2 and T6.4 are
prerequisites for T6.5, which should be last and may end up scoped to two
or three call sites rather than all of them.

**Probe reuse.** T6.2's raw/semantic pair and T5.3's exact/semantic pair
are the same idea at different layers. They should share types, and the
structural-incomparability rule stated in the T5 spec applies here
unchanged: a semantic match must never be able to certify raw equality,
and that should be unrepresentable rather than documented.

**What this tier does not settle.** Whether the game *should* be
bit-identical across targets, or merely provably-divergent-in-known-ways.
That is a program-level decision. This tier makes either choice
defensible by producing the evidence; it deliberately does not make the
choice.
