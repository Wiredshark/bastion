# WIP-STATE — Row B′ paired A/B (Opus lane, fan owner)

**Row B′ committed `7590dfa962`** (parent `f7072cd346`), on
`bastion/wip-batch-verify`. Falsifier survived; **crossing counts base=2 /
variant=2, identical job IDs, positions and strikes — nothing manufactured.**
Both the outcome claim and the mechanism claim are proven at n=1.

## THE REGISTERED DISCIPLINE NEEDS **TWO** FANS, NOT ONE

The gate requires a **repeat-run control per arm**, and a single
`--b5-rowb-paired` fan cannot supply it: it gives base-vs-variant per seed, but
nothing to measure *either arm's own* run-to-run spread against. **We know that
spread is nonzero** — that is what the whole observer-effect bisection
established, and what made the last A/B's ±1 unreadable.

**So: run `--b5-rowb-paired` TWICE at the same commit.** Then:

| comparison | what it measures |
|---|---|
| `base(fan1)` vs `base(fan2)` | **the base arm's noise floor** |
| `variant(fan1)` vs `variant(fan2)` | **the variant arm's noise floor** |
| `base` vs `variant`, within each fan | **the effect** |

> **★ THE EFFECT COUNTS ONLY IF IT EXCEEDS BOTH ARMS' NOISE.** Last time we read a
> ±1 against an unmeasured floor and it turned out to be a favourable draw. This
> design measures the floor *in the same run shape* as the effect, which is the
> only way the comparison is legible.

## Invocation (both legs identical)

```bash
ZONE=us-east1-b STAGGER=25 BRANCH=bastion/wip-batch-verify \
  bash vm-pool.sh 4 e2-standard-8 12 49 "--b5-rowb-paired" 25 90 \
  > corpus-waves/wave2N-ROWBPRIME-fanlog-7590dfa962.txt 2>&1
```

~10 min cooldown between fans (machine-image create-rate). **Nothing gets pushed
while either fan is in flight — the post-commit hook auto-pushes, so hold all
commits.**

## When both land

1. **Attest**: `COMMIT=7590dfa9` ×4 per fan, `DONE=12` each.
2. **Collect** each: `python collect_wave.py corpus-waves/wave2N_ROWBPRIME_..._FULL.json /tmp/bastion-pool/bastion-pool-*.log`
   (exit 2 ⇒ do not use).
3. **Noise floors first, effect second.** Read the two same-arm comparisons
   *before* looking at base-vs-variant, so the effect is judged against a floor
   that was measured rather than assumed.
4. **CONDITION ON THE EXPOSURE POPULATION** — 52 54 61 62 66 71 76 80 85 90 92.
   The 48-seed aggregate diluted the last result ~4:1 and hid it.
5. Pre-registered: **seed 76 must not regress**; release counters move only where
   benching fires; the 11 exposed seeds behave as before or better.

## Ship gate

Net benefit, or **provable harmlessness** at corpus scale. If the conditioned
read shows indifference again, the claim ships as **correctness + player
visibility** with a null aggregate — and Fable takes that re-scope to Ben with
the numbers.

**Then, and only then: the live playthrough.** Scorecard is drafted at
`LIVE-PLAYTHROUGH-PREP.md` (13 features, player language, read-budget check
applied).
