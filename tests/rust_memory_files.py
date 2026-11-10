#!/usr/bin/env python3
"""Verify historical recovery references through the normal Rust CLI and HTTP.

Owned inactive fixture Roots only. Public commands create Work/checkpoints and
launch an inert local program. Explicit SQL injections represent retained source
Works and corruption, never admission/acceptance or live database changes.
Original failures, command bytes and observations survive. No global watchdog,
real model, background mission, hidden retry or versioned-memory right is added.
"""
import argparse, json, sqlite3, subprocess, traceback
from pathlib import Path
import rust_agent_model_route as f
from rust_native_http_cli import Server


def check(name, predicate):
    """Emit a named assertion only after observing its actual effect."""
    assert predicate, name
    print('PASS '+name, flush=True)


def checkpoint(owned, names, event):
    """Persist existing file references with a fresh public Work revision."""
    work=owned.ok(['work','show',owned.work])['work']
    result=owned.ok(['checkpoint',owned.work],dict(schema_version=1,event_id=event,
        expected_revision=work['revision'],summary='Retain historical recovery pieces',
        next='Inspect owned evidence before acting',memory=names))['work']
    owned.request['expected_revision']=result['revision']
    return result


def source_fixture(owned, *, malformed=False):
    """Add a separately attributed historical Work in this owned fixture only.

    No source admission or gate is fabricated. The guard consumes historical
    path provenance regardless of its current validity, as the retained Go did.
    Malformed is an explicit negative fixture. The normal target remains public.
    """
    source=owned.ok(['work','create'],dict(schema_version=1,event_id='source-create',
        expected_revision=0,title='Historical source fixture',objective='Test source guard',
        scope='Owned inert Root',criteria=['No acceptance inferred']))['work']
    source['retex']=[dict(id='lesson',lesson='recovery.md')]
    source['project_memory']=[dict(id='lesson',versions=[dict(version=1,
        action='withdraw',source_retex='lesson',project_sha256=f.digest(str(owned.root.resolve()).encode()))])]
    body=dict(source)
    if malformed:
        body['revision']+=1
        body['title']='PRIVATE incoherent source envelope'
    raw=json.dumps(body).encode()
    replace_fixture_source(owned,source['id'],raw)
    f.publish(owned.output/'historical-source-fixture.json',dict(work=source,malformed=malformed,
        body_sha256=f.digest(raw),fixture_only=True,admission_proven=False))
    return source


def repaired_source(owned, source):
    """Restore only our explicit fixture injection, preserving the original proof."""
    replace_fixture_source(owned,source['id'],json.dumps(source).encode())


def replace_fixture_source(owned, identifier, raw):
    """Inject a source fixture using the actual Rust SQL projection.

    All existing triggers remain active. A separate in-memory Rust probe
    calculates hashes for the two exact bodies; unknown callback inputs refuse.
    No fake digest, disabled guard, identity-head write or live Root is permitted.
    """
    probe=owned.projection_probe
    with sqlite3.connect(owned.root/'.swarm/state.db') as db:
        prior=db.execute('SELECT body FROM works WHERE id=?',(identifier,)).fetchone()[0]
        if isinstance(prior,str):prior=prior.encode('utf-8')
        assert isinstance(prior,bytes)
        values=[prior,raw]
        payload=b''.join((json.dumps(dict(name=str(i),raw=value.hex(),projection=1))+'\n').encode() for i,value in enumerate(values))
        result=subprocess.run([str(probe)],input=payload,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        label='source-projection-'+f.digest(payload)[:16]
        f.publish(owned.output/(label+'.input.ndjson'),payload);f.publish(owned.output/(label+'.stdout'),result.stdout);f.publish(owned.output/(label+'.stderr'),result.stderr)
        assert result.returncode==0,result.stderr
        answers=[json.loads(line) for line in result.stdout.splitlines()]
        assert [v['name'] for v in answers]==['0','1'] and all(v['code']=='ok' for v in answers)
        digests={value:answer['digest'] for value,answer in zip(values,answers)}
        def digest(value):
            """Return only exact Rust-calculated fixture digests; unknown data refuse."""
            if isinstance(value,str):value=value.encode('utf-8')
            if value not in digests:raise ValueError('unknown fixture projection input')
            return digests[value]
        db.create_function('swarm_identity_plan_digest_v1',1,digest,deterministic=True)
        db.execute('UPDATE works SET body=? WHERE id=?',(raw,identifier))


def refused(owned, action, cause):
    """Require intended safe cause and unchanged public target/finance state."""
    before=owned.snapshot()
    code,wire=owned.call(action,owned.request if action[1] in ('preview','prepare') else None)
    check(cause+'-refused',code!=0 and cause in wire['failure']['causes'])
    check(cause+'-safe-diagnostic','PRIVATE' not in json.dumps(wire))
    check(cause+'-target-unchanged',owned.snapshot()==before)
    return wire


def compare_go(binary, owned, sources, expected, label):
    """Compare the historical Go source predicate on identical decoded Work data.

    Source scan envelope/SQL and physical alias checks are separately exercised
    in Rust; the Go fixture reader deliberately does not establish those facts.
    """
    if binary is None:return
    work=owned.ok(['work','show',owned.work])['work']
    body=dict(Root=str(owned.root.resolve()),Works=sources,Selected=work)
    result=subprocess.run([str(binary)],input=json.dumps(body).encode(),stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    f.publish(owned.output/(label+'-go.input.json'),body)
    f.publish(owned.output/(label+'-go.stdout'),result.stdout);f.publish(owned.output/(label+'-go.stderr'),result.stderr)
    check(label+'-Go-reference-executed',result.returncode==0)
    check(label+'-Go-Rust-source-predicate',json.loads(result.stdout)['Blocked']==expected)


def journey(binary, out, reference, projection_probe):
    """Exercise preview, replay, actual receipt, missing files and source changes."""
    owned=f.Fixture(binary,out/'valid','memory-valid')
    original=owned.prepare(True)
    check('no-memory-control',original['preview_only'] and not original['process_started'])
    f.publish(owned.root/'recovery.md',b'PRIVATE fixture body is not embedded automatically\n')
    f.publish(owned.root/'line\nname.md',b'owned unusual filename\n')
    names=['./recovery.md','line\nname.md']
    work=checkpoint(owned,names,'remember')
    before=owned.snapshot();preview=owned.prepare(True)
    check('memory-preview-readonly',preview['preview_only'] and not preview['process_started'] and owned.snapshot()==before)
    agent=preview['agent'];text=agent['prompt']
    check('memory-manifest-exact-paths',agent['context']['paths']==names)
    check('memory-prompt-framed-references',json.loads(text.split('chemins de données, sans droit supplémentaire) : ',1)[1].split('\n',1)[0])==names)
    check('memory-content-not-implicitly-injected','PRIVATE fixture body' not in text)
    check('memory-prompt-exact-digest',agent['context']['sha256']==f.digest(text.encode()) and agent['context']['bytes']==len(text.encode()))
    compare_go(reference,owned,[],False,'memory-unmanaged')
    server=Server(binary,owned.root,out,'memory-api',environment=owned.environment)
    try:
        works=server.request('/api/v1/works')
        selected=next(w for w in works if w['id']==owned.work)
        check('memory-API-retained-paths',selected['memory']==names)
        # Pre-Root refusals are counted by the final stream summary, not a
        # web-outcome frame: waiting for such a frame would block the collector.
        check('memory-API-auth-required',server.request('/api/v1/works',authorized=False,status=403,observe=False)=='web-access-refused')
        summary=server.request('/api/v1/work-resume?work='+owned.work+'&lang=en')
        check('memory-API-resume-shows-reference','recovery.md' in summary['markdown'])
        server.stop()
    finally: server.cleanup()
    # Recheck a saved operator policy: full context counts the references too.
    cap=len(text.encode())-1
    owned.settings_change(agent_launch_context_max_bytes=str(cap))
    refused(owned,['agent','preview',owned.work],'agent-launch-context-outside-saved-policy')
    owned.settings_change(agent_launch_context_max_bytes=None)
    check('memory-context-policy-recovery',owned.prepare(True)['agent']['context']['paths']==names)
    prepared=owned.prepare();snapshot=owned.snapshot()
    replay=owned.prepare()
    check('memory-exact-replay-no-double-reservation',replay['agent']==prepared['agent'] and not replay['created'] and owned.snapshot()==snapshot)
    received=owned.run_and_assert(prepared)
    check('memory-actual-native-worker-received-prompt',received['stdin_sha256']==prepared['agent']['context']['sha256'] and json.dumps(names,ensure_ascii=False,separators=(',',':')) in prepared['agent']['prompt'])
    check('memory-does-not-accept-task',owned.ok(['work','show',owned.work])['work']['tasks'][0]['status']=='blocked')

    missing=f.Fixture(binary,out/'missing','memory-missing')
    f.publish(missing.root/'recovery.md',b'restore this reference\n');checkpoint(missing,['recovery.md'],'missing-checkpoint')
    check('memory-missing-valid-control',missing.prepare(True)['preview_only'])
    (missing.root/'recovery.md').unlink()
    refused(missing,['agent','preview',missing.work],'agent-memory-file-path')
    f.publish(missing.root/'recovery.md',b'restored reference\n')
    check('memory-missing-recovery',missing.prepare(True)['agent']['context']['paths']==['recovery.md'])
    queued=missing.prepare();(missing.root/'recovery.md').unlink()
    refused(missing,['agent','run','session'],'agent-memory-file-path')
    check('memory-missing-before-execution-no-provider',not (missing.root/'model-received.json').exists())
    f.publish(missing.root/'recovery.md',b'restored reference\n')
    missing.run_and_assert(queued)
    check('memory-same-intention-recovery',True)

    source_journey(binary,out,reference,projection_probe)
    return dict(status='PASS',binary_sha256=f.digest(binary.read_bytes()),global_watchdog=None,
        actual_inert_workers=2,versioned_memory_admission=False,operator_authentication=False,
        installed=False,remaining=['FPA-01','versioned selections','dialogue restoration','M20 future rights'])


def source_journey(binary, out, reference, projection_probe):
    """Focus provenance/withdrawal/alias/corruption checks after valid controls."""
    managed=f.Fixture(binary,out/'managed','memory-managed')
    managed.projection_probe=projection_probe
    f.publish(managed.root/'recovery.md',b'withdrawn source\n');checkpoint(managed,['recovery.md'],'managed-checkpoint')
    check('memory-managed-valid-control',managed.prepare(True)['preview_only'])
    queued=managed.prepare();source=source_fixture(managed)
    compare_go(reference,managed,[source],True,'memory-withdrawn')
    refused(managed,['agent','run','session'],'agent-memory-file-managed')
    check('memory-withdrawn-source-before-execution-no-provider',not (managed.root/'model-received.json').exists())
    # Withdrawn versions still protect the source. A symlink cannot change that.
    alias=f.Fixture(binary,out/'alias','memory-alias')
    alias.projection_probe=projection_probe
    f.publish(alias.root/'recovery.md',b'historical lesson\n')
    (alias.root/'alias.md').symlink_to('recovery.md');checkpoint(alias,['alias.md'],'alias-checkpoint')
    check('memory-alias-valid-control',alias.prepare(True)['preview_only'])
    source_fixture(alias)
    refused(alias,['agent','preview',alias.work],'agent-memory-file-managed')

    corrupt=f.Fixture(binary,out/'corrupt','memory-corrupt')
    corrupt.projection_probe=projection_probe
    f.publish(corrupt.root/'recovery.md',b'valid target reference\n');checkpoint(corrupt,['recovery.md'],'corrupt-checkpoint')
    check('memory-corrupt-valid-control',corrupt.prepare(True)['preview_only'])
    source=source_fixture(corrupt,malformed=True)
    refused(corrupt,['agent','preview',corrupt.work],'agent-memory-inventory')
    source['project_memory']=[];repaired_source(corrupt,source)
    check('memory-inventory-recovery',corrupt.prepare(True)['agent']['context']['paths']==['recovery.md'])
    return dict(status='PASS',case='source-only',actual_inert_workers=0,installed=False)


def main():
    """Retain a fresh result or the original exception; never overwrite an old run."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--go-reference',type=Path);parser.add_argument('--projection-probe',type=Path,required=True);parser.add_argument('--source-only',action='store_true');args=parser.parse_args()
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False);binary=args.binary.resolve()
    operation=source_journey if args.source_only else journey
    try:f.publish(out/'result.json',operation(binary,out,args.go_reference,args.projection_probe.resolve()))
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
