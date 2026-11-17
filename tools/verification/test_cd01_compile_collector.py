"""New collector integration contracts; no compiler/provider/service execution."""
import ast
import errno
import io
import stat
import subprocess
import sys
import tomllib
import copy
import hashlib
import inspect
import json
import os
from pathlib import Path
import shutil
import tempfile
import types
import unittest
from unittest import mock
import cd01_compile_collector as collector
import cd01_capture_rustc_wrapper as producer
import finite_input_custody

class CollectorIntegrationGuards(unittest.TestCase):
    def capture(self, root, record):
        """Materialize a caller-owned producer-format hardlink pair, never execute."""
        root.mkdir(); leaf = root / ('cd01-intent-' + str(record['pid']) + '-' + '0'*32)
        leaf.mkdir(); raw = (json.dumps(record, ensure_ascii=True) + '\n').encode('ascii')
        (leaf/'intent.partial').write_bytes(raw); os.link(leaf/'intent.partial', leaf/'intent.json')
        return {'path': str(root), 'identity': collector._path_identity(root)}

    def test_real_producer_schema_empty_bytes_and_rejection(self):
        """Actual producer output is a valid control; invalid hex refuses, then recovery."""
        record = json.loads(producer._render_intent_record(b'/owned/compiler', [b'', b'with space', b'owned-\xff'], b'/owned/guard'))
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            result = collector._parse_intent_records(self.capture(root/'control', record))
            self.assertEqual(result['counts']['total'], 1)
            self.assertEqual(result['records'][0]['outcome'], 'UNKNOWN')
            bad = copy.deepcopy(record); bad['compiler_arguments_hex'][0] = 'xyz'
            with self.assertRaises(ValueError): collector._parse_intent_records(self.capture(root/'bad', bad))
            result = collector._parse_intent_records(self.capture(root/'recovery', record))
            self.assertEqual(result['counts']['total'], 1)

    def test_run_environment_call_binds_actual_signature(self):
        """The actual run caller must bind to its callee before any real build."""
        tree = ast.parse(inspect.getsource(collector.run))
        calls = [x for x in ast.walk(tree) if isinstance(x, ast.Call) and isinstance(x.func, ast.Name) and x.func.id == '_child_environment']
        self.assertEqual(len(calls), 1)
        inspect.signature(collector._child_environment).bind(*([object()]*len(calls[0].args)))
        with mock.patch.dict(collector.os.environ, {'PATH':'/usr/bin:/bin','HOME':'/owned-home'}, clear=True):
            env, facts = collector._child_environment(Path('/owned-target'), Path('/owned-tmp'), Path('/owned-capture'), Path('/owned-guard'), Path('/owned-wrapper'))
        self.assertEqual(env['RUSTC_WRAPPER'], '/owned-wrapper')
        self.assertEqual(env['CD01_SPACEGUARD_PATH'], '/owned-guard')
        self.assertEqual(env['CD01_CAPTURE_DIR'], '/owned-capture')
        self.assertNotIn('CFLAGS', env)

    def test_both_config_names_are_pinned(self):
        """Both config candidates are facts, not an invented precedence rule."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name)/'root'; home=Path(name)/'home'; (root/'.cargo').mkdir(parents=True);home.mkdir()
            a=root/'.cargo/config'; b=root/'.cargo/config.toml';a.write_text('owned=1');b.write_text('owned=2')
            result=collector._discover_cargo_configs(root,str(home))
            observed={x.get('path') for x in result if x.get('present')}
            self.assertTrue({str(a),str(b)} <= observed)
            for x in result:
                if x.get('path') in [str(a),str(b)]:self.assertEqual(x['sha256'],hashlib.sha256(Path(x['path']).read_bytes()).hexdigest())

    def test_config_permission_error_is_not_absence(self):
        """Valid legacy config, inaccessible alternate candidate, fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name)/'root';home=Path(name)/'home';(root/'.cargo').mkdir(parents=True);home.mkdir();(root/'.cargo/config').write_text('owned=1')
            self.assertTrue(any(x.get('present') for x in collector._discover_cargo_configs(root,str(home))))
            probe=root/'.cargo/config.toml';real=os.lstat
            def fail(path,*args,**kwargs):
                """Refuse only the selected config probe, preserving ordinary fixture observations."""
                if Path(path)==probe:raise PermissionError(13,'owned fixture probe denied')
                return real(path,*args,**kwargs)
            with mock.patch.object(collector.os,'lstat',fail):
                with self.assertRaises((OSError,ValueError)):collector._discover_cargo_configs(root,str(home))
            self.assertTrue(any(x.get('present') for x in collector._discover_cargo_configs(root,str(home))))

    def test_relative_path_is_resolved_from_child_cwd(self):
        """Absolute control, relative PATH entry under child cwd, absolute recovery."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name);(root/'owned-bin').mkdir();tool=root/'owned-bin/rustc';tool.write_bytes(b'owned inert tool bytes');tool.chmod(0o700)
            control=collector._resolve_tool_path('rustc',str(tool.parent),root)
            variant=collector._resolve_tool_path('rustc','owned-bin',root)
            self.assertEqual(variant['sha256'],control['sha256'])
            self.assertEqual(Path(variant['resolved']),tool)
            self.assertEqual(collector._resolve_tool_path('rustc',str(tool.parent),root)['sha256'],control['sha256'])

    def registry(self, home, index):
        """Create only owned hashing fixtures; archive bytes are not Cargo executable inputs."""
        stem='owned-fixture-1.0.0';base=home/'.cargo/registry';src=base/'src'/index/stem;cache=base/'cache'/index;src.mkdir(parents=True);cache.mkdir(parents=True)
        (src/'lib.rs').write_bytes(b'owned source bytes');data=b'owned fixture archive bytes';(cache/(stem+'.crate')).write_bytes(data)
        return {'name':'owned-fixture','version':'1.0.0','source':collector.REGISTRY_SOURCE_URL,'checksum':hashlib.sha256(data).hexdigest()}

    def test_registry_namespace_is_discovered_without_ambiguity(self):
        """Historical-layout control, unique alternate namespace, ambiguous refusal, fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name);home=root/'home';old='index.crates.io-1949cf8c6b5b557f';package=self.registry(home,old)
            first=collector._prebind_registry_sources(root,str(home),'0'*64,[package]);self.assertEqual(first['packages_prebound'],1)
            for kind in ['src','cache']:(home/'.cargo/registry'/kind/old).rename(home/'.cargo/registry'/kind/'owned-other-index')
            self.assertEqual(collector._prebind_registry_sources(root,str(home),'0'*64,[package])['packages_prebound'],1)
            self.registry(home,'owned-ambiguous-index')
            with self.assertRaises(ValueError):collector._prebind_registry_sources(root,str(home),'0'*64,[package])
            fresh=root/'fresh-home';self.registry(fresh,'fresh-owned-index')
            self.assertEqual(collector._prebind_registry_sources(root,str(fresh),'0'*64,[package])['packages_prebound'],1)

    def test_fresh_target_refuses_without_deleting(self):
        """Fresh target control, retained prior output refusal, separate fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name);a=root/'a';a.mkdir();self.assertTrue(collector._require_fresh_target_tree(a)['fresh_empty'])
            preserved=a/'owned-output';preserved.write_bytes(b'preserve me')
            with self.assertRaises(ValueError):collector._require_fresh_target_tree(a)
            self.assertEqual(preserved.read_bytes(),b'preserve me')
            b=root/'b';b.mkdir();self.assertTrue(collector._require_fresh_target_tree(b)['fresh_empty'])

    def test_instrument_staging_archive_and_wrong_pin_refusal(self):
        """Known producer bytes staged/reread/archived; wrong hash refuses before copy."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name);source=root/'owned-source.py';source.write_bytes(b'#!/usr/bin/env python3\n# inert owned fixture\n');out=root/'out';out.mkdir();target=root/'target';target.mkdir();sha=hashlib.sha256(source.read_bytes()).hexdigest()
            def publish(path,data):
                """Write only owned fixture data; no production publication claim."""
                path.write_bytes(data.encode() if isinstance(data,str) else data)
            profile=types.SimpleNamespace(publish=publish,digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest())
            staged=collector._stage_rustc_intent_producer(profile,out,target,source,sha)
            self.assertEqual(staged['sha256'],sha)
            archive=collector._archive_instruments(profile,out,Path(staged['path']),source)
            self.assertTrue(archive['reread_published'])
            other=root/'other';other.mkdir()
            with self.assertRaises(ValueError):collector._stage_rustc_intent_producer(profile,other,target,source,'0'*64)
            self.assertFalse((other/'rustc-intent-producer.py').exists())

class FailureCustodyGuard(unittest.TestCase):
    def test_terminal_report_failure_retains_primary_and_numeric_result(self):
        """Healthy diagnostic with known7, failed diagnostic, healthy recovery."""
        primary = OSError(errno.EIO, 'owned fixture primary')
        primary.native_outcome = {'subprocess': {'returncode': 7}}
        report = OSError(errno.ENOSPC, 'owned fixture diagnostic')
        state = {'returncodes': {'cargo_test_compile': 7}, 'facts': {}, 'progress': [], 'out': None}
        def publish():
            """Exercise only the diagnostic consumer with fixture facts and mocked channels."""
            return collector._publish_failure(None, state, primary, None, Path('/owned-fixture'), Path('/owned-absent-target'))
        def walk(error, seen=None):
            """Collect cause objects without cycles or IO for numeric-custody assertions."""
            seen = set() if seen is None else seen
            if not isinstance(error, BaseException) or id(error) in seen: return []
            seen.add(id(error)); result = [error]
            for child in getattr(error, 'exceptions', ()): result += walk(child, seen)
            result += walk(error.__cause__, seen); result += walk(error.__context__, seen)
            result += walk(getattr(error, 'primary', None), seen)
            for item in getattr(error, 'causes', ()):
                result += walk(item[1] if isinstance(item, tuple) else item, seen)
            return result
        for phase in ['control', 'fault', 'recovery']:
            stream = io.StringIO()
            with mock.patch.object(collector, '_free_bytes', return_value=21474836480), mock.patch.object(collector.os.path, 'lexists', return_value=False), mock.patch.object(collector.sys, 'stderr', stream):
                if phase == 'fault':
                    with mock.patch.object(stream, 'write', side_effect=report):
                        try: publish()
                        except BaseException as error:
                            retained = walk(error)
                            self.assertIn(primary, retained)
                            self.assertIn(report, retained)
                            self.assertTrue(any(getattr(e, 'native_outcome', {}).get('subprocess', {}).get('returncode') == 7 for e in retained))
                        else: self.fail('terminal diagnostic swallowed: known process outcome and reporting cause not handed to consumer')
                else:
                    publish(); payload = json.loads(stream.getvalue())
                    self.assertEqual(payload['cached_numeric_results']['cargo_test_compile'], 7)
                    self.assertEqual(payload['all_native_outcomes'][0]['subprocess']['returncode'], 7)
                    self.assertEqual(payload['status'], 'FAIL')
        self.assertEqual(state['returncodes']['cargo_test_compile'], 7)

    def test_postcheck_cached_result_reaches_terminal_consumer(self):
        """Known Cargo0 survives a nonnative postcheck error and failed diagnostic."""
        primary = ValueError('owned postcheck failure, no native outcome attribute')
        reporting = OSError(errno.ENOSPC, 'owned diagnostic failure')
        state = {'returncodes': {'cargo_test_compile': 0}, 'facts': {}, 'progress': [], 'out': None}
        def publish():
            """Exercise the real publisher without spawning a child or using live state."""
            collector._publish_failure(None, state, primary, None, Path('/owned-fixture'), Path('/owned-absent-target'))
        for phase in ['control','fault','recovery']:
            stream=io.StringIO()
            with mock.patch.object(collector,'_free_bytes',return_value=21474836480), mock.patch.object(collector.os.path,'lexists',return_value=False), mock.patch.object(collector.sys,'stderr',stream):
                if phase=='fault':
                    with mock.patch.object(stream,'write',side_effect=reporting):
                        with self.assertRaises(BaseException) as caught:publish()
                    tree=collector._error_tree(caught.exception)
                    self.assertIn('collector_failure',tree,'terminal consumer lost cached result of successful child before failed postcheck')
                    self.assertEqual(tree['collector_failure']['cached_numeric_results']['cargo_test_compile'],0)
                    self.assertEqual(collector._native_outcomes(caught.exception),[],'cached facts must not fabricate a native frame')
                else:
                    publish();self.assertEqual(json.loads(stream.getvalue())['cached_numeric_results']['cargo_test_compile'],0)

class FreeObservationGuard(unittest.TestCase):
    def test_failed_free_observation_keeps_label_and_errno(self):
        """Valid free-space control, ESTALE injection, fresh successful recovery."""
        primary = ValueError('owned fixture body failure')
        state = {'returncodes': {'cargo_test_compile': 7}, 'facts': {}, 'progress': [], 'out': None}
        def publish():
            """Exercise only the diagnostic consumer with fixture facts and mocked channels."""
            stream = io.StringIO()
            with mock.patch.object(collector.sys, 'stderr', stream), mock.patch.object(collector.os.path, 'lexists', return_value=False):
                collector._publish_failure(None, state, primary, None, Path('/owned-fixture'), Path('/owned-absent-target'))
            return json.loads(stream.getvalue())
        with mock.patch.object(collector, '_free_bytes', return_value=21474836480):
            control = publish()
        self.assertEqual(control['free_bytes_after']['host-checkout-root'], 21474836480)
        with mock.patch.object(collector, '_free_bytes', side_effect=OSError(errno.ESTALE, 'owned fixture statvfs failure')):
            failed = publish()
        self.assertIsNone(failed['free_bytes_after']['host-checkout-root'])
        errors = failed.get('free_bytes_after_errors', {})
        self.assertIn('host-checkout-root', errors, 'failed observation lost its source label and cause')
        self.assertEqual(errors['host-checkout-root']['errno'], errno.ESTALE)
        self.assertEqual(failed['cached_numeric_results']['cargo_test_compile'], 7)
        with mock.patch.object(collector, '_free_bytes', return_value=21474836480):
            recovery = publish()
        self.assertEqual(recovery['free_bytes_after']['host-checkout-root'], 21474836480)

class CollectorAdditionalFailureGuards(unittest.TestCase):
    """Same-buffer lock provenance and failed archive custody, no child execution."""
    def test_lock_hash_and_parse_share_the_same_buffer(self):
        """Valid lock, mutation between old separate reads, fresh stable recovery."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name);(root/'rust').mkdir();lock=root/'rust/Cargo.lock'
            a=b'version = 3\n[[package]]\nname = "owned-a"\nversion = "1.0.0"\n'
            b=b'version = 3\n[[package]]\nname = "owned-b"\nversion = "1.0.0"\n'
            lock.write_bytes(a);sha,packages=collector._read_cargo_lock(root)
            self.assertEqual(sha,hashlib.sha256(a).hexdigest());self.assertEqual(packages[0]['name'],'owned-a')
            digest=collector.digest
            def swap(path):
                """Simulate an intervening writer after the vulnerable separate hash read."""
                observed=digest(path)
                if Path(path)==lock:lock.write_bytes(b)
                return observed
            with mock.patch.object(collector,'digest',swap):sha,packages=collector._read_cargo_lock(root)
            now=lock.read_bytes()
            self.assertEqual(sha,hashlib.sha256(now).hexdigest(),'digest does not identify parsed lock bytes')
            self.assertEqual(packages,tomllib.loads(now.decode())['package'])
            lock.write_bytes(a);sha,packages=collector._read_cargo_lock(root)
            self.assertEqual(sha,hashlib.sha256(a).hexdigest());self.assertEqual(packages[0]['name'],'owned-a')

    def test_failed_instrument_archive_keeps_owned_candidate(self):
        """Valid archive, verifier failure retaining its candidate, fresh recovery."""
        with tempfile.TemporaryDirectory() as name:
            root=Path(name);source=root/'owned-source.py';source.write_bytes(b'owned immutable bytes')
            def publish(path,data):
                """Fixture publisher only; no production atomicity/durability claim."""
                path.write_bytes(data.encode() if isinstance(data,str) else data)
            profile=types.SimpleNamespace(publish=publish,digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest())
            control=root/'control';control.mkdir()
            self.assertTrue(collector._archive_instruments(profile,control,source,source)['reread_published'])
            fault=root/'fault';fault.mkdir()
            with mock.patch.object(collector,'_verify_tar_members',side_effect=OSError(errno.EIO,'owned fixture archive verify failure')):
                with self.assertRaises(OSError):collector._archive_instruments(profile,fault,source,source)
            candidates=list(fault.glob('.instruments-*'))
            self.assertEqual(len(candidates),1,'failed archive candidate was deleted')
            self.assertGreater(candidates[0].stat().st_size,0)
            self.assertFalse((fault/'rustc-intent-instruments.tar.gz').exists())
            recovery=root/'recovery';recovery.mkdir()
            self.assertTrue(collector._archive_instruments(profile,recovery,source,source)['reread_published'])



class CollectorPolicyAndIntentGuards(unittest.TestCase):
    """Persisted mount selection and positive intent scope, without child execution."""
    def test_evidence_mount_comes_from_validated_registration(self):
        """Registered operator path is consumed, missing/foreign identity refuses, recovery."""
        root=Path('/owned-root');target=Path('/owned-allocated-medium/cargo/new')
        entry={'path':str(target),'mountpoint':'/owned-allocated-medium','uuid':collector.SWARM_TARGET_UUID,'fstype':'ext4'}
        policy={'schema_version':1,'targets':[entry]}
        storage=types.SimpleNamespace(load_policy=mock.Mock(return_value=(policy,'a'*64)))
        mount,facts=collector._registered_evidence_mount(storage,root,target)
        self.assertEqual(mount,Path(entry['mountpoint']));self.assertEqual(facts['policy_sha256'],'a'*64)
        storage.load_policy.assert_called_once_with(root)
        with self.assertRaises(ValueError):collector._registered_evidence_mount(storage,root,Path('/unregistered-target'))
        bad=dict(entry,uuid='other-owned-fictitious-volume')
        storage.load_policy.return_value=({'schema_version':1,'targets':[bad]},'b'*64)
        with self.assertRaises(ValueError):collector._registered_evidence_mount(storage,root,target)
        storage.load_policy.return_value=(policy,'a'*64)
        self.assertEqual(collector._registered_evidence_mount(storage,root,target)[0],mount)
        caller=inspect.getsource(collector._preflight_evidence_volume)
        self.assertIn('_registered_evidence_mount',caller)
        self.assertNotIn('/run/media/fpizzi/SWARM',caller)

    def test_query_intent_never_becomes_execution_or_libtest_proof(self):
        """Positive name/test intent, metadata-query refusal, then fresh intent recovery."""
        args=[b'--crate-name',b'swarm_engine',b'--test',b'owned.rs']
        record=json.loads(producer._render_intent_record(b'/owned/compiler',args,b'/owned/guard'))
        event={'target':{'name':'swarm_engine'}}
        maker=CollectorIntegrationGuards()
        with tempfile.TemporaryDirectory() as name:
            root=Path(name)
            parsed=collector._parse_intent_records(maker.capture(root/'control',record))
            facts=collector._match_libtest_intent(parsed,{'facts':{},'progress':[]},event)
            self.assertIn('UNKNOWN',facts['binding_status'])
            query=json.loads(producer._render_intent_record(b'/owned/compiler',args+[b'--print=cfg'],b'/owned/guard'))
            parsed=collector._parse_intent_records(maker.capture(root/'query',query))
            self.assertEqual(parsed['counts']['metadata_probes'],1)
            self.assertEqual(parsed['counts']['libtest_candidates'],0)
            denied={'facts':{},'progress':[]}
            with self.assertRaises(ValueError):collector._match_libtest_intent(parsed,denied,event)
            self.assertEqual(denied,{'facts':{},'progress':[]})
            parsed=collector._parse_intent_records(maker.capture(root/'recovery',record))
            self.assertIn('UNKNOWN',collector._match_libtest_intent(parsed,{'facts':{},'progress':[]},event)['binding_status'])


class StdinV2ConsumerTests(unittest.TestCase):
    """Synthetic protocol/real custody fixtures; never compiler or FD-transfer evidence."""

    def setUp(self):
        """Retain a private fixture root and bind the actual reviewed validator."""
        import finite_input_custody
        self.custody = finite_input_custody
        self.root = Path(tempfile.mkdtemp(prefix='v2-consumer-tests-'))
        self.serial = 0
        (self.root/'SYNTHETIC-SCOPE.json').write_text(json.dumps({
            'kind':'synthetic structural protocol fixtures',
            'actual':'producer formatters and custody.capture only',
            'not_executed':['dup2','guard','compiler','Cargo','provider']}))

    def make(self, v2=True):
        """Build a new synthetic record, then validate it before any injection."""
        self.serial += 1
        capture = self.root / ('capture-'+str(self.serial)); capture.mkdir(mode=0o700)
        pid = os.getpid(); leaf = capture/('cd01-intent-'+str(pid)+'-'+'0'*32);leaf.mkdir(mode=0o700)
        allocation = 'allocated-'+str(self.serial)
        plan = {'allocation_id':allocation,'custody_relative_directory':'stdin-custody'} if v2 else None
        raw = producer._render_intent_record(b'/owned/compiler',[b'--crate-name',b'probe'],b'/owned/guard',plan)
        (leaf/'intent.partial').write_bytes(raw);(leaf/'intent.partial').chmod(0o600)
        os.link(leaf/'intent.partial',leaf/'intent.json')
        if v2:
            custody_dir=leaf/'stdin-custody';custody_dir.mkdir(mode=0o700)
            source=self.root/('input-'+str(self.serial));source.write_bytes(b'x')
            source_fd=os.open(source,os.O_RDONLY)
            try:
                ready, manifest=self.custody.capture(source_fd,custody_dir,allocation)
                os.close(ready)
            finally:
                os.close(source_fd)
            # Formatter declares transfer facts; this synthetic builder does not perform them.
            (leaf/'stdin-transfer.json').write_bytes(producer._render_stdin_transfer_record(allocation,manifest))
            (leaf/'stdin-transfer.json').chmod(0o600)
        descriptor={'path':str(capture),'identity':collector._path_identity(capture)}
        result=collector._parse_intent_records(descriptor,custody_validator=self.custody.validate_staged)
        self.assertEqual(result['counts']['total'],1)
        self.assertEqual(result['records'][0]['outcome'],'UNKNOWN')
        self.assertFalse(result['records'][0]['counted_as_test'])
        return descriptor,leaf

    def snapshot(self, leaf):
        """Snapshot content and names only, without claiming atime or immutable lifetime."""
        found={}
        for path in sorted(leaf.rglob('*')):
            key=str(path.relative_to(leaf))
            if path.is_symlink():found[key]=('symlink',os.readlink(path))
            elif path.is_dir():found[key]=('directory',)
            else:found[key]=('file',path.read_bytes())
        return found

    def deny(self, descriptor, leaf, validator=None):
        """Assert a refusal changes no fixture content/names; then run fresh recovery."""
        before=self.snapshot(leaf)
        chosen=self.custody.validate_staged if validator is None else validator
        with self.assertRaises((ValueError,collector.CustodyValidationFailure)) as caught:
            collector._parse_intent_records(descriptor,custody_validator=chosen)
        self.assertEqual(self.snapshot(leaf),before)
        self.make()
        return caught.exception

    def mutate_receipt(self, leaf, mutation):
        """Mutate owned synthetic receipt bytes, preserving its private file mode."""
        path=leaf/'stdin-transfer.json';value=json.loads(path.read_bytes());mutation(value)
        path.write_text(json.dumps(value,ensure_ascii=True)+'\n')

    def test_v1_and_v2_consistency_stay_unknown(self):
        """Current v1 default and explicit-v2 validation retain nonexecution scope."""
        descriptor,leaf=self.make(False)
        before=self.snapshot(leaf);result=collector._parse_intent_records(descriptor)
        self.assertNotIn('stdin_custody',result['records'][0]);self.assertEqual(self.snapshot(leaf),before)
        descriptor,leaf=self.make();before=self.snapshot(leaf)
        result=collector._parse_intent_records(descriptor,custody_validator=self.custody.validate_staged)
        observation=result['records'][0]['stdin_custody']
        self.assertEqual(observation['manifest']['sha256'],hashlib.sha256(b'x').hexdigest())
        self.assertEqual(observation['manifest']['byte_count'],1)
        self.assertEqual(observation['outcome'],'UNKNOWN');self.assertEqual(self.snapshot(leaf),before)

    def test_missing_validator_failed_and_foreign_records_refuse(self):
        """Valid controls precede missing callback, failure file and foreign member."""
        descriptor,leaf=self.make();before=self.snapshot(leaf)
        with self.assertRaisesRegex(ValueError,'validator-required'):collector._parse_intent_records(descriptor)
        self.assertEqual(self.snapshot(leaf),before);self.make()
        for filename in ['failure.json','foreign.txt']:
            with self.subTest(filename=filename):
                descriptor,leaf=self.make();(leaf/filename).write_text('{}')
                self.deny(descriptor,leaf)

    def test_staged_tamper_and_typed_linkage_refuse(self):
        """Fresh validated controls precede byte, lineage, bool/int and flag mutations."""
        mutations=[('pid',lambda v:v.update(pid=v['pid']+1)),
                   ('allocation',lambda v:v.update(allocation_id='different')),
                   ('relative',lambda v:v.update(custody_relative_directory='../escape')),
                   ('count-bool',lambda v:v['observed_custody_manifest'].update(byte_count=True)),
                   ('eof-int',lambda v:v['observed_custody_manifest'].update(eof_observed=1)),
                   ('target-bool',lambda v:v['descriptor_replacement'].update(target_fd=False)),
                   ('flag-int',lambda v:v['descriptor_replacement'].update(dup2_completed=1)),
                   ('flag-false',lambda v:v['descriptor_replacement'].update(spare_close_completed=False))]
        for label,mutation in mutations:
            with self.subTest(label=label):
                descriptor,leaf=self.make();self.mutate_receipt(leaf,mutation);self.deny(descriptor,leaf)
        descriptor,leaf=self.make();(leaf/'stdin-custody/input.bin').write_bytes(b'y')
        error=self.deny(descriptor,leaf);self.assertIsInstance(error,collector.CustodyValidationFailure)

    def test_bad_receipt_json_and_member_sets_refuse(self):
        """Malformed/duplicate JSON and wrong member sets cannot become empty success."""
        for payload in [b'{',b'{"schema":"x","schema":"y"}']:
            with self.subTest(payload=payload):
                descriptor,leaf=self.make();(leaf/'stdin-transfer.json').write_bytes(payload);self.deny(descriptor,leaf)
        for label,mutation in [('extra',lambda v:v.update(foreign='private-value')),
                               ('missing',lambda v:v.pop('descriptor_replacement'))]:
            with self.subTest(label=label):
                descriptor,leaf=self.make();self.mutate_receipt(leaf,mutation);self.deny(descriptor,leaf)

    def test_callback_causes_reach_safe_terminal_projection(self):
        """Direct IO/interruption and normalized cleanup facts survive without private text."""
        for mode in ['io','interrupt','normalized-cleanup']:
            with self.subTest(mode=mode):
                descriptor,leaf=self.make()
                primary=OSError(errno.EACCES,'PRIVATE_PRIMARY')
                cleanup=KeyboardInterrupt('PRIVATE_CLEANUP')
                if mode=='interrupt':error=KeyboardInterrupt('PRIVATE_INTERRUPT')
                elif mode=='normalized-cleanup':
                    error=self.custody.FiniteInputCustodyError('verify-input',
                        self.custody._cause('read','input.bin',primary),
                        [self.custody._cause('close','input.bin',cleanup)])
                else:error=primary
                def rejected(path,allocation):
                    """Validate the healthy record, then inject a known callback failure."""
                    self.custody.validate_staged(path,allocation)
                    raise error
                caught=self.deny(descriptor,leaf,rejected)
                self.assertIs(caught.original_exception,error)
                terminal=collector._error_tree(caught);text=json.dumps(terminal)
                self.assertNotIn('PRIVATE',text)
                facts=terminal['custody_validation'];self.assertEqual(facts['outcome'],'UNKNOWN')
                self.assertEqual(facts['interrupted'],mode!='io')
                self.assertEqual(collector._native_outcomes(caught),[])
                if mode=='normalized-cleanup':
                    self.assertIs(caught.secondary_exceptions[0],cleanup)
                    self.assertEqual(facts['secondary_metadata'][0]['name'],'input.bin')
                    self.assertEqual(facts['primary_errno'],errno.EACCES)

    def test_identity_hardlink_and_bad_validator_result_refuse(self):
        """Current identity/linkage and callback result shape remain required."""
        descriptor,leaf=self.make();wrong=dict(descriptor);wrong['identity']=(0,0);self.deny(wrong,leaf)
        descriptor,leaf=self.make();path=leaf/'intent.json';raw=path.read_bytes()
        replacement=leaf/'copy';replacement.write_bytes(raw);replacement.chmod(0o600);os.replace(replacement,path)
        self.deny(descriptor,leaf)
        descriptor,leaf=self.make()
        def invalid(path,allocation):
            """Return a deliberately unsupported mapping after a valid actual read."""
            self.custody.validate_staged(path,allocation)
            return {'not':'a manifest'}
        self.deny(descriptor,leaf,invalid)


class StdinLeaseWiringTests(unittest.TestCase):
    """Packet24 structural wiring: real byte staging/private module execution, no compiler.

    All fixtures are structural synthetic bytes in retained private TMPDIR
    directories, labeled synthetic: the collector never borrows descriptor 0,
    never executes the wrapper, guard, compiler or Cargo, and no native PASS,
    consumption or compiler outcome is invented. A healthy valid control
    precedes every injection and fresh recovery follows it.
    """

    def setUp(self):
        """Retain one private fixture root and bind the actual reviewed helper."""
        import finite_input_custody
        self.custody = finite_input_custody
        self.root = Path(tempfile.mkdtemp(prefix='lease-wiring-tests-'))
        (self.root / 'SYNTHETIC-SCOPE.json').write_text(json.dumps({
            'kind': 'synthetic structural wiring fixtures',
            'actual': 'byte staging, private module compile, document API, reconciliation',
            'not_executed': ['dup2', 'guard', 'compiler', 'Cargo', 'provider']}))

    def operator_files(self):
        """Write fixture operator sources: a reviewed-helper copy and a manifest."""
        helper_source = self.root / 'operator-custody-helper.py'
        helper_source.write_bytes(Path(self.custody.__file__).read_bytes())
        selector = {'compiler_path_hex': b'/owned/compiler'.hex(),
                    'compiler_arguments_hex': [b'--crate-name'.hex()],
                    'cwd_hex': b'/owned/cwd'.hex()}
        document = {'schema': 'cd01.stdin_lease_manifest/1',
                    'record_kind': 'operator_stdin_lease_manifest',
                    'leases': [dict(allocation_id='allocated-1', input_fd=0,
                                    selector=selector,
                                    declarations={
                                        'finite_cooperative_single_writer_input': True,
                                        'producer_will_close_input_fd': True,
                                        'persistence_authorized': True},
                                    repeat_policy='repeatable')]}
        manifest_source = self.root / 'operator-stdin-lease-manifest.json'
        manifest_source.write_bytes(
            (json.dumps(document, sort_keys=True) + '\n').encode('utf-8'))
        helper_source.chmod(0o600)
        manifest_source.chmod(0o600)
        return helper_source, manifest_source

    def profile(self):
        """Return the shared byte-publishing fixture profile; no production claim."""
        def publish(path, data):
            """Write only owned fixture bytes; no production publication claim."""
            path.write_bytes(data.encode() if isinstance(data, str) else data)
        return types.SimpleNamespace(
            publish=publish,
            digest=lambda p: hashlib.sha256(p.read_bytes()).hexdigest())

    def test_cli_group_all_or_none_and_namespace_compatibility(self):
        """argparse accepts absent/complete group, refuses partial pre-filesystem."""
        base = ['--root', '/owned-root', '--output', '/owned-out/new', '--target',
                '/owned-target/new', '--inventory', '/owned-inventory.json',
                '--rustc-intent-producer', '/owned-producer.py',
                '--rustc-intent-producer-sha256', '0' * 64,
                '--root-guard', '/owned-root/tools/verification/cargo_space_guard.py',
                '--root-guard-sha256', '1' * 64]
        absent = collector._parse_args(base)
        self.assertIsNone(absent.rustc_stdin_lease_manifest)
        self.assertIsNone(absent.rustc_custody_helper_sha256)
        complete = collector._parse_args(base + [
            '--rustc-stdin-lease-manifest', '/owned-manifest.json',
            '--rustc-stdin-lease-manifest-sha256', '2' * 64,
            '--rustc-custody-helper', '/owned-helper.py',
            '--rustc-custody-helper-sha256', '3' * 64])
        self.assertEqual(str(complete.rustc_custody_helper), '/owned-helper.py')
        self.assertTrue(collector._leased_group_state(complete)[0])
        stream = io.StringIO()
        with mock.patch.object(sys, 'stderr', stream):
            with self.assertRaises(SystemExit) as caught:
                collector._parse_args(base + ['--rustc-stdin-lease-manifest',
                                              '/owned-manifest.json'])
        self.assertEqual(caught.exception.code, 2)
        self.assertIn('cd01-stdin-lease-group-partial', stream.getvalue())
        with self.assertRaises(ValueError):
            collector._leased_group_state(types.SimpleNamespace(
                rustc_stdin_lease_manifest='/owned-manifest.json'))
        self.assertFalse(collector._leased_group_state(types.SimpleNamespace())[0])
        output_probe = self.root / 'output-leaf'
        observed = subprocess.run(
            [sys.executable, '-B', str(Path(collector.__file__)),
             '--root', str(self.root), '--output', str(output_probe),
             '--target', str(self.root / 'target-leaf'),
             '--inventory', str(self.root / 'inventory.json'),
             '--rustc-intent-producer', str(self.root / 'producer.py'),
             '--rustc-intent-producer-sha256', '0' * 64,
             '--root-guard', str(self.root / 'guard.py'),
             '--root-guard-sha256', '1' * 64,
             '--rustc-stdin-lease-manifest', str(self.root / 'manifest.json')],
            capture_output=True)
        self.assertEqual(observed.returncode, 2)
        self.assertIn(b'cd01-stdin-lease-group-partial', observed.stderr)
        self.assertFalse(output_probe.exists(), 'partial group mutated the filesystem')

    def test_default_and_leased_child_environments(self):
        """Default env/declaration unchanged; leased adds exactly the wrapper23 pair."""
        with mock.patch.dict(collector.os.environ,
                             {'PATH': '/usr/bin:/bin', 'HOME': '/owned-home'}, clear=True):
            default_env, default_declaration = collector._child_environment(
                Path('/owned-target'), Path('/owned-tmp'), Path('/owned-capture'),
                Path('/owned-guard'), Path('/owned-producer'))
            leased_env, leased_declaration = collector._child_environment(
                Path('/owned-target'), Path('/owned-tmp'), Path('/owned-capture'),
                Path('/owned-guard'), Path('/owned-producer'),
                lease_manifest_pair=('/owned-out/stdin-lease-manifest.json', 'a' * 64))
        pair_names = (collector.STDIN_LEASE_MANIFEST_ENV_NAME,
                      collector.STDIN_LEASE_MANIFEST_SHA256_ENV_NAME)
        self.assertNotIn(pair_names[0], default_env)
        self.assertNotIn('operator_stdin_lease_manifest', default_declaration)
        self.assertEqual(default_env, {name: value for name, value in leased_env.items()
                                       if name not in pair_names})
        self.assertEqual(default_declaration['explicit'],
                         {name: value for name, value in leased_declaration['explicit'].items()
                          if name not in pair_names})
        self.assertEqual(leased_env[pair_names[0]], '/owned-out/stdin-lease-manifest.json')
        self.assertEqual(leased_env[pair_names[1]], 'a' * 64)
        pair = leased_declaration['operator_stdin_lease_manifest']
        self.assertEqual(pair['manifest_env'], pair_names[0])
        self.assertEqual(pair['sha256_env'], pair_names[1])

    def test_staged_copies_module_loader_and_leased_archive(self):
        """Real bytes stage 0600, load privately, validate manifest, archive extras."""
        helper_source, manifest_source = self.operator_files()
        out = self.root / 'out'; out.mkdir(mode=0o700)
        target = self.root / 'target'; target.mkdir(mode=0o700)
        profile = self.profile()
        helper_sha = hashlib.sha256(helper_source.read_bytes()).hexdigest()
        manifest_sha = hashlib.sha256(manifest_source.read_bytes()).hexdigest()
        producer_sha = hashlib.sha256(Path(producer.__file__).read_bytes()).hexdigest()
        producer_stage = collector._stage_rustc_intent_producer(
            profile, out, target, Path(producer.__file__), producer_sha)
        helper_stage = collector._stage_leased_artifact(
            profile, out, target, helper_source, helper_sha,
            collector.STAGED_CUSTODY_HELPER_NAME)
        manifest_stage = collector._stage_leased_artifact(
            profile, out, target, manifest_source, manifest_sha,
            collector.STAGED_STDIN_LEASE_MANIFEST_NAME)
        self.assertEqual(stat.S_IMODE(os.stat(helper_stage['path']).st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(os.stat(manifest_stage['path']).st_mode), 0o600)
        loaded_producer = collector._load_reviewed_module(
            Path(producer_stage['path']), producer_sha, 'cd01_staged_intent_producer',
            Path(producer_stage['path']),
            ('_parse_manifest_document', '_validate_manifest_document'),
            'rustc-intent-producer')
        loaded_helper = collector._load_reviewed_module(
            Path(helper_stage['path']), helper_sha, 'cd01_staged_finite_input_custody',
            Path(helper_stage['path']), ('validate_staged',), 'rustc-custody-helper')
        self.assertEqual(loaded_helper.__name__, 'cd01_staged_finite_input_custody')
        self.assertNotEqual(loaded_helper.__name__, '__main__')
        self.assertEqual(Path(loaded_helper.__file__), Path(helper_stage['path']))
        self.assertTrue(callable(loaded_helper.validate_staged))
        manifest_bytes = collector._read_pinned_buffer(
            Path(manifest_stage['path']), manifest_sha, 'stdin-lease-manifest')
        entries = collector._parse_staged_lease_manifest(loaded_producer, manifest_bytes)
        self.assertEqual(entries[0][0], 'allocated-1')
        self.assertEqual(entries[0][1], 0)
        self.assertEqual(entries[0][2][0], b'/owned/compiler')
        recovery_out = self.root / 'recovery-out'; recovery_out.mkdir(mode=0o700)
        default_archive = collector._archive_instruments(
            profile, recovery_out, manifest_source, manifest_source)
        self.assertEqual(sorted(default_archive['members']),
                         ['root-guard.py', 'rustc-intent-producer.py'])
        leased_archive = collector._archive_instruments(
            profile, out, manifest_source, manifest_source,
            leased_copies={collector.STAGED_CUSTODY_HELPER_NAME: Path(helper_stage['path']),
                           collector.STAGED_STDIN_LEASE_MANIFEST_NAME:
                               Path(manifest_stage['path'])})
        self.assertEqual(sorted(leased_archive['members']),
                         ['finite_input_custody.py', 'root-guard.py',
                          'rustc-intent-producer.py', 'stdin-lease-manifest.json'])
        self.assertEqual(leased_archive['members'][collector.STAGED_CUSTODY_HELPER_NAME],
                         helper_sha)
        self.assertTrue(leased_archive['reread_published'])

    def test_pinned_pair_and_checker_refusals(self):
        """Reusable pair pins pass healthy, refuse copy/original drift, and recover."""
        helper_source, _ = self.operator_files()
        out = self.root / 'out'; out.mkdir(mode=0o700)
        target = self.root / 'target'; target.mkdir(mode=0o700)
        profile = self.profile()
        helper_sha = hashlib.sha256(helper_source.read_bytes()).hexdigest()
        stage = collector._stage_leased_artifact(profile, out, target, helper_source,
                                                 helper_sha,
                                                 collector.STAGED_CUSTODY_HELPER_NAME)
        collector._require_pinned_pair(stage, 'leased-custody-helper')
        copy_path = Path(stage['path'])
        retained = copy_path.read_bytes()
        copy_path.write_bytes(retained + b'\n')
        with self.assertRaises(ValueError) as caught:
            collector._require_pinned_pair(stage, 'leased-custody-helper')
        self.assertIn('cd01-pinned-pair-copy-drift', str(caught.exception))
        copy_path.write_bytes(retained)
        collector._require_pinned_pair(stage, 'leased-custody-helper')
        helper_source.write_bytes(helper_source.read_bytes() + b'\n')
        with self.assertRaises(ValueError) as caught:
            collector._require_pinned_pair(stage, 'leased-custody-helper')
        self.assertIn('cd01-pinned-pair-original-drift', str(caught.exception))
        helper_source.write_bytes(retained)
        collector._require_pinned_pair(stage, 'leased-custody-helper')
        with self.assertRaises(ValueError) as caught:
            collector._stage_leased_artifact(profile, out, target, helper_source,
                                             '0' * 64,
                                             collector.STAGED_CUSTODY_HELPER_NAME)
        self.assertIn('cd01-leased-source-bytes-mismatch', str(caught.exception))

    def test_loader_io_close_wrong_api_and_cause_projection(self):
        """Healthy load control; IO/close/interrupt carriers project without paths."""
        helper_source, _ = self.operator_files()
        healthy = collector._load_reviewed_module(
            helper_source, hashlib.sha256(helper_source.read_bytes()).hexdigest(),
            'owned_load_control', helper_source, ('validate_staged',),
            'rustc-custody-helper')
        self.assertTrue(callable(healthy.validate_staged))
        wrong_api = self.root / 'wrong-api.py'
        wrong_api.write_bytes(b'owned_constant = 1\n')
        wrong_api.chmod(0o600)
        with self.assertRaises(ValueError) as caught:
            collector._load_reviewed_module(
                wrong_api, hashlib.sha256(wrong_api.read_bytes()).hexdigest(),
                'owned', wrong_api, ('validate_staged',), 'rustc-custody-helper')
        self.assertIn('cd01-leased-module-api-mismatch', str(caught.exception))
        real = self.root / 'real.py'
        real.write_bytes(b'owned_fixture_bytes = b"x"\n')
        real.chmod(0o600)
        real_sha = hashlib.sha256(real.read_bytes()).hexdigest()
        self.assertEqual(collector._read_pinned_buffer(real, real_sha, 'owned-label'),
                         b'owned_fixture_bytes = b"x"\n')
        with self.assertRaises(ValueError) as caught:
            collector._read_pinned_buffer(real, '0' * 64, 'owned-label')
        self.assertIn('cd01-leased-buffer-bytes-mismatch', str(caught.exception))
        injected = OSError(errno.EIO, 'owned fixture read failure')
        with mock.patch.object(collector.os, 'read', side_effect=injected):
            with self.assertRaises(collector.LeasedArtifactFailure) as caught:
                collector._read_pinned_buffer(real, real_sha, 'owned-label')
        self.assertIs(caught.exception.original_exception, injected)
        tree = collector._error_tree(caught.exception)
        rendered = json.dumps(tree)
        self.assertNotIn(str(self.root), rendered)
        self.assertEqual(tree['leased_artifact']['stage'], 'load')
        self.assertEqual(tree['leased_artifact']['primary_errno'], errno.EIO)
        close_error = OSError(errno.EIO, 'owned fixture close failure')
        with mock.patch.object(collector.os, 'read', side_effect=injected), \
                mock.patch.object(collector.os, 'close', side_effect=close_error):
            with self.assertRaises(collector.LeasedArtifactFailure) as caught:
                collector._read_pinned_buffer(real, real_sha, 'owned-label')
        self.assertIs(caught.exception.secondary_exceptions[0], close_error)
        self.assertEqual(collector._error_tree(caught.exception)
                         ['leased_artifact']['secondary_types'], ['OSError'])
        with mock.patch.object(collector.os, 'read', side_effect=KeyboardInterrupt()):
            with self.assertRaises(collector.LeasedArtifactFailure) as caught:
                collector._read_pinned_buffer(real, real_sha, 'owned-label')
        self.assertTrue(collector._error_tree(caught.exception)
                        ['leased_artifact']['interrupted'])
        self.assertEqual(collector._read_pinned_buffer(real, real_sha, 'owned-label'),
                         b'owned_fixture_bytes = b"x"\n')

    def test_reconciliation_repeats_missing_foreign_mismatch_and_matching_v1(self):
        """Real v2/v1 receipts: repeat control, four refusals, fresh recovery control."""
        def parsed_v2(tag, allocation, arguments):
            """Build one real parsed v2 receipt through producer and custody helper."""
            capture = self.root / tag
            capture.mkdir(mode=0o700)
            leaf = capture / ('cd01-intent-' + str(os.getpid()) + '-' + os.urandom(16).hex())
            leaf.mkdir(mode=0o700)
            custody_dir = leaf / 'stdin-custody'
            custody_dir.mkdir(mode=0o700)
            source = self.root / (tag + '-input')
            source.write_bytes(b'x')
            source_fd = os.open(source, os.O_RDONLY)
            try:
                ready, manifest = self.custody.capture(source_fd, custody_dir, allocation)
                os.close(ready)
            finally:
                os.close(source_fd)
            raw = producer._render_intent_record(
                b'/owned/compiler', arguments, b'/owned/guard',
                {'allocation_id': allocation,
                 'custody_relative_directory': 'stdin-custody'})
            (leaf / 'intent.partial').write_bytes(raw)
            (leaf / 'intent.partial').chmod(0o600)
            os.link(leaf / 'intent.partial', leaf / 'intent.json')
            (leaf / 'stdin-transfer.json').write_bytes(
                producer._render_stdin_transfer_record(allocation, manifest))
            (leaf / 'stdin-transfer.json').chmod(0o600)
            descriptor = {'path': str(capture),
                          'identity': collector._path_identity(capture)}
            return collector._parse_intent_records(
                descriptor, custody_validator=self.custody.validate_staged)

        def declared_entries(pairs):
            """Normalize declared pairs through the actual wrapper23 document API."""
            leases = [dict(allocation_id=allocation, input_fd=0, selector=selector,
                           declarations={
                               'finite_cooperative_single_writer_input': True,
                               'producer_will_close_input_fd': True,
                               'persistence_authorized': True},
                           repeat_policy='repeatable')
                      for allocation, selector in pairs]
            return producer._validate_manifest_document(
                {'schema': 'cd01.stdin_lease_manifest/1',
                 'record_kind': 'operator_stdin_lease_manifest', 'leases': leases})

        first = parsed_v2('repeat-a', 'allocated-a', [b'--crate-name', b'owned'])
        record = first['records'][0]
        selector = {'compiler_path_hex': record['compiler_path_hex'],
                    'compiler_arguments_hex': record['compiler_arguments_hex'],
                    'cwd_hex': record['cwd_hex']}
        second = parsed_v2('repeat-b', 'allocated-a', [b'--crate-name', b'owned'])
        merged = {'records': first['records'] + second['records']}
        healthy = collector._reconcile_leased_selectors(
            declared_entries([('allocated-a', selector)]), merged)
        self.assertEqual(healthy['observed_v2_records'], 2)
        repeated = healthy['record_dirs_by_allocation']['allocated-a']
        self.assertEqual(len(repeated), 2)
        self.assertNotEqual(repeated[0], repeated[1])
        missing = declared_entries([
            ('allocated-a', selector),
            ('allocated-b', dict(selector,
                                 compiler_arguments_hex=[b'--crate-name'.hex(),
                                                         b'other'.hex()]))])
        with self.assertRaises(ValueError) as caught:
            collector._reconcile_leased_selectors(missing, merged)
        self.assertIn('cd01-lease-allocation-unobserved', str(caught.exception))
        foreign = parsed_v2('foreign', 'allocated-c', [b'--crate-name', b'owned'])
        with self.assertRaises(ValueError) as caught:
            collector._reconcile_leased_selectors(
                declared_entries([('allocated-a', selector)]), foreign)
        self.assertIn('cd01-lease-record-undeclared', str(caught.exception))
        mismatched = dict(selector, compiler_arguments_hex=[b'--crate-name'.hex(),
                                                            b'foreign'.hex()])
        with self.assertRaises(ValueError) as caught:
            collector._reconcile_leased_selectors(
                declared_entries([('allocated-a', mismatched)]), first)
        self.assertIn('cd01-lease-selector-mismatch', str(caught.exception))
        v1_capture = self.root / 'v1-matched'
        v1_capture.mkdir(mode=0o700)
        v1_leaf = v1_capture / ('cd01-intent-' + str(os.getpid()) + '-' + os.urandom(16).hex())
        v1_leaf.mkdir(mode=0o700)
        v1_raw = producer._render_intent_record(b'/owned/compiler',
                                                [b'--crate-name', b'owned'],
                                                b'/owned/guard')
        (v1_leaf / 'intent.partial').write_bytes(v1_raw)
        (v1_leaf / 'intent.partial').chmod(0o600)
        os.link(v1_leaf / 'intent.partial', v1_leaf / 'intent.json')
        v1_parsed = collector._parse_intent_records(
            {'path': str(v1_capture), 'identity': collector._path_identity(v1_capture)})
        v1_record = v1_parsed['records'][0]
        v1_selector = {'compiler_path_hex': v1_record['compiler_path_hex'],
                       'compiler_arguments_hex': v1_record['compiler_arguments_hex'],
                       'cwd_hex': v1_record['cwd_hex']}
        with self.assertRaises(ValueError) as caught:
            collector._reconcile_leased_selectors(
                declared_entries([('allocated-a', v1_selector)]), v1_parsed)
        self.assertIn('cd01-lease-selector-recorded-v1', str(caught.exception))
        recovered = collector._reconcile_leased_selectors(
            declared_entries([('allocated-a', selector)]), first)
        self.assertEqual(recovered['observed_v2_records'], 1)



    def test_empty_group_values_refuse_instead_of_disabling(self):
        """Absent/valid control, explicitly empty declarations refuse, fresh recovery."""
        valid = types.SimpleNamespace(rustc_stdin_lease_manifest='/owned/manifest',
            rustc_stdin_lease_manifest_sha256='a' * 64,
            rustc_custody_helper='/owned/helper', rustc_custody_helper_sha256='b' * 64)
        self.assertTrue(collector._leased_group_state(valid)[0])
        self.assertFalse(collector._leased_group_state(types.SimpleNamespace())[0])
        for values in [dict.fromkeys(vars(valid), ''),
                       dict(vars(valid), rustc_stdin_lease_manifest_sha256='')]:
            with self.subTest(values=list(values)), self.assertRaises(ValueError):
                collector._leased_group_state(types.SimpleNamespace(**values))
        self.assertTrue(collector._leased_group_state(valid)[0])

    def test_source_manifest_privacy_and_helper_readable_mode(self):
        """Private manifest control, nonprivate refusal, readable helper and recovery."""
        helper, manifest = self.operator_files()
        target = self.root / 'target'; target.mkdir(mode=0o700)
        out = self.root / 'private-control'; out.mkdir(mode=0o700)
        profile = self.profile()
        sha = hashlib.sha256(manifest.read_bytes()).hexdigest()
        collector._stage_leased_artifact(profile, out, target, manifest, sha,
                                        collector.STAGED_STDIN_LEASE_MANIFEST_NAME)
        manifest.chmod(0o644)
        refused = self.root / 'nonprivate'; refused.mkdir(mode=0o700)
        with self.assertRaises(collector.LeasedArtifactFailure):
            collector._stage_leased_artifact(profile, refused, target, manifest, sha,
                                            collector.STAGED_STDIN_LEASE_MANIFEST_NAME)
        self.assertEqual(list(refused.iterdir()), [])
        manifest.chmod(0o600)
        recovery = self.root / 'recovery'; recovery.mkdir(mode=0o700)
        collector._stage_leased_artifact(profile, recovery, target, manifest, sha,
                                        collector.STAGED_STDIN_LEASE_MANIFEST_NAME)
        helper.chmod(0o644)  # Source module may be readable; staged copy is private.
        result = collector._stage_leased_artifact(profile, recovery, target, helper,
            hashlib.sha256(helper.read_bytes()).hexdigest(), collector.STAGED_CUSTODY_HELPER_NAME)
        self.assertEqual(stat.S_IMODE(Path(result['path']).stat().st_mode), 0o600)

    def test_pin_io_and_secondary_errno_reach_safe_terminal(self):
        """Healthy current pins, missing-original safe diagnostic, cleanup errno, recovery."""
        helper, _ = self.operator_files()
        out = self.root / 'out'; out.mkdir(mode=0o700)
        target = self.root / 'target'; target.mkdir(mode=0o700)
        profile = self.profile()
        record = collector._stage_leased_artifact(profile, out, target, helper,
            hashlib.sha256(helper.read_bytes()).hexdigest(), collector.STAGED_CUSTODY_HELPER_NAME)
        collector._require_pinned_pair(record, 'leased-custody-helper')
        moved = helper.with_suffix('.retained')
        helper.rename(moved)
        try:
            with self.assertRaises(collector.LeasedArtifactFailure) as caught:
                collector._require_pinned_pair(record, 'leased-custody-helper')
            tree = collector._error_tree(caught.exception)
            self.assertNotIn(str(self.root), json.dumps(tree))
            self.assertEqual(tree['leased_artifact']['primary_errno'], errno.ENOENT)
        finally:
            moved.rename(helper)
        primary = OSError(errno.EIO, 'PRIMARY_PRIVATE_TEXT')
        secondary = OSError(errno.ENOSPC, 'SECONDARY_PRIVATE_TEXT')
        carrier = collector.LeasedArtifactFailure('cd01-leased-artifact-failed', 'load',
                                                  primary, (secondary,))
        tree = collector._error_tree(carrier)
        self.assertEqual(tree['leased_artifact']['secondary_errno'], [errno.ENOSPC])
        self.assertNotIn('PRIVATE', json.dumps(tree))
        collector._require_pinned_pair(record, 'leased-custody-helper')

if __name__=='__main__':unittest.main()
