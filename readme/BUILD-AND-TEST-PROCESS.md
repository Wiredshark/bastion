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

## 11. Remote VM on-demand ops (the box manages its own uptime — 2026-07-19)
- **Instance** `instance-20260719-131242` · zone `us-central1-a` · static IP `34.9.50.247` · project
  `project-850d63d4-bf88-46df-8cb`. SSH key is METADATA-managed (survives restarts — do NOT rely on manual
  `~/.ssh/authorized_keys`, the guest agent rewrites it on boot).
- **Auto-STOP:** `/etc/cron.d/vm-idle-stop` powers the VM off after ~15 min with no cargo/rustc/bastion-harness
  process and no SSH session. Stops on IDLENESS, not a clock — so it never interrupts the 24/7 op's overnight runs.
- **Auto-START:** `vm-run.sh` (repo root, local-only tool) starts the VM if stopped before running. Uses
  `gcloud` at `C:\Program Files (x86)\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd` — **space-containing
  args (ssh-keys, --command) must be called from PowerShell**; plain calls (start/stop/describe) work from Git Bash.
- **Billing:** ~$0.36/hr RUNNING only; stopped ≈ $0 compute + ~$3.6/mo (disk + reserved static IP). On-demand
  keeps idle cost near-zero. Manual override: `gcloud compute instances start|stop instance-20260719-131242 --zone=us-central1-a`.

## 12. Elastic VM POOL — spin up many, run a corpus, delete them (heavy testing only)
For a big parallel corpus (M3 crowd, broad regression) — the "many servers for a burst, on one trial" pattern.
GCP quota here is **200 vCPUs** (not the 8 the research claimed), so up to ~24 × e2-highmem-8 concurrently;
the real limit is the $300 credit budget, and bursts are cheap (~20 VMs × 15 min ≈ $2).
- **Golden image** `bastion-golden` = a snapshot of the box (toolchain + repo + warm build) so a clone boots
  ready-to-run in ~30s instead of a 15-min setup.
- **Run a corpus:** `bash /e/veloren-master/vm-pool.sh <N> <first-seed> "<harness-args, no --seed>"` — creates N
  clones, runs one seed each IN PARALLEL, collects `/tmp/pool-results/*.json`, then DELETES every clone. Pay
  only for the burst minutes.
- **★ ALWAYS UP TO DATE:** every clone (and the on-demand box) **git-pulls latest on boot before running**, so
  it ALWAYS runs current code no matter how old the image is. The image is just the expensive baseline.
- **Keep the baseline fresh (speed only):** `bash /e/veloren-master/vm-refresh-image.sh` rebuilds `bastion-golden`
  from the current HEAD. Run it **after significant merges** (or nightly). Skipping it never breaks correctness —
  it just makes the boot-time pull bigger. So a stale image = still-correct, slightly slower.
- Scaling past GCP: crowd/behavioral corpora don't need bit-exact determinism, so the pool can extend to Azure /
  Oracle free tiers too (each its own trial) for even more parallelism — build per-provider spawners when needed.
