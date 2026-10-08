"""Disposable-trial worker adapter. Never configure this as planner/reviewer.

Configuration is created by the trial operator outside agent copies. The engine
binary and Store must belong to that trial. Arguments after the config file are
forwarded unchanged to the configured real producer executable.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
from controlled_business_fault import inject_once, digest


def run(config_path, args, prompt):
    config = json.loads(Path(config_path).read_text())
    def verify_engine():
        if digest(Path(config['engine']).read_bytes()) != config['engine_sha256']:
            raise ValueError('engine changed: trial version is no longer frozen')
    verify_engine()
    cwd = Path.cwd().resolve()
    copies = Path(config['copies_root']).resolve(strict=True)
    if cwd.parent != copies:
        raise ValueError('adapter only allowed inside this trial’s disposable copies')
    snapshot = subprocess.run([config['engine'], '--root', config['store'], '--json',
                               'agent', 'list', config['work']],
                              capture_output=True, text=True, timeout=30, check=True)
    agents = [entry.get('agent', entry) for entry in json.loads(snapshot.stdout)['agents']]
    matches = [a for a in agents if a.get('role') == 'worker'
               and a.get('status') in ('queued', 'starting', 'running')
               and Path(a['workspace']).resolve() == cwd]
    if len(matches) != 1:
        raise ValueError('exactly one active worker must own this copy')
    agent = matches[0]
    if agent['work_id'] != config['work']:
        raise ValueError('worker belongs to another mission')
    # Inherited stdout/stderr preserve the producer's observable event stream.
    producer = subprocess.run([config['producer'], *args], input=prompt)
    if producer.returncode:
        return producer.returncode
    verify_engine()
    inject_once(copies, cwd, config['evidence_dir'], work=config['work'],
                agent=agent['id'], attempt=agent['attempt_id'])
    return 0


if __name__ == '__main__':
    try:
        code = run(sys.argv[1], sys.argv[2:], sys.stdin.buffer.read())
    except Exception as exc:
        print('Controlled trial adapter failed: '+str(exc), file=sys.stderr)
        code = 78
    raise SystemExit(code)
