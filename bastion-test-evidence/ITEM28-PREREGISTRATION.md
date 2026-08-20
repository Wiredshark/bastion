# Item 28 (Tool quality/wear) — PRE-REGISTRATION

**Substrate, read not assumed:** vanilla items carry durability (armor/
weapon loss-on-death machinery exists); colony WORK consumes no tool today —
work rate is `skills.level_for(work)`-scaled only (item 17's seam), and
colonists work bare-handed. The B6 fetch contract (required_item +
reservation) is proven; a tool is a required_item that DOESN'T get consumed
at completion — it WEARS.

## Build shape (v1)

1. **A tool def per work class** (data first: `TOOL_DEFS: work → def`,
   e.g. Mine→pickaxe def, Chop→axe def, from vanilla's existing item
   assets — no new assets, placeholder-first law).
2. **The work-rate seam takes a tool factor**: carrying the matching tool
   multiplies work rate (constant ×1.5 v1); the factor rides the same
   function the skill multiplier rides — one seam, no drift.
3. **Wear**: each completion with a tool decrements its durability (reuse
   the vanilla durability field); at zero the item is destroyed with a
   witness (a break is an EVENT: witnessed, chronicle-carryable — a
   thought-table row can price it later).
4. Acquisition rides existing machinery: tools are craftable at item 26's
   recipe table (or seeded by fixture lever until then — never gate on
   the chain).

## BARS

1. Tooled vs bare A/B (same seed, same work): tooled completions-per-window
   measurably higher; both arms' counts stated.
2. Wear is REAL: durability strictly decreases per completion (witnessed
   counter), and the break event fires at zero with the tool gone
   (conservation: the item is destroyed, not leaked).
3. After the break, the colonist's rate returns to bare-handed — the
   factor cannot outlive its tool (fallback = identity).
4. Twin determinism.

VOID branches: the tool never reaches the worker (fetch witnesses — the
item-27 lessons apply verbatim); durability field absent on the chosen def
(pick another or carry a bastion-side counter; report which).
