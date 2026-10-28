#!/usr/bin/env python3
"""Preparation method selection through CLI and protected HTTP on an owned Root.

Historical approval and unsafe method sources are explicit isolated fixtures.
Valid controls precede faults; exact SQL stability, recovery and replay are
asserted. No live datastore, provider, global watchdog or implicit retry.
"""
import argparse
import json
import os
from pathlib import Path
from rust_preparation_reads import Driver
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Persist channels and named assertions; stop the owned server on failure."""
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',required=True)
    parser.add_argument('--output',required=True)
    a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
    d=Driver(Path(a.binary).resolve(),out);checks=[]

    def check(name,value):
        """Publish each unique assertion only after its actual observation."""
        assert value,name
        assert name not in checks,name
        checks.append(name);publish(out/'checks.json',checks)
        print('PASS '+name,flush=True)

    def refused(value,name,code=2):
        """Assert exact refusal class, no successful output and no SQL mutation."""
        before=d.database();exit_code,stdout,stderr=d.call(['prepare','method',pid],value)
        check(name,exit_code==code and not stdout and json.loads(stderr)['failure']['code'] and d.database()==before)
        return json.loads(stderr)

    d.ok(['init'])
    p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='method-create',expected_revision=0,text='original need'))
    pid=p['id']
    for kind in ['brief','plan']:
        p=d.ok(['prepare','save',pid],dict(version=1,action='save',preparation_id=pid,event_id='method-save-'+kind,expected_revision=p['revision'],document=kind,text='retained '+kind))
    # Historical approval is a fixture, not an adoption/validation implementation.
    fixture=dict(p);fixture['adopted_brief']=p['documents']['brief']
    fixture['verdict']=dict(plan_hash=p['documents']['plan']['sha256'],brief_hash=p['documents']['brief']['sha256'],method_hash=p['method_hash'],at=p['updated_at'])
    d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(fixture).encode(),pid))
    histories={k:d.ok(['prepare','history',pid,k]) for k in ['besoin','brief','plan']}
    request=dict(version=1,action='method',event_id='select-debug',preparation_id=pid,expected_revision=3,method='debug')
    selected=d.ok(['prepare','method',pid],request)
    catalog=d.ok(['prepare','methods'])
    check('method-current-hash-and-revision',selected['method']=='debug' and selected['method_hash']==next(m['sha256'] for m in catalog if m['id']=='debug') and selected['revision']==4)
    check('method-preserves-documents-adopted-brief',selected['documents']==p['documents'] and selected['adopted_brief']==fixture['adopted_brief'])
    check('method-clears-approval',not selected.get('verdict') and selected['plan_ready'] is False)
    check('method-history-unchanged',all(d.ok(['prepare','history',pid,k])==v for k,v in histories.items()))
    before=d.database();check('method-replay-exact-no-write',d.ok(['prepare','method',pid],request)==selected and d.database()==before)
    refused({**request,'method':'apex'},'method-event-conflict',3)
    refused({**request,'event_id':'stale'},'method-stale-cas',3)
    for name,extra,code in [('missing',{'method':''},5),('unknown',{'method':'../unknown'},5),('document',{'document':'plan'},2),('text',{'text':'unexpected'},2),('title',{'title':'unexpected'},2),('hash',{'sha256':'unexpected'},2),('decisions-empty',{'decisions':[]},2),('null-revision',{'expected_revision':None},2)]:
        refused({**request,'event_id':'bad-'+name,'expected_revision':4,**extra},'method-invalid-'+name,code)
    before=d.database();code,stdout,stderr=d.call(['prepare','method','wrong-id'],request)
    check('method-positional-binding',code==2 and not stdout and d.database()==before)
    alias={**request,'event_id':'select-alias','expected_revision':4,'method':'audit_pdca'}
    aliased=d.ok(['prepare','method',pid],alias)
    check('method-alias-canonical',aliased['method']=='audit-pdca' and aliased['revision']==5)
    # Valid available-source control before a local unsafe override.
    debug=d.root/'.claude/skills/debug/SKILL.md';debug.parent.mkdir(parents=True)
    original=Path('.claude/skills/debug/SKILL.md').read_bytes();publish(debug,original)
    control={**request,'event_id':'valid-local','expected_revision':5}
    p=d.ok(['prepare','method',pid],control);check('method-valid-local-control',p['revision']==6 and p['method']=='debug')
    debug.unlink();os.symlink('missing-private-fixture',debug)
    before=d.database();check('method-replay-before-source-validation',d.ok(['prepare','method',pid],control)==p and d.database()==before)
    unavailable={**request,'event_id':'unsafe-source','expected_revision':6}
    error=refused(unavailable,'method-unsafe-source-refused',5)
    check('method-source-cause-preserved',error['failure']['code']=='preparation-method-unavailable' and 'preparation-method-open' in error['failure']['causes'] and 'missing-private-fixture' not in json.dumps(error))
    debug.unlink();publish(debug,original)
    recovered=d.ok(['prepare','method',pid],unavailable)
    check('method-same-event-recovery',recovered['revision']==7 and recovered['method_hash']==p['method_hash'])
    d.settings(preparation_write_enabled='0');refused(unavailable,'method-disabled-denies-replay',4)
    d.settings(preparation_write_enabled='1')
    server=native.Server(d.binary,d.root,out,'method');server.capture_async(out/'http-observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        capabilities=server.request('/api/v1/native-capabilities',observe=False)
        check('method-api-capability','method' in capabilities['preparations'])
        api={**request,'event_id':'http-method','expected_revision':7,'method':'apex'}
        before=d.database()
        server.request('/api/v1/preparations/command',api,authorized=False,status=403,observe=False)
        server.request('/api/v1/preparations/command',api,status=403,observe=False)
        check('method-http-protected',d.database()==before)
        response=server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False)
        check('method-http-cli-shared-result',response['revision']==8 and response['method']=='apex' and response==d.ok(['prepare','show',pid]))
        debug.unlink();os.symlink('missing-private-fixture',debug)
        before=d.database()
        failure=server.request('/api/v1/preparations/command',{**request,'event_id':'http-source','expected_revision':8},csrf=csrf,status=400,observe=False)
        check('method-http-source-refusal',failure['failure']['code']=='preparation-method-unavailable' and d.database()==before)
        debug.unlink();publish(debug,original)
        before=d.database();check('method-http-replay',server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False)==response and d.database()==before)
        server.request('/api/v1/preparations/command',{**api,'event_id':'http-stale'},csrf=csrf,status=409,observe=False)
        server.request('/api/v1/preparations/command',{**api,'event_id':'http-invalid','expected_revision':8,'method':''},csrf=csrf,status=400,observe=False)
        check('method-http-refusals-no-write',d.database()==before)
        server.request('/api/v1/preparations/command',{**api,'method':'debug'},csrf=csrf,status=409,observe=False)
        server.request('/api/v1/preparations/command',{**api,'preparation_id':'not-recorded','event_id':'missing-preparation','expected_revision':0},csrf=csrf,status=404,observe=False)
        check('method-http-event-conflict-and-missing',d.database()==before)
        d.settings(preparation_write_enabled='0');before=d.database()
        server.request('/api/v1/preparations/command',api,csrf=csrf,status=403,observe=False)
        check('method-http-disabled-replay',d.database()==before)
        d.settings(preparation_write_enabled='1')
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    old=d.ok(['prepare','method',pid],request)
    check('method-historical-receipt',old['revision']==4 and old['receipt_historical'] is True and old['plan_ready'] is False)
    # Different local bytes must affect the selected hash, not merely be readable.
    publish(debug,original+b'\nLocal fixture change.\n')
    changed=d.ok(['prepare','method',pid],{**request,'event_id':'changed-local','expected_revision':8})
    current=next(m for m in d.ok(['prepare','methods']) if m['id']=='debug')
    check('method-prefers-actual-local-bytes',changed['revision']==9 and changed['method_hash']==current['sha256'] and changed['method_hash']!=p['method_hash'])
    same=d.ok(['prepare','method',pid],{**request,'event_id':'same-method','expected_revision':9})
    check('method-reselection-increments-without-doc-change',same['revision']==10 and same['method_hash']==changed['method_hash'] and same['documents']==changed['documents'] and not same.get('verdict') and not same['plan_ready'])
    limit=d.ok(['engine-settings','show'])['values']['preparation_method_max_bytes']
    d.settings(preparation_method_max_bytes='0')
    capped={**request,'event_id':'source-cap','expected_revision':10}
    refused(capped,'method-source-cap-refuses-without-fallback',5)
    d.settings(preparation_method_max_bytes=limit)
    check('method-source-cap-same-event-recovery',d.ok(['prepare','method',pid],capped)['revision']==11)
    check('method-history-after-reopen',all(d.ok(['prepare','history',pid,k])==v for k,v in histories.items()))
    publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0))
    print('PREPARATION_METHOD_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
