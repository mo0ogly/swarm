#!/usr/bin/env python3
"""Five actual default-binary anchor-read journeys; private synthetic pins only.
Requires SWARM_PAEA_ANCHOR_BINARY, BINARY_SHA256 and fresh EVIDENCE. No helper,
provider, Root, key/proof, package generation or global watchdog. Trusted-locator
provenance/current authority are not established by these fixtures or the CLI.
"""
from pathlib import Path
import os,json,hashlib,tempfile,subprocess,contextlib
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
PREFIX='SWARM_PAEA_ANCHOR_'
BINARY=Path(os.environ[PREFIX+'BINARY']).resolve();EXPECTED=os.environ[PREFIX+'BINARY_SHA256']
OUT=Path(os.environ[PREFIX+'EVIDENCE']);OUT.mkdir(mode=0o700,parents=True,exist_ok=False)
SCHEMA=json.loads((ROOT/'schemas/paea/operator-anchor-cli-v1.schema.json').read_text());Draft202012Validator.check_schema(SCHEMA);VALIDATOR=Draft202012Validator(SCHEMA)
COUNT=0;CASES=[]

def digest(path):
    """Hash actual named bytes without exposing them."""
    with path.open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def keep(path,raw):
    """Exclusive atomic private logs/fixture writes with readback/fsync."""
    assert not path.exists();fd,tmp=tempfile.mkstemp(prefix='.anchor-proof-',dir=path.parent)
    try:
        with os.fdopen(fd,'wb')as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())
        assert Path(tmp).read_bytes()==raw;os.chmod(tmp,0o600);os.rename(tmp,path)
        directory=os.open(path.parent,os.O_DIRECTORY);os.fsync(directory);os.close(directory)
    finally:
        if os.path.exists(tmp):os.unlink(tmp)

def run(argv,full=False):
    """One immutable default ELF child, preserved argv/output/exit, no stdin."""
    global COUNT
    assert digest(BINARY)==EXPECTED;COUNT+=1
    if full:
        with open('/dev/full','wb',buffering=0)as sink:result=subprocess.run(argv,stdout=sink,stderr=subprocess.PIPE)
        result.stdout=b''
    else:result=subprocess.run(argv,capture_output=True)
    stem=OUT/f'{COUNT:03d}'
    keep(stem.with_suffix('.stdout'),result.stdout);keep(stem.with_suffix('.stderr'),result.stderr)
    keep(stem.with_suffix('.json'),json.dumps({'argv':argv,'exit':result.returncode,'stdout_dev_full':full},indent=2).encode());return result

@contextlib.contextmanager
def fixture(raw=None):
    """Canonical synthetic anchor metadata only in an isolated private /tmp root."""
    with tempfile.TemporaryDirectory(prefix='paea-anchor-native-',dir='/tmp')as directory:
        root=Path(directory);root.chmod(0o700);path=root/'anchor.json'
        record={'schema_version':1,'protocol':'swarm-operator-anchor-v1','request_pin':'ab'*32,'package_pin':'cd'*32}
        keep(path,raw if raw is not None else json.dumps(record,separators=(',',':')).encode())
        yield root,path
        assert not(root/'.swarm').exists()and not(root/'initial-provisioning-v1.bin').exists()

def command(path,pin='ab'*32,lang='en',extra=(),full=False):
    """Exact public read command; no selected storage UID, Root or output path."""
    return run([str(BINARY),'identity','checkpoint-check',pin,'--input',str(path),'--json','--lang',lang,*extra],full)

def checked(result):
    """Assert meaningful report, explicit unproved claims and safe blank diagnostics."""
    assert result.returncode==0,result.stderr.decode();assert not result.stderr
    value=json.loads(result.stdout);VALIDATOR.validate(value);assert value['request_pin']=='ab'*32 and value['package_pin']=='cd'*32
    assert value['anchor_decoded']and value['read_validation']=='bytes_read'and value['read_durability']=='directory_acknowledged'
    assert all(value[k]is False for k in ['locator_trust_verified','current_authority_verified','human_authentication_verified','separate_os_uid_verified','operator_package_read','material_generated','service_installed']);return value

def refused(root,result):
    """No usable report/private path or digest leaks; refusal is never empty success."""
    assert result.returncode!=0 and not result.stdout
    assert str(root).encode()not in result.stderr and('ab'*32).encode()not in result.stderr and('cd'*32).encode()not in result.stderr

def languages():
    """Read in both languages and inspect public help without any package effect."""
    with fixture()as(root,path):
        before=digest(path)
        for lang in['fr','en']:
            checked(command(path,lang=lang));help_result=run([str(BINARY),'--help','--lang',lang]);assert help_result.returncode==0 and b'checkpoint-check'in help_result.stdout
        assert digest(path)==before

def binding_and_canonical():
    """Valid witness precedes wrong binding/shape, noncanonical/unknown record and recovery."""
    for pin in['ef'*32,'bad','AB'*32,'ab'*32+'\n']:
        with fixture()as(root,path):checked(command(path));before=digest(path);refused(root,command(path,pin=pin));assert digest(path)==before;checked(command(path))
    for raw in[b'{}',b'{"schema_version":1,"extra":true}',json.dumps({'schema_version':1,'protocol':'swarm-operator-anchor-v1','request_pin':'ab'*32,'package_pin':'cd'*32},separators=(',',':')).encode()+b' ']:
        with fixture()as(root,path):
            checked(command(path));valid=path.with_name('valid.json');path.rename(valid);keep(path,raw);before=digest(path);refused(root,command(path));assert digest(path)==before;path.unlink();valid.rename(path);checked(command(path))

def metadata():
    """Valid same-precondition witness, unsafe metadata/name refusal and exact restore."""
    for variant in['mode','parent','hardlink','symlink','basename','reserved','missing']:
        with fixture()as(root,path):
            checked(command(path));before=digest(path);held=path.with_name('held.json');selected=path
            if variant=='mode':path.chmod(0o644)
            elif variant=='parent':root.chmod(0o755)
            elif variant=='hardlink':os.link(path,held)
            elif variant=='symlink':path.rename(held);path.symlink_to(held)
            elif variant=='basename':selected=root/'-bad';path.rename(selected)
            elif variant=='reserved':selected=root/'private-documents.lock'
            else:path.rename(held)
            refused(root,command(selected))
            if variant=='mode':path.chmod(0o600)
            elif variant=='parent':root.chmod(0o700)
            elif variant=='hardlink':held.unlink()
            elif variant=='symlink':path.unlink();held.rename(path)
            elif variant=='basename':selected.rename(path)
            elif variant=='missing':held.rename(path)
            assert digest(path)==before;checked(command(path))

def closed_flags():
    """Duplicates/default forbidden options refuse before opening a fresh lock."""
    for flags in[('--root','.'),('--phase','delivery'),('--output',''),('--server','false'),('--json',),('--input','duplicate')]:
        with fixture()as(root,path):
            checked(command(path));lock=root/'private-documents.lock';assert lock.is_file();lock.unlink()
            refused(root,command(path,extra=flags));assert not lock.exists();checked(command(path))

def stdout_full():
    """Real ENOSPC retains read/ack/decoded facts and does not cause reread/creation."""
    with fixture()as(root,path):
        checked(command(path));before=digest(path);result=command(path,full=True);refused(root,result)
        assert b'anchor-decoded:true'in result.stderr and b'anchor-read-durability:directory_acknowledged'in result.stderr
        assert digest(path)==before;checked(command(path))

def main():
    """Require all five named actual journeys and real immutable ELF child count."""
    for name,case in[('fr-en-read-and-help',languages),('request-binding-canonical-refusal-and-recovery',binding_and_canonical),('private-metadata-and-name-refusal-and-recovery',metadata),('closed-flags-before-io',closed_flags),('stdout-enospc-keeps-known-read',stdout_full)]:
        case();CASES.append(name);print('PASS '+name,flush=True)
    assert digest(BINARY)==EXPECTED
    keep(OUT/'result.json',json.dumps({'status':'PASS','count':len(CASES),'commands':COUNT,'cases':CASES,'binary_sha256':EXPECTED,'scope':'actual default readonly anchor metadata only; no locator provenance/current authority/human/real UID/package/service proof'},indent=2).encode())
    print('CHECKPOINT_NATIVE_PASS cases=5',flush=True)
if __name__=='__main__':main()
