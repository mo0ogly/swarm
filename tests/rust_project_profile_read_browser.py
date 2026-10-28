#!/usr/bin/env python3
"""Observe real served per-role metadata, including safe-public-reason UI recovery."""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Keep real reader paths and synthetic response injection separately labelled."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'project-profile-browser');profile=dict(version=1,name='<img src=x onerror="profileExecuted=true">',source='custom',instructions=[dict(path='AGENTS.md',roles=['preparation','planner','subplanner','worker','reviewer'])]);fixture.publish(f.root/'swarm.project.json',profile);fixture.publish(f.root/'AGENTS.md',b'PRIVATE_SOURCE_SENTINEL: stay in the assigned task.');(f.root/'.claude').mkdir();(f.root/'.claude/settings.json').write_text('PRIVATE_SETTINGS_SENTINEL');before=f.ok(['project-profile','show']);s=native.Server(f.binary,f.root,out,'project-profile-browser',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        cfg=dict(base='http://'+s.address,cookie=s.cookie,profile=profile,output=str(out))
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_project_profile_read_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,'Project-profile browser failed';print((out/'browser.stdout').read_text(),end='');assert before==f.ok(['project-profile','show']);assert not (f.root/'model-received.json').exists();print('PROJECT_PROFILE_UI_PASS read-only-state-preserved')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
if __name__=='__main__':main()
