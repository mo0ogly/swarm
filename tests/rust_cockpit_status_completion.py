"""Exercise actual status reads overlapping the completion of inert workers.

HTTP readers sample the protected mission status while the owned workers end.
The baseline expects a server error; the candidate expects a typed conflict.
Both witness stable reads before and after. The browser checks the recovered
state and plays back the conflict body for frontend coverage. The smoke run
reports whether it sampled a conflict. Baseline and --require-overlap assert
actual overlap. No paid provider, live Root or global watchdog is used.
"""
import http.client, json, os, select, subprocess, sys, threading, time, traceback, concurrent.futures
from contextlib import ExitStack
from pathlib import Path
import argparse
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_mission_watch_public as public
import rust_mission_watch_worker as worker
import rust_native_http_cli as native
import rust_native_parent_death as parent
import rust_planning_attach as attachment
import rust_planning_worker_public_flow as planning
from rust_orchestration_scopes import IDLE_WORK_PROGRAM

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--sha256', required=True)
parser.add_argument('--baseline', action='store_true')
parser.add_argument('--require-overlap', action='store_true')
args = parser.parse_args()
binary = args.binary.resolve(strict=True)
out = args.output.resolve()
out.mkdir(mode=0o700)
digest = fixture.digest(binary.read_bytes())
assert digest == args.sha256
browser_script = Path(__file__).with_suffix('.cjs')

def run():
    """Create real public mission configurations and hold only owned processes."""
    f=attachment.run(binary,out/'attachment',graph=True,return_fixture=True)
    root=f.root
    a=f.work
    first=root/'workspace-task'
    other=root/'workspace-parallel'
    shared=first/'partage'
    shared.mkdir(mode=0o700)
    nonce=fixture.digest(str(root).encode())
    def update(cmd, work, event, revision, **data):
        return f.ok(cmd+[work],dict(schema_version=1,event_id=event,expected_revision=revision,**data))
    b=f.ok(['work','create'],dict(schema_version=1,event_id='fifo-create',expected_revision=0,title='Mission B — attend l’espace partagé',objective='Observer le FIFO et reprendre',scope='Local inert fixture',criteria=['File réelle']))['work']['id']
    v=update(['task','add'],b,'fifo-add',1,id='fifo-worker',title='Reprendre l’espace partagé',deliverable='handoff.md',criteria=['Trace inerte'])['work']
    v=update(['task','update'],b,'fifo-authorize',v['revision'],id='fifo-worker',validation_policy={'mode':'human'})['work']
    control=dict(id='fifo-witness',command=['python3','-c','print(1)'],inputs=['input.txt'],justification='Contrôle local de fixture')
    update(['planning','attach'],b,'fifo-attach',v['revision'],provider='fixture',level='standard',max_activations=3,max_review_calls=3,max_tasks=1,max_decisions=3,checks={'req-1':[control]},task_scopes={'fifo-worker':'child'},task_workspaces={'fifo-worker':str(shared)})
    program_path=root/'codex'
    fixture.publish(program_path,IDLE_WORK_PROGRAM.encode())
    program_path.chmod(0o700)
    # Drain B's adopted attachment events via the real planner with inert empty
    # decisions. Same configured provider key, no remote adapter or account.
    while True:
        current=f.ok(['work','show',b])['work']
        pending=[e for e in current['planning']['inbox'] if not e.get('decision')]
        if not pending:break
        assert f.ok(['planning','step',b])['run']['decision_applied']
        after=f.ok(['work','show',b])['work']
        assert len([e for e in after['planning']['inbox'] if not e.get('decision')]) < len(pending)
    program=worker.provider_program(nonce).replace("'total_cost_usd':1","'total_cost_usd':0")
    gate_start=program.index('    with socket.socket(')
    gate_end=program.index('        if json.loads(',gate_start)
    program=program[:gate_start]+"""    while not Path('release-worker.json').exists():
        try:
            os.read(fd,65536)
        except BlockingIOError:
            import select
            select.select([fd],[],[])
"""+program[gate_end:].replace('        if json.loads(', '    if json.loads(', 1)
    fixture.publish(program_path,program.encode())
    program_path.chmod(0o700)
    fixture.publish(out/'inert-worker.py',program.encode())
    fixture.publish(out/'setup.json',dict(root=str(root),missions=[a,b],binary_sha256=digest,paid_provider=False,global_watchdog=None))
    with ExitStack() as stack:
        observed_directories={}
        for directory in (first,other,shared):
            fd=worker.watch_directory(directory,modifications=True)
            stack.callback(os.close,fd)
            observed_directories[fd]=directory
        server=native.Server(binary,root,out,'web',environment=f.environment)
        primary=None
        try:
            server.capture_async(out/'web-outcomes.ndjson')
            identity=worker.observation(server.process.pid)
            sf=worker.attach(identity)
            stack.callback(os.close,sf)
            assert fixture.digest(Path(f'/proc/{identity["pid"]}/exe').read_bytes())==digest
            get=lambda path:server.request(path,observe=False)
            path=lambda work,action:'/api/v1/mission?'+urlencode(dict(work=work,action=action))
            csrf=get('/api/v1/native-session')['csrf_token']
            def mission(work,action,event,**data):
                rev=f.ok(['work','show',work])['work']['revision']
                return server.request(path(work,action),dict(schema_version=1,event_id=event,expected_revision=rev,reason='Scénario cockpit inerte isolé',**data),csrf,observe=False)
            def wait(label, observe, accept):
                print('WAIT '+label,flush=True)
                while server.process.poll() is None:
                    value=observe()
                    if accept(value):return value
                    time.sleep(.5)
                raise RuntimeError('Serveur arrêté pendant '+label)
            def take():
                while True:
                    ready,_,_=select.select([sf,*observed_directories],[],[])
                    assert sf not in ready,'Web conductor exited before local provider grant'
                    for source in ready:
                        directory=observed_directories[source]
                        os.read(source,65536)
                        marker=directory/'model-received.json'
                        if not marker.exists():continue
                        try: entry=json.loads(marker.read_bytes())
                        except json.JSONDecodeError:continue
                        del observed_directories[source]
                        provider=worker.observation(entry['pid'])
                        supervisor=worker.observation(provider['ppid'])
                        assert provider['ppid']==entry['ppid'] and supervisor['ppid']==identity['pid']
                        proc=Path('/proc',str(supervisor['pid']))
                        assert fixture.digest((proc/'exe').read_bytes())==digest
                        command=(proc/'cmdline').read_bytes().split(b'\0')[:-1]
                        assert len(command)==6 and command[:2]==[os.fsencode(binary),b'_mission_worker_entry']
                        assert command[3:]==[b'--root',os.fsencode(root),b'--json']
                        sufd=worker.attach(supervisor)
                        stack.callback(os.close,sufd)
                        chfd=worker.attach(provider)
                        stack.callback(os.close,chfd)
                        ag=worker.wait_running(f,os.fsdecode(command[2]),sf,sufd)
                        assert ag['child_pid']==provider['pid'] and ag['native_execution']['phase']=='permission_recorded'
                        assert Path(os.readlink(f'/proc/{provider["pid"]}/cwd'))==directory
                        return ag,directory,chfd
            def cockpit(work):
                return get('/api/v1/cockpit-read?'+urlencode(dict(work=work)))['agents']
            def shot(phase, work, agents, conflict=None):
                config=dict(phase=phase,work=work,address=server.address,key=server.key,root=str(root),out=str(out),agents=agents,conflict=conflict)
                with (out/f'{phase}.stdout').open('xb') as stdout,(out/f'{phase}.stderr').open('xb') as stderr:
                    subprocess.run(['node',str(browser_script)],input=json.dumps(config).encode(),stdout=stdout,stderr=stderr,check=True)
                print((out/f'{phase}.stdout').read_text().strip(),flush=True)
            def release(directory):
                p=directory/'release-worker.json'
                if not p.exists():fixture.publish(p,dict(nonce=nonce,release=True))
            def profile(directory):
                return dict(provider='fixture',role='worker',workspace=str(directory),level='standard',capture_output=True)
            assert cockpit(a)==cockpit(b)==[]
            mission(a,'start','active-a',slots=2,profile=profile(first))
            print('WAIT deux workers A',flush=True)
            held={d:(ag,fd) for ag,d,fd in (take(),take())}
            assert set(held)=={first,other}
            assert all(not select.select([fd],[],[],0)[0] for _,fd in held.values())
            running=cockpit(a)
            assert {row['task_id'] for row in running if row['status']=='running'}=={'task','parallel'}
            fixture.publish(out/'parallel.json',running)
            mission(b,'start','active-b',slots=1,profile=profile(shared))
            queue=wait('tour B en attente',lambda:get(path(b,'queue')),lambda q:any(x['older_work']==a and x['task']=='fifo-worker' for x in q['blockers']))
            blocker,=(x for x in queue['blockers'] if x['older_work']==a and x['task']=='fifo-worker')
            assert blocker['older_task']=='task' and int(blocker['older_seq'])<int(blocker['candidate_seq'])
            assert cockpit(b)==[] and not (shared/'model-received.json').exists()
            diag=get(path(b,'launch-diagnostics'))
            assert any(x['task']=='fifo-worker' and 'workspace-turn-fifo-blocked' in x['codes'] for x in diag['refusals']),diag
            fixture.publish(out/'fifo.json',dict(queue=queue,diagnostics=diag,agents_b=[]))
            mission(a,'pause','pause-new-departures-a')
            assert any(x['seq']==blocker['older_seq'] and x['state']=='acquired' for x in get(path(b,'queue'))['turns'])
            release(first)
            print('WAIT reprise B après fin du worker A sur espace partagé',flush=True)
            ag,d,fd=take()
            assert d==shared and ag['task_id']=='fifo-worker' and ag['work_id']==b
            resumed=get(path(b,'queue'))
            assert any(x['seq']==blocker['older_seq'] and x['state']=='released' for x in resumed['turns'])
            assert any(x['seq']==blocker['candidate_seq'] and x['state']=='acquired' and x['agent']==ag['id'] for x in resumed['turns'])
            assert not resumed['blockers']
            assert not select.select([held[other][1],fd],[],[],0)[0]
            original=cockpit(a); restarted=cockpit(b)
            assert {x['task_id']:x['status'] for x in original}=={'task':'completed','parallel':'running'}
            assert [x['status'] for x in restarted]==['running']
            fixture.publish(out/'repris.json',dict(queue=resumed,agents_a=original,agents_b=restarted))
            mission(b,'pause','pause-new-departures-b')
            def observe_http():
                conn=http.client.HTTPConnection(server.host,server.port)
                try:
                    conn.request('GET',path(b,'status'),headers={'Origin':'http://'+server.address,'Cookie':server.cookie})
                    resp=conn.getresponse()
                    return resp.status,json.loads(resp.read())
                finally:conn.close()
            finished=threading.Event()
            entered=threading.Barrier(5)
            http_failures=[]
            cli_failures=[]
            def watch(http):
                entered.wait()
                count=0
                while not finished.is_set():
                    if http:
                        code,value=observe_http()
                        if code>=400:http_failures.append((code,value))
                    else:
                        result=subprocess.run([str(binary),'mission','status',b,'--root',str(root),'--json'],env=f.environment,input=b'',capture_output=True,check=False)
                        if result.returncode:cli_failures.append(json.loads(result.stderr))
                    count+=1
                return count
            assert observe_http()[0]==200
            with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
                futures=[pool.submit(watch,i<3) for i in range(4)]
                entered.wait()
                try:
                    release(shared)
                    release(other)
                    final_b=wait('fin B',lambda:cockpit(b),lambda rows:rows and all(x['status']=='completed' for x in rows))
                    final_a=wait('fin A',lambda:cockpit(a),lambda rows:len(rows)==2 and all(x['status']=='completed' for x in rows))
                finally:finished.set()
                sample_counts=[f.result() for f in futures]
            fixture.publish(out/'race.json',dict(http_failures=http_failures,cli_failures=cli_failures,sample_counts=sample_counts))
            print('RACE',len(http_failures),len(cli_failures),sample_counts,flush=True)
            assert not any(t['status']=='accepted' for work in (a,b) for t in f.ok(['work','show',work])['work']['tasks'])
            fixture.publish(out/'complete.json',dict(agents_a=final_a,agents_b=final_b))
            stable_code, stable = observe_http()
            assert stable_code == 200 and stable['active_agents'] == 0
            assert any(task['id'] == 'fifo-worker' for task in stable['tasks'])
            fixture.publish(out/'stable-status.json', stable)
            assert sum(sample_counts) > 0
            if args.baseline or args.require_overlap:
                assert http_failures, 'completion overlap was not observed'
            expected = (500, 'root-cleanup') if args.baseline else (409, 'mission-status-observation-changed')
            assert all((status, body['failure']['code']) == expected for status, body in http_failures), http_failures
            for wire in cli_failures:
                failure = wire['failure']
                assert failure['code'] == expected[1], failure
                assert any(c.startswith('mission-status-') and c.endswith('-changed')
                           for c in failure['causes']), failure
            if not args.baseline:
                shot('completion-stable', b, final_b)
                body = http_failures[0][1] if http_failures else dict(failure=dict(code=expected[1]))
                print('Conflict presentation source:', 'actual HTTP reply' if http_failures else 'contract fixture', flush=True)
                shot('completion-conflict', b, final_b, conflict=body)
            print('PASS status endpoint recovered after worker completion', flush=True)
        except BaseException as exc:
            primary=exc
            raise
        finally:
            errors=[]
            worker.retain_failure(errors,lambda:native.finish(server,None))
            for directory in (first,other,shared):worker.retain_failure(errors,lambda directory=directory:release(directory))
            worker.retain_failure(errors,lambda:public.cleanup_owned_children(binary,root,out))
            if errors:raise BaseExceptionGroup('scenario et nettoyage',([primary] if primary else [])+errors)
        assert server.process.returncode==0
        fixture.publish(out/'result.json',dict(status='PASS',binary_sha256=digest,baseline=args.baseline,paid_provider_calls=0,global_watchdog=None))

worker.subreaper_scope(run,lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent.subreaper)
