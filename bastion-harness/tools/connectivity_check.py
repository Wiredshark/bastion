"""bastion (B-ASSET1, quality gate): geometry-connection audit.

Floods 6-connected voxel components per .vox; any component that never reaches
the ground plane (z == 0) is FLOATING — catches gapped roofs, unsupported
cantilevers, and part-to-part disconnections from EVERY angle at once (the
defect class single-angle iso renders miss — ASSET_COMMON_MISTAKES #1/#2, the
floating-roof Ben caught). Marker-band (>=200) cells are classified separately
(they may legitimately float, e.g. the pier mooring point).

Run from the repo root (reads asset-lab READ-ONLY, prints offenders only):
    python bastion-harness/tools/connectivity_check.py
Pair with turntable renders (gen/render.py default + --flip, plus rotated
copies written to a scratch dir) for the visual confirmation of anything
flagged SUSPECT.
"""
import sys, glob, os
sys.path.insert(0, 'asset-lab/gen')
sys.stdout.reconfigure(encoding='utf-8')
from voxlib import read_vox

def components(vox):
    seen = set()
    comps = []
    for start in vox:
        if start in seen:
            continue
        stack = [start]
        seen.add(start)
        cells = []
        while stack:
            c = stack.pop()
            cells.append(c)
            x, y, z = c
            for n in ((x+1,y,z),(x-1,y,z),(x,y+1,z),(x,y-1,z),(x,y,z+1),(x,y,z-1)):
                if n in vox and n not in seen:
                    seen.add(n)
                    stack.append(n)
        comps.append(cells)
    return comps

def audit(path):
    try:
        d = read_vox(path)
    except Exception as e:
        print(f'{os.path.basename(path)}: UNREADABLE {e}')
        return
    if not d['models']:
        return
    sx, sy, sz, vox = d['models'][0]
    if not vox:
        return
    comps = components(vox)
    if len(comps) == 1:
        return
    grounded = [c for c in comps if min(z for _, _, z in c) == 0]
    floats = [c for c in comps if min(z for _, _, z in c) > 0]
    if not floats:
        return  # multiple grounded components (e.g. fence posts) — fine
    name = os.path.basename(path)
    notes = []
    for c in sorted(floats, key=len, reverse=True):
        zmin = min(z for _, _, z in c)
        zmax = max(z for _, _, z in c)
        bytes_in = {vox[cell] for cell in c}
        marker = all(b >= 200 for b in bytes_in)
        kind = 'MARKER-CELL' if marker else ('SUSPECT' if len(c) >= 8 else 'small')
        notes.append(f'{kind} size={len(c)} z={zmin}..{zmax} bytes={sorted(bytes_in)[:4]}')
    print(
        f'{name}: {len(comps)} comps ({len(grounded)} grounded, '
        f'{len(floats)} floating) -> ' + ' | '.join(notes[:6])
    )

if __name__ == '__main__':
    # File args override the default catalog+components sweep (the gate
    # passes explicit globs; unstaged batches are auditable pre-staging).
    targets = sys.argv[1:] or (
        sorted(glob.glob('asset-lab/vox/real/*.vox'))
        + sorted(glob.glob('asset-lab/vox/components/*.vox')))
    print(f'scanning {len(targets)} files (only offenders print)...')
    for t in targets:
        audit(t)
    print('SCAN COMPLETE')
