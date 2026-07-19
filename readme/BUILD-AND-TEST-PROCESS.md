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

**VM** (committed + heavy), one line — NOTE the `source` (non-interactive SSH doesn't load cargo's env); `flock`
guards the Linux box:
`ssh benshumeyko@34.9.50.247 'source $HOME/.cargo/env; cd ~/bastion && git pull -q && flock /tmp/bastion-build.lock cargo build --profile verify -p bastion-harness -q && ./target/verify/bastion-harness --<scenario>'`

- `flock` = the OOM guard on the **VM (Linux) only**; locally cargo's target-dir lock + one-build-at-a-time
  discipline serializes. One heavy build per machine at a time either way.
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
Do NOT grind on one problem indefinitely (the 8-hour BACKSTOP / multi-hour mining-fix failure mode).
**TRIGGER (whichever comes first):** ~**3 failed gate iterations** on the same problem, OR ~**45 minutes**
grinding without convergence.
**ACTION:** STOP iterating. Package ALL the data — every failing gate result, each attempt + why it failed,
the tapes, the current hypotheses — and **ESCALATE to the reviewer** (Sonnet first-line; Opus if
hard/safety-critical) for a **fresh-eyes root-cause + a proposed better approach.** A builder deep in a problem
has tunnel vision; the reviewer sees all the data at once and proposes the path. **Reviewers and fresh sessions
are the loop-breakers — more grinding is not.** Resume once the reviewer proposes a direction (or confirms the
current one). This is the standing "escalate out of a loop" rule.

## 10. Queued speedups — do them RIGHT AFTER the current task, BEFORE resuming testing (Ben-directed)
Sequence: **finish (or escalate) the current mining fix → implement BOTH speedups → then back to testing.**
Front-loaded because they make everything after them faster.
- **World-snapshot caching** → skip the ~74s worldgen boot on repeated runs (biggest per-run win; hard
  determinism guard — snapshot-load tapes must byte-match fresh-gen).
- **Parallel seed execution** → run gate seeds concurrently (halves 2-seed gates; ~8× on corpora).
When they land, amend §1/§3 to make them the default.
