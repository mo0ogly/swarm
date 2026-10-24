#!/usr/bin/env python3
"""Replay real owned reviewer streaming, private capture and saved policy variants.

Requires an explicitly qualified normal CLI oracle and optionally retained SDK
sources. Eleven sequential fixtures, no paid provider/Go/installation or global
watchdog. Capturing raw diagnostics never changes the independent reply cap.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def main():
    """Preserve every subprocess channel and assert exact complete case coverage.

    Six lifecycle cases and five explicit saved-policy variants exercise real
    pipe reads and receipt validation. Complete cases require a datagram ack
    before the inert provider can exit, proving live decoding without sleeps.
    Failures retain partial coverage and originals; no failed setup counts PASS.
    Output must not exist. All subprocesses run sequentially on the shared cache.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    cases=[(name,name,None) for name in ['complete','duration','source','output-error','domain-invalid','changes-requested']]
    cases += [('zero-bytes','complete',{'agent_capture_max_bytes':'0','agent_capture_max_lines':None}),('zero-lines','complete',{'agent_capture_max_bytes':None,'agent_capture_max_lines':'0'}),('unlimited','complete',{'agent_capture_max_bytes':None,'agent_capture_max_lines':None}),('shared-bytes','complete',{'agent_capture_max_bytes':'5','agent_capture_max_lines':None}),('shared-lines','complete',{'agent_capture_max_bytes':None,'agent_capture_max_lines':'1'})]
    complete=[]
    sdk.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        for name,case,policy in cases:
            target=out/name
            argv=[sys.executable,str(Path(__file__).with_name('rust_reviewer_claim_candidate.py')),'--binary',str(args.binary.resolve()),'--output',str(target),'--execute-reviewer','--final-reserved-call','--controller-case',case,'--require-raw-capture']
            if case=='complete':argv.append('--stream-handshake')
            if policy is not None:argv+=['--capture-policy',json.dumps(policy)]
            if args.sdk_source_root:argv+=['--sdk-source-root',str(args.sdk_source_root.resolve())]
            sdk.f.publish(out/(name+'.command.json'),dict(argv=argv,global_watchdog=None))
            with (out/(name+'.stdout')).open('xb') as stdout,(out/(name+'.stderr')).open('xb') as stderr:
                result=subprocess.run(argv,stdout=stdout,stderr=stderr,stdin=subprocess.DEVNULL)
            if result.returncode:raise ValueError('streaming-case-failed:'+name)
            actual=json.loads((target/'result.json').read_text());receipt=json.loads((target/'owned-review-private-receipt.json').read_text());raw=receipt['raw_capture']
            if actual['status']!='PASS' or not actual['streaming_capture_required'] or actual['business_result_published']:raise ValueError('streaming-case-contract:'+name)
            if name in ('zero-bytes','zero-lines'):
                if raw['retained_bytes']!=0 or not raw['stdout']['truncated'] or not raw['stderr']['truncated'] or receipt['failed']:raise ValueError('zero-capture-invalidates-valid-review')
            if name=='unlimited' and (raw['stdout']['truncated'] or raw['stderr']['truncated']):raise ValueError('unlimited-capture-truncated')
            if name=='shared-bytes' and raw['retained_bytes']!=5:raise ValueError('shared-byte-cap-not-consumed-once')
            if name=='shared-lines':
                lines=sum(bytes(raw[channel]['retained']).count(b'\n')+int(bool(raw[channel]['retained']) and raw[channel]['retained'][-1]!=10) for channel in ['stdout','stderr'])
                if lines!=1:raise ValueError('shared-line-cap-not-consumed-once')
            complete.append(name);sdk.f.publish(out/'progress.json',dict(completed=complete,expected=len(cases),global_watchdog=None))
            print('PASS streaming capture '+name,flush=True)
        if len(complete)!=len(cases):raise ValueError('streaming-case-count-incomplete')
        sdk.f.publish(out/'result.json',dict(status='PASS',cases=complete,count=len(cases),live_decoder_handshake_cases=6,production_wiring=False,business_result_published=False,paid_provider=False,global_watchdog=None))
        print('PASS reviewer streaming capture 11 cases',flush=True)
    except BaseException:
        sdk.f.publish(out/'failure.json',dict(status='FAIL',completed=complete,expected=len(cases),cause=traceback.format_exc()))
        raise


if __name__=='__main__':main()
