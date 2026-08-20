//! renderer-bench golden policy (fork build): compare a candidate semantic
//! tape against a golden baseline.
//!
//! W0 invariants honored here:
//! - "production encoder cannot regenerate or bless expected vector bytes" —
//!   generalized: THE PRODUCER NEVER PROMOTES. A missing golden yields
//!   `NoGolden` and the candidate stays a candidate; promotion (copying a
//!   candidate over the golden path) is a HUMAN action, deliberately not
//!   implemented as code.
//! - fail closed: an unreadable or malformed tape on either side is a
//!   verdict of its own (`Malformed`), never treated as "no golden".
//!
//! The comparison is semantic-root-first: run_root equality decides; on
//! mismatch, the first diverging frame is named (tick + index) so the
//! investigation starts AT the divergence, not at a byte offset.

use serde_json::Value;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum GoldenVerdict {
    /// run_root identical (frame count too — a prefix match is a mismatch).
    Pass,
    /// Golden absent: candidate recorded, promotion is a human action.
    NoGolden,
    /// Roots differ; the first diverging frame is named.
    Mismatch {
        first_divergent_index: Option<usize>,
        first_divergent_tick: Option<u64>,
        candidate_root: String,
        golden_root: String,
    },
    /// Either file unreadable/malformed — fail closed, never "no golden".
    Malformed(String),
}

fn load(path: &Path) -> Result<(String, Vec<(u64, String)>), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: Value =
        serde_json::from_str(&raw).map_err(|e| format!("{}: bad JSON: {e}", path.display()))?;
    if v["schema"].as_str() != Some("renderer-bench-tape-v1") {
        return Err(format!("{}: wrong schema", path.display()));
    }
    let root = v["run_root"]
        .as_str()
        .ok_or_else(|| format!("{}: missing run_root", path.display()))?
        .to_string();
    let frames = v["frames"]
        .as_array()
        .ok_or_else(|| format!("{}: missing frames", path.display()))?
        .iter()
        .map(|f| {
            Ok((
                f["tick"]
                    .as_u64()
                    .ok_or_else(|| format!("{}: frame without tick", path.display()))?,
                f["frame_root"]
                    .as_str()
                    .ok_or_else(|| format!("{}: frame without root", path.display()))?
                    .to_string(),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((root, frames))
}

pub fn compare(candidate: &Path, golden: &Path) -> GoldenVerdict {
    let cand = match load(candidate) {
        Ok(c) => c,
        Err(e) => return GoldenVerdict::Malformed(e),
    };
    if !golden.exists() {
        return GoldenVerdict::NoGolden;
    }
    let gold = match load(golden) {
        Ok(g) => g,
        Err(e) => return GoldenVerdict::Malformed(e),
    };
    if cand.0 == gold.0 && cand.1.len() == gold.1.len() {
        return GoldenVerdict::Pass;
    }
    let mut first_divergent_index = None;
    let mut first_divergent_tick = None;
    for (i, (c, g)) in cand.1.iter().zip(gold.1.iter()).enumerate() {
        if c.1 != g.1 {
            first_divergent_index = Some(i);
            first_divergent_tick = Some(c.0);
            break;
        }
    }
    if first_divergent_index.is_none() && cand.1.len() != gold.1.len() {
        let i = cand.1.len().min(gold.1.len());
        first_divergent_index = Some(i);
        first_divergent_tick = cand.1.get(i).or_else(|| gold.1.get(i)).map(|f| f.0);
    }
    GoldenVerdict::Mismatch {
        first_divergent_index,
        first_divergent_tick,
        candidate_root: cand.0,
        golden_root: gold.0,
    }
}

/// CLI entry: `--renderer-bench-golden <candidate> <golden>`.
/// Exit semantics: PASS → 0, NoGolden → 3 (distinct from failure so runners
/// can branch), Mismatch → 1, Malformed → 2.
pub fn run_cli(candidate: &str, golden: &str) -> i32 {
    match compare(Path::new(candidate), Path::new(golden)) {
        GoldenVerdict::Pass => {
            println!("renderer-bench golden: PASS (run_root identical)");
            0
        },
        GoldenVerdict::NoGolden => {
            println!(
                "renderer-bench golden: NO GOLDEN at {golden} — candidate stays a candidate; \
                 promotion is a human action (copy it there yourself)"
            );
            3
        },
        GoldenVerdict::Mismatch {
            first_divergent_index,
            first_divergent_tick,
            candidate_root,
            golden_root,
        } => {
            println!(
                "renderer-bench golden: MISMATCH candidate={candidate_root} golden={golden_root} \
                 first_divergent_frame={first_divergent_index:?} tick={first_divergent_tick:?}"
            );
            1
        },
        GoldenVerdict::Malformed(e) => {
            println!("renderer-bench golden: MALFORMED — {e}");
            2
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn tape(dir: &std::path::Path, name: &str, root: &str, frames: &[(u64, &str)]) -> std::path::PathBuf {
        let f: Vec<String> = frames
            .iter()
            .map(|(t, r)| format!("{{\"tick\":{t},\"token\":\"00\",\"frame_root\":\"{r}\"}}"))
            .collect();
        let body = format!(
            "{{\"schema\":\"renderer-bench-tape-v1\",\"scenario_id\":\"t\",\"manifest_payload_sha256\":\"aa\",\"manifest_domain_sha256\":\"bb\",\"cadence\":30,\"frames\":[{}],\"run_root\":\"{root}\",\"terminal_count\":0}}",
            f.join(",")
        );
        let p = dir.join(name);
        let mut fh = std::fs::File::create(&p).unwrap();
        fh.write_all(body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn pass_no_golden_mismatch_and_malformed() {
        let dir = std::env::temp_dir().join("rbench-golden-test");
        let _ = std::fs::create_dir_all(&dir);
        let a = tape(&dir, "a.json", "r1", &[(0, "f0"), (30, "f1")]);
        let b = tape(&dir, "b.json", "r1", &[(0, "f0"), (30, "f1")]);
        assert_eq!(compare(&a, &b), GoldenVerdict::Pass);

        let missing = dir.join("nope.json");
        let _ = std::fs::remove_file(&missing);
        assert_eq!(compare(&a, &missing), GoldenVerdict::NoGolden);

        let c = tape(&dir, "c.json", "r2", &[(0, "f0"), (30, "fX")]);
        match compare(&a, &c) {
            GoldenVerdict::Mismatch {
                first_divergent_index,
                first_divergent_tick,
                ..
            } => {
                assert_eq!(first_divergent_index, Some(1));
                assert_eq!(first_divergent_tick, Some(30));
            },
            other => panic!("expected mismatch, got {other:?}"),
        }

        let bad = dir.join("bad.json");
        std::fs::write(&bad, b"not json").unwrap();
        assert!(matches!(compare(&bad, &a), GoldenVerdict::Malformed(_)));
        // Malformed GOLDEN is Malformed too — never mistaken for NoGolden.
        assert!(matches!(compare(&a, &bad), GoldenVerdict::Malformed(_)));
    }
}
