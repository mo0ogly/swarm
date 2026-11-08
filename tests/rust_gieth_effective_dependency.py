#!/usr/bin/env python3
"""Reproduce review reuse after an untracked effective configuration changes.

Reuse actual managed worker/control/reviewer/publication fixtures and a previously
qualified normal binary, after comparing its entire retained effective inventory.
No production edit, SQL write, paid model, live mission or global watchdog.
The reviewer is an inert protocol fixture, not an independent security auditor.
Exit 1 preserves a demonstrated security-contract failure, never a green result.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import traceback

import rust_native_first_publication as publication
from rust_native_task_validation import invoke

native = publication.native
inspection = native.inspection
fixture = inspection.native.fixture

CONSUMER = b'''"""Read the effective local policy and check that fixture-B is denied.

Input: an owned untracked JSON configuration in this program's working directory.
Output: the actual policy hash and decision on stdout; exit 42 if B is allowed.
No transmission or business mutation; this is an isolated decision consumer.
"""
from pathlib import Path
import hashlib,json,sys
raw=Path('effective-security.json').read_bytes()
policy=json.loads(raw)
allowed='fixture-B' in policy['allowed_targets']
print(json.dumps(dict(policy_sha256=hashlib.sha256(raw).hexdigest(),
    target='fixture-B',decision='allow' if allowed else 'deny')),flush=True)
raise SystemExit(42 if allowed else 0)
'''
V1 = b'{"allowed_targets":["fixture-A"]}\n'
V2 = b'{"allowed_targets":["fixture-A","fixture-B"]}\n'


def digest(raw):
    """Return SHA-256 of complete observed bytes, without asserting authority."""
    return hashlib.sha256(raw).hexdigest()


def qualify(binary, profile, out):
    """Retain normal-profile reuse checks before invoking any product process.

    Compare every effective source, checkout Cargo configuration and exact binary
    to the original qualification. Preserve profile/inventory bytes and hashes;
    no build, installation or independent authentication of the compiler occurs.
    Missing/drifting inputs raise before fixture execution.
    """
    proof = json.loads(profile.read_bytes())
    inventory = json.loads((profile.parent / 'after.json').read_bytes())
    assert proof['status'] == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
    assert proof['no_fixture_feature'] and proof['all_core_features_empty'] and proof['all_core_test_false']
    assert digest(binary.read_bytes()) == proof['sha256']
    assert all(Path(name).is_file() and digest(Path(name).read_bytes()) == sha
               for name, sha in inventory.items()), 'effective build inputs changed'
    for name, sha in proof['cargo_config_inputs'].items():
        assert digest(Path(name).read_bytes()) == sha
    fixture.publish(out / 'profile.original.json', profile.read_bytes())
    fixture.publish(out / 'effective-inputs.original.json', (profile.parent / 'after.json').read_bytes())
    result = dict(binary=str(binary), binary_sha256=proof['sha256'],
                  profile=str(profile), effective_inputs=len(inventory),
                  all_effective_inputs_equal=True, no_fixture_feature=True,
                  global_watchdog=None)
    fixture.publish(out / 'profile-reuse.json', result)
    return inventory


def call(binary, root, out, label, args, request=None):
    """Use the real CLI and retain exact argv/channels; reject unexpected errors.

    CODEX_HOME selects this owned fixture only. Public refusal is returned for
    explicit diagnosis by the caller; it never becomes empty success or a retry.
    """
    return invoke(binary, root, out, label, args, request,
                  dict(os.environ, CODEX_HOME=str(root / 'codex-home')))


def shown(binary, root, out, label, work):
    """Require an actual successful public Work read; no fabricated fallback."""
    code, value = call(binary, root, out, label, ['work', 'show', work])
    assert code == 0, (label, value)
    return value['work']


def prepared_v1(binary, profile, out):
    """Create public worker/control originals with a real untracked dependency.

    Commit the consumer before worker admission; persist three controls whose
    declared inputs deliberately omit the effective policy. After candidate
    construction create only that untracked policy. Native controls must read V1
    and pass, with their exact outputs retained. Reuse the maintained inspector
    and native inert reviewer, including their existing failure/restoration cases.
    No private Root record is seeded or modified by this helper.
    """
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    controls = [dict(id='security-' + name,
                     command=['python3', 'security-consumer.py'], criteria=[1],
                     inputs=['tracked.txt', 'security-consumer.py'],
                     justification='Observe actual effective policy; fixture-B must be denied',
                     review_output=True) for name in ['first', 'second', 'third']]
    entries = {'whole-source.txt': b'Complete extra tracked source for candidate inspection.\n',
               'literal-link': ('symlink', '../../do-not-read-link-target'),
               'security-consumer.py': CONSUMER}
    managed = inspection.native.managed
    managed.worker.subreaper_scope(
        lambda: managed.run(binary, profile, out / 'workers', project_subdir='app',
                            controls=controls, source_entries=entries),
        lambda error: None, managed.parent_death.subreaper)
    birth = json.loads((out / 'workers/fixture.json').read_bytes())
    root = Path(birth['root'])
    work = birth['work']
    state = shown(binary, root, out, 'setup-work', work)
    code, mission = call(binary, root, out, 'setup-mission', ['mission', 'show', work])
    assert code == 0
    for kind, paused in [('mission', mission['paused']),
                         ('planning', state['planning'].get('paused', False))]:
        if paused:
            state = shown(binary, root, out, kind + '-before', work)
            code, value = call(binary, root, out, kind + '-resume', [kind, 'resume', work],
                               dict(schema_version=1, event_id='gi-' + kind + '-resume',
                                    expected_revision=state['revision'],
                                    reason='Explicit isolated Gi.eth dependency witness'))
            assert code == 0, value
    state = shown(binary, root, out, 'candidate-before', work)
    code, value = call(binary, root, out, 'candidate', ['task', 'validation-candidate', work],
                       dict(schema_version=1, event_id='gi-candidate',
                            expected_revision=state['revision'], task_id='task'))
    assert code == 0 and value['control_count'] == 3, value
    storage = Path(state['planning']['repository']['storage'])
    candidates = [json.loads(p.read_bytes()) for p in (storage / 'candidates').glob('*/candidate.json')]
    checkpoint = next(c for c in candidates if c['task'] == 'task')
    workspace = Path(checkpoint['workspace'])
    assert 'app/effective-security.json' not in checkpoint['tracked']
    fixture.publish(workspace / 'effective-security.json', V1)
    state = shown(binary, root, out, 'controls-before', work)
    code, value = call(binary, root, out, 'controls', ['task', 'validate', work],
                       dict(schema_version=1, event_id='gi-controls',
                            expected_revision=state['revision'], task_id='task'))
    assert code == 0 and value['controls_passed'] and not value['task_accepted'], value
    state = shown(binary, root, out, 'controls-after', work)
    task = next(t for t in state['tasks'] if t['id'] == 'task')
    observations = task['automatic_validation']['controls']
    assert len(observations) == 3
    assert all(c['passed'] and json.loads(c['review_output'])['policy_sha256'] == digest(V1)
               and json.loads(c['review_output'])['decision'] == 'deny' for c in observations)
    seed = out / 'inspection-seed'
    seed.mkdir()
    fixture.publish(seed / 'setup.json', dict(root=str(root), work=work,
                    workspace=str(workspace), candidate=checkpoint, binary_source=False))
    native.run(binary, profile, out / 'review', reuse=seed)
    return root, work, workspace, checkpoint


def tracked_inventory(workspace, checkpoint):
    """Hash actual raw tracked bytes/link targets, preserving every tree entry.

    Names come from the engine's retained checkpoint, not Git index flags. This
    read-only witness covers the retained tree, not a filesystem race or OS lease.
    """
    repository = workspace.parent
    result = {}
    for name, entry in checkpoint['tracked'].items():
        path = repository / name
        raw = os.readlink(path).encode() if entry['mode'] == '120000' else path.read_bytes()
        result[name] = dict(mode=entry['mode'], sha256=digest(raw))
        assert result[name]['sha256'] == entry['sha256']
    return result


def publication_handoff(control, target):
    """Supply exact original birth bytes with the reused reviewer evidence.

    The maintained publication helper requires birth evidence inside its supplied
    inspection context. Reused inspection does not copy the worker's birth there.
    Copy only original result/request/birth bytes into a fresh evidence handoff,
    retaining source paths and hashes. Never edit an old proof or synthesize birth.
    Missing or mismatched actual birth refuses before any publication attempt.
    """
    review = control / 'review'
    source = json.loads((review / 'result.json').read_bytes())
    births = [p for p in (control / 'workers').rglob('owned-root-*.json')
              if json.loads(p.read_bytes()).get('root') == source['root']]
    assert len(births) == 1, 'exact original fixture birth required'
    target.mkdir(mode=0o700, parents=True, exist_ok=False)
    (target / 'inspection').mkdir()
    mapping = [(review / 'result.json', target / 'result.json'),
               (review / 'native-review.input.json', target / 'native-review.input.json'),
               (births[0], target / 'inspection' / births[0].name)]
    provenance = []
    for original, destination in mapping:
        raw = original.read_bytes()
        fixture.publish(destination, raw)
        assert destination.read_bytes() == original.read_bytes()
        provenance.append(dict(original=str(original), supplied=str(destination), sha256=digest(raw)))
    fixture.publish(target / 'provenance.json', provenance)


def run(binary, profile, out, reuse_control=None, resume_from=None):
    """Establish accepted V1, then mutate V2 after genuine R1 in a fresh fixture.

    Preserve old review/receipts, current revisions and financial observations.
    The V2 consumer diagnostic executes the exact control argv in its actual cwd
    as the fixture owner; it is not a second native control or a Biba assertion.
    The acceptance attempt itself is exclusively the supported public CLI.
    Unexpected acceptance records security FAIL and returns false; no correction.
    An unqualified refusal records PARTIAL and also returns false: it must not
    turn an unrelated error into proof of the expected dependency guard.
    resume_from retains an already accepted positive witness and a reviewed V1
    mutation fixture stopped before injection. It never repeats their producers.
    """
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    inventory = qualify(binary, profile, out)
    if resume_from is None:
        control = reuse_control or out / 'control'
        if reuse_control is None:
            prepared_v1(binary, profile, control)
        publication_handoff(control, out / 'control-handoff')
        publication.run(binary, profile, out / 'v1-publication', reuse=out / 'control-handoff')
    else:
        source = resume_from / 'v1-publication/result.json'
        witness = json.loads(source.read_bytes())
        assert witness['status'] == 'PASS' and witness['task_accepted']
        state = shown(binary, Path(witness['root']), out, 'accepted-v1-reused', witness['work'])
        assert next(t for t in state['tasks'] if t['id'] == 'task')['status'] == 'accepted'
        fixture.publish(out / 'accepted-v1-provenance.json', dict(original=str(source), sha256=digest(source.read_bytes())))
    fixture.publish(out / 'progress.json', dict(valid_v1_accepted=True, changed_dependency=False))
    print('GI_V1_VALID_ACCEPTED', flush=True)
    mutation = resume_from / 'mutation' if resume_from else out / 'mutation'
    if resume_from is None:
        root, work, workspace, checkpoint = prepared_v1(binary, profile, mutation)
    else:
        source = mutation / 'review/inspection/setup.json'
        setup = json.loads(source.read_bytes())
        root, work, workspace, checkpoint = Path(setup['root']), setup['work'], Path(setup['workspace']), setup['candidate']
        fixture.publish(out / 'mutation-provenance.json', dict(original=str(source), sha256=digest(source.read_bytes())))
    before = shown(binary, root, out, 'v2-before', work)
    task = next(t for t in before['tasks'] if t['id'] == 'task')
    review = task['independent_review']
    request = dict(schema_version=1, event_id='gi-v2-publication',
                   expected_revision=before['revision'], task_id='task')
    code, preview = call(binary, root, out, 'v1-preflight', ['task', 'publication-preview', work], request)
    assert (code == 0 and preview['native_original_verified'] and preview['current_candidate_verified']
            and preview['publication_supported'] and not preview['publication_ready']
            and preview['blockers'] == ['managed-publication-write-admission-required']), preview
    assert (workspace / 'effective-security.json').read_bytes() == V1
    tracked = tracked_inventory(workspace, checkpoint)
    original = root / '.swarm' / ('native-review-' + digest(review['id'].encode()) + '.receipt')
    originals = {str(p): p.read_bytes() for p in [original, root / review['receipt'], root / review['report']]}
    event = json.loads((mutation / 'review/native-review.input.json').read_bytes())['event_id']
    marker = native.read_reviewer_marker(root, event)
    rows = native.finance(root, work)
    fixture.publish(out / 'v1-tracked.json', tracked)
    fixture.publish(out / 'policy-v1.json', V1)
    fixture.publish(out / 'old-review-original.frame', originals[str(original)])
    fixture.publish(workspace / 'effective-security.json', V2)
    fixture.publish(out / 'policy-v2.json', (workspace / 'effective-security.json').read_bytes())
    assert tracked_inventory(workspace, checkpoint) == tracked
    command = ['python3', 'security-consumer.py']
    result = subprocess.run(command, cwd=workspace, stdin=subprocess.DEVNULL, capture_output=True)
    fixture.publish(out / 'v2-consumer.command.json', dict(argv=command, cwd=str(workspace),
                    executor='owned fixture diagnostic; not native control/Biba', global_watchdog=None))
    fixture.publish(out / 'v2-consumer.stdout', result.stdout)
    fixture.publish(out / 'v2-consumer.stderr', result.stderr)
    observed = json.loads(result.stdout)
    assert result.returncode == 42 and observed['decision'] == 'allow' and observed['policy_sha256'] == digest(V2)
    assert shown(binary, root, out, 'after-dependency-change', work) == before
    code, value = call(binary, root, out, 'v2-publish', ['task', 'publish', work], request)
    after = shown(binary, root, out, 'v2-after', work)
    accepted = code == 0 and value['task_accepted'] and next(t for t in after['tasks'] if t['id'] == 'task')['status'] == 'accepted'
    assert all(p == Path(name).read_bytes() for name, p in originals.items())
    assert native.read_reviewer_marker(root, event) == marker and native.finance(root, work) == rows
    assert tracked_inventory(workspace, checkpoint) == tracked
    assert all(digest(Path(name).read_bytes()) == sha for name, sha in inventory.items())
    if not accepted:
        assert code != 0 and after == before, value
    result = dict(status='FAIL' if accepted else 'PARTIAL', valid_v1_accepted=True,
                  effective_dependency_changed=True, tracked_files_unchanged=True,
                  old_verdict_preserved=True, new_reviewer_calls=0,
                  v2_consumer_exit=42, v2_decision='allow', v2_task_accepted=accepted,
                  vulnerability_reproduced=accepted, root=str(root), work=work,
                  binary_sha256=digest(binary.read_bytes()), candidate_commit=checkpoint['commit'],
                  review_id=review['id'], security_expectation='V2 must refuse old V1 review',
                  refusal=None if accepted else value.get('failure'), independent_security_review=False,
                  paid_provider=False, product_correction=False, global_watchdog=None)
    fixture.publish(out / 'result.json', result)
    print('GI_V2_ACCEPTED_WITH_OLD_REVIEW' if accepted else 'GI_V2_REFUSAL_NEEDS_QUALIFICATION', flush=True)
    return False


def main():
    """Retain helper identities and any original failure; never retry or fix."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['binary', 'profile', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--reuse-control', type=Path)
    parser.add_argument('--resume-from', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    helpers = inspection.native.helper_inputs(Path(__file__))
    try:
        safe = run(args.binary.resolve(), args.profile.resolve(), out,
                   args.reuse_control.resolve() if args.reuse_control else None,
                   args.resume_from.resolve() if args.resume_from else None)
        assert all(digest(Path(name).read_bytes()) == sha for name, sha in helpers.items())
        fixture.publish(out / 'helper-inputs.json', helpers)
    except BaseException:
        if out.exists():
            fixture.publish(out / 'helper-inputs.json', helpers)
            fixture.publish(out / 'failure.json', dict(status='INCOMPLETE', cause=traceback.format_exc()))
        raise
    raise SystemExit(0 if safe else 1)


if __name__ == '__main__':
    main()
