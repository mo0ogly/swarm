#!/usr/bin/env python3
"""Compare actual Go/native report text on owned inactive Roots; no authority grant."""
import argparse,json,subprocess,re,urllib.parse,traceback,http.client
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server
from rust_agent_exchanges import logical
from rust_planning_history import clean

def run(binary,go,out,unported=False):
    """Retain positive Go and native refusal witnesses, with checked owned cleanup.
    Private-marker files are inert fixture data, never real runtime evidence.
    """
    out.mkdir(parents=True,exist_ok=False);publish(out/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,out,'report');gf=Fixture(binary,out/'go','report-go');s=None;g=None;primary=None;calls=[];checks=[]
    def wire(address,cookie,path,status=200,method='GET',body=None,csrf=None):
        """Require exact protected status/content; retain no session/CSRF bytes."""
        host,port=address.rsplit(':',1);c=http.client.HTTPConnection(host,int(port),timeout=None);headers={'Origin':'http://'+address}
        if cookie:headers['Cookie']=cookie
        if csrf:headers['X-Swarm-CSRF']=csrf
        try:c.request(method,path,body,headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        try:value=json.loads(raw) if raw else None
        except (ValueError,UnicodeError):value=raw.decode(errors='replace')
        calls.append(dict(path=path,status=actual,method=method,value=clean(value)));publish(out/'http.json',calls);assert actual==status,(actual,value);return value
    try:
        for fixture in [f,gf]:
            (fixture.root/'docs').mkdir();publish(fixture.root/'docs/visible.md',b'fixture <img> \xff text');publish(fixture.root/'docs/large.md',b'A'*262144+b'B');publish(fixture.root/'.swarm/private-report.txt',b'private-inert-fixture-not-runtime')
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(gf.root)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=gf.environment);m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',g.stdout.readline().decode().strip());assert m;address,key=m.groups();path='/api/v1/report?'+urllib.parse.urlencode({'path':'docs/visible.md'});reference=wire(address,'swarm_session='+key,path);assert reference=={'text':'fixture <img> \ufffd text'};large_path='/api/v1/report?'+urllib.parse.urlencode({'path':'docs/large.md'});large=wire(address,'swarm_session='+key,large_path);assert large=={'text':'A'*262144+'\n[Affichage limité à 256 Kio]'};wire(address,'swarm_session='+key,'/api/v1/report?path=.swarm%2Fprivate-report.txt',403);checks.append('actual-go-docs-invalid-utf-default-clipping-private-refusal')
        s=Server(binary,f.root,out,'http',environment=f.environment);s.capture_async(out/'stream.jsonl');before=logical(f.root)
        if unported:wire(s.address,s.cookie,path,404);assert logical(f.root)==before;checks.append('native-report-route-absence')
        else:raise AssertionError('Final native report cases must be specified before implementation closure')
    except BaseException as e:primary=e
    errors=[]
    if g is not None:
        try:g.terminate();g.communicate()
        except BaseException as e:errors.append(e)
    if s is not None:
        try:s.cleanup()
        except BaseException as e:errors.append(e)
    if primary:errors.insert(0,primary)
    if errors:raise BaseExceptionGroup('Report read and owned cleanup',errors)
    assert not s.capture_errors
    publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),inert_fixture_files=True,provider_requested=False));print('REPORT_READ_UNPORTED_WITNESS' if unported else 'REPORT_READ_PUBLIC_VERIFIED',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--go-binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
