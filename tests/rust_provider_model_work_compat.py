#!/usr/bin/env python3
"""Compare native work route/source receipts across explicit normal binaries.

One owned Root and exact same policy/catalogue/settings/requests; five inherited
level variants. Compare route and model-source witness only, not whole preview
identity/time/environment output. No provider/Agent reservation or SQL mutation.
"""
import argparse,json,sys
from pathlib import Path
import rust_agent_model_route as retained


def run(before_binary,binary,output):
    """Read both public agent previews, binding binary hashes and unchanged state.

    Profile qualification remains external. The older binary initializes the
    fixture; the new reader must consume the same persisted schema. Failures and
    originals survive, with no retry or fallback. Local program never executes.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(before_binary,output,'work-compat')
    retained.publish(f.cache,{'models':[{'slug':'fixture-simple','visibility':'list'},{'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]},{'slug':'fixture-frontier','visibility':'list'}]})
    before=f.snapshot();source=f.providers.read_bytes();checks=[]
    for level in [None,'auto','simple','standard','exigeant']:
        request=dict(f.request)
        if level is not None:request['level']=level
        f.binary=before_binary;old=f.ok(['agent','preview',f.work],request)
        f.binary=binary;new=f.ok(['agent','preview',f.work],request)
        assert old['preview_only'] and new['preview_only'] and not old['process_started'] and not new['process_started']
        assert old['agent']['model_route']==new['agent']['model_route']
        def model_source(value):
            """Retain exactly one unverified source record, including original bytes."""
            checks=[item for item in value['agent']['preflight']['checks'] if item['name']=='model_route_sources']
            assert len(checks)==1 and checks[0]['status']=='declared_unverified'
            return checks[0]
        assert model_source(old)==model_source(new)
        assert f.snapshot()==before and f.providers.read_bytes()==source
        assert not list(f.root.rglob('model-received.json'))
        checks.append({'level':level,'route_equal':True,'model_source_equal':True})
    result={'status':'PASS','checks':checks,'root':str(f.root),'before_binary':str(before_binary),'before_sha256':retained.digest(before_binary.read_bytes()),'binary':str(binary),'sha256':retained.digest(binary.read_bytes()),'provider_started':False,'scope':'native work route + model-source record only, not whole-preview/Go/global equivalence'}
    retained.publish(output/'result.json',result);return result


def main():
    """Require both explicit binaries and a fresh private output directory."""
    p=argparse.ArgumentParser();p.add_argument('--before-binary',type=Path,required=True);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    print(json.dumps(run(a.before_binary.resolve(),a.binary.resolve(),a.output.resolve())))

if __name__=='__main__':main()
