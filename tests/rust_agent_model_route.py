#!/usr/bin/env python3
"""Verify requested native model routing with owned CLI/cache/program fixtures.

No real model, Go process, paid provider or live mission is used. The executable
named codex is a local Python fixture; local announcements remain unverified.
No global watchdog, forced acceptance, budget refund or hidden retry is added.
Each CLI request/original output and fixture root is retained for replay. Failure
stays failure; a source/binary hash establishes neither authenticity nor review.
"""
from pathlib import Path
import argparse, hashlib, json, os, sqlite3, subprocess, tempfile, time, traceback


def digest(raw):
    """Bind complete bytes, not a declared model, identity or price."""
    return hashlib.sha256(raw).hexdigest()


def publish(path, raw):
    """Atomically publish fixture/evidence bytes, preserving an original on error.

    Adjacent temporary write/fsync/verification precedes replace and directory
    fsync. Errors propagate; an applied rename followed by sync error is not a
    rollback claim. This helper never authorizes live-state modification.
    """
    path = Path(path)
    if not isinstance(raw, bytes):
        raw = (json.dumps(raw, ensure_ascii=False, indent=2) + '\n').encode()
    fd, name = tempfile.mkstemp(dir=path.parent, prefix='.model-proof-')
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        assert Path(name).read_bytes() == raw
        os.replace(name, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(name):
            os.unlink(name)


PROGRAM = r'''#!/usr/bin/python3
import sys, os, json, hashlib
args=sys.argv[1:]
model=args[args.index('--model')+1]
raw=sys.stdin.buffer.read()
observed={'argv':args,'stdin_sha256':hashlib.sha256(raw).hexdigest(),'stdin_bytes':len(raw),'pid':os.getpid(),'ppid':os.getppid(),'codex_home_present':'CODEX_HOME' in os.environ}
with open('model-received.json','x',encoding='utf-8') as stream:
    json.dump(observed,stream);stream.flush();os.fsync(stream.fileno())
print(json.dumps({'type':'system','subtype':'init','model':model}),flush=True)
print(json.dumps({'type':'assistant','message':{'model':model,'content':[{'type':'text','text':'local model route fixture'}]}}),flush=True)
print(json.dumps({'type':'result','usage':{'input_tokens':7,'output_tokens':8},'total_cost_usd':25}),flush=True)
'''


class Fixture:
    """One owned inactive test root; only public CLI mutates work/state/budget."""
    def __init__(self, binary, output, label):
        """Create an owned inert Root through public commands and retain its birth.

        Accept a string/path binary, read its hash before Root creation, create
        the evidence directory before side effects and publish attribution
        before CLI initialization; partial initialization still leaves its Root
        path/binary/label for recovery. A marker is no actor authentication or
        permission. IO/CLI failures propagate, no cleanup or provider call is
        inferred. An empty temporary directory can remain if birth publication
        itself fails; no Root initialization runs before that write succeeds.
        """
        binary = Path(binary)
        output = Path(output)
        self.binary = binary
        self.output = output
        self.output.mkdir(parents=True, exist_ok=True)
        self.label = label
        binary_sha256 = digest(Path(self.binary).read_bytes())
        self.root = Path(tempfile.mkdtemp(prefix='swarm-model83-'))
        publish(self.output / ('owned-root-'+digest(str(self.root).encode())+'.json'),
                dict(schema_version=1, kind='owned_inert_fixture_birth', root=str(self.root),
                     binary_sha256=binary_sha256, label=label,
                     actor_authenticated=False, runtime_permission=False))
        self.calls = []
        self.expected_prefix=['exec','--json','--safe-fixture']
        self.environment = dict(os.environ)
        self.cache_root = self.root / 'codex-home'
        self.cache_root.mkdir(mode=0o700)
        self.environment['CODEX_HOME'] = str(self.cache_root)
        self.provider = dict(command=str(self.root / 'codex'), args=['exec', '--json', '--model', 'fixture-old', '-c', 'model="fixture-old"', '--config=model_reasoning_effort="old"', '--safe-fixture', '-'], model_policy=dict(version=1, billing='external', work_level='standard', page_level='simple', levels={
            'simple': dict(model='fixture-simple', effort=''),
            'standard': dict(model='fixture-standard', effort='fixture-medium'),
            'exigeant': dict(model='fixture-frontier', effort=''),
        }))
        # All writer FDs close before any executable CLI child can fork/exec.
        publish(self.root / 'codex', PROGRAM.encode())
        os.chmod(self.root / 'codex', 0o700)
        self.cache = self.cache_root / 'models_cache.json'
        self.cache_bytes = json.dumps({'models':[{'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]}]}).encode()
        publish(self.cache, self.cache_bytes)
        self.ok(['init'])
        self.work = self.ok(['work','create'], dict(schema_version=1,event_id='create',expected_revision=0,title='Requested model fixture',objective='Observe route',scope='Owned local program',criteria=['Observed, not accepted']))['work']['id']
        self.ok(['task','add',self.work], dict(schema_version=1,event_id='add',expected_revision=1,id='task',title='Run',deliverable='Evidence',criteria=['Observed, not accepted']))
        self.ok(['budget','apply',self.work], dict(schema_version=1,event_id='budget',expected_revision=2,budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Local model fixture',reference_date='2026-10-19')))
        self.request = dict(schema_version=1,event_id='session',expected_revision=3,task_id='task',provider='fixture',capture_output=True)
        self.providers = self.root / '.swarm/providers.json'
        self.save_provider()
        publish(output / (label+'-fixture.json'), dict(root=str(self.root),work=self.work,binary_sha256=digest(binary.read_bytes()),scope='owned local fixture, no account/model authentication'))

    def save_provider(self):
        """Write fixture configuration atomically; never modify runtime SQL."""
        publish(self.providers, dict(schema_version=1, providers={'fixture':self.provider}))

    def call(self, args, value=None):
        """Invoke exact argv, retain original bytes and return actual exit/result.

        No shell, retry or global deadline is used. Input is a fresh owned file;
        parent env overrides only fixture context, never account tokens.
        """
        number = len(self.calls) + 1
        name = f'{self.label}-{number:03}'
        argv = [str(self.binary), *args, '--root', str(self.root), '--json']
        if value is not None:
            path = self.output / (name+'.input.json')
            assert not path.exists()
            publish(path, value)
            argv += ['--input',str(path)]
        started = time.time()
        result = subprocess.run(argv, env=self.environment, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        publish(self.output / (name+'.stdout'), result.stdout)
        publish(self.output / (name+'.stderr'), result.stderr)
        record = dict(argv=argv,start=started,end=time.time(),exit=result.returncode)
        self.calls.append(record)
        publish(self.output / (self.label+'-commands.json'), self.calls)
        raw = result.stdout if result.returncode == 0 else result.stderr
        return result.returncode, json.loads(raw)

    def ok(self, args, value=None):
        """Require meaningful JSON success, never a refused/empty fallback."""
        code, value = self.call(args, value)
        assert code == 0, (self.label,args,code,value)
        return value

    def prepare(self, preview=False):
        """Exercise the public native intention path, without a process in preview."""
        return self.ok(['agent','preview' if preview else 'prepare',self.work],self.request)

    def reservation(self):
        """Read the isolated receipt only; no direct state manipulation is allowed."""
        db=sqlite3.connect('file:'+str(self.root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:
            return db.execute('SELECT state,amount FROM reservations WHERE agent_id=?',('session',)).fetchone()
        finally:
            db.close()

    def denied(self,args,value,expected):
        """Require a safe explicit refusal, not empty successful output."""
        code,wire=self.call(args,value)
        assert code!=0,(self.label,args,wire)
        assert wire['failure']['code']==expected,(expected,wire)
        return wire

    def settings_change(self,**updates):
        """Publish only through actual preview/apply, preserving preview bytes
        and history. No test resets consumption or rewrites runtime SQL.
        """
        doc=self.ok(['engine-settings','show']);values=doc['values'];values.update(updates)
        request=dict(schema_version=doc['schema_version'],event_id='settings-'+str(len(self.calls)),expected_revision=doc['revision'],values=values,reason='Explicit owned model routing fixture change',preview_token='')
        before=(self.root/'.swarm/engine-settings.json').read_bytes()
        preview=self.ok(['engine-settings','preview'],request)
        assert (self.root/'.swarm/engine-settings.json').read_bytes()==before
        request['preview_token']=preview['preview_token'];self.ok(['engine-settings','apply'],request)
        assert self.ok(['engine-settings','show'])['values']==values

    def snapshot(self):
        """Observe exact owned Work/agent/receipt state around one failed path.
        Read-only fixture SQL complements public Work JSON; it mutates nothing.
        """
        work=self.ok(['work','show',self.work])['work']
        db=sqlite3.connect('file:'+str(self.root/'.swarm/state.db')+'?mode=ro',uri=True)
        try:
            agent=db.execute('SELECT status,body,request FROM agents WHERE id=?',('session',)).fetchone()
        finally:
            db.close()
        return work,agent,self.reservation()

    def run_and_assert(self, prepared, expected_level=None, *, run_observed=None):
        """Verify actual provider argv/stdin and persisted route/observations.

        expected_level is a test assertion for task-inherited choices; it does not
        change the launch request or engine policy.
        Optional run_observed invokes a caller-owned public execution observer
        instead of the ordinary synchronous CLI helper; it returns the same
        actual result, whose route/usage/PID/report invariants stay checked.
        This test callback adds no product policy or execution authority.
        Final task stays blocked and estimate1 stays estimated despite declared
        cost25. No invoice, applied model or human acceptance is established.
        """
        route=prepared['agent']['model_route']
        policy=self.provider['model_policy'];level=self.request.get('level','auto')
        level=expected_level if expected_level is not None else policy['work_level'] if level in ('','auto') else level
        choice=policy['levels'][level]
        assert route['level']==level and route['model']==choice['model'] and route['effort']==choice['effort']
        canonical={key:policy[key] for key in ['version','billing','work_level','page_level']}
        canonical['levels']={name:dict(model=policy['levels'][name]['model'],effort=policy['levels'][name]['effort']) for name in sorted(policy['levels'])}
        assert route['policy_hash']==digest(json.dumps(canonical,separators=(',',':'),ensure_ascii=False).encode())
        assert prepared['agent']['preflight']['verification']=='unverified'
        sources=[c for c in prepared['agent']['preflight']['checks'] if c['name']=='model_route_sources']
        assert len(sources)==1 and sources[0]['status']=='declared_unverified' and len(sources[0]['detail'])==64
        assert self.reservation()==('reserved',1.0)
        assert not (self.root/'model-received.json').exists()
        result=run_observed() if run_observed is not None else self.ok(['agent','run','session'])
        assert result['task_accepted'] is False and result['agent']['status']=='completed',result
        actual=json.loads((self.root/'model-received.json').read_bytes())
        args=actual['argv']
        expected=[*self.expected_prefix,'--model',choice['model']]
        if choice['effort']: expected+=['-c','model_reasoning_effort='+json.dumps(choice['effort'],ensure_ascii=False)]
        expected+=['-'];assert args==expected,args
        assert actual['stdin_sha256']==digest(prepared['agent']['prompt'].encode())
        assert actual['stdin_bytes']>0 and actual['pid']==result['agent']['child_pid'] and actual['ppid']==result['agent']['supervisor_pid']
        assert result['agent']['model_route']==route
        assert result['agent']['reported_model']['model']==choice['model']
        assert result['agent']['usage']['input_tokens']==7 and result['agent']['usage']['output_tokens']==8
        assert self.reservation()==('estimated',1.0)
        work=self.ok(['work','show',self.work])['work']
        assert work['tasks'][0]['status']=='blocked'
        shown=self.ok(['agent','show','session'])
        assert shown['agent']==result['agent']
        return actual


def valid(binary, output):
    """Only requested tier is announced; absent other-tier models do not block it."""
    f=Fixture(binary,output,'valid')
    before=f.ok(['work','show',f.work])['work']
    preview=f.prepare(True)
    assert preview['preview_only'] is True and preview['process_started'] is False
    assert f.reservation() is None and f.ok(['work','show',f.work])['work']==before
    f.request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
    prepared=f.prepare()
    f.run_and_assert(prepared)
    return dict(case='valid',status='PASS',root=str(f.root))


def cache_change(binary, output):
    """A changed declared cache refuses an existing intent, preserving its lease;
    restoring the exact owned source lets the same intent run once, not replay.
    """
    f=Fixture(binary,output,'cache-change')
    f.prepare(True)
    prepared=f.prepare()
    before=f.snapshot()
    changed=json.loads(f.cache_bytes);changed['cache_generation']='changed-but-same-selected-model'
    publish(f.cache,changed)
    f.denied(['agent','run','session'],None,'agent-execution-model-route-source-changed')
    assert f.snapshot()==before
    assert not (f.root/'model-received.json').exists()
    publish(f.cache,f.cache_bytes)
    f.run_and_assert(prepared)
    return dict(case='cache-change',status='PASS',root=str(f.root))


def rules_change(binary, output):
    """Relevant rules refuse before launch; restored values at a new revision
    recover, while an unrelated saved presentation change does not stale a route.
    """
    f=Fixture(binary,output,'rules-change')
    f.prepare(True);prepared=f.prepare()
    f.settings_change(agent_model_automatic_levels=['simple'])
    before=f.snapshot()
    f.denied(['agent','run','session'],None,'settings-model-automatic-level-denied')
    assert f.snapshot()==before and not (f.root/'model-received.json').exists()
    f.settings_change(agent_model_automatic_levels=['simple','standard'])
    f.settings_change(work_view_recent_decisions='3')
    f.run_and_assert(prepared)
    return dict(case='rules-change',status='PASS',root=str(f.root))


def cache_refusals(binary, output):
    """Valid controls precede malformed/nonregular/oversize cache inputs. Each
    refused preview leaves Work/agent/estimate unchanged, then corrected IO recovers.
    """
    f=Fixture(binary,output,'cache-refusals')
    publish(output/'cache-refusals-original.json',f.cache_bytes)
    before=f.snapshot()
    f.prepare(True)
    publish(f.cache,b'{"models":[{"slug":{"PRIVATE_MODEL_CACHE_PAYLOAD":"must-not-leak"}}]}')
    wire=f.denied(['agent','preview',f.work],f.request,'agent-model-catalogue-invalid')
    assert 'PRIVATE_MODEL_CACHE_PAYLOAD' not in json.dumps(wire)
    assert f.snapshot()==before
    publish(f.cache,f.cache_bytes);f.prepare(True)
    f.cache.unlink();os.mkfifo(f.cache,0o600)
    f.denied(['agent','preview',f.work],f.request,'agent-model-catalogue-not-regular')
    assert f.snapshot()==before
    f.cache.unlink();publish(f.cache,f.cache_bytes);f.prepare(True)
    provider_bytes=f.providers.stat().st_size
    large=json.loads(f.cache_bytes);large['ignored_padding']='x'*provider_bytes
    raw=json.dumps(large).encode();assert len(raw)>provider_bytes
    publish(f.cache,raw);f.prepare(True)
    original_cap=f.ok(['engine-settings','show'])['values']['agent_provider_config_max_bytes']
    f.settings_change(agent_provider_config_max_bytes=str(provider_bytes))
    f.denied(['agent','preview',f.work],f.request,'agent-model-catalogue-outside-saved-policy')
    assert f.snapshot()==before
    f.settings_change(agent_provider_config_max_bytes=original_cap)
    f.prepare(True);prepared=f.prepare();f.run_and_assert(prepared)
    return dict(case='cache-refusals',status='PASS',root=str(f.root),provider_bytes=provider_bytes,cache_bytes=len(raw))


def policy_controls(binary, output):
    """Hash, unavailable effort and unsupported requirements refuse without
    mutation. An exigeant automatic default needs an explicit saved Admin change.
    """
    f=Fixture(binary,output,'policy-controls');preview=f.prepare(True);before=f.snapshot()
    f.request['model_policy_hash']='0'*64
    f.denied(['agent','preview',f.work],f.request,'agent-model-policy-hash-changed')
    assert f.snapshot()==before
    f.request.pop('model_policy_hash');f.prepare(True)
    choice=f.provider['model_policy']['levels']['standard']
    original_model=choice['model']
    for invalid in ['', '-name', '_name', '.name', 'name space', 'nameé', 'a'*161]:
        f.prepare(True)
        choice['model']=invalid;f.save_provider()
        publish(f.cache,{'models':[{'slug':invalid,'visibility':'list','supported_reasoning_levels':[{'effort':choice['effort']}]}]})
        f.denied(['agent','preview',f.work],f.request,'agent-model-policy-name-v1-invalid');assert f.snapshot()==before
        choice['model']=original_model;f.save_provider();publish(f.cache,f.cache_bytes);f.prepare(True)
    choice['effort']='not-announced';f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'agent-model-choice-not-announced');assert f.snapshot()==before
    choice['effort']='fixture-medium';f.save_provider();f.prepare(True)
    choice['effort']='-c';f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'agent-model-choice-invalid');assert f.snapshot()==before
    choice['effort']='fixture-medium';f.save_provider();f.prepare(True)
    f.provider['preflight_required']=True;f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'agent-provider-adapter-or-model-route-unported');assert f.snapshot()==before
    f.provider.pop('preflight_required');f.save_provider();f.prepare(True)
    f.provider['api_connection_id']='fixture-api';f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'agent-provider-adapter-or-model-route-unported');assert f.snapshot()==before
    f.provider.pop('api_connection_id');f.save_provider();f.prepare(True)
    saved=f.provider.pop('model_policy');f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'agent-model-saved-policy-required');assert f.snapshot()==before
    f.provider['model_policy']=saved;f.save_provider();f.prepare(True)
    f.settings_change(agent_model_restriction_rules=['exact:fixture-standard'],agent_model_restricted_levels=['exigeant'])
    f.denied(['agent','preview',f.work],f.request,'settings-model-restricted-level-denied');assert f.snapshot()==before
    f.settings_change(agent_model_restricted_levels=['standard','exigeant']);f.prepare(True)
    cache=json.loads(f.cache_bytes);cache['models'].append(dict(slug='fixture-frontier',visibility='list',supported_reasoning_levels=[]));publish(f.cache,cache)
    f.request['level']='exigeant';f.prepare(True)
    f.request['level']='auto';f.prepare(True)
    f.provider['model_policy']['work_level']='exigeant';f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'settings-model-automatic-level-denied');assert f.snapshot()==before
    f.settings_change(agent_model_automatic_levels=['simple','standard','exigeant'])
    f.prepare(True);prepared=f.prepare();f.run_and_assert(prepared)
    return dict(case='policy-controls',status='PASS',root=str(f.root),automatic_default='explicitly_adopted')


def effective_environment(binary, output):
    """An unauthorized parent CODEX_HOME cannot select/poison the child cache.
    The actual provider receives HOME only, and parent-only changes do not stale it.
    """
    f=Fixture(binary,output,'effective-environment');f.prepare(True)
    home=f.root/'effective-home';(home/'.codex').mkdir(parents=True,mode=0o700)
    os.chmod(home,0o700);publish(home/'.codex/models_cache.json',f.cache_bytes)
    ignored=f.root/'ignored-parent';ignored.mkdir(mode=0o700);publish(ignored/'models_cache.json',f.cache_bytes)
    f.settings_change(agent_environment_allow=['HOME'])
    f.environment['HOME']=str(home);f.environment['CODEX_HOME']=str(ignored)
    f.prepare(True)
    publish(ignored/'models_cache.json',b'PRIVATE_BAD_CACHE')
    f.prepare(True);prepared=f.prepare()
    assert prepared['agent']['env_allow']==['HOME']
    f.environment['CODEX_HOME']=str(f.root/'another-ignored-parent')
    actual=f.run_and_assert(prepared)
    assert actual['codex_home_present'] is False
    return dict(case='effective-environment',status='PASS',root=str(f.root))


def configured_without_cache(binary, output):
    """A configured model is only an unverified announcement; missing cache
    invents no effort and never turns a malformed cache into a successful fallback.
    """
    f=Fixture(binary,output,'configured-without-cache');f.prepare(True)
    f.provider['args'][f.provider['args'].index('--model')+1]='fixture-standard'
    f.provider['model_policy']['levels']['standard']['effort']='';f.save_provider();f.prepare(True)
    f.cache.unlink();f.prepare(True);before=f.snapshot()
    f.provider['model_policy']['levels']['standard']['effort']='fixture-medium';f.save_provider()
    f.denied(['agent','preview',f.work],f.request,'agent-model-choice-not-announced');assert f.snapshot()==before
    f.provider['model_policy']['levels']['standard']['effort']='';f.save_provider();f.prepare(True)
    prepared=f.prepare();f.run_and_assert(prepared)
    return dict(case='configured-without-cache',status='PASS',root=str(f.root),announcement='configured_argv_unverified')

def generic_mode(binary,output):
    """Custom routing preserves unrelated single-dash flags instead of assuming
    Codex short-option spelling; configured announcement and actual argv agree.
    """
    f=Fixture(binary,output,'generic-mode');f.prepare(True)
    publish(f.root/'custom-provider',PROGRAM.encode());os.chmod(f.root/'custom-provider',0o700)
    f.provider['command']=str(f.root/'custom-provider')
    f.provider['args']=['-mode','safe',*f.provider['args']]
    f.provider['args'][f.provider['args'].index('--model')+1]='fixture-standard'
    f.provider['model_policy']['levels']['standard']['effort']=''
    f.expected_prefix=['-mode','safe',*f.expected_prefix]
    f.save_provider();f.prepare(True);prepared=f.prepare();f.run_and_assert(prepared)
    return dict(case='generic-mode',status='PASS',root=str(f.root))


def api_catalogue(binary,output):
    """Valid unrelated API metadata permits actual CLI execution, while active
    collisions/selected APIs/type/read/cap faults refuse without state mutation.
    Every fault follows a valid control; original secrets never enter diagnostics.
    """
    f=Fixture(binary,output,'api-catalogue');f.prepare(True);before=f.snapshot()
    path=f.root/'.swarm/ai-connections.json'
    for raw in [b'{}',b'null',b'{"local":null}',b'{"local":{"disabled":false,"key":"PRIVATE_API_KEY","base_url":"not contacted","model":"unverified"}}']:
        publish(path,raw);f.prepare(True);assert f.snapshot()==before
    for raw in [b'',b'[]',b'{} {}',b'{},"connections":{}',b'{"local":{"disabled":"false"}}',b'{"local":{"key":42}}',b'{"local":{"PRIVATE_UNKNOWN":true}}',b'{"local":{"key":"\xff"}}']:
        publish(path,{});f.prepare(True)
        publish(path,raw);wire=f.denied(['agent','preview',f.work],f.request,'agent-api-connections-catalogue-invalid')
        assert 'PRIVATE' not in json.dumps(wire) and f.snapshot()==before
        publish(path,{});f.prepare(True)
    publish(path,{'local':{'key':'PRIVATE_API_KEY'}});f.prepare(True)
    f.request['provider']='api-local'
    wire=f.denied(['agent','preview',f.work],f.request,'agent-api-provider-execution-unported')
    assert 'PRIVATE_API_KEY' not in json.dumps(wire) and f.snapshot()==before
    f.request['provider']='fixture';f.prepare(True)
    # The Go namespace checks collisions globally, even for an unrelated request.
    providers=dict(schema_version=1,providers={'fixture':f.provider,'api-local':f.provider})
    publish(f.providers,providers)
    f.denied(['agent','preview',f.work],f.request,'agent-provider-api-name-conflict');assert f.snapshot()==before
    publish(path,{'local':{'disabled':True,'key':'PRIVATE_API_KEY'}});f.prepare(True)
    f.save_provider();publish(path,{});f.prepare(True)
    # Reuse the actual saved per-file cap; both the valid provider/cache fit.
    default_cap=f.ok(['engine-settings','show'])['values']['agent_provider_config_max_bytes']
    cap=max(len(f.providers.read_bytes()),len(f.cache.read_bytes()))
    f.settings_change(agent_provider_config_max_bytes=str(cap));f.prepare(True)
    publish(path,{'local':{'key':'PRIVATE_API_KEY'+'x'*(cap+1)}})
    wire=f.denied(['agent','preview',f.work],f.request,'agent-provider-catalogue-read')
    assert 'PRIVATE_API_KEY' not in json.dumps(wire) and f.snapshot()==before
    publish(path,{});f.prepare(True);f.settings_change(agent_provider_config_max_bytes=default_cap)
    # Descriptor-backed config reads reject nonregular/symlink paths without wait.
    for kind in ['fifo','symlink']:
        publish(path,{});f.prepare(True);path.unlink()
        if kind=='fifo':os.mkfifo(path,0o600)
        else:
            target=f.root/'api-symlink-target';publish(target,{});path.symlink_to(target)
        f.denied(['agent','preview',f.work],f.request,'agent-provider-catalogue-read');assert f.snapshot()==before
        path.unlink();publish(path,{});f.prepare(True)
    publish(path,{'local':{'disabled':False,'key':'PRIVATE_API_KEY','base_url':'not contacted','model':'unverified'}})
    f.prepare(True);prepared=f.prepare();f.run_and_assert(prepared)
    return dict(case='api-catalogue',status='PASS',root=str(f.root),selected_api='explicitly_refused',secrets='not exposed',network_calls=0)


def provider_configuration(binary,output):
    """Configure through the public CLI, preserving existing bytes and budgets.
    Readback is explicitly file-only, never API-secret export or readiness proof;
    the final actual agent run separately establishes the local execution witness.
    """
    f=Fixture(binary,output,'provider-configuration');f.prepare(True);before=f.snapshot()
    request=dict(schema_version=1,providers={'fixture':f.provider})
    original=f.providers.read_bytes()
    shown=f.ok(['providers','config','show'])
    assert shown['digest']==digest(original) and shown['created'] is False and shown['provider_execution_started'] is False
    f.denied(['providers','config','init'],request,'provider-configuration-already-exists');assert f.providers.read_bytes()==original and f.snapshot()==before
    for invalid,code in [(dict(schema_version=2,providers={}), 'provider-configuration-schema-invalid'),(dict(schema_version=1,PRIVATE_UNKNOWN=True),'provider-configuration-invalid')]:
        f.prepare(True);wire=f.denied(['providers','config','init'],invalid,code)
        assert 'PRIVATE' not in json.dumps(wire) and f.providers.read_bytes()==original and f.snapshot()==before
    # Removing our setup file is a fixture precondition, not runtime repair/reset.
    publish(output/'provider-configuration-original.json',original);f.providers.unlink()
    f.settings_change(agent_provider_config_max_bytes='0')
    f.denied(['providers','config','init'],request,'provider-configuration-outside-saved-policy');assert not f.providers.exists() and f.snapshot()==before
    f.settings_change(agent_provider_config_max_bytes=None)
    created=f.ok(['providers','config','init'],request)
    assert created['created'] is True and created['provider_execution_started'] is False and created['scope']=='cli_configuration_file_only_unverified'
    raw=f.providers.read_bytes();assert created['digest']==digest(raw) and f.snapshot()==before
    shown=f.ok(['providers','config','show']);assert shown['digest']==created['digest'] and shown['created'] is False
    f.denied(['providers','config','init'],request,'provider-configuration-already-exists');assert f.providers.read_bytes()==raw
    f.prepare(True);prepared=f.prepare();f.run_and_assert(prepared)
    after=f.snapshot();api=f.root/'.swarm/ai-connections.json';publish(api,{'local':{'key':'PRIVATE_API_READBACK_KEY','disabled':False}})
    shown=f.ok(['providers','config','show']);assert 'PRIVATE_API_READBACK_KEY' not in json.dumps(shown) and f.snapshot()==after
    api.unlink()
    return dict(case='provider-configuration',status='PASS',root=str(f.root),initialized_through_public_cli=True,readback='CLI file only, no API secret merge',task_accepted=False)


def configuration_proposal(f,configuration=None,reference=None):
    """Use actual metadata/readonly preview; token is a binding, not authority."""
    current=f.ok(['providers','config','inspect'])
    request=dict(schema_version=1,expected_digest=current['digest'])
    if configuration is not None:request['configuration']=configuration
    if reference is not None:request['restore_reference']=reference
    before=f.providers.read_bytes()
    preview=f.ok(['providers','config','preview'],request)
    assert preview['preview'] is True and preview['applied'] is False and preview['provider_execution_started'] is False
    assert f.providers.read_bytes()==before and preview['before_digest']==digest(before)
    request['preview_token']=preview['preview_token']
    return request,preview


def configuration_update_restore(binary,output):
    """Changing a requested model after preparation refuses the same queued agent
    without spending its reservation; exact supported restoration lets it run.
    Catalogue bytes stay unchanged across the fault to isolate configuration.
    """
    f=Fixture(binary,output,'configuration-update-restore');f.prepare(True)
    catalogue=json.loads(f.cache_bytes);catalogue['models'].append(dict(slug='fixture-alternate',visibility='list',supported_reasoning_levels=[dict(effort='fixture-medium')]))
    f.cache_bytes=json.dumps(catalogue).encode();publish(f.cache,f.cache_bytes);f.prepare(True)
    original=f.providers.read_bytes();original_provider=json.loads(json.dumps(f.provider));prepared=f.prepare();before=f.snapshot()
    assert f.reservation()==('reserved',1.0)
    f.provider['model_policy']['levels']['standard']['model']='fixture-alternate'
    request,preview=configuration_proposal(f,dict(schema_version=1,providers={'fixture':f.provider}))
    assert f.snapshot()==before
    bad=dict(request);bad['preview_token']='0'*64
    f.denied(['providers','config','apply'],bad,'provider-configuration-preview-required');assert f.providers.read_bytes()==original and f.snapshot()==before
    applied=f.ok(['providers','config','apply'],request);reference=applied['backup_reference']
    assert applied['changed'] is True and applied['applied'] is True and applied['provider_execution_started'] is False
    assert f.providers.read_bytes()!=original and (f.root/'.swarm'/reference).read_bytes()==original and f.snapshot()==before
    changed=f.providers.read_bytes()
    f.denied(['providers','config','apply'],request,'revision_conflict');assert f.providers.read_bytes()==changed and f.snapshot()==before
    f.denied(['agent','run','session'],None,'agent-execution-model-route-changed');assert f.snapshot()==before and not (f.root/'model-received.json').exists()
    restored,_=configuration_proposal(f,reference=reference);result=f.ok(['providers','config','apply'],restored)
    assert f.providers.read_bytes()==original and f.snapshot()==before
    assert (f.root/'.swarm'/result['backup_reference']).read_bytes()==changed
    meta=f.ok(['providers','config','inspect',reference]);assert meta['format_valid'] is True and meta['matches_reference'] is True
    refs=f.ok(['providers','config','backups']);assert reference in refs['references'] and refs['scope']=='snapshot_references_only_unverified'
    f.provider=original_provider;f.run_and_assert(prepared)
    return dict(case='configuration-update-restore',status='PASS',root=str(f.root),queued_agent='refused on changed route, same agent runs after exact restore',reservation='retained until real execution',catalogue_unchanged=True)


def configuration_repair(binary,output):
    """Explicit inspection/preview repairs malformed data without losing it or
    allowing a malformed/symlink snapshot to become current configuration.
    """
    f=Fixture(binary,output,'configuration-repair');f.prepare(True);before=f.snapshot();original=f.providers.read_bytes();configuration=json.loads(original)
    publish(f.providers,b'PRIVATE corrupted provider source')
    wire=f.denied(['providers','config','show'],None,'provider-configuration-invalid');assert 'PRIVATE' not in json.dumps(wire) and f.snapshot()==before
    meta=f.ok(['providers','config','inspect']);assert meta['format_valid'] is False and meta['digest']==digest(f.providers.read_bytes()) and 'PRIVATE' not in json.dumps(meta)
    request,_=configuration_proposal(f,configuration);fixed=f.ok(['providers','config','apply'],request);reference=fixed['backup_reference'];repaired=f.providers.read_bytes()
    assert (f.root/'.swarm'/reference).read_bytes()==b'PRIVATE corrupted provider source' and f.snapshot()==before
    selected=f.ok(['providers','config','inspect',reference]);assert selected['format_valid'] is False and selected['matches_reference'] is True
    bad=dict(schema_version=1,expected_digest=digest(repaired),restore_reference=reference)
    f.denied(['providers','config','preview'],bad,'provider-configuration-invalid');assert f.providers.read_bytes()==repaired and f.snapshot()==before
    # A syntactically valid reference is still not a trusted descriptor or grant.
    link='providers-before-'+digest(repaired)+'-snapshot-abcdef.json';(f.root/'.swarm'/link).symlink_to('providers.json')
    bad['restore_reference']=link
    f.denied(['providers','config','preview'],bad,'local-storage');assert f.providers.read_bytes()==repaired and f.snapshot()==before
    good,_=configuration_proposal(f,configuration);noop=f.ok(['providers','config','apply'],good);assert noop['applied'] is True and noop['changed'] is False and noop['backup_reference'] is None
    f.prepare(True);prepared=f.prepare();f.run_and_assert(prepared)
    return dict(case='configuration-repair',status='PASS',root=str(f.root),original_corruption='retained in snapshot',invalid_restore='refused without mutation',recovery='public update then real execution')


def configuration_scope(binary,output):
    """Same data/revision across owned Roots cannot reuse a target preview.
    The public token is not authentication; each Root's own preview still works.
    """
    a=Fixture(binary,output,'configuration-scope-a');b=Fixture(binary,output,'configuration-scope-b')
    b.provider=json.loads(json.dumps(a.provider));b.save_provider();a.prepare(True);b.prepare(True)
    assert a.providers.read_bytes()==b.providers.read_bytes()
    configuration=dict(schema_version=1,providers={'fixture':json.loads(json.dumps(a.provider))});configuration['providers']['fixture']['args'].append('--scope-witness')
    ra,pa=configuration_proposal(a,configuration);rb,pb=configuration_proposal(b,configuration)
    assert pa['before_digest']==pb['before_digest'] and pa['preview_token']!=pb['preview_token']
    before=b.snapshot();raw=b.providers.read_bytes();foreign=dict(rb,preview_token=ra['preview_token'])
    b.denied(['providers','config','apply'],foreign,'provider-configuration-preview-required');assert b.snapshot()==before and b.providers.read_bytes()==raw
    applied=b.ok(['providers','config','apply'],rb);assert applied['changed'] is True and b.snapshot()==before
    applied=a.ok(['providers','config','apply'],ra);assert applied['changed'] is True
    return dict(case='configuration-scope',status='PASS',roots=[str(a.root),str(b.root)],scope='separate root targets, no actor authentication or execution inferred')


def task_model(binary,output):
    """Persist/reopen/replay a task choice, refuse conflicts/active changes, run
    actual inherited argv, then clear only the choice while preserving history.
    Policy drift is injected only after a valid selection/launch preview witness.
    """
    f=Fixture(binary,output,'task-model')
    route=f.prepare(True)['agent']['model_route']
    publish(f.cache,{'models':[{'slug':'fixture-simple','visibility':'list'},{'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]}]})
    r=dict(schema_version=1,event_id='select',expected_revision=3,task_id='task',provider='fixture',level='simple',model_policy_hash=route['policy_hash'],inherit=False)
    before=f.snapshot();preview=f.ok(['task-model','preview',f.work],r)
    assert preview['before'] is None and preview['after']['route']['model']=='fixture-simple' and f.snapshot()==before
    changed=f.ok(['task-model','apply',f.work],r);assert changed['revision']==4
    assert changed['tasks'][0]['model_selection']['route']['level']=='simple'
    assert f.ok(['task-model','apply',f.work],r)==changed
    shown=f.ok(['task-model','show',f.work]);assert shown==changed
    before=f.snapshot();bad=dict(r,event_id='stale')
    f.denied(['task-model','apply',f.work],bad,'revision_conflict');assert f.snapshot()==before
    f.request['expected_revision']=4
    valid=f.prepare(True);assert valid['agent']['model_route']['model']=='fixture-simple'
    original=dict(f.request)
    for patch in [dict(provider='other'),dict(level='exigeant'),dict(model_policy_hash='a'*64)]:
        f.request={**original,**patch};f.denied(['agent','preview',f.work],f.request,'task-model-launch-mismatch');assert f.snapshot()==before
    f.request=original
    good=json.dumps(f.provider);good_bytes=f.providers.read_bytes();f.provider['model_policy']['page_level']='standard';f.save_provider()
    f.denied(['agent','prepare',f.work],f.request,'agent-model-policy-hash-changed');assert f.snapshot()==before and f.reservation() is None
    f.provider=json.loads(good);f.save_provider();assert f.providers.read_bytes()==good_bytes;assert f.prepare(True)['agent']['model_route']['model']=='fixture-simple'
    prepared=f.prepare();queued=f.snapshot();clear=dict(r,event_id='clear',expected_revision=queued[0]['revision'],inherit=True)
    f.denied(['task-model','preview',f.work],clear,'task-model-active-attempt');f.denied(['task-model','apply',f.work],clear,'task-model-active-attempt');assert f.snapshot()==queued
    # Same valid queued intention is the witness for policy-drift injection.
    f.provider['model_policy']['page_level']='standard';f.save_provider()
    f.denied(['agent','run','session'],None,'agent-model-policy-hash-changed');assert f.snapshot()==queued and not (f.root/'model-received.json').exists()
    f.provider=json.loads(good);f.save_provider();assert f.providers.read_bytes()==good_bytes
    actual=f.run_and_assert(prepared,expected_level='simple')
    current=f.ok(['task-model','show',f.work]);history=current['tasks'][0]['attempts']
    clear['expected_revision']=current['revision']
    cleared=f.ok(['task-model','apply',f.work],clear)
    assert cleared['tasks'][0].get('model_selection') is None and cleared['tasks'][0]['attempts']==history
    assert f.ok(['task-model','apply',f.work],clear)==cleared
    state=f.snapshot();compatible=dict(schema_version=1,event_id='compat',expected_revision=cleared['revision'],task_id='task',inherit=True,provider=None,level=None,model_policy_hash=None)
    assert f.ok(['task-model','preview',f.work],compatible)['after'] is None and f.snapshot()==state
    return dict(case='task-model',status='PASS',root=str(f.root),actual=actual,scope='explicit local task selection, not authentication or acceptance')


def policy_proposal(f,policies):
    """Preview policy-only data against real source/settings without mutation.
    Return an explicit apply request; token is public binding, never permission.
    """
    raw=f.providers.read_bytes();before=f.snapshot()
    request=dict(version=1,expected_digest=digest(raw),policies=json.loads(json.dumps(policies)),preview=True)
    view=f.ok(['providers','policies','preview'],request)
    assert view['preview'] is True and view['applied'] is False and view['provider_execution_started'] is False
    assert view['policies']==policies and view['before_digest']==digest(raw) and f.providers.read_bytes()==raw and f.snapshot()==before
    assert not {'configuration','command','env_allow','key'} & set(view)
    request.update(preview=False,preview_token=view['preview_token'])
    return request,view


def policy_import(binary,output):
    """Import portable explicit policies, preserving commands/history and exact
    originals; reject invalid/stale data, exercise saved Admin rules and recover.
    Actual local process runs only after independent admission. Large fixtures
    demonstrate no hidden32/64KiB policy, not a performance claim or paid call.
    """
    f=Fixture(binary,output,'policy-import')
    cache={'models':[{'slug':'fixture-simple','visibility':'list'},{'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]},{'slug':'fixture-frontier','visibility':'list'}]}
    publish(f.cache,cache);f.prepare(True)
    policy=json.loads(json.dumps(f.provider.pop('model_policy')));f.save_provider()
    raw=f.providers.read_bytes();previous=f.ok(['providers','config','show'])['configuration']['providers']['fixture'];before=f.snapshot()
    f.denied(['agent','preview',f.work],f.request,'agent-model-saved-policy-required')
    f.denied(['providers','policies','export'],None,'provider-policy-implicit-default-migration-required')
    assert f.snapshot()==before and f.providers.read_bytes()==raw
    portable=dict(version=1,policies={'fixture':policy});publish(output/'policy-import-portable.v1.json',portable)
    r,view=policy_proposal(f,portable['policies']);assert view['changed'] is True
    f.denied(['providers','policies','apply'],dict(r,preview=True),'provider-policy-preview-not-application')
    missing=dict(r);missing.pop('preview_token');f.denied(['providers','policies','apply'],missing,'provider-configuration-preview-required')
    assert f.snapshot()==before and f.providers.read_bytes()==raw
    # The valid preview is the witness before every isolated input/source fault.
    invalid=[(dict(r,policies=None),'provider-policy-request-invalid'),(dict(r,policies={}), 'provider-policy-request-invalid'),(dict(r,policies={'fixture':None}),'provider-policy-request-invalid'),(dict(r,policies={'unknown':policy}),'provider-policy-provider-missing'),(dict(r,key='PRIVATE'), 'provider-configuration-invalid')]
    for request,code in invalid:
        policy_proposal(f,portable['policies']);f.denied(['providers','policies','preview'],request,code);assert f.snapshot()==before and f.providers.read_bytes()==raw
    cache_bytes=f.cache.read_bytes();publish(f.cache,b'PRIVATE malformed')
    f.denied(['providers','policies','preview'],r,'agent-model-catalogue-invalid');assert f.snapshot()==before and f.providers.read_bytes()==raw
    publish(f.cache,cache_bytes);policy_proposal(f,portable['policies'])
    # All tiers are validated: hiding an unselected tier cannot look like success.
    publish(f.cache,{'models':[cache['models'][1]]})
    f.denied(['providers','policies','preview'],r,'agent-model-choice-not-announced');assert f.snapshot()==before and f.providers.read_bytes()==raw
    publish(f.cache,cache_bytes);policy_proposal(f,portable['policies'])
    f.settings_change(agent_model_automatic_levels=['simple'])
    f.denied(['providers','policies','preview'],r,'settings-model-automatic-level-denied');assert f.snapshot()==before
    f.settings_change(agent_model_automatic_levels=['simple','standard'])
    f.denied(['providers','policies','apply'],r,'provider-configuration-preview-required');assert f.snapshot()==before and f.providers.read_bytes()==raw
    # Configuration validation remains possible when execution is explicitly denied.
    f.settings_change(agent_allow_unverified_provider_context='0')
    r,view=policy_proposal(f,portable['policies']);applied=f.ok(['providers','policies','apply'],r)
    assert applied['applied'] is True and applied['changed'] is True and f.snapshot()==before
    assert (f.root/'.swarm'/applied['backup_reference']).read_bytes()==raw
    stored=json.loads(f.providers.read_bytes())['providers']['fixture']
    assert {k:v for k,v in stored.items() if k!='model_policy'}==previous
    assert f.ok(['providers','policies','export'])==portable
    f.denied(['providers','policies','apply'],r,'revision_conflict')
    f.denied(['agent','prepare',f.work],f.request,'agent-provider-verified-context-required');assert f.reservation() is None and f.snapshot()==before
    f.settings_change(agent_allow_unverified_provider_context='1');f.provider=stored
    prepared=f.prepare();actual=f.run_and_assert(prepared)
    terminal=f.snapshot()
    # Same operator file policy governs larger configured populations and bytes.
    legacy=json.loads(json.dumps(stored));legacy.pop('model_policy');legacy['args'].append('PRIVATE_ARGUMENT_'+('x'*2200))
    names=['fixture']+['fixture-'+str(i) for i in range(1,35)]
    configuration=dict(schema_version=1,providers={n:json.loads(json.dumps(legacy)) for n in names})
    change,_=configuration_proposal(f,configuration=configuration);f.ok(['providers','config','apply'],change)
    assert len(f.providers.read_bytes())>65536 and f.snapshot()==terminal
    policies={n:json.loads(json.dumps(policy)) for n in names};r,_=policy_proposal(f,policies);applied=f.ok(['providers','policies','apply'],r)
    assert len(applied['policies'])==35 and 'PRIVATE_ARGUMENT_' not in json.dumps(applied) and f.snapshot()==terminal
    exported=f.ok(['providers','policies','export']);assert exported==dict(version=1,policies=policies) and 'PRIVATE_ARGUMENT_' not in json.dumps(exported)
    return dict(case='policy-import',status='PASS',root=str(f.root),actual=actual,policy_count=35,provider_file_bytes=len(f.providers.read_bytes()),scope='unverified policy metadata, actual local child only after separate admission; no account/authentication/HTTP or paid call')


def main():
    """Run explicit owned scenarios; preserve failures and exact binary/source."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--case',choices=['valid','generic-mode','api-catalogue','provider-configuration','configuration-update-restore','configuration-repair','configuration-scope','task-model','policy-import','all'],default='all')
    args=parser.parse_args();binary=args.binary.resolve();output=args.output.resolve()
    output.mkdir(parents=True,exist_ok=False)
    source=Path(__file__).read_bytes()
    publish(output/'driver.py',source)
    publish(output/'inputs.json',dict(binary=str(binary),binary_sha256=digest(binary.read_bytes()),source_sha256=digest(source),global_watchdog=None))
    checks=[]
    try:
        selected=[valid] if args.case=='valid' else [generic_mode] if args.case=='generic-mode' else [api_catalogue] if args.case=='api-catalogue' else [provider_configuration] if args.case=='provider-configuration' else [configuration_update_restore] if args.case=='configuration-update-restore' else [configuration_repair] if args.case=='configuration-repair' else [configuration_scope] if args.case=='configuration-scope' else [task_model] if args.case=='task-model' else [policy_import] if args.case=='policy-import' else [valid,cache_change,rules_change,cache_refusals,policy_controls,effective_environment,configured_without_cache,generic_mode,api_catalogue,provider_configuration,configuration_update_restore,configuration_repair,configuration_scope,task_model,policy_import]
        for scenario in selected:
            result=scenario(binary,output);checks.append(result)
            print('PASS native-model-route-'+result['case'],flush=True)
        publish(output/'result.json',dict(status='PASS',scope='local native requested route only, no account/model authenticity or acceptance',checks=checks))
    except BaseException as error:
        publish(output/'failure.json',dict(status='FAIL',error_type=type(error).__name__,message=str(error),traceback=traceback.format_exc(),checks=checks))
        raise


if __name__=='__main__':
    main()
