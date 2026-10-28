#!/usr/bin/env python3
"""Seven normal public reviewer lifecycles with retained originals and no watchdog.

Owned inert programs only; no libtest producer, fixture feature, Go, paid provider,
server, acceptance or settlement. The supplied normal profile must match bytes.
"""
import argparse,json,subprocess,sys,traceback
from pathlib import Path
import rust_agent_model_route as fixture

CASES=['passed','changes-requested','domain-invalid','malformed-output','duration','source','zero-capture']


def main():
    """Require seven distinct real runs and original/error/corruption/replay proofs.

    Each case is exclusive and retains commands/channels/frames before PASS.
    Preserve partial completion on failure; do not average failed checks or
    convert a duration policy into a whole-test watchdog. No retry is automatic.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[]
    fixture.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        for case in CASES:
            command=[sys.executable,str(Path(__file__).with_name('rust_native_reviewer.py')),'--binary',str(a.binary.resolve()),'--profile',str(a.profile.resolve()),'--case',case,'--output',str(out/case)]
            fixture.publish(out/(case+'.command.json'),dict(argv=command,global_watchdog=None))
            with (out/(case+'.stdout')).open('xb') as stdout,(out/(case+'.stderr')).open('xb') as stderr:r=subprocess.run(command,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr)
            if r.returncode:raise ValueError('normal-reviewer-case-failed:'+case)
            result=json.loads((out/case/'result.json').read_text())
            required=['one_native_reviewer_call','replay_without_call','explicit_recovery','original_retained','public_private_corruption_refused_and_restored','regular_stdin_refused','reservation_preserved']
            if result['status']!='PASS' or not all(result[k] for k in required) or result['task_accepted'] or result['model_authenticated']:raise ValueError('normal-reviewer-proof-invalid:'+case)
            completed.append(case);fixture.publish(out/'progress.json',dict(completed=completed,expected=len(CASES)));print('PASS normal reviewer '+case,flush=True)
        fixture.publish(out/'result.json',dict(status='PASS',cases=completed,normal_original_execs=len(CASES),recovery_execs=0,task_accepted=False,settlement=False,authenticated_identity=False,global_watchdog=None));print('PASS normal reviewer 7 cases',flush=True)
    except BaseException:fixture.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
