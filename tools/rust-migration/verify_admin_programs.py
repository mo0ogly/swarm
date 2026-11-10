"""Qualify Admin program-line correction and verification runtime, reusing identical backend."""
import datetime,hashlib,json,os,subprocess,sys,tarfile,tempfile,time
from pathlib import Path,PurePosixPath
from qualification_inputs import collect
from reuse_ui_backend import reuse
ROOT=Path(__file__).resolve().parents[2]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def verify_runtime_archive(output):
    r=json.loads((output/'runtime-inputs.json').read_text());seen={}
    archive=output/'npm-sources.tar.gz';assert sha(archive)==r['npm_archive_sha256']
    with tarfile.open(archive)as tar:
        for m in tar.getmembers():
            name=PurePosixPath(m.name);assert not name.is_absolute()and '..'not in name.parts
            if m.isdir():continue
            key=str(name);expected=r['dependencies'][key]
            if m.issym():assert dict(symlink=m.linkname)==expected;seen[key]=expected
            else:
                assert m.isfile()or m.islnk()
                if m.islnk():target=PurePosixPath(m.linkname);assert not target.is_absolute()and '..'not in target.parts
                digest=hashlib.sha256(tar.extractfile(m).read()).hexdigest();assert dict(sha256=digest)==expected;seen[key]=expected
    assert seen==r['dependencies'];return dict(status='PASS',files=len(seen),archive_sha256=r['npm_archive_sha256'],node_version=r['node_version'],browser=r['browser'],timeouts=r['timeouts'])
def main():
    output=Path(sys.argv[1]).resolve();assert not output.exists(),'fresh evidence required';output.mkdir(parents=True)
    baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-15/qualified-01/backend-qualified'
    scratch=Path(tempfile.mkdtemp(prefix='.swarm-rust-admin-check-',dir=Path.home().resolve()))
    env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch));commands=[]
    def save(name,value):
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def run(label,args):
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(args,cwd=ROOT,env=env,capture_output=True)
        for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
        commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=0));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        assert p.returncode==0,label+': original failure retained';return p.stdout
    try:
        frontend=[ROOT/'package.json',ROOT/'package-lock.json',ROOT/'tests/rust_engine_settings_admin_ui.cjs',ROOT/'tests/browser_verification_runtime_test.cjs',*sorted((ROOT/'frontend/engine-settings').rglob('*')),*sorted((ROOT/'tests/helpers').glob('browser_verification_runtime.cjs'))]
        frontend=[p for p in frontend if p.is_file()]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./cmd/swarm','./internal/engine'],allow_root=True,extra_files=frontend)
        retained=reuse(baseline,output,before);save('reused-backend.json',retained)
        target=Path(json.loads((output/'cargo-metadata.stdout').read_text())['target_directory']);cli=target/'debug/swarm-rust';probe=target/'debug/examples/root_store_probe'
        binary={str(p):sha(p)for p in [cli,probe]};assert sha(cli)==sha(baseline/'native-cli.binary')and sha(probe)==sha(baseline/'native-root-probe.binary')
        run('runtime-guards',['node','tests/browser_verification_runtime_test.cjs']);run('generated-i18n',['node','scripts/i18n/build.cjs','--check'])
        args=['node','tests/rust_engine_settings_admin_ui.cjs',str(cli),str(probe),str(output/'browser')]
        print('Browser source/correctness checks running',flush=True)
        # Stream progress while retaining the original bytes; no global watchdog.
        with (output/'browser.stdout').open('wb')as stdout,(output/'browser.stderr').open('wb')as stderr:
            process=subprocess.Popen(args,cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=stderr)
            for line in process.stdout:stdout.write(line);stdout.flush();print(line.decode(errors='replace'),end='',flush=True)
            code=process.wait()
        commands.append(dict(label='browser',command=args,exit=code,expected_exit=0));save('commands.json',commands);assert code==0,'browser: original failure retained'
        ui=json.loads((output/'browser/result.json').read_text());assert ui['status']=='PASS'and len(ui['checks'])==4 and all(c['program_lines_not_split_on_spaces']for c in ui['checks'])
        runtime=verify_runtime_archive(output/'browser');save('runtime-archive-check.json',runtime)
        assert binary=={str(p):sha(p)for p in [cli,probe]};close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        run('diff-check',['git','diff','--check'])
        save('result.json',dict(status='PARTIAL',native_component_status='PASS',overall_migration='PARTIAL',scope='R3s shared Admin program-line correction and verified browser runtime; no native HTTP acceptance',reused_backend=retained,ui_checks=ui['checks'],runtime=runtime,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,admin_installed=False,http_server=False,independent_review=False,deployment=False))
        print('PASS Admin program lines/runtime; PARTIAL full migration',flush=True)
    except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__,scratch=str(scratch)));raise
if __name__=='__main__':main()
