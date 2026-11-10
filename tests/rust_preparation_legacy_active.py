#!/usr/bin/env python3
"""Historical active recovery through real CLI/HTTP on an owned inert Root.

Synthetic Go-format rows only; no real provider or legacy executable is launched.
A separately owned inert child supplies kernel identity observations. SQL writes
seed this fixture exclusively, never a live application Root. No watchdog/retry.
"""
import argparse,json,os,sqlite3,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native


def main():
    """Assert observation, explicit stop, reservation preservation and rollback."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');ap.add_argument('--browser',action='store_true');a=ap.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
    f=fixture.Fixture(Path(a.binary).resolve(),out,'legacy-active');f.settings_change(preparation_write_enabled='1')
    p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='legacy-active-create',expected_revision=0,text='Owned legacy recovery fixture'))
    dbpath=f.root/'.swarm/state.db';checks=[];server=None;child=None;primary=None
    def check(name,value):
        """Retain named content/state assertions, refusing failed prerequisites."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    host=os.uname().nodename+':'+Path('/proc/sys/kernel/random/boot_id').read_text().strip()+':'+os.readlink('/proc/self/ns/pid')
    def seed(status='pending',state='reserved',**extras):
        """Install one synthetic legacy row and its exact ledger in this fixture."""
        t=dict(id='pt-legacy-active-fixture',preparation_id=p['id'],status=status,source_revision=p['revision'],method_hash=p['method_hash'],created_at='2026-10-27T00:00:00Z',provider='api-historical',provider_digest='historical',request_hash='historical',question='Historical question',prompt='PRIVATE_LEGACY_PROMPT',opaque_extension={'keep':'PRIVATE_EXTENSION'},**extras)
        with sqlite3.connect(dbpath) as db:
            db.execute('DELETE FROM preparation_turns WHERE id=?',(t['id'],));db.execute('INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?)',(t['id'],p['id'],status,json.dumps(t).encode()))
            db.execute('CREATE TABLE IF NOT EXISTS preparation_reservations(turn_id TEXT PRIMARY KEY,preparation_id TEXT,work_id TEXT,amount REAL,work_amount REAL,state TEXT)')
            db.execute('DELETE FROM preparation_reservations WHERE turn_id=?',(t['id'],))
            if state is not None:db.execute('INSERT INTO preparation_reservations VALUES(?,?,?,?,?,?)',(t['id'],p['id'],'',2.5,0,state))
        return t
    def snapshot():
        """Read exact turn bytes and ledger; no normalization hiding mutations."""
        with sqlite3.connect(dbpath) as db:return db.execute('SELECT status,body FROM preparation_turns WHERE id=?',('pt-legacy-active-fixture',)).fetchone(),db.execute('SELECT * FROM preparation_reservations').fetchall()
    def stop():return f.call(['prepare','stop',p['id']],dict(preparation_id=p['id'],turn_id='pt-legacy-active-fixture'))
    try:
        t=seed();before=snapshot();rc,result=f.call(['prepare','dialogue',p['id']])
        if a.expect_unported:
            check('legacy-active-baseline-history-refuses',rc!=0 and snapshot()==before)
            rc,result=stop();check('legacy-active-baseline-stop-refuses',rc!=0 and snapshot()==before);print('LEGACY_ACTIVE_BASELINE 2');return
        check('legacy-active-readonly-observation',rc==0 and result[0]['status']=='pending' and snapshot()==before)
        check('legacy-active-private-fields-not-public',all(s not in json.dumps(result) for s in ['PRIVATE_LEGACY_PROMPT','PRIVATE_EXTENSION','_native']))
        rc,result=stop();check('legacy-pending-interrupted',rc==0 and result['status']=='interrupted' and result['ended_at'])
        after=snapshot();saved=json.loads(after[0][1]);check('legacy-pending-reservation-released',after[1][0][-1]=='released' and after[1][0][3:5]==(2.5,0))
        check('legacy-raw-prefix-unknown-prompt-preserved',saved['prompt']==t['prompt'] and saved['opaque_extension']==t['opaque_extension'] and '_native' not in saved and saved['status']=='interrupted')
        rc,result=stop();check('legacy-terminal-stop-idempotent',rc==0 and snapshot()==after)
        with sqlite3.connect(dbpath) as db:n=db.execute("UPDATE preparation_turns SET status='running' WHERE id=? AND status='pending'",(t['id'],)).rowcount
        check('legacy-go-claim-fenced',n==0 and snapshot()==after)
        with sqlite3.connect(dbpath) as db:n=db.execute("UPDATE preparation_turns SET status='answered' WHERE id=? AND status='running'",(t['id'],)).rowcount
        check('legacy-go-finish-fenced',n==0 and snapshot()==after)
        child=subprocess.Popen(['python3','-c','import sys; sys.stdin.buffer.read()'],stdin=subprocess.PIPE)
        stamp=Path(f'/proc/{child.pid}/stat').read_text().rsplit(')',1)[1].split()[19]
        seed('running','estimated',supervisor_pid=child.pid,supervisor_stamp=stamp,host=host)
        rc,result=stop();check('legacy-live-stopping-no-signal',rc==0 and result['status']=='stopping' and child.poll() is None and snapshot()[1][0][-1]=='estimated')
        before=snapshot();rc,result=stop();check('legacy-live-stopping-idempotent',rc==0 and snapshot()==before and child.poll() is None)
        child.stdin.close();child.wait();child=None
        rc,result=stop();check('legacy-stopping-dead-recovery',rc==0 and result['status']=='interrupted' and snapshot()[1][0][-1]=='estimated')
        seed('running','reserved',supervisor_pid=os.getpid(),supervisor_stamp=str(int(Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()[19])+1),host=host)
        rc,result=stop();check('legacy-reused-stamp-no-signal',rc==0 and result['status']=='interrupted' and snapshot()[1][0][-1]=='released')
        for label,kwargs in [('missing',{}),('empty-stamp',dict(supervisor_pid=os.getpid(),host=host)),('foreign-host',dict(supervisor_pid=os.getpid(),supervisor_stamp='1',host='foreign-host'))]:
            seed('running','reserved',**kwargs);before=snapshot();rc,result=stop();check('legacy-unknown-'+label+'-refuses-no-write',rc!=0 and snapshot()==before)
        seed(_native=None);before=snapshot();rc,result=stop();check('legacy-null-native-not-adopted',rc!=0 and snapshot()==before)
        seed(state=None);before=snapshot();rc,result=stop();check('legacy-missing-ledger-refuses-and-rolls-back',rc!=0 and snapshot()==before)
        seed()
        with sqlite3.connect(dbpath) as db:db.execute('UPDATE preparation_reservations SET amount=-1')
        before=snapshot();rc,result=stop();check('legacy-invalid-ledger-refuses-and-rolls-back',rc!=0 and snapshot()==before)
        seed();before=snapshot()
        with sqlite3.connect(dbpath) as db:db.execute("CREATE TRIGGER legacy_sabotage AFTER UPDATE ON preparation_reservations BEGIN UPDATE preparation_turns SET status='running' WHERE id=NEW.turn_id; END")
        rc,result=stop();check('legacy-trigger-tamper-rolls-back',rc!=0 and snapshot()==before)
        with sqlite3.connect(dbpath) as db:db.execute('DROP TRIGGER legacy_sabotage')
        rc,result=stop();check('legacy-trigger-removal-recovers',rc==0 and result['status']=='interrupted')
        seed();server=native.Server(f.binary,f.root,out,'legacy-active-http',environment=f.environment);server.capture_async(out/'http.raw');csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        turns=server.request('/api/v1/preparations/dialogue?id='+p['id'],observe=False);check('legacy-active-http-history',turns[0]['status']=='pending' and 'prompt' not in turns[0])
        before=snapshot();server.request('/api/v1/preparations/stop',dict(preparation_id=p['id'],turn_id=t['id']),status=403,observe=False);check('legacy-http-csrf-refuses-no-write',snapshot()==before)
        result=server.request('/api/v1/preparations/stop',dict(preparation_id=p['id'],turn_id=t['id']),csrf=csrf,observe=False);check('legacy-http-stop',result['status']=='interrupted' and snapshot()[1][0][-1]=='released')
        check('legacy-calls-still-disabled',f.ok(['engine-settings','show'])['values']['preparation_call_enabled']=='0')
        if a.browser:
            # This fixture normally registers a fake CLI provider. Explicitly
            # remove that fixture configuration to exercise the readonly path;
            # policy remains disabled, and no provider was ever contacted.
            fixture.publish(f.providers,dict(schema_version=1,providers={}))
            seed('running','estimated',supervisor_pid=os.getpid(),supervisor_stamp=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()[19],host=host)
            run=subprocess.run(['node','tests/rust_preparation_legacy_active_ui.cjs'],input=json.dumps(dict(address=server.address,cookie=server.cookie,preparation=p['id'],output=str(out))).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            fixture.publish(out/'browser.stdout',run.stdout);fixture.publish(out/'browser.stderr',run.stderr)
            print(run.stdout.decode(),end='',flush=True);assert run.returncode==0,run.stderr.decode()
        fixture.publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=fixture.digest(f.binary.read_bytes()),root=str(f.root),limitations='Synthetic history and owned inert supervisor only; no old Go executable, provider descendant termination or remote billing proof.'))
        print('LEGACY_ACTIVE_PASS '+str(len(checks)))
    except BaseException as e:primary=e
    finally:
        if child is not None:
            child.stdin.close();child.wait()
        if server is not None:native.finish(server,primary)
        elif primary is not None:raise primary
if __name__=='__main__':main()
