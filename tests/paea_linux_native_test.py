#!/usr/bin/env python3
"""Qualified normal ELF prerequisite CLI/API checks in one fresh isolated Root.

Require exact supported build evidence and hash, never accept a failed profile or
use the live Root. No provider/namespace/helper/account operation or test watchdog.
Only owned server is signaled/reaped; private session material never enters proof.
"""
from pathlib import Path
import hashlib,http.client,json,os,re,signal,subprocess,sys,threading
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]

def sha(path):
    """Hash actual file bytes, not a pathname or claimed version."""
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    """Validate candidate prerequisites before mutation, then retain named outcomes."""
    binary=Path(os.environ['SWARM_PAEA_PREREQ_BINARY']).resolve();expected=os.environ['SWARM_PAEA_PREREQ_SHA256']
    build=Path(os.environ['SWARM_PAEA_PREREQ_BUILD']).resolve();out=Path(os.environ['SWARM_PAEA_PREREQ_EVIDENCE']).resolve()
    assert re.fullmatch('[a-f0-9]{64}',expected) and sha(binary)==expected
    candidate=json.loads((build/'result.json').read_text());profile=json.loads((build/'profile/result.json').read_text())
    assert candidate['status']=='VERIFIED_NATIVE_CANDIDATE' and candidate['sha256']==expected
    assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['sha256']==expected
    assert all(profile.get(k) is True for k in ['no_fixture_feature','all_core_features_empty','all_core_test_false','actual_dependencies_stable'])
    rows=[json.loads(line)for line in (build/'profile/cargo.stdout').read_text().splitlines()if line.startswith('{')]
    artifact=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='swarm_linux_prerequisites']
    assert len(artifact)==1 and artifact[0]['features']==[] and artifact[0]['profile']['test'] is False
    schema=json.loads((ROOT/'schemas/paea/linux-prerequisites-v2.schema.json').read_text());Draft202012Validator.check_schema(schema);validator=Draft202012Validator(schema)
    out.mkdir(mode=0o700);cwd=out/'empty-cwd';cwd.mkdir(mode=0o700);root=out/'isolated-root';root.mkdir(mode=0o700)
    checks=[];calls=[];http_calls=[];app=None;drain=None;server_stdout=[];failure=None;errors=None
    def check(name,value):
        """Reject duplicate/false assertions; exit0 alone never constitutes PASS."""
        assert name not in checks and value,name;checks.append(name);print('PASS '+name,flush=True)
    def command(name,args,workdir=cwd,success=True):
        """Run exact normal CLI without shell/timeout, retaining both public streams."""
        result=subprocess.run([str(binary),*args],cwd=workdir,capture_output=True)
        (out/(name+'.stdout')).write_bytes(result.stdout);(out/(name+'.stderr')).write_bytes(result.stderr)
        calls.append({'name':name,'argv':[str(binary),*args],'exit':result.returncode,'stdout_sha256':hashlib.sha256(result.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(result.stderr).hexdigest()})
        assert (result.returncode==0)==success,(name,result.returncode)
        return result
    def valid_cli(name,lang):
        """Validate actual native process data, explicit non-authority and empty cwd."""
        result=command(name,['identity','linux-prerequisites','--json','--lang',lang]);value=json.loads(result.stdout);validator.validate(value)
        assert not result.stderr and value['identity']['value']['euid']==os.geteuid() and value['identity']['value']['egid']==os.getegid()
        assert sorted(value['identity']['value']['groups'])==sorted(os.getgroups())
        assert not list(cwd.iterdir());return value
    def snapshot():
        """Hash only this fixture Root, never a live database; retain relative names."""
        return {str(p.relative_to(root)):sha(p)for p in root.rglob('*')if p.is_file()}
    try:
        for lang in ['fr','en']:
            valid_cli('cli-'+lang,lang);check(lang+'-native-report-schema-identity-no-root',True)
            faults=[['--root','.'],['--input','PRIVATE_PREREQ_CANARY'],['--input',''],['--output','PRIVATE_PREREQ_CANARY'],['--phase','delivery'],['--json'],['--unknown'],['extra']]
            for index,options in enumerate(faults):
                valid_cli(f'{lang}-{index}-control',lang);r=command(f'{lang}-{index}-refusal',['identity','linux-prerequisites','--json','--lang',lang,*options],success=False)
                check(f'{lang}-{index}-refusal-no-output-or-root',not r.stdout and b'PRIVATE_PREREQ_CANARY' not in r.stderr and not list(cwd.iterdir()));valid_cli(f'{lang}-{index}-recovery',lang)
        r=command('no-json',['identity','linux-prerequisites'],success=False);check('explicit-json-required',not r.stdout and not list(cwd.iterdir()))
        command('init',['--root',str(root),'init'])
        errors=(out/'server-private.stderr').open('wb');app=subprocess.Popen([str(binary),'--root',str(root),'--json','web','127.0.0.1:0'],stdout=subprocess.PIPE,stderr=errors)
        first=app.stdout.readline()
        def drain_stdout():
            """Drain child output throughout checks/shutdown to avoid pipe deadlock."""
            for raw in app.stdout:server_stdout.append(re.sub(rb'session-[A-Za-z0-9_-]+',b'session-[private]',raw))
        drain=threading.Thread(target=drain_stdout);drain.start()
        bound=json.loads(first);assert bound['event']=='web-bound';address=bound['address'];assert re.fullmatch(r'127\.0\.0\.1:[1-9][0-9]*',address)
        check('actual-owned-server-elf',sha(f'/proc/{app.pid}/exe')==expected)
        key=root/'.swarm'/('web-session-'+hashlib.sha256(address.encode()).hexdigest()[:16]);fd=os.open(key,os.O_RDONLY|os.O_NOFOLLOW)
        with os.fdopen(fd)as stream:
            import stat
            info=os.fstat(stream.fileno());assert stat.S_ISREG(info.st_mode) and info.st_uid==os.getuid() and info.st_mode&0o077==0;token=stream.read().strip()
        assert re.fullmatch('session-[A-Za-z0-9_-]{24}',token);cookie='swarm_session_'+hashlib.sha256(address.encode()).hexdigest()[:12]+'='+token
        def request(name,path,method='GET',body=None,fault=None,private=False,csrf=None):
            """One native HTTP operation; never retain cookie/CSRF or arbitrary headers."""
            headers={'Host':address,'Cookie':cookie}
            if method!='GET':headers.update({'Origin':'http://'+address,'x-swarm-csrf':csrf or ''})
            if fault=='cookie':headers.pop('Cookie')
            if fault=='host':headers['Host']='invalid.test'
            connection=http.client.HTTPConnection(address)
            try:connection.request(method,path,body=body,headers=headers);response=connection.getresponse();raw=response.read();status=response.status;ctype=response.getheader('Content-Type');cache=response.getheader('Cache-Control')
            finally:connection.close()
            if not private:
                (out/(name+'.body')).write_bytes(raw);http_calls.append({'name':name,'method':method,'path':path,'status':status,'content_type':ctype,'cache_control':cache,'body_sha256':hashlib.sha256(raw).hexdigest()})
            return status,raw,cache
        route='/api/v1/protection/linux-prerequisites';status,raw,_=request('session','/api/v1/native-session',private=True);assert status==200;csrf=json.loads(raw)['csrf_token']
        before=snapshot();(out/'root-before.json').write_text(json.dumps({n:('[private credential hash omitted]'if 'web-session-'in n else h)for n,h in before.items()},indent=2)+'\n')
        for index,fault in enumerate(['cookie','host','method','query','body']):
            status,raw,cache=request(f'api-{index}-control',route);assert status==200;validator.validate(json.loads(raw));check(f'api-{index}-valid-control',cache=='no-store')
            kwargs={'fault':fault}if fault in ['cookie','host']else {'method':'POST','csrf':csrf}if fault=='method'else {'body':b'PRIVATE_PREREQ_CANARY'}if fault=='body'else {}
            status,raw,_=request(f'api-{index}-refusal',route+'?root=PRIVATE_PREREQ_CANARY'if fault=='query'else route,**kwargs)
            expected_status=403 if fault in ['cookie','host']else 405 if fault=='method'else 400
            check(f'api-{index}-refusal-and-safe-diagnostic',status==expected_status and b'PRIVATE_PREREQ_CANARY'not in raw)
            status,raw,_=request(f'api-{index}-recovery',route);assert status==200;value=json.loads(raw);validator.validate(value);check(f'api-{index}-recovery-current-identity',value['identity']['value']['euid']==os.geteuid())
        after=snapshot();(out/'root-after.json').write_text(json.dumps({n:('[private credential hash omitted]'if 'web-session-'in n else h)for n,h in after.items()},indent=2)+'\n');check('api-refusals-successes-preserve-fixture-root-bytes',before==after)
        status,raw,_=request('actual-version','/api/v1/native-version');check('same-qualified-version',status==200 and json.loads(raw)['binary']['version']==candidate['metadata']['version'])
        print('NATIVE_PREREQUISITES_PASS checks='+str(len(checks)),flush=True)
    except BaseException as error:failure=error;raise
    finally:
        cleanup=None
        if app is not None:
            if app.poll()is None:app.send_signal(signal.SIGINT)
            app.wait()
            if drain is not None:drain.join()
            errors.close();raw=(out/'server-private.stderr').read_bytes();(out/'server.stderr').write_bytes(re.sub(rb'session-[A-Za-z0-9_-]+',b'session-[private]',raw));(out/'server-private.stderr').unlink();(out/'server.stdout').write_bytes(b''.join(server_stdout))
            if app.returncode!=0:cleanup=RuntimeError('owned-native-server-shutdown-failed')
        if errors is not None and not errors.closed:errors.close()
        (out/'result.json').write_text(json.dumps({'status':'PASS'if failure is None and cleanup is None else'FAIL','scope':'qualified normal candidate; isolated CLI/HTTP metadata only, no launch/confinement/human evidence','sha256':expected,'checks':checks,'commands':calls,'http':http_calls},indent=2)+'\n')
        if cleanup is not None:
            if failure is not None:raise BaseExceptionGroup('native check and shutdown failed',[failure,cleanup])
            raise cleanup
if __name__=='__main__':main()
