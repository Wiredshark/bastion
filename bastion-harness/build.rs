// bastion: stamp the harness with the git sha + build time so every test
// log entry can be tied to the exact exe that produced it (the stale-exe
// incident: results from a pre-merge exe were read as post-merge green).
use std::process::Command;

fn main() {
    // DET-BLD-023 (v6 deep-pass, Critical): ambient-RUSTFLAGS witness. An
    // environment RUSTFLAGS OVERRIDES the repository target rustflags
    // entirely (cargo precedence), silently changing codegen/link behavior
    // of the certified binary. Detect and warn LOUDLY so no certified build
    // carries invisible ambient flags; the VM/cert lane treats the warning
    // as a red flag.
    for var in ["RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS"] {
        if let Ok(flags) = std::env::var(var) {
            if !flags.trim().is_empty() {
                println!(
                    "cargo:warning=DET-BLD-023: ambient {var}={flags:?} overrides the                      repository target rustflags — this build is NOT flag-canonical"
                );
            }
        }
    }

    // Re-stamp on ANY new commit / checkout move — without this, a commit
    // touching only OTHER crates leaves the stamp stale while the exe is
    // fresh (the --print-git-hash pre-flight would false-alarm), and the
    // reverse staleness defeated the guard's purpose entirely (the flag's
    // first live test printed a 3-commit-old hash). HEAD covers branch
    // moves; the index covers the dirty-flag's freshness. Worktree-safe
    // via --absolute-git-dir.
    if let Some(git_dir) = Command::new("git")
        .args(["rev-parse", "--absolute-git-dir"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
    let sha = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = Command::new("git")
        // untracked scratch (e.g. tools/__pycache__) is not dirty CODE
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    println!(
        "cargo:rustc-env=BASTION_BUILD_SHA={}{}",
        sha,
        if dirty { "+dirty" } else { "" }
    );
    println!(
        "cargo:rustc-env=BASTION_BUILD_TIME={}",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
    // re-stamp when HEAD moves. NOT the HEAD file — that's a ref POINTER
    // that only changes on checkout (commits move refs/heads/*, the stamp
    // went 10 commits stale that way). The reflog appends on EVERY commit,
    // so track logs/HEAD (worktree-safe via --git-path).
    if let Some(log) = Command::new("git")
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "logs/HEAD",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        println!("cargo:rerun-if-changed={log}");
    }
}
