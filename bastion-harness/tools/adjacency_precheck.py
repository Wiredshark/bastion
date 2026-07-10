"""bastion (B-ASSET1, quality gate): STATIC pre-check for mistake class #23
(sprite adjacency violations → silent one-tick vacate in-engine).

The engine validates sprite attachment on every terrain change
(`common/state/src/state.rs` "Indirectly modified sprites") against
`common/src/terrain/sprite/mod.rs::adjecency_requirement` (~line 924). This
tool mirrors that table CONTENT-SIDE: for every cataloged asset, each marker
cell whose custom_indices target is a requirement-carrying sprite is checked
against the VOX geometry (a cell at z==0 counts as terrain-supported below).
Violations = the sprite will be silently removed one tick after runtime
placement — catch them at staging, not in the arena.

Run from the repo root (reads asset-lab READ-ONLY):
    python bastion-harness/tools/adjacency_precheck.py
Prototype scope: the sprites actually in use by the marker registry/sidecars.
Extend REQUIREMENTS as the pilot adopts more attachment-carrying sprites.
"""
import sys, os, glob, json, re
sys.path.insert(0, 'asset-lab/gen')
sys.stdout.reconfigure(encoding='utf-8')
from voxlib import read_vox

# sprite name (as written in RON sidecars) -> list of required-solid offsets,
# mode ('all' | 'any'). Mirrors adjecency_requirement — extend with the table.
REQUIREMENTS = {
    'Lantern': ([(0, 0, -1)], 'all'),            # STANDING lantern: solid below
    'LanternpostWoodLantern': ([(-1, 0, 0)], 'all'),  # wall-mounted (unrotated)
    'Beehive': ([(0, 0, 1)], 'all'),             # hangs: solid above
    'CeilingMushroom': ([(0, 0, 1)], 'all'),
    'CeilingLanternPlant': ([(0, 0, 1)], 'all'),
    'CeilingLanternFlower': ([(0, 0, 1)], 'all'),
    'Apple': ([(0, 0, 1), (0, 0, -1)], 'any'),
    'Coconut': ([(0, 0, 1), (0, 0, -1)], 'any'),
}

def sidecar_sprites(ron_path):
    """byte -> sprite name for Sprite(...) entries in a custom_indices RON."""
    if not os.path.isfile(ron_path):
        return {}
    txt = open(ron_path, encoding='utf-8').read()
    return {
        int(m.group(1)): m.group(2)
        for m in re.finditer(r'(\d+)\s*:\s*Sprite\((\w+)', txt)
    }

def check(vox_path):
    ron = sidecar_sprites(os.path.splitext(vox_path)[0] + '.ron')
    wanted = {b: s for b, s in ron.items() if s in REQUIREMENTS}
    if not wanted:
        return 0
    d = read_vox(vox_path)
    _, _, _, vox = d['models'][0]
    filled = set(vox)
    bad = 0
    for (x, y, z), b in vox.items():
        byte = b  # voxlib returns the raw color index; sidecar bytes match
        if byte not in wanted:
            continue
        offsets, mode = REQUIREMENTS[wanted[byte]]
        def solid(o):
            p = (x + o[0], y + o[1], z + o[2])
            # z==0 below = rests on terrain at placement (ground anchor).
            if o == (0, 0, -1) and z == 0:
                return True
            return p in filled
        oks = [solid(o) for o in offsets]
        valid = all(oks) if mode == 'all' else any(oks)
        if not valid:
            print(
                f'{os.path.basename(vox_path)}: byte {byte} -> '
                f'{wanted[byte]} at ({x},{y},{z}) VIOLATES adjacency '
                f'({mode} of {offsets}) — will vacate one tick after placement'
            )
            bad += 1
    return bad

if __name__ == '__main__':
    targets = sorted(glob.glob('asset-lab/vox/real/*.vox'))
    total_bad = 0
    for t in targets:
        total_bad += check(t)
    print(f'PRECHECK COMPLETE: {len(targets)} assets, {total_bad} violations')
    sys.exit(1 if total_bad else 0)
