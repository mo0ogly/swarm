#!/usr/bin/env python3
"""Verify enrolled-button availability and real native governance on owned roots.

Explicit binary, fresh evidence, no provider, retry or global watchdog. Declared
sources are supplementary; the delivery compiler profile is separate evidence.
"""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path

def main():
    """Run named JS guards and public browser recipe; preserve both channels."""
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--binary',required=True);ap.add_argument('--output-parent',required=True);args=ap.parse_args()
    root=Path(__file__).resolve().parent.parent;out=Path(tempfile.mkdtemp(prefix='trust-availability-',dir=Path(args.output_parent).resolve()));binary=Path(args.binary).resolve()
    names=['web/trust-native.js','tests/trust_enrolled_availability_test.cjs','tests/trust_enrolled_outcome_test.cjs','tests/rust_trust_governance.py','tests/rust_trust_governance_ui.cjs','tests/rust_trust_availability.py']
    snap=lambda:{n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in names}
    before=snap();commands=[['node',names[1]],['node',names[2]],['python3',names[3],'--binary',str(binary),'--browser','--output',str(out/'public')]]
    (out/'invocation.json').write_text(json.dumps({'commands':commands,'sources_before':before,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'global_watchdog':None},indent=2)+'\n')
    for i,command in enumerate(commands):
        with (out/f'{i}.stdout').open('wb') as stdout,(out/f'{i}.stderr').open('wb') as stderr:r=subprocess.run(command,cwd=root,stdout=stdout,stderr=stderr)
        assert r.returncode==0,{'step':i,'exit':r.returncode,'evidence':str(out)}
    assert snap()==before,'declared sources changed'
    public=json.loads((out/'public/result.json').read_text());browser=json.loads((out/'public/browser/result.json').read_text())
    assert 'Absent-enrollment-stays-disabled-after-preview' in browser['assertions']
    (out/'result.json').write_text(json.dumps({'status':'PASS','sources_before_after':before,'public_assertions':len(public['assertions']),'browser_assertions':len(browser['assertions']),'independent_acceptance':False},indent=2)+'\n');print('AVAILABILITY_PUBLIC_PASS',out,flush=True)
if __name__=='__main__':main()
