# Item 31 (God-hand v1, server half / POWER-0) — DISPOSITION: bars 1+2+4 **PASS**

Legs: `smite` (chain15/16) + `smiteref` (chain18, BASTION_FAVOR_ZERO).

**Bar 1 — a funded cast applies the REAL effect: PASS.** Three live casts:
health 1.0 → 0.39996 (exactly the specced 0.6 max-fraction through the
standard `HealthChange` seam — `derive_attack_instance("bastion/smite/v1")`),
`Outcome::Lightning` emitted, favor paid (5.16→0.16; 15→10), all in one
witness line per cast. The spec's ★(f) held: never a light show without
the kill... and stacked casts took the target to health 0.0 — which
exposed item 36's premise (colonist death does not exist; banked as that
row's first build step).

**Bar 2 — an unfunded cast refuses LOUDLY and applies NOTHING: PASS.**
Under the FAVOR_ZERO pin (added because the pool out-trickles any script
timing during the ~2-minute boot — 3/3 unpinned casts landed including one
at t=0, disclosed): 3/3 refusals with the price beside the balance
(`favor=0.0 cost=5.0`), zero casts, and the target's health 1.0 at every
sample — the refusal's couldn't-happen control.

**Bar 4 — the no-command invariant: PASS.** The target's drive/activity
unchanged across every cast bracket; the handler touches health only —
a smitten colonist decides afresh, per the dispatch spec's ★(b)/(c).

**Bar 3 (twin determinism): rides the standing twin queue.**

Successors chartered, not now-work: more powers are dispatch-table rows on
this pipeline; devotion replaces the trickle when DF-RELIGION lands; item
32 (faith economy) builds directly on this pool.
