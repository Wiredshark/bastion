#!/usr/bin/env python3
"""APEX-T1.3 orchestrator — same-worker exact-output rebuild + host-path
impurity smoke (real packet, minute steps T1.3.03-T1.3.12).

Runs INSIDE an A.1-admitted checkout on the nix lane. All canonical
crypto (path tokens, roots, CBOR, PASS admission) lives in the Rust
`apex_local_repro_record` bin — this script only executes builds and
gathers raw evidence.

Typed terminals are printed as `TERMINAL: T1.3-BLOCK-*`; the final
canonical verdict comes from the record bin.

Divergence from packet command sketches, documented: `--offline` is NOT
passed on dependency-fetching builds — immutable dependency substitution
is explicitly allowed (packet policy 3/10.4); the FINAL derivation has
`allowSubstitutes = false` in the flake, which no network flag can
override.
"""
import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone

LOCK_PATH = os.environ.get("APEX_RUST_BUILD_LOCK", "/tmp/bastion-apex-rust-build.lock")
INSTALLABLE = "bastion-harness-repro"
CANARIES = {
    "stable": ("apex-repro-canary-stable", True),   # expected: rebuild EQUAL
    "time": ("apex-repro-canary-time", False),
    "random": ("apex-repro-canary-random", False),
    "tmppath": ("apex-repro-canary-tmppath", False),
}


def now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def die(terminal: str, detail: str) -> "NoReturn":
    print(detail, file=sys.stderr)
    print(f"TERMINAL: T1.3-{terminal}")
    sys.exit(11)


def run(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest(), os.path.getsize(path)


def sri_to_hex(sri: str) -> str:
    import base64
    assert sri.startswith("sha256-"), sri
    return base64.b64decode(sri[len("sha256-"):]).hex()


def nix(args, log_path=None):
    started = now()
    r = run(["nix", *args])
    finished = now()
    if log_path:
        with open(log_path, "w") as f:
            f.write(r.stdout + "\n--- stderr ---\n" + r.stderr)
    return r, started, finished


def acquire_lock():
    """T1.3.03: one machine-wide advisory lock around ALL Rust builds."""
    f = open(LOCK_PATH, "w")
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        die("BLOCK-CONCURRENT-BUILD", f"another whole-build holds {LOCK_PATH}")
    f.write(f"pid={os.getpid()} started={now()}\n")
    f.flush()
    return f  # held for process lifetime


def materialize(repo_root, commit, label):
    """T1.3.04: randomized-path worktree of the admitted commit."""
    path = tempfile.mkdtemp(prefix=f"apex-t13-{label}-")
    dest = os.path.join(path, "src")
    r = run(["git", "-C", repo_root, "worktree", "add", "--detach", dest, commit])
    if r.returncode != 0:
        die("BLOCK-MATERIALIZATION" if False else "BLOCK-SOURCE-CLOSURE",
            f"worktree add {label} failed: {r.stderr}")
    return dest


def evaluate_drv(src):
    """T1.3.05: pure evaluation → drv path + derivation JSON."""
    r, _, _ = nix(["path-info", "--derivation", f"{src}#{INSTALLABLE}", "--no-write-lock-file"])
    if r.returncode != 0:
        die("BLOCK-IMPURE-EVALUATION", f"drv evaluation failed at {src}: {r.stderr}")
    drv = r.stdout.strip()
    r2, _, _ = nix(["derivation", "show", f"{src}#{INSTALLABLE}", "--no-write-lock-file"])
    if r2.returncode != 0:
        die("BLOCK-IMPURE-EVALUATION", f"derivation show failed: {r2.stderr}")
    return drv, r2.stdout


def leaf_manifest(out_path):
    """T1.3.09: sorted full-output leaf inventory."""
    leaves = []
    for root, dirs, files in os.walk(out_path):
        dirs.sort()
        for name in sorted(files):
            p = os.path.join(root, name)
            rel = os.path.relpath(p, out_path).replace(os.sep, "/")
            st = os.lstat(p)
            if os.path.islink(p):
                leaves.append({"path": rel, "kind": "symlink", "mode": st.st_mode & 0o7777,
                               "target": os.readlink(p)})
            else:
                digest, size = sha256_file(p)
                leaves.append({"path": rel, "kind": "file", "mode": st.st_mode & 0o7777,
                               "size": size, "sha256": digest})
    if not leaves:
        die("BLOCK-OUTPUT-INVENTORY", f"empty output at {out_path}")
    return leaves


def rebuild_check(src, evidence_dir, ordinal, hook):
    log = os.path.join(evidence_dir, f"rebuild-{ordinal}.log")
    r, started, finished = nix([
        "build", f"{src}#{INSTALLABLE}", "--rebuild", "--keep-failed", "--no-link",
        "--no-write-lock-file", "--max-jobs", "1",
        "--option", "run-diff-hook", "true", "--option", "diff-hook", hook,
    ], log_path=log)
    digest, size = sha256_file(log)
    return {
        "ordinal": ordinal, "kind": "rebuild", "locally_executed": True,
        "source_path": src, "out_link_path": f"{src}#no-link-{ordinal}",
        "exit_code": r.returncode, "log_sha256": digest, "log_size": size,
        "started_at": started, "finished_at": finished,
    }, r.returncode


def canary_campaign(src, evidence_dir, hook):
    """T1.3.11: stable must survive --rebuild; the bad three must not."""
    results = []
    for cid, (pkg, expect_equal) in sorted(CANARIES.items()):
        b, _, _ = nix(["build", f"{src}#{pkg}", "--no-link", "--no-write-lock-file"])
        if b.returncode != 0:
            die("BLOCK-CANARY-FALSE-POSITIVE", f"canary {cid} failed to build: {b.stderr}")
        log = os.path.join(evidence_dir, f"canary-{cid}.log")
        r, _, _ = nix(["build", f"{src}#{pkg}", "--rebuild", "--keep-failed", "--no-link",
                       "--no-write-lock-file",
                       "--option", "run-diff-hook", "true", "--option", "diff-hook", hook],
                      log_path=log)
        equal = r.returncode == 0
        ok = equal == expect_equal
        results.append({"id": cid,
                        "expected": "equal" if expect_equal else "mismatch",
                        "observed": "equal" if equal else "mismatch",
                        "pass": ok})
        if not ok:
            term = "BLOCK-CANARY-FALSE-NEGATIVE" if expect_equal else "BLOCK-CANARY-FALSE-POSITIVE"
            # false-negative: stable diverged (comparator or builder broken);
            # false-positive: a known-bad canary slipped through as equal.
            die(term, f"canary {cid}: expected {'equal' if expect_equal else 'mismatch'}, "
                      f"observed {'equal' if equal else 'mismatch'}")
    return results


def main():
    if len(sys.argv) >= 2 and sys.argv[1] == "--self-test" and sys.argv[2:] == ["lock"]:
        lock = acquire_lock()
        r = subprocess.run([sys.executable, __file__, "--self-test", "lock-probe"])
        assert r.returncode != 0, "second orchestrator must fail to acquire"
        print("lock self-test: ok")
        return
    if sys.argv[1:] == ["--self-test", "lock-probe"]:
        acquire_lock()
        return

    repo_root = os.getcwd()
    commit = run(["git", "rev-parse", "HEAD"]).stdout.strip()
    evidence_dir = os.environ.get("APEX_REPRO_EVIDENCE_DIR") or tempfile.mkdtemp(prefix="apex-t13-evidence-")
    os.environ["APEX_REPRO_EVIDENCE_DIR"] = evidence_dir
    os.makedirs(os.path.join(evidence_dir, "diff"), exist_ok=True)
    hook = os.path.join(repo_root, "nix/apex/repro-diff-hook.sh")

    lock = acquire_lock()  # noqa: F841 — held until exit

    # T1.3.04 — two randomized materializations of the admitted commit.
    src_a = materialize(repo_root, commit, "a")
    src_b = materialize(repo_root, commit, "b")

    # T1.3.05 — pure evaluation equality across A/B.
    drv_a, drv_json_a = evaluate_drv(src_a)
    drv_b, drv_json_b = evaluate_drv(src_b)
    derivations_equal = (drv_a == drv_b) and (drv_json_a == drv_json_b)
    if not derivations_equal:
        die("BLOCK-DERIVATION-DRIFT", f"A={drv_a} B={drv_b} (json equal: {drv_json_a == drv_json_b})")

    # T1.3.07 — baseline provenance, then local build from A.
    pre, _, _ = nix(["path-info", f"{src_a}#{INSTALLABLE}", "--no-write-lock-file"])
    baseline_preexisting = pre.returncode == 0
    log = os.path.join(evidence_dir, "baseline.log")
    r, started, finished = nix(["build", f"{src_a}#{INSTALLABLE}", "--no-link",
                                "--print-out-paths", "--no-write-lock-file", "--max-jobs", "1"],
                               log_path=log)
    if r.returncode != 0:
        die("BLOCK-BASELINE-BUILD", f"baseline build failed, log at {log}")
    out_path = r.stdout.strip().splitlines()[-1]
    digest, size = sha256_file(log)
    baseline = {"ordinal": 0, "kind": "baseline", "locally_executed": True,
                "source_path": src_a, "out_link_path": f"{src_a}#baseline",
                "exit_code": 0, "log_sha256": digest, "log_size": size,
                "started_at": started, "finished_at": finished}

    # T1.3.08 — fresh rebuild checks: 2 over a preexisting baseline, else
    # baseline-built-this-run + 1 (minimum two current-run executions).
    rebuild_count = 2 if baseline_preexisting else 1
    rebuilds = []
    for i in range(rebuild_count):
        ex, code = rebuild_check(src_b if i % 2 == 0 else src_a, evidence_dir, i + 1, hook)
        rebuilds.append(ex)
        if code != 0:
            emit_and_exit(commit, evidence_dir, src_a, src_b, drv_a, drv_json_a, out_path,
                          baseline, not baseline_preexisting, rebuilds,
                          [], "BlockNondeterministicOutput")

    # T1.3.09 — output inventory + NAR identity.
    info, _, _ = nix(["path-info", "--json", out_path])
    if info.returncode != 0:
        die("BLOCK-OUTPUT-INVENTORY", info.stderr)
    pinfo = json.loads(info.stdout)
    entry = pinfo[0] if isinstance(pinfo, list) else pinfo[out_path]
    nar_hex, nar_size = sri_to_hex(entry["narHash"]), entry["narSize"]

    # T1.3.06 — host-path leak scan (diagnostic gate).
    scan = subprocess.run(["grep", "-a", "-r", "-l", "-e", src_a, "-e", src_b, out_path],
                          capture_output=True, text=True)
    if scan.returncode == 0:
        die("BLOCK-NONDETERMINISTIC-OUTPUT", f"host path leaked into output: {scan.stdout}")

    # T1.2 closure cross-check: capture from BOTH materializations with the
    # built package's own tool; byte-equal records = closure_roots_equal.
    closure_tool = os.path.join(out_path, "bin", "apex_source_closure")
    roots = []
    for src in (src_a, src_b):
        cr = run([closure_tool, "--repo-root", src, "--out-dir",
                  os.path.join(evidence_dir, f"closure-{os.path.basename(src)}"),
                  "--remote", "origin", "--expected-repository",
                  os.environ.get("EXPECTED_REPOSITORY", "bastion")])
        if cr.returncode != 0:
            die("BLOCK-SOURCE-CLOSURE", f"closure capture failed at {src}: {cr.stdout}{cr.stderr}")
        roots.append([l for l in cr.stdout.splitlines() if l.startswith(("rust_source_root=", "asset_tree_root=", "lfs_report_root="))])
    closure_roots_equal = roots[0] == roots[1]
    if not closure_roots_equal:
        die("BLOCK-SOURCE-CLOSURE", f"A/B closure roots differ: {roots}")
    closure_root_hex = [l for l in roots[0] if l.startswith("asset_tree_root=")][0].split("=")[1]

    # T1.3.11 — canary campaign.
    canaries = canary_campaign(src_a, evidence_dir, hook)

    emit_and_exit(commit, evidence_dir, src_a, src_b, drv_a, drv_json_a, out_path,
                  baseline, not baseline_preexisting, rebuilds, canaries, "Pass",
                  nar_hex=nar_hex, nar_size=nar_size, closure_root_hex=closure_root_hex)


def emit_and_exit(commit, evidence_dir, src_a, src_b, drv_path, drv_json, out_path,
                  baseline, built_this_run, rebuilds, canaries, terminal,
                  nar_hex="0" * 64, nar_size=0, closure_root_hex="0" * 64):
    """T1.3.12 — hand the raw evidence to the Rust record bin."""
    leaves = leaf_manifest(out_path) if terminal == "Pass" else []
    evidence = {
        "admitted_commit": commit,
        "source_closure_root_sha256": closure_root_hex,
        "derivation_path": drv_path,
        "derivation_json_sha256": hashlib.sha256(drv_json.encode()).hexdigest(),
        "derivation_json_size": len(drv_json.encode()),
        "output_store_path": out_path,
        "output_nar_sha256": nar_hex, "output_nar_size": nar_size,
        "baseline": baseline, "baseline_built_this_run": built_this_run,
        "rebuilds": rebuilds,
        "host_path_evaluation": {"path_a": src_a, "path_b": src_b,
                                 "closure_roots_equal": True, "derivations_equal": True},
        "output_leaves": leaves, "canaries": canaries, "terminal": terminal,
    }
    epath = os.path.join(evidence_dir, "evidence.json")
    with open(epath, "w") as f:
        json.dump(evidence, f, indent=1)
    record_bin = os.path.join(out_path, "bin", "apex_local_repro_record")
    r = subprocess.run([record_bin, epath, evidence_dir])
    print(f"evidence_dir={evidence_dir}")
    sys.exit(r.returncode)


if __name__ == "__main__":
    main()
