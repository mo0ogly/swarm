"""Source-bound SQL event qualification; caller-owned readers, not full Store."""
from collections import Counter
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
from qualification_inputs import collect
from qualify_recorded_work import prepare as prepare_work, qualify as qualify_work
from recorded_event_vectors import vectors
from verify_recorded_work import verify_reuse_environment
ROOT=Path(__file__).resolve().parents[2]

def main():
    output=Path(sys.argv[1]).resolve()
    if output.exists()and any(output.iterdir()):raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True,exist_ok=True);(ROOT/'.cache').mkdir(exist_ok=True)
    work=Path(tempfile.mkdtemp(prefix='swarm-event-read-',dir=ROOT/'.cache'));scratch=work/'tmp';scratch.mkdir(mode=0o700)
    env=dict(os.environ,TMPDIR=str(scratch),GOTMPDIR=str(scratch),CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(work/'target')))
    commands=[]
    def save(name,value):
        p=output/name;fd,tmp=tempfile.mkstemp(dir=p.parent,prefix='.'+p.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,p)
    def execute(label,args,data=None,expected=0):
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
        p=subprocess.run(args,cwd=ROOT,env=env,input=data,capture_output=True)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        for extension,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+extension)).write_bytes(raw)
        commands.append(dict(label=label,command=args,start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands)
        print(label,'exit',p.returncode,flush=True)
        if expected is not None and p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p
    def run(label,args,data=None):return execute(label,args,data).stdout
    try:
        prepared=prepare_work(ROOT,output,run,save)
        event=Path(tempfile.mkdtemp(prefix='swarm-r3f-private-',dir=ROOT/'.cache'))
        reference_source=run('event-reference-source',['go','run','./tools/rust-migration/event-reference']);(event/'main.go').write_bytes(reference_source)
        packages=['./cmd/swarm','./internal/engine','./tools/rust-migration/work-row-reference','./tools/rust-migration/work-row-fixture','./tools/rust-migration/event-reference','./'+str(prepared.relative_to(ROOT)),'./'+str(event.relative_to(ROOT))]
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,packages,allow_root=True)
        baseline=ROOT/'docs/plans/migration-rust/evidence/20261018-02/qualified-01';old=json.loads((baseline/'inputs-before.json').read_text())
        verify_reuse_environment(baseline,output,old,before)
        def go_inputs(values):return{k:v for k,v in values.items()if not k.startswith(('rust/','tools/rust-migration/','.cache/'))}
        assert go_inputs(old)==go_inputs(before),'Go compiled/root/embed inputs changed'
        shared=[k for k in old if k.startswith('rust/crates/')and not k.startswith('rust/crates/swarm-engine/')]
        assert all(old[k]==before[k]for k in shared),'leaf inputs changed'
        codecs=[k for k in old if k.startswith('rust/crates/swarm-engine/src/')and Path(k).name in ['models.rs','model_wire.rs','work.rs','request.rs','identity_plan.rs','identity_plan_rules.rs','identity_plan_upgrade.rs','identity_plan_heads.sql','identity_plan_upgrade.sql','identity_plan_upgrade_finish.sql']]
        assert codecs and all(old[k]==before[k]for k in codecs),'codec or identity inputs changed'
        save('reused-proof.json',dict(baseline=str(baseline.relative_to(ROOT)),go_inputs=len(go_inputs(before)),leaf_inputs=len(shared),engine_contracts=codecs,limits='Identical codec/identity/leaf inputs only; changed Work SQL scanner requalified'))
        run('format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
        tests={}
        for label,flags in [('engine-all',['--all-features']),('engine-default',[])]:
            args=['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine',*flags,'--all-targets']
            raw=run(label,args).decode();listed=run(label+'-inventory',args+['--','--list']).decode()
            actual=Counter(l.removeprefix('test ').removesuffix(' ... ok')for l in raw.splitlines()if l.startswith('test ')and l.endswith(' ... ok'))
            expected=Counter(l.removesuffix(': test')for l in listed.splitlines()if l.endswith(': test'));assert actual and actual==expected;tests[label]=list(actual.elements())
        run('workspace-compile',['cargo','check','--manifest-path','rust/Cargo.toml','--locked','--offline','--workspace','--all-features','--all-targets'])
        run('engine-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--all-features','--all-targets','--','-D','warnings'])
        run('go-vet',['go','vet',*packages])
        for name in ['recorded_work_probe','recorded_event_probe']:
            run(name+'-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-engine','--features','test-fixtures','--example',name])
        run('fixture-cli-build',['go','build','-o',str(work/'go-cli-bin'),'./cmd/swarm'])
        run('fixture-setter-build',['go','build','-o',str(work/'go-row-setter-bin'),'./tools/rust-migration/work-row-fixture'])
        examples=Path(env['CARGO_TARGET_DIR'])/'debug/examples'
        work_result=qualify_work(ROOT,output,work,env,run,execute,save,prepared,examples)
        assert run('event-reference-source-check',['go','run','./tools/rust-migration/event-reference'])==reference_source
        reference=work/'go-event-reference';run('event-reference-build',['go','build','-o',str(reference),str(event/'main.go')])
        values=vectors();data=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in values)
        go=[json.loads(l)for l in run('event-corpus-go',[str(reference)],data).splitlines()]
        native=[json.loads(l)for l in run('event-corpus-rust',[str(examples/'recorded_event_probe')],data).splitlines()]
        assert len(go)==len(native)==len(values)and values
        differences=[dict(input=v,go=g,rust=r)for v,g,r in zip(values,go,native)if g!=r];save('event-differences.json',differences);assert not differences
        assert all(v['code']=='ok'and v['json_code']=='ok'for i,v in zip(values,go)if i['control'])
        fixture=work/'public-event-fixture';fixture.mkdir(mode=0o700)
        cli=work/'go-cli-bin';setter=work/'go-row-setter-bin';probe=examples/'recorded_event_probe'
        run('event-public-init',[str(cli),'--root',str(fixture),'--json','init']);(fixture/'.rust-migration-fixture').write_text('Owned isolated event fixture.\n')
        body=b'{"schema_version":1,"id":"legacy","revision":3,"title":"Event migration","tasks":[]}'
        run('event-public-work',[str(setter),str(fixture)],json.dumps(dict(raw=body.hex(),revision=3)).encode())
        database=fixture/'.swarm/state.db'
        with sqlite3.connect(database)as db:
            db.execute('PRAGMA foreign_keys=ON');assert db.execute('PRAGMA foreign_keys').fetchone()==(1,)
            for revision in [2,1]:db.execute('INSERT INTO events VALUES(?,?,?,?,?,?,?)',(f'event-{revision}','legacy',revision,'change','2026-10-18T00:00:00Z',b'{ "text":"<>&", "n":1e3 }',b'{}'))
            assert db.execute('PRAGMA foreign_key_check').fetchall()==[]
        def digest():
            with sqlite3.connect(database)as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
        def public(label):return execute(label,[str(cli),'--root',str(fixture),'--json','work','show','legacy'])
        def reader(label,expected=0):return execute(label,[str(probe),str(fixture),'legacy'],expected=expected)
        stamp=digest();public_events=json.loads(public('event-public-go-valid').stdout)['events'];native_events=json.loads(reader('event-public-rust-valid').stdout)
        assert public_events==native_events and [e['revision']for e in native_events]==[1,2] and native_events[0]['payload']['text']=='<>&' and digest()==stamp
        with open('/dev/full','wb')as sink:n=subprocess.run([str(probe),str(fixture),'legacy'],env=env,stdout=sink,stderr=subprocess.PIPE)
        (output/'event-native-full.stderr').write_bytes(n.stderr);assert n.returncode!=0 and n.stderr and digest()==stamp
        with sqlite3.connect(database)as db:db.execute('UPDATE events SET payload=? WHERE revision=2',(b'PRIVATE invalid payload',))
        bad_stamp=digest();g=public('event-public-go-invalid');assert g.returncode==0 and not g.stdout and not g.stderr
        n=reader('event-public-rust-invalid',expected=None);assert n.returncode!=0 and n.stderr and b'PRIVATE'not in n.stderr and digest()==bad_stamp
        with sqlite3.connect(database)as db:db.execute('UPDATE events SET payload=? WHERE revision=2',(b'{"recovered":true}',))
        recovered=digest();assert json.loads(public('event-public-go-recovery').stdout)['events']==json.loads(reader('event-public-rust-recovery').stdout) and digest()==recovered
        public_report=dict(status='PASS',valid_and_recovery_comparisons=2,unchanged_on_reads=True,native_output_error_rejected=True,native_payload_error_rejected=True,go_invalid_payload_output='OPEN: exit 0 with empty stdout/stderr',go_live_repaired=False,full_store=False,native_public_command=False)
        save('event-public-flow.json',public_report)
        for name,binary in [('go-event-reference',reference),('native-event-reader',probe)]:
            target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after)
        report=dict(status='PARTIAL',native_component_status='PASS',scope='R3f caller-owned SQL event reads/encoding and R3e shared-scanner regression',event_cases=len(values),valid_controls=sum(v['control']for v in values),accepted_reads=sum(v['code']=='ok'for v in go),accepted_json=sum(v['json_code']=='ok'for v in go),tests=tests,work_regression=work_result,public_flow=public_report,source_stable=True,inputs=len(before),registry_packages=len(registry),fixture_root=str(work),scratch_device=os.stat(work).st_dev,scratch_free_bytes=shutil.disk_usage(work).free,global_watchdog=None,full_store=False,deployment=False,independent_review=False)
        save('result.json',report);print(json.dumps({k:report[k]for k in ['status','native_component_status','event_cases','inputs']}))
    except Exception as error:save('failure.json',dict(type=type(error).__name__,cause=str(error),completed_commands=len(commands)));raise
if __name__=='__main__':main()
