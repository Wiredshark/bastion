//! R12 traversal-contract model checker (standalone; zero non-std deps).
//!
//! Exhaustively explores the ladder-traversal ownership contract's state
//! space and checks safety (S1-S6) + liveness (L1-L2). See RESULTS.md.
//!
//! Usage: cargo run -- [--members 2|3] [--max-depth N] [--break-fence]
//!        [--break-queue] [--break-bound] [--break-revision] [--break-death]

mod checker;
mod model;

use model::Config;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut members = 2usize;
    let mut max_depth: Option<u32> = None;
    let mut breaks: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--members" => {
                members = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&n| (2..=3).contains(&n))
                    .unwrap_or_else(|| {
                        eprintln!("--members takes 2 or 3");
                        std::process::exit(2)
                    });
            },
            "--max-depth" => {
                max_depth = args.next().and_then(|v| v.parse().ok());
            },
            b @ ("--break-fence" | "--break-queue" | "--break-bound" | "--break-revision"
            | "--break-death") => breaks.push(b.to_string()),
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::from(2);
            },
        }
    }

    let mut cfg = Config::faithful(members);
    for b in &breaks {
        match b.as_str() {
            "--break-fence" => cfg.epoch_fence = false,
            "--break-queue" => cfg.fair_queue = false,
            "--break-bound" => cfg.reengage_bound = false,
            "--break-revision" => cfg.revision_guard = false,
            "--break-death" => cfg.death_releases = false,
            _ => unreachable!(),
        }
    }

    let label = if breaks.is_empty() {
        format!("faithful contract, {members} members")
    } else {
        format!("{members} members, broken: {}", breaks.join(" "))
    };
    let report = checker::check(&cfg, max_depth);
    checker::print_report(&label, &report);
    if report.violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
