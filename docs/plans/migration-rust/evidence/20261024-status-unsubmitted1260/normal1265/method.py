#!/usr/bin/env python3
"""Qualify a default native Cargo build and freeze its exact executable.

Bind source bytes before compilation, then check actual depfiles and artifact
features, not an all-features workspace. Literal includes and web/frontend files
are candidates only: depfiles decide the effective set. Every effective source
is archived and verified so later concurrent edits cannot erase replay inputs.
Checkout-local Cargo config is also prebound/archived even when depfiles omit it;
its bytes and declared environment do not prove actual C flags or capabilities.
No process graph sandbox,
independent review, installation, paid provider, retry or global watchdog.
"""
import argparse,hashlib,json,os,re,shlex,subprocess,tempfile,tarfile
from pathlib import Path


def require(condition,reason):
    """Reject a missing proof even under Python -O; never return an empty PASS."""
    if not condition:raise ValueError(reason)


def digest(path):
    """Hash complete observed bytes; this proves neither origin nor authority."""
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def publish(path,data):
    """Publish adjacent verified/fsynced bytes atomically; preserve existing data.

    A rename followed by sync failure remains applied/uncertain, not rollback.
    Failure keeps its original cause; callers retain the enclosing evidence Root.
    """
    if isinstance(data,str):data=data.encode()
    descriptor,name=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
    with os.fdopen(descriptor,'wb') as stream:
        stream.write(data);stream.flush();os.fsync(stream.fileno())
    require(Path(name).read_bytes()==data,'profile-publish-readback')
    os.replace(name,path)
    directory=os.open(path.parent,os.O_DIRECTORY)
    try:os.fsync(directory)
    finally:os.close(directory)


def manifest(root,paths):
    """Require regular existing inputs within the checkout, with canonical keys.

    This handles effective depfile paths; unresolved/external inputs cannot be
    silently omitted. No source content or private environment values is printed.
    """
    values={}
    for path in paths:
        path=path.resolve();require(path.is_file() and path.is_relative_to(root),'profile-input-absent-or-external')
        values[str(path.relative_to(root))]=digest(path)
    return values


CARGO_CONFIG_NAMES=('.cargo/config','.cargo/config.toml')


def cargo_config_inputs(root):
    """Return checkout-local Cargo configuration selected from the build cwd.

    Both Cargo filename spellings are retained if present; Cargo resolves their
    precedence. Aliased/symlinked config paths refuse: their canonical manifest
    key could otherwise hide config removal. Missing historical config is valid.
    Ancestor/CARGO_HOME configuration and C compiler flags are not authenticated
    by this local inventory. Exact config bytes are evidence, not runtime proof.
    """
    paths=[root/name for name in CARGO_CONFIG_NAMES
           if (root/name).exists() or (root/name).is_symlink()]
    require(all(path.resolve()==path for path in paths),'profile-cargo-config-symlink-unsupported')
    return paths


def config_manifest(inputs):
    """Project an existing manifest onto Cargo config keys, preserving absence."""
    return {name:inputs[name] for name in CARGO_CONFIG_NAMES if name in inputs}


def verify_config_invocation(root,call):
    """Bind declared prebuild config and relevant pre-Cargo environment.

    New configs require explicit invocation declarations. Historical invocations
    with no checkout config may omit these additive fields. This validates the
    retained record, not its authenticity or Cargo's eventual C flags.
    """
    current=manifest(root,cargo_config_inputs(root))
    if current or 'cargo_config_inputs' in call:
        require(call.get('cargo_config_inputs')==current,'profile-invocation-config-mismatch')
    if current or 'cargo_environment_before_config' in call:
        environment=call.get('cargo_environment_before_config')
        require(isinstance(environment,dict) and set(environment)=={'LIBSQLITE3_FLAGS'}
                and (environment['LIBSQLITE3_FLAGS'] is None or isinstance(environment['LIBSQLITE3_FLAGS'],str)),
                'profile-invocation-config-environment-missing-or-invalid')
    return current


def dependencies(root,depfile):
    """Read the actual Cargo Make depfile and all workspace manifest inputs.

    Linux paths follow the observed Cargo escaping; unsupported/missing data
    refuses. Checkout-local Cargo config is an effective invocation input even
    when rustc depfiles omit it. This is not an arbitrary compiler parser.
    """
    lines=depfile.read_text().splitlines();require(bool(lines) and ': ' in lines[0],'profile-depfile-invalid')
    paths=[Path(p) for p in shlex.split(lines[0].split(': ',1)[1])]
    paths += [root/'rust/Cargo.toml',root/'rust/Cargo.lock',*(root/'rust/crates').glob('*/Cargo.toml'),*cargo_config_inputs(root)]
    return manifest(root,paths)


def candidates(root,prior=None):
    """Prebind candidate bytes before Cargo, including newly added embeds.

    Literal include syntax is recognized conservatively; candidate discovery is
    not proof of complete compilation. Actual postbuild depfiles must be a subset
    with identical hashes. Prior depfiles cannot enlarge the declared input set:
    the optional prior argument is retained only for call compatibility. An old
    build output is not producer authority. Unused candidate changes do not prove
    source drift in the effective build. Unknown/generated includes require a
    diagnosed producer contract and fresh evidence, never admission from a cache.
    Local Cargo config is prebound independently of Rust include discovery.
    """
    paths={root/'rust/Cargo.toml',root/'rust/Cargo.lock',*(root/'rust/crates').glob('*/Cargo.toml'),*cargo_config_inputs(root)}
    rust=set((root/'rust/crates').glob('*/src/**/*.rs'))
    rust.update((root/'rust/crates').glob('*/build.rs'));paths.update(rust)
    for folder in ['web','frontend']:
        paths.update(p for p in (root/folder).rglob('*') if p.is_file())
    for source in rust:
        for match in re.finditer(r'include_(?:str|bytes)!\s*\(\s*("(?:[^"\\]|\\.)*")',source.read_text()):
            try:relative=json.loads(match[1])
            except json.JSONDecodeError:continue
            path=(source.parent/relative).resolve()
            if path.is_file() and path.is_relative_to(root):paths.add(path)
    bound=manifest(root,paths)
    return bound


def effective_unchanged(root,before,actual):
    """Require every actual compiled input to have its original prebuild bytes.

    Extra candidates are not promoted to actual dependencies. A missing newly
    effective file or a changed used file refuses without inventing a baseline.
    Cargo config removal/appearance also refuses, unlike unused candidates.
    """
    require(config_manifest(before)==config_manifest(actual),'profile-cargo-config-changed-or-removed')
    missing=[name for name in actual if name not in before]
    changed=[name for name,value in actual.items() if name in before and before[name]!=value]
    require(not missing,'profile-effective-input-not-prebound:'+','.join(missing))
    require(not changed,'profile-effective-input-changed:'+','.join(changed))
    require(all(digest(root/name)==value for name,value in actual.items()),'profile-effective-input-changed-after-read')


def archive_effective_sources(root,out,actual):
    """Retain every effective input byte, validating contents and post-read state.

    Only prebound effective compiler/config inputs, not runtime Roots or all
    dirty source candidates. Cargo configuration is retained even without a
    corresponding rustc depfile entry; it does not prove observed C flags.
    Adjacent temporary archive is verified before replacement; failures retain
    originals and refuse qualification. Hashes are integrity, not authenticity.
    No guessed payload ceiling or copy of the development checkout is introduced.
    """
    descriptor,temporary=tempfile.mkstemp(dir=out,prefix='.effective-sources-')
    os.close(descriptor);temporary=Path(temporary)
    with tarfile.open(temporary,'w:gz') as archive:
        for name,sha in actual.items():
            require(digest(root/name)==sha,'profile-archive-source-changed')
            archive.add(root/name,arcname=name,recursive=False)
    with tarfile.open(temporary,'r:gz') as archive:
        names=archive.getnames()
        require(len(names)==len(actual) and set(names)==set(actual),'profile-archive-members')
        for name,sha in actual.items():
            require(hashlib.sha256(archive.extractfile(name).read()).hexdigest()==sha,'profile-archive-bytes')
    require(all(digest(root/name)==sha for name,sha in actual.items()),'profile-source-changed-after-archive')
    publish(out/'effective-sources.tar.gz',temporary.read_bytes());temporary.unlink()
    proof=dict(inputs=len(actual),sha256=digest(out/'effective-sources.tar.gz'),all_members_verified=True,source_stable_after_archive=True)
    publish(out/'effective-sources.json',json.dumps(proof,indent=2)+'\n')
    return proof


def build_metadata_binding(environment):
    """Bind presence and exact compiler declaration bytes without logging values.

    This is a declared invocation input only, not proof rustc consumed it. The
    public binary version recipe separately verifies consumption. Missing and
    present-empty differ; malformed declarations are refused by the binary.
    No runtime environment or credential is copied to the public result.
    """
    raw=environment.get('SWARM_NATIVE_BUILD_METADATA')
    return dict(present=raw is not None,sha256=None if raw is None else hashlib.sha256(raw.encode()).hexdigest())


def run(root,out,target):
    """Build offline/locked without fixture features, retain exact observations.

    This command does not install or restart. Cargo/manifest/IO/qualification
    failures preserve stdout/stderr and leave the build unqualified. The caller
    must keep sources stable and avoid competing writers to this target directory.
    """
    out.mkdir(parents=False,exist_ok=False)
    publish(out/'method.py',Path(__file__).read_bytes())
    try:
        depfile=target/'debug/swarm-rust.d';before=candidates(root)
        publish(out/'before.json',json.dumps(before,indent=2)+'\n')
        env={**os.environ,'CARGO_TARGET_DIR':str(target),'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0'}
        names=['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME']
        metadata=build_metadata_binding(env)
        presence={key:bool(env.get(key)) for key in names};require(not any(presence.values()),'profile-ambient-compiler-override')
        args=['cargo','build','--manifest-path','rust/Cargo.toml','--offline','--locked','-p','swarm-cli','--bin','swarm-rust','--message-format=json']
        publish(out/'invocation.json',json.dumps(dict(argv=args,cwd=str(root),target_dir=str(target),native_build_metadata=metadata,ambient_override_presence=presence,cargo_config_inputs=config_manifest(before),cargo_environment_before_config={'LIBSQLITE3_FLAGS':env.get('LIBSQLITE3_FLAGS')},profile_environment={key:env[key] for key in ['CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL']},global_watchdog=None),indent=2)+'\n')
        with (out/'cargo.stdout').open('xb') as stdout,(out/'cargo.stderr').open('xb') as stderr:
            result=subprocess.run(args,cwd=root,env=env,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
        require(result.returncode==0,'profile-cargo-failed')
        events=[json.loads(line) for line in (out/'cargo.stdout').read_text().splitlines()]
        require(any(e.get('reason')=='build-finished' and e.get('success') is True for e in events),'profile-build-finished-missing')
        artifacts=[e for e in events if e.get('reason')=='compiler-artifact'];require(bool(artifacts),'profile-no-artifacts')
        for artifact in artifacts:require(not set(artifact['features'])&{'test-fixtures','test-util'},'profile-fixture-feature')
        selected=[e for e in artifacts if e['target']['name'] in ['swarm_engine','swarm_web','swarm_cli','swarm-rust']]
        require(len(selected)==4 and all(not e['profile']['test'] and e['features']==[] for e in selected),'profile-core-artifacts-invalid')
        fingerprints={}
        for artifact in selected:
            if artifact['target']['name']=='swarm-rust':continue
            filename=next(p for p in artifact['filenames'] if p.endswith('.rlib'));match=re.fullmatch(r'lib(.+)-([0-9a-f]+)\.rlib',Path(filename).name);require(match is not None,'profile-library-name')
            package=artifact['target']['name'].replace('_','-');path=target/'debug/.fingerprint'/(package+'-'+match[2])/('lib-'+artifact['target']['name']+'.json');value=json.loads(path.read_bytes());require(value['rustflags']==[],'profile-compiler-flags')
            fingerprints[str(path.relative_to(root))]=dict(sha256=digest(path),features=value['features'],rustflags=value['rustflags'],compile_kind=value['compile_kind'])
        actual=dependencies(root,depfile);publish(out/'after.json',json.dumps(actual,indent=2)+'\n');effective_unchanged(root,before,actual)
        source_archive=archive_effective_sources(root,out,actual)
        binary=Path(next(e for e in selected if e['target']['name']=='swarm-rust')['executable']);raw=binary.read_bytes();sha=hashlib.sha256(raw).hexdigest();frozen=root/'.cache/rust-migration-binaries'/('swarm-rust-'+sha);frozen.parent.mkdir(parents=True,exist_ok=True)
        if frozen.exists():require(digest(frozen)==sha,'profile-frozen-binary-mismatch')
        else:publish(frozen,raw);frozen.chmod(0o700)
        require(digest(frozen)==sha,'profile-frozen-readback')
        proof=dict(native_build_metadata=metadata,status='VERIFIED_DEFAULT_NATIVE_PROFILE',binary=str(frozen.relative_to(root)),sha256=sha,artifacts_observed=len(artifacts),selected_core_artifacts=4,all_core_features_empty=True,all_core_test_false=True,no_fixture_feature=True,ambient_override_presence=presence,actual_dependencies_stable=True,cargo_config_inputs=config_manifest(actual),configuration_scope='Checkout config bytes and declared pre-Cargo environment; not observed C flags or runtime capability',effective_sources_archived=source_archive,installed=False,global_watchdog=None)
        publish(out/'artifacts.json',json.dumps(selected,indent=2)+'\n');publish(out/'fingerprints.json',json.dumps(fingerprints,indent=2)+'\n');publish(out/'result.json',json.dumps(proof,indent=2)+'\n');return proof
    except BaseException as error:
        try:publish(out/'failure.json',json.dumps(dict(status='FAIL',type=type(error).__name__,reason=str(error),scope='Build unqualified; inspect original Cargo channels',global_watchdog=None),indent=2)+'\n')
        except BaseException as secondary:raise BaseExceptionGroup('Qualification and evidence publication failed',[error,secondary])
        raise


def main():
    """Qualify the explicit checkout/cache/output, without hidden execution caps."""
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--root',type=Path,default=Path.cwd());parser.add_argument('--target-dir',type=Path);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();root=args.root.resolve();target=(args.target_dir or root/'rust/target/r97').resolve();require(target.is_relative_to(root),'profile-target-outside-checkout');print(json.dumps(run(root,args.output.resolve(),target)),flush=True)


if __name__=='__main__':main()
