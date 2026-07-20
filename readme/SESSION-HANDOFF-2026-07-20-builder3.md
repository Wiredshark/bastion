# SESSION HANDOFF — builder-3, 2026-07-20 (M3 block complete, wound down at the cycle point)

Builder 4 starts HERE with zero archaeology. Branch: `bastion/builder` (worktree
`.claude/worktrees/builder`, own target dir). Everything below is committed + pushed to
`bastion-origin/bastion/builder`; tip at handoff = `8c4543094a` + Sonnet's bookkeeping
(`b9fbb18ff8`). Tags `bastion-block-M3` (Opus-gate PASS) pushed.

## ★ YOUR JOB: the CRATE-SPLIT (efficiency slate #2, architect-GO'd, Sonnet-supplied)
- PACKET: `readme/CRATE-SPLIT-BASTION-SERVER-PACKET.md` (shared checkout E:\veloren-master\readme\
  — NOT yet in this worktree's branch; read it there or cherry it). Extract the 12
  `server/src/bastion_*.rs` modules (~18.4k lines) → new leaf crate `bastion-server`. PURE
  STRUCTURAL — byte-identical behavior is the acceptance bar, not a behavioral gate.
- ACCEPTANCE (architect's exact bar): (1) full workspace builds rc=0; (2) bastion tests pass FROM
  THE NEW HOME — ★ the R10/M3 exhaustiveness pins use `include_str!("bastion_jobs.rs")`: the path
  moves with the file so it likely still works, but VERIFY the pin actually reads the moved file
  (a wrong path reading an empty/stale file = a silently-dead pin — the worst failure mode);
  (3) `--mine-fidelity-scenario` + `--dig-access-scenario` at a fixed seed BYTE-IDENTICAL vs
  pre-split (any diff = a move mistake); (4) record the incremental-rebuild timing delta after a
  1-line bastion_jobs.rs edit — the delta IS the deliverable.
- The 3 coupling knots (Tick / RtSim / RepositionToFreeSpace) are surveyed in the packet with
  ranked options. Re-run the survey across all 12 modules before moving.
- OPTIONAL rider (architect: "your call, or keep the split pure"): the M3-review follow-up
  debug_assert that the unified corridor drive never fires for a member holding a live
  bastion_traversal_task (upgrades a behavioral invariant to by-construction; you'll be in
  bastion_traversal anyway). If you fold it in, OWN COMMIT — keep the split commit pure-move.
- Own commit(s), bisect-clean isolation, quiet tree (it is — nothing else is mid-block).
- FLAG THE ARCHITECT at the tag for review. AFTER the split: the FULL VALIDATION PASS
  (Ben's gate, task #4 shape: fan the catalog via vm-jobs, canonical seeds) BEFORE new features;
  boot-cache (Codex, `codex/boot-cache`) merges architect-side in parallel — different files.

## State of everything this session closed
- ★ M3 (ladder contention): TAGGED `bastion-block-M3` @ 8c4543094a, OPUS-GATE PASS (verdict in
  BUILD_REVIEW_LOG.md §M3). Package: `readme/M3-TAG-PACKAGE.md` (commit chain, safety surface,
  episode results, the 11-version M3A fix arc, honest riders). Queue invariants clean on all 24
  matrix runs; determinism x3; N2 green x13.
- ★ DPA no-wood livelock (Ben's live "no ladders" bug): FIXED @ 96bbf1d2bb, verified local+VM
  attested on 3 seeds + dig-access regression at documented baseline. CLOSED by the architect.
- Registry adds: B57 (own-prefix self-hit class, 4 sites, all fixed) + B58 (inherited frontier
  net-reliance at organic seeds — tag-accepted tracked-open; the N2-at-same-seeds discriminator
  is the named evidence). Both Sonnet-filed @ b9fbb18ff8.
- Infra hardened today (architect-side commits): CREATE_FAIL error capture, build-fail guard,
  per-job ATTEST, create-stagger (machine-image rate limit), sha-part-only stale-binary guard
  wired to my `--print-git-hash` flag (e500402b3b), §13 big-vs-many statement rule, §17 few-big-VMs.
- Corpus runner: `--ladder-episode` forwarding rider (1dca97248f) — corpora can fan the b58
  episode family.

## Cautions that WILL bite you (each cost this session real time)
- EVIDENCE TRAPS (all memory-banked, `log-time-namespace-and-vm-attestation`): strip ANSI before
  grepping logs (`sed 's/\x1b\[[0-9;]*m//g'` — raw `grep uid=3` matches NOTHING); `$?` captures
  the LAST statement (echo it IMMEDIATELY or grep the verdict line); wall-clock ≠ sim-time (~7-9x
  headless); a VM result without an ATTEST line ≠ evidence; `gcloud compute images list` ≠
  `machine-images list` (the golden is a MACHINE-image).
- VM realities: e2 sizes are 2/4/8/16/32 (no 6!); C2_CPUS quota = 8 (no c2-60 until a bump);
  parallel creates from one machine-image rate-limit (~5/6 died once) — stagger is in the
  wrappers now, but prefer FEW BIG VMs (e2-standard-32 = THE big VM); state the §13 big-vs-many
  choice per heavy run. Call wrappers by ABSOLUTE path (`bash /e/veloren-master/vm-jobs.sh …`) —
  the Bash tool's cwd drifts after any `cd`.
- The VM checkouts stamp `+dirty` from golden-image LFS noise ("597 files should have been
  pointers") — the guard now compares sha-part only; the LFS noise itself is a logged cosmetic
  cleanup, don't chase it.
- Corpus children DISCARD stderr (Stdio::null) — per-seed forensics are lost; tee-per-seed is on
  the hardening list. Multi-child in-process corpora are LOAD-SUSPECT until the quiet-machine
  rerun arbitrates (it flipped nothing this session in the end, but check before classifying).
- The b58 fixture's SOFT-0 predicate counts the CARVED shaft columns — organic rolls can site the
  planner's lane elsewhere (false-positive violations; the a2 fixture-hardening item).

## Open loose ends (owner in brackets)
- B58 frontier-approach corridor-unification — its own block, design blessed [next-block queue]
- Corridor-drive debug_assert [optional rider on YOUR block, else the B58 block]
- Fixture hardening: a2 SOFT-0 plan-lane predicate + a3 M3D timing bars on non-canonical rolls
  [with the B58 block or the validation pass]
- Corpus runner: stderr tee per seed [small, any gap-fill]
- Full validation pass (Ben's gate) [YOU, after the crate-split]
- Boot-cache merge review [architect + Codex; merges after/parallel — different files]
- c2-60 quota bump [architect, deferred]

## Process (unchanged, canonical)
BUILD-AND-TEST-PROCESS.md rules everything: §2 local-vs-VM, §9 grind-limit (3/45min checkpoint,
6/90min ceiling — reviewer-directed fixes reset it; this session used the ladder 3x and it worked
every time), §13 big-vs-many statement, §16 never-foreground-wait, §17 max-testing + Sonnet
analysis fan-out. Sonnet (local_5f3f9b01) = first-line review + next-block supply + corpus triage;
architect (local_635eaffb) = gates at tags, exec authority while Ben's out. Every inbound message
gets an outbound ack; queued ≠ delivered for blocking sends.
