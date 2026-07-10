"""bastion (B-ASSET1): REDO-CAMPAIGN anti-skip audit (Ben's completeness gate).

Ben's rule: EVERY asset must show a genuine redo — the ONLY exemptions are
the god-hands and the galleon (which must carry the bow→stern rudder fix;
the anatomy gate asserts that continuously). "KEEP" verdicts, native
variants, and 2D glyphs are NOT exempt.

THREE checks (Ben's directives, 2026-07-10):
  1+2 COMPLETENESS (below): did the redo actually happen for all-but-2?
  3 DETAIL FLOOR: a redone asset's programmatic detail metrics must be >=
    its matched native comparator's. THE MATH LIVES IN THE PILOT'S MODULE
    (asset-lab/gen/detail_metrics.py) — this tool IMPORTS it, never
    reimplements (divergent math = false pass/fail; one definition of
    "cleared the floor" on both sides). Comparator mapping comes from
    asset-lab/reference_index.json. Until both exist, the floor check
    reports PENDING and does not gate.

Completeness detection:
  - SILENT KEEP: current file hash == the tester's independent BEFORE
    snapshot (baselines/redo_before_hashes.txt, taken 2026-07-10 when the
    directive landed — asset-lab is untracked by git, so this snapshot is
    the only tamper-proof before-record from now on) and the asset is not
    exempt. CAVEAT: redos completed BEFORE the snapshot show identical
    hashes — those must be evidenced by their REDO-CAMPAIGN.md entry +
    before/after renders; list them for manual confirmation, don't auto-fail.
  - MISSING ENTRY: hash changed but no what-changed line in
    readme/REDO-CAMPAIGN.md mentions the asset id.

Run from the repo root when the pilot reports a milestone:
    python bastion-harness/tools/redo_audit.py
Exits nonzero if any non-exempt silent keep or missing entry remains.
"""
import sys, os, glob, hashlib
sys.stdout.reconfigure(encoding='utf-8')

BASE = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                    'baselines', 'redo_before_hashes.txt')
CAMPAIGN = 'readme/REDO-CAMPAIGN.md'
EXEMPT_SUBSTRINGS = ('godhand', 'vehicle_galleon_warship')

def asset_id(path):
    """The id an entry line would mention: basename without extension."""
    b = os.path.basename(path)
    for suf in ('.vox', '.ron', '.json', '.png'):
        if b.endswith(suf):
            return b[:-len(suf)]
    return b

if __name__ == '__main__':
    before = {}
    for line in open(BASE, encoding='utf-8'):
        if line.startswith('#') or not line.strip():
            continue
        h, p = line.split(None, 1)
        before[p.strip()] = h
    campaign = (open(CAMPAIGN, encoding='utf-8').read()
                if os.path.isfile(CAMPAIGN) else '')
    if not campaign:
        print(f'NOTE: {CAMPAIGN} does not exist yet — every changed asset '
              f'counts as MISSING ENTRY until it does.')
    silent = []
    missing_entry = []
    for pp in sorted(before):
        if not os.path.isfile(pp):
            continue  # deleted/moved files are the manifest's business
        h = hashlib.sha256(open(pp, 'rb').read()).hexdigest()[:16]
        if any(s in pp for s in EXEMPT_SUBSTRINGS):
            continue
        if h == before[pp]:
            silent.append(pp)
        elif asset_id(pp) not in campaign:
            missing_entry.append(pp)
    current = {p.replace(os.sep, '/') for p in
               glob.glob('asset-lab/vox/**/*.vox', recursive=True)}
    new_files = sorted(current - set(before))

    # Check 3: DETAIL FLOOR — the pilot's metric module is the single
    # source of the math; we only orchestrate (metric(asset) >= metric(native)
    # per metric, comparator from reference_index.json).
    under_floor = []
    floor_state = 'PENDING'
    idx_path = 'asset-lab/reference_index.json'
    sys.path.insert(0, 'asset-lab/gen')
    try:
        import json as _json
        # The pilot's module is the WHOLE shared definition: metrics_of()
        # (fill_density/bands/surface_ratio/protrusions/cells), FLOOR_KEYS,
        # combine_natives() (per-metric MEDIAN over natives), and
        # meets_floor() (owns TOL=0.90 and the comparison itself). We only
        # orchestrate — zero math on this side.
        import detail_metrics
        refmap = _json.load(open(idx_path, encoding='utf-8'))
        floor_state = 'ACTIVE'
        for pattern, ref in refmap.items():
            refs = ref if isinstance(ref, list) else [ref]
            missing = [r for r in refs if not os.path.isfile(r)]
            if missing:
                print(f'  FLOOR: comparator missing {missing} (pattern {pattern})')
                continue
            floor = detail_metrics.combine_natives(
                [detail_metrics.metrics_of(r) for r in refs])
            for asset in sorted(glob.glob(pattern)):
                ok, report = detail_metrics.meets_floor(
                    detail_metrics.metrics_of(asset), floor)
                if not ok:
                    under_floor.append((asset, report))
                    print(f'  UNDER-FLOOR {asset} vs {refs}: {report}')
    except (ImportError, FileNotFoundError):
        pass  # module or index not shipped yet — floor check stays PENDING
    print(f'=== DETAIL FLOOR: {floor_state}'
          + (f', {len(under_floor)} under-floor' if floor_state == 'ACTIVE' else
             ' (needs gen/detail_metrics.py + reference_index.json from the pilot)'))
    print(f'=== SILENT KEEPS (hash unchanged since the tester snapshot, '
          f'not exempt): {len(silent)}')
    for p in silent:
        print(f'  KEEP? {p}  — needs a redo, or pre-snapshot-redo evidence '
              f'(campaign entry + before/after render)')
    print(f'=== CHANGED BUT NO CAMPAIGN ENTRY: {len(missing_entry)}')
    for p in missing_entry:
        print(f'  NO-ENTRY {p}')
    if new_files:
        print(f'=== NEW since snapshot (not redos, informational): {len(new_files)}')
    print(f'REDO AUDIT: {len(silent)} silent keeps, '
          f'{len(missing_entry)} missing entries, '
          f'{len(under_floor)} under-floor ({floor_state})')
    sys.exit(1 if (silent or missing_entry or under_floor) else 0)
