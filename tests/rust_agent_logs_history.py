#!/usr/bin/env python3
"""Exercise historical agent logs on a qualified native binary and owned roots.

Only public prepare/run writes SQLite. A local inert provider executes once. Exact lifecycle logs are the public
fixture; the SQL unit tests separately cover the 200-row boundary. Read-only SQL is an independent
oracle. Every public read/refusal must preserve typed and physical database bytes.
No paid provider, global watchdog, live mission, acceptance or automatic retry.
"""
import argparse,json,os,sqlite3,stat,traceback
from pathlib import Path
import rust_agent_model_route as fixture
import rust_mission_status_contract as shared


def exercise(binary,output):
    """Run real output then verify exact pagination, export refusal and recovery.

    Retains original argv/channels and born fixture attribution. File export is
    an intentional separate filesystem effect; DB purity never claims rollback
    of a partly written output. Failures propagate with original artifacts.
    """
    owned=fixture.Fixture(binary,output,'logs')
    driver=shared.Driver(binary,None,output,owned.root,None,owned.environment)
    driver.work=owned.work
    prepared=owned.prepare()
    # A valid queued Agent precedes every missing/invalid refusal control.
    base=[str(binary),'agent','logs','session','--root',str(owned.root),'--json']
    def read(label,args=None,full=False):
        """Keep the shared no-mutation witness around the actual CLI operation."""
        result,_=driver.read(label,base if args is None else args,output_full=full)
        return result
    def rows(label,cursor=None):
        """Require bare JSON arrays, even when the caller omits --json."""
        argv=base[:-1] if cursor is None else base+[str(cursor)]
        code,raw,err=read(label,argv)
        assert code==0 and not err,(label,code,err)
        value=json.loads(raw);assert isinstance(value,list),value
        return value
    with sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        queued=db.execute('SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=? AND seq>0 ORDER BY seq',('session',)).fetchall()
    assert rows('queued-control')==[dict(zip(['seq','agent_id','at','kind','message'],row)) for row in queued][-200:]
    assert rows('queued-empty-after-max',9223372036854775807)==[]
    owned.run_and_assert(prepared)
    with sqlite3.connect('file:'+str(owned.root/'.swarm/state.db')+'?mode=ro',uri=True) as db:
        data=db.execute('SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=? AND seq>0 ORDER BY seq',('session',)).fetchall()
    expected=[dict(zip(['seq','agent_id','at','kind','message'],row)) for row in data]
    assert expected and all(row["agent_id"]=="session" for row in expected),expected
    assert rows('latest')==expected[-200:]
    assert rows('first-negative',-1)==expected[:200]
    assert rows('minimum-cursor',-9223372036854775808)==expected[:200]
    assert rows('zero',0)==expected[-200:]
    cursor=expected[len(expected)//2]['seq']
    assert rows('forward',cursor)==[row for row in expected if row['seq']>cursor][:200]
    assert rows('end',9223372036854775807)==[]
    traversed=[];cursor=-1
    while True:
        page=rows('traverse-'+str(len(traversed)),cursor)
        if not page:break
        traversed+=page;cursor=page[-1]['seq']
    assert traversed==expected
    for label,cursor in [('invalid','abc'),('overflow','9223372036854775808')]:
        code,raw,err=read(label,base+[cursor]);assert code!=0 and not raw and err
    missing=base.copy();missing[3]='missing'
    code,raw,err=read('missing',missing);assert code!=0 and not raw and err
    export=output/'export.ndjson'
    code,raw,err=read('export',base+['--output',str(export)])
    assert code==0 and not raw and not err,(code,err)
    original=export.read_bytes()
    assert [json.loads(row) for row in original.splitlines()]==expected[-200:]
    assert stat.S_IMODE(export.stat().st_mode)==0o600
    code,raw,err=read('exclusive',base+['--output',str(export)])
    assert code!=0 and not raw and err and export.read_bytes()==original
    empty=output/'empty.ndjson'
    code,raw,err=read('empty-export',base+['9223372036854775807','--output',str(empty)])
    assert code==0 and not raw and not err and empty.read_bytes()==b''
    target=output/'missing-parent'/'export.ndjson'
    code,raw,err=read('parent-refusal',base+['--output',str(target)])
    assert code!=0 and not raw and err and not target.exists()
    target.parent.mkdir()
    code,raw,err=read('parent-recovery',base+['--output',str(target)])
    assert code==0 and not raw and not err and target.read_bytes()==original
    # Inject EFBIG only into our CLI child. The wrapper captures diagnostics in
    # pipes so its evidence files do not inherit the injected file-size limit.
    # Zero is an explicit OS fault fixture, not a product/test watchdog policy.
    fault=output/'partial.ndjson'
    wrapper="""import resource,signal,subprocess,sys
def fault():
    resource.setrlimit(resource.RLIMIT_FSIZE,(0,0))
    signal.signal(signal.SIGXFSZ,signal.SIG_IGN)
r=subprocess.run(sys.argv[1:],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,preexec_fn=fault)
sys.stdout.buffer.write(r.stdout);sys.stdout.buffer.flush()
sys.stderr.buffer.write(r.stderr);sys.stderr.buffer.flush()
sys.exit(r.returncode)
"""
    code,raw,err=read('file-write-fault',['python3','-c',wrapper,*base,'--output',str(fault)])
    assert code!=0 and not raw and err and fault.exists() and fault.read_bytes()==b''
    diagnostic=json.loads(err);assert 'partial' in diagnostic['failure']['message'] or 'partiel' in diagnostic['failure']['message']
    assert 'os-errno:27' in diagnostic['failure']['causes'],diagnostic
    # Existing partial output remains exclusive; recovery uses a new destination.
    code,raw,err=read('partial-preserved',base+['--output',str(fault)])
    assert code!=0 and not raw and fault.read_bytes()==b''
    recovered=output/'file-recovery.ndjson'
    code,raw,err=read('file-recovery',base+['--output',str(recovered)])
    assert code==0 and not raw and not err and recovered.read_bytes()==original
    code,raw,err=read('stdout-full',full=True)
    assert code!=0 and not raw and json.loads(err)['failure']['code']=='storage_unavailable'
    assert rows('stdout-recovery')==expected[-200:]
    fixture.publish(output/'result.json',dict(status='PASS',reads=driver.reads,rows=len(expected),root=str(owned.root),full_migration=False))
    print('PASS historical agent logs pagination, exclusive export and recovery',flush=True)


def main():
    """Bind supplied default native profile before any public fixture mutation."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--profile',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=False)
    try:
        profile=json.loads(args.profile.read_bytes())
        assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['sha256']==fixture.digest(args.binary.read_bytes()),profile.get('status')
        # Store the entire profile and exact executable hash; the profile's own
        # collector validates compiler closure, not this fixture.
        fixture.publish(args.output/'profile.json',profile)
        fixture.publish(args.output/'binary.json',dict(path=str(args.binary.resolve()),sha256=fixture.digest(args.binary.read_bytes())))
        exercise(args.binary.resolve(),args.output)
    except BaseException:
        fixture.publish(args.output/'failure.txt',traceback.format_exc().encode());raise

if __name__=='__main__':main()
