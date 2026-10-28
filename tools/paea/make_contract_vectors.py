#!/usr/bin/env python3
"""Produce PA-01 fixture vectors from the published v1 contract, not Rust output.

Writes only a new destination using the repository's atomic exclusive publisher;
the caller must preserve older vectors/evidence. Python JSON/SHA256 is a local
cross-implementation oracle, not an independent E3 evaluation. No service, account,
policy, secret or effect is read or mutated. Usage: SCRIPT NEW_DESTINATION.
"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True  # Standalone generation must not write helper caches.

ROOT = Path(__file__).resolve().parents[2]


def canonical(value):
    """Encode paea-json-v1 data: sorted ASCII schema keys, ordered arrays and UTF-8.

    Input fixtures contain exact integers and strings, never floats or non-string
    object keys. This does not implement or claim the full RFC8785 number profile.
    """
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def vector(name, action, request='request-1', **context_changes):
    """Build one deterministic fixture with expected bytes and domain business key.

    Context values are explicit test data, never installed policies or identities.
    Request keys are excluded from action identity. No mutable input is retained.
    """
    context = {
        'project_id': 'project-1', 'principal_id': 'requester-1',
        'instance_id': 'instance-1', 'mandate_id': 'mandate-1',
        'mandate_revision': 1, 'policy_revision': 1,
        'policy_digest': 'a' * 64, 'tool_manifest_digest': 'b' * 64,
        'sources': [{'source_id': 'document-1', 'version': 'v1', 'digest': 'c' * 64,
                     'integrity': 0, 'confidentiality_labels': ['restricted', 'internal']}],
    }
    context.update(copy.deepcopy(context_changes))
    normalized = copy.deepcopy(context)
    for source in normalized['sources']:
        source['confidentiality_labels'].sort()
    preimage = canonical({'schema_version': 1, 'canonicalization': 'paea-json-v1',
                          'action': action, 'context': normalized})
    intent = {'project_id': context['project_id'], 'kind': action['kind']}
    if action['kind'] == 'payment.settle':
        intent['invoice_id'] = action['invoice_id']
    else:
        intent.update({k: action[k] for k in ('repository_id', 'target_ref', 'expected_old_oid')})
    return {'name': name, 'proposal': {'schema_version': 1, 'request_id': request, 'action': action},
            'context': context, 'canonical': preimage.decode(),
            'action_digest': hashlib.sha256(preimage).hexdigest(),
            'intent_id': hashlib.sha256(b'paea-intent-v1\n' + canonical(intent)).hexdigest()}


def main():
    """Create v1 vectors in a fresh file; preserve any existing destination.

    Argument/publication failures propagate with nonzero exit. The existing
    evidence publisher fsyncs adjacent bytes and never overwrites past output.
    """
    if len(sys.argv) != 2:
        raise SystemExit('usage: make_contract_vectors.py NEW_DESTINATION')
    payment = {'kind': 'payment.settle', 'invoice_id': 'invoice-17', 'invoice_version': 'v7',
               'amount_minor': '420000', 'currency': 'EUR', 'beneficiary_account_id': 'account-17'}
    git = {'kind': 'repository.promote', 'repository_id': 'repository-1',
           'target_ref': 'refs/heads/candidate', 'expected_old_oid': '1' * 40, 'candidate_oid': '2' * 40}
    vectors = [vector('payment-control', payment), vector('payment-new-request', payment, 'request-2'),
               vector('payment-changed-amount', {**payment, 'amount_minor': '420001'}),
               vector('payment-changed-account', {**payment, 'beneficiary_account_id': 'account-18'}),
               vector('payment-new-invoice-version', {**payment, 'invoice_version': 'v8'}),
               vector('payment-new-policy', payment, policy_revision=2, policy_digest='d' * 64),
               vector('payment-other-project', payment, project_id='project-2'),
               vector('payment-large-exact', {**payment, 'amount_minor': '9223372036854775807'}),
               vector('git-control', git), vector('git-new-request', git, 'request-2'),
               vector('git-changed-candidate', {**git, 'candidate_oid': '3' * 40}),
               vector('git-new-base', {**git, 'expected_old_oid': '4' * 40}),
               vector('git-unicode-ref', {**git, 'target_ref': 'refs/heads/déploiement'}),
               vector('git-sha256', {**git, 'expected_old_oid': '1' * 64, 'candidate_oid': '2' * 64})]
    spec = importlib.util.spec_from_file_location('paea_evidence_publisher', ROOT / 'tools/verification/recipe.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    destination = Path(sys.argv[1])
    destination.parent.mkdir(parents=True, exist_ok=True)
    module.publish(destination, {'schema_version': 1, 'vectors': vectors})
    print(f'PAEA_VECTORS_CREATED {len(vectors)}')


if __name__ == '__main__':
    main()
