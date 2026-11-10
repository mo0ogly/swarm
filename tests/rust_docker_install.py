#!/usr/bin/env python3
"""Build and exercise Rust Docker CLI/API on an owned retained Root.

No Compose configuration, real mission, provider or global watchdog. BuildKit
caches are derivatives; image identity, original channels and data remain in fresh
private evidence. Missing output parents are created; existing output is refused. Cleanup can remove only this test's uniquely named container.
"""
import argparse,hashlib,json,os,socket,subprocess,urllib.request,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def main():
    """Check image build, singular JSON output, persistence, API and no Go tool."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(mode=0o700,parents=True,exist_ok=False);project=out/'project';project.mkdir();calls=[];checks=[];name='swarm-rust-install-'+uuid.uuid4().hex;image=name+':test';started=False;primary=None
    def call(args,value=None):
        """Record exact subprocess channels, propagating every failure without retry."""
        index=len(calls)+1
        with (out/(str(index)+'.stdout')).open('xb') as stdout,(out/(str(index)+'.stderr')).open('xb') as stderr:
            r=subprocess.run(args,cwd=ROOT,input=None if value is None else json.dumps(value).encode(),stdout=stdout,stderr=stderr)
        calls.append(dict(argv=args,exit=r.returncode));(out/'commands.json').write_text(json.dumps(calls,indent=2))
        if r.returncode != 0:
            # This fixture runs no provider or secret transfer. Preserve full original
            # channels and expose the failed build/runtime diagnostic to CI.
            import sys
            for suffix in ('stdout','stderr'):
                sys.stderr.buffer.write((out/(str(index)+'.'+suffix)).read_bytes())
            sys.stderr.flush()
            raise AssertionError((index,r.returncode))
        return (out/(str(index)+'.stdout')).read_bytes()
    def check(label,value):
        """Retain distinct observable assertions, never count zero-test success."""
        assert value,label;assert label not in checks;checks.append(label);print('PASS docker-install-'+label,flush=True)
    base=['docker','run','--rm','--network','none','--user',str(os.getuid())+':'+str(os.getgid()),'--mount','type=bind,src='+str(project)+',dst=/workspace',image]
    def cli(args,value=None):
        """Run a new installed container process against this fixture only."""
        command=base.copy()
        if value is not None:command.insert(3,'-i')
        return json.loads(call([*command,*args,'--json'],value))
    try:
        call(['docker','build','--progress=plain','-t',image,'.']);identity=json.loads(call(['docker','image','inspect',image]))[0]['Id'];check('image-identity',identity.startswith('sha256:'))
        check('fresh-CLI-singular-JSON',cli(['work','list'])==[])
        version=cli(['version']);check('truthful-build-identity',version['binary']['version']=='devel' and version['binary']['commit'] is None and version['binary']['provenance']=='unknown')
        methods=cli(['prepare','methods']);check('bundled-methods',len(methods)==5 and all(m['available'] and len(m['sha256'])==64 for m in methods))
        work=cli(['work','create','--input','-'],dict(schema_version=1,event_id='docker-install',expected_revision=0,title='Docker Rust fixture',objective='Preserve data across containers',scope='Owned fixture',criteria=['Recorded work remains']))['work'];check('fresh-container-persistence',cli(['work','show',work['id']])['work']==work)
        check('database-private-owned',(project/'.swarm/state.db').stat().st_uid==os.getuid() and ((project/'.swarm/state.db').stat().st_mode&0o777)==0o600)
        tools=call(['docker','run','--rm','--network','none','--entrypoint','/bin/sh',image,'-c','if command -v go; then exit 7; fi; node --version; python3 --version']);check('runtime-no-Go',b'Python 3' in tools and b'v22.' in tools)
        with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
        address='127.0.0.1:'+str(port)
        call(['docker','run','--detach','--name',name,'--network','host','--user',str(os.getuid())+':'+str(os.getgid()),'--mount','type=bind,src='+str(project)+',dst=/workspace',image,'web',address,'--json']);started=True
        log=subprocess.Popen(['docker','logs','--follow',name],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        log_error=None
        try:
            with (out/'server-start.stdout').open('xb') as saved:
                while True:
                    line=log.stdout.readline();assert line,'container ended before bound event';saved.write(line);saved.flush()
                    if json.loads(line).get('event')=='web-bound':break
        except BaseException as error:log_error=error
        try:
            log.terminate();channels=log.communicate();(out/'server-follow.stderr').write_bytes(channels[1])
        except BaseException as cleanup:
            if log_error:raise BaseExceptionGroup('Docker startup log and follower cleanup failed',[log_error,cleanup])
            raise
        if log_error:raise log_error
        digest=hashlib.sha256(address.encode()).hexdigest();key=(project/'.swarm'/('web-session-'+digest[:16])).read_text();cookie='swarm_session_'+digest[:12]+'='+key
        def get(path):
            """Use the actual protected host-scoped session; never retain the key."""
            with urllib.request.urlopen(urllib.request.Request('http://'+address+path,headers={'Cookie':cookie})) as response:return json.load(response)
        check('protected-native-API',get('/api/v1/native-capabilities')['profile']=='rust-native-http1-observed')
        check('CLI-HTTP-version',get('/api/v1/native-version')['binary']==version['binary']);check('CLI-HTTP-methods',get('/api/v1/preparations/methods')==methods)
        call(['docker','kill','--signal=INT',name]);check('graceful-stop',call(['docker','wait',name]).strip()==b'0');call(['docker','rm',name]);started=False
        check('post-server-persistence',cli(['work','show',work['id']])['work']==work)
        (out/'result.json').write_text(json.dumps(dict(status='PASS',checks=checks,image_id=identity,version=version['binary'],fixture_root=str(project),global_watchdog=None,limits='No real provider, publication, Compose lifecycle or global migration acceptance'),indent=2)+'\n');print('RUST_DOCKER_INSTALL_PASS '+str(len(checks))+' assertions',flush=True)
    except BaseException as error:primary=error
    finally:
        if started:
            try:call(['docker','rm','--force',name])
            except BaseException as cleanup:primary=BaseExceptionGroup('Docker fixture and cleanup failed',([primary] if primary else [])+[cleanup])
    if primary:raise primary
if __name__=='__main__':main()
