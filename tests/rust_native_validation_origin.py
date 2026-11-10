#!/usr/bin/env python3
"""Require source-origin backing after a fully public native control execution.

Owned isolated Roots only, no paid provider or live application data. SQL reads
inspect actual producer rows; the sole write injects/removes and restores one
origin row AFTER the real controller has exited, never seeds a job, permission,
positive receipt or acceptance. Exact original frames and execution witnesses
must remain unchanged. No global verification watchdog or implicit retry.
"""
import argparse, json, sqlite3, sys, traceback
from pathlib import Path
import rust_native_task_validation as native


def scenario(binary, out):
    """Real execution and replay precede missing-origin refusal and exact restore.

    This is an offline owned-fixture corruption test, not a supported operator
    repair command or evidence that SQL identities confer execution authority.
    """
    setup = native.public.flow(binary, out, control_script=native.CONTROL)
    root = Path(setup['root']); work = setup['work_id']
    request = dict(schema_version=1, event_id='validation-origin', expected_revision=setup['revision'], task_id='task')
    code, done = native.invoke(binary, root, out, 'validate', ['task', 'validate', work], request)
    assert code == 0 and done['controls_passed'] and not done['task_accepted']
    job = done['job_id']
    original = root / '.swarm' / ('native-validation-job-' + native.fixture.digest(job.encode()) + '.receipt')
    raw = original.read_bytes(); magic, digits, digest, body = raw.split(b'\n', 3)
    assert magic == b'SR1' and len(digits) == 20 and len(digest) == 64 and int(digits) == len(body) and native.fixture.digest(body).encode() == digest
    record = json.loads(body)
    assert record.get('source_origin'), 'new native job lacks recorded source origin'
    origin = record['source_origin']; assert record['schema_version'] == 2 and origin['job'] == job
    assert origin['source_sha256'] != record['job']['source_sha256']
    assert 'candidate_binding_sha256' not in origin, 'ordinary controls must not invent a managed candidate'
    witness = (root/'validation-executed.json').read_bytes()
    code, replay = native.invoke(binary, root, out, 'valid-replay', ['task', 'validate', work], request)
    assert code == 0 and replay['replayed'] and (root/'validation-executed.json').read_bytes() == witness
    db = sqlite3.connect(root/'.swarm/state.db')
    try:
        row = db.execute('SELECT * FROM native_validation_source_origins WHERE job_id=?', (job,)).fetchone()
        assert row == (job, origin['source_sha256'], origin['controls_sha256'], None)
        db.execute('DELETE FROM native_validation_source_origins WHERE job_id=?', (job,)); db.commit()
        try:
            code, denied = native.invoke(binary, root, out, 'missing-origin', ['task', 'validate', work], request)
            assert code != 0 and 'native-validation-origin-receipt-unbacked' in json.dumps(denied)
            assert original.read_bytes() == raw and (root/'validation-executed.json').read_bytes() == witness
        finally:
            db.execute('INSERT INTO native_validation_source_origins VALUES(?,?,?,?)', row); db.commit()
    finally:
        db.close()
    code, restored = native.invoke(binary, root, out, 'restored-replay', ['task', 'validate', work], request)
    assert code == 0 and restored['replayed'] and (root/'validation-executed.json').read_bytes() == witness
    assert original.read_bytes() == raw
    return dict(status='PASS', actual_native_control=True, origin_row_and_original_match=True,
                missing_origin_refused=True, restored_replay_without_execution=True,
                task_accepted=False, managed_candidate_bound=False, real_provider=False)


def legacy_replay(binary, out, retained):
    """Replay a real retained version1 job, without fabricating legacy SQL/data.

    The caller supplies the earlier public fixture's evidence. Both original
    frame and execution witness stay byte-identical; reads create no origin row.
    """
    call=json.loads((retained/'validate.command.json').read_text())['argv']
    root=Path(call[call.index('--root')+1])
    request=json.loads((retained/'validate.input.json').read_text())
    previous=json.loads((retained/'validate.stdout').read_bytes())
    work=previous['work']['id'];job=previous['job_id']
    original=root/'.swarm'/('native-validation-job-'+native.fixture.digest(job.encode())+'.receipt')
    raw=original.read_bytes();record=json.loads(raw.split(b'\n',3)[3])
    assert record['schema_version']==1 and 'source_origin' not in record
    witness=(root/'validation-executed.json').read_bytes()
    code,result=native.invoke(binary,root,out,'legacy-replay',['task','validate',work],request)
    assert code==0 and result['replayed'] and result['controls_passed']
    assert original.read_bytes()==raw and (root/'validation-executed.json').read_bytes()==witness
    with sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        assert db.execute("SELECT count(*) FROM sqlite_schema WHERE name='native_validation_source_origins'").fetchone()==(0,)
    return dict(actual_version1_original=True,replayed_without_execution=True,original_unchanged=True,no_origin_backfill=True)


def retained_v2_replay(binary, out, retained):
    """Replay an actual earlier producer's v2 frame unchanged, without controls.

    Reads the retained public argv/response to locate its isolated Root. Checks
    raw private frame, execution witness, origin row and terminal Job tuple before
    and after the public exact-event replay. No SQL mutation or version backfill;
    this verifies compatibility, not managed execution or independent acceptance.
    """
    call=json.loads((retained/'validate.command.json').read_text())['argv']
    root=Path(call[call.index('--root')+1])
    request=json.loads((retained/'validate.input.json').read_text())
    previous=json.loads((retained/'validate.stdout').read_bytes())
    work=previous['work']['id'];job=previous['job_id']
    original=root/'.swarm'/('native-validation-job-'+native.fixture.digest(job.encode())+'.receipt')
    raw=original.read_bytes();magic,digits,digest,body=raw.split(b'\n',3)
    assert magic==b'SR1' and len(digits)==20 and int(digits)==len(body) and native.fixture.digest(body).encode()==digest
    record=json.loads(body)
    assert record['schema_version']==2 and record['source_origin'] and record['effective_policy']
    assert 'managed_candidate' not in record and 'candidate_binding_sha256' not in record['source_origin']
    witness=(root/'validation-executed.json').read_bytes()
    def rows():
        """Observe only the retained fixture's exact origin and terminal job."""
        with sqlite3.connect('file:'+str(root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
            origin=db.execute('SELECT * FROM native_validation_source_origins WHERE job_id=?',(job,)).fetchone()
            terminal=db.execute('SELECT source_sha256,phase,receipt_sha256 FROM native_validation_jobs WHERE id=?',(job,)).fetchone()
        assert origin and terminal and terminal[1]=='recorded'
        return origin,terminal
    before=rows()
    code,result=native.invoke(binary,root,out,'retained-v2-replay',['task','validate',work],request)
    assert code==0 and result['replayed'] and result['controls_passed'] and not result['task_accepted']
    assert original.read_bytes()==raw and (root/'validation-executed.json').read_bytes()==witness and rows()==before
    return dict(actual_version2_original=True,replayed_without_execution=True,original_and_rows_unchanged=True,no_schema3_backfill=True)


def main():
    """Retain actual binary/helper/script identity and originals on every failure."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary', required=True, type=Path);p.add_argument('--output', required=True, type=Path);p.add_argument('--legacy-evidence', type=Path, help='Retained real version1 public fixture, never synthesized');p.add_argument('--prior-v2-evidence', type=Path, help='Retained actual version2 origin fixture from an earlier binary');a=p.parse_args()
    out=a.output.resolve();out.mkdir(exist_ok=False);binary=a.binary.resolve()
    try:
        sources=list(dict.fromkeys([Path(__file__),Path(native.__file__),Path(native.public.__file__),Path(native.fixture.__file__),Path(native.public.feedback.__file__),Path(native.public.mission.__file__),Path(native.public.feedback.native.__file__)]+[Path(sys.modules[f.__module__].__file__) for f in [native.public.request,native.public.feedback.fresh_work,native.public.feedback.ledger,native.public.feedback.apply_policy]]))
        inputs=dict(binary=str(binary), binary_sha256=native.fixture.digest(binary.read_bytes()),
                    sources={str(p):native.fixture.digest(p.read_bytes()) for p in sources})
        native.fixture.publish(out/'inputs.json', inputs)
        result=scenario(binary,out)
        result['legacy'] = legacy_replay(binary,out,a.legacy_evidence.resolve()) if a.legacy_evidence else None
        result['prior_v2'] = retained_v2_replay(binary,out,a.prior_v2_evidence.resolve()) if a.prior_v2_evidence else None
        assert native.fixture.digest(binary.read_bytes())==inputs['binary_sha256']
        assert all(native.fixture.digest(Path(name).read_bytes())==sha for name,sha in inputs['sources'].items())
        native.fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:
        native.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__': main()
