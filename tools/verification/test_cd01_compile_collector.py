"""New collector integration contracts; no compiler/provider/service execution."""
import ast
import errno
import io
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

if __name__=='__main__':unittest.main()
