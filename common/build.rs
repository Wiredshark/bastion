use regex::Regex;
use std::process::Command;

// Get the current githash+timestamp
// Note: It will compare commits. As long as the commits do not diverge from the
// server no version change will be detected.
fn get_git_hash_timestamp() -> Result<String, String> {
    let output = Command::new("git")
        .args(["log", "-n", "1", "--pretty=format:%h/%ct", "--abbrev=8"])
        .output()
        .map_err(|e| format!("Git version command couldn't be run with error: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Git version command was unsuccessful: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let hash_timestamp = String::from_utf8(output.stdout)
        .map_err(|e| format!("Git version command output isn't valid UTF-8: {}", e))?;
    let hash = hash_timestamp
        .split('/')
        .next()
        .ok_or("Git hash not found".to_string())?;
    // We only use the first 32 bits of the git hash
    if hash.len() != 8 {
        Ok(format!(
            "{}/{}",
            hash.get(..8)
                .ok_or("Git hash not long enough".to_string())?,
            hash_timestamp
                .split('/')
                .nth(1)
                .ok_or("Git timestamp not found".to_string())?
        ))
    } else {
        Ok(hash_timestamp)
    }
}

// Get the current gittag
fn get_git_tag() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let tag = String::from_utf8(output.stdout).ok()?.trim().to_string();

    if Regex::new(r"^v[0-9]+\.[0-9]+\.[0-9]+$")
        .unwrap()
        .is_match(&tag)
    {
        Some(tag)
    } else {
        None
    }
}

// Resolve a path inside the real git-dir (handles worktrees correctly:
// `--git-path` returns the worktree-specific file, e.g.
// `.git/worktrees/<name>/HEAD`, not the main checkout's `.git/HEAD`).
fn git_path(subpath: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-path", subpath])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

fn main() {
    // Without an explicit rerun-if-changed, cargo falls back to watching
    // every file under this crate's own directory -- a commit that only
    // touches OTHER crates (the common case: this baked-in version is read
    // by server/client binaries elsewhere in the workspace) never triggers
    // a rerun, so VELOREN_GIT_VERSION silently goes stale. Watch the actual
    // git-dir files that change on every commit/checkout instead.
    if let Some(head) = git_path("HEAD") {
        println!("cargo::rerun-if-changed={head}");
    }
    if let Some(logs_head) = git_path("logs/HEAD") {
        println!("cargo::rerun-if-changed={logs_head}");
    }

    // If this env var exists, it'll be used instead
    if option_env!("VELOREN_GIT_VERSION").is_none() {
        let hash_timestamp = match get_git_hash_timestamp() {
            Ok(hash_timestamp) => hash_timestamp,
            Err(e) => {
                println!("cargo::error={}", e);
                println!(
                    "cargo::error=It is highly recommended to build Veloren from the cloned git \
                     repository with the git command available in order to give the game access \
                     to proper versioning information."
                );
                println!(
                    "cargo::error=However, if you wish to proceed building Veloren anyway, you \
                     can set the environment variable \"VELOREN_GIT_VERSION\" to \"/0/0\" before \
                     re-running the given cargo command (the specific procedure for this will \
                     depend on your shell). Note that this will compile the game with git commit \
                     hash and commit timestamp set to 0, which will cause version mismatch \
                     warnings where applicable, whether the version is actually mismatched or not."
                );
                return;
            },
        };

        let tag = get_git_tag().unwrap_or("".to_string());

        // Format: <git-tag?>/<git-hash>/<git-timestamp>
        println!(
            "cargo::rustc-env=VELOREN_GIT_VERSION={}/{}",
            tag, hash_timestamp
        );
    }
}
