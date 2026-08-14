# THE BASTION BUILD LOOP — canonical prompt & testing protocol

Paste the block in §0 to start or resume the loop. Everything after it is the protocol
that block refers to. Keep this file updated as the process changes — it is the single
reference.

---

## §0 · THE PROMPT

> Continue the Project Bastion build loop autonomously and do not stop until the build
> list is complete. Each iteration: pick the next row yourself (queue = newest
> `bastion-test-evidence/*-RESULTS.md` "SESSION QUEUE STATE", else
> `readme/BUILD-ROADMAP.md` open items), **PRE-REGISTER acceptance and commit it before
> any data exists**, build it, **RED-DEMONSTRATE every bar with a matched control**, run
> it **LIVE** with the named emit observed, derive numbers from the system's own
> constants, commit the row disposition, and immediately start the next row. Never end
> idle; never wait on a gate without parallel-filling.
>
> Follow `readme/BASTION-BUILD-LOOP-PROMPT.md` — the testing protocol, the speed levers
> (uncapped TPS, parallel ports, headless determinism), and the standing refusals.
>
> Code + commits in `E:\veloren-master\.engine-integration-wt` (`bastion/wip-batch-verify`);
> docs/dispositions on `bastion/block-B6HAUL`. Check for running cargo/veloren before
> builds and never kill a process you did not start.

---

## §1 · THE ROW SHAPE (non-negotiable order)

1. **READ THE PRODUCER FIRST.** Before designing a bar or an instrument, read the code
   that makes the number. This has changed the row four times: the chop witness belonged
   in the felling stagger not the completion; `WorkPriorities` already existed; the
   server already logged its build; `decision_job_ids` already sorted.
2. **PRE-REGISTER, THEN COMMIT.** Bars, named witness emits, planted failures, and an
   explicit *"what I will not do at scoring time"* list. Committed **before** any data.
3. **BUILD**, with the instrument as a first-class deliverable.
4. **RED-DEMONSTRATE** every bar (§3).
5. **RUN LIVE**, attested (§2).
6. **COMMIT THE DISPOSITION** — including everything refuted, void, or withdrawn.
7. **NEXT ROW IMMEDIATELY.**

## §2 · BEFORE EVERY SCORED LEG

- Run `bastion-test-evidence/attest-run.sh <out> <binary>` — HEAD, dirty `.rs` count with
  files named, binary build time, and whether any tracked source is newer than the binary.
- **PRINT THE PRECONDITION ABOVE THE RESULT.** A VOID run and a RED run look identical.
  Four voids this session were caught only this way, and one was nearly published as a
  finding.
- **NEVER put a cargo build and a long run in the same timed call.** A killed build leaves
  a plausible stale binary — that cost a scored run.
- **Assert the leg's ENABLING CONDITION by reading the artefact, not by running the step
  that should have created it.** An Admin-gated command needs `--no-auth` on the **grant**
  as well as the run: without it the CLI resolves the username through the auth server,
  fails, and writes an **empty `admins.ron`** while exiting 0. Three arms scored `witness=0`
  and read as a refutation of the feature; the feature was never reached. **Check
  `admins.ron` has an entry, not that `admin add` returned success.**

## §3 · PLANT CRAFT

- Every bar gets a plant: break it, watch the **named** test fail on the **claimed axis**,
  restore, watch it pass.
- **The plant must sit at the stage it disables** (a save-plant on a pre-populated
  userdata proves nothing).
- **A plant on existence is not a plant on the claim.** If the registered plant would not
  discriminate, run a better one and say so — registered plants are a **floor**.
- **Revert plants with a targeted Edit. NEVER `git checkout` a file holding uncommitted
  work.**
- Check the test **count**, not just pass/fail: "0 passed, 124 filtered out" reads almost
  exactly like "the plant did not fire".

## §4 · WHAT A NUMBER IS ALLOWED TO BE

| scored through | admissible |
|---|---|
| a connected **client** | **separations** (47.6% vs 0.0%) or **geometric invariants** (15 = 5+6+4) |
| **headless + deterministic** | magnitudes are valid — runs are bit-identical |

- Live magnitudes through a client are **not reproducible**: the client's arrival tick is
  unpinned and it is what loads chunks.
- **Half the counters cannot vary** (`tilled = 30` is the farm's 5×6). They make excellent
  bars and useless determinism probes.

## §5 · THE DETERMINISTIC HEADLESS CAPTURE

All three are required; any one missing silently voids the comparison:

```
BASTION_DETERMINISTIC=1        # pins the RNG authority
BASTION_AUTOFOUND_COLONY=<n>   # seeded founding + preset + colony presence
                               # (presence = chunks load with NO CLIENT)
BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1
```

Verified: two such legs are **1035 log lines identical**, and with the flag unset the same
setup spreads `sown` 12→36. Always run the **flag-off control** — "two headless runs
agree" is also consistent with nothing varying headlessly at all.

## §6 · SPEED — use all of it

- **`BASTION_UNCAPPED_TPS=1` — HEADLESS LEGS ONLY.** It removes the wall-clock sleep. Its
  own source comment is reassuring and true as far as it goes: skips `clock.tick()` only,
  *"changes ONLY wall-clock pacing, never simulation dt … unset = today's behavior,
  bit-for-bit."* A 28 000-tick leg is ~15 min capped.
  > ⛔ **But the skip is ungated — it free-runs from server boot and never waits for a
  > client.** On a client-driven leg the server can burn the entire scored window before
  > the driver finishes connecting, so a command "issued after founding" lands after the
  > window instead. **The comment's guarantee is about `dt`; the hazard is about
  > *arrival*, and no comment on the producer mentions it.** Read what a flag does to the
  > *other* participants, not just to the subsystem it names.
  >
  > On a client-driven leg, **the parallelism lever is ports, not TPS** — which costs
  > nothing, since three arms at 30 tps in parallel take exactly as long as one.
- **Parallel legs**: each leg gets its own `VELOREN_USERDATA`, and the port lives in
  `<userdata>/server/server_config/settings.ron` (`gameserver_protocols`, default 14004).
  Give each leg a distinct port and they run concurrently.
- **Headless legs need no driver at all**, so parallelism is bounded only by CPU
  (16 logical / 8 physical here; keep ~half free if another lane is building).
- **Background long runs** (`run_in_background`), never a foreground call that can hit the
  10-minute cap.
- VM pool exists for corpus fans (`vm-pool.sh`, `IN_USE_ADDRESSES=8`) — attest the commit
  on any VM leg.

## §7 · STANDING REFUSALS

1. Don't score a bar whose plant never fired — that bar is **unsound**, retire and replace
   it (A2 was).
2. Don't report a number without its **denominator's unit** (95 all-item deliveries vs 54
   food stock compared two different populations of thing).
3. A grep pattern is a **claim about naming** — verify it matches the real ids (a food
   classifier that omitted `wheat` reported 0 food hauls while wheat was being hauled).
4. Don't retro-attribute old logs, don't rewrite a prereg — **amend it in public**.
5. A conclusion that **names its own fragility** is recoverable; one that doesn't becomes a
   permanent false fact. H3's refutation was reversed only because the row wrote down that
   its effect was smaller than its noise.
