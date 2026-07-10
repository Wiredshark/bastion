"""bastion (B-ASSET1): REDO-CAMPAIGN anti-skip audit (Ben's completeness gate).

Ben's rule: EVERY asset must show a genuine redo — the ONLY exemptions are
the god-hands and the galleon (which must carry the bow→stern rudder fix;
the anatomy gate asserts that continuously). "KEEP" verdicts, native
variants, and 2D glyphs are NOT exempt.

This is a COMPLETENESS gate, separate from quality: did the redo actually
happen for all-but-2? Detection:
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
    new_files = []
    for p in sorted(before) + sorted(
            set(glob.glob('asset-lab/vox/**/*.vox', recursive=True))
            - {q.replace('/', os.sep) for q in before} - set(before)):
        pp = p.replace(os.sep, '/')
        if not os.path.isfile(pp):
            continue  # deleted/moved files are the manifest's business
        h = hashlib.sha256(open(pp, 'rb').read()).hexdigest()[:16]
        exempt = any(s in pp for s in EXEMPT_SUBSTRINGS)
        aid = asset_id(pp)
        if pp not in before:
            new_files.append(pp)
            continue
        if h == before[pp]:
            if not exempt:
                silent.append(pp)
        elif aid not in campaign and not exempt:
            missing_entry.append(pp)
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
          f'{len(missing_entry)} missing entries')
    sys.exit(1 if (silent or missing_entry) else 0)
