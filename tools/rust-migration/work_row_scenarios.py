"""Compare the real Go work-show reader with the native row codec on owned data."""
import hashlib
import json
import sqlite3
import subprocess


def run(go_cli, rust_probe, setter, work, output, env):
    root=work/'public-row-fixture';root.mkdir(mode=0o700)
    init=subprocess.run([str(go_cli),'--root',str(root),'--json','init'],env=env,capture_output=True)
    (output/'row-init.stdout').write_bytes(init.stdout);(output/'row-init.stderr').write_bytes(init.stderr)
    assert init.returncode==0, 'Go public fixture init failed'
    (root/'.rust-migration-fixture').write_text('Owned isolated fault fixture; never a live mission.\n')
    path=root/'.swarm/state.db'
    valid=b'{"schema_version":1,"id":"legacy","revision":3,"title":"Historical source","tasks":[],"future_optional":{"value":"retained"}}'
    records=[]
    def dump():
        with sqlite3.connect(path)as db:return hashlib.sha256('\n'.join(db.iterdump()).encode()).hexdigest()
    def apply(name,raw,revision=3):
        before=dump()
        p=subprocess.run([str(setter),str(root)],env=env,input=json.dumps(dict(raw=raw.hex(),revision=revision)).encode(),capture_output=True)
        (output/(name+'-fixture.stdout')).write_bytes(p.stdout);(output/(name+'-fixture.stderr')).write_bytes(p.stderr)
        if p.returncode:
            assert name in ['null','bad-tail'] and p.returncode==2 and before==dump(), 'unexpected fixture write refusal'
        return p.returncode==0
    def check(name,raw,revision=3,key='legacy',ok=True):
        stored=apply(name,raw,revision);before=dump()
        if not stored:
            query=dict(name=name,model='Work',row_id=key,revision=revision,raw=raw.hex())
            native=subprocess.run([str(rust_probe)],env=env,input=(json.dumps(query)+'\n').encode(),capture_output=True)
            (output/(name+'-rust.stdout')).write_bytes(native.stdout);(output/(name+'-rust.stderr')).write_bytes(native.stderr)
            assert native.returncode==0 and json.loads(native.stdout)['code']=='format' and not json.loads(native.stdout)['output']
            records.append(dict(name=name,accepted=False,stage='real-Go-trigger-refused-fixture; native-codec-refused',public_row_read=False,unchanged_database=True))
            return
        p=subprocess.run([str(go_cli),'--root',str(root),'--json','work','show',key],env=env,capture_output=True)
        query=dict(name=name,model='Work',row_id=key,revision=revision,raw=raw.hex())
        native=subprocess.run([str(rust_probe)],env=env,input=(json.dumps(query)+'\n').encode(),capture_output=True)
        for label,proc in [('go',p),('rust',native)]:
            (output/(name+'-'+label+'.stdout')).write_bytes(proc.stdout)
            (output/(name+'-'+label+'.stderr')).write_bytes(proc.stderr)
        assert native.returncode==0,(name,'native harness failure')
        result=json.loads(native.stdout)
        assert (p.returncode==0)==ok and (result['code']=='ok')==ok,(name,'acceptance mismatch')
        if ok:
            envelope=json.loads(p.stdout)
            assert set(envelope)=={'work','validation','events','current_gates','current_git','resume_markdown'},name+': unexpected Go public envelope'
            assert envelope['events']==[] and envelope['validation']['state']=='open' and 'Historical source' in envelope['resume_markdown'],name
            go_value=envelope['work'];rust_value=json.loads(bytes.fromhex(result['output']))
            assert go_value==rust_value and go_value['id']=='legacy' and go_value['revision']==3,name
            assert go_value['title']=='Historical source',name
        else:
            assert p.returncode==2 and not p.stdout and p.stderr and not result['output'],name+': partial result'
        after=dump();assert before==after,name+': inspection mutated fixture'
        records.append(dict(name=name,accepted=ok,go_exit=p.returncode,public_row_read=True,unchanged_database=True,before_sha256=before,after_sha256=after))
    check('valid-legacy',valid)
    variants=[('wrong-id',valid.replace(b'"id":"legacy"',b'"id":"other"'),3,'legacy'),
        ('wrong-schema',valid.replace(b'"schema_version":1',b'"schema_version":2'),3,'legacy'),
        ('wrong-revision',valid,4,'legacy'),('zero-revision',valid,0,'legacy'),
        ('negative-revision',valid,-1,'legacy'),('null',b'null',3,'legacy'),
        ('invalid-utf8',valid.replace(b'Historical',b'\xff'),3,'legacy'),
        ('invalid-type',valid.replace(b'"revision":3',b'"revision":false'),3,'legacy'),
        ('bad-tail',valid+b'{}',3,'legacy'),('unsafe-key',valid,3,'../legacy'),
        ('missing-body-id',valid.replace(b'"id":"legacy",',b''),3,'legacy'),
        ('negative-zero-version',valid.replace(b'"schema_version":1',b'"schema_version":-0'),3,'legacy')]
    for name,raw,revision,key in variants:
        check('valid-before-'+name,valid)
        check(name,raw,revision,key,False)
        check('recovered-after-'+name,valid)
    return dict(status='PASS',public_go_cli=True,native_row_codec=True,native_sql_reader=False,
        operations=len(records),public_row_reads=sum(v['public_row_read']for v in records),write_boundary_refusals=sum(not v['public_row_read']for v in records),cases=records,no_partial_results=True,inspection_read_only=True,fixture_root=str(root))
