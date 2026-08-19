# BUILD-SPEED ROW — CLOSURE against §5's acceptance criteria

## §5 acceptance, item by item

| | criterion | status |
|---|---|---|
| **A1** | §2 baseline committed, three numbers, producers named | **✅** `cargo build --timings`, warm iteration after a one-line `bastion_jobs.rs` edit: **total 19.41 s · compile 18.76 s (3 units) · link 0.65 s**. Producer is the timings HTML, unit-level |
| **A2** | each applied lever carries before/after on the same measurement | **✅** see the ledger below |
| **A3** | post-change determinism fingerprint A/B (shell-change revalidation) | **N/A — the trigger did not fire.** No profile, config or crate-structure change was applied; the split was *refused by measurement*, so the binary's shape is unchanged |
| **A4** | daily-spend arithmetic re-run at close | **✅** below |
| — | planted control: revert the lever, the saving must disappear | **N/A for the split** — no lever was applied, so there is nothing to revert. For image freshness the equivalent control is the measured decay itself: **+5.9 s/hour since bake**, i.e. the saving *does* disappear as the image ages, which is the same statement |

## A4 — daily-spend census at close (2026-08-19 fans)

| | |
|---|---|
| host-builds measured | **96** |
| median per-host build | **164 s** |
| **total VM build time** | **257.3 min** |
| at the warm-image floor (50 s) | 80.0 min |
| **recoverable by image freshness** | **177.3 min/day** |

Measured decay **+5.9 s per hour since bake** ⇒ from the floor, crossing the 90 s
re-bake trigger takes **6.8 hours**. That is the re-bake period, derived rather
than chosen.

## Lever ledger — every number carries its producer

| lever | disposition |
|---|---|
| **image freshness** | **177.3 min/day recoverable** [this census]. Re-bake ~every 7 h. ★ And the bake script had a bug: it always sourced the **2026-07-19** image, so every re-bake rebuilt a month and compounded nothing — fixed to source the newest *dated* image |
| **41 GB incremental cache** | Not a marginal lever: it made `veloren-server-cli` **fail to link at all** on `no_overflow`, and `cargo clean -p` never touches it. Removal → builds, exit 0. **Local iteration unblocked** |
| **crate split (§4)** | **REFUSE — with numbers.** Link is **0.65 s of a 19.41 s** warm iteration (**3.3%**) against a registered bar of *<30% ⇒ not justified by link time*. Recompile-scope ceiling is **22%** (a perfect split saves `bastion-server`'s 4.34 s; the two dependents' 14.4 s rebuild regardless) |
| **port parameterisation** (`PIT_SLOT`) | built, in the runner |
| **`TWINS=1`** for coverage sweeps | built, in the runner |
| **skip driver build** (`PITNODRIVER`) | built, in the runner |
| **sccache on fan hosts** | **built as a gate, deliberately NOT enabled.** Its downside is not slowness but *wrong evidence* — a shared object cache sits underneath the fan's "no tracked `.rs` newer than the binary" attestation. Registered test before it can default: same arm built twice on one host, cache on vs `RUSTC_WRAPPER` empty, binaries **byte-identical** |

## §3 Defender exclusion

**Ben's action, not mine.** Applied by Ben ~21:15 2026-08-18 (Ben-attested).
Consequence recorded and respected: timings against pre-21:15 builds are
cross-condition, so **no Defender claim is made** and the post-exclusion numbers
above are treated as the baseline. Never touched from this side.

## §6's own escape clause, invoked honestly

> *"It is acceptable for this row to close as 'measured: remaining spend is X, no
> lever passes the cost bar' — that is a finding, not a failure."*

The remaining **local** spend is a 19.41 s warm iteration of which 3.3% is link,
and the split does not pass its bar. The remaining **VM** spend is 257 min/day of
which **177 min is recoverable by image freshness alone** — an order of magnitude
above the split's ceiling, and it needs no code change.
