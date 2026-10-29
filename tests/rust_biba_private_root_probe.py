#!/usr/bin/env python3
"""Reproduce native I0-provider access to its owned fixture's private Root.

Use public CLI setup/launch and a valid model-route witness before the probe.
No real provider/account, live Root, raw secret or database mutation is used.
O_RDWR is opened and closed without writing the DB; only a fresh canary is
written under the disposable private Root. This is a before-fix discriminator,
not a claim that Biba or provider confinement is qualified. No retry/watchdog.
"""
import argparse,json,hashlib
from pathlib import Path
import rust_agent_model_route as route


def main():
    """Retain exact image, public channels, valid witness and private-access facts."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    binary=args.binary.resolve()
    valid=route.Fixture(binary,out/'valid','biba-valid')
    valid.run_and_assert(valid.prepare())
    probe=route.Fixture(binary,out/'probe','biba-probe')
    private=probe.root/'.swarm';canary=private/'biba-i3-canary'
    assert not canary.exists()
    prefix='''import os,json
facts={}
try:
 with open(PRIVATE_SETTINGS,'rb') as f:facts['private_read']=bool(f.read(1))
except OSError:facts['private_read']=False
try:
 fd=os.open(PRIVATE_DB,os.O_RDWR);os.close(fd);facts['private_database_writable']=True
except OSError:facts['private_database_writable']=False
try:
 with open(PRIVATE_CANARY,'xb') as f:f.write(b'UNTRUSTED_I0_FIXTURE')
 facts['private_write']=True
except OSError:facts['private_write']=False
with open('biba-probe-result.json','x') as f:json.dump(facts,f)
'''.replace('PRIVATE_SETTINGS',repr(str(private/'engine-settings.json'))).replace('PRIVATE_DB',repr(str(private/'state.db'))).replace('PRIVATE_CANARY',repr(str(canary)))
    program=route.PROGRAM.replace('import sys, os, json, hashlib',prefix+'\nimport sys, os, json, hashlib',1)
    route.publish(probe.root/'codex',program.encode())
    (probe.root/'codex').chmod(0o700)
    probe.run_and_assert(probe.prepare())
    facts=json.loads((probe.root/'biba-probe-result.json').read_text())
    assert facts==dict(private_read=True,private_database_writable=True,private_write=True),facts
    assert canary.read_bytes()==b'UNTRUSTED_I0_FIXTURE'
    result=dict(status='REPRODUCED_UNPROTECTED_PRIVATE_ROOT',binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),valid_control=True,observed=facts,roots=[str(valid.root),str(probe.root)],scope='Owned synthetic fixtures; no raw private content retained, no DB writes or real provider')
    route.publish(out/'result.json',result)
    print(json.dumps(result),flush=True)


if __name__=='__main__':main()
