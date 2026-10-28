#!/usr/bin/env python3
"""Actual connection-test API on an owned Root and loopback-only text provider.

No account, model executable, mission mutation or global watchdog. Finite mock
delays are test stimuli derived from the saved maximum, not production cutoffs.
Original status/body, safe request observations and owned cleanup are retained.
"""
import argparse,json,threading,time,traceback,signal
from pathlib import Path
from http.server import ThreadingHTTPServer,BaseHTTPRequestHandler
from http.client import HTTPConnection
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean

PROMPT='Réponds seulement : Connexion réussie.'
KEY='OWNED_FAKE_PROBE_PRIVATE_KEY'
def run(binary,output):
    """Exercise real route, source binding, redaction, invalid/timeout/recovery.

    Only explicit fixture settings use public preview/apply. Provider observations
    are redacted booleans, never real credentials. Outcome false remains failure.
    """
    output.mkdir(parents=True,exist_ok=False);retained.publish(output/'harness-source.py',Path(__file__).read_bytes());f=retained.Fixture(binary,output,'connection-probe');before=f.snapshot();events=[];errors=[];mode='ok';delay=0;calls=[];checks=[];primary=None;csrf=None;contact_started=threading.Event()
    class Handler(BaseHTTPRequestHandler):
        """Owned OpenAI-compatible JSON fixture; no commands, tools or DNS."""
        def log_message(self,*args):pass  # Suppress standard raw HTTP log; structured proof below.
        def do_POST(self):
            """Assert fixed request and return one selected complete/refused reply."""
            try:
                selected=mode;selected_delay=delay
                payload=json.loads(self.rfile.read(int(self.headers.get('Content-Length','0'))))
                assert self.path in ['/v1/chat/completions','/alternate/v1/chat/completions']
                assert payload=={'model':'fixture-probe','messages':[{'role':'user','content':PROMPT}],'stream':False}
                observation={'mode':selected,'fixed_prompt_only':True,'authorization':self.headers.get('Authorization')=='Bearer '+KEY,'no_authorization':self.headers.get('Authorization') is None};events.append(observation)
                retained.publish(output/'provider-observations.json',events);contact_started.set()
                if selected=='slow':time.sleep(selected_delay)
                if selected=='redirect':
                    self.send_response(302);self.send_header('Location',base+'/stolen');self.end_headers();return
                message={'content':'Connexion réussie.' if selected!='echo' else 'Echo '+KEY}
                if selected=='tools':message['tool_calls']=[{'type':'function','function':{'name':'must-not-run','arguments':'{}'}}]
                if selected=='empty':message['content']=''
                raw=json.dumps({'choices':[{'index':0,'finish_reason':'stop','message':message}]}).encode()
                self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(raw)));self.end_headers()
                try:self.wfile.write(raw)
                except (BrokenPipeError,ConnectionResetError):
                    # A saved timeout may cancel this owned local socket; remote
                    # billing/execution is still not established by that event.
                    observation['peer_closed']=True;retained.publish(output/'provider-observations.json',events)
            except BaseException:
                errors.append(traceback.format_exc());retained.publish(output/'provider-errors.json',errors);raise
    class Provider(ThreadingHTTPServer):
        """Join every owned handler during close; no detached fixture threads."""
        daemon_threads=False
    provider=Provider(('127.0.0.1',0),Handler);base='http://127.0.0.1:'+str(provider.server_port);thread=threading.Thread(target=provider.serve_forever);thread.start()
    environment={**f.environment,'NO_PROXY':'127.0.0.1,localhost','no_proxy':'127.0.0.1,localhost'}
    try:s=Server(binary,f.root,output,'http',environment=environment);s.capture_async(output/'stream.jsonl')
    except BaseException as primary:
        secondary=[]
        for cleanup in [provider.shutdown,provider.server_close,thread.join]:
            try:cleanup()
            except BaseException as error:secondary.append(error)
        raise BaseExceptionGroup('Native bootstrap or provider cleanup failed',[primary,*secondary])

    def request(path,body=None,status=200):
        """Real cookie, Origin and CSRF; retain safe JSON and exact status."""
        headers={'Cookie':s.cookie,'Origin':'http://'+s.address}
        if body is not None:headers.update({'Content-Type':'application/json','X-Swarm-CSRF':csrf})
        c=HTTPConnection(s.host,s.port,timeout=None)
        try:c.request('GET' if body is None else 'POST',path,None if body is None else json.dumps(body).encode(),headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        value=json.loads(raw);assert KEY.encode() not in raw,'Private fixture credential leaked in public response';calls.append({'path':path,'status':actual,'value':clean(value)});retained.publish(output/'http.json',calls);assert actual==status,(actual,clean(value));return value
    def change(**extra):
        """Use current source digest and explicit replacement/anonymous choice."""
        return {'version':1,'expected_digest':request('/api/v1/providers/connections')['digest'],'connection':{'id':'probe','label':'Owned probe','base_url':base+'/v1','model':'fixture-probe','key':'','disabled':False},'replace_key':True,**extra}
    def unchanged():
        """No model executable or Work/Agent/reservation mutation."""
        assert f.snapshot()==before and not list(f.root.rglob('model-received.json'))
    try:
        csrf=request('/api/v1/native-session')['csrf_token'];assert request('/api/v1/provider-wait')['values']['max_duration_seconds']==0
        assert request('/api/v1/providers/connections')['connections']==[] and not events
        r=change();result=request('/api/v1/providers/connections/test',r);assert result['ok'] is True and result['provider_contact_may_have_occurred'] is True and 'Connexion' in result['text'];assert events[-1]['no_authorization'];unchanged();checks.append('anonymous-real-loopback-fixed-prompt-no-mission-context')
        mode='echo';r=change();r['connection']['key']=KEY;result=request('/api/v1/providers/connections/test',r);assert result['ok'] is True and KEY not in result['text'] and events[-1]['authorization'];unchanged();checks.append('explicit-key-and-echo-redaction')
        # Retained credentials use the writer's actual saved source and the
        # same exact destination; a submitted replacement is ignored when false.
        mode='ok';saved=change();saved['connection']['key']=KEY;saved['preview']=True
        preview=request('/api/v1/providers/connections',saved);request('/api/v1/providers/connections',{**saved,'preview':False,'preview_token':preview['preview_token']})
        raw_before=(f.root/'.swarm/ai-connections.json').read_bytes()
        keep=change(replace_key=False);keep['connection']['key']='IGNORED_FAKE_INPUT'
        result=request('/api/v1/providers/connections/test',keep);assert result['ok'] is True and events[-1]['authorization']
        alternate=change();alternate['connection']['base_url']=base+'/alternate/v1';alternate['connection']['key']=KEY
        assert request('/api/v1/providers/connections/test',alternate)['ok'] is True
        count=len(events);refused={**alternate,'replace_key':False};request('/api/v1/providers/connections/test',refused,status=400);assert len(events)==count
        assert request('/api/v1/providers/connections/test',alternate)['ok'] is True
        assert (f.root/'.swarm/ai-connections.json').read_bytes()==raw_before;unchanged();checks.append('retained-key-exact-destination-refusal-explicit-replacement-and-recovery')
        count=len(events)
        for method,body,status in [('GET',None,405),('POST',b'{',400)]:
            c=HTTPConnection(s.host,s.port,timeout=None)
            try:
                c.request(method,'/api/v1/providers/connections/test',body,{'Cookie':s.cookie,'Origin':'http://'+s.address,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'});reply=c.getresponse();raw=reply.read();assert reply.status==status,(method,reply.status,raw)
            finally:c.close()
        assert len(events)==count;unchanged();checks.append('method-and-malformed-refused-before-provider')
        r=change();r['connection']['disabled']=True;request('/api/v1/providers/connections/test',r,status=400);r=change();r['expected_digest']='0'*64;request('/api/v1/providers/connections/test',r,status=409);assert len(events)==count;unchanged();checks.append('disabled-and-stale-source-before-contact')
        for selected in ['redirect','tools','empty']:
            mode=selected;result=request('/api/v1/providers/connections/test',change());assert result['ok'] is False and result['provider_contact_may_have_occurred'] is True and result.get('error')
        mode='ok';assert request('/api/v1/providers/connections/test',change())['ok'] is True;unchanged();checks.append('redirect-tool-empty-refusal-and-recovery')
        settings=f.ok(['engine-settings','show'])['values'];cap=settings['preparation_call_response_max_bytes']
        f.settings_change(preparation_call_response_max_bytes='0')
        result=request('/api/v1/providers/connections/test',change());assert result['ok'] is False and result['error']=='api-response-over-limit' and result['provider_contact_may_have_occurred'] is True
        f.settings_change(preparation_call_response_max_bytes=cap)
        assert request('/api/v1/providers/connections/test',change())['ok'] is True;unchanged();checks.append('saved-response-cap-refusal-and-recovery')
        values=request('/api/v1/provider-wait')['values'];policy={'schema_version':1,'event_id':'probe-deadline','expected_revision':0,'values':{**values,'max_duration_seconds':1},'reason':'Owned actual saved probe deadline'}
        pv=request('/api/v1/provider-wait/preview',policy);request('/api/v1/provider-wait/apply',{**policy,'preview_token':pv['preview_token']})
        mode='slow';delay=2;started=time.monotonic();result=request('/api/v1/providers/connections/test',change());elapsed=time.monotonic()-started;assert result['ok'] is False and result['provider_contact_may_have_occurred'] is True and elapsed>=1;assert 'timeout' in result['error']
        mode='ok';assert request('/api/v1/providers/connections/test',change())['ok'] is True;unchanged();checks.append('saved-total-maximum-applied-and-recovery')
        # Restore unlimited via the same public policy before cancelling an
        # owned actual HTTP request. The finite2s mock delay is stimulus only.
        view=request('/api/v1/provider-wait');unlimited={'schema_version':1,'event_id':'probe-unlimited','expected_revision':view['revision'],'values':{**view['values'],'max_duration_seconds':0},'reason':'Owned unlimited cancellation fixture'}
        preview=request('/api/v1/provider-wait/preview',unlimited);request('/api/v1/provider-wait/apply',{**unlimited,'preview_token':preview['preview_token']})
        mode='slow';delay=2;draft=change();contact_started.clear();cancel_result=[]
        def pending_probe():
            # Server shutdown closes this owned connection; retain original type
            # safely, never treat a missing HTTP reply as remote nonexecution.
            try:cancel_result.append({'reply':request('/api/v1/providers/connections/test',draft)})
            except BaseException as error:cancel_result.append({'failure_type':type(error).__name__})
            finally:contact_started.set()
        waiter=threading.Thread(target=pending_probe);waiter.start();contact_started.wait()
        assert events[-1]['mode']=='slow' and not cancel_result
        s.process.send_signal(signal.SIGINT);s.stop_sent=True;waiter.join();assert cancel_result and 'failure_type' in cancel_result[0]
        retained.publish(output/'cancelled-client.json',cancel_result);unchanged();checks.append('unlimited-probe-local-shutdown-drained-without-mission-mutation')
    except BaseException as error:primary=error
    cleanups=[]
    for cleanup in [s.cleanup,provider.shutdown,provider.server_close,thread.join]:
        try:cleanup()
        except BaseException as error:cleanups.append(error)
    if primary or cleanups:raise BaseExceptionGroup('Probe or owned cleanup failed',([primary] if primary else [])+cleanups)
    stream=[json.loads(row) for row in (output/'stream.jsonl').read_bytes().splitlines()]
    outcomes=[r['outcome'] for r in stream if r.get('event')=='web-outcome' and r['outcome'].get('provider_probe')]
    assert any(o['provider_probe']['error']=='api-probe-timeout' and o['provider_probe']['provider_contact_may_have_occurred'] is True for o in outcomes)
    assert all(o['recorded_work'] is None and o['configuration_may_be_recorded'] is False for o in outcomes)
    assert stream[-1]['event']=='web-stopped' and stream[-1]['provider_contact_may_have_occurred'] is True
    assert any(o['provider_probe']['error']=='api-probe-cancelled' and o['provider_probe']['provider_contact_may_have_occurred'] is True for o in outcomes)
    assert KEY.encode() not in (output/'stream.jsonl').read_bytes();checks.append('negative-result-owned-through-observer-and-clean-stop')
    assert not errors and not s.capture_errors and len(checks)==10
    retained.publish(output/'result.json',{'status':'PASS','checks':checks,'model_process_started':False,'remote_accounts_contacted':False,'binary_sha256':retained.digest(binary.read_bytes())});print('API_CONNECTION_PROBE_VERIFIED 10',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
