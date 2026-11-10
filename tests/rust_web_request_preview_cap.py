#!/usr/bin/env python3
"""Metadata-aware preview cap regression on owned private Root, no provider.

A valid application precedes the saved cap. No deadline/margin or live database
mutation; both refusal and explicit saved-policy recovery must be observed.
"""
from pathlib import Path
import argparse,json,traceback,copy
from rust_agent_model_route import Fixture,publish,digest

def run(binary,output):
    """Derive the old undercount from typed actual bytes, preserve current state.

    Preview actor/time are observations, not a commit or authentication proof.
    """
    output.mkdir(parents=True,exist_ok=False);publish(output/'harness.py',Path(__file__).read_bytes());f=Fixture(binary,output,'preview-cap');before=f.snapshot();r=dict(schema_version=1,event_id='control',expected_revision=0,values=dict(general_ms=0,snapshot_ms=0,connection_test_ms=0),reason='Owned valid control')
    assert f.ok(['run-limits','web-requests','apply'],r)['config']['revision']==1
    source=f.root/'.swarm/web-requests.json';original=source.read_bytes();r.update(event_id='next',expected_revision=1)
    preview=f.ok(['run-limits','web-requests','preview'],r);row=preview['config']['history'][-1];assert row['actor'] and row['at']
    empty=copy.deepcopy(preview['config']);empty['history'][-1].update(actor='',at='');cap=len(json.dumps(empty,ensure_ascii=False,separators=(',',':')).encode());assert len(original)<cap<len(json.dumps(preview['config'],ensure_ascii=False,separators=(',',':')).encode())
    f.settings_change(agent_provider_config_max_bytes=str(cap))
    for op in ['preview','apply']:
        code,error=f.call(['run-limits','web-requests',op],r);assert code!=0 and error['failure']['code']=='web-request-policy-file-outside-saved-policy',(op,error)
        assert source.read_bytes()==original
    assert f.snapshot()==before
    f.settings_change(agent_provider_config_max_bytes=None);pv=f.ok(['run-limits','web-requests','preview'],r);assert pv['config']['history'][-1]['actor'] and pv['config']['history'][-1]['at'];saved=f.ok(['run-limits','web-requests','apply'],{**r,'preview_token':pv['preview_token']});assert saved['config']['revision']==2 and len(saved['config']['history'])==2 and f.snapshot()==before
    publish(output/'result.json',dict(status='PASS',checks=['valid-control-and-observed-preview-metadata','complete-metadata-cap-refuses-preview-and-apply-without-mutation','saved-null-cap-explicit-recovery'],derived_cap=cap,binary_sha256=digest(binary.read_bytes()),provider_started=False));print('WEB_REQUEST_PREVIEW_CAP_VERIFIED 3',flush=True)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
