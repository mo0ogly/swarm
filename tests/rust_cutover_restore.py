"""Cold owned Root data restore through qualified native CLI; no live processes.

Corpus: public init/Work/task/budget/settings and an explicitly inert private file.
No agents, scopes, provider calls or existing physical grants. This is not Go
parity, active backup, historical-schema migration or global R6 acceptance.
"""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,sqlite3,subprocess,tempfile
spec=importlib.util.spec_from_file_location('cutover',Path(__file__).resolve().parents[1]/'tools/rust-migration/cutover_snapshot.py')
snapshot=importlib.util.module_from_spec(spec);spec.loader.exec_module(snapshot)


def tables(root):
    """Read exact typed rows/schema/user_version plus integrity in an idle fixture.

    SQLite is opened read-only with zero busy wait (a nonblocking observation,
    not a test watchdog). Retain primary and close causes; never write authority
    rows, normalize provenance or conceal BLOBs/numbers via string conversion.
    """
    db=sqlite3.connect((root/'.swarm/state.db').as_uri()+'?mode=ro',uri=True,timeout=0)
    error=None;result=None
    def typed(value):
        if isinstance(value,bytes):return dict(type='blob',hex=value.hex())
        return dict(type=type(value).__name__,value=value)
    try:
        assert db.execute('PRAGMA integrity_check').fetchall()==[('ok',)]
        assert db.execute('PRAGMA foreign_key_check').fetchall()==[]
        schema=db.execute('SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name').fetchall();values={}
        for kind,name,_,_ in schema:
            if kind!='table':continue
            quoted='"'+name.replace('"','""')+'"'
            rows=[[typed(v) for v in row] for row in db.execute('SELECT * FROM '+quoted)]
            values[name]=sorted(rows,key=lambda row:json.dumps(row,sort_keys=True))
        result=dict(user_version=db.execute('PRAGMA user_version').fetchone()[0],schema=schema,rows=values)
    except BaseException as cause:error=cause
    try:db.close()
    except BaseException as cause:
        if error is not None:raise BaseExceptionGroup('fixture SQL read and close',[error,cause])
        raise
    if error is not None:raise error
    return result


def run(binary,profile,out):
    """Public create→capture→mutate→fresh restore→compare→new public write.

    Verify immutable binary first. Keep originals and every refusal channel. A
    restored idle fixture can perform new writes; no old execution binding is
    rebased or adopted. Full schema/typed row/config comparison is corpus-specific.
    Failure leaves evidence, partial staging and source roots for diagnosis.
    """
    qualified=json.loads(profile.read_bytes());sha=hashlib.sha256(binary.read_bytes()).hexdigest()
    assert qualified['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and qualified['sha256']==sha and qualified['no_fixture_feature'] and qualified['all_core_features_empty']
    out.mkdir(exist_ok=False);root=out/'original-root';root.mkdir(mode=0o700);calls=[]
    def cli(where,args,value=None):
        number=len(calls);argv=[str(binary),*args,'--json','--root',str(where)]
        if value is not None:
            request=out/f'call-{number}.input.json';snapshot.io.publish(request,json.dumps(value));argv+=['--input',str(request)]
        result=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        snapshot.io.publish(out/f'call-{number}.stdout',result.stdout);snapshot.io.publish(out/f'call-{number}.stderr',result.stderr)
        calls.append(dict(argv=argv,exit=result.returncode));snapshot.io.publish(out/'calls.json',json.dumps(calls,indent=2))
        assert result.returncode==0,(args,result.returncode)
        return json.loads(result.stdout)
    cli(root,['init'])
    created=cli(root,['work','create'],dict(schema_version=1,event_id='restore-create',expected_revision=0,title='Cold restore corpus',objective='Preserve real data',scope='Owned inactive Root',criteria=['Exact public state and typed SQLite rows']))
    work=created['work']['id']
    cli(root,['task','add',work],dict(schema_version=1,event_id='restore-task',expected_revision=1,id='task',title='Original task',deliverable='Fixture proof',criteria=['Original retained']))
    cli(root,['budget','apply',work],dict(schema_version=1,event_id='restore-budget',expected_revision=2,budget=dict(limit_usd=10,reserve_per_launch_usd=1,estimate_source='Inert restore fixture',reference_date='2026-10-24')))
    snapshot.io.publish(root/'fixture-private-config.json',b'{"inert_private_fixture":"PRIVATE_NOT_A_CREDENTIAL"}')
    before=cli(root,['work','show',work])['work'];settings=cli(root,['engine-settings','show']);rows=tables(root)
    snapshot.io.publish(out/'typed-tables-before.json',json.dumps(rows,indent=2))
    db=root/'.swarm/state.db';inode=db.stat().st_ino;archive=out/'snapshot';manifest=snapshot.capture(root,archive)
    assert '.swarm/engine-settings.json' in manifest['entries'] and manifest['entries']['fixture-private-config.json']['mode']==0o600
    changed=cli(root,['task','add',work],dict(schema_version=1,event_id='after-snapshot',expected_revision=3,id='after',title='Mutation after snapshot',deliverable='Must not appear after restore',criteria=['Mutation remains only in original']))
    assert changed['work']['revision']==4 and tables(root)!=rows
    restored=out/'restored-root';snapshot.restore(archive,restored)
    assert cli(restored,['work','show',work])['work']==before
    assert cli(restored,['engine-settings','show'])==settings and tables(restored)==rows
    assert (restored/'fixture-private-config.json').read_bytes()==(root/'fixture-private-config.json').read_bytes()
    assert cli(root,['work','show',work])['work']==changed['work'] and db.stat().st_ino==inode
    try:snapshot.restore(archive,root)
    except ValueError as error:assert str(error)=='snapshot-destination-exists'
    else:raise AssertionError('existing destination accepted')
    assert cli(root,['work','show',work])['work']==changed['work']
    piece=archive/'data/.swarm/state.db';original=piece.read_bytes();snapshot.verify(archive)
    snapshot.io.publish(piece,b'corrupt snapshot fixture')
    try:
        try:snapshot.restore(archive,out/'corrupted-target')
        except ValueError as error:assert str(error)=='snapshot-retained-tree-mismatch'
        else:raise AssertionError('corrupted snapshot accepted')
        assert not (out/'corrupted-target').exists()
    finally:snapshot.io.publish(piece,original)
    snapshot.restore(archive,out/'recovered-root');assert cli(out/'recovered-root',['work','show',work])['work']==before
    resumed=cli(restored,['task','add',work],dict(schema_version=1,event_id='after-restore-new-write',expected_revision=3,id='new',title='New legitimate fixture write',deliverable='Reopened Root',criteria=['Data restoration resumes public operations']))
    assert resumed['work']['revision']==4 and {t['id'] for t in resumed['work']['tasks']}=={'task','new'}
    assert cli(root,['work','show',work])['work']==changed['work']
    result=dict(status='PASS',binary_sha256=sha,schema_version=rows['user_version'],tables_compared=len(rows['rows']),manifest_entries=len(manifest['entries']),public_data_settings_budget_preserved=True,private_fixture_bytes_preserved=True,original_retained=True,existing_and_corrupt_destinations_refused=True,restored_new_public_write=True,old_execution_authority_tested=False,go_parity_verified=False,installed=False)
    snapshot.io.publish(out/'result.json',json.dumps(result,indent=2)+'\n');print('CUTOVER_FIXTURE_RESTORE_PASS '+str(out),flush=True);return result


def main():
    """Use supplied exact profile/binary; create only fresh owned evidence roots."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=os.environ.get('SWARM_STATUS_CONTRACT_BINARY'));p.add_argument('--profile',type=Path,default=os.environ.get('SWARM_STATUS_CONTRACT_PROFILE'));p.add_argument('--output',type=Path);a=p.parse_args();assert a.binary and a.profile
    out=a.output or Path(tempfile.mkdtemp(prefix='rust-cutover-parent-',dir='.cache'))/'proof'
    run(a.binary.resolve(),a.profile.resolve(),out.resolve())

if __name__=='__main__':main()
