#!/usr/bin/env python3
"""Generate versioned fictional PAEA downloads from retained contract fixtures.

No Root, provider, approval or effect is opened. --check compares all eight exact
outputs without writing. Generation replaces each source asset atomically; an IO
failure propagates and leaves that file's previous bytes intact. The output set
is not a multi-file transaction; run --check after a failed/interrupted generation.
"""
import argparse
import copy
import json
import os
from pathlib import Path
import tempfile

ROOT = Path(__file__).resolve().parents[2]
DESTINATION = ROOT / 'web/paea-examples/v1'


def examples():
    """Return eight deterministic JSON byte sets from named fictional v1 vectors.

    Existing precomputed identities bind each receipt to its unchanged document;
    native receipt/comparison journeys must verify them. Corrected terms preserve
    the business intent and request key. Missing/changed fixture shape raises;
    these unsigned files confer no authority and contain no runtime data.
    """
    vectors = json.loads((ROOT / 'tests/fixtures/paea/contract-v1.json').read_text())
    assert vectors['schema_version'] == 1
    result = {}
    for prefix, name in [('payment', 'payment-control'), ('git', 'git-control')]:
        matches = [v for v in vectors['vectors'] if v['name'] == name]
        assert len(matches) == 1
        vector = matches[0]
        document = copy.deepcopy(dict(proposal=vector['proposal'], context=vector['context']))
        result[prefix + '-document.json'] = document
        result[prefix + '-receipt.json'] = dict(
            schema_version=1, action_digest=vector['action_digest'],
            intent_id=vector['intent_id'], sink_id='sink-paea-example',
            outcome='COMMITTED', observation_digest='d' * 64)
        after = copy.deepcopy(document)
        action = after['proposal']['action']
        if prefix == 'payment':
            action['amount_minor'] = '420001'
            action['beneficiary_account_id'] = 'account-example-corrected'
        else:
            action['candidate_oid'] = next(
                c * len(action['candidate_oid']) for c in ['e', 'd', 'c']
                if c * len(action['candidate_oid']) not in
                [action['candidate_oid'], action['expected_old_oid']])
        result[prefix + '-after.json'] = after
    for kind in ['registry', 'request']:
        result['payment-' + kind + '.json'] = json.loads(
            (ROOT / ('tests/fixtures/paea/payment-' + kind + '-v1.json')).read_text())
    return {name: (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()
            for name, value in result.items()}


def publish(path, content):
    """Fsync an adjacent temporary file before replacing one generated asset.

    Parent must exist. Write/replace failures propagate, remove only this owned
    temporary file and preserve the original until replacement succeeds.
    """
    fd, temporary = tempfile.mkstemp(prefix='.' + path.name + '.', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main():
    """Check or regenerate the fixed versioned asset set; report exact filenames.

    Unsupported arguments and missing/stale files fail visibly; no silent fallback
    or deletion of extra files. Success is generation consistency, not protection.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = examples()
    if not args.check:
        DESTINATION.mkdir(parents=True, exist_ok=True)
    for name, content in expected.items():
        target = DESTINATION / name
        if args.check:
            if target.read_bytes() != content:
                raise ValueError('stale generated PAEA example: ' + name)
        else:
            publish(target, content)
        print(('CHECK ' if args.check else 'WRITE ') + name)


if __name__ == '__main__':
    main()
