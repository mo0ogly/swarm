from pathlib import Path
import datetime,hashlib,json,subprocess,sys
B=Path(__file__).resolve().parent; ROOT=Path.cwd()
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def inputs():
    paths=[ROOT/'go.mod',ROOT/'go.sum']
    paths += list((ROOT/'internal').rglob('*.go')) + list((ROOT/'cmd').rglob('*.go'))
    replacement=json.loads((B/'overlay.json').read_text())['Replace']
    return {str(p.relative_to(ROOT)):sha(replacement.get(str(p.resolve()),p)) for p in sorted(paths)}

def run(name,command):
    if (B/(name+'-run.json')).exists() or (B/(name+'.jsonl')).exists():raise SystemExit('Preserve existing attempt '+name)
    if command[0]=='go': command=command[:2]+['-overlay',str((B/'overlay.json').resolve())]+command[2:]
    before=inputs();(B/(name+'-inputs.json')).write_text(json.dumps(before,indent=2)+'\n')
    start=datetime.datetime.now(datetime.timezone.utc).isoformat()
    with (B/(name+'.jsonl')).open('wb') as out,(B/(name+'.stderr')).open('wb') as err:r=subprocess.run(command,stdout=out,stderr=err)
    after=inputs();changed=sorted(p for p in set(before)|set(after) if before.get(p)!=after.get(p));events=[]
    for line in (B/(name+'.jsonl')).read_text().splitlines():
        try:events.append(json.loads(line))
        except ValueError:pass
    names={a:sorted({e['Test'] for e in events if e.get('Action')==a and e.get('Test')}) for a in ['pass','fail','skip']}
    packages={pkg:sum(e.get('Action')=='pass' and bool(e.get('Test')) for e in events if e.get('Package')==pkg) for pkg in sorted({e.get('Package') for e in events if e.get('Package')})}
    result={'packages':packages,'command':command,'exit':r.returncode,'start':start,'end':datetime.datetime.now(datetime.timezone.utc).isoformat(),'counts':{k:len(v) for k,v in names.items()},'names':names,'changed_inputs':changed,'input_count':len(before),'overlay':json.loads((B/'overlay.json').read_text())}
    (B/(name+'-run.json')).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'step':name,'exit':r.returncode,'counts':result['counts'],'changed_inputs':changed}),flush=True)
    assert not changed,result
    if r.returncode:raise SystemExit(r.returncode)
    return result
if __name__=='__main__':
    label=sys.argv[1] if len(sys.argv)>1 else '01'
    regex='^TestAgentIdentity(ConfigClient|ConfigService|Channel)'
    packages=['./internal/engine','./internal/enrollmentconfig']
    # Different selectors: transport/service tests in engine; full coordinator regression package.
    for mode in ['normal','race']:
        flags=['-race'] if mode=='race' else []
        a=run(mode+'-engine-'+label,['go','test',*flags,'-json','./internal/engine','-run',regex,'-count=1','-timeout=0'])
        assert a['names']['pass'] and not a['names']['fail'] and not a['names']['skip']
        # Reuse 106 config names from lot 01: this repair changes engine only.
    run('vet-'+label,['go','vet',*packages])
    run('build-'+label,['go','build','-o','/tmp/swarm-identity-config-client-03b-fixture-'+label,'./cmd/swarm'])
