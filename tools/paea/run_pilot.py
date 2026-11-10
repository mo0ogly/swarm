#!/usr/bin/env python3
"""Run the existing isolated PAEA laboratory and export a portable reading report.

Explicit pinned binary and fresh output only; never build, contact providers,
use a live Root, retry or grant authority. Python/Git and repository sources are
required on the operator's machine; the native web server never launches this.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tools.verification.recipe import run_recipe, verify_evidence, publish

# Keep historical imports of CASES compatible with the v1 report producer.
from tools.paea.lab_evidence import CASES, INVENTORY, collect


class PublicationUnknown(RuntimeError):
    """Artifact publication failed with potentially complete files; preserve originals.

    The expected inventory digest is known before either exclusive publication.
    It permits checking retained files after failure, not author authentication.
    """
    def __init__(self, inventory_sha256):
        """Retain the expected digest and a safe static diagnostic; cause is chained."""
        super().__init__('laboratory-report-publication-uncertain')
        self.inventory_sha256 = inventory_sha256


def digest(raw):
    """Return a local content hash; it authenticates neither author nor authority."""
    return hashlib.sha256(raw).hexdigest()


def export_report(evidence, observations, binary_hash):
    """Verify retained PASS evidence before publishing a closed, non-authorising report.

    Require all eight named tests and nonempty captured calls. Hash the exact
    retained recipe result, source inventory and call index, without leaking local
    paths into the portable report. Invalid evidence refuses before publication.
    Integrity is local and never establishes independence. Recorded v2 call artifacts
    must exist with exact hashes before the inventory/report are published. The
    inventory precedes the report. IO failure may leave an incomplete OR fully
    published bundle: PublicationUnknown retains its expected digest and cause.
    A caller must verify retained bytes; no signature or durability is established.
    """
    if not re.fullmatch('[a-f0-9]{64}', binary_hash):
        raise ValueError('laboratory-binary-hash-invalid')
    verified = verify_evidence(evidence)
    if verified != {'integrity': 'VERIFIED', 'recorded_status': 'PASS', 'recipe': 'paea-pilot-lab'}:
        raise ValueError('laboratory-evidence-not-passing')
    manifest = json.loads((evidence / 'recipe.original.json').read_bytes())
    steps = manifest['steps']
    if len(steps) != 1 or not set(CASES).issubset(steps[0]['required_stdout']):
        raise ValueError('laboratory-case-contract-mismatch')
    calls_raw = (observations / 'calls.json').read_bytes()
    calls = json.loads(calls_raw)
    if not isinstance(calls, list) or not calls:
        raise ValueError('laboratory-calls-missing')
    value = dict(schema_version=1, scope='isolated-laboratory', recipe_id='paea-pilot-lab',
        binary_sha256=binary_hash,
        recipe_result_sha256=digest((evidence / 'result.json').read_bytes()),
        source_inventory_sha256=digest((evidence / 'inputs.before.json').read_bytes()),
        calls_sha256=digest(calls_raw),
        tests=[dict(id=name, status='PASS') for name in CASES],
        refuted_claim='raw-invoice-identifiers-guarantee-global-debt-uniqueness',
        authority='simulated', isolation='same-uid', independent_evaluation='not-established',
        protected_invariants='not-established')
    raw = (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()
    if observations.resolve() != (evidence.parent / 'observations').resolve():
        raise ValueError('laboratory-observation-root-mismatch')
    inventory = collect(evidence.parent, raw)
    inventory_raw = (json.dumps(inventory, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()
    try:
        publish(evidence.parent / INVENTORY, inventory_raw)
        publish(evidence.parent / 'paea-lab-report.json', raw)
    except OSError as error:
        raise PublicationUnknown(digest(inventory_raw)) from error
    return value


def main():
    """Validate explicit inputs, run once, verify, then export; errors exit nonzero.

    Output must be a new directory outside live .swarm/.git state and outside the
    source trees. Existing files remain untouched. Partial failures retain their
    evidence. Artifact publication UNKNOWN requires checking the retained bundle
    with its expected digest; correct prerequisites before a fresh execution.
    Environment changes affect this command and its children only.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--sha256', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        binary = args.binary.resolve(strict=True)
        if not re.fullmatch('[a-f0-9]{64}', args.sha256) or digest(binary.read_bytes()) != args.sha256:
            raise ValueError('laboratory-binary-hash-mismatch')
        output = args.output.resolve()
        if output.exists() or any(output.is_relative_to(ROOT / name) for name in ('.swarm', '.git', 'rust', 'web', 'tools', 'tests')):
            raise ValueError('laboratory-output-must-be-new-outside-runtime-and-sources')
        output.mkdir(mode=0o700)
        os.environ.update(SWARM_PAEA_TEST_BINARY=str(binary), SWARM_PAEA_TEST_BINARY_SHA256=args.sha256,
                          SWARM_PAEA_PILOT_EVIDENCE=str(output / 'observations'), PYTHONDONTWRITEBYTECODE='1')
        result = run_recipe(ROOT, ROOT / 'tools/verification/recipes/paea-pilot-lab.json', output / 'recipe')
        if result['status'] != 'PASS':
            raise ValueError('laboratory-recipe-failed-see-retained-evidence')
        if digest(binary.read_bytes()) != args.sha256:
            raise ValueError('laboratory-binary-changed')
        report = export_report(output / 'recipe', output / 'observations', args.sha256)
        print(json.dumps({'status': 'PASS', 'tests': len(report['tests']), 'report': str(output / 'paea-lab-report.json'),
                          'integrity': 'VERIFIED', 'protected_invariants': 'not-established',
                          'evidence_inventory': str(output / INVENTORY),
                          'evidence_inventory_sha256': digest((output / INVENTORY).read_bytes())}))
        return 0
    except PublicationUnknown as error:
        print(json.dumps({'status': 'UNKNOWN', 'scope': 'laboratory-report-publication',
                          'cause': str(error), 'expected_inventory_sha256': error.inventory_sha256,
                          'next': 'verify-retained-bundle-before-new-execution'}), file=sys.stderr, flush=True)
        return 2
    except Exception as error:
        print(json.dumps({'status': 'REFUSED', 'cause': str(error), 'type': type(error).__name__}), file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
