# Project Bastion — Baseline Record (B0)

## Pinned upstream baseline

| | |
|---|---|
| **Upstream** | `gitlab.com/veloren/veloren`, branch `master` |
| **Upstream SHA** | `bfef92fcb33e7e610ba24fecd5920d0c0e227221` |
| **Upstream commit date** | 2026-07-07T08:20:06Z ("Merge branch 'uncomfy/st_presets' into 'master'") |
| **Workspace version** | 0.18.0 (dev) |
| **Snapshot date** | 2026-07-07 (source archive; recorded 2026-07-08) |
| **Local baseline commit** | `2f789af`, tag `bastion-baseline`, branch `master` |
| **Working branch** | `bastion/main` |

### Provenance caveat (important)

`E:\veloren-master` began life as a **source archive of upstream master, not a git clone** — it had
no `.git` directory. B0 initialized a fresh local git repo and committed the snapshot as-is
(`bastion-baseline`). The upstream SHA above was recovered afterwards and verified by:

1. Archive file mtimes (2026-07-07 08:20 UTC) exactly matching the upstream commit timestamp, and
2. Byte-identical SHA-256 content matches against that commit for `Cargo.lock` (highly
   discriminating), `CHANGELOG.md`, `server/src/lib.rs`, and `rust-toolchain`, fetched via the
   GitLab raw-file API.

Confidence is very high, but the local repo shares **no git history** with upstream. Before any
future upstream merge (a stated project goal), re-graft history: `git fetch upstream master`
(remote `upstream` is already configured), verify `git diff bastion-baseline bfef92fc…` is limited
to Bastion-added files, then rebase `bastion/main` onto the real upstream commit. Until then, all
Bastion work stays additive and namespaced so that graft stays trivial.

The baseline commit intentionally includes two non-upstream files that were already in the tree:
the design doc at `readme/veloren-colony-rts-build-report.md` and its copy at
`nix/veloren-colony-rts-build-report.md`.

### git-lfs note

Upstream declares LFS patterns in `.gitattributes`, but this snapshot already contains the **real
binary contents** (verified), so no `git lfs pull` is needed or possible. The local repo is
configured with no-op LFS filters (`filter.lfs.clean/smudge = cat`, `required = false`) so the
binaries are stored as plain git blobs locally. A future push to a GitLab fork should either keep
that (files are ≤16 MB) or re-run `git lfs migrate` at that point.

## Toolchain (installed by B0 on this machine — none was present)

| Component | Version / location |
|---|---|
| rustup | per-user, `C:\Users\q\.cargo\bin`, default host `x86_64-pc-windows-gnu` |
| Rust toolchain | `nightly-2026-06-13-x86_64-pc-windows-gnu` (pinned by repo `rust-toolchain` file; auto-selected in-repo) |
| C toolchain | mingw-w64 GCC 16.1.0 (WinLibs UCRT/POSIX/SEH r3) at `C:\Users\q\toolchains\mingw64\bin` (on user `Path`) |

**Why `windows-gnu` and not MSVC:** the machine had no Visual Studio Build Tools and no admin
elevation available; the GNU toolchain installs entirely per-user. It is also exactly the target
upstream CI uses for official Windows releases (`.gitlab/scripts/windows-x86_64.sh` builds
`--target=x86_64-pc-windows-gnu`), and the repo carries gnu-specific rustflags in
`.cargo/config.toml` (`-lpsapi` for mimalloc), so it is a first-class, CI-tested configuration.

## How to build

```powershell
# from E:\veloren-master (rust-toolchain auto-selects the pinned nightly)
cargo build --bin veloren-voxygen --bin veloren-server-cli
```

Requirements: git repo present (`common/build.rs` shells out to `git log`; escape hatch:
`VELOREN_GIT_VERSION=/0/0`), mingw-w64 `gcc` on PATH (bundled sqlite via `libsqlite3-sys`).

## How to run vanilla

```powershell
target\debug\veloren-server-cli.exe --non-interactive   # headless dedicated server
target\debug\veloren-voxygen.exe                        # client; Singleplayer for local world
```

## How to run the Bastion harness

See `docs/BASTION_HARNESS.md`. Quick version:

```powershell
cargo run -p bastion-harness -- --seed 1337 --ticks 1000          # single run, JSON dump
cargo run -p bastion-harness -- --seed 1337 --ticks 1000 --verify # determinism self-check
scripts\bastion-check.ps1                                          # fast cargo check inner loop
```

## Determinism status

**DIVERGED-BY-DESIGN at exact-trajectory level; aggregate determinism measured by the harness.**
rtsim's per-tick rules seed their RNGs from OS entropy, not the world seed
(`rtsim/src/rule/npc_ai/mod.rs:179`, `migrate.rs:26`, `cleanup.rs:21`) — see
`docs/BASTION_B0_FINDINGS.md` §4. Current measured verdict for the standard check
(seed 1337, 1000 ticks, aggregate counts): recorded in `BASTION.md` after each harness change.
