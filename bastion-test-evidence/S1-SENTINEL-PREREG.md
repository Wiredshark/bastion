# S1 SENTINEL — **PRE-REGISTRATION**

Written before any code change. **It also corrects the row's own framing**, because the
code does not match the brief in one important way.

## 1 · WHAT ALREADY EXISTS — read, not assumed

- `bastion_jobs::colony_terminal_should_fire(streak_after_increment, threshold)` is
  **already a pure predicate**: `streak == threshold`, edge-triggered by construction.
- `COLONY_TERMINAL_ZERO_STREAK_SAMPLES = 10` (`bastion_jobs.rs:2046`).
- A both-polarity unit test already exists
  (`colony_terminal_fires_once_and_the_mirror_case_never_fires`): fires at 10, not at
  11, not below.

**So "pure predicate extraction" is done.** Reporting this row as if it were the work
would be claiming credit for a commit that already landed.

## 2 · WHAT IS ACTUALLY MISSING — and it is the part the corpus cases test

The predicate takes a **single already-computed streak value**. Everything that makes
the sentinel *correct over a run* — the `+= 1` on a zero sample, the **reset to 0** on
any nonzero sample, and the once-per-window edge trigger — lives **inline at the firing
site**, inside a 20 000-line function:

```rust
if food_stock == 0 {
    board.colony_terminal_zero_streak += 1;
    if colony_terminal_should_fire(board.colony_terminal_zero_streak, THRESHOLD) { … }
} else {
    board.colony_terminal_zero_streak = 0;
}
```

The registered corpus cases are **claims about a SEQUENCE**, not about a value:
- **v4's 517-zero-streak famine MUST fire.**
- **v5's sawtooth-341 MUST NOT fire** — and it must not fire *precisely because of the
  reset arm*, which no test currently touches.

A test of `colony_terminal_should_fire` alone **cannot distinguish those two cases at
all**: both would be reduced to "is this number 10". The sawtooth's whole content is
that the streak never *reaches* 10 because nonzero samples keep resetting it. **That
logic is currently untestable except by running a server.**

## 3 · THE EXTRACTION

A pure **sequence** function beside the predicate — increment, reset and edge-trigger
together — and the inline site becomes its caller, so the tested path is the shipping
path (registry B17: identity by construction, no parallel copy).

```
colony_terminal_scan(samples, threshold) -> Option<usize>   // index of the firing sample
```

**Refusal, stated now:** if I write this as a second implementation and leave the inline
arms in place, the tests pass while the shipping code is untouched — the F8 defect,
which has now bitten this program twice (the eight-mutation batch, and my own W2 number
last row). **The site must delegate, and a test must assert that it does** by driving
the same function the server drives.

## 4 · THE BARS

### S1-A · **THE FAMINE FIRES** — v4's case
- **PASS:** a 517-long run of zero samples fires **exactly once**, at index **9**
  (the 10th sample, `threshold = 10`, derived from the constant — not chosen).
- Asserting *once* is the point: an off-by-one from `==` to `>=` would fire **508**
  times and still "fire".

### S1-B · **THE SAWTOOTH DOES NOT FIRE** — v5's case, the registered non-trigger
- **PASS:** a sawtooth of 341 samples whose zero-runs never reach 10 fires **zero**
  times.
- **This is the bar that needs the reset arm**, and it is the one the existing test
  cannot express.

### S1-C · **THE BOUNDARY, BOTH SIDES**
- A zero-run of exactly **9** must **not** fire; a run of exactly **10** must fire once.
- Derived from `COLONY_TERMINAL_ZERO_STREAK_SAMPLES = 10`; if the constant moves, the
  bar moves with it, because the test reads the constant rather than hard-coding 10.

### S1-D · **THE LIVE PATH STILL EMITS** — gate-must-test-live-path
- **PASS:** the live server still emits
  `bastion: COLONY TERMINAL (sentinel S1, log-only)` with
  `consecutive_zero_samples = 10` under a starved colony.
- A refactor that greens S1-A–C while silencing the live emit is a **failure**, not a
  pass. This is the bar that catches "the harness is green and the feature is inert".

### PLANTS (each red-demonstrated, then restored)
1. `==` → `>=`: **S1-A must go red** (fires 508 times, not once).
2. Delete the reset arm (`else { streak = 0 }`): **S1-B must go red** (the sawtooth
   accumulates and fires).
3. Make the site stop calling the extracted function: **a delegation test must go red**,
   proving the extraction is load-bearing rather than decorative.

## 5 · WHAT I WILL **NOT** DO

1. **I will not report the existing predicate + its existing test as this row's work.**
2. **I will not count S1-A as passing if it fires more than once** — "fired" is not the
   bar, "fired exactly once at index 9" is.
3. **I will not leave the inline arms in place** beside an extracted copy. If delegation
   turns out to be impossible without a larger refactor than this row warrants, I stop
   and report that as the blocker, naming the function and line.
4. **LIVE-ABORT STAYS OUT.** The emit is log-only by design — v3 would have terminated
   79 minutes early had a naive version been wired to act. Nothing in this row makes the
   sentinel gate, terminate, or alter behaviour. It stays an observer.
5. **I will not synthesise the v4/v5 sample sequences from their conclusions.** The
   shapes are registered here — 517 zeros; a 341-sample sawtooth whose zero-runs stay
   under 10 — and if the real corpus disagrees with those shapes, the corpus wins and I
   report the discrepancy.
