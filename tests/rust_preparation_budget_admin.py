#!/usr/bin/env python3
"""Read the served Admin contract for preparation warning without writes/providers."""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Retain actual browser help and compare saved settings before/after."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'budget-admin');before=f.ok(['engine-settings','show']);s=native.Server(f.binary,f.root,out,'budget-admin',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        cfg=dict(base='http://'+s.address,cookie=s.cookie,output=str(out))
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_budget_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,'Admin help browser failed';assert before==f.ok(['engine-settings','show']),'Help changed settings';print((out/'browser.stdout').read_text(),end='');print('PREPARATION_BUDGET_ADMIN_PASS settings preserved')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
    print('PREPARATION_BUDGET_ADMIN_EVIDENCE '+str(out))
if __name__=='__main__':main()
