#!/usr/bin/env python3
"""Actual Go/native manual-probe differential on owned roots and loopback HTTP.

Compares fixed request, optional auth, redaction, redirect/empty refusal and saved
maximum. Native error codes/receipts/source admission and Go diagnostics/truncation
are explicit differences. No real provider, mission execution or global deadline.
"""
import argparse,json,subprocess,tempfile,threading,time,traceback,re
from pathlib import Path
from http.server import ThreadingHTTPServer,BaseHTTPRequestHandler
from http.client import HTTPConnection
import rust_agent_model_route as proof
from rust_native_http_cli import Server,clean
KEY='OWNED_PARITY_FAKE_KEY'
def run(native,reference,output):
    """Qualify unchanged Go producers, retain safe outcomes and close owned IO.
    Go embedded UI changed since the reference; it is not exercised or qualified
    here. Source Go files and wait defaults must still match the frozen inventory.
    """
    output.mkdir(parents=True,exist_ok=False);proof.publish(output/'harness-source.py',Path(__file__).read_bytes());ref=json.loads(reference.read_bytes());assert ref['status']=='QUALIFIED_PROJECT_SOURCE_REFERENCE';go=Path(ref['binary']);assert proof.digest(go.read_bytes())==ref['sha256'];inventory=json.loads((reference.parent/'inputs.before.json').read_bytes())
    for name,sha in inventory.items():
        if name.endswith('.go') or name=='config/provider-wait.json':assert proof.digest(Path(name).read_bytes())==sha,('reference-producer-changed',name)
    f=proof.Fixture(native,output,'native');before=f.snapshot();go_root=Path(tempfile.mkdtemp(prefix='swarm-go-probe-parity-'));(go_root/'empty-bin').mkdir();env={'PATH':str(go_root/'empty-bin'),'LANG':'C.UTF-8','USER':'fixture','NO_PROXY':'127.0.0.1','no_proxy':'127.0.0.1'};commands=[];events=[];checks=[];mode='ok';primary=None;owned=[];calls=[]
    def cli(args):
        """Run only Go init with no executable/model credentials available."""
        argv=[str(go),*args,'--root',str(go_root),'--json'];r=subprocess.run(argv,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE);proof.publish(output/'go-init.stdout',r.stdout);proof.publish(output/'go-init.stderr',r.stderr);commands.append({'argv':argv,'exit':r.returncode});proof.publish(output/'commands.json',commands);assert r.returncode==0
    class Handler(BaseHTTPRequestHandler):
        """Owned protocol witness: no remote account or command execution."""
        def log_message(self,*args):pass  # Raw standard logs suppressed; typed safe witness retained.
        def do_POST(self):
            selected=mode;body=json.loads(self.rfile.read(int(self.headers['Content-Length'])));assert self.path=='/v1/chat/completions';assert body=={'model':'fixture-parity','messages':[{'role':'user','content':'Réponds seulement : Connexion réussie.'}],'stream':False};events.append({'mode':selected,'fixed_only':True,'anonymous':self.headers.get('Authorization') is None,'authenticated':self.headers.get('Authorization')=='Bearer '+KEY});proof.publish(output/'provider-observations.json',events)
            if selected=='slow':time.sleep(2)  # Finite stimulus exceeds the saved1s maximum, not a product cutoff.
            if selected=='redirect':self.send_response(302);self.send_header('Location','/must-not-follow');self.end_headers();return
            content='' if selected=='empty' else KEY if selected=='echo' else 'Connexion réussie.';raw=json.dumps({'choices':[{'index':0,'finish_reason':'stop','message':{'content':content}}]}).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(raw)));self.end_headers()
            try:self.wfile.write(raw)
            except (BrokenPipeError,ConnectionResetError):pass  # Expected after local saved timeout; never evidence of remote rollback.
    provider=ThreadingHTTPServer(('127.0.0.1',0),Handler);provider.daemon_threads=False;thread=threading.Thread(target=provider.serve_forever);thread.start();base='http://127.0.0.1:'+str(provider.server_port)+'/v1';native_server=None;go_process=None;token=None
    def request(kind,path,body=None):
        """One real protected request; credential/CSRF remain memory-only."""
        address,headers=auth[kind];h={**headers};method='GET' if body is None else 'POST'
        if body is not None:h['Content-Type']='application/json'
        host,port=address.rsplit(':',1);c=HTTPConnection(host,int(port),timeout=None)
        try:c.request(method,path,None if body is None else json.dumps(body).encode(),h);r=c.getresponse();raw=r.read();status=r.status
        finally:c.close()
        assert KEY.encode() not in raw,'Credential leaked';value=json.loads(raw);calls.append({'kind':kind,'path':path,'status':status,'value':clean(value)});proof.publish(output/'http.json',calls);assert status==200,(kind,status,value);return value
    try:
        cli(['init']);go_process=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(go_root)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        line=go_process.stdout.readline().decode();match=re.fullmatch(r'Cockpit local : http://(127\.0\.0\.1:[0-9]+)/session/([^\s]+)\n',line);assert match,('Go startup refused',go_process.poll());address,token=match.groups();cookie='swarm_session='+token
        native_server=Server(native,f.root,output,'native-http',environment={**f.environment,'NO_PROXY':'127.0.0.1','no_proxy':'127.0.0.1'});native_server.capture_async(output/'native-stream.jsonl')
        auth={'go':(address,{'Cookie':cookie,'Origin':'http://'+address,'X-Swarm-CSRF':token}),'native':(native_server.address,{'Cookie':native_server.cookie,'Origin':'http://'+native_server.address})};csrf=request('native','/api/v1/native-session')['csrf_token'];auth['native'][1]['X-Swarm-CSRF']=csrf
        proof.publish(output/'go-startup.json',{'address':address,'private_session_observed':True,'argv':[str(go),'web','127.0.0.1:0','--root',str(go_root)]})
        for selected in ['ok','echo','redirect','empty']:
            mode=selected;results=[]
            for kind in ['go','native']:
                digest=request(kind,'/api/v1/providers/connections')['digest'];change={'version':1,'expected_digest':digest,'connection':{'id':'parity-probe','label':'Owned parity','base_url':base,'model':'fixture-parity','key':KEY if selected=='echo' else '','disabled':False},'replace_key':True};value=request(kind,'/api/v1/providers/connections/test',change);results.append(value);assert value['ok']==(selected in ['ok','echo']);assert events[-1]['authenticated' if selected=='echo' else 'anonymous']
            if selected=='ok':assert results[0]['text']==results[1]['text']=='Connexion réussie.'
            if selected=='echo':assert all(KEY not in v['text'] and v['text'] for v in results)
            checks.append('actual-go-native-'+selected)
        for kind in ['go','native']:
            current=request(kind,'/api/v1/provider-wait');r={'schema_version':1,'event_id':'parity-probe-maximum','expected_revision':current['revision'],'values':{**current['values'],'max_duration_seconds':1},'reason':'Owned differential saved maximum'};preview=request(kind,'/api/v1/provider-wait/preview',r)
            if kind=='native':r['preview_token']=preview['preview_token']
            request(kind,'/api/v1/provider-wait/apply',r)
            mode='slow';digest=request(kind,'/api/v1/providers/connections')['digest'];value=request(kind,'/api/v1/providers/connections/test',{'version':1,'expected_digest':digest,'connection':{'id':'parity-probe','label':'Owned parity','base_url':base,'model':'fixture-parity','key':'','disabled':False},'replace_key':True});assert value['ok'] is False and value.get('error')
        checks.append('actual-go-native-saved-total-maximum');assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    except BaseException as error:primary=error
    cleanup=[]
    if native_server:
        try:native_server.cleanup()
        except BaseException as error:cleanup.append(error)
    if go_process:
        try:
            if go_process.poll() is None:go_process.terminate()
            _stdout,stderr=go_process.communicate();assert token is None or token.encode() not in stderr;proof.publish(output/'go-stop.json',{'exit':go_process.returncode,'stderr_sha256':proof.digest(stderr)})
        except BaseException as error:cleanup.append(error)
    for action in [provider.shutdown,provider.server_close,thread.join]:
        try:action()
        except BaseException as error:cleanup.append(error)
    if primary or cleanup:raise BaseExceptionGroup('Differential or cleanup failed',([primary] if primary else [])+cleanup)
    assert len(checks)==5 and len(events)==10;proof.publish(output/'result.json',{'status':'PASS','checks':checks,'go_sha256':ref['sha256'],'native_sha256':proof.digest(native.read_bytes()),'account_contacted':False,'scope':'fixed request/optional auth/redaction/refusals/saved maximum only; no UI/global parity'});print('API_CONNECTION_PROBE_GO_PARITY_VERIFIED 5',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-reference',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_reference.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():proof.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
