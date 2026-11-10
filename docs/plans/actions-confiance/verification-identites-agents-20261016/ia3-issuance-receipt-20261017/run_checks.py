from pathlib import Path
import hashlib,importlib.util,json,sys,tempfile
B=Path(__file__).resolve().parent
OLD=B.parent/'ia3-issuer-20261017'/'frozen_checks.py'
spec=importlib.util.spec_from_file_location('receipt_checks',OLD)
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);m.B=B
m.OVERLAY=str(B/'overlay.json');m.REPLACE=json.loads(Path(m.OVERLAY).read_text())['Replace']
original_run=m.run
def unique_run(name,*args,**kwargs):
    if (B/(name+'-run.json')).exists() or (B/(name+'.jsonl')).exists():
        raise SystemExit('Preserve existing run '+name+'; diagnose and select a new run identifier.')
    return original_run(name,*args,**kwargs)
m.run=unique_run
if __name__=='__main__':
    target=sys.argv[1]
    if target=='before':
        m.run('receipt-before-02',['go','test','-overlay='+m.OVERLAY,'./internal/engine','-run','^TestAgentIdentityIssuanceReceiptSurvivesReopen$','-count=1','-v','-timeout=0','-json'])
    elif target in ['normal','normal-final']:
        label='02' if target=='normal-final' else '01'
        binary=Path(tempfile.mkdtemp(prefix='swarm-identity-receipt-'))/'engine.test'
        m.run('normal-build-'+label,['go','test','-overlay='+m.OVERLAY,'-c','-o',str(binary),'./internal/engine'])
        (B/'normal-binary.json').write_text(json.dumps({'path':str(binary),'sha256':m.digest(binary)},indent=2)+'\n')
        m.run('normal-contracts-'+label,['go','tool','test2json','-p','swarm.local/companion/internal/engine','-t',str(binary),'-test.v','-test.run=^TestAgentIdentity(Issuance|Issuer|Registry)','-test.count=1','-test.timeout=0'],binary)
    elif target=='race':
        binary=Path(tempfile.mkdtemp(prefix='swarm-identity-receipt-race-'))/'engine.test'
        m.run('race-build-01',['go','test','-overlay='+m.OVERLAY,'-race','-c','-o',str(binary),'./internal/engine'])
        (B/'race-binary.json').write_text(json.dumps({'path':str(binary),'sha256':m.digest(binary)},indent=2)+'\n')
        for name in ['Issuance','Issuer','Registry']:
            m.run('race-'+name.lower()+'-01',['go','tool','test2json','-p','swarm.local/companion/internal/engine','-t',str(binary),'-test.v','-test.run=^TestAgentIdentity'+name,'-test.count=1','-test.timeout=0'],binary)
    elif target=='registry-split':
        binary=Path(json.loads((B/'race-binary.json').read_text())['path'])
        selections={
            'a':'^TestAgentIdentityRegistry(AtomicWrite|PublicationPersistenceAndOwnedSource|RejectsUnauthorizedAndHistoricalRewrite)',
            'b':'^TestAgentIdentityRegistry(Recovery|RollbackAndTamper|ConcurrentPublishers|SourceFeeds|Migration)',
        }
        for name,pattern in selections.items():
            m.run('race-registry-split-'+name+'-01',['go','tool','test2json','-p','swarm.local/companion/internal/engine','-t',str(binary),'-test.v','-test.run='+pattern,'-test.count=1','-test.timeout=0'],binary)
    elif target=='race-final':
        binary=Path(json.loads((B/'race-binary.json').read_text())['path'])
        m.run('race-contracts-02',['go','tool','test2json','-p','swarm.local/companion/internal/engine','-t',str(binary),'-test.v','-test.run=^TestAgentIdentity(Issuance|Issuer|Registry)','-test.count=1','-test.timeout=0'],binary)
    elif target=='vet':
        m.run('vet-01',['go','vet','-overlay='+m.OVERLAY,'./internal/engine'])
    elif target=='build':
        binary=Path(tempfile.mkdtemp(prefix='swarm-identity-receipt-app-'))/'swarm'
        m.run('app-build-01',['go','build','-overlay='+m.OVERLAY,'-o',str(binary),'./cmd/swarm'])
        (B/'app-binary.json').write_text(json.dumps({'path':str(binary),'sha256':m.digest(binary),'deployed':False},indent=2)+'\n')
    else:raise SystemExit('unknown check')
