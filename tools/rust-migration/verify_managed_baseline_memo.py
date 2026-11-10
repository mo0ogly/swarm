"""Qualify actual Go baseline memo guards; Rust baseline execution stays unported."""
from pathlib import Path
import datetime,json,os,subprocess,sys,tempfile,time
from qualification_inputs import collect
ROOT=Path(__file__).resolve().parents[2]
def main():
    """Write a fresh source-bound fixture report; raise on failed checks or changed inputs."""
    output=Path(sys.argv[1]).resolve()
    if output.exists():raise RuntimeError('Fresh evidence directory required')
    output.mkdir(parents=True);commands=[];env=dict(os.environ,CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR',str(ROOT/'rust/target/r25')))
    def save(name,value):
        """Atomically write a report JSON in the new area, preserving write errors."""
        path=output/name;fd,tmp=tempfile.mkstemp(dir=path.parent,prefix='.'+path.name+'.')
        with os.fdopen(fd,'w')as f:json.dump(value,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    def run(label,args,data=None,expected=0):
        """Run one check without watchdog; retain inputs/output and refuse an unexpected exit."""
        start=datetime.datetime.now(datetime.timezone.utc).isoformat();clock=time.monotonic();p=subprocess.run(list(map(str,args)),cwd=ROOT,env=env,input=data,capture_output=True)
        for ext,raw in [('stdout',p.stdout),('stderr',p.stderr)]:(output/(label+'.'+ext)).write_bytes(raw)
        if data is not None:(output/(label+'.stdin')).write_bytes(data)
        commands.append(dict(label=label,command=list(map(str,args)),start_utc=start,seconds=time.monotonic()-clock,exit=p.returncode,expected_exit=expected));save('commands.json',commands);print(label,'exit',p.returncode,flush=True)
        if p.returncode!=expected:raise RuntimeError(label+': original failure retained')
        return p.stdout
    try:
        before,snapshot,registry,close_graphs=collect(ROOT,output,env,run,['./internal/engine'],allow_root=True)
        selected=['TestManagedReviewBaselineMemoKeepsEveryReviewBinding','TestManagedReviewEvidenceAliasesKeepEveryDigestBinding','TestReviewMethodRestartAliasesKeepReceiptBinding','TestManagedIndependentReviewPublishesSameSHAAndSeparateProcess','TestManagedIndependentReviewBindsReportsContractAndCandidate','TestReviewMethodRestartRequiresExplicitCorrectionAndKeepsHistory','TestReviewMethodRestartRejectsTamperedOriginalsAndSemanticRefusal']
        raw=run('go-regressions',['go','test','-timeout=0','-count=1','-v','./internal/engine','-run','^('+'|'.join(selected)+')$']).decode()
        for name in selected:assert '--- PASS: '+name+' 'in raw,name+': not executed'
        for field in ['receipt','report','batch']:assert '--- PASS: TestManagedReviewBaselineMemoKeepsEveryReviewBinding/'+field+'/'in raw,field+': not executed'
        run('go-vet',['go','vet','./internal/engine'])
        close_graphs();after=snapshot();assert before==after;save('inputs-after.json',after);run('diff-check',['git','diff','--check'])
        save('result.json',dict(status='PARTIAL',go_component_status='PASS',overall_migration='PARTIAL',scope='R3ao Go review-specific integrity before context-baseline memoization',go_selected_tests=selected,baseline_faults=5,valid_control_before_each_fault=True,recovery_and_unchanged_originals=True,inputs=len(before),registry_packages=len(registry),source_stable=True,global_watchdog=None,providers='local fixture only',native_baseline_ported=False,deployment=False,independent_review=False))
        print('PASS Go memoization integrity; PARTIAL native baseline/full migration',flush=True)
    except BaseException as error:save('failure.json',dict(status='FAIL',cause=str(error),type=type(error).__name__));raise
if __name__=='__main__':main()
