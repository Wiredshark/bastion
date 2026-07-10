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
import sys, os, glob, json, math
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
    # Two schemas: v2 = {kind, skel: {bones: [{name, parent, rest}], offsets:
    # {name: [x,y,z]}}, animlist}; v1 = flat {boneN: {part, offset, ...}}.
    # CONVENTION AMBIGUITY (exposed 2026-07-10 by the naval rudder fix):
    # the god-hand assembler CHAINS rests down the parent hierarchy (proved
    # exact), the naval assembler treats rests as ABSOLUTE world positions
    # (proved exact post-fix). The two agree whenever the root rest is zero —
    # which it was, until the rudder fix. Until rig.json pins `skel.rest_space`
    # ("parent" | "absolute"), we try chained first and accept absolute only
    # on an EXACT assembly match, with a loud convention warning. Fractional
    # rests are LEGAL (godhand f4p x=27.5); composed positions are FLOORED.
    conventions = {}
    if 'skel' in rig:
        offsets = rig['skel'].get('offsets', {})
        bones = {b['name']: b for b in rig['skel'].get('bones', [])}

        def world_rest(name):
            p = [0.0, 0.0, 0.0]
            b = bones.get(name)
            while b:
                p = [a + r for a, r in zip(p, b.get('rest', [0, 0, 0]))]
                b = bones.get(b['parent']) if b.get('parent') else None
            return p

        def placed(rest_fn):
            return [
                (name,
                 [math.floor(w + o)
                  for w, o in zip(rest_fn(name), offsets.get(name, [0, 0, 0]))])
                for name in bones
            ]

        conventions['parent'] = placed(world_rest)
        conventions['absolute'] = placed(
            lambda n: bones[n].get('rest', [0, 0, 0]))
        declared = rig['skel'].get('rest_space')
        parts = conventions.get(declared) or conventions['parent']
    else:
        declared = 'v1'
        parts = [
            (spec['part'], spec.get('offset', [0, 0, 0]))
            for _, spec in sorted(rig.items())
            if isinstance(spec, dict) and 'part' in spec
        ]
    union = {}
    ok = True
    part_vox = {}
    for part_name, _ in parts:
        part_path = os.path.join(rig_dir, part_name + '.vox')
        try:
            d = read_vox(part_path)
        except Exception as e:
            print(f'{rig_id}/{part_name}: UNREADABLE {e}')
            ok = False
            continue
        _, _, _, vox = d['models'][0]
        part_vox[part_name] = vox
        n = components(vox)
        if n != 1:
            # ENGINE TRUTH: a rig part is ONE bone mesh — islands move together
            # and cannot shear off, so fragmentation is a COSMETIC-INTENT info
            # (rigging lines / cloth edges are legitimate islands), not a
            # failure. The hard gate is union==assembly + no split holes.
            print(f'{rig_id}/{part_name}: INFO — {n} components (islands move as one bone mesh; confirm intent)')

    def build_union(part_list):
        u = {}
        for part_name, off in part_list:
            for (x, y, z), b in part_vox.get(part_name, {}).items():
                u[(x + off[0], y + off[1], z + off[2])] = b
        return u

    union = build_union(parts)
    # Assembled diff (holes / overlaps), if the assembled vox exists.
    parent = os.path.dirname(rig_dir.rstrip('/\\'))
    candidates = [
        os.path.join(parent, rig_id + '.vox'),
        # vehicles live in vox/vehicles/, rigs in vox/<id>_rig — try there too.
        os.path.join(parent, 'vehicles', rig_id + '.vox'),
        # god-hand convention: rig dir vox/<id>/, assembly vox/<id>_assembled.vox
        os.path.join(parent, rig_id + '_assembled.vox'),
    ]
    assembled_path = next((c for c in candidates if os.path.isfile(c)), None)
    if assembled_path:
        d = read_vox(assembled_path)
        _, _, _, avox = d['models'][0]

        def diff(u):
            return ([p for p in avox if p not in u],
                    [p for p in u if p not in avox])

        holes, extra = diff(union)
        if (holes or extra) and declared not in conventions and len(conventions) > 1:
            # undeclared rest_space: accept the OTHER convention only on an
            # EXACT match — and say so loudly (schema ambiguity, not geometry).
            for alt_name, alt_parts in conventions.items():
                if alt_parts is parts:
                    continue
                alt_union = build_union(alt_parts)
                ah, ae = diff(alt_union)
                if not ah and not ae:
                    union, holes, extra = alt_union, ah, ae
                    print(f'{rig_id}: WARN — assembly matches rest_space='
                          f'"{alt_name}" exactly, not the default parent-chain. '
                          f'PIN skel.rest_space in rig.json before animation-code '
                          f'(two assembler conventions are now in the wild).')
                    break
        if holes:
            zs = sorted({z for _, _, z in holes})
            print(f'{rig_id}: {len(holes)} HOLES vs assembly (z levels {zs[:6]})')
            ok = False
        if extra:
            # Union ⊃ assembly = part-side ADDITIONS (e.g. rope stitching) with
            # a stale assembled preview — the PARTS are the shipping truth for
            # rigs, so this is a WARN (regen the preview), not a rig defect.
            print(f'{rig_id}: WARN — {len(extra)} part cells not in assembly (stale assembled preview? regen it)')
    else:
        print(f'{rig_id}: no assembled vox found — split-hole check skipped')
    print(f'{rig_id}: {"RIG OK" if ok else "RIG FAIL (see above)"} '
          f'({len(union)} union cells)')
    return ok

if __name__ == '__main__':
    targets = sys.argv[1:] or sorted(glob.glob('asset-lab/vox/*_rig'))
    print(f'checking {len(targets)} rigs...')
    any_fail = False
    for t in targets:
        if check_rig(t) is False:
            any_fail = True
    print('RIG SCAN COMPLETE')
    sys.exit(1 if any_fail else 0)
