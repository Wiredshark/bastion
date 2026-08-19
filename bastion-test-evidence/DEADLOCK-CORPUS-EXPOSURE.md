# HOW MANY BANKED RUNS ENDED DEADLOCKED?

Banked read, no spend, while two fans run.

## The signature, and why the obvious measure is wrong

First cut: *"any `materials` refusal"* → **529 of 653 runs (81%)**. That number
is nearly useless — a job briefly waiting for materials is **normal**, and a
transient is not a deadlock.

Refined: **at the FINAL census, `materials > 0` AND `eligible = 0`** — still
blocked, with nothing claimable, at the end of the run.

| | |
|---|---|
| runs carrying a claim-refusal census | **653** |
| any materials refusal (weak measure) | 529 (81%) |
| **still blocked at the final census** | **375 (57%)** |

## ★ The signature CALIBRATES against ground truth

On `endurseed` — the only arm where the outcome is independently known — the
signature fires on **11 of 26 runs = 42%**, which is **exactly** the measured
collapse rate (11 of 26 runs finish under 50 maturations).

**A metric defined from the census alone reproduces a rate measured from crop
counts.** That is the check that makes the signature worth anything.

## ★★ But the corpus-wide 57% is INFLATED, and here is why

Most arms in that 653 are **~1 minute long**. A one-minute colony ending with
materials blocked has not deadlocked — **it has simply not had time to clear
anything**. `guardspread`, `wallctl`, `fleehealth`, `wall`, `hostile*` all show
100%, and at ~1 minute per run that is what an unfinished colony looks like, not
a defect.

| arm class | end-blocked | reading |
|---|---|---|
| `endurseed` (~25 min, 271k ticks) | **42%** | **real** — matches the known collapse rate |
| short arms (~1 min) | 95–100% | **uninformative** — the run ends before anything could clear |

**So the honest claim is: the deadlock signature is established on long runs and
is not measurable on short ones.** The 57% is not a corpus-wide defect rate; it
is one real rate mixed with a lot of runs that stopped too early to say.

★ This is the same trap as counting an unloaded chunk as air, and as reading
`refused` without splitting out correct skips: **a number that mixes "it failed"
with "it never got the chance" reports the wrong thing confidently.**

## What it does establish

Every arm that runs a colony **can** enter the state. The terrain-only arms
(`provtrav*`, `anchorplant`, `ordering`, `P0/P1/P2`) show **zero** materials
refusals in any run — as expected, since they carry no colony work. The exposure
is real and it is not confined to farming; it is confined to *having a colony*.
