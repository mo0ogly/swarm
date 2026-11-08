#!/usr/bin/env python3
"""Verify the retained BLP source-flow inventory without executing any flow.

Checks only declared source hashes/anchors and the inventory's explicit graph.
It discovers no arbitrary hidden sink, proves no compiler graph, authenticates
no principal and grants no runtime operation. Drift requires semantic review;
there is deliberately no automatic hash-refresh or permissive fallback mode.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re


def _object(pairs):
    """Reject ambiguous repeated JSON keys before any dictionary erases them."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('blp-inventory-duplicate-key')
        result[key] = value
    return result


def inspect(root, document):
    """Return safe named failures for declared source/flow contracts, read-only.

    root must identify the reviewed checkout; document is untrusted inventory
    data, never policy. Require unique references/flows, current complete source
    hashes and exact anchor lines, explicit owners and open BLP follow-ups. Source
    aliases escaping root refuse. IO errors retain their cause to the caller;
    no file, setting, process, network or runtime authority is changed. Success
    establishes inventory freshness only, never completeness or enforcement.
    """
    failures = []
    if not isinstance(document, dict) or set(document) != {
        'schema_version', 'scope', 'runtime_enforced', 'references', 'flows',
        'exclusions', 'remaining_evidence',
    } or type(document.get('schema_version')) is not int or document['schema_version'] != 1 \
            or document.get('runtime_enforced') is not False:
        return ['blp-inventory-shape']
    if not isinstance(document['scope'], str) or not document['scope'].strip():
        failures.append('blp-inventory-scope-required')
    for name in ('exclusions', 'remaining_evidence'):
        if not isinstance(document[name], list) or not document[name] \
                or any(not isinstance(value, str) or not value.strip() for value in document[name]):
            failures.append('blp-inventory-scope-required')
    refs = document['references']
    flows = document['flows']
    if not isinstance(refs, list) or not refs or not isinstance(flows, list) or not flows:
        return failures + ['blp-inventory-empty']
    root = Path(root).resolve(strict=True)
    identifiers = set()
    for ref in refs:
        if not isinstance(ref, dict) or set(ref) != {'id', 'path', 'sha256', 'line', 'anchor'}:
            failures.append('blp-inventory-reference-shape'); continue
        key = ref['id']
        if not isinstance(key, str) or not re.fullmatch(r'[A-Za-z0-9_-]+', key) or key in identifiers:
            failures.append('blp-inventory-reference-id'); continue
        identifiers.add(key)
        name = ref['path']
        if not isinstance(name, str) or not name or '\\' in name or '\0' in name \
                or Path(name).is_absolute() or any(part in ('', '.', '..') for part in name.split('/')):
            failures.append(key + ':path-invalid'); continue
        path = (root / name).resolve(strict=True)
        if not path.is_relative_to(root):
            failures.append(key + ':path-outside-root'); continue
        raw = path.read_bytes()
        if not isinstance(ref['sha256'], str) or not re.fullmatch(r'[0-9a-f]{64}', ref['sha256']) \
                or hashlib.sha256(raw).hexdigest() != ref['sha256']:
            failures.append(key + ':source-changed')
        lines = raw.decode('utf-8').splitlines()
        number, anchor = ref['line'], ref['anchor']
        if type(number) is not int or number < 1 or number > len(lines) \
                or not isinstance(anchor, str) or not anchor or anchor not in lines[number-1]:
            failures.append(key + ':anchor-changed')
    flow_ids, used = set(), set()
    for flow in flows:
        if not isinstance(flow, dict) or set(flow) != {
            'id', 'name_fr', 'name_en', 'producer', 'consumer', 'references', 'material',
            'current_controls', 'missing_blp', 'owner', 'required_observable', 'status',
        }:
            failures.append('blp-inventory-flow-shape'); continue
        key = flow['id']
        if not isinstance(key, str) or not re.fullmatch(r'BF-[0-9]+', key) or key in flow_ids:
            failures.append('blp-inventory-flow-id'); continue
        flow_ids.add(key)
        if flow['status'] != 'OPEN' or not isinstance(flow['references'], list) or not flow['references'] \
                or any(not isinstance(value, str) or value not in identifiers for value in flow['references']):
            failures.append(key + ':unreviewed-boundary'); continue
        used.update(flow['references'])
        if any(not isinstance(flow[field], str) or not flow[field].strip() for field in ('name_fr', 'name_en', 'producer', 'consumer',
                                            'material', 'current_controls', 'owner', 'required_observable')) \
                or not isinstance(flow['missing_blp'], list) or not flow['missing_blp'] \
                or any(value not in {f'BLP-0{i}' for i in range(1, 8)} for value in flow['missing_blp']):
            failures.append(key + ':contract-incomplete')
    if used != identifiers:
        failures.append('blp-inventory-unused-reference')
    return failures


def main():
    """Read one strict inventory, report freshness and exit 0/1/2, without writes.

    0 means declared contracts are structurally current, 1 means a stale/invalid
    declaration, 2 means input/IO failed. No runtime or compliance PASS is emitted.
    inspect propagates original IO causes to library callers; CLI output reports
    only the failure category and cause type, without source contents or paths.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--inventory', type=Path, required=True)
    args = parser.parse_args()
    try:
        document = json.loads(args.inventory.read_text(), object_pairs_hook=_object)
        failures = inspect(args.root, document)
    except (OSError, ValueError, TypeError) as cause:
        print(json.dumps({'scope': 'declared-source-inventory-only', 'runtime_enforced': False,
                          'failure': 'blp-inventory-input-failed', 'cause_type': type(cause).__name__}))
        return 2
    refs = document.get('references') if isinstance(document, dict) else None
    flows = document.get('flows') if isinstance(document, dict) else None
    print(json.dumps({'scope': 'declared-source-inventory-only', 'runtime_enforced': False,
                      'failures': failures, 'reference_count': len(refs) if isinstance(refs, list) else None,
                      'flow_count': len(flows) if isinstance(flows, list) else None}))
    if failures:
        return 1
    print('BLP_INVENTORY_CURRENT declared anchors and source hashes only; runtime protection not established')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
