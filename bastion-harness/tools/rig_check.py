"""bastion (B-ASSET1, quality gate): vessel/creature RIG verification.

For each `<id>_rig/` directory (multi-part animation-ready rigs: hull/body +
separable sail/oar/flag/rudder parts + rig.json):
1. PER-PART INTEGRITY — every part vox must be ONE 6-connected component
   (fragmented parts shear apart under bone transforms). Ground contact is
   N/A for parts (they mount on bones).
2. SPLIT-HOLE CHECK — the union of parts placed at their rig.json offsets is
   diffed against the assembled `<id>.vox`: cells present in the assembly but
   missing from the union are HOLES exposed by the split; union cells absent
   from the assembly are OVERLAPS/misalignments.

Run from the repo root (reads asset-lab READ-ONLY, prints per rig):
    python bastion-harness/tools/rig_check.py [rig_dir ...]
Defaults to every `asset-lab/vox/*_rig` directory.
"""
import sys, os, glob, json
sys.path.insert(0, 'asset-lab/gen')
sys.stdout.reconfigure(encoding='utf-8')
from voxlib import read_vox

def components(vox):
    seen, comps = set(), 0
    for start in vox:
        if start in seen:
            continue
        comps += 1
        stack = [start]
        seen.add(start)
        while stack:
            x, y, z = stack.pop()
            for n in ((x+1,y,z),(x-1,y,z),(x,y+1,z),(x,y-1,z),(x,y,z+1),(x,y,z-1)):
                if n in vox and n not in seen:
                    seen.add(n)
                    stack.append(n)
    return comps

def check_rig(rig_dir):
    rig_id = os.path.basename(rig_dir.rstrip('/\\')).removesuffix('_rig')
    rig_path = os.path.join(rig_dir, 'rig.json')
    if not os.path.isfile(rig_path):
        print(f'{rig_id}: no rig.json — SKIP')
        return
    rig = json.load(open(rig_path, encoding='utf-8'))
    union = {}
    ok = True
    for bone, spec in sorted(rig.items()):
        if not isinstance(spec, dict) or 'part' not in spec:
            continue
        part_path = os.path.join(rig_dir, spec['part'] + '.vox')
        try:
            d = read_vox(part_path)
        except Exception as e:
            print(f'{rig_id}/{spec["part"]}: UNREADABLE {e}')
            ok = False
            continue
        _, _, _, vox = d['models'][0]
        n = components(vox)
        if n != 1:
            print(f'{rig_id}/{spec["part"]}: FRAGMENTED — {n} components (must be 1)')
            ok = False
        off = spec.get('offset', [0, 0, 0])
        for (x, y, z), b in vox.items():
            union[(x + off[0], y + off[1], z + off[2])] = b
    # Assembled diff (holes / overlaps), if the assembled vox exists.
    assembled_path = os.path.join(os.path.dirname(rig_dir.rstrip('/\\')), rig_id + '.vox')
    if not os.path.isfile(assembled_path):
        # vehicles live in vox/vehicles/, rigs in vox/<id>_rig — try there too.
        alt = os.path.join(os.path.dirname(rig_dir.rstrip('/\\')), 'vehicles', rig_id + '.vox')
        assembled_path = alt if os.path.isfile(alt) else None
    if assembled_path:
        d = read_vox(assembled_path)
        _, _, _, avox = d['models'][0]
        holes = [p for p in avox if p not in union]
        extra = [p for p in union if p not in avox]
        if holes:
            zs = sorted({z for _, _, z in holes})
            print(f'{rig_id}: {len(holes)} HOLES vs assembly (z levels {zs[:6]})')
            ok = False
        if extra:
            print(f'{rig_id}: {len(extra)} union cells NOT in assembly (offset drift?)')
            ok = False
    else:
        print(f'{rig_id}: no assembled vox found — split-hole check skipped')
    print(f'{rig_id}: {"RIG OK" if ok else "RIG FAIL (see above)"} '
          f'({len(union)} union cells)')

if __name__ == '__main__':
    targets = sys.argv[1:] or sorted(glob.glob('asset-lab/vox/*_rig'))
    print(f'checking {len(targets)} rigs...')
    for t in targets:
        check_rig(t)
    print('RIG SCAN COMPLETE')
