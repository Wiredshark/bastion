# ITEM 8 v3 — WRONG BINARY FINDING (Opus's catch, verified)

**v3's `veloren-server-cli.exe` ran the pre-fix binary
(`fb9a740110`, the ROW-COLONY-PRESENCE-ACCEPTANCE build), not the fix pin
(`f5267f15bb`). This voids v3's crash-fix result — the 2h34m no-panic
run is not evidence the fix works, because the fix was never in the
running binary.** Everything else in v3 (the famine cascade, the
instrumentation gap, the founding/presence data) is real and stands —
only the crash-fix claim specifically is void.

## THE EVIDENCE, each read directly, not inferred

**1. The binary's mtime:**

    target/no_overflow/veloren-server-cli.exe -- Aug 11 08:07 EDT

**2. The boot log's own version line** (`server-stdout-item8-endurance-v3.log:190`):

    Server version: fb9a7401 [2026-08-11]

**3. That hash resolves to a real, dated commit** —
`fb9a740110f60446722f3f3416f45aa6e2a68465`, 2026-08-11 08:04:40 EDT:
`"ROW-COLONY-PRESENCE acceptance: per-colonist loaded+needs diagnostic"`.
**This predates every fix-related commit landed later this session**
(`468fe8f07c`, `517cb50f6d`, `f5267f15bb`, all committed after 09:00 EDT).
Confirmed also: `git merge-base --is-ancestor fb9a740110 e14795700e` → yes
(it's an ancestor of the fix, not a sibling — the binary is simply old,
not from a diverged branch).

## THE MECHANISM — exactly identified, not guessed

**My "in-worktree build" command, run before v3's launch:**

    cargo build --profile no_overflow -p veloren-server-cli -p veloren-client --bin bastion_playtest

**`--bin bastion_playtest` restricts cargo's target selection to ONLY the
binary named `bastion_playtest`** — a `[[bin]]` target that exists in
`veloren-client`'s `Cargo.toml`, **not** in `veloren-server-cli`'s.
Despite `-p veloren-server-cli` being passed, cargo builds only targets
matching the `--bin` filter across the named packages — since
`veloren-server-cli` has no binary by that name, **it was never compiled,
never relinked, and the stale `08:07` binary from the acceptance run was
what actually launched.**

**Confirmed directly from the captured build output** (task `bz88gdak7`,
this session) — every `Compiling` line it produced:

    Compiling veloren-common-net v0.18.0
    Compiling veloren-common-systems v0.18.0
    Compiling veloren-common-state v0.18.0
    Compiling veloren-client v0.18.0

**Neither `bastion-server` nor `veloren-server-cli` appears.** I checked
this build's output for `error`/`Finished` lines at the time and did not
scrutinize which packages actually compiled — that is the exact gap.

## CORRECTION to the "50× log-rate" figure in Opus's message

**v2's log is not 8.25 MB.** Re-verified directly:

    server-stdout-item8-endurance-v2.log:                       286,961 bytes,  ~26 min  -> ~11.0 KB/min
    server-stdout-item8-endurance-v3.log:                      1,123,528 bytes, ~154 min -> ~7.3 KB/min  (already noted in ITEM8-V3-CAPTURE-REPORT.md)
    server-stdout-colony-presence-acceptance-v3.log:           8,252,395 bytes, ~15 min  -> ~550 KB/min

**v2 and v3 are ~1.5× apart, not 50×.** The 8.25 MB figure belongs to a
**different run entirely** — `ROW-COLONY-PRESENCE-ACCEPTANCE-RESULTS.md`'s
scored leg (script-19), which had `BASTION_COLONY_PRESENCE_ACCEPTANCE_DIAG=1`
set (27,640 diag lines confirmed present in that log via grep; **0** in
both v2's and v3's logs). That diagnostic logs every colonist every pass
unconditionally — the documented reason it was deliberately left unset for
both v2 and v3 (`ITEM8-LAUNCH-RECORD-V2.md` and `-V3.md` both state this).
**No env/diag difference between v2 and v3 themselves explains anything —
there wasn't one to explain.**

## WHY v3 DIDN'T CRASH ANYWAY — plausible mechanism, not proven

v2 crashed at tick 45000 during active haul/farm/eat cycling. v3 ran the
**identical unfixed binary** for 274,200 ticks without panicking. The
most likely reason: **v3's famine cascade minimized exposure to the
crashing code path.** Only 40 total eat completions occurred across the
whole 2h34m run (vs. v2's more active economy before its crash), food
stock hit 0 by tick 99300 and never recovered — far fewer opportunities
for the pre-fix `split_off_one` to build the invariant-violating shape
that `try_merge`'s `debug_assert!` catches. **This is inference from
adjacent numbers, not a count of how many times the bad path actually
fired** — consistent with this run's own instrumentation gap (no
periodic witness existed even in the fixed binary, and this binary
predates the counter entirely). Named as the likely explanation, not
claimed as proven.

## WHAT THIS VOIDS AND WHAT IT DOESN'T

- **VOID:** the crash-fix consequence-half claim ("zero panics across
  2h34m, 3× v2's fuse, proves the fix holds"). It proves nothing about
  the fix — the fix was not present.
- **STANDS:** the famine-cascade data, the founding/presence results
  (0 demotions — `ROW-COLONY-PRESENCE` predates this binary too, at
  `fb9a740110`, so that result is genuinely valid), the instrumentation
  gap finding (the counter has no emitter in *any* binary, fixed or not
  — a real defect in the source regardless of which binary ran), and the
  teardown/capture procedure itself (which behaved correctly regardless
  of which binary it was capturing).

## THE PROCESS FIX (Opus's proposal, endorsed)

**A mandatory first-minute check on every future launch: grep the boot
log's `Server version:` line against the intended pin before letting the
launch turn end.** One line, would have caught this immediately. Adding
to the launch checklist for whenever v4 (the real fix pin) launches.
**Also:** any pre-launch "verify the build" step must name which
*packages* a build command actually touched (the `Compiling` lines), not
just check for `error`/`Finished` — a scoped `-p`/`--bin` filter can
silently exclude the one binary that matters while still exiting 0.
