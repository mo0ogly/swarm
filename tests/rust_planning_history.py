#!/usr/bin/env python3
"""Historical planning data on owned Roots; synthetic events are not decisions applied.
Go/native normal binaries, checked controls/refusal/recovery; no provider, acceptance,
live datastore mutation, watchdog or invented event-chain validity.
"""
import argparse,json,http.client,re,sqlite3,subprocess,traceback,urllib.parse
from pathlib import Path
from rust_agent_model_route import Fixture,publish,digest
from rust_native_http_cli import Server,clean
from rust_agent_exchanges import logical

def seed_synthetic_history(fixture,rows):
    """Replace our own synthetic rows only; no agent/review/Work change.
    These rows deliberately establish no applied decision or revision chain.
    """
    with sqlite3.connect(fixture.root/'.swarm/state.db') as c:
        c.execute("DELETE FROM events WHERE work_id=? AND id LIKE 'synthetic-history-%'",(fixture.work,))
        for index,(kind,payload) in enumerate(rows):
            c.execute('INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)',('synthetic-history-'+str(index),fixture.work,100+index,kind,'fixture-time-'+str(index),sqlite3.Binary(payload),sqlite3.Binary(payload)))

def synthetic_cases():
    """Unverified recorded data only, deliberately not applied Work decisions."""
    return [
        ('planning.decide',b'{"scope":"root","reason":"fixture <img>","operations":null}'),
        ('planning.decide',b'{"Scope":"child","scope":null,"operations":[]}'),
        ('planning.decide',b'{"schema_version":0,"scope":"old","future_field":{"ignored":true},"operations":[{"kind":"revise-dependencies","id":"t","title":"","requirements":null,"depends":[]}]}'),
        ('planning.decide',b'{"reason":"\xff","operations":[{"kind":"historical-unknown","id":"","title":"literal <b>","requirements":[],"max_tasks":9223372036854775807}]}'),
        ('planning.decide',b'null'),
        ('planning.decide',b'{"scope":"\\ud800","reason":"\\udfff","operations":[{"kind":"revise-dependencies","id":"clear","title":"","requirements":[],"future_nested":{"ignored":true}}]}'),
        ('task.update',b'not-json-irrelevant-history'),
    ]

def run(binary,go,output,unported=False):
    """Compare complete actual Go values, retaining originals and cleanup causes.
    Seed only explicitly synthetic recorded rows on inactive test Roots. Public
    fixture creation precedes faults; business Work/tasks remain unchanged.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'history');gd=output/'go';gd.mkdir();gf=Fixture(binary,gd,'history-go');s=None;g=None;primary=None;calls=[];checks=[]
    def wire(address,cookie,path,status=200,method='GET',body=None,csrf=None):
        """Protected exact status/content; never retain session/CSRF tokens."""
        host,port=address.rsplit(':',1);c=http.client.HTTPConnection(host,int(port),timeout=None);headers={'Origin':'http://'+address}
        if cookie:headers['Cookie']=cookie
        if csrf:headers['X-Swarm-CSRF']=csrf
        try:c.request(method,path,body,headers);r=c.getresponse();raw=r.read();actual=r.status
        finally:c.close()
        try:value=json.loads(raw) if raw else None
        except (ValueError,UnicodeError):value=raw.decode(errors='replace')
        calls.append(dict(path=path,method=method,status=actual,value=clean(value)));publish(output/'http.json',calls);assert actual==status,(actual,value);return value
    seed=seed_synthetic_history
    cases=synthetic_cases()
    try:
        # Valid existing Work and empty retained history are controls before data.
        g=subprocess.Popen([str(go),'web','127.0.0.1:0','--root',str(gf.root)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=gf.environment);m=re.fullmatch(r'Cockpit local : http://(127[.]0[.]0[.]1:[0-9]+)/session/([A-Za-z0-9_-]+)',g.stdout.readline().decode().strip());assert m;address,token=m.groups();go_path='/api/v1/planning-history?'+urllib.parse.urlencode(dict(work=gf.work));assert wire(address,'swarm_session='+token,go_path)==[];seed(gf,cases);reference=wire(address,'swarm_session='+token,go_path);assert len(reference)==6 and reference[0]['operations'] is None and reference[1]['operations']==[] and reference[2]['operations'][0]['depends']==[] and 'depends' not in reference[5]['operations'][0] and reference[5]['scope']=='\ufffd';checks.append('actual-go-complete-history-nil-empty-unknown-utf-high-data')
        s=Server(binary,f.root,output,'http',environment=f.environment);s.capture_async(output/'stream.jsonl');path='/api/v1/planning-history?'+urllib.parse.urlencode(dict(work=f.work));before=logical(f.root)
        if unported:wire(s.address,s.cookie,path,404);assert logical(f.root)==before;checks.append('native211-route-absence')
        else:
            assert wire(s.address,s.cookie,path)==[];assert f.ok(['planning','history',f.work])==[];seed(f,cases);before=logical(f.root)
            value=wire(s.address,s.cookie,path);assert value==reference and f.ok(['planning','history',f.work])==reference;assert logical(f.root)==before;checks.append('actual-native-go-cli-equality-no-mutation')
            assert wire(s.address,s.cookie,path,method='HEAD') is None
            csrf=wire(s.address,s.cookie,'/api/v1/native-session')['csrf_token']
            wire(s.address,s.cookie,path,405,method='POST',body=b'',csrf=csrf);wire(s.address,s.cookie,path,400,body=b'{}');wire(s.address,None,path,403);wire(s.address,s.cookie,path+'&work='+f.work,400);wire(s.address,s.cookie,path+'&action=show',400);wire(s.address,s.cookie,'/api/v1/planning-history?work=missing',404);assert logical(f.root)==before;checks.append('head-method-query-body-auth-unknown-work-no-mutation')
            # Each malformed known field follows its own valid restored control.
            for invalid in [b'',b'{',b'{"max_tasks":"bad-known-unused-field"}',b'{"operations":[{"judge_cycle":1.5}]}']:
                assert wire(s.address,s.cookie,path)==reference
                seed(f,[('planning.decide',invalid)]);broken=logical(f.root)
                reply=wire(s.address,s.cookie,path,500);assert 'planning-history-payload-invalid' in json.dumps(reply);code,reply=f.call(['planning','history',f.work]);assert code!=0 and 'planning-history-payload-invalid' in json.dumps(reply);assert logical(f.root)==broken
                seed(f,cases);assert wire(s.address,s.cookie,path)==reference
            checks.append('malformed-empty-known-unused-fraction-refusal-and-recovery')
            assert not list(f.root.rglob('model-received.json'));checks.append('no-provider-or-authority-applied')
        assert not list(gf.root.rglob('model-received.json'))
    except BaseException as error:primary=error
    errors=[]
    if g is not None:
        try:g.terminate();g.communicate()
        except BaseException as error:errors.append(error)
    if s is not None:
        try:s.cleanup()
        except BaseException as error:errors.append(error)
    if primary:errors.insert(0,primary)
    if errors:raise BaseExceptionGroup('History and owned cleanup',errors)
    if s:assert not s.capture_errors
    publish(output/'result.json',dict(status='PASS',checks=checks,binary_sha256=digest(binary.read_bytes()),go_sha256=digest(go.read_bytes()),synthetic_recorded_rows_not_applied=True,provider_requested=False,live_root_mutation=False));print('PLANNING_HISTORY_UNPORTED_WITNESS' if unported else 'PLANNING_HISTORY_PUBLIC_VERIFIED 5',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--go-binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);p.add_argument('--expect-unported',action='store_true');a=p.parse_args()
    try:run(a.binary.resolve(),a.go_binary.resolve(),a.output.resolve(),a.expect_unported)
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
