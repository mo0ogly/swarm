#!/usr/bin/env python3
"""Reject invalid negative gate numbers through the actual public native CLI.

Creates only an owned isolated Root and proof file. A valid passing document and
legitimate quality failure precede invalid penalty/cap/FAIL-count cases in both
method2 and saved-policy method3; zero penalties remain valid. No provider, live
mission, direct SQL, global watchdog or policy mutation. Original argv/stdout/
stderr and exact binary/helper hashes are retained; rejection must be typed and
must not alter evidence or saved settings. A failed run is never overwritten.
"""
import argparse,hashlib,json,os,subprocess,tempfile,traceback
from pathlib import Path


def run(binary,out):
    """Verify positive/negative/zero and restored-input behavior on a fresh Root.

    Propagates command/decoding/assertion failures after retaining channels. No
    acceptance or arbitrary timing claim follows from these local CLI checks.
    """
    out.mkdir(parents=True,exist_ok=False);binary=binary.resolve();before=hashlib.sha256(binary.read_bytes()).hexdigest()
    root=Path(tempfile.mkdtemp(prefix='gate-nonnegative-'));(root/'proof.txt').write_bytes(b'proof')
    def invoke(label,args,document=None):
        """Retain one exact public invocation and both channels; never retry it."""
        request=out/(label+'.input.json')
        argv=[str(binary),*args,'--root',str(root),'--json']
        if document is not None:request.write_text(json.dumps(document)+'\n');argv+=['--input',str(request)]
        (out/(label+'.command.json')).write_text(json.dumps({'argv':argv},indent=2)+'\n')
        with (out/(label+'.stdout')).open('w') as stdout,(out/(label+'.stderr')).open('w') as stderr:
            result=subprocess.run(argv,stdout=stdout,stderr=stderr)
        (out/(label+'.exit.json')).write_text(json.dumps({'exit':result.returncode})+'\n')
        return result.returncode,((out/(label+'.stdout')).read_text(),(out/(label+'.stderr')).read_text())
    assert invoke('init',['init'])[0]==0
    code,settings=invoke('settings-before',['engine-settings','show']);assert code==0
    checks=[]
    for method in ['2','3']:
        good=dict(method_version=method,scope_id='numeric',artifacts={'proof.txt':hashlib.sha256(b'proof').hexdigest()},domains={'quality':1},checks=[dict(id='score',domain='quality',mandatory=False,gate='delivery',severity='minor',penalty=100,max_penalty=100)],results=[dict(id='score',status='PASS',count=0,evidence=['proof.txt'])])
        def evaluate(label,document):
            """Exercise evaluation's real method-specific decoder and Root path."""
            return invoke('m'+method+'-'+label,['evaluate','--phase','delivery'],document)
        def value(wire):
            """Read actual ordinary vs saved-policy DTO, refusing missing fields."""
            parsed=json.loads(wire);return parsed['evaluation'] if method=='3' else parsed
        code,channels=evaluate('valid',good);assert code==0 and value(channels[0])['quality']==100
        checks.append(method+'-valid-witness')
        failure=json.loads(json.dumps(good));failure['results'][0].update(status='FAIL',count=1)
        code,channels=evaluate('legitimate-failure',failure);assert code==1 and value(channels[0])['quality']==0 and not value(channels[0])['ship_allowed']
        checks.append(method+'-legitimate-failure-blocks')
        for section,key in [('checks','penalty'),('checks','max_penalty'),('results','count')]:
            bad=json.loads(json.dumps(failure));bad[section][0][key]=-1
            code,channels=evaluate('negative-'+key,bad)
            assert code!=0 and not channels[0].strip() and 'finite-nonnegative-number-required' in channels[1],(method,key,code,channels)
            checks.append(method+'-negative-'+key+'-refuses')
        for key in ['penalty','max_penalty']:
            zero=json.loads(json.dumps(failure));zero['checks'][0][key]=0
            code,channels=evaluate('zero-'+key,zero);assert code==0 and value(channels[0])['quality']==100
            checks.append(method+'-zero-'+key+'-valid')
        code,channels=evaluate('restored',good);assert code==0 and value(channels[0])['quality']==100
        checks.append(method+'-restored-valid')
    assert invoke('settings-after',['engine-settings','show'])==(0,settings)
    assert (root/'proof.txt').read_bytes()==b'proof' and hashlib.sha256(binary.read_bytes()).hexdigest()==before
    result=dict(status='PASS',root=str(root),checks=checks,binary=str(binary),binary_sha256=before,helper_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),accepted=False,global_watchdog=None)
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n');return result


def main():
    """Return nonzero on the first failed assertion, retaining original failure."""
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',required=True,type=Path);parser.add_argument('--output',required=True,type=Path);args=parser.parse_args()
    try:result=run(args.binary,args.output);print('GATE_NONNEGATIVE_PASS '+str(len(result['checks']))+' assertions');return 0
    except BaseException:
        traceback.print_exc();return 1


if __name__=='__main__':raise SystemExit(main())
