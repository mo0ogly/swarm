#!/usr/bin/env python3
"""Public Rust draft writes on a fresh owned Root: replay, CAS, history, policy,
protected HTTP effects and actual Admin consumption. Receipt corruption is injected
only into this owned fixture. Atomic SQL faults use the
separate retained-Root Rust integration test. No provider or live DB writes,
no implicit retry/global watchdog. Original channels and named assertions persist.
"""
import argparse,hashlib,json,os,sqlite3,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Exercise supported CLI/API with valid controls before each injected fault.
    Product mutations use public paths; receipt corruption touches only the owned
    fixture, while atomic SQL faults have a separate Root test.
    """
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);binary=Path(a.binary).resolve();d=Driver(binary,out);checks=[]
    def check(name,value):
        """Retain distinct assertions; a false result stops the journey."""
        assert value,name;assert name not in checks,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def refused(args,value,label,expected_exit=2):
        """Require nonzero/no success and exact business DB stability."""
        before=d.database();code,stdout,stderr=d.call(args,value);check(label,code==expected_exit and not stdout and bool(json.loads(stderr)['failure']['code']) and d.database()==before)
    def save(event,revision,kind,text):
        """Build an explicit versioned intention, never invent an observed revision."""
        return dict(version=1,action='save',preparation_id=pid,event_id=event,expected_revision=revision,document=kind,text=text)
    d.ok(['init']);policy=d.ok(['engine-settings','show']);check('current-schema42',policy['schema_version']==42)
    create=dict(version=1,action='create',event_id='create-test',expected_revision=0,title=' Title <b> ',text='é besoin')
    p=d.ok(['prepare','create'],create);pid=p['id'];check('create-real-content',pid=='prep-'+hashlib.sha256(b'create-test').hexdigest()[:24] and p['revision']==1 and p['title']=='Title <b>' and p['method']=='apex' and p['documents']['besoin']['text']=='é besoin' and p['plan_ready'] is False)
    before=d.database();check('replay-create-equal',d.ok(['prepare','create'],create)==p and d.database()==before)
    check('initial-history',d.ok(['prepare','history',pid,'besoin'])==[p['documents']['besoin']])
    # Fault injection only in this newly owned Root, never a live datastore.
    valid_state=d.database()
    with sqlite3.connect(d.root/'.swarm/state.db') as db:
        receipt=db.execute('SELECT response FROM preparation_commands WHERE preparation_id=? AND event_id=?',(pid,create['event_id'])).fetchone()[0]
        altered=json.loads(receipt);altered['title']='altered fixture receipt'
        db.execute('UPDATE preparation_commands SET response=? WHERE preparation_id=? AND event_id=?',(json.dumps(altered).encode(),pid,create['event_id']))
    before=d.database();code,stdout,stderr=d.call(['prepare','create'],create)
    check('current-receipt-corruption-refused',code==2 and not stdout and json.loads(stderr)['failure']['code']=='preparation-receipt-identity-mismatch' and d.database()==before)
    with sqlite3.connect(d.root/'.swarm/state.db') as db:
        db.execute('UPDATE preparation_commands SET response=? WHERE preparation_id=? AND event_id=?',(receipt,pid,create['event_id']))
    check('current-receipt-repair-same-event',d.ok(['prepare','create'],create)==p and d.database()==valid_state)
    brief=save('save-brief',1,'brief','literal <script>é</script>');p2=d.ok(['prepare','save',pid],brief)
    check('save-increments-and-links',p2['revision']==2 and p2['documents']['brief']['revision']==1 and p2['documents']['brief']['need_hash']==p['documents']['besoin']['sha256'] and not p2['plan_ready'])
    before=d.database();old=d.ok(['prepare','create'],create);check('historical-receipt',old['revision']==1 and old['receipt_historical'] is True and d.database()==before)
    refused(['prepare','save',pid],{**brief,'text':'changed'},'event-conflict-no-mutation',3)
    refused(['prepare','save',pid],save('stale',1,'plan','stale'),'stale-revision-no-mutation',3)
    refused(['prepare','save','another-id'],brief,'positional-id-no-mutation')
    refused(['prepare','create'],{**create,'event_id':'missing-work','work_id':'not-a-work'},'missing-linked-work')
    refused(['prepare','create'],{**create,'event_id':'unknown-method','method':'../unknown'},'unknown-method-exit5',5)
    for label,extra in [('null-revision',{'expected_revision':None}),('unknown-field',{'surprise':1}),('foreign-action',{'action':'validate-plan'}),('invalid-kind',{'document':'notes'}),('foreign-text-type',{'text':True})]:
        refused(['prepare','save',pid],{**save(label,2,'plan','body'),**extra},label)
    d.settings(preparation_document_max_bytes='1',preparation_default_method='debug')
    refused(['prepare','save',pid],save('byte-bound',2,'plan','é'),'byte-cap-refusal')
    d.settings(preparation_document_max_bytes='2');p3=d.ok(['prepare','save',pid],save('byte-bound',2,'plan','é'));check('byte-cap-recovery',p3['revision']==3 and p3['documents']['plan']['text']=='é' and not p3['plan_ready'])
    d.settings(preparation_write_enabled='0');refused(['prepare','create'],create,'disabled-denies-replay',4);check('disabled-reads-retained',d.ok(['prepare','show',pid])['revision']==3)
    d.settings(preparation_write_enabled='1',preparation_document_max_bytes=None,preparation_title_max_bytes=None)
    p4=d.ok(['prepare','save',pid],save('long-unlimited',3,'plan','x'*16001));check('explicit-unlimited-consumed',p4['revision']==4 and len(p4['documents']['plan']['text'])==16001)
    default=d.ok(['prepare','create'],dict(version=1,action='create',event_id='method-default',expected_revision=0));check('saved-default-method',default['method']=='debug')
    # SQL child-insert rollback is covered by the retained-Root Rust integration
    # test; CLI correctly refuses injected schema objects before reaching writes.
    recovered=d.ok(['prepare','save',pid],save('valid-before-output',4,'plan','valid witness'))
    check('valid-control-before-output',recovered['revision']==5)
    # Known replay must survive a real stdout ENOSPC, not an invented error text.
    value=save('output-failure',5,'plan','retained output');path=out/'output-request.json';publish(path,value)
    with open('/dev/full','wb',buffering=0) as full:
        result=subprocess.run([str(binary),'prepare','save',pid,'--root',str(d.root),'--json','--input',str(path)],stdout=full,stderr=subprocess.PIPE)
    publish(out/'output-failure.stderr',result.stderr)
    error=json.loads(result.stderr);check('output-failure-known-receipt',result.returncode==2 and error['recorded_preparation']==dict(id=pid,revision=6))
    check('output-failure-replay',d.ok(['prepare','save',pid],value)['revision']==6)
    d.settings(preparation_list_limit='2',preparation_default_method='apex')
    work=d.ok(['work','create'],dict(schema_version=1,event_id='write-admin-fixture',expected_revision=0,title='Owned Admin consumer',objective='Verify saved draft policy',scope='Isolated fixture without execution',criteria=['No provider']))['work']['id']
    server=native.Server(binary,d.root,out,'writes');server.capture_async(out/'http-observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        api=save('api-save',6,'besoin','HTTP saved')
        before=d.database();server.request('/api/v1/preparations/command',api,authorized=False,status=403,observe=False);server.request('/api/v1/preparations/command',api,status=403,observe=False);check('api-auth-csrf-no-mutation',d.database()==before)
        server.request('/api/v1/preparations/command',status=405,observe=False)
        response=server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False);check('api-writes-shared-store',response['revision']==7 and d.ok(['prepare','show',pid])==response)
        before=d.database();check('api-replay-no-write',server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False)==response and d.database()==before)
        server.request('/api/v1/preparations/command',save('api-stale',6,'plan','stale'),csrf=csrf,status=409,observe=False);check('api-conflict-no-write',d.database()==before)
        server.request('/api/v1/preparations/command',{**api,'event_id':'api-bad','document':'bad'},csrf=csrf,status=400,observe=False);check('api-malformed-no-write',d.database()==before)
        server.request('/api/v1/preparations/command',{**create,'event_id':'api-unknown-method','method':'../unknown'},csrf=csrf,status=400,observe=False);check('api-unknown-method-no-write',d.database()==before)
        cfg=dict(address=server.address,key=server.key,root=str(d.root),work=work,output=str(out))
        with (out/'browser.stdout').open('xb') as o,(out/'browser.stderr').open('xb') as e:r=subprocess.run(['node','tests/rust_preparation_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=o,stderr=e)
        check('Admin-browser',r.returncode==0)
        check('Admin-method-persisted',d.ok(['engine-settings','show'])['values']['preparation_default_method']=='debug')
        created=d.ok(['prepare','create'],dict(version=1,action='create',event_id='Admin-method-consumer',expected_revision=0))
        check('Admin-method-real-consumer',created['method']=='debug')
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    records=[json.loads(line) for line in (out/'http-observer.raw').read_text().splitlines()]
    receipts=[r['outcome'].get('recorded_preparation') for r in records if r.get('event')=='web-outcome']
    check('http-owner-receipt-independent',sum(r==dict(id=pid,revision=7) for r in receipts)>=2)
    check('history-after-reopen',d.ok(['prepare','show',pid])['revision']==7 and d.ok(['prepare','history',pid,'besoin'])[0]['text']=='HTTP saved')
    verify_glm_boundaries(d,create,pid,check)
    publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),fixture_root=str(d.root),global_watchdog=None,limitations=['No provider/adoption/editor UI or native business authorization proof']))
    print('PREPARATION_WRITES_PASS '+str(len(checks))+' assertions',flush=True)

def verify_glm_boundaries(d,create,pid,check):
    """GLM-proposed boundary/cross-Root cases adapted to the actual Go contract.
    The original proposal's automatic ID fill, unconditional historical marker
    and mandatory Work were rejected against actual producers. No SQL here.
    """
    before=d.database()
    replay=d.ok(['prepare','create'],{**create,'organization':None,'source':None,'budget':None,'adopt_brief':False,'expected_work_revision':None,'decisions':[],'turn_id':'','sha256':''})
    check('legacy-zero-fields-same-receipt',replay['revision']==1 and replay['receipt_historical'] and d.database()==before)
    d.settings(preparation_title_max_bytes='3')
    intent=dict(version=1,action='create',event_id='glm-title-boundary',expected_revision=0,title='éé',method='apex')
    before=d.database();code,out,err=d.call(['prepare','create'],intent);check('glm-title-utf8-refusal',code==2 and not out and bool(err) and d.database()==before)
    exact=d.ok(['prepare','create'],{**intent,'event_id':'glm-title-exact','title':'éa'});check('glm-title-exact-boundary',exact['title']=='éa')
    d.settings(preparation_title_max_bytes='4');saved=d.ok(['prepare','create'],intent);check('glm-title-same-event-recovery',saved['title']=='éé')
    d.settings(preparation_title_max_bytes='0')
    default=d.ok(['prepare','create'],dict(version=1,action='create',event_id='title-default-zero',expected_revision=0))
    check('title-cap-raw-input-default-exempt',default['title']=='Nouvelle préparation')
    before=d.database();code,out,err=d.call(['prepare','create'],dict(version=1,action='create',event_id='title-zero-spaces',expected_revision=0,title=' '));check('title-cap-before-trim',code==2 and not out and bool(err) and d.database()==before)
    d.settings(preparation_title_max_bytes='10')
    default=d.ok(['prepare','create'],dict(version=1,action='create',event_id='title-spaces-default',expected_revision=0,title='   '))
    check('title-cap-whitespace-default-exempt',default['title']=='Nouvelle préparation')
    twin_out=d.out/'glm-cross-root';twin_out.mkdir();twin=Driver(d.binary,twin_out);twin.ok(['init']);other=twin.ok(['prepare','create'],intent);check('glm-deterministic-id-cross-root',other['id']==saved['id'])
    d.settings(preparation_method_max_bytes='0')
    draft=d.ok(['prepare','create'],dict(version=1,action='create',event_id='glm-unavailable-method',expected_revision=0));check('unavailable-method-draft-retained',not draft.get('method_hash') and not draft['plan_ready'])
    d.settings(preparation_document_max_bytes='0')
    current=d.ok(['prepare','show',pid]);empty=dict(version=1,action='save',preparation_id=pid,event_id='glm-zero-empty',expected_revision=current['revision'],document='plan',text='')
    result=d.ok(['prepare','save',pid],empty);check('glm-zero-cap-empty-saved',result['documents']['plan']['text']=='')
    before=d.database();code,out,err=d.call(['prepare','save',pid],{**empty,'event_id':'glm-zero-nonempty','expected_revision':result['revision'],'text':'a'});check('glm-zero-cap-refusal',code==2 and not out and bool(err) and d.database()==before)

if __name__=='__main__':main()
