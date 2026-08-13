# SERVER FINGERPRINT / RUN ATTESTATION — **RESULTS & ROW DISPOSITION**

Scored against `SERVER-FINGERPRINT-PREREG.md` (`09106a6559`) **as amended by**
`SERVER-FINGERPRINT-AMENDMENT.md` (`829762f2db`). No engine code changed — and that is
the finding, not a shortfall.

## THE SCORE

| bar | verdict | evidence |
|---|---|---|
| **S1** the server declares its build | ✅ **ALREADY TRUE** — premise refuted | `Server version: <hash8> [date]` in **63** logs, `server/src/lib.rs:1095` |
| **S2** the fingerprint is correct | ✅ **ALREADY TRUE** for HEAD | void run `31b5928d` vs control `9a832215` — they differ, correctly |
| **V1** a dirty build says so | ⛔ **REFUSED AS UNSOUND IN build.rs** — see below | — |
| **A1** run-time attestation detects dirt | ✅ PASS | `dirty .rs : 1` + named files |
| **A2** run-time attestation detects staleness | ✅ PASS | `STALE: source is NEWER than this binary` |

| plant | required red | observed |
|---|---|---|
| `touch` a source (mtime only, content identical) | STALE only | **STALE fires, `dirty .rs : 0`** — axes isolated |
| real content edit | both | **dirty=1 AND stale** |
| restore | back to green | `dirty .rs : 0` |

## ★ THE ROW I PLANNED WAS THE WRONG ROW, TWICE

**First correction (the amendment):** I asserted the server "declares nothing". It has
declared its build all along, in 63 logs I generated myself — including both arms of the
void I blamed on the missing fingerprint. **The void was diagnosable from the log the
whole time.**

**Second correction (this document):** the narrowed row — add a dirty-tree marker in
`common/build.rs` — is **unsound and I refuse to ship it.**

`common/build.rs` declares:

```rust
println!("cargo::rerun-if-changed={head}");        // .git/HEAD
println!("cargo::rerun-if-changed={logs_head}");   // .git/logs/HEAD
```

with a comment recording that this was a deliberate fix for a real staleness bug. But
those are the *only* watched paths. **Editing `server/src/lib.rs` touches neither**, so
the build script does not re-run, so a dirty flag it computed would describe the tree as
of the last **commit or checkout** — not now.

It would print **"clean" for a dirty build.** That is strictly worse than no marker,
because it would look like provenance while being wrong — the exact failure mode this
row exists to close. Making it sound would require forcing the script to re-run on every
build, discarding the optimisation that comment exists to protect.

**So the marker moves to where it can be evaluated at run time and cannot go stale:
`bastion-test-evidence/attest-run.sh`.**

## WHAT ATTESTATION MEASURES

```
HEAD          : 9a83221505
dirty .rs     : 0
binary        : veloren-server-cli.exe  built 2026-08-13 17:23:38
  fresh: no tracked .rs source is newer than the binary
```

Two **independent** checks — neither subsumes the other. A committed, clean tree can
still be stale (built before the last commit); a dirty tree can be freshly built. The
`touch`-only plant proves the isolation: STALE fires with `dirty .rs : 0`.

**⚠ The staleness check is mtime-based and therefore CONSERVATIVE.** `git checkout --`
rewrites mtime without changing a byte, so it over-warns on a binary that is in fact
current — observed in this row's own restore step and documented in the script. That is
the correct bias: over-warning costs a rebuild; **under-warning cost this session a
scored run.**

## WHAT I DECLINE TO CLAIM

- **Not** that this prevents stale-binary runs. It makes them **visible before the run**
  instead of after — which is the whole difference, but it is not prevention.
- **Not** that `Server version` is untrustworthy. It is *exactly* right about HEAD. It is
  silent about uncommitted content, and that silence is what misled me.
- **Not** that a build-time dirty flag is impossible in general — only that it is unsound
  **given this build script's rerun policy**, which exists for a good reason I am not
  overturning to win a smaller point.

## SESSION QUEUE STATE — nine rows closed

1–7 as recorded · 8. Cancel across restart (`71d06226a4`) · 9. **Server fingerprint /
run attestation**, this document.

**Next:** §8 N2's widget tier — still the one acceptance tier no bar has ever run at.
Attestation now runs ahead of each scored leg.
