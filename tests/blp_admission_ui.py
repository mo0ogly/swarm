#!/usr/bin/env python3
"""Verify the same qualified image UI on an owned owner-key fixture.

This deliberately does NOT claim real Authority-session acceptance. It runs the
unchanged browser assertions against actual HTTP assets from the production
image; no real root, policy, credentials or business data is modified.
"""
import argparse,json,os,subprocess,tarfile
from pathlib import Path
from rust_agent_model_route import Fixture,publish
from rust_native_http_cli import Server
from blp_attribution_history_cli import digest


def main():
    """Bind normal image/profile, run real FR/EN UI checks and close only owned
    fixture server/browser. Preserve primary and cleanup failures; browser cookies
    stay below the ignored evidence subtree and no global watchdog is invented.
    """
    ap=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','profile','output']:ap.add_argument('--'+name,type=Path,required=True)
    a=ap.parse_args();binary=a.binary.resolve(strict=True);out=a.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    profile=json.loads(a.profile.read_bytes());assert profile['status']=='VERIFIED_DEFAULT_NATIVE_PROFILE' and profile['sha256']==digest(binary) and profile['no_fixture_feature']
    f=Fixture(binary,out,'admission-ui');publish(f.root/'.blp-ui-fixture',b'owned image UI fixture\n')
    inputs=a.profile.parent/'after.json'
    with tarfile.open(a.profile.parent/'effective-sources.tar.gz') as archive:notes=archive.extractfile('version_history.json').read()
    publish(out/'compiled-notes.json',notes);publish(out/'profile-inputs.json',inputs.read_bytes())
    (out/'evidence').mkdir(mode=0o700);assert subprocess.run(['git','check-ignore','-q',str(out/'evidence/browser/Default/Cookies')]).returncode==0
    server=None;primary=None
    try:
        server=Server(binary,f.root,out,'image-fixture',environment=f.environment);server.capture_async(out/'http.raw')
        version=server.request('/api/v1/native-version',observe=False)
        status=dict(running=True,ready=True,pid=server.process.pid,root=str(f.root),url='http://'+server.address+'/',binary=version['binary'],binary_sha256=profile['sha256'],verification_scope='owned-fixture-same-production-image')
        publish(out/'status.json',status)
        env=dict(os.environ,SWARM_PAEA_LIVE_STATUS=str(out/'status.json'),SWARM_PAEA_LIVE_EVIDENCE=str(out/'evidence/browser-proof'),SWARM_PAEA_LIVE_SOURCE_INVENTORY=str(out/'profile-inputs.json'),SWARM_PAEA_LIVE_NOTES=str(out/'compiled-notes.json'),SWARM_PAEA_IMAGE_FIXTURE='1')
        run=subprocess.run(['node','tests/paea_local_delivery_test.cjs'],env=env,capture_output=True)
        publish(out/'browser.stdout',run.stdout);publish(out/'browser.stderr',run.stderr)
        assert run.returncode==0 and b'FIXTURE_PAEA_IMAGE_VERIFIED 56' in run.stdout,'see retained browser diagnostics'
    except BaseException as error:primary=error;raise
    finally:
        if server is not None:
            try:server.cleanup()
            except BaseException as secondary:
                if primary is not None:raise BaseExceptionGroup('UI fixture and cleanup failed',[primary,secondary])
                raise
    publish(out/'result.json',dict(status='PASS_FIXTURE_UI_ONLY',browser_assertions=56,binary_sha256=profile['sha256'],live_authority_ui_verified=False,scope='Actual protected HTTP bytes and FR/EN version UI on owned fixture using the same production image; real Authority session remains unverified'))
    print('BLP_ADMISSION_FIXTURE_UI_PASS 56; LIVE_AUTHORITY_UI_OPEN',flush=True)


if __name__=='__main__':main()
