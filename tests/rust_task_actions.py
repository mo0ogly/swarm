#!/usr/bin/env python3
"""Exercise action files with an owned Rust worker, HTTP and Chrome.

No provider account, live mission, SQL seed or global watchdog. The fixture
declares known tool events; the engine still owns execution, capture and review.
"""
import argparse
import json
import os
import subprocess
import time
from pathlib import Path
from urllib.parse import urlencode

import rust_agent_model_route as fixture
import rust_manual_mission as mission
import rust_native_http_cli as native

EVENTS = """
def emit(kind, content):
 print(json.dumps({'type':kind,'message':{'content':[content]}}),flush=True)
start = {'type':'tool_use','id':'private-provider-read-id','name':'Read','input':{'path':'NEVER-EXPOSE-INPUT'}}
end = {'type':'tool_result','tool_use_id':'private-provider-read-id','content':'NEVER-EXPOSE-OUTPUT'}
emit('assistant',start)
# The coordinator admits the result only after reading the recorded action.
import time
from pathlib import Path
while not Path('release-journal-action').exists():
 time.sleep(0.05)
emit('user',end)
emit('assistant',start)
emit('user',end)
emit('assistant',{'type':'tool_use','id':'private-provider-failed-id','name':'Edit','input':{'path':'NEVER-EXPOSE-INPUT'}})
emit('user',{'type':'tool_result','tool_use_id':'private-provider-failed-id','is_error':True,'content':'NEVER-EXPOSE-OUTPUT'})
"""


def main():
    """Read before launch, after completion, after human attention and an explicit premature acceptance refusal."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--browser',action='store_true')
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.output.resolve()
    out.mkdir(exist_ok=False)
    owned = fixture.Fixture(binary,out,'task-actions')
    original = "print(json.dumps({'type':'result'"
    assert mission.PROGRAM.count(original) == 1
    program = mission.PROGRAM.replace(original, EVENTS + original)
    fixture.publish(owned.root/'codex',program.encode())
    os.chmod(owned.root/'codex',0o700)
    work = owned.ok(['work','show',owned.work])['work']
    work = owned.ok(['task','update',owned.work],dict(schema_version=1,event_id='journal-human',
        expected_revision=work['revision'],id='task',
        next='Check result and adjust if needed', validation_policy=dict(mode='human')))['work']
    work = owned.ok(['planning','attach',owned.work],dict(schema_version=1,event_id='journal-attach',
        expected_revision=work['revision'],provider='fixture',level='standard',
        max_activations=3,max_review_calls=3,max_tasks=3,max_decisions=3,
        task_scopes={'task':'journal-scope'},task_workspaces={'task':str(owned.workspace)},
        checks={'req-1':[dict(id='root-check',command=['python3','-c','print(1)'],
            justification='Bind the owned root requirement')]}))
    owned.request.update(expected_revision=work['revision'], capture_output=False)
    server = native.Server(binary,owned.root,out,'task-actions',environment=owned.environment)
    primary = None
    try:
        server.capture_async(out/'frames.jsonl')
        path = '/api/v1/task-actions?' + urlencode(dict(work=owned.work,task='task'))
        def read():
            """Read the protected file through the native HTTP adapter."""
            return server.request(path,observe=False)
        empty = read()
        assert empty['revision']==work['revision'] and empty['plan']['deliverable']
        assert empty['attempts']==[] and empty['review']==dict(mode='human',awaiting_human=False)
        print('PASS protected file exists before the first attempt',flush=True)
        server.request(path,authorized=False,status=403,observe=False)
        server.request(path+'&task=other',status=400,observe=False)
        missing=server.request(path.replace('task=task','task=missing'),status=404,observe=False)
        assert missing['failure']['code']=='task-actions-task-missing'
        assert read()==empty
        preview=owned.prepare(True)
        owned.request['model_policy_hash']=preview['agent']['model_route']['policy_hash']
        prepared=owned.prepare()
        def observe_runner():
            """Hold a valid inert event pending until the actual heartbeat is read."""
            argv=[str(binary),'agent','run','session','--root',str(owned.root),'--json']
            with (out/'worker.stdout').open('xb') as stdout, (out/'worker.stderr').open('xb') as stderr:
                with subprocess.Popen(argv,stdout=stdout,stderr=stderr,env=owned.environment) as worker:
                    try:
                        while worker.poll() is None:
                            agent=owned.ok(['agent','show','session'])['agent']
                            if agent['progress'].get('tool_calls',0)>=1:
                                live=read()
                                attempt=live['attempts'][0]
                                assert attempt['source']=='last_heartbeat',attempt
                                assert attempt['observation']['actions']==[
                                    dict(number=1,family='read',outcome='pending')
                                ],attempt
                                print('PASS heartbeat exposes pending action in the protected file',flush=True)
                                break
                            time.sleep(0.05)
                        else:
                            raise AssertionError('fixture stopped before the pending heartbeat was observed')
                    finally:
                        (owned.workspace/'release-journal-action').touch()
                    assert worker.wait()==0
            return json.loads((out/'worker.stdout').read_bytes())
        owned.run_and_assert(prepared,run_observed=observe_runner)
        observed=read()
        assert len(observed['attempts'])==1
        attempt=observed['attempts'][0]
        assert attempt['id']==prepared['agent']['attempt_id'] and attempt['source']=='terminal_receipt'
        assert attempt['observation']['actions']==[
            dict(number=1,family='read',outcome='succeeded'),
            dict(number=2,family='write',outcome='failed'),
        ],attempt
        assert all(secret not in json.dumps(observed) for secret in ['NEVER-EXPOSE','private-provider'])
        print('PASS actual provider stream stores ordered redacted actions and deduplicates replay',flush=True)
        before=owned.ok(['work','show',owned.work])
        report=owned.workspace/'handoff.md'
        owned.ok(['task','submit',owned.work],dict(schema_version=1,event_id='journal-submit',
            expected_revision=before['work']['revision'],id='task',agent_id='session',
            report_path=str(report.relative_to(owned.root)),report_sha256=fixture.digest(report.read_bytes())))
        submitted=read()
        assert submitted['status']=='submitted' and submitted['review']['awaiting_human']
        assert submitted['attempts']==observed['attempts']
        assert read()==submitted and owned.ok(['work','show',owned.work])['work']['revision']==submitted['revision']
        print('PASS human gate waits after actual native handoff and HTTP is read only',flush=True)
        if args.browser:
            case=dict(address=server.address,key=server.key,work=owned.work,output=str(out))
            subprocess.run(['node','tests/rust_task_actions_ui.cjs'],input=json.dumps(case),text=True,check=True)
        code, refusal=owned.call(['task','update',owned.work],mission.update(owned,'accepted','journal-incomplete-accept'))
        assert code!=0 and refusal['failure']['code'] and read()==submitted
        print('PASS acceptance without required evidence is refused and preserves action history',flush=True)
        fixture.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,
            checked_revision=submitted['revision'],binary_sha256=fixture.digest(binary.read_bytes())))
    except BaseException as error:
        primary=error
    finally:
        native.finish(server,primary)


if __name__=='__main__':
    main()
