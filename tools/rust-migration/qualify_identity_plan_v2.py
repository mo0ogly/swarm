"""R3d checks: adapted Go test references, producer audit and real Go DB fixture."""
import hashlib
import json
import shutil
import sqlite3
from identity_plan_field_audit import audit


def qualify(root,output,work,env,run,execute,compare,save,layout,examples,binaries,values):
    model_layout=json.loads(run('projection-producer-layout',['go','run','./tools/rust-migration/work-layout']))
    save('projection-field-audit.json',audit(root,model_layout,layout))
    references={}
    hashes={}
    for version in [1,2]:
        source=run('projection-reference-source-v'+str(version),['go','run','./tools/rust-migration/plan-version-reference',str(version)])
        path=work/('projection-v'+str(version)+'.go');path.write_bytes(source)
        binary=work/('projection-v'+str(version)+'-bin')
        run('projection-reference-build-v'+str(version),['go','build','-o',str(binary),str(path)])
        references[version]=binary
        shutil.copyfile(binary,output/('projection-reference-v'+str(version)+'.binary'))
        (output/('projection-reference-v'+str(version)+'.binary')).chmod(0o600)
        hashes[str(version)]=dict(source_sha256=hashlib.sha256(source).hexdigest(),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())
    ordinary=[v for v in values if not(v['kind']=='blob'and not v['raw'])]
    raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in ordinary)
    control=compare('adapted-v1-vs-real-go-sql',raw,binaries['go-plan'],references[1])
    newer=[dict(v,projection=2)for v in values]
    for key in ['automatic_validation','manual_override','auto_validation','override']:
        for value in [None,{},[],True,0,'runtime',{'state':'passed','actor':'human','reason':'fixture'}]:
            newer.extend([
                dict(name=f'v2-{key}-{len(newer)}-control',kind='blob',projection=2,raw=b'{"tasks":[{"id":"task-a","owner":"agent-a"}]}'.hex(),control=True),
                dict(name=f'v2-{key}-{len(newer)}-runtime',kind='blob',projection=2,raw=json.dumps({'tasks':[{'id':'task-a','owner':'agent-a',key:value}]},separators=(',',':')).encode().hex(),control=True),
            ])
    v2raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in newer)
    compared=compare('projection-v2-corpus',v2raw,references[2],examples/'identity_plan_probe')
    frames=[json.loads(l)for l in (output/'projection-v2-corpus-rust.stdout').read_bytes().splitlines()]
    for index in range(len(values),len(newer),2):assert frames[index]['digest']==frames[index+1]['digest']
    # A genuine database from the public Go CLI, not a hand-built minimal schema.
    cli=work/'go-cli-bin';setter=work/'go-row-setter-bin'
    run('fixture-cli-build',['go','build','-o',str(cli),'./cmd/swarm'])
    run('fixture-setter-build',['go','build','-o',str(setter),'./tools/rust-migration/work-row-fixture'])
    fixture=work/'go-native-upgrade-fixture';fixture.mkdir(mode=0o700)
    run('fixture-go-init',[str(cli),'--root',str(fixture),'--json','init'])
    (fixture/'.rust-migration-fixture').write_text('Owned isolated R3d fixture. No live mission.\n')
    body=b'{"schema_version":1,"id":"legacy","revision":3,"title":"Plan fixture","tasks":[{"id":"task-a","owner":"agent-a"}]}'
    request=json.dumps(dict(raw=body.hex(),revision=3)).encode()
    run('fixture-go-valid-row',[str(setter),str(fixture)],request)
    database=fixture/'.swarm/state.db'
    def state():
        with sqlite3.connect(database)as db:
            rows=db.execute('SELECT work_id,generation,digest,projection_version FROM agent_identity_plan_heads ORDER BY work_id').fetchall()
            dump='\n'.join(db.iterdump()).encode()
            # Verify all other contracts' schemas and data remain untouched.
            tables=[v[0]for v in db.execute("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT IN ('agent_identity_plan_heads','agent_identity_plan_projection_history') ORDER BY name")]
            stable={name:repr(db.execute('SELECT * FROM "'+name.replace('"','""')+'"').fetchall())for name in tables}
            foreign=db.execute('PRAGMA foreign_key_check').fetchall()
            return dict(dump_sha256=hashlib.sha256(dump).hexdigest(),stable_sha256=hashlib.sha256(json.dumps(stable,sort_keys=True).encode()).hexdigest(),heads=rows,foreign_key_errors=foreign,schema_version=db.execute('PRAGMA user_version').fetchone()[0],quick_check=db.execute('PRAGMA quick_check').fetchone()[0])
    before=state();assert before['schema_version']==37 and not before['foreign_key_errors'] and before['quick_check']=='ok'
    rolled=json.loads(run('fixture-native-rollback',[str(examples/'identity_plan_upgrade_probe'),str(fixture),'rollback']))
    assert rolled['projection']==1 and state()==before
    # The same valid rollback just passed. A separately held writer lock is the
    # only changed condition. Zero wait belongs to the fixture, not product policy.
    with sqlite3.connect(database,timeout=0) as blocker:
        blocker.execute('BEGIN IMMEDIATE')
        busy=execute('fixture-native-writer-busy',[str(examples/'identity_plan_upgrade_probe'),str(fixture),'rollback'],expected=1)
        assert b'DatabaseBusy' in busy.stderr and not busy.stdout and state()==before
        blocker.rollback()
    recovered=json.loads(run('fixture-native-writer-recovery',[str(examples/'identity_plan_upgrade_probe'),str(fixture),'rollback']))
    assert recovered==rolled and state()==before
    committed=json.loads(run('fixture-native-commit',[str(examples/'identity_plan_upgrade_probe'),str(fixture),'commit']))
    after=state();assert after['stable_sha256']==before['stable_sha256'] and after['schema_version']==37 and not after['foreign_key_errors']and after['quick_check']=='ok'
    assert committed['projection']==2 and committed['generation']==before['heads'][0][1]+1
    assert after['heads'][0][2]!=before['heads'][0][2]and after['heads'][0][3]==2
    reopened=json.loads(run('fixture-native-reopened',[str(examples/'identity_plan_upgrade_probe'),str(fixture),'verify']))
    assert reopened['digest']==committed['digest'] and reopened['generation']==committed['generation']and state()==after
    # Go v1 cannot write v2: missing function must refuse, never return success.
    old=execute('fixture-legacy-writer-refused',[str(setter),str(fixture)],request,expected=2)
    assert b'swarm_identity_plan_digest_v2'in old.stderr and state()==after
    save('fixture-conversion.json',dict(status='PASS',before=before,after=after,reopened=reopened,legacy_writer_refused=True,writer_lock_refused_then_recovered=True,scope='Owned real Go-created v37 database; component conversion only; global user_version intentionally unchanged, no native Store/public command or rollback witness'))
    return dict(status='PASS',adapted_v1_control=control,v2=compared,references=hashes,real_go_fixture=True,writer_contention_recovery=True,global_store_migration=False,anti_rollback_witness=False)
