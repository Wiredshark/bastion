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

