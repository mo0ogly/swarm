#!/usr/bin/env python3
"""Observe actual delegated review then explicit acceptance and stale evidence.

Existing public native reviewer fixture executes one local worker, controls and
reviewer and retains originals. No paid provider, runtime SQL write, live mission,
watchdog or acceptance inferred from an opinion. All status reads preserve exact
logical and physical DB bytes. A stale owned report is restored in finally.
"""
import argparse,json,os,traceback
from pathlib import Path
import rust_native_reviewer as reviewer
import rust_native_review_acceptance as acceptance
import rust_mission_status_contract as shared
import rust_agent_model_route as fixture


def scenario(binary,output):
    """Reuse actual public lifecycle; require supported status before and after acceptance.

    Final passed opinion is not accepted. Explicit task update uses existing
    evidence-verifying public helper. Tampering is confined to the born fixture's
    report; original bytes survive restoration and all failures remain retained.
    """
    proof=output/'reviewer';proof.mkdir()
    result=reviewer.scenario(binary,proof,'passed')
    assert result['status']=='PASS' and result['task_accepted'] is False
    fixture.publish(output/'review-source.json',result)
    accepted=observe(binary,output,proof,Path(result['root']),result['work'])
    cases=[]
    for case in ['changes-requested','domain-invalid']:
        case_out=output/case;case_out.mkdir()
        cases.append(nonpassed(binary,case_out,case))
    fixture.publish(output/'result.json',dict(status='PASS',root=accepted['root'],work=accepted['work'],reads=accepted['reads']+[row for case in cases for row in case['reads']],actual_review=True,explicit_acceptance=True,stale_and_recovery=True,nonpassed_cases=cases,global_migration_complete=False))
    print('PASS delegated final review, explicit acceptance and freshness recovery',flush=True)


def observe(binary,output,proof,root,work,report_relative="handoff.md"):
    """Observe a genuine existing final-review fixture without rerunning providers.

    Caller supplies its retained root/proof coordinates; actual public acceptance
    rechecks originals. report_relative selects the actual fixture report, never
    a product fallback. This debugging entry permits targeted resumption after a
    harness failure, not creating state or claiming another full recipe run.
    """
    driver=shared.Driver(binary,None,output,root,'status-review-fixture',os.environ.copy());driver.work=work
    argv=[str(binary),'mission','status',work,'--root',str(root),'--json']
    def read(label):
        """Read exact historical CLI under the shared DB nonmutation witness."""
        (code,raw,err),_=driver.read(label,argv)
        assert code==0 and not err,(label,code,err)
        return json.loads(raw)
    before=read('review-passed')
    assert before['validated']==0 and before['review']==1 and before['tasks'][0]['state']=='review'
    assert before['tasks'][0]['result']['state']=='result_to_review'
    acceptance.accept(binary,root,work,proof,'status-explicit-accept',True)
    accepted=read('review-accepted')
    assert accepted['validated']==1 and accepted['tasks'][0]['result']['state']=='validated'
    report=root/report_relative;original=report.read_bytes();fixture.publish(output/'report.original',original)
    try:
        fixture.publish(report,original+b'\nOwned stale report probe\n')
        stale=read('review-stale')
        assert stale['validated']==0 and stale['tasks'][0]['result']['state']=='validation_stale'
    finally:fixture.publish(report,original)
    restored=read('review-restored');assert restored['validated']==1
    return dict(root=str(root),work=work,reads=driver.reads)


def nonpassed(binary,output,case):
    """Run actual final negative opinion; verify safe projection and stale recovery.

    Hyphenated native fixture case names are actual public-test inputs. A failed
    reviewer remains an error independently of its recorded state; no task is
    accepted. Only our report is changed then restored; status must never mutate
    SQLite, rerun reviewer or manufacture an applicable verdict.
    """
    proof=output/'reviewer';proof.mkdir()
    result=reviewer.scenario(binary,proof,case)
    fixture.publish(output/'review-source.json',result)
    assert result['status']=='PASS' and result['task_accepted'] is False
    root=Path(result['root']);work=result['work']
    driver=shared.Driver(binary,None,output,root,'status-review-fixture',os.environ.copy());driver.work=work
    argv=[str(binary),'mission','status',work,'--root',str(root),'--json']
    def read(label):
        """Require actual review result fields, with original refusal preserved."""
        (code,raw,err),_=driver.read(label,argv)
        assert code==0 and not err,(label,code,err)
        value=json.loads(raw);assert value['validated']==value['running']==value['review']==0
        task=value['tasks'][0];assert task['state']=='intervention' and task['result']['state']=='review_blocked',task
        return task['result']
    actual=read('negative-review')
    expected='review_changes_requested' if case=='changes-requested' else 'review_unvalidated'
    assert actual['validation_state']==expected and actual['report_state']=='submitted',actual
    report=root/'handoff.md';original=report.read_bytes();fixture.publish(output/'report.original',original)
    try:
        fixture.publish(report,original+b'\nOwned changed negative-review report\n')
        stale=read('negative-stale')
        assert stale['validation_state']=='failed_or_stale' and stale['report_state']=='stale',stale
    finally:fixture.publish(report,original)
    restored=read('negative-restored');assert restored['validation_state']==expected
    return dict(case=case,root=str(root),work=work,reads=driver.reads,task_accepted=False)


def main():
    """Bind normal production-profile bytes before invoking public owned fixtures."""
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--profile',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=False)
    try:
        profile=json.loads(a.profile.read_text());assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['sha256']==fixture.digest(a.binary.read_bytes())
        fixture.publish(a.output/'profile.json',profile);fixture.publish(a.output/'method.py',Path(__file__).read_bytes())
        scenario(a.binary.resolve(),a.output)
    except BaseException:fixture.publish(a.output/'failure.txt',traceback.format_exc().encode());raise

if __name__=='__main__':main()
