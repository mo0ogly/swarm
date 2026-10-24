"""Source-bound context codec parity and stored review/Git binding verification."""
import datetime,json,os,shutil,subprocess,sys,tempfile,time
from collections import Counter
from pathlib import Path
from qualification_inputs import collect
ROOT=Path(__file__).resolve().parents[2]

def main():
    """Qualify current sources on isolated fixtures; preserve failures and never deploy or invoke providers."""
    output=Path(sys.argv[1]).resolve()
    if output.exists():raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True);work=Path(tempfile.mkdtemp(prefix='review-context-',dir=ROOT/'.cache'))
    fixture=Path(tempfile.mkdtemp(prefix='.swarm-rust-context-',dir=Path.home()));fixture.chmod(0o700)
    env=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')),TMPDIR=str(fixture));commands=[]
    def save(name,value):
        """Atomically write one completed JSON record and propagate storage errors."""
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def run(label,args,data=None,expected=0):
        """Run exact arguments without a global watchdog; preserve original bytes and reject unexpected exits."""
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
        p=subprocess.run([os.fspath(a)if isinstance(a,os.PathLike)else a for a in args],cwd=ROOT,env=env,input=data,capture_output=True)
        for suffix,raw in [('stdout',p.stdout),('stderr',p.stderr)]: (output/(label+'.'+suffix)).write_bytes(raw)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        commands.append(dict(label=label,command=[str(a)for a in args],start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p.stdout
    def inventory(label,args):
        """Require every named collected test to execute successfully, rejecting a zero-test proof."""
        text=run(label,args).decode();listing=run(label+'-inventory',args+['--','--list']).decode()
        actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in text.splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
        expected=Counter(l.removesuffix(': test')for l in listing.splitlines()if l.endswith(': test'))
        assert actual and actual==expected,(label,actual-expected,expected-actual);save(label+'-names.json',list(actual.elements()));return sum(actual.values())
    cargo=['--manifest-path','rust/Cargo.toml','--locked','--offline']
    try:
        raw=run('reference-source',['go','run','./tools/rust-migration/review-context-reference']);source=work/'main.go';source.write_bytes(raw);package='./'+str(work.relative_to(ROOT))
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./internal/engine','./tools/rust-migration/review-context-reference',package],allow_root=True,extra_files=[source])
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check']);run('compile',['cargo','check',*cargo,'--workspace','--all-features','--all-targets']);run('clippy',['cargo','clippy',*cargo,'--workspace','--all-features','--all-targets','--','-D','warnings'])
        reference=work/'go-reference';run('reference-build',['go','build','-o',reference,source]);run('reference-vet',['go','vet','./tools/rust-migration/review-context-reference',package]);assert raw==run('reference-after',['go','run','./tools/rust-migration/review-context-reference'])
        tests=dict(workspace=inventory('workspace-tests',['cargo','test',*cargo,'--workspace','--all-features','--all-targets']),default_engine=inventory('default-engine-tests',['cargo','test',*cargo,'-p','swarm-engine','--all-targets']))
        run('rustdoc',['cargo','doc',*cargo,'-p','swarm-engine','--all-features','--no-deps']);doc=run('doctest',['cargo','test',*cargo,'-p','swarm-engine','--doc']).decode();assert '1 passed; 0 failed' in doc
        run('probe-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--example','review_context_probe']);native=Path(env['CARGO_TARGET_DIR'])/'debug/examples/review_context_probe'
        vectors=[('empty','{}',True,True),('null','null',True,True),('tasks-empty','{"tasks":[]}',True,True),('raw-large','{"receipt":{"opaque":1e999,"n":-0,"unicode":"<>&\\u2028"}}',True,True),('aliases-null','{"candidate_commit":"A","CANDIDATE_COMMIT":null,"tasks":[{"title":"T","TITLE":null}]}',True,True),('slice-reuse','{"tasks":[{"task":"A","title":"T"}],"TASKS":[{"task":"B"}]}',True,True),('slice-empty','{"tasks":[{"task":"A","title":"T"}],"tasks":[],"TASKS":[{"task":"B"}]}',True,True),('pointer-merge','{"accepted_baseline":{"candidate_commit":"A"},"ACCEPTED_BASELINE":{"version":2}}',True,True),('pointer-null','{"accepted_baseline":{"candidate_commit":"A"},"accepted_baseline":null}',True,True),('surrogate','{"diff":"\\ud800"}',True,True)]
        full=dict(citation_contract_version=2,inspection_scope_version=1,inspection_only=True,candidate_commit='candidate',previous_candidate='previous',diff='diff <>&\u2028',receipt=dict(raw=[1,2]),sources=[dict(path='path',git_blob='blob',sha256='hash',bytes=3,content='Text')],tasks=[dict(task='task',title='Task',deliverable='report.md',criteria=['Criterion'],report='Report',controls=[dict(id='control',executed=True,passed=True)],binding=dict(task='task',attempt='attempt',producer='producer',contract='contract',policy='policy',report='report.md',report_sha256='digest',reason='Observed',criteria=[dict(index=1,verdict='pass',evidence='A quote')]),delivery=dict(version=2,task='task',attempt='attempt',contract='contract',outcome='ready',criteria=[dict(index=1,status='not_tested',reason='Precondition',controls=['control'],evidence=['source.rs'])]))],accepted_baseline=dict(version=2,candidate_commit='previous',accepted_reviews=[dict(id='prior',state='passed',report_artifacts={'arbitrary-key':'digest'})]))
        vectors.append(('full',json.dumps(full),True,True))
        anchor={}
        for _ in range(140):anchor=dict(replanned_from=anchor)
        vectors.append(('deep-fragment-data',json.dumps(dict(accepted_baseline=dict(accepted_reviews=[dict(fragment_journal=anchor)]))),True,True))
        unknown=[
            dict(unknown=1),dict(tasks=[dict(unknown=1)]),
            dict(tasks=[dict(delivery=dict(unknown=1))]),
            dict(tasks=[dict(delivery=dict(criteria=[dict(unknown=1)]))]),
            dict(accepted_baseline=dict(unknown=1)),
            dict(accepted_baseline=dict(accepted_reviews=[dict(unknown=1)])),
            dict(sources=[dict(unknown=1)]),
            dict(tasks=[dict(binding=dict(unknown=1))]),
            dict(tasks=[dict(controls=[dict(unknown=1)])]),
        ]
        for i,value in enumerate(unknown):vectors.append((f'unknown-{i}',json.dumps(value),False,True))
        for name,raw in [('version-float','{"citation_contract_version":1.0}'),('version-exponent','{"citation_contract_version":1e0}'),('version-overflow','{"citation_contract_version":9223372036854775808}'),('known-type','{"tasks":[{"criteria":{}}]}'),('nested-type','{"tasks":[{"delivery":{"criteria":[{"index":"1"}]}}]}'),('baseline-type','{"accepted_baseline":{"accepted_reviews":"wrong"}}'),('raw-syntax','{"receipt":[1,]}'),('array','[]'),('trailing','{}{}'),('syntax','{')]:vectors.append((name,raw,False,False))
        for name,raw,strict_ok,compatible_ok in vectors:
            for mode,valid in [('strict',strict_ok),('compatible',compatible_ok)]:
                witness=b'{"candidate_commit":"control"}';assert run(name+'-'+mode+'-control-go',[reference,mode],witness)==run(name+'-'+mode+'-control-native',[native,mode],witness)
                data=raw.encode();expected=0 if valid else 2;go=run(name+'-'+mode+'-go',[reference,mode],data,expected);rust=run(name+'-'+mode+'-native',[native,mode],data,expected)
                if valid:assert go==rust,(name,mode)
                else:assert not go and not rust
        save('vectors.json',[dict(name=n,raw=r,strict_ok=s,compatible_ok=c)for n,r,s,c in vectors])
        run('diff-check',['git','diff','--check'])
        for name,binary in [('go-reference',reference),('native-context-probe',native)]:dest=output/(name+'.binary');shutil.copyfile(binary,dest);dest.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3ar42 typed Go context codec and stored Work/files/Git observation; no managed acceptance',tests=tests,codec_vectors=len(vectors),codec_comparisons=len(vectors)*2,valid_controls_before_each=True,stored_row_refusal_recovery_variants=10,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,provider_calls=False,independent_review=False,deployment=False))
        print('PASS typed context and stored review/Git component; PARTIAL full migration',flush=True)
    except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,fixture_root=str(fixture)));raise

if __name__=='__main__':main()
