"""Final checks once, using isolated server storage for the browser recipe."""
import json
import pathlib
import subprocess
import tempfile
import time
from collections import deque

def execute(command):
    print('COMMAND ' + json.dumps(command), flush=True)
    started = time.monotonic()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    tail = deque(maxlen=35)
    last_progress = started
    for line in process.stdout:
        if '-json' not in command:
            print(line, end='', flush=True)
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            print(line, end='', flush=True)
            continue
        tail.append(event.get('Output', '').rstrip())
        action = event.get('Action')
        elapsed = time.monotonic() - started
        if action == 'run' and elapsed - last_progress >= 10:
            print(f"RUN elapsed={elapsed:.1f}s test={event.get('Test')} package={event.get('Package')}", flush=True)
            last_progress = time.monotonic()
        if action == 'fail':
            print('FAIL ' + json.dumps(event), flush=True)
            print('\n'.join(tail), flush=True)
        elif action in ('pass', 'skip') and not event.get('Test'):
            print(action.upper() + ' ' + json.dumps(event), flush=True)
    code = process.wait()
    print(f'exit_code={code} duration_seconds={time.monotonic()-started:.1f}', flush=True)
    if code:
        print('LAST OUTPUT\n' + '\n'.join(tail), flush=True)
        raise SystemExit(code)

execute(['python3', 'tests/supervision_go_suite.py'])
execute(['go', 'vet', './...'])
execute(['python3', 'tools/agent-workflows/check.py'])
execute(['git', 'diff', '--check'])
with tempfile.TemporaryDirectory(prefix='swarm-final-controls-') as root:
    binary = str(pathlib.Path(root) / 'swarm')
    execute(['sh', 'build.sh', binary])
    destination = str(pathlib.Path(root) / 'version-browser')
    execute(['node', 'tests/version_ui.cjs', binary, destination])
    result = json.loads((pathlib.Path(destination) / 'result.json').read_text())
    print(json.dumps({key: result[key] for key in ('status','mode','checks','errors','failed')}, ensure_ascii=False), flush=True)
execute(['python3', 'tests/observable_validation_acceptance.py'])
print('PASS final package, vet, configuration and isolated version UI checks; not proof of external provider autonomy.', flush=True)
