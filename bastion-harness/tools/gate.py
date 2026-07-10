"""bastion (B-ASSET1): LOUD gate runner. One command, one verdict.

Kills the silent grep-pipe failure mode (the stale-exe incident): every step
is a subprocess whose exit code is CHECKED; the word PASS is printed only if
EVERY step passed, and the script itself exits nonzero otherwise. Never
chain this behind `| tail` and eyeball it — check $?/exit code.

Default = the STATIC asset battery (machine-light, no engine):
    catalog_recheck, rig_check, anim_lint, adjacency_precheck,
    connectivity vs committed baseline (NEW findings fail; resolved ones
    are reported as improvements — regen the baseline deliberately).

Custom gates (e.g. the morning full gate: cargo test + scenarios):
    python bastion-harness/tools/gate.py --steps mygate.json
where mygate.json = [{"name": "...", "cmd": ["...", ...]}, ...]
(every step is exit-code checked; add "baseline": "file" to diff stdout
against a committed baseline instead).

Run from the repo root.
"""
import sys, os, json, subprocess
sys.stdout.reconfigure(encoding='utf-8')

TOOLS = os.path.dirname(os.path.abspath(__file__))
PY = sys.executable

DEFAULT_STEPS = [
    {'name': 'catalog_recheck', 'cmd': [PY, os.path.join(TOOLS, 'catalog_recheck.py')]},
    {'name': 'rig_check', 'cmd': [PY, os.path.join(TOOLS, 'rig_check.py')]},
    {'name': 'anim_lint', 'cmd': [PY, os.path.join(TOOLS, 'anim_lint.py')]},
    {'name': 'adjacency_precheck', 'cmd': [PY, os.path.join(TOOLS, 'adjacency_precheck.py')]},
    {'name': 'connectivity_vs_baseline',
     'cmd': [PY, os.path.join(TOOLS, 'connectivity_check.py')],
     'glob_args': 'asset-lab/vox/real/*.vox',
     'baseline': os.path.join(TOOLS, 'baselines', 'connectivity_catalog.txt')},
]

def run_step(step):
    import glob as _glob
    cmd = list(step['cmd'])
    if step.get('glob_args'):
        cmd += sorted(_glob.glob(step['glob_args']))
    r = subprocess.run(cmd, capture_output=True, text=True, encoding='utf-8')
    out = (r.stdout or '') + (r.stderr or '')
    if 'baseline' in step:
        want = open(step['baseline'], encoding='utf-8').read().splitlines()
        got = out.splitlines()
        new = [l for l in got if l and l not in want]
        gone = [l for l in want if l and l not in got]
        for l in gone:
            print(f'  [{step["name"]}] resolved vs baseline (regen deliberately): {l[:120]}')
        if new:
            for l in new:
                print(f'  [{step["name"]}] NEW FINDING: {l[:160]}')
            return False, out
        return True, out
    if r.returncode != 0:
        return False, out
    return True, out

if __name__ == '__main__':
    steps = DEFAULT_STEPS
    if len(sys.argv) > 2 and sys.argv[1] == '--steps':
        steps = json.load(open(sys.argv[2], encoding='utf-8'))
    results = []
    for step in steps:
        ok, out = run_step(step)
        results.append((step['name'], ok))
        print(f'{"ok  " if ok else "FAIL"} {step["name"]}')
        if not ok:
            tail = out.splitlines()[-12:]
            for l in tail:
                print(f'     | {l[:160]}')
    print('-' * 50)
    failed = [n for n, ok in results if not ok]
    if failed:
        print(f'GATE FAIL ({len(failed)}/{len(results)} steps): {", ".join(failed)}')
        sys.exit(1)
    print(f'GATE PASS ({len(results)}/{len(results)} steps)')
    sys.exit(0)
