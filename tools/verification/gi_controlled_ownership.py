"""Run exactly three controlled ownership regressions in one admitted assembly.

Called only after root allocation with Python -I -S -B. Helpers remain real;
no campaign, CLI, network or nested subprocess is authorized. Actual module
origins and hashes are recorded after import/tests even on a failing case.
"""
import hashlib
import importlib
import json
from pathlib import Path
import sys
import unittest

assembly = Path(sys.argv[1]).resolve(strict=True)
receipt = Path(sys.argv[2]).resolve()
assert sys.flags.isolated and sys.flags.no_site and sys.dont_write_bytecode
assert not sys.flags.optimize
assert not (assembly / '.swarm').exists()
original_path = list(sys.path)
sys.path.insert(0, str(assembly / 'tests'))
audit_denials = []
def audit(event, args):
    """Refuse unallocated nested execution/network without exposing arguments."""
    if event in ('subprocess.Popen', 'os.system', 'os.posix_spawn', 'os.fork', 'socket.connect', 'socket.bind'):
        audit_denials.append(event)
        raise RuntimeError('controlled-unit-unallocated-effect:' + event)
sys.addaudithook(audit)
result = None
try:
    module = importlib.import_module('test_rust_unstarted_review_parent')
    names = ['test_observer_stops_before_next_root', 'test_close_refusal_is_not_closed_or_retried', 'test_consumption_survives_validation_refusal']
    suite = unittest.TestSuite(module.OwnershipRegression(name) for name in names)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
finally:
    origins = {}
    for name, module in sorted(sys.modules.items()):
        file = getattr(module, '__file__', None)
        if file and Path(file).is_file():
            path = Path(file).resolve()
            origins[name] = dict(path=str(path), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    fixture = sys.modules.get('fixture_area')
    data = dict(executable=sys.executable, version=sys.version, original_sys_path=original_path,
                final_sys_path=sys.path, origins=origins, audit_denials=audit_denials,
                fixture_repo=str(getattr(fixture, '_REPO', None)), fixture_area=str(getattr(fixture, '_AREA', None)),
                tests_run=None if result is None else result.testsRun,
                failures=None if result is None else len(result.failures),
                errors=None if result is None else len(result.errors),
                scope='CONTROLLED_DOUBLES_ONLY; no OS/child/provider/campaign acceptance')
    receipt.write_text(json.dumps(data, indent=2) + '\n')
if result is None or result.testsRun != 3 or audit_denials:
    raise SystemExit(2)
raise SystemExit(0 if result.wasSuccessful() else 1)
