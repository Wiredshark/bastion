"""bastion (B-ASSET1, quality gate): STATIC catalog re-verification battery.

Replicates the load-time contract checks of server/src/bastion_assets.rs
WITHOUT an engine build (machine-light re-verification):
  1. vox exists + readable; catalog `dims` == vox model size
  2. catalog `blocks` == actual filled-cell count
  3. MARKER FIDELITY (the load-time assertion, statically): for every marker
     byte, the vox's cells of that byte == EXACTLY the authored cell list
     (missing cells AND undeclared extras both fail)
  4. marker-band coverage: any vox byte in 200..=224 absent from `markers`
     = undeclared marker (would silently hit the registry default on load)
  5. sidecar liveness: referenced .ron exists, has custom_indices, and every
     sidecar byte actually occurs in the vox (dead mappings = drift warning)

Coordinates are raw vox cells (voxlib) — the same convention the catalog was
authored in and that in-engine census fidelity passed against (73/73).

Run from the repo root (reads asset-lab READ-ONLY):
    python bastion-harness/tools/catalog_recheck.py [asset_id ...]
"""
import sys, os, json, re
sys.path.insert(0, 'asset-lab/gen')
sys.stdout.reconfigure(encoding='utf-8')
from voxlib import read_vox

MARKER_BAND = range(200, 225)

def recheck(aid, spec):
    problems, warns = [], []
    vp = spec['vox']
    if not os.path.isfile(vp):
        return [f'vox MISSING: {vp}'], warns
    d = read_vox(vp)
    sx, sy, sz, vox = d['models'][0]
    if list(spec.get('dims', [])) != [sx, sy, sz]:
        problems.append(f'dims drift: catalog {spec.get("dims")} vs vox {[sx, sy, sz]}')
    nblocks = len(vox)
    if spec.get('blocks') is not None and spec['blocks'] != nblocks:
        problems.append(f'blocks drift: catalog {spec["blocks"]} vs vox {nblocks}')
    # marker fidelity, exact-cell, both directions
    declared = {int(b): {tuple(c) for c in cells}
                for b, cells in spec.get('markers', {}).items()}
    actual = {}
    for cell, b in vox.items():
        if b in MARKER_BAND or b in declared:
            actual.setdefault(b, set()).add(cell)
    for b, want in declared.items():
        got = actual.get(b, set())
        if got != want:
            problems.append(
                f'marker byte {b}: authored {len(want)} cells, vox has {len(got)}'
                f' (missing {len(want - got)}, undeclared {len(got - want)})')
    for b in actual:
        if b in MARKER_BAND and b not in declared:
            problems.append(f'UNDECLARED marker byte {b} in vox ({len(actual[b])} cells)')
    # sidecar liveness
    rp = spec.get('ron')
    if rp:
        if not os.path.isfile(rp):
            problems.append(f'ron MISSING: {rp}')
        else:
            txt = open(rp, encoding='utf-8').read()
            if 'custom_indices' not in txt:
                problems.append('ron has no custom_indices')
            present = {b for b in vox.values()}
            for m in re.finditer(r'(\d+)\s*:', txt):
                b = int(m.group(1))
                if b not in present:
                    warns.append(f'sidecar maps byte {b} not present in vox (dead mapping)')
    return problems, warns

if __name__ == '__main__':
    cat = json.load(open('asset-lab/vox/real/catalog.json', encoding='utf-8'))
    assets = cat['assets']
    only = set(sys.argv[1:])
    npass = nfail = nwarn = 0
    for aid, spec in sorted(assets.items()):
        if only and aid not in only:
            continue
        problems, warns = recheck(aid, spec)
        for w in warns:
            print(f'{aid}: WARN {w}')
        if problems:
            nfail += 1
            for p in problems:
                print(f'{aid}: FAIL {p}')
        else:
            npass += 1
        nwarn += len(warns)
    print(f'CATALOG RECHECK: {npass} PASS, {nfail} FAIL, {nwarn} warnings')
