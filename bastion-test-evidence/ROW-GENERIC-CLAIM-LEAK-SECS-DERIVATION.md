# ROW: `GENERIC_CLAIM_LEAK_SECS`'s real derivation is still open

**Filed per the v4 packet's own permission** (1a-i: "if no such bound can
be established, ship a named conservative guess and file a row for the
derivation — but do not present an observed median as a derived bound").

## THE CURRENT SHIP

`bastion_jobs::generic_claim_leak_secs()` returns `2.0 *
access_stall_secs()` (~1860s today) — a conservative multiplier on an
*existing* derived bound (the queue-wait backstop), not itself derived
from what actually bounds a legitimate claim's duration. Named as a
guess in its own doc comment, not presented as derived.

## WHAT THE REAL DERIVATION NEEDS

Per the packet's own framing: **"a legitimate claim's duration is
bounded by travel budget + work duration — derive from that, do not
observe it."**

- **Travel budget**: `STUCK_TIMEOUT` (10s) is the per-attempt watchdog,
  but a claim can survive MULTIPLE stall-and-retry cycles before any
  release path engages (the churn/humanitarian-bubble pattern —
  `stuck_strikes`, `PERSIST_ESCALATE_STRIKES`, the soft-collision grace
  window). The real travel-budget term is `STUCK_TIMEOUT × (max retries
  before a release path fires)`, and that retry ceiling was not derived
  or even fully enumerated by this row's reads — the code has several
  distinct release/degrade paths (queue-release, carve-planner,
  churn-drop) with different retry counts, not one shared ceiling.
- **Work duration**: bounded by `job.progress` accrual rate (skill +
  tool factors) against the completion threshold (1.0 for Farm). Not
  measured or derived here — would need a per-`WorkType` worst-case
  accrual-rate read, which this row did not attempt.

## WHY THIS WASN'T DONE HERE

Scope and time: this row's mandate was the famine fix (routes 1–3) plus
the F6 backstop Opus's F5-tension resolution required. Deriving the
retry ceiling across every stall/release path in `bastion_jobs.rs`
(several thousand lines, many interacting timeout mechanisms) is a
reads-first investigation on its own, not a corollary of this row's
existing reads.

## WHY THE CURRENT GUESS IS SAFE ANYWAY

F6 is explicitly a LAST-RESORT backstop with an INVERTED bar — it should
essentially never fire on a correctly-functioning system (routes 1–3
are the enumerated fix). `2× access_stall_secs()` is comfortably above
every other timing constant in the file (`STUCK_TIMEOUT`=10s,
`access_stall_secs()`≈930s), so even without a precise derivation it
cannot race any known release mechanism — it can only ever fire on a
genuine leak, which is exactly its job. The imprecision here costs
detection LATENCY on a rare finding, not correctness.
