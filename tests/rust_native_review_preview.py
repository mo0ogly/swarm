#!/usr/bin/env python3
"""Read actual native control provenance before any future reviewer invocation.

Owned inert fixtures only. Valid published evidence precedes each fault. A live
publishing controller is held by real stdout backpressure, not a synthetic PID.
No paid model, live mission, hidden retry, global deadline or reviewer is used.
"""
import argparse, fcntl, json, os, select, sqlite3, subprocess, traceback
from pathlib import Path
import rust_native_task_validation as native
import rust_native_task_validation_failures as guards


def call(binary, root, out, name, argv, value=None):
    """Retain exact public argv/output/exit; no shell, retry or global watchdog."""
    return native.invoke(binary, root, out, name, argv, value)


def shown(binary, root, out, name, work):
    """Read actual Work or fail with its original diagnostic, never empty success."""
    code, wire = call(binary, root, out, name, ['work', 'show', work])
    assert code == 0, wire
    return wire['work']


def preview(binary, root, out, name, work, request, expected=True):
    """Assert read-only source/evidence coordinates or refusal without state loss."""
    before = shown(binary, root, out, name+'-before', work)
    code, wire = call(binary, root, out, name, ['task', 'review-preview', work], request)
    assert (code == 0) == expected, wire
    assert shown(binary, root, out, name+'-after', work) == before
    if expected:
        assert wire['native_control_evidence_verified'] and wire['validation']['job_id']
        assert wire['work_revision'] == before['revision'] and wire['task_id'] == 'task'
        assert not any(wire[k] for k in ['invocation_context_materialized','invocation_authorized','provider_started','task_accepted'])
    return wire


def source(binary, out):
    """Create a worker/report through public planning operations; controls not yet run."""
    out.mkdir()
    setup = native.public.flow(binary, out, control_script=native.CONTROL)
    return setup, Path(setup['root'])


def saved_setting(binary, root, out, label, field, value):
    """Save a fixture policy through preview/apply/reopen with history intact.

    No settings JSON or budget is rewritten privately; preview bytes stay exact.
    IO, refusal and close/output errors fail with original command diagnostics.
    """
    code, old=call(binary,root,out,label+'-show',['engine-settings','show']);assert code==0,old
    values=dict(old['values']);values[field]=value
    request=dict(schema_version=old['schema_version'],event_id=label,expected_revision=old['revision'],values=values,reason='Verify actual reviewer workflow cap',preview_token='')
    before=(root/'.swarm/engine-settings.json').read_bytes()
    code, proposed=call(binary,root,out,label+'-preview',['engine-settings','preview'],request);assert code==0,proposed
    assert (root/'.swarm/engine-settings.json').read_bytes()==before
    request['preview_token']=proposed['preview_token']
    code, applied=call(binary,root,out,label+'-apply',['engine-settings','apply'],request);assert code==0,applied
    code, saved=call(binary,root,out,label+'-reopen',['engine-settings','show']);assert code==0,saved
    assert saved['values']==values and saved['history'][:len(old['history'])]==old['history']
    return old, saved


def faults(binary, out):
    """Require current registered original evidence, then reject and restore faults.

    Only owned fixture receipt bytes and one scalar slot permission are faulted,
    after two healthy actual executions; the current receipt must select the second
    job while retaining the first. Actual /dev/full output refuses without a saved
    mutation claim. Original bytes/permission are restored in
    finally; no fake positive receipt or live DB is ever seeded.
    """
    setup, root = source(binary, out)
    work = setup['work_id']
    request = dict(schema_version=1,event_id='review-controls',expected_revision=setup['revision'],task_id='task')
    preview(binary,root,out,'unexecuted',work,request,False)
    code, done = call(binary,root,out,'controls',['task','validate',work],request)
    assert code == 0 and done['controls_passed'], done
    first_job = done['job_id']
    (root/'validation-executed.json').rename(root/'validation-executed.first.json')
    again = dict(request,event_id='review-controls-second',expected_revision=done['work']['revision'])
    code, done = call(binary,root,out,'controls-second',['task','validate',work],again)
    assert code == 0 and done['controls_passed'] and done['job_id'] != first_job, done
    with guards.database(root) as db:
        count=db.execute("SELECT count(*) FROM native_validation_jobs WHERE work_id=? AND task_id='task' AND phase='recorded'",[work]).fetchone()[0]
    assert count == 2
    request = dict(request,event_id='read-review',expected_revision=done['work']['revision'])
    healthy = preview(binary,root,out,'healthy',work,request)
    assert healthy['validation']['job_id'] == done['job_id'] and healthy['validation']['job_id'] != first_job
    before = shown(binary,root,out,'output-before',work)
    input_path=out/'output-fault.input.json';native.fixture.publish(input_path,request)
    argv=[str(binary),'task','review-preview',work,'--root',str(root),'--json','--input',str(input_path)]
    native.fixture.publish(out/'output-fault.command.json',dict(argv=argv,stdout='/dev/full',global_watchdog=None))
    with open('/dev/full','wb',buffering=0) as full:
        output=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=full,stderr=subprocess.PIPE)
    native.fixture.publish(out/'output-fault.stderr',output.stderr)
    error=json.loads(output.stderr)
    assert output.returncode != 0 and error['failure']['code'] == 'storage_unavailable',error
    assert work not in error['failure']['message'],error
    assert shown(binary,root,out,'output-after',work) == before
    original_settings, limited=saved_setting(binary,root,out,'review-cap-low','agent_workflow_max_body_bytes','1')
    rejected=preview(binary,root,out,'saved-workflow-cap',work,request,False)
    assert 'agent-workflow-body-limit-exceeded' in str(rejected),rejected
    saved_setting(binary,root,out,'review-cap-restore','agent_workflow_max_body_bytes',original_settings['values']['agent_workflow_max_body_bytes'])
    restored=preview(binary,root,out,'saved-workflow-restored',work,request)
    assert restored['validation']==healthy['validation'] and restored['workflow_prompt_sha256']==healthy['workflow_prompt_sha256']
    assert restored['settings_revision']!=healthy['settings_revision']
    healthy=restored
    witness = (root/'validation-executed.json').read_bytes()
    # Original journal existence is checked independently from its copied result.
    with guards.database(root) as db:
        slot = db.execute('SELECT id,permission FROM native_validation_steps WHERE job_id=?',[done['job_id']]).fetchone()
    assert slot and slot[1] == 1
    journal = root/'.swarm'/('native-validation-step-'+native.fixture.digest(slot[0].encode())+'.receipt')
    saved = journal.with_name(journal.name+'.owned-held')
    journal.rename(saved)
    try: preview(binary,root,out,'missing-slot',work,request,False)
    finally: saved.rename(journal)
    assert preview(binary,root,out,'slot-restored',work,request) == healthy
    public = root/done['work']['tasks'][0]['automatic_validation']['receipt']
    original = public.read_bytes();native.fixture.publish(out/'public.original',original)
    native.fixture.publish(public, original+b'\nfixture corruption')
    try: preview(binary,root,out,'changed-public',work,request,False)
    finally: native.fixture.publish(public, original)
    assert preview(binary,root,out,'public-restored',work,request) == healthy
    db = sqlite3.connect(root/'.swarm/state.db')
    try:
        db.execute('UPDATE native_validation_steps SET permission=0 WHERE id=?',[slot[0]]);db.commit()
        try: preview(binary,root,out,'changed-slot-row',work,request,False)
        finally: db.execute('UPDATE native_validation_steps SET permission=? WHERE id=?',[slot[1],slot[0]]);db.commit()
    finally: db.close()
    assert preview(binary,root,out,'row-restored',work,request) == healthy
    preview(binary,root,out,'asserted-actor',work,dict(request,actor='invented-reviewer'),False)
    # Public report bytes are part of the current canonical source.
    report=root/'handoff.md';original=report.read_bytes();native.fixture.publish(report,original+b'\nfixture change')
    try: preview(binary,root,out,'changed-report',work,request,False)
    finally: native.fixture.publish(report,original)
    assert preview(binary,root,out,'report-restored',work,request) == healthy
    assert (root/'validation-executed.json').read_bytes()==witness
    return dict(root=str(root),published_history_jobs=2,current_job_selected=True,saved_cap_consumed_and_restored=True,failed_preview_output_keeps_state=True,original_journal_required=True,public_receipt_required=True,normalized_slot_required=True,report_source_required=True,restored_without_execution=True)


def live_owner(binary, out):
    """Read a terminal job while its real controller remains blocked on stdout.

    The Linux pipe capacity is an observed fixture transport mechanism, not a
    product policy or time limit. Work JSON must exceed it before launch. pidfd
    watches actual exit. The first readable stdout byte proves the CLI reached
    its post-Root-close output path; no byte is consumed and no SQLite polling or
    watchdog is needed. Original stdout drains and process wait follow the read-only
    assertion, so this never adopts or replaces the live owner.
    """
    setup, root = source(binary, out)
    work=setup['work_id'];request=dict(schema_version=1,event_id='review-live-owner',expected_revision=setup['revision'],task_id='task')
    current=shown(binary,root,out,'prepared',work)
    input_path=out/'control.input.json';native.fixture.publish(input_path,request)
    command=[str(binary),'task','validate',work,'--root',str(root),'--json','--input',str(input_path)]
    native.fixture.publish(out/'control.command.json',dict(argv=command,global_watchdog=None))
    read_fd, write_fd = os.pipe2(os.O_CLOEXEC)
    capacity=fcntl.fcntl(read_fd,fcntl.F_SETPIPE_SZ,4096)
    assert len(json.dumps(current,separators=(',',':')).encode())>capacity
    with os.fdopen(read_fd,'rb') as reader, (out/'control.stderr').open('xb') as err:
        try:
            process=subprocess.Popen(command,stdout=write_fd,stderr=err,stdin=subprocess.DEVNULL)
        finally: os.close(write_fd)
        pidfd=os.pidfd_open(process.pid)
        try:
            native.fixture.publish(out/'controller.json',dict(pid=process.pid,pipe_bytes=capacity,root=str(root)))
            ready, _, _ = select.select([reader.fileno(), pidfd], [], [])
            assert reader.fileno() in ready and process.poll() is None, 'Controller did not reach held output'
            with guards.database(root) as db:
                row=db.execute('SELECT owner_pid,phase,published_revision FROM native_validation_jobs WHERE event_id=?',[request['event_id']]).fetchone()
            assert row and row[0]==process.pid and row[1]=='recorded' and row[2] and process.poll() is None
            current=shown(binary,root,out,'published',work)
            value=dict(request,event_id='read-live-review',expected_revision=current['revision'])
            preview(binary,root,out,'live-owner-read',work,value)
            assert process.poll() is None, 'Read unexpectedly consumed or terminated publisher'
            output=reader.read();process.wait();native.fixture.publish(out/'control.stdout',output)
            assert process.returncode==0 and json.loads(output)['controls_passed']
        finally:
            os.close(pidfd)
            if process.poll() is None:
                output=reader.read();process.wait();native.fixture.publish(out/'cleanup.stdout',output)
    return dict(root=str(root),actual_owner_pid=process.pid,live_published_owner_read=True,no_ownership_transfer=True)


def main():
    """Freeze source binary in fresh proof and retain failures/owned roots for replay."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);raw=a.binary.resolve().read_bytes();binary=out/'swarm-rust';native.fixture.publish(binary,raw);binary.chmod(0o700)
    digest=native.fixture.digest(raw)
    native.fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=digest,source_sha256=native.fixture.digest(Path(__file__).read_bytes())))
    try:
        cases=dict(faults=faults(binary,out/'faults'),live_owner=live_owner(binary,out/'live-owner'))
        assert native.fixture.digest(binary.read_bytes())==digest
        native.fixture.publish(out/'result.json',dict(status='PASS',cases=cases,binary_sha256=digest,reviewer_provider_executed=False,setup_inert_worker_and_planners_executed=True,paid_provider=False,review_accepted=False,installed=False))
        print('NATIVE_REVIEW_PREVIEW_PUBLIC_PASS')
    except BaseException:
        native.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
