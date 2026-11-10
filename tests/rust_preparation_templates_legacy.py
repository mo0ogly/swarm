#!/usr/bin/env python3
"""Keep historical v41 catalogue refusal explicit and recover through Admin."""
import argparse,json,subprocess
from pathlib import Path
import rust_agent_model_route as fixture
import rust_native_http_cli as native

def main():
    """Observe valid Root reads before the missing-plan-policy route condition."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--old-binary',required=True);p.add_argument('--output',required=True);p.add_argument('--expect-defect',action='store_true');a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.old_binary).resolve(),out,'templates-legacy');before=f.ok(['engine-settings','show']);assert before['schema_version']==41;f.binary=Path(a.binary).resolve();assert f.ok(['work','show',f.work])['work']['id']==f.work;assert f.ok(['engine-settings','show'])==before;print('PASS template-legacy-valid-root-control',flush=True);s=native.Server(f.binary,f.root,out,'templates-legacy',environment=f.environment);primary=None
    try:
        value=s.request('/api/v1/preparations/templates',status=500 if a.expect_defect else 501,observe=False);assert value['failure']['code']=='settings-preparation-plan-not-configured';assert f.ok(['engine-settings','show'])==before;print('PASS template-legacy-observed-policy-status',flush=True)
        if not a.expect_defect:
            cfg=dict(base='http://'+s.address,cookie=s.cookie,output=str(out))
            with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_templates_legacy_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
            assert r.returncode==0,'Legacy policy browser failed';print((out/'browser.stdout').read_text(),end='')
    except BaseException as e:primary=e
    finally:native.finish(s,primary)
    print('PREPARATION_TEMPLATES_LEGACY_'+('DEFECT' if a.expect_defect else 'PASS'))
if __name__=='__main__':main()
