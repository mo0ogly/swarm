#!/usr/bin/env python3
"""Check documentary policy validation through public preview with no mutation."""
import argparse,copy
from pathlib import Path
import rust_agent_model_route as fixture

def main():
    """Require valid witnesses, malformed refusal and recovery for each policy."""
    p=argparse.ArgumentParser();p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False);f=fixture.Fixture(Path(a.binary).resolve(),out,'source-policy');doc=f.ok(['engine-settings','show']);original=(f.root/'.swarm/engine-settings.json').read_bytes();checks=[]
    def request(values,event):return dict(schema_version=doc['schema_version'],event_id=event,expected_revision=doc['revision'],values=values,reason='Owned source policy validation fixture')
    for name,key,value in [('duplicate-extension','preparation_source_allowed_extensions',['.md','.MD']),('unsafe-extension','preparation_source_allowed_extensions',['md']),('escape-component','preparation_source_excluded_components',['../other']),('duplicate-component','preparation_source_excluded_components',['Build','build']),('component-edge-space','preparation_source_excluded_components',['private ']),('negative-cap','preparation_source_file_max_bytes','-1'),('partial-group','preparation_source_scan_limit',None)]:
        f.ok(['engine-settings','preview'],request(doc['values'],'valid-'+name));values=copy.deepcopy(doc['values'])
        if name=='partial-group':del values[key]
        else:values[key]=value
        code,reply=f.call(['engine-settings','preview'],request(values,'invalid-'+name));assert code!=0 and 'failure' in reply and (f.root/'.swarm/engine-settings.json').read_bytes()==original,name;f.ok(['engine-settings','preview'],request(doc['values'],'recovery-'+name));checks.append(name);fixture.publish(out/'checks.json',checks);print('PASS source-policy-'+name,flush=True)
    values=copy.deepcopy(doc['values']);values.update(preparation_source_allowed_extensions=[],preparation_source_excluded_components=[])
    f.ok(['engine-settings','preview'],request(values,'explicit-empty-lists'));assert (f.root/'.swarm/engine-settings.json').read_bytes()==original;print('PASS source-policy-explicit-empty-lists');print('PREPARATION_SOURCE_POLICY_PASS 8')
if __name__=='__main__':main()
