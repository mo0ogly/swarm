"""Bound the proof to actual local Go inputs; no global execution watchdog."""
from pathlib import Path
import datetime
import hashlib
import json
import os
import subprocess
import sys
import tempfile

ROOT = Path.cwd()
OUT = Path(__file__).resolve().parent
LABEL, PATTERN = sys.argv[1:3]
DELIVERY = '--delivery' in sys.argv[3:]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def inputs():
    raw = subprocess.check_output(['go', 'list', '-mod=readonly', '-deps', '-test', '-json', './internal/engine', './cmd/swarm'], text=True)
    decoder = json.JSONDecoder()
    paths = {ROOT / 'go.mod', ROOT / 'go.sum', Path(__file__).resolve()}
    while raw.strip():
        item, end = decoder.raw_decode(raw.lstrip())
        raw = raw.lstrip()[end:]
        directory = Path(item.get('Dir', '/')).resolve()
        if not directory.is_relative_to(ROOT):
            continue
        for field in ['GoFiles', 'CgoFiles', 'CFiles', 'HFiles', 'SFiles', 'SysoFiles', 'TestGoFiles', 'XTestGoFiles', 'EmbedFiles', 'TestEmbedFiles', 'XTestEmbedFiles']:
            for name in item.get(field, []):
                path = (directory / name).resolve()
                if path.is_relative_to(ROOT) and path.is_file():
                    paths.add(path)
    return {str(p.relative_to(ROOT)): digest(p) for p in sorted(paths)}

def save(name, value):
    with (OUT / name).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')

before = inputs()
save(LABEL + '-inputs-before.json', before)
binary = Path(tempfile.mkdtemp(prefix='swarm-m28-bin-')) / 'swarm'
steps = [('tests', ['go', 'test', '-mod=readonly', './internal/engine', '-run', PATTERN, '-count=1', '-timeout=0', '-json'])]
if DELIVERY:
    steps += [('vet', ['go', 'vet', '-mod=readonly', './internal/engine']), ('build', ['go', 'build', '-mod=readonly', '-o', str(binary), './cmd/swarm'])]
results = []
for kind, command in steps:
    log = LABEL + '-' + kind + ('.jsonl' if kind == 'tests' else '.log')
    start = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with (OUT / log).open('x') as stream:
        result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT)  # deliberately no timeout
    record = dict(kind=kind, command=command, start=start, end=datetime.datetime.now(datetime.timezone.utc).isoformat(), exit_code=result.returncode, log=log)
    results.append(record)
    print(json.dumps(record), flush=True)
    if result.returncode:
        break
after = inputs()
save(LABEL + '-inputs-after.json', after)
events = []
for line in (OUT / (LABEL + '-tests.jsonl')).read_text().splitlines():
    try:
        events.append(json.loads(line))
    except ValueError:
        pass
passed = [e['Test'] for e in events if e.get('Action') == 'pass' and 'Test' in e]
failed = [e.get('Test', e.get('Package')) for e in events if e.get('Action') == 'fail']
skipped = [e.get('Test') for e in events if e.get('Action') == 'skip']
ok = bool(passed) and not failed and not skipped and all(r['exit_code'] == 0 for r in results) and before == after
proof = dict(test_environment={k: os.environ[k] for k in ['SWARM_MEMORY_RECIPE_BUNDLE'] if k in os.environ}, status='PASS' if ok else 'FAIL', steps=results, passed=passed, failed=failed, skipped=skipped,
             inputs_unchanged=before == after, changed_inputs=[n for n in before.keys() | after.keys() if before.get(n) != after.get(n)],
             binary=str(binary) if binary.exists() else None, binary_sha256=digest(binary) if binary.exists() else None,
             global_watchdog='disabled: go test -timeout=0, subprocess without timeout', toolchain=subprocess.check_output(['go', 'version'], text=True).strip())
save(LABEL + '-run.json', proof)
print(json.dumps(dict(status=proof['status'], passed=len(passed), failed=failed, inputs_unchanged=before == after)), flush=True)
sys.exit(0 if ok else 1)
