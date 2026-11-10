#!/usr/bin/env python3
"""Freeze/execute the exact namespace-owner test target with observed normal rlib.
Not a daemon, full effective dependency graph, image approval or distinctUID proof.
Explicit fresh directory or a fresh child of the explicit parent; no global
watchdog/retry/provider/live database. Integrity guards run before final receipt.
"""
from pathlib import Path
import subprocess,os,json,hashlib,stat,sys,re,tempfile
ROOT=Path(__file__).resolve().parents[1]
NAME="namespace_owner_nondumpable_child_refuses_host_domain_but_accepts_owned_userns"

def sha(path):
    """Hash exact observed bytes; not an identity/approval attestation."""
    with Path(path).open('rb')as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()

def save(path,raw):
    """Only fresh owned evidence, fsync before use; never replace past results."""
    with path.open('xb')as stream:
        stream.write(raw);stream.flush();os.fsync(stream.fileno())

def verify(out,expected):
    """Verify nested originals against the SHA anchored in outer recipe stdout.
    Fixed local file names only; no symlink/path traversal or full-graph/security
    attestation. Read-only failures preserve their actual cause and never pass.
    """
    assert re.fullmatch('[0-9a-f]{64}',expected),'fixture-anchor-format'
    assert not (out/'result.json').is_symlink()and (out/'result.json').is_file(),'fixture-result-kind'
    assert sha(out/'result.json')==expected,'fixture-result-anchor-mismatch'
    metadata=json.loads((out/'result.json').read_text())
    names={'override-presence.json','cargo.stdout','cargo.stderr','namespace-owner-test-harness','test.stdout','test.stderr'}
    assert metadata['status']=='PASS'and set(metadata['artifacts'])==names,'fixture-artifact-inventory'
    assert metadata['normal_library_artifact']['features']==[]and not metadata['normal_library_artifact']['profile']['test']and metadata['test_artifact']['profile']['test'],'fixture-artifact-profile'
    assert metadata['artifacts']['namespace-owner-test-harness']==metadata['test_harness_sha256'],'fixture-harness-binding'
    for name,digest in metadata['artifacts'].items():
        file=out/name;assert not file.is_symlink()and file.is_file(),'fixture-artifact-kind';assert sha(file)==digest,'fixture-artifact-bytes'
    print('NAMESPACE_OWNER_SUBEVIDENCE_VERIFIED files=6 harness_test=true',flush=True)

def main():
    """Compile selected target, require normal channel artifact, run frozen harness.
    Retain original compiler/test channels and before/after executable identity.
    Failure remains evidence, never an excuse to retry or claim zero tests pass.
    """
    explicit=os.environ.get('SWARM_PAEA_NAMESPACE_OWNER_EVIDENCE')
    if len(sys.argv)==3 and sys.argv[1]=='--verify-sha':
        assert explicit,'fixture-verification-directory-required'
        verify(Path(explicit),sys.argv[2]);return
    assert len(sys.argv)==1,'fixture-mode'
    if explicit:
        out=Path(explicit);out.mkdir(mode=0o700,parents=True,exist_ok=False)
    else:
        parent=Path(os.environ['SWARM_PAEA_NAMESPACE_OWNER_EVIDENCE_PARENT'])
        assert parent.is_absolute()and parent.is_dir(),'fixture-evidence-parent-required'
        out=Path(tempfile.mkdtemp(prefix='namespace-owner-proof-',dir=parent))
    print('NAMESPACE_OWNER_EVIDENCE_PATH='+str(out),flush=True)
    overrides={k:bool(os.environ.get(k))for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN']}
    save(out/'override-presence.json',(json.dumps(overrides,indent=2)+'\n').encode());assert not any(overrides.values()),'fixture-ambient-compiler-override'
    args=['cargo','test','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-linux-identity-channel','--test','namespace_owner','--no-run','--message-format=json']
    build=subprocess.run(args,cwd=ROOT,capture_output=True);save(out/'cargo.stdout',build.stdout);save(out/'cargo.stderr',build.stderr);assert build.returncode==0,'fixture-compile-refused'
    rows=[json.loads(line)for line in build.stdout.decode().splitlines()if line.startswith('{')];artifacts=[r for r in rows if r.get('reason')=='compiler-artifact'];core=[r for r in artifacts if r['target']['name']=='swarm_linux_identity_channel'and not r['profile']['test']];tests=[r for r in artifacts if r['target']['name']=='namespace_owner'and r['profile']['test']and r.get('executable')]
    assert len(core)==1 and core[0]['features']==[],'normal-channel-artifact-required';assert len(tests)==1,'exact-test-harness-required'
    source=Path(tests[0]['executable']);raw=source.read_bytes();digest=hashlib.sha256(raw).hexdigest();binary=out/'namespace-owner-test-harness';save(binary,raw);binary.chmod(stat.S_IRUSR|stat.S_IXUSR);assert sha(binary)==digest
    command=[str(binary),'--nocapture'];run=subprocess.run(command,cwd=ROOT,capture_output=True);save(out/'test.stdout',run.stdout);save(out/'test.stderr',run.stderr)
    expected=[NAME+' ... ok','namespace_owner_child_failure_keeps_native_diagnostic ... ok','2 passed; 0 failed','NAMESPACE_OWNER_DIAGNOSTIC_PASS controls=2 original_loss_observed=true errno_retained=true','NAMESPACE_OWNER host-nondumpable-refused','NAMESPACE_OWNER profile=owned-userns stage=1 accepted','NAMESPACE_OWNER_PASS profiles=2 stages=6 external_uid_shared=true host_caps_zero=true']
    assert run.returncode==0 and all(s in run.stdout.decode()for s in expected),'actual-six-phase-test-not-passed';assert sha(binary)==digest,'frozen-harness-changed'
    result={'status':'PASS','scope':'Frozen test harness with observed default normal channel rlib; not installed daemon/distinctUID/human/confinement/full compiler graph','compiler_argv':args,'test_argv':command,'test_harness_sha256':digest,'normal_library_artifact':core[0],'test_artifact':tests[0],'global_watchdog':None,'source_binary_observed_sha256':sha(source)}
    result['artifacts']={name:sha(out/name)for name in ['override-presence.json','cargo.stdout','cargo.stderr','namespace-owner-test-harness','test.stdout','test.stderr']}
    save(out/'result.json',(json.dumps(result,indent=2)+'\n').encode())
    check=subprocess.run(['python3','tests/paea_namespace_owner_integrity_test.py'],cwd=ROOT,env={**os.environ,'SWARM_PAEA_NAMESPACE_OWNER_EVIDENCE':str(out)},capture_output=True)
    save(out/'integrity.stdout',check.stdout);save(out/'integrity.stderr',check.stderr)
    assert check.returncode==0 and b'NAMESPACE_OWNER_INTEGRITY_PASS controls=5 refusals=3 original_intact=true'in check.stdout,'fixture-integrity-guards-refused'
    print(check.stdout.decode(),end='',flush=True)
    print('NAMESPACE_OWNER_EVIDENCE_SHA256='+sha(out/'result.json'),flush=True)
    print('NAMESPACE_OWNER_PROFILE_PASS named_tests=2 profiles=2 stages=6 channel_features_empty=true harness_test=true',flush=True)
if __name__=='__main__':main()
