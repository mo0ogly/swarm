#!/usr/bin/env python3
"""Compare real frozen Go/native wait producers on equivalent private wait data.

No provider or live mission. Reference project source/archive qualification is
required; this checks a bounded data/read/replay contract, not global CLI parity.
Native source/preview projections, new-row actor/time and safe diagnostic envelopes
are explicit differences; original history metadata and JSON types are preserved.
"""
import argparse,json,subprocess,tempfile,traceback
from pathlib import Path
import rust_agent_model_route as proof

CORE={'schema_version','revision','values','history'}
EXTRA={'source','digest','exact','lease_min_seconds','lease_max_seconds','duration_max_seconds'}
def canonical(value):return json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':'))
def run(native,reference,output):
    """Retain original commands/status/bytes before named strict data assertions.
    Independent minimal environments contain no credentials or executable tools.
    CLI init creates only private state; direct writes are fixture-only source data.
    """
    output.mkdir(parents=True,exist_ok=False);proof.publish(output/'harness-source.py',Path(__file__).read_bytes());ref=json.loads(reference.read_bytes());assert ref['status']=='QUALIFIED_PROJECT_SOURCE_REFERENCE';go=Path(ref['binary']);assert proof.digest(go.read_bytes())==ref['sha256'];inventory=json.loads((reference.parent/'inputs.before.json').read_bytes())
    for name,h in inventory.items():
        if name.endswith('.go') or name=='config/provider-wait.json':assert proof.digest(Path(name).read_bytes())==h,('reference-producer-changed',name)
    roots={name:Path(tempfile.mkdtemp(prefix='swarm-wait-parity-'+name+'-')) for name in ['go','native']};binaries={'go':go,'native':native};commands=[];checks=[];serial=0
    def call(kind,args,value=None,success=True):
        """One real CLI call with raw output and exact input/return retained."""
        nonlocal serial
        serial+=1;label=str(serial).zfill(3)+'-'+kind;root=roots[kind];(root/'empty-bin').mkdir(exist_ok=True);argv=[str(binaries[kind]),*args,'--root',str(root),'--json']
        if value is not None:
            file=output/(label+'.input.json');proof.publish(file,value);argv+=['--input',str(file)]
        r=subprocess.run(argv,env={'PATH':str(root/'empty-bin'),'LANG':'C.UTF-8','USER':'fixture'},stdout=subprocess.PIPE,stderr=subprocess.PIPE);proof.publish(output/(label+'.stdout'),r.stdout);proof.publish(output/(label+'.stderr'),r.stderr);commands.append({'argv':argv,'exit':r.returncode});proof.publish(output/'commands.json',commands)
        assert (r.returncode==0)==success,(kind,args,r.returncode,r.stderr.decode());return json.loads(r.stdout) if success else None
    def source(raw):
        """Set identical fixture-only bytes; None means absent, never live state."""
        for root in roots.values():
            file=root/'.swarm/provider-wait.json'
            if raw is None:file.unlink(missing_ok=True)
            else:proof.publish(file,raw if isinstance(raw,bytes) else (json.dumps(raw)+'\n').encode())
    def same(label,success=True):
        """Compare exact core JSON types/data and refuse without source mutation."""
        before={k:(r/'.swarm/provider-wait.json').read_bytes() if (r/'.swarm/provider-wait.json').exists() else None for k,r in roots.items()};results={k:call(k,['run-limits','provider-wait','show'],success=success) for k in roots}
        if success:
            assert set(results['go'])==CORE;assert set(results['native'])==CORE|EXTRA;assert canonical(results['go'])==canonical({k:results['native'][k] for k in CORE}),label
        for k,r in roots.items():assert ((r/'.swarm/provider-wait.json').read_bytes() if (r/'.swarm/provider-wait.json').exists() else None)==before[k],label
        checks.append(label);proof.publish(output/'checks.json',checks);return results['go']
    for k in roots:call(k,['init'])
    proof.publish(output/'roots.json',{k:str(r) for k,r in roots.items()});proof.publish(output/'binary-identities.json',{k:proof.digest(b.read_bytes()) for k,b in binaries.items()})
    defaults=same('absent-defaults-core-json-types-equal');values=defaults['values'];source(('{"schema_version":0,"SCHEMA_VERSION":1,"revision":0,"values":'+json.dumps(values)+',"VALUES":null,"history":null}').encode());same('go-null-history-case-folded-duplicate-null-merge')
    legacy={**defaults,'revision':1,'history':[{'revision':1,'values':values,'actor':'Historical actor','at':'Unverified historical time','change':None}]};source(legacy);same('historical-null-change-read-preserved')
    unusual=json.loads(json.dumps(legacy));unusual['history'][0]['change']={'schema_version':-1,'event_id':'../legacy','expected_revision':123,'values':{'silence_seconds':-1,'max_duration_seconds':-1,'planning_lease_seconds':0},'reason':''};source(unusual);same('historical-change-metadata-is-not-new-request-validation')
    requests=[{'schema_version':1,'event_id':'historical-duplicate','expected_revision':i,'values':{**values,'max_duration_seconds':i+1},'reason':'Historical reason '+str(i)} for i in range(2)];duplicated={**defaults,'revision':2,'values':requests[1]['values'],'history':[{'revision':i+1,'values':r['values'],'actor':'Historical actor '+str(i),'at':'Historical time '+str(i),'change':r} for i,r in enumerate(requests)]};source(duplicated);same('historical-repeated-event-id-read-preserved')
    for kind in roots:
        file=roots[kind]/'.swarm/provider-wait.json';before=file.read_bytes();response=call(kind,['run-limits','provider-wait','apply'],requests[0]);assert response['config']['revision']==2 and response['config']['values']==requests[1]['values'] and file.read_bytes()==before
        call(kind,['run-limits','provider-wait','apply'],requests[1],success=False);assert file.read_bytes()==before
    checks.append('first-match-replay-and-conflict-for-repeated-history-id')
    source(legacy);same('valid-control-before-invalid-row-sequence')
    bad=json.loads(json.dumps(legacy));bad['history'][0]['revision']=2;source(bad);same('invalid-row-sequence-refuses-without-mutation',False)
    source(legacy);same('valid-control-before-current-last-divergence')
    bad=json.loads(json.dumps(legacy));bad['values']={**values,'max_duration_seconds':1};source(bad);same('current-last-values-divergence-refuses-without-mutation',False)
    source(legacy);request={'schema_version':1,'event_id':'post-legacy-change','expected_revision':1,'values':{**values,'max_duration_seconds':1},'reason':'Owned new change after historical metadata'}
    previews={kind:call(kind,['run-limits','provider-wait','preview'],request) for kind in roots}
    for bad in [{**request,'schema_version':1.0},{**request,'event_id':'../bad'},{**request,'values':{**values,'planning_lease_seconds':0}}]:
        for kind in roots:
            file=roots[kind]/'.swarm/provider-wait.json';before=file.read_bytes();call(kind,['run-limits','provider-wait','preview'],bad,False);assert file.read_bytes()==before
    checks.append('new-requests-remain-strict-before-historical-replay')

    applied={}
    for kind in roots:
        req={**request,**({'preview_token':previews[kind]['preview_token']} if kind=='native' else {})};applied[kind]=call(kind,['run-limits','provider-wait','apply'],req);assert applied[kind]['applied'] is True
    def without_new_observations(c):
        """Only the generated new-row actor/time differ; old metadata is exact."""
        c=json.loads(json.dumps(c));row=c['history'][-1];assert isinstance(row['actor'],str) and isinstance(row['at'],str) and row['at'];row['actor']=row['at']='GENERATED_OBSERVATION_NOT_AUTHENTICATION';return c
    assert canonical(without_new_observations(applied['go']['config']))==canonical(without_new_observations(applied['native']['config']));checks.append('apply-preserves-original-typed-history-and-current-data')
    for kind in roots:
        file=roots[kind]/'.swarm/provider-wait.json';before=file.read_bytes();view=call(kind,['run-limits','provider-wait','apply'],request);assert view['config']['revision']==2 and file.read_bytes()==before
        call(kind,['run-limits','provider-wait','apply'],{**request,'reason':'Different event reuse reason'},False);assert file.read_bytes()==before
    checks.append('exact-event-replay-and-divergent-event-refusal-preserve-bytes')
    assert len(checks)==13
    identities=json.loads((output/'binary-identities.json').read_bytes())
    for kind,binary in binaries.items():assert proof.digest(binary.read_bytes())==identities[kind],('binary-changed',kind)
    proof.publish(output/'result.json',{'status':'PASS','checks':checks,'go_reference':str(reference),'allowlist':['native read-only source/exact/bounds projection keys only','native preview token required only for new publication; never stored in original Change','newly generated actor/time are unauthenticated differing observations; historical fields exact','safe diagnostic envelopes/messages may differ; rejected command must fail without source mutation','Go preview creates an advisory lock; native preview does not; private native backups are additive history'],'scope':'Executed bounded Go/native wait data/refusal/replay contract; no global migration/SQL/security/account/acceptance claim'});print('PROVIDER_WAIT_GO_PARITY_VERIFIED 13',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-reference',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.go_reference.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():proof.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
