#!/usr/bin/env python3
"""Read-only observation path for PAEA lab sinks, separate from their decisions.

This process reads actual SQLite effect rows or Git refs. It never imports the
fixture service, consumes a receipt, approves, reconciles or repairs. Same-UID
access and common authorship mean neither protected nor independent evaluation.
"""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys


def observe(root, domain):
    """Return raw sink observations, or propagate unavailability with its cause.

    root must be a test-owned fixture, domain payment/git. SQLite uses mode=ro
    plus query_only; missing files never create an empty database. Git routing
    and global config are excluded. No decision/approval/audit table is read.
    No absence of effect is inferred here from unavailable observation.
    """
    root = Path(root).resolve(strict=True)
    if domain == 'payment':
        db = sqlite3.connect((root / 'sink.sqlite').as_uri() + '?mode=ro', uri=True, timeout=0)
        try:
            db.execute('PRAGMA query_only=ON')
            rows = db.execute('SELECT intent,digest,canonical FROM effects ORDER BY intent').fetchall()
            return {'domain': domain, 'effects': [{'intent': r[0], 'digest': r[1], 'canonical': r[2]} for r in rows]}
        finally:
            db.close()
    if domain != 'git':
        raise ValueError('oracle-domain-invalid')
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
    result = subprocess.run(['git', '--git-dir=' + str(root / 'repository.git'),
                             'for-each-ref', '--format=%(refname) %(objectname) %(symref)'],
                            capture_output=True, env=env)
    if result.returncode != 0:
        raise RuntimeError({'exit': result.returncode, 'stderr': result.stderr.decode(errors='replace')})
    refs = []
    for line in result.stdout.decode().splitlines():
        parts = line.split(maxsplit=2)
        item = {'ref': parts[0], 'oid': parts[1], 'symbolic_target': parts[2] if len(parts) == 3 else ''}
        if parts[0].startswith('refs/paea-lab/intents/'):
            blob = subprocess.run(['git', '--git-dir=' + str(root / 'repository.git'),
                                   'cat-file', 'blob', parts[1]], capture_output=True, env=env)
            if blob.returncode != 0:
                raise RuntimeError({'exit': blob.returncode, 'stderr': blob.stderr.decode(errors='replace')})
            item['canonical'] = blob.stdout.decode()
        refs.append(item)
    return {'domain': domain, 'refs': refs}


def main():
    """Emit a complete raw observation or nonzero UNKNOWN; never successful empty IO."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path)
    parser.add_argument('domain', choices=('payment', 'git'))
    args = parser.parse_args()
    try:
        print(json.dumps({'scope': 'isolated-laboratory-only', 'available': True,
                          **observe(args.root, args.domain)}), flush=True)
        return 0
    except (OSError, sqlite3.Error, RuntimeError, ValueError) as error:
        print(json.dumps({'available': False, 'state': 'UNKNOWN', 'cause': str(error)}), file=sys.stderr, flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
