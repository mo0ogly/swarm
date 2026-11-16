#!/usr/bin/env python3
"""Additional inert caller-branch guards; no Biba, Git, real child or provider.

This separate suite leaves the original eight-case harness and caller frozen.
It reuses that harness's typed evidence helpers, loaded only on an authorized
future invocation. The candidate public run_git_stdio and real stdio_probe are
called under fully patched subprocess.run and explicit proof/fixture doubles.
Embedded fixture programs are never executed. Candidate PASS markers remain
captured privately; the only public result is INERT_HANDLER_BRANCHES_*.
"""
import argparse
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import patch


class HandlerBranches(unittest.TestCase):
    """Exercise additional known/unknown completion and output-custody branches.

    main supplies exact bytes/pins and an existing admitted private parent.
    Each call owns its files; trials are retained and never reused or cleaned.
    These outcomes qualify only Python caller handling with inert doubles.
    """
    config=None

    def exercise(self, fault='healthy'):
        """Invoke the public caller once with controlled custody failures.

        The real collector hashes channels before the caller's new handler.
        Read/parse instrumentation distinguishes that first hash read from a
        later handler read. Completion-unknown may leave bytes but no known exit;
        missing hash refuses the first read. Later channel faults occur only
        after the recorded hash and initial outcome receipt. No actual process
        or external filesystem is mutated by any scenario.
        """
        cfg=self.config;helpers=cfg['helpers']
        trial=Path(tempfile.mkdtemp(prefix=fault+'-',dir=cfg['output']));os.chmod(trial,0o700)
        fixtures=trial/'fixtures';fixtures.mkdir(mode=0o700)
        driver=trial/'inert-driver';driver.write_bytes(b'INERT_DRIVER_NEVER_EXECUTE\n');os.chmod(driver,0o600)
        pin=helpers.digest(driver.read_bytes());out=trial/'caller'
        facts={'null_device_verified':True,'python_optimization':0,
               'git_witness':{'exit':1},'git_refusal':{'exit':128},'git_recovery':{'exit':1}}
        raw=(json.dumps(facts)+'\n').encode() if fault=='healthy' else b'INERT_BAD_JSON\n'
        stderr=b'INERT_HANDLER_CHILD_DIAGNOSTIC\n'
        state={'calls':0,'completed':False,'stdout_reads':0,'stdout_json_parses':0,
               'initial_receipt_written':False,'post_driver_reads':0,'events':[]}
        original_read=Path.read_bytes;original_loads=json.loads

        def read(path):
            """Count channel/postpin reads and inject only at owned boundaries."""
            if path==out/'stdio.stdout':
                state['stdout_reads']+=1;state['events'].append('stdout-read')
                if fault=='missing_hash' and state['stdout_reads']==1:
                    raise PermissionError('INERT_CHANNEL_HASH_READ_REFUSAL')
                if fault=='raw_unreadable' and state['initial_receipt_written']:
                    raise PermissionError('INERT_RETAINED_STDOUT_READ_REFUSAL')
            if path==driver and state['calls']:
                state['post_driver_reads']+=1;state['events'].append('post-driver-read')
            return original_read(path)

        def loads(value,*args,**kwargs):
            """Count only candidate parsing of this owned channel's exact bytes."""
            if isinstance(value,bytes) and value in (raw,b'INERT_CHANGED_RAW\n'):
                state['stdout_json_parses']+=1;state['events'].append('stdout-json-parse')
            return original_loads(value,*args,**kwargs)

        def publish(path,value):
            """Synthetic publisher double; inject after a known receipt or at failure.

            This does not qualify atomic writing or rollback. A channel mutation
            is a deliberate owned-fixture fault after its recorded hash, never
            a retry or modification of retained original execution evidence.
            """
            path=Path(path)
            if path==out/'failure.json' and fault=='final_publication':
                state['events'].append('final-publication-refused')
                raise OSError('INERT_FINAL_FAILURE_PUBLICATION_REFUSAL')
            data=value if isinstance(value,bytes) else (json.dumps(value,indent=2)+'\n').encode()
            path.write_bytes(data)
            if path==out/'stdio-result.json':
                state['initial_receipt_written']=True;state['events'].append('initial-receipt-written')
                if fault=='raw_changed':(out/'stdio.stdout').write_bytes(b'INERT_CHANGED_RAW\n')

        def fake_run(argv,**kwargs):
            """Simulate exactly one public driver call; no subprocess fallback exists."""
            if len(argv)!=4 or Path(argv[0])!=driver:raise RuntimeError('INERT_UNEXPECTED_ARGV')
            root,workspace,script=map(Path,argv[1:])
            if root.parent!=fixtures or workspace!=root/'out' or script!=root/'stdio-null-git.py' or kwargs.get('cwd')!=workspace:
                raise RuntimeError('INERT_UNOWNED_ARGV')
            state['calls']+=1
            if state['calls']!=1:raise RuntimeError('INERT_REPEATED_CHILD_FORBIDDEN')
            helpers.write_new(trial/'owned-attempt.json',{'kind':'INERT_DOUBLE_ONLY','argv':[str(x) for x in argv]})
            if fault=='spawn_failure':raise OSError('INERT_SPAWN_FAILURE_NO_COMPLETION')
            kwargs['stdout'].write(raw);kwargs['stderr'].write(stderr)
            if fault=='completion_unknown':raise OSError('INERT_COMPLETION_UNKNOWN_WITH_RAW_BYTES')
            state['completed']=True;state['events'].append('owned-completion')
            helpers.write_new(trial/'owned-completion.json',{'kind':'INERT_DOUBLE_ONLY','returncode':0,
                'stdout_sha256':helpers.digest(raw),'stderr_sha256':helpers.digest(stderr)})
            if fault in ('malformed_post','raw_changed','raw_unreadable','final_publication'):
                driver.write_bytes(b'INERT_DRIVER_DRIFT\n')
            return subprocess.CompletedProcess(argv,0)

        fixture=types.ModuleType('fixture_area');fixture.fixture_area=lambda:fixtures
        proof=types.ModuleType('rust_agent_model_route');proof.publish=publish;proof.digest=helpers.digest
        outcomes=types.ModuleType('native_build_outcome');outcomes.__file__=str(cfg['outcome_source'])
        candidate=types.ModuleType('inert_handler_candidate');candidate.__file__=str(cfg['candidate'])
        captured=io.StringIO();captured_err=io.StringIO();error=None;old_path=list(sys.path)
        try:
            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.dict(sys.modules,{'fixture_area':fixture,'rust_agent_model_route':proof,'native_build_outcome':outcomes}))
                stack.enter_context(patch.object(subprocess,'run',side_effect=fake_run))
                stack.enter_context(patch.object(Path,'read_bytes',read))
                stack.enter_context(patch.object(json,'loads',side_effect=loads))
                stack.enter_context(contextlib.redirect_stdout(captured));stack.enter_context(contextlib.redirect_stderr(captured_err))
                exec(compile(cfg['outcome_bytes'],outcomes.__file__,'exec'),outcomes.__dict__)
                exec(compile(cfg['candidate_bytes'],candidate.__file__,'exec'),candidate.__dict__)
                candidate.run_git_stdio(driver,out,pin)
        except BaseException as caught:error=caught
        finally:sys.path[:]=old_path
        documents={p.name:original_loads(original_read(p)) for p in out.glob('*.json')}
        channels={p.name:{'hex':original_read(p).hex(),'sha256':helpers.digest(original_read(p))}
                  for p in (out/'stdio.stdout',out/'stdio.stderr') if p.is_file()}
        observation={'kind':'INERT_DOUBLE_ONLY','trial':str(trial),'fault':fault,'state':state,'error':helpers.exception_tree(error),
            'documents':documents,'channels':channels,'captured_candidate_stdout':captured.getvalue(),
            'captured_candidate_stderr':captured_err.getvalue(),'biba_executed':False,'git_executed':False,
            'installed':False,'product_qualified':False}
        helpers.write_new(trial/'handler-observation.json',observation)
        return observation

    def assert_healthy(self):
        """Require genuine common caller success with inert bytes before each fault."""
        result=self.exercise();self.assertIsNone(result['error'])
        self.assertEqual(result['state']['calls'],1);self.assertTrue(result['state']['completed'])
        self.assertEqual(result['state']['stdout_reads'],2)
        self.assertEqual(result['state']['stdout_json_parses'],1)
        self.assertEqual(result['documents']['result.json']['status'],'PASS')
        self.assertFalse(result['documents']['result.json']['product_qualified'])

    def assert_fault(self,fault):
        """Retain healthy/fault/recovery, then assert exact typed custody states.

        Unknown completion/hash must cause no second channel read or JSON parse.
        Known corrupted/unreadable bytes must group original postcheck and the
        actual read/hash cause, without JSON parsing. Malformed known bytes must
        retain actual JSONDecodeError. Final-publication refusal must leave both
        original causes in the thrown tree and an actual outcome-bearing node.
        """
        h=self.config['helpers'];self.assert_healthy();result=self.exercise(fault);self.assert_healthy()
        state=result['state'];tree=result['error'];docs=result['documents']
        self.assertEqual(state['calls'],1);self.assertIsNotNone(tree)
        self.assertNotIn('result.json',docs);self.assertNotIn('BIBA_GIT_STDIO_PASS',result['captured_candidate_stdout'])
        carriers=[node['native_outcome'] for node in h.cause_nodes(tree) if isinstance(node.get('native_outcome'),dict)]
        self.assertTrue(carriers,'actual exception must carry collector state')
        unknown=fault in ('spawn_failure','completion_unknown')
        self.assertIs(state['completed'],not unknown)
        for value in carriers:
            process=value.get('subprocess');self.assertIsInstance(process,dict)
            self.assertIs(process['execution_attempted'],True)
            if unknown:self.assertIsNone(process['returncode'])
            else:self.assertIs(type(process['returncode']),int);self.assertEqual(process['returncode'],0)
            self.assertIs(value['admission_after']['checked'],True)
            self.assertIs(value['admission_after']['ok'],fault not in ('malformed_post','raw_changed','raw_unreadable','final_publication'))
            self.assertIs(value['receipt_publication']['ok'],True)
        self.assertEqual(state['post_driver_reads'],1)
        if fault in ('spawn_failure','completion_unknown','missing_hash'):
            self.assertEqual(state['stdout_reads'],1);self.assertEqual(state['stdout_json_parses'],0)
            self.assertFalse(h.typed_cause(tree,'JSONDecodeError'))
            kind='PermissionError' if fault=='missing_hash' else 'OSError'
            diagnostic={'spawn_failure':'INERT_SPAWN_FAILURE_NO_COMPLETION',
                'completion_unknown':'INERT_COMPLETION_UNKNOWN_WITH_RAW_BYTES',
                'missing_hash':'INERT_CHANNEL_HASH_READ_REFUSAL'}[fault]
            self.assertTrue(h.typed_cause(tree,kind,{diagnostic}))
            if fault=='missing_hash':
                for value in carriers:self.assertIsNone(value['subprocess']['stdout_sha256'])
        else:
            self.assertTrue(h.typed_cause(tree,'ValueError',{'driver image differs from the supplied SHA256'}))
            self.assertEqual(state['stdout_reads'],2)
            if fault=='raw_changed':
                self.assertEqual(state['stdout_json_parses'],0)
                self.assertTrue(h.typed_cause(tree,'ValueError',{'retained stdio stdout differs from recorded channel hash'}))
                for value in carriers:self.assertNotEqual(value['subprocess']['stdout_sha256'],result['channels']['stdio.stdout']['sha256'])
            elif fault=='raw_unreadable':
                self.assertEqual(state['stdout_json_parses'],0)
                self.assertTrue(h.typed_cause(tree,'PermissionError',{'INERT_RETAINED_STDOUT_READ_REFUSAL'}))
            else:
                self.assertEqual(state['stdout_json_parses'],1)
                self.assertTrue(h.typed_cause(tree,'JSONDecodeError'))
        if fault=='final_publication':
            self.assertNotIn('failure.json',docs)
            self.assertTrue(h.typed_cause(tree,'OSError',{'INERT_FINAL_FAILURE_PUBLICATION_REFUSAL'}))
        else:
            self.assertIn('failure.json',docs);self.assertIsInstance(docs['failure.json'].get('native_outcome'),dict)

    def test_healthy_handler_control(self):
        """Known valid output follows the same public entry with one inert call."""
        self.assert_healthy()

    def test_known_completion_malformed_and_postcheck_control(self):
        """Positive discriminator for independent JSON parsing after postcheck error."""
        self.assert_fault('malformed_post')

    def test_unknown_completion_does_not_read_or_parse_retained_bytes(self):
        """Raw bytes alone cannot authorize parsing without a known process exit."""
        self.assert_fault('completion_unknown')

    def test_spawn_failure_does_not_read_or_parse_empty_channels(self):
        """An interrupted spawn remains unknown, never parsed as empty success."""
        self.assert_fault('spawn_failure')

    def test_missing_stdout_hash_does_not_read_or_parse_again(self):
        """A failed collector hash cannot be replaced by an unbound protocol read."""
        self.assert_fault('missing_hash')

    def test_changed_retained_stdout_groups_hash_and_original_errors(self):
        """Recorded bytes must still match before independent JSON parsing."""
        self.assert_fault('raw_changed')

    def test_unreadable_retained_stdout_groups_read_and_original_errors(self):
        """Late read failure retains the known exit, postcheck cause and receipts."""
        self.assert_fault('raw_unreadable')

    def test_final_failure_publication_keeps_both_original_causes(self):
        """No final receipt does not erase the grouped protocol/collector failures."""
        self.assert_fault('final_publication')


def main():
    """Pin all sources and run only this additional eight-case inert inventory.

    Requires a separately authorized invocation and existing allocator-admitted
    evidence parent. No direct import of fixture_area or engine configuration.
    Before/after source reads, every trial, unittest stream and summary are
    retained. Missing/changed source or zero/missing/skipped tests refuse. No
    global watchdog, retries, subprocess or external model call is provided.
    """
    if sys.flags.optimize:raise RuntimeError('inert branch assertions require normal Python')
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('candidate','outcome-source','helpers-source','evidence-parent'):parser.add_argument('--'+name,type=Path,required=True)
    for name in ('candidate-sha256','outcome-sha256','helpers-sha256'):parser.add_argument('--'+name,required=True)
    args=parser.parse_args();sources={}
    declared=[('candidate',args.candidate,args.candidate_sha256),('outcome',args.outcome_source,args.outcome_sha256),
              ('helpers',args.helpers_source,args.helpers_sha256)]
    for name,path,pin in declared:
        if len(pin)!=64 or any(c not in '0123456789abcdef' for c in pin):raise ValueError('invalid source SHA256')
        data=path.read_bytes()
        if __import__('hashlib').sha256(data).hexdigest()!=pin:raise ValueError(name+' source hash differs')
        sources[name]=data
    helpers=types.ModuleType('inert_retention_helpers');helpers.__file__=str(args.helpers_source.resolve())
    exec(compile(sources['helpers'],helpers.__file__,'exec'),helpers.__dict__)
    parent=args.evidence_parent.resolve(strict=True)
    if not parent.is_dir():raise ValueError('evidence parent is not a directory')
    out=Path(tempfile.mkdtemp(prefix='inert-handler-branches-',dir=parent));os.chmod(out,0o700)
    HandlerBranches.config={'helpers':helpers,'candidate':args.candidate.resolve(),'candidate_bytes':sources['candidate'],
        'outcome_source':args.outcome_source.resolve(),'outcome_bytes':sources['outcome'],'output':out}
    helpers.write_new(out/'inputs.json',dict(kind='INERT_DOUBLE_ONLY',argv=sys.argv,python=sys.version,
        declared=[dict(role=name,path=str(path.resolve()),sha256=pin) for name,path,pin in declared],
        harness_sha256=helpers.digest(Path(__file__).read_bytes()),biba_executed=False,git_executed=False,global_watchdog=None))
    for name,data in sources.items():
        path=out/(name+'-source.py');path.write_bytes(data);os.chmod(path,0o600)
    with (out/'unittest.stderr').open('x') as stream:
        result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(HandlerBranches))
        stream.flush();os.fsync(stream.fileno())
    post={}
    for name,path,pin in declared:
        try:
            actual=helpers.digest(path.read_bytes());post[name]={'ok':actual==pin,'sha256':actual}
        except OSError as error:post[name]={'ok':False,'error':helpers.exception_tree(error)}
    ok=result.wasSuccessful() and result.testsRun==8 and not result.skipped and all(value['ok'] for value in post.values())
    helpers.write_new(out/'summary.json',dict(status='PASS' if ok else 'FAIL',kind='INERT_DOUBLE_ONLY',
        tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),skipped=len(result.skipped),
        source_postcheck=post,biba_executed=False,git_executed=False,installed=False,product_qualified=False))
    print('INERT_HANDLER_BRANCHES_EVIDENCE '+str(out));print('INERT_HANDLER_BRANCHES_PASS' if ok else 'INERT_HANDLER_BRANCHES_FAIL')
    return 0 if ok else 1


if __name__=='__main__':raise SystemExit(main())
