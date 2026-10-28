#!/usr/bin/env python3
"""Stored-digest corruption on owned fixtures: refuse verdicts, remain repairable.

Valid public controls precede each private SQL fault. Legacy mode reproduces an
incorrect ready verdict on the retained pre-fix binary. Normal mode verifies
readiness, CLI/API refusal and public save/adopt recovery, without live DB use.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Retain exact faulty metadata/channels and all unique named assertions."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--legacy',action='store_true');a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);checks=[]

    def check(name,value):
        """Persist a named condition only after observation."""
        assert value,name;assert name not in checks;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)

    for target in ['plan','brief','besoin','adopted']:
        directory=out/target;directory.mkdir();d=Driver(Path(a.binary).resolve(),directory);d.ok(['init'])
        p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='create',expected_revision=0,text='original need'));pid=p['id']

        def save(state,event,kind,text):
            """Public replacement operation, with a revision observed in this Root."""
            return d.ok(['prepare','save',pid],dict(version=1,action='save',event_id=event,preparation_id=pid,expected_revision=state['revision'],document=kind,text=text))

        def adopt(state,event):
            """Publicly adopt current coherent bytes to replace an old copied brief."""
            return d.ok(['prepare','adopt-brief',pid],dict(version=1,action='adopt-brief',event_id=event,preparation_id=pid,expected_revision=state['revision'],sha256=state['documents']['brief']['sha256']))

        def intent(state,event):
            """Explicit validation event based on stored plan metadata."""
            return dict(version=1,action='validate-plan',event_id=event,preparation_id=pid,expected_revision=state['revision'],sha256=state['documents']['plan']['sha256'])

        p=save(p,'brief-save','brief','original brief');p=adopt(p,'adopt');p=save(p,'plan-save','plan',json.dumps(plan()));p=d.ok(['prepare','validate-plan',pid],intent(p,'control-verdict'))
        check(target+'-valid-control',p['plan_ready'])
        original=copy.deepcopy(p);damaged=copy.deepcopy(p)
        if target=='plan':
            value=json.loads(damaged['documents']['plan']['text']);value['objective']='PRIVATE_PLAN_CORRUPTION';damaged['documents']['plan']['text']=json.dumps(value)
        elif target=='adopted':damaged['adopted_brief']['text']='PRIVATE_ADOPTED_CORRUPTION'
        else:damaged['documents'][target]['text']='PRIVATE_DOCUMENT_CORRUPTION'
        publish(directory/'fault.json',damaged)
        d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(damaged).encode(),pid));before=d.database();request=intent(damaged,'fault-verdict')
        if a.legacy:
            wrong=d.ok(['prepare','validate-plan',pid],request)
            witness=wrong['adopted_brief'] if target=='adopted' else wrong['documents'][target]
            check('legacy-'+target+'-ready-with-wrong-digest',wrong['plan_ready'] and witness['sha256']!=hashlib.sha256(witness['text'].encode()).hexdigest())
            if target in ['brief','besoin']:
                old_adopted=adopt(wrong,'legacy-adopt')
                source=old_adopted['adopted_brief'] if target=='brief' else old_adopted['documents']['besoin']
                check('legacy-'+target+'-adoption-consumes-corruption',source['sha256']!=hashlib.sha256(source['text'].encode()).hexdigest())
            if target in ['besoin','adopted']:
                kind='brief' if target=='besoin' else 'plan'
                stamped=save(old_adopted if target=='besoin' else wrong,'legacy-save',kind,'legacy saved text')
                source=stamped['documents']['besoin'] if target=='besoin' else stamped['adopted_brief']
                key='need_hash' if target=='besoin' else 'brief_hash'
                check('legacy-'+target+'-save-stamps-wrong-source',stamped['documents'][kind][key]!=hashlib.sha256(source['text'].encode()).hexdigest())
            continue
        shown=d.ok(['prepare','show',pid]);check(target+'-corruption-never-ready',not shown['plan_ready'] and d.database()==before)
        code,stdout,stderr=d.call(['prepare','validate-plan',pid],request)
        check(target+'-verdict-refused-no-mutation',code==2 and not stdout and json.loads(stderr)['failure']['code']=='preparation-document-digest-mismatch' and d.database()==before)
        check(target+'-safe-diagnostic','PRIVATE_' not in stderr.decode())
        if target in ['brief','besoin']:
            body=dict(version=1,action='adopt-brief',event_id='bad-adopt',preparation_id=pid,expected_revision=p['revision'],sha256=p['documents']['brief']['sha256'])
            code,stdout,stderr=d.call(['prepare','adopt-brief',pid],body)
            check(target+'-adoption-consumer-refused',code==2 and not stdout and json.loads(stderr)['failure']['code']=='preparation-document-digest-mismatch' and d.database()==before)
        if target in ['besoin','adopted']:
            kind='brief' if target=='besoin' else 'plan'
            body=dict(version=1,action='save',event_id='bad-provenance-save',preparation_id=pid,expected_revision=p['revision'],document=kind,text='must not stamp corrupt provenance')
            code,stdout,stderr=d.call(['prepare','save',pid],body)
            check(target+'-save-provenance-refused',code==2 and not stdout and json.loads(stderr)['failure']['code']=='preparation-document-digest-mismatch' and d.database()==before)
        if target=='plan':
            server=native.Server(d.binary,d.root,directory,'integrity');server.capture_async(directory/'observer.raw');primary=None
            try:
                csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
                error=server.request('/api/v1/preparations/command',request,csrf=csrf,status=500,observe=False)
                check('plan-corruption-http500-safe',error['failure']['code']=='preparation-document-digest-mismatch' and 'PRIVATE_' not in json.dumps(error) and d.database()==before)
            except BaseException as error:primary=error
            finally:native.finish(server,primary)
        # Recovery uses supported commands only. A damaged target is replaceable;
        # no original history/receipt is erased or rewritten to manufacture PASS.
        if target=='adopted':repaired=adopt(shown,'repair-adopted')
        else:repaired=save(shown,'repair-document',target,original['documents'][target]['text'])
        recovered=d.ok(['prepare','validate-plan',pid],intent(repaired,'fault-verdict'))
        check(target+'-public-repair-same-event',recovered['plan_ready'] and recovered['revision']==original['revision']+2)
        before=d.database();check(target+'-recovered-replay',d.ok(['prepare','validate-plan',pid],intent(repaired,'fault-verdict'))==recovered and d.database()==before)
    publish(out/'result.json',dict(status='PASS',legacy=a.legacy,checks=checks));print('PREPARATION_PLAN_INTEGRITY_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
