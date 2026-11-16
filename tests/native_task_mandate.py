#!/usr/bin/env python3
"""Exercise native task mandates through the real Linux gate and Trust broker.

Owned inert Roots, deterministic provider descendants and public CLI only. No
model, account, live mission, SQL mutation, global watchdog or automatic retry.
A candidate binary is not an installation; production build identity is retained
separately. Native and connector simulation evidence stay distinct.
"""
import argparse
import ast
import re
import errno
import hashlib
import json
import os
from pathlib import Path
import subprocess
import selectors
import socket
import sys
import traceback

from rust_agent_model_route import Fixture, publish, digest
from rust_trust_biba import TOKENS, command, configuration


PROGRAM = r'''#!/usr/bin/python3
import errno,hashlib,json,os,sys
from pathlib import Path
prompt=sys.stdin.buffer.read()
workspace=Path.cwd()
plan=json.loads((workspace/'native-plan.json').read_bytes())
blocked=None
try:
    with open(plan['private_target'],'wb') as stream: stream.write(b'UNAUTHORIZED')
except OSError as error: blocked=error.errno
observed={'pid':os.getpid(),'ppid':os.getppid(),'stdin_sha256':hashlib.sha256(prompt).hexdigest(),
          'biba_write_errno':blocked,'responses':[],'barriers':[]}
channel=os.fdopen(os.dup(1),'rb',buffering=0)
for index,step in enumerate(plan['steps']):
    if 'barrier' in step:
        observed['barriers'].append(step['barrier'])
        with open(workspace/'native-observed.json','w') as stream:
            json.dump(observed,stream);stream.flush();os.fsync(stream.fileno())
        with open(workspace/'native-notify','wb',buffering=0) as stream: stream.write(b'R')
        with open(workspace/'native-go','rb',buffering=0) as stream:
            if stream.read(1)!=b'G': raise RuntimeError('fixture barrier not released')
        continue
    if 'from' in step: step=json.loads((workspace/step['from']).read_bytes())
    if step.get('final_intent'):
        text='SWARM_TRUST_REQUEST '+json.dumps(step['request'],separators=(',',':'))
        event=({'type':'assistant','message':{'content':[{'type':'text','text':text}]}} if step.get('format')=='claude'
               else {'type':'item.completed','item':{'id':'final-intent','type':'agent_message','text':text}})
        print(json.dumps(event),flush=True)
        continue
    print('SWARM_NATIVE_TRUST_V1 '+json.dumps(step['request'],separators=(',',':')),flush=True)
    raw=channel.readline()
    if not raw: raise RuntimeError('missing native broker reply')
    observed['responses'].append(json.loads(raw))
    with open(workspace/'native-observed.json','w') as stream:
        json.dump(observed,stream);stream.flush();os.fsync(stream.fileno())
with open(workspace/'native-observed.json','w') as stream:
    json.dump(observed,stream);stream.flush();os.fsync(stream.fileno())
print(json.dumps({'type':'result','result':'Deterministic native protocol fixture; no task acceptance'}),flush=True)
'''


def require(value, name):
    """Retain explicit failures even with optimization; never infer a missing fact."""
    if not value:
        raise ValueError(name)




def protocol_suffix():
    """Read the reviewed static Rust byte literal; reject unsupported syntax.

    The full suite binds this source to the qualified binary's effective input
    inventory before and after execution. This literal decoder is not a second
    protocol implementation or proof of embedding: actual child stdin supplies
    the separate observed byte witness.
    """
    path=Path(__file__).resolve().parent.parent/'rust/crates/swarm-engine/src/native_provider_proxy.rs'
    source=path.read_text()
    pattern=r'pub\(crate\) const INSTRUCTIONS: &\[u8\] = (b"(?:[^"\\]|\\.)*");'
    matches=re.findall(pattern,source)
    require(len(matches)==1,'native-protocol-suffix-not-unique')
    value=ast.literal_eval(matches[0])
    require(isinstance(value,bytes),'native-protocol-suffix-not-bytes')
    return value


def proxy_protocol_probe(binary, output, mode, legacy=False):
    """Exercise the actual private proxy with an owned dummy peer, no Root/grant.

    A real Python backend does nothing. The peer supplies a correct, stale or
    empty acknowledgment to test transport correlation only. Legacy mode exists
    solely to retain the pre-fix candidate counterexample. No Biba/native effect
    assurance follows from this unconfined protocol component fixture.
    """
    output.mkdir(mode=0o700,parents=True,exist_ok=False)
    peer,child=socket.socketpair()
    argv=[str(binary),'_native_provider_proxy',str(Path(sys.executable).resolve()),'-c','pass']
    with (output/'stderr').open('xb') as err:
        process=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=child,stderr=err)
        child.close()
        reader=peer.makefile('rb')
        try:
            raw=reader.readline()
            require(raw,'proxy-finalization-frame-missing')
            publish(output/'finalization.raw',raw)
            if not legacy:
                require(raw.startswith(b'SWARM_NATIVE_TRUST_V1 '),'proxy-framing-missing')
                raw=raw[len(b'SWARM_NATIVE_TRUST_V1 '):]
            frame=json.loads(raw)
            require(frame['type']=='swarm.trust.finalize' and frame['schema_version']==1,'proxy-finalization-shape')
            nonce=frame.get('id','native-finalize')
            reply=dict(type='swarm.trust.reply',schema_version=1,id=nonce,
                       ok=True,result=dict(finished=True))
            if mode=='stale':reply['id']='unrelated-old-reply'
            elif mode=='empty':reply.pop('result')
            elif mode!='valid':raise ValueError('unknown probe mode')
            publish(output/'reply.json',reply)
            peer.sendall(json.dumps(reply).encode()+b'\n')
            code=process.wait()
        finally:
            reader.close();peer.close()
    publish(output/'result.json',dict(argv=argv,exit=code,binary_sha256=digest(Path(binary).read_bytes()),
            mode=mode,legacy=legacy,scope='standalone proxy protocol only; no Root, grant or sandbox qualification'))
    return code



def unmapped_final_intent_probe(binary, output):
    """Observe an explicit final intent without a configured native identity.

    Uses public worker operations and a deterministic provider. No Trust journal,
    credential or native authority is created. Retains the actual exit and agent
    state to distinguish an explicit refusal from silent successful omission.
    """
    output.mkdir(mode=0o700,parents=True,exist_ok=False)
    f=Fixture(binary,output/'cli','unmapped-native')
    intent='SWARM_TRUST_REQUEST '+json.dumps(dict(type='swarm.trust.request',schema_version=1,
        id='unmapped-intent',operation='commit',input=command('unmapped-commit',action_id='absent-action')))
    program="#!/usr/bin/python3\nimport sys,json\nsys.stdin.buffer.read()\nprint(json.dumps("+repr(dict(
        type='item.completed',item=dict(id='final',type='agent_message',text=intent)))+"),flush=True)\n"
    publish(f.root/'codex',program.encode());os.chmod(f.root/'codex',0o700)
    preview=f.prepare(True);f.request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
    f.prepare();code,reply=f.call(['agent','run','session'])
    agent=f.ok(['agent','show','session'])['agent']
    publish(output/'observed.json',dict(exit=code,reply=reply,status=agent['status'],
            trust_journal_present=(f.root/'.swarm/trust-actions.json').exists(),
            native_result=agent['native_execution_result'],binary_sha256=digest(Path(binary).read_bytes())))
    return code,reply,agent,(f.root/'.swarm/trust-actions.json').exists()


class Run:
    """One replayable native run, preserving original public IO and fixture data."""
    def __init__(self, binary, output, integrity=2, planned_denials=3):
        self.output = output
        output.mkdir(mode=0o700, parents=True, exist_ok=False)
        self.passed = []
        self.fixture = Fixture(binary, output / 'cli', 'native-mandate')
        self.f = self.fixture
        publish(self.f.root / 'codex', PROGRAM.encode())
        os.chmod(self.f.root / 'codex', 0o700)
        # Allocate one more error than the declared deliberate refusal count,
        # through public persisted Admin validation. Launch cannot relax a
        # provider policy; product defaults/ceilings are never bypassed.
        self.f.settings_change(runtime_max_consecutive_errors_default=str(planned_denials+1))
        preview = self.f.prepare(True)
        require(preview['preview_only'] and not preview['process_started'], 'preview-is-inert')
        self.f.request['model_policy_hash'] = preview['agent']['model_route']['policy_hash']
        self.prepared = self.f.prepare()['agent']
        self.prepared_by_agent={'session':self.prepared}
        self.scope = self.f.ok(['agent', 'native-scope', 'session'])
        self.config = configuration()
        self.config['identities'].append(dict(subject='native-worker',native_scope=self.scope,
                                              integrity=integrity,roles=['effect']))
        self.trust('config-apply', self.config)
        self.check('scope-binds-prepared-agent', self.scope['agent_id']=='session'
                   and self.scope['task_id']=='task' and self.scope['work_id']==self.f.work
                   and self.scope['attempt_id']==self.prepared['attempt_id'])
        self.check('scope-is-data-without-token', set(self.scope)=={
            'root_digest','work_id','task_id','attempt_id','agent_id'})

    def check(self, name, value):
        """Publish named observations atomically; false aborts without cleanup."""
        require(value, name)
        self.passed.append(name)
        publish(self.output/'assertions.json', self.passed)
        print('PASS '+name, flush=True)

    def trust(self, op, value=None, actor=''):
        """Use public Trust CLI with explicit synthetic credentials only."""
        old = self.f.environment.pop('SWARM_TRUST_TOKEN', None)
        try:
            if actor:
                self.f.environment['SWARM_TRUST_TOKEN'] = TOKENS[actor]
            return self.f.ok(['trust',op], value)
        finally:
            self.f.environment.pop('SWARM_TRUST_TOKEN', None)
            if old is not None:
                self.f.environment['SWARM_TRUST_TOKEN'] = old

    def action(self, suffix, scope=None, approve=True, parent=None):
        """Issue an exact operator-bound mandate, request and separate approval.

        Native effect authority is never borrowed from this fixture requester or
        its synthetic bearer. Mandate scope is explicit, not an agent claim.
        """
        mandate='mandate-'+suffix
        created=self.trust('mandate-create', command('create-'+suffix, mandate_id=mandate,
            native_scope=scope or self.scope,subject='requester',actions=['settle'],
            resource='invoice/native',max_amount_minor=1000,currency='EUR',beneficiary='supplier',
            **({'parent_mandate_id':parent} if parent else {})), 'admin')
        require(created.get('id')==mandate or (created.get('object_id')==mandate and created.get('state')=='admitted'),
                'mandate-not-created-'+suffix)
        action=self.trust('request',command('request-'+suffix,mandate_id=mandate,
            resource='invoice/native',amount_minor=25,currency='EUR',beneficiary='supplier',
            idempotency_key='once-'+suffix),'requester')
        require(action['state']=='pending_approval','request-not-pending-'+suffix)
        if approve:
            approved=self.trust('approve',command('approve-'+suffix,action_id=action['id'],**{
                k:action[k] for k in ['mandate_id','content_digest','resource','amount_minor','currency','beneficiary']}),'approver')
            require(approved['state']=='approved','action-not-approved-'+suffix)
        return action

    def step(self, suffix, action, **terms):
        """Declare untrusted native terms; the host supplies every effective scope."""
        return {'request':dict(type='swarm.trust.request',schema_version=1,id='rpc-'+suffix,
            operation='commit',input=command('commit-'+suffix,action_id=action['id'],**terms))}

    def execute(self, steps):
        """Run the real native supervisor, proxy and descendant under Biba.

        Retain original process result and descendant observation. Success here
        means reaping, not task acceptance or success of every requested action.
        """
        private=self.f.root/'.swarm/native-untrusted-write'
        publish(self.f.workspace/'native-plan.json',dict(private_target=str(private),steps=steps))
        result=self.f.ok(['agent','run','session'])
        observed=json.loads((self.f.workspace/'native-observed.json').read_bytes())
        publish(self.output/'native-observed.json',observed)
        publish(self.output/'native-result.json',result)
        self.check('native-proxy-is-registered-leader',observed['ppid']==result['agent']['child_pid']
                   and observed['pid']!=result['agent']['child_pid'])
        self.check('biba-root-write-refused',observed['biba_write_errno']==errno.EACCES and not private.exists())
        self.check('native-received-exact-sealed-prompt-and-protocol',observed['stdin_sha256']==digest(
            self.prepared['prompt'].encode()+protocol_suffix()))
        self.check('native-completed-without-task-acceptance',result['agent']['status']=='completed'
                   and result['task_accepted'] is False)
        execution=result['agent']['native_execution_result']
        self.check('native-reaped-and-output-complete',execution['kernel_reaped'] and execution['output_complete'])
        return observed



class NativeProcess:
    """Own one public CLI child and deterministic FIFO barriers, with no deadline.

    A pidfd wakes on early exit; FIFO readiness wakes on observed backend progress.
    These local fixtures do not authorize native calls. Retain outputs and roots;
    on a test failure request stop through the public CLI and release its barrier.
    """
    def __init__(self, run, steps, agent='session', workspace=None):
        self.r=run; self.agent=agent; self.workspace=workspace or run.f.workspace
        self.notify=self.workspace/'native-notify'; self.go=self.workspace/'native-go'
        os.mkfifo(self.notify,0o600); os.mkfifo(self.go,0o600)
        self.nfd=os.open(self.notify,os.O_RDWR|os.O_NONBLOCK)
        self.gfd=os.open(self.go,os.O_RDWR|os.O_NONBLOCK)
        publish(self.workspace/'native-plan.json',dict(
            private_target=str(run.f.root/'.swarm/native-untrusted-write'),steps=steps))
        self.stdout_path=run.output/(agent+'-run.stdout')
        self.stderr_path=run.output/(agent+'-run.stderr')
        self.out=self.stdout_path.open('xb'); self.err=self.stderr_path.open('xb')
        self.argv=[str(run.f.binary),'agent','run',agent,'--root',str(run.f.root),'--json']
        publish(run.output/(agent+'-run.argv.json'),self.argv)
        self.process=subprocess.Popen(self.argv,env=run.f.environment,stdin=subprocess.DEVNULL,
                                      stdout=self.out,stderr=self.err)
        self.pidfd=os.pidfd_open(self.process.pid)
        self.selector=selectors.DefaultSelector()
        self.selector.register(self.nfd,selectors.EVENT_READ,'progress')
        self.selector.register(self.pidfd,selectors.EVENT_READ,'exit')
        self.closed=False

    def barrier(self, expected):
        """Wait for a named producer observation or an actual early CLI exit."""
        print('WAIT native barrier '+expected,flush=True)
        events=self.selector.select()
        if any(key.data=='progress' for key,_ in events):
            require(os.read(self.nfd,1)==b'R','barrier-notification-invalid')
            observed=json.loads((self.workspace/'native-observed.json').read_bytes())
            require(observed['barriers'][-1]==expected,'wrong-native-barrier')
            publish(self.r.output/(self.agent+'-'+expected+'.json'),observed)
            print('OBSERVED native barrier '+expected,flush=True)
            return observed
        self.process.wait();self.out.flush();self.err.flush()
        raise RuntimeError('native CLI exited before barrier: '+self.stderr_path.read_text())

    def release(self):
        """Release exactly one test barrier; no native right follows from a byte."""
        require(os.write(self.gfd,b'G')==1,'barrier-release-incomplete')

    def finish(self):
        """Observe the real CLI exit and complete result, then close owned FDs."""
        code=self.process.wait(); self.out.close(); self.err.close()
        self.selector.close(); os.close(self.pidfd); os.close(self.nfd); os.close(self.gfd)
        self.closed=True
        require(code==0,'native-cli-failure: '+self.stderr_path.read_text())
        result=json.loads(self.stdout_path.read_bytes())
        observed=json.loads((self.workspace/'native-observed.json').read_bytes())
        require(result['task_accepted'] is False,'native-task-accepted')
        require(result['agent']['native_execution_result']['kernel_reaped']
                and result['agent']['native_execution_result']['output_complete'],'native-output-incomplete')
        require(observed['biba_write_errno']==errno.EACCES,'native-biba-write-not-refused')
        require(observed['ppid']==result['agent']['child_pid'],'native-proxy-parent-mismatch')
        require(observed['stdin_sha256']==digest(self.r.prepared_by_agent[self.agent]['prompt'].encode()+protocol_suffix()),
                'native-async-received-prompt-mismatch')
        publish(self.r.output/(self.agent+'-finished-observation.json'),observed)
        return result,observed

    def stop(self):
        """Stop only this owned fixture through its public supervisor on failure."""
        if not self.closed and self.process.poll() is None:
            self.r.f.ok(['agent','stop',self.agent])
            os.write(self.gfd,b'G')
        if not self.closed:
            self.process.wait(); self.out.close(); self.err.close(); self.selector.close()
            os.close(self.pidfd);os.close(self.nfd);os.close(self.gfd);self.closed=True


def reply_denied(reply):
    """Recognize an explicit broker or business refusal, never an empty success."""
    return reply.get('ok') is False or (reply.get('ok') is True and reply.get('result',{}).get('state')=='denied')


def new_sibling(run, requester=False):
    """Prepare B in the same actual Root with an independent Work/workspace.

    Update only explicit fixture mappings through public Trust configuration,
    before issuing mandates, so no historical mandate is silently refreshed.
    """
    f=run.f
    work=f.ok(['work','create'],dict(schema_version=1,event_id='create-b',expected_revision=0,
        title='Native sibling B',objective='Independent revocation control',scope='Owned fixture',criteria=['Observed']))['work']['id']
    f.ok(['task','add',work],dict(schema_version=1,event_id='task-b',expected_revision=1,id='task-b',
        title='Run B',deliverable='Receipt',criteria=['Observed']))
    f.ok(['budget','apply',work],dict(schema_version=1,event_id='budget-b',expected_revision=2,
        budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Owned native sibling fixture',reference_date='2026-11-09')))
    workspace=f.root/'i0-workspace-b';workspace.mkdir(mode=0o700)
    request=dict(schema_version=1,event_id='session-b',expected_revision=3,task_id='task-b',provider='fixture',
                 capture_output=True,workspace=str(workspace))
    preview=f.ok(['agent','preview',work],request)
    request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
    run.prepared_by_agent['session-b']=f.ok(['agent','prepare',work],request)['agent']
    scope=f.ok(['agent','native-scope','session-b'])
    revision=run.trust('show')['revision']
    run.config.update(expected_revision=revision,event_id='configure-b')
    run.config['identities'].append(dict(subject='native-worker-b',native_scope=scope,integrity=2,roles=['requester','effect'] if requester else ['effect']))
    run.trust('config-apply',run.config,'admin')
    return scope,workspace


def revocation_scenario(binary, output):
    """Same-Root A/B witnesses, A revocation/replay denial, B survival and A recovery."""
    r=Run(binary,output,planned_denials=2); scope_b,workspace_b=new_sibling(r)
    a1=r.action('a1'); a2=r.action('a2'); b1=r.action('b1',scope_b); b2=r.action('b2',scope_b)
    # Revoke both A mandates at the deterministic boundary; B keeps its own.
    a=NativeProcess(r,[r.step('a1',a1),{'barrier':'before-revocation'},
        dict(request=dict(r.step('a1',a1)['request'],id='rpc-a1-replay')),r.step('a2',a2),
        {'barrier':'after-revocation'},{'from':'recovery.json'}])
    b=None
    try:
        first=a.barrier('before-revocation')
        r.check('a-native-witness',first['responses'][0]['result']['state']=='committed')
        b=NativeProcess(r,[r.step('b1',b1),{'barrier':'b-witness'},r.step('b2',b2)],'session-b',workspace_b)
        bfirst=b.barrier('b-witness')
        r.check('b-native-witness-before-revocation',bfirst['responses'][0]['result']['state']=='committed')
        for suffix in ['a1','a2']:
            r.trust('mandate-revoke',command('revoke-'+suffix,mandate_id='mandate-'+suffix),'admin')
        a.release(); denied=a.barrier('after-revocation')
        r.check('a-known-replay-and-new-effect-refused',len(denied['responses'])==3
                and all(reply_denied(v) for v in denied['responses'][1:]))
        b.release();_,bfinal=b.finish()
        r.check('b-survives-a-revocation-on-same-receiver',bfinal['responses'][1]['result']['state']=='committed')
        state=r.trust('show')
        r.check('revocation-keeps-exact-three-known-effects',len(state['ledger'])==3
                and {v['action_id'] for v in state['ledger']}=={a1['id'],b1['id'],b2['id']})
        recovery=r.action('a-recovery')
        publish(r.f.workspace/'recovery.json',r.step('a-recovery',recovery))
        a.release();_,afinal=a.finish()
        r.check('same-agent-recovers-with-new-authorized-mandate',afinal['responses'][-1]['result']['state']=='committed'
                and len(r.trust('show')['ledger'])==4)
        return r
    finally:
        a.stop()
        if b is not None:b.stop()


def biba_scenario(binary, output):
    """An explicit I1 native effect identity cannot invoke the I2 service."""
    r=Run(binary,output,integrity=1,planned_denials=1);action=r.action('biba-low')
    observed=r.execute([r.step('biba-low',action)])
    reply=observed['responses'][0]
    r.check('biba-refuses-authenticated-i1-effect',reply['ok'] is True
            and reply['result']['state']=='denied'
            and reply['result']['reason']=='service d’effet I2 authentifié requis')
    r.check('biba-denial-has-no-ledger-effect',r.trust('show')['ledger']==[])
    return r



def check_apply_scenario(binary, output):
    """Withdraw an identity after a valid effect, then explicitly re-admit it.

    Fresh mandates are issued after each configuration revision. A refusal is
    therefore not masked by an old policy revision. The same native process,
    task, attempt, Root and receiver survive the authorized recovery.
    """
    r=Run(binary,output,planned_denials=2); witness=r.action('identity-witness')
    process=NativeProcess(r,[r.step('identity-witness',witness),{'barrier':'identity-current'},
        {'from':'disabled.json'},{'barrier':'identity-disabled'},{'from':'enabled.json'},
        {'barrier':'identity-enabled'},{'from':'classified.json'}])
    try:
        observed=process.barrier('identity-current')
        r.check('identity-witness-before-withdrawal',observed['responses'][0]['result']['state']=='committed')
        revision=r.trust('show')['revision']
        r.config.update(expected_revision=revision,event_id='disable-native')
        r.config['identities'][-1]['disabled']=True
        r.trust('config-apply',r.config,'admin')
        denied=r.action('identity-disabled')
        publish(r.f.workspace/'disabled.json',r.step('identity-disabled',denied))
        process.release();observed=process.barrier('identity-disabled')
        r.check('current-disabled-identity-refused-with-current-mandate',reply_denied(observed['responses'][-1])
                and len(r.trust('show')['ledger'])==1)
        revision=r.trust('show')['revision']
        r.config.update(expected_revision=revision,event_id='enable-native')
        r.config['identities'][-1]['disabled']=False
        r.trust('config-apply',r.config,'admin')
        recovery=r.action('identity-enabled')
        publish(r.f.workspace/'enabled.json',r.step('identity-enabled',recovery))
        process.release();observed=process.barrier('identity-enabled')
        r.check('identity-explicit-recovery-on-same-execution',observed['responses'][-1]['result']['state']=='committed'
                and len(r.trust('show')['ledger'])==2)
        revision=r.trust('show')['revision']
        policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public')],
                    compartments=[],clearances=dict(admin=dict(level='public',compartments=[])),preparation_quarantine=False)
        r.trust('blp-policy-apply',dict(schema_version=1,event_id='native-blp-policy',expected_revision=0,
            expected_trust_revision=revision,reason='Explicit native egress refusal fixture',policy=policy),'admin')
        classified=r.action('classified-no-grant')
        publish(r.f.workspace/'classified.json',r.step('classified-no-grant',classified))
        process.release();_,observed=process.finish()
        reply=observed['responses'][-1]
        r.check('configured-blp-does-not-become-native-release-authority',reply['ok'] is False
                and reply['reason']=='native-trust-confidentiality-not-admitted'
                and len(r.trust('show')['ledger'])==2)
        return r
    finally:process.stop()


def delegation_scenario(binary, output):
    """Native scope narrowing, local bearer refusal and live ancestor revocation."""
    r=Run(binary,output,planned_denials=2)
    revision=r.trust('show')['revision']
    r.config.update(expected_revision=revision,event_id='enable-delegation',delegation=dict(max_depth=2))
    r.trust('config-apply',r.config,'admin')
    r.action('parent')
    child=r.action('child',parent='mandate-parent')
    pending=r.action('child-pending',parent='mandate-parent')
    for suffix,scope in [('scope-removed',None),('scope-switched',dict(r.scope,attempt_id='other-attempt'))]:
        fields=dict(parent_mandate_id='mandate-parent',mandate_id=suffix,subject='requester',actions=['settle'],
                    resource='invoice/native',max_amount_minor=1000,currency='EUR',beneficiary='supplier')
        if scope is not None:fields['native_scope']=scope
        rejected=r.trust('mandate-create',command('create-'+suffix,**fields),'admin')
        r.check('delegation-'+suffix+'-refused',rejected['state']=='denied'
                and suffix not in r.trust('show')['mandates'])
    r.f.environment['SWARM_TRUST_TOKEN']=TOKENS['admin']
    try:code,reply=r.f.call(['trust','commit'],command('bearer-bypass',action_id=child['id']))
    finally:r.f.environment.pop('SWARM_TRUST_TOKEN',None)
    r.check('local-effect-bearer-cannot-bypass-native-binding',code!=0
            and reply['failure']['code']=='authorization_required' and r.trust('show')['ledger']==[])
    process=NativeProcess(r,[r.step('child',child),{'barrier':'child-current'},
        dict(request=dict(r.step('child',child)['request'],id='rpc-child-replay')),r.step('child-pending',pending),
        {'barrier':'parent-revoked'},{'from':'new-chain.json'}])
    try:
        observed=process.barrier('child-current')
        r.check('native-delegation-valid-witness',observed['responses'][0]['result']['state']=='committed')
        r.trust('mandate-revoke',command('revoke-parent',mandate_id='mandate-parent'),'admin')
        process.release();observed=process.barrier('parent-revoked')
        r.check('ancestor-revocation-refuses-cached-and-fresh-child',all(reply_denied(v) for v in observed['responses'][1:])
                and len(r.trust('show')['ledger'])==1)
        r.action('new-parent')
        recovery=r.action('new-child',parent='mandate-new-parent')
        publish(r.f.workspace/'new-chain.json',r.step('new-child',recovery))
        process.release();_,observed=process.finish()
        r.check('new-authorized-chain-recovers-same-native-agent',observed['responses'][-1]['result']['state']=='committed'
                and len(r.trust('show')['ledger'])==2)
        return r
    finally:process.stop()


def final_intent_scenario(binary, output):
    """Codex-shaped final assistant text remains an intent until host mediation."""
    r=Run(binary,output,planned_denials=0); action=r.action('final')
    step=r.step('final',action);step['final_intent']=True
    observed=r.execute([step])
    r.check('final-intent-consumed-by-host-not-backend-tool',observed['responses']==[])
    state=r.trust('show')
    r.check('final-intent-has-one-authorized-effect',len(state['ledger'])==1 and state['ledger'][0]['action_id']==action['id'])
    scope_b,workspace_b=new_sibling(r,requester=True)
    r.trust('mandate-create',command('create-native-request-b',mandate_id='native-request-b',
        subject='native-worker-b',native_scope=scope_b,actions=['settle'],resource='invoice/native',
        max_amount_minor=1000,currency='EUR',beneficiary='supplier'),'admin')
    request=dict(type='swarm.trust.request',schema_version=1,id='rpc-request-b',operation='request',input=command(
        'request-by-native-b',mandate_id='native-request-b',resource='invoice/native',amount_minor=25,
        currency='EUR',beneficiary='supplier',idempotency_key='native-request-once'))
    process=NativeProcess(r,[dict(request=request),{'barrier':'native-request-recorded'},{'from':'approved-final.json'}],
                          'session-b',workspace_b)
    try:
        observed=process.barrier('native-request-recorded');reply=observed['responses'][0]
        r.check('native-request-is-proposal-not-effect',reply['ok'] and reply['result']['state']=='pending_approval'
                and reply['result']['content_redacted'] is True and len(r.trust('show')['ledger'])==1)
        official=r.trust('show')['actions'][reply['result']['object_id']]
        approved=r.trust('approve',command('approve-native-b',action_id=official['id'],**{
            k:official[k] for k in ['mandate_id','content_digest','resource','amount_minor','currency','beneficiary']}),'approver')
        require(approved['state']=='approved','native-proposal-not-separately-approved')
        step=r.step('approved-final-b',official);step.update(final_intent=True,format='claude')
        publish(workspace_b/'approved-final.json',step)
        process.release();result,observed=process.finish()
        r.check('claude-shaped-final-intent-after-separate-approval',result['agent']['status']=='completed'
                and len(observed['responses'])==1 and len(r.trust('show')['ledger'])==2)
    finally:process.stop()
    return r


def main():
    """Healthy native witness, wrong task, target and approval refusals and audit.

    Six maintained native cases cover all scope coordinates, Biba, delegation,
    current identity changes, selective revocation/replay and provider final intent.
    No real model or external payment is established. Failures retain Roots.
    """
    if sys.flags.optimize:
        raise RuntimeError('optimization forbidden: imported fixture uses assertions')
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--build-record',type=Path)
    parser.add_argument('--scenario',choices=['all','basic','biba','delegation','check_apply','revocation','final_intent'],default='all')
    args=parser.parse_args()
    r=None
    try:
        binary=args.binary.resolve(); output=args.output.resolve()
        binary_hash=digest(binary.read_bytes())
        if args.scenario=='all':
            output.mkdir(mode=0o700,parents=True,exist_ok=False)
            if args.build_record:
                record=json.loads(args.build_record.read_bytes())
                require(record['binary_sha256']==binary_hash,'build-record-image-mismatch')
                publish(output/'build-record.json',args.build_record.read_bytes())
                profile=Path(record['evidence'])/('corrected-number' if (Path(record['evidence'])/'corrected-number').exists() else 'build')/'profile/result.json'
                qualification=json.loads(profile.read_bytes())
                require(qualification['sha256']==binary_hash and qualification['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE'
                        and qualification['no_fixture_feature'] is True
                        and qualification['all_core_features_empty'] is True
                        and qualification['all_core_test_false'] is True,'default-profile-not-qualified')
                inputs=json.loads((profile.parent/'after.json').read_bytes())
                suffix_source='rust/crates/swarm-engine/src/native_provider_proxy.rs'
                repo=Path(__file__).resolve().parent.parent
                require(inputs[suffix_source]==digest((repo/suffix_source).read_bytes()),'suffix-source-not-compiled-input')
                publish(output/'profile-result.json',profile.read_bytes())
                publish(output/'effective-inputs.json',inputs)
            protocol=[]
            for mode,expected in [('valid',0),('stale',1),('empty',1)]:
                code=proxy_protocol_probe(binary,output/('protocol-'+mode),mode)
                require(code==expected,'proxy-protocol-'+mode+'-wrong-exit')
                protocol.append('proxy-protocol-'+mode)
                print('PASS '+protocol[-1],flush=True)
            code,reply,agent,journal=unmapped_final_intent_probe(binary,output/'unmapped-intent')
            require(code!=0 and agent['status']=='failed' and not journal,
                    'unmapped-final-intent-silently-completed')
            require('native-trust-incomplete' in reply['failure']['causes'],'unmapped-intent-cause-missing')
            protocol.append('unmapped-final-intent-explicit-refusal')
            print('PASS '+protocol[-1],flush=True)
            cases=['basic','biba','delegation','check_apply','revocation','final_intent']
            summaries={}
            for case in cases:
                log=output/(case+'.log')
                print('RUN native scenario '+case,flush=True)
                with log.open('xb') as stream:
                    completed=subprocess.run([sys.executable,__file__,'--binary',str(binary),
                        '--scenario',case,'--output',str(output/case)],stdout=stream,stderr=subprocess.STDOUT)
                if completed.returncode:
                    print(log.read_text()[-4000:],flush=True)
                    raise RuntimeError('native scenario failed: '+case)
                summary=json.loads((output/case/'summary.json').read_bytes())
                require(summary['status']=='PASS' and summary['assertions'],'native-scenario-no-proofs')
                expected={
                    'basic':{'valid-native-effect','known-response-loss-reconciles-without-second-effect',
                        'root-work-attempt-and-agent-bindings-refuse-reuse','native-effect-does-not-grant-object-read',
                        'cross-task-and-spoofed-identity-refused','missing-approval-refused','wrong-resource-refused',
                        'one-exact-ledger-effect','durable-host-trace-covers-every-request-and-refusal',
                        'native-audit-has-exact-current-scope','native-proof-chain-verifies'},
                    'biba':{'biba-refuses-authenticated-i1-effect','biba-denial-has-no-ledger-effect'},
                    'delegation':{'delegation-scope-removed-refused','delegation-scope-switched-refused',
                        'local-effect-bearer-cannot-bypass-native-binding','native-delegation-valid-witness',
                        'ancestor-revocation-refuses-cached-and-fresh-child','new-authorized-chain-recovers-same-native-agent'},
                    'check_apply':{'identity-witness-before-withdrawal',
                        'current-disabled-identity-refused-with-current-mandate','identity-explicit-recovery-on-same-execution',
                        'configured-blp-does-not-become-native-release-authority'},
                    'revocation':{'a-native-witness','b-native-witness-before-revocation',
                        'a-known-replay-and-new-effect-refused','b-survives-a-revocation-on-same-receiver',
                        'revocation-keeps-exact-three-known-effects','same-agent-recovers-with-new-authorized-mandate'},
                    'final_intent':{'final-intent-consumed-by-host-not-backend-tool',
                        'final-intent-has-one-authorized-effect','native-request-is-proposal-not-effect',
                        'claude-shaped-final-intent-after-separate-approval'},
                }[case]
                require(expected.issubset(set(summary['assertions'])),'native-scenario-required-observation-missing')
                require(len(summary['assertions'])==len(set(summary['assertions'])),'native-scenario-duplicate-observation')
                summaries[case]=summary
                print('PASS native scenario '+case+' assertions='+str(len(summary['assertions'])),flush=True)
            require(digest(binary.read_bytes())==binary_hash,'binary-changed-during-recipe')
            if args.build_record:
                require(all(digest((repo/p).read_bytes())==h for p,h in inputs.items()),'compiled-input-changed-during-recipe')
            total=sum(len(v['assertions']) for v in summaries.values())+len(protocol)
            publish(output/'summary.json',dict(status='PASS',cases=6,assertions=total,protocol=protocol,scenarios=summaries,
                binary_sha256=binary_hash,scope='actual native Linux workers; deterministic provider; simulated connector'))
            print('NATIVE_TASK_MANDATE_VERIFIED cases=6 assertions='+str(total),flush=True)
            return
        if args.scenario!='basic':
            function={'biba':biba_scenario,'delegation':delegation_scenario,'check_apply':check_apply_scenario,
                      'revocation':revocation_scenario,'final_intent':final_intent_scenario}[args.scenario]
            r=function(binary,output)
            publish(output/'summary.json',dict(status='PASS',scenario=args.scenario,assertions=r.passed,
                root=str(r.f.root),binary_sha256=digest(binary.read_bytes()),native_scope=r.scope,
                scope='deterministic native protocol; simulated Trust connector'))
            print('NATIVE_TASK_MANDATE_PASS '+args.scenario+' '+str(len(r.passed)),flush=True)
            return
        r=Run(binary,output,planned_denials=7)
        publish(output/'binary.json',dict(path=str(binary),sha256=digest(binary.read_bytes()),
            profile='default Cargo features; separate delivery qualification required'))
        valid=r.action('valid')
        other=dict(r.scope,task_id='other-task')
        wrong=r.action('wrong-task',other)
        unapproved=r.action('unapproved',approve=False)
        wrong_target=r.action('wrong-target')
        steps=[r.step('valid',valid,lose_response=True),dict(request=dict(type='swarm.trust.request',schema_version=1,
            id='rpc-reconcile-valid',operation='reconcile',input=command('reconcile-valid',action_id=valid['id']))),
               r.step('wrong-task',wrong,actor='admin',integrity=3),
               r.step('unapproved',unapproved),r.step('wrong-target',wrong_target,resource='invoice/other')]
        for coordinate in ['root_digest','work_id','attempt_id','agent_id']:
            changed=dict(r.scope)
            changed[coordinate]='b'*64 if coordinate=='root_digest' and r.scope[coordinate]!='b'*64 else 'other-'+coordinate
            if coordinate=='root_digest' and len(changed[coordinate])!=64:changed[coordinate]='c'*64
            bad_action=r.action('wrong-'+coordinate.replace('_','-'),changed)
            steps.append(r.step('wrong-'+coordinate.replace('_','-'),bad_action))
        observed=r.execute(steps)
        replies=observed['responses']
        r.check('valid-native-effect',len(replies)==9 and replies[0]['ok'] and replies[0]['result']['state']=='unknown')
        r.check('native-effect-does-not-grant-object-read',replies[0]['result'].get('content_redacted') is True
                and not any(k in replies[0]['result'] for k in ['resource','amount_minor','currency','beneficiary','content_digest']))
        r.check('known-response-loss-reconciles-without-second-effect',replies[1]['ok'] and replies[1]['result']['state']=='reconciled')
        r.check('root-work-attempt-and-agent-bindings-refuse-reuse',all(reply_denied(v) for v in replies[5:]))
        r.check('cross-task-and-spoofed-identity-refused',replies[2]['ok'] is False)
        r.check('missing-approval-refused',replies[3]['ok'] and replies[3]['result']['state']=='denied')
        r.check('wrong-resource-refused',replies[4]['ok'] and replies[4]['result']['state']=='denied')
        state=r.trust('show')
        r.check('one-exact-ledger-effect',len(state['ledger'])==1 and state['ledger'][0]['action_id']==valid['id'])
        logs=r.f.ok(['agent','logs','session'])
        tool_logs=[json.loads(row['message']) for row in logs if row['kind']=='native-trust']
        r.check('durable-host-trace-covers-every-request-and-refusal',len(tool_logs)==9
                and sum(v['failed'] for v in tool_logs)==7
                and all(v['agent_id']=='session' and v['attempt_id']==r.scope['attempt_id']
                        and v['delivered'] is False for v in tool_logs))
        exported=r.trust('export')
        matching=[e for e in exported['evidence'] if e['kind']=='commit' and e.get('result')!='denied'
                  and e.get('identity_context',{}).get('principal_kind')=='native_agent']
        r.check('native-audit-has-exact-current-scope',matching and all(
            e['identity_context']['native_scope']==r.scope and
            e['identity_context']['assurance']['method']=='native_owned_execution' and
            e['identity_context']['token_digest']=='' for e in matching))
        anchor=r.trust('anchor')
        key=(r.f.root/'.swarm/trust-actions-proof.key').read_text()
        verified=r.trust('verify',dict(export=exported,anchor=anchor,verification_key=key))
        r.check('native-proof-chain-verifies',verified['verified'] is True)
        publish(output/'summary.json',dict(status='PASS',assertions=r.passed,root=str(r.f.root),
                                          native_scope=r.scope,scope='deterministic native broker; simulated connector'))
        print('NATIVE_TASK_MANDATE_PASS '+str(len(r.passed)),flush=True)
    except BaseException as error:
        if r is not None:
            publish(r.output/'failure.json',dict(type=type(error).__name__,error=str(error),trace=traceback.format_exc()))
        raise

if __name__=='__main__':
    main()
