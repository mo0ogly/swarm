#!/usr/bin/env python3
"""Own a real normal CLI web process and contend its Admin settings flock.

Saved mission cadence supplies observation sampling, never a global watchdog.
The only thread inspected belongs to that actual child; counters witness progress,
not agent identity or business admission. No live Root/provider/database injection.
"""
import argparse,fcntl,json,time
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native


def main():
    """Assert a valid HTTP witness, real contention and observation-only recovery."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-defect',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'discovery-lock');f.settings_change(mission_supervision_poll_interval_ms='25',storage_busy_retries='0')
    doc=f.ok(['engine-settings','show']);cadence=int(doc['values']['mission_supervision_poll_interval_ms'])/1000;before=(f.root/'.swarm/engine-settings.json').read_bytes();checks=[];server=None;primary=None
    def check(name,value):
        """Retain named semantic facts, never a zero-test/exit-only success."""
        assert value,name;checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    def thread():
        """Find the named thread of this still-owned live child only."""
        for p in Path('/proc',str(server.process.pid),'task').iterdir():
            try:
                if (p/'comm').read_text().strip()=='swarm-web-conductor'[:15]:return p
            except FileNotFoundError:continue
        return None
    def observation(p):
        """Linux scheduling facts; no inferred permission, PID signal or timeout."""
        lines=(p/'status').read_text().splitlines();switches=int(next(v.split(':',1)[1] for v in lines if v.startswith('voluntary_ctxt_switches:')))
        return dict(switches=switches,wchan=(p/'wchan').read_text().strip())
    try:
        server=native.Server(f.binary,f.root,out,'discovery-lock-http',environment=f.environment);server.capture_async(out/'observer.raw')
        v=server.request('/api/v1/native-version',observe=False);check('discovery-http-valid-control',isinstance(v['binary']['version'],str));tid=thread();assert tid is not None,'owned conductor thread missing before fault';initial=observation(tid)
        with (f.root/'.swarm/.engine-settings.lock').open('r+b') as lock:
            # Establish the writer stimulus, waiting only for existing short
            # readers to release. No attempt cap or timed PASS is introduced.
            fcntl.flock(lock,fcntl.LOCK_EX)
            if a.expect_defect:
                # The baseline conductor must actually fail. No time-based PASS.
                while server.process.poll() is None:time.sleep(cadence)
                check('discovery-baseline-server-stopped',server.process.returncode!=0)
            else:
                while True:
                    assert server.process.poll() is None,'server stopped under pure Admin contention'
                    try:current=observation(tid)
                    except FileNotFoundError:raise AssertionError('conductor thread disappeared under Admin contention')
                    if current['switches']-initial['switches']>=2 and 'futex' in current['wchan']:break
                    time.sleep(cadence)
                fixture.publish(out/'observed-thread-progress.json',dict(tid=tid.name,before=initial,after=current,sampling_seconds=cadence,global_watchdog=None))
                check('discovery-thread-progresses-under-exclusive-lock',server.process.poll() is None)
            fcntl.flock(lock,fcntl.LOCK_UN)
        if not a.expect_defect:
            check('discovery-http-after-contention',server.request('/api/v1/native-version',observe=False)==v)
            check('discovery-settings-preserved',before==(f.root/'.swarm/engine-settings.json').read_bytes() and f.ok(['engine-settings','show'])==doc)
            check('discovery-no-provider-contact',not (f.root/'model-received.json').exists())
    except BaseException as error:primary=error
    finally:
        if server is not None:native.finish(server,primary)
        elif primary is not None:raise primary
    if a.expect_defect:check('discovery-baseline-retains-lock-cause',all(code in json.loads(server.stderr)['failure']['causes'] for code in ['web-conductor-after-http','storage-operation:Lock','os-errno:11']))
    fixture.publish(out/'result.json',dict(status='GAP_OBSERVED' if a.expect_defect else 'PASS',checks=checks,binary_sha256=fixture.digest(f.binary.read_bytes()),root=str(f.root),scope='No admission/mission execution claimed; owned scheduling observation and real HTTP'))
    print(('WEB_DISCOVERY_LOCK_BASELINE ' if a.expect_defect else 'WEB_DISCOVERY_LOCK_PASS ')+str(len(checks)))
if __name__=='__main__':main()
