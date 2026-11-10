"""Real public Go task edits compared at the pure native helper boundary."""
import hashlib,json,sqlite3
def qualify(root,output,work,env,run,execute,save,probe):
    fixture=work/'public-tasks';fixture.mkdir(mode=0o700);cli=work/'go-cli'
    run('public-init',[str(cli),'--root',str(fixture),'--json','init'])
    def command(label,args,r,expected=0):
        path=work/(label+'.json');path.write_text(json.dumps(r))
        p=execute(label,[str(cli),'--root',str(fixture),'--json',*args,'--input',str(path)],expected=expected)
        return json.loads(p.stdout)['work']if p.returncode==0 else p
    def native(label,before,r):
        f=dict(Name=label,Mode='metadata',Body=json.dumps(before).encode().hex(),Request=json.dumps(r).encode().hex(),Policy='7b7d',control=False,classification='parity')
        value=json.loads(run(label,[str(probe)],json.dumps(f).encode()+b'\n'));return value,json.loads(bytes.fromhex(value['body']))
    current=command('public-create',['work','create'],dict(schema_version=1,event_id='create-r3m',expected_revision=0,title='Tasks',objective='Pure contract parity',scope='Owned fixture',criteria=['Proof']))
    identifier=current['id']
    for task,depends in [('prior',[]),('task',['prior'])]:
        current=command('public-add-'+task,['task','add',identifier],dict(schema_version=1,event_id='add-'+task,expected_revision=current['revision'],id=task,title=task,deliverable='report',criteria=['old'],depends=depends))
    assert current['revision']==3
    requests=[
        ('edit',dict(title='Changed',deliverable='new report',criteria=['first','second'],depends=[],max_attempts=2,max_tool_calls=20)),
        ('metadata',dict(owner='owner',next='next steps',blocker='retained metadata blocker')),
    ]
    for label,fields in requests:
        request=dict(schema_version=1,event_id=label+'-r3m',expected_revision=current['revision'],id='task',**fields)
        value,body=native('public-native-'+label,current,request);assert value['code']=='ok'
        after=command('public-go-'+label,['task','update',identifier],request);body.update(revision=after['revision'],updated=after['updated']);assert body==after;current=after
    db=fixture/'.swarm/state.db';digest=hashlib.sha256(db.read_bytes()).hexdigest()
    invalid=dict(schema_version=1,event_id='recover-r3m',expected_revision=current['revision'],id='task',depends=['task'],title='Recovered')
    value,body=native('public-native-invalid',current,invalid);assert value['code']=='format'and body==current
    p=command('public-go-invalid',['task','update',identifier],invalid,expected=2);assert p.stderr and not p.stdout
    assert hashlib.sha256(db.read_bytes()).hexdigest()==digest
    recovered=dict(invalid,depends=[])
    value,body=native('public-native-recovery',current,recovered);assert value['code']=='ok'
    after=command('public-go-same-event-recovery',['task','update',identifier],recovered);body.update(revision=after['revision'],updated=after['updated']);assert body==after;current=after
    replay=command('public-go-replay',['task','update',identifier],recovered);assert replay==current
    reopened=json.loads(run('public-go-reopen',[str(cli),'--root',str(fixture),'--json','work','show',identifier]));assert reopened['work']==current and len(reopened['events'])==6
    with sqlite3.connect(db)as connection:
        request=json.loads(connection.execute("SELECT request FROM events WHERE id='recover-r3m'").fetchone()[0]);assert request['depends']==[]
    report=dict(status='PASS',actual_go_public_definition_and_metadata=True,native_helper_candidates_match=True,go_refusal_database_unchanged=True,same_failed_event_explicit_corrected_request=True,go_public_replay_and_reopen_revision=6,envelope_clock_supplied_by_go=True,public_native_mutation=False,authority=False)
    save('public-flow.json',report);return report
