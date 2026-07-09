# Project Bastion — restore ledger (append-only)

Rollback map for `bastion/main`. Every tag below is a fully-tested,
gate-passed block boundary — `git checkout <tag>` (or `git reset --hard
<tag>` on `bastion/main` if a later block needs to be discarded entirely)
returns the tree to that block's known-good state. Never delete or rewrite
an entry; if a block is later reverted, add a new entry noting it rather
than editing the old one.

| Tag | Represents | Rolls back past |
|---|---|---|
| `bastion-baseline` | Pre-Bastion vanilla Veloren, before any block work. | Everything. |
| `bastion-block-B1.6` | Ortho overseer camera + Z-slice + 4-mode occlusion/relight (B1 + B1.5 + B1.6 + B1.7, retro-tagged). | All camera/viewmode/input work. |
| `bastion-block-B1.7` | Same commit as B1.6 (B1.7's fixes landed inside B1.6's QA rounds, not a separate merge). | Same as B1.6. |
| `bastion-block-B2a` | Overseer interaction surface: tool palette, radial menu, designate-paint + echo overlay, selection. | All designation-UI work; colonists/jobs do not exist yet at this tag. |
| `bastion-block-B3` | Colonist entity model: `Colonist`/`PlayerColony`/`BastionGodAnchor` comps, promote/demote, §4 god-anchor invulnerability, founding + selection UI. | All job-board/work-execution work; colonists exist but are idle (vanilla civilised AI only). |
| `bastion-block-B4` | Designation → job board → autonomous arbitration + pathing. Colonists claim/travel to jobs; nothing completes work yet (`Arrived` was terminal). | All work-execution/item-drop/skill-XP work. |
| `bastion-block-B5` | Work execution: dig/chop/build terrain effects, item drops, skill XP, Build material gating. `Arrived` is now transient (jobs complete and release). Colonist opportunistic item-pickup AI gated off. | Hauling (B6) and everything after. |

## Notes for future rollback decisions

- Tags mark **merge boundaries on `bastion/main`**, not every commit on a
  block's working branch (`bastion/block-<N>`) — those branches carry the
  fine-grained history (checkpoint → build → self-test → commit-or-rollback
  per sub-step) if a *partial* revert within a block is ever needed instead
  of rolling back the whole block.
- `server/agent/src/action_nodes.rs` and `server/agent/src/data.rs` (the
  `ReadData::colonists` field + colonist item-pickup gate) are vanilla
  agent-AI files touched for the first time in B5 — rolling back past
  `bastion-block-B5` also reverts that gate, meaning any *other* future
  code that came to depend on `ReadData::colonists` existing would need to
  be rolled back too. None does yet as of B5.
- `common::bastion::BUILD_MATERIAL_ITEM` and the single-material Build
  stand-in are B5-only; B6 is expected to replace (not extend) that
  mechanism, so rolling back to B4 cleanly removes it with no dangling
  references.
- **`bastion-block-B5` was moved once**, same session it was first cut: a
  wider post-merge re-verification pass (running the gate far more than
  the original 5 times) turned up a third reachability bug (the mine
  quarry pit had no exit ramp — see `BASTION_B5_FINDINGS.md` §4b) that the
  original tag's state didn't include the fix for. Since nothing had yet
  been built on top of the original tag, the tag was force-moved to the
  commit with the fix rather than leaving a known-flaky boundary as the
  permanent rollback target — judged more honest for future rollback
  purposes than a tag whose name promises "fully-tested" but whose
  content sometimes wasn't. If a rollback to "B5 before the ramp fix" is
  ever specifically needed, it's `ec29fda` on `bastion/block-B5` (the gate
  fixes commit, pre-ramp) — not tagged, but preserved in that branch's
  history.

## B5.5 (2026-07-09)

| Field | Value |
|---|---|
| Block | B5.5 — zone deletion + item-drop pile aggregation (patch block) |
| Tag | `bastion-block-B5.5` |
| Previous green tag | `bastion-block-B5` (at `297cc0f`, post tag-move) |
| Revert command | `git reset --hard bastion-block-B5` (on `bastion/main`) |
| Reverting undoes | Erase tool + radial Delete-zone + designation-removal echo/overlay subtraction; persistent pile aggregation (colonist drops would resume carpeting one entity per block AND regain the 300 s despawn timer — i.e. reverting reintroduces a known item-LOSS hazard); the `--b55-scenario` gate; the B5 scenario's amount-sum assertions (reverts to entity counts). |
| Data-format caveats | `CreateItemDropEvent` gained `persistent: bool` (in-memory only, not serialized). `ServerGeneral::BastionDesignationRemoved` is a new net message — old client + new server (or vice versa) across this boundary would break protocol; irrelevant for the single-tree singleplayer setup. No rtsim `data.dat` changes. `comp::bastion::BastionPile` is a new server-side comp (not persisted, not synced). |
