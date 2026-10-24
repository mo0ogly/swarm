"""Public Go commands and native inspection on an owned real-schema fixture."""
import hashlib
import json
import sqlite3

def qualify(root,output,work,env,run,execute,save,probe):
    fixture=work/'public-guards';fixture.mkdir(mode=0o700);cli=work/'go-cli'
    run('public-init',[str(cli),'--root',str(fixture),'--json','init']);(fixture/'.rust-migration-fixture').write_text('Owned R3i guard fixture.\n')
    def command(label,kind,request,identifier=None,expected=0):
        path=work/(label+'.json');path.write_text(json.dumps(request))
        args=[str(cli),'--root',str(fixture),'--json',*kind.split('.')]
        if identifier is not None:args.append(identifier)
        return execute(label,args+['--input',str(path)],expected=expected)
    def digest():
        with sqlite3.connect(fixture/'.swarm/state.db')as db:
            rows={k:db.execute(q).fetchall()for k,q in [('works','SELECT hex(id),revision,hex(body) FROM works ORDER BY id'),('events','SELECT hex(id),hex(work_id),revision,hex(kind),hex(at),hex(payload),hex(request) FROM events ORDER BY id'),('heads','SELECT work_id,generation,digest,projection_version FROM agent_identity_plan_heads ORDER BY work_id')]}
        return hashlib.sha256(json.dumps(rows,sort_keys=True).encode()).hexdigest()
    def inspect(label,kind,request,identifier,revision,event,code):
        f=dict(Name=label,Body='',ID=identifier,Kind=kind,Event=event,Expected=revision,Request=request.hex(),SQL='')
        before=digest();result=json.loads(run(label,[str(probe),str(fixture)],json.dumps(f).encode()+b'\n'));assert result['code']==code and digest()==before;return result
    r=dict(schema_version=1,event_id='create-r3i',expected_revision=0,title='Before',objective='Guard verification',scope='Owned isolated fixture',criteria=['Refuse then recover'])
    created=json.loads(command('public-create','work.create',r).stdout)['work'];identifier=created['id']
    r=dict(schema_version=1,event_id='add-r3i',expected_revision=1,id='task',title='Task',deliverable='Proof',criteria=['Evidence'])
    added=json.loads(command('public-add','task.add',r,identifier).stdout)['work'];assert added['revision']==2
    def req(event,revision=2,**kw):return dict(schema_version=1,event_id=event,expected_revision=revision,**kw)
    inspect('public-native-positive-control','work.update',json.dumps(req('work-check',title='Next')).encode(),identifier,2,'work-check','existing')
    with sqlite3.connect(fixture/'.swarm/state.db')as db:
        agent=dict(id='fixture-agent',task_id='task',status='queued')
        db.execute('INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)',('fixture-agent',identifier,'task',str(fixture/'agent'), 'queued',json.dumps(agent).encode(),b'{}'))
    for kind,request,code in [('work.update',req('refuse-work',title='Forbidden'),'format'),('task.update',req('refuse-task',id='task',title='Forbidden'),'active_agent'),('task.update',req('refuse-status',id='task',status='running'),'active_agent')]:
        label=request['event_id'];inspect('public-native-'+label,kind,json.dumps(request).encode(),identifier,2,label,code)
        before=digest();p=command('public-go-'+label,kind,request,identifier,expected=None);assert p.returncode!=0 and p.stderr and digest()==before
    request=req('owner-r3i',id='task',owner='reviewer')
    inspect('public-native-metadata','task.update',json.dumps(request).encode(),identifier,2,'owner-r3i','existing')
    updated=json.loads(command('public-go-metadata','task.update',request,identifier).stdout)['work'];assert updated['revision']==3 and updated['tasks'][0]['owner']=='reviewer'
    inspect('public-native-stale','work.update',b'not-json',identifier,2,'stale-r3i','revision_conflict')
    with sqlite3.connect(fixture/'.swarm/state.db')as db:raw=db.execute("SELECT request FROM events WHERE id='create-r3i'").fetchone()[0]
    inspect('public-native-replay-current','work.create',raw,'',-1,'create-r3i','replay')
    with sqlite3.connect(fixture/'.swarm/state.db')as db:db.execute("DELETE FROM agents WHERE id='fixture-agent'")
    request=req('recovery-r3i',revision=3,title='Recovered')
    inspect('public-native-recovery','work.update',json.dumps(request).encode(),identifier,3,'recovery-r3i','existing')
    recovered=json.loads(command('public-go-recovery','work.update',request,identifier).stdout)['work'];assert recovered['revision']==4 and recovered['title']=='Recovered'
    observed=json.loads(run('public-go-reopen',[str(cli),'--root',str(fixture),'--json','work','show',identifier]));assert observed['work']==recovered and len(observed['events'])==4
    report=dict(status='PASS',actual_go_created_schema=True,native_inspection_rollback_no_business_writes=True,active_work_definition_and_status_refused=True,metadata_allowed_with_active_agent=True,stale_revision_refused_before_decode=True,replay_bypasses_new_guards=True,recovery_after_fixture_agent_removed=True,go_public_reopen_revision=4,go_events=4,native_public_command=False,full_mutation=False,permissions=False)
    save('public-flow.json',report);return report
