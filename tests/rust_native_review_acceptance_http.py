#!/usr/bin/env python3
"""Verify existing protected HTTP task action inherits native acceptance checks.

Only owned ephemeral Roots/servers, real inert worker/control/reviewer originals.
Cookies/CSRF are existing local access protection, not authenticated human identity.
No paid provider, real mission, global watchdog or other process interruption.
"""
import argparse,json,traceback
from pathlib import Path
import rust_native_review_acceptance as native
import rust_native_http_cli as http


def main():
    """Assert actual HTTP state, refusal without mutation, restoration and replay.

    Positive control precedes a separate corrupted-original fixture. Only this
    function's owned server is signaled; independent primary/cleanup failures
    survive. HTTP requests use the real Host cookie/CSRF and retain redacted
    channels, while provider calls/financial rows/private originals stay unchanged.
    """
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--profile',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();profile=a.profile.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False);completed=[]
    native.public.f.publish(out/'method.py',Path(__file__).read_bytes())
    try:
        for variant in ['control','private']:
            case=out/variant;root,work=native.prepare(binary,profile,case);server=None;primary=None
            try:
                before=native.public.preview.shown(binary,root,case,'http-before',work);review=before['tasks'][0]['independent_review'];rows=native.public.finance(root,work);invoked=(root/'reviewer-invoked.json').read_bytes()
                server=http.Server(binary,root,case,'acceptance-server');csrf=server.request('/api/v1/native-session')['csrf_token']
                body=dict(kind='task',work=work,task='task',event_id='http-native-acceptance',expected_revision=before['revision'],request=dict(status='accepted',next='Owned HTTP native acceptance fixture'))
                if variant=='private':
                    path=root/'.swarm'/('native-review-'+native.public.f.digest(review['id'].encode())+'.receipt');raw=path.read_bytes();changed=bytearray(raw);changed[-1]^=1
                    try:
                        native.public.f.publish(path,bytes(changed));refused=server.request('/api/v1/action',body,csrf,status=400)
                        assert refused['failure']['code']=='review-evidence-invalid' and 'recorded_work' not in refused,refused
                        assert native.public.preview.shown(binary,root,case,'http-refused',work)==before and native.public.finance(root,work)==rows and (root/'reviewer-invoked.json').read_bytes()==invoked
                    finally:native.public.f.publish(path,raw)
                accepted=server.request('/api/v1/action',body,csrf);assert accepted['tasks'][0]['status']=='accepted' and accepted['revision']==before['revision']+1,accepted
                replayed=server.request('/api/v1/action',body,csrf);assert replayed==accepted
                after=native.public.preview.shown(binary,root,case,'http-after',work);assert after==accepted and native.public.finance(root,work)==rows and (root/'reviewer-invoked.json').read_bytes()==invoked
                summary=server.stop();assert server.process.returncode==0
                native.public.f.publish(case/'http-result.json',dict(status='PASS',variant=variant,one_original_reviewer=True,recovery_execs=0,replay_without_mutation=True,reservations_preserved=True,owned_server_stopped=True,authenticated_human=False))
            except BaseException as error:primary=error
            finally:
                if server is not None:http.finish(server,primary)
                elif primary is not None:raise primary
            completed.append(variant);native.public.f.publish(out/'progress.json',dict(completed=completed));print('PASS native acceptance HTTP '+variant,flush=True)
        native.public.f.publish(out/'result.json',dict(status='PASS',cases=completed,real_http=True,accepted_fixtures_only=True,authenticated_human=False,reviewer_execs=2,recovery_execs=0,owned_servers_stopped=True,global_watchdog=None));print('PASS native review acceptance HTTP 2 cases',flush=True)
    except BaseException:native.public.f.publish(out/'failure.json',dict(status='FAIL',completed=completed,cause=traceback.format_exc()));raise


if __name__=='__main__':main()
