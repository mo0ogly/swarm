#!/usr/bin/env python3
"""Replay an already committed EPIPE fixture command with a repaired normal CLI.

Only an owned output-recipe birth/command plus an existing exact canonical pause
may be consumed. No new command, SQL seed, live service, model or global watchdog.
Preserving a fixture marker does not authenticate a human or guarantee backup.
"""
import argparse,json,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_review_acceptance as review
from rust_native_review_decision import rows


def main():
    """Require an original event before replay and compare actual state/finance.

    Input is a retained output-recipe proof, never an arbitrary --root mutation.
    Absent original Root/event is an error, not reconstructed evidence. Store
    exact argv/channels and retain both known state and no-op assertions. The
    verifier depends on common canonical replay guarantees; it creates no rights.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','original-proof','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);root=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and f.digest(binary.read_bytes())==profile['sha256']
        old=a.original_proof.resolve();births=list((old/'source').glob('owned-root-*.json'));assert len(births)==1;birth=json.loads(births[0].read_bytes());assert birth['kind']=='owned_inert_fixture_birth' and birth['label']=='mission-output'
        root=Path(birth['root']);argv=json.loads((old/'command.json').read_bytes())['argv'];assert argv[1:3]==['mission','pause'] and argv[4:6]==['--root',str(root)] and argv[6]=='--input' and argv[-1]=='--json';work=argv[3]
        request_path=old/'broken-output.input.json';request=json.loads(request_path.read_bytes());assert Path(argv[7])==request_path and request['schema_version']==1
        recorded=rows(root,'SELECT work_id,kind,revision,request FROM events WHERE id=?',(request['event_id'],));assert len(recorded)==1 and recorded[0][:2]==(work,'mission.pause') and json.loads(recorded[0][3])==request
        def call(label,command):
            """Actual public CLI, with original success/refusal channels retained."""
            actual=[str(binary),*command,'--root',str(root),'--json'];r=subprocess.run(actual,stdout=subprocess.PIPE,stderr=subprocess.PIPE,stdin=subprocess.DEVNULL);f.publish(out/(label+'.command.json'),dict(argv=actual,global_watchdog=None));f.publish(out/(label+'.stdout'),r.stdout);f.publish(out/(label+'.stderr'),r.stderr);assert r.returncode==0,(label,r.stderr.decode());return json.loads(r.stdout)
        def state():
            """Read canonical/control rows and financial state without SQL writes."""
            return (rows(root,'SELECT * FROM events ORDER BY id',()),rows(root,'SELECT * FROM cockpit_controls ORDER BY work_id',()),rows(root,'SELECT * FROM cockpit_events ORDER BY seq',()),review.public.finance(root,work))
        before=call('before',['work','show',work])['work'];original=state();assert before['revision']>=recorded[0][2] and recorded[0][2]==request['expected_revision']+1
        replay=call('replay',['mission','pause',work,'--input',str(request_path)]);assert replay==before and state()==original and call('after',['work','show',work])['work']==before
        f.publish(out/'result.json',dict(status='PASS',root=str(root),work=work,binary_sha256=profile['sha256'],original_proof=str(old),existing_canonical_request_verified=True,original_recorded_revision=recorded[0][2],actual_work_events_controls_finance_unchanged=True,no_new_call_or_refund=True,global_watchdog=None,installed=False))
        print('PASS retained EPIPE fixture exact canonical replay preserves Work, events, controls and finance',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(root) if root else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
