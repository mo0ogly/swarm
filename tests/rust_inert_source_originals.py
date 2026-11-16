"""Proposed I0 originals for public source fixtures; no execution at import.

Fixture programs are inert data, not actors/authority. Only an allocated caller
may invoke a public operation. Original receipts/outputs are retained after that
operation returns; no model reads Root, claim rewrite, retry or acceptance.
"""
import json
import os
import secrets
import rust_agent_model_route as fixture
import rust_planning_rejection_recovery as feedback
import rust_manual_mission as mission


def planner_step(owned, work, scope, operations, label):
    """Freeze one explicit proposal before claim, then retain its actual I0 run.

    Uses the existing public planning step once. Success requires engine receipt
    exit0/grant/complete proposal plus exact claim/Work/scope/PID/prompt witnesses.
    Receipt status is engine-reported wait, not an independently owned OS wait.
    Missing/stale/altered evidence or cleanup refuses, preserving original causes
    and known public result. Never reads Root-relative files from the provider.
    Returns (public result, original prompt bytes, retained binding metadata).
    """
    # Resolve only at use: direct process/retention imports must finish first.
    from rust_unstarted_review_process import Failure, check
    from rust_unstarted_review_retention import read_at
    check(isinstance(label, str) and label not in ('', '.', '..') and '/' not in label, 'source-planner-output-label')
    value = dict(input_events=[e['id'] for e in work['planning']['inbox'] if e['scope'] == scope and not e.get('decision')],
                 reason='Explicit public integration fixture', operations=operations)
    program = feedback.program_with_proposal(value).encode()
    frozen = fixture.digest(program)
    output = owned.output / ('planner-originals-' + label)
    output.mkdir(mode=0o700, exist_ok=False)
    fixture.publish(output/'proposal.before.json', value)
    fixture.publish(output/'program.before.py', program)
    # Both files are prepared BEFORE planning::claim, never patched in flight.
    fixture.publish(owned.root/'planning-proposal.json', value)
    fixture.publish(owned.root/'codex', program)
    (owned.root/'codex').chmod(0o700)
    result = owned.ok(['planning', 'step', owned.work])
    run = result['run']
    claim = run['claim_id']
    check(run['replayed'] is False and run['decision_applied'] is True and run['work']['id'] == owned.work
          and type(run['work']['revision']) is int and run['work']['revision'] > work['revision'], 'source-planner-new-recorded-decision-required')
    check(isinstance(claim, str) and claim not in ('', '.', '..') and '/' not in claim, 'source-planner-claim-component')
    held, errors, retained = [], [], None
    try:
        rootfd = os.open(owned.root, os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW)
        held.append(rootfd)
        privatefd = os.open('.swarm', os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW, dir_fd=rootfd)
        held.append(privatefd)
        frame, identity = read_at(privatefd, ['native-planning-'+fixture.digest(claim.encode())+'.receipt'])
        fixture.publish(output/'original.receipt', frame)
        magic, extent, digest, body = frame.split(b'\n', 3)
        check(magic == b'SR1' and extent.isdigit() and int(extent) == len(body) and digest.decode('ascii') == fixture.digest(body), 'source-planner-receipt-frame')
        receipt = json.loads(body)
        job = receipt['run']
        private = os.fstat(privatefd)
        check(receipt['schema_version'] == 1 and receipt['exit_code'] == 0 and receipt['signal'] is None
              and receipt['output_complete'] is True and receipt['grant_attempted'] is True
              and receipt['failure'] is None and receipt['gate'] is None and bool(receipt['finished_at']), 'source-planner-complete-native-receipt')
        check(job['claim_id'] == claim and job['work_id'] == owned.work and job['scope_id'] == scope
              and job['permission'] == 1 and job['phase'] == 'permitted'
              and job['directory_device'] == str(private.st_dev) and job['directory_inode'] == str(private.st_ino), 'source-planner-run-binding')
        pid = job['child_pid']
        check(type(pid) is int and pid > 0 and type(job['owner_pid']) is int and job['owner_pid'] > 0
              and job['parent_pid'] == job['owner_pid'] and job['group_id'] == pid
              and bool(job['child_stamp']) and bool(job['owner_stamp']) and bool(job['owner_host']), 'source-planner-native-process-shape')
        leaf = ['biba-i0', 'planning', claim]
        prompt, prompt_identity = read_at(privatefd, [*leaf, 'fixture-prompt-'+str(pid)+'.bin'])
        marker_raw, marker_identity = read_at(privatefd, [*leaf, 'model-received-'+str(pid)+'.json'])
        fixture.publish(output/'prompt.original.bin', prompt)
        fixture.publish(output/'marker.original.json', marker_raw)
        marker = json.loads(marker_raw)
        check(marker['pid'] == pid and marker['ppid'] == job['owner_pid']
              and marker['prompt_sha256'] == fixture.digest(prompt) == job['prompt_sha256']
              and marker['prompt_bytes'] == len(prompt), 'source-planner-marker-binding')
        context_raw = prompt.rsplit(b'\n\n', 1)[1]
        context = json.loads(context_raw)
        context_events = [e['id'] for e in context['events']]
        check(fixture.digest(context_raw) == job['context_sha256']
              and context['scope']['id'] == scope and context['scope']['revision'] == job['scope_revision']
              and len(set(context_events)) == len(context_events)
              and sorted(context_events) == sorted(value['input_events'])
              and type(job['generation']) is int and job['generation'] > 0,
              'source-planner-context-binding')
        check(isinstance(receipt['proposal'], str)
              and json.dumps(json.loads(receipt['proposal']), sort_keys=True, allow_nan=False)
              == json.dumps(value, sort_keys=True, allow_nan=False), 'source-planner-frozen-proposal-mismatch')
        current_program, _ = read_at(rootfd, ['codex'])
        check(fixture.digest(current_program) == frozen, 'source-planner-program-changed')
        retained = dict(claim_id=claim, scope=scope, generation=job['generation'], receipt_sha256=fixture.digest(body),
                        frame_identity=identity, prompt_identity=prompt_identity, marker_identity=marker_identity,
                        program_sha256=frozen, prompt_sha256=fixture.digest(prompt),
                        pid=pid, ppid=job['owner_pid'], wait_scope='successful public call and engine original exit receipt',
                        no_actor_authentication=True)
        fixture.publish(output/'binding.json', retained)
    except BaseException as error:
        errors.append(error)
    finally:
        for fd in reversed(held):
            try: os.close(fd)
            except BaseException as error: errors.append(error)
    if errors:
        raise Failure('source-planner-originals-or-close', errors, facts=dict(work=owned.work, recorded_revision=run['work']['revision'], no_rollback_inference=True))
    return result, prompt, retained


def publish_fresh(rootfd, name, raw):
    """Publish complete bytes at one new held-Root entry without replacement.

    Technical adjacent temp+exclusive hardlink, fsync and complete readback. An
    existing destination refuses even if equal; no unlink/retry of it is allowed.
    Failures retain original causes, temporary basename and creation identity
    when observed, attempted stage and known link return. These observations do
    not establish current path identity or absence/rollback after an error.
    Failure performs no rollback or extra cleanup: late errors can leave a
    visible destination and an already removed temporary. This is staging,
    not provider permission.
    Returns the observed destination device/inode only after complete readback.
    """
    # Resolve only at use: direct process/retention imports must finish first.
    from rust_unstarted_review_process import Failure, check
    from rust_unstarted_review_retention import read_at
    check(isinstance(name, str) and name not in ('', '.', '..') and '/' not in name, 'source-publication-component')
    temporary = '.source-report-' + secrets.token_hex(16)
    fd, errors, temporary_identity = None, [], None
    linked, close_failed, stage = False, False, 'stage-create'
    try:
        fd = os.open(temporary, os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_CLOEXEC|os.O_NOFOLLOW, 0o600, dir_fd=rootfd)
        stage = 'stage-identity'
        created = os.fstat(fd)
        temporary_identity = dict(device=created.st_dev, inode=created.st_ino)
        stage = 'stage-write'
        remaining = memoryview(raw)
        while remaining:
            count = os.write(fd, remaining)
            check(count > 0, 'source-publication-write-no-progress')
            remaining = remaining[count:]
        stage = 'stage-fsync'
        os.fsync(fd)
    except BaseException as error:
        errors.append(error)
    finally:
        if fd is not None:
            if not errors:
                stage = 'stage-close'
            try: os.close(fd)
            except BaseException as error:
                close_failed = True
                errors.append(error)
    if errors:
        raise Failure('source-publication-stage-or-close', errors, facts=dict(temporary=temporary, temporary_creation_identity=temporary_identity, stage=stage, close_failed=close_failed, destination_attempted=False, destination_link_observed=False, no_rollback_inference=True))
    try:
        staged, identity = read_at(rootfd, [temporary])
        check(staged == raw, 'source-publication-stage-readback')
    except BaseException as error:
        raise Failure('source-publication-stage-readback-failed', [error], facts=dict(temporary=temporary, temporary_creation_identity=temporary_identity, stage='stage-readback', destination_attempted=False, destination_link_observed=False, no_rollback_inference=True))
    stage = 'destination-link'
    try:
        os.link(temporary, name, src_dir_fd=rootfd, dst_dir_fd=rootfd, follow_symlinks=False)
        linked = True
        stage = 'destination-fsync'
        os.fsync(rootfd)
        stage = 'destination-readback'
        saved, saved_identity = read_at(rootfd, [name])
        check(saved == raw and saved_identity == identity, 'source-publication-target-readback')
        # Remove only the unchanged owned temporary AFTER the new link is known.
        stage = 'temporary-identity-check'
        current = os.stat(temporary, dir_fd=rootfd, follow_symlinks=False)
        check((current.st_dev, current.st_ino) == (identity['device'], identity['inode']), 'source-publication-temp-identity-changed')
        stage = 'temporary-unlink'
        os.unlink(temporary, dir_fd=rootfd)
        stage = 'temporary-unlink-fsync'
        os.fsync(rootfd)
    except BaseException as error:
        raise Failure('source-publication-link-or-readback', [error], facts=dict(temporary=temporary, temporary_creation_identity=temporary_identity, stage=stage, destination_attempted=True, destination_link_observed=linked, no_rollback_inference=True))
    return saved_identity


def worker_report(owned, prepared, actual):
    """Retain a completed native worker's I0 report, then stage its public copy.

    Requires the synchronous successful run_and_assert witness and fresh agent
    readback, matching prompt/PID/PPID/native completion. No output is consumed
    before that wait. Original I0 bytes remain untouched. End-to-end source
    controls require Root/handoff.md; task submit alone also accepts I0 paths.
    The shared inert REPORT_TEMPLATE is an oracle only: missing/changed actual
    output is never reconstructed from it. Existing destination or drift refuses
    without overwrite/retry or a
    positive report/receipt invented from a failed worker. Root decides recovery.
    Returns source/destination hashes, identities and producer attribution only
    after publication/readback; exceptions preserve possible partial effects.
    """
    # Resolve only at use: direct process/retention imports must finish first.
    from rust_unstarted_review_process import Failure, check
    from rust_unstarted_review_retention import read_at
    output = owned.output/'worker-originals'
    output.mkdir(mode=0o700, exist_ok=False)
    agent = owned.ok(['agent', 'show', 'session'])['agent']
    check(agent['id'] == prepared['agent']['id'] and agent['work_id'] == owned.work
          and agent['task_id'] == owned.request['task_id'] and agent['role'] == 'worker'
          and agent['attempt_id'] == prepared['agent']['attempt_id']
          and agent['workspace'] == prepared['agent']['workspace'] and agent['status'] == 'completed'
          and agent['exit_code'] == 0 and bool(agent['ended'])
          and agent.get('native_execution') is not None and agent.get('native_execution_result') is not None,
          'source-worker-native-completion-required')
    check(type(actual['pid']) is int and actual['pid'] > 0 and type(actual['ppid']) is int
          and actual['pid'] == agent['child_pid'] and actual['ppid'] == agent['supervisor_pid']
          and actual['stdin_sha256'] == fixture.digest(prepared['agent']['prompt'].encode()), 'source-worker-native-witness-binding')
    root = owned.root.resolve()
    workspace = owned.workspace.relative_to(root)
    check(bool(workspace.parts) and all(v not in ('', '.', '..') for v in workspace.parts)
          and str(owned.workspace) == owned.request['workspace'], 'source-worker-workspace-binding')
    held, errors, report = [], [], None
    publication_attempted = False
    try:
        rootfd = os.open(root, os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW)
        held.append(rootfd)
        raw, identity = read_at(rootfd, [*workspace.parts, 'handoff.md'])
        fixture.publish(output/'report.original.md', raw)
        marker_raw, marker_identity = read_at(rootfd, [*workspace.parts, 'model-received.json'])
        fixture.publish(output/'marker.original.json', marker_raw)
        fixture.publish(output/'agent.completed.json', agent)
        marker = json.loads(marker_raw)
        expected_report = mission.REPORT_TEMPLATE.format(input_sha256=fixture.digest(prepared['agent']['prompt'].encode()), model=prepared['agent']['model_route']['model']).encode()
        check(json.dumps(marker, sort_keys=True) == json.dumps(actual, sort_keys=True)
              and raw == expected_report, 'source-worker-report-witness-mismatch')
        expected = fixture.digest(raw)
        again, same_identity = read_at(rootfd, [*workspace.parts, 'handoff.md'])
        check(again == raw and same_identity == identity, 'source-worker-report-changed-before-publication')
        fixture.publish(output/'publication.intent.json', dict(source=str(workspace/'handoff.md'), destination='handoff.md', source_sha256=expected, source_identity=identity))
        publication_attempted = True
        destination_identity = publish_fresh(rootfd, 'handoff.md', raw)
        after, after_identity = read_at(rootfd, [*workspace.parts, 'handoff.md'])
        check(after == raw and after_identity == identity, 'source-worker-report-changed-after-publication')
        report = dict(source=str(workspace/'handoff.md'), destination='handoff.md', sha256=expected,
                      source_identity=identity, destination_identity=destination_identity,
                      marker_identity=marker_identity, agent=agent['id'], attempt=agent['attempt_id'],
                      child_pid=agent['child_pid'], parent_pid=agent['supervisor_pid'],
                      wait_scope='completed public native run_and_assert and agent readback; no independent OS re-wait',
                      task_accepted=False, no_actor_authentication=True)
        fixture.publish(output/'publication.observed.json', report)
    except BaseException as error:
        errors.append(error)
    finally:
        for fd in reversed(held):
            try: os.close(fd)
            except BaseException as error: errors.append(error)
    if errors:
        raise Failure('source-worker-report-or-close', errors, facts=dict(work=owned.work, publication_attempted=publication_attempted, destination_state='not inferred from failure; inspect retained intent and actual public path', no_rollback_inference=True))
    return report
