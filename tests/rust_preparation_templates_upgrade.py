#!/usr/bin/env python3
"""Check historical v44 model reads and explicit v45 Admin cap adoption."""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Bind actual Admin publication/history to the template-check consumer."""
    p=argparse.ArgumentParser();p.add_argument('--old-binary',required=True);p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.old_binary).resolve(),out,'templates-upgrade');old=f.ok(['engine-settings','show']);assert old['schema_version']==44;f.binary=Path(a.binary).resolve();templates=f.ok(['prepare','templates']);t=next(t for t in templates if t['id']=='application');request=dict(template_id=t['id'],answers={q['id']:'Fixture '+q['id'] for q in t['questions']});code,value=f.call(['prepare','template-check'],request);assert code!=0 and value['failure']['code']=='settings-preparation-templates-not-configured' and old==f.ok(['engine-settings','show']);print('PASS template-v44-not-silently-adopted',flush=True)
    s=native.Server(f.binary,f.root,out,'template-upgrade',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        csrf=s.request('/api/v1/native-session',observe=False)['csrf_token'];s.request('/api/v1/preparations/template-check',request,csrf=csrf,status=501,observe=False)
        cfg=dict(base='http://'+s.address,cookie=s.cookie,output=str(out),expected_schema=f.ok(['engine-settings','describe'])['document_schema_version'])
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_templates_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,'Template Admin browser failed';print((out/'browser.stdout').read_text(),end='');new=f.ok(['engine-settings','show']);assert new['schema_version']==cfg['expected_schema'] and new['values']['preparation_template_answer_max_bytes']=='1' and new['history'][:len(old['history'])]==old['history'] and len(new['history'])==len(old['history'])+1;print('PASS template-admin-history-preserved');code,value=f.call(['prepare','template-check'],request);assert code!=0 and value['failure']['code']=='preparation-request-invalid';f.settings_change(preparation_template_answer_max_bytes=None);assert f.ok(['prepare','template-check'],request)['need_complete'];print('PASS template-admin-real-consumer-and-recovery');assert not (f.root/'model-received.json').exists();print('PREPARATION_TEMPLATES_UPGRADE_PASS')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
if __name__=='__main__':main()
