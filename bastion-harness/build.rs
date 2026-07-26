// bastion: stamp the harness with the source revision + build time so every
// test log entry can be tied to the exact exe that produced it (the stale-exe
// incident: results from a pre-merge exe were read as post-merge green).
//
// APEX-T1.1.02 (DET-BLD-019/029 lane): ENVIRONMENT-FIRST identity. The stamp
// is a pure function of DECLARED inputs in the certified lane — never `.git`
// state, never the wall clock — so a Nix store build (no `.git`, sandboxed
// clock) is self-identifying and reproducible-by-construction:
//
//   BASTION_BUILD_LANE=apex-nix-v1   selects the certified lane (fail-closed)
//   BASTION_SOURCE_REVISION          full 40-lower-hex commit  (required there)
//   SOURCE_DATE_EPOCH                source-derived UTC seconds (required there)
//
// Modes (DeclaredBuildIdentityV1.mode):
//   DeclaredCertified        declared revision + epoch (apex-nix-v1 lane)
//   GitDeveloperFallback     legacy `.git` + current-clock stamp (dev builds)
//   UnknownDeveloperFallback no declared input and no usable `.git`
//
// The certified lane REFUSES to build on missing/invalid inputs rather than
// emitting `unknown` or the current clock (T1.1-BLOCK-AMBIENT-TIME /
// T1.1-BLOCK-UNKNOWN-REVISION are build failures here, not warnings).
use std::process::Command;

fn fail(msg: &str) -> ! {
    // A cargo:warning is ignorable; a certified-lane identity violation must
    // stop the build (fail-closed, packet APEX-T1.1 §6.6).
    panic!("APEX-T1.1.02: {msg}");
}

fn declared_revision() -> Option<String> {
    let rev = std::env::var("BASTION_SOURCE_REVISION").ok()?;
    let rev = rev.trim().to_string();
    if rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        Some(rev)
    } else {
        fail(&format!(
            "BASTION_SOURCE_REVISION must be 40 lowercase hex chars, got {rev:?}"
        ));
    }
}

fn declared_epoch() -> Option<i64> {
    let raw = std::env::var("SOURCE_DATE_EPOCH").ok()?;
    match raw.trim().parse::<i64>() {
        Ok(secs) if secs >= 0 => Some(secs),
        _ => fail(&format!(
            "SOURCE_DATE_EPOCH must be a non-negative integer of UTC seconds, got {raw:?}"
        )),
    }
}

fn epoch_to_utc_string(secs: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_else(|| fail(&format!("SOURCE_DATE_EPOCH {secs} is out of range")))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

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
                    "cargo:warning=DET-BLD-023: ambient {var}={flags:?} overrides the \
                     repository target rustflags — this build is NOT flag-canonical"
                );
            }
        }
    }

    // Declared inputs re-stamp on change (T1.1.02: explicit + rerun-tracked).
    println!("cargo:rerun-if-env-changed=BASTION_BUILD_LANE");
    println!("cargo:rerun-if-env-changed=BASTION_SOURCE_REVISION");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let lane = std::env::var("BASTION_BUILD_LANE").unwrap_or_default();
    let certified = lane == "apex-nix-v1";

    let (mode, full_rev, sha_display, build_time) = match (declared_revision(), declared_epoch()) {
        // ── DeclaredCertified: identity is a pure function of declared env ──
        (Some(rev), Some(epoch)) => {
            let display = rev[..10].to_string();
            (
                "DeclaredCertified",
                rev.clone(),
                display,
                epoch_to_utc_string(epoch),
            )
        },
        // Certified lane with a missing half: fail closed, never fall back.
        (Some(_), None) if certified => fail(
            "BASTION_BUILD_LANE=apex-nix-v1 requires SOURCE_DATE_EPOCH (declared source epoch)",
        ),
        (None, Some(_)) if certified => fail(
            "BASTION_BUILD_LANE=apex-nix-v1 requires BASTION_SOURCE_REVISION (full 40-hex commit)",
        ),
        (None, None) if certified => fail(
            "BASTION_BUILD_LANE=apex-nix-v1 requires BASTION_SOURCE_REVISION and \
             SOURCE_DATE_EPOCH; refusing to stamp from .git or the wall clock",
        ),
        // ── Developer fallback: legacy .git + wall-clock stamp (noncertified) ──
        _ => {
            // Re-stamp on ANY new commit / checkout move — without this, a
            // commit touching only OTHER crates leaves the stamp stale while
            // the exe is fresh (the --print-git-hash pre-flight would
            // false-alarm). HEAD covers branch moves; the index covers the
            // dirty-flag's freshness. Worktree-safe via --absolute-git-dir.
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
            // The reflog appends on EVERY commit, so track logs/HEAD (a ref
            // POINTER alone only changes on checkout; the stamp went 10
            // commits stale that way). Worktree-safe via --git-path.
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
            let full = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
            let dirty = Command::new("git")
                // untracked scratch (e.g. tools/__pycache__) is not dirty CODE
                .args(["status", "--porcelain", "--untracked-files=no"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| !o.stdout.is_empty())
                .unwrap_or(false);
            match full {
                Some(full) => {
                    let display =
                        format!("{}{}", &full[..10], if dirty { "+dirty" } else { "" });
                    (
                        "GitDeveloperFallback",
                        format!("{full}{}", if dirty { "+dirty" } else { "" }),
                        display,
                        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    )
                },
                None => (
                    "UnknownDeveloperFallback",
                    "unknown".to_string(),
                    "unknown".to_string(),
                    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                ),
            }
        },
    };

    // Legacy 10-char display projection (existing --print-git-hash CLI checks
    // compare `${H%%+*}` against `git rev-parse --short=10`); full identity is
    // retained separately for future manifests (APEX-T1.5).
    println!("cargo:rustc-env=BASTION_BUILD_SHA={sha_display}");
    println!("cargo:rustc-env=BASTION_BUILD_TIME={build_time}");
    println!("cargo:rustc-env=BASTION_BUILD_REVISION_FULL={full_rev}");
    println!("cargo:rustc-env=BASTION_BUILD_IDENTITY_MODE={mode}");
    println!(
        "cargo:rustc-env=BASTION_BUILD_LANE={}",
        if certified { "apex-nix-v1" } else { "developer" }
    );
}
