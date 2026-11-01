#!/usr/bin/env python3
"""Verify preparation Claude/Codex subprocesses using inert owned Python doubles.

Uses the public CLI for policy, budget, send/stop/proposal and readonly dialogue.
No real CLI/account/model, live mission DB, global watchdog or implicit retry.
Retains exact requests, outputs, fixture source and process observations.
"""
import argparse, json, os, time, subprocess, sqlite3, hashlib
import rust_native_http_cli as native
from pathlib import Path
import rust_agent_model_route as fixture

PROGRAM = r'''#!/usr/bin/python3
import json,sys,os,time,hashlib
from pathlib import Path
base=Path(__BASE__)
raw=sys.stdin.buffer.read();mode=(base/'mode').read_text()
# The Biba boundary leaves the provider the workspace (cwd, I0) and reads the
# Root; the Root itself is never writable to the provider. The execution
# witness therefore rides the structured answer the engine already captures,
# inside message (the strict native answer allows only message/brief/plan);
# no file outside the workspace is written.
record={'pid':os.getpid(),'ppid':os.getppid(),'argv':sys.argv[1:],'stdin_sha256':hashlib.sha256(raw).hexdigest(),'stdin_bytes':len(raw),'cwd':os.getcwd(),'environment_names':sorted(os.environ)}
if mode=='wait':
 while not (base/'release').exists():time.sleep(.02)
if mode=='tool':print(json.dumps({'type':'assistant','message':{'content':[{'type':'tool_use','id':'fixture-tool','name':'Read','input':{}}]}}),flush=True)
if mode=='error':print(json.dumps({'type':'result','is_error':True,'result':'PRIVATE_DIAGNOSTIC'}),flush=True)
elif mode=='incomplete':print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'premature'}}),flush=True)
elif mode=='overflow':print('x'*4096,flush=True)
else:
 content=json.dumps({'message':'Owned CLI answer\n'+json.dumps(record),'brief':'Owned CLI brief'})
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
    # Execution witness rides the answer message (second line, strict JSON):
    # the enforcing Biba boundary gives the provider only the private workspace
    # (cwd, I0) and read-only Root, so no outside file can be written.
    def witness(answer):
        """Parse the double's answer-embedded record; strict one-line JSON."""
        return json.loads(answer['message'].splitlines()[-1])
    record=witness(t['answer'])
    check('stdin-prompt-not-argv',record['stdin_bytes']>0 and 'Propose brief' not in json.dumps(record['argv']))
    check('codex-tool-free-request',record['argv'][:2]==['exec','--json'] and '--sandbox' in record['argv'] and '--disable' in record['argv'])
    check('private-workspace-removed',not Path(record['cwd']).exists())
    check('no-native-prompt-published','prompt' not in t and '_native' not in t and 'PRIVATE_' not in json.dumps(t))
    replay=f.ok(['prepare','send',p['id']],request)
    check('exact-replay-no-reexecution',replay['id']==t['id'] and replay['answer']['message']==t['answer']['message'])
    check('attempt-estimate-kept',f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==.5)
    if a.expect_tool_defect:
        fixture.publish(f.root/'mode',b'tool')
        observed=terminal(f.ok(['prepare','send',p['id']],send('tool-declaration-baseline'))['id'])
        assert observed['status']=='answered' and observed.get('answer'),observed
        fixture.publish(out/'observed-defect.json',dict(code='CLI_TOOL_DECLARATION_ACCEPTED',status='DEFECT_OBSERVED',turn=observed))
        print('DEFECT_OBSERVED CLI_TOOL_DECLARATION_ACCEPTED',flush=True);return
    if a.expect_levels_unported:
        request={**send('explicit-level-baseline'), 'level':'standard'}
        turns_before=len(f.ok(['prepare','dialogue',p['id']]))
        rc,error=f.call(['prepare','send',p['id']],request)
        assert rc!=0 and error['failure']['code']=='preparation-send-invalid',error
        assert len(f.ok(['prepare','dialogue',p['id']]))==turns_before
        fixture.publish(out/'observed-gap.json',dict(status='GAP_OBSERVED',code='CLI_EXPLICIT_LEVEL_UNPORTED',error=error))
        print('GAP_OBSERVED CLI_EXPLICIT_LEVEL_UNPORTED',flush=True);return
    def private_shape(id):
        """Read an inert fixture row and verify complete producer byte coverage.

        Returns only version, requested level and digest. No prompt/gate material
        enters the evidence projection. Hashes prove consistency, never BLP rights.
        """
        db=sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:body=json.loads(db.execute('SELECT body FROM preparation_turns WHERE id=?',(id,)).fetchone()[0])
        finally:db.close()
        n=body['_native'];m=n['prompt_provenance'];raw=body['prompt'].encode()
        assert m['schema_version']==1 and m['prompt_bytes']==len(raw)
        assert m['prompt_sha256']==hashlib.sha256(raw).hexdigest()
        cursor=0;fields=[]
        for part in m['parts']:
            assert part['start']==cursor and part['end']>cursor
            observed=raw[cursor:part['end']];observed.decode('utf-8')
            assert hashlib.sha256(observed).hexdigest()==part['sha256']
            if part['origin']['kind']=='preparation_data':fields.append(part['origin']['field'])
            cursor=part['end']
        assert cursor==len(raw)
        assert sorted(fields)==['adopted_brief','documents','earlier_exchanges_not_sent','history','history_mode','method','method_sources','sources','user_message']
        return n['version'],n.get('request_level'),m['prompt_sha256']
    shape=private_shape(t['id'])
    check('auto-private-v3-provenance',shape[:2]==(3,None))
    check('producer-provenance-matches-provider-stdin',shape[2]==record['stdin_sha256'])
    explicit_request=send('explicit-standard','standard')
    explicit=terminal(f.ok(['prepare','send',p['id']],explicit_request)['id'])
    check('explicit-level-reaches-worker',explicit['status']=='answered' and explicit['model_route']['level']=='standard' and explicit['model_route']['model']=='fixture-standard')
    check('explicit-level-private-v3-provenance',private_shape(explicit['id'])[:2]==(3,'standard'))
    if a.older_binary:
        old_env=f.environment
        def old_replay(body):
            """Same inert Root, exact earlier intent; no process/model dispatch expected."""
            run=subprocess.run([str(Path(a.older_binary).resolve()),'prepare','send',p['id'],'--root',str(f.root),'--json','--input','-'],input=json.dumps(body).encode(),capture_output=True,env=old_env)
            fixture.publish(out/('older-'+body['event_id']+'.stdout'),run.stdout);fixture.publish(out/('older-'+body['event_id']+'.stderr'),run.stderr)
            return run
        # Both new routes require v3 provenance; an old worker must refuse
        # this wire version. Legacy v1/v2 readability has separate unit guards.
        for body in [request,explicit_request]:
            failed=old_replay(body)
            check('older-private-v3-refuses-'+body['event_id'],failed.returncode!=0 and json.loads(failed.stderr)['failure']['code']=='preparation-dialogue-row-identity-mismatch')
        check('older-v3-refusal-preserves-terminal-state',f.ok(['prepare','dialogue',p['id']])[-1]['status']=='answered')
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
    # Wait mode holds the provider alive, so start detection scans /proc for a
    # process whose cwd sits under the root's private helper workspace parent
    # (root/preparation-cli/swarm-preparation-cli-*); no file witness exists.
    fixture.publish(f.root/'mode',b'wait')
    t=f.ok(['prepare','send',p['id']],send('stop-cli'))
    def running_provider():
        """Scan /proc cwds under the root's helper workspace; oldest first."""
        found=[]
        for pid in os.listdir('/proc'):
            if not pid.isdigit():continue
            try:cwd=os.readlink('/proc/'+pid+'/cwd')
            except OSError:continue
            if cwd.startswith(str(f.root/'preparation-cli')+'/'):
                try:found.append((int(open('/proc/'+pid+'/stat').read().split(') ',1)[1].split()[19]),int(pid),cwd))
                except OSError:continue
        found.sort();return found
    while not running_provider():time.sleep(.02)
    _,pid,cwd=running_provider()[0]
    f.ok(['prepare','stop',p['id']],dict(preparation_id=p['id'],turn_id=t['id']))
    t=terminal(t['id']);check('explicit-stop-retained',t['status']=='interrupted' and not t.get('answer'))
    while Path('/proc/'+str(pid)).exists():time.sleep(.02)
    check('owned-cli-child-reaped',not Path('/proc/'+str(pid)).exists())
    check('stopped-workspace-removed',not Path(cwd).exists())
    fixture.publish(f.root/'mode',b'answer')
    t=terminal(f.ok(['prepare','send',p['id']],send('recovery'))['id'])
    check('new-explicit-turn-recovers',t['status']=='answered')
    check('attempted-failures-stop-not-refunded',f.ok(['prepare','budget',p['id']])['committed_estimate_usd']==4.5)
    stale=send('stale-executable');before=len(f.ok(['prepare','dialogue',p['id']]))
    command=Path(f.provider['command']);original=command.read_bytes();fixture.publish(command,original+b'\n# changed fixture\n');command.chmod(0o700)
    rc,value=f.call(['prepare','send',p['id']],stale)
    check('executable-stale-refuses-before-launch',rc!=0 and len(f.ok(['prepare','dialogue',p['id']]))==before)
    fixture.publish(command,original);command.chmod(0o700)
    if a.browser:
        web=native.Server(f.binary,f.root,out,'cli-browser-http',environment=f.environment)
        # Browser requests own the observer: drain concurrently, preserving IO
        # errors and avoiding stdout backpressure as an accidental test boundary.
        web.capture_async(out/'browser-observer.raw');primary=None
        try:
            browser_out=out/'browser';browser_out.mkdir()
            cfg=dict(base='http://'+web.address,cookie=web.cookie,preparation=p,output=str(browser_out),model='fixture-standard',message='Owned CLI answer',brief='Owned CLI brief',explicit_level='standard')
            before=len(f.ok(['prepare','dialogue',p['id']]))
            with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:
                run=subprocess.run(['node','tests/rust_preparation_calls_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
            print((out/'browser.stdout').read_text(),end='',flush=True)
            answered=[t for t in f.ok(['prepare','dialogue',p['id']]) if t['status']=='answered' and t.get('answer')]
            check('cli-browser-served-journey',run.returncode==0 and len(answered)-before==5)
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
