from pathlib import Path
import base64, datetime, hashlib, json, os, subprocess, sys, tempfile

B = Path(__file__).resolve().parent
label = sys.argv[1]
binary_label = sys.argv[2] if len(sys.argv)>2 else '07'
binary = Path('/tmp/swarm-identity-bootstrap-03d-fixture-'+binary_label)
assert binary.is_file()
binary_digest = hashlib.sha256(binary.read_bytes()).hexdigest()

def invoke(name, args, cwd, expected):
    path = B / (name + '-' + label)
    assert not path.with_suffix('.json').exists(), 'preserve existing attempt'
    start = datetime.datetime.now(datetime.timezone.utc).isoformat()
    result = subprocess.run(args, cwd=cwd, env={'PATH':'/usr/bin:/bin'}, capture_output=True)
    path.with_suffix('.stdout').write_bytes(result.stdout)
    path.with_suffix('.stderr').write_bytes(result.stderr)
    record = dict(command=list(map(str,args)),exit=result.returncode,expected=expected,start=start,
                  end=datetime.datetime.now(datetime.timezone.utc).isoformat(),binary_sha256=binary_digest)
    path.with_suffix('.json').write_text(json.dumps(record,indent=2)+'\n')
    assert result.returncode == expected, (name,result.returncode,result.stderr.decode())
    if expected:
        assert not result.stdout and result.stderr
        return None
    reply = json.loads(result.stdout)
    assert reply['storage_verified'] is True
    assert all(reply[name] is False for name in ['human_authentication_verified','confinement_verified','service_started'])
    return reply

def document(base, uid):
    return dict(schema_version=1,pins=dict(installation_digest='a'*64,
        authority_public_key=base64.urlsafe_b64encode(bytes([7])*32).decode().rstrip('='),
        bootstrap_principal=dict(principal_id='operator',principal_kind='human',issuer='https://issuer.example',subject='fixture-human')),
        service_uid=uid,controller_uid=uid ^ 0xffffffff,
        configuration_directory=str(base/'config'),witness_directory=str(base/'witness'))

with tempfile.TemporaryDirectory(prefix='swarm-bootstrap-cli-') as temporary:
    root = Path(temporary)
    root.chmod(0o700)
    for name in ['bootstrap','config','witness','empty-project']:
        (root/name).mkdir(mode=0o700)
    fixtures = {}
    for name,base,uid in [('host',root,os.geteuid()),('namespace',Path('/tmp/fixture'),0)]:
        raw = json.dumps(document(base,uid),ensure_ascii=False,separators=(',',':')).encode()
        path = root/'bootstrap'/(name+'.json')
        path.write_bytes(raw)
        path.chmod(0o600)
        fixtures[name] = dict(document=document(base,uid),digest=hashlib.sha256(raw).hexdigest(),path=str(path))
    (B/('cli-fixtures-'+label+'.json')).write_text(json.dumps(fixtures,indent=2)+'\n')
    host=fixtures['host']
    invoke('cli-valid',[str(binary),'identity','bootstrap-check',host['digest'],'--input',host['path'],'--json'],root/'empty-project',0)
    invoke('cli-wrong-pin',[str(binary),'identity','bootstrap-check','b'*64,'--input',host['path'],'--json'],root/'empty-project',2)
    assert not list((root/'empty-project').iterdir())
    # Private user/mount/PID/network namespaces only; no host account or mount
    # changes. UID 0 here maps to the invoking unprivileged host user.
    (root/'swarm-fixture').touch(mode=0o755)
    prefix=['/usr/bin/bwrap','--unshare-user','--uid','0','--gid','0','--unshare-pid','--unshare-net',
            '--ro-bind','/','/','--tmpfs','/tmp','--proc','/proc','--ro-bind',str(root),'/tmp/fixture',
            '--ro-bind',str(binary),'/tmp/fixture/swarm-fixture','--chdir','/tmp/fixture/empty-project']
    request=['/tmp/fixture/swarm-fixture','identity','bootstrap-check',fixtures['namespace']['digest'],
             '--input','/tmp/fixture/bootstrap/namespace.json','--json']
    invoke('namespace-valid',prefix+['--']+request,root,0)
    invoke('namespace-alias',prefix+['--ro-bind',str(root/'config'),'/tmp/fixture/witness','--']+request,root,2)
    for path in [root/'config',root/'witness',root/'empty-project']:
        assert not list(path.iterdir()), ('read-only violation',path)
    for name in ['host','namespace']:
        path=Path(fixtures[name]['path'])
        assert hashlib.sha256(path.read_bytes()).hexdigest()==fixtures[name]['digest']
assert hashlib.sha256(binary.read_bytes()).hexdigest()==binary_digest
print(json.dumps(dict(cli='PASS',namespace_baseline='PASS',bind_alias_rejected=True,mutation=False,host_accounts_created=False)))
