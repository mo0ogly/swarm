#!/usr/bin/env python3
"""Cold initialization and historical settings continuity through supported CLI.

Only owned /tmp Roots; two exact normal historical binaries witness the v46
journal and the reproduced malformed-v47 initializer. No live Root, provider,
watchdog, retry, direct SQL edit or historical defaults adoption by a read.
"""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path
from rust_agent_model_route import publish


def main():
    """Assert current defaults, failed-init recovery and explicit v46→current adoption."""
    ap=argparse.ArgumentParser();ap.add_argument('--binary',required=True);ap.add_argument('--legacy-binary',required=True);ap.add_argument('--broken-binary',required=True);ap.add_argument('--output',required=True);a=ap.parse_args();out=Path(a.output).resolve();out.mkdir(parents=True,exist_ok=False)
    binaries={k:Path(getattr(a,k.replace('-','_'))).resolve() for k in ['binary','legacy-binary','broken-binary']};checks=[];commands=[]
    def root(label):
        """Attribute one owned Root before invoking any initializer."""
        p=Path(tempfile.mkdtemp(prefix='swarm-schema-boundary-'));publish(out/('owned-root-'+label+'.json'),dict(root=str(p),label=label,scope='owned settings fixture only'));return p
    def run(which,r,args,doc=None):
        """Retain exact CLI channels/argv; return JSON without erasing refusals."""
        n=str(len(commands)+1);argv=[str(binaries[which]),*args,'--root',str(r),'--json']
        if doc is not None:p=out/(n+'.input.json');publish(p,doc);argv+=['--input',str(p)]
        v=subprocess.run(argv,stdin=subprocess.DEVNULL,capture_output=True);publish(out/(n+'.stdout'),v.stdout);publish(out/(n+'.stderr'),v.stderr);commands.append(dict(argv=argv,exit=v.returncode));publish(out/'commands.json',commands);return v.returncode,json.loads(v.stdout if v.returncode==0 else v.stderr)
    def ok(which,r,args,doc=None):
        code,result=run(which,r,args,doc);assert code==0,(args,result);return result
    def check(name,value):
        """Require concrete state/content results, retaining every named assertion."""
        assert value,name;checks.append(name);publish(out/'checks.json',checks);print('PASS '+name,flush=True)
    fresh=root('fresh');ok('binary',fresh,['init']);current=ok('binary',fresh,['engine-settings','show']);check('settings-current-default-envelope',current['schema_version']==48 and current['history'][-1]['change']['schema_version']==48)
    check('settings-current-default-groups',all(k in current['values'] for k in ['report_preview_max_bytes','report_verification_max_bytes','cockpit_default_autonomy']))
    before=(fresh/'.swarm/engine-settings.json').read_bytes();ok('binary',fresh,['init']);check('settings-current-init-idempotent',before==(fresh/'.swarm/engine-settings.json').read_bytes())
    failed=root('partial');code,err=run('broken-binary',failed,['init']);check('settings-broken-init-witness',code!=0 and err['failure']['code']=='settings-document-envelope' and not (failed/'.swarm/engine-settings.json').exists())
    ok('binary',failed,['init']);check('settings-partial-init-public-recovery',ok('binary',failed,['engine-settings','show'])['schema_version']==48)
    old=root('legacy');ok('legacy-binary',old,['init']);old_doc=ok('legacy-binary',old,['engine-settings','show']);check('settings-legacy-positive-control',old_doc['schema_version']==46)
    original=(old/'.swarm/engine-settings.json').read_bytes();observed=ok('binary',old,['engine-settings','show']);check('settings-legacy-read-no-adoption',observed==old_doc and original==(old/'.swarm/engine-settings.json').read_bytes())
    values={**current['values'],**old_doc['values']};request=dict(schema_version=48,event_id='fixture-schema-adopt',expected_revision=old_doc['revision'],values=values,reason='Explicit fixture adoption of current settings format',preview_token='')
    future={**request,'schema_version':49};code,err=run('binary',old,['engine-settings','preview'],future);check('settings-unknown-future-schema-refuses',code!=0 and err['failure']['code']=='settings-change-schema-event-revision-reason' and original==(old/'.swarm/engine-settings.json').read_bytes())
    preview=ok('binary',old,['engine-settings','preview'],request);check('settings-current-preview-readonly',original==(old/'.swarm/engine-settings.json').read_bytes() and bool(preview['preview_token']))
    request['preview_token']=preview['preview_token'];ok('binary',old,['engine-settings','apply'],request);reopened=ok('binary',old,['engine-settings','show']);check('settings-adoption-reopens',reopened['schema_version']==48 and reopened['revision']==str(int(old_doc['revision'])+1) and reopened['values']==values)
    check('settings-adoption-preserves-actual-history',reopened['history'][:-1]==old_doc['history'] and reopened['history'][-1]['previous_digest']==old_doc['history'][-1]['digest'])
    publish(out/'result.json',dict(status='PASS',checks=checks,binaries={k:dict(path=str(v),sha256=hashlib.sha256(v.read_bytes()).hexdigest()) for k,v in binaries.items()},scope='Settings envelopes and public fixture continuity; not acceptance of report-read policy or live adoption'))
    print('SETTINGS_SCHEMA_BOUNDARY_PASS '+str(len(checks)))
if __name__=='__main__':main()
