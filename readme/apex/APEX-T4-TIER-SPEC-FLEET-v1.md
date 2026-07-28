# APEX-T4 — Bootstrap, world baseline, and durable save epochs (fleet-authored spec v1)

Authored by Builder Opus 5 on `bastion/apex-t34` @ `bd66209b97`, from the
master-order rows `APEX-T4.1`..`T4.6`, grounded in live code reads at that
tip. Every symbol cited below was read, not recalled; where a claim needed
a compiler I say so instead of asserting it.

**Standing constraint that shapes all six rows:** this tier is where APEX
stops describing *messages in flight* and starts describing *state at
rest*. T3 could fail closed by refusing a frame. T4 cannot — a save
already exists on disk, written by a build that never heard of any of
this. Every row below is therefore written so that adoption cannot make
an existing save worse than leaving it alone.

---

## Shared failure surface (verified)

Three independent stores, three independent write paths, no common epoch:

| Store | Path | Writer | Atomicity today |
|---|---|---|---|
| Character/player SQL | `<db_dir>/db.sqlite` (`server/src/persistence/mod.rs:256`) | `VelorenConnection`, refinery migrations (`mod.rs:53-54`, `embed_migrations!("./src/migrations")`) | Per-transaction |
| RTSim world state | `<data_dir>/rtsim/data.dat` (`RtSim::get_file_path`, `server/src/rtsim/mod.rs:154-162`) | `RtSim::save` → `save_thread` (`mod.rs:574`, `:630`) via `AtomicFile` / `OverwriteBehavior::AllowOverwrite` | Per-file |
| Terrain / map | server-owned chunk persistence (`server/src/lib.rs`) | tick-driven | Per-artifact |

Each store is individually careful and collectively unguarded. `AtomicFile`
guarantees the rtsim blob is never *torn*; nothing guarantees it describes
the same simulation tick as `db.sqlite`. A crash between the two writes
leaves a pair of internally-valid files that never coexisted in a running
world, and no reader can currently detect that.

RTSim already carries a version (`rtsim/src/data/mod.rs:39`,
`CURRENT_VERSION: u32 = 10`, with `version` defaulting to 0 when the field
is absent) — so the *store* has a schema epoch while the *save* has none.
That asymmetry is the tier's root problem, not a detail.

On the wire, `ServerInit::GameSync` (`common/net/src/msg/server.rs:80`)
ships the entire bootstrap payload — `entity_package`, `world_map`,
`recipe_book`, `component_recipe_book`, `material_stats`, `ability_map`,
`server_constants` — in one message. T3.1.11 already added
`server_boot_id` to it so a client can reject state mixed across a restart
(`server.rs:80-84`), and T3.2 added the binding echo shared with
`RegisterAnswer` (`server.rs:115-132`). Those are the two anchors T4.1 and
T4.2 extend; there is no compatibility negotiation before the bulk state.

---

## T4.1 — `BootstrapManifestV1`

**Objective.** A client must be able to refuse a server's bootstrap
*before* applying bulk `GameSync` state, on the basis of a total,
classified compatibility report rather than a version string.

**Verified failure surface.** `GameSync` is applied wholesale; the client
constructs `State::client` from it with no prior agreement on wire,
content, plugin, schedule or numeric protocol. `ServerInfo` carries
human-facing fields (`server.rs:74`, `rules: Option<String>`), not
machine-checkable identity.

**Selected architecture.** Reuse T0.5's `SubsystemDescriptorV1` vocabulary
— do *not* invent a parallel one, and do *not* reuse save-lifecycle
fields (row step 1; the save side is T4.6 and its epochs must not leak
into a live handshake). Each descriptor carries one of three
classifications, as a typed enum, never a bool pair:

- **EqualityCritical** — wire/command/entity schema, content identity,
  plugin activation set, schedule identity, numeric/prediction protocol,
  world identity. Any difference is a refusal.
- **Negotiated** — compression, terrain/snapshot encoding, optional
  extensions. Server selects from the client-supported set under a
  *versioned selection algorithm* whose identity is itself
  equality-critical (otherwise two servers can "negotiate" differently
  from the same inputs and both be right).
- **ProvenanceOnly** — role-specific executable/build/OS/target identity.
  Recorded, never compared, unless policy independently denies.

The classification is what makes the row's acceptance criterion reachable:
a Linux server and a Windows client must interoperate, so build identity
cannot be equality-critical, while content identity must be.

**Migration steps.** (1) Define the manifest and its classification enum.
(2) Emit it server-side immediately before `GameSync`, as its own message.
(3) Client validates and produces a *total* mismatch report — every
descriptor, not the first failure — then fails closed. (4) Only then apply
`GameSync`. Order matters: a manifest that arrives with or after the bulk
state cannot prevent its application.

**Required tests.** Total-report completeness (a fixture with three
simultaneous mismatches reports all three); each classification's
behaviour separately; a negotiated-set intersection that is empty fails
closed rather than defaulting; a provenance-only difference does *not*
refuse. Mutation test: flipping one descriptor's classification must
change the verdict, or the classification is decorative.

**Canary sketch.** `BOOT-001..` — mismatch in each equality-critical
descriptor; negotiated set empty; negotiated selection algorithm version
differs; provenance-only difference accepted; manifest after `GameSync`;
manifest absent entirely; report truncated at first mismatch.

---

## T4.2 — Bootstrap freshness

**Objective.** An authentic but *stale* bootstrap must not be applicable.

**Verified failure surface.** `GameSync`'s `server_boot_id`
(`server.rs:80-84`) rejects cross-restart mixing, which is one axis.
Nothing binds a bootstrap to a *sequence* within one boot, so a replayed
authentic manifest from earlier in the same boot is indistinguishable
from a current one.

**Selected architecture.** Bind the manifest to the full tuple already
available from T0.4/T3.2 — `ServerBootId`, `SessionId`, `ConnectionEpoch`
— plus a bootstrap *sequence* monotone within the boot, the snapshot
epoch, and the predecessor root. This is deliberately the same shape as
T3.5's command journal: a monotone sequence with a floor, because the
failure mode is identical (an authentic-but-superseded artifact
replaying). Bind the live handshake to a fresh nonce or an authenticated
transcript so possession of a recorded manifest is not sufficient.

Reject: lower or replayed sequence for the same boot; predecessor-chain
fork unless a full bootstrap reset is *declared* (a fork that arrives
undeclared is the attack, a declared one is a legitimate reset).

**Migration steps.** (1) Extend the manifest with the freshness tuple.
(2) Add the nonce/transcript binding to the handshake. (3) Add the
sequence floor server-side. (4) Add a cache-expiration policy **only if**
manifests are allowed to outlive one live handshake — if they are not,
say so and skip it rather than writing an unused policy.

**Required tests.** Rollback (replay an earlier valid manifest), freeze
(replay the current one after the epoch advances), mix-and-match roots
(valid manifest, foreign predecessor). Each must fail with a distinct
typed terminal — collapsing them into one "invalid" loses the diagnosis.

---

## T4.3 — `WorldBaselineManifestV1`

**Objective.** RTSim migration must never silently interpret an old world
against a worldgen baseline it does not recognise.

**Verified failure surface.** RTSim data carries `version`
(`rtsim/src/data/mod.rs:39-45`) and reconciles against whatever world the
current binary generates. The world seed alone does not pin the *result*:
identical seed with altered worldgen, content, or economy math produces a
different world that RTSim will nonetheless reconcile against.

**Selected architecture.** Emit after world generation, before RTSim
reconciliation — that ordering is the whole mechanism. Bind world seed
plus worldgen/content/numeric protocol identity, and hash: canonical map
geometry, site identity, the site origin/kind graph, and the economic
baseline. Record one complete root into the save inventory (T4.4) and the
later save manifest (T4.6), so the baseline that a save was written
against is recoverable without re-running worldgen.

On mismatch, produce an explicit migration/incompatibility terminal
*before* destructive reconciliation. "Before" is load-bearing: RTSim
reconciliation mutates, so a post-hoc check reports damage rather than
preventing it.

**Required tests.** Same seed with each of: altered worldgen, altered
content, altered economy math, and permuted site ordering. The last is
the non-vacuity case — site ordering must not change the root, or the
hash is over an iteration order rather than over the world.

---

## T4.4 — Non-authoritative existing-save inventory

**Objective.** Diagnose what is on disk without certifying it, and without
writing to it.

**Verified failure surface.** The three stores above have no shared
descriptor. There is no artifact today that answers "what is in this save
directory" without opening each store with the current binary's
assumptions.

**Selected architecture.** A read-only sidecar enumerating SQLite, RTSim,
terrain, map, and replay/evidence artifacts. For each: existing
schema/version as *found* (RTSim's `version` field, SQLite's refinery
state), bytes digest via T0.3's `hash_artifact_bytes_v1`, observed
metadata, and typed missing/corrupt states.

Consistency is recorded as `Unverified` and **no common checkpoint or
tick is inferred** — this is the row's real discipline. The inventory's
value is that it refuses to synthesise a coherence that the stores do not
actually have; T4.6 is what creates that coherence, and until it exists
an inventory that claimed it would be lying.

Original bytes preserved; no writes on any path, including no "repair on
read".

**Migration steps.** (1) Enumerate + digest. (2) Typed
unsupported/corrupt/missing results. (3) Scan historical saves into a
corpus index, which is T4.5's input.

**Required tests.** Missing store; truncated store; a store from a future
version; a directory with two RTSim files; a save whose SQLite and RTSim
disagree about anything — all must produce a report, none may produce a
verdict. Falsifier: a fixture where the stores *are* coherent must still
report `Unverified`, or the field means nothing.

**BUILT — `server/src/save_inventory.rs`, 13 tests.** All five required
cases plus the falsifier, plus three the build showed were needed.

The falsifier is not a test at all in the end: `SaveConsistencyV1` has
**one variant**. A `Coherent` variant would be produced by a
coherent-LOOKING fixture and then read as "the stores agree", which
nothing here checks — so reporting anything else is unrepresentable
rather than merely tested. A test pins the arity with T4.6 named as what
would have to exist first.

Three things the build found:

1. **`Data::from_reader` cannot be used by an inventory.** It rejects
   anything that is not `CURRENT_VERSION`, so it cannot distinguish a
   version-17 save from garbage — precisely the case a diagnosis exists
   for. `rtsim::data::Data::probe_version_v1` was added where the format
   lives: it decodes a one-field struct, so serde skips the rest and a
   future version REPORTS ITS NUMBER. The test asserts the fixture is
   genuinely unloadable, so it cannot rot into proving nothing.
2. **"Read-only" is a claim about the filesystem, not about intent.** A
   WAL-mode SQLite file opened `mode=ro` still makes SQLite create a
   `-shm` sidecar in the directory being diagnosed. The connection uses
   `mode=ro&immutable=1`, and the no-write test digests the whole tree
   either side of the call. Removing `immutable=1` turns that test red —
   verified, so the flag is load-bearing rather than decorative.
3. **The tier spec named stores this build does not persist** (map,
   replay/evidence). They are recorded in `NOT_PERSISTED_BY_THIS_BUILD`
   with the evidence for each, not silently omitted: an inventory that
   dropped them would read as "there are none" and the next reader would
   re-derive it from the whole server.

`missing` deliberately covers only the character db and rtsim data. A
save with no rtsim backup is a save that never failed, and a world with
no edited chunks has nothing to persist — reporting those absent would
manufacture findings.

`corpus_index_v1` is `T4.5`'s input: the sorted multiset of content
identities. Byte equality is the only equality this row is entitled to
assert.

---

## T4.5 — Historical save corpus and migration policy

**Objective.** Existing saves must not become collateral damage of
manifest adoption.

**Selected architecture.** Fixtures from every supported
schema/worldgen/content epoch, and a four-way declared state:
**Supported**, **Migratable**, **ExplicitRecoveryOnly**, **Unsupported**.
Ordered pure migration steps, each carrying its code digest, with
deterministic identity — the same discipline T2.4's plugin DAG uses, for
the same reason.

**Direct-to-latest must equal stepwise** where both are defined. That
equality is the test that makes a migration graph trustworthy; without it
the two paths are two implementations of one policy.

Tombstone/alias/content/world resolution policies are declared *before*
code (row step 5), because each is a judgement call about player data
that a builder should not be making mid-implementation.

**Do not mandate save manifests until fixtures and offline recovery
exist** (row step 7). This is the row's sequencing rule and it constrains
T4.6: the durable-epoch work may land, but making it *required* is gated
on this row.

**Required tests.** Corruption, future-version, and missing-store
fixtures; direct-vs-stepwise equality on every migratable epoch pair.

**BUILT — `server/src/save_migration.rs`, 10 tests.** With a correction
to this row's premise, established by reading the loader rather than
assuming the row's shape:

**There is no rtsim migration machinery in this build, so the honest
graph is EMPTY.** `Data::from_reader` rejects any version that is not
`CURRENT_VERSION`, and `server/src/rtsim/mod.rs` responds by PURGING and
regenerating — unless the operator sets `RTSIM_IGNORE_VERSION`, which
loads the mismatched data unmigrated. So every non-current rtsim version
is `ExplicitRecoveryOnly`: a path exists and it is not automatic. Not
`Unsupported` (there IS a recovery), not `Migratable` (nothing transforms
the data; serde defaults absorb the difference, which is a very different
promise). Constructing an elaborate graph over steps that do not exist
would have been theatre — what is built is the ENGINE and its law, so the
first real step is born already bound by them. A test fails the moment an
rtsim step appears, forcing the support policy to be re-derived rather
than left stale.

**Also corrected:** the `.ron_backup` rename that `T4.4` inventories
fires on a DECODE failure, not on a version mismatch. Those are different
paths and I had them conflated.

The direct-equals-stepwise law is enforced by the graph and demonstrated
both ways: a consistent graph passes, and a graph whose direct edge has
drifted from its stepwise path is caught by name. Stepwise walking breaks
ties by smallest `to` then name, so migration is a function of the graph
and not of the order steps were written down. Non-advancing steps are
refused at construction, which is what makes the walk unable to fail to
terminate. `NoPathFrom` and `NoPathTo` are distinguished because they are
different problems for whoever has to fix the save.

The per-step digest is called `behaviour_fingerprint_v1` and NOT a code
digest, because it is not one: it digests what the steps do to a probe
corpus. It catches a step whose behaviour moved and misses a change that
is a no-op on every probe. A true per-function code digest needs `T1.2`'s
source closure at function granularity, which does not exist.

**Step 5 is deliberately unanswered.** The tombstone, alias, content and
world-resolution policies are carried in `RESOLUTION_POLICIES` as stated
QUESTIONS, all `PendingRuling`. This row says they are declared before
code and are not a builder's call; a test fails if any becomes
`Declared`, so a policy about player data cannot change without somebody
noticing. **Step 7's sequencing rule is a value** —
`SAVE_MANIFEST_MANDATE_READY = false` — so `T4.6` cannot quietly assume
it has been satisfied.

---

## T4.6 — Multi-store staged save epochs

**Objective.** A manifest cannot exist without its complete state, and
complete state is not active without its durable commit pointer.

**Verified failure surface.** The sharpest one in the tier.
`RtSim::save` (`server/src/rtsim/mod.rs:574`) clones `Data` and hands it
to a dedicated thread (`:630`) which writes through `AtomicFile` with
`OverwriteBehavior::AllowOverwrite`. SQLite commits on its own schedule
through `VelorenConnection`. Neither knows the other's tick. `AtomicFile`
prevents a *torn file*; nothing prevents a *torn save*. A crash between
the two produces two internally-valid stores that never coexisted, and
`OverwriteBehavior::AllowOverwrite` means the previous good rtsim blob is
already gone.

**Selected architecture.** Freeze one canonical simulation tick and save
epoch. Snapshot each store into an immutable *staged* payload. Flush and
verify each payload's digest. Write and flush `SaveUniverseManifestV1`
binding: every store's payload digest, world/content/build/numeric/
schedule identity, the frozen tick, the parent epoch, and the migration
journal. Then atomically publish **one** current-epoch pointer — that
rename is the commit point, and it is the only step whose completion
means the save exists.

Recovery accepts only a valid published pointer whose payloads *all*
match their digests. Garbage-collect old and staged epochs only after
policy-safe acknowledgment, never as part of the commit.

**Migration note.** This changes the rtsim write from overwrite-in-place
to staged-plus-pointer. That is the migration risk of the row, and it is
why T4.5's corpus is its prerequisite: the first boot after adoption must
be able to read an old, pointer-less save directory and treat it as epoch
zero rather than as a corrupt one.

**Required tests.** Crash injection at *every* write, flush, rename, and
pointer step, across supported filesystems. The acceptance criterion is
directional and both directions need a test: a manifest without complete
state must be unreadable, and complete state without a pointer must be
inactive.

**Canary sketch.** `SAVE-001..` — crash before staging; mid-payload;
after payload before manifest; after manifest before pointer; during
pointer rename; pointer to a missing payload; pointer to a digest
mismatch; two pointers; stale staged epochs present; GC racing a reader;
old pointer-less save directory on first boot.

---

## Cross-tier notes

**What T3 already gives this tier.** The identity substrate is done and
should not be re-invented: T0.3 digests and domain separation, T0.4
opaque ids, T0.5 subsystem descriptors, T3.2's session binding, T3.4's
checkpoint epochs, and T3.5's monotone-sequence-with-floor pattern, which
T4.2 should copy rather than re-derive.

**Sequencing.** T4.4 is `READY after T0.5` per its own row and is the
only one of the six that can start immediately; the other five are
`SPEC-COMPLETE / PREREQUISITE-MISSING`. The natural order is
T4.4 → T4.5 → T4.6 (save side, each feeding the next) with
T4.1 → T4.2 → T4.3 (bootstrap side) independently, joining where T4.3's
baseline root is recorded into T4.4's inventory and T4.6's manifest.

**What this spec does not do.** It selects architecture and names the
tests; it does not pin numeric limits, retention windows, or filesystem
support matrices. Those are deployment values, and this program's
standing rule is that no production value is invented by the builder who
happens to reach the row first.
