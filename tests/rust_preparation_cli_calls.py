#!/usr/bin/env python3
"""Verify preparation Claude/Codex subprocesses using inert owned Python doubles.

Uses the public CLI for policy, budget, send/stop/proposal and readonly dialogue.
No real CLI/account/model, live mission DB, global watchdog or implicit retry.
Retains exact requests, outputs, fixture source and process observations.
"""
import argparse, json, os, time, subprocess, sqlite3
import rust_native_http_cli as native
from pathlib import Path
import rust_agent_model_route as fixture

PROGRAM = r'''#!/usr/bin/python3
import json,sys,os,time,hashlib
from pathlib import Path
base=Path(__BASE__)
raw=sys.stdin.buffer.read();mode=(base/'mode').read_text()
record={'pid':os.getpid(),'ppid':os.getppid(),'argv':sys.argv[1:],'stdin_sha256':hashlib.sha256(raw).hexdigest(),'stdin_bytes':len(raw),'cwd':os.getcwd(),'environment_names':sorted(os.environ)}
# Publish the witness only after its JSON is complete. The parent's existence
# observation must not race an open-but-empty file; no timing retry is needed.
pending=base/('pending-entered-'+str(os.getpid())+'.json')
with pending.open('x') as out:
 json.dump(record,out);out.flush();os.fsync(out.fileno())
os.link(pending,base/('entered-'+str(os.getpid())+'.json'))
pending.unlink()
if mode=='wait':
 while not (base/'release').exists():time.sleep(.02)
if mode=='tool':print(json.dumps({'type':'assistant','message':{'content':[{'type':'tool_use','id':'fixture-tool','name':'Read','input':{}}]}}),flush=True)
if mode=='error':print(json.dumps({'type':'result','is_error':True,'result':'PRIVATE_DIAGNOSTIC'}),flush=True)
elif mode=='incomplete':print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'premature'}}),flush=True)
elif mode=='overflow':print('x'*4096,flush=True)
else:
 content=json.dumps({'message':'Owned CLI answer','brief':'Owned CLI brief'})
 if Path(sys.argv[0]).name=='claude':print(json.dumps({'type':'result','structured_output':json.loads(content),'usage':{'input_tokens':7,'output_tokens':8}}),flush=True)
 else:
  print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':content}}),flush=True)
  print(json.dumps({'type':'turn.completed','usage':{'input_tokens':7,'output_tokens':8}}),flush=True)
'''

def main():
    """Assert real owned helper execution, refusals, budget and explicit recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');ap.add_argument('--browser',action='store_true');ap.add_argument('--expect-tool-defect',action='store_true');ap.add_argument('--expect-levels-unported',action='store_true');ap.add_argument('--older-binary',default=os.environ.get('SWARM_PREPARATION_OLD_BINARY'));a=ap.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
    # The optional older-worker wire test requires the older journal format it
    # actually supports. A separate normal invocation covers fresh current Roots.
    # Birth/command records retain both executable identities; no live migration.
    initial_binary=Path(a.older_binary or a.binary).resolve()
    f=fixture.Fixture(initial_binary,out,'preparation-cli');checks=[]
    if a.older_binary:
        assert f.ok(['engine-settings','show'])['schema_version']<=46,'Older-worker witness requires its compatible saved journal'
        f.binary=Path(a.binary).resolve()
    def check(name,value):
        """Retain each named result immediately; failure preserves originals."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    f.environment.pop('SWARM_PREPARATION_DISABLED',None)
    f.settings_change(preparation_write_enabled='1',preparation_call_enabled='1',agent_allow_unverified_provider_context='1',agent_environment_allow=['PATH','HOME','CODEX_HOME'])
    fixture.publish(f.root/'mode',b'answer')
    code=PROGRAM.replace('__BASE__',repr(str(f.root)))
    for name in ['codex','claude']:
        fixture.publish(f.root/name,code.encode());os.chmod(f.root/name,0o700)
    f.provider['args']=['--model','fixture-standard']
    f.provider['model_policy']['levels']['standard']['effort']=''
    f.save_provider()
    if a.expect_unported:
        caps=f.ok(['prepare','capabilities']);check('baseline-cli-adapter-unavailable',all(c['name']!='fixture' for c in caps['providers']));return
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='create-cli',expected_revision=0,title='Owned CLI preparation',text='CLI fixture need'))
    p=f.ok(['prepare','budget',p['id']],dict(version=1,action='budget',preparation_id=p['id'],event_id='budget-cli',expected_revision=p['revision'],budget=dict(limit_usd=10,reserve_per_launch_usd=.5,estimate_source='Synthetic fixture',reference_date='2026-10-27')))
    def send(event,level="auto"):
        """Bind each explicit send to its current saved provider capability."""
        caps=f.ok(['prepare','capabilities']);c=next(c for c in caps['providers'] if c['name']=='fixture')
        if level=='auto':digest=c['digest'];policy_hash=''
        else:
            mode=next(m for m in c['levels'] if m['level']==level);assert mode['available'];digest=mode['digest'];policy_hash=mode['policy_hash']
        return dict(version=1,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],provider='fixture',capability_hash=digest,model_policy_hash=policy_hash,level=level,message='Propose brief',context_mode='full')
    def terminal(id):
        """Observe real terminal state; no inherited whole-test cutoff."""
        previous=None
        while True:
            t=next(t for t in f.ok(['prepare','dialogue',p['id']]) if t['id']==id)
            fixture.publish(out/'last-turn.json',t)
            if t['status'] not in ['pending','running','stopping']:return t
            if t['status']!=previous:print('OBSERVE '+t['status'],flush=True);previous=t['status']
            time.sleep(.05)
    request=send('codex-success');t=f.ok(['prepare','send',p['id']],request);t=terminal(t['id'])
    check('codex-real-helper-answer',t['status']=='answered' and t['answer']['brief']=='Owned CLI brief')
    check('declared-usage-not-invoice',t['usage']['input_tokens']==7 and t['usage']['output_tokens']==8)
    check('requested-route-persisted',t['model_route']['model']=='fixture-standard' and t['model_route']['level']=='standard')
    records=list(f.root.glob('entered-*.json'));record=json.loads(records[-1].read_text())
    check('stdin-prompt-not-argv',record['stdin_bytes']>0 and 'Propose brief' not in json.dumps(record['argv']))
    check('codex-tool-free-request',record['argv'][:2]==['exec','--json'] and '--sandbox' in record['argv'] and '--disable' in record['argv'])
    check('private-workspace-removed',not Path(record['cwd']).exists())
    check('no-native-prompt-published','prompt' not in t and '_native' not in t and 'PRIVATE_' not in json.dumps(t))
    replay=f.ok(['prepare','send',p['id']],request)
    check('exact-replay-no-reexecution',replay['id']==t['id'] and len(list(f.root.glob('entered-*.json')))==1)
    check('attempt-estimate-kept',f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==.5)
    if a.expect_tool_defect:
        fixture.publish(f.root/'mode',b'tool')
        observed=terminal(f.ok(['prepare','send',p['id']],send('tool-declaration-baseline'))['id'])
        assert observed['status']=='answered' and observed.get('answer'),observed
        fixture.publish(out/'observed-defect.json',dict(code='CLI_TOOL_DECLARATION_ACCEPTED',status='DEFECT_OBSERVED',turn=observed))
        print('DEFECT_OBSERVED CLI_TOOL_DECLARATION_ACCEPTED',flush=True);return
    if a.expect_levels_unported:
        request={**send('explicit-level-baseline'), 'level':'standard'}
        count=len(list(f.root.glob('entered-*.json')))
        rc,error=f.call(['prepare','send',p['id']],request)
        assert rc!=0 and error['failure']['code']=='preparation-send-invalid',error
        assert len(list(f.root.glob('entered-*.json')))==count
        fixture.publish(out/'observed-gap.json',dict(status='GAP_OBSERVED',code='CLI_EXPLICIT_LEVEL_UNPORTED',error=error))
        print('GAP_OBSERVED CLI_EXPLICIT_LEVEL_UNPORTED',flush=True);return
    def private_shape(id):
        """Readonly inert SQL observation; return only schema/level, not prompt/gate."""
        db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:body=json.loads(db.execute('SELECT body FROM preparation_turns WHERE id=?',(id,)).fetchone()[0])
        finally:db.close()
        n=body['_native'];return n['version'],n.get('request_level')
    check('auto-retains-private-v1',private_shape(t['id'])==(1,None))
    explicit_request=send('explicit-standard','standard')
    explicit=terminal(f.ok(['prepare','send',p['id']],explicit_request)['id'])
    check('explicit-level-reaches-worker',explicit['status']=='answered' and explicit['model_route']['level']=='standard' and explicit['model_route']['model']=='fixture-standard')
    check('explicit-level-private-v2',private_shape(explicit['id'])==(2,'standard'))
    if a.older_binary:
        old_env=f.environment
        def old_replay(body):
            """Same inert Root, exact earlier intent; no process/model dispatch expected."""
            run=subprocess.run([str(Path(a.older_binary).resolve()),'prepare','send',p['id'],'--root',str(f.root),'--json','--input','-'],input=json.dumps(body).encode(),capture_output=True,env=old_env)
            fixture.publish(out/('older-'+body['event_id']+'.stdout'),run.stdout);fixture.publish(out/('older-'+body['event_id']+'.stderr'),run.stderr)
            return run
        # Valid v1 witness uses the SAME Root containing the v2 row before refusal.
        valid=old_replay(request);check('older-v1-replay-still-readable',valid.returncode==0 and json.loads(valid.stdout)['id']==t['id'])
        before=len(list(f.root.glob('entered-*.json')));failed=old_replay(explicit_request)
        check('older-v2-replay-refuses-without-change',failed.returncode!=0 and json.loads(failed.stderr)['failure']['code']=='preparation-dialogue-row-identity-mismatch' and len(list(f.root.glob('entered-*.json')))==before and f.ok(['prepare','dialogue',p['id']])[-1]['status']=='answered')
    proposal=dict(version=1,action='use-proposal',preparation_id=p['id'],event_id='use-cli',expected_revision=p['revision'],turn_id=t['id'])
    p=f.ok(['prepare','use-proposal',p['id']],proposal)
    check('proposal-explicit-not-adopted',p['documents']['brief']['text']=='Owned CLI brief' and not p.get('adopted_brief'))
    f.provider['command']=str(f.root/'claude');f.save_provider()
    t=terminal(f.ok(['prepare','send',p['id']],send('claude-success'))['id'])
    check('claude-structured-reply',t['status']=='answered' and t['answer']['brief']=='Owned CLI brief')
    for mode in ['tool','error','incomplete','overflow']:
        fixture.publish(f.root/'mode',mode.encode())
        if mode=='overflow':f.settings_change(preparation_call_response_max_bytes='2048')
        t=terminal(f.ok(['prepare','send',p['id']],send('reject-'+mode))['id'])
        check(mode+'-refuses-answer',t['status']=='failed' and not t.get('answer') and 'PRIVATE_DIAGNOSTIC' not in json.dumps(t))
    fixture.publish(f.root/'mode',b'wait');before=set(f.root.glob('entered-*.json'))
    t=f.ok(['prepare','send',p['id']],send('stop-cli'))
    while not (set(f.root.glob('entered-*.json'))-before):time.sleep(.02)
    entered=next(iter(set(f.root.glob('entered-*.json'))-before));record=json.loads(entered.read_text())
    f.ok(['prepare','stop',p['id']],dict(preparation_id=p['id'],turn_id=t['id']))
    t=terminal(t['id']);check('explicit-stop-retained',t['status']=='interrupted' and not t.get('answer'))
    while Path('/proc/'+str(record['pid'])).exists():time.sleep(.02)
    check('owned-cli-child-reaped',not Path('/proc/'+str(record['pid'])).exists())
    check('stopped-workspace-removed',not Path(record['cwd']).exists())
    fixture.publish(f.root/'mode',b'answer')
    t=terminal(f.ok(['prepare','send',p['id']],send('recovery'))['id'])
    check('new-explicit-turn-recovers',t['status']=='answered')
    check('attempted-failures-stop-not-refunded',f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==4.5)
    stale=send('stale-executable');before=len(list(f.root.glob('entered-*.json')))
    command=Path(f.provider['command']);original=command.read_bytes();fixture.publish(command,original+b'\n# changed fixture\n');command.chmod(0o700)
    rc,value=f.call(['prepare','send',p['id']],stale)
    check('executable-stale-refuses-before-launch',rc!=0 and len(list(f.root.glob('entered-*.json')))==before)
    fixture.publish(command,original);command.chmod(0o700)
    if a.browser:
        web=native.Server(f.binary,f.root,out,'cli-browser-http',environment=f.environment)
        # Browser requests own the observer: drain concurrently, preserving IO
        # errors and avoiding stdout backpressure as an accidental test boundary.
        web.capture_async(out/'browser-observer.raw');primary=None
        try:
            browser_out=out/'browser';browser_out.mkdir()
            cfg=dict(base='http://'+web.address,cookie=web.cookie,preparation=p,output=str(browser_out),model='fixture-standard',message='Owned CLI answer',brief='Owned CLI brief',explicit_level='standard')
            before=len(list(f.root.glob('entered-*.json')))
            with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:
                run=subprocess.run(['node','tests/rust_preparation_calls_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
            print((out/'browser.stdout').read_text(),end='',flush=True)
            check('cli-browser-served-journey',run.returncode==0 and len(list(f.root.glob('entered-*.json')))-before==5)
        except BaseException as error:primary=error
        finally:native.finish(web,primary)
    p=f.ok(['prepare','show',p['id']])
    # Executable bytes are an inert fixture: no shebang/ELF, no shell fallback.
    fixture.publish(command,b'PRIVATE_INVALID_EXECUTABLE\n');command.chmod(0o700)
    t=terminal(f.ok(['prepare','send',p['id']],send('preexec-failure'))['id'])
    check('preexec-cause-private-channel',t['status']=='failed' and 'preparation-cli-helper' in t.get('error','') and 'PRIVATE_INVALID_EXECUTABLE' not in json.dumps(t))
    fixture.publish(command,original);command.chmod(0o700)
    t=terminal(f.ok(['prepare','send',p['id']],send('preexec-recovery'))['id'])
    check('explicit-preexec-recovery',t['status']=='answered')
    f.provider['model_policy']['levels']['standard']['model']='fixture-not-announced';f.save_provider()
    caps=f.ok(['prepare','capabilities'])
    check('unavailable-route-explicit',not caps['providers'] and any(c['name']=='fixture' and c['reason']=='agent-model-choice-not-announced' for c in caps.get('unavailable_providers',[])))
    f.provider['model_policy']['levels']['standard']['model']='fixture-standard';f.save_provider()
    f.settings_change(agent_allow_unverified_provider_context='0')
    caps=f.ok(['prepare','capabilities']);check('operator-unverified-grant-required',all(c['name']!='fixture' for c in caps['providers']))
    fixture.publish(out/'result.json',dict(passed=True,assertions=checks,binary_sha256=fixture.digest(f.binary.read_bytes()),root=str(f.root),scope='Owned fake CLI only; requested restrictions, not external CLI confinement/account proof'))
    print('PREPARATION_CLI_PASS '+str(len(checks)),flush=True)
if __name__=='__main__':main()
