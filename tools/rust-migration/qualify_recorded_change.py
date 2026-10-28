"""Isolated real Go schema; native storage only, no public native authority."""
import hashlib
import json
import sqlite3
import subprocess

def qualify(root,output,work,env,run,execute,save,probe):
    cli=work/'go-cli';setter=work/'go-row-setter'
    def create_root(name):
        fixture=work/name;fixture.mkdir(mode=0o700);run(name+'-init',[str(cli),'--root',str(fixture),'--json','init']);(fixture/'.rust-migration-fixture').write_text('Owned R3h fixture.\n');return fixture
    def read_rows(fixture):
        with sqlite3.connect(fixture/'.swarm/state.db')as db:
            return {key:db.execute(query).fetchall()for key,query in [('works','SELECT hex(id),revision,hex(body) FROM works ORDER BY id'),('events','SELECT hex(id),hex(work_id),revision,hex(kind),hex(at),hex(payload),hex(request) FROM events ORDER BY id'),('heads','SELECT work_id,generation,digest,projection_version FROM agent_identity_plan_heads ORDER BY work_id')]}
    def digest(fixture):return hashlib.sha256(json.dumps(read_rows(fixture),sort_keys=True).encode()).hexdigest()
    def record(fixture,event):
        with sqlite3.connect(fixture/'.swarm/state.db')as db:
            row=db.execute('SELECT w.body,e.request FROM works w JOIN events e ON e.work_id=w.id WHERE e.id=?',(event,)).fetchone();assert row;return row
    def frame(name,body,request,expected,kind,event,rollback=False):return dict(Name=name,Existing='',Candidate=body.hex(),Expected=expected,Kind=kind,Event=event,Request=request.hex(),Fault='',Rollback=rollback)
    def native(label,fixture,f):return json.loads(run(label,[str(probe),str(fixture)],json.dumps(f).encode()+b'\n'))
    go=create_root('go-public');rust=create_root('native-public')
    request=dict(schema_version=1,event_id='create-r3h',expected_revision=0,title='Initial',objective='Atomic SQL persistence',scope='Owned fixture',criteria=['Work and event persist together'])
    path=work/'create.json';path.write_text(json.dumps(request));created=json.loads(run('public-go-create',[str(cli),'--root',str(go),'--json','work','create','--input',str(path)]))['work'];identifier=created['id']
    body,raw=record(go,'create-r3h');first=frame('create',body,raw,0,'work.create','create-r3h');assert native('public-native-create',rust,first)['code']=='ok';assert read_rows(go)==read_rows(rust)
    request=dict(schema_version=1,event_id='update-r3h',expected_revision=1,title='Updated');path=work/'update.json';path.write_text(json.dumps(request));updated=json.loads(run('public-go-update',[str(cli),'--root',str(go),'--json','work','update',identifier,'--input',str(path)]))['work']
    body,raw=record(go,'update-r3h');second=frame('update',body,raw,1,'work.update','update-r3h');assert native('public-native-update',rust,second)['code']=='ok';assert read_rows(go)==read_rows(rust)
    observed=json.loads(run('public-go-reads-native',[str(cli),'--root',str(rust),'--json','work','show',identifier]));assert observed['work']==updated and len(observed['events'])==2
    stamp=digest(rust);replayed=native('public-native-replay-current',rust,first);assert replayed['code']=='replay'and json.loads(bytes.fromhex(replayed['current']))==updated and digest(rust)==stamp
    next_work=json.loads(body);next_work['revision']=3;next_work['title']='After lost response';next_work['updated']='2026-10-18T00:00:01Z'
    third=frame('third',json.dumps(next_work,separators=(',',':')).encode(),b'{"storage_fixture":true}',2,'checkpoint','third-r3h')
    control=dict(third,Rollback=True);assert native('public-positive-control-rollback',rust,control)['code']=='staged'and digest(rust)==stamp
    with sqlite3.connect(rust/'.swarm/state.db')as db:db.execute("CREATE TRIGGER fail_r3h BEFORE INSERT ON events WHEN NEW.id='third-r3h' BEGIN SELECT RAISE(ABORT,'fixture');END")
    rejected=native('public-event-failure',rust,third);assert rejected['code']=='reject'and digest(rust)==stamp
    with sqlite3.connect(rust/'.swarm/state.db')as db:db.execute('DROP TRIGGER fail_r3h')
    assert native('public-positive-recovery-rollback',rust,control)['code']=='staged'and digest(rust)==stamp
    # Transport failure after outer commit is an uncertain response, not rollback.
    data=json.dumps(third).encode()+b'\n'
    with open('/dev/full','wb')as sink:p=subprocess.run([str(probe),str(rust)],env=env,input=data,stdout=sink,stderr=subprocess.PIPE)
    (output/'public-native-full.stderr').write_bytes(p.stderr);assert p.returncode!=0 and p.stderr
    after=read_rows(rust);assert after['works'][0][1]==3 and len(after['events'])==3
    observed=json.loads(run('public-go-read-after-output-failure',[str(cli),'--root',str(rust),'--json','work','show',identifier]));assert observed['work']['revision']==3 and observed['work']['title']=='After lost response'
    stamp=digest(rust);again=native('public-replay-after-output-failure',rust,third);assert again['code']=='replay'and json.loads(bytes.fromhex(again['current']))['revision']==3 and digest(rust)==stamp
    # Real complete Go command overflow, not merely the extracted tail.
    maximum=9223372036854775807;max_body=json.dumps(dict(schema_version=1,id='legacy',revision=maximum,title='Before overflow',tasks=[]),separators=(',',':')).encode()
    go_max=create_root('go-overflow');native_max=create_root('native-overflow')
    for name,fixture in [('go',go_max),('native',native_max)]:run('overflow-'+name+'-valid-row',[str(setter),str(fixture)],json.dumps(dict(raw=max_body.hex(),revision=maximum)).encode())
    native_before=digest(native_max);safe=frame('overflow',max_body,b'{}',maximum,'work.update','overflow-r3h');assert native('public-native-overflow-refusal',native_max,safe)['code']=='reject'and digest(native_max)==native_before
    good=json.loads(run('public-go-valid-after-native-overflow',[str(cli),'--root',str(native_max),'--json','work','show','legacy']));assert good['work']['revision']==maximum
    change=dict(schema_version=1,event_id='overflow-r3h',expected_revision=maximum,title='After overflow');path=work/'overflow.json';path.write_text(json.dumps(change))
    result=execute('public-go-overflow',[str(cli),'--root',str(go_max),'--json','work','update','legacy','--input',str(path)],expected=None)
    damaged=read_rows(go_max);assert damaged['works'][0][1]==-9223372036854775808 and damaged['events'][0][2]==-9223372036854775808
    fail=execute('public-go-refuses-overflowed-row',[str(cli),'--root',str(go_max),'--json','work','show','legacy'],expected=None);assert fail.returncode!=0 and fail.stderr
    upgrade=probe.parent/'identity_plan_upgrade_probe'
    before_conversion=digest(native_max)
    rolled=json.loads(run('public-native-conversion-rollback',[str(upgrade),str(native_max),'rollback']));assert rolled['projection']==1 and rolled['global_schema_unchanged'] and digest(native_max)==before_conversion
    converted=json.loads(run('public-native-conversion-commit',[str(upgrade),str(native_max),'commit']));assert converted['projection']==2 and converted['global_schema_unchanged']
    verified=json.loads(run('public-native-conversion-reopen',[str(upgrade),str(native_max),'verify']));assert verified['projection']==2 and verified['digest']==converted['digest'] and verified['generation']==converted['generation']
    report=dict(status='PASS',normal_create_update_rows_and_stamps_equal=True,old_go_reads_native=True,persistence_reopen=True,replay_current=True,event_failure_and_recovery=True,outer_commit_owned_by_fixture=True,conversion_real_go_db_rollback_commit_reopen=True,output_failure_after_commit_observed=True,output_failure_replay_no_duplicate=True,go_overflow='OPEN: real command persisted int64 minimum; next read refused',go_overflow_exit=result.returncode,native_overflow_refused_without_mutation=True,native_business_guards=False,native_public_command=False)
    save('public-flow.json',report);return report
