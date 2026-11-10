#!/usr/bin/env python3
"""Real committed conversion with a valid-JSON wrong acknowledgement.

Baseline mode records the pre-fix false acknowledgement, not a passing product.
The corrected mode requires the exact same-event recovery; isolated Root only.
"""
import argparse,json,subprocess
from pathlib import Path
from rust_preparation_reads import Driver
from rust_preparation_plan import plan
from rust_agent_model_route import publish
import rust_native_http_cli as native

def main():
    """Retain actual CLI/server/browser channels; no provider or global watchdog."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--output',required=True);ap.add_argument('--expect-bad-ack',action='store_true');a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);d=Driver(Path(a.binary).resolve(),out);d.ok(['init']);d.settings(preparation_write_enabled='1');p=d.ok(['prepare','create'],dict(version=1,action='create',event_id='receipt-create',expected_revision=0,title='Receipt integrity',text='Need'))
    def command(action,event,**extra):
        """Use the actual current preparation CAS; never invent a conversion."""
        nonlocal p
        p=d.ok(['prepare',action,p['id']],dict(version=1,action=action,preparation_id=p['id'],event_id=event,expected_revision=p['revision'],**extra))
    command('save','receipt-brief',document='brief',text='Brief');command('adopt-brief','receipt-adopt',sha256=p['documents']['brief']['sha256']);command('save','receipt-plan',document='plan',text=json.dumps(plan()));command('validate-plan','receipt-validate',sha256=p['documents']['plan']['sha256']);before=p
    server=native.Server(d.binary,d.root,out,'receipt-integrity');primary=None
    try:
        cfg=dict(base='http://'+server.address,cookie=server.cookie,output=str(out),preparation=p,expect_bad_ack=a.expect_bad_ack)
        with (out/'browser.stdout').open('wb') as stdout,(out/'browser.stderr').open('wb') as stderr:r=subprocess.run(['node','tests/rust_preparation_receipt_integrity_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert r.returncode==0,(out/'browser.stderr').read_text();print((out/'browser.stdout').read_text(),end='');p=d.ok(['prepare','show',p['id']]);assert p['conversion']['plan_hash']==before['documents']['plan']['sha256'] and p['revision']==before['revision']+1
        publish(out/'result.json',dict(status='BASELINE_DEFECT_OBSERVED' if a.expect_bad_ack else 'PASS',browser=json.loads((out/'browser-result.json').read_text()),real_conversion_recorded=True,provider_calls=0));print('PREPARATION_RECEIPT_BASELINE_OBSERVED' if a.expect_bad_ack else 'PREPARATION_RECEIPT_INTEGRITY_PASS')
    except BaseException as error:primary=error
    finally:native.finish(server,primary)
if __name__=='__main__':main()
