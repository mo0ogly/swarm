#!/usr/bin/env python3
"""Simulated mount-admission regressions; no Cargo, real mount or live Root.

Real ext4/UUID/FD qualification is separate. Every fault starts after a valid
owned-directory control with simulated findmnt facts and checks recovery.
"""
import fcntl,importlib.util,json,os,subprocess,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
from fixture_area import fixture_area

TOOLS=Path(__file__).resolve().parents[1]/'tools'
sys.path.insert(0,str(TOOLS/'verification'))
import build_storage as storage
observe_unmocked=storage.observe_mount
import verify_native_profile as profile
spec=importlib.util.spec_from_file_location('external_native_builder',TOOLS/'rust-migration/build_native.py');builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)


class ExternalTargetAdmission(unittest.TestCase):
    """Exercise exact registration/identity and refusal before consumer delegation."""
    def setUp(self):
        """Allocate isolated owned paths and demonstrate the valid witness first."""
        self.tmp=tempfile.TemporaryDirectory(prefix='external-target-',dir=fixture_area());self.area=Path(self.tmp.name);self.root=self.area/'source';self.root.mkdir();(self.root/'.cargo').mkdir();self.mount=self.area/'volume';self.mount.mkdir();self.target=self.mount/'cargo'/'target'
        self.entry=dict(path=str(self.target),mountpoint=str(self.mount),uuid='fixture-ext4-uuid',fstype='ext4');self.policy=self.root/storage.POLICY;self.save()
        device=self.mount.stat().st_dev;self.facts=dict(target=str(self.mount),source='fixture-device',fstype='ext4',uuid=self.entry['uuid'],id=101,**{'maj:min':f'{os.major(device)}:{os.minor(device)}'})
        self.observer=patch.object(storage,'observe_mount',return_value=self.facts.copy());self.observer.start()
        # The builder loads its own helper module. Its observation is explicitly
        # simulated too; it does not run findmnt or inherit a native claim.
        self.builder_observer=patch.object(builder.profile.storage,'observe_mount',return_value=self.facts.copy());self.builder_observer.start()
        self.profile_observer=patch.object(profile.storage,'observe_mount',return_value=self.facts.copy());self.profile_observer.start()
        with storage.admitted(self.root,self.target) as admitted:self.assertEqual(admitted.check()['mode'],'registered-external')
        self.original=self.policy.read_bytes()
    def tearDown(self):
        """Close only test-owned patches/directories; failures are visible."""
        self.observer.stop();self.builder_observer.stop();self.profile_observer.stop();self.tmp.cleanup()
    def save(self,entry=None):
        """Persist explicit fixture data; never touch the workstation policy."""
        self.policy.write_text(json.dumps(dict(schema_version=1,targets=[entry or self.entry])));self.policy.chmod(0o600)
    def recover(self):
        """Reestablish original policy and show a valid guard after refusal."""
        self.policy.write_bytes(self.original);self.policy.chmod(0o600)
        with storage.admitted(self.root,self.target) as guard:self.assertEqual(guard.check()['mode'],'registered-external')
    def test_internal_default_and_unregistered_external_refuse_before_consumer(self):
        """Default internal admission persists; external absence reaches no Cargo body."""
        self.policy.unlink()
        with storage.admitted(self.root,self.root/'cache') as guard:self.assertEqual(guard.check()['mode'],'checkout')
        for module,body in [(profile,'_run_admitted'),(builder,'_build_admitted')]:
            with patch.object(module,body) as invoke:
                with self.assertRaisesRegex(ValueError,'not-registered'):
                    if module is profile:module.run(self.root,self.area/'proof',self.target)
                    else:module.build(self.root,self.area/'proof',self.target,'devel')
                invoke.assert_not_called();self.assertFalse((self.area/'proof').exists())
        self.recover()
    def test_missing_mount_never_falls_back_to_ordinary_directory(self):
        """An existing unmounted path is insufficient; no target creation occurs."""
        self.target.rmdir();self.target.parent.rmdir()
        with patch.object(storage,'observe_mount',side_effect=ValueError('fixture-mount-absent')):
            with self.assertRaisesRegex(ValueError,'mount-absent'):
                with storage.admitted(self.root,self.target):self.fail('absent mount admitted')
        self.assertFalse(self.target.parent.exists());self.recover()
    def test_wrong_uuid_or_open_device_refuses_then_recovers(self):
        """UUID mismatch and FD major/minor mismatch are separate safe refusals."""
        for key,value,code in [('uuid','fixture-other','identity-mismatch'),('maj:min','0:0','open-device-mismatch'),('fstype','tmpfs','identity-mismatch')]:
            with patch.object(storage,'observe_mount',return_value={**self.facts,key:value}):
                with self.assertRaisesRegex(ValueError,code):
                    with storage.admitted(self.root,self.target):self.fail('changed identity admitted')
            self.recover()
    def test_target_symlink_escape_refuses_without_foreign_write(self):
        """The exact registered path cannot alias a different output tree."""
        moved=self.target.with_name('held-original');self.target.rename(moved);escape=self.root/'foreign';self.target.symlink_to(escape,target_is_directory=True)
        with self.assertRaisesRegex(ValueError,'symlink'):
            with storage.admitted(self.root,self.target):self.fail('alias admitted')
        self.assertFalse(escape.exists());self.target.unlink();moved.rename(self.target);self.recover()
    def test_changed_mount_id_and_target_inode_refuse_under_held_fd(self):
        """Path/mount replacement cannot retain a stale descriptor declaration."""
        with storage.admitted(self.root,self.target) as guard:
            held=list(guard.fds)
            with patch.object(storage,'observe_mount',return_value={**self.facts,'id':102}):
                with self.assertRaisesRegex(ValueError,'mount-changed'):guard.check()
            self.assertEqual(guard.check()['mode'],'registered-external')
            moved=self.target.with_name('original');self.target.rename(moved);self.target.mkdir()
            with self.assertRaisesRegex(ValueError,'target-changed'):guard.check()
            self.target.rmdir();moved.rename(self.target);guard.check()
            self.assertTrue(all(os.fstat(fd) for fd in held))
        for fd in held:
            with self.assertRaises(OSError):os.fstat(fd)
        self.recover()
    def test_policy_bytes_change_refuses_without_losing_original_error(self):
        """A pending build cannot silently adopt a changed persistent admission."""
        with storage.admitted(self.root,self.target) as guard:
            self.policy.write_bytes(self.original+b'\n')
            with self.assertRaisesRegex(ValueError,'policy-changed'):guard.check()
            self.policy.write_bytes(self.original);guard.check()
        self.recover()
    def test_unknown_filesystem_and_policy_symlink_refuse(self):
        """Unsupported filesystems and aliased configuration never become authority."""
        self.save({**self.entry,'fstype':'fixture-unqualified'})
        with self.assertRaisesRegex(ValueError,'not-qualified'):
            with storage.admitted(self.root,self.target):self.fail('unsupported filesystem')
        self.policy.write_bytes(self.original);other=self.root/'policy-copy';self.policy.rename(other);self.policy.symlink_to(other)
        with self.assertRaises(OSError):storage.load_policy(self.root)
        self.policy.unlink();other.rename(self.policy);self.recover()
    def test_registration_enospc_preserves_original_and_recovery(self):
        """Pre-replacement write failure preserves policy; explicit retry can recover."""
        import errno
        with patch.object(storage.os,'fsync',side_effect=OSError(errno.ENOSPC,'fixture-full')):
            with self.assertRaises(OSError):storage.register(self.root,self.mount/'next',self.mount,self.entry['uuid'],'ext4')
        self.assertEqual(self.policy.read_bytes(),self.original)
        storage.register(self.root,self.mount/'next',self.mount,self.entry['uuid'],'ext4')
        self.assertEqual(len(storage.load_policy(self.root)[0]['targets']),2)
        self.recover()
    def test_mount_transport_capabilities_refuse_and_recover(self):
        """Real parser requires writable/executable output before any Cargo call."""
        def response(options):
            return subprocess.CompletedProcess(['findmnt'],0,json.dumps({'filesystems':[{**self.facts,'options':options}]}).encode(),b'')
        with patch.object(storage.subprocess,'run',return_value=response('rw,nodev')):
            self.assertEqual(observe_unmocked(self.mount)['options'],'rw,nodev')
        for options,code in [('ro,nodev','not-writable'),('rw,noexec','noexec')]:
            with patch.object(storage.subprocess,'run',return_value=response(options)):
                with self.assertRaisesRegex(ValueError,code):observe_unmocked(self.mount)
        with patch.object(storage.subprocess,'run',return_value=response('rw,nodev')):
            self.assertEqual(observe_unmocked(self.mount)['uuid'],self.entry['uuid'])
        self.recover()
    def test_busy_registration_preserves_policy_then_recovers(self):
        """A concurrent cooperative policy writer refuses without a lost update."""
        lock=self.root/'.cargo/external-targets.lock'
        with lock.open('w') as stream:
            lock.chmod(0o600);fcntl.flock(stream,fcntl.LOCK_EX|fcntl.LOCK_NB)
            with self.assertRaisesRegex(ValueError,'registration-busy'):
                storage.register(self.root,self.mount/'next',self.mount,self.entry['uuid'],'ext4')
            self.assertEqual(self.policy.read_bytes(),self.original)
        storage.register(self.root,self.mount/'next',self.mount,self.entry['uuid'],'ext4')
        self.assertEqual(len(storage.load_policy(self.root)[0]['targets']),2);self.recover()
    def test_unsupported_python_refuses_before_admission(self):
        """Unsupported exception/lifetime APIs refuse at import before any FD opens."""
        script=TOOLS/'verification/build_storage.py'
        with patch.object(sys,'version_info',(3,10)):
            with self.assertRaisesRegex(RuntimeError,'Python 3.11'):
                exec(compile(script.read_text(),str(script),'exec'),{'__name__':'unsupported_storage_fixture','__file__':str(script)})
        self.recover()
    def test_primary_and_close_causes_survive(self):
        """A close fault cannot hide an operation refusal; all FDs still close."""
        import errno
        original_close=storage.os.close;failed=[]
        def close_then_fail(fd):
            original_close(fd)
            if not failed:failed.append(fd);raise OSError(errno.EIO,'fixture-close-cause')
        with patch.object(storage.os,'close',side_effect=close_then_fail):
            with self.assertRaises(BaseExceptionGroup) as result:
                with storage.admitted(self.root,self.target):raise ValueError('fixture-operation-cause')
        def messages(error):
            return [str(error)] + [text for cause in getattr(error,'exceptions',()) for text in messages(cause)]
        text=' '.join(messages(result.exception));self.assertIn('fixture-operation-cause',text);self.assertIn('fixture-close-cause',text)
        self.recover()
    def test_policy_and_method_bytes_bind_profile_inputs(self):
        """Registry/helper drift invalidates retained default-profile source inputs."""
        method=self.root/'tools/verification/build_storage.py';method.parent.mkdir(parents=True);method.write_bytes((TOOLS/'verification/build_storage.py').read_bytes());outcome=method.with_name('native_build_outcome.py');outcome.write_bytes((TOOLS/'verification/native_build_outcome.py').read_bytes())
        before=profile.manifest(self.root,profile.cargo_config_inputs(self.root))
        self.assertIn(storage.POLICY,before);self.assertIn('tools/verification/build_storage.py',before);self.assertIn('tools/verification/native_build_outcome.py',before)
        profile.effective_unchanged(self.root,before,before)
        for path in [self.policy,method,outcome]:
            original=path.read_bytes();path.write_bytes(original+b'\n')
            with self.assertRaisesRegex(ValueError,'config-changed-or-removed'):
                profile.effective_unchanged(self.root,before,profile.manifest(self.root,profile.cargo_config_inputs(self.root)))
            path.write_bytes(original);profile.effective_unchanged(self.root,before,profile.manifest(self.root,profile.cargo_config_inputs(self.root)))
        self.recover()
    def test_registered_builder_and_collector_hold_fds_across_delegate(self):
        """Both public entrypoints delegate only inside an active checked guard."""
        def witness(*args):
            guard=args[-1];self.assertTrue(guard.fds);self.assertEqual(guard.check()['mode'],'registered-external');return 'admitted'
        for module,body in [(profile,'_run_admitted'),(builder,'_build_admitted')]:
            with patch.object(module,body,side_effect=witness) as invoke:
                result=module.run(self.root,self.area/'proof',self.target) if module is profile else module.build(self.root,self.area/'proof',self.target,'devel')
                self.assertEqual(result,'admitted');self.assertEqual(invoke.call_count,1)
        self.recover()


if __name__=='__main__':unittest.main(verbosity=2)
