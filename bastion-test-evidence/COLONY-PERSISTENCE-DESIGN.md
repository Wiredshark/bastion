# COLONY-STATE PERSISTENCE — **DESIGN, SETTLED**

Companion to `COLONY-PERSISTENCE-PREREG.md` (`e1120a9284`). Read-only analysis; no code
changed. Everything below is a **read**, cited at file::symbol.

## 1 · FEASIBILITY — **no migration blocker**

`rtsim/src/data/mod.rs`:

```rust
/// Note that this number does *not* need incrementing on every change: most
/// field removals/additions are fine. This number should only be incremented
/// when we wish to perform a *hard purge* of rtsim data.
pub const CURRENT_VERSION: u32 = 10;
```

and the file already carries **three** sibling fields added with `#[serde(default)]`,
each annotated *"no version bump"*. So a `#[serde(default)] pub bastion_designations`
field is the **established pattern in this very file**, not a new precedent.

Prereg §5.5 said I would stop and report a blocker if the format could not carry
designations. **It can.** That refusal is discharged, not invoked.

## 2 · THE PAYLOAD IS ALREADY SERIALIZABLE

`common::bastion::Region` and `common::bastion::DesignationKind` both derive
`Serialize, Deserialize` — they cross the client wire already. **No new serde work.**

## 3 · THE RESTORE PATH — replay the ORDERS through the real function

`JobBoard::place_designation(&mut self, terrain, region, kind) -> Vec<JobId>` is the
**single entry point**: it records the region, registers stockpile/zone footprints, and
creates jobs.

> **Persist the orders; replay them through `place_designation`.**

This is the whole design, and it is why prereg **P2 (work returns, not just data)** comes
out of the shipping path rather than from parallel restore code. A hand-written restore
that rebuilt jobs itself would be the F8 defect — a second implementation of the thing
under test.

## 4 · ⚠ THE CONSTRAINT THAT SHAPES THE ROW — terrain is not loaded at start

`place_designation` takes `&TerrainGrid`. **At server start no chunks are loaded**, so
the replay cannot run at load time. It must be **deferred** until the chunks covering
each region exist — which is, notably, the same condition the original founding already
lives under.

This is the row's real work, and it is a scheduling problem, not a serialization one.
Naming it here so it is not discovered halfway through an implementation.

## 5 · ★ THE STORE DECISION — one structure, not two

The obvious cheap move is a parallel `Vec<(Region, DesignationKind)>` log beside the
existing `designated: Vec<Region>`. **Rejected**, and for a concrete reason rather than
taste:

`JobBoard::cancel_region` removes designations by **intersection**, not exact match:

```rust
self.stockpiles.retain(|(_, r)| !r.intersects(&region));
```

A parallel log would have to replicate that predicate, in a second place, forever. Two
structures that must agree is the drift risk this session has already been bitten by
twice (the verb table that could under-declare silently; the relief emit that measured
`submerged` while nothing consumed it).

**The drift-free design:** give the existing store its kind —
`designated: Vec<Region>` → `Vec<(Region, DesignationKind)>`. Then cancellation's
`retain` keeps working unchanged, persistence reads the store directly, and there is
**one producer**.

### Blast radius, measured

**19 call sites** — 18 in `bastion-server/src/bastion_jobs.rs`, 1 in
`server/src/sys/msg/in_game.rs`. Shapes: 4 × `push`, 3 × `iter`, 3 × `clone`, 1 × `len`.
The three `clone()`s mean at least one downstream signature carrying `Vec<Region>` moves
with it.

Tractable and mechanical — but it is a single coherent change that must land whole. It is
**not** a change to start with a shallow budget and abandon half-applied.

## 6 · THE STAGES, IN ORDER

1. **`designated` carries its kind** (19 sites) + a test that cancel-by-intersection
   still removes both region and kind together.
2. **rtsim `Data` gains `#[serde(default)] bastion_designations`**; save copies from the
   board.
3. **Deferred replay** — a pending queue drained as chunks load, calling
   `place_designation`. This is where P2 is won.
4. Score P1–P4 against the prereg, with the **60 s save precondition asserted by
   content** (prereg §4 P1, and this session's own thrice-learned law).

## 7 · WHAT IS ALREADY TRUE AND NEEDS NO WORK

- The gap is **measured** (3 plots → 0 across a restart, same userdata).
- It is **Ben-observed live** — *"colonists came back, the zones did not"*.
- The one-colony predicate already reads rtsim records and is **unaffected** by any of
  this (prereg P4 guards it).
