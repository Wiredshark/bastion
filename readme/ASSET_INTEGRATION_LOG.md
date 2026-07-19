# ASSET INTEGRATION LOG (game-side, append-only)

Written by `bastion-harness --asset-test` (B-ASSET1). The asset session reads
this back to promote READY-pending-dynamic → READY-INTEGRATED. One dated block
per run; one JSON line per asset (schema: AssetResult in
`bastion-harness/src/asset_test.rs`).

FORMAT CONTRACT (engine side — see docs/BASTION_BASSET1_FINDINGS.md):
- Input: flattened `.vox` under `asset-lab/vox/` (compose.py output). Sidecar
metadata optional; category inferred from id prefix.
- Byte bands (ASSET_LESSONS L3): 1–16 world-reserved (engine defaults),
32–199 literals, 200–255 gameplay markers via the engine marker registry:
200 = gate KeyholeBars (closed) / carved air (open variant),
206/207/208/209 = pressure-plate/desk/bench/bed → carved air, cells recorded
as function points. UNKNOWN 200-255 bytes fail marker fidelity — extend
`server/src/bastion_assets.rs::marker_registry` first.
- Figure-layer assets (props/items at 11 vox/block, creatures) are load-only /
SKIP here; their world integration is manifest work (a later block).
- `test_*` fixtures run only when named explicitly (deliberate-FAIL demos).

## RUN 2026-07-09 22:59 UTC · seed 1337 · target `structure_housing_human_cottage`

ASSET structure_housing_human_cottage DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_housing_human_cottage","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]"],"blocks_placed":1295,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.1s"},{"name":"egress","pass":true,"detail":"arrived in 3.6s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 11.0s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.7s"},{"name":"integrated-reach","pass":false,"detail":"could not stage at integrated spot: STUCK (watchdog) after 26.0s, best dist 337.1"}],"pass":false}
```

## RUN 2026-07-09 23:02 UTC · seed 1337 · target `structure_housing_human_cottage`

ASSET structure_housing_human_cottage DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_housing_human_cottage","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]"],"blocks_placed":1295,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.2s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 12.6s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 5.2s"},{"name":"integrated-reach","pass":true,"detail":"slope 3 across footprint; arrived in 2.8s"},{"name":"integrated-egress","pass":true,"detail":"arrived in 2.4s"}],"pass":true}
```

## RUN 2026-07-09 23:05 UTC · seed 1337 · target `all`

ASSET armor_civ_apron DYNAMIC-ISOLATED: PASS
```json
{"id":"armor_civ_apron","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET armor_civ_robe DYNAMIC-ISOLATED: PASS
```json
{"id":"armor_civ_robe","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET armor_civ_straw_hat DYNAMIC-ISOLATED: PASS
```json
{"id":"armor_civ_straw_hat","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET armor_civ_tunic DYNAMIC-ISOLATED: PASS
```json
{"id":"armor_civ_tunic","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET armor_warden_chest DYNAMIC-ISOLATED: PASS
```json
{"id":"armor_warden_chest","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET defense_palisade_line_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"defense_palisade_line_demo","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 200 x8: expected KeyholeBars, resolved KeyholeBars [ok]"],"blocks_placed":398,"sprite_cfgs_dropped":8,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.1s, best dist 11.3"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.4s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.5s"}],"pass":true}
```
ASSET flora_highland_rowan DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_highland_rowan","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 1 x456: expected world-band default, resolved TemperateLeaves [ok]","byte 8 x12: expected world-band default, resolved Fruit [ok]"],"blocks_placed":655,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 0.8s"},{"name":"path-back","pass":true,"detail":"arrived in 5.7s"}],"pass":true}
```
ASSET flora_pine_snowdusted DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_pine_snowdusted","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 2 x2683: expected world-band default, resolved PineLeaves [ok]"],"blocks_placed":4137,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 8.5s"},{"name":"path-back","pass":true,"detail":"arrived in 6.7s"}],"pass":true}
```
ASSET flora_rowan_sapling DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_rowan_sapling","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 1 x40: expected world-band default, resolved TemperateLeaves [ok]"],"blocks_placed":47,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.4s"},{"name":"path-back","pass":true,"detail":"arrived in 4.9s"}],"pass":true}
```
ASSET flora_rowan_snag DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_rowan_snag","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":42,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.9s"},{"name":"path-back","pass":true,"detail":"arrived in 5.1s"}],"pass":true}
```
ASSET item_hoe_iron DYNAMIC-ISOLATED: PASS
```json
{"id":"item_hoe_iron","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET item_pickaxe_iron DYNAMIC-ISOLATED: PASS
```json
{"id":"item_pickaxe_iron","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET item_quarry_maul DYNAMIC-ISOLATED: PASS
```json
{"id":"item_quarry_maul","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":["byte 14 x2: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET item_shovel_iron DYNAMIC-ISOLATED: PASS
```json
{"id":"item_shovel_iron","category":"Item","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_banner_post DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_banner_post","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_handcart DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_handcart","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_logs DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_logs","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_ore DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_ore","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_stone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_stone","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_stockpile_post DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_stockpile_post","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":["byte 14 x2: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET prop_waystone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_waystone","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET sprite_bin_wood DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_bin_wood","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_cave_gloomcap DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_cave_gloomcap","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":["byte 14 x52: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x44: expected world-band default, resolved PalmLeavesInner [ok]","byte 16 x31: expected world-band default, resolved Hollow [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"}],"pass":true}
```
ASSET sprite_ladder_rope DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_ladder_rope","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_sack_grain DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_sack_grain","category":"Prop","mode":"load-only","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 8 x2227: expected world-band default, resolved Fruit [ok]","byte 217 x13: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":2874,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"2 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 8.7s"},{"name":"egress","pass":true,"detail":"arrived in 8.8s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 16.5s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 14.4s"}],"pass":false}
```
ASSET structure_housing_human_cottage DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_housing_human_cottage","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]"],"blocks_placed":1295,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.5s"},{"name":"egress","pass":true,"detail":"arrived in 3.9s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 13.6s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.4s"},{"name":"integrated-reach","pass":true,"detail":"slope 3 across footprint; arrived in 3.4s"},{"name":"integrated-egress","pass":true,"detail":"arrived in 2.5s"}],"pass":true}
```
ASSET structure_production_smithy DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_production_smithy","category":"Structure","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 201 x1: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]","byte 202 x1: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":532,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.0s"},{"name":"egress","pass":true,"detail":"arrived in 4.0s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 13.0s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.6s"}],"pass":false}
```
ASSET structure_production_smithy_v2 DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_production_smithy_v2","category":"Structure","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 201 x1: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]","byte 202 x1: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":532,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 3.8s"},{"name":"egress","pass":true,"detail":"arrived in 3.9s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 12.3s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.7s"}],"pass":false}
```
ASSET terracotta_set_demo DYNAMIC-ISOLATED: FAIL
```json
{"id":"terracotta_set_demo","category":"Other","mode":"load-only","fidelity_ok":false,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 217 x22: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"2 distinct bytes checked"}],"pass":false}
```
ASSET workshop_carpenter DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_carpenter","category":"Other","mode":"load-only","fidelity_ok":false,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 210 x1: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"2 distinct bytes checked"}],"pass":false}
```
ASSET workshop_mason DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_mason","category":"Other","mode":"load-only","fidelity_ok":false,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 211 x60: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"2 distinct bytes checked"}],"pass":false}
```
ASSET workshop_smelter DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_smelter","category":"Other","mode":"load-only","fidelity_ok":false,"marker_checks":["byte 212 x2: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"1 distinct bytes checked"}],"pass":false}
```
ASSET workshop_tannery DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_tannery","category":"Other","mode":"load-only","fidelity_ok":false,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 213 x1: expected UNKNOWN-MARKER (declare in marker_registry), resolved Filled [MISMATCH]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"2 distinct bytes checked"}],"pass":false}
```

## RUN 2026-07-09 23:06 UTC · seed 1337 · target `test_room_door_closed`

ASSET test_room_door_closed DYNAMIC-ISOLATED: FAIL
```json
{"id":"test_room_door_closed","category":"TestFixture","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":146,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"reach-interior","pass":false,"detail":"STUCK (watchdog) after 13.5s, best dist 4.5"}],"pass":false}
```

## RUN 2026-07-09 23:06 UTC · seed 1337 · target `test_room_door_open`

ASSET test_room_door_open DYNAMIC-ISOLATED: PASS
```json
{"id":"test_room_door_open","category":"TestFixture","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":146,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.4s"},{"name":"egress","pass":true,"detail":"arrived in 3.6s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 13.7s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.2s"}],"pass":true}
```

## RUN 2026-07-10 01:59 UTC · seed 1337 · target `all`

ASSET defense_palisade_line_demo DYNAMIC-ISOLATED: FAIL
```json
{"id":"defense_palisade_line_demo","category":"Defense","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 0 x0: expected parseable .ron custom_indices sidecar, resolved PARSE ERROR: 3:14-3:22: Unexpected variant named `DoorBars` in enum `StructureBlock`, expected one of `None`, `Grass`, `TemperateLeaves`, `PineLeaves`, `Acacia`, `Mangrove`, `PalmLeavesInner`, `PalmLeavesOuter`, `Water`, `GreenSludge`, `Fruit`, `Coconut`, `MaybeChest`, `Hollow`, `Liana`, `Normal`, `Log`, `Filled`, `Sprite`, `Chestnut`, `Baobab`, `BirchWood`, `FrostpineLeaves`, `EntitySpawner`, `Keyhole`, `BoneKeyhole`, `GlassKeyhole`, `Sign`, `KeyholeBars`, `HaniwaKeyhole`, `TerracottaKeyhole`, `SahaginKeyhole`, `VampireKeyhole`, `MyrmidonKeyhole`, `MinotaurKeyhole`, `MapleLeaves`, `CherryLeaves`, `AutumnLeaves`, `RedwoodWood`, `SpriteWithCfg`, or `Choice` instead [MISMATCH]","byte 200 x8: expected Sprite, resolved Sprite [ok]","byte 200 x8: expected 8 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":398,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.2s, best dist 18.0"},{"name":"gate-open-admits","pass":false,"detail":"goto refused (colonist not loaded or has a job)"}],"pass":false}
```
ASSET flora_highland_rowan DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_highland_rowan","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 1 x456: expected world-band default, resolved TemperateLeaves [ok]","byte 8 x12: expected world-band default, resolved Fruit [ok]"],"blocks_placed":655,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 7.1s"},{"name":"path-back","pass":true,"detail":"arrived in 6.2s"}],"pass":true}
```
ASSET flora_pine_snowdusted DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_pine_snowdusted","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 2 x2683: expected world-band default, resolved PineLeaves [ok]"],"blocks_placed":4137,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 7.0s"},{"name":"path-back","pass":true,"detail":"arrived in 6.5s"}],"pass":true}
```
ASSET flora_rowan_sapling DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_rowan_sapling","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 1 x40: expected world-band default, resolved TemperateLeaves [ok]"],"blocks_placed":47,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.3s"},{"name":"path-back","pass":true,"detail":"arrived in 5.4s"}],"pass":true}
```
ASSET flora_rowan_snag DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_rowan_snag","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":42,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.2s"},{"name":"path-back","pass":true,"detail":"arrived in 5.5s"}],"pass":true}
```
ASSET gate_brick_line DYNAMIC-ISOLATED: FAIL
```json
{"id":"gate_brick_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 0 x0: expected parseable .ron custom_indices sidecar, resolved PARSE ERROR: 3:14-3:22: Unexpected variant named `DoorBars` in enum `StructureBlock`, expected one of `None`, `Grass`, `TemperateLeaves`, `PineLeaves`, `Acacia`, `Mangrove`, `PalmLeavesInner`, `PalmLeavesOuter`, `Water`, `GreenSludge`, `Fruit`, `Coconut`, `MaybeChest`, `Hollow`, `Liana`, `Normal`, `Log`, `Filled`, `Sprite`, `Chestnut`, `Baobab`, `BirchWood`, `FrostpineLeaves`, `EntitySpawner`, `Keyhole`, `BoneKeyhole`, `GlassKeyhole`, `Sign`, `KeyholeBars`, `HaniwaKeyhole`, `TerracottaKeyhole`, `SahaginKeyhole`, `VampireKeyhole`, `MyrmidonKeyhole`, `MinotaurKeyhole`, `MapleLeaves`, `CherryLeaves`, `AutumnLeaves`, `RedwoodWood`, `SpriteWithCfg`, or `Choice` instead [MISMATCH]","byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1236,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.4s, best dist 11.5"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.6s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.9s"}],"pass":false}
```
ASSET gate_dwarven_line DYNAMIC-ISOLATED: FAIL
```json
{"id":"gate_dwarven_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 0 x0: expected parseable .ron custom_indices sidecar, resolved PARSE ERROR: 3:14-3:22: Unexpected variant named `DoorBars` in enum `StructureBlock`, expected one of `None`, `Grass`, `TemperateLeaves`, `PineLeaves`, `Acacia`, `Mangrove`, `PalmLeavesInner`, `PalmLeavesOuter`, `Water`, `GreenSludge`, `Fruit`, `Coconut`, `MaybeChest`, `Hollow`, `Liana`, `Normal`, `Log`, `Filled`, `Sprite`, `Chestnut`, `Baobab`, `BirchWood`, `FrostpineLeaves`, `EntitySpawner`, `Keyhole`, `BoneKeyhole`, `GlassKeyhole`, `Sign`, `KeyholeBars`, `HaniwaKeyhole`, `TerracottaKeyhole`, `SahaginKeyhole`, `VampireKeyhole`, `MyrmidonKeyhole`, `MinotaurKeyhole`, `MapleLeaves`, `CherryLeaves`, `AutumnLeaves`, `RedwoodWood`, `SpriteWithCfg`, or `Choice` instead [MISMATCH]","byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1366,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.4s, best dist 11.5"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.5s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.7s"}],"pass":false}
```
ASSET mine_breach_maw DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_breach_maw","category":"Prop","mode":"load-only (figure-scale dims Some((11, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 217 x3: expected Filled, resolved Filled [ok]","byte 217 x3: expected 3 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```
ASSET mine_headframe_human DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_headframe_human","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 14)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET mine_pithead_human DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_pithead_human","category":"Prop","mode":"load-only (figure-scale dims Some((9, 9, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_altar_stone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_altar_stone","category":"Prop","mode":"load-only (figure-scale dims Some((4, 3, 3)); declared cast 'interact-adjacent' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x1: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET prop_banner_post DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_banner_post","category":"Prop","mode":"load-only (figure-scale dims Some((7, 3, 15)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_bed_fourpost DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_bed_fourpost","category":"Prop","mode":"load-only (figure-scale dims Some((9, 13, 12)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_chair_fine DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_chair_fine","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_chair_masterwork DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_chair_masterwork","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET prop_chair_plain DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_chair_plain","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_claim_cairn DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_claim_cairn","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 8)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_handcart DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_handcart","category":"Prop","mode":"load-only (figure-scale dims Some((22, 12, 12)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_hanging_lantern DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_hanging_lantern","category":"Prop","mode":"load-only (figure-scale dims Some((4, 4, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_hearth_human DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_hearth_human","category":"Prop","mode":"load-only (figure-scale dims Some((9, 4, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x1: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x3: expected world-band default, resolved PalmLeavesInner [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```
ASSET prop_muster_bell DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_muster_bell","category":"Prop","mode":"load-only (figure-scale dims Some((7, 5, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_logs DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_logs","category":"Prop","mode":"load-only (figure-scale dims Some((11, 7, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_ore DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_ore","category":"Prop","mode":"load-only (figure-scale dims Some((8, 8, 4)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_stone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_stone","category":"Prop","mode":"load-only (figure-scale dims Some((9, 9, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_potted_herb DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_potted_herb","category":"Prop","mode":"load-only (figure-scale dims Some((4, 4, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_statue_ancestor DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_statue_ancestor","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 14)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_stockpile_post DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_stockpile_post","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x2: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET prop_throne DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_throne","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_wallart_tapestry DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_wallart_tapestry","category":"Prop","mode":"load-only (figure-scale dims Some((11, 2, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_wallart_trophy_skull DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_wallart_trophy_skull","category":"Prop","mode":"load-only (figure-scale dims Some((7, 4, 8)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_waystone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_waystone","category":"Prop","mode":"load-only (figure-scale dims Some((7, 5, 11)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET prop_zonemarker_meeting_totem DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_zonemarker_meeting_totem","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 17)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET prop_zonemarker_refuse_stake DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_zonemarker_refuse_stake","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 11)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_bin_wood DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_bin_wood","category":"Prop","mode":"load-only (figure-scale dims Some((10, 8, 6)); declared cast 'interact-adjacent' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_cave_gloomcap DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_cave_gloomcap","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x52: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x44: expected world-band default, resolved PalmLeavesInner [ok]","byte 16 x31: expected world-band default, resolved Hollow [ok]"],"blocks_placed":201,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 1.5s"},{"name":"path-back","pass":true,"detail":"arrived in 5.8s"}],"pass":true}
```
ASSET sprite_caveflora_deep DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_caveflora_deep","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x43: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x6: expected world-band default, resolved PalmLeavesInner [ok]"],"blocks_placed":78,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.6s"},{"name":"path-back","pass":true,"detail":"arrived in 5.7s"}],"pass":true}
```
ASSET sprite_caveflora_shallow DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_caveflora_shallow","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x4: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":38,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.6s"},{"name":"path-back","pass":true,"detail":"arrived in 5.7s"}],"pass":true}
```
ASSET sprite_crop_barley_0 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_0","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.1s"},{"name":"path-back","pass":true,"detail":"arrived in 5.4s"}],"pass":true}
```
ASSET sprite_crop_barley_5 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_5","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":423,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.6s"},{"name":"path-back","pass":true,"detail":"arrived in 5.9s"}],"pass":true}
```
ASSET sprite_goods_ale_keg DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_ale_keg","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 7)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_bread DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_bread","category":"Prop","mode":"load-only (figure-scale dims Some((10, 7, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_cloth DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_cloth","category":"Prop","mode":"load-only (figure-scale dims Some((9, 6, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_ingots DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_ingots","category":"Prop","mode":"load-only (figure-scale dims Some((8, 6, 4)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_leather DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_leather","category":"Prop","mode":"load-only (figure-scale dims Some((9, 6, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_planks DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_planks","category":"Prop","mode":"load-only (figure-scale dims Some((9, 7, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_stoneblocks DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_stoneblocks","category":"Prop","mode":"load-only (figure-scale dims Some((9, 7, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_ladder_iron_dwarven DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_ladder_iron_dwarven","category":"Prop","mode":"load-only (figure-scale dims Some((11, 3, 33)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x1: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"}],"pass":true}
```
ASSET sprite_ladder_rope DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_ladder_rope","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 11)); declared cast 'climb' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET sprite_orevein_bloodstone DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_orevein_bloodstone","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]"],"blocks_placed":126,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"1 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.1s"},{"name":"path-back","pass":true,"detail":"arrived in 5.6s"}],"pass":true}
```
ASSET sprite_orevein_velorite DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_orevein_velorite","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x8: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x24: expected world-band default, resolved PalmLeavesInner [ok]"],"blocks_placed":141,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 7.7s"},{"name":"path-back","pass":true,"detail":"arrived in 5.9s"}],"pass":true}
```
ASSET sprite_sack_grain DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_sack_grain","category":"Prop","mode":"load-only (figure-scale dims Some((8, 8, 8)); declared cast 'interact-adjacent' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```
ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 6.9s"},{"name":"egress","pass":true,"detail":"arrived in 8.1s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 16.2s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.7s"}],"pass":true}
```
ASSET structure_faith_shrine DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_faith_shrine","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 218 x1: expected Filled, resolved Filled [ok]","byte 218 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":233,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 3.7s"},{"name":"egress","pass":true,"detail":"arrived in 3.6s"}],"pass":true}
```
ASSET structure_faith_temple_human DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_faith_temple_human","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 204 x2: expected Sprite, resolved Sprite [ok]","byte 204 x2: expected 2 authored cells (exact), resolved all cells match [ok]","byte 208 x9: expected Sprite, resolved Sprite [ok]","byte 208 x9: expected 9 authored cells (exact), resolved all cells match [ok]","byte 218 x1: expected Filled, resolved Filled [ok]","byte 218 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":863,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.0s"},{"name":"egress","pass":true,"detail":"arrived in 3.6s"}],"pass":true}
```
ASSET structure_housing_human_cottage DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_housing_human_cottage","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1295,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.2s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 14.0s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.3s"},{"name":"integrated-reach","pass":true,"detail":"slope 3 across footprint; arrived in 2.6s"},{"name":"integrated-egress","pass":true,"detail":"arrived in 2.3s"}],"pass":true}
```
ASSET structure_production_smithy DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_production_smithy","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 201 x1: expected Sprite, resolved Sprite [ok]","byte 201 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 202 x1: expected Sprite, resolved Sprite [ok]","byte 202 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":532,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.6s"},{"name":"egress","pass":true,"detail":"arrived in 4.5s"}],"pass":true}
```
ASSET structure_trade_depot DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_trade_depot","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x2: expected world-band default, resolved MaybeChest [ok]","byte 11 x2: expected 2 authored cells (exact), resolved all cells match [ok]","byte 219 x1: expected Sprite, resolved Sprite [ok]","byte 219 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":418,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.3s"},{"name":"egress","pass":true,"detail":"arrived in 4.9s"}],"pass":true}
```
ASSET terracotta_set_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"terracotta_set_demo","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 217 x22: expected Filled, resolved Filled [ok]","byte 217 x22: expected 22 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":6153,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 9.4s"},{"name":"egress","pass":true,"detail":"arrived in 5.8s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 12.1s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.7s"}],"pass":true}
```
ASSET workshop_carpenter DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_carpenter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 210 x1: expected Sprite, resolved Sprite [ok]","byte 210 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":502,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.0s"},{"name":"egress","pass":true,"detail":"arrived in 4.2s"}],"pass":true}
```
ASSET workshop_kitchen DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_kitchen","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 215 x2: expected Sprite, resolved Sprite [ok]","byte 215 x2: expected 2 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":506,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.4s"},{"name":"egress","pass":true,"detail":"arrived in 5.0s"}],"pass":true}
```
ASSET workshop_loomhouse DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_loomhouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 216 x1: expected Sprite, resolved Sprite [ok]","byte 216 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":494,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.4s"},{"name":"egress","pass":true,"detail":"arrived in 4.5s"}],"pass":true}
```
ASSET workshop_mason DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_mason","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 211 x60: expected Sprite, resolved Sprite [ok]","byte 211 x60: expected 60 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":498,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 3.9s"},{"name":"egress","pass":true,"detail":"arrived in 3.3s"}],"pass":true}
```
ASSET workshop_smelter DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_smelter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 212 x2: expected Sprite, resolved Sprite [ok]","byte 212 x2: expected 2 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":506,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.5s"},{"name":"egress","pass":true,"detail":"arrived in 4.9s"}],"pass":true}
```
ASSET workshop_tannery DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_tannery","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 213 x1: expected Sprite, resolved Sprite [ok]","byte 213 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":494,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.6s"},{"name":"egress","pass":true,"detail":"arrived in 4.4s"}],"pass":true}
```

## RUN 2026-07-10 02:01 UTC · seed 1337 · target `defense_palisade_line_demo`

ASSET defense_palisade_line_demo DYNAMIC-ISOLATED: FAIL
```json
{"id":"defense_palisade_line_demo","category":"Defense","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 0 x0: expected parseable .ron custom_indices sidecar, resolved PARSE ERROR: 3:14-3:22: Unexpected variant named `DoorBars` in enum `StructureBlock`, expected one of `None`, `Grass`, `TemperateLeaves`, `PineLeaves`, `Acacia`, `Mangrove`, `PalmLeavesInner`, `PalmLeavesOuter`, `Water`, `GreenSludge`, `Fruit`, `Coconut`, `MaybeChest`, `Hollow`, `Liana`, `Normal`, `Log`, `Filled`, `Sprite`, `Chestnut`, `Baobab`, `BirchWood`, `FrostpineLeaves`, `EntitySpawner`, `Keyhole`, `BoneKeyhole`, `GlassKeyhole`, `Sign`, `KeyholeBars`, `HaniwaKeyhole`, `TerracottaKeyhole`, `SahaginKeyhole`, `VampireKeyhole`, `MyrmidonKeyhole`, `MinotaurKeyhole`, `MapleLeaves`, `CherryLeaves`, `AutumnLeaves`, `RedwoodWood`, `SpriteWithCfg`, or `Choice` instead [MISMATCH]","byte 200 x8: expected Sprite, resolved Sprite [ok]","byte 200 x8: expected 8 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":398,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.4s, best dist 11.5"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.7s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.8s"}],"pass":false}
```

## RUN 2026-07-10 02:01 UTC · seed 1337 · target `gate_brick_line`

ASSET gate_brick_line DYNAMIC-ISOLATED: FAIL
```json
{"id":"gate_brick_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 0 x0: expected parseable .ron custom_indices sidecar, resolved PARSE ERROR: 3:14-3:22: Unexpected variant named `DoorBars` in enum `StructureBlock`, expected one of `None`, `Grass`, `TemperateLeaves`, `PineLeaves`, `Acacia`, `Mangrove`, `PalmLeavesInner`, `PalmLeavesOuter`, `Water`, `GreenSludge`, `Fruit`, `Coconut`, `MaybeChest`, `Hollow`, `Liana`, `Normal`, `Log`, `Filled`, `Sprite`, `Chestnut`, `Baobab`, `BirchWood`, `FrostpineLeaves`, `EntitySpawner`, `Keyhole`, `BoneKeyhole`, `GlassKeyhole`, `Sign`, `KeyholeBars`, `HaniwaKeyhole`, `TerracottaKeyhole`, `SahaginKeyhole`, `VampireKeyhole`, `MyrmidonKeyhole`, `MinotaurKeyhole`, `MapleLeaves`, `CherryLeaves`, `AutumnLeaves`, `RedwoodWood`, `SpriteWithCfg`, or `Choice` instead [MISMATCH]","byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1236,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.5s, best dist 11.6"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.7s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.6s"}],"pass":false}
```

## RUN 2026-07-10 02:01 UTC · seed 1337 · target `gate_dwarven_line`

ASSET gate_dwarven_line DYNAMIC-ISOLATED: FAIL
```json
{"id":"gate_dwarven_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":false,"marker_checks":["byte 0 x0: expected parseable .ron custom_indices sidecar, resolved PARSE ERROR: 3:14-3:22: Unexpected variant named `DoorBars` in enum `StructureBlock`, expected one of `None`, `Grass`, `TemperateLeaves`, `PineLeaves`, `Acacia`, `Mangrove`, `PalmLeavesInner`, `PalmLeavesOuter`, `Water`, `GreenSludge`, `Fruit`, `Coconut`, `MaybeChest`, `Hollow`, `Liana`, `Normal`, `Log`, `Filled`, `Sprite`, `Chestnut`, `Baobab`, `BirchWood`, `FrostpineLeaves`, `EntitySpawner`, `Keyhole`, `BoneKeyhole`, `GlassKeyhole`, `Sign`, `KeyholeBars`, `HaniwaKeyhole`, `TerracottaKeyhole`, `SahaginKeyhole`, `VampireKeyhole`, `MyrmidonKeyhole`, `MinotaurKeyhole`, `MapleLeaves`, `CherryLeaves`, `AutumnLeaves`, `RedwoodWood`, `SpriteWithCfg`, or `Choice` instead [MISMATCH]","byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1366,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":false,"detail":"3 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.5s, best dist 11.3"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.7s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.8s"}],"pass":false}
```

## RUN 2026-07-10 02:10 UTC · seed 1337 · target `defense_palisade_line_demo`

ASSET defense_palisade_line_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"defense_palisade_line_demo","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 200 x8: expected Sprite, resolved Sprite [ok]","byte 200 x8: expected 8 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":398,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.5s, best dist 11.1"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.6s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.7s"}],"pass":true}
```

## RUN 2026-07-10 02:10 UTC · seed 1337 · target `gate_brick_line`

ASSET gate_brick_line DYNAMIC-ISOLATED: PASS
```json
{"id":"gate_brick_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1236,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.6s, best dist 11.6"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.9s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.8s"}],"pass":true}
```

## RUN 2026-07-10 02:11 UTC · seed 1337 · target `gate_dwarven_line`

ASSET gate_dwarven_line DYNAMIC-ISOLATED: PASS
```json
{"id":"gate_dwarven_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1366,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.6s, best dist 11.4"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.8s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.8s"}],"pass":true}
```

## RUN 2026-07-10 02:11 UTC · seed 1337 · target `pier_line_demo`

ASSET pier_line_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"pier_line_demo","category":"Prop","mode":"load-only (figure-scale dims Some((6, 24, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 224 x1: expected Sprite, resolved Sprite [ok]","byte 224 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 02:12 UTC · seed 1337 · target `prop_mooring_bollard`

ASSET prop_mooring_bollard DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_mooring_bollard","category":"Prop","mode":"load-only (figure-scale dims Some((3, 3, 4)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 02:12 UTC · seed 1337 · target `structure_boathouse`

ASSET structure_boathouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_boathouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":475,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 3.8s"},{"name":"egress","pass":true,"detail":"arrived in 3.4s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 11.2s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 3.8s"}],"pass":true}
```

## RUN 2026-07-10 02:12 UTC · seed 1337 · target `structure_lighthouse`

ASSET structure_lighthouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_lighthouse","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 24)); declared cast 'interior' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 223 x5: expected Filled, resolved Filled [ok]","byte 223 x5: expected 5 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 02:13 UTC · seed 1337 · target `prop_harbor_crane`

ASSET prop_harbor_crane DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_harbor_crane","category":"Prop","mode":"load-only (figure-scale dims Some((11, 7, 13)); declared cast 'work-marker' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 224 x1: expected Sprite, resolved Sprite [ok]","byte 224 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```

## QUALITY + DESIGN-CRITIQUE REVIEW v1 (integration tester, 2026-07-09 — new role per Ben)

Method: isometric judge renders (gen/render.py contact sheets, 9 groups, full 68-entry world-layer
catalog), graded weak/solid/strong on craftsmanship (silhouette at colonist scale, proportion,
coherence, style-bar fit). Sheets archived tester-side. Single odd-color voxels in renders are
MARKER bytes (expected), not defects. Routing: craftsmanship → pilot; brief/intent → designer.

GRADES (by group; unlisted = solid, no notes):
- STRONG: cottage (timber grammar sings), pine_snowdusted (best-in-library silhouette), gloomcap,
  gate_dwarven_line (basalt+amber identity), bed_fourpost, trophy_skull, handcart, the whole
  goods/pile family (12 pieces — instantly readable at a glance, great stockpile legibility),
  ladder_iron_dwarven, orevein pair.
- SOLID: palisade line, pier, trees (rowan family), workshops (as a family — see flag 1), lighthouse,
  trade depot, temple, boathouse (see flag 2), mine set, zonemarkers, furniture set, harbor pieces.
- WEAK: none outright — lowest are faith_shrine + statue_ancestor (see flags 5/6).

WEIRD-CHOICE / QUALITY FLAGS (numbered for reply):
1. ROUTE→DESIGNER — workshop family legibility: all 6 trades share one 14×11×12 template with
   material swaps; carpenter/loomhouse/tannery are near-identical dark boxes at colonist scale.
   Zone-differentiation is the stated design (DF-WORKSHOP), but a colony of 6 workshops will read
   as 6 same-boxes. Suggest per-trade tells (roofline variant / signage sprite / tool silhouette).
2. ROUTE→PILOT — saturation outliers: boathouse + trade_depot roofs are a much hotter orange than
   the library's muted ramps; terracotta_set_demo reads as solid GOLD (confirm terracotta ramp
   intent — it currently outshines everything including the dwarven gate's deliberate amber).
3. ROUTE→PILOT — crop_barley_5 is ~28 vox tall (≈2.5 blocks at sprite scale): mature barley
   towers over a colonist. Vanilla crops stay ≈1 block. Confirm intent or trim to ~12-14 vox.
4. ROUTE→PILOT — quarry_hall exterior is a featureless box with a flat light-gray door slab.
   Fine while dungeon-embedded (exterior unseen); weird the day it surface-places. Note in catalog
   ("embed-only") or dress the door face.
5. ROUTE→DESIGNER — faith_shrine (9×8×9) reads as a well/hutch, not a shrine; affordance unclear
   next to the (clear) altar_stone. Consider integrating the altar silhouette or an icon tell.
6. ROUTE→PILOT (minor) — statue_ancestor silhouette is a vague hooded lump (acceptable as
   "weathered", flagging for a look); muster_bell's bell is nearly invisible (reads as an empty
   gallows frame — one voxel-row bell swing would fix); mine_headframe legs read spindly/one
   appears to float at the SW corner (verify ground contact); caveflora_deep has detached floating
   voxels (confirm spore-particle intent); gate_brick_line cream speckle is dense — reads noisy at
   distance (compare against native brick ramps).

Functional note alongside: all 8 quality-flagged assets PASS their functional/fidelity tiers —
these are craftsmanship/intent observations, not integration failures.
## RUN 2026-07-10 02:31 UTC · seed 1337 · target `structure_boathouse`

ASSET structure_boathouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_boathouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":475,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 3.6s"},{"name":"egress","pass":true,"detail":"arrived in 3.5s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 13.7s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 3.9s"}],"pass":true}
```

## RUN 2026-07-10 02:32 UTC · seed 1337 · target `structure_trade_depot`

ASSET structure_trade_depot DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_trade_depot","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x2: expected world-band default, resolved MaybeChest [ok]","byte 11 x2: expected 2 authored cells (exact), resolved all cells match [ok]","byte 219 x1: expected Sprite, resolved Sprite [ok]","byte 219 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":418,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.1s"},{"name":"egress","pass":true,"detail":"arrived in 4.8s"}],"pass":true}
```

## RUN 2026-07-10 02:32 UTC · seed 1337 · target `terracotta_set_demo`

ASSET terracotta_set_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"terracotta_set_demo","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 217 x22: expected Filled, resolved Filled [ok]","byte 217 x22: expected 22 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":6153,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 9.6s"},{"name":"egress","pass":true,"detail":"arrived in 5.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 15.7s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 7.8s"}],"pass":true}
```

## RUN 2026-07-10 02:33 UTC · seed 1337 · target `sprite_crop_barley_0`

ASSET sprite_crop_barley_0 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_0","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.7s"},{"name":"path-back","pass":true,"detail":"arrived in 5.7s"}],"pass":true}
```

## RUN 2026-07-10 02:33 UTC · seed 1337 · target `sprite_crop_barley_5`

ASSET sprite_crop_barley_5 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_5","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":[],"blocks_placed":273,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.1s"},{"name":"path-back","pass":true,"detail":"arrived in 5.8s"}],"pass":true}
```

## RUN 2026-07-10 02:33 UTC · seed 1337 · target `prop_muster_bell`

ASSET prop_muster_bell DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_muster_bell","category":"Prop","mode":"load-only (figure-scale dims Some((7, 5, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 02:34 UTC · seed 1337 · target `mine_headframe_human`

ASSET mine_headframe_human DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_headframe_human","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 14)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":[],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"0 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 02:34 UTC · seed 1337 · target `sprite_caveflora_deep`

ASSET sprite_caveflora_deep DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_caveflora_deep","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x40: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x6: expected world-band default, resolved PalmLeavesInner [ok]"],"blocks_placed":78,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.2s"},{"name":"path-back","pass":true,"detail":"arrived in 5.4s"}],"pass":true}
```

## RUN 2026-07-10 02:35 UTC · seed 1337 · target `gate_brick_line`

ASSET gate_brick_line DYNAMIC-ISOLATED: PASS
```json
{"id":"gate_brick_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1236,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.5s, best dist 11.6"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.7s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.9s"}],"pass":true}
```

## RUN 2026-07-10 04:09 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.7s"},{"name":"egress","pass":true,"detail":"arrived in 8.3s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 12.5s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 10.4s"}],"pass":true}
```

## RUN 2026-07-10 04:10 UTC · seed 1337 · target `terracotta_set_demo`

ASSET terracotta_set_demo DYNAMIC-ISOLATED: FAIL
```json
{"id":"terracotta_set_demo","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 217 x22: expected Filled, resolved Filled [ok]","byte 217 x22: expected 22 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":6153,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 9.5s"},{"name":"egress","pass":true,"detail":"arrived in 5.9s"},{"name":"multi-occupancy-in","pass":false,"detail":"Ivo of the Ford STUCK after 10.0s"},{"name":"multi-occupancy-out","pass":false,"detail":"Ivo of the Ford STUCK after 10.0s"}],"pass":false}
```

## RUN 2026-07-10 04:10 UTC · seed 1337 · target `structure_lighthouse`

ASSET structure_lighthouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_lighthouse","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 24)); declared cast 'interior' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 223 x5: expected Filled, resolved Filled [ok]","byte 223 x5: expected 5 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 04:12 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":2077,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit"},{"name":"glow-emission-b217","pass":true,"detail":"13/13 light cells emit"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.2s"},{"name":"egress","pass":true,"detail":"arrived in 8.5s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 11.0s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.0s"}],"pass":false}
```

## RUN 2026-07-10 04:13 UTC · seed 1337 · target `terracotta_set_demo`

ASSET terracotta_set_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"terracotta_set_demo","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 217 x22: expected Filled, resolved Filled [ok]","byte 217 x22: expected 22 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":6153,"misc_blocks":6130,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"},{"name":"glow-emission-b217","pass":true,"detail":"22/22 light cells emit"},{"name":"reach-interior","pass":true,"detail":"arrived in 9.1s"},{"name":"egress","pass":true,"detail":"arrived in 5.6s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 9.9s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.3s"}],"pass":true}
```

## RUN 2026-07-10 04:13 UTC · seed 1337 · target `structure_lighthouse`

ASSET structure_lighthouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_lighthouse","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 24)); declared cast 'interior' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 223 x5: expected Filled, resolved Filled [ok]","byte 223 x5: expected 5 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 04:19 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":2077,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"13/13 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.5s"},{"name":"egress","pass":true,"detail":"arrived in 7.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 11.8s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.6s"}],"pass":false}
```

## RUN 2026-07-10 04:24 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":2077,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"13/13 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.2s"},{"name":"egress","pass":true,"detail":"arrived in 8.4s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 7.8s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 9.7s"}],"pass":false}
```

## RUN 2026-07-10 04:26 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":2077,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"13/13 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.7s"},{"name":"egress","pass":true,"detail":"arrived in 8.6s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 7.4s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 9.3s"}],"pass":false}
```

## RUN 2026-07-10 04:28 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2227: expected world-band default, resolved Hollow [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x13: expected Filled, resolved Filled [ok]","byte 217 x13: expected 13 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":2077,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"13/13 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 6.7s"},{"name":"egress","pass":true,"detail":"arrived in 8.8s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 12.5s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 9.7s"}],"pass":false}
```

## RUN 2026-07-10 04:34 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2229: expected world-band default, resolved Hollow [ok]","byte 32 x503: expected Filled, resolved Filled [ok]","byte 33 x461: expected Filled, resolved Filled [ok]","byte 34 x319: expected Filled, resolved Filled [ok]","byte 35 x316: expected Filled, resolved Filled [ok]","byte 36 x154: expected Filled, resolved Filled [ok]","byte 37 x163: expected Filled, resolved Filled [ok]","byte 38 x125: expected Filled, resolved Filled [ok]","byte 39 x31: expected Filled, resolved Filled [ok]","byte 40 x5: expected Filled, resolved Filled [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x11: expected Filled, resolved Filled [ok]","byte 217 x11: expected 11 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"14 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"11/11 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 6.8s"},{"name":"egress","pass":true,"detail":"arrived in 8.1s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 12.2s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.5s"}],"pass":false}
```

## RUN 2026-07-10 04:38 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2229: expected world-band default, resolved Hollow [ok]","byte 32 x503: expected Filled, resolved Filled [ok]","byte 33 x461: expected Filled, resolved Filled [ok]","byte 34 x319: expected Filled, resolved Filled [ok]","byte 35 x316: expected Filled, resolved Filled [ok]","byte 36 x154: expected Filled, resolved Filled [ok]","byte 37 x163: expected Filled, resolved Filled [ok]","byte 38 x125: expected Filled, resolved Filled [ok]","byte 39 x31: expected Filled, resolved Filled [ok]","byte 40 x5: expected Filled, resolved Filled [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x11: expected Filled, resolved Filled [ok]","byte 217 x11: expected 11 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"14 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"11/11 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.0s"},{"name":"egress","pass":true,"detail":"arrived in 8.4s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 11.7s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 8.4s"}],"pass":false}
```

## RUN 2026-07-10 04:45 UTC · seed 1337 · target `all`

ASSET defense_palisade_line_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"defense_palisade_line_demo","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x32: expected Filled, resolved Filled [ok]","byte 33 x14: expected Filled, resolved Filled [ok]","byte 34 x155: expected Filled, resolved Filled [ok]","byte 35 x102: expected Filled, resolved Filled [ok]","byte 36 x51: expected Filled, resolved Filled [ok]","byte 37 x16: expected Filled, resolved Filled [ok]","byte 38 x18: expected Filled, resolved Filled [ok]","byte 39 x1: expected Filled, resolved Filled [ok]","byte 40 x1: expected Filled, resolved Filled [ok]","byte 200 x8: expected Sprite, resolved Sprite [ok]","byte 200 x8: expected 8 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":398,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"11 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.4s, best dist 11.2"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.6s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.5s"}],"pass":true}
```
ASSET flora_highland_rowan DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_highland_rowan","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 1 x449: expected world-band default, resolved TemperateLeaves [ok]","byte 8 x11: expected world-band default, resolved Fruit [ok]","byte 32 x54: expected Filled, resolved Filled [ok]","byte 33 x112: expected Filled, resolved Filled [ok]","byte 34 x52: expected Filled, resolved Filled [ok]"],"blocks_placed":672,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 0.8s"},{"name":"path-back","pass":true,"detail":"arrived in 5.8s"}],"pass":true}
```
ASSET flora_pine_snowdusted DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_pine_snowdusted","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 2 x2683: expected world-band default, resolved PineLeaves [ok]","byte 97 x72: expected Filled, resolved Filled [ok]","byte 98 x229: expected Filled, resolved Filled [ok]","byte 190 x576: expected Filled, resolved Filled [ok]","byte 191 x577: expected Filled, resolved Filled [ok]"],"blocks_placed":4137,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.8s"},{"name":"path-back","pass":true,"detail":"arrived in 6.7s"}],"pass":true}
```
ASSET flora_rowan_sapling DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_rowan_sapling","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 1 x40: expected world-band default, resolved TemperateLeaves [ok]","byte 32 x4: expected Filled, resolved Filled [ok]","byte 33 x3: expected Filled, resolved Filled [ok]"],"blocks_placed":47,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.7s"},{"name":"path-back","pass":true,"detail":"arrived in 5.1s"}],"pass":true}
```
ASSET flora_rowan_snag DYNAMIC-ISOLATED: PASS
```json
{"id":"flora_rowan_snag","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x16: expected Filled, resolved Filled [ok]","byte 33 x26: expected Filled, resolved Filled [ok]","byte 34 x17: expected Filled, resolved Filled [ok]"],"blocks_placed":59,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 4.9s"},{"name":"path-back","pass":true,"detail":"arrived in 5.1s"}],"pass":true}
```
ASSET gate_brick_line DYNAMIC-ISOLATED: PASS
```json
{"id":"gate_brick_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x282: expected Filled, resolved Filled [ok]","byte 33 x286: expected Filled, resolved Filled [ok]","byte 34 x395: expected Filled, resolved Filled [ok]","byte 35 x185: expected Filled, resolved Filled [ok]","byte 36 x32: expected Filled, resolved Filled [ok]","byte 37 x16: expected Filled, resolved Filled [ok]","byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1236,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"8 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.6s, best dist 15.8"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 2.6s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.7s"}],"pass":true}
```
ASSET gate_dwarven_line DYNAMIC-ISOLATED: PASS
```json
{"id":"gate_dwarven_line","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x263: expected Filled, resolved Filled [ok]","byte 33 x268: expected Filled, resolved Filled [ok]","byte 34 x297: expected Filled, resolved Filled [ok]","byte 35 x322: expected Filled, resolved Filled [ok]","byte 36 x102: expected Filled, resolved Filled [ok]","byte 37 x74: expected Filled, resolved Filled [ok]","byte 200 x40: expected Sprite, resolved Sprite [ok]","byte 200 x40: expected 40 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":1366,"misc_blocks":74,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"8 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.7s, best dist 11.3"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 3.3s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.7s"}],"pass":true}
```
ASSET mine_breach_maw DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_breach_maw","category":"Prop","mode":"load-only (figure-scale dims Some((11, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x56: expected Filled, resolved Filled [ok]","byte 33 x21: expected Filled, resolved Filled [ok]","byte 34 x33: expected Filled, resolved Filled [ok]","byte 35 x19: expected Filled, resolved Filled [ok]","byte 36 x5: expected Filled, resolved Filled [ok]","byte 37 x3: expected Filled, resolved Filled [ok]","byte 38 x1: expected Filled, resolved Filled [ok]","byte 40 x6: expected Filled, resolved Filled [ok]","byte 42 x2: expected Filled, resolved Filled [ok]","byte 217 x3: expected Filled, resolved Filled [ok]","byte 217 x3: expected 3 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"11 distinct bytes checked"}],"pass":true}
```
ASSET mine_headframe_human DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_headframe_human","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 14)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x39: expected Filled, resolved Filled [ok]","byte 33 x31: expected Filled, resolved Filled [ok]","byte 34 x36: expected Filled, resolved Filled [ok]","byte 35 x35: expected Filled, resolved Filled [ok]","byte 36 x9: expected Filled, resolved Filled [ok]","byte 37 x4: expected Filled, resolved Filled [ok]","byte 38 x11: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"7 distinct bytes checked"}],"pass":true}
```
ASSET mine_pithead_human DYNAMIC-ISOLATED: PASS
```json
{"id":"mine_pithead_human","category":"Prop","mode":"load-only (figure-scale dims Some((9, 9, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x21: expected Filled, resolved Filled [ok]","byte 33 x24: expected Filled, resolved Filled [ok]","byte 34 x21: expected Filled, resolved Filled [ok]","byte 35 x14: expected Filled, resolved Filled [ok]","byte 36 x1: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET pier_line_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"pier_line_demo","category":"Prop","mode":"load-only (figure-scale dims Some((6, 24, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x34: expected Filled, resolved Filled [ok]","byte 33 x36: expected Filled, resolved Filled [ok]","byte 34 x61: expected Filled, resolved Filled [ok]","byte 35 x19: expected Filled, resolved Filled [ok]","byte 36 x19: expected Filled, resolved Filled [ok]","byte 37 x5: expected Filled, resolved Filled [ok]","byte 38 x4: expected Filled, resolved Filled [ok]","byte 224 x1: expected Sprite, resolved Sprite [ok]","byte 224 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"9 distinct bytes checked"}],"pass":true}
```
ASSET prop_altar_stone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_altar_stone","category":"Prop","mode":"load-only (figure-scale dims Some((4, 3, 3)); declared cast 'interact-adjacent' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x1: expected world-band default, resolved PalmLeavesOuter [ok]","byte 32 x6: expected Filled, resolved Filled [ok]","byte 33 x10: expected Filled, resolved Filled [ok]","byte 34 x7: expected Filled, resolved Filled [ok]","byte 36 x4: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET prop_banner_post DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_banner_post","category":"Prop","mode":"load-only (figure-scale dims Some((7, 3, 15)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x9: expected Filled, resolved Filled [ok]","byte 33 x9: expected Filled, resolved Filled [ok]","byte 35 x7: expected Filled, resolved Filled [ok]","byte 36 x6: expected Filled, resolved Filled [ok]","byte 37 x6: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET prop_bed_fourpost DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_bed_fourpost","category":"Prop","mode":"load-only (figure-scale dims Some((9, 13, 12)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x35: expected Filled, resolved Filled [ok]","byte 33 x35: expected Filled, resolved Filled [ok]","byte 34 x35: expected Filled, resolved Filled [ok]","byte 35 x36: expected Filled, resolved Filled [ok]","byte 36 x29: expected Filled, resolved Filled [ok]","byte 37 x12: expected Filled, resolved Filled [ok]","byte 38 x24: expected Filled, resolved Filled [ok]","byte 39 x17: expected Filled, resolved Filled [ok]","byte 40 x6: expected Filled, resolved Filled [ok]","byte 41 x6: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"10 distinct bytes checked"}],"pass":true}
```
ASSET prop_chair_fine DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_chair_fine","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 35 x9: expected Filled, resolved Filled [ok]","byte 36 x7: expected Filled, resolved Filled [ok]","byte 37 x9: expected Filled, resolved Filled [ok]","byte 38 x8: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET prop_chair_masterwork DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_chair_masterwork","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]","byte 35 x9: expected Filled, resolved Filled [ok]","byte 36 x8: expected Filled, resolved Filled [ok]","byte 37 x9: expected Filled, resolved Filled [ok]","byte 38 x8: expected Filled, resolved Filled [ok]","byte 39 x2: expected Filled, resolved Filled [ok]","byte 40 x1: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"7 distinct bytes checked"}],"pass":true}
```
ASSET prop_chair_plain DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_chair_plain","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x7: expected Filled, resolved Filled [ok]","byte 33 x10: expected Filled, resolved Filled [ok]","byte 34 x8: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"}],"pass":true}
```
ASSET prop_claim_cairn DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_claim_cairn","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 8)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x25: expected Filled, resolved Filled [ok]","byte 33 x25: expected Filled, resolved Filled [ok]","byte 34 x26: expected Filled, resolved Filled [ok]","byte 35 x2: expected Filled, resolved Filled [ok]","byte 36 x3: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET prop_handcart DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_handcart","category":"Prop","mode":"load-only (figure-scale dims Some((22, 12, 12)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x46: expected Filled, resolved Filled [ok]","byte 33 x89: expected Filled, resolved Filled [ok]","byte 34 x43: expected Filled, resolved Filled [ok]","byte 35 x24: expected Filled, resolved Filled [ok]","byte 36 x16: expected Filled, resolved Filled [ok]","byte 37 x29: expected Filled, resolved Filled [ok]","byte 38 x19: expected Filled, resolved Filled [ok]","byte 39 x4: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"8 distinct bytes checked"}],"pass":true}
```
ASSET prop_hanging_lantern DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_hanging_lantern","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 10)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x15: expected Filled, resolved Filled [ok]","byte 33 x18: expected Filled, resolved Filled [ok]","byte 34 x2: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"}],"pass":true}
```
ASSET prop_harbor_crane DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_harbor_crane","category":"Prop","mode":"load-only (figure-scale dims Some((11, 7, 13)); declared cast 'work-marker' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x6: expected Filled, resolved Filled [ok]","byte 33 x14: expected Filled, resolved Filled [ok]","byte 35 x3: expected Filled, resolved Filled [ok]","byte 36 x5: expected Filled, resolved Filled [ok]","byte 37 x8: expected Filled, resolved Filled [ok]","byte 38 x4: expected Filled, resolved Filled [ok]","byte 40 x2: expected Filled, resolved Filled [ok]","byte 41 x1: expected Filled, resolved Filled [ok]","byte 42 x5: expected Filled, resolved Filled [ok]","byte 224 x1: expected Sprite, resolved Sprite [ok]","byte 224 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"11 distinct bytes checked"}],"pass":true}
```
ASSET prop_hearth_human DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_hearth_human","category":"Prop","mode":"load-only (figure-scale dims Some((9, 4, 9)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x1: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x3: expected world-band default, resolved PalmLeavesInner [ok]","byte 32 x26: expected Filled, resolved Filled [ok]","byte 33 x27: expected Filled, resolved Filled [ok]","byte 34 x24: expected Filled, resolved Filled [ok]","byte 35 x15: expected Filled, resolved Filled [ok]","byte 39 x2: expected Filled, resolved Filled [ok]","byte 40 x2: expected Filled, resolved Filled [ok]","byte 41 x2: expected Filled, resolved Filled [ok]","byte 42 x2: expected Filled, resolved Filled [ok]","byte 43 x5: expected Filled, resolved Filled [ok]","byte 44 x1: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"12 distinct bytes checked"}],"pass":true}
```
ASSET prop_mooring_bollard DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_mooring_bollard","category":"Prop","mode":"load-only (figure-scale dims Some((3, 3, 4)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x3: expected Filled, resolved Filled [ok]","byte 33 x2: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```
ASSET prop_muster_bell DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_muster_bell","category":"Prop","mode":"load-only (figure-scale dims Some((7, 5, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x8: expected Filled, resolved Filled [ok]","byte 33 x8: expected Filled, resolved Filled [ok]","byte 34 x8: expected Filled, resolved Filled [ok]","byte 35 x7: expected Filled, resolved Filled [ok]","byte 36 x1: expected Filled, resolved Filled [ok]","byte 37 x5: expected Filled, resolved Filled [ok]","byte 38 x6: expected Filled, resolved Filled [ok]","byte 39 x2: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"8 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_logs DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_logs","category":"Prop","mode":"load-only (figure-scale dims Some((11, 7, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x51: expected Filled, resolved Filled [ok]","byte 33 x66: expected Filled, resolved Filled [ok]","byte 35 x8: expected Filled, resolved Filled [ok]","byte 36 x8: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_ore DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_ore","category":"Prop","mode":"load-only (figure-scale dims Some((8, 8, 4)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x19: expected Filled, resolved Filled [ok]","byte 33 x15: expected Filled, resolved Filled [ok]","byte 34 x26: expected Filled, resolved Filled [ok]","byte 35 x25: expected Filled, resolved Filled [ok]","byte 36 x4: expected Filled, resolved Filled [ok]","byte 37 x3: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"}],"pass":true}
```
ASSET prop_pile_stone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_pile_stone","category":"Prop","mode":"load-only (figure-scale dims Some((9, 9, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x41: expected Filled, resolved Filled [ok]","byte 33 x52: expected Filled, resolved Filled [ok]","byte 34 x63: expected Filled, resolved Filled [ok]","byte 35 x52: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET prop_potted_herb DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_potted_herb","category":"Prop","mode":"load-only (figure-scale dims Some((4, 4, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x5: expected Filled, resolved Filled [ok]","byte 33 x3: expected Filled, resolved Filled [ok]","byte 34 x2: expected Filled, resolved Filled [ok]","byte 35 x3: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET prop_statue_ancestor DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_statue_ancestor","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 14)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x18: expected Filled, resolved Filled [ok]","byte 33 x22: expected Filled, resolved Filled [ok]","byte 34 x23: expected Filled, resolved Filled [ok]","byte 35 x4: expected Filled, resolved Filled [ok]","byte 36 x15: expected Filled, resolved Filled [ok]","byte 37 x15: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"}],"pass":true}
```
ASSET prop_stockpile_post DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_stockpile_post","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x2: expected world-band default, resolved PalmLeavesOuter [ok]","byte 32 x14: expected Filled, resolved Filled [ok]","byte 33 x11: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"}],"pass":true}
```
ASSET prop_throne DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_throne","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x34: expected Filled, resolved Filled [ok]","byte 33 x26: expected Filled, resolved Filled [ok]","byte 34 x26: expected Filled, resolved Filled [ok]","byte 35 x26: expected Filled, resolved Filled [ok]","byte 36 x3: expected Filled, resolved Filled [ok]","byte 37 x1: expected Filled, resolved Filled [ok]","byte 38 x1: expected Filled, resolved Filled [ok]","byte 39 x1: expected Filled, resolved Filled [ok]","byte 40 x1: expected Filled, resolved Filled [ok]","byte 41 x1: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"10 distinct bytes checked"}],"pass":true}
```
ASSET prop_wallart_tapestry DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_wallart_tapestry","category":"Prop","mode":"load-only (figure-scale dims Some((11, 2, 13)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x18: expected Filled, resolved Filled [ok]","byte 33 x18: expected Filled, resolved Filled [ok]","byte 34 x19: expected Filled, resolved Filled [ok]","byte 35 x19: expected Filled, resolved Filled [ok]","byte 36 x15: expected Filled, resolved Filled [ok]","byte 37 x11: expected Filled, resolved Filled [ok]","byte 38 x10: expected Filled, resolved Filled [ok]","byte 39 x10: expected Filled, resolved Filled [ok]","byte 40 x7: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"9 distinct bytes checked"}],"pass":true}
```
ASSET prop_wallart_trophy_skull DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_wallart_trophy_skull","category":"Prop","mode":"load-only (figure-scale dims Some((7, 4, 8)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x4: expected Filled, resolved Filled [ok]","byte 33 x4: expected Filled, resolved Filled [ok]","byte 34 x4: expected Filled, resolved Filled [ok]","byte 35 x7: expected Filled, resolved Filled [ok]","byte 36 x7: expected Filled, resolved Filled [ok]","byte 37 x8: expected Filled, resolved Filled [ok]","byte 38 x8: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"7 distinct bytes checked"}],"pass":true}
```
ASSET prop_waystone DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_waystone","category":"Prop","mode":"load-only (figure-scale dims Some((7, 5, 11)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]","byte 32 x61: expected Filled, resolved Filled [ok]","byte 33 x31: expected Filled, resolved Filled [ok]","byte 34 x34: expected Filled, resolved Filled [ok]","byte 35 x24: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET prop_zonemarker_meeting_totem DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_zonemarker_meeting_totem","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 17)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x9: expected Filled, resolved Filled [ok]","byte 33 x4: expected Filled, resolved Filled [ok]","byte 34 x20: expected Filled, resolved Filled [ok]","byte 35 x3: expected Filled, resolved Filled [ok]","byte 36 x3: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET prop_zonemarker_refuse_stake DYNAMIC-ISOLATED: PASS
```json
{"id":"prop_zonemarker_refuse_stake","category":"Prop","mode":"load-only (figure-scale dims Some((5, 5, 11)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x4: expected Filled, resolved Filled [ok]","byte 33 x3: expected Filled, resolved Filled [ok]","byte 34 x2: expected Filled, resolved Filled [ok]","byte 36 x3: expected Filled, resolved Filled [ok]","byte 37 x1: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET sprite_bin_wood DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_bin_wood","category":"Prop","mode":"load-only (figure-scale dims Some((10, 8, 6)); declared cast 'interact-adjacent' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x52: expected Filled, resolved Filled [ok]","byte 33 x18: expected Filled, resolved Filled [ok]","byte 34 x19: expected Filled, resolved Filled [ok]","byte 35 x55: expected Filled, resolved Filled [ok]","byte 36 x44: expected Filled, resolved Filled [ok]","byte 37 x11: expected Filled, resolved Filled [ok]","byte 38 x13: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"7 distinct bytes checked"}],"pass":true}
```
ASSET sprite_cave_gloomcap DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_cave_gloomcap","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x52: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x44: expected world-band default, resolved PalmLeavesInner [ok]","byte 16 x31: expected world-band default, resolved Hollow [ok]","byte 32 x12: expected Filled, resolved Filled [ok]","byte 33 x10: expected Filled, resolved Filled [ok]","byte 34 x52: expected Filled, resolved Filled [ok]"],"blocks_placed":201,"misc_blocks":74,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 2.2s"},{"name":"path-back","pass":true,"detail":"arrived in 5.5s"}],"pass":true}
```
ASSET sprite_caveflora_deep DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_caveflora_deep","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x46: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x6: expected world-band default, resolved PalmLeavesInner [ok]","byte 32 x7: expected Filled, resolved Filled [ok]","byte 34 x18: expected Filled, resolved Filled [ok]","byte 35 x10: expected Filled, resolved Filled [ok]"],"blocks_placed":87,"misc_blocks":18,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 6.0s"},{"name":"path-back","pass":true,"detail":"arrived in 5.5s"}],"pass":true}
```
ASSET sprite_caveflora_shallow DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_caveflora_shallow","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x4: expected world-band default, resolved PalmLeavesOuter [ok]","byte 32 x2: expected Filled, resolved Filled [ok]","byte 33 x7: expected Filled, resolved Filled [ok]","byte 35 x16: expected Filled, resolved Filled [ok]","byte 36 x9: expected Filled, resolved Filled [ok]"],"blocks_placed":38,"misc_blocks":16,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.8s"},{"name":"path-back","pass":true,"detail":"arrived in 6.1s"}],"pass":true}
```
ASSET sprite_crop_barley_0 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_0","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x2: expected Filled, resolved Filled [ok]","byte 33 x2: expected Filled, resolved Filled [ok]","byte 34 x1: expected Filled, resolved Filled [ok]"],"blocks_placed":5,"misc_blocks":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.5s"},{"name":"path-back","pass":true,"detail":"arrived in 5.3s"}],"pass":true}
```
ASSET sprite_crop_barley_1 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_1","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x10: expected Filled, resolved Filled [ok]","byte 33 x11: expected Filled, resolved Filled [ok]","byte 34 x11: expected Filled, resolved Filled [ok]"],"blocks_placed":32,"misc_blocks":32,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.4s"},{"name":"path-back","pass":true,"detail":"arrived in 5.1s"}],"pass":true}
```
ASSET sprite_crop_barley_2 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_2","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x24: expected Filled, resolved Filled [ok]","byte 33 x25: expected Filled, resolved Filled [ok]","byte 34 x25: expected Filled, resolved Filled [ok]"],"blocks_placed":74,"misc_blocks":74,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.8s"},{"name":"path-back","pass":true,"detail":"arrived in 5.3s"}],"pass":true}
```
ASSET sprite_crop_barley_3 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_3","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x44: expected Filled, resolved Filled [ok]","byte 33 x47: expected Filled, resolved Filled [ok]","byte 34 x46: expected Filled, resolved Filled [ok]"],"blocks_placed":137,"misc_blocks":137,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.6s"},{"name":"path-back","pass":true,"detail":"arrived in 5.5s"}],"pass":true}
```
ASSET sprite_crop_barley_4 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_4","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x61: expected Filled, resolved Filled [ok]","byte 33 x58: expected Filled, resolved Filled [ok]","byte 34 x61: expected Filled, resolved Filled [ok]","byte 35 x15: expected Filled, resolved Filled [ok]","byte 36 x15: expected Filled, resolved Filled [ok]"],"blocks_placed":210,"misc_blocks":210,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.6s"},{"name":"path-back","pass":true,"detail":"arrived in 5.7s"}],"pass":true}
```
ASSET sprite_crop_barley_5 DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_crop_barley_5","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x78: expected Filled, resolved Filled [ok]","byte 33 x75: expected Filled, resolved Filled [ok]","byte 34 x75: expected Filled, resolved Filled [ok]","byte 35 x30: expected Filled, resolved Filled [ok]","byte 36 x15: expected Filled, resolved Filled [ok]"],"blocks_placed":273,"misc_blocks":273,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 5.6s"},{"name":"path-back","pass":true,"detail":"arrived in 6.2s"}],"pass":true}
```
ASSET sprite_goods_ale_keg DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_ale_keg","category":"Prop","mode":"load-only (figure-scale dims Some((7, 7, 7)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x53: expected Filled, resolved Filled [ok]","byte 33 x49: expected Filled, resolved Filled [ok]","byte 34 x7: expected Filled, resolved Filled [ok]","byte 35 x33: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_bread DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_bread","category":"Prop","mode":"load-only (figure-scale dims Some((10, 7, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x18: expected Filled, resolved Filled [ok]","byte 33 x9: expected Filled, resolved Filled [ok]","byte 34 x9: expected Filled, resolved Filled [ok]","byte 35 x35: expected Filled, resolved Filled [ok]","byte 36 x31: expected Filled, resolved Filled [ok]","byte 37 x4: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_cloth DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_cloth","category":"Prop","mode":"load-only (figure-scale dims Some((9, 6, 6)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x25: expected Filled, resolved Filled [ok]","byte 33 x32: expected Filled, resolved Filled [ok]","byte 34 x31: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_ingots DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_ingots","category":"Prop","mode":"load-only (figure-scale dims Some((8, 6, 4)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x33: expected Filled, resolved Filled [ok]","byte 33 x15: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"2 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_leather DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_leather","category":"Prop","mode":"load-only (figure-scale dims Some((9, 6, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x17: expected Filled, resolved Filled [ok]","byte 33 x17: expected Filled, resolved Filled [ok]","byte 34 x18: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"3 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_planks DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_planks","category":"Prop","mode":"load-only (figure-scale dims Some((9, 7, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x11: expected Filled, resolved Filled [ok]","byte 33 x7: expected Filled, resolved Filled [ok]","byte 34 x25: expected Filled, resolved Filled [ok]","byte 35 x27: expected Filled, resolved Filled [ok]","byte 36 x37: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"5 distinct bytes checked"}],"pass":true}
```
ASSET sprite_goods_stoneblocks DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_goods_stoneblocks","category":"Prop","mode":"load-only (figure-scale dims Some((9, 7, 5)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x60: expected Filled, resolved Filled [ok]","byte 33 x28: expected Filled, resolved Filled [ok]","byte 34 x35: expected Filled, resolved Filled [ok]","byte 35 x21: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET sprite_ladder_iron_dwarven DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_ladder_iron_dwarven","category":"Prop","mode":"load-only (figure-scale dims Some((11, 3, 33)); declared cast 'path-past' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 14 x1: expected world-band default, resolved PalmLeavesOuter [ok]","byte 32 x45: expected Filled, resolved Filled [ok]","byte 33 x46: expected Filled, resolved Filled [ok]","byte 34 x16: expected Filled, resolved Filled [ok]","byte 35 x182: expected Filled, resolved Filled [ok]","byte 36 x46: expected Filled, resolved Filled [ok]","byte 37 x88: expected Filled, resolved Filled [ok]","byte 38 x46: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"8 distinct bytes checked"}],"pass":true}
```
ASSET sprite_ladder_rope DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_ladder_rope","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 11)); declared cast 'climb' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x46: expected Filled, resolved Filled [ok]","byte 33 x36: expected Filled, resolved Filled [ok]","byte 84 x3: expected Filled, resolved Filled [ok]","byte 85 x3: expected Filled, resolved Filled [ok]","byte 86 x13: expected Filled, resolved Filled [ok]","byte 87 x8: expected Filled, resolved Filled [ok]","byte 88 x4: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"7 distinct bytes checked"}],"pass":true}
```
ASSET sprite_orevein_bloodstone DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_orevein_bloodstone","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x3: expected world-band default, resolved PalmLeavesOuter [ok]","byte 32 x27: expected Filled, resolved Filled [ok]","byte 33 x29: expected Filled, resolved Filled [ok]","byte 34 x24: expected Filled, resolved Filled [ok]","byte 35 x25: expected Filled, resolved Filled [ok]","byte 36 x7: expected Filled, resolved Filled [ok]","byte 37 x11: expected Filled, resolved Filled [ok]"],"blocks_placed":126,"misc_blocks":18,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"7 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 4.9s"},{"name":"path-back","pass":true,"detail":"arrived in 5.5s"}],"pass":true}
```
ASSET sprite_orevein_velorite DYNAMIC-ISOLATED: FAIL
```json
{"id":"sprite_orevein_velorite","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x8: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x24: expected world-band default, resolved PalmLeavesInner [ok]","byte 32 x22: expected Filled, resolved Filled [ok]","byte 33 x31: expected Filled, resolved Filled [ok]","byte 34 x27: expected Filled, resolved Filled [ok]","byte 35 x29: expected Filled, resolved Filled [ok]"],"blocks_placed":141,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"},{"name":"path-around","pass":false,"detail":"STUCK (watchdog) after 14.0s, best dist 16.1"}],"pass":false}
```
ASSET sprite_sack_grain DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_sack_grain","category":"Prop","mode":"load-only (figure-scale dims Some((8, 8, 8)); declared cast 'interact-adjacent' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x64: expected Filled, resolved Filled [ok]","byte 33 x64: expected Filled, resolved Filled [ok]","byte 34 x54: expected Filled, resolved Filled [ok]","byte 35 x8: expected Filled, resolved Filled [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"4 distinct bytes checked"}],"pass":true}
```
ASSET structure_boathouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_boathouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x128: expected Filled, resolved Filled [ok]","byte 33 x133: expected Filled, resolved Filled [ok]","byte 34 x18: expected Filled, resolved Filled [ok]","byte 36 x44: expected Filled, resolved Filled [ok]","byte 37 x46: expected Filled, resolved Filled [ok]","byte 38 x105: expected Filled, resolved Filled [ok]"],"blocks_placed":475,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"8 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 0.2s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 8.9s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.1s"}],"pass":true}
```
ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2229: expected world-band default, resolved Hollow [ok]","byte 32 x503: expected Filled, resolved Filled [ok]","byte 33 x461: expected Filled, resolved Filled [ok]","byte 34 x319: expected Filled, resolved Filled [ok]","byte 35 x316: expected Filled, resolved Filled [ok]","byte 36 x154: expected Filled, resolved Filled [ok]","byte 37 x163: expected Filled, resolved Filled [ok]","byte 38 x125: expected Filled, resolved Filled [ok]","byte 39 x31: expected Filled, resolved Filled [ok]","byte 40 x5: expected Filled, resolved Filled [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x11: expected Filled, resolved Filled [ok]","byte 217 x11: expected 11 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"14 distinct bytes checked"},{"name":"glow-emission-b136","pass":false,"detail":"0/3 light cells emit (first cell: kind=Air sprite=Some(Empty))"},{"name":"glow-emission-b217","pass":true,"detail":"11/11 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.1s"},{"name":"egress","pass":true,"detail":"arrived in 8.2s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 7.4s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 9.0s"}],"pass":false}
```
ASSET structure_faith_shrine DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_faith_shrine","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x40: expected Filled, resolved Filled [ok]","byte 33 x29: expected Filled, resolved Filled [ok]","byte 34 x35: expected Filled, resolved Filled [ok]","byte 35 x37: expected Filled, resolved Filled [ok]","byte 40 x12: expected Filled, resolved Filled [ok]","byte 41 x44: expected Filled, resolved Filled [ok]","byte 42 x16: expected Filled, resolved Filled [ok]","byte 43 x21: expected Filled, resolved Filled [ok]","byte 44 x4: expected Filled, resolved Filled [ok]","byte 45 x1: expected Filled, resolved Filled [ok]","byte 46 x2: expected Filled, resolved Filled [ok]","byte 47 x3: expected Filled, resolved Filled [ok]","byte 218 x1: expected Filled, resolved Filled [ok]","byte 218 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":245,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"14 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 3.9s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"}],"pass":true}
```
ASSET structure_faith_temple_human DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_faith_temple_human","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x158: expected Filled, resolved Filled [ok]","byte 33 x117: expected Filled, resolved Filled [ok]","byte 34 x124: expected Filled, resolved Filled [ok]","byte 35 x101: expected Filled, resolved Filled [ok]","byte 36 x6: expected Filled, resolved Filled [ok]","byte 37 x9: expected Filled, resolved Filled [ok]","byte 38 x7: expected Filled, resolved Filled [ok]","byte 39 x78: expected Filled, resolved Filled [ok]","byte 40 x43: expected Filled, resolved Filled [ok]","byte 41 x64: expected Filled, resolved Filled [ok]","byte 42 x83: expected Filled, resolved Filled [ok]","byte 43 x87: expected Filled, resolved Filled [ok]","byte 44 x25: expected Filled, resolved Filled [ok]","byte 45 x17: expected Filled, resolved Filled [ok]","byte 47 x1: expected Filled, resolved Filled [ok]","byte 204 x2: expected Sprite, resolved Sprite [ok]","byte 204 x2: expected 2 authored cells (exact), resolved all cells match [ok]","byte 208 x9: expected Sprite, resolved Sprite [ok]","byte 208 x9: expected 9 authored cells (exact), resolved all cells match [ok]","byte 218 x1: expected Filled, resolved Filled [ok]","byte 218 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":932,"misc_blocks":1,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"21 distinct bytes checked"},{"name":"glow-emission-b204","pass":true,"detail":"2/2 light cells emit (first cell: kind=Air sprite=Some(FireBowlGround))"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.1s"},{"name":"egress","pass":true,"detail":"arrived in 3.5s"}],"pass":true}
```
ASSET structure_housing_human_cottage DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_housing_human_cottage","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x54: expected Filled, resolved Filled [ok]","byte 33 x125: expected Filled, resolved Filled [ok]","byte 34 x109: expected Filled, resolved Filled [ok]","byte 35 x83: expected Filled, resolved Filled [ok]","byte 36 x64: expected Filled, resolved Filled [ok]","byte 37 x70: expected Filled, resolved Filled [ok]","byte 38 x94: expected Filled, resolved Filled [ok]","byte 39 x167: expected Filled, resolved Filled [ok]","byte 40 x87: expected Filled, resolved Filled [ok]","byte 41 x65: expected Filled, resolved Filled [ok]","byte 42 x44: expected Filled, resolved Filled [ok]","byte 43 x28: expected Filled, resolved Filled [ok]","byte 44 x26: expected Filled, resolved Filled [ok]","byte 45 x110: expected Filled, resolved Filled [ok]","byte 46 x117: expected Filled, resolved Filled [ok]","byte 47 x51: expected Filled, resolved Filled [ok]"],"blocks_placed":1295,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"18 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.9s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 17.7s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.2s"},{"name":"integrated-reach","pass":true,"detail":"slope 3 across footprint; arrived in 2.9s"},{"name":"integrated-egress","pass":true,"detail":"arrived in 2.5s"}],"pass":true}
```
ASSET structure_lighthouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_lighthouse","category":"Prop","mode":"load-only (figure-scale dims Some((11, 11, 24)); declared cast 'interior' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x106: expected Filled, resolved Filled [ok]","byte 33 x111: expected Filled, resolved Filled [ok]","byte 34 x107: expected Filled, resolved Filled [ok]","byte 35 x124: expected Filled, resolved Filled [ok]","byte 39 x71: expected Filled, resolved Filled [ok]","byte 40 x72: expected Filled, resolved Filled [ok]","byte 41 x69: expected Filled, resolved Filled [ok]","byte 223 x6: expected Filled, resolved Filled [ok]","byte 223 x6: expected 6 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"9 distinct bytes checked"}],"pass":true}
```
ASSET structure_production_smithy DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_production_smithy","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x66: expected Filled, resolved Filled [ok]","byte 33 x135: expected Filled, resolved Filled [ok]","byte 34 x96: expected Filled, resolved Filled [ok]","byte 35 x45: expected Filled, resolved Filled [ok]","byte 36 x17: expected Filled, resolved Filled [ok]","byte 37 x24: expected Filled, resolved Filled [ok]","byte 38 x10: expected Filled, resolved Filled [ok]","byte 39 x43: expected Filled, resolved Filled [ok]","byte 40 x39: expected Filled, resolved Filled [ok]","byte 41 x33: expected Filled, resolved Filled [ok]","byte 42 x45: expected Filled, resolved Filled [ok]","byte 43 x36: expected Filled, resolved Filled [ok]","byte 44 x9: expected Filled, resolved Filled [ok]","byte 45 x15: expected Filled, resolved Filled [ok]","byte 46 x4: expected Filled, resolved Filled [ok]","byte 201 x1: expected Sprite, resolved Sprite [ok]","byte 201 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 202 x1: expected Sprite, resolved Sprite [ok]","byte 202 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":620,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"21 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.4s"},{"name":"egress","pass":true,"detail":"arrived in 4.8s"}],"pass":true}
```
ASSET structure_trade_depot DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_trade_depot","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x2: expected world-band default, resolved MaybeChest [ok]","byte 11 x2: expected 2 authored cells (exact), resolved all cells match [ok]","byte 32 x19: expected Filled, resolved Filled [ok]","byte 33 x12: expected Filled, resolved Filled [ok]","byte 34 x11: expected Filled, resolved Filled [ok]","byte 35 x28: expected Filled, resolved Filled [ok]","byte 36 x10: expected Filled, resolved Filled [ok]","byte 37 x11: expected Filled, resolved Filled [ok]","byte 38 x12: expected Filled, resolved Filled [ok]","byte 40 x30: expected Filled, resolved Filled [ok]","byte 41 x33: expected Filled, resolved Filled [ok]","byte 42 x28: expected Filled, resolved Filled [ok]","byte 43 x26: expected Filled, resolved Filled [ok]","byte 48 x65: expected Filled, resolved Filled [ok]","byte 49 x64: expected Filled, resolved Filled [ok]","byte 50 x66: expected Filled, resolved Filled [ok]","byte 219 x1: expected Sprite, resolved Sprite [ok]","byte 219 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":418,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"18 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 3.8s"},{"name":"egress","pass":true,"detail":"arrived in 5.2s"}],"pass":true}
```
ASSET terracotta_set_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"terracotta_set_demo","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x1284: expected Filled, resolved Filled [ok]","byte 33 x596: expected Filled, resolved Filled [ok]","byte 34 x2489: expected Filled, resolved Filled [ok]","byte 35 x1156: expected Filled, resolved Filled [ok]","byte 36 x605: expected Filled, resolved Filled [ok]","byte 217 x22: expected Filled, resolved Filled [ok]","byte 217 x22: expected 22 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":6153,"misc_blocks":5525,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"9 distinct bytes checked"},{"name":"glow-emission-b217","pass":true,"detail":"22/22 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 9.2s"},{"name":"egress","pass":true,"detail":"arrived in 5.5s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 9.6s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 9.8s"}],"pass":true}
```
ASSET workshop_carpenter DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_carpenter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x37: expected Filled, resolved Filled [ok]","byte 33 x65: expected Filled, resolved Filled [ok]","byte 34 x82: expected Filled, resolved Filled [ok]","byte 35 x112: expected Filled, resolved Filled [ok]","byte 36 x87: expected Filled, resolved Filled [ok]","byte 37 x27: expected Filled, resolved Filled [ok]","byte 38 x24: expected Filled, resolved Filled [ok]","byte 40 x32: expected Filled, resolved Filled [ok]","byte 41 x22: expected Filled, resolved Filled [ok]","byte 42 x32: expected Filled, resolved Filled [ok]","byte 43 x22: expected Filled, resolved Filled [ok]","byte 210 x1: expected Sprite, resolved Sprite [ok]","byte 210 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":544,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"15 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.0s"},{"name":"egress","pass":true,"detail":"arrived in 4.4s"}],"pass":true}
```
ASSET workshop_kitchen DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_kitchen","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x82: expected Filled, resolved Filled [ok]","byte 33 x70: expected Filled, resolved Filled [ok]","byte 34 x75: expected Filled, resolved Filled [ok]","byte 35 x75: expected Filled, resolved Filled [ok]","byte 40 x27: expected Filled, resolved Filled [ok]","byte 41 x23: expected Filled, resolved Filled [ok]","byte 43 x5: expected Filled, resolved Filled [ok]","byte 44 x30: expected Filled, resolved Filled [ok]","byte 45 x44: expected Filled, resolved Filled [ok]","byte 46 x68: expected Filled, resolved Filled [ok]","byte 47 x33: expected Filled, resolved Filled [ok]","byte 48 x1: expected Filled, resolved Filled [ok]","byte 49 x1: expected Filled, resolved Filled [ok]","byte 215 x2: expected Sprite, resolved Sprite [ok]","byte 215 x2: expected 2 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":536,"misc_blocks":1,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"15 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.3s"},{"name":"egress","pass":true,"detail":"arrived in 4.5s"}],"pass":true}
```
ASSET workshop_loomhouse DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_loomhouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x43: expected Filled, resolved Filled [ok]","byte 33 x65: expected Filled, resolved Filled [ok]","byte 34 x77: expected Filled, resolved Filled [ok]","byte 35 x96: expected Filled, resolved Filled [ok]","byte 36 x68: expected Filled, resolved Filled [ok]","byte 37 x31: expected Filled, resolved Filled [ok]","byte 38 x23: expected Filled, resolved Filled [ok]","byte 40 x32: expected Filled, resolved Filled [ok]","byte 41 x22: expected Filled, resolved Filled [ok]","byte 42 x32: expected Filled, resolved Filled [ok]","byte 43 x22: expected Filled, resolved Filled [ok]","byte 50 x8: expected Filled, resolved Filled [ok]","byte 51 x2: expected Filled, resolved Filled [ok]","byte 52 x2: expected Filled, resolved Filled [ok]","byte 216 x1: expected Sprite, resolved Sprite [ok]","byte 216 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":525,"misc_blocks":10,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"18 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.3s"},{"name":"egress","pass":true,"detail":"arrived in 4.7s"}],"pass":true}
```
ASSET workshop_mason DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_mason","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x80: expected Filled, resolved Filled [ok]","byte 33 x73: expected Filled, resolved Filled [ok]","byte 34 x81: expected Filled, resolved Filled [ok]","byte 35 x69: expected Filled, resolved Filled [ok]","byte 40 x24: expected Filled, resolved Filled [ok]","byte 41 x20: expected Filled, resolved Filled [ok]","byte 43 x5: expected Filled, resolved Filled [ok]","byte 44 x31: expected Filled, resolved Filled [ok]","byte 45 x44: expected Filled, resolved Filled [ok]","byte 46 x68: expected Filled, resolved Filled [ok]","byte 47 x35: expected Filled, resolved Filled [ok]","byte 211 x1: expected Sprite, resolved Sprite [ok]","byte 211 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":532,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"15 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.6s"},{"name":"egress","pass":true,"detail":"arrived in 4.0s"}],"pass":true}
```
ASSET workshop_smelter DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_smelter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x74: expected Filled, resolved Filled [ok]","byte 33 x44: expected Filled, resolved Filled [ok]","byte 34 x39: expected Filled, resolved Filled [ok]","byte 35 x47: expected Filled, resolved Filled [ok]","byte 36 x26: expected Filled, resolved Filled [ok]","byte 37 x20: expected Filled, resolved Filled [ok]","byte 39 x5: expected Filled, resolved Filled [ok]","byte 40 x30: expected Filled, resolved Filled [ok]","byte 41 x44: expected Filled, resolved Filled [ok]","byte 42 x68: expected Filled, resolved Filled [ok]","byte 43 x33: expected Filled, resolved Filled [ok]","byte 44 x32: expected Filled, resolved Filled [ok]","byte 45 x22: expected Filled, resolved Filled [ok]","byte 46 x32: expected Filled, resolved Filled [ok]","byte 47 x22: expected Filled, resolved Filled [ok]","byte 204 x1: expected Sprite, resolved Sprite [ok]","byte 204 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 212 x2: expected Sprite, resolved Sprite [ok]","byte 212 x2: expected 2 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":541,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"19 distinct bytes checked"},{"name":"glow-emission-b204","pass":true,"detail":"1/1 light cells emit (first cell: kind=Air sprite=Some(FireBowlGround))"},{"name":"reach-work-marker","pass":false,"detail":"STUCK (watchdog) after 18.1s, best dist 9.3"}],"pass":false}
```
ASSET workshop_tannery DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_tannery","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x112: expected Filled, resolved Filled [ok]","byte 33 x91: expected Filled, resolved Filled [ok]","byte 34 x19: expected Filled, resolved Filled [ok]","byte 36 x8: expected Filled, resolved Filled [ok]","byte 37 x34: expected Filled, resolved Filled [ok]","byte 38 x44: expected Filled, resolved Filled [ok]","byte 39 x68: expected Filled, resolved Filled [ok]","byte 40 x37: expected Filled, resolved Filled [ok]","byte 41 x32: expected Filled, resolved Filled [ok]","byte 42 x22: expected Filled, resolved Filled [ok]","byte 43 x32: expected Filled, resolved Filled [ok]","byte 44 x22: expected Filled, resolved Filled [ok]","byte 51 x8: expected Filled, resolved Filled [ok]","byte 52 x2: expected Filled, resolved Filled [ok]","byte 53 x2: expected Filled, resolved Filled [ok]","byte 213 x1: expected Sprite, resolved Sprite [ok]","byte 213 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":535,"misc_blocks":8,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"19 distinct bytes checked"},{"name":"reach-work-marker","pass":false,"detail":"STUCK (watchdog) after 11.2s, best dist 4.2"}],"pass":false}
```

## RUN 2026-07-10 04:47 UTC · seed 1337 · target `sprite_orevein_velorite`

ASSET sprite_orevein_velorite DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_orevein_velorite","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 14 x8: expected world-band default, resolved PalmLeavesOuter [ok]","byte 15 x24: expected world-band default, resolved PalmLeavesInner [ok]","byte 32 x22: expected Filled, resolved Filled [ok]","byte 33 x31: expected Filled, resolved Filled [ok]","byte 34 x27: expected Filled, resolved Filled [ok]","byte 35 x29: expected Filled, resolved Filled [ok]"],"blocks_placed":141,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"6 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 7.0s"},{"name":"path-back","pass":true,"detail":"arrived in 5.7s"}],"pass":true}
```

## RUN 2026-07-10 04:47 UTC · seed 1337 · target `workshop_smelter`

ASSET workshop_smelter DYNAMIC-ISOLATED: FAIL
```json
{"id":"workshop_smelter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x74: expected Filled, resolved Filled [ok]","byte 33 x44: expected Filled, resolved Filled [ok]","byte 34 x39: expected Filled, resolved Filled [ok]","byte 35 x47: expected Filled, resolved Filled [ok]","byte 36 x26: expected Filled, resolved Filled [ok]","byte 37 x20: expected Filled, resolved Filled [ok]","byte 39 x5: expected Filled, resolved Filled [ok]","byte 40 x30: expected Filled, resolved Filled [ok]","byte 41 x44: expected Filled, resolved Filled [ok]","byte 42 x68: expected Filled, resolved Filled [ok]","byte 43 x33: expected Filled, resolved Filled [ok]","byte 44 x32: expected Filled, resolved Filled [ok]","byte 45 x22: expected Filled, resolved Filled [ok]","byte 46 x32: expected Filled, resolved Filled [ok]","byte 47 x22: expected Filled, resolved Filled [ok]","byte 204 x1: expected Sprite, resolved Sprite [ok]","byte 204 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 212 x2: expected Sprite, resolved Sprite [ok]","byte 212 x2: expected 2 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":541,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"19 distinct bytes checked"},{"name":"glow-emission-b204","pass":true,"detail":"1/1 light cells emit (first cell: kind=Air sprite=Some(FireBowlGround))"},{"name":"reach-work-marker","pass":false,"detail":"STUCK (watchdog) after 18.7s, best dist 9.3"}],"pass":false}
```

## RUN 2026-07-10 05:04 UTC · seed 1337 · target `workshop_smelter`

ASSET workshop_smelter DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_smelter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x74: expected Filled, resolved Filled [ok]","byte 33 x44: expected Filled, resolved Filled [ok]","byte 34 x39: expected Filled, resolved Filled [ok]","byte 35 x47: expected Filled, resolved Filled [ok]","byte 36 x27: expected Filled, resolved Filled [ok]","byte 37 x21: expected Filled, resolved Filled [ok]","byte 39 x5: expected Filled, resolved Filled [ok]","byte 40 x30: expected Filled, resolved Filled [ok]","byte 41 x44: expected Filled, resolved Filled [ok]","byte 42 x68: expected Filled, resolved Filled [ok]","byte 43 x33: expected Filled, resolved Filled [ok]","byte 44 x32: expected Filled, resolved Filled [ok]","byte 45 x22: expected Filled, resolved Filled [ok]","byte 46 x32: expected Filled, resolved Filled [ok]","byte 47 x22: expected Filled, resolved Filled [ok]","byte 204 x2: expected Sprite, resolved Sprite [ok]","byte 204 x2: expected 2 authored cells (exact), resolved all cells match [ok]","byte 212 x1: expected Sprite, resolved Sprite [ok]","byte 212 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":543,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"19 distinct bytes checked"},{"name":"glow-emission-b204","pass":true,"detail":"2/2 light cells emit (first cell: kind=Air sprite=Some(FireBowlGround))"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.6s"},{"name":"egress","pass":true,"detail":"arrived in 4.6s"}],"pass":true}
```

## RUN 2026-07-10 05:04 UTC · seed 1337 · target `workshop_tannery`

ASSET workshop_tannery DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_tannery","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x114: expected Filled, resolved Filled [ok]","byte 33 x91: expected Filled, resolved Filled [ok]","byte 34 x19: expected Filled, resolved Filled [ok]","byte 36 x8: expected Filled, resolved Filled [ok]","byte 37 x33: expected Filled, resolved Filled [ok]","byte 38 x44: expected Filled, resolved Filled [ok]","byte 39 x68: expected Filled, resolved Filled [ok]","byte 40 x38: expected Filled, resolved Filled [ok]","byte 41 x32: expected Filled, resolved Filled [ok]","byte 42 x22: expected Filled, resolved Filled [ok]","byte 43 x32: expected Filled, resolved Filled [ok]","byte 44 x22: expected Filled, resolved Filled [ok]","byte 51 x8: expected Filled, resolved Filled [ok]","byte 52 x2: expected Filled, resolved Filled [ok]","byte 53 x2: expected Filled, resolved Filled [ok]","byte 213 x1: expected Sprite, resolved Sprite [ok]","byte 213 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":537,"misc_blocks":8,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"19 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.0s"},{"name":"egress","pass":true,"detail":"arrived in 4.6s"}],"pass":true}
```

## RUN 2026-07-10 05:05 UTC · seed 1337 · target `workshop_carpenter`

ASSET workshop_carpenter DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_carpenter","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x37: expected Filled, resolved Filled [ok]","byte 33 x65: expected Filled, resolved Filled [ok]","byte 34 x82: expected Filled, resolved Filled [ok]","byte 35 x112: expected Filled, resolved Filled [ok]","byte 36 x88: expected Filled, resolved Filled [ok]","byte 37 x29: expected Filled, resolved Filled [ok]","byte 38 x23: expected Filled, resolved Filled [ok]","byte 40 x32: expected Filled, resolved Filled [ok]","byte 41 x22: expected Filled, resolved Filled [ok]","byte 42 x32: expected Filled, resolved Filled [ok]","byte 43 x22: expected Filled, resolved Filled [ok]","byte 210 x1: expected Sprite, resolved Sprite [ok]","byte 210 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":546,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"15 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 5.1s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"}],"pass":true}
```

## RUN 2026-07-10 05:05 UTC · seed 1337 · target `workshop_mason`

ASSET workshop_mason DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_mason","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x80: expected Filled, resolved Filled [ok]","byte 33 x73: expected Filled, resolved Filled [ok]","byte 34 x81: expected Filled, resolved Filled [ok]","byte 35 x69: expected Filled, resolved Filled [ok]","byte 40 x25: expected Filled, resolved Filled [ok]","byte 41 x21: expected Filled, resolved Filled [ok]","byte 43 x5: expected Filled, resolved Filled [ok]","byte 44 x31: expected Filled, resolved Filled [ok]","byte 45 x44: expected Filled, resolved Filled [ok]","byte 46 x68: expected Filled, resolved Filled [ok]","byte 47 x35: expected Filled, resolved Filled [ok]","byte 211 x1: expected Sprite, resolved Sprite [ok]","byte 211 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":534,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"15 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.7s"},{"name":"egress","pass":true,"detail":"arrived in 4.3s"}],"pass":true}
```

## RUN 2026-07-10 05:05 UTC · seed 1337 · target `workshop_kitchen`

ASSET workshop_kitchen DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_kitchen","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x82: expected Filled, resolved Filled [ok]","byte 33 x70: expected Filled, resolved Filled [ok]","byte 34 x75: expected Filled, resolved Filled [ok]","byte 35 x75: expected Filled, resolved Filled [ok]","byte 40 x28: expected Filled, resolved Filled [ok]","byte 41 x24: expected Filled, resolved Filled [ok]","byte 43 x5: expected Filled, resolved Filled [ok]","byte 44 x30: expected Filled, resolved Filled [ok]","byte 45 x44: expected Filled, resolved Filled [ok]","byte 46 x68: expected Filled, resolved Filled [ok]","byte 47 x33: expected Filled, resolved Filled [ok]","byte 48 x1: expected Filled, resolved Filled [ok]","byte 49 x1: expected Filled, resolved Filled [ok]","byte 204 x1: expected Sprite, resolved Sprite [ok]","byte 204 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 215 x1: expected Sprite, resolved Sprite [ok]","byte 215 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":538,"misc_blocks":1,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"17 distinct bytes checked"},{"name":"glow-emission-b204","pass":true,"detail":"1/1 light cells emit (first cell: kind=Air sprite=Some(FireBowlGround))"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.6s"},{"name":"egress","pass":true,"detail":"arrived in 4.4s"}],"pass":true}
```

## RUN 2026-07-10 05:06 UTC · seed 1337 · target `workshop_loomhouse`

ASSET workshop_loomhouse DYNAMIC-ISOLATED: PASS
```json
{"id":"workshop_loomhouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x43: expected Filled, resolved Filled [ok]","byte 33 x65: expected Filled, resolved Filled [ok]","byte 34 x77: expected Filled, resolved Filled [ok]","byte 35 x96: expected Filled, resolved Filled [ok]","byte 36 x68: expected Filled, resolved Filled [ok]","byte 37 x32: expected Filled, resolved Filled [ok]","byte 38 x24: expected Filled, resolved Filled [ok]","byte 40 x32: expected Filled, resolved Filled [ok]","byte 41 x22: expected Filled, resolved Filled [ok]","byte 42 x32: expected Filled, resolved Filled [ok]","byte 43 x22: expected Filled, resolved Filled [ok]","byte 50 x8: expected Filled, resolved Filled [ok]","byte 51 x2: expected Filled, resolved Filled [ok]","byte 52 x2: expected Filled, resolved Filled [ok]","byte 216 x1: expected Sprite, resolved Sprite [ok]","byte 216 x1: expected 1 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":527,"misc_blocks":10,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"18 distinct bytes checked"},{"name":"reach-work-marker","pass":true,"detail":"arrived in 4.2s"},{"name":"egress","pass":true,"detail":"arrived in 4.5s"}],"pass":true}
```

## RUN 2026-07-10 05:06 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: FAIL
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2226: expected world-band default, resolved Hollow [ok]","byte 32 x503: expected Filled, resolved Filled [ok]","byte 33 x461: expected Filled, resolved Filled [ok]","byte 34 x319: expected Filled, resolved Filled [ok]","byte 35 x316: expected Filled, resolved Filled [ok]","byte 36 x154: expected Filled, resolved Filled [ok]","byte 37 x163: expected Filled, resolved Filled [ok]","byte 38 x125: expected Filled, resolved Filled [ok]","byte 39 x34: expected Filled, resolved Filled [ok]","byte 40 x5: expected Filled, resolved Filled [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x11: expected Filled, resolved Filled [ok]","byte 217 x11: expected 11 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"14 distinct bytes checked"},{"name":"glow-emission-b136","pass":true,"detail":"3/3 light cells emit (first cell: kind=Air sprite=Some(Lantern))"},{"name":"glow-emission-b217","pass":true,"detail":"11/11 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 8.0s"},{"name":"egress","pass":true,"detail":"arrived in 8.7s"},{"name":"multi-occupancy-in","pass":false,"detail":"an order was lost (demote mid-travel?)"},{"name":"multi-occupancy-out","pass":false,"detail":"an order was lost (demote mid-travel?)"}],"pass":false}
```

## RUN 2026-07-10 05:06 UTC · seed 1337 · target `structure_lighthouse`

ASSET structure_lighthouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_lighthouse","category":"Other","mode":"load-only (figure-scale dims Some((11, 11, 24)); declared cast 'interior' deferred — world-scale version or sprite-manifest rung needed)","fidelity_ok":true,"marker_checks":["byte 32 x106: expected Filled, resolved Filled [ok]","byte 33 x111: expected Filled, resolved Filled [ok]","byte 34 x107: expected Filled, resolved Filled [ok]","byte 35 x124: expected Filled, resolved Filled [ok]","byte 39 x71: expected Filled, resolved Filled [ok]","byte 40 x72: expected Filled, resolved Filled [ok]","byte 41 x69: expected Filled, resolved Filled [ok]","byte 223 x6: expected Filled, resolved Filled [ok]","byte 223 x6: expected 6 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":0,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"9 distinct bytes checked"}],"pass":true}
```

## RUN 2026-07-10 05:08 UTC · seed 1337 · target `structure_lighthouse`

ASSET structure_lighthouse DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_lighthouse","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x106: expected Filled, resolved Filled [ok]","byte 33 x111: expected Filled, resolved Filled [ok]","byte 34 x107: expected Filled, resolved Filled [ok]","byte 35 x124: expected Filled, resolved Filled [ok]","byte 39 x71: expected Filled, resolved Filled [ok]","byte 40 x72: expected Filled, resolved Filled [ok]","byte 41 x69: expected Filled, resolved Filled [ok]","byte 223 x6: expected Filled, resolved Filled [ok]","byte 223 x6: expected 6 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":666,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"9 distinct bytes checked"},{"name":"glow-emission-b223","pass":true,"detail":"6/6 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 4.0s"},{"name":"egress","pass":true,"detail":"arrived in 3.7s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 15.8s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.6s"}],"pass":true}
```

## RUN 2026-07-10 05:09 UTC · seed 1337 · target `structure_dungeon_quarry_hall`

ASSET structure_dungeon_quarry_hall DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_dungeon_quarry_hall","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 16 x2226: expected world-band default, resolved Hollow [ok]","byte 32 x503: expected Filled, resolved Filled [ok]","byte 33 x461: expected Filled, resolved Filled [ok]","byte 34 x319: expected Filled, resolved Filled [ok]","byte 35 x316: expected Filled, resolved Filled [ok]","byte 36 x154: expected Filled, resolved Filled [ok]","byte 37 x163: expected Filled, resolved Filled [ok]","byte 38 x125: expected Filled, resolved Filled [ok]","byte 39 x34: expected Filled, resolved Filled [ok]","byte 40 x5: expected Filled, resolved Filled [ok]","byte 136 x3: expected Sprite, resolved Sprite [ok]","byte 136 x3: expected 3 authored cells (exact), resolved all cells match [ok]","byte 217 x11: expected Filled, resolved Filled [ok]","byte 217 x11: expected 11 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":4320,"misc_blocks":5,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"14 distinct bytes checked"},{"name":"glow-emission-b136","pass":true,"detail":"3/3 light cells emit (first cell: kind=Air sprite=Some(Lantern))"},{"name":"glow-emission-b217","pass":true,"detail":"11/11 light cells emit (first cell: kind=GlowingRock sprite=None)"},{"name":"reach-interior","pass":true,"detail":"arrived in 7.3s"},{"name":"egress","pass":true,"detail":"arrived in 8.4s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 11.6s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 10.1s"}],"pass":true}
```

## RUN 2026-07-10 06:06 UTC · seed 1337 · target `structure_housing_human_cottage`

ASSET structure_housing_human_cottage DYNAMIC-ISOLATED: PASS
```json
{"id":"structure_housing_human_cottage","category":"Structure","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 11 x1: expected world-band default, resolved MaybeChest [ok]","byte 11 x1: expected 1 authored cells (exact), resolved all cells match [ok]","byte 32 x54: expected Filled, resolved Filled [ok]","byte 33 x125: expected Filled, resolved Filled [ok]","byte 34 x109: expected Filled, resolved Filled [ok]","byte 35 x83: expected Filled, resolved Filled [ok]","byte 36 x64: expected Filled, resolved Filled [ok]","byte 37 x70: expected Filled, resolved Filled [ok]","byte 38 x94: expected Filled, resolved Filled [ok]","byte 39 x167: expected Filled, resolved Filled [ok]","byte 40 x87: expected Filled, resolved Filled [ok]","byte 41 x65: expected Filled, resolved Filled [ok]","byte 42 x44: expected Filled, resolved Filled [ok]","byte 43 x28: expected Filled, resolved Filled [ok]","byte 44 x26: expected Filled, resolved Filled [ok]","byte 45 x110: expected Filled, resolved Filled [ok]","byte 46 x117: expected Filled, resolved Filled [ok]","byte 47 x51: expected Filled, resolved Filled [ok]"],"blocks_placed":1295,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"18 distinct bytes checked"},{"name":"reach-interior","pass":true,"detail":"arrived in 5.0s"},{"name":"egress","pass":true,"detail":"arrived in 4.0s"},{"name":"multi-occupancy-in","pass":true,"detail":"all 3 arrived by 10.7s"},{"name":"multi-occupancy-out","pass":true,"detail":"all 3 arrived by 4.2s"},{"name":"integrated-reach","pass":true,"detail":"slope 3 across footprint; arrived in 3.1s"},{"name":"integrated-egress","pass":true,"detail":"arrived in 2.6s"}],"pass":true}
```


## VFX PRESETS BUS-NAME VERIFICATION (2026-07-10, tester creative pass — static, no builds)
`asset-lab/vfx/divine_vfx_presets.md` cites engine bus names; verified every one against source:
- ParticleMode (voxygen/src/render/pipelines/particle.rs enum): BlackSmoke=37, CultistFlame=23,
  EnergyHealing=14, Firefly=11, FireworkRed=7, FireworkWhite=8, Water=30 — **7/7 REAL**.
- Outcome (common/src/outcome.rs): Explosion, Lightning, HealthChange — **3/3 REAL**;
  `Outcome::Blessing` is EXPLICITLY proposed-new in the doc (with a HealthChange piggyback
  fallback for zero enum touch) — correctly hedged, not a stale citation.
VERDICT: the pilot's citation discipline held — the engine tier can build on this doc as-is.
(God-hand trio rig/quality grade: see ASSET_QUALITY_AUDIT.md same date — STRONG ×3.)

## NEW PROTOCOL: STAGING MANIFEST (2026-07-10, Ben-approved via architect)
PILOT: from now on, append ONE line per staged batch to readme/ASSET_STAGING_MANIFEST.md
(format + example inside). It replaces the tester's find-newer delta hunting; the static
gate runs against every manifest line. Also FYI two findings from today's anim lint
(bastion-harness/tools/anim_lint.py, runs in the gate):
- 15 WARN: vessel rig anims missing `desc` (cosmetic — hands have them, vessels predate
  the convention; add at leisure).
- 5 INFO: vessel rig dirs are staged TWICE (vox/vehicle_*_rig/ AND vox/vehicles/
  vehicle_*_rig/) — identical today, but two copies with no declared authority WILL
  drift. Please retire one location (log the move in the manifest).
God-hand anims: 0 FAIL — schema vocabulary (semantic *_s keys, *_hz loops, modifier:
true, states poses) is now encoded in the linter; cross-variant identity (blend
contract) asserted and holding.
## RUN 2026-07-10 14:55 UTC · seed 1337 · target `defense_palisade_line_demo` · exe 381ea20049+dirty built 2026-07-10T13:00:05Z

ASSET defense_palisade_line_demo DYNAMIC-ISOLATED: PASS
```json
{"id":"defense_palisade_line_demo","category":"Defense","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 32 x32: expected Filled, resolved Filled [ok]","byte 33 x14: expected Filled, resolved Filled [ok]","byte 34 x155: expected Filled, resolved Filled [ok]","byte 35 x102: expected Filled, resolved Filled [ok]","byte 36 x51: expected Filled, resolved Filled [ok]","byte 37 x16: expected Filled, resolved Filled [ok]","byte 38 x18: expected Filled, resolved Filled [ok]","byte 39 x1: expected Filled, resolved Filled [ok]","byte 40 x1: expected Filled, resolved Filled [ok]","byte 200 x8: expected Sprite, resolved Sprite [ok]","byte 200 x8: expected 8 authored cells (exact), resolved all cells match [ok]"],"blocks_placed":398,"misc_blocks":0,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"11 distinct bytes checked"},{"name":"gate-closed-blocks","pass":true,"detail":"STUCK (watchdog) after 11.6s, best dist 11.4"},{"name":"gate-open-admits","pass":true,"detail":"arrived in 3.2s"},{"name":"gate-open-egress","pass":true,"detail":"arrived in 2.6s"}],"pass":true}
```


## ENGINE-OWES LEDGER (2026-07-11, tester STATE synthesis over manifest+logs+audit+design docs)
All asset content FINISHED + gate 9/9 static-clean; below = ENGINE-CODE debt only. Ranked most-blocked-first:
1. **WORLD-SCALE / SPRITE-MANIFEST RUNG (~40 assets, biggest single blocker)** — figure-scale (11 vox/block) props+sprites PASS load-only; declared dynamic cast (path/interact/climb/work-marker) NEVER RUN. Gameplay-load-bearing first: ladder_rope (climb), harbor_crane (work), bin_wood/sack_grain/altar_stone (interact). Gate: declared-cast-deferred.
2. **GOD-HAND TRIO + DIVINE-VFX (3 hands + 8 presets, biggest anim debt)** — skeleton reg (READ rest_space="parent", FLOOR fractional bones), 18 anim-key bindings (motions don't exist), alignment-blend crossfade code (blocks ALIGN-0/1), HAND-CURSOR entity, aura LightEmitter wiring; VFX: Outcome::Blessing new variant + 7-preset particle/light wiring (particle.rs, clone-recolor tints, NO new system). Gate: NEEDS:animation-code+HAND-CURSOR+alignment-blend-code.
3. **NAVAL VESSELS (5, framework INERT)** — NAVAL-MOVEMENT sim ($$-$$$, whole framework gated on it), collider gate, anim honoring rest_space="absolute" (hull_idle/sail/flag/rudder; oars deferred). Gate: NEEDS:naval-movement.
4. **NIGHT_HORROR (1 creature, 11-part lib) — BUILDER-READY (reviewer FR14 FEASIBLE, NIGHT-HORROR-INTEGRATION-design.md)** — species reg (biped_large::Species::NightHorror=13 APPEND + name/AllSpecies/.ron/FromStr), figure-loader manifest (offsets=Wendigo verbatim, copy 11 parts to assets/voxygen/voxel/npc/night_horror/male/), (Female)→reuse-male rows, per-species skeleton-offset table (~mod.rs:222, missing=COMPILE ERROR), ward-light=flinch. Gate: NEEDS:species-reg+DF-NIGHT. **Most concrete/cheapest complete win.**
5. **WORLD-PROPS DYNAMIC + GLOW EMISSION (7 props + boss_arena)** — dynamic_engine pass NEVER RUN (0 integration-log runs); in-engine glow EMISSION unverified (bonfire 14/15, well 14, boss sconces) — headless gate confirms the byte, not the light. Gate: dynamic_engine PENDING.
6. **TOOLS/ITEMS (13 tiers)** — item-held equip/attach path (FIGURE-layer, deliberately not vox/real). Gate: FIGURE-item-held.
TESTER CAN RUN once the builder wires ANY path: --asset-test dynamic cast (groups 1/5), arena eyeball, in-engine glow-emission verify (group 5). 31 discrete owed items / 4 sources.
## RUN 2026-07-19 05:53 UTC · seed 1337 · target `sprite_orevein_velorite` · exe c1dce2c4a6+dirty built 2026-07-19T05:37:34Z

ASSET sprite_orevein_velorite DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_orevein_velorite","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 15 x5: expected Filled, resolved Filled [ok]","byte 32 x50: expected Filled, resolved Filled [ok]","byte 33 x32: expected Filled, resolved Filled [ok]","byte 34 x294: expected Filled, resolved Filled [ok]","byte 35 x126: expected Filled, resolved Filled [ok]","byte 36 x103: expected Filled, resolved Filled [ok]","byte 37 x33: expected Filled, resolved Filled [ok]","byte 38 x130: expected Filled, resolved Filled [ok]","byte 39 x126: expected Filled, resolved Filled [ok]","byte 40 x41: expected Filled, resolved Filled [ok]","byte 41 x45: expected Filled, resolved Filled [ok]","byte 42 x51: expected Filled, resolved Filled [ok]","byte 43 x53: expected Filled, resolved Filled [ok]","byte 44 x75: expected Filled, resolved Filled [ok]","byte 45 x87: expected Filled, resolved Filled [ok]","byte 46 x6: expected Filled, resolved Filled [ok]","byte 47 x2: expected Filled, resolved Filled [ok]","byte 48 x37: expected Filled, resolved Filled [ok]","byte 49 x4: expected Filled, resolved Filled [ok]","byte 50 x17: expected Filled, resolved Filled [ok]","byte 51 x1: expected Filled, resolved Filled [ok]","byte 52 x18: expected Filled, resolved Filled [ok]","byte 53 x13: expected Filled, resolved Filled [ok]","byte 54 x6: expected Filled, resolved Filled [ok]"],"blocks_placed":1355,"misc_blocks":1231,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"24 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 9.0s"},{"name":"path-back","pass":true,"detail":"arrived in 8.8s"}],"pass":true}
```

## RUN 2026-07-19 05:56 UTC · seed 1337 · target `sprite_orevein_velorite` · exe c1dce2c4a6+dirty built 2026-07-19T05:37:34Z

ASSET sprite_orevein_velorite DYNAMIC-ISOLATED: PASS
```json
{"id":"sprite_orevein_velorite","category":"Flora","mode":"isolated-dynamic","fidelity_ok":true,"marker_checks":["byte 15 x5: expected Filled, resolved Filled [ok]","byte 32 x50: expected Filled, resolved Filled [ok]","byte 33 x32: expected Filled, resolved Filled [ok]","byte 34 x294: expected Filled, resolved Filled [ok]","byte 35 x126: expected Filled, resolved Filled [ok]","byte 36 x103: expected Filled, resolved Filled [ok]","byte 37 x33: expected Filled, resolved Filled [ok]","byte 38 x130: expected Filled, resolved Filled [ok]","byte 39 x126: expected Filled, resolved Filled [ok]","byte 40 x41: expected Filled, resolved Filled [ok]","byte 41 x45: expected Filled, resolved Filled [ok]","byte 42 x51: expected Filled, resolved Filled [ok]","byte 43 x53: expected Filled, resolved Filled [ok]","byte 44 x75: expected Filled, resolved Filled [ok]","byte 45 x87: expected Filled, resolved Filled [ok]","byte 46 x6: expected Filled, resolved Filled [ok]","byte 47 x2: expected Filled, resolved Filled [ok]","byte 48 x37: expected Filled, resolved Filled [ok]","byte 49 x4: expected Filled, resolved Filled [ok]","byte 50 x17: expected Filled, resolved Filled [ok]","byte 51 x1: expected Filled, resolved Filled [ok]","byte 52 x18: expected Filled, resolved Filled [ok]","byte 53 x13: expected Filled, resolved Filled [ok]","byte 54 x6: expected Filled, resolved Filled [ok]"],"blocks_placed":1355,"misc_blocks":1231,"sprite_cfgs_dropped":0,"entity_spawners_skipped":0,"assertions":[{"name":"marker-fidelity","pass":true,"detail":"24 distinct bytes checked"},{"name":"path-around","pass":true,"detail":"arrived in 9.0s"},{"name":"path-back","pass":true,"detail":"arrived in 8.8s"}],"pass":true}
```

