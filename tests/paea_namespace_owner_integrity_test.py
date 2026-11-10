#!/usr/bin/env python3
"""Exercise nested local proof verification on fresh private copies only.
Originals remain read-only. Self-selected fixture SHA tests verifier behavior,
not trusted provenance; runtime anchor stays in the enclosing recipe stdout.
"""
from pathlib import Path
import os,subprocess,hashlib,json,shutil
ROOT=Path(__file__).resolve().parents[1]

def sha(path):
    """Hash observed fixture bytes without formatting private content."""
    with path.open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def clone(original,destination,names):
    """Fresh private hardlinks except the only byte-mutated file, copied distinctly.
    Tests remove only copy links, never edit linked originals or source binaries.
    """
    destination.mkdir(mode=0o700)
    for name in ['result.json',*names]:
        if name=='test.stderr':shutil.copy2(original/name,destination/name)
        else:os.link(original/name,destination/name)
    assert (original/'test.stderr').stat().st_ino!=(destination/'test.stderr').stat().st_ino

def check(path,anchor,out,label,expected,cause):
    """Run the actual read-only verifier; retain argv/exit/original channels."""
    argv=['python3','tests/paea_namespace_owner_profile_test.py','--verify-sha',anchor]
    run=subprocess.run(argv,cwd=ROOT,env={**os.environ,'SWARM_PAEA_NAMESPACE_OWNER_EVIDENCE':str(path)},capture_output=True)
    for suffix,raw in [('stdout',run.stdout),('stderr',run.stderr)]:
        with (out/(label+'.'+suffix)).open('xb')as stream:stream.write(raw)
    record={'argv':argv,'evidence':str(path),'exit':run.returncode,'expected':expected}
    with (out/(label+'.json')).open('x')as stream:json.dump(record,stream,indent=2)
    assert run.returncode==expected,'integrity-guard-exit'
    assert cause.encode()in(run.stdout if expected==0 else run.stderr),'integrity-guard-cause'

def main():
    """Valid controls precede byte/kind faults; original hashes remain unchanged.
    Fixed finite fixture matrix, no product limit/retry/watchdog or service launch.
    """
    original=Path(os.environ['SWARM_PAEA_NAMESPACE_OWNER_EVIDENCE']);metadata=json.loads((original/'result.json').read_text());anchor=sha(original/'result.json');out=original/'verification-copies';out.mkdir(mode=0o700,exist_ok=False);names=list(metadata['artifacts']);before={n:sha(original/n)for n in ['result.json',*names]}
    check(original,anchor,out,'original-before',0,'NAMESPACE_OWNER_SUBEVIDENCE_VERIFIED')
    for label,kind,cause in [('changed-bytes','bytes','fixture-artifact-bytes'),('metadata-symlink','metadata','fixture-result-kind'),('artifact-symlink','artifact','fixture-artifact-kind')]:
        copy=out/label;clone(original,copy,names);check(copy,anchor,out,label+'-control',0,'NAMESPACE_OWNER_SUBEVIDENCE_VERIFIED')
        if kind=='bytes':
            with (copy/'test.stderr').open('ab')as stream:stream.write(b'\nchanged-private-copy\n')
        else:
            name='result.json'if kind=='metadata'else'test.stdout';(copy/name).unlink();(copy/name).symlink_to(original/name)
        check(copy,anchor,out,label+'-refused',1,cause)
    check(original,anchor,out,'original-after',0,'NAMESPACE_OWNER_SUBEVIDENCE_VERIFIED');assert before=={n:sha(original/n)for n in before},'original-proof-mutated'
    with (out/'result.json').open('x')as stream:json.dump({'status':'PASS_LOCAL_VERIFIER_REGRESSION','controls':5,'refusals':3,'original_intact':True,'scope':'Local fixture integrity, not provenance/security acceptance'},stream,indent=2)
    print('NAMESPACE_OWNER_INTEGRITY_PASS controls=5 refusals=3 original_intact=true',flush=True)
if __name__=='__main__':main()
