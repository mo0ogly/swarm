"""Qualify the backend, then shared Admin on exactly those binaries/sources.
CDP remains fixture transport, never HTTP authentication or installed Admin proof.
"""
import hashlib,json,os,subprocess,sys,tempfile,tarfile,io
from pathlib import Path
from qualify_command_input import qualify as qualify_input
from verify_admin_programs import verify_runtime_archive
ROOT=Path(__file__).resolve().parents[2]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists():raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True)
    frontend=[ROOT/'package.json',ROOT/'package-lock.json',ROOT/'tests/rust_engine_settings_admin_ui.cjs',ROOT/'tests/helpers/browser_verification_runtime.cjs',ROOT/'tests/browser_verification_runtime_test.cjs',*sorted((ROOT/'frontend/engine-settings').rglob('*'))]
    before={str(p.relative_to(ROOT)):sha(p)for p in frontend if p.is_file()}
    # Preserve actual candidate contents before execution, not only hashes.
    with tarfile.open(output/'frontend-sources.tar.gz','w:gz',compresslevel=1)as archive:
        for name,digest in before.items():
            data=(ROOT/name).read_bytes();assert hashlib.sha256(data).hexdigest()==digest
            member=tarfile.TarInfo(name);member.size=len(data);member.mode=0o644;archive.addfile(member,io.BytesIO(data))

    commands=[]
    def save(name,value):
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def run(label,args):
        with (output/(label+'.stdout')).open('wb')as stdout,(output/(label+'.stderr')).open('wb')as stderr:
            p=subprocess.Popen(args,cwd=ROOT,stdout=subprocess.PIPE,stderr=stderr)
            for line in p.stdout:stdout.write(line);stdout.flush();print(line.decode(errors='replace'),end='',flush=True)
            code=p.wait()
        commands.append(dict(label=label,args=args,exit=code));save('commands.json',commands)
        if code:raise RuntimeError(label+': original failure retained')
    try:
        run('backend',['python3','tools/rust-migration/verify_root_store.py',str(output/'backend-qualified')])
        backend=json.loads((output/'backend-qualified/result.json').read_text());assert backend['native_component_status']=='PASS'and backend['source_stable']and backend['project_context']['freshness_guard']and backend['root_row_review_observation']
        target=Path(json.loads((output/'backend-qualified/cargo-metadata.stdout').read_text())['target_directory']);cli=target/'debug/swarm-rust';probe=target/'debug/examples/root_store_probe'
        for binary,name in [(cli,'native-cli.binary'),(probe,'native-root-probe.binary'),(target/'debug/examples/project_context_probe','native-project-context-probe.binary')]:assert sha(binary)==sha(output/'backend-qualified'/name)
        framing=qualify_input(ROOT,output/'command-input',cli,Path(backend['fixture_root'])/'go-cli')
        run('i18n-generated',['node','scripts/i18n/build.cjs','--check'])
        run('i18n-tests',['node','tests/i18n_test.cjs'])
        run('runtime-guards',['node','tests/browser_verification_runtime_test.cjs'])
        run('browser',['node','tests/rust_engine_settings_admin_ui.cjs',str(cli),str(probe),str(output/'browser'),str(target/'debug/examples/project_context_probe')])
        runtime=verify_runtime_archive(output/'browser');save('runtime-archive-check.json',runtime)
        ui=json.loads((output/'browser/result.json').read_text());assert ui['status']=='PASS'and len(ui['checks'])==4 and not ui['errors']
        assert all(c['saved_admin_quality_changes_real_calculation']and c['legacy_quality_requires_explicit_adoption']and c['saved_admin_context_changes_real_reader']and c['context_cap_null_and_legacy_adoption']and c['saved_admin_review_cap_changes_real_reader']for c in ui['checks'])
        now=[ROOT/'package.json',ROOT/'package-lock.json',ROOT/'tests/rust_engine_settings_admin_ui.cjs',ROOT/'tests/helpers/browser_verification_runtime.cjs',ROOT/'tests/browser_verification_runtime_test.cjs',*sorted((ROOT/'frontend/engine-settings').rglob('*'))]
        assert before=={str(p.relative_to(ROOT)):sha(p)for p in now if p.is_file()},'Admin sources or membership changed'
        inputs=json.loads((output/'backend-qualified/inputs-after.json').read_text());local={k:v for k,v in inputs.items()if(ROOT/k).is_file()}
        assert local and all(sha(ROOT/k)==v for k,v in local.items()),'backend inputs changed before UI acceptance'
        run('diff-check',['git','diff','--check'])
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3aj actual Root review observations and shared Admin policy effects; authority/full public transport pending',backend_tests={k:len(v)for k,v in backend['tests'].items()},validation_controls=backend['validation_controls'],score=backend['score'],gates=backend['gates'],root_row_review_observation=backend['root_row_review_observation'],review_observation_scope=backend['review_observation_scope'],project_context=backend['project_context'],backend_inputs=backend['inputs'],registry_packages=backend['registry_packages'],ui_checks=ui['checks'],verification_runtime=runtime,command_input=framing,frontend_inputs=before,matching_qualified_binaries=True,frontend_sources_archive_sha256=sha(output/'frontend-sources.tar.gz'),source_stable=True,global_watchdog=None,http_server=False,admin_installed=False,authentication=False,independent_review=False,deployment=False))
        print('PASS native settings/shared Admin; PARTIAL server and full migration',flush=True)
    except BaseException as error:
        save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise
if __name__=='__main__':main()
