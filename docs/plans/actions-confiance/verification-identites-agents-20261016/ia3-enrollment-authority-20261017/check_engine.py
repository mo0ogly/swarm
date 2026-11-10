from pathlib import Path
import datetime,hashlib,json,subprocess,tempfile
B=Path(__file__).resolve().parent;ROOT=Path.cwd().resolve()
def inputs():
    result=subprocess.run(['go','list','-deps','-test','-json','./internal/engine','./cmd/swarm'],capture_output=True,text=True)
    if result.returncode:
        (B/'engine-list-error.txt').write_text(result.stderr)
        raise SystemExit(result.returncode)
    dec=json.JSONDecoder();raw=result.stdout;packages=[]
    while raw.strip():
        raw=raw.lstrip();p,n=dec.raw_decode(raw);raw=raw[n:]
        directory=Path(p.get('Dir','/'))
        if directory.is_relative_to(ROOT):packages.append(p)
    paths={ROOT/'go.mod',ROOT/'go.sum'}
    for p in packages:
        for key in ['GoFiles','CgoFiles','TestGoFiles','XTestGoFiles','EmbedFiles','TestEmbedFiles','XTestEmbedFiles']:
            paths.update(Path(p['Dir'])/name for name in p.get(key,[]) if (Path(p['Dir'])/name).is_file())
    return {(str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}
def run(label,cmd,tests=False):
    if (B/(label+'-run.json')).exists():raise SystemExit('Preserve '+label)
    before=inputs();(B/(label+'-inputs.json')).write_text(json.dumps(before,indent=2)+'\n')
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();print(json.dumps({'step':label,'phase':'started','inputs':len(before),'command':cmd}),flush=True)
    with (B/(label+'.jsonl')).open('wb') as out,(B/(label+'.stderr')).open('wb') as err:r=subprocess.run(cmd,stdout=out,stderr=err)
    after=inputs();changed=sorted(p for p in set(before)|set(after) if before.get(p)!=after.get(p));events=[]
    for line in (B/(label+'.jsonl')).read_text().splitlines():
        try:events.append(json.loads(line))
        except ValueError:pass
    names={a:sorted({e['Test'] for e in events if e.get('Action')==a and e.get('Test')}) for a in ['pass','fail','skip']}
    record={'command':cmd,'start':start,'end':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit':r.returncode,'input_count':len(before),'changed_inputs':changed,'counts':{a:len(v) for a,v in names.items()},'names':names}
    (B/(label+'-run.json')).write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:v for k,v in record.items() if k!='names'}),flush=True)
    assert not changed,changed
    assert not r.returncode,record
    if tests:assert names['pass'] and not names['fail'] and not names['skip']
if __name__=='__main__':
    expression='^TestAgentIdentity(RequestStrictContract|PrincipalKeepsOpaqueIssuerAndSubject|Authority.*|RejectsCachedCaller|DecisionReauthenticationAndExactReplay|SnapshotIsDataNotImportedAuthority|SnapshotTamperingCannotRefreshItsOwnAuthority|MultilevelDelegationRevocation|TransitionPreservesHistoryAndRevocation|Issuer.*|IssuanceReceipt.*)$'
    run('engine-normal-03',['go','test','-json','./internal/engine','-run',expression,'-count=1','-timeout=0'],True)
    run('engine-vet-03',['go','vet','./internal/engine'])
    with tempfile.TemporaryDirectory(prefix='swarm-enrollment-authority-build-') as temporary:
        binary=Path(temporary)/'swarm'
        run('application-build-03',['go','build','-o',str(binary),'./cmd/swarm'])
        (B/'application-binary.json').write_text(json.dumps({'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'bytes':binary.stat().st_size,'deployed':False,'temporary_fixture_removed_after_hash':True},indent=2)+'\n')
