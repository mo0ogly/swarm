"""R1 checks on isolated storage; real browser/server and labeled fixtures."""
import json
import pathlib
import subprocess
import tempfile

def execute(command):
    print('COMMAND ' + json.dumps(command), flush=True)
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode:
        print(result.stdout, result.stderr, flush=True)
        raise SystemExit(result.returncode)
    print('exit_code=0', flush=True)
    return result.stdout

with tempfile.TemporaryDirectory(prefix='swarm-r1-controls-') as root:
    binary = str(pathlib.Path(root) / 'swarm')
    execute(['sh', 'build.sh', binary])
    output = execute(['go', 'test', '-v', './...', '-run',
                      'TestVersion|TestServerVersion|TestRuntimeHealthEndpoint', '-count=1'])
    print(output, flush=True)
    for fixture in (False, True):
        destination = str(pathlib.Path(root) / ('fixture' if fixture else 'real'))
        command = ['node', 'tests/version_ui.cjs', binary, destination]
        if fixture:
            command.append('--fixture')
        execute(command)
        observed = json.loads((pathlib.Path(destination) / 'result.json').read_text())
        print(json.dumps({key: observed[key] for key in
                          ('status', 'mode', 'checks', 'errors', 'failed')}, ensure_ascii=False), flush=True)
    version = json.loads(execute([binary, '--json', 'version']))
    print(json.dumps({'public_cli_version': version}, ensure_ascii=False), flush=True)
    print('PASS R1 targeted controls. Fixture identities are simulated; real server was started separately. No claim of live mission autonomy.', flush=True)
