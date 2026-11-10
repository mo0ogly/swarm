#!/usr/bin/env python3
"""Actual manual-origin automatic review, then same-directory DB-copy refusal.

Positive native review first, fresh inactive Root second. Rename original SQLite
and atomically copy its exact closed bytes to a new inode in the same directory.
No SQL mutation, fabricated provenance, live Root, retry or global watchdog.
Keep both databases and restore the original name after the probe, with all
primary/cleanup causes. Existing binding rows must not authorize another DB.
"""
import argparse,json,os,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_manual_review as manual
import rust_mission_report_relay as relay
import rust_mission_watch_worker as worker
import rust_native_parent_death as parent_death
check=worker.check

def fault(binary,out,manual_controls=True):
    """Copy closed owned bytes without modifying SQL authority, require exact
    physical-origin refusal before review debit, then retain/restore both files.
    The successful witness is a separate fresh Root with identical setup.
    A genuinely absent optional review table means no claim; all other SQL or
    representation failures remain failures rather than successful empty data.
    manual_controls selects the retained explicit-control reference or delegates
    the first native controls to watch; it never adds synthetic authority.
    """
    out.mkdir();owned,agent,calls,finance_before,program_sha=manual.prepare(binary,out,manual_controls=manual_controls)
    before=relay.work(owned);db=owned.root/'.swarm/state.db';backup=db.with_name('origin-original-state.db');faulted=db.with_name('origin-refused-state.db')
    check(db.is_file() and not db.is_symlink() and not backup.exists() and not faulted.exists(),'origin fixture database ownership invalid')
    check(not db.with_name('state.db-wal').exists() and not db.with_name('state.db-shm').exists(),'fixture SQLite is not closed/checkpointed before physical copy')
    original=db.read_bytes();info=db.stat();errors=[];moved=False;replaced=False
    try:
        db.rename(backup);moved=True;fixture.publish(db,original);replaced=True
        check(db.stat().st_dev==info.st_dev and db.stat().st_ino!=info.st_ino and db.read_bytes()==original,'negative did not create a distinct same-directory DB with exact original bytes')
        fixture.publish(out/'physical-copy.json',dict(original_inode=info.st_ino,copy_inode=db.stat().st_ino,device=info.st_dev,sha256=fixture.digest(original)))
        events=relay.cycle(owned,out/'watch',refusal=True)
        failure=json.loads((out/'watch/watch.stderr').read_bytes());check('agent-origin-instance-changed' in failure['failure']['causes'],'wrong physical-producer refusal')
        check(relay.work(owned)==before and relay.rows(owned,'SELECT id,scope_id,estimate,state FROM planning_calls WHERE work_id=? ORDER BY id',(owned.work,))==finance_before,'refusal changed Work or financial authority')
        check(not (owned.root/'reviewer-invoked.json').exists() and not any(e.get('event')=='mission-review-recorded' for e in events),'different DB started or published reviewer')
        # The first refused claim may leave the optional job table genuinely
        # absent. Only schema absence counts as zero; SQL/type failures refuse.
        tables=relay.rows(owned,"SELECT type FROM sqlite_schema WHERE name=? COLLATE NOCASE",('native_review_jobs',))
        if tables:
            check(tables==[('table',)],'unexpected reviewer journal representation')
            check(relay.rows(owned,'SELECT count(*) FROM native_review_jobs WHERE work_id=?',(owned.work,))==[(0,)],'physical refusal left a reviewer claim')
        fixture.publish(out/'public-failure.json',failure)
    except BaseException as e:errors.append(e)
    finally:
        if moved:
            try:
                if replaced:db.rename(faulted)
                backup.rename(db)
            except BaseException as e:errors.append(e)
    if errors:raise BaseExceptionGroup('manual origin copy/refusal/restoration',errors)
    check(db.read_bytes()==original and db.stat().st_ino==info.st_ino and faulted.exists(),'original inode/bytes not restored or refused copy lost')
    check(relay.work(owned)==before,'restored original Work differs')
    return dict(root=str(owned.root),work=owned.work,same_directory_different_database=True,no_reviewer_started=True,no_financial_mutation=True,original_restored=True,task_accepted=False)

def main():
    """Validate actual profile, execute positive before fault and restore the
    owned process/subreaper boundary, retaining every failure and original.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for flag in ['binary','profile','output']:p.add_argument('--'+flag,type=Path,required=True)
    p.add_argument('--automatic-controls',action='store_true',help='Require first native controls and review to be admitted by watch')
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True);binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());check(profile['no_fixture_feature'] and profile['sha256']==fixture.digest(binary.read_bytes()),'origin refusal profile mismatch')
    def run():
        positive=out/'positive';positive.mkdir();control=manual.run(binary,positive,manual_controls=not a.automatic_controls);fixture.publish(positive/'result.json',control)
        return dict(status='PASS',positive_control=control,physical_refusal=fault(binary,out/'different-database',manual_controls=not a.automatic_controls),installed=False,global_watchdog=None)
    result=worker.subreaper_scope(run,lambda e:fixture.publish(out/'failure.json',dict(status='FAIL',cause=''.join(traceback.format_exception(e)))),parent_death.subreaper)
    fixture.publish(out/'result.json',result);print('PASS manual native origin supports automatic review and refuses another SQLite instance',flush=True)
if __name__=='__main__':main()
