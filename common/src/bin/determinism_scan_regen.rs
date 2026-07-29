//! `E14-4`: the thin CLI wrapper around [`veloren_common::baseline_regen`]
//! -- reads args, resolves the real workspace tree, drives the pure logic
//! against [`veloren_common::determinism_scan::FAMILIES`] and
//! [`veloren_common::scanner_framework::AUTHORITATIVE_SCAN_ROOTS`], and
//! does the actual file I/O (including writing `.new` siblings on
//! refusal). All the DECISIONS live in `baseline_regen.rs`, tested there
//! without touching a filesystem; this file is deliberately dumb.
//!
//! Usage: `cargo run -p veloren-common --bin determinism_scan_regen
//! [--check]`
//!
//! Without `--check`: regenerates every family's baseline file in place
//! where safe (pure additions), writes a `.new` sibling and prints a
//! report where a removal would be required, and leaves every already-
//! up-to-date file untouched.
//!
//! With `--check`: does no writes at all -- reports what WOULD happen,
//! exit code 1 if any family is not `Unchanged` (a CI-friendly staleness
//! check, matching this family's existing `every_family_baseline_is_complete_and_not_stale`
//! test's own intent but exercised against a live regeneration rather
//! than an in-process comparison).

use std::{fs, path::PathBuf};

use veloren_common::{
    baseline_regen::{RegenOutcomeV1, detect_line_ending_v1, parse_baseline_file_v1, regenerate_baseline_v1},
    determinism_scan::FAMILIES,
    scanner_framework::{self, AUTHORITATIVE_SCAN_ROOTS},
};

fn main() {
    let check_only = std::env::args().any(|a| a == "--check");

    let workspace_root = scanner_framework::workspace_root_from_manifest_dir(env!("CARGO_MANIFEST_DIR"));
    let roots: Vec<PathBuf> = AUTHORITATIVE_SCAN_ROOTS.iter().map(|r| workspace_root.join(r)).collect();
    let root_refs: Vec<&std::path::Path> = roots.iter().map(|p| p.as_path()).collect();

    let mut any_stale = false;
    let mut any_refused = false;

    for family in FAMILIES {
        let baseline_path = workspace_root.join("common/src").join(family.baseline_file);
        let existing_content = match fs::read_to_string(&baseline_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}: could not read {} ({e}), skipping", family.name, baseline_path.display());
                continue;
            },
        };
        let existing = parse_baseline_file_v1(&existing_content);
        let line_ending = detect_line_ending_v1(&existing_content);
        let live = scanner_framework::scan_lines_matching(&workspace_root, &root_refs, family.patterns, family.self_exempt);

        match regenerate_baseline_v1(&existing, &live, line_ending) {
            RegenOutcomeV1::Unchanged => {
                println!("{}: unchanged ({} sites)", family.name, live.len());
            },
            RegenOutcomeV1::AppendedInPlace { added, new_content } => {
                any_stale = true;
                println!("{}: {} new site(s) to add", family.name, added.len());
                for (file, snippet, occ) in &added {
                    println!("  + {file} [{occ}]: {snippet}");
                }
                if check_only {
                    println!("  (--check: not writing)");
                } else {
                    fs::write(&baseline_path, new_content).expect("write baseline file");
                    println!("  wrote {}", baseline_path.display());
                }
            },
            RegenOutcomeV1::RefusedRemovalRequired { removed, new_content } => {
                any_stale = true;
                any_refused = true;
                println!(
                    "{}: REFUSED -- {} existing site(s) would need to be removed, review before applying:",
                    family.name,
                    removed.len()
                );
                for (file, snippet, occ) in &removed {
                    println!("  - {file} [{occ}]: {snippet}");
                }
                if !check_only {
                    let new_path = baseline_path.with_extension("rs.new");
                    fs::write(&new_path, new_content).expect("write .new file");
                    println!("  wrote {} for hand review -- the real baseline file was NOT touched", new_path.display());
                }
            },
        }
    }

    if check_only && any_stale {
        eprintln!("\n--check: at least one family's baseline is stale");
        std::process::exit(1);
    }
    if any_refused {
        eprintln!("\nAt least one family needs a hand-reviewed merge -- see the .new file(s) above.");
    }
}
