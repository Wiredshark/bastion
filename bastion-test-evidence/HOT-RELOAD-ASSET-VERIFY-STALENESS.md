# `bastion-harness` doesn't hot-reload assets — and if it ever does, the asset-root gate needs fixing first

Standalone note, per Opus's instruction (AUTON-2 Step 1, 2026-08-08): this
outlives the row it was found in. Two facts, neither discoverable without
actually reading the feature-forwarding chain and the gate's call site.

## Fact 1 — `bastion-harness` does not compile `hot-reloading`; assets are frozen for the process

`assets_manager`'s hot-reload watcher (`HotReloader::start`) is compiled in
only under the `hot-reloading` cargo feature, forwarded through three crates:
`common-assets/Cargo.toml` → `common/Cargo.toml` → `server/Cargo.toml`.
`bastion-harness/Cargo.toml`'s own dependency declarations for `server` and
`common` enable **neither**. So for the whole life of a `bastion-harness`
process, every asset (e.g. `assets/common/bastion_mood.ron`, loaded via
`MoodConfig::current()`) is read exactly once and cached — rewriting the file
under a running scenario changes nothing, immediately or after any delay
(confirmed with a 500ms sleep: no effect). This is a **compile-time**
absence, not a timing issue. Full detail in `MoodConfig::current`'s own doc
comment (`common/src/bastion.rs`).

## Fact 2 — if it's ever enabled, `BASTION_VERIFY_ASSET_ROOT` becomes a pre-sim-only claim

`bastion-harness/src/main.rs`'s `BASTION_VERIFY_ASSET_ROOT` gate
(~line 1509, APEX-T1.2.08) recomputes the asset tree's content hash and
compares it against a declared value **before `Server::new` runs, before any
simulation starts**. That's deliberate today: hot-reload isn't compiled in,
so nothing can change the tree afterward, and the pre-sim check is exact for
the entire run.

**Turning on `hot-reloading` breaks that invariant without touching the gate
at all.** The two mechanisms are temporally disjoint by construction — the
hash check happens once, before the sim; hot-reload only fires strictly
after. Nobody has to reconcile them today because they never overlap. But
that disjointness is exactly what makes this dangerous rather than safe: if
asset content changes mid-run, the gate has already reported success against
a tree that no longer describes what executed. The certification doesn't
fail — it just stops meaning anything, silently, because the check already
ran and was true when it ran.

This is the same shape as two other findings from the last week (the stale
version-banner commit; a hold-check with no valid baseline, i.e. a gate that
cannot fail) — a check that reports honestly and certifies the wrong thing.

**Consequence:** if anyone enables `hot-reloading` for `bastion-harness` (or
any crate reachable from a certified run) in the future, `BASTION_VERIFY_ASSET_ROOT`
must be re-scoped first — either re-hash post-run and compare, hash
continuously, or explicitly document that certified runs must never enable
hot-reload. Fixing the gate is a prerequisite, not a follow-up.

## Status

Not acted on — `hot-reloading` is not enabled anywhere in this workspace
today, and AUTON-2 Step 1 uses the env-gated `MoodConfig::current()` override
instead (bypasses the asset pipeline entirely, so neither fact applies to
it). This note exists so the next person who reaches for hot-reload doesn't
have to rediscover either fact the hard way.
