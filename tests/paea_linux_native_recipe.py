#!/usr/bin/env python3
"""Collect portable CLI/HTTP proof on an explicitly qualified frozen normal ELF.

No live Root/provider/launch protection. Inner recipe retains exact executable and
qualified build inputs. Anchored receipt covers safe journey top-level outputs;
isolated Root/cwd and credentials are excluded. No independent custody claim.
"""
from pathlib import Path
import hashlib,json,os,re,stat,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parents[1]

def file_bytes(root,name):
    """Require contained ordinary files and real directories before reading hashes."""
    parts=Path(name).parts
    if Path(name).is_absolute()or'..'in parts:raise ValueError('proof-name')
    path=root
    for i,part in enumerate(parts):
        path/=part;mode=path.lstat().st_mode
        if i==len(parts)-1:
            if not stat.S_ISREG(mode):raise ValueError('proof-file-kind')
        elif not stat.S_ISDIR(mode):raise ValueError('proof-directory-kind')
    return path.read_bytes()

def sha(raw):
    """Hash retained bytes, not mutable path labels or a version assertion."""
    return hashlib.sha256(raw).hexdigest()

def verify(proof,expected):
    """Verify anchored receipt and all declared safe native outputs only."""
    if not re.fullmatch('[a-f0-9]{64}',expected):raise ValueError('proof-anchor')
    raw=file_bytes(proof,'receipt.json')
    if sha(raw)!=expected:raise ValueError('proof-receipt-hash')
    receipt=json.loads(raw)
    if set(receipt)!={'schema_version','scope','files','binary_sha256'}or receipt['schema_version']!=1 or receipt['scope']!='qualified-normal-cli-http-isolated':raise ValueError('proof-schema')
    for name,h in receipt['files'].items():
        if sha(file_bytes(proof,name))!=h:raise ValueError('proof-artifact-hash')
    result=json.loads(file_bytes(proof,'journeys/result.json'))
    if result['status']!='PASS'or len(result['checks'])!=37 or len(set(result['checks']))!=37 or len(result['commands'])!=52 or len(result['http'])!=16 or result['sha256']!=receipt['binary_sha256']:raise ValueError('proof-native-outcome')
    needed={'journeys/result.json','journeys/root-before.json','journeys/root-after.json','journeys/server.stdout','journeys/server.stderr','run/result.json'}
    for c in result['commands']:needed|={'journeys/'+c['name']+'.stdout','journeys/'+c['name']+'.stderr'}
    needed|={'journeys/'+c['name']+'.body'for c in result['http']}
    if not needed.issubset(receipt['files']):raise ValueError('proof-output-inventory')
    run=json.loads(file_bytes(proof,'run/result.json'))
    if run['status']!='PASS'or run['source_stable']is not True:raise ValueError('inner-recipe-refused')
    return receipt

def collect():
    """Create a fresh proof, retain profile/binary in inner recipe, verify and anchor."""
    binary=Path(os.environ['SWARM_PAEA_PREREQ_BINARY']).resolve();build=Path(os.environ['SWARM_PAEA_PREREQ_BUILD']).resolve();expected=os.environ['SWARM_PAEA_PREREQ_SHA256'];parent=Path(os.environ['SWARM_PAEA_PREREQ_NATIVE_PARENT']).resolve()
    if not binary.is_relative_to(ROOT)or not build.is_relative_to(ROOT):raise ValueError('source-managed-qualified-input-required')
    info=binary.stat()
    if not stat.S_ISREG(info.st_mode)or info.st_uid!=os.getuid()or info.st_mode&0o6000:raise ValueError('ordinary-owned-normal-binary-required')
    if sha(binary.read_bytes())!=expected:raise ValueError('normal-binary-hash')
    proof=Path(tempfile.mkdtemp(prefix='linux-native-proof-',dir=parent));os.chmod(proof,0o700);print('NATIVE_PREREQUISITES_EVIDENCE='+str(proof),flush=True)
    recipe={'schema_version':1,'id':'paea-linux-prerequisites-qualified-invocation','inputs':['tests/paea_linux_native_test.py','tests/paea_linux_native_recipe.py','schemas/paea/linux-prerequisites-v2.schema.json','tools/verification/recipe.py',str(binary.relative_to(ROOT)),str(build.relative_to(ROOT))+'/**/*'],'environment':{'SWARM_PAEA_PREREQ_BINARY':str(binary),'SWARM_PAEA_PREREQ_BUILD':str(build),'SWARM_PAEA_PREREQ_SHA256':expected,'SWARM_PAEA_PREREQ_EVIDENCE':str(proof/'journeys')},'steps':[{'id':'actual-normal-cli-http-refusals-and-recovery','command':['python3','tests/paea_linux_native_test.py'],'expected_exit':0,'required_stdout':['NATIVE_PREREQUISITES_PASS checks=37'],'required_stderr':[]}]}
    (proof/'invocation-recipe.json').write_text(json.dumps(recipe,indent=2)+'\n')
    for action in ['run','verify']:
        args=['python3','tools/verification/recipe.py',action]
        args+=['--recipe',str(proof/'invocation-recipe.json'),'--output',str(proof/'run')]if action=='run'else['--evidence',str(proof/'run')]
        result=subprocess.run(args,cwd=ROOT,capture_output=True);(proof/(action+'.stdout')).write_bytes(result.stdout);(proof/(action+'.stderr')).write_bytes(result.stderr);sys.stdout.buffer.write(result.stdout);sys.stdout.flush();assert result.returncode==0,(action,result.stderr.decode())
    outputs=['invocation-recipe.json','run.stdout','run.stderr','verify.stdout','verify.stderr',*('journeys/'+p.name for p in (proof/'journeys').iterdir()if p.is_file()),*('run/'+p.name for p in (proof/'run').iterdir()if p.is_file())]
    receipt={'schema_version':1,'scope':'qualified-normal-cli-http-isolated','binary_sha256':expected,'files':{n:sha(file_bytes(proof,n))for n in outputs}}
    raw=(json.dumps(receipt,indent=2)+'\n').encode();(proof/'receipt.json').write_bytes(raw);anchor=sha(raw);verify(proof,anchor)
    print('NATIVE_RECEIPT_SHA256='+anchor,flush=True);print('NATIVE_QUALIFIED_RECIPE_PASS checks=37 commands=52 http=16',flush=True)

if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='--verify-sha':verify(Path(os.environ['SWARM_PAEA_PREREQ_NATIVE_EVIDENCE']).resolve(),sys.argv[2]);print('NATIVE_NESTED_DECLARED_OUTPUT_INTEGRITY_PASS')
    elif len(sys.argv)==1:collect()
    else:raise SystemExit('Use no args to collect, or --verify-sha HASH with explicit evidence env')
