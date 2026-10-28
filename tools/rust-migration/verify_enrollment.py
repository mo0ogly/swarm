"""Qualify R2.2 on disposable SQLite files; no provider or mission operations."""
import datetime
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
from build_environment import capture as capture_build_environment
ROOT=Path(__file__).resolve().parents[2]
OUTPUT=Path(sys.argv[1]).resolve()
if OUTPUT.exists() and any(OUTPUT.iterdir()):raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True,exist_ok=True)
# Honor TMPDIR so builds and fixtures can use the volume with observed free space.
WORK=Path(tempfile.mkdtemp(prefix='swarm-rust-enrollment-'))
ENV=dict(os.environ,CARGO_TARGET_DIR=str(WORK/'target'))
commands=[]
REFERENCE=Path(sys.argv[2]).resolve() if len(sys.argv)>2 else None
REUSE={'go-tests','go-race','go-vet','0001-go-signatures','go-invalid-recovery','0004-go-reply-schema','0002-go-go-to-rust-revoke-sign','0003-go-rust-to-go-revoke-sign'}
reused=[]

def run(label,args,data=None,expected=0):
    if REFERENCE and label in REUSE:
        old_input=REFERENCE/(label+'.stdin')
        assert (old_input.read_bytes() if old_input.exists() else None)==data, 'reference input differs'
        old_commands=json.loads((REFERENCE/'commands.json').read_text())
        entry=next(c for c in old_commands if c['label']==label)
        assert entry['exit']==expected and entry['expected_exit']==expected
        for ending in ['stdout','stderr']:
            shutil.copyfile(REFERENCE/(label+'.'+ending),OUTPUT/(label+'.'+ending))
        if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
        reused.append(label)
        commands.append(dict(label=label,executed=False,reference=str(REFERENCE),reference_command=entry,stdout_sha256=hashlib.sha256((OUTPUT/(label+'.stdout')).read_bytes()).hexdigest()))
        (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'reused from verified unchanged Go inputs',flush=True)
        return (OUTPUT/(label+'.stdout')).read_bytes()
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic()
    if data is not None:(OUTPUT/(label+'.stdin')).write_bytes(data)
    result=subprocess.run(args,cwd=ROOT,env=ENV,input=data,capture_output=True)
    (OUTPUT/(label+'.stdout')).write_bytes(result.stdout);(OUTPUT/(label+'.stderr')).write_bytes(result.stderr)
    commands.append(dict(label=label,command=args,start_utc=start,end_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),seconds=time.monotonic()-clock,exit=result.returncode,expected_exit=expected))
    (OUTPUT/'commands.json').write_text(json.dumps(commands,indent=2)+'\n');print(label,'exit',result.returncode,flush=True)
    if result.returncode!=expected:raise RuntimeError(label+': unexpected exit; originals preserved')
    return result.stdout

def stream(raw):
    remaining=raw.decode().lstrip();d=json.JSONDecoder()
    while remaining:
        item,n=d.raw_decode(remaining);yield item;remaining=remaining[n:].lstrip()

def graph(raw):
    fields=('GoFiles','CompiledGoFiles','TestGoFiles','XTestGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles','Imports')
    return {p['ImportPath']:{'Dir':p.get('Dir'),**{f:sorted(p.get(f,[])) for f in fields}} for p in stream(raw)}

def local_files():
    return {p for folder,endings in ((ROOT/'rust',('.rs','.toml','.lock')),(ROOT/'tools/rust-migration',('.go','.py'))) for p in folder.rglob('*') if p.is_file() and p.suffix in endings and 'target' not in p.relative_to(folder).parts}

def collect():
    goenv=json.loads(run('go-env',['go','env','-json','GOROOT','GOOS','GOARCH','GOFLAGS','GOWORK']))
    if 'overlay' in goenv['GOFLAGS'] or goenv['GOWORK']:raise RuntimeError('effective overlay/workspace collector required')
    version=run('rust-version',['rustc','-vV']).decode();host=next(s.split(': ',1)[1]for s in version.splitlines()if s.startswith('host: '))
    run('go-version',['go','version']);run('cc-version',['cc','--version'])
    build_env=capture_build_environment(ENV)
    (OUTPUT/'build-environment.json').write_text(json.dumps(build_env,indent=2)+'\n')
    inputs={ROOT/'go.mod',ROOT/'go.sum'}
    for folder,endings in ((ROOT/'rust',('.rs','.toml','.lock')),(ROOT/'tools/rust-migration',('.go','.py'))):
        inputs.update(p for p in folder.rglob('*')if p.is_file()and p.suffix in endings and 'target' not in p.relative_to(folder).parts)
    graphs={}
    for label,flags in [('normal',[]),('race',['-race'])]:
        raw=run('go-inputs-'+label,['go','list',*flags,'-compiled','-deps','-test','-json','./internal/enrollmentstate','./tools/rust-migration/enrollment-oracle'])
        graphs[label]=(flags,graph(raw))
        for package in stream(raw):
            if package['ImportPath']=='swarm.local/companion':raise RuntimeError('unexpected root/embed dependency')
            directory=Path(package.get('Dir','/nonexistent'))
            for field in ('GoFiles','CompiledGoFiles','TestGoFiles','XTestGoFiles','CgoFiles','CFiles','CXXFiles','HFiles','SFiles','SysoFiles','EmbedFiles'):
                inputs.update(directory/name for name in package.get(field,[])if(directory/name).is_file())
    metadata=json.loads(run('cargo-metadata',['cargo','metadata','--manifest-path','rust/Cargo.toml','--locked','--offline','--all-features','--format-version','1','--filter-platform',host]))
    lock=tomllib.loads((ROOT/'rust/Cargo.lock').read_text());checksums={(p['name'],p['version']):p.get('checksum')for p in lock['package']}
    registry=[];roots={};archives=OUTPUT/'registry-crates';archives.mkdir()
    for package in metadata['packages']:
        if not package['source']:continue
        directory=Path(package['manifest_path']).parent;name=package['name']+'-'+package['version'];cache=directory.parents[2]/'cache'/directory.parent.name/(name+'.crate')
        digest=hashlib.sha256(cache.read_bytes()).hexdigest();assert digest==checksums[(package['name'],package['version'])]
        shutil.copyfile(cache,archives/cache.name);roots[directory]='registry/'+name;count=0
        with tarfile.open(cache)as archive:
            for member in archive.getmembers():
                if not member.isfile():continue
                parts=PurePosixPath(member.name).parts;assert parts[0]==name and '..'not in parts
                p=directory.joinpath(*parts[1:]);assert hashlib.sha256(p.read_bytes()).digest()==hashlib.sha256(archive.extractfile(member).read()).digest(),(name,member.name)
                inputs.add(p);count+=1
        registry.append(dict(name=package['name'],version=package['version'],crate_sha256=digest,actual_files=count))
    goroot=Path(goenv['GOROOT'])
    def key(p):
        if p.is_relative_to(ROOT):return str(p.relative_to(ROOT))
        if p.is_relative_to(goroot):return 'go-toolchain/'+str(p.relative_to(goroot))
        for directory,prefix in roots.items():
            if p.is_relative_to(directory):return prefix+'/'+str(p.relative_to(directory))
        # Go dependencies are outside the local repository, never silently omitted.
        return 'external-go/'+hashlib.sha256(str(p.parent).encode()).hexdigest()+'/'+p.name
    local_before=local_files()
    def close_graphs():
        for label,(flags,expected_graph) in graphs.items():
            raw=run('go-inputs-'+label+'-after',['go','list',*flags,'-compiled','-deps','-test','-json','./internal/enrollmentstate','./tools/rust-migration/enrollment-oracle'])
            assert graph(raw)==expected_graph, 'actual Go input graph changed'
    def snapshot():
        assert local_files()==local_before, 'local input added or removed during qualification'
        out={key(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(inputs)};assert len(out)==len(inputs);return out
    before=snapshot();(OUTPUT/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n');(OUTPUT/'registry.json').write_text(json.dumps(registry,indent=2)+'\n')
    with tarfile.open(OUTPUT/'sources.tar.gz','w:gz')as archive:
        for p in sorted(inputs):
            if not any(p.is_relative_to(d)for d in roots):archive.add(p,arcname=key(p))
    return before,snapshot,registry,close_graphs

def differential(go,rust):
    fixtures=WORK/'journals';fixtures.mkdir();checks=[];serial=0
    def call(binary,label,items):
        out=run(label,[str(binary),str(fixtures)],b''.join(json.dumps(i,ensure_ascii=True,separators=(',',':')).encode()+b'\n'for i in items))
        results=[json.loads(line)for line in out.splitlines()];assert len(results)==len(items);return results
    def compare(items,name):
        nonlocal serial
        serial+=1;g=call(go,f'{serial:04d}-go-{name}',items);r=call(rust,f'{serial:04d}-rust-{name}',items)
        mismatches=[dict(command=i,go=a,rust=b)for i,a,b in zip(items,g,r)if a!=b]
        if mismatches:(OUTPUT/'mismatches.json').write_text(json.dumps(mismatches,indent=2)+'\n');raise AssertionError(name+' diverged')
        checks.extend(dict(name=name,command=i,result=a)for i,a in zip(items,g));return g
    zero={'sequence':0,'digest':''}
    signs=[dict(op='sign',base=zero,operation='enroll',id=i,body=b.hex())for i,b in [('alice',b'owned'),(' alice ',b''),('<>&é\u2028\u2029',b'\0\xff'),("x'; DROP TABLE enrollment_events;--",b'')]]
    signs.extend(dict(op='sign',base=zero,operation='enroll',id='alice',body=bytes([i]).hex())for i in range(256))
    signs.extend(dict(op='sign',base=h,operation=o,id=i,body=b.hex())for h,o,i,b in [(zero,'reactivate','alice',b''),(zero,'revoke','alice',b'x'),(zero,'enroll','  ',b''),(zero,'enroll','a\0',b''),({'sequence':-1,'digest':''},'enroll','a',b''),({'sequence':9223372036854775807,'digest':'a'*64},'enroll','a',b'')])
    signed=compare(signs,'signatures');raw=bytes.fromhex(signed[0]['raw']);head={'sequence':1,'digest':hashlib.sha256(raw).hexdigest()}
    # Exact bytes, malformed canonicality and signed-domain/context errors.
    bad=[b' '+raw,raw+b'{}',raw.replace(b'"id":',b'"ID":'),raw.replace(b'{',b'{"extra":"",',1),raw.replace(b'{',b'{"id":"alice",',1),raw.replace(b'"body":"b3duZWQ"',b'"body":null'),raw.replace(b'"sequence":1',b'"sequence":"1"'),raw.replace(b'"protocol":"swarm-enrollment-change-v1"',b'"protocol":"other"'),raw.replace(b'"authority_key_id":"',b'"authority_key_id":"a'),raw.replace(b'"project_digest":"'+b'0b'*32+b'"',b'"project_digest":"'+b'0c'*32+b'"')]
    bad.extend(raw[:i]for i in range(len(raw)))
    bad.extend(raw[:i]+bytes([raw[i]^1])+raw[i+1:]for i in range(len(raw)))
    # Each invalid input has a valid control and valid recovery, without sharing DB state across languages.
    invalid_results=[]
    for engine,label in [(go,'go'),(rust,'rust')]:
        items=[]
        for i,b in enumerate(bad):
            items.extend([dict(op='apply',file=f'{label}-control-{i}.sqlite',base=zero,raw=raw.hex()),dict(op='apply',file=f'{label}-bad-{i}.sqlite',base=zero,raw=b.hex()),dict(op='apply',file=f'{label}-bad-{i}.sqlite',base=zero,raw=raw.hex())])
        results=call(engine,label+'-invalid-recovery',items)
        assert all(results[i]['code']=='ok'and results[i+1]['code']!='ok'and results[i+2]==results[i]for i in range(0,len(results),3))
        invalid_results.append(results)
        checks.append(dict(name=label+'-invalid-recovery',cases=len(bad)))
    assert invalid_results[0]==invalid_results[1], 'invalid change error/recovery divergence'
    # A DB created by either implementation is read and extended by the other.
    for first,second,label in [(go,rust,'go-to-rust'),(rust,go,'rust-to-go')]:
        file=label+'.sqlite';a=call(first,label+'-create',[dict(op='apply',file=file,base=zero,raw=raw.hex())])[0];assert a==dict(code='ok',head=head)
        assert call(second,label+'-read',[dict(op='read',file=file,base=head,id='alice')])[0]==dict(code='ok',record={'id':'alice','body':'b3duZWQ=','enrolled_sequence':1})
        revoke=compare([dict(op='sign',base=head,operation='revoke',id='alice',body='')],label+'-revoke-sign')[0]['raw']
        revoked=call(second,label+'-revoke',[dict(op='apply',file=file,base=head,raw=revoke)])[0]['head']
        result=call(first,label+'-check',[dict(op='read',file=file,base=revoked,id='alice'),dict(op='apply',file=file,base=zero,raw=raw.hex()),dict(op='apply',file=file,base=head,raw=revoke)])
        assert result==[dict(code='revoked'),dict(code='head'),dict(code='ok',head=revoked)]
        checks.append(dict(name=label,persistence=True,head=revoked))
    good={'schema_version':1,'head':{'sequence':2,'digest':'a'*64},'record':{'id':'alice','body':'b3duZWQ=','enrolled_sequence':1}}
    encode=lambda value:json.dumps(value,ensure_ascii=False,separators=(',',':')).encode()
    goodraw=encode(good);replies=[goodraw,b' '+goodraw,goodraw+b'{}',b'null',b'{}']
    for name in ['schema_version','head','record']:
        r=dict(good);del r[name];replies.append(encode(r));r=dict(good);r[name]=None;replies.append(encode(r))
    for version in [-1,0,1,2,9223372036854775807,'1',True,1.0]:
        for body in ['b3duZWQ=','',None,[],[1,255,None],'Zh==','Zm8=',42,True]:
            r=json.loads(goodraw);r['schema_version']=version;r['record']['body']=body;replies.append(encode(r))
    for key in ['schema_version','head','record','id','body','sequence','enrolled_sequence','digest']:
        needle=('"'+key+'":').encode();replies.extend([goodraw.replace(needle,('"'+key.upper()+'":').encode()),goodraw.replace(needle,needle+b'null,'+needle)])
    replies.extend(goodraw[:i]for i in range(len(goodraw)))
    items=[]
    for r in replies:items.extend([dict(op='decode-reply',raw=goodraw.hex()),dict(op='decode-reply',raw=r.hex()),dict(op='decode-reply',raw=goodraw.hex())])
    compare(items,'reply-schema')
    file='sql-storage-class.sqlite'
    control=call(go,'sql-cell-control',[dict(op='apply',file=file,base=zero,raw=raw.hex())])[0]
    assert control['code']=='ok'
    query=dict(op='read',file=file,base=control['head'],id='alice')
    assert call(go,'sql-blob-go',[query])==call(rust,'sql-blob-rust',[query])
    assert call(go,'sql-cell-text',[dict(op='sql',file=file,sql='UPDATE enrollment_events SET body=CAST(body AS TEXT)')])[0]['code']=='ok'
    a=call(go,'sql-text-go',[query]);b=call(rust,'sql-text-rust',[query]);assert a==b and a[0]['code']=='ok'
    assert call(go,'sql-cell-forge',[dict(op='sql',file=file,sql="UPDATE enrollment_events SET body='forged'")])[0]['code']=='ok'
    a=call(go,'sql-forged-go',[query]);b=call(rust,'sql-forged-rust',[query]);assert a==b and a[0]['code']=='change'
    checks.append(dict(name='SQLite TEXT/BLOB same signed bytes; forged TEXT denied',verified=True))
    (OUTPUT/'differential-checks.json').write_text(json.dumps(checks,indent=2)+'\n')
    return dict(signature_cases=len(signs),invalid_changes=len(bad),active_reply_cases=len(replies),cross_language_databases=2)

def qualify():
    before,snapshot,registry,close_graphs=collect()
    if REFERENCE:
        reference=json.loads((REFERENCE/'result.json').read_text());assert reference['status']=='PASS' and reference['source_stable']
        old=json.loads((REFERENCE/'inputs-before.json').read_text())
        go_inputs=lambda values:{k:v for k,v in values.items() if k.endswith('.go') or k.startswith(('go-toolchain/','external-go/')) or k in ('go.mod','go.sum')}
        assert go_inputs(before)==go_inputs(old), 'Go sources or input graph changed; cannot reuse'
        assert json.loads((OUTPUT/'go-env.stdout').read_text())==json.loads((REFERENCE/'go-env.stdout').read_text())
        assert (OUTPUT/'go-version.stdout').read_bytes()==(REFERENCE/'go-version.stdout').read_bytes()

    run('go-tests',['go','test','-json','-count=1','-timeout=0','./internal/enrollmentstate'])
    run('go-race',['go','test','-race','-json','-count=1','-timeout=0','./internal/enrollmentstate','-run','^(TestActiveReadAndRevocationSerializeLocally|TestConcurrentIdenticalChangeHasOneEvent)$'])
    run('go-vet',['go','vet','./internal/enrollmentstate','./tools/rust-migration/enrollment-oracle'])
    run('rust-format',['cargo','fmt','--manifest-path','rust/Cargo.toml','--all','--','--check'])
    run('rust-tests',['cargo','test','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','--all-features','--all-targets'])
    run('rust-clippy',['cargo','clippy','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-identity-contract','-p','swarm-wire-json','-p','swarm-enrollment-proof','-p','swarm-enrollment-state','--all-features','--all-targets','--','-D','warnings'])
    go=WORK/'enrollment-oracle';rust=WORK/'target/debug/examples/enrollment_probe'
    run('go-build',['go','build','-o',str(go),'./tools/rust-migration/enrollment-oracle'])
    run('rust-build',['cargo','build','--manifest-path','rust/Cargo.toml','--locked','--offline','-p','swarm-enrollment-state','--example','enrollment_probe'])
    compared=differential(go,rust)
    close_graphs()
    after=snapshot();(OUTPUT/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after,'qualification inputs changed'
    go_pass=[e['Test']for e in map(json.loads,(OUTPUT/'go-tests.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
    race_pass=[e['Test']for e in map(json.loads,(OUTPUT/'go-race.stdout').read_text().splitlines())if e.get('Action')=='pass'and'Test'in e]
    rust_pass=[s for s in (OUTPUT/'rust-tests.stdout').read_text().splitlines()if s.startswith('test ')and s.endswith(' ... ok')]
    assert go_pass and len(race_pass)==2 and len(rust_pass)==35,'test collection differs; inspect named tests'
    binaries={};(OUTPUT/'binaries').mkdir()
    for name,p in [('go_oracle',go),('rust_probe',rust)]:shutil.copyfile(p,OUTPUT/'binaries'/name);binaries[name]=hashlib.sha256(p.read_bytes()).hexdigest()
    result=dict(status='PASS',scope='R2.2 local enrollment journal; no Swarm cutover or production enrollment',inputs=len(before),source_stable=True,registry_packages=len(registry),reused_go_checks=reused,input_graph_closed=True,go_named_passes=go_pass,race_named_passes=race_pass,rust_named_passes=rust_pass,binary_sha256=binaries,temporary_fixture_root=str(WORK),global_watchdog='none; Go -timeout=0',deployment=False,independent_review=False,**compared)
    (OUTPUT/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:result[k]for k in ('status','inputs','signature_cases','invalid_changes','active_reply_cases')}))
try:qualify()
except Exception as error:
    (OUTPUT/'failure.json').write_text(json.dumps(dict(type=type(error).__name__,error=str(error),stage_count=len(commands)),indent=2)+'\n');raise
