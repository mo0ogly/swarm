#!/usr/bin/env python3
"""Verify real ordered control batches, failed checks and unattempted pause slots.

Positive two-control fixture precedes exit7 and pause faults under the same saved
policy. Actual Unix peer/pidfd observations, original framed receipts and terminal
SQL prove the observed cases. No paid provider, seed or global watchdog is used.
"""
import argparse,json,sys,traceback
from pathlib import Path
import rust_native_task_validation_failures as cases


def main():
    """Bind three owned batch flows to exact binary and imported fixture sources."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path)
    p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(exist_ok=False)
    sources={str(Path(m.__file__)):cases.fixture.digest(Path(m.__file__).read_bytes())
        for m in list(sys.modules.values()) if getattr(m,'__file__',None)
        and str(Path(m.__file__).resolve()).startswith(str(Path('tests').resolve()))}
    sources[str(Path(__file__))]=cases.fixture.digest(Path(__file__).read_bytes())
    inputs=dict(binary=str(binary),binary_sha256=cases.fixture.digest(binary.read_bytes()),sources=sources)
    cases.fixture.publish(out/'inputs.json',inputs)
    try:
        results=[cases.held(binary,out/mode,mode,batch=True) for mode in ('healthy','exit7','pause')]
        assert cases.fixture.digest(binary.read_bytes())==inputs['binary_sha256']
        assert all(cases.fixture.digest(Path(n).read_bytes())==h for n,h in sources.items())
        result=dict(status='PASS',results=results,real_provider=False,installed=False,independent_review=False,global_acceptance=False)
        cases.fixture.publish(out/'result.json',result);print(json.dumps(result))
    except BaseException:
        cases.fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise


if __name__=='__main__':main()
