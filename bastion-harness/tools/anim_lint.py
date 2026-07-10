"""bastion (B-ASSET1, quality gate): STATIC linter for rig.json `anims` data.

Validates the pilot's animation data BEFORE animation-code exists to debug
against it. Three legal anim shapes (all seen in the god-hand batch):
  - loop-layer LIST:  [{name, desc, ...params, period_s}, ...]
  - phased DICT:      {desc, <phase>: {...params, dur_s}, ...}
  - flat one-shot:    {desc, ...params, dur_s | period_s}

Timing vocabulary (learned from the pilot's actual data — do not narrow):
  any `*_s` key = seconds; any `*_hz`/`hz` key = loop rate; `period_s` =
  loop period; `modifier: true` = stateless idle-blend (NO duration needed;
  `blend_over` names the anim it modifies); `states` = container of poses
  (not phases); `loop: true` loops via a `*_hz` rate.

Checks (FAIL = animation-code would misbehave; WARN = suspicious):
  1. every hand anim has a desc (list layers: name+desc) — vessels predate
     the desc convention, so missing desc = WARN
  2. every non-modifier anim is schedulable: some `*_s` > 0 or `*_hz` > 0
     at some level
  3. numeric sanity: *_s >= 0, *_deg within +/-180, lag/stagger < its
     phase/loop duration (WARN)
  4. `ease` from the known tween set (WARN on new names so the code-side
     lookup grows deliberately, not by KeyError)
  5. explicit bone refs ("bone"/"bones") exist in skel.bones; `blend_over`
     refs name an existing anim
  6. CROSS-VARIANT CONTRACT (kind=hand only): alignment-blend variants
     must carry IDENTICAL anim data (the blend crossfades all variants
     over one animation state). Different ships legitimately differ.

Run from the repo root (reads asset-lab READ-ONLY):
    python bastion-harness/tools/anim_lint.py [rig_dir ...]
Defaults to every dir under asset-lab/vox with a rig.json carrying anims.
Exits nonzero if any FAIL.
"""
import sys, os, glob, json
sys.stdout.reconfigure(encoding='utf-8')

KNOWN_EASE = {
    'linear', 'in-quad', 'out-quad', 'in-out-quad', 'in-cubic', 'out-cubic',
    'in-out-cubic', 'out-back', 'out-elastic', 'out-bounce', 'hold',
}
IDENTICAL_DATA_KINDS = {'hand'}  # alignment-blend variants: full data identity

fails = warns = 0

def fail(msg):
    global fails
    fails += 1
    print(f'FAIL {msg}')

def warn(msg):
    global warns
    warns += 1
    print(f'WARN {msg}')

def check_params(where, d, bones, dur_ctx=None):
    """Numeric/ease/bone checks on one flat param dict."""
    dur = d.get('dur_s', d.get('period_s', dur_ctx))
    for k, v in d.items():
        if k in ('desc', 'name'):
            continue
        if k in ('bone', 'bones'):
            refs = v if isinstance(v, list) else [v]
            for r in refs:
                if r not in bones:
                    fail(f'{where}: bone ref "{r}" not in skeleton {sorted(bones)}')
            continue
        if k == 'ease':
            if v not in KNOWN_EASE:
                warn(f'{where}: unknown ease "{v}" (extend the code lookup deliberately)')
            continue
        if isinstance(v, (int, float)):
            if k.endswith('_s') and v < 0:
                fail(f'{where}: {k}={v} negative time')
            if k.endswith('_deg') and abs(v) > 180:
                fail(f'{where}: {k}={v} outside +/-180')
            if k.endswith(('_lag_s', 'stagger_s')) and dur and v >= dur:
                warn(f'{where}: {k}={v} >= its duration {dur} (lag longer than the move)')

def schedulable(d):
    """Any seconds- or rate-valued key > 0, at any nesting level."""
    for k, v in d.items():
        if isinstance(v, dict) and schedulable(v):
            return True
        if isinstance(v, (int, float)) and v > 0 and (
                k.endswith('_s') or k.endswith('_hz') or k == 'hz'):
            return True
    return False

def check_anim(rid, aname, spec, bones, anim_names, is_hand):
    where = f'{rid}.{aname}'
    if isinstance(spec, list):
        for i, layer in enumerate(spec):
            lw = f'{where}[{i}]'
            if not isinstance(layer, dict):
                fail(f'{lw}: loop layer is not a dict'); continue
            if 'name' not in layer or 'desc' not in layer:
                (fail if is_hand else warn)(f'{lw}: loop layer missing name/desc')
            if not schedulable(layer):
                fail(f'{lw}: loop layer has no *_s/*_hz > 0')
            check_params(lw, layer, bones)
    elif isinstance(spec, dict):
        if 'desc' not in spec:
            (fail if is_hand else warn)(f'{where}: missing desc')
        bo = spec.get('blend_over')
        if bo and bo not in anim_names:
            fail(f'{where}: blend_over "{bo}" names no anim in this rig')
        # `states` holds poses (no duration); other sub-dicts are phases.
        for pname, ph in spec.items():
            if not isinstance(ph, dict):
                continue
            if pname == 'states':
                for sname, pose in ph.items():
                    if isinstance(pose, dict):
                        check_params(f'{where}.states.{sname}', pose, bones)
                continue
            pw = f'{where}.{pname}'
            if not schedulable(ph):
                fail(f'{pw}: phase has no *_s/*_hz > 0')
            check_params(pw, ph, bones)
        flat = {k: v for k, v in spec.items() if not isinstance(v, dict)}
        check_params(where, flat, bones)
        if not spec.get('modifier') and not schedulable(spec):
            fail(f'{where}: not schedulable (no *_s/*_hz > 0) and not a modifier')
    else:
        fail(f'{where}: anim is neither dict nor list')

if __name__ == '__main__':
    targets = sys.argv[1:] or sorted({
        os.path.dirname(p) for p in glob.glob('asset-lab/vox/**/rig.json', recursive=True)})
    by_kind = {}
    seen_ids = {}
    for rd in targets:
        rp = os.path.join(rd, 'rig.json')
        if not os.path.isfile(rp):
            continue
        dup = seen_ids.get(os.path.basename(rd.rstrip('/\\')))
        if dup:
            # duplicate staging locations (e.g. vox/ and vox/vehicles/) — lint
            # once, but surface it: two copies with no declared authority DRIFT.
            print(f'INFO duplicate rig staging: {rd} also at {dup} (lint once; '
                  f'pilot should keep ONE authoritative copy)')
            continue
        seen_ids[os.path.basename(rd.rstrip('/\\'))] = rd
        rig = json.load(open(rp, encoding='utf-8'))
        anims = rig.get('anims')
        if not anims:
            continue
        rid = os.path.basename(rd.rstrip('/\\'))
        kind = rig.get('kind', '?')
        bones = {b['name'] for b in rig.get('skel', {}).get('bones', [])}
        for aname, spec in anims.items():
            check_anim(rid, aname, spec, bones, set(anims), kind in IDENTICAL_DATA_KINDS)
        by_kind.setdefault(kind, []).append((rid, anims))
    # cross-variant contract: only for variant-family kinds (hand); different
    # ships legitimately carry different anim sets (oars vs sails).
    for kind in IDENTICAL_DATA_KINDS:
        rigs = by_kind.get(kind, [])
        if len(rigs) < 2:
            continue
        base_id, base = rigs[0]
        for rid, anims in rigs[1:]:
            if set(anims) != set(base):
                fail(f'kind={kind}: anim KEY SET drift {base_id} vs {rid}: '
                     f'{set(anims) ^ set(base)}')
            elif anims != base:
                diff = [a for a in base if anims.get(a) != base[a]]
                fail(f'kind={kind}: anim DATA drift {base_id} vs {rid} in {diff} '
                     f'(alignment-blend requires identical data)')
    print(f'ANIM LINT: {fails} FAIL, {warns} WARN')
    sys.exit(1 if fails else 0)
