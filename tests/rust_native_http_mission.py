#!/usr/bin/env python3
"""Exercise the native mission HTTP bridge against actual CLI contracts.

Owned public CLI setup, real authenticated TCP, no live Root/Go or
watchdog. Write setup uses existing inert planning processes; HTTP adds none.
Reads, exact replay and refusals compare the complete logical database.
A byte-identical database at a different inode must refuse under the server's
retained binding, then recover after original restoration. This is a native HTTP
extension, not historical Go snapshot or autonomous server-conductor acceptance.

Historical bridge-only journey bound to the pre-conductor1156 candidate. Its
no-agent/no-supervision writer assertions are superseded for the current `web`
entry by rust_native_web_conductor.py; native-http-mission.json now runs that
journey. Shared request, comparison and physical-restoration helpers remain used.
"""
from pathlib import Path
import argparse, contextlib, copy, http.client, json, os, shutil, traceback
from datetime import datetime
from urllib.parse import urlencode
import rust_agent_model_route as fixture
import rust_native_http_cli as native
import rust_planning_worker_public_flow as planning
from rust_mission_task_order import snapshot
from rust_mission_report_relay import rows


def request(server, method, path, body=None, csrf=None, authorized=True, expected=200):
    """Issue one real request and drain exactly its owned observation when admitted.

    Fixed access-refusal403 is pre-Root and has no Root outcome. HEAD has no body;
    other admitted replies must carry JSON/no-store. Private cookie/CSRF remain
    only in memory. HTTP/transport/decoder failures propagate; no retry or timeout.
    """
    headers={'Origin':'http://'+server.address}
    if authorized: headers['Cookie']=server.cookie
    if csrf is not None: headers['X-Swarm-CSRF']=csrf
    raw=None if body is None else json.dumps(body,separators=(',',':')).encode()
    if raw is not None: headers['Content-Type']='application/json'
    connection=http.client.HTTPConnection(server.host,server.port,timeout=None)
    try:
        connection.request(method,path,body=raw,headers=headers)
        response=connection.getresponse();data=response.read();status=response.status
        content_type=response.getheader('Content-Type','');cache=response.getheader('Cache-Control','')
    finally: connection.close()
    assert status==expected, (method,path,status,data[:300])
    if expected==403:
        assert data==b'web-access-refused'
        parsed='web-access-refused'
    else:
        assert content_type.startswith('application/json') and cache=='no-store'
        if method=='HEAD': assert data==b'';parsed=None
        else: parsed=json.loads(data)
        frame=server.record();assert frame['event']=='web-outcome' and frame['outcome']['status']==expected
        assert server.key not in json.dumps(frame)
    server.requests.append(dict(method=method,path=path,status=status,response=native.clean(parsed)))
    return parsed


@contextlib.contextmanager
def foreign_inode(database):
    """Temporarily replace one quiescent owned fixture DB with identical bytes.

    Caller owns the fresh fixture and all requests; no live/project DB is accepted
    by this recipe. Retain original pathname before substitution, then restore it
    after the check. Every restoration is attempted; primary and independent cleanup causes are grouped.
    Only the read-only identity check may run inside this context.
    This manipulates fixture identity only, never grants Root or process authority.
    """
    original=database.with_name('state.original-owned.db')
    replacement=database.with_name('state.replacement-owned.db')
    assert not original.exists() and not replacement.exists()
    shutil.copy2(database,replacement)
    os.replace(database,original)
    errors=[]
    try:
        os.replace(replacement,database)
        yield
    except BaseException as error: errors.append(error)
    finally:
        try:
            if database.exists(): os.replace(database,replacement)
        except BaseException as error: errors.append(error)
        # Independently attempt restoration even if retaining our identical copy failed.
        try: os.replace(original,database)
        except BaseException as error: errors.append(error)
    if errors: raise BaseExceptionGroup('Owned fixture observation/copy retention/restoration',errors)


def comparable_status(value):
    """Compare every status field except four declared live observation paths.

    Return a deep comparison copy and the exact observed values. Storage/spending timestamps and
    free-byte/inode counters are independently sampled by each process; validate
    their types without inventing freshness/performance bounds. Keep states,
    permissions, paths, version diagnostics and all task facts exact. Originals
    remain unchanged in retained CLI/HTTP channels; this is no product fallback.
    """
    result=copy.deepcopy(value);runtime=result['runtime']
    stamp=runtime['observed_at'];assert isinstance(stamp,str) and stamp.endswith('Z')
    datetime.fromisoformat(stamp)
    measurements={'observed_at':stamp,'volumes':[]};runtime['observed_at']='<sample-instant>'
    spending_stamp=result['spending']['observed_at'];assert isinstance(spending_stamp,str) and spending_stamp.endswith('Z')
    datetime.fromisoformat(spending_stamp);measurements['spending_observed_at']=spending_stamp
    result['spending']['observed_at']='<sample-instant>'
    for volume in runtime['volumes']:
        actual={'kind':volume['kind']}
        for field in ['available_bytes','available_inodes']:
            number=volume[field];assert isinstance(number,int) and not isinstance(number,bool) and number>=0
            actual[field]=number;volume[field]='<sample-value>'
        measurements['volumes'].append(actual)
    return result,measurements


def run(binary, profile, out):
    """Qualify reads, all six writes, rejection/replay/reopen and physical binding.

    Public setup uses an ordinary read fixture and a structurally ready planning
    fixture for writes. Planning setup runs inert local processes before HTTP;
    subsequent commands must leave that history/consumption intact. Start validates an
    existing local profile but must never launch a process or acquire a lease.
    Every malformed/stale/unauthorized variant follows a valid witness and keeps
    the actual DB unchanged. All owned servers close on failures; fixtures and
    original channels remain. This does not inject real close/output IO faults.
    """
    qualified=json.loads(profile.read_bytes())
    assert qualified['no_fixture_feature'] and fixture.digest(binary.read_bytes())==qualified['sha256']
    f=fixture.Fixture(binary,out/'setup','mission-http');(f.root/'mission-worker').mkdir()
    fixture.publish(out/'fixture.json',dict(root=str(f.root),work=f.work,binary_sha256=qualified['sha256']))
    program=fixture.digest((f.root/'codex').read_bytes());checks=[];server=native.Server(binary,f.root,out,'mission',environment=f.environment);primary=None
    path=lambda action=None:'/api/v1/mission?'+urlencode(dict(work=f.work,**({} if action is None else dict(action=action))))
    try:
        session=server.request('/api/v1/native-session');csrf=session['csrf_token']
        capabilities=server.request('/api/v1/native-capabilities')['mission']
        assert set(capabilities)=={'show','order','queue','status','start','pause','unpause','stop','resume','priority'}
        before=snapshot(f)
        for action in ['show','order','queue','status']:
            expected=f.ok(['mission',action,f.work]);actual=request(server,'GET',path(action))
            if action=='status':
                left,left_samples=comparable_status(expected);right,right_samples=comparable_status(actual)
                fixture.publish(out/'status-independent-samples.json',dict(cli=left_samples,http=right_samples,allowed_differences=['runtime.observed_at','runtime.volumes[*].available_bytes','runtime.volumes[*].available_inodes','spending.observed_at']))
                assert left==right,action
                outcome=server.records[-1]['outcome'];assert outcome['error'] is None and outcome['observation_causes']
                assert all(isinstance(code,str) and code for code in outcome['observation_causes'])
            else: assert actual==expected,action
            assert snapshot(f)==before and server.records[-1]['outcome']['recorded_work'] is None
            checks.append('read-'+action+'-equals-cli-no-mutation')
        assert request(server,'GET',path())==f.ok(['mission','show',f.work]);request(server,'HEAD',path('queue'))
        for target in ['/api/v1/mission?work=../escape',path()+'&work='+f.work,path()+'&root=elsewhere',path()+'&action=']:
            request(server,'GET',target,expected=400);assert snapshot(f)==before
        request(server,'GET',path('start'),expected=501)
        request(server,'GET',path(),body={},expected=400)
        request(server,'GET','/api/v1/mission?work=missing-work',expected=404)
        request(server,'GET',path(),authorized=False,expected=403)
        request(server,'PUT',path(),body={},csrf=csrf,expected=405)
        assert snapshot(f)==before;assert request(server,'GET',path())==f.ok(['mission','show',f.work])
        checks.append('read-refusals-head-auth-and-recovery')

        # Start requires the same real organization as CLI; do not weaken the gate.
        previous=server;server=None;stop_error=None
        try: previous.stop()
        except BaseException as error: stop_error=error
        native.finish(previous,stop_error)
        ready,f=planning.flow(binary,out/'planned-setup',before_worker=True,return_fixture=True)
        assert ready['worker_started'] is False
        (f.root/'mission-worker').mkdir()
        program=fixture.digest((f.root/'codex').read_bytes())
        finance_before=rows(f,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(f.work,))
        fixture.publish(out/'finance-before.json',finance_before)
        before_execution=f.ok(['mission','status-native',f.work]);assert before_execution['agents']==[] and before_execution['supervision'] is None
        fixture.publish(out/'planned-fixture.json',dict(root=str(f.root),work=f.work,setup='public inert planning; no worker admitted'))
        server=native.Server(binary,f.root,out,'configured',environment=f.environment)
        csrf=server.request('/api/v1/native-session')['csrf_token']


        def document(event, **extra):
            """Bind a new explicit envelope to the current actual Work revision."""
            current=f.ok(['work','show',f.work])['work']
            return dict(schema_version=1,event_id=event,expected_revision=current['revision'],reason='Owned HTTP mission fixture',**extra)
        def apply(action, body):
            """Require actual mutation, exact logical revocation, preserved counters and Work equality and exact replay."""
            before_work=f.ok(['work','show',f.work])['work'];before_revision=before_work['revision']
            actual=request(server,'POST',path(action),body,csrf)
            assert actual['revision']==before_revision+1
            expected_planning=copy.deepcopy(before_work.get('planning'))
            if action in ['pause','stop'] and expected_planning is not None:
                # Exact documented revoke: advance generations, clear logical claims;
                # preserve all counters, delivery originals, history and financing.
                for scope in expected_planning['scopes']:
                    scope['generation']+=1;scope.pop('holder',None);scope.pop('lease_until',None)
            assert actual.get('planning')==expected_planning
            assert actual.get('tasks')==before_work.get('tasks')
            assert actual==f.ok(['work','show',f.work])['work']
            recorded=server.records[-1]['outcome']['recorded_work'];assert recorded=={'work':f.work,'revision':actual['revision']}
            saved=snapshot(f);assert f.ok(['mission',action,f.work],body)==actual and snapshot(f)==saved
            assert request(server,'POST',path(action),body,csrf)==actual and snapshot(f)==saved
            checks.append('write-'+action+'-persisted-cli-http-replay')
            return actual
        start=document('http-start',profile=dict(provider='fixture',role='worker',workspace='mission-worker',capture_output=True),slots=1)
        apply('start',start);view=request(server,'GET',path());assert view['consent_enabled'] is True and view['paused'] is False
        apply('pause',document('http-pause'));assert request(server,'GET',path())['paused'] is True
        apply('resume',document('http-resume'));assert request(server,'GET',path())['paused'] is False
        apply('priority',document('http-priority',task_id='task',priority='9223372036854775807'))
        assert request(server,'GET',path('order'))['tasks'][0]['priority']=='9223372036854775807'
        before=snapshot(f);valid=document('fault-witness')
        for action,patch in [('pause',dict(actor='forged')),('resume',dict(slots=2)),('priority',dict(task_id='task',priority=0)),('start',dict(slots=0))]:
            request(server,'POST',path(action),{**valid,**patch},csrf,expected=400);assert snapshot(f)==before
        request(server,'POST',path('pause'),valid,expected=403);request(server,'POST',path('pause'),valid,'wrong-csrf',expected=403)
        request(server,'POST',path('show'),valid,csrf,expected=501);request(server,'POST',path(),valid,csrf,expected=501)
        stale={**valid,'expected_revision':valid['expected_revision']-1}
        request(server,'POST',path('pause'),stale,csrf,expected=409);assert snapshot(f)==before
        checks.append('write-auth-schema-stale-refusals-without-mutation')
        apply('stop',document('http-stop'));assert request(server,'GET',path())['consent_enabled'] is False
        apply('unpause',document('http-unpause'));assert request(server,'GET',path())['consent_enabled'] is False
        before=snapshot(f);request(server,'POST',path('resume'),document('no-consent'),csrf,expected=400);assert snapshot(f)==before
        apply('start',document('http-restart-consent'))
        checks.append('unpause-is-not-consent-and-start-recovery')

        # Quiescent owned fixture only. Retain both originals before replacement.
        database=f.root/'.swarm/state.db'
        before=snapshot(f);original_sha=fixture.digest(database.read_bytes())
        with foreign_inode(database):
            request(server,'GET',path(),expected=409)
        assert fixture.digest(database.read_bytes())==original_sha and snapshot(f)==before
        assert request(server,'GET',path())==f.ok(['mission','show',f.work])
        checks.append('byte-identical-foreign-inode-refused-and-original-recovered')
        assert f.ok(['mission','status-native',f.work])['agents']==[] and f.ok(['mission','status-native',f.work])['supervision'] is None
        assert not (f.root/'model-received.json').exists() and fixture.digest((f.root/'codex').read_bytes())==program
        finance_after=rows(f,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(f.work,))
        fixture.publish(out/'finance-after.json',finance_after);assert finance_after==finance_before
        assert all(task['status']!='accepted' for task in f.ok(['work','show',f.work])['work']['tasks'])
        checks.append('http-preserves-original-planning-calls-and-financing')
        summary=server.stop();assert summary['completed_jobs']==summary['observed_outcomes']
    except BaseException as error: primary=error
    if server is None:
        if primary is not None: raise primary
    else: native.finish(server,primary)
    # The same saved mission state survives a real owned HTTP process restart.
    server=native.Server(binary,f.root,out,'reopened',environment=f.environment);primary=None
    try:
        assert request(server,'GET',path())==f.ok(['mission','show',f.work])
        assert f.ok(['mission','status-native',f.work])['agents']==[] and f.ok(['mission','status-native',f.work])['supervision'] is None and not (f.root/'model-received.json').exists()
        server.stop();checks.append('reopen-retains-controls-without-agent-or-conductor-start')
    except BaseException as error: primary=error
    native.finish(server,primary)
    result=dict(status='PASS',checks=checks,count=len(checks),root=str(f.root),work=f.work,profile_sha256=qualified['sha256'],native_extension=True,automatic_web_conductor=False,setup_inert_planning=True,http_provider_started=False,task_accepted=False,installed=False,global_watchdog=None)
    fixture.publish(out/'result.json',result)
    for check in checks: print('PASS '+check,flush=True)
    print('NATIVE_HTTP_MISSION_PASS',flush=True)


def main():
    """Require exact qualified binary/profile and fresh retained evidence output."""
    parser=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:parser.add_argument('--'+flag,type=Path,required=True)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(mode=0o700)
    fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:run(args.binary.resolve(),args.profile.resolve(),out)
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
