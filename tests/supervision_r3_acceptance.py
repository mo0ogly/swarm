"""R3 scoped behavioral checks; requires task-specific tests."""
import json
import subprocess
listed = subprocess.run(['go', 'test', '-list', '^TestSupervisionR3', './internal/engine'], capture_output=True, text=True)
if listed.returncode or 'TestSupervisionR3' not in listed.stdout:
    print(listed.stdout, listed.stderr)
    raise SystemExit('Missing R3 behavioral checks')
command = ['go', 'test', '-v', '-count=1', '-run', 'Test(SupervisionR3|StructuredProviderProgressCounters|StructuredProviderTimeoutProgress|SilenceRecursAfterNewHeartbeat|LargeToolResultMustClosePendingTool)', './internal/engine']
print('COMMAND ' + json.dumps(command), flush=True)
r = subprocess.run(command, capture_output=True, text=True)
print(r.stdout, r.stderr, flush=True)
print('exit_code=' + str(r.returncode), flush=True)
if r.returncode:
    raise SystemExit(r.returncode)
command = ['go', 'test', '-race', '-v', '-count=1', '-run', '^TestSupervisionR3', './internal/engine']
print('COMMAND ' + json.dumps(command), flush=True)
r = subprocess.run(command, capture_output=True, text=True)
print(r.stdout, r.stderr, flush=True)
print('exit_code=' + str(r.returncode), flush=True)
raise SystemExit(r.returncode)
