#!/usr/bin/env python3
"""Strict governance config refusals on an owned Root and production binary.

A valid persisted policy is the control before duplicate/TTL injections. No
provider, restart, global watchdog or mutation of an existing project Root.
"""
import argparse,json,os,subprocess,tempfile,hashlib
from pathlib import Path
from rust_trust_governance import configuration
from rust_agent_model_route import publish

def main():
    """Retain exact valid/refused config calls and prove unchanged state bytes."""
    a=argparse.ArgumentParser();a.add_argument('--binary',required=True);a.add_argument('--output',required=True);args=a.parse_args()
    out=Path(args.output).resolve();out.mkdir(parents=True,exist_ok=False);root=out/'root';root.mkdir();binary=Path(args.binary).resolve();checks=[];calls=[]
    def run(op,raw=None,ok=True):
        """Run a fixed public CLI operation, preserve channels and assert exit."""
        argv=[str(binary),'--root',str(root),'--json',*op];argv+=['--input','-'] if raw is not None else []
        env=os.environ.copy();env.pop('SWARM_TRUST_TOKEN',None)
        r=subprocess.run(argv,input=raw,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);i=len(calls);publish(out/f'{i}.stdout',r.stdout);publish(out/f'{i}.stderr',r.stderr);calls.append(dict(argv=argv,exit=r.returncode));assert (r.returncode==0)==ok,(op,r.returncode,r.stderr.decode());return r
    def check(name):checks.append(name);print('PASS '+name,flush=True)
    run(['init']);run(['trust','config-apply'],json.dumps(configuration(0,'valid')).encode());check('valid-enabled-policy-control')
    state=root/'.swarm/trust-actions.json';before=state.read_bytes();cfg=configuration(1,'invalid');raw=json.dumps(cfg)
    principal=json.dumps(cfg['role_separation']['principals']);duplicate=raw.replace(principal,'{"requester":"untrusted-duplicate",'+principal[1:],1)
    assert duplicate!=raw;run(['trust','config-apply'],duplicate.encode(),False);assert state.read_bytes()==before;check('duplicate-principal-refused-without-mutation')
    for name,value in [('zero',0),('null',None),('fraction',1.5),('negative',-1),('overflow',9223372037)]:
        v=configuration(1,'invalid-'+name);v['evidence_signing']['enabled']=False;v['evidence_signing']['key_ttl_seconds']=value;run(['trust','config-apply'],json.dumps(v).encode(),False);assert state.read_bytes()==before;check('disabled-signing-'+name+'-refused')
    v=configuration(1,'missing');del v['evidence_signing']['key_ttl_seconds'];run(['trust','config-apply'],json.dumps(v).encode(),False);assert state.read_bytes()==before;check('missing-signing-duration-refused')
    publish(out/'result.json',json.dumps(dict(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),checks=checks,calls=calls),indent=2).encode());print('GOVERNANCE_INVALID_PASS '+str(len(checks)),flush=True)
if __name__=='__main__':main()
