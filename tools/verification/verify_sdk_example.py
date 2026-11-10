#!/usr/bin/env python3
"""Qualify an owned Cargo example against a previously qualified normal engine.

This is verification-tool qualification, not a production executable, identity
attestation or install. Read actual hashed depfiles, workspace feature metadata
and a prebuild source manifest; archive every consumed local byte. No Cargo,
Root mutation, provider, retry, watchdog or source rewriting occurs here.
"""
import argparse,json,os,re,shlex,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'tests'))
import rust_reviewer_claim_candidate as sdk
import verify_native_profile as profile

def qualify(source,cargo_log,normal,example,out,invocation):
    """Refuse missing/drifted inputs or fixture features before publishing a tool.

    Only a unique same-inode hashed example is accepted. Relative hashed depfiles
    resolve under the actual Cargo workspace, as in the existing SDK qualifier.
    Shared local feature sets and every normal engine source must match normal;
    additional development dependencies are recorded, never called production.
    Requires retained prebuild argv and explicit environment without compiler
    overrides; this checks the declared execution record, not its authenticity.
    Checkout config must be prebound, archived and identical to normal; new
    config-bearing invocations declare its hashes and LIBSQLITE3_FLAGS before
    Cargo. Neither this record nor features establish actual C flags/runtime.
    An error leaves partial qualification output and no successful result.
    """
    out.mkdir(parents=True,exist_ok=False)
    profile.publish(out/'method.py',Path(__file__).read_bytes())
    call=json.loads(invocation.read_bytes())
    expected=['cargo','build','--manifest-path',str(source/'rust/Cargo.toml'),'--offline','--locked','-p','swarm-cli','--example',example,'--message-format=json']
    profile.require(call['argv']==expected and call['cwd']==str(source),'example-invocation-arguments')
    overrides=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']
    profile.require(call['ambient_override_presence']=={key:False for key in overrides},'example-ambient-compiler-override')
    profile.require(call['profile_environment']=={'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'},'example-profile-environment')
    config_inputs=profile.verify_config_invocation(source,call)
    profile.publish(out/'invocation.json',invocation.read_bytes())
    snapshot=json.loads((source/'sdk-source-snapshot.json').read_bytes())
    profile.require(snapshot['snapshot']==str(source),'example-source-snapshot-mismatch')
    events=[json.loads(line) for line in cargo_log.read_text().splitlines() if line.startswith('{')]
    profile.require(any(r.get('reason')=='build-finished' and r.get('success') is True for r in events),'example-build-not-complete')
    artifacts=[r for r in events if r.get('reason')=='compiler-artifact']
    profile.require(all(not set(r['features'])&{'test-fixtures','test-util'} for r in artifacts),'example-fixture-features')
    local=[r for r in artifacts if Path(r['manifest_path']).is_relative_to(source)]
    baseline=[json.loads(line) for line in (normal/'cargo.stdout').read_text().splitlines() if line.startswith('{')]
    features={r['target']['name']:r['features'] for r in baseline if r.get('reason')=='compiler-artifact'}
    profile.require(local and all(not r['profile']['test'] for r in local),'example-not-default-local-profile')
    profile.require(all(r['features']==features[r['target']['name']] for r in local if r['target']['kind']==['lib'] and r['target']['name'] in features),'example-shared-features-changed')
    engines=[r for r in local if r['target']['name']=='swarm_engine' and r['target']['kind']==['lib']]
    profile.require(len(engines)==1 and not engines[0]['features'],'example-normal-engine-missing')
    targets=[r for r in local if r['target']['name']==example and r['target']['kind']==['example'] and r['executable']]
    profile.require(len(targets)==1,'example-target-missing-or-ambiguous')
    executable=Path(targets[0]['executable'])
    pattern=re.compile(re.escape(example.replace('-','_'))+'-[0-9a-f]{16}')
    matches=[p for p in executable.parent.iterdir() if pattern.fullmatch(p.name) and p.is_file() and os.path.samefile(executable,p)]
    profile.require(len(matches)==1,'example-hashed-owner-missing-or-ambiguous')
    paths={source/'rust/Cargo.toml',source/'rust/Cargo.lock',*(source/'rust/crates').glob('*/Cargo.toml'),*profile.cargo_config_inputs(source)};deps={}
    for r in local:
        dep=matches[0].with_suffix('.d') if r==targets[0] else sdk.sdk_artifact_depfile(r)
        text=dep.read_text();deps[str(dep)]=text;first=text.splitlines()[0]
        profile.require(': ' in first,'example-depfile-invalid')
        for name in shlex.split(first.split(': ',1)[1]):
            p=Path(name);paths.add(p if p.is_absolute() else source/'rust'/p)
    actual=profile.manifest(source,paths)
    profile.effective_unchanged(source,snapshot['files'],actual)
    original=json.loads((normal/'after.json').read_bytes())
    profile.require(profile.config_manifest(original)==profile.config_manifest(actual),'example-config-bytes-differ-from-normal')
    profile.require(config_inputs==profile.config_manifest(actual),'example-config-changed-after-invocation')
    core={n:h for n,h in original.items() if n.startswith('rust/crates/swarm-engine/src/')}
    profile.require(bool(core) and all(actual.get(n)==h for n,h in core.items()),'example-engine-bytes-differ-from-normal')
    profile.publish(out/'depfiles.json',json.dumps(deps,indent=2)+'\n')
    profile.publish(out/'actual-inputs.json',json.dumps(actual,indent=2)+'\n')
    archived=profile.archive_effective_sources(source,out,actual)
    profile.publish(out/'sdk-binary',executable.read_bytes());(out/'sdk-binary').chmod(0o555)
    result=dict(status='VERIFIED_SDK_EXAMPLE_DEFAULT_CORE',example=example,engine_features_empty=True,shared_workspace_features_match_normal=True,test_profile=False,normal_profile_sha256=json.loads((normal/'result.json').read_bytes())['sha256'],normal_engine_inputs_identical=len(core),normal_config_inputs_identical=len(config_inputs),cargo_config_inputs=config_inputs,configuration_scope='Checkout config bytes and declared pre-Cargo environment; not observed C flags or runtime capability',actual_input_files=len(actual),sdk_only_local_dependencies=[r['target']['name'] for r in local if r['target']['kind']==['lib'] and r['target']['name'] not in features],archive=archived,binary_sha256=profile.digest(out/'sdk-binary'),installed=False,global_watchdog=None)
    profile.publish(out/'profile.json',json.dumps(result,indent=2)+'\n')
    print('PASS qualified SDK example and actual default engine bytes',flush=True)
    return result

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['source','cargo-log','normal','output','invocation']:p.add_argument('--'+key,required=True,type=Path)
    p.add_argument('--example',required=True);a=p.parse_args()
    qualify(a.source.resolve(),a.cargo_log.resolve(),a.normal.resolve(),a.example,a.output.resolve(),a.invocation.resolve())
