"""Actual Go CLI planning, inspected by native data validation; no provider."""
import hashlib
import json
import sqlite3

def qualify(root,output,work,env,run,execute,save,probe):
    fixture=work/'public-planning';fixture.mkdir(mode=0o700);cli=work/'go-cli'
    run('public-init',[str(cli),'--root',str(fixture),'--json','init'])
    def command(label,args,request):
        path=work/(label+'.json');path.write_text(json.dumps(request));return json.loads(run(label,[str(cli),'--root',str(fixture),'--json',*args,'--input',str(path)]))
    r=dict(schema_version=1,event_id='create-r3j',expected_revision=0,title='Planning',objective='Durable plan validation',scope='Owned fixture',criteria=['Verifiable outcome'])
    created=command('public-create',['work','create'],r)['work'];identifier=created['id']
    def req(event,revision,**kw):return dict(schema_version=1,event_id=event,expected_revision=revision,**kw)
    enabled=command('public-enable',['planning','enable',identifier],req('enable-r3j',1,max_tasks=10,max_decisions=10,max_activations=10,max_judge_cycles=0));assert enabled['revision']==2 and enabled['planning']['judge']['cycle']==1
    claimed=command('public-claim',['planning','claim',identifier],req('claim-r3j',2,scope='root',scope_revision=enabled['planning']['scopes'][0]['revision'],holder='fixture-owner',lease_seconds=60))
    scope=claimed['planning']['scopes'][0]
    operation=dict(kind='task',id='task',title='Verify',deliverable='proof.txt',criteria=['Observable evidence'],requirements=['req-1'],next='Check source')
    decision=req('decide-r3j',claimed['revision'],scope='root',scope_revision=scope['revision'],holder=scope['holder'],generation=scope['generation'],input_events=scope['delivery']['events'],reason='Decompose the owned scope',operations=[operation])
    decided=command('public-decide',['planning','decide',identifier],decision);assert len(decided['tasks'])==1 and decided['tasks'][0]['scope_id']=='root'
    def digest():
        with sqlite3.connect(fixture/'.swarm/state.db')as db:values=[db.execute(q).fetchall()for q in ['SELECT hex(id),revision,hex(body) FROM works ORDER BY id','SELECT hex(id),hex(work_id),revision,hex(kind),hex(at),hex(payload),hex(request) FROM events ORDER BY id','SELECT work_id,generation,digest,projection_version FROM agent_identity_plan_heads ORDER BY work_id']]
        return hashlib.sha256(json.dumps(values,sort_keys=True).encode()).hexdigest()
    before=digest()
    with sqlite3.connect(fixture/'.swarm/state.db')as db:body=db.execute('SELECT body FROM works WHERE id=?',(identifier,)).fetchone()[0]
    f=dict(Name='public-native-validator',Mode='validate',Body=body.hex(),Before={},Event='fixture',At='2026-10-18T00:00:00Z')
    got=json.loads(run('public-native-validate',[str(probe)],json.dumps(f).encode()+b'\n'));assert got['code']=='ok'and json.loads(bytes.fromhex(got['body']))==decided and digest()==before
    for name,change in [('cycle',lambda w:w['planning']['scopes'][0].update(parent='root')),('judge',lambda w:w['planning']['judge'].update(cycle=0)),('ownership',lambda w:w['tasks'][0].update(requirements=['missing']))]:
        invalid=json.loads(body);change(invalid);bad=dict(f,Name='public-copy-'+name,Body=json.dumps(invalid).encode().hex());got=json.loads(run('public-native-refuse-'+name,[str(probe)],json.dumps(bad).encode()+b'\n'));assert got['code']=='format'and json.loads(bytes.fromhex(got['body']))==invalid and digest()==before
    recovered=json.loads(run('public-native-valid-again',[str(probe)],json.dumps(f).encode()+b'\n'));assert recovered['code']=='ok'and digest()==before
    observed=json.loads(run('public-go-reopen',[str(cli),'--root',str(fixture),'--json','planning','show',identifier]));assert observed==decided
    report=dict(status='PASS',go_public_enable_claim_decide=True,native_validates_actual_work=True,invalid_copies_refused_without_mutation=True,go_database_unchanged_by_native=True,go_public_reopen=True,public_native_mutation=False,provider=False,authority=False)
    save('public-flow.json',report);return report
