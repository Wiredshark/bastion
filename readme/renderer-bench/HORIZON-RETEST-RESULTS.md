# Item-19 horizon RETEST — 2026-08-21, GPU session (Ben-authorized)

**Verdict: the 16→24 far-terrain band DELIVERS. The July "no usable
farther horizon" verdict is refuted on the fixed fixture — the defect was
the fixture, exactly as item 19 diagnosed.**

## Setup
- Fork @ 2fd9b7bdb0 + r1f absent≠invalid fix; voxygen 33ddaa766bb8fd30.
- Two isolated arms, `--bastion-flat-arena` (slab radius 26 — the item-19
  fix, so the slab covers everything the camera can see),
  `VELOREN_CLIENT_TYPE=silent_spectator` (auto-enters a spectating
  session — the July legs' "dark/UI-only views" trap solved), horizon
  fixture `flat-arena-oblique-horizon-v1`, streaming measurement
  `continuous-server-v1`, r0p observer durable. RTX 5070, Vulkan.
- 300s per arm; settled tail (last 500 frames) aggregated.

## The numbers (settled tail, stable min=max throughout)
| | FAR (far-band-v1) | REF (stock) |
|---|---|---|
| view distance (chunks) | 24 | 10 |
| resident chunks | 1,993 | 431 |
| meshed chunks | 1,801 | 347 |
| **visible chunks** | **516** | 110 |
| census near 0–8 | 95 | 92 |
| census reference 9–16 | 232 | 18 |
| **census far 17–24** | **189** | **0** |
| beyond 24 | 0 | 0 |
| max visible radius (chunks) | **23** | 10 |
| max visible distance (blocks) | **736** | 320 |
| LOD distance (blocks) | 675 | 200 |
| draw calls | 2,615 | 785 |

189 chunks of real, rendered, VISIBLE terrain in the 17–24 band; the
visible horizon reaches 23 chunks / 736 blocks. The far band is not a
residency claim — it is on screen.

## Honest caveats
- `visible_horizon_camera_valid` stayed false: the spectator camera never
  entered the fixture's certified Overseer pose, so the canonical camera
  TOKEN is unattested this run. The census does not depend on the token.
  Completing the certified-pose path is instrument polish, owed to W5-
  class work, not to this verdict.
- No PNG captures landed (the r0d capture path waits on freeze-tick
  semantics this run didn't arm). The live windows were the eyeball.
- Cost of the band (same scene): draws 785→2,615, resident 431→1,993 —
  the R0P observatory now has both arms' full frame-time distributions
  on disk (`smoke/gpu/r0p-{FAR,REF}.json/`) for the architecture memo.

## Defects found and fixed by this session (each cost a leg)
1. **loader**: first-ever local voxygen run hit STATUS_ENTRYPOINT_NOT_FOUND
   — shaderc built by `C:\Users\q\toolchains\mingw64` g++, but the loader
   resolved Git's older `libstdc++-6.dll`; matching MinGW runtimes now sit
   beside the exe (target/debug is untracked; note for future builds).
2. **r1f absent≠invalid** (ported-code defect): with the flat arena on and
   NO weather-fixture env, the declaration parsed Invalid and KILLED the
   embedded server two minutes in; absent now parses Disabled, and the
   Invalid arm fails loud-not-fatal (same repair as the streaming arm).
3. **menu-stuck legs**: without `silent_spectator` the client sits at
   character select forever while the server runs the arena — plausibly
   the same trap behind July's dark/UI-only evidence PNGs.
4. Runner: subshell-pid taskkill cannot work (documented Git Bash trap) —
   identity-scoped kill BEFORE wait; r0p output is a DIRECTORY.
