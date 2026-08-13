# THE FOUNDING COLONIST COUNT — **PRE-REGISTRATION**

Written before any code change. Arises from §8 N2 (the widget tier), which turned out to
be answerable by reading rather than by driving a GPU.

## 1 · N2 IS SATISFIED — the two tiers converge

§8 N2 asked whether the widget path and the message path diverge. They do not:

```rust
// voxygen/src/session/mod.rs:3642  (HudEvent::BastionRadialPick → FoundColony)
client.bastion_spawn_colony(point, 6);
```

That is **the same `Client::bastion_spawn_colony`** (`client/src/lib.rs:2844`) the
playtest driver calls. The tiers converge at the client method, so everything downstream
— message, server handler, preset placement — is shared by construction. **No GPU
automation is needed to establish that**, and no bar below requires one.

## 2 · BUT THE ARGUMENT DIVERGES — three numbers, no source of truth

| where | value | provenance |
|---|---|---|
| the shipped widget | **6** | a bare literal at `session/mod.rs:3642` |
| every acceptance script | **8** | `spawn 8`, in **47 of 47** scripts |
| the preset's bed plot | **8 cells** | `min_off (-3,-3,0) … max_off (-2,-2,1)` = 2×2×2, and every live log shows `kind=Bed jobs=8` |

**Every bar in this program was scored at a colonist count the shipped action never
produces.** The tested population is not the shipped population — and nothing in the code
relates the three numbers, so nothing can notice when they drift apart. *A number must
carry its producer;* these carry nothing.

The bed arithmetic is the sharp end: the preset provides sleeping capacity for exactly 8,
so 8-vs-8 is the **saturated** case (which the bars happened to test, fortunately) and
6-vs-8 is what ships.

## 3 · THE BARS

### N1 · **ONE CONSTANT, AND THE WIDGET READS IT**
- **PASS:** the founding count is a named constant in the preset module, and
  `session/mod.rs` passes **that**, not a literal.
- Witness: the founded emit's `colonists=` equals the constant.

### N2 · **BED CAPACITY IS DERIVED, NOT COINCIDENTAL**
- **PASS:** a test computes the bed plot's cell count **from `FOUNDING_PRESET_V1`** and
  requires it to be **≥ the founding count** — so shrinking the bed or raising the count
  fails loudly instead of silently under-bedding the colony.
- Deriving it from the table is the point: hard-coding `8 >= 6` would restate the bug.

### N3 · **THE LIVE PATH STILL FOUNDS** — gate-must-test-live-path
- **PASS:** a live founding still reports `complete=true elements=stockpile,farm,bed` and
  `colonists=<constant>`.

### PLANTS
1. **Bed shrunk by one row** in the preset table ⇒ **N2 red** (capacity < count).
2. **Count raised above bed capacity** ⇒ **N2 red** from the other side — proving the bar
   tests the *relation*, not one side of it.

## 4 · WHAT I WILL **NOT** DO

1. **I will not "fix" the widget to 8, nor the scripts to 6, as if either number were
   authoritative.** Neither has a derivation. The constant gets **one** definition and a
   stated basis; changing the value is a design call, not a refactor, and I will name it
   as such rather than smuggle it in.
2. **I will not claim the widget tier is now covered by an automated bar.** Convergence at
   the client method is established **by reading**, and the mouse/render path above it is
   still unexercised. That is a smaller claim than "N2 closed", and it is the true one.
3. **I will not retro-score the earlier rows at 6.** They were run at 8 and are reported
   at 8; whether their conclusions hold at 6 is a separate question I am registering as
   open, not quietly assuming.
