"""Public Go operations compared with native pure candidate transformations."""
import json
import sqlite3

def qualify(root,output,work,env,run,execute,save,probe):
    fixture=work/'public-definitions';fixture.mkdir(mode=0o700);cli=work/'go-cli';run('public-init',[str(cli),'--root',str(fixture),'--json','init'])
    def command(label,args,r):
        path=work/(label+'.json');path.write_text(json.dumps(r));return json.loads(run(label,[str(cli),'--root',str(fixture),'--json',*args,'--input',str(path)]))['work']
    def native(label,kind,before,r,git=None):
        f=dict(Name=label,Kind=kind,Body=json.dumps(before).encode().hex(),Request=json.dumps(r).encode().hex(),Git=git or dict(head='',branch='',changes=''))
        got=json.loads(run(label,[str(probe)],json.dumps(f).encode()+b'\n'));return got,json.loads(bytes.fromhex(got['body']))
    request=dict(schema_version=1,event_id='create-r3k',expected_revision=0,title='Created',objective='Definition parity',scope='Owned fixture',criteria=['A','B'],requirement_prerequisites={'req-1':['req-2']})
    created=command('public-go-create',['work','create'],request);identifier=created['id'];initial=dict(schema_version=1,id=identifier,revision=0,created=created['created'],tasks=[])
    got,body=native('public-native-create','work.create',initial,request,created['git']);assert got['code']=='ok';body.update(revision=created['revision'],updated=created['updated']);assert body==created
    current=created
    for label,kind,r in [
        ('update','work.update',dict(schema_version=1,event_id='update-r3k',expected_revision=1,title='Updated')),
        ('add','task.add',dict(schema_version=1,event_id='add-r3k',expected_revision=2,id='task',title='Task',deliverable='Proof',criteria=['Evidence'])),
        ('ooda','ooda',dict(schema_version=1,event_id='ooda-r3k',expected_revision=3,observation='Observed',orientation='Oriented',decision='Decided',owner='Owner',next='Next',result='Result')),
    ]:
        got,body=native('public-native-'+label,kind,current,r);assert got['code']=='ok'
        args=kind.split('.')+[identifier]if '.'in kind else[kind,identifier]
        after=command('public-go-'+label,args,r);body.update(revision=after['revision'],updated=after['updated']);assert body==after;current=after
    assert current['revision']==4 and current['tasks'][0]['attempts']==[]and current['summary']=='Result'
    observed=json.loads(run('public-go-reopen',[str(cli),'--root',str(fixture),'--json','work','show',identifier]));assert observed['work']==current and len(observed['events'])==4
    # A full public Go replay hides the explicit empty map in its recorded DTO.
    replay=dict(schema_version=1,event_id='update-r3k',expected_revision=1,title='Updated',requirement_prerequisites={})
    ambiguous=command('public-go-ambiguous-clear-replay',['work','update',identifier],replay);assert ambiguous==current and ambiguous['requirement_prerequisites']=={'req-1':['req-2']}
    with sqlite3.connect(fixture/'.swarm/state.db')as db:
        raw=db.execute("SELECT request FROM events WHERE id='update-r3k'").fetchone()[0];assert 'requirement_prerequisites'not in json.loads(raw)
    report=dict(status='PASS',actual_go_create_update_add_ooda=True,native_pure_transformations_match=True,id_clock_and_git_supplied_not_native=True,go_public_reopen_revision=4,go_public_ambiguous_clear_replay_reproduced=True,native_intent_encoder_and_reservation_covered_in_tests=True,public_native_mutation=False,authority=False)
    save('public-flow.json',report);return report
