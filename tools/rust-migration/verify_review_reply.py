"""Qualify native reply parsing, v7 policy and affected shared settings consumers."""
import datetime,hashlib,json,os,shutil,subprocess,sys,tempfile,time
from collections import Counter
from pathlib import Path
from qualification_inputs import collect
from recorded_process import capture
ROOT=Path(__file__).resolve().parents[2]

def main():
    """Freeze effective sources, verify components and retain original failures; never deploy or call providers."""
    output=Path(sys.argv[1]).resolve()
    if output.exists():raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True)
    scratch=Path(tempfile.mkdtemp(prefix='review-reply-',dir=ROOT/'.cache'))
    fixture=Path(tempfile.mkdtemp(prefix='.swarm-rust-reply-',dir=Path.home()));fixture.chmod(0o700)
    env=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(scratch/'target')),TMPDIR=str(fixture))
    commands=[]
    def save(name,value):
        """Atomically persist one JSON evidence record, surfacing write failures."""
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def run(label,args,data=None,expected=0,extra_env=None):
        """Execute exact arguments without a global watchdog, retaining original input/output and exit status."""
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
        args=[os.fspath(a)if isinstance(a,os.PathLike)else a for a in args]
        p=capture(args,cwd=ROOT,env={**env,**(extra_env or {})},data=data,
                  stdout_path=output/(label+'.stdout'),stderr_path=output/(label+'.stderr'))
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        commands.append(dict(label=label,command=[dict(bytes_hex=a.hex())if isinstance(a,bytes)else a for a in args],start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected,environment_overrides=extra_env or {}));save('commands.json',commands)
        print(label,'exit',p.returncode,flush=True)
        if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p.stdout
    def inventory(label,args):
        """Require every listed named test to execute and pass, excluding zero-test success."""
        text=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
        actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in text.splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
        expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'))
        assert actual and actual==expected,(label,actual-expected,expected-actual)
        save(label+'-names.json',list(actual.elements()));return sum(actual.values())
    cargo=['--manifest-path','rust/Cargo.toml','--locked','--offline']
    try:
        raw=run('reference-source',['go','run','./tools/rust-migration/git-report-reference']);source=scratch/'main.go';source.write_bytes(raw);package='./'+str(scratch.relative_to(ROOT))
        extras=[source,*[ROOT/p for p in ['frontend/engine-settings/admin.js','frontend/engine-settings/admin.css','tests/rust_engine_settings_admin_ui.cjs','tests/helpers/browser_verification_runtime.cjs','locales/en.json','web/i18n-en.js','package-lock.json']]]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./internal/engine','./tools/rust-migration/git-report-reference',package],allow_root=True,extra_files=extras)
        run('git-version',['git','--version'])
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
        run('compile',['cargo','check',*cargo,'--workspace','--all-features','--all-targets'])
        run('clippy',['cargo','clippy',*cargo,'--workspace','--all-features','--all-targets','--','-D','warnings'])
        reference=scratch/'go-reference';run('reference-build',['go','build','-o',reference,source]);run('reference-vet',['go','vet','./tools/rust-migration/git-report-reference',package])
        assert raw==run('reference-after',['go','run','./tools/rust-migration/git-report-reference'])
        counts=dict(workspace=inventory('workspace-tests',['cargo','test',*cargo,'--workspace','--all-features','--all-targets']),default_engine=inventory('default-engine-tests',['cargo','test',*cargo,'-p','swarm-engine','--all-targets']))
        run('rustdoc',['cargo','doc',*cargo,'-p','swarm-engine','--all-features','--no-deps'])
        doc=run('doctest',['cargo','test',*cargo,'-p','swarm-engine','--doc']).decode();assert '1 passed; 0 failed' in doc
        run('cli-build',['cargo','build',*cargo,'-p','swarm-cli'])
        run('probe-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--examples'])
        target=Path(env['CARGO_TARGET_DIR'])/'debug';native=target/'swarm-rust';probe=target/'examples/git_report_probe'
        project=fixture/'project';project.mkdir(mode=0o700);(project/'.rust-migration-fixture').write_text('Owned Git parity fixture\n')
        run('native-init',[native,'init','--root',project,'--json']);repo=project/'repository.git';run('git-init',['git','init','--bare',repo])
        cases=[]
        vectors=[b'',b'report\n',b' \t\r\n report interior \r\n ',b'\x85 raw \xa0',b'\xc2\x85\xe3\x80\x80value\xffinterior\xf0 \xc2\xa0',b'\0\x1c not whitespace \0',b' \xff \n',b'x'*262144+b' \n', '\u200b\ufeff kept \u001c'.encode(), '\u0085\u1680\u2000\u200a\u2028\u2029\u202f\u205f\u3000'.encode()]
        for i,blob in enumerate(vectors):
            oid=run(f'blob-{i}',['git','--git-dir='+str(repo),'hash-object','-w','--stdin'],blob).decode().strip()
            tree=run(f'tree-{i}',['git','--git-dir='+str(repo),'mktree'],f'100644 blob {oid}\treport.md\n'.encode()).decode().strip()
            candidate=run(f'commit-{i}',['git','--git-dir='+str(repo),'-c','user.name=Fixture','-c','user.email=fixture@localhost','commit-tree',tree],b'Fixture\n').decode().strip()
            for version in [0,1,2]:
                frame=dict(repository='repository.git',candidate=candidate,report='report.md',version=version)
                expected=json.loads(run(f'case-{i}-{version}-go',[reference,repo,candidate,'report.md',str(version)]))
                observed=json.loads(run(f'case-{i}-{version}-native',[probe,project],json.dumps(frame).encode()))
                assert observed['sha256']==expected['sha256'];assert observed['settings_revision']=='1';assert observed['scope']=='git-blob-only'
                cases.append(dict(name=f'case-{i}-{version}',candidate=candidate,version=version,input_sha256=hashlib.sha256(blob).hexdigest(),report_sha256=observed['sha256']))
        for name,change,reason in [('absent-file',dict(report='absent.md'),'git-command-failed'),('absent-object',dict(candidate='f'*40),'git-command-failed'),('unknown-version',dict(version=3),'git-citation-version-unsupported'),('option-candidate',dict(candidate='--help'),'git-candidate-object-id-required'),('outside-path',dict(report='../report.md'),'git-report-relative-path-required')]:
            control=dict(repository='repository.git',candidate=candidate,report='report.md',version=2)
            assert json.loads(run(name+'-control',[probe,project],json.dumps(control).encode()))['sha256']==hashlib.sha256(blob).hexdigest()
            fault={**control,**change};error=json.loads(run(name+'-native',[probe,project],json.dumps(fault).encode(),2));assert error['reason']=='git-report: '+reason
            if name.startswith('absent-'):run(name+'-go',[reference,repo,fault['candidate'],fault['report'],'2'],expected=2)
            assert json.loads(run(name+'-recovery',[probe,project],json.dumps(control).encode()))['sha256']==hashlib.sha256(blob).hexdigest()
        save('git-vectors.json',cases)
        # Execute the actual private Go file guard from the real engine test package.
        go_guard=scratch/'go-baseline-tests';run('go-baseline-build',['go','test','-c','-o',go_guard,'./internal/engine'])
        baseline_probe=target/'examples/baseline_files_probe';baseline_cases=[]
        def review(name,candidate,previous='',children=(),version=2,extra=None):
            """Create real immutable proof files from literal Go-contract JSON for one comparison."""
            context=dict(candidate_commit=candidate,previous_candidate=previous,receipt=None,tasks=[])
            if children:context['accepted_baseline']=dict(version=version,candidate_commit=previous,accepted_reviews=list(children))
            if extra:context.update(extra)
            raw=json.dumps(context).encode();receipt=b'Receipt';report=b'Report'
            for suffix,data in [('context',raw),('receipt',receipt),('report',report)]:
                path=project/(name+'.'+suffix);assert not path.exists();path.write_bytes(data);path.chmod(0o600)
            return dict(id=name,state='passed',candidate_commit=candidate,context=name+'.context',context_sha256=hashlib.sha256(raw).hexdigest(),receipt=name+'.receipt',receipt_sha256=hashlib.sha256(receipt).hexdigest(),report=name+'.report',sha256=hashlib.sha256(report).hexdigest())
        def baseline_case(name,current,valid):
            """Compare actual guard outcomes and named Go execution; a transport PASS is not a valid proof."""
            frame=json.dumps(dict(tasks=[dict(independent_review=current)])).encode();input_path=project/'baseline-probe.json';input_path.write_bytes(frame)
            original={str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest()for p in project.iterdir()if p.is_file()}
            stdout=run(name+'-go-baseline',[go_guard,'-test.run','^TestRustMigrationReviewBaselineProbe$','-test.v','-test.timeout=0'],extra_env={'SWARM_RUST_BASELINE_FIXTURE':str(project)}).decode()
            assert '--- PASS: TestRustMigrationReviewBaselineProbe' in stdout and 'SKIP'not in stdout
            payload=[line.removeprefix('RUST_BASELINE_RESULT ')for line in stdout.splitlines()if line.startswith('RUST_BASELINE_RESULT ')];assert len(payload)==1
            go=json.loads(payload[0]);native_value=json.loads(run(name+'-native-baseline',[baseline_probe,project],frame,0 if valid else 2));assert go['valid']==valid
            if valid:assert native_value['files_consistent'] is True
            else:assert native_value['code']=='review-evidence-invalid'
            assert original=={str(p.relative_to(project)):hashlib.sha256(p.read_bytes()).hexdigest()for p in project.iterdir()if p.is_file()}
            baseline_cases.append(dict(name=name,valid=valid,go=go,native=native_value))
        leaf=review('baseline-leaf','old');baseline_case('baseline-leaf',leaf,True)
        for version in [0,1,2]:baseline_case('baseline-version-'+str(version),review('baseline-version-'+str(version),'current','old',[leaf],version),True)
        shared=review('baseline-shared','current','old',[leaf,leaf]);baseline_case('baseline-shared',shared,True)
        for i,(key,value) in enumerate([('receipt_sha256',''),('receipt_sha256','wrong'),('sha256',''),('sha256','wrong'),('state','failed'),('candidate_commit','other')]):
            baseline_case(f'baseline-fault-{i}-control',shared,True);bad={**leaf,key:value};root_review=review(f'baseline-fault-{i}','current','old',[leaf,bad]);baseline_case(f'baseline-fault-{i}',root_review,False);baseline_case(f'baseline-fault-{i}-recovery',shared,True)
        baseline_case('baseline-version-invalid-control',shared,True);baseline_case('baseline-version-invalid',review('baseline-invalid-version','current','old',[leaf],3),False)
        opaque=review('baseline-unknown-context','old',extra={'unknown':dict(raw='data')});baseline_case('baseline-compatible-unknown',review('baseline-compatible-root','current','old',[opaque]),True)
        node=review('baseline-chain-0','candidate-0')
        for i in range(1,130):
            node=review(f'baseline-chain-{i}',f'candidate-{i}',node['candidate_commit'],[node])
            if i==128:baseline_case('baseline-depth128',node,True)
        baseline_case('baseline-depth129-control',shared,True);baseline_case('baseline-depth129',node,False);baseline_case('baseline-depth129-recovery',shared,True)
        save('baseline-vectors.json',baseline_cases)
        from verify_review_quote import vectors as quote_vectors
        reply_probe=target/'examples/review_reply_probe'
        reply_cases=[]
        limits=dict(min_text_bytes=8,max_text_bytes=4000,min_normalized_quote_bytes=8)
        def reply_case(name,raw,count,originals,valid):
            """Compare the actual private Go parser, including named execution and business refusal."""
            frame=dict(raw=raw,criterion_count=count,originals=originals)
            (project/'reply-probe.json').write_text(json.dumps(frame))
            stdout=run(name+'-go-reply',[go_guard,'-test.run','^TestRustMigrationReviewReplyProbe$','-test.v','-test.timeout=0'],extra_env={'SWARM_RUST_REPLY_FIXTURE':str(project)}).decode()
            assert '--- PASS: TestRustMigrationReviewReplyProbe' in stdout and 'SKIP' not in stdout
            payload=[line.removeprefix('RUST_REPLY_RESULT ') for line in stdout.splitlines() if line.startswith('RUST_REPLY_RESULT ')]
            assert len(payload)==1
            expected=json.loads(payload[0]);assert expected['valid']==valid,(name,expected)
            native_result=json.loads(run(name+'-native-reply',[reply_probe,'pure'],json.dumps(dict(frame,limits=limits)).encode(),0 if valid else 2))
            if not valid:native_result.pop('reason')
            assert native_result==expected,(name,native_result,expected)
            reply_cases.append(dict(name=name,frame=frame,valid=valid))
        def ordinary(evidence='Valid literal evidence',verdict='pass',reason='Valid rationale'):
            """Build one complete supplied reply; this is fixture text, not an independent review."""
            return json.dumps(dict(reason=reason,criteria=[dict(index=1,verdict=verdict,evidence=evidence)]))
        plain=[dict(name='proof.go',kind='source',content='Valid literal evidence')]
        for case in quote_vectors():
            f=case['frame'];originals=[dict(name=f['name'],kind=f['kind'],content=f['content']+'\n\nValid literal evidence')]
            reply_case('quote-'+case['name']+'-control',ordinary(),1,originals,True)
            valid=case['present'] and 8<=len(f['evidence'].encode())<=4000
            reply_case('quote-'+case['name'],ordinary(f['evidence']),1,originals,valid)
            reply_case('quote-'+case['name']+'-recovery',ordinary(),1,originals,True)
        extras=[('empty','{}',1,False),('null','null',1,False),('array','[]',1,False),('trailing',ordinary()+'{}',1,False),
            ('fail',ordinary('Missing evidence','fail'),1,True),('unknown',ordinary('Missing evidence','unknown'),1,True),
            ('verdict-case',ordinary('Valid literal evidence','PASS'),1,False),
            ('nil-zero',json.dumps(dict(reason='Valid rationale',criteria=None)),0,True),
            ('empty-zero',json.dumps(dict(reason='Valid rationale',criteria=[])),0,True),
            ('reason-null',ordinary()[:-1]+',"REASON":null}',1,True),
            ('reason-case',ordinary().replace('"reason"','"REASON"'),1,True),
            ('unknown-field',ordinary()[:-1]+',"private_secret":null}',1,False),
            ('criterion-unknown',ordinary().replace('"index": 1','"index": 1, "private_secret": 0'),1,False),
            ('zero-index',ordinary().replace('"index": 1','"index": 0'),1,False),
            ('float-index',ordinary().replace('"index": 1','"index": 1.0'),1,False),
            ('exponent-index',ordinary().replace('"index": 1','"index": 1e0'),1,False),
            ('overflow-index',ordinary().replace('"index": 1','"index": 9223372036854775808'),1,False),
            ('duplicate-index',json.dumps(dict(reason='Valid rationale',criteria=[dict(index=1,verdict='pass',evidence='Valid literal evidence')]*2)),2,False),
            ('reverse-order',json.dumps(dict(reason='Valid rationale',criteria=[dict(index=i,verdict='pass',evidence='Valid literal evidence')for i in (2,1)])),2,True),
            ('slice-reuse','{"reason":"Valid rationale","criteria":[{"index":1,"verdict":"pass","evidence":"Valid literal evidence"}],"CRITERIA":[{"verdict":null}]}',1,True)]
        for field in ('reason','evidence'):
            for size in (7,8,4000,4001):
                text='x'*size;originals=[dict(name='proof.go',kind='source',content=text if field=='evidence' else 'Valid literal evidence')]
                raw=ordinary(**{field:text});name=field+'-'+str(size)
                reply_case(name+'-control',ordinary(verdict='unknown'),1,plain,True)
                reply_case(name,raw,1,originals,8<=size<=4000)
                reply_case(name+'-recovery',ordinary(),1,plain,True)
        for name,raw,count,valid in extras:
            reply_case(name+'-control',ordinary(),1,plain,True)
            reply_case(name,raw,count,plain,valid)
            reply_case(name+'-recovery',ordinary(),1,plain,True)
        save('reply-vectors.json',reply_cases)
        archived=output/'go-baseline-tests.binary';shutil.copyfile(go_guard,archived);archived.chmod(0o600)

        run('admin',['node','tests/rust_engine_settings_admin_ui.cjs',native,target/'examples/root_store_probe',output/'admin',target/'examples/project_context_probe',probe,target/'examples/baseline_files_probe',target/'examples/review_reply_probe'])
        admin=json.loads((output/'admin/result.json').read_bytes());assert admin['status']=='PASS' and len(admin['checks'])==4
        assert all(c['fields']==len(json.loads((ROOT/'rust/crates/swarm-engine/config/engine-settings-catalog.json').read_text())['fields']) and c['saved_admin_git_deadline_changes_real_process'] and c['git_zero_null_saved_reopened']and c['saved_admin_baseline_depth_changes_real_guard'] and c['baseline_zero_null_saved_reopened'] and c['saved_admin_reply_policies_change_stored_task_parser'] and c['reply_three_fields_saved_reopened'] for c in admin['checks'])
        run('methods',['python3','tools/agent-workflows/check.py']);run('i18n',['npm','run','test:i18n']);run('diff-check',['git','diff','--check'])
        for name,binary in [('go-reference',reference),('native-cli',native),('native-git-probe',probe),('native-reply-probe',reply_probe)]:target_path=output/(name+'.binary');shutil.copyfile(binary,target_path);target_path.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3au45 native structured reply parsing and v7 Admin policies; no managed acceptance',tests=counts,reply_comparisons=len(reply_cases),reply_controls_and_recovery_each=True,git_comparisons=len(cases),baseline_comparisons=len(baseline_cases),native_refusal_recovery_cases=5,admin_journeys=4,admin_fields=len(json.loads((ROOT/'rust/crates/swarm-engine/config/engine-settings-catalog.json').read_text())['fields']),inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,fixture_root=str(fixture),provider_calls=False,independent_review=False,deployment=False))
        print('PASS native reply component and Admin; PARTIAL full migration',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,fixture_root=str(fixture)));raise

if __name__=='__main__':main()
