#!/usr/bin/env python3
"""Verify known synchronized configuration state after no-op, never a new write.

Two CLI broken-stdout paths and one HTTP observer on an inert owned Root. Actual
source/backup bytes, Work/Agent/reservations remain unchanged; no provider runs.
"""
import argparse,json,subprocess,traceback
from pathlib import Path
import rust_agent_model_route as retained
from rust_native_http_cli import Server,clean

def run(binary,output):
    """Observe three producer effect flags through the real public CLI/HTTP.

    Positive unchanged previews precede fault injection. Actual broken stdout is
    owned, original stderr and exit retained; all cases recorded before verdict.
    No retry, global watchdog, private runtime database mutation or acceptance.
    """
    output.mkdir(parents=True,exist_ok=False);f=retained.Fixture(binary,output,'noop-state')
    retained.publish(f.cache,{'models':[{'slug':'fixture-simple','visibility':'list'},{'slug':'fixture-standard','visibility':'list','supported_reasoning_levels':[{'effort':'fixture-medium'}]},{'slug':'fixture-frontier','visibility':'list'}]})
    config=f.ok(['providers','config','show'])['configuration'];request,_=retained.configuration_proposal(f,configuration=config);f.ok(['providers','config','apply'],request)
    original=f.providers.read_bytes();before=f.snapshot();backups=sorted(p.name for p in f.providers.parent.glob('providers-before-*.json'));checks=[];s=None;primary=None
    def unchanged():
        """Require identical source, history references and inert execution state."""
        assert f.providers.read_bytes()==original and f.snapshot()==before and sorted(p.name for p in f.providers.parent.glob('providers-before-*.json'))==backups
        assert not list(f.root.rglob('model-received.json'))
    def closed(label,args,change):
        """Close only owned stdout before execution; retain known-effect metadata."""
        path=output/(label+'.input.json');retained.publish(path,change);argv=[str(binary),*args,'--root',str(f.root),'--input',str(path),'--json','--lang','en'];retained.publish(output/(label+'.argv.json'),argv)
        child=subprocess.Popen(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=f.environment);child.stdout.close();raw=child.stderr.read();child.stderr.close();code=child.wait();retained.publish(output/(label+'.stderr'),raw);assert code==2;wire=json.loads(raw);unchanged();checks.append({'case':label,'configuration_may_be_recorded':wire.get('configuration_may_be_recorded'),'code':code,'failure_code':wire['failure']['code']});retained.publish(output/'observations.json',checks)
    try:
        change,pv=retained.configuration_proposal(f,configuration=f.ok(['providers','config','show'])['configuration']);assert not pv['changed'];unchanged();closed('configuration-cli-noop-output',['providers','config','apply'],change)
        policy={'version':1,'expected_digest':retained.digest(original),'policies':{'fixture':f.provider['model_policy']},'preview':True};pv=f.ok(['providers','policies','preview'],policy);assert not pv['changed'];unchanged();change={**policy,'preview':False,'preview_token':pv['preview_token']};closed('policy-cli-noop-output',['providers','policies','apply'],change)
        s=Server(binary,f.root,output,'http',environment=f.environment);csrf=s.request('/api/v1/native-session')['csrf_token'];pv=s.request('/api/v1/providers/policies',policy,csrf);assert not pv['changed'];unchanged();view=s.request('/api/v1/providers/policies',{**policy,'preview':False,'preview_token':pv['preview_token']},csrf);assert view['applied'] and not view['changed'];unchanged();outcome=s.records[-1]['outcome'];checks.append({'case':'policy-http-noop-outcome','configuration_may_be_recorded':outcome.get('configuration_may_be_recorded')});retained.publish(output/'observations.json',checks)
    except BaseException as error:primary=error
    if s is not None:
        try:s.cleanup()
        except BaseException as cleanup:
            if primary:raise BaseExceptionGroup('No-op observation and cleanup failed',[primary,cleanup])
            raise
    if primary:raise primary
    assert len(checks)==3 and all(row['configuration_may_be_recorded'] is True for row in checks),checks
    retained.publish(output/'result.json',{'status':'PASS','checks':checks,'source_unchanged':True,'no_new_snapshot':True,'provider_started':False,'sha256':retained.digest(binary.read_bytes())});print('CONFIGURATION_NOOP_STATE_VERIFIED 3',flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args()
    try:run(a.binary.resolve(),a.output.resolve())
    except BaseException:
        if a.output.exists():retained.publish(a.output/'failure.txt',traceback.format_exc().encode())
        raise
