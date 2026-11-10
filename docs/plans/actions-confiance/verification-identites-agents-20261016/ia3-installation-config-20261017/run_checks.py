from pathlib import Path
import datetime,hashlib,json,subprocess,sys
B=Path(__file__).resolve().parent; ROOT=Path.cwd()
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def inputs():
    paths=[ROOT/'go.mod',ROOT/'go.sum']
    paths += [p for folder in ['identitycontract','enrollmentconfig'] for p in (ROOT/'internal'/folder).glob('*.go')]
    return {str(p.relative_to(ROOT)):sha(p) for p in sorted(paths)}

def run(name,command):
    if (B/(name+'-run.json')).exists() or (B/(name+'.jsonl')).exists():raise SystemExit('Preserve existing attempt '+name)
    before=inputs();(B/(name+'-inputs.json')).write_text(json.dumps(before,indent=2)+'\n')
    start=datetime.datetime.now(datetime.timezone.utc).isoformat()
    with (B/(name+'.jsonl')).open('wb') as out,(B/(name+'.stderr')).open('wb') as err:r=subprocess.run(command,stdout=out,stderr=err)
    after=inputs();changed=sorted(p for p in set(before)|set(after) if before.get(p)!=after.get(p));events=[]
    for line in (B/(name+'.jsonl')).read_text().splitlines():
        try:events.append(json.loads(line))
        except ValueError:pass
    names={a:sorted({e['Test'] for e in events if e.get('Action')==a and e.get('Test')}) for a in ['pass','fail','skip']}
    packages={pkg:sum(e.get('Action')=='pass' and bool(e.get('Test')) for e in events if e.get('Package')==pkg) for pkg in sorted({e.get('Package') for e in events if e.get('Package')})}
    result={'packages':packages,'command':command,'exit':r.returncode,'start':start,'end':datetime.datetime.now(datetime.timezone.utc).isoformat(),'counts':{k:len(v) for k,v in names.items()},'names':names,'changed_inputs':changed,'input_count':len(before)}
    (B/(name+'-run.json')).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'step':name,'exit':r.returncode,'counts':result['counts'],'changed_inputs':changed}),flush=True)
    assert not changed,result
    if r.returncode:raise SystemExit(r.returncode)
    return result
if __name__=='__main__':
    label=sys.argv[1] if len(sys.argv)>1 else '01'
    normal=run('normal-'+label,['go','test','-json','./internal/enrollmentconfig','-count=1','-timeout=0'])
    race=run('race-'+label,['go','test','-race','-json','./internal/enrollmentconfig','-count=1','-timeout=0'])
    assert normal['names']['pass'] and normal['names']==race['names'] and not normal['names']['fail'] and not normal['names']['skip']
    assert all(normal['packages'].get('swarm.local/companion/internal/'+p,0)>0 for p in ['enrollmentconfig'])
    assert normal['packages']==race['packages']
    run('vet-'+label,['go','vet','./internal/enrollmentconfig'])
    run('build-'+label,['go','build','./internal/enrollmentconfig'])
