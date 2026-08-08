# THE ROUTER AND THE PROBE DISAGREE IN 41% OF CASES — AND CLASS B RESTS ON THE PROBE

**wave25, read from disk.** Unit = probe result **with a real timeout**, across
**both** `b5_mine_reachability_probe` and `b5_chop_reachability_probe`.

## §1 — TWO INSTRUMENTS, BOTH DISAGREEING, IN BOTH DIRECTIONS

| kind | any path mode exists (probe) | route_exists ever (router) | n |
|---|---|---|--:|
| mine | ✅ | ✅ | 19 |
| mine | ✅ | ★ **❌** | **9** |
| mine | ★ **❌** | ✅ | **8** |
| mine | ❌ | ❌ | 4 |
| chop | ✅ | ✅ | 2 |
| chop | ★ **❌** | ✅ | **1** |
| chop | ❌ | ❌ | 1 |

> ★★★ **18 of 44 — 41% — are cases where the offline probe and the live router
> give OPPOSITE answers about whether the target can be reached.** Nine each way.
> **They cannot both be right.**

## §2 — ★★★★★ WHY THIS OUTRANKS EVERYTHING ELSE IN THE ROW

> **CLASS B (`step:False, jump:True`) IS A PROBE OUTPUT. If the probe is the
> instrument that's wrong, Class B is an artifact — and "colonists can't jump"
> was never real.**

★ **The probe-validation run already ruled is now doubly motivated**, and by an
*independent* route: not *"is the model right?"* in the abstract, but **"we have
two instruments that contradict each other 41% of the time, and one of them is
the sole basis for a proposed feature."**

★ **The 9 mine cases of `path exists / no route` are the reverse defect** and
matter just as much: **the router failed to find a route the probe says is
there.** *Whichever instrument is wrong, one of these two populations is a real
bug and the other is instrument error.*

## §3 — ★★★★★★ AND THE SIGNAL THAT CUTS ACROSS BOTH FAMILIES: `route_next_idx = 0`

Repeatedly, across mine **and** chop, on seeds 54, 71, 78, 80, 85, 61, 66:

    route_exists = true,  route_complete = false,  route_next_idx = 0

> **A ROUTE WAS OBTAINED AND THE COLONIST NEVER ADVANCED ONE STEP ALONG IT.**

Seed 71 shows `(0,True)` in **eight** of its nine probes. Seed 80's chop:
`(0,True),(0,True)`. Seed 54: `(0,True),(0,True)` twice.

★ **The corpus already carries a field named for this** —
`route_next_idx_pinned` — **true on 78, 80, 54, 71; false on 85, 61, 71-partial;
null elsewhere.** ★ **Someone built a field for "the route pinned" and it has
never been read.** *(Fourth instance this week of an instrument built and never
derived.)*

## §4 — WHAT I DO **NOT** CLAIM

- ★ **Not** that the router is wrong. **It may be the probe.** That is precisely
  what the validation run decides, and I have three errors today from over-reading
  an instrument's output as the world's state.
- ★ **Not** that `route_next_idx = 0` means "never moved." **I have not read the
  field's producer**, and this week has twice punished exactly that — the number
  is suggestive and its semantics are UNVERIFIED. **Read the producer before
  building on it.**
- **Not** that chop and mine share a root cause. The `idx = 0` pattern spans both,
  which is **consistent** with one mechanism and equally consistent with two
  mechanisms sharing a symptom.
- **Not** a large-n result: **44 timeout-bearing probes across 6 seeds.** The
  coverage caveat from the sweep applies unchanged.

## §5 — REVISED PICTURE OF THE CHOP FAMILY

**My cascade hypothesis is HALF right, and the corpus names the half that's
wrong:**

| seed | `chop_cleared` | `log_sum` | verdict |
|---|---|---|---|
| 78, 80, 85, 92 | **False** | **0** | ★ cascade **CONFIRMED**: chop fails → no logs → no materials → build fails |
| ★ **62** | **True** | **1** | ★ **REFUTED** — chop SUCCEEDED; its build failure has a different cause |

★ **Seed 62 must come out of the chop family.** And **seed 78 differs from
80/85/92** — `ch_base_blocked_by` is null for 78 and populated for the other
three, and 78 is the only one whose `any_needs_materials` is true.

> **So the "chop family" is 78 | {80, 85, 92} | and 62 is not in it at all** —
> three groups where I claimed one.

★ **And I was wrong that the chop family has "no diagnostic":**
`b5_chop_reachability_probe` exists and is populated for exactly these four.
**Third correction in this thread, same root cause: I keep reporting an
instrument's absence from a shape rather than from its producer.**
