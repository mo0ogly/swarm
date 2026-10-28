"""Verify the owned Git fault device before trusting a negative journey."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from rust_managed_result_recovery import publication_fault_git
from rust_managed_result_recovery import watch_reports
import rust_managed_result_recovery as recovery
from unittest.mock import patch


class PublicationFaultDevice(unittest.TestCase):
    """Forward real command outcomes and inject only after the matching success."""
    def test_real_success_control_precedes_publication_refusal_device(self):
        """Exact argv and unchanged streams/status establish the device's trigger."""
        with tempfile.TemporaryDirectory(prefix='managed-result-device-') as directory:
            root=Path(directory);log=root/'argv.json';real=root/'real-git'
            real.write_text(f'#!{sys.executable}\nimport json,sys\nfrom pathlib import Path\nPath({str(log)!r}).write_text(json.dumps(sys.argv[1:]))\nprint("git-stdout")\nprint("git-stderr",file=sys.stderr)\nsys.exit(7 if "fail" in sys.argv else 0)\n')
            real.chmod(0o700);target=root/'result/report.md';marker=root/'injected.json';wrapper=root/'git'
            ref='refs/swarm/attempts/agent'
            wrapper.write_bytes(publication_fault_git(real,target,ref,marker));wrapper.chmod(0o700)
            for args,code in [(['status'],0),(['update-ref',ref,'abc','fail'],7),(['update-ref',ref+'-other','abc',''],0)]:
                value=subprocess.run([str(wrapper),*args],capture_output=True)
                self.assertEqual(value.returncode,code);self.assertEqual(json.loads(log.read_bytes()),args)
                self.assertEqual(value.stdout,b'git-stdout\n');self.assertEqual(value.stderr,b'git-stderr\n')
                self.assertFalse(target.exists());self.assertFalse(marker.exists())
            value=subprocess.run([str(wrapper),'update-ref',ref,'abc',''],capture_output=True)
            self.assertEqual(value.returncode,0);self.assertTrue(target.is_dir());self.assertEqual(list(target.iterdir()),[])
            self.assertEqual(json.loads(marker.read_bytes()),dict(git_success=True,ref=ref,target=str(target)))

    def test_existing_original_is_preserved_and_injection_error_is_visible(self):
        """A conflicting target cannot silently become a successful fault test."""
        with tempfile.TemporaryDirectory(prefix='managed-result-conflict-') as directory:
            root=Path(directory);real=root/'real-git';real.write_text(f'#!{sys.executable}\n');real.chmod(0o700)
            target=root/'report.md';target.write_bytes(b'original');marker=root/'marker';wrapper=root/'git'
            wrapper.write_bytes(publication_fault_git(real,target,'refs/swarm/attempts/agent',marker));wrapper.chmod(0o700)
            valid=subprocess.run([str(wrapper),'status'],capture_output=True)
            self.assertEqual(valid.returncode,0);self.assertEqual(target.read_bytes(),b'original')
            failed=subprocess.run([str(wrapper),'update-ref','refs/swarm/attempts/agent','abc',''],capture_output=True)
            self.assertNotEqual(failed.returncode,0);self.assertIn(b'FileExistsError',failed.stderr)
            self.assertEqual(target.read_bytes(),b'original');self.assertFalse(marker.exists())


class WatchDiagnostics(unittest.TestCase):
    """A real child exposes pipe backpressure without a deadline or stalled test."""
    def test_small_control_then_large_diagnostic_both_retain_handoff(self):
        """Nonblocking stderr proves a full pipe rather than silently waiting.

        The old unread-pipe collector receives diagnostic-backpressure and fails;
        a regular retained stderr file accepts all exact bytes and the handoff.
        Byte counts are fault-fixture inputs, never product policy or a watchdog.
        """
        import os
        with tempfile.TemporaryDirectory(prefix='managed-watch-diagnostics-') as directory:
            root=Path(directory);binary=root/'swarm-fixture'
            binary.write_text(f'''#!{sys.executable}
import json,os,signal,sys
signal.signal(signal.SIGINT,lambda *args:sys.exit(0))
os.set_blocking(2,False)
payload=b'D'*int(os.environ['DIAGNOSTIC_BYTES'])
try:
    while payload:
        count=os.write(2,payload)
        payload=payload[count:]
except BlockingIOError:
    print(json.dumps(dict(event='diagnostic-backpressure')),flush=True)
    sys.exit(3)
print(json.dumps(dict(event='mission-report-submitted',agent_id='agent')),flush=True)
signal.pause()
''');binary.chmod(0o700)
            def ok(args,value=None):
                """Only stand in for fixture work/revision and public controls."""
                return dict(work=dict(revision=1)) if args[:2]==['work','show'] else {}
            for size in [16,512*1024]:
                owned=SimpleNamespace(binary=binary,root=root,work='owned',environment=dict(os.environ,DIAGNOSTIC_BYTES=str(size)),ok=ok)
                output=root/('diagnostic-'+str(size))
                self.assertEqual(watch_reports(owned,{'agent'},output),{'agent'})
                self.assertEqual((output/'watch.stderr').read_bytes(),b'D'*size)

    def test_invalid_stdout_line_is_retained_before_json_refusal(self):
        """A codec error preserves the consumed original, not only later tail."""
        import os
        with tempfile.TemporaryDirectory(prefix='managed-watch-codec-') as directory:
            root=Path(directory);binary=root/'swarm-fixture'
            binary.write_text(f'#!{sys.executable}\nimport sys\nsys.stdout.buffer.write(b"invalid-frame\\n");sys.stdout.flush()\n')
            binary.chmod(0o700)
            owned=SimpleNamespace(binary=binary,root=root,work='owned',environment=dict(os.environ),
                ok=lambda args,value=None:dict(work=dict(revision=1)))
            output=root/'codec'
            with self.assertRaises(BaseExceptionGroup):watch_reports(owned,{'agent'},output)
            self.assertEqual((output/'watch.stdout').read_bytes(),b'invalid-frame\n')


class RecoveryCleanup(unittest.TestCase):
    """Helper-only doubles do not establish engine completion or authority."""
    def test_recovery_and_restore_failures_both_survive(self):
        """The parked original survives a failed recovery and failed restoration."""
        import os
        with tempfile.TemporaryDirectory(prefix='managed-recovery-cleanup-') as directory:
            root=Path(directory);storage=root/'managed';storage.mkdir();clone=root/'copy';clone.mkdir()
            (clone/'original').write_bytes(b'retained')
            (storage/'native-worker-agent.json').write_text(json.dumps(dict(path=str(clone))))
            work=dict(revision=1,planning=dict(repository=dict(storage=str(storage),candidate_commit='a'*40)),
                tasks=[dict(id='task',status='blocked',attempts=[dict(id='attempt')])])
            agent=dict(id='agent',task_id='task')
            owned=SimpleNamespace(root=root,work='work',environment=dict(os.environ),ok=lambda args,value=None:
                dict(agents=[agent]) if args[:2]==['mission','status-native'] else dict(work=work))
            output=root/'run';calls=[]
            def watched(owned,expected,out,**kwargs):
                """Only emulate the helper's fault/refusal and later primary error."""
                calls.append(out.name)
                if out.name=='refusal':
                    target=storage/'results/agent/report.md';target.mkdir(parents=True)
                    (output/'injected.json').write_text(json.dumps(dict(git_success=True)))
                    out.mkdir();(out/'watch.stderr').write_bytes(b'managed-result-retention-incomplete')
                    return set()
                raise RuntimeError('recovery-failed')
            original_rename=Path.rename
            def renamed(path,target):
                """Let parking succeed; fail only the actual attempted restore."""
                if path.name.endswith('.publication-retained'):raise OSError('restore-failed')
                return original_rename(path,target)
            with patch.object(recovery,'watch_reports',side_effect=watched),patch.object(recovery.subprocess,'run',return_value=SimpleNamespace(stdout=b'abc\n')),patch.object(Path,'rename',renamed):
                with self.assertRaises(BaseExceptionGroup) as errors:recovery.publication_recovery(owned,[agent],output)
            self.assertEqual([str(e) for e in errors.exception.exceptions],['recovery-failed','restore-failed'])
            self.assertEqual((root/'copy.publication-retained/original').read_bytes(),b'retained')


if __name__=='__main__':unittest.main()
