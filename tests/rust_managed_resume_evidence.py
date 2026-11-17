#!/usr/bin/env python3
"""Read closed managed-review evidence before preparing a continuation.

This module never opens a Root database, starts a child, changes settings or
authorizes a quota. Its result describes historical files only. In particular,
valid evidence is neither permission to execute nor a current-image witness.
The owning coordinator must separately obtain budget authority, inspect current
public state, review the continuation driver and retain ownership of its children.
"""
import hashlib
import json
from pathlib import Path


class EvidenceError(ValueError):
    """A closed-evidence precondition failed; no runtime action was attempted."""


def require(condition, code):
    """Refuse a missing evidence condition with a stable, non-private diagnostic."""
    if not condition:
        raise EvidenceError(code)


def read_regular(path):
    """Read a regular non-symlink leaf; return bytes, propagating IO failures.

    This is an offline consistency check, not same-UID filesystem confinement.
    Ancestor replacement and hostile concurrent writers are outside its scope.
    """
    path = Path(path)
    require(not path.is_symlink() and path.is_file(), 'evidence-regular-leaf-required')
    return path.read_bytes()


def digest(raw):
    """Return the SHA-256 of the supplied bytes without interpreting them."""
    return hashlib.sha256(raw).hexdigest()


def pinned_json(path, sha256):
    """Read a caller-pinned JSON original; hash before decoding, with no fallback."""
    raw = read_regular(path)
    require(digest(raw) == sha256, 'evidence-original-pin-drift')
    return json.loads(raw)


def child_file(directory, name):
    """Resolve a relative evidence key within its declared directory.

    Reject absolute paths, traversal and symlink escapes. This does not make
    concurrent path replacement safe or admit paths for a runtime operation.
    """
    relative = Path(name)
    require(not relative.is_absolute() and relative.parts and '..' not in relative.parts,
            'evidence-relative-key-required')
    root = Path(directory).resolve(strict=True)
    path = root / relative
    require(path.resolve(strict=True).is_relative_to(root), 'evidence-path-escape')
    return path


def closed_failure(manifest_path, manifest_sha256):
    """Verify a pinned closed-run inventory and decode its actual CLI schemas.

    Return the manifest, pre-call Work, post-refusal Work and saved settings.
    Verify all declared bytes; preserve the consumed failing planner activation.
    Financial ledger rows are not native lifecycle phases. No Root is inspected.
    """
    manifest = pinned_json(manifest_path, manifest_sha256)
    require(manifest['schema_version'] == 1 and bool(manifest['files']),
            'evidence-failed-manifest-required')
    directory = Path(manifest['source_run_directory'])
    for name, record in manifest['files'].items():
        raw = read_regular(child_file(directory, name))
        require(len(raw) == record['bytes'] and digest(raw) == record['sha256'],
                'evidence-failed-run-drift')

    def load(name):
        """Decode only an inventoried original after its complete hash check."""
        require(name in manifest['files'], 'evidence-original-not-in-manifest')
        return json.loads(read_regular(child_file(directory, name)))

    before = load('work-original.json')['work']
    failed = load('failure-read-W/stdout.raw')['work']
    require(before['id'] == failed['id'] == manifest['work'], 'evidence-work-binding')
    require(before['tasks'] == failed['tasks'], 'evidence-failed-task-controls-drift')
    require(failed['planning']['reviewer'] == before['planning']['reviewer'],
            'evidence-failed-reviewer-policy-drift')
    require(failed['planning']['activations'] == before['planning']['activations'] + 1
            and failed['planning']['decisions'] == before['planning']['decisions'],
            'evidence-consumed-refused-planner-required')
    require(failed['revision'] > before['revision'], 'evidence-refusal-revision-required')
    commands = [name for name in ('W-before/command.json', 'W-before-retry-step/command.json')
                if name in manifest['files']]
    require(bool(commands), 'evidence-root-command-required')
    for name in commands:
        argv = load(name)['argv']
        require(argv.count('--root') == 1 and '--root' in argv[:-1],
                'evidence-root-argument-required')
        require(argv[argv.index('--root') + 1] == manifest['root'], 'evidence-root-binding')
    failure = load('W-retry-planning-step/stderr.raw')['failure']
    require(failure['code'] == 'work-state-preserved'
            and all(isinstance(cause, str) for cause in failure['causes'])
            and 'planning-decision-managed-operation-unported' in failure['causes'],
            'evidence-dispatcher-refusal-required')
    negative = load('W-retry-refusal-inputs/stderr.raw')['failure']
    require(negative['code'] == 'native-retry-decision-required'
            or 'native-retry-decision-required' in negative['causes'],
            'evidence-canonical-correction-refusal-required')
    require('W-retry-refusal-inputs/stdout.raw' in manifest['files'],
            'evidence-negative-stdout-not-in-manifest')
    require(read_regular(child_file(directory, 'W-retry-refusal-inputs/stdout.raw')) == b'',
            'evidence-negative-preview-stdout-required')
    native = load('planning-runs-readonly.json')
    refused = [row for row in native if row[1] == 'refused']
    require(len(refused) == 1
            and refused[0][2] == 'planning-decision-managed-operation-unported'
            and refused[0][5] is None, 'evidence-native-terminal-refusal-required')
    financial = load('failure-ledger-W.json')['planning_calls']
    matching = [row for row in financial if row[0] == refused[0][0]]
    require(len(matching) == 1 and matching[0][3] == 'estimated',
            'evidence-financial-row-is-not-native-phase')
    settings = load('engine-settings.json')
    require('values' not in settings and 'storage_busy_timeout_ms' in settings,
            'evidence-saved-settings-are-values')
    return manifest, before, failed, settings


def healthy_witness(receipt_path, receipt_sha256, checkout):
    """Validate historical witness files and unchanged helper bytes.

    Require its explicit named review result and actual owned-child closure
    fields. This verifies retained evidence, never reruns the healthy scenario or
    converts the old-image result into proof about a later binary.
    """
    import rust_managed_review_w as base
    receipt = pinned_json(receipt_path, receipt_sha256)
    require(type(receipt['reviewer_calls_before']) is int
            and type(receipt['reviewer_calls_after']) is int,
            'evidence-witness-integer-call-count')
    require(receipt['status'] == base.WITNESS_STATUS
            and receipt['receipt_schema'] == base.WITNESS_SCHEMA
            and receipt['reviewer_calls_before'] == 0
            and receipt['reviewer_calls_after'] == 1
            and receipt['task_accepted'] is False, 'evidence-healthy-witness-required')
    directory = Path(receipt_path).parent.resolve(strict=True)
    require({'work-final.json', 'helper-inputs.json', 'engine-settings.json',
             'original.frame', 'certificate.json', 'invocation.json',
             'inert-provider.py', 'retention.json', 'managed-context.json'}
            <= receipt['output_proofs'].keys(), 'evidence-witness-originals-required')
    for record in receipt['output_proofs'].values():
        path = Path(record['path'])
        require(path.resolve(strict=True).is_relative_to(directory), 'evidence-witness-proof-escape')
        raw = read_regular(path)
        require(digest(raw) == record['sha256'] and len(raw) == record['bytes'],
                'evidence-witness-proof-drift')
    require(bool(receipt['process_waits']), 'evidence-witness-owned-children-required')
    for child in receipt['process_waits']:
        require(child['pending'] == [] and child['returncode'] == 0
                and child['reap_confirmed'] is True and child['wait_validated'] is True
                and child['capture_closed_observed'] is True
                and child['pidfd_held'] is False and child['streams_held'] is False,
                'evidence-witness-child-not-settled')
    require(bool(receipt['helper_inputs']), 'evidence-witness-helpers-required')
    for name in ('work-final.json', 'helper-inputs.json'):
        require(Path(receipt['output_proofs'][name]['path']) == directory / name,
                'evidence-witness-named-original-binding')
    require(json.loads(read_regular(directory / 'helper-inputs.json')) == receipt['helper_inputs'],
            'evidence-helper-map-binding')
    checkout = Path(checkout).resolve(strict=True)
    for name, sha256 in receipt['helper_inputs'].items():
        path = Path(name)
        require(path.resolve(strict=True).is_relative_to(checkout), 'evidence-helper-outside-checkout')
        require(digest(read_regular(path)) == sha256, 'evidence-witness-helper-drift')
    final = json.loads(read_regular(directory / 'work-final.json'))
    require(receipt['output_proofs']['work-final.json'] == receipt['work_final'],
            'evidence-witness-final-work-binding')
    require(type(final['planning']['reviewer']['calls']) is int,
            'evidence-witness-integer-call-count')
    require(final['planning']['reviewer']['calls'] == receipt['reviewer_calls_after'],
            'evidence-witness-final-call-count')
    task = next(task for task in final['tasks'] if task['id'] == receipt['task'])
    require(final['id'] == receipt['work'] and task['status'] == 'submitted'
            and task['independent_review']['state'] == 'passed'
            and task['independent_review']['id'] == receipt['review_id'],
            'evidence-witness-passed-submitted-required')
    require(base.semantics(final, receipt['task']) == receipt['semantic_preconditions'],
            'evidence-witness-semantics-drift')
    return receipt


def profile_bridge(old_profile, new_profile, bridge_path, bridge_sha256):
    """Compare actual retained compiler maps with a caller-pinned change report.

    Only the declared dispatcher source may differ. This comparison is a narrow
    reuse boundary, not qualification of either executable or runtime success.
    The actual execution driver must also call the existing profile qualifier.
    """
    bridge = pinned_json(bridge_path, bridge_sha256)
    old_profile, new_profile = Path(old_profile), Path(new_profile)
    require(bridge['old_profile'] == str(old_profile) and bridge['new_profile'] == str(new_profile),
            'evidence-profile-path-binding')
    profiles = [json.loads(read_regular(path)) for path in (old_profile, new_profile)]
    for profile in profiles:
        require(profile['status'] == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
                and all(profile[key] is True for key in
                        ('all_core_features_empty', 'all_core_test_false',
                         'no_fixture_feature', 'actual_dependencies_stable')),
                'evidence-default-profile-required')
    inventories = [json.loads(read_regular(path.parent / 'after.json'))
                   for path in (old_profile, new_profile)]
    old, new = inventories
    require(old.keys() == new.keys() and len(new) == bridge['effective_inputs'],
            'evidence-compiler-map-keys-drift')
    changes = {name: {'before': old[name], 'after': new[name]}
               for name in old if old[name] != new[name]}
    require(set(changes) == {'rust/crates/swarm-engine/src/planning_decision.rs'}
            and changes == bridge['changed_sources'], 'evidence-dispatcher-only-bridge-required')
    require(profiles[0]['sha256'] == bridge['old_binary']
            and profiles[1]['sha256'] == bridge['new_binary'], 'evidence-profile-image-binding')
    return bridge


def validate_closed_inputs(manifest_path, manifest_sha256, receipt_path,
                           receipt_sha256, checkout, old_profile, new_profile,
                           bridge_path, bridge_sha256):
    """Bind all historical originals and return a non-authorizing summary.

    Requires caller-pinned manifests/receipt/change report and an unchanged
    historical helper closure. Reads regular proof/source files only. Rejects
    malformed, drifted or incompatible evidence before any runtime operation.
    Errors propagate with their causes; no fallback result is returned. Quota
    approval, current Root state, qualified current executable, source review and
    a supervised continuation remain separate requirements, even on success.
    """
    import rust_managed_review_w as base
    manifest, before, failed, settings = closed_failure(manifest_path, manifest_sha256)
    witness = healthy_witness(receipt_path, receipt_sha256, checkout)
    bridge = profile_bridge(old_profile, new_profile, bridge_path, bridge_sha256)
    require(digest(read_regular(old_profile)) == witness['normal_profile_sha256']
            and bridge['old_binary'] == witness['normal_sha256'], 'evidence-witness-profile-binding')
    require(base.semantics(failed, manifest['task']) == witness['semantic_preconditions'],
            'evidence-retry-witness-semantics-mismatch')
    task = next(task for task in failed['tasks'] if task['id'] == manifest['task'])
    review = task['independent_review']
    require(review['state'] == 'error' and task['status'] == 'submitted',
            'evidence-original-failed-review-required')
    require(failed['planning']['activations'] == failed['planning']['max_activations'],
            'evidence-planner-capacity-exhausted')
    return {'status': 'CLOSED_EVIDENCE_CONSISTENT_NOT_EXECUTION_AUTHORITY',
            'root': manifest['root'], 'work': failed['id'], 'task': manifest['task'],
            'revision_before_refusal': before['revision'], 'revision_after_refusal': failed['revision'],
            'prior_failed_review': review['id'], 'closed_files': len(manifest['files']),
            'witness_proofs': len(witness['output_proofs']),
            'unchanged_historical_helpers': len(witness['helper_inputs']),
            'compiler_inputs': bridge['effective_inputs'], 'changed_sources': bridge['changed_sources'],
            'quota_authorized': False, 'runtime_executed': False,
            'scope': 'Historical originals only; no current Root observation, new-image witness or acceptance'}
