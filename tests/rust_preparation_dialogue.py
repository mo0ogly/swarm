#!/usr/bin/env python3
"""Read retained preparation dialogue on an owned inactive Root, never run/stop AI.

Fault injection is synthetic SQL after a valid control; public reads must not
write or publish prompts. Original argv/channels retained, no global watchdog.
"""
import argparse,copy,json,sqlite3
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native


def turn_fixture(p,method):
    """Complete inert historical turn; values are test inputs, never live defaults."""
    return dict(id='turn-first',preparation_id=p['id'],status='answered',created_at='2026-01-01T00:00:00Z',ended_at='2026-01-01T00:00:01Z',source_revision=p['revision'],method_hash=method['sha256'],provider='fixture',provider_digest='observed-digest',request_hash='observed-request',question='<img src=x onerror="dialogueExecuted=true">',prompt='PRIVATE_PROMPT_SENTINEL',answer=dict(message='Message <script>inert</script>',brief='BRIEF_PROPOSAL_SENTINEL',plan=''),used_in_brief=True,used_in_plan=True,stale=True,timeout_seconds=17,timeout_maximum_seconds=23,model_route=dict(model='fixture-model',level='standard'),usage=dict(input_tokens=3,output_tokens=4,source='fixture',scope='turn',at='2026-01-01T00:00:01Z'))


def main():
    """Assert public chronology, projection, freshness, safe refusal and recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-unported',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'preparation-dialogue');f.settings_change(preparation_write_enabled='1');p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='dialogue-create',expected_revision=0,title='Historical conversation',text='Need'));checks=[]
    def check(name,value):
        """Retain each named assertion immediately; no empty-success fallback."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def sql():
        """Read the owned SQL dump, without mutating a running mission datastore."""
        with sqlite3.connect('file:'+str(f.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:return '\n'.join(db.iterdump())
    before=sql();rc,value=f.call(['prepare','dialogue',p['id']])
    if a.expect_unported:check('dialogue-baseline-cli-refuses',rc!=0 and before==sql());print('DIALOGUE_BASELINE_PASS 1');return
    check('dialogue-empty-explicit-array',rc==0 and value==[] and before==sql());method=next(m for m in f.ok(['prepare','methods']) if m['id']==p['method']);base=turn_fixture(p,method);second={**base,'id':'turn-second','source_revision':p['revision']-1,'status':'failed','error':'Recorded provider refusal','answer':None}
    def put(turn,status=None,raw=None):
        """Only this synthetic retained-turn fixture is written outside public writers."""
        with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute('INSERT INTO preparation_turns(id,preparation_id,status,body) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET preparation_id=excluded.preparation_id,status=excluded.status,body=excluded.body',(turn['id'],p['id'],status or turn['status'],json.dumps(turn) if raw is None else raw))
    put(base);put(second);before=sql();turns=f.ok(['prepare','dialogue',p['id']]);check('dialogue-row-order',[t['id'] for t in turns]==['turn-first','turn-second']);check('dialogue-prompt-never-published','PRIVATE_PROMPT_SENTINEL' not in json.dumps(turns) and all('prompt' not in t for t in turns));check('dialogue-current-flags-recomputed',not turns[0]['stale'] and not turns[0]['used_in_brief'] and not turns[0]['used_in_plan'] and turns[1]['stale']);check('dialogue-answer-and-usage-retained',turns[0]['answer']['brief']==base['answer']['brief'] and turns[0]['usage']['input_tokens']==3 and turns[0]['timeout_seconds']==17 and turns[0]['timeout_maximum_seconds']==23);check('dialogue-read-no-write',before==sql())
    server=native.Server(f.binary,f.root,out,'dialogue',environment=f.environment);server.capture_async(out/'observer.raw');primary=None
    try:
        path='/api/v1/preparations/dialogue?id='+p['id'];check('dialogue-http-cli-equal',server.request(path,observe=False)==turns);server.request(path+'&id='+p['id'],status=400,observe=False);check('dialogue-invalid-query-no-write',before==sql())
        for status in ['pending','running','stopping']:
            f.ok(['prepare','dialogue',p['id']]);put(base,status=status);fault=sql();rc,error=f.call(['prepare','dialogue',p['id']]);check('dialogue-active-'+status+'-refuses',rc!=0 and error['failure']['code']=='preparation-dialogue-active-unported' and fault==sql());server.request(path,status=501,observe=False);put(base);check('dialogue-active-'+status+'-recovery',f.ok(['prepare','dialogue',p['id']])==turns)
        for label,raw in [('malformed',b'{'),('wrong-key',json.dumps({**base,'id':'turn-other'})),('wrong-parent',json.dumps({**base,'preparation_id':'prep-other'})),('wrong-prompt-type',json.dumps({**base,'prompt':{}}))]:
            f.ok(['prepare','dialogue',p['id']]);put(base,raw=raw);fault=sql();rc,error=f.call(['prepare','dialogue',p['id']]);check('dialogue-'+label+'-refuses',rc!=0 and fault==sql());response=server.request(path,status=500,observe=False);check('dialogue-'+label+'-http-safe', 'PRIVATE_PROMPT_SENTINEL' not in json.dumps(response) and 'causes' not in response['failure']);put(base);check('dialogue-'+label+'-recovery',f.ok(['prepare','dialogue',p['id']])==turns)
        # Same repeated field/alias/null semantics as Go, before serde decoding.
        raw=json.dumps(base)[:-1]+',"ANSWER":{"message":"Alias final"},"answer":null,"answer":{"brief":"Merged final"}}';put(base,raw=raw);v=f.ok(['prepare','dialogue',p['id']])[0];check('dialogue-nested-alias-null-merge',v['answer']==dict(message='',brief='Merged final'));put(base);check('dialogue-wire-recovery',f.ok(['prepare','dialogue',p['id']])==turns)
        f.ok(['prepare','dialogue',p['id']])
        with sqlite3.connect(f.root/'.swarm/state.db') as db:original=db.execute('SELECT body FROM preparations WHERE id=?',(p['id'],)).fetchone()[0]
        used=json.loads(original);used['documents']['brief']={'source_turn':base['id']};used['documents']['plan']={'source_turn':second['id']}
        with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute('UPDATE preparations SET body=? WHERE id=?',(json.dumps(used),p['id']))
        v=f.ok(['prepare','dialogue',p['id']]);check('dialogue-used-flags-current-documents',v[0]['used_in_brief'] and not v[0]['used_in_plan'] and v[1]['used_in_plan'] and not v[1]['used_in_brief'])
        with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute('UPDATE preparations SET body=? WHERE id=?',(original,p['id']))
        check('dialogue-used-flags-recovery',f.ok(['prepare','dialogue',p['id']])==turns)
        for i in range(21):put({**base,'id':'turn-extra-'+str(i)})
        before=sql();many=f.ok(['prepare','dialogue',p['id']]);check('dialogue-retained-history-not-capped-at20',len(many)==23 and before==sql())
        with sqlite3.connect(f.root/'.swarm/state.db') as db:db.execute('DELETE FROM preparation_turns WHERE id LIKE ?',('turn-extra-%',))
        check('dialogue-retained-history-recovery',f.ok(['prepare','dialogue',p['id']])==turns)
        updated=f.ok(['prepare','save',p['id']],dict(version=1,action='save',preparation_id=p['id'],event_id='dialogue-brief',expected_revision=p['revision'],document='brief',text='Current brief'));after=f.ok(['prepare','dialogue',p['id']]);check('dialogue-revision-change-stale',all(t['stale'] for t in after));before=sql();check('dialogue-current-sql-no-side-effect',f.ok(['prepare','dialogue',p['id']])==after and before==sql());check('dialogue-no-provider',not (f.root/'model-received.json').exists())
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    fixture.publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0));print('DIALOGUE_PASS '+str(len(checks)))
if __name__=='__main__':main()
