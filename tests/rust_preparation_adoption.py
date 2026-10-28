#!/usr/bin/env python3
"""Adopt a brief through Rust CLI/protected API on an isolated owned Root.

Public writes cover hashes, CAS, receipts, history, refusal/recovery and output
failure. A historical verdict is explicitly seeded only in this fixture to
prove its removal, not to claim plan validation. No provider/live DB/watchdog.
"""
import argparse
import json
import subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Retain every original channel and named assertion; stop owned HTTP server."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
    d=Driver(Path(a.binary).resolve(),out);checks=[]

    def check(name,value):
        """Persist a unique assertion only after observing its actual condition."""
        assert value,name;assert name not in checks,name;checks.append(name)
        publish(out/'checks.json',checks);print('PASS '+name,flush=True)

    def save(p,event,kind,text):
        """Use only observed revisions for an explicit public document save."""
        return d.ok(['prepare','save',pid],dict(version=1,action='save',preparation_id=pid,event_id=event,expected_revision=p['revision'],document=kind,text=text))

    def intent(p,event):
        """Describe adoption of the exact observed brief, without authorizing a plan."""
        return dict(version=1,action='adopt-brief',preparation_id=pid,event_id=event,expected_revision=p['revision'],sha256=p.get('documents',{}).get('brief',{}).get('sha256',''))

    def refused(value,name,family='preparation-request-invalid',code=2):
        """Require exact domain failure, empty stdout and complete SQL stability."""
        before=d.database();exit_code,stdout,stderr=d.call(['prepare','adopt-brief',pid],value)
        check(name,exit_code==code and not stdout and json.loads(stderr)['failure']['code']==family and d.database()==before)

    d.ok(['init'])
    p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='adoption-create',expected_revision=0,text='Initial need'))
    pid=p['id'];refused(intent(p,'missing-brief'),'adopt-missing-brief','preparation-stale-document')
    for event,text in [('empty',''),('unicode-space','\u0085\u00a0\u3000')]:
        p=save(p,'save-'+event,'brief',text)
        refused(intent(p,'adopt-'+event),'adopt-'+event+'-refused','preparation-stale-document')
    p=save(p,'save-valid','brief','Reviewed literal <b>brief</b> é')
    p=save(p,'save-plan-before-adoption','plan','Plan remains a draft')
    original_documents=p['documents'];histories={k:d.ok(['prepare','history',pid,k]) for k in ['besoin','brief','plan']}
    # Deliberate historical fixture, not a public plan acceptance operation.
    seeded={**p,'verdict':dict(plan_hash=p['documents']['plan']['sha256'],brief_hash=p['documents']['brief']['sha256'],method_hash=p['method_hash'],at=p['updated_at'])}
    d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(seeded).encode(),pid))
    wanted=intent(p,'adopt-valid')
    refused({**wanted,'sha256':'wrong'},'adopt-wrong-hash','preparation-stale-document')
    refused({**wanted,'sha256':''},'adopt-empty-hash','preparation-stale-document')
    refused({**wanted,'expected_revision':p['revision']-1},'adopt-stale-cas','preparation-revision-conflict',3)
    for name,extra in [('text',{'text':'must not be saved'}),('method',{'method':'debug'}),('document',{'document':'brief'}),('title',{'title':'unexpected'}),('work',{'work_id':'other'}),('adopt-flag',{'adopt_brief':True}),('source',{'source':{}}),('organization',{'organization':{}}),('budget',{'budget':{}}),('work-revision',{'expected_work_revision':0}),('decisions',{'decisions':[]}),('turn',{'turn_id':'other'}),('version',{'version':2}),('event',{'event_id':'invalid/event'}),('null-revision',{'expected_revision':None})]:
        refused({**wanted,'event_id':'invalid-'+name,**extra},'adopt-foreign-'+name)
    before=d.database();code,stdout,stderr=d.call(['prepare','adopt-brief','wrong-id'],wanted)
    check('adopt-positional-binding',code==2 and not stdout and d.database()==before)
    adopted=d.ok(['prepare','adopt-brief',pid],wanted)
    check('adopt-exact-current-brief',adopted['adopted_brief']==original_documents['brief'] and adopted['revision']==p['revision']+1)
    check('adopt-clears-verdict-not-plan-approval',not adopted.get('verdict') and not adopted['plan_ready'])
    check('adopt-preserves-documents-and-history',adopted['documents']==original_documents and all(d.ok(['prepare','history',pid,k])==v for k,v in histories.items()))
    check('adopt-does-not-freshen-saved-plan',not adopted['documents']['plan'].get('brief_hash'))
    before=d.database();check('adopt-replay-no-write',d.ok(['prepare','adopt-brief',pid],wanted)==adopted and d.database()==before)
    refused({**wanted,'sha256':'changed'},'adopt-event-conflict','preparation-event-conflict',3)
    p=save(adopted,'need-changed','besoin','Changed need')
    stale=intent(p,'recover-stale-brief');refused(stale,'adopt-need-changed','preparation-stale-brief')
    before=d.database();old=d.ok(['prepare','adopt-brief',pid],wanted)
    check('adopt-historical-replay-before-freshness',old['revision']==adopted['revision'] and old['receipt_historical'] and not old['plan_ready'] and d.database()==before)
    p=save(p,'brief-rebased','brief','Revised brief')
    recovered=d.ok(['prepare','adopt-brief',pid],intent(p,'recover-stale-brief'))
    check('adopt-refused-event-can-recover',recovered['adopted_brief']==p['documents']['brief'] and recovered['revision']==p['revision']+1)
    # No document text is submitted by adoption: the existing raw-input cap0
    # must not prohibit adopting a previously saved nonempty brief.
    d.settings(preparation_document_max_bytes='0')
    recovered=d.ok(['prepare','adopt-brief',pid],intent(recovered,'zero-cap-valid-control'))
    check('adopt-zero-cap-valid-control',recovered['adopted_brief']['text']=='Revised brief')
    output=intent(recovered,'adopt-output');path=out/'output-intent.json';publish(path,output)
    with open('/dev/full','wb',buffering=0) as full:
        result=subprocess.run([str(d.binary),'prepare','adopt-brief',pid,'--root',str(d.root),'--json','--input',str(path)],stdout=full,stderr=subprocess.PIPE)
    publish(out/'output.stderr',result.stderr);error=json.loads(result.stderr)
    check('adopt-output-failure-retains-receipt',result.returncode==2 and error['recorded_preparation']==dict(id=pid,revision=recovered['revision']+1))
    p=d.ok(['prepare','adopt-brief',pid],output)
    check('adopt-output-replay-and-zero-text-cap',p['revision']==recovered['revision']+1 and p['adopted_brief']==recovered['adopted_brief'])
    d.settings(preparation_write_enabled='0');refused(output,'adopt-disabled-replay','preparation-write-disabled',4)
    d.settings(preparation_write_enabled='1',preparation_document_max_bytes=None)
    server=native.Server(d.binary,d.root,out,'adoption');server.capture_async(out/'http-observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        check('adopt-http-capability','adopt-brief' in server.request('/api/v1/native-capabilities',observe=False)['preparations'])
        api=intent(p,'adopt-api');before=d.database()
        server.request('/api/v1/preparations/command',api,authorized=False,status=403,observe=False)
        server.request('/api/v1/preparations/command',api,status=403,observe=False)
        check('adopt-http-auth-csrf-no-write',d.database()==before)
        invalid=server.request('/api/v1/preparations/command',{**api,'preparation_id':'invalid/id'},csrf=csrf,status=400,observe=False)
        check('adopt-http-invalid-id',invalid['failure']['code']=='preparation-request-invalid' and d.database()==before)
        bad={**api,'sha256':'wrong'}
        failure=server.request('/api/v1/preparations/command',bad,csrf=csrf,status=409,observe=False)
        check('adopt-http-stale-document',failure['failure']['code']=='preparation-stale-document' and d.database()==before)
        response=server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False)
        check('adopt-http-cli-agree',response==d.ok(['prepare','show',pid]) and response['revision']==p['revision']+1 and response['adopted_brief']==p['documents']['brief'])
        before=d.database();check('adopt-http-replay-no-write',server.request('/api/v1/preparations/command',api,csrf=csrf,observe=False)==response and d.database()==before)
        server.request('/api/v1/preparations/command',{**api,'sha256':'conflict'},csrf=csrf,status=409,observe=False)
        check('adopt-http-event-conflict-no-write',d.database()==before)
        p=save(response,'http-need-changed','besoin','Third need')
        failure=server.request('/api/v1/preparations/command',intent(p,'http-stale-need'),csrf=csrf,status=409,observe=False)
        check('adopt-http-stale-need',failure['failure']['code']=='preparation-stale-brief' and d.ok(['prepare','show',pid])['revision']==p['revision'])
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    # Go permits a brief without a need document: both absent need hashes are
    # empty. This is parity, not evidence of a complete specification.
    empty=d.ok(['prepare','create'],dict(version=1,action='create',event_id='no-need-create',expected_revision=0))
    no_need=d.ok(['prepare','save',empty['id']],dict(version=1,action='save',event_id='no-need-brief',preparation_id=empty['id'],expected_revision=1,document='brief',text='Brief without need'))
    accepted=d.ok(['prepare','adopt-brief',empty['id']],dict(version=1,action='adopt-brief',event_id='no-need-adopt',preparation_id=empty['id'],expected_revision=2,sha256=no_need['documents']['brief']['sha256']))
    check('adopt-absent-need-go-parity',accepted['adopted_brief']==no_need['documents']['brief'] and accepted['revision']==3 and not accepted['plan_ready'])
    publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0))
    print('PREPARATION_ADOPTION_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
