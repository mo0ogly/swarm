"""Public cold-start version journey on an explicitly qualified immutable binary.

No Cargo/provider/network or live Root access; one owned Root uses public init. Subprocesses have no arbitrary
watchdog. Assert historical JSON, alias/text, runtime-env immunity, argument and
unsupported-server refusal, no datastore mutation. Record exact binary/profile
before execution; unknown plain Cargo and injected builds stay distinguishable.
"""
from pathlib import Path
import argparse,hashlib,json,os,subprocess,tempfile


def run(binary,profile_path,out):
    """Verify the frozen artifact, then public cold/invalid routes without writes.

    Retain original channels for each command, including refusal. A failure keeps
    all earlier proof and never resets a Root or retries. Unqualified metadata
    remains unknown; injected fields are checked against declared build metadata
    by build_native.py, not guessed from the current checkout.
    """
    proof=json.loads(profile_path.read_bytes());sha=hashlib.sha256(binary.read_bytes()).hexdigest()
    assert proof['sha256']==sha and proof['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and proof['no_fixture_feature'] and proof['all_core_features_empty']
    out.mkdir(exist_ok=False);cold=out/'cold';cold.mkdir();absent=cold/'nonexistent-root'
    def invoke(name,args,code,forge=False):
        env=dict(os.environ)
        if forge:env['SWARM_NATIVE_BUILD_METADATA']='PRIVATE_FORGED_RUNTIME_IDENTITY'
        argv=[str(binary),*args,'--root',str(absent)]
        result=subprocess.run(argv,cwd=cold,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        (out/(name+'.stdout')).write_bytes(result.stdout);(out/(name+'.stderr')).write_bytes(result.stderr);(out/(name+'.argv.json')).write_text(json.dumps(argv))
        assert result.returncode==code,(name,result.returncode)
        assert not any(cold.iterdir()),name
        return result
    a=json.loads(invoke('version',['version','--json'],0).stdout);b=json.loads(invoke('alias',['--version','--json'],0).stdout)
    assert a==b and set(a)=={'schema_version','binary','diagnostic'} and a['schema_version']==1
    assert set(a['binary'])=={'version','commit','modified','build_date','provenance'}
    assert a['diagnostic']['state']=='unknown' and a['diagnostic']['candidate']['available'] is False and a['diagnostic']['active_server']['available'] is False
    assert json.loads(invoke('runtime-forgery',['version','--json'],0,True).stdout)==a
    text=invoke('text',['version'],0).stdout.decode();assert text.startswith('Swarm '+a['binary']['version']+' (commit ') and 'État : unknown' in text
    for name,args in [('unsupported-server',['version','--json','--server','http://127.0.0.1/PRIVATE_SESSION']),('extra-argument',['version','PRIVATE']),('input-refusal',['version','--input','PRIVATE'])]:
        result=invoke(name,args,2);assert not result.stdout and b'PRIVATE' not in result.stderr
    assert json.loads((out/'unsupported-server.stderr').read_bytes())['failure']['code']=='command_not_ported'
    # This owned Root is beneath the actual checkout and observes its Git HEAD.
    # Only public init creates settings; no live database or direct SQL mutation.
    configured=out/'configured-root';configured.mkdir()
    def configured_call(name,expected):
        argv=[str(binary),'version','--json','--root',str(configured)]
        reply=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        (out/(name+'.stdout')).write_bytes(reply.stdout);(out/(name+'.stderr')).write_bytes(reply.stderr)
        assert reply.returncode==expected,(name,reply.returncode);return reply
    init=subprocess.run([str(binary),'init','--json','--root',str(configured)],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    (out/'init.stdout').write_bytes(init.stdout);(out/'init.stderr').write_bytes(init.stderr);assert init.returncode==0
    database=configured/'.swarm/state.db';original=database.read_bytes();before=hashlib.sha256(original).hexdigest()
    healthy=json.loads(configured_call('configured-policy',0).stdout)
    head=subprocess.run(['git','-C',str(configured),'rev-parse','--verify','HEAD'],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    assert head.returncode==0
    assert healthy['diagnostic']['candidate']['available'] and healthy['diagnostic']['candidate']['commit']==head.stdout.decode().strip()
    assert hashlib.sha256(database.read_bytes()).hexdigest()==before
    # Preserve original inode/data before injecting malformed inactive fixture.
    backup=database.with_name('version-original-state.db');os.rename(database,backup)
    try:
        database.write_bytes(b'PRIVATE_INVALID_SQLITE_FIXTURE')
        refusal=configured_call('present-corrupt-root',2)
        assert not refusal.stdout and b'PRIVATE' not in refusal.stderr
        assert database.read_bytes()==b'PRIVATE_INVALID_SQLITE_FIXTURE'
    finally:
        if database.exists():os.rename(database,database.with_name('version-refused-state.db'))
        os.rename(backup,database)
    assert json.loads(configured_call('restored-root',0).stdout)==healthy
    assert hashlib.sha256(database.read_bytes()).hexdigest()==before
    # Actual OS output failure, retaining safe stderr without masking as success.
    with open('/dev/full','wb',buffering=0) as full:
        failed=subprocess.run([str(binary),'version','--json','--root',str(absent)],cwd=cold,stdin=subprocess.DEVNULL,stdout=full,stderr=subprocess.PIPE)
    (out/'output-failure.stderr').write_bytes(failed.stderr);assert failed.returncode==2 and json.loads(failed.stderr)['failure']['code']=='storage_unavailable' and 'os-errno:28' in json.loads(failed.stderr)['failure']['causes']
    result=dict(status='PASS',assertions=['qualified-binary','historical-json','version-alias','runtime-forgery-ignored','text','server-refusal','extra-argument-refusal','input-refusal','cold-root-not-created','saved-policy-candidate','present-root-refusal-and-recovery','actual-output-failure'],binary_sha256=sha,installed=False)
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');print('VERSION_PUBLIC_PASS '+str(out),flush=True);return result


def main():
    """Use explicit profile/binary or existing verification environment; no build."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=os.environ.get('SWARM_STATUS_CONTRACT_BINARY'));p.add_argument('--profile',type=Path,default=os.environ.get('SWARM_STATUS_CONTRACT_PROFILE'));p.add_argument('--output',type=Path);a=p.parse_args();assert a.binary and a.profile
    out=a.output or Path(tempfile.mkdtemp(prefix='rust-version-parent-',dir='.cache'))/'proof'
    run(a.binary.resolve(),a.profile.resolve(),out.resolve())

if __name__=='__main__':main()
