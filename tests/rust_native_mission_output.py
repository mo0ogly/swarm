#!/usr/bin/env python3
"""Public mission control survives deterministic stdout EPIPE after commit.

Owned inactive Root, normal CLI and no model, SQL seed, global watchdog or
foreign signalling. Close the pipe reader before exec, avoiding a timing race.
"""
import argparse,json,os,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as f
import rust_native_review_acceptance as review
from rust_native_review_decision import rows


def main():
    """Valid same-command control precedes output failure, readback and replay.

    The failure must remain nonzero with safe BrokenPipe/recorded-Work metadata,
    while actual controls/canonical event persist once. Replay uses the exact
    original request, not a new event or private repair, and cannot double-revoke.
    Environment is the same isolated fixture catalogue and is never serialized.
    """
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:p.add_argument('--'+name,required=True,type=Path)
    a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);owned=None;f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        binary=a.binary.resolve();profile=json.loads(a.profile.read_bytes());assert profile['no_fixture_feature'] and f.digest(binary.read_bytes())==profile['sha256']
        owned=f.Fixture(binary,out/'source','mission-output');current=owned.ok(['work','show',owned.work])['work']
        positive=dict(schema_version=1,event_id='stdout-control',expected_revision=current['revision'],reason='Valid same command before broken output')
        current=owned.ok(['mission','pause',owned.work],positive);assert owned.ok(['mission','show',owned.work])['paused']
        current=owned.ok(['mission','unpause',owned.work],dict(schema_version=1,event_id='stdout-unpause',expected_revision=current['revision'],reason='Restore before independent output fault'))
        body=dict(schema_version=1,event_id='stdout-failure',expected_revision=current['revision'],reason='Preserve committed pause when actual stdout has no reader')
        request=out/'broken-output.input.json';f.publish(request,body);money=review.public.finance(owned.root,owned.work)
        argv=[str(binary),'mission','pause',owned.work,'--root',str(owned.root),'--input',str(request),'--json'];f.publish(out/'command.json',dict(argv=argv,global_watchdog=None,stdout_reader_closed_before_exec=True))
        reader,writer=os.pipe();os.close(reader)
        try:result=subprocess.run(argv,stdin=subprocess.DEVNULL,stdout=writer,stderr=subprocess.PIPE,env=owned.environment)
        finally:os.close(writer)
        f.publish(out/'stderr.bin',result.stderr);f.publish(out/'execution.json',dict(returncode=result.returncode,stdout_broken_before_exec=True))
        failure=json.loads(result.stderr);assert result.returncode!=0 and 'io-kind:BrokenPipe' in failure['failure']['causes'],failure
        after=owned.ok(['work','show',owned.work])['work'];assert after['revision']==current['revision']+1 and failure['recorded_work']==dict(id=owned.work,revision=after['revision'])
        assert owned.ok(['mission','show',owned.work])['paused'] and rows(owned.root,'SELECT count(*) FROM events WHERE id=?',('stdout-failure',))==[(1,)] and review.public.finance(owned.root,owned.work)==money
        events=rows(owned.root,'SELECT * FROM events ORDER BY id',());controls=rows(owned.root,'SELECT * FROM cockpit_controls ORDER BY work_id',());audit=rows(owned.root,'SELECT * FROM cockpit_events ORDER BY seq',())
        replay=owned.ok(['mission','pause',owned.work],body);assert replay==after and rows(owned.root,'SELECT * FROM events ORDER BY id',())==events and rows(owned.root,'SELECT * FROM cockpit_controls ORDER BY work_id',())==controls and rows(owned.root,'SELECT * FROM cockpit_events ORDER BY seq',())==audit and review.public.finance(owned.root,owned.work)==money
        assert not list(owned.root.glob('fixture-prompt-*.bin'))
        f.publish(out/'result.json',dict(status='PASS',root=str(owned.root),work=owned.work,binary_sha256=profile['sha256'],deterministic_epipe_after_valid_control=True,known_recorded_work_metadata=True,actual_pause_persisted=True,canonical_event_once=True,same_request_replay_without_mutation=True,no_model_process=True,global_watchdog=None,installed=False))
        print('PASS native mission stdout failure preserves committed pause and exact replay',flush=True)
    except BaseException:f.publish(out/'failure.json',dict(status='FAIL',root=str(owned.root) if owned else None,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
