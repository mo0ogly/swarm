"""Actual system environment/native transaction composition on Go-created roots.

Business outputs are compared after normalizing generated root IDs/times only;
format/parity of time and ID helpers is qualified separately. No native root
opening/migration, public CLI or authenticated HTTP claim is made.
"""
import copy,hashlib,json,re,sqlite3
def qualify(root,output,work,scratch,env,run,execute,save,probe):
    cli=work/'go-cli';go=scratch/'go-work-engine';native=scratch/'native-work-engine'
    for name,path in [('go',go),('native',native)]:
        path.mkdir(mode=0o700);run('engine-'+name+'-init',[str(cli),'--root',str(path),'--json','init']);(path/'.rust-migration-fixture').write_text('Owned native WorkEngine fixture.\n')
    def go_command(label,kind,identifier,request,expected=0):
        path=work/(label+'.json');path.write_text(json.dumps(request));args=kind.split('.')if'.'in kind else[kind]
        if kind!='work.create':args.append(identifier)
        p=execute(label,[str(cli),'--root',str(go),'--json',*args,'--input',str(path)],expected=expected)
        return json.loads(p.stdout)['work']if p.returncode==0 else p
    def native_command(label,kind,identifier,request,expected=0):
        frame=dict(Name=label,Work=identifier,Kind=kind,Request=json.dumps(request).encode().hex())
        p=execute(label,[str(probe),str(native)],json.dumps(frame).encode()+b'\n',expected=expected)
        value=json.loads(p.stdout);return value,json.loads(bytes.fromhex(value['body']))if value['body']else None
    def rows(path):
        with sqlite3.connect(path/'.swarm/state.db')as db:return dict(works=db.execute('SELECT hex(id),revision,hex(body)FROM works ORDER BY id').fetchall(),events=db.execute('SELECT hex(id),hex(work_id),revision,hex(kind),hex(at),hex(payload),hex(request)FROM events ORDER BY id').fetchall(),heads=db.execute('SELECT *FROM agent_identity_plan_heads ORDER BY work_id').fetchall())
    def digest(path):return hashlib.sha256(json.dumps(rows(path),sort_keys=True).encode()).hexdigest()
    def normalize(w):
        w=copy.deepcopy(w);assert re.fullmatch(r'w-[0-9a-f]{24}',w['id']);w['id']='generated-id'
        for field in ['created','updated']:
            assert re.fullmatch(r'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{1,9})?Z',w[field]);w[field]='generated-time'
        for t in w['tasks']:
            p=t.get('validation_policy')
            if p:assert p['actor'] and p['authorized_at'];p['authorized_at']='generated-time'
        return w
    def equal(g,n):assert normalize(g)==normalize(n)
    request=dict(schema_version=1,event_id='create-engine',expected_revision=0,title='Native engine',objective='Complete Work transaction',scope='Owned fixture',criteria=['First','Second'],requirement_prerequisites={'req-1':['req-2']},origin='forged-engine')
    g=go_command('engine-go-create','work.create','',request);value,n=native_command('engine-native-create','work.create','',request);assert value['code']=='ok'and value['git_issues']==3;equal(g,n);gid,nid=g['id'],n['id']
    flow=[('work.update',dict(title='Changed')),('task.add',dict(id='task',title='Task',deliverable='report',criteria=['Proof'])),('task.update',dict(id='task',title='Refined',criteria=['First','Second'],depends=[],owner='owner',next='Next')),('task.update',dict(id='task',validation_policy=dict(mode='human'))),('ooda',dict(observation='Observed',orientation='Oriented',decision='Decided',owner='owner',next='Next',result='Recorded result'))]
    for index,(kind,fields)in enumerate(flow):
        request=dict(schema_version=1,event_id='engine-'+str(index),expected_revision=g['revision'],**fields)
        g=go_command('engine-go-'+str(index),kind,gid,request);value,n=native_command('engine-native-'+str(index),kind,nid,request);assert value['code']=='ok';equal(g,n)
    for path in [go,native]:(path/'memory.txt').write_text('Owned memory fixture.\n')
    request=dict(schema_version=1,event_id='checkpoint-engine',expected_revision=g['revision'],summary='Checkpoint',next='Continue',memory=['memory.txt'])
    g=go_command('engine-go-checkpoint','checkpoint',gid,request);value,n=native_command('engine-native-checkpoint','checkpoint',nid,request);assert value['code']=='ok'and value['git_issues']==3;equal(g,n)
    # Refusal, no durable partial mutation, explicit corrected same failed event.
    before_go,before_native=digest(go),digest(native)
    bad=dict(schema_version=1,event_id='engine-recover',expected_revision=g['revision'],id='task',title='Recovered',depends=['task'])
    rejected=go_command('engine-go-invalid','task.update',gid,bad,expected=2);assert rejected.stderr and not rejected.stdout
    value,_=native_command('engine-native-invalid','task.update',nid,bad);assert value['code']!='ok'and not value['requires_reconciliation'];assert digest(go)==before_go and digest(native)==before_native
    repaired=dict(bad,depends=[]);g=go_command('engine-go-recovery','task.update',gid,repaired);value,n=native_command('engine-native-recovery','task.update',nid,repaired);assert value['code']=='ok';equal(g,n)
    before=digest(native);replayed,replay=native_command('engine-native-replay','task.update',nid,repaired);assert replayed['replayed']and replay==n and digest(native)==before
    # The real assembled native engine consumes the safe intent codec.
    ambiguous=dict(schema_version=1,event_id='engine-0',expected_revision=1,title='Changed',criteria=[])
    hidden=go_command('engine-go-ambiguous-replay','work.update',gid,ambiguous);assert hidden==g
    value,_=native_command('engine-native-distinct-intent-refused','work.update',nid,ambiguous);assert value['code']=='event_conflict'and digest(native)==before
    observed=json.loads(run('engine-go-reads-native',[str(cli),'--root',str(native),'--json','work','show',nid]));assert observed['work']==n
    assert len(observed['events'])==n['revision']==8
    with sqlite3.connect(native/'.swarm/state.db')as db:
        for (raw,)in db.execute('SELECT request FROM events'):
            assert 'origin'not in json.loads(raw)
    # Failure to deliver a response after commit must not lose or duplicate it.
    late=dict(schema_version=1,event_id='engine-output-loss',expected_revision=n['revision'],title='After output loss');frame=dict(Name='lost-response',Work=nid,Kind='work.update',Request=json.dumps(late).encode().hex())
    import subprocess
    with open('/dev/full','wb')as sink:p=subprocess.run([str(probe),str(native)],input=json.dumps(frame).encode()+b'\n',env=env,stdout=sink,stderr=subprocess.PIPE)
    (output/'engine-output-loss.stderr').write_bytes(p.stderr);assert p.returncode!=0 and p.stderr
    late_response,late_work=native_command('engine-replay-output-loss','work.update',nid,late);assert late_response['replayed']and late_work['revision']==9 and late_work['title']=='After output loss'
    with sqlite3.connect(native/'.swarm/state.db')as db:assert db.execute("SELECT count(*)FROM events WHERE id='engine-output-loss'").fetchone()[0]==1
    report=dict(status='PASS',actual_system_entropy_clock_operator_git=True,go_and_native_business_state_equal_after_documented_generated_fields_normalization=True,native_real_transaction_commit_reopen=True,go_reads_native=True,git_unavailable_explicit=True,same_failed_event_recovery=True,public_go_ambiguous_replay_reproduced=True,native_assembled_engine_refuses_distinct_intent=True,origin_cleared_before_persistence=True,response_loss_after_commit_replay_without_duplicate=True,native_final_revision=9,task_status_transitions=False,public_native_cli=False,root_migrations=False,admin_settings_wired=False)
    save('engine-public-flow.json',report);return report
