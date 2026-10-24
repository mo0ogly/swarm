#!/usr/bin/env python3
"""Reject a successful historical inert SDK exec that did not write a receipt.

Explicit --sdk-source-root identifies pre-journal SDK inputs, or with --raw-capture
a historical journal SDK that lacks streaming capture.
Fresh mode executes the real public source fixture and expects the specific
missing-receipt refusal. --existing-run verifies preserved outputs only, never
claims a fresh execution. Existing mode uses archived byte-verified originals;
it never requires reopening the temporary Root. Neither mode authorizes a paid
provider or business result.
"""
import argparse,json,subprocess,sys
from pathlib import Path
import rust_reviewer_claim_candidate as sdk


def retained_evidence(run):
    """Validate already copied fixture originals without reopening a live Root.

    Require the fixed process witnesses and every recorded byte hash. Refuse
    missing files, escaping/symlink paths or corruption. This is archival
    integrity only, not authentication, fresh execution or current authority.
    Historical output directories must retain their qualification separately.
    """
    manifest=json.loads((run/'process-evidence-retention.json').read_text())
    required={'owned-review-process-observation.json','owned-review-exec.stdout','owned-review-exec.stderr','reviewer-invoked.json'}
    if manifest.get('status')!='PASS' or not required.issubset(manifest.get('files',{})):raise ValueError('retained-process-witness-missing')
    for name,digest in manifest['files'].items():
        path=run/name
        if Path(name).name!=name or path.is_symlink() or not path.resolve().is_relative_to(run.resolve()):raise ValueError('retained-process-path-invalid')
        if sdk.f.digest(path.read_bytes())!=digest:raise ValueError('retained-process-bytes-changed')
    return manifest


def main():
    """Retain a discriminating expected refusal, not an arbitrary failed setup.

    Require qualified SDK sources, a passed named SDK parent, real invocation and
    wait, then the explicit required-receipt refusal with no journal, or the
    raw-capture refusal with an actual older journal and unknown raw capture.
    Evidence
    output is exclusive; readback failures remain failures. Existing mode checks
    archived originals without reopening or modifying their temporary Root.
    No global watchdog.
    """
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--raw-capture',action='store_true')
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--existing-run',type=Path)
    parser.add_argument('--binary',type=Path)
    parser.add_argument('--sdk-source-root',type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    if args.existing_run:
        run=args.existing_run.resolve();invocation=None
    else:
        if not args.binary or not args.sdk_source_root:parser.error('fresh mode requires --binary and pre-journal --sdk-source-root')
        run=out/'owned-sdk';invocation=[sys.executable,str(Path(__file__).with_name('rust_reviewer_claim_candidate.py')),'--binary',str(args.binary.resolve()),'--output',str(run),'--execute-reviewer','--final-reserved-call','--controller-case','complete','--require-private-receipt','--sdk-source-root',str(args.sdk_source_root.resolve())]
        if args.raw_capture:invocation.append('--require-raw-capture')
        result=subprocess.run(invocation,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        sdk.f.publish(out/'stdout.log',result.stdout.encode());sdk.f.publish(out/'stderr.log',result.stderr.encode())
        if result.returncode==0:raise ValueError('missing-private-receipt-falsely-passed')
    failure=json.loads((run/'failure.json').read_text())
    expected='required streaming raw capture absent' if args.raw_capture else 'required private receipt absent'
    if expected not in failure['cause']:raise ValueError('wrong-negative-prerequisite-failure')
    profile=json.loads((run/'sdk.profile.json').read_text())
    if not profile['source_stable'] or '1 passed; 0 failed' not in (run/'sdk.stdout').read_text():raise ValueError('negative-sdk-witness-not-qualified')
    if not args.existing_run:
        marker=json.loads((run/'fixture.json').read_text());root=Path(marker['root'])
        sdk.retain_process_observations(root,run)
    retained_evidence(run)
    process=json.loads((run/'owned-review-process-observation.json').read_text())
    if not process['reaped'] or not process['grant_attempted']:raise ValueError('negative-journal-witness-invalid')
    receipt=run/'owned-review-private-receipt.json'
    if args.raw_capture:
        if not receipt.exists() or json.loads(receipt.read_text()).get('raw_capture_streamed'):raise ValueError('negative-capture-witness-invalid')
    elif receipt.exists():raise ValueError('negative-journal-witness-invalid')
    sdk.f.publish(out/'result.json',dict(status='PASS_EXPECTED_REFUSAL',fresh_execution=not bool(args.existing_run),invocation=invocation,source_run=str(run),source_failure_sha256=sdk.f.digest((run/'failure.json').read_bytes()),missing_receipt_refused=not args.raw_capture,missing_streaming_capture_refused=args.raw_capture,real_successful_inert_exec=True,global_watchdog=None))
    print('PASS explicit missing '+('streaming capture' if args.raw_capture else 'receipt')+' refusal with successful SDK witness',flush=True)


if __name__=='__main__':main()
