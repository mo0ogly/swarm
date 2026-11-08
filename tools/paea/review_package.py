#!/usr/bin/env python3
"""G6 review-package assembler: closed, fingerprinted handover inventory for E3.

Lot G6's first object (« Paquet de revue »): the dossier handed to the
designated third parties (H3) when they accept the mission. It assembles the
E3 preparation deliverables that exist in the repository into a closed
manifest — conception, realization, qualification — each entry with its exact
SHA256, published atomically via recipe.py::publish. It proves existence and
byte-identity at assembly time; it proves neither quality, nor acceptance, nor
independent evaluation. A missing or empty deliverable refuses assembly; a
source change during assembly refuses and keeps no half package. The
evaluator's own corpus, preregistration and compared campaigns stay human
gates (H3/H4) — this package never stands in for them.
"""
import hashlib
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

REPORT = 'paea-review-package.json'

# Closed inventory of E3 preparation deliverables. Every path must exist with
# non-empty content; globs expand and refuse if nothing matches.
DELIVERABLES = {
    'conception': [
        'docs/plans/actions-confiance/paea/E3-PROTOCOL.md',
        'docs/plans/actions-confiance/paea/GUIDE-INTEGRATION.md',
        'docs/plans/actions-confiance/paea/PILOTE-OPERATIONNEL.md',
        'docs/plans/actions-confiance/paea/GAP-CLOSURE-PLAN-20261103.md',
        'docs/plans/actions-confiance/paea/DASHBOARD-T20261101.md',
        'docs/plans/actions-confiance/paea/G3-INVENTAIRE-VOIES-20261108.md',
        'docs/plans/actions-confiance/paea/B2-COMPARATOR-20261109.md',
    ],
    'realisation': [
        'tools/paea/pilot_lab.py',
        'tools/paea/pilot_oracle.py',
        'tools/paea/run_pilot.py',
        'tools/paea/verify_pilot.py',
        'tools/paea/lab_evidence.py',
        'tools/paea/make_contract_vectors.py',
        'tools/paea/b2_comparator.py',
        'tests/paea_pilot_lab_test.py',
        'tests/helpers/paea_pilot_contention.py',
        'tests/fixtures/paea/contract-v1.json',
        'tests/fixtures/paea/payment-registry-v1.json',
        'tests/fixtures/paea/payment-request-v1.json',
        'rust/crates/swarm-protection-contract/src/*.rs',
    ],
    'qualification': [
        'tools/verification/recipes/paea-contract.json',
        'tools/verification/recipes/paea-pilot-lab.json',
        'tools/verification/recipes/paea-pilot-contention.json',
        'tools/verification/recipes/paea-laboratory-evidence.json',
        'tools/verification/recipes/paea-b2-comparator.json',
    ],
}


def digest(raw):
    """SHA256 of exact bytes; content identity only, never an author."""
    return hashlib.sha256(raw).hexdigest()


def collect():
    """Expand the closed inventory; refuse any missing or empty deliverable.

    Globs cover the protection-contract sources without pinning a file count
    into this list; a glob matching nothing is the same refusal as a missing
    path — a half inventory is never published.
    """
    entries = []
    for folder, patterns in DELIVERABLES.items():
        for pattern in patterns:
            matches = sorted(p for p in ROOT.glob(pattern) if p.is_file())
            if not matches:
                raise SystemExit('review-package-deliverable-missing: %s' % pattern)
            for path in matches:
                raw = path.read_bytes()
                if not raw.strip():
                    raise SystemExit('review-package-deliverable-empty: %s' % pattern)
                entries.append({
                    'folder': folder,
                    'path': str(path.relative_to(ROOT)),
                    'sha256': digest(raw),
                    'bytes': len(raw),
                })
    return entries


def main():
    """Assemble, then publish the closed manifest only if sources stayed stable.

    A second pass must produce the same digests: any source change during
    assembly (concurrent session, shared tree) refuses with the divergent
    paths listed, and nothing is published. Evidence goes to the directory in
    SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE (created here, must be new) — never
    /tmp.
    """
    evidence = os.environ.get('SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE')
    if not evidence or Path(evidence).exists():
        raise SystemExit('usage: SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE=<new dir> '
                         'review_package.py')
    entries = collect()
    first = {entry['path']: entry['sha256'] for entry in entries}
    repacked = collect()
    second = {entry['path']: entry['sha256'] for entry in repacked}
    diverged = sorted(path for path in first if first[path] != second.get(path))
    if diverged:
        raise SystemExit('review-package-source-changed-during-assembly: %s'
                         % ', '.join(diverged))
    value = {
        'schema_version': 1,
        'package': 'paea-e3-review-package',
        'scope': 'preparation-handover-inventory',
        'folders': {name: sum(1 for entry in entries if entry['folder'] == name)
                    for name in DELIVERABLES},
        'deliverables': entries,
        'counts': {
            'deliverables': len(entries),
            'bytes': sum(entry['bytes'] for entry in entries),
        },
        'honest_labels': {
            'proves': 'existence-and-byte-identity-at-assembly-time',
            'does_not_prove': ['quality', 'acceptance', 'independent-evaluation',
                               'preregistration', 'reserved-corpus'],
            'handover_gates': ['H3-evaluator-and-second-implementation-designation',
                               'H4-business-criteria-and-human-trials'],
        },
    }
    out = Path(evidence)
    out.mkdir()
    spec = __import__('importlib').util.spec_from_file_location(
        'paea_evidence_publisher', ROOT / 'tools/verification/recipe.py')
    module = __import__('importlib').util.module_from_spec(spec)
    spec.loader.exec_module(module)
    destination = out / REPORT
    module.publish(destination, (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode())
    print('PAEA_REVIEW_PACKAGE_PUBLISHED deliverables=%d bytes=%d sha256=%s' % (
        value['counts']['deliverables'], value['counts']['bytes'],
        digest(destination.read_bytes())), flush=True)
    return 0


if __name__ == '__main__':
    sys.exit(main())
