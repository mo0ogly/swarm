"""Actual Go checkpoint refusal/recovery and native candidate comparison."""
import hashlib,json,os,sqlite3
def qualify(root,output,work,env,run,execute,save,probe):
    fixture=work/'public-checkpoint';fixture.mkdir(mode=0o700);alias=work/'public-root-alias';alias.symlink_to(fixture,target_is_directory=True);cli=work/'go-cli';run('public-init',[str(cli),'--root',str(fixture),'--json','init'])
    def command(label,kind,r,identifier=None,expected=0):
        path=work/(label+'.json');path.write_text(json.dumps(r));args=[str(cli),'--root',str(alias),'--json',*kind.split('.')]
        if identifier is not None:args.append(identifier)
        return execute(label,args+['--input',str(path)],expected=expected)
    request=dict(schema_version=1,event_id='create-r3l',expected_revision=0,title='Memory checkpoint',objective='Path parity',scope='Owned fixture',criteria=['Observable checkpoint'])
    before=json.loads(command('public-create','work.create',request).stdout)['work'];identifier=before['id'];(fixture/'memory.txt').write_bytes(b'fixture memory')
    def native(label,w,r,git):
        f=dict(Name=label,Mode='checkpoint',RootHex=os.fsencode(alias).hex(),PathHex='',File='',Body=json.dumps(w).encode().hex(),Request=json.dumps(r).encode().hex(),Git=git)
        got=json.loads(run(label,[str(probe)],json.dumps(f).encode()+b'\n'));return got,json.loads(bytes.fromhex(got['body']))
    request=dict(schema_version=1,event_id='checkpoint-r3l',expected_revision=1,summary='Checkpoint',next='Next',memory=['memory.txt'])
    saved=json.loads(command('public-go-checkpoint','checkpoint',request,identifier).stdout)['work'];got,body=native('public-native-checkpoint',before,request,saved['git']);assert got['code']=='ok';body.update(revision=saved['revision'],updated=saved['updated']);assert body==saved
    def digest():
        with sqlite3.connect(fixture/'.swarm/state.db')as db:rows=[db.execute(q).fetchall()for q in ['SELECT hex(id),revision,hex(body) FROM works ORDER BY id','SELECT hex(id),hex(work_id),revision,hex(kind),hex(at),hex(payload),hex(request) FROM events ORDER BY id','SELECT work_id,generation,digest,projection_version FROM agent_identity_plan_heads ORDER BY work_id']]
        return hashlib.sha256(json.dumps(rows,sort_keys=True).encode()).hexdigest()
    request=dict(schema_version=1,event_id='recover-r3l',expected_revision=2,summary='Recovered',memory=['memory.txt','missing.txt']);stamp=digest();got,body=native('public-native-missing',saved,request,saved['git']);assert got['code']=='format'and body==saved and digest()==stamp
    failed=command('public-go-missing','checkpoint',request,identifier,expected=None);assert failed.returncode!=0 and failed.stderr and digest()==stamp
    (fixture/'missing.txt').write_bytes(b'now present');recovered=json.loads(command('public-go-same-event-recovery','checkpoint',request,identifier).stdout)['work'];assert recovered['revision']==3
    got,body=native('public-native-same-request-recovery',saved,request,recovered['git']);assert got['code']=='ok';body.update(revision=recovered['revision'],updated=recovered['updated']);assert body==recovered
    request=dict(schema_version=1,event_id='nil-memory-r3l',expected_revision=3,summary='Memory cleared');cleared=json.loads(command('public-go-nil-memory','checkpoint',request,identifier).stdout)['work'];assert cleared['revision']==4 and cleared['memory']is None
    replay=dict(request,memory=[]);again=json.loads(command('public-go-ambiguous-empty-memory','checkpoint',replay,identifier).stdout)['work'];assert again==cleared and again['memory']is None
    got,body=native('public-native-explicit-empty-memory',recovered,replay,cleared['git']);assert got['code']=='ok'and body['memory']==[]
    observed=json.loads(run('public-go-reopen',[str(cli),'--root',str(alias),'--json','work','show',identifier]));assert observed['work']==cleared and len(observed['events'])==4
    report=dict(status='PASS',actual_go_checkpoint=True,native_candidate_parity=True,root_alias_supported=True,missing_file_refused_without_sql_changes=True,same_failed_event_and_request_recovered=True,go_memory_empty_replay_ambiguity_reproduced=True,native_explicit_empty_retained=True,revision=4,events=4,public_native_mutation=False,git_and_envelope_supplied=True,authority=False)
    save('public-flow.json',report);return report
