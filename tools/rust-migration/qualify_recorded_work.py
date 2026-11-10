"""Qualify caller-owned SQL readers; no live roots, providers or public mutation."""
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
from recorded_work_vectors import vectors


def prepare(root,output,run,save):
    folder=Path(tempfile.mkdtemp(prefix='swarm-r3e-private-',dir=root/'.cache'))
    source=run('reader-reference-source',['go','run','./tools/rust-migration/work-row-reference'])
    path=folder/'main.go';path.write_bytes(source)
    save('reader-private-fixture.json',dict(root=str(folder),source_sha256=hashlib.sha256(source).hexdigest(),generated_source=str(path)))
    return folder


def qualify(root,output,work,env,run,execute,save,prepared,examples):
    source=run('reader-reference-source-check',['go','run','./tools/rust-migration/work-row-reference'])
    assert source==(prepared/'main.go').read_bytes()
    reference=work/'go-read-reference-bin'
    run('reader-reference-build',['go','build','-o',str(reference),str(prepared/'main.go')])
    values=vectors();raw=b''.join(json.dumps(v,separators=(',',':')).encode()+b'\n'for v in values)
    go=[json.loads(l)for l in run('sql-reader-corpus-go',[str(reference)],raw).splitlines()]
    native=[json.loads(l)for l in run('sql-reader-corpus-rust',[str(examples/'recorded_work_probe')],raw).splitlines()]
    assert len(go)==len(native)==len(values)and values
    differences=[dict(input=v,go=g,rust=r)for v,g,r in zip(values,go,native)if g!=r];save('sql-reader-differences.json',differences);assert not differences
    assert all(v['code']=='ok'for i,v in zip(values,go)if i.get('control'))
    cli=work/'go-cli-bin';setter=work/'go-row-setter-bin'
    assert cli.is_file()and setter.is_file() # Built by the v2 fixture checks.
    fixture=work/'public-work-reader-fixture';fixture.mkdir(mode=0o700)
    run('reader-public-init',[str(cli),'--root',str(fixture),'--json','init'])
    (fixture/'.rust-migration-fixture').write_text('Owned isolated native reader fixture.\n')
    database=fixture/'.swarm/state.db'
    def digest():
        with sqlite3.connect(database)as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    body=b'{"schema_version":1,"id":"legacy","revision":3,"title":"Public reader comparison","tasks":[]}'
    request=json.dumps(dict(raw=body.hex(),revision=3)).encode()
    run('reader-public-valid-row',[str(setter),str(fixture)],request)
    before=digest();public=[]
    for operation,go_op in [('read','show'),('list','list')]:
        args=[str(cli),'--root',str(fixture),'--json','work',go_op]+(['legacy']if operation=='read'else[])
        actual=json.loads(run('reader-public-go-'+operation,args))
        native=json.loads(run('reader-public-native-'+operation,[str(examples/'recorded_work_probe'),str(fixture),operation]))
        expected=actual['work']if operation=='read'else actual
        assert expected==native and digest()==before
        assert (native if operation=='read'else native[0])['title']=='Public reader comparison'
        public.append(dict(operation=operation,unchanged_database=True,sha256=before))
    # Original Go CLI ignores printJSON failures in work.list. Keep the anomaly,
    # never count it as a passing output-error parity check or a live Go repair.
    with open('/dev/full','wb')as sink:
        p=subprocess.run([str(cli),'--root',str(fixture),'--json','work','list'],env=env,stdout=sink,stderr=subprocess.PIPE)
    (output/'go-list-output-full.stderr').write_bytes(p.stderr)
    save('go-list-output-full.json',dict(exit=p.returncode,status='OPEN'if p.returncode==0 else'REJECTED',origin='internal/engine/main.go: work.list ignores printJSON error',live_go_repaired=False))
    assert (p.returncode==0 or p.stderr) and digest()==before
    with open('/dev/full','wb')as sink:
        n=subprocess.run([str(examples/'recorded_work_probe'),str(fixture),'list'],env=env,stdout=sink,stderr=subprocess.PIPE)
    (output/'native-reader-output-full.stderr').write_bytes(n.stderr)
    assert n.returncode!=0 and n.stderr and digest()==before
    save('reader-public-flow.json',dict(status='PASS',operations=public,native_fixture_output_failure_refused=True,go_output_failure='OPEN'if p.returncode==0 else'PASS',native_user_work_command=False))
    for name,binary in [('go-read-reference',reference),('native-sql-reader',examples/'recorded_work_probe')]:
        target=output/(name+'.binary');shutil.copyfile(binary,target);target.chmod(0o600)
    return dict(status='PASS',sql_cases=len(values),controls=sum(v.get('control',False)for v in values),accepted=sum(v['code']=='ok'for v in go),public_comparisons=len(public),go_output_failure='OPEN'if p.returncode==0 else'PASS',native_fixture_output_failure_refused=True,native_user_command=False,connection_or_transaction_owned_by_caller=True)
