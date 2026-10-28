#!/usr/bin/env python3
"""Actual historical Rust v41 Root → explicit Admin v43 adoption → new writer.
Requires retained old/new binaries; no live Root, Go process, provider, timeout
or reset of configuration history. Original CLI/browser channels are retained.
"""
import argparse,json,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Witness old configuration, refusal without mutation, explicit browser
    adoption, retained history and consumption by the new command binary.
    """
    p=argparse.ArgumentParser();p.add_argument('--old-binary',required=True);p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args()
    out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);publish(out/'driver.py',Path(__file__).read_bytes());d=Driver(Path(a.old_binary).resolve(),out);d.ok(['init']);checks=[]
    def check(name,value):
        """Require distinct named evidence, stopping at the first failed boundary."""
        assert value,name;assert name not in checks;checks.append(name);print('PASS '+name,flush=True)
    d.settings(preparation_list_limit='2');old=d.ok(['engine-settings','show']);check('historical-v41',old['schema_version']==41 and 'preparation_write_enabled' not in old['values'])
    work=d.ok(['work','create'],dict(schema_version=1,event_id='upgrade-work',expected_revision=0,title='Owned historical upgrade',objective='Verify explicit adoption',scope='No execution',criteria=['History preserved']))['work']['id']
    d.binary=Path(a.binary).resolve();intent=dict(version=1,action='create',event_id='upgrade-draft',expected_revision=0);before=d.database();code,stdout,stderr=d.call(['prepare','create'],intent)
    check('missing-group-refused-without-default',code==2 and not stdout and json.loads(stderr)['failure']['code']=='settings-preparation-write-not-configured' and d.database()==before)
    check('old-reads-work',d.ok(['prepare','list'])==dict(preparations=[],limit=2))
    server=native.Server(d.binary,d.root,out,'upgrade');server.capture_async(out/'observer.raw');primary=None
    try:
        cfg=dict(address=server.address,key=server.key,root=str(d.root),work=work,output=str(out))
        with (out/'browser.stdout').open('xb') as o,(out/'browser.stderr').open('xb') as e:r=subprocess.run(['node','tests/rust_preparation_admin_ui.cjs'],input=json.dumps(cfg).encode(),stdout=o,stderr=e)
        check('explicit-Admin-adoption',r.returncode==0)
        new=d.ok(['engine-settings','show']);check('v43-history-prefix-preserved',new['schema_version']==43 and new['history'][:len(old['history'])]==old['history'] and len(new['history'])==len(old['history'])+1)
        result=d.ok(['prepare','create'],intent);check('same-intent-now-writable',result['method']=='debug' and result['revision']==1 and not result['plan_ready'])
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',checks=checks,fixture_root=str(d.root),installed=False,global_watchdog=None));print('PREPARATION_UPGRADE_PASS '+str(len(checks))+' assertions',flush=True)

if __name__=='__main__':main()
