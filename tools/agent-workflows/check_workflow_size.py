#!/usr/bin/env python3
"""Check that compiled workflow sources fit the shipped initial operator policy.

Read canonical descriptor/settings and referenced UTF-8 method bytes only. This
preflight mirrors assemble_agent_workflow's byte framing; it does not establish
Rust execution, model capacity or receipt of conditional references. Runtime
operators may save smaller caps and receive the existing explicit refusal.
"""
from pathlib import Path
import argparse
import json
import sys


def inspect(root):
    """Return role body sizes and saved initial cap; malformed inputs raise.

    No write, provider, compiler or Root datastore is opened. Resolve source paths
    inside the selected source tree; unknown schemas/roles or policy types refuse.
    None keeps the existing unlimited meaning; zero refuses every nonempty body.
    """
    root = Path(root).resolve(strict=True)
    config = root / 'rust/crates/swarm-engine/config'
    descriptor = json.loads((config / 'agent-workflow-descriptor.json').read_text())
    settings = json.loads((config / 'engine-settings.json').read_text())
    if type(descriptor['schema_version']) is not int or descriptor['schema_version'] != 1 or set(descriptor['roles']) != {'planner', 'subplanner', 'worker', 'reviewer'}:
        raise ValueError('workflow-descriptor-invalid')
    raw_cap = settings['agent_workflow_max_body_bytes']
    if raw_cap is not None and (not isinstance(raw_cap, str) or not raw_cap.isascii() or not raw_cap.isdecimal()):
        raise ValueError('workflow-initial-policy-invalid')
    cap = None if raw_cap is None else int(raw_cap)
    sizes = {}
    for role, value in descriptor['roles'].items():
        size = len(value['prefix'].encode()) + len(value['suffix'].encode())
        for name in value['paths']:
            source = (root / name).resolve(strict=True)
            if not source.is_relative_to(root):
                raise ValueError('workflow-source-outside-tree')
            raw = source.read_bytes()
            raw.decode('utf-8')
            size += len(b'\nSOURCE ') + len(name.encode()) + 2 + len(raw)
        sizes[role] = size
    return {'initial_cap_bytes': cap, 'role_body_bytes': sizes,
            'fits': all(cap is None or size <= cap for size in sizes.values())}


def main():
    """Print sizes only and return nonzero for unreadable/invalid/oversized inputs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    try:
        result = inspect(args.root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(json.dumps({'status': 'ERROR', 'cause_type': type(error).__name__}), file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0 if result['fits'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
