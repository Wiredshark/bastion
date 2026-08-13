# SERVER-BINARY FINGERPRINT (F3's missing half) — **PRE-REGISTRATION**

Written before any code change. Promoted to the front of the queue by the void recorded
in `CANCEL-ACROSS-RESTART-RESULTS.md`.

## 1 · THE GAP, DEMONSTRATED RATHER THAN ARGUED

The driver-freshness row (`03d36e10f1`) made `bastion_playtest` declare its commit and
verb table. Hours later a scored plant run produced a clean, publishable negative result
— *"the append-only defect does not resurrect anything"* — from a **server binary 15
minutes older than its source**, because a `cargo build` had been killed by a timeout.

Nothing in the server log said which build wrote it. **Every bar in this program reads
SERVER emits** — `founding site relief`, `colony orders replayed`, `COLONY TERMINAL`,
`colony founded`, `founding refused` — and not one of them can be attributed to a build.

The driver half was the half that had already been caught once. This is the half that
carries the evidence.

## 2 · WHAT THIS IS *NOT*

- **Not** an mtime check. mtime is not identity — a touched file, a restored backup or a
  copied binary all lie. The same refusal F3 made.
- **Not** a staleness *detector*. The server cannot know its source has moved. It can only
  state what it is; comparing that to intent is the reader's job (and the runner's
  `ls -la` until something automates it).

## 3 · THE BARS

### S1 · **THE SERVER DECLARES ITS BUILD**
- **PASS:** an early server log line carries `bastion: server build=<hash> built_at=<ts>`,
  before any colony emit.
- Placed early on purpose: a fingerprint after the emits it is meant to attribute is
  useless for a truncated or crashed log.

### S2 · **THE FINGERPRINT IS CORRECT, NOT MERELY PRESENT**
- **PASS:** the logged hash equals the first 8 hex of the commit the binary was built
  from, checked against `git rev-parse HEAD` at build time.
- A present-but-wrong fingerprint is worse than none: it would make a stale run look
  attributed. **S1 without S2 is decoration** — the same shape as F2's "measured but
  nothing consumes it".

### S3 · **LIVE**
- **PASS:** the line appears in a real `server-cli` run driven by the playtest driver, in
  the same log the bars are read from.

### PLANT
- Replace the `GIT_HASH` read with a hardcoded constant ⇒ **S2 red** (logged hash ≠
  `git rev-parse`), while **S1 stays green** (a line is still emitted).
- This isolates *correct* from *present*, which one plant on the line's existence could
  not do — the same lesson the water gate's second plant taught.

## 4 · THE KNOWN CAVEAT, CARRIED FORWARD

`common::util::GIT_HASH` derives from `VELOREN_GIT_VERSION`, which is **runtime
overridable** by an env var. Unset — as in every run here — it reports the build. This
was named in F3 and is inherited, not rediscovered; it will be stated in the code beside
the emit, as there.

## 5 · WHAT I WILL **NOT** DO

1. **I will not claim this prevents stale-binary runs.** It makes them *visible after the
   fact*. Prevention is the runner asserting freshness before the run — which is now a
   memory, not a hope.
2. **I will not retro-attribute existing logs.** Every server log in this session was
   written by an unfingerprinted binary; they stay unattributed. Inventing provenance is
   worse than the gap.
3. **I will not score S1 alone.** Present-but-wrong is the failure mode that matters.
