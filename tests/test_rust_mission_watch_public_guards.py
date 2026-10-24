"""Public watch journey guard tests; no product processes or providers execute."""
import copy
import signal
from pathlib import Path
import json
import unittest
from unittest import mock

import rust_mission_watch_public as public


class PublicWatchGuardTests(unittest.TestCase):
    """Valid controls precede malformed/empty/overclaim fixtures and recovery."""
    def events(self):
        """Return the explicitly native schema1 one-worker SIGINT control."""
        return [dict(schema_version=1,event='mission-watch-started',work='w',generation=1,
                     poll_interval_ms='100',settings_revision='1',mission_complete=False,observation_is_permission=False),
                dict(schema_version=1,event='mission-worker-started',work='w',agent_id='a',supervisor_pid=123,
                     session_state='pending',provider_start_verified=False,mission_complete=False),
                dict(schema_version=1,event='mission-watch-stopped',work='w',reason='sigint',children_still_running=1,
                     lease_released=True,root_closed=True,mission_complete=False)]

    def check_events(self, events):
        """Use the real JSONL validator with fixed explicit fixture identities."""
        raw = b'\n'.join(json.dumps(event).encode() for event in events) + b'\n'
        return public.public_watch_events(raw, 'w', 'a', 'sigint')

    def test_canonical_desired_omitted_empty_and_present_strings(self):
        """Canonical omission is empty String, never a policy or scalar fallback."""
        self.assertEqual(public.canonical_desired({}), '')
        for value in ('', 'stop', '0'):
            self.assertEqual(public.canonical_desired({'desired': value}), value)
        for value in (None, 0, False, [], {}):
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                public.canonical_desired({'desired': value})
        self.assertEqual(public.canonical_desired({}), '')

    def test_known_provider_cleanup_signals_only_stamp_bound_adopted_peer(self):
        """Valid ownership precedes mismatched stamp and nonchild refusal controls."""
        identity=dict(pid=42,stamp='original')
        for stamp, expected in (('original', True), ('changed', False)):
            with mock.patch.object(public.os,'waitpid',side_effect=[(0,0),(42,15)]) as waited, \
                 mock.patch.object(public.os,'getpid',return_value=11), \
                 mock.patch.object(public.select,'select',return_value=([],[],[])), \
                 mock.patch.object(public.worker,'observation',return_value=dict(stamp=stamp,ppid=11,state='S')), \
                 mock.patch.object(public.signal,'pidfd_send_signal') as sent, \
                 mock.patch.object(public.fixture,'publish'):
                if expected:
                    public.reap_known_provider(identity,7,Path('/owned-output'))
                    sent.assert_called_once_with(7,signal.SIGTERM)
                    self.assertEqual(waited.call_args_list[-1].args,(42,0))
                else:
                    with self.assertRaises(RuntimeError):
                        public.reap_known_provider(identity,7,Path('/owned-output'))
                    sent.assert_not_called()
        with mock.patch.object(public.os,'waitpid',side_effect=ChildProcessError), \
             mock.patch.object(public.select,'select',return_value=([],[],[])), \
             mock.patch.object(public.signal,'pidfd_send_signal') as sent:
            with self.assertRaises(RuntimeError):
                public.reap_known_provider(identity,7,Path('/owned-output'))
            sent.assert_not_called()

    def test_known_provider_exit_is_reaped_without_signal(self):
        """Actual terminal child and already-reaped peer both avoid live-PID actions."""
        identity=dict(pid=42,stamp='original')
        for outcome in ((42,0), ChildProcessError()):
            with mock.patch.object(public.os,'waitpid',side_effect=[outcome]) as waited, \
                 mock.patch.object(public.select,'select',return_value=([7],[],[])), \
                 mock.patch.object(public.signal,'pidfd_send_signal') as sent, \
                 mock.patch.object(public.fixture,'publish') as publish:
                public.reap_known_provider(identity,7,Path('/owned-output'))
                sent.assert_not_called()
                publish.assert_called_once()
                waited.assert_called_once_with(42,public.os.WNOHANG)

    def test_cleanup_helper_live_terminal_and_replaced_stamp_are_distinct(self):
        """Live ownership witness precedes terminal race and replacement refusals."""
        identity=dict(pid=42,stamp='original',ppid=11,state='S')
        for state,stamp,allowed,signalled in [('S','original',True,True),('Z','original',True,False),('S','changed',False,False),('S','original',True,True)]:
            after={**identity,'state':state,'stamp':stamp}
            with mock.patch.object(public.os,'getpid',return_value=11), \
                 mock.patch.object(public.os,'waitpid',side_effect=[(0,0),(42,0)]), \
                 mock.patch.object(public.os,'pidfd_open',return_value=7), \
                 mock.patch.object(public.worker,'observation',side_effect=[identity,after]), \
                 mock.patch.object(public.worker,'close_descriptor') as closed, \
                 mock.patch.object(public.select,'select',return_value=([],[],[])), \
                 mock.patch.object(public.signal,'pidfd_send_signal') as sent:
                if allowed:
                    result=public.reap_cleanup_helper(identity)
                    self.assertEqual(result['pid'],42)
                    self.assertEqual(result['signal_sent'],signalled)
                else:
                    with self.assertRaises(RuntimeError):public.reap_cleanup_helper(identity)
                self.assertEqual(sent.called,signalled)
                closed.assert_called_once()

    def test_complete_native_event_control(self):
        self.assertEqual(len(self.check_events(self.events())), 3)

    def test_optional_idle_planning_exact_record_refusal_and_recovery(self):
        """Already-planned watch accepts exact idle only, never extra model activity."""
        original=self.events();self.check_events(original)
        idle=dict(schema_version=1,event='mission-planning-step',work='w',status='idle',
                  selected_scope=None,input_event=None,claim_id=None,settings_revision='7',
                  decision_applied=False,work_revision=8,mission_complete=False,
                  automatic_planner=True,automatic_reviewer=False,automatic_validation=False,
                  observation_is_permission=False)
        control=[original[0],idle,*original[1:]]
        self.assertEqual(self.check_events(control),control)
        variants=[('status','applied'),('work','other'),('schema_version',True),('schema_version','1'),
                  ('settings_revision',1),('settings_revision',True),('settings_revision','01'),
                  ('settings_revision','0'),('settings_revision','١'),('settings_revision',None),
                  ('work_revision',True),('work_revision',0),('work_revision','8'),('work_revision',None),
                  ('selected_scope','root'),('input_event','input'),('claim_id','claim'),
                  ('selected_scope',False),('input_event',{}),('claim_id',0),
                  ('decision_applied',True),('decision_applied',0),('mission_complete',True),
                  ('automatic_planner',False),('automatic_planner',1),('automatic_reviewer',True),
                  ('automatic_validation',None),('observation_is_permission',True),
                  ('prompt','private fixture raw'),('raw_stdout','unparsed'),('raw_stderr','unparsed')]
        for key,value in variants:
            invalid=copy.deepcopy(control);invalid[1][key]=value
            with self.subTest(key=key,value=value),self.assertRaises(RuntimeError):self.check_events(invalid)
        for key in idle:
            invalid=copy.deepcopy(control);del invalid[1][key]
            with self.subTest(missing=key),self.assertRaises(RuntimeError):self.check_events(invalid)
        applied=copy.deepcopy(control);applied[1].update(status='applied',selected_scope='root',
            input_event='input',claim_id='claim',decision_applied=True)
        with self.assertRaises(RuntimeError):self.check_events(applied)
        self.assertEqual(self.check_events(control),control)
        self.assertEqual(self.check_events(original),original)

    def test_empty_wrong_work_and_duplicate_start_refuse(self):
        self.check_events(self.events())
        wrong = self.events();wrong[1]['work'] = 'other'
        for invalid in ([], wrong, [self.events()[0], *self.events()]):
            with self.subTest(invalid=invalid), self.assertRaises(RuntimeError):
                self.check_events(invalid)
        self.check_events(self.events())

    def test_provider_or_mission_permission_overclaim_refuses(self):
        self.check_events(self.events())
        for index, field in ((0,'observation_is_permission'),(1,'provider_start_verified'),(2,'mission_complete')):
            invalid=self.events();invalid[index][field]=True
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.check_events(invalid)
        self.check_events(self.events())

    def test_cleanup_boolean_count_and_wrong_worker_refuse(self):
        self.check_events(self.events())
        for index, field, value in ((2,'children_still_running',True),(2,'children_still_running',0),
                                    (2,'lease_released',False),(2,'root_closed',False),
                                    (2,'reason','sigterm'),(1,'agent_id','other')):
            invalid=self.events();invalid[index][field]=value
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                self.check_events(invalid)
        self.check_events(self.events())

    def test_exit_claim_before_release_refused(self):
        self.check_events(self.events())
        invalid=self.events();invalid.insert(2,dict(schema_version=1,event='mission-worker-exited',work='w',agent_id='a',exit_code=0))
        with self.assertRaises(RuntimeError):
            self.check_events(invalid)
        self.check_events(self.events())

    def test_control_uses_latest_public_revision(self):
        owned=mock.Mock(work='w')
        owned.ok.side_effect=[{'work':{'revision':7}},{'revision':8}]
        self.assertEqual(public.mission_control(owned,'pause','owned-event'),{'revision':8})
        args, request=owned.ok.call_args.args
        self.assertEqual(args,['mission','pause','w'])
        self.assertEqual(request['expected_revision'],7)
        self.assertEqual(request['event_id'],'owned-event')

    def test_status_requires_unique_actual_target(self):
        good=dict(schema_version=1,work='w',revision=1,
                  controls=dict(revision=1,paused=False,autonomy='autonome',slots=1,consent_enabled=True,archived=False),
                  supervision=None,limitations=['stored-registration-is-not-process-liveness'],
                  agents=[dict(id='a',task='task',status='running',desired='running',supervisor_pid=11,child_pid=12)])
        owned=mock.Mock(work='w')
        owned.ok.return_value=good
        self.assertEqual(public.status_native(owned,'a'),good)
        for change in ({'agents':[]},{'agents':good['agents']*2}, {'schema_version':True},{'revision':True},{'work':'other'}):
            owned.ok.return_value={**good,**change}
            with self.subTest(change=change), self.assertRaises(RuntimeError):
                public.status_native(owned,'a')
        owned.ok.return_value=good
        self.assertEqual(public.status_native(owned,'a'),good)


if __name__=='__main__':
    unittest.main(verbosity=2)
