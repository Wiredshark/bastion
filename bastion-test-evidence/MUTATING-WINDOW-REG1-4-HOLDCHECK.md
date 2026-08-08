# Mutating window: REG-1..4 implemented, holdcheck against the registered deltas

Implements `MUTATING-WINDOW-DELTA-REGISTRATIONS.md` (commit `0a5781e4ee`,
baseline `wave26_ROWA_d5b56d1c79_FULL.json`, 48 seeds). Plus Fable's
Rider-1 doc amendment. Rider-2 (message-surfacing follow-up) filed, no
code this window.

## REG-1 — tool-factor sentinel

`tl_stone`/`tl_steel` are now `Option<f32>` (the raw measurement, no
`.unwrap_or(0.0)`); `tl_ok` is `Option<bool>`, `None` when either
measurement is missing rather than a computed `false` that reads as a
real verdict. `tl_stone_chop`'s own sentinel is untouched (out of the
registered 3-field delta). The `("tl_ok", tl_ok)` pass/fail gating
clause now reads `tl_ok.unwrap_or(false)` — preserves the scenario's
exact prior gating behavior (a missing measurement failed the old
sentinel-`false` gate too) while the *reported* `b5_tool_ok` field
honestly carries `null`.

## REG-2 — `route_next_idx_pinned` three-way split

Old: `Option<bool>` collapsing "too few samples" and "a sample had no
route" into the same `null`. New: `&'static str`, one of
`too_few_samples` / `no_route_present` / `compared: pinned` /
`compared: advancing`, computed by reading `route_exists` from each
`timeout_route_states` entry directly (not inferred from
`route_next_idx`'s presence, even though `ChaserDiagnosticSnapshot`
derives both from the same `Option` and so happen to agree). Raw
`timeout_route_states` list untouched.

**Verification method, two independent legs:**

1. **Classification correctness** — before porting to Rust, simulated
   the exact three-way rule in Python against wave26's raw
   `timeout_route_states` arrays (not the old summary field — checking
   a summary-split against the summary would be circular). Reproduced
   the registered 79/10/8/6 split with the exact seed sets, before any
   Rust code was written.
2. **Port fidelity** — a live 48-seed local sweep (seeds 49-96,
   `--b5-scenario --seed N`), diffed against wave26 field-by-field.

## REG-3 — `b5_55_*` → `b5_blocked_designation_*`

Pure rename, matching the `b5_mine_*`/`b5_ch_*` family convention.
`names_blocker`/`notified_once` remain `false` on every seed (the known
"fixed cells, wrong coordinates" defect) — the rename does not fix
that, only names it consistently.

## REG-4 — `build_ok_jobs`/`build_stall_jobs`/`build_stall_untouched` rename

→ `b5_build_ok_fixture_count` / `b5_build_stall_fixture_count` /
`b5_build_stall_control_untouched`. Constant 1/1/true on all 48 seeds,
unchanged. `build_stall_control_untouched`'s name now conveys CONTROL
(Fable's reclassification, propagating from this session's own
CHOP-BUILD-INSTRUMENT-WINDOW finding that `build_stall_pos` is a
deliberate negative-control fixture, not a stalled diagnostic still
waiting to fire).

## Rider-1 — Area2D doc amendment

`resolve_column_surface`'s doc claimed "ONE function... ALL resolve the
SAME surface." Amended to name the excluded case: Area2D kinds (Farm,
Chop) never carry a `ZExtent` and cannot call this function at all —
they resolve separately via `column_surface_z` directly, by signature,
not by choice (see `FARM-PAINT-FIX.md`). "If Area2D kinds ever gain
flat-mode semantics, unify first" — names both the trap and the
required order.

## Holdcheck result

48-seed local sweep (`bastion-harness --b5-scenario --seed N`, N =
49..96, matching wave26's seed range), diffed field-by-field against
`wave26_ROWA_d5b56d1c79_FULL.json`:

- **REG-1**: exact — seed 66 only, `b5_tool_stone`/`b5_tool_steel`/
  `b5_tool_ok` → `null`; all 47 other seeds and all other fields on
  seed 66 unchanged.
- **REG-3 / REG-4**: exact — new keys present with values identical to
  the old keys on all 48 seeds; old keys absent.
- **Unexpected moves outside the registered deltas: 0** — every other
  field that exists in both wave26 and the live run is byte-identical
  (excluding two known-noisy, non-deterministic fields: the build
  identity stamp and per-run tick timing, neither a correctness
  signal).
- **REG-2**: on the SAME producer set wave26 measured (Mine/Chop
  reachability probes, the original `probe_target` call sites), the
  split reproduces **exactly** 79 `too_few_samples` / 10
  `no_route_present` / 8 `compared: pinned` / 6 `compared: advancing`,
  with the exact registered seed sets (52,54,61,66,71,90 /
  52,54,92 / 54,71,78,80 / 61,71,85,90).

**One thing that looked like a mismatch and wasn't, recorded rather
than smoothed over:** the live sweep's *raw* tally (before splitting
by producer) was 88/11/12/11 — not 79/10/8/6. Traced directly rather
than reported as a finding: `b5_self_job_reachability_probe` (landed
this session, after wave26 was captured, via the self-job mode-triple
wiring) reuses the same `probe_target` closure REG-2 touches, so it
independently emits its own `route_next_idx_pinned` values at a JSON
path (`/b5_self_job_reachability_probe[N]/route_next_idx_pinned`) that
did not exist when wave26 was captured. Isolating by path confirms the
extra 19 results (9/4/5/1) come entirely from that producer, and the
original producer set's tally is exact. Not a REG-2 defect — a
different, already-landed, already-verified change surfacing more
instances of a field this window happens to also touch. Recorded here
so the same appearance doesn't cost someone else the investigation
twice.

**Verdict: all four REGs land exactly as registered. No unregistered
field moved. The window closes.**
