#!/usr/bin/env python3
"""Six actual default-CLI dormant creation/reconciliation journeys.
Controller Request helper only creates canonical metadata, never keys/proof.
SWARM_PAEA_INITIAL_{BINARY,BINARY_SHA256,EVIDENCE} and explicit
SWARM_PAEA_INITIAL_REQUEST_{FIXTURE_HELPER,FIXTURE_META} are required.
All roots are private /tmp fixtures; no live Root, provider, account, namespace or
service. Known publication and CLI transport are separate; no global watchdog.
"""
from pathlib import Path
import os,json,hashlib,tempfile,contextlib,subprocess
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
BINARY=Path(os.environ['SWARM_PAEA_INITIAL_BINARY']).resolve();BINARY_SHA=os.environ['SWARM_PAEA_INITIAL_BINARY_SHA256']
HELPER=Path(os.environ['SWARM_PAEA_INITIAL_REQUEST_FIXTURE_HELPER']).resolve();META=json.loads(Path(os.environ['SWARM_PAEA_INITIAL_REQUEST_FIXTURE_META']).read_text())
OUT=Path(os.environ['SWARM_PAEA_INITIAL_EVIDENCE']);OUT.mkdir(parents=True,mode=0o700,exist_ok=False)
SCHEMA=json.loads((ROOT/'schemas/paea/initial-creation-cli-v1.schema.json').read_text());Draft202012Validator.check_schema(SCHEMA);VALIDATOR=Draft202012Validator(SCHEMA)
COUNT=0;CASES=[]

def file_digest(path):
    """Read exact bytes without logging private material; digest is observation only."""
    with path.open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def raw_digest(raw):
    """Fixture-only independent source-byte pin, never derive trust from a package."""
    return hashlib.sha256(raw).hexdigest()

def keep(path,raw,replace=False):
    """Atomic private owned fixture/log writes with fsync/readback before replace."""
    assert replace or not path.exists();fd,tmp=tempfile.mkstemp(prefix='.initial-proof-',dir=path.parent)
    try:
        with os.fdopen(fd,'wb')as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())
        assert Path(tmp).read_bytes()==raw;os.chmod(tmp,0o600);os.rename(tmp,path)
        directory=os.open(path.parent,os.O_DIRECTORY);os.fsync(directory);os.close(directory)
    finally:
        if os.path.exists(tmp):os.unlink(tmp)

def run(name,argv,full=False):
    """One exact default/fixture child with retained argv/output/exit, no stdin log.
    /dev/full injects actual stdout ENOSPC after successful private publication.
    """
    global COUNT
    if argv[0]==str(BINARY):assert file_digest(BINARY)==BINARY_SHA
    else:assert argv[0]==str(HELPER)and file_digest(HELPER)==META['sha256']
    COUNT+=1
    if full:
        with open('/dev/full','wb',buffering=0)as sink:result=subprocess.run(argv,stdout=sink,stderr=subprocess.PIPE)
        result.stdout=b''
    else:result=subprocess.run(argv,capture_output=True)
    stem=OUT/f'{COUNT:03d}-{name}';keep(stem.with_suffix('.stdout'),result.stdout);keep(stem.with_suffix('.stderr'),result.stderr);keep(stem.with_suffix('.json'),json.dumps({'argv':argv,'exit':result.returncode,'stdout_dev_full':full},indent=2).encode());return result

@contextlib.contextmanager
def fixture():
    """Canonical controller data only; production CLI must generate/store all keys."""
    with tempfile.TemporaryDirectory(prefix='paea-initial-native-',dir='/tmp')as directory:
        root=Path(directory);root.chmod(0o700);result=run('controller-request-fixture',[str(HELPER),directory]);assert result.returncode==0
        meta=json.loads(result.stdout);assert meta['fixture_kind']=='initial-operator-request-v1';assert not meta['human_verified']and not meta['material_generated']and not meta['separate_os_uid_verified']
        source=Path(meta['request_path']);assert file_digest(source)==meta['request_pin'];body=json.loads(source.read_bytes());assert body['controller_uid']==os.geteuid()and body['service_uid']!=os.geteuid()
        op=root/'operator';anchors=root/'anchors';op.mkdir(mode=0o700);anchors.mkdir(mode=0o700)
        meta.update(root=root,operator=op,anchor=anchors/'pin.json',intent=anchors/'pin.json.intent',package=op/'initial-provisioning-v1.bin',source=source)
        assert not Path(meta['target']).exists();yield meta
        assert not Path(meta['target']).exists()and not(root/'.swarm').exists()

def command(m,action='initial-create',pin=None,lang='en',extra=(),full=False,operator=None,anchor=None):
    """Closed public request: no Root/UID/key/clock/default aliases or stdin."""
    return run(action,[str(BINARY),'identity',action,m['request_pin']if pin is None else pin,str(m['operator']if operator is None else operator),str(m['anchor']if anchor is None else anchor),'--input',str(m['source']),'--json','--lang',lang,*extra],full)

def preflight(m):
    """Valid same-fixture request witness before injecting a source/target refusal."""
    result=run('request-control',[str(BINARY),'identity','provisioning-check',m['request_pin'],'--input',str(m['source']),'--json']);assert result.returncode==0 and json.loads(result.stdout)['request_validated']

def report(m,result,action):
    """Validate meaningful closed metadata and exact private publication/read state."""
    assert result.returncode==0,result.stderr.decode();assert not result.stderr
    value=json.loads(result.stdout);VALIDATOR.validate(value);assert value['operation']==action and value['request_pin']==m['request_pin'];assert value['package_pin']==file_digest(m['package']);assert value['package_request_matched']
    for key in ['locator_trust_verified','current_authority_verified','human_authentication_verified','separate_os_uid_verified','confinement_verified','service_installed','service_started']:assert value[key]is False
    assert m['intent'].is_file()and m['anchor'].is_file()and m['package'].is_file()
    anchor=json.loads(m['anchor'].read_bytes());intent=json.loads(m['intent'].read_bytes());assert anchor['request_pin']==m['request_pin']and anchor['package_pin']==value['package_pin'];assert intent['request_pin']==m['request_pin']
    for path in [m['intent'],m['anchor'],m['package']]:assert path.stat().st_mode&0o7777==0o600 and path.stat().st_nlink==1 and path.stat().st_uid==os.geteuid()
    assert not Path(m['target']).exists();return value

def refused(m,result):
    """Refusal never succeeds empty or discloses private paths/source/package pins."""
    assert result.returncode!=0 and not result.stdout
    assert str(m['root']).encode()not in result.stderr and m['request_pin'].encode()not in result.stderr
    if m['package'].exists():assert file_digest(m['package']).encode()not in result.stderr

def languages_and_exact_recovery():
    """Actual FR/EN generation once then repeated exact reads, no service effect."""
    for lang in ['fr','en']:
        with fixture()as m:
            preflight(m);before=file_digest(m['source']);value=report(m,command(m,lang=lang),'initial-create');assert value['material_generation_attempted']and value['package_publication']=='durable'
            package=file_digest(m['package']);report(m,command(m,'initial-reconcile',lang=lang),'initial-reconcile');report(m,command(m,'initial-reconcile',lang=lang),'initial-reconcile');assert file_digest(m['package'])==package and file_digest(m['source'])==before
            help_result=run('help-'+lang,[str(BINARY),'--help','--lang',lang]);assert help_result.returncode==0 and all(name in help_result.stdout for name in [b'initial-create',b'initial-reconcile'])

def source_refusals():
    """Known valid source first; bad byte pin/canonical/UID/private metadata refuse
    before targets. Preserve original bytes and explicitly restore/retry once.
    """
    for variant in ['pin','canonical','uid','mode']:
        with fixture()as m:
            preflight(m);original=m['source'].read_bytes();selected_pin=m['request_pin']
            if variant=='pin':selected_pin='00'*32
            elif variant=='canonical':
                changed=original+b' ';keep(m['source'],changed,replace=True);selected_pin=raw_digest(changed)
            elif variant=='uid':
                body=json.loads(original);old=body['controller_uid'];other=body['service_uid'];changed=original.replace(('"controller_uid":'+str(old)).encode(),b'"controller_uid":__TEMP__',1).replace(('"service_uid":'+str(other)).encode(),('"service_uid":'+str(old)).encode(),1).replace(b'__TEMP__',str(other).encode(),1);keep(m['source'],changed,replace=True);selected_pin=raw_digest(changed)
            else:m['source'].chmod(0o644)
            refused(m,command(m,pin=selected_pin));assert not m['intent'].exists()and not m['anchor'].exists()and not m['package'].exists()
            keep(m['source'],original,replace=True);report(m,command(m),'initial-create')

def target_refusals():
    """Private guards/physical alias and every existing slot refuse before material.
    Existing files remain byte-exact; safe original targets subsequently succeed.
    """
    for variant in ['operator-mode','anchor-mode','alias','intent-existing','anchor-existing','package-existing']:
        with fixture()as m:
            preflight(m);operator=m['operator'];anchor=m['anchor'];existing=None
            if variant=='operator-mode':operator.chmod(0o755)
            elif variant=='anchor-mode':anchor.parent.chmod(0o755)
            elif variant=='alias':anchor=operator/'pin.json'
            else:
                existing=m[{'intent-existing':'intent','anchor-existing':'anchor','package-existing':'package'}[variant]];keep(existing,b'EXISTING')
            result=command(m,operator=operator,anchor=anchor);refused(m,result)
            assert b'initial-material-attempted:false'in result.stderr
            if existing is not None:assert existing.read_bytes()==b'EXISTING';existing.unlink()
            if variant=='operator-mode':operator.chmod(0o700)
            elif variant=='anchor-mode':anchor.parent.chmod(0o700)
            assert not m['package'].exists();report(m,command(m),'initial-create')

def existing_and_orphan_slots():
    """Once published, create cannot regenerate; orphan intent never becomes an
    empty success or automatic new key. No destructive abandonment is simulated.
    """
    with fixture()as m:
        report(m,command(m),'initial-create');original=file_digest(m['package']);intent=file_digest(m['intent']);anchor=file_digest(m['anchor']);refused(m,command(m));report(m,command(m,'initial-reconcile'),'initial-reconcile');assert file_digest(m['package'])==original and file_digest(m['intent'])==intent and file_digest(m['anchor'])==anchor
    with fixture()as m:
        preflight(m);raw=json.dumps({'schema_version':1,'protocol':'swarm-initial-provisioning-intent-v1','request_pin':m['request_pin']},separators=(',',':')).encode();keep(m['intent'],raw)
        result=command(m);refused(m,result);assert b'initial-material-attempted:false'in result.stderr
        result=command(m,'initial-reconcile');refused(m,result);assert b'initial-material-attempted:false'in result.stderr
        assert m['intent'].read_bytes()==raw and not m['anchor'].exists()and not m['package'].exists()

def closed_flags():
    """Default forbidden aliases/duplicates refuse before source/target locks."""
    for action in ['initial-create','initial-reconcile']:
        for flags in [('--root','.'),('--phase','delivery'),('--output',''),('--json',),('--server','false'),('--input','duplicate')]:
            with fixture()as m:
                preflight(m);lock=m['source'].parent/'private-documents.lock';assert lock.is_file();lock.unlink()
                refused(m,command(m,action,extra=flags));assert not lock.exists()and not(m['operator']/'private-documents.lock').exists()and not(m['anchor'].parent/'private-documents.lock').exists()
                report(m,command(m),'initial-create')

def stdout_full():
    """Actual ENOSPC after all private commits preserves same pinned bytes and
    explicit read recovery; repeated create still refuses and never overwrites.
    """
    with fixture()as m:
        preflight(m);result=command(m,full=True);refused(m,result);assert b'initial-package-publication:durable'in result.stderr
        before=file_digest(m['package']);report(m,command(m,'initial-reconcile'),'initial-reconcile');refused(m,command(m));assert file_digest(m['package'])==before

def main():
    """Require six actual named default-binary journeys and immutable role ELFs."""
    for name,case in [('fr-en-create-exact-read-recovery-and-help',languages_and_exact_recovery),('source-pin-canonical-uid-metadata-refusal-and-recovery',source_refusals),('private-target-alias-existing-slot-refusal-and-recovery',target_refusals),('existing-and-orphan-intent-never-regenerate',existing_and_orphan_slots),('closed-flags-before-source-or-target-io',closed_flags),('stdout-enospc-keeps-pinned-material-and-reconciles',stdout_full)]:
        case();CASES.append(name);print('PASS '+name,flush=True)
    assert file_digest(BINARY)==BINARY_SHA and file_digest(HELPER)==META['sha256']
    keep(OUT/'result.json',json.dumps({'status':'PASS','count':len(CASES),'commands':COUNT,'cases':CASES,'binary_sha256':BINARY_SHA,'scope':'actual default dormant package creation/reconciliation; helper request data only; no live Root/account/service/human/real UID separation/current authority/rollback proof'},indent=2).encode());print('INITIAL_CREATION_NATIVE_PASS cases=6',flush=True)
if __name__=='__main__':main()
