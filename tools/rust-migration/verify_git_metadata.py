"""Qualify shared filter-safe Git metadata and authoritative Root deadlines.

Real Git, native consumers and isolated process shims; no provider or deployment.
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import sys
import tempfile
import time
from collections import Counter
from qualification_inputs import collect
from recorded_process import capture

ROOT = Path(__file__).resolve().parents[2]


def main():
    """Freeze effective inputs; require actual named tests, exact facts and recovery."""
    if not __debug__ or len(sys.argv) != 2:
        raise RuntimeError('active assertions and one fresh evidence directory required')
    output = Path(sys.argv[1]).resolve()
    output.mkdir(parents=True, exist_ok=False)
    work = Path(tempfile.mkdtemp(prefix='git-metadata-', dir=ROOT / '.cache'))
    env = dict(os.environ)
    commands = []

    def save(name, value):
        """Publish completed records atomically, preserving an existing record on failure."""
        path = output / name
        fd, temporary = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
        with os.fdopen(fd, 'w') as stream:
            json.dump(value, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)

    def run(label, args, data=None, expected=0, extra_env=None):
        """Stream original outputs with no global watchdog; reject an unexpected exit."""
        args = list(map(str, args))
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        start = time.monotonic()
        result = capture(args, cwd=ROOT, env={**env,**(extra_env or {})}, data=data,
                         stdout_path=output / (label + '.stdout'),
                         stderr_path=output / (label + '.stderr'))
        if data is not None:
            (output / (label + '.stdin')).write_bytes(data)
        commands.append(dict(label=label, command=args, start_utc=started,
                             seconds=time.monotonic()-start, exit=result.returncode,
                             expected_exit=expected, environment_overrides=extra_env or {}))
        save('commands.json', commands)
        print(label, 'exit', result.returncode, flush=True)
        if result.returncode != expected:
            raise RuntimeError(label + ': original failure retained')
        return result.stdout

    try:
        before, snapshot, registry, close_graphs = collect(
            ROOT, output, env, run, ['./internal/engine'], allow_root=True, extra_files=[ROOT/p for p in ['frontend/engine-settings/admin.js','frontend/engine-settings/admin.css','tests/rust_engine_settings_admin_ui.cjs','tests/helpers/browser_verification_runtime.cjs','locales/en.json','web/i18n-en.js','package-lock.json']])
        cargo = ['--manifest-path', 'rust/Cargo.toml', '--locked', '--offline']
        run('revision', ['git', 'rev-parse', 'HEAD'])
        run('format', ['cargo', 'fmt', '--manifest-path', 'rust/Cargo.toml', '--all', '--', '--check'])
        run('compile', ['cargo', 'check', *cargo, '--workspace', '--all-features', '--all-targets'])
        run('clippy', ['cargo', 'clippy', *cargo, '--workspace', '--all-features', '--all-targets', '--', '-D', 'warnings'])
        tests=['cargo','test',*cargo,'--workspace','--all-features','--all-targets']
        actual_raw=run('workspace-tests',tests).decode();inventory=run('workspace-inventory',tests+['--','--list']).decode()
        actual=Counter(line[5:-7]for line in actual_raw.splitlines()if line.startswith('test ')and line.endswith(' ... ok'))
        listed=Counter(line[:-6]for line in inventory.splitlines()if line.endswith(': test'))
        assert actual and actual==listed,(actual-listed,listed-actual);save('native-test-names.json',list(actual.elements()))
        run('cli-build',['cargo','build',*cargo,'-p','swarm-cli'])
        run('probe-build',['cargo','build',*cargo,'-p','swarm-engine','--features','test-fixtures','--examples'])
        target=Path(env['CARGO_TARGET_DIR'])/'debug';native=target/'swarm-rust';probe=target/'examples/git_metadata_probe'
        git=Path(shutil.which('git')).resolve();python=Path(sys.executable).resolve()
        programs={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in [git,python]};save('external-programs-before.json',programs)
        scratch=Path(tempfile.mkdtemp(prefix='.swarm-git-qualified-',dir=Path.home()));scratch.chmod(0o700)
        counter=scratch/'git-invocations.jsonl';shimdir=scratch/'bin';shimdir.mkdir(mode=0o700)
        shim=shimdir/'git'
        shim.write_text('#!'+str(python)+'\nimport os,sys,json\nwith open('+repr(str(counter))+',"a") as f: f.write(json.dumps(sys.argv[1:])+"\\n")\nos.execv('+repr(str(git))+',["git",*sys.argv[1:]])\n');shim.chmod(0o700)
        overrides={'PATH':str(shimdir)+os.pathsep+env['PATH']}
        project=scratch/'project';project.mkdir(mode=0o700);(project/'.rust-migration-fixture').touch()
        run('init',[native,'init','--root',project,'--json'])
        run('git-init',[git,'-C',project,'init']);(project/'probe.txt').write_bytes(b'original\n');(project/'.gitattributes').write_text('*.txt filter=fixture\n')
        run('git-add',[git,'-C',project,'add','probe.txt','.gitattributes']);run('git-commit',[git,'-C',project,'-c','user.name=Fixture','-c','user.email=fixture@localhost','commit','-m','Fixture'])
        baseline=json.loads(run('metadata-control',[probe,'read',project],extra_env=overrides));assert baseline['issues']==0
        reference={}
        for name,args in [('head',['rev-parse','HEAD']),('branch',['branch','--show-current']),('changes',['status','--porcelain','--untracked-files=normal'])]:reference[name]=run('reference-'+name,[git,'-C',project,*args]).decode().strip()
        assert baseline['state']==reference
        marker=project/'.swarm/filter-invoked';script=project/'.swarm/clean.script';script.write_text('import sys\nfrom pathlib import Path\nPath(__file__).with_name("filter-invoked").write_text("callback executed")\nsys.stdout.buffer.write(sys.stdin.buffer.read())\n')
        callback=str(python)+' '+str(script)
        run('filter-config',[git,'-C',project,'config','filter.fixture.clean',callback]);run('filter-required',[git,'-C',project,'config','filter.fixture.required','true']);(project/'probe.txt').write_bytes(b'modified\n')
        index=(project/'.git/index').read_bytes();protected=json.loads(run('metadata-protected',[probe,'read',project],extra_env=overrides));assert protected['issues']==0 and protected['external_filters_disabled']and not marker.exists()
        legacy=json.loads(run('legacy-protected',[probe,'legacy',project],extra_env=overrides));assert legacy['state']==protected['state']and not marker.exists();assert(project/'.git/index').read_bytes()==index
        request=dict(schema_version=1,event_id='create-limited',expected_revision=0,title='Work',objective='Data',scope='Fixture',criteria=['Proof'])
        def cli(label,op,root,request=None):
            """Use the actual native CLI; retain complete request bytes and real CAS token."""
            args=[native,*op,'--root',root,'--json']
            if request is not None:
                path=scratch/(label+'.json');path.write_text(json.dumps(request));args += ['--input',path]
            return json.loads(run(label,args))
        def policy(label,root,cap):
            """Save a changed operator deadline without touching consumption/history directly."""
            d=cli(label+'-before',['engine-settings','show'],root);values=d['values'];values['git_command_timeout_seconds']=cap
            r=dict(schema_version=d['schema_version'],event_id=label,expected_revision=d['revision'],values=values,reason='Actual metadata policy',preview_token='')
            r['preview_token']=cli(label+'-preview',['engine-settings','preview'],root,r)['preview_token'];cli(label+'-apply',['engine-settings','apply'],root,r)
        policy('immediate',project,'0');before_db=(project/'.swarm/state.db').read_bytes();before_settings=(project/'.swarm/engine-settings.json').read_bytes()
        rejected=json.loads(run('metadata-deadline-refusal',[probe,'read',project],expected=2,extra_env=overrides));assert rejected['reason']=='git-command-timeout'
        rejected=json.loads(run('creation-deadline-refusal',[probe,'create',project],json.dumps(request).encode(),2,overrides));assert rejected['reason']=='git-command-timeout'
        assert cli('works-after-refusal',['work','list'],project)==[]
        assert(project/'.swarm/state.db').read_bytes()==before_db and(project/'.swarm/engine-settings.json').read_bytes()==before_settings and not marker.exists()
        policy('unlimited',project,None)
        restored=json.loads(run('metadata-recovered',[probe,'read',project],extra_env=overrides));assert restored['issues']==0 and not marker.exists()
        created=json.loads(run('creation-recovered',[probe,'create',project],json.dumps(request).encode(),extra_env=overrides));assert created['work']['git']==restored['state']and not marker.exists()
        assert len(cli('works-recovered',['work','list'],project))==1
        old=scratch/'legacy';old.mkdir(mode=0o700);(old/'.rust-migration-fixture').touch();cli('legacy-init',['init'],old)
        (old/'.swarm/engine-settings.json').write_bytes((ROOT/'rust/crates/swarm-engine/tests/fixtures/legacy-v1-document.json').read_bytes())
        original_counter=counter.read_bytes()
        missing=json.loads(run('legacy-policy-read-refusal',[probe,'read',old],expected=2,extra_env=overrides));assert missing['reason']=='settings-git-policy-not-configured'
        optional=json.loads(run('legacy-policy-create',[probe,'create',old],json.dumps(dict(request,event_id='legacy-create')).encode(),extra_env=overrides));assert optional['issues']==3
        assert counter.read_bytes()==original_counter,'Git executed without adopted policy'
        doc=cli('legacy-document',['engine-settings','show'],old);catalog=cli('catalog',['engine-settings','describe'],old)
        adoption=dict(schema_version=catalog['document_schema_version'],event_id='adopt',expected_revision=doc['revision'],values=catalog['defaults'],reason='Explicit policy adoption',preview_token='')
        adoption['preview_token']=cli('adopt-preview',['engine-settings','preview'],old,adoption)['preview_token'];cli('adopt-apply',['engine-settings','apply'],old,adoption)
        adopted=cli('adopted-document',['engine-settings','show'],old);assert adopted['history'][:len(doc['history'])]==doc['history']
        after=json.loads(run('legacy-policy-adopted',[probe,'read',old],extra_env=overrides));assert after['issues']==3 and counter.read_bytes()!=original_counter
        save('metadata-assertions.json',dict(nominal_matches_real_git=True,configured_filter_not_executed=True,legacy_and_root_share_guard=True,index_unchanged=True,saved_deadline_refuses_read_and_creation=True,refusal_preserves_db_settings=True,recovery_after_saved_unlimited=True,unconfigured_policy_zero_git_launches=True,explicit_adoption_keeps_history=True,filter_script_sha256=hashlib.sha256(script.read_bytes()).hexdigest(),shim_sha256=hashlib.sha256(shim.read_bytes()).hexdigest(),fixture_root=str(scratch)))
        run('rustdoc',['cargo','doc',*cargo,'-p','swarm-engine','--no-deps']);run('doctest',['cargo','test',*cargo,'-p','swarm-engine','--doc'])
        run('i18n-build-check',['node','scripts/i18n/build.cjs','--check']);run('i18n-tests',['npm','run','test:i18n'])
        run('admin',['node','tests/rust_engine_settings_admin_ui.cjs',native,target/'examples/root_store_probe',output/'admin',target/'examples/project_context_probe',target/'examples/git_report_probe',target/'examples/baseline_files_probe',target/'examples/review_reply_probe',target/'examples/work_projection_probe'])
        admin=json.loads((output/'admin/result.json').read_text());assert admin['status']=='PASS'and len(admin['checks'])==4
        run('methods',['python3','tools/agent-workflows/check.py']);run('diff-check',['git','diff','--check'])
        binaries={}
        for name,source in [('native-cli',native),('metadata-probe',probe)]:
            destination=output/(name+'.binary');shutil.copyfile(source,destination);destination.chmod(0o600);binaries[name]=hashlib.sha256(destination.read_bytes()).hexdigest()
        save('binaries.json',binaries);assert programs=={p:hashlib.sha256(Path(p).read_bytes()).hexdigest()for p in programs};save('external-programs-after.json',programs)
        close_graphs();after=snapshot();assert before==after,'effective inputs changed during qualification';save('inputs-after.json',after)
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',scope='R5d50 shared Linux Git metadata and Root policy, not public Work rendering',native_tests=sum(actual.values()),admin_journeys=4,filter_execution_prevented=True,root_read_creation_and_legacy_guard_verified=True,unadopted_policy_no_git_launch=True,saved_deadline_and_recovery=True,source_stable=True,inputs=len(before),registry_packages=len(registry),global_watchdog=None,provider_calls=False,independent_review=False,deployment=False))
        print('PASS shared Git metadata; PARTIAL migration',flush=True)
    except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise


if __name__=='__main__':main()
