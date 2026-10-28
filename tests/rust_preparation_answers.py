#!/usr/bin/env python3
"""Answer a preparation plan through CLI and protected HTTP on an isolated Root.

Bind decisions to the exact revision, hash and ordered questions. Assert history,
source provenance, replay, invalidation, saved caps, failures and public recovery.
No provider, live database or global watchdog is used.
"""
import argparse
import copy
import hashlib
import json
import subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Retain named observable assertions and stop only this fixture's server."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);a=parser.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);checks=[]
    def check(name,value):
        """Keep successful assertions separately from process exit status."""
        assert value,name;assert name not in checks,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def save(p,label,kind,text):
        """Write through public CAS instead of editing persisted fixture rows."""
        return d.ok(['prepare','save',p['id']],dict(version=1,action='save',preparation_id=p['id'],event_id=label,expected_revision=p['revision'],document=kind,text=text))
    def intent(p,label,answers):
        """Bind all supplied answers to the stored ordered questions and hash."""
        return dict(version=1,action='answer-questions',preparation_id=p['id'],event_id=label,expected_revision=p['revision'],sha256=p['documents']['plan']['sha256'],decisions=answers)
    def refused(value,label,code='preparation-request-invalid',status=2):
        """Every refusal must retain exact preparation tables and a safe cause."""
        before=d.database();exit_code,_,err=d.call(['prepare','answer-questions',value['preparation_id']],value)
        check(label,exit_code==status and json.loads(err)['failure']['code']==code and d.database()==before)
    d.ok(['init']);d.settings(preparation_write_enabled='1')
    p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='answers-create',expected_revision=0,text='Need'))
    p=save(p,'answers-brief','brief','Brief');p=d.ok(['prepare','adopt-brief',p['id']],dict(version=1,action='adopt-brief',preparation_id=p['id'],event_id='answers-adopt',expected_revision=p['revision'],sha256=p['documents']['brief']['sha256']))
    value=plan();value['questions']=[dict(question='First <choice>?',answer=''),dict(question='Second?',answer='')]
    p=save(p,'answers-plan','plan',json.dumps(value));original=p['documents']['plan'];history=d.ok(['prepare','history',p['id'],'plan'])
    answers=[dict(question='First <choice>?',answer='Yes <&>\u2028é'),dict(question='Second?',answer='')]
    request=intent(p,'answers-partial',answers);before=d.database();good=d.ok(['prepare','answer-questions',p['id']],request)
    actual=json.loads(good['documents']['plan']['text']);check('answers-only-answers-changed',actual=={**value,'questions':answers})
    doc=good['documents']['plan'];check('answers-document-hash-revision',doc['sha256']==hashlib.sha256(doc['text'].encode()).hexdigest() and doc['revision']==original['revision']+1 and good['revision']==p['revision']+1)
    check('answers-preserve-provenance',all(doc.get(k)==original.get(k) for k in ('need_hash','brief_hash','method_hash','source_turn')) and good['adopted_brief']==p['adopted_brief'])
    check('answers-no-verdict',not good.get('verdict') and not good['plan_ready'])
    check('answers-go-json-escaping',r'\u003c' in doc['text'] and r'\u0026' in doc['text'] and r'\u2028' in doc['text'] and '\n  \"version\": 1,' in doc['text'] and not doc['text'].endswith('\n'))
    check('answers-history-appended',len(d.ok(['prepare','history',p['id'],'plan']))==len(history)+1 and d.database()!=before)
    before=d.database();check('answers-exact-replay',d.ok(['prepare','answer-questions',p['id']],request)==good and d.database()==before)
    refused({**request,'decisions':[]},'answers-event-conflict','preparation-event-conflict',3)
    p=good
    refused({**intent(p,'answers-cas',answers),'expected_revision':p['revision']-1},'answers-stale-cas','preparation-revision-conflict',3)
    refused({**intent(p,'answers-hash',answers),'sha256':'wrong'},'answers-stale-hash','preparation-stale-document')
    for label,decisions in [('count',answers[:1]),('order',list(reversed(answers))),('question',[{**answers[0],'question':'Changed'},answers[1]]),('foreign',[{**answers[0],'foreign':True},answers[1]]),('type',[{**answers[0],'answer':3},answers[1]]),('null',[None,answers[1]])]:
        refused(intent(p,'answers-invalid-'+label,decisions),'answers-refuse-'+label,'preparation-stale-document' if label in ('order','question') else 'preparation-request-invalid')
    complete=copy.deepcopy(answers);complete[1]['answer']='Approved';p=d.ok(['prepare','answer-questions',p['id']],intent(p,'answers-complete',complete))
    p=d.ok(['prepare','validate-plan',p['id']],dict(version=1,action='validate-plan',preparation_id=p['id'],event_id='answers-verdict',expected_revision=p['revision'],sha256=p['documents']['plan']['sha256']))
    check('answers-validation-after-completion',p['plan_ready'])
    p=d.ok(['prepare','answer-questions',p['id']],intent(p,'answers-clear',answers));check('answers-clear-invalidates-verdict',not p['plan_ready'] and not p.get('verdict'))
    d.settings(preparation_document_max_bytes='1');refused(intent(p,'answers-cap',complete),'answers-document-cap');d.settings(preparation_document_max_bytes=None)
    d.settings(preparation_plan_max_questions='1');refused(intent(p,'answers-question-cap',complete),'answers-question-policy','preparation-invalid-plan');d.settings(preparation_plan_max_questions=None)
    p=save(p,'answers-obsolete-need','besoin','Changed need');source=p['documents']['plan'];p=d.ok(['prepare','answer-questions',p['id']],intent(p,'answers-obsolete-plan',complete))
    check('answers-never-freshen-source',p['documents']['plan']['brief_hash']==source['brief_hash'] and p['documents']['plan']['method_hash']==source['method_hash'] and not p['plan_ready'])
    p=d.ok(['prepare','method',p['id']],dict(version=1,action='method',preparation_id=p['id'],event_id='answers-product-method',expected_revision=p['revision'],method='ks-product'))
    p=d.ok(['prepare','answer-questions',p['id']],intent(p,'answers-no-product-draft',answers));check('answers-draft-not-product-approval',not p['plan_ready'] and not p.get('verdict'))
    # Same-precondition valid answer precedes each injected storage failure.
    pending=intent(p,'answers-rollback',answers);before=d.database()
    d.write_fixture("CREATE TRIGGER answers_receipt_fault BEFORE INSERT ON preparation_commands BEGIN SELECT RAISE(ABORT,'PRIVATE_ANSWERS_FAULT'); END",())
    faulted=d.database();exit_code,_,err=d.call(['prepare','answer-questions',p['id']],pending)
    check('answers-receipt-rollback',exit_code==2 and d.database()==faulted and 'PRIVATE_ANSWERS_FAULT' not in err.decode())
    d.write_fixture('DROP TRIGGER answers_receipt_fault',());check('answers-rollback-no-partial-history',d.database()==before)
    p=d.ok(['prepare','answer-questions',p['id']],pending);check('answers-storage-public-recovery',json.loads(p['documents']['plan']['text'])['questions']==answers)
    row=d.ok(['prepare','show',p['id']]);damaged=copy.deepcopy(row);damaged['documents']['plan']['sha256']='bad-stored-hash'
    d.write_fixture('UPDATE preparations SET body=? WHERE id=?',(json.dumps(damaged).encode(),p['id']))
    bad=intent(damaged,'answers-corrupt',complete);refused(bad,'answers-corrupt-plan-digest','preparation-document-digest-mismatch')
    p=save(damaged,'answers-repair','plan',row['documents']['plan']['text']);p=d.ok(['prepare','answer-questions',p['id']],intent(p,'answers-after-repair',answers));check('answers-corruption-public-recovery',not p['plan_ready'])
    pending=intent(p,'answers-output',complete);path=out/'output.input.json';publish(path,pending)
    with open('/dev/full','wb',buffering=0) as full:r=subprocess.run([str(d.binary),'prepare','answer-questions',p['id'],'--root',str(d.root),'--json','--input',str(path)],stdout=full,stderr=subprocess.PIPE)
    publish(out/'output.stderr',r.stderr);check('answers-output-known-receipt',r.returncode==2 and json.loads(r.stderr)['recorded_preparation']==dict(id=p['id'],revision=p['revision']+1))
    p=d.ok(['prepare','answer-questions',p['id']],pending);check('answers-output-recovery',json.loads(p['documents']['plan']['text'])['questions']==complete)
    d.settings(preparation_write_enabled='0');refused(pending,'answers-disabled-denies-replay','preparation-write-disabled',4);d.settings(preparation_write_enabled='1')
    before=d.database();old=d.ok(['prepare','answer-questions',p['id']],request);check('answers-historical-replay',old['receipt_historical'] and not old['plan_ready'] and d.database()==before)
    server=native.Server(d.binary,d.root,out,'answers-http');server.capture_async(out/'observer.raw');primary=None
    try:
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token'];caps=server.request('/api/v1/native-capabilities',observe=False);check('answers-http-capability','answer-questions' in caps['preparations'])
        wanted=intent(p,'answers-http',answers);before=d.database();server.request('/api/v1/preparations/command',wanted,csrf='wrong',status=403,observe=False);check('answers-csrf-no-write',d.database()==before)
        p=server.request('/api/v1/preparations/command',wanted,csrf=csrf,observe=False);check('answers-http-shared-store',d.ok(['prepare','show',p['id']])==p and json.loads(p['documents']['plan']['text'])['questions']==answers)
        failure=server.request('/api/v1/preparations/command',{**intent(p,'answers-http-stale',complete),'sha256':'stale'},csrf=csrf,status=409,observe=False);check('answers-http-stale',failure['failure']['code']=='preparation-stale-document')
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,provider_calls=0));print('PREPARATION_ANSWERS_PASS '+str(len(checks)),flush=True)


if __name__=='__main__':main()
