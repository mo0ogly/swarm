"""Proposed eight controlled contracts for source16/17; NOT executed by preparation.

No engine, database, real claim/provider, subprocess or failed historical fixture.
Public-call/agent/completion records are EXPLICIT DOUBLES, never native evidence.
SR1 parsing, nofollow reads, files, hardlinks and ordinary closes are real when
future allocated tests run. Fault errors are injected, not observed OS failures.
Caller must admit real43-source imports/cycle before running this new test module.
All fixtures remain under an explicitly allocated area as retained test evidence.
"""
import copy
import errno
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
import rust_inert_source_originals as subject
import rust_manual_mission as manual
from rust_unstarted_review_process import Failure, safe


class SourceOriginalsContracts(unittest.TestCase):
    """Exercise the actual helper API, with healthy controls before each fault."""

    def setUp(self):
        """Require an allocated non-/tmp directory; never select a fallback."""
        area = Path(os.environ['SWARM_GI_SOURCE_TEST_AREA']).resolve(strict=True)
        if not area.is_dir() or area == Path('/tmp') or Path('/tmp') in area.parents:
            raise ValueError('source-test-area-not-admitted')
        if (area/'.swarm/state.db').exists():
            raise ValueError('source-tests-refuse-live-root')
        self.area = Path(tempfile.mkdtemp(prefix='source-contract-', dir=area))

    def case_root(self, label):
        """Allocate only a synthetic receiver directory, with no RootStore/DB."""
        root = self.area/label
        root.mkdir(mode=0o700)
        return root

    def planner_case(self, label):
        """Supply schema fixtures for the REAL SR1 reader, not a native claim.

        The public-call double returns a controlled decision. Its original frame
        and I0 markers are test data; actual engine/OS identity is not established.
        """
        root = self.case_root(label)
        output = root/'evidence'
        output.mkdir()
        private = root/'.swarm'
        private.mkdir(mode=0o700)
        claim, pid, parent = 'planning-claim-unit', 7, 8
        leaf = private/'biba-i0/planning'/claim
        leaf.mkdir(parents=True)
        work = dict(id='unit-work', revision=1, planning=dict(inbox=[dict(id='event-unit', scope='root')]))
        value = dict(input_events=['event-unit'], reason='Explicit public integration fixture', operations=[])
        context = json.dumps(dict(scope=dict(id='root', revision=1), events=[dict(id='event-unit')]), separators=(',', ':')).encode()
        prompt = b'unit workflow\n\n'+context
        marker = dict(pid=pid, ppid=parent, prompt_sha256=subject.fixture.digest(prompt), prompt_bytes=len(prompt))
        (leaf/('fixture-prompt-'+str(pid)+'.bin')).write_bytes(prompt)
        (leaf/('model-received-'+str(pid)+'.json')).write_text(json.dumps(marker))
        identity = private.stat()
        job = dict(claim_id=claim, work_id='unit-work', scope_id='root', permission=1, phase='permitted',
                   directory_device=str(identity.st_dev), directory_inode=str(identity.st_ino),
                   child_pid=pid, owner_pid=parent, parent_pid=parent, group_id=pid,
                   child_stamp='unit-synthetic-child', owner_stamp='unit-synthetic-parent', owner_host='unit-not-Authority',
                   prompt_sha256=subject.fixture.digest(prompt), context_sha256=subject.fixture.digest(context),
                   scope_revision=1, generation=1)
        receipt = dict(schema_version=1, exit_code=0, signal=None, output_complete=True,
                       grant_attempted=True, failure=None, gate=None, finished_at='unit-synthetic-completion',
                       proposal=json.dumps(value), run=job)
        frame = private/('native-planning-'+subject.fixture.digest(claim.encode())+'.receipt')
        outcome = dict(run=dict(claim_id=claim, replayed=False, decision_applied=True, work=dict(id='unit-work', revision=2)))
        owned = SimpleNamespace(root=root, output=output, work='unit-work', ok=Mock(return_value=outcome))
        self.frame(frame, receipt)
        return SimpleNamespace(owned=owned, work=work, receipt=receipt, frame=frame, leaf=leaf, prompt=prompt)

    def frame(self, path, value):
        """Encode normative SR1 fixture DATA; never replace the reader/helper."""
        raw = json.dumps(value, separators=(',', ':')).encode()
        path.write_bytes(b'SR1\n'+str(len(raw)).encode()+b'\n'+subject.fixture.digest(raw).encode()+b'\n'+raw)

    def worker_case(self, label):
        """Model completed-agent readback and public submit in memory only.

        Real report/marker files are controlled fixture bytes. No actual worker
        ran: metadata here is a double, not a manufactured production receipt.
        """
        root = self.case_root(label)
        workspace = root/'i0-workspace'
        workspace.mkdir(mode=0o700)
        output = root/'evidence'
        output.mkdir()
        prompt = 'unit source prompt'
        actual = dict(pid=7, ppid=8, stdin_sha256=subject.fixture.digest(prompt.encode()))
        prepared = dict(agent=dict(id='session', attempt_id='unit-attempt', workspace=str(workspace),
                                   prompt=prompt, model_route=dict(model='fixture-unit')))
        agent = dict(id='session', work_id='unit-work', task_id='task', role='worker', attempt_id='unit-attempt',
                     workspace=str(workspace), status='completed', exit_code=0, ended='unit-synthetic-completion',
                     native_execution=dict(test_double=True), native_execution_result=dict(test_double=True),
                     child_pid=7, supervisor_pid=8)
        raw = manual.REPORT_TEMPLATE.format(input_sha256=actual['stdin_sha256'], model='fixture-unit').encode()
        (workspace/'handoff.md').write_bytes(raw)
        (workspace/'model-received.json').write_text(json.dumps(actual))
        state = dict(tasks=[dict(status='blocked', attempts=[dict(id='unit-attempt')])])
        calls = []
        def public(args, request=None):
            """API double: record reads/submits; no CLI, claims, SQL or budget."""
            calls.append(copy.deepcopy((args, request)))
            if args == ['agent', 'show', 'session']:
                return dict(agent=copy.deepcopy(agent))
            if args == ['work', 'show', 'unit-work']:
                return dict(work=dict(id='unit-work', revision=1))
            if args == ['task', 'submit', 'unit-work']:
                state['tasks'][0]['status'] = 'submitted'
                return dict(work=copy.deepcopy(state))
            raise AssertionError('unexpected public operation in controlled double')
        owned = SimpleNamespace(root=root, workspace=workspace, output=output, work='unit-work',
                                request=dict(task_id='task', workspace=str(workspace)), ok=public,
                                snapshot=lambda: (copy.deepcopy(state), b'unchanged-agent', b'unchanged-budget'))
        return SimpleNamespace(owned=owned, prepared=prepared, actual=actual, agent=agent, raw=raw, calls=calls, state=state)

    def test_embedded_proposal_with_root_input_absent(self):
        """Run only the inert string in-process with I0 cwd; no child/sandbox.

        Mutating supplied data after generation must not change its emitted
        proposal. Root-relative input is absent; real I0 witness files are read.
        """
        root = self.case_root('embedded')
        leaf = root/'i0'
        leaf.mkdir()
        value = dict(input_events=['event-unit'], reason='frozen', operations=[])
        program = subject.feedback.program_with_proposal(value)
        value['reason'] = 'changed caller data'
        self.assertFalse((root/'planning-proposal.json').exists())
        original_cwd = Path.cwd()
        stdout = io.StringIO()
        try:
            os.chdir(leaf)
            with patch.object(sys, 'stdin', SimpleNamespace(buffer=io.BytesIO(b'unit prompt'))), patch.object(sys, 'stdout', stdout), patch.object(sys, 'argv', ['codex', 'exec', '--model', 'fixture-unit', '-']):
                exec(program, dict(__name__='__main__'))
        finally:
            os.chdir(original_cwd)
        (leaf/'program.stdout.raw').write_text(stdout.getvalue())
        (leaf/'program.before.py').write_text(program)
        result = json.loads(stdout.getvalue())
        self.assertEqual(result['structured_output']['reason'], 'frozen')
        self.assertEqual(result['structured_output']['input_events'], ['event-unit'])
        self.assertEqual((leaf/('fixture-prompt-'+str(os.getpid())+'.bin')).read_bytes(), b'unit prompt')
        self.assertFalse((root/'planning-proposal.json').exists())
        with patch.object(subject.feedback, 'PROGRAM', 'no expected proposal boundary'):
            with self.assertRaises(ValueError):
                subject.feedback.program_with_proposal(value)

    def test_planner_bindings_refuse_and_preserve_originals(self):
        """Healthy real reader before each corrupted fixture frame/claim/PID/prompt.

        Only the public-call result is doubled. Failed source artifacts remain
        byte-identical, and the helper never issues an automatic second call.
        """
        for fault in ('SR1-hash', 'claim', 'PID', 'prompt'):
            with self.subTest(fault=fault):
                healthy = self.planner_case('control-'+fault)
                _, prompt, binding = subject.planner_step(healthy.owned, healthy.work, 'root', [], 'control')
                self.assertEqual(prompt, healthy.prompt)
                self.assertEqual(binding['pid'], 7)
                healthy.owned.ok.assert_called_once()
                case = self.planner_case('fault-'+fault)
                if fault == 'SR1-hash':
                    fields = case.frame.read_bytes().split(b'\n', 3)
                    fields[2] = b'0'*64
                    case.frame.write_bytes(b'\n'.join(fields))
                elif fault == 'claim':
                    case.receipt['run']['claim_id'] = 'other-claim'
                    self.frame(case.frame, case.receipt)
                elif fault == 'PID':
                    marker = case.leaf/'model-received-7.json'
                    value = json.loads(marker.read_text())
                    value['pid'] = 9
                    marker.write_text(json.dumps(value))
                else:
                    (case.leaf/'fixture-prompt-7.bin').write_bytes(b'changed source prompt')
                originals = {f: f.read_bytes() for f in [case.frame, case.leaf/'fixture-prompt-7.bin', case.leaf/'model-received-7.json']}
                with self.assertRaises(Failure):
                    subject.planner_step(case.owned, case.work, 'root', [], 'fault')
                self.assertEqual({f: f.read_bytes() for f in originals}, originals)
                case.owned.ok.assert_called_once()
                self.assertFalse((case.owned.output/'planner-originals-fault/binding.json').exists())

    def test_worker_transfer_binds_public_submit_hash(self):
        """Actual file transfer, doubled completion/API; public hash stays bound."""
        healthy = self.worker_case('worker-control')
        binding = subject.worker_report(healthy.owned, healthy.prepared, healthy.actual)
        self.assertEqual((healthy.owned.root/'handoff.md').read_bytes(), healthy.raw)
        self.assertEqual((healthy.owned.workspace/'handoff.md').read_bytes(), healthy.raw)
        manual.submit(healthy.owned, expected_sha256=binding['sha256'])
        self.assertEqual(healthy.state['tasks'][0]['status'], 'submitted')
        case = self.worker_case('worker-public-altered')
        binding = subject.worker_report(case.owned, case.prepared, case.actual)
        (case.owned.root/'handoff.md').write_bytes(b'changed public report')
        with self.assertRaises(ValueError):
            manual.submit(case.owned, expected_sha256=binding['sha256'])
        self.assertFalse(any(args[0:2] == ['task', 'submit'] for args, _ in case.calls))
        self.assertEqual(case.state['tasks'][0]['status'], 'blocked')
        self.assertEqual((case.owned.workspace/'handoff.md').read_bytes(), case.raw)

    def test_worker_source_and_completion_refuse_then_consumer_resumes(self):
        """Valid control before altered/unknown/racing source; fresh consumer only.

        Restore ORIGINAL retained fixture bytes, never template-generated report.
        Recovery reads the same doubled agent, with no provider/claim/retry/refund.
        """
        for fault in ('altered-before-read', 'unknown-completion', 'changed-between-reads'):
            with self.subTest(fault=fault):
                healthy = self.worker_case('worker-control-'+fault)
                subject.worker_report(healthy.owned, healthy.prepared, healthy.actual)
                case = self.worker_case('worker-fault-'+fault)
                source = case.owned.workspace/'handoff.md'
                preserved = source.read_bytes()
                preserved_agent = copy.deepcopy(case.agent)
                actual_open, actual_close = os.open, os.close
                report_fds, changed = [], []
                def opened(name, flags, *args, **kwargs):
                    """Observe report descriptor through the common OS seam."""
                    fd = actual_open(name, flags, *args, **kwargs)
                    if name == 'handoff.md': report_fds.append(fd)
                    return fd
                def closed(fd):
                    """Change source only after first real read/fstat/close completes."""
                    actual_close(fd)
                    if fault == 'changed-between-reads' and fd in report_fds and not changed:
                        changed.append(True)
                        source.write_bytes(b'controlled changed source')
                if fault == 'altered-before-read':
                    source.write_bytes(b'altered before observation')
                if fault == 'unknown-completion':
                    case.agent['native_execution_result'] = None
                with patch.object(subject.os, 'open', side_effect=opened), patch.object(subject.os, 'close', side_effect=closed):
                    with self.assertRaises(Failure):
                        subject.worker_report(case.owned, case.prepared, case.actual)
                self.assertFalse((case.owned.root/'handoff.md').exists())
                self.assertFalse((case.owned.output/'worker-originals/publication.observed.json').exists())
                self.assertTrue(all(args == ['agent', 'show', 'session'] for args, _ in case.calls))
                source.write_bytes(preserved)
                case.agent.clear()
                case.agent.update(preserved_agent)
                failed_output = case.owned.output
                case.owned.output = case.owned.root/'fresh-consumer-evidence'
                case.owned.output.mkdir()
                subject.worker_report(case.owned, case.prepared, case.actual)
                self.assertTrue((failed_output/'worker-originals').is_dir())
                self.assertEqual((case.owned.root/'handoff.md').read_bytes(), preserved)
                self.assertEqual(source.read_bytes(), preserved)
                self.assertTrue(all(args == ['agent', 'show', 'session'] for args, _ in case.calls))

    def test_existing_target_even_equal_or_dangling_refuses(self):
        """Real exclusive-link refusal preserves all existing target entries."""
        for fault in ('different', 'equal', 'dangling'):
            with self.subTest(fault=fault):
                root = self.case_root('existing-'+fault)
                fd = os.open(root, os.O_RDONLY|os.O_DIRECTORY)
                try:
                    subject.publish_fresh(fd, 'healthy.md', b'actual bytes')
                    target = root/'handoff.md'
                    if fault == 'dangling': target.symlink_to('missing-owned-entry')
                    else: target.write_bytes(b'actual bytes' if fault == 'equal' else b'existing original')
                    before = target.lstat()
                    with self.assertRaises(Failure) as refused:
                        subject.publish_fresh(fd, 'handoff.md', b'actual bytes')
                    self.assertTrue(any(n.get('errno') == errno.EEXIST for n in safe(refused.exception)['causes']))
                    self.assertEqual(target.lstat().st_ino, before.st_ino)
                    if fault == 'dangling': self.assertEqual(os.readlink(target), 'missing-owned-entry')
                    else: self.assertEqual(target.read_bytes(), b'actual bytes' if fault == 'equal' else b'existing original')
                    subject.publish_fresh(fd, 'fresh-recovery.md', b'actual bytes')
                    self.assertEqual((root/'fresh-recovery.md').read_bytes(), b'actual bytes')
                finally:
                    os.close(fd)

    def test_link_failure_keeps_cause_and_fresh_recovery(self):
        """Injected errors before/after real link retain cause and possible effect.

        A link call raising need not mean absence: in the after-effect double
        the actual link exists even though no successful return was observed.
        """
        for phase in ('before-effect', 'after-effect'):
            with self.subTest(phase=phase):
                root = self.case_root('link-refusal-'+phase)
                fd = os.open(root, os.O_RDONLY|os.O_DIRECTORY)
                original = PermissionError(errno.EACCES, 'controlled link refusal')
                actual_link = os.link
                def refused_link(*args, **kwargs):
                    """Model a causal before/after-effect refusal without rollback."""
                    if phase == 'after-effect':
                        actual_link(*args, **kwargs)
                    raise original
                try:
                    subject.publish_fresh(fd, 'healthy.md', b'actual bytes')
                    with patch.object(subject.os, 'link', side_effect=refused_link) as link:
                        with self.assertRaises(Failure) as refused:
                            subject.publish_fresh(fd, 'handoff.md', b'actual bytes')
                    self.assertIs(refused.exception.causes[0], original)
                    self.assertFalse(refused.exception.facts['destination_link_observed'])
                    link.assert_called_once()
                    if phase == 'before-effect':
                        self.assertFalse((root/'handoff.md').exists())
                    else:
                        self.assertEqual((root/'handoff.md').read_bytes(), b'actual bytes')
                    subject.publish_fresh(fd, 'fresh-recovery.md', b'actual bytes')
                    self.assertEqual((root/'fresh-recovery.md').read_bytes(), b'actual bytes')
                finally:
                    os.close(fd)

    def test_stage_readback_refusal_keeps_original_temporary(self):
        """Retain real staging entries on read/close/data-check refusal.

        Same API/OS seam on16/17; diagnostic assertions specify the new17
        contract, not a previously observed runtime RED. No helper is replaced.
        Healthy control precedes each fault with the same target precondition.
        A pre-existing target disproves absence inferred from no link attempt.
        """
        for phase, existing in (('identity', False), ('read', False), ('read', True), ('close', False), ('mismatch', False)):
            with self.subTest(phase=phase, existing=existing):
                root = self.case_root('stage-readback-'+phase+'-'+str(existing))
                fd = os.open(root, os.O_RDONLY|os.O_DIRECTORY)
                target = root/'handoff.md'
                if existing:
                    target.write_bytes(b'pre-existing target')
                original = OSError(errno.EIO, 'controlled stage readback refusal')
                actual_open, actual_read, actual_close, actual_fstat = os.open, os.read, os.close, os.fstat
                observed, injected = [], []
                def opened(name, flags, *args, **kwargs):
                    """Track the actual stage reader; optionally change its data first."""
                    stage_reader = isinstance(name, str) and name.startswith('.source-report-') and flags & os.O_ACCMODE == os.O_RDONLY
                    if stage_reader and phase == 'mismatch' and not injected:
                        (root/name).write_bytes(b'controlled changed stage')
                        injected.append(True)
                    result = actual_open(name, flags, *args, **kwargs)
                    stage_writer = isinstance(name, str) and name.startswith('.source-report-') and flags & os.O_ACCMODE == os.O_WRONLY
                    if (phase == 'identity' and stage_writer) or (phase != 'identity' and stage_reader):
                        observed.append((result, name))
                    return result
                def fstat(value):
                    """Refuse the new observation once; no identity is then claimed."""
                    if phase == 'identity' and any(value == item[0] for item in observed) and not injected:
                        injected.append(True)
                        raise original
                    return actual_fstat(value)
                def read(value, count):
                    """Inject once through the real reader, preserving its cleanup."""
                    if phase == 'read' and any(value == item[0] for item in observed) and not injected:
                        injected.append(True)
                        raise original
                    return actual_read(value, count)
                def closed(value):
                    """Close the stage reader for real before injected uncertainty."""
                    actual_close(value)
                    if phase == 'close' and any(value == item[0] for item in observed) and not injected:
                        injected.append(True)
                        raise original
                try:
                    subject.publish_fresh(fd, 'healthy.md', b'actual bytes')
                    with patch.object(subject.os, 'open', side_effect=opened), patch.object(subject.os, 'read', side_effect=read), patch.object(subject.os, 'close', side_effect=closed), patch.object(subject.os, 'fstat', side_effect=fstat):
                        with self.assertRaises(Failure) as refused:
                            subject.publish_fresh(fd, 'handoff.md', b'actual bytes')
                    self.assertEqual(len(injected), 1)
                    self.assertEqual(len(observed), 1)
                    temporary = root/observed[0][1]
                    expected = b'' if phase == 'identity' else (b'controlled changed stage' if phase == 'mismatch' else b'actual bytes')
                    self.assertEqual(temporary.read_bytes(), expected)
                    identity = temporary.stat()
                    facts = refused.exception.facts
                    self.assertEqual(facts.get('temporary'), temporary.name)
                    self.assertEqual(facts.get('temporary_creation_identity'), None if phase == 'identity' else dict(device=identity.st_dev, inode=identity.st_ino))
                    self.assertEqual(facts.get('stage'), 'stage-identity' if phase == 'identity' else 'stage-readback')
                    self.assertIs(facts.get('destination_attempted'), False)
                    self.assertIs(facts.get('destination_link_observed'), False)
                    self.assertIs(facts.get('no_rollback_inference'), True)
                    if phase != 'mismatch':
                        causes = refused.exception.causes if phase == 'identity' else refused.exception.causes[0].causes
                        self.assertIn(original, causes)
                        self.assertTrue(any(node.get('errno') == errno.EIO for node in safe(refused.exception)['causes']))
                    else:
                        self.assertEqual(refused.exception.causes[0].code, 'source-publication-stage-readback')
                    if existing:
                        self.assertEqual(target.read_bytes(), b'pre-existing target')
                    else:
                        self.assertFalse(target.exists())
                    subject.publish_fresh(fd, 'fresh-recovery.md', b'actual bytes')
                    self.assertEqual((root/'fresh-recovery.md').read_bytes(), b'actual bytes')
                    self.assertEqual(temporary.read_bytes(), expected)
                    self.assertEqual(temporary.stat().st_ino, identity.st_ino)
                finally:
                    actual_close(fd)

    def test_close_refusal_tracks_prelink_and_visible_postlink_state(self):
        """Real closes followed by one injected EIO; no repeated ambiguous close.

        Stage close failure leaves no destination; target-reader close failure
        leaves a REAL visible link. Injected errors are not OS fault evidence.
        """
        for phase in ('stage', 'target-readback'):
            with self.subTest(phase=phase):
                root = self.case_root('close-'+phase)
                fd = os.open(root, os.O_RDONLY|os.O_DIRECTORY)
                original = OSError(errno.EIO, 'controlled close refusal')
                actual_open, actual_close = os.open, os.close
                tracked, injected = [], []
                def opened(name, flags, *args, **kwargs):
                    """Observe only the allocated publication descriptor's identity."""
                    result = actual_open(name, flags, *args, **kwargs)
                    if (phase == 'stage' and isinstance(name, str) and name.startswith('.source-report-') and flags & os.O_WRONLY) or (phase == 'target-readback' and name == 'handoff.md'):
                        tracked.append(result)
                    return result
                def closed(value):
                    """Close once for real, then return a controlled uncertain error."""
                    actual_close(value)
                    if value in tracked and not injected:
                        injected.append(value)
                        raise original
                try:
                    subject.publish_fresh(fd, 'healthy.md', b'actual bytes')
                    with patch.object(subject.os, 'open', side_effect=opened), patch.object(subject.os, 'close', side_effect=closed):
                        with self.assertRaises(Failure) as refused:
                            subject.publish_fresh(fd, 'handoff.md', b'actual bytes')
                    self.assertEqual(len(injected), 1)
                    self.assertTrue(any(n.get('errno') == errno.EIO for n in safe(refused.exception)['causes']))
                    if phase == 'stage':
                        self.assertIn(original, refused.exception.causes)
                        self.assertFalse(refused.exception.facts['destination_attempted'])
                        self.assertFalse((root/'handoff.md').exists())
                    else:
                        self.assertIn(original, refused.exception.causes[0].causes)
                        self.assertTrue(refused.exception.facts['destination_link_observed'])
                        self.assertEqual((root/'handoff.md').read_bytes(), b'actual bytes')
                    subject.publish_fresh(fd, 'fresh-recovery.md', b'actual bytes')
                    self.assertEqual((root/'fresh-recovery.md').read_bytes(), b'actual bytes')
                finally:
                    actual_close(fd)


NAMES = [
    'test_embedded_proposal_with_root_input_absent',
    'test_planner_bindings_refuse_and_preserve_originals',
    'test_worker_transfer_binds_public_submit_hash',
    'test_worker_source_and_completion_refuse_then_consumer_resumes',
    'test_existing_target_even_equal_or_dangling_refuses',
    'test_link_failure_keeps_cause_and_fresh_recovery',
    'test_stage_readback_refusal_keeps_original_temporary',
    'test_close_refusal_tracks_prelink_and_visible_postlink_state',
]

if __name__ == '__main__':
    suite = unittest.TestSuite(SourceOriginalsContracts(name) for name in NAMES)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    complete = result.testsRun == len(NAMES) and not result.skipped and not result.expectedFailures
    origins = {}
    for name, module in sorted(sys.modules.items()):
        file = getattr(module, '__file__', None)
        if file and Path(file).is_file():
            path = Path(file).resolve()
            origins[name] = dict(path=str(path), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    resolver = sys.modules.get(subject.fixture.fixture_root.__module__)
    receipt = dict(scope='CONTROLLED_SOURCE_CONTRACTS_ONLY', test_area=os.environ['SWARM_GI_SOURCE_TEST_AREA'], fixtures_retained=True, tests_run=result.testsRun,
                   failures=len(result.failures), errors=len(result.errors), complete=complete,
                   interpreter=sys.executable, version=sys.version, sys_path=sys.path,
                   origins=origins, fixture_resolver_module=subject.fixture.fixture_root.__module__,
                   fixture_repo=str(getattr(resolver, '_REPO', None)),
                   fixture_area_cached=str(getattr(resolver, '_AREA', None)),
                   limits='Doubled public/completion records, in-process program, real bounded file operations, injected faults; not engine/OS-Authority/WF acceptance')
    # Fresh output only. A write/close failure is incomplete evidence and exits
    # nonzero; outer owner must preserve wait/streams before origin validation.
    with Path(os.environ['SWARM_GI_SOURCE_TEST_RECEIPT']).open('x') as stream:
        json.dump(receipt, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(dict(scope=receipt['scope'], tests_run=result.testsRun,
                          failures=len(result.failures), errors=len(result.errors), complete=complete)), flush=True)
    raise SystemExit(0 if complete and result.wasSuccessful() else 1)
