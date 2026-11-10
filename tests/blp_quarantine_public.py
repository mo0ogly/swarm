#!/usr/bin/env python3
"""Qualify preparation quarantine with real CLI/HTTP and inert API/CLI receivers.

Uses an owned Root, public configuration/producer calls and a separately built
fixture-only pending-turn helper. Its private child execs the exact qualified
production image. No SQL writes, live data, paid provider, watchdog or retry.
Original commands, network observations and artifact hashes survive failure.
"""
import argparse
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

from blp_attribution_history_cli import digest
from fixture_area import fixture_area
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server


def main():
    """Assert legacy controls, zero-egress refusals, pending-worker fencing,
    serialized activation, explicit recovery and Admin FR/EN editing. Requires
    immutable qualified production binary/profile, separate helper and fresh
    evidence output. All mutations target the marked fixture only. Cleanup
    preserves primary and secondary errors, without forcing a passing result.
    """
    ap=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','helper','output']:ap.add_argument('--'+name,type=Path,required=True)
    a=ap.parse_args();binary=a.binary.resolve(strict=True);out=a.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(a.profile.read_bytes());sha=digest(binary)
    assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['sha256']==sha
    assert profile['no_fixture_feature'] and profile['all_core_features_empty'] and profile['all_core_test_false']
    publish(out/'qualified-profile.json',profile)
    f=Fixture(binary,out,'quarantine');checks=[];received=[];accepted=[];receiver_errors=[];hold=threading.Event();entered=threading.Event();release=threading.Event();web=None;writer=None
    publish(f.root/'.blp-quarantine-fixture',b'owned inert quarantine fixture\n')
    f.environment.update(SWARM_BLP_BINARY=str(binary),SWARM_BLP_BINARY_SHA256=sha,SWARM_BLP_FIXTURE_AREA=str(fixture_area()),SWARM_TRUST_TOKEN='fixture-admin-token')

    def check(name,condition):
        """Retain the named assertion before raising; no empty success witness."""
        checks.append(dict(name=name,passed=bool(condition)));publish(out/'checks.json',checks);assert condition,name;print('PASS '+name,flush=True)

    class Receiver(ThreadingHTTPServer):
        """Record accepted sockets as well as complete requests, not just HTTP200."""
        daemon_threads=True
        def handle_error(self,request,address):
            receiver_errors.append(type(sys.exc_info()[1]).__name__);publish(out/'receiver-errors.json',receiver_errors)
            super().handle_error(request,address)
        def get_request(self):
            connection,address=super().get_request();accepted.append(dict(at=time.time(),peer=address[0]));publish(out/'receiver-connections.json',accepted);return connection,address

    class Handler(BaseHTTPRequestHandler):
        """Owned inert receiver: records actual body and controlled hold/release."""
        def log_message(self,*args):pass
        def do_POST(self):
            raw=self.rfile.read(int(self.headers['Content-Length']));data=json.loads(raw)
            cli=self.path=='/cli-observe'
            observed=dict(path=self.path,kind='cli' if cli else 'api',bytes=len(raw),sha256=hashlib.sha256(raw).hexdigest(),sentinel_present=data.get('sentinel_present') if cli else 'PRIVATE_QUARANTINE_FIXTURE' in json.dumps(data))
            received.append(observed);publish(out/'receiver-requests.json',received)
            if hold.is_set():entered.set();release.wait()
            body={'ok':True} if cli else {'model':'fixed-model','choices':[{'message':{'content':json.dumps({'message':'Inert quarantine answer','brief':'Inert brief'})},'finish_reason':'stop'}],'usage':{'prompt_tokens':1,'completion_tokens':1}}
            wire=json.dumps(body).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(wire)));self.end_headers();self.wfile.write(wire);self.wfile.flush()

    receiver=Receiver(('127.0.0.1',0),Handler);thread=threading.Thread(target=receiver.serve_forever,daemon=True);thread.start()

    def trust(op,value):
        """Use normal CLI and independent fixture bearer, never a claimed actor."""
        return f.ok(['trust',op],value)

    def policy(revision,enabled):
        """Explicit test input, not a live default or authority grant."""
        return dict(schema_version=1,event_id='quarantine-'+str(revision),expected_revision=revision,expected_trust_revision=1,reason='Owned quarantine receiver fixture',policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public')],compartments=[],clearances={},preparation_quarantine=enabled))

    def create(event):
        """Create/budget through public intents, without mission launch/adoption."""
        p=f.ok(['prepare','create'],dict(version=1,action='create',event_id=event,expected_revision=0,title='Quarantine fixture',text='Owned need'))
        return f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget-'+event,expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=.5,estimate_source='Inert fixture',reference_date='2026-11-01')))

    def intent(p,event,provider='api-local'):
        """Bind every send to its actual saved provider capability and revision."""
        c=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']==provider)
        return dict(version=1,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],provider=provider,capability_hash=c['digest'],message='PRIVATE_QUARANTINE_FIXTURE',context_mode='full')

    def terminal(p,turn):
        """Observe named transitions without an arbitrary whole-test deadline."""
        previous=None
        while True:
            t=next(t for t in f.ok(['prepare','dialogue',p['id']]) if t['id']==turn['id']);publish(out/'last-turn.json',t)
            if t['status']!=previous:print('OBSERVE '+t['id']+' '+t['status'],flush=True);previous=t['status']
            if t['status'] not in ['pending','running','stopping']:return t
            time.sleep(.05)

    def helper(op,value):
        """Keep fixture helper identity distinct; production worker is hash-pinned."""
        arg=out/('helper-'+op+'.input.json') if op=='stage' else value
        if op=='stage':publish(arg,value)
        argv=[str(a.helper.resolve()),op,str(f.root),str(arg)];r=subprocess.run(argv,env=f.environment,capture_output=True)
        publish(out/('helper-'+op+'.stdout'),r.stdout);publish(out/('helper-'+op+'.stderr'),r.stderr)
        publish(out/('helper-'+op+'.command.json'),dict(argv=argv,exit=r.returncode,fixture_binary_sha256=digest(a.helper),production_binary_sha256=sha))
        assert r.returncode==0,('helper failed',op);return json.loads(r.stdout)

    primary=None
    try:
        f.environment.pop('SWARM_PREPARATION_DISABLED',None)
        f.settings_change(preparation_write_enabled='1',preparation_call_enabled='1',agent_allow_unverified_provider_context='1',agent_environment_allow=['PATH','HOME','CODEX_HOME'])
        code='''#!/usr/bin/python3
import sys,json,urllib.request
raw=sys.stdin.buffer.read()
r=urllib.request.Request(__URL__,data=json.dumps({'sentinel_present':b'PRIVATE_QUARANTINE_FIXTURE' in raw}).encode(),headers={'Content-Type':'application/json'})
with urllib.request.urlopen(r) as response:response.read()
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':json.dumps({'message':'Inert CLI answer','brief':'Inert CLI brief'})}}),flush=True)
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':1,'output_tokens':1}}),flush=True)
'''.replace('__URL__',repr('http://127.0.0.1:'+str(receiver.server_port)+'/cli-observe'))
        publish(f.root/'codex',code.encode());os.chmod(f.root/'codex',0o700);f.provider['args']=['--model','fixture-standard'];f.provider['model_policy']['levels']['standard']['effort']='';f.save_provider()
        current=f.ok(['providers','connections','show']);change=dict(version=1,expected_digest=current['digest'],connection=dict(id='local',label='Owned inert API',base_url='http://127.0.0.1:'+str(receiver.server_port)+'/v1',model='fixed-model',key='PRIVATE_INERT_KEY',disabled=False),replace_key=True,preview=True)
        preview=f.ok(['providers','connections','preview'],change);f.ok(['providers','connections','apply'],{**change,'preview':False,'preview_token':preview['preview_token']})
        trust('config-apply',dict(schema_version=1,event_id='trust-init',expected_revision=0,reason='Owned quarantine fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='worker',integrity=1,roles=['requester'],token='fixture-worker-token'),dict(subject='approver',integrity=2,roles=['approver'],token='fixture-approver-token')]))
        trust('blp-policy-apply',policy(0,False));p=create('main');queued=create('queued')
        t=terminal(p,f.ok(['prepare','send',p['id']],intent(p,'api-control')))
        check('valid-api-control-before-quarantine',t['status']=='answered' and len(received)==1 and received[0]['kind']=='api' and received[0]['sentinel_present'])
        t=terminal(p,f.ok(['prepare','send',p['id']],intent(p,'cli-control','fixture')))
        check('valid-cli-control-before-quarantine',t['status']=='answered' and len(received)==2 and received[1]['kind']=='cli' and received[1]['sentinel_present'])
        pending=helper('stage',intent(queued,'pending'));check('actual-pending-engine-producer',pending['status']=='pending' and len(received)==2)
        hold.set();inflight=f.ok(['prepare','send',p['id']],intent(p,'engaged'))
        while not entered.is_set():
            seen=next(t for t in f.ok(['prepare','dialogue',p['id']]) if t['id']==inflight['id']);assert seen['status'] in ['pending','running'],seen;time.sleep(.05)
        check('actual-engaged-api-request-received',len(received)==3 and received[-1]['kind']=='api')
        import fcntl
        with (f.root/'.swarm/preparation-egress.lock').open('r+b') as probe:
            blocked=False
            try:fcntl.flock(probe,fcntl.LOCK_EX|fcntl.LOCK_NB)
            except BlockingIOError:blocked=True
            else:fcntl.flock(probe,fcntl.LOCK_UN)
        check('actual-http-operation-holds-egress-reservation',blocked)
        with (f.root/'.swarm/trust-actions.lock').open('r+b') as probe:
            fcntl.flock(probe,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(probe,fcntl.LOCK_UN)
        check('business-trust-reservation-remains-available',True)
        state=f.root/'.swarm/trust-actions.json';before=state.read_bytes();value=policy(1,True);publish(out/'activate.input.json',value)
        argv=[str(binary),'trust','blp-policy-apply','--root',str(f.root),'--input',str(out/'activate.input.json'),'--json'];writer=subprocess.Popen(argv,env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        check('activation-cannot-publish-during-engaged-egress',state.read_bytes()==before and writer.poll() is None)
        release.set();stdout,stderr=writer.communicate();publish(out/'activate.stdout',stdout);publish(out/'activate.stderr',stderr);publish(out/'activate.command.json',dict(argv=argv,exit=writer.returncode));assert writer.returncode==0
        check('engaged-operation-completes-before-quarantine',terminal(p,inflight)['status']=='answered')
        check('quarantine-persisted',trust('blp-show',{})['capability']['preparation_quarantined'] is True)
        before=state.read_bytes();count=len(received);connections=len(accepted);budget=f.ok(['prepare','budget',p['id']]);turns=f.ok(['prepare','dialogue',p['id']])
        for name,provider in [('api','api-local'),('cli','fixture')]:
            code,result=f.call(['prepare','send',p['id']],intent(p,'blocked-'+name,provider));check(name+'-send-explicitly-refuses',code!=0 and result['failure']['code']=='blp-preparation-quarantined')
        check('refusal-keeps-journal-budget-and-turns',state.read_bytes()==before and f.ok(['prepare','budget',p['id']])==budget and f.ok(['prepare','dialogue',p['id']])==turns)
        check('refused-cli-api-send-no-socket-or-byte',len(received)==count and len(accepted)==connections)
        done=helper('dispatch',pending['id']);check('pending-production-worker-refuses-at-claim',done['status']=='failed' and 'blp-preparation-quarantined' in done['error'])
        check('pending-refusal-no-socket-or-byte',len(received)==count and len(accepted)==connections)
        qb=f.ok(['prepare','budget',queued['id']]);check('unstarted-reservation-released-not-estimated',qb['reserved_usd']==0 and qb['committed_estimate_usd']==0)
        web=Server(binary,f.root,out,'quarantine-http',environment=f.environment);web.capture_async(out/'http-observer.raw');csrf=web.request('/api/v1/native-session',observe=False)['csrf_token']
        connection=http.client.HTTPConnection(web.host,web.port,timeout=None)
        try:
            connection.request('POST','/api/v1/preparations/send',body=json.dumps(intent(p,'http-refused')).encode(),headers={'Origin':'http://'+web.address,'Cookie':web.cookie,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'})
            response=connection.getresponse();raw=response.read();status=response.status
        finally:connection.close()
        publish(out/'http-refusal.json',dict(status=status,body=json.loads(raw)));check('http403-quarantine-refusal',status==403 and json.loads(raw)['failure']['code']=='blp-preparation-quarantined')
        check('http-refusal-no-socket-or-byte',len(received)==count and len(accepted)==connections)
        hold.clear();trust('blp-policy-apply',policy(2,False));check('explicit-disabled-state-reread',trust('blp-show',{})['capability']['preparation_quarantined'] is False)
        t=terminal(p,f.ok(['prepare','send',p['id']],intent(p,'recovery')));check('explicit-recovery-resumes-legacy-egress',t['status']=='answered' and len(received)==count+1)
        hold.set();entered.clear();release.clear();cli_inflight=f.ok(['prepare','send',p['id']],intent(p,'engaged-cli','fixture'))
        while not entered.is_set():
            seen=next(t for t in f.ok(['prepare','dialogue',p['id']]) if t['id']==cli_inflight['id']);assert seen['status'] in ['pending','running'],seen;time.sleep(.05)
        check('actual-engaged-cli-request-received',received[-1]['kind']=='cli')
        with (f.root/'.swarm/preparation-egress.lock').open('r+b') as probe:
            blocked=False
            try:fcntl.flock(probe,fcntl.LOCK_EX|fcntl.LOCK_NB)
            except BlockingIOError:blocked=True
            else:fcntl.flock(probe,fcntl.LOCK_UN)
        check('actual-cli-operation-holds-egress-reservation',blocked)
        before=state.read_bytes();publish(out/'activate-cli.input.json',policy(3,True))
        argv=[str(binary),'trust','blp-policy-apply','--root',str(f.root),'--input',str(out/'activate-cli.input.json'),'--json'];writer=subprocess.Popen(argv,env=f.environment,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        check('activation-cannot-publish-during-cli-egress',state.read_bytes()==before and writer.poll() is None)
        release.set();stdout,stderr=writer.communicate();publish(out/'activate-cli.stdout',stdout);publish(out/'activate-cli.stderr',stderr);publish(out/'activate-cli.command.json',dict(argv=argv,exit=writer.returncode));assert writer.returncode==0
        check('engaged-cli-completes-before-quarantine',terminal(p,cli_inflight)['status']=='answered')
        hold.clear();trust('blp-policy-apply',policy(4,False));check('second-explicit-disabled-state-reread',trust('blp-show',{})['capability']['preparation_quarantined'] is False)
        (out/'evidence').mkdir(mode=0o700)
        config=dict(url='http://'+web.address,root=str(f.root),cookie=web.cookie,token='fixture-admin-token',output=str(out/'evidence/ui'),public_output=str(out/'ui-result.json'),compiled_inputs=str(a.profile.parent/'after.json'))
        publish(out/'evidence/ui-config.json',config)
        run=subprocess.run(['node','tests/blp_quarantine_ui.cjs',str(out/'evidence/ui-config.json')],env=f.environment,capture_output=True);publish(out/'ui.stdout',run.stdout);publish(out/'ui.stderr',run.stderr);assert run.returncode==0,'browser failed; see retained diagnostics'
        check('admin-ui-fr-en-themes-mobile-save-reopen',json.loads((out/'ui-result.json').read_bytes())['status']=='PASS' and json.loads((out/'ui-result.json').read_bytes())['assertions']==59)
        check('qualified-production-image-unchanged',digest(binary)==sha)
    except BaseException as error:primary=error;raise
    finally:
        errors=[];release.set()
        if writer is not None and writer.poll() is None:
            try:
                stdout,stderr=writer.communicate();publish(out/'late-activation.stdout',stdout);publish(out/'late-activation.stderr',stderr)
            except BaseException as error:errors.append(error)
        if web is not None:
            try:web.cleanup()
            except BaseException as error:errors.append(error)
        try:receiver.shutdown();receiver.server_close();thread.join()
        except BaseException as error:errors.append(error)
        if errors:raise BaseExceptionGroup('Journey and owned cleanup failures',([primary] if primary else [])+errors)
    check('receiver-has-no-unexpected-errors',not receiver_errors)
    artifacts={str(p.relative_to(out)):digest(p) for p in sorted(out.rglob('*')) if p.is_file() and 'evidence' not in p.relative_to(out).parts}
    publish(out/'result.json',dict(status='PASS',assertions=len(checks),artifacts=artifacts,binary_sha256=sha,scope='explicit quarantine of controlled preparation sends; no classified allow-path, source identity or descendant confinement'))
    print('QUARANTINE_PUBLIC_PASS '+str(len(checks)),flush=True);print('QUARANTINE_JOURNEY_SHA256 '+digest(out/'result.json'),flush=True)


if __name__=='__main__':main()
