from pathlib import Path
import sys
sys.path.insert(0,'.cache/rust-git-port');from editlib import edit
p='tests/rust_mission_worker_recovery.py'
old="""        status=request(server,'GET','/api/v1/mission?'+urlencode(dict(work=f.work,action='status')))
        actual=next(t for t in status['tasks'] if t['id']=='task')['result']
        assert actual['state']=='completed_unproven' and actual['validation_state']=='not_started'
"""
new="""        status_path='/api/v1/mission?'+urlencode(dict(work=f.work,action='status'))
        status_checks=[]
        def status_read(label, expected_state, expected_report):
            \"\"\"Compare real historical CLI/HTTP results and prove each read is nonmutating.

            Fixture report edits/policy writes occur outside this snapshot. Native
            process/provider markers and settlement resources must remain exact;
            no status result can accept a task or submit its candidate report.
            \"\"\"
            before_read=snapshot(f,agent);providers=completion.provider_files(root)
            shown=f.ok(['mission','status',f.work])
            observed=request(server,'GET',status_path)
            cli_result=next(t for t in shown['tasks'] if t['id']=='task')['result']
            http_result=next(t for t in observed['tasks'] if t['id']=='task')['result']
            assert cli_result==http_result and cli_result['state']==expected_state,(label,cli_result)
            assert cli_result['report_state']==expected_report and cli_result['validation_state']=='not_started'
            assert snapshot(f,agent)==before_read and completion.provider_files(root)==providers
            assert shown['validated']==0 and observed['validated']==0
            status_checks.append(dict(case=label,result=cli_result));return cli_result
        status_read('absent-root-report','completed_unproven','absent_or_unattributed')
        docs=root/'docs';docs.mkdir(exist_ok=True);report=docs/'task.md'
        assert not report.exists(),'Fixture root report unexpectedly already exists'
        fixture.publish(report,b'Owned status report, not submitted or accepted')
        fresh=status_read('fresh-root-report','result_to_review','attributable_unsubmitted')
        assert fresh['report_id']=='docs/task.md'
        duplicate=docs/'task-extra-handoff.md';fixture.publish(duplicate,b'Owned ambiguous candidate')
        ambiguous=status_read('ambiguous-root-reports','completed_unproven','absent_or_unattributed')
        assert 'plusieurs rapports' in ambiguous['reason']
        duplicate.unlink();status_read('ambiguity-restored','result_to_review','attributable_unsubmitted')
        from datetime import datetime
        from rust_planning_activation import apply_policy
        started=datetime.fromisoformat(final['started'].replace('Z','+00:00'))
        original_stat=report.stat();old_ns=int(started.timestamp())*1_000_000_000-1_000_000_000
        os.utime(report,ns=(original_stat.st_atime_ns,old_ns))
        apply_policy(f,'status-margin-zero',dict(agent_report_clock_skew_seconds='0'))
        stale=status_read('saved-zero-rejects-old-report','completed_unproven','absent_or_unattributed')
        assert '1 rapport(s) antérieur(s)' in stale['reason']
        apply_policy(f,'status-margin-two',dict(agent_report_clock_skew_seconds='2'))
        status_read('saved-margin-reopens-old-report','result_to_review','attributable_unsubmitted')
        fixture.publish(report,b'');status_read('empty-root-report','completed_unproven','absent_or_unattributed')
        fixture.publish(report,b'Restored owned root report');status_read('empty-restored','result_to_review','attributable_unsubmitted')
        original_report=report.read_bytes();foreign=out/'outside-status-report';fixture.publish(foreign,b'Outside root')
        report.unlink();report.symlink_to(foreign)
        before_read=snapshot(f,agent)
        refused_code,refused=f.call(['mission','status',f.work]);assert refused_code!=0 and refused
        request(server,'GET',status_path,expected=500)
        assert snapshot(f,agent)==before_read
        report.unlink();fixture.publish(report,original_report)
        status_read('outside-path-restored','result_to_review','attributable_unsubmitted')
        report.unlink();status_read('root-report-removed','completed_unproven','absent_or_unattributed')
        fixture.publish(out/'historical-status-cases.json',status_checks)
        print('PASS recovery-historical-cli-http-report-attribution-and-saved-policy',flush=True)
"""
edit(p,old,new)
edit(p,'http_historical_completed_blocked_readable=True','http_historical_completed_blocked_readable=True,status_read_cases=len(status_checks)')
# Permanent manifest consumes the changed helpers and requires the new public assertion.
p=Path('tools/verification/recipes/mission-worker-completion-recovery.json')
import json
from editlib import atomic
r=json.loads(p.read_bytes());r['inputs']=sorted(set(r['inputs']+['rust/crates/swarm-engine/src/root_store/mission_status_reports.rs','rust/crates/swarm-engine/src/root_store/mission_status_historical.rs','rust/crates/swarm-engine/src/root_store/mission_status_review.rs','rust/crates/swarm-engine/src/mission_status_tasks.rs','rust/crates/swarm-engine/src/gate_evaluation.rs']))
r['steps'][0]['required_stdout'].append('PASS recovery-historical-cli-http-report-attribution-and-saved-policy');atomic(p,json.dumps(r,indent=2)+'\n')
