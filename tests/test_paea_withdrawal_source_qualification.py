"""Inert qualification-layer guards: no native CLI, child or provider executes.

subprocess.run is replaced with an explicit CompletedProcess fixture. These
checks exercise supervisor publication/refusal only, not a native source birth,
real process exit, physical attribution, H2, installation or authority.
"""
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import Mock, patch

from fixture_area import fixture_root
import paea_concurrent_withdrawal_source as source


class SourceQualificationGuards(unittest.TestCase):
    """Retain each owned inert fixture and require valid control before faults."""
    def setUp(self):
        """Allocate one retained inert directory; no native Root is initialized."""
        self.root = fixture_root('p4-qualification-inert-')

    def child(self, out, *, exit_code=0, mutation=None):
        """Return an inert subprocess replacement and retained synthetic bytes.

        No executable is opened. The fixture's PASS is deliberately untrusted;
        the real supervisor must refuse failure/incomplete or mismatched data.
        """
        def execute(command, *, stdin, capture_output):
            """Write synthetic child observations and return its declared exit."""
            self.assertTrue(capture_output)
            self.assertEqual(stdin, subprocess.DEVNULL)
            self.assertNotIn('--supervise', command)
            out.mkdir(mode=0o700)
            marker = out/'inert-marker.json'
            original = out/'inert-original.json'
            contents = {
                'inputs.json': b'{"inert":true}\n',
                'observed-publication.json': b'{"status":"SYNTHETIC_NOT_NATIVE"}\n',
                'accepted.stdout': b'{"work":{"tasks":[{"status":"accepted"}]}}\n',
                marker.name: b'inert marker\n',
                original.name: b'inert original\n',
            }
            for name, raw in contents.items():
                (out/name).write_bytes(raw)
            result = dict(status='PASS', inputs_sha256=source.fixture.digest(contents['inputs.json']),
                          observation_sha256=source.fixture.digest(contents['observed-publication.json']),
                          artifacts={name:source.fixture.digest(raw) for name,raw in contents.items()},
                          consumed_originals={str(original):source.fixture.digest(original.read_bytes())},
                          review_marker=dict(path=str(marker),sha256=source.fixture.digest(marker.read_bytes())))
            if mutation:
                mutation(result)
            (out/'result.json').write_text(json.dumps(result))
            return subprocess.CompletedProcess(command,exit_code,b'inert stdout',b'inert stderr')
        return execute

    def invoke(self, out, **options):
        """Exercise only supervise() with one mocked child and no retries."""
        with patch.object(source.subprocess,'run',side_effect=self.child(out,**options)) as child:
            try:
                return source.supervise(self.root/'never-executed-binary',self.root/'never-opened-profile',out)
            finally:
                self.assertEqual(child.call_count,1)

    def valid_control(self, label):
        """The same intact synthetic prerequisites must publish a bound receipt."""
        out=self.root/label
        self.invoke(out)
        terminal=json.loads((out/'terminal.json').read_bytes())
        self.assertEqual(terminal['status'],'QUALIFIED_P4_FRESH_SOURCE')
        self.assertEqual(terminal['exit'],0)
        self.assertEqual(terminal['result_sha256'],source.fixture.digest((out/'result.json').read_bytes()))
        self.assertEqual(terminal['process_sha256'],source.fixture.digest((out/'process.json').read_bytes()))

    def test_valid_mock_child_binds_result_and_process(self):
        """Intact mocked child evidence produces the expected hash-bound receipt."""
        self.valid_control('valid')

    def test_visible_pass_with_failed_child_is_not_qualified(self):
        """A nonzero child preserves visible PASS bytes but cannot qualify them."""
        self.valid_control('valid-before-child-failure')
        out=self.root/'failed-child'
        with self.assertRaises(AssertionError):
            self.invoke(out,exit_code=41)
        self.assertEqual(json.loads((out/'result.json').read_bytes())['status'],'PASS')
        self.assertEqual(json.loads((out/'process.json').read_bytes())['exit'],41)
        self.assertFalse((out/'terminal.json').exists())
        self.assertEqual((out/'process.stderr').read_bytes(),b'inert stderr')

    def test_visible_pass_with_incomplete_child_result_is_not_qualified(self):
        """A missing required hash refuses qualification even after exit zero."""
        self.valid_control('valid-before-incomplete')
        out=self.root/'incomplete'
        with self.assertRaises(KeyError):
            self.invoke(out,mutation=lambda result:result.pop('inputs_sha256'))
        self.assertEqual(json.loads((out/'process.json').read_bytes())['exit'],0)
        self.assertEqual(json.loads((out/'result.json').read_bytes())['status'],'PASS')
        self.assertFalse((out/'terminal.json').exists())

    def test_visible_pass_with_wrong_artifact_hash_is_not_qualified(self):
        """A mismatched retained-artifact digest leaves no terminal receipt."""
        self.valid_control('valid-before-wrong-hash')
        out=self.root/'wrong-hash'
        def corrupt(result):
            """Inject one digest mismatch into the owned synthetic result only."""
            result['artifacts']['accepted.stdout']='0'*64
        with self.assertRaises(AssertionError):
            self.invoke(out,mutation=corrupt)
        self.assertEqual(json.loads((out/'process.json').read_bytes())['exit'],0)
        self.assertFalse((out/'terminal.json').exists())
        self.assertEqual(json.loads((out/'accepted.stdout').read_bytes())['work']['tasks'][0]['status'],'accepted')



class AutomaticSourceGuards(unittest.TestCase):
    """Pure stage guards; no native command, conductor or provider executes."""
    def setUp(self):
        """Retain a synthetic directory without creating a native datastore."""
        self.root = fixture_root('p4-automatic-inert-')

    def prerequisites(self):
        """Intact public projections for one TODO task and no prior worker."""
        return (dict(id='work', planning=dict(paused=True), tasks=[dict(id='task',status='todo',attempts=[])]),
                dict(agents=[],controls=dict(paused=False,consent_enabled=True,slots=1,archived=False)))

    def event(self, kind, **fields):
        """Construct a synthetic observation, never an execution grant."""
        return dict(schema_version=1,work='work',event=kind,**fields)

    def valid_progress(self):
        """A complete healthy event control precedes every injected failure."""
        progress=source.AutomaticProgress('work')
        self.assertFalse(progress.accept(self.event('mission-worker-started',agent_id='worker')))
        self.assertFalse(progress.accept(self.event('mission-worker-exited',agent_id='worker',exit_code=0,exit_signal=None)))
        self.assertTrue(progress.accept(self.event('mission-report-submitted',agent_id='worker',report='retained/report.md',task_accepted=False)))

    def test_completed_manual_worker_refuses_before_popen(self):
        """Observed manual/blocked state cannot launch the old useless watch."""
        work,native=self.prerequisites()
        source.require_fresh_automatic_work(work,native)
        manual=dict(id='worker',status='completed',role='worker',native_mission=None)
        work['tasks'][0].update(status='blocked',attempts=[dict(id='attempt',status='completed')])
        native['agents']=[manual]
        owned=Mock(work='work')
        owned.ok.side_effect=[{'work':work},native]
        with patch.object(source.subprocess,'Popen') as launch:
            with self.assertRaises(source.AutomaticSourceRefused) as caught:
                source.automatic_handoff(owned,self.root/'must-not-start')
            launch.assert_not_called()
        self.assertIs(caught.exception.observation['native']['agents'][0],manual)
        self.assertEqual(caught.exception.reason,'fresh-worker-required')

    def test_progress_waits_for_same_worker_exit_and_handoff(self):
        """Submission alone does not prove the actual helper was waited."""
        self.valid_progress()
        progress=source.AutomaticProgress('work')
        self.assertFalse(progress.accept(self.event('mission-worker-started',agent_id='worker')))
        self.assertFalse(progress.accept(self.event('mission-report-submitted',agent_id='worker',report='retained/report.md',task_accepted=False)))
        self.assertTrue(progress.accept(self.event('mission-worker-exited',agent_id='worker',exit_code=0,exit_signal=None)))

    def test_terminal_worker_failure_keeps_payload(self):
        """An actual nonzero helper exit remains terminal with its original data."""
        self.valid_progress()
        progress=source.AutomaticProgress('work')
        progress.accept(self.event('mission-worker-started',agent_id='worker'))
        failed=self.event('mission-worker-exited',agent_id='worker',exit_code=17,exit_signal=None)
        with self.assertRaises(source.AutomaticSourceRefused) as caught:
            progress.accept(failed)
        self.assertIs(caught.exception.observation,failed)

    def test_waiting_before_exit_can_later_submit(self):
        """Regression: started, transient waiting, wait0, then handoff succeeds."""
        self.valid_progress()
        progress=source.AutomaticProgress('work')
        self.assertFalse(progress.accept(self.event('mission-worker-started',agent_id='worker')))
        self.assertFalse(progress.accept(self.event('mission-report-waiting',agent_id='worker',reason='managed-operation-busy')))
        self.assertFalse(progress.accept(self.event('mission-worker-exited',agent_id='worker',exit_code=0,exit_signal=None)))
        self.assertTrue(progress.accept(self.event('mission-report-submitted',agent_id='worker',report='retained/report.md',task_accepted=False)))

    def test_waiting_after_exit_and_blocked_can_later_submit(self):
        """Transient lane contention/readiness after wait0 is not terminal."""
        self.valid_progress()
        progress=source.AutomaticProgress('work')
        self.assertFalse(progress.accept(self.event('mission-worker-started',agent_id='worker')))
        self.assertFalse(progress.accept(self.event('mission-worker-exited',agent_id='worker',exit_code=0,exit_signal=None)))
        self.assertFalse(progress.accept(self.event('mission-report-waiting',agent_id='worker',reason='managed-operation-busy')))
        self.assertFalse(progress.accept(self.event('mission-watch-blocked',reason='no-ready-worker',ready_count=0)))
        self.assertTrue(progress.accept(self.event('mission-report-submitted',agent_id='worker',report='retained/report.md',task_accepted=False)))

    def test_stop_before_handoff_is_refused(self):
        """A real watch stop without handoff is still a terminal incomplete run."""
        self.valid_progress()
        progress=source.AutomaticProgress('work')
        progress.accept(self.event('mission-worker-started',agent_id='worker'))
        progress.accept(self.event('mission-worker-exited',agent_id='worker',exit_code=0,exit_signal=None))
        stopped=self.event('mission-watch-stopped',reason='sigint')
        with self.assertRaises(source.AutomaticSourceRefused) as caught:
            progress.accept(stopped)
        self.assertIs(caught.exception.observation,stopped)

    def test_unpaused_planning_refuses_before_popen(self):
        """Protect the declared worker-only stage from implicit role calls."""
        work,native=self.prerequisites()
        source.require_fresh_automatic_work(work,native)
        work['planning']['paused']=False
        owned=Mock(work='work')
        owned.ok.side_effect=[{'work':work},native]
        with patch.object(source.subprocess,'Popen') as launch:
            with self.assertRaises(source.AutomaticSourceRefused):
                source.automatic_handoff(owned,self.root/'must-not-start')
            launch.assert_not_called()


if __name__=='__main__':
    unittest.main(verbosity=2)
