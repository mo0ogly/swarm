#!/usr/bin/env python3
"""Own a normal-binary preparation browser fixture, never a live mission Root.

Public settings choose caps/default method; Chrome exercises the real protected
page. Retain every channel and the original failure. No provider/global watchdog.
"""
import argparse
import json
import subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan
from rust_agent_model_route import publish
import rust_native_http_cli as native


def main():
    """Build the fixture publicly, run Chrome and drain only its owned server."""
    parser=argparse.ArgumentParser();parser.add_argument('--binary',required=True);parser.add_argument('--output',required=True);parser.add_argument('--scenario',choices=['full','refresh'],default='full');args=parser.parse_args()
    out=Path(args.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(args.binary).resolve(),out)
    d.ok(['init']);d.settings(preparation_write_enabled='1',preparation_default_method='debug',preparation_title_max_bytes='300',preparation_document_max_bytes='50000',preparation_history_limit='1')
    spec=plan();spec['questions']=[dict(question='Can we proceed?',answer='')]
    before=d.ok(['work','list']);server=native.Server(d.binary,d.root,out,'preparation-ui');server.capture_async(out/'observer.raw');primary=None
    try:
        server.request('/api/v1/native-capabilities',authorized=False,status=403,observe=False)
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),root=str(d.root),plan=spec,scenario=args.scenario)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:
            result=subprocess.run(['node','tests/rust_preparation_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert result.returncode==0,(out/'browser.stderr').read_text()
        assert d.ok(['work','list'])==before,'preparation must not materialize work/launch agents'
        browser=json.loads((out/'browser-result.json').read_text());assert browser['status']=='PASS' and browser['checks']
        assert {'cas-retains-local-text','refresh-no-unsaved-loss','lost-save-no-second-write','fr-etat-history-more-locks-during-read'}<=set(browser['checks'])
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
    publish(out/'result.json',dict(status='PASS',browser_checks=len(browser['checks']),provider_calls=0,work_unchanged=True))
    print((out/'browser.stdout').read_text(),end='',flush=True)
    print('PREPARATION_UI_PASS '+str(len(browser['checks']))+' browser assertions and unchanged work',flush=True)


if __name__=='__main__':main()
