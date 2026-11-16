#!/usr/bin/env python3
"""Verify caller retention with inert doubles, never Biba, Git or a driver.

The same harness consumes either hash-pinned old or corrected source. It invokes
the existing public run_git_stdio; the candidate's stdio_probe remains real Python
but its subprocess.run is replaced before import and for the whole invocation.
Embedded programs are written as fixture text, never imported or executed.
Proof/fixture modules are explicit doubles, not atomic-publication qualification.
All candidate PASS markers are captured privately and never printed as evidence
of Biba, Git, installation, enforcement or production acceptance. No retry,
provider, build, global watchdog or real child is supported by this harness.
"""
import argparse
import contextlib
import hashlib
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


def digest(raw):
    """Return SHA256 of full input bytes; a hash is not origin or authority."""
    return hashlib.sha256(raw).hexdigest()


def write_new(path, value):
    """Retain fresh private JSON without replacing another run's evidence.

    Encode before opening, then exclusive-create, flush and fsync. A write/sync
    error propagates; the visible file may be incomplete, never rollback proof.
    """
    data=(json.dumps(value,indent=2,ensure_ascii=False)+'\n').encode()
    with path.open('xb') as stream:
        os.chmod(path,0o600)
        stream.write(data);stream.flush();os.fsync(stream.fileno())


def exception_tree(error):
    """Expose all actual exception types/causes/outcomes without flattening them.

    The controlled errors contain only fixture diagnostics. Cycle markers avoid
    recursion loops. This never changes or rethrows the candidate's exception.
    """
    seen=set()
    def visit(value):
        """Visit one observed cause once, retaining nested collector receipts."""
        if id(value) in seen:return {'cycle':True}
        seen.add(id(value))
        node={'type':type(value).__name__,'message':str(value)}
        if hasattr(value,'native_outcome'):node['native_outcome']=value.native_outcome
        if getattr(value,'exceptions',None):node['exceptions']=[visit(e) for e in value.exceptions]
        if value.__cause__ is not None:node['cause']=visit(value.__cause__)
        elif value.__context__ is not None and not value.__suppress_context__:node['context']=visit(value.__context__)
        return node
    return None if error is None else visit(error)


def cause_nodes(tree):
    """Traverse only typed exception branches produced by exception_tree/describe.

    Messages and arbitrary metadata are never reparsed as exceptions. Missing,
    malformed or cycle nodes yield no inferred cause; this traversal grants no
    evidence from text merely naming a desired type or outcome field.
    """
    if not isinstance(tree,dict):return
    if isinstance(tree.get('type'),str):yield tree
    children=tree.get('exceptions')
    if isinstance(children,list):
        for child in children:yield from cause_nodes(child)
    for key in ('cause','context'):
        yield from cause_nodes(tree.get(key))


def typed_cause(tree, kind, messages=None):
    """Find an actual typed cause, optionally matching its own safe diagnostic.

    A message in another node, metadata or a serialized tree cannot satisfy the
    predicate. messages is a set of diagnostics from the pinned source/double,
    not a parser for exception type names.
    """
    return any(node.get('type')==kind and (messages is None or node.get('message') in messages)
               for node in cause_nodes(tree))


def outcome_issues(value, code, post, publication):
    """Check exact collector outcome fields, returning independent violations.

    This pure assertion helper consumes observed native_outcome dictionaries.
    It verifies exact integers/booleans, known process completion, failed attempted
    postcheck and the typed diagnostic at that boundary. Publication faults must
    occur in their own structured field. Missing/text-only data always refuses;
    no conversion, default success or search in serialized messages is allowed.
    """
    issues=[]
    if not isinstance(value,dict):return ['native_outcome is not an object']
    process=value.get('subprocess')
    if not isinstance(process,dict):issues.append('subprocess is not an object')
    else:
        if type(process.get('returncode')) is not int or process['returncode']!=code:issues.append('wrong observed returncode')
        if process.get('signal','MISSING') is not None:issues.append('unexpected or missing signal')
        if process.get('execution_attempted') is not True:issues.append('execution attempt is not true')
    after=value.get('admission_after')
    if not isinstance(after,dict):issues.append('admission_after is not an object')
    else:
        if after.get('checked') is not True:issues.append('postcheck not attempted')
        if after.get('ok') is not False:issues.append('postcheck is not explicitly failed')
        kind='PermissionError' if post=='unreadable' else 'ValueError'
        messages=({'INERT_POSTHASH_READ_REFUSAL'} if post=='unreadable' else
                  {'driver image differs from the supplied SHA256','driver image changed during stdio verification'})
        if not typed_cause(after.get('error'),kind,messages):issues.append('typed postcheck cause missing at admission boundary')
    receipt=value.get('receipt_publication')
    if not isinstance(receipt,dict):issues.append('receipt_publication is not an object')
    elif publication=='first':
        if receipt.get('ok') is not False:issues.append('first publication is not explicitly failed')
        if not typed_cause(receipt.get('error'),'OSError',{'INERT_PUBLICATION_REFUSAL_first'}):issues.append('first typed publication error missing')
    elif receipt.get('ok') is not True:issues.append('original process receipt is not confirmed published')
    if publication=='second':
        diagnostic=value.get('diagnostic_publication')
        if not isinstance(diagnostic,dict):issues.append('diagnostic_publication is not an object')
        else:
            if diagnostic.get('ok') is not False:issues.append('second publication is not explicitly failed')
            if not typed_cause(diagnostic.get('error'),'OSError',{'INERT_PUBLICATION_REFUSAL_second'}):issues.append('second typed publication error missing')
    return issues


class InertRetention(unittest.TestCase):
    """Common requirements: healthy controls, single completion, independent causes.

    config is injected by main after explicit source pins and private parent
    validation. Each trial creates a new retained directory; none is cleaned up.
    """
    config=None

    def exercise(self, scenario='healthy', post='unchanged', publication=None):
        """Call the public entry once with no subprocess execution possible.

        A fake completed process provides exact stdout/stderr. Only its inert
        driver file may drift or become unreadable to the candidate after the
        completion marker. Publication injection means the first channel/receipt
        publication after completion for either source (old stdout, new receipt),
        or the second (old stderr, new postcheck receipt). Atomicity is not tested.
        Returns observed errors/files/events; no failure becomes a green no-op.
        """
        config=self.config
        trial=Path(tempfile.mkdtemp(prefix=scenario+'-',dir=config['output']))
        os.chmod(trial,0o700)
        fixtures=trial/'fixtures';fixtures.mkdir(mode=0o700)
        driver=trial/'inert-driver';driver.write_bytes(b'INERT_DOUBLE_NEVER_EXECUTE\n');os.chmod(driver,0o600)
        expected=digest(driver.read_bytes());output=trial/'caller'
        facts={'null_device_verified':True,'python_optimization':0,
               'git_witness':{'exit':1},'git_refusal':{'exit':128},'git_recovery':{'exit':1}}
        raw_stdout=(json.dumps(facts)+'\n').encode() if scenario!='malformed' else b'INERT_MALFORMED_JSON\n'
        raw_stderr=b'INERT_PRIMARY_CHILD_DIAGNOSTIC\n'
        code=7 if scenario=='nonzero' else 0
        state={'calls':0,'completed':False,'post_reads':0,'publication_attempts':[],
               'publication_failures':[],'raw_stdout_sha256':digest(raw_stdout),
               'raw_stderr_sha256':digest(raw_stderr),'returncode':code,'events':[]}
        original_read=Path.read_bytes

        def read(path):
            """Observe all post-completion driver reads, optionally refusing them."""
            if path==driver and state['completed']:
                state['post_reads']+=1;state['events'].append('post-driver-read')
                if post=='unreadable':raise PermissionError('INERT_POSTHASH_READ_REFUSAL')
            return original_read(path)

        def publish(path, value):
            """Controlled proof publisher double, with one logical publication fault.

            Preserve byte/dict encoding accepted by the actual helper; do not
            simulate fsync, atomic rename or rollback. Injection occurs only
            after the exact fake-child completion marker is retained.
            """
            path=Path(path);name=path.name
            state['publication_attempts'].append(name)
            first=name in ('stdio.stdout','stdio-result.json')
            second=name in ('stdio.stderr','stdio-admission-after.json')
            if state['completed'] and ((publication=='first' and first) or (publication=='second' and second)):
                state['publication_failures'].append(name)
                raise OSError('INERT_PUBLICATION_REFUSAL_'+publication)
            raw=value if isinstance(value,bytes) else (json.dumps(value,indent=2)+'\n').encode()
            path.write_bytes(raw)

        def fake_run(argv, **kwargs):
            """Complete one owned inert call; reject every unexpected command.

            No fallback to subprocess.run exists. Accept only driver/root/out/
            script fixture argv. Write to provided direct streams or return the
            same bytes for legacy PIPE. Then record completion and optional
            inert-file drift. stderr is known even for nonzero/invalid JSON.
            """
            if len(argv)!=4 or Path(argv[0])!=driver or kwargs.get('cwd') is None:
                raise RuntimeError('INERT_HARNESS_UNEXPECTED_COMMAND')
            root=Path(argv[1]);workspace=Path(argv[2]);script=Path(argv[3])
            if root.parent!=fixtures or workspace!=root/'out' or script!=root/'stdio-null-git.py':
                raise RuntimeError('INERT_HARNESS_NONOWNED_COMMAND')
            if not script.is_file():raise RuntimeError('INERT_HARNESS_MISSING_SCRIPT')
            state['calls']+=1
            if state['calls']!=1:raise RuntimeError('INERT_HARNESS_REPEAT_FORBIDDEN')
            for key,raw in [('stdout',raw_stdout),('stderr',raw_stderr)]:
                target=kwargs.get(key)
                if target!=subprocess.PIPE:
                    if not hasattr(target,'write'):raise RuntimeError('INERT_HARNESS_MISSING_CHANNEL')
                    target.write(raw)
            write_new(trial/'owned-completion.json',dict(kind='INERT_DOUBLE_ONLY',argv=[str(x) for x in argv],
                returncode=code,stdout_sha256=digest(raw_stdout),stderr_sha256=digest(raw_stderr)))
            state['completed']=True;state['events'].append('owned-completion')
            if post=='drift':driver.write_bytes(b'INERT_CHANGED_DRIVER_NOT_EXECUTED\n')
            return subprocess.CompletedProcess(argv,code,raw_stdout if kwargs.get('stdout')==subprocess.PIPE else None,
                raw_stderr if kwargs.get('stderr')==subprocess.PIPE else None)

        fixture=types.ModuleType('fixture_area');fixture.fixture_area=lambda:fixtures
        proof=types.ModuleType('rust_agent_model_route');proof.publish=publish;proof.digest=digest
        outcomes=types.ModuleType('native_build_outcome');outcomes.__file__=str(config['outcome_source'])
        module=types.ModuleType('inert_boundary_candidate');module.__file__=str(config['candidate'])
        captured_out=io.StringIO();captured_err=io.StringIO();error=None;old_path=list(sys.path)
        try:
            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.dict(sys.modules,{'fixture_area':fixture,'rust_agent_model_route':proof,
                    'native_build_outcome':outcomes}))
                stack.enter_context(patch.object(subprocess,'run',side_effect=fake_run))
                stack.enter_context(patch.object(Path,'read_bytes',read))
                stack.enter_context(contextlib.redirect_stdout(captured_out));stack.enter_context(contextlib.redirect_stderr(captured_err))
                # Outcome module has only stdlib imports/definitions. Candidate imports
                # above names as doubles; main guard never runs. No embedded script runs.
                exec(compile(config['outcome_bytes'],outcomes.__file__,'exec'),outcomes.__dict__)
                exec(compile(config['candidate_bytes'],module.__file__,'exec'),module.__dict__)
                module.run_git_stdio(driver,output,expected)
        except BaseException as caught:error=caught
        finally:sys.path[:]=old_path
        channels={name:original_read(output/name) for name in ('stdio.stdout','stdio.stderr') if (output/name).is_file()}
        documents={p.name:json.loads(original_read(p)) for p in output.glob('*.json')}
        result={'trial':str(trial),'state':state,'error':exception_tree(error),'documents':documents,
                'channels':{name:{'sha256':digest(raw),'hex':raw.hex()} for name,raw in channels.items()},
                'captured_candidate_stdout':captured_out.getvalue(),'captured_candidate_stderr':captured_err.getvalue(),
                'kind':'INERT_DOUBLE_ONLY','biba_executed':False,'git_executed':False,'installed':False}
        write_new(trial/'harness-observation.json',result)
        return result,raw_stdout,raw_stderr

    def assert_healthy(self):
        """Require the same public path to succeed with exact inert channels first."""
        result,stdout,stderr=self.exercise()
        self.assertIsNone(result['error']);self.assertEqual(result['state']['calls'],1)
        self.assertTrue(result['state']['completed']);self.assertGreaterEqual(result['state']['post_reads'],1)
        self.assertEqual(result['channels']['stdio.stdout']['hex'],stdout.hex())
        self.assertEqual(result['channels']['stdio.stderr']['hex'],stderr.hex())
        self.assertEqual(result['documents']['result.json']['scenario'],'git-stdio')
        self.assertFalse(result['documents']['result.json']['product_qualified'])

    def assert_retention(self,scenario,post,publication=None):
        """Healthy before/after, then check every causal dimension independently.

        Assertions are invariant across sources. No expected-failure decorator,
        version branch, symbol substitution or missing-file-as-empty convention
        hides a missing receipt/cause. Invalid JSON must retain its parser cause
        together with postcheck failure; a raw malformed buffer alone is weaker.
        """
        self.assert_healthy()
        result,stdout,stderr=self.exercise(scenario,post,publication)
        # Run recovery before assertions so a failing RED still retains recovery.
        self.assert_healthy()
        with self.subTest(contract='one-completed-inert-child'):
            self.assertEqual(result['state']['calls'],1);self.assertTrue(result['state']['completed'])
        with self.subTest(contract='postcheck-attempted'):
            self.assertGreaterEqual(result['state']['post_reads'],1)
        for name,raw in [('stdio.stdout',stdout),('stdio.stderr',stderr)]:
            with self.subTest(contract='exact-raw-channel',channel=name):
                self.assertIn(name,result['channels'])
                if name in result['channels']:self.assertEqual(result['channels'][name]['hex'],raw.hex())
        with self.subTest(contract='refusal-no-final-pass'):
            self.assertIsNotNone(result['error']);self.assertNotIn('result.json',result['documents'])
            self.assertNotIn('BIBA_GIT_STDIO_PASS',result['captured_candidate_stdout'])
        error_tree=result['error']
        with self.subTest(contract='postcheck-cause'):
            kind='PermissionError' if post=='unreadable' else 'ValueError'
            messages=({'INERT_POSTHASH_READ_REFUSAL'} if post=='unreadable' else
                      {'driver image differs from the supplied SHA256','driver image changed during stdio verification'})
            self.assertTrue(typed_cause(error_tree,kind,messages))
        if scenario=='malformed':
            with self.subTest(contract='primary-json-cause'):
                self.assertTrue(typed_cause(error_tree,'JSONDecodeError'))
        with self.subTest(contract='structured-process-postcheck'):
            self.assertIn('failure.json',result['documents'])
            expected_code=7 if scenario=='nonzero' else 0
            carriers=[node['native_outcome'] for node in cause_nodes(error_tree) if 'native_outcome' in node]
            self.assertTrue(carriers,'no actual exception node carries native_outcome')
            for carrier in carriers:
                self.assertEqual(outcome_issues(carrier,expected_code,post,publication),[])
                process=carrier.get('subprocess',{}) if isinstance(carrier,dict) else {}
                self.assertEqual(process.get('stdout_sha256'),digest(stdout))
                self.assertEqual(process.get('stderr_sha256'),digest(stderr))
            failure=result['documents'].get('failure.json')
            self.assertIsInstance(failure,dict)
            if isinstance(failure,dict):
                self.assertEqual(outcome_issues(failure.get('native_outcome'),expected_code,post,publication),[])
                if scenario=='malformed':self.assertTrue(typed_cause(failure.get('causes'),'JSONDecodeError'))
        if scenario=='nonzero':
            with self.subTest(contract='primary-process-cause'):
                self.assertTrue(typed_cause(error_tree,'ValueError',{'stdio-driver-nonzero'}))
        if publication:
            with self.subTest(contract='publication-cause-and-attempt'):
                self.assertEqual(len(result['state']['publication_failures']),1)
                self.assertTrue(typed_cause(error_tree,'OSError',{'INERT_PUBLICATION_REFUSAL_'+publication}))

    def assert_typed_assertion_guard(self):
        """Guard against message-only false positives, using inert exception objects.

        Future execution compares genuine JSONDecodeError and grouped typed
        causes with RuntimeError merely naming those types/fields. Also reject
        text-only or wrong-code receipts. These are pure assertion controls,
        never driver/Git evidence or additional named tests.
        """
        fake=exception_tree(RuntimeError('JSONDecodeError INERT_POSTHASH_READ_REFUSAL returncode 7 admission_after checked true ok false'))
        self.assertFalse(typed_cause(fake,'JSONDecodeError'))
        self.assertFalse(typed_cause(fake,'PermissionError',{'INERT_POSTHASH_READ_REFUSAL'}))
        for branch in ('exceptions','cause','context'):
            with self.subTest(guard='message-in-causal-branch',branch=branch):
                wrapper={'type':'RuntimeError','message':'outer',branch:[fake] if branch=='exceptions' else fake}
                self.assertFalse(typed_cause(wrapper,'JSONDecodeError'))
        genuine=exception_tree(BaseExceptionGroup('inert causes',[
            json.JSONDecodeError('inert malformed JSON','x',0),PermissionError('INERT_POSTHASH_READ_REFUSAL')]))
        self.assertTrue(typed_cause(genuine,'JSONDecodeError'))
        self.assertTrue(typed_cause(genuine,'PermissionError',{'INERT_POSTHASH_READ_REFUSAL'}))
        mismatched={'type':'RuntimeError','message':'INERT_POSTHASH_READ_REFUSAL','cause':{'type':'PermissionError','message':'other diagnostic'}}
        self.assertFalse(typed_cause(mismatched,'PermissionError',{'INERT_POSTHASH_READ_REFUSAL'}))
        self.assertNotEqual(outcome_issues({'message':'returncode 7 admission_after checked true ok false'},7,'unreadable',None),[])
        valid={'subprocess':{'returncode':7,'signal':None,'execution_attempted':True},
               'admission_after':{'checked':True,'ok':False,'error':{'type':'PermissionError','message':'INERT_POSTHASH_READ_REFUSAL'}},
               'receipt_publication':{'ok':True}}
        self.assertEqual(outcome_issues(valid,7,'unreadable',None),[])
        wrong_code={**valid,'subprocess':{**valid['subprocess'],'returncode':0}}
        self.assertNotEqual(outcome_issues(wrong_code,7,'unreadable',None),[])
        wrong_bool={**valid,'admission_after':{**valid['admission_after'],'checked':1}}
        self.assertNotEqual(outcome_issues(wrong_bool,7,'unreadable',None),[])
        wrong_publication={**valid,'receipt_publication':{'ok':False,'error':{'type':'RuntimeError','message':'OSError INERT_PUBLICATION_REFUSAL_first'}}}
        self.assertNotEqual(outcome_issues(wrong_publication,7,'unreadable','first'),[])
        for branch in ('__cause__','__context__'):
            with self.subTest(guard='actual-nested-parser-type',branch=branch):
                outer=RuntimeError('inert outer')
                setattr(outer,branch,json.JSONDecodeError('inert malformed JSON','x',0))
                self.assertTrue(typed_cause(exception_tree(outer),'JSONDecodeError'))

    def test_healthy_control(self):
        """No-fault common path must pass before any failure evidence is meaningful."""
        self.assert_healthy()
        self.assert_typed_assertion_guard()

    def test_nonzero_and_driver_drift(self):
        """Preserve original process failure and an independent changed-image cause."""
        self.assert_retention('nonzero','drift')

    def test_nonzero_and_posthash_read_refusal(self):
        """Unknown posthash remains distinct from known child completion/nonzero."""
        self.assert_retention('nonzero','unreadable')

    def test_malformed_json_and_driver_drift(self):
        """Retain parser and driver causes, rather than only malformed raw bytes."""
        self.assert_retention('malformed','drift')

    def test_malformed_json_and_posthash_read_refusal(self):
        """Posthash observation failure must not erase the primary parser cause."""
        self.assert_retention('malformed','unreadable')

    def test_first_postcompletion_publication_and_drift(self):
        """Same injection: old stdout publication versus new process receipt."""
        self.assert_retention('nonzero','drift','first')

    def test_first_postcompletion_publication_and_read_refusal(self):
        """First publication refusal keeps both channels and unknown posthash cause."""
        self.assert_retention('nonzero','unreadable','first')

    def test_second_postcompletion_publication_and_drift(self):
        """Second channel/receipt refusal reaches the final structured consumer."""
        self.assert_retention('nonzero','drift','second')


def main():
    """Validate explicit input pins, allocate fresh evidence and run eight tests.

    Parent storage admission belongs to the allocator; require an existing
    writable parent, never auto-create an absent mount point. Preserve inputs,
    every trial, unittest stream and summary. Exit nonzero for failures/zero
    collection; no baseline mode converts RED to PASS. Execution needs a separate
    grant; possessing this file or a recipe is not that grant.
    """
    if sys.flags.optimize:raise RuntimeError('inert harness requires Python assertions active')
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('candidate','outcome-source','evidence-parent'):parser.add_argument('--'+name,type=Path,required=True)
    for name in ('candidate-sha256','outcome-sha256'):parser.add_argument('--'+name,required=True)
    args=parser.parse_args()
    sources={}
    for label,path,pin in [('candidate',args.candidate,args.candidate_sha256),('outcome',args.outcome_source,args.outcome_sha256)]:
        if len(pin)!=64 or any(c not in '0123456789abcdef' for c in pin):raise ValueError('invalid source SHA256')
        data=path.read_bytes()
        if digest(data)!=pin:raise ValueError(label+' source differs from supplied SHA256')
        sources[label]=data
    parent=args.evidence_parent.resolve(strict=True)
    if not parent.is_dir():raise ValueError('evidence parent is not a directory')
    out=Path(tempfile.mkdtemp(prefix='inert-stdio-retention-',dir=parent));os.chmod(out,0o700)
    InertRetention.config={'candidate':args.candidate.resolve(),'outcome_source':args.outcome_source.resolve(),
        'candidate_bytes':sources['candidate'],'outcome_bytes':sources['outcome'],'output':out}
    write_new(out/'inputs.json',dict(kind='INERT_DOUBLE_ONLY',candidate=str(args.candidate.resolve()),
        candidate_sha256=args.candidate_sha256,outcome_source=str(args.outcome_source.resolve()),
        outcome_sha256=args.outcome_sha256,harness_sha256=digest(Path(__file__).read_bytes()),argv=sys.argv,
        python=sys.version,global_watchdog=None,biba_executed=False,git_executed=False,provider_calls=0))
    for name,data in sources.items():
        target=out/(name+'-source.py');target.write_bytes(data);os.chmod(target,0o600)
    with (out/'unittest.stderr').open('x') as stream:
        result=unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(InertRetention))
        stream.flush();os.fsync(stream.fileno())
    post_observations={}
    for label,path,pin in [('candidate',args.candidate,args.candidate_sha256),('outcome',args.outcome_source,args.outcome_sha256)]:
        try:
            actual=digest(path.read_bytes())
            post_observations[label]={'ok':actual==pin,'sha256':actual}
        except OSError as error:
            post_observations[label]={'ok':False,'error':exception_tree(error)}
    stable=all(value['ok'] for value in post_observations.values())
    ok=result.wasSuccessful() and result.testsRun==8 and not result.skipped and stable
    write_new(out/'summary.json',dict(kind='INERT_DOUBLE_ONLY',status='PASS' if ok else 'FAIL',tests_run=result.testsRun,
        failures=len(result.failures),errors=len(result.errors),skipped=len(result.skipped),source_stable=stable,
        post_source_observations=post_observations,
        biba_executed=False,git_executed=False,installed=False,product_qualified=False))
    print('INERT_STDIO_RETENTION_EVIDENCE '+str(out))
    print('INERT_STDIO_RETENTION_PASS' if ok else 'INERT_STDIO_RETENTION_FAIL')
    return 0 if ok else 1


if __name__=='__main__':raise SystemExit(main())
