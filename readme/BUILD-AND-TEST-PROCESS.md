# BUILD & TEST PROCESS — canonical (architect + builder follow this EXACTLY)

The single source of truth for how we build, test, sync, and verify. If a step here conflicts with habit,
this doc wins. Last set: 2026-07-19 (post remote-VM + fast-loop setup).

---

## 0. Environment (the fixed facts)
- **Fleet line:** `bastion/block-B6HAUL`. **Active build branch:** `bastion/builder` (off the fleet line).
- **Builder works in its OWN git worktree** (`.claude/worktrees/builder`) with its **own `target/`** — never the
  shared checkout. This is what prevents torn-tree collisions.
- **Remote box:** `benshumeyko@34.9.50.247` (GCP e2-highmem-8, Debian x86, 8 cores / 64 GB). Repo at `~/bastion`.
- **Repo remote:** `bastion-origin` → `github.com/Wiredshark/bastion` (public).

## 1. Build profiles (use the RIGHT one)
- **`--profile verify`** = release **minus LTO**. THE default for all harness builds. Cold ~8–13 min,
  **incremental ~18s** (sccache + mold/rust-lld). Command:
  `cargo build --profile verify -p bastion-harness`
- Never build `--release` (full LTO) for tests — it pays the LTO link tax for no runtime benefit here.
- `cargo check` only when you need pure correctness (no run) — ~2× faster than a build.

## 2. THE TEST DECISION (right tool per job — this is the core rule)
Decide WHERE a test runs by TWO questions:

**Is the code committed + pushed yet?**
- **No (uncommitted working-tree edits)** → **LOCAL only.** The VM literally cannot see uncommitted code.
  All rapid write→test→test iteration is local.

**Is it a quick check or a heavy run?**
- **Quick check / single short scenario / correctness gate** → **LOCAL** (18s loop beats any network round-trip).
- **Long soak (multi-minute) OR a multi-seed corpus** → **VM** (offload so it doesn't block local coding;
  and for corpora, parallelism pays). Requires the code to be COMMITTED + PUSHED first (see §4).

> Rule of thumb: **local = fast + uncommitted; VM = heavy + committed.** Never route small/uncommitted tests
> to the VM (overhead > the test). Never block the local machine on a multi-minute soak that could go remote.

## 3. Running tests
**Local** (uncommitted or quick), from the builder's worktree — NO `flock` (Windows/PowerShell; `flock` is
Linux-only). Local serialization = cargo's own target-dir lock + one-build-at-a-time discipline:
`cargo build --profile verify -p bastion-harness && ./target/verify/bastion-harness --<scenario>`

**VM** (committed + heavy) — use the **ON-DEMAND WRAPPER** (auto-starts the stopped VM, pulls latest, builds,
runs the scenario, streams results back; the VM self-stops ~15 min after):
`bash /e/veloren-master/vm-run.sh --<scenario> [args]`
The VM stays STOPPED when idle (idle-watcher saves credits); the wrapper starts it on demand (~30s boot). Do
NOT SSH the VM directly for tests — always go through the wrapper so start/stop + pull + build are handled. (§11.)

- `flock` = the OOM guard on the **VM (Linux) only** (the wrapper applies it); locally cargo's target-dir lock +
  one-build-at-a-time discipline serializes. One heavy build per machine at a time either way.
- Multi-seed gates: run seeds CONCURRENTLY where the harness supports it (parallel-seeds task queued); until
  then they run serially — a known slow spot.

## 4. Commit → sync flow (fully automatic once committed)
1. Builder commits in its worktree (at every boundary — never leave long uncommitted WIP).
2. **Post-commit hook auto-pushes** `bastion/builder` → GitHub (installed local; runs inside git, no agent action).
3. **VM auto-pulls every 2 min** (cron) → the VM has your committed code within ~2 min.
4. Only THEN can the VM test that code. (Uncommitted work never reaches the VM — that's §2.)
- Neither the architect nor the builder can `git push` directly (classifier-gated); the hook is the only path.

## 5. Determinism rules (do NOT get this wrong)
- **Canonical reproducibility certification = consistent architecture.** Bit-exact tape comparisons stay on the
  same arch/target as the baseline.
- **Behavioral metrics** (completion %, throughput, jam counts, timelines) are cross-machine-safe — the VM's
  Linux-x86 fidelity timeline matched local EXACTLY (verified 2026-07-19), so **VM behavioral numbers are
  directly comparable to local** with no fudge factor.
- **Crowd/M3 corpora** measure behavior → fine on ANY box (even ARM free tiers). Only bit-exact determinism
  gates need the pinned arch.

## 6. Verification discipline (what "done" means)
- A change is not done until it's VERIFIED, not just compiled. Run the scenario, read the result — don't trust
  green-compile as green-behavior (the ARENA/Farm/chop-felling lesson).
- Gate multi-seed before trusting (variance is real). MEASURE-then-confirm for investigations.
- Live-play (Ben's eyeball) is the final arbiter for anything client-visible.

## 7. Review escalation ladder (kept)
- **Sonnet** = first-line on every real change (lean single pass).
- Escalate **HARD / safety-critical / gate-grade** → **Opus**; **apex / capstone / adversarial** → **Fable**.
- Routed tier-by-tier VIA the architect. Higher tiers engage only when routed up. No parallel review-of-reviews.

## 8. Lean discipline (standing rules)
- One builder + the review ladder + Ben testing. No sub-agents, no fleet sprawl.
- **Background long builds and keep writing** — never idle-wait on a running gate if there's non-colliding work.
- **Cycle the builder per block** — don't run one session so long its context rots (the 16-hour-session failure).
- Commit at every boundary; own worktree + own target; no drive-by edits outside the current change.

## 9. GRIND LIMIT — escalate a stuck problem, don't tunnel (Ben-directed, STANDING RULE)
Do NOT grind on one problem indefinitely (the 8-hour BACKSTOP failure mode). The real discriminator isn't raw
count — it's **CONVERGING vs LOOPING** (mining-fix insight, 2026-07-19):
- **CONVERGING** = each iteration lands a DISTINCT verified fix with NEW evidence, closing toward done (not
  retrying variations of one idea).
- **LOOPING** = retrying the same idea, no new information, not closing — THE failure mode.

**CHECKPOINT (whichever first): ~3 iterations on the same problem OR ~45 min.** At the checkpoint, STOP and
assess honestly:
- Genuinely CONVERGING → you MAY continue, but state WHY (the evidence of distinct progress); checkpoint re-arms.
- LOOPING → ESCALATE IMMEDIATELY (even before the count).
**HARD CEILING (no exceptions): ~6 iterations OR ~90 min → ESCALATE regardless of self-assessed convergence** —
"I'm converging" can be wishful; the ceiling is the honest backstop.

**ESCALATION:** STOP, package ALL the data — every gate result, each attempt + why it failed, the tapes, current
hypotheses — and hand it to the **reviewer** (Sonnet first-line; Opus if hard/safety-critical) for a FRESH-EYES
root-cause + a proposed better approach. A builder deep in a problem has tunnel vision; the reviewer sees all
the data at once. **Reviewers and fresh sessions are the loop-breakers — grinding is not.** Resume once the
reviewer proposes a direction (or confirms the current one).
- **A reviewer-directed fix does NOT count against the ceiling.** Implementing the reviewer's proposed
  root-cause/approach after an escalation is a FRESH, evidence-backed direction — not continued grinding. The
  iteration counter resets on escalation. (Validated on the 2026-07-19 mining fix: Sonnet found leg C's real
  root — a rescue-rung claim cap wrongly catching proactive rungs — in ONE pass, after the builder's 3 plan-side
  iterations couldn't; that fix is v6 but doesn't re-arm the grind limit.)

## 10. Queued speedups — do them RIGHT AFTER the current task, BEFORE resuming testing (Ben-directed)
Sequence: **finish (or escalate) the current mining fix → implement BOTH speedups → then back to testing.**
Front-loaded because they make everything after them faster.
- **World-snapshot caching** → skip the ~74s worldgen boot on repeated runs (biggest per-run win; hard
  determinism guard — snapshot-load tapes must byte-match fresh-gen).
- **Parallel seed execution** → run gate seeds concurrently (halves 2-seed gates; ~8× on corpora).
When they land, amend §1/§3 to make them the default.

## 11. Remote test-offload — FULLY EPHEMERAL (2026-07-19)
No persistent box. Every run CREATES a VM from the golden image, VALIDATES it's on the latest commit, builds,
runs, and DELETES itself. Idle cost ≈ $0 (only the ~4.5 GB `bastion-golden` image).
- **Project** `project-850d63d4-bf88-46df-8cb` · zone `us-central1-a` · SSH `benshumeyko@` with
  `~/.ssh/id_ed25519`; each VM gets the key via `--metadata-from-file=ssh-keys=C:/Users/q/.ssh/bastion-sshkeys.txt`
  (the guest agent rewrites `~/.ssh/authorized_keys` from metadata on boot — metadata is the source of truth).
- **gcloud** at `C:\Program Files (x86)\...\gcloud.cmd` — space-containing args must run from PowerShell; plain
  calls work from Git Bash. The wrappers use `--metadata-from-file` to sidestep the space-arg issue.
- **Golden image** `bastion-golden` = 30 GB Debian-13, rustup stable+nightly, sccache+mold, repo cloned at
  `bastion/builder` with a warm `--profile verify -p bastion-harness` build. Rebuild after big merges:
  `bash vm-build-image.sh` (cold build ~4 min on 32 cores; deletes the old image only after the new verifies).
- **★ RUNS-LATEST, PROVEN:** every VM does `git fetch + reset --hard origin/bastion/builder` and ASSERTS the SHA
  (fail-loud exit 3) before building — each result is stamped with its commit; a stale checkout can't slip through.

## 12. QUOTAS — the real ceilings (know these before sizing a run)
- `CPUS_ALL_REGIONS` = **32 vCPU GLOBAL** — total across ALL running VMs. THE binding limit. (Bump to 128
  requested 2026-07-19 after Ben upgraded off trial; pending.) **NEVER schedule to the exact cap** — creates
  bounce (8×4=32 lost 2 VMs). Leave headroom.
- `SSD_TOTAL_GB` = **500 GB/region** — with 30 GB VMs, ~16 concurrent. (Bump to 2000 pending.)
- At a FIXED vCPU budget, **scale-UP (one big VM) beats the clone pool** — the pool pays per-VM build + boot
  overhead. Measured: 24 seeds = 142 s on one 32-core VM vs 367 s on 8×4-core. The pool only wins once you need
  MORE cores than one VM can hold (i.e., after the quota bump).

## 13. THE RUN MODES — which to use
```
1. Uncommitted / quick single check?          -> LOCAL (18s loop, §2). The VM only sees committed code.
2. One test, one VM (single scenario/soak)?   -> bash vm-run.sh --<scenario>
3. One test, MANY seeds (corpus/throughput)?  -> bash vm-scale.sh <machine> <N_seeds> <first-seed> "<args>" [$max] [min]
                                                 DEFAULT: one big VM, N cores, one build (§12: beats the pool).
   3b. Need MORE cores than one VM holds?      -> bash vm-pool-safe.sh <N> <machine> <seeds/VM> <first-seed> "<args>" [$max] [min]
                                                 (auto-halves if the live burn-guard trips)
4. MANY DIFFERENT tests at once (breadth /     -> bash vm-jobs.sh <jobs_file> <machine> [$max] [min]
   general data / full validation)?              one VM per job line, all parallel; template test-suite.jobs
```
All modes are ephemeral, SHA-validated, and burn-guarded. Keep (#VMs × vCPU) under the CPU quota with headroom.
For a fixed core budget prefer scale-up (mode 3 → one machine); reach for breadth (mode 4) when the tests DIFFER.

## 14. COST DISCIPLINE — ephemeral + guarded (protect the credit)
- **Idle cost ≈ $0** — no persistent VM/disk/IP; only the ~$0.02/mo image. Nothing bills unless a run is live.
- **Per-run burn-guard** (in vm-pool/jobs): meters cost LIVE (`vCPU × time × $0.035`), CUTS THE RUN OFF at
  `$MAX_USD`/`$MAX_MIN`; `vm-pool-safe.sh` then retries smaller. A wide/heavy run can't run away.
- **10-min watchdog** `vm-watchdog.sh` — deletes any `bastion-*` VM older than the age cap (forgotten/hung run).
  Schedule every 10 min (Windows Task Scheduler) for durable protection.
- **Panic button** `vm-cleanup.sh` — kills every stray VM + prunes images/snapshots.
- **Hard cutoff** `gcp-billing-setup.sh` — a budget that DISABLES billing when spend hits your credit amount
  (the "credits exhausted → stop everything" backstop; GCP has no real-time credit API, so this is the clean way).
- Every wrapper self-deletes its VM on exit (trap), on success OR failure. Behavioral metrics are cross-machine
  safe (VM Linux ≡ local Windows); only bit-exact certs stay pinned-arch.

## 15. CHEAP-TEST POSTURE — test heavier, test EARLIER (Ben-directed, 2026-07-19)
Heavy tests are now pennies and return in minutes, so testing is an INPUT to planning, not just a gate at the end:
- **Test-informed planning (BEFORE coding):** for any non-trivial feature/change, first run a CHARACTERIZATION
  sweep — a scenario/parameter matrix across many seeds (via vm-jobs) — to see how the current system ACTUALLY
  behaves, find the edge cases, and get baseline numbers. Design against data, not assumption.
- **Corpus-by-DEFAULT:** a single-seed gate is a lottery (the b4@seed-1 trap). Every gate runs an N-seed sweep;
  green means green across the distribution. Validation uses the CANONICAL gate seed per scenario (not arbitrary)
  so a red = real regression, not a bad-terrain roll.
- **Standing regression matrix:** the full scenario catalog × canonical seeds via vm-jobs (test-suite.jobs), run
  on every meaningful commit — regressions surface immediately, not at a milestone.
- Sequence unchanged (build → gate → tag), but the DESIGN step is now test-backed and VALIDATION is a fanned-out
  matrix, not a single run. (Corpus-first + assert-the-precondition are the standing anti-flake disciplines.)
