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
        /// W4: which semantic domains diverged at that frame (empty when
        /// the tapes lack domain maps, or when only the token moved).
        divergent_domains: Vec<String>,
    },
    /// Either file unreadable/malformed — fail closed, never "no golden".
    Malformed(String),
}

/// W4: per-frame domain roots when the tape carries them (older tapes
/// don't — absence compares as absent, never as a mismatch by itself).
type FrameEntry = (u64, String, Option<[(String, String); 3]>);

fn load(path: &Path) -> Result<(String, Vec<FrameEntry>), String> {
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
            let domains = f.get("domains").and_then(|d| {
                Some([
                    ("script".to_string(), d.get("script")?.as_str()?.to_string()),
                    ("movement".to_string(), d.get("movement")?.as_str()?.to_string()),
                    ("identity".to_string(), d.get("identity")?.as_str()?.to_string()),
                ])
            });
            Ok((
                f["tick"]
                    .as_u64()
                    .ok_or_else(|| format!("{}: frame without tick", path.display()))?,
                f["frame_root"]
                    .as_str()
                    .ok_or_else(|| format!("{}: frame without root", path.display()))?
                    .to_string(),
                domains,
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
    let mut divergent_domains: Vec<String> = Vec::new();
    for (i, (c, g)) in cand.1.iter().zip(gold.1.iter()).enumerate() {
        if c.1 != g.1 {
            first_divergent_index = Some(i);
            first_divergent_tick = Some(c.0);
            // W4: NAME the divergent domain(s) when both tapes carry the
            // per-domain roots (a frame_root can differ via the token
            // alone — then the list is empty and says so).
            if let (Some(cd), Some(gd)) = (&c.2, &g.2) {
                for ((name, cv), (_, gv)) in cd.iter().zip(gd.iter()) {
                    if cv != gv {
                        divergent_domains.push(name.clone());
                    }
                }
            }
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
        divergent_domains,
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
            divergent_domains,
        } => {
            println!(
                "renderer-bench golden: MISMATCH candidate={candidate_root} golden={golden_root} \
                 first_divergent_frame={first_divergent_index:?} tick={first_divergent_tick:?} \
                 divergent_domains={divergent_domains:?}"
            );
            1
        },
        GoldenVerdict::Malformed(e) => {
            println!("renderer-bench golden: MALFORMED — {e}");
            2
        },
    }
}

/// W6 — privileged golden promotion (`--renderer-bench-promote`).
///
/// Promotion is a HUMAN-ATTESTED action: it refuses without `--attest
/// "<who/why>"`, refuses a malformed candidate, and every promotion
/// appends an audit line (`PROMOTIONS.log` beside the golden) carrying
/// the candidate's sha256, its run_root, and the attestation — so a
/// blessed reference can always answer "who blessed you, and why".
pub fn promote_cli(candidate: &str, golden: &str, attest: Option<&str>) -> i32 {
    let Some(attest) = attest.filter(|a| !a.trim().is_empty()) else {
        println!(
            "renderer-bench promote: REFUSED — promotion requires --attest \"<who/why>\" \
             (an unattested golden is an unaccountable oracle)"
        );
        return 2;
    };
    let cand_path = Path::new(candidate);
    let (run_root, frames) = match load(cand_path) {
        Ok((r, f)) => (r, f.len()),
        Err(e) => {
            println!("renderer-bench promote: REFUSED — candidate malformed: {e}");
            return 2;
        },
    };
    let bytes = match std::fs::read(cand_path) {
        Ok(b) => b,
        Err(e) => {
            println!("renderer-bench promote: REFUSED — {e}");
            return 2;
        },
    };
    let sha = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&bytes);
        let d = h.finalize();
        d.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let golden_path = Path::new(golden);
    if let Some(parent) = golden_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let replaced = golden_path.exists();
    if let Err(e) = std::fs::write(golden_path, &bytes) {
        println!("renderer-bench promote: FAILED writing golden: {e}");
        return 2;
    }
    let ledger = golden_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("PROMOTIONS.log");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!(
        "epoch={stamp} golden={} candidate_sha256={sha} run_root={run_root} frames={frames} \
         replaced={replaced} attest={attest}\n",
        golden_path.display()
    );
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger)
        .and_then(|mut f| {
            use std::io::Write as _;
            f.write_all(line.as_bytes())
        })
    {
        println!(
            "renderer-bench promote: golden written but LEDGER FAILED ({e}) — treat the \
             promotion as VOID and redo it"
        );
        return 2;
    }
    println!(
        "renderer-bench promote: PROMOTED sha256={sha} run_root={run_root} frames={frames} \
         replaced={replaced} ledger={}",
        ledger.display()
    );
    0
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

    /// W4: a tape with per-frame domain roots.
    fn tape_with_domains(
        dir: &std::path::Path,
        name: &str,
        root: &str,
        frames: &[(u64, &str, [&str; 3])],
    ) -> std::path::PathBuf {
        let f: Vec<String> = frames
            .iter()
            .map(|(t, r, d)| {
                format!(
                    "{{\"tick\":{t},\"token\":\"00\",\"frame_root\":\"{r}\",\"domains\":{{\"script\":\"{}\",\"movement\":\"{}\",\"identity\":\"{}\"}}}}",
                    d[0], d[1], d[2]
                )
            })
            .collect();
        let body = format!(
            "{{\"schema\":\"renderer-bench-tape-v1\",\"scenario_id\":\"t\",\"manifest_payload_sha256\":\"aa\",\"manifest_domain_sha256\":\"bb\",\"cadence\":30,\"frames\":[{}],\"run_root\":\"{root}\",\"terminal_count\":0}}",
            f.join(",")
        );
        let p = dir.join(name);
        std::fs::write(&p, body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn w4_mismatch_names_the_divergent_domain() {
        let dir = std::env::temp_dir().join("rbench-golden-w4-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let a = tape_with_domains(&dir, "w4a.json", "rA", &[
            (0, "f0", ["s0", "m0", "i0"]),
            (30, "f1", ["s1", "m1", "i1"]),
        ]);
        let b = tape_with_domains(&dir, "w4b.json", "rB", &[
            (0, "f0", ["s0", "m0", "i0"]),
            (30, "fX", ["s1", "mX", "i1"]),
        ]);
        match compare(&a, &b) {
            GoldenVerdict::Mismatch {
                first_divergent_index,
                divergent_domains,
                ..
            } => {
                assert_eq!(first_divergent_index, Some(1));
                assert_eq!(divergent_domains, vec!["movement".to_string()]);
            },
            other => panic!("expected mismatch, got {other:?}"),
        }
        // Old tapes without domain maps still compare — attribution empty.
        let c = tape(&dir, "w4c.json", "rC", &[(0, "f0"), (30, "fY")]);
        match compare(&a, &c) {
            GoldenVerdict::Mismatch { divergent_domains, .. } => {
                assert!(divergent_domains.is_empty());
            },
            other => panic!("expected mismatch, got {other:?}"),
        }
    }

    #[test]
    fn w6_promotion_refuses_unattested_and_audits_when_attested() {
        let dir = std::env::temp_dir().join("rbench-golden-w6-tests");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cand = tape(&dir, "cand.json", "rP", &[(0, "f0")]);
        let gold = dir.join("store").join("golden.json");

        // No attestation → refused, nothing written.
        assert_eq!(promote_cli(cand.to_str().unwrap(), gold.to_str().unwrap(), None), 2);
        assert_eq!(
            promote_cli(cand.to_str().unwrap(), gold.to_str().unwrap(), Some("  ")),
            2
        );
        assert!(!gold.exists());

        // Malformed candidate → refused even WITH attestation.
        let bad = dir.join("bad.json");
        std::fs::write(&bad, b"nope").unwrap();
        assert_eq!(
            promote_cli(bad.to_str().unwrap(), gold.to_str().unwrap(), Some("test")),
            2
        );
        assert!(!gold.exists());

        // Attested → golden written byte-identical + audit line appended.
        assert_eq!(
            promote_cli(cand.to_str().unwrap(), gold.to_str().unwrap(), Some("w6 test")),
            0
        );
        assert_eq!(std::fs::read(&gold).unwrap(), std::fs::read(&cand).unwrap());
        let ledger = std::fs::read_to_string(dir.join("store").join("PROMOTIONS.log")).unwrap();
        assert!(ledger.contains("run_root=rP"));
        assert!(ledger.contains("attest=w6 test"));
        assert!(ledger.contains("replaced=false"));

        // Re-promotion records replaced=true.
        assert_eq!(
            promote_cli(cand.to_str().unwrap(), gold.to_str().unwrap(), Some("again")),
            0
        );
        let ledger = std::fs::read_to_string(dir.join("store").join("PROMOTIONS.log")).unwrap();
        assert!(ledger.contains("replaced=true"));
    }
}
