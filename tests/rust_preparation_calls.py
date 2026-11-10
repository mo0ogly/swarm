#!/usr/bin/env python3
"""Exercise preparation AI via public Rust CLI/HTTP with an owned fake endpoint.

No paid provider, live mission mutation, global watchdog or retry of a failed
scenario. Polling observes terminal state; a missing progression is diagnosed
by recorded worker/kernel/SQL evidence, never manufactured PASS.
"""
import argparse,json,threading,time,sqlite3,subprocess,os,signal,select
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
import rust_agent_model_route as fixture
from rust_project_memory_source import seed_lesson, seed_work
import rust_native_http_cli as native
from rust_preparation_plan import task as plan_task,plan as plan_fixture


def main():
    """Verify send/replay/budget/answer/stop with retained named assertions."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');ap.add_argument('--browser',action='store_true');ap.add_argument('--expect-crash-open',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'preparation-calls');checks=[];received=[];web=None;mode={'value':'json'};entered=threading.Event();release=threading.Event()
    def check(name,value):
        """Publish each assertion as it completes, preserving original failures."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    if a.expect_unported:
        rc,error=f.call(['prepare','capabilities']);check('preparation-calls-baseline-unported',rc!=0);return
    def change_settings(**updates):
        """Use public CLI before server start, then the active Admin HTTP path.

        Do not race an external settings writer with the owned server's readers.
        Preview stays readonly, apply preserves the current revision/history and
        readback proves effective values. No retry, SQL edit or ledger reset.
        """
        if web is None:return f.settings_change(**updates)
        session=web.request('/api/v1/native-session',observe=False)
        doc=web.request('/api/v1/native-settings',observe=False);values=dict(doc['values']);values.update(updates)
        request=dict(schema_version=doc['schema_version'],event_id='call-settings-'+str(len(web.requests)),expected_revision=doc['revision'],values=values,reason='Owned active Admin path fixture',preview_token='')
        before=(f.root/'.swarm/engine-settings.json').read_bytes()
        preview=web.request('/api/v1/native-settings/preview',request,csrf=session['csrf_token'],observe=False)
        assert (f.root/'.swarm/engine-settings.json').read_bytes()==before
        request['preview_token']=preview['preview_token']
        web.request('/api/v1/native-settings/apply',request,csrf=session['csrf_token'],observe=False)
        assert web.request('/api/v1/native-settings',observe=False)['values']==values
    class Handler(BaseHTTPRequestHandler):
        """Deterministic text provider, fixture credentials never a real account."""
        def log_message(self,*args):pass
        def do_POST(self):
            raw=self.rfile.read(int(self.headers['Content-Length']));request=json.loads(raw);received.append({'path':self.path,'authorization_ok':self.headers.get('Authorization')=='Bearer PRIVATE_PREPARATION_FIXTURE','request':request});fixture.publish(out/'provider-requests.json',received);entered.set()
            if mode['value']=='wait':release.wait()
            if mode['value']=='tool':value={'model':'fixed-model','choices':[{'message':{'content':'unsafe','tool_calls':[]},'finish_reason':'stop'}]}
            elif mode['value']=='plan':value={'model':'fixed-model','choices':[{'message':{'content':json.dumps({'message':'Plan proposal','plan':json.dumps({**plan_fixture(),'tasks':[plan_task('t1'),plan_task('t2',['t1'])],'questions':[dict(question='Keep decision?',answer='MODEL_FORGED'),dict(question='New decision?',answer='')]})})},'finish_reason':'stop'}]}
            else:value={'model':'fixed-model','choices':[{'message':{'content':json.dumps({'message':'Fixture response é','brief':'Proposed brief'})},'finish_reason':'stop'}],'usage':{'prompt_tokens':7,'completion_tokens':11}}
            try:
                if mode['value']=='sse':
                    content=value['choices'][0]['message']['content'];wire=('data: '+json.dumps({'model':'fixed-model','choices':[{'index':0,'delta':{'reasoning_content':'PRIVATE_REASONING','content':content},'finish_reason':None}]})+'\n\ndata: '+json.dumps({'choices':[{'index':0,'delta':{},'finish_reason':'stop'}],'usage':value['usage']})+'\n\ndata: [DONE]\n\n').encode();self.send_response(200);self.send_header('Content-Type','text/event-stream')
                else:wire=json.dumps(value).encode();self.send_response(200);self.send_header('Content-Type','application/json')
                self.send_header('Content-Length',str(len(wire)));self.end_headers();self.wfile.write(wire);self.wfile.flush()
            except (BrokenPipeError,ConnectionResetError):pass # Expected owned cancellation transport outcome.
    server=ThreadingHTTPServer(('127.0.0.1',0),Handler);server.daemon_threads=True;thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    try:
        f.environment.pop('SWARM_PREPARATION_DISABLED',None)
        change_settings(preparation_write_enabled='1',preparation_call_enabled='1')
        current=f.ok(['providers','connections','show']);change={'version':1,'expected_digest':current['digest'],'connection':{'id':'local','label':'Owned text API','base_url':'http://127.0.0.1:'+str(server.server_port)+'/v1','model':'fixed-model','key':'PRIVATE_PREPARATION_FIXTURE','disabled':False},'replace_key':True,'preview':True};preview=f.ok(['providers','connections','preview'],change);f.ok(['providers','connections','apply'],{**change,'preview':False,'preview_token':preview['preview_token']})
        p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='create-preparation',expected_revision=0,title='Owned AI preparation',text='Need fixture'))
        p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget-preparation',expected_revision=p['revision'],budget=dict(limit_usd=2,reserve_per_launch_usd=0.5,estimate_source='Synthetic fixture',reference_date='2026-10-27')))
        def send(event):
            """Bind each explicit call to freshly read saved capability metadata."""
            caps=f.ok(['prepare','capabilities']);c=next(c for c in caps['providers'] if c['name']=='api-local');return dict(version=1,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],provider=c['name'],capability_hash=c['digest'],message='Propose a brief',context_mode='full')
        def terminal(id):
            """Observe without arbitrary global timeout; preserve state each poll."""
            previous=None
            while True:
                turns=f.ok(['prepare','dialogue',p['id']]);turn=next(t for t in turns if t['id']==id)
                if turn['status']!=previous:print('STATE '+id+' '+turn['status'],flush=True);previous=turn['status']
                if turn['status'] not in ['pending','running','stopping']:return turn
                time.sleep(0.05) # Fixture observation cadence, no product cutoff.
        request=send('json-call');t=f.ok(['prepare','send',p['id']],request);check('send-recorded-pending',t['id'].startswith('pt-') and t['status']=='pending');t=terminal(t['id']);check('json-answer-recorded',t['status']=='answered' and t['answer']['brief']=='Proposed brief');check('usage-declared-preserved',t['usage']['input_tokens']==7 and t['usage']['output_tokens']==11 and 'provider_reported_cost_usd' not in t['usage']);check('no-tools-or-max-tokens-in-request',len(received)==1 and set(received[0]['request'])=={'model','messages','stream'} and received[0]['authorization_ok'] and received[0]['path']=='/v1/chat/completions');check('no-private-provider-state-published','prompt' not in t and '_native' not in t and 'PRIVATE_PREPARATION_FIXTURE' not in json.dumps(t));check('documents-not-automatically-adopted',f.ok(['prepare','show',p['id']])['documents'].get('brief') is None)
        invalid={**send('api-explicit-level'),'level':'standard'};count_before=len(received)
        rc,invalid_result=f.call(['prepare','send',p['id']],invalid)
        check('api-explicit-level-refuses-without-contact',rc!=0 and invalid_result['failure']['code']=='preparation-send-invalid' and len(received)==count_before)
        replay=f.ok(['prepare','send',p['id']],request);check('exact-replay-no-second-contact',replay['id']==t['id'] and len(received)==1);budget=f.ok(['prepare','budget',p['id']]);check('estimated-reservation-retained',budget['committed_estimate_usd']==0.5 and budget['reserved_usd']==0)
        adopt=dict(version=1,action='use-proposal',preparation_id=p['id'],event_id='use-proposal',expected_revision=p['revision'],turn_id=t['id']);p=f.ok(['prepare','use-proposal',p['id']],adopt);check('proposal-explicit-cas-save',p['documents']['brief']['source_turn']==t['id'] and p['documents']['brief']['text']=='Proposed brief' and not p.get('adopted_brief'));check('proposal-replay-no-history-rewrite',f.ok(['prepare','use-proposal',p['id']],adopt)==p)
        mode['value']='tool';t=f.ok(['prepare','send',p['id']],send('tool-call'));t=terminal(t['id']);check('tool-declaration-refuses-answer',t['status']=='failed' and not t.get('answer') and 'api-tool-declaration-refused' in t.get('error',''));check('failed-attempt-not-refunded',f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==1)
        change_settings(preparation_call_transport='sse');mode['value']='sse'
        web=native.Server(f.binary,f.root,out,'calls-http',environment=f.environment);web.capture_async(out/'http-observer.raw');csrf=web.request('/api/v1/native-session',observe=False)['csrf_token']
        check('http-capabilities-equal-cli',web.request('/api/v1/preparations/capabilities',observe=False)==f.ok(['prepare','capabilities']))
        request=send('sse-call');count=len(received)
        web.request('/api/v1/preparations/send',request,status=403,observe=False)
        check('http-csrf-refuses-before-contact',len(received)==count)
        t=web.request('/api/v1/preparations/send',request,csrf=csrf,observe=False);t=terminal(t['id']);check('sse-answer-and-no-reasoning',t['status']=='answered' and t['answer']['message']=='Fixture response é' and 'PRIVATE_REASONING' not in json.dumps(t))
        mode['value']='wait';entered.clear();request=send('stop-call');t=f.ok(['prepare','send',p['id']],request);entered.wait();stopped=web.request('/api/v1/preparations/stop',dict(preparation_id=p['id'],turn_id=t['id']),csrf=csrf,observe=False);check('stop-request-recorded',stopped['status'] in ['stopping','interrupted']);t=terminal(t['id']);check('stop-settled-without-answer',t['status'] in ['failed','interrupted'] and not t.get('answer'));release.set();check('stop-attempt-not-refunded',f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==2)
        count=len(received);rc,error=f.call(['prepare','send',p['id']],send('over-budget'));check('budget-refuses-before-provider',rc!=0 and error['failure']['code']=='preparation-budget-exhausted' and len(received)==count)
        # A separate owned preparation shares the Work envelope. Reconfiguration
        # preserves already committed consumption; it never resets a ledger.
        change_settings(preparation_call_transport='json');mode['value']='json'
        linked=f.ok(['prepare','create'],dict(version=1,action='create',event_id='linked-preparation',expected_revision=0,title='Linked budget fixture',text='Need linked fixture',work_id=f.work))
        linked=f.ok(['prepare','budget',linked['id']],dict(version=1,action='budget',preparation_id=linked['id'],event_id='linked-budget',expected_revision=linked['revision'],budget=dict(limit_usd=2,reserve_per_launch_usd=0.5,estimate_source='Synthetic linked fixture',reference_date='2026-10-27')))
        def linked_request(event):
            """Fresh explicit API source with the linked preparation's own CAS."""
            c=next(c for c in f.ok(['prepare','capabilities'])['providers'] if c['name']=='api-local');return dict(version=1,preparation_id=linked['id'],event_id=event,expected_revision=linked['revision'],provider=c['name'],capability_hash=c['digest'],message='Shared budget fixture')
        original=p;p=linked
        t=f.ok(['prepare','send',p['id']],linked_request('linked-first'));t=terminal(t['id']);check('linked-work-answer',t['status']=='answered');view=f.ok(['prepare','budget',p['id']]);check('linked-work-and-prep-estimates',view['committed_estimate_usd']==0.5 and view['work_budget']['committed_estimate_usd']==1)
        def work_limit(limit,event):
            """Adjust only the owned envelope through the supported public CAS."""
            work=f.ok(['work','show',f.work])['work'];f.ok(['budget','apply',f.work],dict(schema_version=1,event_id=event,expected_revision=work['revision'],budget=dict(limit_usd=limit,reserve_per_launch_usd=1,estimate_source='Owned shared budget fixture',reference_date='2026-10-27')))
        work_limit(1,'limit-shared');request=linked_request('linked-recovery');count=len(received);rc,error=f.call(['prepare','send',p['id']],request);check('work-envelope-refuses-before-contact',rc!=0 and error['failure']['code']=='work-budget-exhausted' and len(received)==count);check('work-refusal-no-turn-or-prep-debit',len(f.ok(['prepare','dialogue',p['id']]))==1 and f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==0.5)
        work_limit(10,'restore-shared');check('work-envelope-change-keeps-consumption',f.ok(['prepare','budget',p['id']])['work_budget']['committed_estimate_usd']==1);t=f.ok(['prepare','send',p['id']],request);t=terminal(t['id']);check('work-envelope-public-recovery',t['status']=='answered' and f.ok(['prepare','budget',p['id']])['work_budget']['committed_estimate_usd']==2);p=original
        # Kill only the actual owned worker through pidfd after matching its
        # kernel start stamp to the public registration. No raw-PID signal.
        mode['value']='wait';entered.clear();release.clear();p=linked
        t=web.request('/api/v1/preparations/send',linked_request('owned-worker-crash'),csrf=csrf,observe=False);entered.wait();observed=next(tn for tn in f.ok(['prepare','dialogue',p['id']]) if tn['id']==t['id']);pid=observed['supervisor_pid']
        def stamp():
            """Read actual Linux starttime on the owned fixture PID, no fallback."""
            return Path('/proc/'+str(pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
        assert stamp()==observed['supervisor_stamp'];fd=os.pidfd_open(pid)
        try:
            assert stamp()==observed['supervisor_stamp'];signal.pidfd_send_signal(fd,signal.SIGKILL);poll=select.poll();poll.register(fd,select.POLLIN);poll.poll()
        finally:os.close(fd)
        if a.expect_crash_open:
            dead=next(tn for tn in f.ok(['prepare','dialogue',p['id']]) if tn['id']==t['id']);check('baseline-dead-worker-still-active',dead['status']=='running');web.request('/api/v1/preparations/stop',dict(preparation_id=p['id'],turn_id=t['id']),csrf=csrf,observe=False)
        t=terminal(t['id']);check('owned-worker-crash-settled',t['status']=='interrupted' and not t.get('answer') and (a.expect_crash_open or t.get('error')=='preparation-worker-exited-without-final-state'));release.set();view=f.ok(['prepare','budget',p['id']]);check('crash-does-not-refund-attempt',view['committed_estimate_usd']==1.5 and view['work_budget']['committed_estimate_usd']==3);p=original
        plan_p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='plan-preparation',expected_revision=0,title='Plan proposal fixture',text='Need plan fixture'))
        plan_p=f.ok(['prepare','save',plan_p['id']],dict(version=1,action='save',preparation_id=plan_p['id'],event_id='plan-brief',expected_revision=plan_p['revision'],document='brief',text='Adopted plan brief'))
        plan_p=f.ok(['prepare','adopt-brief',plan_p['id']],dict(version=1,action='adopt-brief',preparation_id=plan_p['id'],event_id='plan-adopt',expected_revision=plan_p['revision'],sha256=plan_p['documents']['brief']['sha256']))
        prior={**plan_fixture(),'tasks':[plan_task('t1'),plan_task('t2',['t1'])],'questions':[dict(question='Keep decision?',answer='OPERATOR_ONLY'),dict(question='New decision?',answer='')]}
        plan_p=f.ok(['prepare','save',plan_p['id']],dict(version=1,action='save',preparation_id=plan_p['id'],event_id='prior-plan',expected_revision=plan_p['revision'],document='plan',text=json.dumps(prior)))
        p=plan_p;mode['value']='plan';before_plan=f.snapshot();t=f.ok(['prepare','send',p['id']],{**send('plan-proposal'),'target':'plan'});t=terminal(t['id']);proposal=json.loads(t['answer']['plan']);check('plan-proposal-counts-and-no-model-decision',t['status']=='answered' and t['plan_summary']==dict(missions=2,dependencies=1,open_questions=2) and all(not q['answer'] for q in proposal['questions']))
        p=f.ok(['prepare','use-proposal',p['id']],dict(version=1,action='use-proposal',preparation_id=p['id'],event_id='use-plan',expected_revision=p['revision'],turn_id=t['id']));saved_plan=json.loads(p['documents']['plan']['text']);check('plan-keeps-only-known-operator-answer',saved_plan['questions'][0]['answer']=='OPERATOR_ONLY' and saved_plan['questions'][1]['answer']=='' and p['documents']['plan']['source_turn']==t['id'] and not p['plan_ready']);check('plan-proposal-no-mission-launch',f.snapshot()==before_plan and not p.get('work_id') and not p.get('conversion'));p=original
        # Retain the existing versioned-memory guard through the new API
        # caller. A positive control precedes the source fault and recovery.
        memory=f.ok(['prepare','create'],dict(version=1,action='create',event_id='memory-preparation',expected_revision=0,title='Memory guard fixture',text='Need memory fixture'));p=memory;mode['value']='json'
        t=f.ok(['prepare','send',p['id']],send('memory-control'));t=terminal(t['id']);check('project-context-positive-control',t['status']=='answered')
        profile=f.root/'swarm.project.json';profile_before=profile.read_bytes() if profile.exists() else None;guide=f.root/'.claude/field-guide';guide.mkdir(parents=True);memory_work_before=seed_lesson(f.root,f.work);fixture.publish(guide/'index.md','[Managed source](g.md)');fixture.publish(profile,dict(version=1,name='Owned memory guard',source='custom',instructions=[dict(path='.claude/field-guide/index.md',roles=['preparation'])]));count=len(received);rows=len(f.ok(['prepare','dialogue',p['id']]))
        blocked=web.request('/api/v1/preparations/send',send('memory-guard'),csrf=csrf,status=500,observe=False);check('memory-guard-safe-no-contact',blocked['failure'].get('reason')=='project-source-memory-managed' and 'causes' not in blocked['failure'] and len(received)==count and len(f.ok(['prepare','dialogue',p['id']]))==rows)
        # Restore only this owned fixture's synthetic memory setup, not budgets.
        seed_work(f.root,f.work,memory_work_before)
        if profile_before is None:profile.unlink()
        else:fixture.publish(profile,profile_before)
        t=f.ok(['prepare','send',p['id']],send('memory-recovery'));t=terminal(t['id']);check('project-context-public-recovery',t['status']=='answered');p=original
        if a.browser:
            change_settings(preparation_call_transport='json');mode['value']='json'
            browser_p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='browser-preparation',expected_revision=0,title='Browser preparation',text='Need browser fixture'))
            before_browser=len(received);browser_out=out/'browser';browser_out.mkdir();cfg=dict(base='http://'+web.address,cookie=web.cookie,preparation=browser_p,output=str(browser_out))
            with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_calls_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
            print((out/'browser.stdout').read_text(),end='',flush=True);check('browser-public-journey',r.returncode==0 and len(received)-before_browser==5)
        fixture.publish(out/'result.json',dict(passed=True,assertions=checks,provider_calls=len(received),binary_sha256=fixture.digest(f.binary.read_bytes()),root=str(f.root),limits='Owned fake HTTP only; no TLS/account/authenticity or global migration acceptance.'));print('PREPARATION_CALLS_PASS '+str(len(checks)),flush=True)
    finally:
        release.set();server.shutdown();server.server_close();thread.join()
        if web is not None:native.finish(web,None)
if __name__=='__main__':main()
