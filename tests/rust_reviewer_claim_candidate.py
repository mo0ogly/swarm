#!/usr/bin/env python3
"""Exercise test-loaded claim SDK on real owned public worker/control evidence.

Default: no reviewer execution. Optional inert exec remains test-only; no paid
provider, private positive SQL seed or global watchdog. Explicit --publish-review
exercises candidate result publication and rollback; it never accepts a task.
The SDK harness remains cfg(test); the shared controller also compiles normally.
This harness does not qualify normal CLI execution; use the normal CLI recipe.
"""
import argparse,json,os,subprocess,sys,traceback,shlex,tarfile
import rust_native_reviewer as reviewer
from pathlib import Path
import rust_agent_model_route as f
import rust_native_review_inputs as inputs
import rust_native_review_preview as preview
import rust_native_task_validation_failures as guards


def sdk_artifact_depfile(artifact):
    """Select dep-info belonging to the actual Cargo workspace artifact.

    Test executables and rlibs use their own hashed path. Cargo test may also
    emit a normal companion bin without updating debug/<bin>.d; that top-level
    depfile can describe an older checkout. Resolve exactly one hashed deps bin
    sharing the executable inode and use its adjacent dep-info. No timestamps,
    matching content alone, stale top-level file or outside-source substitution
    is admitted. Missing/ambiguous links, unknown executable kinds or missing
    dep-info raise ValueError. Reads filesystem metadata only; writes nothing.
    This assumes the owned Cargo target has no concurrent writer, as the caller
    contract requires; it is not protection against hostile same-UID replacement.
    """
    import re
    executable=artifact.get('executable')
    if executable:
        binary=Path(executable)
        if artifact['profile']['test']:
            dep=binary.with_suffix('.d')
        else:
            if artifact['target']['kind']!=['bin']:
                raise ValueError('sdk-normal-executable-kind-unknown')
            pattern=re.compile(re.escape(artifact['target']['name'].replace('-','_'))+r'-[0-9a-f]+')
            matches=[p for p in (binary.parent/'deps').iterdir() if pattern.fullmatch(p.name) and p.is_file() and os.path.samefile(binary,p)]
            if len(matches)!=1:
                raise ValueError('sdk-binary-depfile-owner-missing-or-ambiguous')
            dep=matches[0].with_suffix('.d')
    else:
        library=next((Path(p) for p in artifact['filenames'] if p.endswith('.rlib')),None)
        if library is None:raise ValueError('sdk-library-artifact-missing')
        dep=library.with_name(library.name.removeprefix('lib')).with_suffix('.d')
    if not dep.is_file():raise ValueError('sdk-artifact-depfile-missing')
    return dep


def qualify_sdk(checkout,out,before,test_target="swarm_engine"):
    """Bind actual workspace SDK depfiles and features to precompiled bytes.

    Uses the shared candidate/effective-source contract, including literal embeds.
    Unused new files do not invalidate compilation; a newly effective or changed
    input does. Preserve Cargo events, binary SHA, depfiles and source archive.
    Normal companion bins require the exact same-inode hashed dep-info, not a
    possibly stale top-level depfile. Missing/unknown artifacts, external workspace inputs or source drift refuse;
    this qualifies a test build only, never a normal production binary. An explicit
    integration target additionally requires the normal engine library artifact;
    omitted target retains the exact library-test qualification contract.
    """
    import verify_native_profile as profile
    events=[]
    for line in (out/'sdk.stdout').read_text().splitlines():
        if line.startswith('{'):
            events.append(json.loads(line))
    profile.require(any(e.get('reason')=='build-finished' and e.get('success') is True for e in events),'sdk-build-finished-missing')
    artifacts=[e for e in events if e.get('reason')=='compiler-artifact']
    profile.require(bool(artifacts),'sdk-artifacts-missing')
    profile.require(all(not set(e['features']) & {'test-fixtures','test-util'} for e in artifacts),'sdk-unexpected-fixture-feature')
    selected=[e for e in artifacts if e['target']['name']==test_target and e['profile']['test']]
    profile.require(len(selected)==1 and selected[0]['features']==[] and selected[0]['executable'],'sdk-engine-test-profile-invalid')
    if test_target!='swarm_engine':
        engines=[e for e in artifacts if e['target']['name']=='swarm_engine' and not e['profile']['test']]
        profile.require(len(engines)==1 and engines[0]['features']==[],'sdk-integration-normal-engine-invalid')
    paths={checkout/'rust/Cargo.toml',checkout/'rust/Cargo.lock',*(checkout/'rust/crates').glob('*/Cargo.toml')}
    depfiles={}
    artifact_deps=[]
    for artifact in artifacts:
        if not Path(artifact['target']['src_path']).is_relative_to(checkout/'rust/crates'):
            continue
        dep=sdk_artifact_depfile(artifact)
        artifact_deps.append(dict(target=artifact['target']['name'],test=artifact['profile']['test'],executable=artifact['executable'],executable_sha256=f.digest(Path(artifact['executable']).read_bytes()) if artifact['executable'] else None,depfile=str(dep)))
        text=dep.read_text();depfiles[str(dep)]=text
        first=text.splitlines()[0]
        profile.require(': ' in first,'sdk-depfile-invalid')
        for name in shlex.split(first.split(': ',1)[1]):
            path=Path(name)
            paths.add(path if path.is_absolute() else checkout/'rust'/path)
    f.publish(out/'sdk.artifact-depfiles.json',artifact_deps)
    actual=profile.manifest(checkout,paths)
    f.publish(out/'sdk.sources.actual.json',actual)
    profile.effective_unchanged(checkout,before,actual)
    with tarfile.open(out/'sdk.sources.tar.gz','x:gz') as archive:
        for name in sorted(actual):archive.add(checkout/name,arcname=name,recursive=False)
    archived={}
    with tarfile.open(out/'sdk.sources.tar.gz','r:gz') as archive:
        for member in archive:
            profile.require(member.isfile(),'sdk-archive-unexpected-member')
            profile.require(member.name not in archived,'sdk-archive-duplicate-member')
            archived[member.name]=f.digest(archive.extractfile(member).read())
    profile.require(archived==actual,'sdk-archive-bytes-mismatch')
    profile.require(profile.manifest(checkout,paths)==actual,'sdk-source-drift-during-archive')
    executable=Path(selected[0]['executable'])
    f.publish(out/'sdk.profile.json',dict(scope='SDK library test only; not production' if test_target=='swarm_engine' else 'SDK integration test only, with normal engine library; not installed',core_artifact=selected[0],executable_sha256=f.digest(executable.read_bytes()),effective_inputs=actual,depfiles=depfiles,source_stable=True))


def retain_process_observations(root,out):
    """Retain owned SDK process originals beside durable recipe evidence.

    Require this output's exact Root/token marker before copying fixed filenames;
    this prevents accidental live-Root collection, not malicious-UID forgery.
    Hash every copied byte; preserve IO/marker failures as failures, never PASS.
    No Work, ledger, process or provider is changed, and no artifact authenticity
    beyond observed bytes is inferred. The destination belongs to this recipe.
    """
    declared=json.loads((out/'fixture.json').read_text())
    if Path(declared['root']).resolve()!=root.resolve() or (root/'.swarm/owned-review-claim-fixture').read_text()!=declared['token']:
        raise ValueError('owned-review-evidence-fixture-required')
    retained={}
    for name in ['owned-review-process-observation.json','owned-review-exec.stdout','owned-review-exec.stderr','reviewer-invoked.json']:
        original=(root/name).read_bytes()
        f.publish(out/name,original)
        if (out/name).read_bytes()!=original:
            raise ValueError('owned-review-evidence-copy-mismatch')
        retained[name]=f.digest(original)
    for name in [f'owned-review-stale-{label}.{suffix}' for label in ['settings-show','settings-preview','settings-apply','pause','resume','budget-apply'] for suffix in ['input.json','command.json','stdout','stderr']]+['owned-review-private-receipt.json','owned-review-original-receipt.frame','owned-review-source-original','owned-review-stream-handshake','owned-review-publication.json','owned-review-publication-sql-failure.json','owned-review-public-certificate.json','owned-review-publication-source-original','owned-review-supervisor-context.json','owned-review-consumer-faults.json','owned-review-failure-reconciliation.json','owned-review-failure-context.json','owned-review-foreign-owner.json','owned-review-foreign-owner.stdout','owned-review-foreign-owner.stderr','owned-review-departed-recovery.json','owned-review-departed-context.json','owned-review-recovery-source-original','owned-review-certificate-compat.json','owned-review-stale-attempt.json','owned-review-stale-context.json','owned-review-stale-recovery.json','owned-review-budget-attempt.json','owned-review-budget-result.json','owned-review-budget-context.json']:
        path=root/name
        if path.exists():
            original=path.read_bytes();f.publish(out/name,original)
            if (out/name).read_bytes()!=original:raise ValueError('owned-review-evidence-copy-mismatch')
            retained[name]=f.digest(original)
    if (root/'owned-review-private-receipt.json').exists():
        receipts=list((root/'.swarm').glob('native-review-*.receipt'))
        if len(receipts)!=1:raise ValueError('owned-review-single-receipt-required')
        original=receipts[0].read_bytes();f.publish(out/'owned-review-private-receipt.frame',original)
        if (out/'owned-review-private-receipt.frame').read_bytes()!=original:raise ValueError('owned-review-evidence-copy-mismatch')
        retained['owned-review-private-receipt.frame']=f.digest(original)
    f.publish(out/'process-evidence-retention.json',dict(status='PASS',files=retained,scope='Owned fixture byte copies only; not model authenticity or global acceptance'))
    return retained


def sdk_source_inputs(repository,selected):
    """Select repository or explicit owned source fixture, verifying its bytes.

    Path/token metadata is an accidental-mixup guard, not cryptographic source
    authentication. A fixture must declare this repository and its exact path;
    each manifest member must stay inside it and match its digest. Unknown,
    missing, malformed or changed sources refuse before Cargo; no files, process
    or authority are changed. Later effective-depfile checks remain mandatory.
    """
    checkout=selected.resolve() if selected else repository
    if selected:
        snapshot=json.loads((checkout/'sdk-source-snapshot.json').read_text())
        if snapshot['source_root']!=str(repository) or snapshot['snapshot']!=str(checkout):
            raise ValueError('sdk-owned-source-snapshot-required')
        for name,digest in snapshot['files'].items():
            path=(checkout/name).resolve()
            if not path.is_relative_to(checkout) or f.digest(path.read_bytes())!=digest:
                raise ValueError('sdk-owned-source-snapshot-changed')
    return checkout


def main():
    """Exercise owned claim/gate or optional sealed-input exec on real source data.

    Publish the Root and ownership marker before selecting an ignored SDK test.
    The optional inert program is declared before claim, never substituted after
    authorization. Preserve compiler/test channels and failures; no result is
    published or accepted. Reject optimized Python before creating any fixture,
    because downstream public fixture oracles require active assertions. Optional
    duration/source cases use a publicly saved mission deadline or actual source
    mutation after exec; neither adds a global watchdog. Original stdout/stderr,
    interruption, wait, held funding and absence of publication are asserted. Current SDK sources are bound before
    Cargo, checked against actual workspace depfiles and archived with features;
    unused concurrent files do not masquerade as compiled input drift. The
    output-error case fails a test sink only after the exact SDK header, requires
    real provider invocation and retained wait/cause, and forbids successful parse.
    An optional explicitly manifested source-input fixture isolates compilation
    from other writers; its original bytes and actual compiled closure are still
    checked. It is not a second development checkout or a normal delivered build.
    --require-private-receipt refuses absence instead of inferring journal success
    from exit status; private data publication stays distinct from business state.
    --require-raw-capture additionally requires live capture in the private
    receipt. Optional --capture-policy uses public settings preview/apply/reopen,
    never edits SQL or invents a policy default. Null/zero stay explicit choices.
    --stream-handshake requires the complete fixture to wait for an actual pipe
    callback acknowledgement before exiting, proving streaming without sleeps.
    --publish-review exercises private candidate publication on this owned Root:
    source/reply/artifact faults and SQL rollback precede successful recording,
    one supervisor inbox event and replay without another call; no acceptance.
    Explicit --reconcile-failed instead checks non-verdict finalization of a
    waited failed original, keeping its estimate and never reusing a verdict.
    """
    if sys.flags.optimize:
        raise RuntimeError('review-candidate-oracle-requires-python-assertions')
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--reconcile-failed',action='store_true');p.add_argument('--publish-review',action='store_true');p.add_argument('--stream-handshake',action='store_true');p.add_argument('--require-raw-capture',action='store_true');p.add_argument('--capture-policy',type=json.loads);p.add_argument('--require-private-receipt',action='store_true');p.add_argument('--sdk-source-root',type=Path);p.add_argument('--controller-case',choices=('complete','duration','source','output-error','domain-invalid','changes-requested'),default='complete');p.add_argument('--final-reserved-call',action='store_true');p.add_argument('--execute-reviewer',action='store_true');p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=out/'binary';f.publish(binary,a.binary.resolve().read_bytes());binary.chmod(0o700)
    if a.reconcile_failed and (a.publish_review or not a.execute_reviewer or not a.require_raw_capture or a.controller_case not in ('duration','source','output-error','domain-invalid')):
        p.error('--reconcile-failed requires one explicit failed raw-captured exec and excludes --publish-review')
    if a.publish_review and not (a.execute_reviewer and a.require_raw_capture):
        p.error('--publish-review requires explicit owned exec with raw capture proof')
    if a.stream_handshake and (not a.require_raw_capture or a.controller_case!='complete'):
        p.error('--stream-handshake requires explicit raw-capture proof on the complete fixture')
    if a.require_raw_capture:
        a.require_private_receipt=True
    if a.capture_policy is not None and (not a.execute_reviewer or not isinstance(a.capture_policy,dict) or not a.capture_policy or set(a.capture_policy)-{'agent_capture_max_bytes','agent_capture_max_lines'}):
        p.error('--capture-policy requires owned exec and a nonempty capture-settings object')
    if a.require_private_receipt and not a.execute_reviewer:
        p.error('--require-private-receipt requires --execute-reviewer')
    if a.controller_case!='complete' and not a.execute_reviewer:
        p.error('--controller-case requires the explicitly owned --execute-reviewer fixture')
    root=None
    try:
        root,work,request=inputs.source(binary,out/'source')
        if a.capture_policy is not None:
            code,doc=inputs.native.invoke(binary,root,out,'capture-settings-show',['engine-settings','show']);assert code==0,doc
            values=doc['values'];values.update(a.capture_policy)
            change=dict(schema_version=doc['schema_version'],event_id='owned-review-capture-policy',expected_revision=doc['revision'],values=values,reason='Explicit owned reviewer capture verification',preview_token='')
            original=(root/'.swarm/engine-settings.json').read_bytes()
            code,prepared=inputs.native.invoke(binary,root,out,'capture-settings-preview',['engine-settings','preview'],change);assert code==0,prepared
            assert (root/'.swarm/engine-settings.json').read_bytes()==original
            change['preview_token']=prepared['preview_token']
            code,applied=inputs.native.invoke(binary,root,out,'capture-settings-apply',['engine-settings','apply'],change);assert code==0,applied
            code,reopened=inputs.native.invoke(binary,root,out,'capture-settings-reopen',['engine-settings','show']);assert code==0,reopened
            assert all(reopened['values'][key]==value for key,value in a.capture_policy.items())
        if a.final_reserved_call:
            inputs.quotas(binary,root,out,'single-review-cap',work,review_calls=1)
            request['expected_revision']=preview.shown(binary,root,out,'single-review-revision',work)['revision']
        if a.controller_case=='duration':
            # The duration is a ROOT request field, not a Go QuotaValues key.
            code,current=inputs.native.invoke(binary,root,out,'owned-review-duration-show',['quotas','show',work]);assert code==0,current
            code,changed=inputs.native.invoke(binary,root,out,'owned-review-duration',['quotas','apply',work],dict(schema_version=1,event_id='owned-review-duration',expected_revision=current['revision'],limits=current['limits'],review_timeout_seconds=1,reason='Exercise explicit saved reviewer duration'))
            assert code==0 and changed['planning']['reviewer']['timeout_seconds']==1,changed
            code,saved=inputs.native.invoke(binary,root,out,'owned-review-duration-reopen',['quotas','show',work])
            assert code==0 and saved['review_timeout_seconds']==1,saved
            request['expected_revision']=preview.shown(binary,root,out,'owned-review-duration-revision',work)['revision']
        request['event_id']='owned-review-claim';token=os.urandom(32).hex();f.publish(root/'.swarm/owned-review-claim-fixture',token.encode())
        manifest=out/'fixture.json';f.publish(manifest,dict(root=str(root),work=work,request=request,token=token,scope='Owned inert public fixture; exec only when explicitly requested',execute_reviewer=a.execute_reviewer,controller_case=a.controller_case,stream_handshake=a.stream_handshake,publish_review=a.publish_review,reconcile_failed=a.reconcile_failed))
        before=preview.shown(binary,root,out,'before-sdk',work);prompts={str(p):f.digest(p.read_bytes()) for p in root.glob('fixture-prompt-*.bin')}
        if a.execute_reviewer:
            # The inert executable is declared before claim. Its negative cases
            # block on a signal (not a verification watchdog); saved mission1s
            # or actual source revocation must stop the owned process.
            program=reviewer.PROGRAM.replace(b"reply=Path('fixture-review-reply.json').read_text()",b'''import signal
control=json.loads(Path('fixture-review-controller.json').read_text())
os.write(2,b'owned reviewer stderr\\n')
if control.get('stream_handshake'):
 import socket
 ack=socket.socket(socket.AF_UNIX,socket.SOCK_DGRAM);ack.bind('review-ack.sock')
if control['case']=='duration':
 signal.signal(signal.SIGTERM,signal.SIG_IGN)
 signal.pause()
elif control['case']=='source':
 Path(control['report']).write_text('Owned negative fixture: source changed after reviewer admission')
 signal.pause()
reply='PRIVATE invalid domain fixture' if control['case']=='domain-invalid' else Path('fixture-review-reply.json').read_text()
if control['case']=='changes-requested':
 changed=json.loads(reply);changed['criteria'][0]['verdict']='fail';reply=json.dumps(changed)''')
            assert program!=reviewer.PROGRAM
            program+=b"\nif control.get('stream_handshake'):\n data=ack.recv(64);assert data==b'streamed';ack.close();Path('owned-review-stream-handshake').write_bytes(data)\n"
            f.publish(out/'inert-reviewer.py',program)
            f.publish(root/'codex',program);(root/'codex').chmod(0o700)
            f.publish(root/'fixture-review-reply.json',dict(reason='Complete owned fixture review, not independent human acceptance',criteria=[dict(index=1,verdict='pass',evidence='Observed fixture only; not authenticated model or task acceptance.')]))
        f.publish(out/'reviewer-driver.py',Path(__file__).read_bytes())
        env={**os.environ,'CARGO_TARGET_DIR':str(Path('rust/target/r97').resolve()),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0','CODEX_HOME':str(root/'codex-home'),'SWARM_OWNED_REVIEW_CLAIM_FIXTURE':str(manifest)}
        selected='reviewer_execute_owned_public_fixture' if a.execute_reviewer else 'reviewer_claim_owned_public_fixture_rollback_and_replay'
        repository=Path(__file__).resolve().parents[1]
        checkout=sdk_source_inputs(repository,a.sdk_source_root)
        sys.path.insert(0,str(repository/'tools/verification'))
        import verify_native_profile as profile
        names=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']
        profile.require(not any(env.get(name) for name in names),'sdk-ambient-compiler-override')
        bound=profile.candidates(checkout)
        f.publish(out/'sdk.sources.before.json',bound)
        argv=['cargo','test','--manifest-path',str(checkout/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-engine','--lib','--message-format=json',selected,'--','--ignored','--nocapture']
        f.publish(out/'sdk.command.json',dict(argv=argv,fixture=str(manifest),profile='library test candidate; not production wiring',sdk_source_root=str(checkout),global_watchdog=None))
        with (out/'sdk.stdout').open('xb') as stdout,(out/'sdk.stderr').open('xb') as stderr:result=subprocess.run(argv,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        if result.returncode:
            original=AssertionError(f'SDK failed with status {result.returncode}; original compiler/test output retained')
            try:qualify_sdk(checkout,out,bound)
            except BaseException as secondary:
                raise BaseExceptionGroup('SDK failure and input qualification failure',[original,secondary])
            raise original
        qualify_sdk(checkout,out,bound)
        stdout=(out/'sdk.stdout').read_text();assert '1 passed; 0 failed' in stdout
        if a.execute_reviewer:
            assert 'REVIEW_CANDIDATE_SEALED_EXEC' in stdout
        else:
            assert 'REVIEW_CLAIM_CANDIDATE_ATOMIC_NO_EXEC' in stdout and 'REVIEW_GATE_ACTUAL_CHILD_NO_EXEC' in stdout
        published=(a.publish_review and a.controller_case in ('complete','changes-requested')) or a.reconcile_failed
        if a.publish_review:assert 'REVIEW_CANDIDATE_PUBLICATION_CHECKED' in stdout
        after=preview.shown(binary,root,out,'after-sdk',work);assert after['revision']==before['revision']+1+int(published)
        assert after['planning']['reviewer']['calls']==before['planning']['reviewer'].get('calls',0)+1
        expected_state='error' if a.reconcile_failed else ('changes_requested' if a.controller_case=='changes-requested' else 'passed') if published else 'running'
        assert after['tasks'][0]['status']==('blocked' if published and a.controller_case=='changes-requested' else 'submitted') and after['tasks'][0]['independent_review']['state']==expected_state
        with guards.database(root) as db:
            rows=db.execute('SELECT id,phase,permission,child_pid FROM native_review_jobs WHERE work_id=?',[work]).fetchall();assert len(rows)==1 and rows[0][1:3]==('refused' if a.reconcile_failed else 'recorded' if published else 'permitted',1) and rows[0][3]>0,rows
            record=db.execute('SELECT estimate,state FROM planning_calls WHERE id=?',[rows[0][0]]).fetchone();assert record and record[0]>0 and record[1]=='estimated',record
        assert (root/'reviewer-invoked.json').exists()==a.execute_reviewer
        if a.execute_reviewer:
            observed=json.loads((root/'owned-review-process-observation.json').read_text())
            if a.stream_handshake:
                assert observed.get('stream_handshake') is True and (root/'owned-review-stream-handshake').read_bytes()==b'streamed'
            assert observed['controller_case']==a.controller_case and observed['reaped'] and observed['channel_closed'] and observed['grant_attempted']
            assert observed['duration_expired']==(a.controller_case=='duration')
            assert observed['source_interrupted']==(a.controller_case=='source')
            assert observed['output_failed']==(a.controller_case=='output-error')
            assert observed['result_published'] is False
            if a.require_private_receipt:
                assert (root/'owned-review-private-receipt.json').is_file(),'required private receipt absent'
            if (root/'owned-review-private-receipt.json').exists():
                receipt=json.loads((root/'owned-review-private-receipt.json').read_text())
                assert receipt['waited'] and receipt['failed']==(a.controller_case not in ('complete','changes-requested')) and receipt['output_complete']==(a.controller_case in ('complete','changes-requested'))
                assert receipt['create_only_refused'] and receipt['corrupt_frame_refused'] and receipt['inconsistent_fields_refused'] and receipt['original_restored']
                assert receipt['business_result_published'] is False
            if a.require_raw_capture:
                assert receipt.get('raw_capture_streamed') is True and isinstance(receipt.get('raw_capture'),dict),'required streaming raw capture absent'
                raw=receipt['raw_capture']
                provider=(root/'owned-review-exec.stdout').read_bytes();header=b'\nrunning 1 test\n';assert provider.startswith(header);provider=provider[len(header):]
                stderr=(root/'owned-review-exec.stderr').read_bytes()
                assert raw['stdout']['observed_bytes']==len(provider) and raw['stdout']['sha256']==f.digest(provider)
                assert raw['stderr']['observed_bytes']==len(stderr) and raw['stderr']['sha256']==f.digest(stderr)
                if a.capture_policy is not None:
                    assert all(raw['policy'][key.removeprefix('agent_')]==value for key,value in a.capture_policy.items())
                assert raw['retained_bytes']==len(raw['stdout']['retained'])+len(raw['stderr']['retained'])
            if a.reconcile_failed:
                proof=json.loads((root/'owned-review-failure-reconciliation.json').read_text())
                assert proof['failure_recorded'] and proof['foreign_owner_refused'] and proof['foreign_owner_child_waited'] and not proof['verdict_applicable'] and not proof['task_accepted']
            if a.publish_review:
                publication=json.loads((root/'owned-review-publication.json').read_text())
                assert publication['business_result_published']==published and publication['task_accepted'] is False
                if published:assert publication['inbox_events']==1 and publication['sql_rollback_verified'] and publication['replay_without_call']
                if published:
                    context=json.loads((root/'owned-review-supervisor-context.json').read_text())
                    consumer=json.loads((root/'owned-review-consumer-faults.json').read_text())
                    assert any(review['id']==after['tasks'][0]['independent_review']['id'] for review in context['reviews'])
                    assert all(consumer[key] for key in ['context_materialized','original_report_present','public_changed_refused','private_changed_refused','report_changed_refused','forged_inbox_refused','work_unchanged'])
                    assert consumer['decision_acknowledged'] is False and consumer['model_received'] is False
            retain_process_observations(root,out)

        assert {str(p):f.digest(p.read_bytes()) for p in root.glob('fixture-prompt-*.bin')}==prompts
        f.publish(out/'result.json',dict(status='PASS',atomic_claim_candidate=True,actual_child_gate_candidate=True,rollback_recovery_duplicate_preserved=not a.execute_reviewer,real_public_worker_and_controls=True,reviewer_started=a.execute_reviewer,result_published=published,production_wiring=False,paid_provider=False,library_tests=1,selected_parent_tests=1,owned_child_test_processes=1+int(a.reconcile_failed),owned_child_test_processes_scope="SDK gate child and optional foreign-owner helper; excludes public fixture setup",child_replaced_by_exec=a.execute_reviewer,controller_case=a.controller_case,private_receipt_written=(root/'owned-review-private-receipt.json').exists(),streaming_capture_required=a.require_raw_capture,explicit_capture_policy=a.capture_policy,business_result_published=published,failure_reconciled=a.reconcile_failed,verdict_applicable=published and not a.reconcile_failed,binary_sha256=f.digest(binary.read_bytes()),root=str(root),global_watchdog=None))
        print('PASS candidate reviewer failed call reconciliation' if a.reconcile_failed else 'PASS candidate reviewer publication checks' if a.publish_review else 'PASS candidate reviewer sealed exec without result publication' if a.execute_reviewer else 'PASS candidate reviewer atomic SDK claim without execution',flush=True)
    except BaseException as error:
        original_trace=traceback.format_exc();retention_error=None
        # Keep original fault witnesses even when an SDK assertion fails. Only
        # the already token-verified owned process evidence is eligible.
        required=['owned-review-process-observation.json','owned-review-exec.stdout','owned-review-exec.stderr','reviewer-invoked.json']
        if root is not None and (out/'fixture.json').exists() and all((root/name).exists() for name in required):
            try:retain_process_observations(root,out)
            except BaseException as secondary:retention_error=secondary
        f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=original_trace,retention_error_type=type(retention_error).__name__ if retention_error else None))
        if retention_error:raise BaseExceptionGroup('SDK failure and evidence retention failure',[error,retention_error])
        raise


if __name__=='__main__':main()
