#!/usr/bin/env python3
"""Exercise served model journeys with actual save/readback and no provider."""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Retain actual browser writes, documents and unadopted plan state."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'templates-ui');f.settings_change(preparation_write_enabled='1');templates=f.ok(['prepare','templates']);portal=next(t for t in templates if t['id']=='client-portal');s=native.Server(f.binary,f.root,out,'templates-ui',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        cfg=dict(base='http://'+s.address,cookie=s.cookie,output=str(out),questions=[q['id'] for q in portal['questions']])
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_templates_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,'Template browser failed';print((out/'browser.stdout').read_text(),end='');items=f.ok(['prepare','list'])['preparations'];assert len(items)==4
        for item in items:
            value=f.ok(['prepare','show',item['id']]);plan=json.loads(value['documents']['plan']['text']);assert value['revision']==2 and value['method']=='ks-product' and not value.get('adopted_brief') and not value.get('plan_ready') and not value.get('conversion') and len(plan['tasks'])==8 and plan['product']['mode']=='existing'
        assert not (f.root/'model-received.json').exists();print('PREPARATION_TEMPLATES_UI_PASS 4-persisted-unadopted-plans')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
if __name__=='__main__':main()
