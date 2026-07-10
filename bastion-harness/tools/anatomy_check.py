"""bastion (B-ASSET1, quality gate): SEMANTIC-PLACEMENT / ANATOMY validator.

Ben's mandate: per-asset-class part-position rules that flag anatomically
wrong geometry — found live: GIANT RUDDERS mounted on ship BOWS (galleon +
skiff authored rudders at the tapered end; the cog's is correct), and marker
cells floating in mid-air (the connectivity sweep EXEMPTS marker-band cells,
which hid exactly this).

RULES (derived from geometry, never from an authoring coordinate convention —
the rudder bug happened BECAUSE hulls point different ways along y):

SHIP rigs (kind=ship, composed bone transforms like rig_check):
  s1 rudder-at-stern: stern = the BLUNTER hull y-end (wider cross-section);
     rudder centroid within 20% of hull length from it — and never beyond
     the bow tip. FAIL.
  s2 centerline: rudder/sail/mast/sweep centroid |x - hull cx| <= 1.5. FAIL
     (pennants/flags exempt — jack staffs sit off-center legitimately).
  s3 rudder hangs LOW: rudder z-centroid <= hull z-centroid. WARN.
  s4 size sanity: rudder cells <= 5% of hull cells ("giant rudder"). FAIL.
  s5 oar symmetry: oar_l/oar_r mirror about the centerline within 1. FAIL.

MARKERS (every catalog entry + component vox):
  m1 support: marker-band (200..224) cells group into 26-connected CLUSTERS
     per byte (a gate leaf / portcullis bar-grid is ONE cluster even where
     bars touch only diagonally); a cluster is supported if ANY of its cells
     is face-adjacent to a NON-marker solid cell or sits at z=0. An
     unsupported cluster = a marker floating in mid-air (GlowingRock etc.
     reads as a levitating prop in-game). FAIL — unless the file is in
     ALLOWLIST (pilot-confirmed intent), then WARN.
  m2 coherence: cells of one marker byte should cluster (workstations are
     1-8 cells in a small region). Spread > 2/3 of the asset's own diagonal
     AND count > 4 = WARN (braziers legitimately ring a hall; a gate spans
     its aperture — hence WARN, human judges intent).

Run from the repo root (reads asset-lab READ-ONLY):
    python bastion-harness/tools/anatomy_check.py
Exits nonzero on any FAIL. Extend per class as Ben flags new anatomy.
"""
import sys, os, glob, json, math
sys.path.insert(0, 'asset-lab/gen')
sys.stdout.reconfigure(encoding='utf-8')
from voxlib import read_vox

MARKER_BAND = range(200, 225)
fails = warns = 0

def fail(msg):
    global fails
    fails += 1
    print(f'FAIL {msg}')

def warn(msg):
    global warns
    warns += 1
    print(f'WARN {msg}')

def world_rest(bones, name):
    p = [0.0, 0.0, 0.0]
    b = bones.get(name)
    while b:
        p = [a + r for a, r in zip(p, b.get('rest', [0, 0, 0]))]
        b = bones.get(b['parent']) if b.get('parent') else None
    return p

def centroid(cells):
    n = len(cells)
    return tuple(sum(c[i] for c in cells) / n for i in range(3))

def check_ship(rig_dir):
    rid = os.path.basename(rig_dir.rstrip('/\\')).removesuffix('_rig')
    rig = json.load(open(os.path.join(rig_dir, 'rig.json'), encoding='utf-8'))
    if rig.get('kind') != 'ship':
        return
    skel = rig.get('skel', {})
    bones = {b['name']: b for b in skel.get('bones', [])}
    offs = skel.get('offsets', {})
    placed = {}
    for name in bones:
        p = os.path.join(rig_dir, name + '.vox')
        if not os.path.isfile(p):
            continue
        w = world_rest(bones, name)
        off = offs.get(name, [0, 0, 0])
        _, _, _, vox = read_vox(p)['models'][0]
        placed[name] = [
            (math.floor(x + w[0] + off[0]), math.floor(y + w[1] + off[1]),
             math.floor(z + w[2] + off[2])) for (x, y, z) in vox]
    hull = placed.get('hull')
    if not hull:
        return
    hy = [c[1] for c in hull]
    y0, y1 = min(hy), max(hy)
    span = max(y1 - y0, 1)
    hull_c = centroid(hull)

    def width_at(yq):
        xs = [c[0] for c in hull if abs(c[1] - yq) <= max(1, span // 10)]
        return (max(xs) - min(xs) + 1) if xs else 0

    w0, w1 = width_at(y0), width_at(y1)
    # bow = tapered end; equal widths (double-ender barge/rowboat) = no
    # bow/stern verdict, skip s1 for those.
    bow_y, stern_y = ((y1, y0) if w1 < w0 else (y0, y1)) if w0 != w1 else (None, None)

    for name, cells in placed.items():
        if name == 'hull':
            continue
        c = centroid(cells)
        if 'rudder' in name or 'sweep' in name:
            if stern_y is not None:
                if abs(c[1] - stern_y) > 0.2 * span:
                    fail(f'{rid}.{name}: s1 rudder at the WRONG END — centroid y={c[1]:.0f}, '
                         f'stern(blunt end)={stern_y}, bow(tapered)={bow_y} (hull y {y0}..{y1})')
                if bow_y is not None and (
                        (bow_y == y1 and c[1] > y1) or (bow_y == y0 and c[1] < y0)):
                    fail(f'{rid}.{name}: s1 rudder hangs BEYOND the bow tip (y={c[1]:.0f})')
            if len(cells) > 0.05 * len(hull):
                fail(f'{rid}.{name}: s4 giant rudder — {len(cells)} cells vs hull {len(hull)} (>5%)')
            if c[2] > hull_c[2]:
                warn(f'{rid}.{name}: s3 rudder rides high (z {c[2]:.1f} > hull z {hull_c[2]:.1f})')
        if any(k in name for k in ('rudder', 'sweep', 'sail', 'mast')):
            if abs(c[0] - hull_c[0]) > 1.5:
                fail(f'{rid}.{name}: s2 off-centerline by {abs(c[0]-hull_c[0]):.1f}')
    if 'oar_l' in placed and 'oar_r' in placed:
        cl, cr = centroid(placed['oar_l']), centroid(placed['oar_r'])
        if abs((cl[0] - hull_c[0]) + (cr[0] - hull_c[0])) > 1 or abs(cl[1] - cr[1]) > 1:
            fail(f'{rid}: s5 oars not mirrored (l {cl}, r {cr}, hull cx {hull_c[0]:.1f})')

# pilot-confirmed floating-marker intent (m1 demoted to WARN):
ALLOWLIST = {
    'mine_breach_maw.vox': 'floating glow motes = breach magic ambience',
    'terracotta_set_demo.vox': 'demo set, pilot by-design allowlist',
}

def clusters26(cells):
    """26-connected components (diagonal contact groups a bar-grid)."""
    cellset, seen, out = set(cells), set(), []
    for start in cells:
        if start in seen:
            continue
        comp, stack = [], [start]
        seen.add(start)
        while stack:
            x, y, z = stack.pop()
            comp.append((x, y, z))
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    for dz in (-1, 0, 1):
                        n = (x+dx, y+dy, z+dz)
                        if n in cellset and n not in seen:
                            seen.add(n)
                            stack.append(n)
        out.append(comp)
    return out

def check_markers(vox_path):
    name = os.path.basename(vox_path)
    _, _, _, vox = read_vox(vox_path)['models'][0]
    by_byte = {}
    for cell, b in vox.items():
        if b in MARKER_BAND:
            by_byte.setdefault(b, []).append(cell)
    if not by_byte:
        return
    xs = [c[0] for c in vox]; ys = [c[1] for c in vox]; zs = [c[2] for c in vox]
    diag = math.dist((min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs)))
    for b, cells in by_byte.items():
        for comp in clusters26(cells):
            supported = any(
                z == 0 or any(
                    (x+dx, y+dy, z+dz) in vox
                    and vox[(x+dx, y+dy, z+dz)] not in MARKER_BAND
                    for dx, dy, dz in ((1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)))
                for (x, y, z) in comp)
            if not supported:
                cx, cy, cz = (round(v) for v in centroid(comp))
                msg = (f'{name}: m1 marker byte {b} cluster ({len(comp)} cells @ '
                       f'~({cx},{cy},{cz})) FLOATS — no cell touches non-marker solid '
                       f'or ground (levitating prop in-game)')
                if name in ALLOWLIST:
                    warn(msg + f' [allowlisted: {ALLOWLIST[name]}]')
                else:
                    fail(msg)
        if len(cells) > 4:
            spread = max(math.dist(a, c) for a in cells for c in cells)
            if spread > diag * 2 / 3:
                warn(f'{name}: m2 marker byte {b} scattered — {len(cells)} cells spread '
                     f'{spread:.0f} vs asset diagonal {diag:.0f} (confirm intent)')

if __name__ == '__main__':
    args = sys.argv[1:]
    rig_dirs = [a for a in args if os.path.isdir(a)] or sorted(
        glob.glob('asset-lab/vox/*_rig'))
    vox_files = [a for a in args if a.endswith('.vox')] or (
        sorted(glob.glob('asset-lab/vox/real/*.vox'))
        + sorted(glob.glob('asset-lab/vox/components/*.vox')))
    for rd in rig_dirs:
        check_ship(rd)
    for vf in vox_files:
        check_markers(vf)
    print(f'ANATOMY CHECK: {fails} FAIL, {warns} WARN '
          f'({len(rig_dirs)} rigs, {len(vox_files)} vox)')
    sys.exit(1 if fails else 0)
