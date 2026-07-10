// bastion: stamp the harness with the git sha + build time so every test
// log entry can be tied to the exact exe that produced it (the stale-exe
// incident: results from a pre-merge exe were read as post-merge green).
use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
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
        .args(["rev-parse", "--path-format=absolute", "--git-path", "logs/HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        println!("cargo:rerun-if-changed={log}");
    }
}
