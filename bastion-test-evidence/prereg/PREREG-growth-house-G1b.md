# PREREG — G1b: the colony grows its own town's site in place

Registered 2026-09-02 14:50, before the binary exists. Follows G1a
(`world/src/site/bastion_layout.rs`, reviewed; commits behind W4).

## The borrow, and the decision

`layout_plot_for_colony` needs `&mut Site` and an `IndexRef`. On the
server the Site lives in `IndexOwned { index: Arc<Index> }`; the chunk
generator holds `IndexOwned` clones while jobs are in flight
(server/src/chunk_generator.rs:63). Site is not Clone and PlotKind has
47 variants, so a deep-cloned shadow site is a wide vanilla change and
a rebuilt shadow would have to reproduce plazas and roads for
`find_roadside_aabr` (which dereferences both) — a silent-drift risk.

Decision: MUTATE THE INDEX'S OWN SITE IN PLACE when the Arc is unshared
(`Arc::get_mut`), REFUSE when it is shared (a chunk job is reading it;
try next tick). Split the layout into a placement half (`&mut Site`, no
index) and a render half (`&Site` + `IndexRef`) so the borrows never
overlap. Consequences, stated:

- Chunk generation renders the grown plot natively from then on (the
  Arc's contents changed in place; every later clone sees the plot). A
  house is ALSO delivered as a block list for build-over-time (G1c); a
  field has no block list and is rendered by regenerating its chunks
  (G2, via the `reload_chunks_inner` path, server/src/cmd.rs:4762).
- The tick at which a plot is laid out varies with chunk traffic; the
  plot itself does not (one ChaChaRng from the seed on the site's state;
  pinned in G1a).
- Persistence is a growth log of (site, kind, seed) replayed in order at
  boot — determinism by construction, not a saved Site (G1d).
- One vanilla addition: `IndexOwned::try_index_mut(&mut self) ->
  Option<&mut Index>` (a method; no behaviour changes for vanilla).

## Pre-registered pins (must be red when the defect is planted)

- a shared index is refused before anything is spent (a held clone ->
  `IndexShared { strong_count: 2 }`, plot count unchanged).
- an unshared index grows a house in place (plot count +1 read back
  through `as_index_ref()`, blocks >= 100, beds >= 1, door Some).
- no room leaves the site untouched.
- the split layout equals the composed layout (same seed, same town ->
  identical blocks, beds, door, aabr).

## Witnesses

PLOT LAID OUT (kind, plot, aabr, door, beds, blocks, seed) and PLOT
LAYOUT REFUSED (the refusal). The refusal witness is the failure face:
if IndexShared fires every tick on a live server, the design's premise
(quiet ticks exist) is false and the fallback is a colony-owned shadow.

## NOT in G1b

Job-board wiring, block placement over time, the housing verdict's call
(G1c); persistence/replay (G1d); fields (G2); workshops/mines (G3).
