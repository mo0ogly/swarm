#!/usr/bin/env python3
"""Pure v3 decoder/ownership/EOF guards; no SDK, CLI, provider or subprocess runs."""
import copy
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

import rust_mission_legacy_v3_intent as subject


class LegacyV3Guards(unittest.TestCase):
    """Exercise original-v3 generation, exact EOF and exceptional owned SDK cleanup."""

    @staticmethod
    def binding():
        """Return decoder input only, without executable authority or persisted grant."""
        return dict(schema_version=3, parent_profile_sha256='a' * 64, workspace_turn_seq='7')

    @classmethod
    def resources(cls):
        """Build SQL/DTO observations for an original generation, never a live fixture."""
        agent = dict(id='agent', task_id='task', native_mission=cls.binding(), ended='later')
        turn = dict(seq='7', work='work', task='task', workspace='owned', agent='agent', state='acquired',
                    requested_at='first', acquired_at='second', released_at='')
        row = dict(event_seq=11, turn_seq=7, work_id='work', task_id='task', workspace='owned', agent_id='agent',
                   state='acquired', transition='acquired', requested_at='first', acquired_at='second', released_at='')
        before = dict(turn=turn, journal=[row], reservation=['reserved', 1.0])
        after = copy.deepcopy(before)
        after['turn'].update(state='released', released_at='later')
        after['reservation'][0] = 'released'
        after['journal'].append(dict(row, event_seq=12, state='released', transition='released', released_at='later'))
        return agent, before, after

    def test_binding3_requires_historical_absence_and_canonical_positive_sequence(self):
        """Historical shape passes first; v4/null authority, numeric schema/seq and overflow refuse."""
        subject.assert_binding3(self.binding())
        for key, value in [('schema_version', True), ('schema_version', 3.0), ('schema_version', 4),
                           ('root_instance', None), ('root_instance', {}), ('parent_profile_sha256', 'A'*64),
                           ('workspace_turn_seq', 7), ('workspace_turn_seq', '07'), ('workspace_turn_seq', '0'),
                           ('workspace_turn_seq', str(2**63)), ('workspace_turn_seq', '٧')]:
            with self.subTest(key=key, value=value), self.assertRaises(RuntimeError):
                subject.assert_binding3(dict(self.binding(), **{key:value}))

    def test_acquired_generation_and_owner_match_actual_journal(self):
        """Held snapshot witnesses first; wrong Agent/generation/transition cannot supply ownership."""
        agent, before, _ = self.resources()
        subject.acquired(before, agent, 'work')
        for target, key, value in [('turn', 'agent', 'other'), ('turn', 'seq', '8'),
                                   ('journal', 'turn_seq', True), ('journal', 'work_id', 'other'),
                                   ('journal', 'transition', 'released')]:
            bad = copy.deepcopy(before)
            (bad['journal'][-1] if target == 'journal' else bad[target])[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                subject.acquired(bad, agent, 'work')

    def test_release_preserves_exact_old_history_and_reservation_amount_type(self):
        """One matching release passes; mutations to old rows, ownership or typed amount refuse."""
        agent, before, after = self.resources()
        subject.released(before, after, agent)
        bads = []
        bad = copy.deepcopy(after); bad['journal'][0]['workspace']='changed'; bads.append(bad)
        bad = copy.deepcopy(after); bad['journal'][-1]['agent_id']='other'; bads.append(bad)
        bad = copy.deepcopy(after); bad['journal'][-1]['turn_seq']=True; bads.append(bad)
        bad = copy.deepcopy(after); bad['reservation'][1]=True; bads.append(bad)
        bad = copy.deepcopy(after); bad['turn']['seq']='8'; bads.append(bad)
        bad = copy.deepcopy(after); bad['journal'].append(copy.deepcopy(bad['journal'][-1])); bads.append(bad)
        for bad in bads:
            with self.assertRaises(RuntimeError):
                subject.released(before, bad, agent)

    def test_eof_accepts_only_exact_typed_action_failure_and_nonzero_real_exit(self):
        """Source-derived EOF witness passes; arbitrary text, IO, extra cleanup and coercion refuse."""
        failure, cycle = subject.eof_documents()
        subject.verify_eof(failure, cycle, 1)
        for code in (0, True, 1.0, None):
            with self.assertRaises(RuntimeError):
                subject.verify_eof(failure, cycle, code)
        for mutation in ('phase', 'io', 'secondary', 'boolean', 'message'):
            bad, bad_cycle = copy.deepcopy(failure), copy.deepcopy(cycle)
            if mutation == 'phase': bad_cycle['diagnostics']['causes'][0]['phase']='cleanup'
            if mutation == 'io': bad_cycle['causes'][-1]['io_errno']=32
            if mutation == 'secondary': bad['causes'].append([dict(labels=['root-storage'],sqlite_code=5,io_errno=None)])
            if mutation == 'boolean': bad['child_ownership']['spawned_publication_confirmed']=0
            if mutation == 'message': bad['causes']=[[dict(message='watch-fixture-input-protocol')]]
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                subject.verify_eof(bad, bad_cycle, 1)

    def test_optimized_setup_refuses_before_request_or_archived_import(self):
        """Enabled asserts pass; optimization refuses before even reading the child request."""
        self.assertEqual(subject.runtime_flags()['optimize'], 0)
        with patch.object(subject.sys, 'flags', types.SimpleNamespace(optimize=1)), \
             patch.object(subject.importlib, 'import_module') as imports:
            with self.assertRaises(RuntimeError):
                subject.setup_child(Path('/absent-owned-request'))
            imports.assert_not_called()

    def test_bundle_requires_exact_qualified_old_binary_and_confined_source_bytes(self):
        """A minimal inventory is valid locally; changed bytes, oldbinary or escaping paths refuse."""
        with tempfile.TemporaryDirectory() as temporary:
            base=Path(temporary); root=base/'root'; (root/'tests').mkdir(parents=True)
            source=root/'tests/rust_owned.py'; source.write_text('x=1\n')
            binary=base/'old'; binary.write_bytes(b'normal-qualified')
            row=dict(path='root/tests/rust_owned.py',bytes=source.stat().st_size,sha256=subject.digest(source.read_bytes()))
            value=dict(schema_version=1,binding_schema=3,helpers_root='root',files=[row],missing_local_imports=[],
                       entrypoint='tests/rust_planning_worker_public_flow.py:flow',source_recipe_result=dict(status='PASS'),
                       old_binary=dict(sha256=subject.digest(binary.read_bytes())))
            manifest=base/'manifest.json'; manifest.write_text(json.dumps(value))
            _,files,_=subject.bundle_inputs(manifest,root,binary)
            self.assertEqual(set(files),{source})
            source.write_text('x=2\n')
            with self.assertRaises(RuntimeError):subject.bundle_inputs(manifest,root,binary)
            source.write_text('x=1\n'); binary.write_bytes(b'wrong')
            with self.assertRaises(RuntimeError):subject.bundle_inputs(manifest,root,binary)
            binary.write_bytes(b'normal-qualified'); value['files'][0]['path']='../outside.py'; manifest.write_text(json.dumps(value))
            with self.assertRaises(RuntimeError):subject.bundle_inputs(manifest,root,binary)

    def test_loaded_historical_imports_reject_current_fallback_and_missing_module(self):
        """Only exact loaded rust_* inventory qualifies; preserve real stdlib modules in mocks."""
        with tempfile.TemporaryDirectory() as temporary:
            source=Path(temporary)/'rust_owned.py'; source.write_text('x=1\n')
            files={source:subject.digest(source.read_bytes())}
            kept={name:value for name,value in sys.modules.items() if not name.startswith('rust_')}
            kept['rust_owned']=types.SimpleNamespace(__file__=str(source))
            with patch.dict(sys.modules,kept,clear=True):
                self.assertEqual(set(subject.old_imports(files)),{'rust_owned'})
                sys.modules['rust_current']=types.SimpleNamespace(__file__=__file__)
                with self.assertRaises(RuntimeError):subject.old_imports(files)
                del sys.modules['rust_current']; del sys.modules['rust_owned']
                with self.assertRaises(RuntimeError):subject.old_imports(files)

    def test_normal_eof_closes_only_stdin_and_actually_reaps_without_signal(self):
        """Normal protocol never writes resume; wait and publication remain independently checked."""
        peer=Mock(pid=101,returncode=1); peer.poll.return_value=None
        stream=peer.stdin
        with patch.object(subject,'publish') as publications,patch.object(signal,'pidfd_send_signal') as signals:
            self.assertEqual(subject.stop_probe(peer,9,dict(pid=101,stamp='stamp'),Path('/owned')),1)
        stream.close.assert_called_once(); stream.write.assert_not_called(); peer.wait.assert_called_once()
        signals.assert_not_called(); self.assertTrue(publications.call_args.args[1]['actual_popen_wait'])

    def test_pipe_close_failure_signals_only_fresh_owned_pidfd_then_reaps_but_stays_failed(self):
        """Bounded exceptional cleanup preserves close cause, signals held SDK only and never qualifies EOF."""
        original=OSError('close-failed')
        peer=Mock(pid=101,returncode=-15); peer.stdin.close.side_effect=original; peer.poll.return_value=None
        watch=types.SimpleNamespace(observation=Mock(return_value=dict(pid=101,stamp='s',ppid=os.getpid())))
        helpers=types.SimpleNamespace(current=lambda:types.SimpleNamespace(watch=watch))
        with patch.object(subject,'shared',return_value=helpers),patch.object(subject,'publish'), \
             patch.object(signal,'pidfd_send_signal') as signals:
            with self.assertRaises(BaseExceptionGroup) as caught:
                subject.stop_probe(peer,9,dict(pid=101,stamp='s'),Path('/owned'))
        self.assertIn(original,caught.exception.exceptions)
        signals.assert_called_once_with(9,signal.SIGTERM); peer.wait.assert_called_once()

    def test_changed_sdk_identity_refuses_signal_and_records_unresolved_without_fake_reap(self):
        """Stamp/parent mismatches cannot signal; preserve original close plus ownership error."""
        for observed in (dict(pid=101,stamp='changed',ppid=os.getpid()),dict(pid=101,stamp='s',ppid=os.getpid()+1)):
            peer=Mock(pid=101); original=OSError('close'); peer.stdin.close.side_effect=original; peer.poll.return_value=None
            helpers=types.SimpleNamespace(current=lambda:types.SimpleNamespace(watch=types.SimpleNamespace(observation=lambda pid:observed)))
            with patch.object(subject,'shared',return_value=helpers),patch.object(subject,'publish') as publications, \
                 patch.object(signal,'pidfd_send_signal') as signals:
                with self.assertRaises(BaseExceptionGroup) as caught:
                    subject.stop_probe(peer,9,dict(pid=101,stamp='s'),Path('/owned'))
            self.assertIn(original,caught.exception.exceptions); signals.assert_not_called(); peer.wait.assert_not_called()
            self.assertEqual(publications.call_args.args[1]['status'],'FAIL_KNOWN_SDK_NOT_REAPED')

    def test_signal_and_diagnostic_publication_failures_preserve_all_causes(self):
        """An unsuccessful signal does not enter an unsupervised wait or erase publication/close failures."""
        primary=OSError('close'); signal_failure=OSError('signal'); publication=OSError('publish')
        peer=Mock(pid=101); peer.stdin.close.side_effect=primary; peer.poll.return_value=None
        helpers=types.SimpleNamespace(current=lambda:types.SimpleNamespace(watch=types.SimpleNamespace(observation=lambda pid:dict(pid=101,stamp='s',ppid=os.getpid()))))
        with patch.object(subject,'shared',return_value=helpers),patch.object(subject,'publish',side_effect=publication), \
             patch.object(signal,'pidfd_send_signal',side_effect=signal_failure):
            with self.assertRaises(BaseExceptionGroup) as caught:
                subject.stop_probe(peer,9,dict(pid=101,stamp='s'),Path('/owned'))
        self.assertEqual(caught.exception.exceptions,(primary,signal_failure,publication)); peer.wait.assert_not_called()

    def test_wait_and_exit_publication_failures_never_qualify_expected_eof(self):
        """Actual wait failure and evidence IO are independent terminal failures, never inferred closure."""
        for phase in ('wait','publication'):
            peer=Mock(pid=101,returncode=1); peer.poll.return_value=1
            error=OSError(phase)
            if phase=='wait':peer.wait.side_effect=error
            with patch.object(subject,'publish',side_effect=error if phase=='publication' else None), \
                 self.assertRaises(BaseExceptionGroup) as caught:
                subject.stop_probe(peer,9,dict(pid=101,stamp='s'),Path('/owned'))
            self.assertIn(error,caught.exception.exceptions)


if __name__ == '__main__':
    unittest.main()
