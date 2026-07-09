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
