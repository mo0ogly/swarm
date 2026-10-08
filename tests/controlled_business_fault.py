"""One-shot fault fixture for autonomy trials; never an autonomy verdict.

This module does not start a provider or change Swarm's Store. The trial's worker
adapter must call inject_once only after a successful producer process exits,
before returning to the engine. Planner/reviewer adapters must never call it.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

GOOD = 'def is_expected(value):\n    return value == "expected"\n'
BAD = 'def is_expected(value):\n    return True\n'
CHECK = '''import importlib.util, pathlib, sys
p=pathlib.Path(sys.argv[1])
s=importlib.util.spec_from_file_location("fixture_feature",p)
m=importlib.util.module_from_spec(s)
s.loader.exec_module(m)
for v,want in [("expected",True),("wrong",False),("",False),(None,False)]:
    got=m.is_expected(v)
    if got is not want: raise SystemExit("business mismatch: %r -> %r, expected %r" % (v,got,want))
'''


def digest(data):
    return hashlib.sha256(data).hexdigest()


def check_feature(path):
    # -B prevents incidental cache mutations from becoming the injected defect.
    return subprocess.run([sys.executable, '-B', '-c', CHECK, str(path)],
                          capture_output=True, text=True, timeout=10)


def atomic_json(path, value):
    fd, name = tempfile.mkstemp(prefix='.fault-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def inject_once(copies_root, workspace, evidence_dir, *, work, agent, attempt):
    """Inject only into a verified disposable copy; persist attribution once.

    evidence_dir is outside all agent copies. A pending marker after interruption
    requires explicit examination: never infer injection success or try again.
    """
    root, folder, evidence = map(Path, (copies_root, workspace, evidence_dir))
    if not all(isinstance(x, str) and x.strip() for x in (work, agent, attempt)):
        raise ValueError('work, agent and attempt identities required')
    if root.is_symlink() or folder.is_symlink():
        raise ValueError('symlinked copy refused')
    root, folder, evidence = root.resolve(strict=True), folder.resolve(strict=True), evidence.resolve()
    if folder.parent != root:
        raise ValueError('workspace must be a direct disposable copy')
    if evidence == root or root in evidence.parents:
        raise ValueError('evidence must remain outside agent copies')
    target = folder / 'feature.py'
    if target.is_symlink() or not target.is_file():
        raise ValueError('regular feature.py required')
    evidence.mkdir(parents=True, exist_ok=True)
    marker = evidence / 'injection.json'
    with (evidence / 'injection.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if marker.exists():
            prior = json.loads(marker.read_text())
            if prior['work'] != work:
                raise ValueError('evidence belongs to a different work')
            if prior['state'] != 'injected':
                raise ValueError('incomplete injection: examine evidence before retry')
            return {'injected_now': False, 'evidence': prior}
        before = target.read_bytes()
        good = check_feature(target)
        if good.returncode:
            raise ValueError('producer result already fails; controlled fault not demonstrated')
        record = dict(schema_version=1, state='pending', work=work, agent=agent,
                      attempt=attempt, workspace=str(folder), target='feature.py',
                      before_sha256=digest(before), after_sha256=digest(BAD.encode()),
                      precheck_exit=good.returncode, checker_sha256=digest(CHECK.encode()),
                      injector_sha256=digest(Path(__file__).read_bytes()), origin='authorized_test_fault_injection')
        atomic_json(marker, record)  # Reserve before changing any producer file.
        (evidence / 'before-feature.py').write_bytes(before)
        target.write_text(BAD)
        failed = check_feature(target)
        record.update(state='injected', postcheck_exit=failed.returncode,
                      diagnostic=failed.stdout + failed.stderr)
        if failed.returncode == 0:
            raise RuntimeError('fault failed to trigger the independent business check')
        atomic_json(marker, record)
        return {'injected_now': True, 'evidence': record}
