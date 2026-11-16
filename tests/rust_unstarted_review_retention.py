"""I0-aware copies of actual Gi review originals; no Root writes or acceptance."""
import json
import os
from pathlib import Path
import stat
import rust_agent_model_route as fixture
from rust_unstarted_review_process import Failure, check


def read_at(root, components):
    """Read relative to held NOFOLLOW directories and explicitly close all fds.

    Fixed protocol components/validated event IDs only. Regular file/UID and
    before/after identity are checked; this is not a hostile same-UID sandbox.
    Read and every close failure remain typed together, never empty success.
    """
    fds, errors, raw, identity = [], [], None, None
    try:
        fd = os.dup(root) if isinstance(root,int) else os.open(root, os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW)
        fds.append(fd)
        for name in components[:-1]:
            check(isinstance(name,str) and name not in ('','.','..') and '/' not in name, 'retention-path-component')
            fd = os.open(name, os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW, dir_fd=fd)
            fds.append(fd)
        name = components[-1]
        check(isinstance(name,str) and name not in ('','.','..') and '/' not in name, 'retention-file-component')
        file = os.open(name, os.O_RDONLY|os.O_CLOEXEC|os.O_NOFOLLOW, dir_fd=fd)
        fds.append(file)
        before = os.fstat(file)
        check(stat.S_ISREG(before.st_mode) and before.st_uid == os.getuid(), 'retention-owned-regular-file')
        chunks = []
        while True:
            chunk = os.read(file, 65536)  # technical IO batch, not a source/output cap
            if not chunk:
                break
            chunks.append(chunk)
        raw = b''.join(chunks)
        after = os.fstat(file)
        check((before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns) ==
              (after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns), 'retention-file-changed')
        identity = dict(device=before.st_dev,inode=before.st_ino)
    except BaseException as error:
        errors.append(error)
    finally:
        for fd in reversed(fds):
            try:
                os.close(fd)
            except BaseException as error:
                errors.append(error)
    if errors:
        raise Failure('retention-read-close-failed', errors)
    return raw, identity


def retain_review(root, work, task_id, result, owner, output):
    """Hold Root and private directory through all original reads and close them.

    No pathname stat substitutes for the Job directory identity. Primary/read
    and both descriptor-close failures remain owned until the final consumer.
    """
    held, errors, value = [], [], None
    try:
        rootfd=os.open(root,os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW)
        held.append(rootfd)
        privatefd=os.open('.swarm',os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC|os.O_NOFOLLOW,dir_fd=rootfd)
        held.append(privatefd)
        value=_retain_review(root,work,task_id,result,owner,output,rootfd,privatefd)
    except BaseException as error:
        errors.append(error)
    finally:
        for fd in reversed(held):
            try:os.close(fd)
            except BaseException as error:errors.append(error)
    if errors:raise Failure('retention-originals-or-held-directory-close',errors)
    return value


def _retain_review(root, work, task_id, result, owner, output, rootfd, privatefd):
    """Copy genuine SR1, content-v2 certificate and event-addressed I0 marker.

    Requires already successful normal task review and Work readback. Checks full
    schema/bindings, actual waited/granted child, raw capture and marker before
    declaring a witness. No fabricated Root-relative reply/marker or journal.
    Public reconciliation by the caller remains an independent engine check.
    """
    root, output = Path(root).resolve(), Path(output)
    output.mkdir(mode=0o700, exist_ok=False)
    task = next(t for t in work['tasks'] if t['id'] == task_id)
    review = task['independent_review']
    check(result['review_recorded'] is True and result['verdict_applicable'] is True
          and result['task_accepted'] is False and result['replayed'] is False
          and result['review_id'] == review['id'] and review['state'] == 'passed', 'retention-applicable-review-required')
    digest = fixture.digest(review['id'].encode())
    frame, _ = read_at(privatefd, ['native-review-'+digest+'.receipt'])
    fixture.publish(output/'original.frame', frame)
    magic, extent, checksum, body = frame.split(b'\n',3)
    check(magic == b'SR1' and extent.isdigit() and int(extent) == len(body)
          and checksum.decode('ascii') == fixture.digest(body), 'retention-SR1-frame')
    receipt = json.loads(body)
    job = receipt['job']
    check(receipt['schema_version'] == 2 and job['id'] == review['id']
          and job['work_id'] == work['id'] and job['task_id'] == task_id
          and job['attempt_id'] == review['attempt'] and job['producer_id'] == review['producer']
          and job['contract_sha256'] == review['contract'], 'retention-job-binding')
    check(all(receipt[k] is True for k in ('process_attempted','grant_attempted','channel_closed','waited','output_complete'))
          and receipt['failed'] is False and receipt['exit_code'] == 0 and receipt['signal'] is None
          and all(receipt[k] is False for k in ('duration_expired','drain_expired','source_interrupted'))
          and receipt['gate'] is None and receipt['unresolved_pid'] is None and receipt['ownership_lost_pid'] is None,
          'retention-real-reviewer-wait-required')
    check(owner.reaped and owner.status == 0 and owner.identity is not None
          and job['owner_pid'] == owner.process.pid and job['owner_stamp'] == owner.identity['stamp']
          and job['owner_host'] == owner.identity['host'] and job['permission'] == 1
          and type(job['child_pid']) is int and job['child_pid'] > 0
          and job['parent_pid'] == job['owner_pid'] and job['group_id'] == job['child_pid']
          and job['phase'] == 'permitted' and isinstance(job['child_stamp'],str) and bool(job['child_stamp']), 'retention-observed-owner-child')
    check(receipt['original_reply'] is not None and receipt['raw_capture']['failed'] is False, 'retention-complete-capture-required')
    streams = {}
    for channel in ('stdout','stderr'):
        stream = receipt['raw_capture'][channel]
        streams[channel] = bytes(stream['retained'])
        check(stream['truncated'] is False and len(streams[channel]) == stream['observed_bytes']
              and fixture.digest(streams[channel]) == stream['sha256'], 'retention-full-raw-stream-binding')
    retained = streams['stdout']
    check(sum(map(len,streams.values())) == receipt['raw_capture']['retained_bytes']
          and fixture.digest(bytes(receipt['original_reply'])) == review['reply_sha256'], 'retention-reply-capture-binding')
    event = job['event_id']
    check(isinstance(event,str) and event and event.isascii()
          and all(c.isalnum() or c in '-_' for c in event), 'retention-event-component')
    marker, _ = read_at(privatefd, ['biba-i0','reviewer',event,'reviewer-invoked.json'])
    invoked = json.loads(marker)
    check(invoked['pid'] == job['child_pid'] and invoked['ppid'] == job['parent_pid']
          and invoked['stdin_sha256'] == job['prompt_sha256'], 'retention-I0-invocation-binding')
    certname = 'native-review-public-v2-'+review['receipt_sha256']+'.json'
    check(review['receipt'] == '.swarm/'+certname, 'retention-certificate-reference')
    certificate, _ = read_at(privatefd, [certname])
    check(fixture.digest(certificate) == review['receipt_sha256'], 'retention-certificate-hash')
    cert = json.loads(certificate)
    expected_review = dict(review)
    expected_review.pop('receipt',None)
    expected_review.pop('receipt_sha256',None)
    expected = dict(schema_version=2,review=expected_review,work_id=work['id'],task_id=task_id,
                    source_sha256=job['source_sha256'],validation_receipt_sha256=job['validation_receipt_sha256'])
    check(cert == expected, 'retention-certificate-content')
    st = os.fstat(privatefd)
    check((str(st.st_dev),str(st.st_ino)) == (job['directory_device'],job['directory_inode']), 'retention-Root-directory-binding')
    program, _ = read_at(rootfd, ['codex'])
    check(fixture.digest(program) == job['executable_sha256'],'retention-actual-program-digest')
    for name, raw in [('original.body.json',body),('certificate.json',certificate),('invocation.json',marker),('inert-provider.py',program),('capture.stdout.raw',retained),('capture.stderr.raw',streams['stderr'])]:
        fixture.publish(output/name,raw)
        check((output/name).read_bytes() == raw, 'retention-copy-readback')
    facts = dict(review=review['id'],work=work['id'],task=task_id,original_expiry=job['preflight_until'],
                 owner_pid=job['owner_pid'],child_pid=job['child_pid'],settings_revision=job['settings_revision'],frame_sha256=fixture.digest(frame),
                 certificate_sha256=review['receipt_sha256'],program_sha256=fixture.digest(program),
                 marker_sha256=fixture.digest(marker),reply_sha256=review['reply_sha256'],
                 scope='actual owned inert reviewer, not independent expert judgement or acceptance')
    fixture.publish(output/'retention.json', facts)
    return facts
