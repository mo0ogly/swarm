#!/usr/bin/env python3
"""Create only the fresh public native prerequisites for P4 withdrawal.

Two inert planning calls, one actual automatic worker, immutable report retention,
three controls, one inert managed reviewer, and one public task publication.
No historical reuse, injected corruption, direct SQL writes, paid provider,
retry, acceptance bypass, default-profile build or global watchdog.
"""
import argparse
import json
import os
import subprocess
import signal
import sys
from pathlib import Path
import traceback

import rust_agent_model_route as fixture
import rust_managed_planning as planning
import rust_mission_watch_worker as watch_worker
import rust_mission_watch_public as watch_public
import rust_native_parent_death as parent_death
import rust_native_managed_reviewer as reviewer
from rust_native_managed_candidate import helper_inputs
from rust_native_task_validation import invoke


class AutomaticSourceRefused(RuntimeError):
    """Safe stage code with the original observed public payload retained."""
    def __init__(self, reason, observation):
        """Retain the original payload while exposing only a static stage code."""
        self.reason = reason
        self.observation = observation
        super().__init__('p4-automatic-source-' + reason)


def require_fresh_automatic_work(work, native):
    """Refuse observed non-fresh/manual state before starting any watch.

    This checks fixture prerequisites, never grants automatic-worker authority.
    Actual dispatch, native producer, immutable result and submission remain
    engine-owned checks. Keep the offending payload rather than inventing state.
    """
    tasks = work.get('tasks') if isinstance(work, dict) else None
    planning = work.get('planning') if isinstance(work, dict) else None
    agents = native.get('agents') if isinstance(native, dict) else None
    controls = native.get('controls') if isinstance(native, dict) else None
    valid = (isinstance(tasks, list) and len(tasks) == 1
             and tasks[0].get('id') == 'task' and tasks[0].get('status') == 'todo'
             and not tasks[0].get('attempts') and isinstance(planning, dict)
             and planning.get('paused') is True and agents == []
             and isinstance(controls, dict) and controls.get('paused') is False
             and controls.get('consent_enabled') is True
             and type(controls.get('slots')) is int and controls['slots'] == 1 and controls.get('archived') is False)
    if not valid:
        raise AutomaticSourceRefused('fresh-worker-required', dict(work=work, native=native))


class AutomaticProgress:
    """Observe one departure, actual helper wait and canonical handoff.

    Stream events are observations, not grants. Stop only after both OS exit0
    and submission for the single admitted agent. Reject actual terminal
    failures only; waiting/blocked observations remain transient.
    No timer, retry count or elapsed-time success is used.
    """
    def __init__(self, work):
        """Track one fixture work without acquiring authority or resources."""
        self.work = work
        self.agent = None
        self.exited = False
        self.submitted = False
        self.waiting = None

    def accept(self, event):
        """Consume one real event or refuse with its complete original payload."""
        if not isinstance(event, dict) or (type(event.get('schema_version')) is not int or event['schema_version'] != 1) or event.get('work') != self.work:
            raise AutomaticSourceRefused('event-shape', event)
        kind = event.get('event')
        if kind == 'mission-worker-started':
            if self.agent is not None or not isinstance(event.get('agent_id'), str) or not event['agent_id']:
                raise AutomaticSourceRefused('unexpected-departure', event)
            self.agent = event['agent_id']
        elif kind in ('mission-worker-exited', 'mission-report-submitted', 'mission-report-waiting'):
            if self.agent is None or event.get('agent_id') != self.agent:
                raise AutomaticSourceRefused('producer-mismatch', event)
            if kind == 'mission-worker-exited':
                if type(event.get('exit_code')) is not int or event['exit_code'] != 0 or event.get('exit_signal') is not None:
                    raise AutomaticSourceRefused('worker-exit', event)
                self.exited = True
            elif kind == 'mission-report-submitted':
                if self.submitted or not event.get('report') or event.get('task_accepted') is not False:
                    raise AutomaticSourceRefused('handoff-shape', event)
                self.submitted = True
                self.waiting = None
            else:
                self.waiting = event
        elif kind == 'mission-worker-adopted':
            raise AutomaticSourceRefused('unexpected-adoption', event)
        elif kind == 'mission-watch-blocked':
            # NoAdmission/changed readiness is an observation, not a terminal
            # refusal. The engine keeps its own saved-cadence reobservation;
            # this collector grants no provider retry or new business action.
            pass
        elif kind == 'mission-watch-stopped':
            if not (self.exited and self.submitted):
                raise AutomaticSourceRefused('stopped-before-handoff', event)
        return self.exited and self.submitted


def automatic_handoff(owned, out):
    """Run one public CLI conductor from verified fresh TODO prerequisites.

    Planning stays paused, so actual validator/reviewer/planner gates launch no
    extra calls. Require both helper wait0 and handoff before SIGINT; then drain,
    wait, pause only this mission and verify the actual native producer. Failure
    preserves event/error/close causes and cleans only exact owned adopted helper
    children under the existing subreaper contract. No server or SQL mutation.
    """
    work = owned.ok(['work', 'show', owned.work])['work']
    native = owned.ok(['mission', 'status-native', owned.work])
    require_fresh_automatic_work(work, native)
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    fixture.publish(out/'prerequisites.json', dict(work=work, native=native))

    def operation():
        """Own one conductor and preserve primary plus independent cleanup failures."""
        process = None
        channels = []
        primary = None
        errors = []
        events = []
        progress = AutomaticProgress(owned.work)
        try:
            log = (out/'watch.stdout').open('xb')
            channels.append(log)
            diagnostic = (out/'watch.stderr').open('xb')
            channels.append(diagnostic)
            command = [str(owned.binary), 'mission', 'watch', owned.work, '--root', str(owned.root), '--json']
            fixture.publish(out/'command.json', dict(argv=command, global_watchdog=None))
            process = subprocess.Popen(command, env=owned.environment, stdin=subprocess.DEVNULL,
                                       stdout=subprocess.PIPE, stderr=diagnostic)
            while True:
                raw = process.stdout.readline()
                if not raw:
                    raise AutomaticSourceRefused('eof-before-handoff', dict(events=events, returncode=process.poll()))
                log.write(raw)
                log.flush()
                os.fsync(log.fileno())
                event = json.loads(raw)
                events.append(event)
                fixture.publish(out/'events.json', events)
                if progress.accept(event):
                    break
        except BaseException as error:
            primary = error
            watch_worker.retain_failure(errors, lambda: fixture.publish(out/'observation-failure.json',
                dict(reason=getattr(error, 'reason', type(error).__name__), observation=getattr(error, 'observation', None))))
        finally:
            if process is not None:
                if process.poll() is None:
                    watch_worker.retain_failure(errors, lambda: process.send_signal(signal.SIGINT))
                def drain():
                    """Wait the owned conductor and preserve remaining bytes and exit."""
                    tail, _ = process.communicate()
                    log.write(tail)
                    log.flush()
                    os.fsync(log.fileno())
                    fixture.publish(out/'tail.stdout', tail)
                    terminal_events = [json.loads(line) for line in tail.splitlines() if line]
                    events.extend(terminal_events)
                    fixture.publish(out/'events.json', events)
                    fixture.publish(out/'exit.json', dict(exit=process.returncode, global_watchdog=None))
                    stopped = [event for event in events if event.get('event') == 'mission-watch-stopped']
                    if (process.returncode != 0 or len(stopped) != 1
                            or stopped[0].get('lease_released') is not True
                            or stopped[0].get('root_closed') is not True
                            or stopped[0].get('children_still_running') != 0):
                        raise AutomaticSourceRefused('conductor-exit', dict(exit=process.returncode, stopped=stopped))
                watch_worker.retain_failure(errors, drain)
            for channel in channels:
                watch_worker.retain_failure(errors, channel.close)
            def pause():
                """Withdraw only this fixture mission consent after conductor drain."""
                current = owned.ok(['work', 'show', owned.work])['work']
                owned.ok(['mission', 'pause', owned.work], dict(schema_version=1, event_id='p4-automatic-pause',
                    expected_revision=current['revision'], reason='Retain owned automatic source fixture'))
            watch_worker.retain_failure(errors, pause)
            if primary is not None or errors:
                watch_worker.retain_failure(errors, lambda: watch_public.cleanup_owned_children(owned.binary, owned.root, out))
        if primary is not None or errors:
            raise BaseExceptionGroup('P4 automatic handoff and cleanup', ([primary] if primary else []) + errors)
        # Read after the conductor and helper exited, avoiding a live writer probe.
        agent = owned.ok(['agent', 'show', progress.agent])['agent']
        current = owned.ok(['work', 'show', owned.work])['work']
        fixture.publish(out/'producer-after.json', agent)
        fixture.publish(out/'work-after.json', current)
        if (agent.get('id') != progress.agent or agent.get('status') != 'completed' or agent.get('role') != 'worker'
                or type(agent.get('exit_code')) is not int or agent['exit_code'] != 0
                or agent.get('work_id') != owned.work or agent.get('task_id') != 'task'
                or not isinstance(agent.get('native_mission'), dict)
                or not agent['native_mission'] or current['tasks'][0]['status'] != 'submitted'):
            raise AutomaticSourceRefused('native-producer-required', dict(agent=agent, work=current))
        return agent

    return watch_worker.subreaper_scope(operation, lambda error: None, parent_death.subreaper)



def run(binary, profile, out):
    """Create a new owned root, then use only declared public positive operations.

    Inputs are an existing qualified default native binary/profile and a fresh
    evidence directory. Retain physical fixture birth, exact command channels,
    provider markers, candidate and original review. Output result.json is
    accepted only after public publication/readback and unchanged binary/helper
    hashes. On failure preserve partial state and original traceback; no replay
    or cleanup substitutes another inode. Fixture budgets/controls are declared
    scenario inputs, not production defaults or money spent on an actual LLM.
    """
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    qualified = json.loads(profile.read_bytes())
    assert qualified['no_fixture_feature'] and qualified['sha256'] == fixture.digest(binary.read_bytes())
    f = fixture.Fixture(binary, out / 'birth', 'p4-fresh-source')
    root = f.root
    source = root / 'source'
    source.mkdir(mode=0o700)
    planning.repository.git(source, 'init', '--quiet')
    checkout = Path(__file__).resolve().parents[1]
    name = planning.repository.git(checkout, 'config', 'user.name').decode().strip()
    email = planning.repository.git(checkout, 'config', 'user.email').decode().strip()
    assert name and email
    project = source / 'app'
    project.mkdir(mode=0o700)
    fixture.publish(project / 'tracked.txt', b'Committed planning input\n')
    planning.repository.git(source, 'add', '--', 'app/tracked.txt')
    planning.repository.git(source, '-c', 'user.name=' + name, '-c', 'user.email=' + email,
                            '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false',
                            'commit', '--quiet', '-m', 'Owned P4 native source fixture')
    program = planning.feedback.PROGRAM.replace("open('planning-proposal.json')", 'open(' + repr(str(root / 'planning-proposal.json')) + ')')
    fixture.publish(root / 'codex', program.encode())
    (root / 'codex').chmod(0o700)
    f.work = planning.fresh_work(f, 'p4-managed-work')
    work = f.ok(['planning', 'enable', f.work], planning.request(
        event='p4-planning-enable', max_activations=6, max_tasks=1,
        repository=dict(path='source/app', committed_only=True),
        checks={'req-1': reviewer.inspection.native.CONTROLS}))
    planning.proposal(f, work, 'root', [dict(kind='delegate', id='child', title='P4 fixture child', next='Plan one inert task', requirements=['req-1'], max_tasks=1, max_activations=3)])
    delegated = f.ok(['planning', 'step', f.work])
    assert delegated['run']['decision_applied']
    work = delegated['run']['work']
    planning.proposal(f, work, 'child', [dict(kind='task', id='task', title='P4 inert worker', next='Produce the owned report', requirements=['req-1'], deliverable='handoff.md', criteria=['Owned handoff'])])
    planned = f.ok(['planning', 'step', f.work])
    assert planned['run']['decision_applied']
    work = planned['run']['work']
    assert len(work['tasks']) == 1 and work['tasks'][0]['status'] == 'todo'
    work = f.ok(['mission', 'start', f.work], dict(schema_version=1, event_id='p4-consent', expected_revision=work['revision'], reason='One explicit inert automatic worker', slots=1, profile=dict(provider='fixture', role='worker', workspace='source', level='standard', capture_output=True)))
    work = f.ok(['budget', 'apply', f.work], dict(schema_version=1, event_id='p4-worker-budget', expected_revision=work['revision'], budget=dict(limit_usd=10, reserve_per_launch_usd=1, estimate_source='Owned P4 inert worker fixture', reference_date='2026-11-18')))
    # The actual inert worker creates its report in its attributed workspace;
    # the parent never synthesizes a successful immutable handoff or receipt.
    report = "\nfrom pathlib import Path\nPath('docs').mkdir(exist_ok=True)\nwith Path('docs/task.md').open('x') as f:\n f.write('Owned P4 native worker report.\\n');f.flush();os.fsync(f.fileno())\nwith Path('handoff.md').open('x') as f:\n f.write('Owned P4 handoff.\\n');f.flush();os.fsync(f.fileno())\n"
    worker = fixture.PROGRAM.replace('raw=sys.stdin.buffer.read()', 'raw=sys.stdin.buffer.read()' + report)
    fixture.publish(root / 'codex', worker.encode())
    (root / 'codex').chmod(0o700)
    current = f.ok(['work', 'show', f.work])['work']
    if not current['planning'].get('paused'):
        f.ok(['planning', 'pause', f.work], dict(schema_version=1, event_id='p4-pause-planner', expected_revision=current['revision'], reason='Only the single native automatic worker may depart'))
    automatic_handoff(f, out / 'immutable-retention')
    environment = dict(os.environ, CODEX_HOME=str(root / 'codex-home'))

    def call(label, args, value=None):
        """Retain one exact CLI result; a refused or malformed reply aborts."""
        code, wire = invoke(binary, root, out, label, args, value, environment)
        assert code == 0, (label, wire)
        return wire

    def shown(label):
        """Observe current Work through the same public native executable."""
        return call(label, ['work', 'show', f.work])['work']

    current = shown('submitted')
    assert current['tasks'][0]['status'] == 'submitted'
    if call('mission-before-controls', ['mission', 'show', f.work])['paused']:
        call('resume-for-controls', ['mission', 'resume', f.work], dict(schema_version=1, event_id='p4-controls-resume', expected_revision=current['revision'], reason='Explicit controls and review; no conductor'))
    current = shown('before-planning-resume')
    if current['planning'].get('paused'):
        call('resume-planning-for-controls', ['planning', 'resume', f.work], dict(schema_version=1, event_id='p4-plan-resume', expected_revision=current['revision'], reason='Explicit controls and review; no conductor'))
    current = shown('before-candidate')
    request = dict(schema_version=1, event_id='p4-candidate', expected_revision=current['revision'], task_id='task')
    candidate = call('candidate', ['task', 'validation-candidate', f.work], request)
    assert candidate['control_count'] == 3 and not candidate['job_created']
    current = shown('before-controls')
    controls = call('controls', ['task', 'validate', f.work], dict(schema_version=1, event_id='p4-controls', expected_revision=current['revision'], task_id='task'))
    assert controls['controls_passed'] and not controls['task_accepted']
    current = shown('before-review')
    request = dict(schema_version=1, event_id='p4-native-review', expected_revision=current['revision'], task_id='task')
    inputs = call('review-inputs', ['task', 'review-inputs', f.work], request)
    protocol = inputs['managed_protocol']
    program = reviewer.provider_program(dict(case='pass', source='', marker_event=request['event_id']))
    fixture.publish(root / 'codex', program)
    (root / 'codex').chmod(0o700)
    review_result = call('native-review', ['task', 'review', f.work], request)
    assert not review_result['task_accepted']
    current = shown('reviewed')
    review = current['tasks'][0]['independent_review']
    assert review['state'] == 'passed' and review['candidate_commit'] == protocol['context']['candidate_commit']
    marker = reviewer.read_reviewer_marker(root, request['event_id'])
    assert json.loads(marker)['context_sha256'] == protocol['context_sha256']
    request = dict(schema_version=1, event_id='p4-first-publication', expected_revision=current['revision'], task_id='task')
    preview = call('publication-preview', ['task', 'publication-preview', f.work], request)
    assert preview['native_original_verified'] and preview['current_candidate_verified'] and preview['opinion_state'] == 'passed'
    published = call('publish', ['task', 'publish', f.work], request)
    assert published['task_accepted'] and not published['replayed'] and not published['provider_started']
    final = shown('accepted')
    assert final['tasks'][0]['status'] == 'accepted'
    assert final['planning']['repository']['candidate_commit'] == review['candidate_commit']
    assert reviewer.read_reviewer_marker(root, 'p4-native-review') == marker
    storage = Path(final['planning']['repository']['storage'])
    candidates = [(path, path.read_bytes()) for path in (storage / 'candidates').glob('*/candidate.json')]
    candidates = [(path, raw) for path,raw in candidates if json.loads(raw)['task'] == 'task']
    assert len(candidates) == 1
    candidate_path, candidate_raw = candidates[0]
    original_candidate = json.loads(candidate_raw)
    assert original_candidate['commit'] == review['candidate_commit'] == candidate['candidate_commit']
    assert original_candidate['binding_sha256'] == candidate['candidate_binding_sha256']
    fixture.publish(out / 'retained-candidate.json', candidate_raw)
    original_path = root / '.swarm' / ('native-review-' + fixture.digest(review['id'].encode()) + '.receipt')
    original_raw = original_path.read_bytes()
    magic, length, digest, body = original_raw.split(b'\n', 3)
    assert magic == b'SR1' and int(length) == len(body) and digest.decode() == fixture.digest(body)
    original = json.loads(body)
    assert original['waited'] and original['grant_attempted'] and original['channel_closed'] and not original['failed']
    assert original['managed_review']['protocol'] == protocol
    fixture.publish(out / 'retained-review.frame', original_raw)
    physical = os.stat(root / '.swarm', follow_symlinks=False)
    observation = dict(status='OBSERVED_PUBLICATION_NOT_QUALIFIED', root=str(root), work=f.work, candidate=review['candidate_commit'], task_accepted=True, mission_accepted=False, paid_provider=False, publication_provider_started=False, inert_provider_calls=dict(planner=2,worker=1,reviewer=1), control_processes=3, private_sql_writes=False, physical=dict(device=physical.st_dev,inode=physical.st_ino), binary_sha256=qualified['sha256'], global_watchdog=None,
                       consumed_originals={str(candidate_path): fixture.digest(candidate_raw), str(original_path): fixture.digest(original_raw)},
                       review_marker=dict(path=str(reviewer.reviewer_leaf(root,'p4-native-review')/'managed-reviewer-invoked.json'),sha256=fixture.digest(marker)))
    fixture.publish(out / 'observed-publication.json', observation)
    # A public effect is retained even if final qualification subsequently fails.
    return observation


def supervise(binary, profile, out):
    """Observe one child exit and bind its final result; never retry it.

    This outer process publishes terminal.json only after actual exit zero,
    absent failure.json, and readback of every final result/artifact/original.
    A visible PASS without this successful terminal receipt is unqualified.
    Retain child stdout/stderr/argv even when it fails; no global watchdog.
    """
    assert not out.exists(), 'fresh P4 evidence directory required'
    command = [sys.executable, str(Path(__file__).resolve()), '--binary', str(binary), '--profile', str(profile), '--output', str(out)]
    child = subprocess.run(command, stdin=subprocess.DEVNULL, capture_output=True)
    out.mkdir(mode=0o700, parents=True, exist_ok=True)
    fixture.publish(out/'process.stdout', child.stdout)
    fixture.publish(out/'process.stderr', child.stderr)
    fixture.publish(out/'process.json', dict(argv=command, exit=child.returncode, global_watchdog=None))
    assert child.returncode == 0 and not (out/'failure.json').exists(), 'P4 child not qualified; original channels retained'
    raw = (out/'result.json').read_bytes()
    result = json.loads(raw)
    assert result['status'] == 'PASS'
    assert fixture.digest((out/'inputs.json').read_bytes()) == result['inputs_sha256']
    assert fixture.digest((out/'observed-publication.json').read_bytes()) == result['observation_sha256']
    for name,digest in result['artifacts'].items():
        relative=Path(name)
        assert not relative.is_absolute() and '..' not in relative.parts
        assert fixture.digest((out/relative).read_bytes()) == digest
    assert all(fixture.digest(Path(path).read_bytes()) == digest for path,digest in result['consumed_originals'].items())
    marker=result['review_marker']
    assert fixture.digest(Path(marker['path']).read_bytes()) == marker['sha256']
    assert (out/'result.json').read_bytes() == raw
    fixture.publish(out/'terminal.json', dict(status='QUALIFIED_P4_FRESH_SOURCE',exit=0,result_sha256=fixture.digest(raw),process_sha256=fixture.digest((out/'process.json').read_bytes()),scope='Owned fixture publication qualification only; no H2 or installation'))
    print('P4_FRESH_SOURCE_TERMINAL_VERIFIED',flush=True)


def main():
    """Bind exact binary/profile/static helper closure and preserve failures."""
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ['binary', 'profile', 'output']:
        parser.add_argument('--' + field, required=True, type=Path)
    parser.add_argument('--supervise', action='store_true')
    args = parser.parse_args()
    if args.supervise:
        try:
            supervise(args.binary.resolve(), args.profile.resolve(), args.output.resolve())
        except BaseException:
            if args.output.is_dir():
                fixture.publish(args.output/'qualification-failure.json', dict(status='FAIL',cause=traceback.format_exc()))
            raise
        return
    inputs = dict(binary_sha256=fixture.digest(args.binary.read_bytes()), profile_sha256=fixture.digest(args.profile.read_bytes()), helpers=helper_inputs(Path(__file__)), global_watchdog=None)
    try:
        observed = run(args.binary.resolve(), args.profile.resolve(), args.output.resolve())
        fixture.publish(args.output / 'inputs.json', inputs)
        assert fixture.digest(args.binary.read_bytes()) == inputs['binary_sha256']
        assert fixture.digest(args.profile.read_bytes()) == inputs['profile_sha256']
        assert all(fixture.digest(Path(path).read_bytes()) == digest for path,digest in inputs['helpers'].items())
        assert all(fixture.digest(Path(path).read_bytes()) == digest for path,digest in observed['consumed_originals'].items())
        marker = observed['review_marker']
        assert fixture.digest(Path(marker['path']).read_bytes()) == marker['sha256']
        physical = os.stat(Path(observed['root']) / '.swarm', follow_symlinks=False)
        assert observed['physical'] == dict(device=physical.st_dev,inode=physical.st_ino)
        assert json.loads((args.output / 'accepted.stdout').read_bytes())['work']['tasks'][0]['status'] == 'accepted'
        # Freeze all retained observations before the final qualification marker.
        artifacts = {str(path.relative_to(args.output)): fixture.digest(path.read_bytes())
                     for path in args.output.rglob('*') if path.is_file()}
        result = dict(observed, status='PASS', artifacts=artifacts,
                      inputs_sha256=fixture.digest((args.output/'inputs.json').read_bytes()),
                      observation_sha256=fixture.digest((args.output/'observed-publication.json').read_bytes()),
                      qualification_scope='Default native fixture birth and one public publication; not H2 or installation')
        fixture.publish(args.output / 'result.json', result)
        print('P4_FRESH_NATIVE_SOURCE_PUBLISHED', flush=True)
    except BaseException:
        if args.output.is_dir():
            if not (args.output/'inputs.json').exists():
                fixture.publish(args.output/'inputs.json', inputs)
            # An already published result may remain visible after a late IO
            # error. Consumers must reject failure.json and require retained
            # successful process exit plus a result hash from the outer runner.
            fixture.publish(args.output / 'failure.json', dict(status='FAIL', cause=traceback.format_exc(), visible_result=(args.output/'result.json').exists()))
        raise



if __name__ == '__main__':
    main()
