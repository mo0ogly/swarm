#!/usr/bin/env python3
"""Consumer outcome retention with explicit subprocess/mount doubles, no Cargo.

A successful control precedes each injected exit/signal plus post-admission or
receipt-publication failure. Simulated qualification files are fixture-only and
never used as production evidence or executed binaries.
"""
import contextlib,hashlib,importlib.util,json,subprocess,sys,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
from fixture_area import fixture_area
TOOLS=Path(__file__).resolve().parents[1]/'tools';sys.path.insert(0,str(TOOLS/'verification'))
import verify_native_profile as profile
spec=importlib.util.spec_from_file_location('outcome_builder',TOOLS/'rust-migration/build_native.py');builder=importlib.util.module_from_spec(spec);spec.loader.exec_module(builder)


class Gate:
    """Mark only the post-subprocess boundary as changed; no real mount exists."""
    def __init__(self,drift):self.drift=drift;self.completed=False
    def check(self):
        if self.completed and self.drift:raise ValueError('fixture-post-admission-change')
        return {'mode':'fixture-only'}


class NativeOutcomeRetention(unittest.TestCase):
    """Original process result and postcheck/publication failures must coexist."""
    def exercise(self,kind,code=0,drift=False,publish_failure=False):
        """Exercise the public consumer with complete inert positive-path doubles."""
        with tempfile.TemporaryDirectory(prefix='native-outcome-',dir=fixture_area()) as directory:
            root=Path(directory);out=root/'proof';target=root/'cache';gate=Gate(drift);mod=builder if kind=='collector' else profile;p=builder.profile if kind=='collector' else profile
            binary=root/'inert-binary';binary.write_bytes(b'inert-not-executed');declared={};events=[]
            for name in ['swarm_engine','swarm_web','swarm_cli']:
                suffix='a'*16;lib=target/'debug/deps'/f'lib{name}-{suffix}.rlib';lib.parent.mkdir(parents=True,exist_ok=True);lib.write_bytes(b'inert-library');fp=target/'debug/.fingerprint'/(name.replace('_','-')+'-'+suffix)/('lib-'+name+'.json');fp.parent.mkdir(parents=True);fp.write_text(json.dumps(dict(rustflags=[],features='[]',compile_kind=0)))
                events.append(dict(reason='compiler-artifact',target=dict(name=name),filenames=[str(lib)],features=[],profile=dict(test=False)))
            events += [dict(reason='compiler-artifact',target=dict(name='swarm-rust'),executable=str(binary),filenames=[str(binary)],features=[],profile=dict(test=False)),dict(reason='build-finished',success=True)]
            @contextlib.contextmanager
            def admitted(*args):yield gate
            def run(argv,**kwargs):
                if kind=='collector' and '--output' not in argv:
                    return subprocess.CompletedProcess(argv,0,stdout=json.dumps(dict(schema_version=1,binary={**{k:v for k,v in declared.items() if k!='schema_version'},'provenance':'injected'})).encode(),stderr=b'')
                gate.completed=True
                kwargs['stderr'].write(b'fixture primary process diagnosis\n')
                if kind=='cargo':kwargs['stdout'].write(('\n'.join(json.dumps(e) for e in events)+'\n').encode())
                if kind=='collector' and code==0:
                    metadata=kwargs['env']['SWARM_NATIVE_BUILD_METADATA'];declared.update(json.loads(metadata));folder=Path(argv[argv.index('--output')+1]);folder.mkdir();(folder/'result.json').write_text(json.dumps(dict(native_build_metadata=dict(present=True,sha256=hashlib.sha256(metadata.encode()).hexdigest()),binary='inert-binary',sha256=hashlib.sha256(binary.read_bytes()).hexdigest())))
                return subprocess.CompletedProcess(argv,code)
            original_publish=p.publish
            def publish(path,data):
                if publish_failure and path.name==kind+('-admission-after.json' if publish_failure=='after' else '-result.json'):raise OSError('fixture-outcome-publication-failed')
                return original_publish(path,data)
            error=None;result=None
            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(p.storage,'admitted',admitted));stack.enter_context(patch.object(p.subprocess,'run',side_effect=run));stack.enter_context(patch.object(p,'publish',side_effect=publish))
                if kind=='collector':stack.enter_context(patch.object(builder,'observe',return_value=dict(commit='a'*40,modified=True)))
                else:
                    stack.enter_context(patch.object(profile,'candidates',return_value={}));stack.enter_context(patch.object(profile,'dependencies',return_value={}));stack.enter_context(patch.object(profile,'archive_effective_sources',return_value={'fixture_only':True}))
                try:result=builder.build(root,out,target,'devel') if kind=='collector' else profile.run(root,out,target)
                except BaseException as caught:error=caught
            files={p.name:json.loads(p.read_text()) for p in out.glob('*.json')};channels={p.name:p.read_bytes() for p in out.glob('*.stderr')}
            return result,error,files,channels
    def assert_failure_retained(self,kind,code,drift=True,publish_failure=False):
        """Run valid control first, then require both failure dimensions in records."""
        valid,error,_,_=self.exercise(kind);self.assertIsNone(error);self.assertTrue(valid['status'].startswith('VERIFIED_'))
        _,error,files,channels=self.exercise(kind,code,drift,publish_failure);self.assertIsNotNone(error)
        receipt=files[kind+'-admission-after.json'];self.assertEqual(receipt['subprocess']['returncode'],code);self.assertEqual(receipt['subprocess']['signal'],-code if code<0 else None)
        self.assertFalse(receipt['admission_after']['ok']);self.assertIn('fixture-post-admission-change',json.dumps(receipt));self.assertIn(b'fixture primary process diagnosis',channels[kind+'.stderr'])
        failure=files['failure.json'];self.assertIn('fixture-post-admission-change',json.dumps(failure));self.assertEqual(failure['native_outcome']['subprocess']['returncode'],code)
        if publish_failure:self.assertIn('fixture-outcome-publication-failed',json.dumps(failure));self.assertFalse(receipt['receipt_publication']['ok'])
        else:self.assertEqual(files[kind+'-result.json']['returncode'],code)
        # Same consumer recovers with unchanged valid preconditions.
        valid,error,_,_=self.exercise(kind);self.assertIsNone(error);self.assertTrue(valid['status'].startswith('VERIFIED_'))
    def test_collector_signal_and_drift_are_both_retained(self):self.assert_failure_retained('collector',-9)
    def test_cargo_exit_and_drift_are_both_retained(self):self.assert_failure_retained('cargo',2)
    def test_publication_process_and_drift_are_all_retained(self):self.assert_failure_retained('collector',-9,publish_failure=True)
    def test_after_receipt_failure_keeps_process_and_postcheck_in_final_consumer(self):
        """If the second receipt fails, failure.json still carries all outcomes."""
        valid,error,_,_=self.exercise('collector');self.assertIsNone(error);self.assertEqual(valid['status'],'VERIFIED_NATIVE_CANDIDATE')
        _,error,files,channels=self.exercise('collector',-9,True,'after');self.assertIsNotNone(error)
        self.assertEqual(files['collector-result.json']['returncode'],-9)
        value=files['failure.json'];self.assertEqual(value['native_outcome']['subprocess']['signal'],9)
        self.assertFalse(value['native_outcome']['admission_after']['ok']);self.assertIn('fixture-outcome-publication-failed',json.dumps(value));self.assertIn(b'fixture primary process diagnosis',channels['collector.stderr'])
        valid,error,_,_=self.exercise('collector');self.assertIsNone(error);self.assertEqual(valid['status'],'VERIFIED_NATIVE_CANDIDATE')
    def test_spawn_failure_and_postcheck_both_reach_final_consumer(self):
        """An interrupted spawn has unknown exit status, never a made-up zero."""
        # Public consumers already exercise positive controls in exercise().
        valid,error,_,_=self.exercise('collector');self.assertIsNone(error);self.assertEqual(valid['status'],'VERIFIED_NATIVE_CANDIDATE')
        import native_build_outcome as outcomes
        with tempfile.TemporaryDirectory(prefix='native-spawn-outcome-',dir=fixture_area()) as directory:
            out=Path(directory);gate=Gate(True)
            def failed(*args,**kwargs):gate.completed=True;raise OSError('fixture-spawn-outcome-unknown')
            with patch.object(outcomes.subprocess,'run',side_effect=failed):
                with self.assertRaises(BaseExceptionGroup) as caught:
                    outcomes.execute(['inert-not-executed'],out=out,name='cargo',admission=gate,publish=profile.publish,digest=profile.digest,failure_code='fixture-nonzero')
            value=outcomes.failure_document(caught.exception);self.assertIsNone(value['native_outcome']['subprocess']['returncode']);self.assertTrue(value['native_outcome']['subprocess']['execution_attempted'])
            self.assertIn('fixture-spawn-outcome-unknown',json.dumps(value));self.assertIn('fixture-post-admission-change',json.dumps(value))



if __name__=='__main__':unittest.main(verbosity=2)
