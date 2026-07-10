"""bastion (B-ASSET1, quality gate): COMPARE-TO-REFERENCE check.

Ben's ask: beyond connectivity/markers, does the asset actually READ as the
thing? Flags proportion/silhouette divergence from a declared reference
(usually the native Veloren model the piece must evoke).

Reference mapping: the in-file REFERENCE_MAP (glob -> reference vox) is the
default; `asset-lab/reference_index.json` (the pilot's reference-index,
same shape: {"glob": "ref path"}) OVERRIDES/extends it when present — the
pilot owns which reference a piece answers to.

Metrics per (asset, reference), both canonicalized by PCA principal axes —
NOT bbox extents: the pilot authors held items DIAGONALLY (a 45-degree haft
inflates the bbox square), which fooled the bbox version of this tool while
the axis-aligned native stayed elongated. PCA is rotation-invariant.
  r1 PROPORTION: sqrt-eigenvalue ratios (mid/long, short/long) of the cell
     covariance. Distance = sum |deltas|; > PROP_WARN warns, > PROP_FAIL fails.
  r2 SILHOUETTE: cells projected onto principal-axis planes, rasterized to
     24x24; per-view IoU maximized over flips, averaged over the 3 views.
     < SIL_WARN warns, < SIL_FAIL fails ("does not read as the thing").
  r3 MASS: cell-count ratio vs reference outside [0.4x, 3x] warns
     (complexity floor, mistake class #15 proxy).

Thresholds calibrated 2026-07-10 against the tool-tier family: healthy
tiers pass, the broken 30-cell bare-stick irons FAIL, and cross-kind
controls (pick judged against the shovel reference) score clearly worse —
see the calibration block in ASSET_QUALITY_AUDIT.md before retuning.

Run from the repo root (reads asset-lab + assets/ READ-ONLY):
    python bastion-harness/tools/compare_reference.py
Exits nonzero on any FAIL; assets with no declared reference are skipped.
"""
import sys, os, glob, json, fnmatch, math
sys.path.insert(0, 'asset-lab/gen')
sys.stdout.reconfigure(encoding='utf-8')
from voxlib import read_vox

# NATIVE references are ADVISORY (WARN-only): the native model is a style
# benchmark and complexity FLOOR — the pilot's richer version SHOULD diverge
# upward (native shovel_blue is itself a 51-cell near-stick). The hard gate
# is FAMILY self-consistency below.
REFERENCE_MAP = {
    'asset-lab/vox/item_pickaxe_*.vox': 'assets/voxygen/voxel/weapon/tool/pickaxe_stone.vox',
    'asset-lab/vox/item_shovel_*.vox': 'assets/voxygen/voxel/weapon/tool/shovel_blue.vox',
}
# FAMILY consistency (HARD gate): every member of a tier family is compared
# against the family's densest member (the exemplar). Same tool at different
# tiers must silhouette the same — a regen that degrades one tier to a bare
# stick (the iron pick/shovel regression, caught 2026-07-10) FAILS here.
FAMILY_GLOBS = [
    'asset-lab/vox/item_pickaxe_*.vox',
    'asset-lab/vox/item_shovel_*.vox',
    'asset-lab/vox/item_axe_*.vox',
]
PROP_WARN, PROP_FAIL = 0.35, 0.60
SIL_WARN, SIL_FAIL = 0.45, 0.30
GRID = 24

fails = warns = checked = 0

def fail(msg):
    global fails
    fails += 1
    print(f'FAIL {msg}')

def warn(msg):
    global warns
    warns += 1
    print(f'WARN {msg}')

def principal_axes(cells):
    """Eigen-decomposition of the 3x3 cell covariance (power iteration +
    deflation — no numpy). Returns (eigvals desc, eigvecs) and the centroid."""
    n = len(cells)
    cx = sum(c[0] for c in cells) / n
    cy = sum(c[1] for c in cells) / n
    cz = sum(c[2] for c in cells) / n
    cov = [[0.0]*3 for _ in range(3)]
    for (x, y, z) in cells:
        d = (x - cx, y - cy, z - cz)
        for i in range(3):
            for j in range(3):
                cov[i][j] += d[i] * d[j]
    for i in range(3):
        for j in range(3):
            cov[i][j] /= n

    def matvec(m, v):
        return [sum(m[i][j] * v[j] for j in range(3)) for i in range(3)]

    def norm(v):
        return math.sqrt(sum(a*a for a in v))

    vals, vecs = [], []
    m = [row[:] for row in cov]
    for k in range(3):
        v = [1.0, 0.7 + k, 0.3]  # deterministic non-degenerate start
        for _ in range(60):
            w = matvec(m, v)
            nw = norm(w)
            if nw < 1e-12:
                w = [1.0 if i == k else 0.0 for i in range(3)]
                nw = 1.0
            v = [a / nw for a in w]
        lam = sum(v[i] * matvec(m, v)[i] for i in range(3))
        vals.append(max(lam, 1e-9))
        vecs.append(v)
        for i in range(3):  # deflate
            for j in range(3):
                m[i][j] -= lam * v[i] * v[j]
    order = sorted(range(3), key=lambda i: -vals[i])
    return [vals[i] for i in order], [vecs[i] for i in order], (cx, cy, cz)

def canon(vox):
    """Cells in principal-axis coordinates (rotation-invariant), plus
    sqrt-eigenvalue spreads (long, mid, short)."""
    cells = list(vox)
    vals, vecs, ctr = principal_axes(cells)
    proj = [tuple(sum((c[i] - ctr[i]) * vecs[a][i] for i in range(3))
                  for a in range(3)) for c in cells]
    spreads = tuple(math.sqrt(v) for v in vals)
    return proj, spreads

def proportions(spreads):
    long_, mid, short = spreads
    return (mid / long_, short / long_)

def views(proj):
    """Principal-plane projections rasterized to GRIDxGRID occupancy sets."""
    out = []
    for a, b in ((0, 1), (0, 2), (1, 2)):
        us = [c[a] for c in proj]; vs = [c[b] for c in proj]
        u0, u1 = min(us), max(us); v0, v1 = min(vs), max(vs)
        du = max(u1 - u0, 1e-9); dv = max(v1 - v0, 1e-9)
        ras = {(min(GRID-1, int((u - u0) / du * GRID)),
                min(GRID-1, int((v - v0) / dv * GRID))) for u, v in zip(us, vs)}
        out.append(ras)
    return out

def iou_flips(a, b):
    best = 0.0
    for fu in (False, True):
        for fv in (False, True):
            bb = {((GRID-1-u) if fu else u, (GRID-1-v) if fv else v) for u, v in b}
            inter = len(a & bb)
            union = len(a | bb)
            best = max(best, inter / union if union else 0.0)
    return best

def compare(asset_path, ref_path, advisory=False):
    global checked
    checked += 1
    name = os.path.basename(asset_path)
    ref_name = os.path.basename(ref_path)
    tag = 'advisory ' if advisory else ''
    flag = warn if advisory else fail  # advisory refs never hard-fail
    _, _, _, avox = read_vox(asset_path)['models'][0]
    _, _, _, rvox = read_vox(ref_path)['models'][0]
    ac, asp = canon(avox)
    rc, rsp = canon(rvox)
    ap, rp = proportions(asp), proportions(rsp)
    pdist = sum(abs(a - r) for a, r in zip(ap, rp))
    sil = sum(iou_flips(av, rv) for av, rv in zip(views(ac), views(rc))) / 3
    mass = len(avox) / max(len(rvox), 1)
    bad = False
    if not 0.4 <= mass <= 3.0:
        flag(f'{tag}{name} vs {ref_name}: r3 mass ratio {mass:.2f}x '
             f'({len(avox)} vs {len(rvox)} cells — complexity floor, class #15)')
        bad = True
    if pdist > PROP_FAIL:
        flag(f'{tag}{name} vs {ref_name}: r1 proportions diverge {pdist:.2f} '
             f'(ratios {tuple(round(v,2) for v in ap)} vs {tuple(round(v,2) for v in rp)})')
        bad = True
    elif pdist > PROP_WARN:
        warn(f'{tag}{name} vs {ref_name}: r1 proportion drift {pdist:.2f}')
        bad = True
    if sil < SIL_FAIL:
        flag(f'{tag}{name} vs {ref_name}: r2 silhouette IoU {sil:.2f} — does not read as the thing')
        bad = True
    elif sil < SIL_WARN:
        warn(f'{tag}{name} vs {ref_name}: r2 silhouette IoU {sil:.2f} — squint test, confirm by eye')
        bad = True
    if not bad:
        print(f'ok   {tag}{name} vs {ref_name}: prop {pdist:.2f}, silhouette IoU {sil:.2f}')

if __name__ == '__main__':
    refmap = dict(REFERENCE_MAP)
    idx = 'asset-lab/reference_index.json'
    if os.path.isfile(idx):
        refmap.update(json.load(open(idx, encoding='utf-8')))
    for pattern, ref in refmap.items():
        if not os.path.isfile(ref):
            warn(f'reference missing: {ref} (pattern {pattern})')
            continue
        for asset in sorted(glob.glob(pattern)):
            compare(asset, ref, advisory=True)
    # HARD gate: family self-consistency against the densest member.
    for pattern in FAMILY_GLOBS:
        members = sorted(glob.glob(pattern))
        if len(members) < 2:
            continue
        def cellcount(p):
            _, _, _, v = read_vox(p)['models'][0]
            return len(v)
        exemplar = max(members, key=cellcount)
        for m in members:
            if m != exemplar:
                # the crude tier is DESIGNED divergent ("asymmetric, primitive"
                # per the tool-tier brief) — advisory only; refined tiers must
                # hold the family silhouette.
                compare(m, exemplar, advisory='_crude' in os.path.basename(m))
    print(f'COMPARE-REFERENCE: {checked} pairs, {fails} FAIL, {warns} WARN')
    sys.exit(1 if fails else 0)
