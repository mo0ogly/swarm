"""Number and qualify local Rust deliveries before replacing any running server.

Cargo's observed default-profile depfiles determine effective inputs. A broader
candidate fingerprint only invalidates cache reuse; unused inputs do not advance
the delivery number. No Go fallback, provider call or external publication.
Caller holds the build lock and preserves source/target exclusivity.
"""
import datetime,hashlib,importlib.util,json,os,re,signal,subprocess,sys,tempfile
from pathlib import Path
import urllib.request


def native_builder(source):
    """Load the repository's maintained native builder lazily; configure needs none.

    Import failures propagate before any installation; no alternate engine is used.
    """
    spec=importlib.util.spec_from_file_location('swarm_native_builder',source/'tools/rust-migration/build_native.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


def source_identity(source,effective=None):
    """Hash candidate/effective bytes plus delivery inputs and actual toolchain.

    None selects conservative candidates for cache invalidation only. Supplying
    observed depfile rows selects effective bytes for numbering. Git is observed,
    not authenticated; configuration is bounded by the native collector's limits.
    Missing inputs and tool failures refuse without mutating installed artifacts.
    """
    profile=native_builder(source).profile
    rows=profile.candidates(source) if effective is None else {name:profile.digest(source/name) for name in effective}
    for name in ['build.sh','tools/swarm_version.py','tools/swarm_local.py','tools/rust-migration/build_native.py','tools/verification/verify_native_profile.py','version_history.json']:
        rows[name]=profile.digest(source/name)
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=source,text=True).strip()
    modified=bool(subprocess.check_output(['git','status','--porcelain','--untracked-files=normal'],cwd=source,text=True))
    config={name:os.environ.get(name) for name in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_BUILD_TARGET','RUSTUP_TOOLCHAIN','CARGO_HOME','LIBSQLITE3_FLAGS']}
    config['cargo']=subprocess.check_output(['cargo','--version'],cwd=source,text=True).strip()
    config['rustc']=subprocess.check_output(['rustc','-vV'],cwd=source,text=True).strip()
    raw=json.dumps(dict(commit=commit,modified=modified,configuration=config,inputs=rows),sort_keys=True).encode()
    return hashlib.sha256(raw).hexdigest(),rows


def select_version(published, digest, previous):
    match = re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:[-+].*)?", published)
    if not match:
        raise ValueError("A valid published base version is required")
    major, minor, patch = map(int, match.groups())
    prefix = f"v{major}.{minor}.{patch + 1}-dev."
    previous = previous or {}
    prior = previous.get("version", "")
    if prior.startswith(prefix) and prior[len(prefix):].isdigit():
        if previous.get("source_digest") == digest:
            return prior
        return prefix + str(int(prior[len(prefix):]) + 1)
    return prefix + "1"



def session_request(address,root,path):
    """Construct a local cookie-authenticated native request; never log its token.

    Reject symlinks/public/nonowned credentials before sending them. A caller
    must already own the server/listener; a pathname alone proves no identity.
    """
    import stat
    key=root/'.swarm'/('web-session-'+hashlib.sha256(address.encode()).hexdigest()[:16])
    descriptor=os.open(key,os.O_RDONLY|os.O_NOFOLLOW)
    with os.fdopen(descriptor,'r') as stream:
        info=os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_uid!=os.getuid() or info.st_mode&0o077:
            raise ValueError('Private owned session credential required')
        token=stream.read().strip()
    if not re.fullmatch('session-[A-Za-z0-9_-]{24}',token):
        raise ValueError('Invalid native session credential')
    return urllib.request.Request('http://'+address+path,headers={'Cookie':'swarm_session_'+hashlib.sha256(address.encode()).hexdigest()[:12]+'='+token})


def verify_candidate(candidate,version,notes,metadata=None):
    """Probe actual protected Rust metadata in an isolated Root, then drain SIGINT.

    Optional metadata binds every declared binary field, not only its version.
    No live datastore/provider is used. Initialization/bootstrap/HTTP/close errors
    refuse qualification, retain the previous server and preserve original causes.
    No global verification watchdog is invented. Per-request timeout only bounds
    one local probe; launcher startup policy separately owns startup waiting.
    """
    print('Vérification isolée du serveur Rust et de sa version…',flush=True)
    with tempfile.TemporaryDirectory(prefix='swarm-native-delivery-') as temporary:
        root=Path(temporary)
        subprocess.run([str(candidate),'--root',str(root),'init'],check=True,stdout=subprocess.DEVNULL)
        with tempfile.TemporaryFile() as errors:
            app=subprocess.Popen([str(candidate),'--root',str(root),'--json','web','127.0.0.1:0'],stdout=subprocess.PIPE,stderr=errors,text=True)
            try:
                first=app.stdout.readline()
                if not first:raise RuntimeError('Native candidate exited before listener announcement')
                bound=json.loads(first)
                if bound.get('event')!='web-bound':raise ValueError('Native listener announcement invalid')
                request=session_request(bound['address'],root,'/api/v1/native-version')
                with urllib.request.urlopen(request,timeout=1) as response:actual=json.load(response)
                if actual['binary']['version']!=version or actual['history'].get('local_changes')!=notes:
                    raise ValueError('Native candidate version/notes mismatch')
                if metadata is not None and actual['binary']!={**{k:v for k,v in metadata.items() if k!='schema_version'},'provenance':'injected'}:
                    raise ValueError('Native candidate metadata mismatch')
            finally:
                if app.poll() is None:app.send_signal(signal.SIGINT)
                # Keep draining observations while shutdown completes; no pipe deadlock.
                app.communicate()
                if app.returncode!=0:
                    failure=RuntimeError('Native candidate did not stop cleanly')
                    primary=sys.exception()
                    if primary is not None:raise BaseExceptionGroup('Native verification and shutdown failed',[primary,failure])
                    raise failure


def prepare(source,binary):
    """Build/freeze/probe Rust before any live stop; return a replaceable candidate.

    Caller owns build lock. Exact observed dependencies determine numbering. A
    changed dependency graph may require one corrected-number build; failures
    preserve prior binary/number and retained evidence. State publication is
    atomic, but a durability error never implies rollback. Unused candidate-file
    edits invalidate cache only; actual profile bytes, initial/final delivery tools,
    notes, Git and toolchain facts must remain equal. These are observations, not
    an atomic filesystem lease. Cached reuse revalidates retained qualification; missing files rebuild,
    incoherent evidence refuses. No paid calls occur.
    """
    manifest=json.loads((source/'version_history.json').read_text());notes=manifest.get('local_changes')
    valid=isinstance(notes,dict) and isinstance(notes.get('items'),list) and bool(notes['items'])
    if valid:valid=all(isinstance(item,dict) and all(isinstance(item.get(lang),str) and item[lang].strip() for lang in ['fr','en']) for item in [notes.get('summary'),*notes['items']])
    if not valid:raise ValueError('Update version_history.json local_changes in French and English')
    builder=native_builder(source);profile=builder.profile
    candidate_digest,_=source_identity(source)
    # Empty effective selection binds delivery tools/notes, Git and toolchain,
    # while unused candidates remain only a cache-invalidation inventory.
    delivery_digest,_=source_identity(source,{})
    state_path=source/'deploy/local-build.json'
    previous=json.loads(state_path.read_text()) if state_path.exists() else None
    numbering_previous=previous
    if previous and (previous.get('schema_version')!=2 or previous.get('engine')!='rust'):previous=None
    published=manifest['releases'][0]['version'] if manifest.get('releases') else 'v0.0.0'
    override=os.environ.get('SWARM_VERSION')
    if override is not None and not builder.release(override):raise ValueError('Invalid SWARM_VERSION')
    if previous and previous.get('candidate_digest')==candidate_digest and (override is None or override==previous['version']):
        for artifact in [binary,Path(previous['artifact'])]:
            if artifact.is_file() and profile.digest(artifact)==previous['binary_sha256']:
                try:_read_prepared(source,previous['binary_sha256'])
                except FileNotFoundError:
                    print('Preuves de livraison absentes ; reconstruction qualifiée requise.',flush=True)
                    break
                print('Version Rust locale inchangée : '+previous['version'],flush=True)
                return artifact,previous
    settings_path=source/'deploy/local-web.json'
    settings=json.loads(settings_path.read_text()) if settings_path.exists() else {}
    target=Path(os.environ.get('SWARM_RUST_TARGET_DIR',settings.get('rust_target_dir',str(source/'.cache/rust-build-local')))).resolve()
    evidence_base=source/'.cache/local-delivery';evidence_base.mkdir(parents=True,exist_ok=True)
    evidence=Path(tempfile.mkdtemp(prefix='rust-',dir=evidence_base))
    prior_effective=previous.get('effective_inputs') if previous else None
    predicted=source_identity(source,prior_effective)[0] if prior_effective and all((source/n).is_file() for n in prior_effective) else 'new-effective-inputs'
    version=override if override is not None else select_version(published,predicted,numbering_previous)
    print('Construction Rust : '+version,flush=True)
    built=builder.build(source,evidence/'build',target,version)
    actual=json.loads((evidence/'build/profile/after.json').read_text())
    digest,inputs=source_identity(source,actual)
    if any(inputs.get(name)!=value for name,value in actual.items()):
        raise RuntimeError('Native effective inputs changed after qualified build; previous server retained')
    selected=override if override is not None else select_version(published,digest,numbering_previous)
    if selected!=version:
        version=selected
        built=builder.build(source,evidence/'corrected-number',target,version)
        corrected=json.loads((evidence/'corrected-number/profile/after.json').read_text())
        if corrected!=actual:raise RuntimeError('Native effective inputs changed during corrected-number build')
    # Refresh the broad cache key before the final actual-input/control checks.
    # Unused tests may change; qualified sources and delivery facts may not.
    candidate_digest,_=source_identity(source)
    if source_identity(source,{})[0]!=delivery_digest or source_identity(source,actual)[0]!=digest:
        raise RuntimeError('Native inputs changed during build; previous server retained')
    frozen=Path(built['binary']);binary.parent.mkdir(parents=True,exist_ok=True)
    fd,name=tempfile.mkstemp(prefix=binary.name+'.candidate-',dir=binary.parent);os.close(fd);candidate=Path(name)
    try:
        profile.publish(candidate,frozen.read_bytes());candidate.chmod(0o700)
        verify_candidate(candidate,version,notes)
        state=dict(schema_version=2,engine='rust',version=version,source_digest=digest,candidate_digest=candidate_digest,effective_inputs=actual,inputs=inputs,binary_sha256=profile.digest(candidate),artifact=str(candidate),binary=built['metadata'],evidence=str(evidence),prepared_at=datetime.datetime.now(datetime.timezone.utc).isoformat())
        state_path.parent.mkdir(parents=True,exist_ok=True);profile.publish(state_path,json.dumps(state,indent=2)+'\n')
        print('Livraison Rust locale préparée : '+version,flush=True)
        return candidate,state
    except BaseException:
        candidate.unlink(missing_ok=True)
        raise


def _read_prepared(source,expected_sha):
    """Read and validate retained qualification without staging or live changes.

    Returns state, builder, image bytes, notes, metadata and effective inventory.
    Missing files propagate FileNotFoundError; incoherence/refusal propagates its
    original error. Caller owns lifecycle/build locks. No probe, provider or write.
    """
    import tarfile
    source=source.resolve()
    if not re.fullmatch('[0-9a-f]{64}',expected_sha):raise ValueError('prepared-build-sha256-required')
    state=json.loads((source/'deploy/local-build.json').read_bytes())
    if state.get('schema_version')!=2 or state.get('engine')!='rust' or state.get('binary_sha256')!=expected_sha:
        raise ValueError('prepared-build-selection-mismatch')
    evidence=Path(state['evidence']).resolve()
    if not evidence.is_relative_to(source/'.cache/local-delivery'):raise ValueError('prepared-build-evidence-outside-source')
    built=json.loads((evidence/'build/result.json').read_bytes())
    profile_path=Path(built['profile']).resolve()
    if not profile_path.is_relative_to(evidence):raise ValueError('prepared-build-profile-outside-evidence')
    checked=json.loads(profile_path.read_bytes())
    if built.get('status')!='VERIFIED_NATIVE_CANDIDATE' or checked.get('status')!='VERIFIED_DEFAULT_NATIVE_PROFILE':raise ValueError('prepared-build-unqualified')
    if built.get('sha256')!=expected_sha or checked.get('sha256')!=expected_sha or built.get('metadata')!=state.get('binary') or state.get('version')!=built['metadata'].get('version'):raise ValueError('prepared-build-receipt-mismatch')
    if not all(checked.get(key) is True for key in ['all_core_features_empty','all_core_test_false','no_fixture_feature','actual_dependencies_stable']):raise ValueError('prepared-build-profile-refused')
    if checked.get('native_build_metadata',{}).get('present') is not True or not built.get('metadata_sha256') or checked['native_build_metadata'].get('sha256')!=built['metadata_sha256']:raise ValueError('prepared-build-metadata-record-mismatch')
    builder=native_builder(source);profile=builder.profile
    frozen=Path(built['binary']).resolve()
    if not frozen.is_relative_to(source/'.cache/rust-migration-binaries') or profile.digest(frozen)!=expected_sha:raise ValueError('prepared-build-artifact-mismatch')
    after=json.loads((profile_path.parent/'after.json').read_bytes())
    if state.get('effective_inputs')!=after:raise ValueError('prepared-build-input-record-mismatch')
    archive_path=profile_path.parent/'effective-sources.tar.gz'
    proof=checked.get('effective_sources_archived',{})
    if proof.get('sha256')!=profile.digest(archive_path) or proof.get('all_members_verified') is not True:raise ValueError('prepared-build-archive-mismatch')
    with tarfile.open(archive_path,'r:gz') as archive:
        members=archive.getmembers()
        if len(members)!=len(after) or set(m.name for m in members)!=set(after):raise ValueError('prepared-build-archive-inventory')
        for member in members:
            if not member.isfile() or hashlib.sha256(archive.extractfile(member).read()).hexdigest()!=after[member.name]:raise ValueError('prepared-build-archive-bytes')
        notes=json.loads(archive.extractfile('version_history.json').read())['local_changes']
    raw=frozen.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=expected_sha:raise ValueError('prepared-build-artifact-changed')
    return state,builder,raw,notes,built['metadata'],after


def prepare_recorded(source,binary,expected_sha):
    """Stage the exact last qualified source build without recompiling hot sources.

    Caller holds the existing build/lifecycle locks and explicitly supplies its
    reviewed binary SHA256. Validate the retained normal-profile record, archived
    inputs, metadata and real protected version/notes before any live stop. Current
    source changes remain pending; they neither invalidate this frozen proof nor
    become installed. This is local artifact integrity, not signed provenance,
    authorization, global acceptance or a rollback operation. No provider calls.
    Missing/tampered evidence refuses, preserving the current binary and server.
    The returned adjacent temporary is replaced through the normal lifecycle path.
    """
    state,builder,raw,notes,metadata,after=_read_prepared(source,expected_sha)
    source=source.resolve();profile=builder.profile
    binary.parent.mkdir(parents=True,exist_ok=True)
    fd,name=tempfile.mkstemp(prefix=binary.name+'.candidate-',dir=binary.parent);os.close(fd);candidate=Path(name)
    try:
        profile.publish(candidate,raw);candidate.chmod(0o700)
        verify_candidate(candidate,state['version'],notes,metadata)
        changed=sum(not (source/name).is_file() or profile.digest(source/name)!=sha for name,sha in after.items())
        print('Candidat Rust qualifié sélectionné : '+state['version']+' · SHA256 '+expected_sha,flush=True)
        print('Entrées compilées modifiées depuis ce candidat, non installées : '+str(changed),flush=True)
        return candidate,state
    except BaseException as primary:
        try:candidate.unlink(missing_ok=True)
        except BaseException as cleanup:raise BaseExceptionGroup('Prepared staging and candidate cleanup failed',[primary,cleanup])
        raise


def require_prepared_not_older(installed,selected,installed_sha,selected_sha):
    """Refuse a prepared downgrade before stopping a real owned server.

    Equal exact binary hashes are the same image. Otherwise both local dev labels
    must be canonical comparable numeric versions; older/unknown labels refuse.
    These labels order local deliveries, never prove source or actor identity.
    """
    if installed_sha==selected_sha:return
    pattern=r'v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)-dev\.(0|[1-9][0-9]*)'
    a=re.fullmatch(pattern,installed);b=re.fullmatch(pattern,selected)
    if not a or not b:raise ValueError('prepared-build-version-comparison-unavailable')
    if tuple(map(int,b.groups()))<tuple(map(int,a.groups())):raise ValueError('prepared-build-downgrade-refused')
