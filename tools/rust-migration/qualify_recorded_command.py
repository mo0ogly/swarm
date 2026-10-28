"""Owned public Go creation/update/replay, compared with native reservation only."""
import hashlib
import json
import sqlite3
import subprocess

def qualify(root,output,work,env,run,execute,save,probe):
    fixture=work/'public-command-fixture';fixture.mkdir(mode=0o700);cli=work/'go-cli'
    run('command-public-init',[str(cli),'--root',str(fixture),'--json','init']);(fixture/'.rust-migration-fixture').write_text('Owned isolated command fixture.\n')
    create=dict(schema_version=1,event_id='create-r3g',expected_revision=0,title='Initial',objective='Replay the current state',scope='Isolated fixture',criteria=['Current Work is retained'])
    create_path=work/'create.json';create_path.write_text(json.dumps(create))
    created=json.loads(run('command-public-create',[str(cli),'--root',str(fixture),'--json','work','create','--input',str(create_path)]))['work'];identifier=created['id'];assert created['revision']==1
    update=dict(schema_version=1,event_id='update-r3g',expected_revision=1,title='Updated state')
    update_path=work/'update.json';update_path.write_text(json.dumps(update))
    updated=json.loads(run('command-public-update',[str(cli),'--root',str(fixture),'--json','work','update',identifier,'--input',str(update_path)]))['work'];assert updated['revision']==2 and updated['title']=='Updated state'
    database=fixture/'.swarm/state.db'
    def digest():
        with sqlite3.connect(database)as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    stamp=digest()
    replay=json.loads(run('command-public-go-replay',[str(cli),'--root',str(fixture),'--json','work','create','--input',str(create_path)]))['work'];assert replay==updated and digest()==stamp
    with sqlite3.connect(database)as db:
        assert db.execute('PRAGMA user_version').fetchone()==(37,)
        request=db.execute('SELECT request FROM events WHERE id=?',('create-r3g',)).fetchone()[0]
    frame=dict(Name='public-replay',ID='',Kind='work.create',Event='create-r3g',Request=request.hex())
    raw=json.dumps(frame).encode()+b'\n'
    native=json.loads(run('command-public-native-replay',[str(probe),str(fixture)],raw));assert native['code']=='replay'and json.loads(bytes.fromhex(native['output']))==updated and digest()==stamp
    wrong=dict(frame,Request=(request+b' ').hex());bad=json.loads(run('command-public-native-conflict',[str(probe),str(fixture)],json.dumps(wrong).encode()+b'\n'));assert bad['code']=='event_conflict'and digest()==stamp
    again=json.loads(run('command-public-native-recovery',[str(probe),str(fixture)],raw));assert again==native and digest()==stamp
    # A fresh observation is not business permission, even for invalid input.
    fresh=dict(frame,Name='not-a-permit',ID='missing',Event='new-r3g',Request=b'bad request'.hex())
    pending=json.loads(run('command-public-native-new',[str(probe),str(fixture)],json.dumps(fresh).encode()+b'\n'));assert pending['code']=='new'and digest()==stamp
    with open('/dev/full','wb')as sink:p=subprocess.run([str(probe),str(fixture)],env=env,input=raw,stdout=sink,stderr=subprocess.PIPE)
    (output/'command-public-native-full.stderr').write_bytes(p.stderr);assert p.returncode!=0 and p.stderr and digest()==stamp
    result=dict(status='PASS',actual_go_public_create_update_replay=True,current_revision=2,original_expected_revision=0,native_replay_equal=True,conflict_and_recovery=True,absence_does_not_authorize=True,native_output_failure_refused=True,unchanged_database_after_checks=True,database_sha256=stamp,native_public_mutation=False,full_mutation_guards=False)
    save('command-public-flow.json',result);return result
