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
- **Behavioral metrics** (completion %, throughput, jam counts, timelines) are cross-machine-comparable
  **ONLY PER-SCENARIO — proven, not assumed.** Mine-fidelity's timeline matched VM ≡ local EXACTLY
  (verified 2026-07-19) → its VM numbers compare directly. BUT dig-access leg C DIVERGED (VM 425/432 vs
  local 324/432, same seed+code, 2026-07-19) — churn amplifies scheduling variance across machine classes.
  **RULE:** before comparing a VM result to a LOCAL baseline, confirm that scenario is cross-machine-stable;
  otherwise establish the baseline on the SAME machine class it'll be judged on (VM baseline for VM runs).
  Churn-heavy scenarios are the risk. Same-machine comparisons (VM-vs-VM, corpus seed sweeps) are always fine.
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
- `CPUS_ALL_REGIONS` = **96 vCPU GLOBAL** (granted 2026-07-19 in steps 32→64→96; 128 still denied on account
  age) — total across ALL running VMs. THE binding limit, and now the SOLE one. **NEVER schedule to the exact
  cap** — creates bounce (and racing a prior batch's teardown re-bounces; wrappers now create-retry w/ backoff).
  Leave headroom. Max ≈ 22 × 4-core, or ~46 × 2-core VMs.
- `SSD_TOTAL_GB` (us-central1) = **effectively UNLIMITED** (granted 2026-07-19; was 500 GB) — disk is no
  longer a constraint; size the run by vCPU alone.
- **★ DON'T BE STINGY (Ben directive 2026-07-19):** 96 cores + unlimited disk + ephemeral auto-delete +
  burn-guard = generous VM use is SAFE and ENCOURAGED. Size runs BIG (a big scale-up VM and/or many VMs,
  up to ~88 vCPU with headroom) to finish sooner — don't conserve out of over-caution. Idle cost stays
  ~$0 (self-delete); a burst is pennies. The ONLY guardrails: keep the burn-guard $/time ceilings on,
  don't schedule to the exact cap (~8 vCPU headroom for teardown races), let the wrappers self-delete.
  Bias: more VM / bigger VM / done sooner.
- **BIG single VM:** e2 caps at 32 vCPU/VM. **e2-standard-32 is THE big-VM** (32 cores, one build). c2-60 is
  BLOCKED — the c2 FAMILY has its own quota, `C2_CPUS = 8` in us-central1 (a live c2-60 create failed
  "C2_CPUS exceeded. Limit: 8.0"; my earlier regions-describe 200 read was the wrong metric). n2/n2d = 0.
  To exceed 32 cores in ONE VM, request a C2_CPUS family bump (same flow as the 96 bump; account-age may bite).
- **Golden image auto-refresh:** `vm-golden-autorefresh.sh` (schedule nightly) keeps the image at the
  latest tip incrementally + idle-guarded, so each run's catch-up build stays small.
- At a FIXED vCPU budget, **scale-UP (one big VM) beats the clone pool** — the pool pays per-VM build + boot
  overhead. Measured: 24 seeds = 142 s on one 32-core VM vs 367 s on 8×4-core. The pool only wins once you need
  MORE cores than one VM can hold (i.e., after the quota bump).

## 13. ★ BEFORE EACH HEAVY RUN — ASK: BIG VM or MANY VMs? (state the choice + why — Ben, MANDATORY)
This is a CONSCIOUS decision per run, NOT a default. The tradeoff: a **BIG VM** = ONE build, N parallel
processes on one machine (most efficient — fewest builds). **MANY VMs** = one build EACH (more overhead)
but scales past one machine + isolates faults + runs heterogeneous work. Pick deliberately every time.
```
 quick / single check              -> LOCAL (§2) or  vm-run.sh --<scenario>   (1 small VM; no cores needed)
 SAME scenario, many seeds,        -> BIG VM:  vm-scale.sh e2-standard-32 <N_seeds> ...   (32 seeds, ONE build,
   (<=32 seeds fit ONE VM)              ONE create — dodges the rate limit). e2-32 is THE big-VM. c2-standard-60
                                        is BLOCKED (C2_CPUS quota = 8, needs a family bump). BIG = fewest builds.
 SAME scenario, >32 seeds           -> 3 × e2-standard-32 via vm-pool-safe (staggered) = 96 cores, 3 creates.
 Want the FULL 96 cores (>60,      -> MANY VMs: vm-pool-safe.sh <N> <machine> <seeds/VM> <first-seed> "<args>" [$][m]
   can't fit one VM) OR fault-         96 can't fit in one VM; also for fault isolation across machines.
   isolation
 DIFFERENT scenarios at once       -> MANY VMs: vm-jobs.sh <jobs_file> <machine> [$][m]
   (a suite / the FULL validation)     one VM per job line, heterogeneous; template test-suite.jobs
```
**RULE:** BIG-VM when the work is homogeneous + fits one machine (fewest builds = most efficient);
MANY-VMs when you need >60 cores, fault isolation, or different tests at once. The builder STATES which +
why before every heavy run — this is the pre-run question, asked deliberately, not a habit.
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

## 16. ASYNC TESTING — the builder NEVER blocks on a VM test (throughput, Ben-directed 2026-07-19)
VM tests are fire-and-forget: the wrapper creates its own VM, runs, self-deletes. So the builder must NEVER
sit foreground-waiting on one. At every moment the builder is CODING; tests run BESIDE it.
- **Dispatch async:** `bash /e/veloren-master/vm-run.sh --<scenario> > /tmp/r.log 2>&1 &` (or vm-scale / vm-jobs
  backgrounded), then IMMEDIATELY continue to the next code block. Collect the result when it lands — don't watch.
- **Local 18s loop** stays for tight iteration (foreground OK — it's fast). VMs carry the heavy/validation runs,
  always backgrounded.
- **A slow/stuck test never blocks progress** — the burn-guard + watchdog kill it; you move on and re-dispatch.
- A test result arriving is an INTERRUPT to triage, not a thing to wait for. This supersedes any
  "build → wait for the gate → then continue" habit. Waiting on a test is the one thing we no longer do.

## 17. MAX-TESTING — run BIG corpora, fill the 96 cores, fan analysis to Sonnet (Ben-directed 2026-07-19)
Single-seed checks waste the hardware. Every verification/validation run goes WIDE by default.
- **The ceiling is ~96 tests in parallel** (1 core each during sim; 96 vCPU cap). FILL it — don't run 2
  seeds when you can run 50.
- **Sizing (efficient max):** fewer MEDIUM VMs each running MANY seeds — e.g. `vm-pool-safe.sh 12
  e2-standard-8 8 <seed> "<scenario>" <$> <min>` = 12×8 = ~96 concurrent tests. (★ VALID e2 sizes are
  2/4/8/16/32 ONLY — NO e2-standard-6; an invalid type 8/8-fails the whole pool, now visible via the
  CREATE_FAIL error capture.) e2-standard-32 is THE big-VM (c2-60 is C2_CPUS-quota-blocked at 8). NOT 1-core
  VMs (the ~65s boot wants several cores → slow) and NOT many separate VMs (redundant builds + rate limit).
  Seeds-per-VM fills the cores.
- **★ CREATE-RATE LIMIT (found 2026-07-19):** GCP rate-limits PARALLEL instantiations from ONE machine-image
  ("too frequent operations from the source resource") — firing N creates at once bounces most (a 6-VM pool
  lost 5/6). So FEWER CREATES wins TWICE (build overhead + rate limit): prefer FEW BIG VMs. To fill 96 with
  the fewest creates: **3 × e2-standard-32** (3 creates, staggered) — NOT 12+ small VMs. The pool
  wrappers now STAGGER creates ~10s (STAGGER env) so pools still work, but the big-VM (mode 3, one create)
  DODGES the limit entirely — it's now the REQUIRED default for wide runs, not just the efficient one.
- **IMAGE-COPY POOL — BANKED (Ben 2026-07-19), trigger-gated:** few-big-VMs (packing many seeds per VM)
  beats many-VMs on every axis WHILE tests can be PACKED multiple-per-VM (each ~1 core, independent). The
  ONE case that flips this: a test that CANNOT be packed — **exclusive / whole-machine / one-per-VM** (e.g.
  GPU/client/voxygen render tests, perf/stress tests that bind all cores, networked multi-client tests with
  port conflicts). Then N parallel such tests REQUIRE N VMs → N rapid creates → the per-image rate limit
  bites. ★ MONITOR for this test class. When it appears, BUILD the image-copy pool: N copies of the golden
  spread the per-image create rate (scoped ~20 min — differently-named copies so vm-cleanup's golden-prune
  skips them, round-robin creates in the wrappers, refresh-all in vm-golden-autorefresh). Until then: don't
  build it; few-big-VMs is strictly better.
- **Broaden everything:** a fix's confirmation = a 30-50 seed corpus, not a 2-seed sample; scenario
  verification = a matrix across seeds; the post-M3 FULL VALIDATION = the whole catalog × canonical+corpus
  seeds, sized to fill 96 cores.
- **★ ANALYSIS FAN-OUT:** at max scale the bottleneck is RESULT-READING, not compute. When a corpus
  produces more than you can triage, ROUTE THE RESULTS TO SONNET (local_5f3f9b01) to classify reds
  (real / seed-premise / flake) + spot patterns in parallel. Offload the reading so it's never the ceiling
  on how wide we test.
