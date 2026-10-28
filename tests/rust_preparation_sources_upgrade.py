#!/usr/bin/env python3
"""Verify v43 refusal, explicit Admin adoption and actual v44 source consumer.

Both binaries run only against an owned inactive fixture. No automatic policy
upgrade or live state edit; all writes use public preview/apply. No model call.
"""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Bind browser publication to reopened settings/history and source refusal."""
    ap=argparse.ArgumentParser();ap.add_argument('--old-binary',required=True);ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.old_binary).resolve(),out,'sources-upgrade');f.settings_change(preparation_write_enabled='1');old=f.ok(['engine-settings','show']);assert old['schema_version']==43 and 'preparation_source_file_max_bytes' not in old['values'];p=f.ok(['prepare','create'],dict(version=1,action='create',event_id='upgrade-source',expected_revision=0,title='Upgrade',text='Need'));(f.root/'a.md').write_text('valid text');f.binary=Path(a.binary).resolve();args=['prepare','source',p['id'],'a.md','1','1'];code,value=f.call(args);assert code!=0 and value['failure']['code']=='settings-preparation-sources-not-configured';assert old==f.ok(['engine-settings','show']);print('PASS source-v43-not-silently-adopted',flush=True)
    s=native.Server(f.binary,f.root,out,'sources-upgrade',environment=f.environment);s.capture_async(out/'observer.raw');primary=None
    try:
        s.request('/api/v1/preparations/source?id='+p['id']+'&path=a.md&start=1&end=1',status=501,observe=False)
        cfg=dict(base='http://'+s.address,cookie=s.cookie,output=str(out),expected_schema=f.ok(['engine-settings','describe'])['document_schema_version'])
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_sources_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,'Source Admin browser failed';print((out/'browser.stdout').read_text(),end='');new=f.ok(['engine-settings','show']);assert new['schema_version']==cfg['expected_schema'] and new['values']['preparation_source_file_max_bytes']=='1' and new['history'][:len(old['history'])]==old['history'] and len(new['history'])==len(old['history'])+1;print('PASS source-admin-history-preserved');code,value=f.call(args);assert code!=0 and value['failure']['code']=='preparation-source-file-over-limit';f.settings_change(preparation_source_file_max_bytes=None);assert f.ok(args)['text']=='valid text';print('PASS source-admin-real-consumer-and-recovery');assert not (f.root/'model-received.json').exists();print('PREPARATION_SOURCES_UPGRADE_PASS')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
if __name__=='__main__':main()
