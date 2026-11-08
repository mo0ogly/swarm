"""Final native cockpit recipe: private Root, real CLI/API, provider-free Chrome.

No live database or secret is retained in reports. Server records are drained
concurrently; no global watchdog is installed. Failed Roots and originals survive.
A passing browser recipe is not complete migration or independent acceptance.
"""
import argparse,hashlib,json,signal,subprocess,threading,os
from pathlib import Path
import rust_native_http_cli as native
import rust_agent_model_route as retained


def main():
    """Create private Work/task fixtures via public CLI (plus one planning Work in
    existing mode). Exercise real reads and explicit isolated budget/quota/price
    operations and marked client faults, then close only the owned server.
    Preserve primary/cleanup causes,
    exact binary/source hashes and browser outputs; never start a provider."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--existing',action='store_true');a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    root=native.owned_root(binary);works=[];title='Alpha <img src=x onerror=alert(1)>'
    for i,name in enumerate([title,'Mes missions']):
        spec=out/('create-'+str(i)+'.json');retained.publish(spec,dict(schema_version=1,event_id='cockpit-'+str(i),expected_revision=0,title=name,objective='Owned objective <b>text</b>',scope='Owned UX fixture',criteria=['Readable']))
        works.append(native.cli(binary,root,['work','create','--input',str(spec)])['work']['id'])
    if a.existing:
        spec=out/'add-task.json';retained.publish(spec,dict(schema_version=1,event_id='ux-task',expected_revision=1,id='ux-task',title='Task <b>plain</b>',deliverable='Read only fixture',criteria=['Visible, not accepted']))
        native.cli(binary,root,['task','add',works[0],'--input',str(spec)])
        before=native.cli(binary,root,['work','show',works[0]])['work']
        spec=out/'add-dependent.json';retained.publish(spec,dict(schema_version=1,event_id='ux-dependent',expected_revision=before['revision'],id='ux-dependent',title='Dependent task',deliverable='Read only dependency',criteria=['Dependency visible'],depends=['ux-task']))
        native.cli(binary,root,['task','add',works[0],'--input',str(spec)])
        before=native.cli(binary,root,['work','show',works[0]])['work']
        spec=out/'budget.json';retained.publish(spec,dict(schema_version=1,event_id='ux-budget',expected_revision=before['revision'],budget=dict(limit_usd=37.25,reserve_per_launch_usd=1.5,estimate_source='Explicit inert UX fixture',reference_date='2026-10-24')))
        native.cli(binary,root,['budget','apply',works[0],'--input',str(spec)])
    quota_work=None
    if a.existing:
        helper=root/'skynet_harness';retained.publish(helper,b"#!/usr/bin/python3\nfrom pathlib import Path\nPath(__file__).with_name('unexpected-provider-execution').write_text('must not run')\nraise SystemExit(89)\n");os.chmod(helper,0o700)
        policy=dict(version=1,billing='external',work_level='standard',page_level='simple',levels={k:dict(model='fixture-admin-model',effort='') for k in ['simple','standard','exigeant']})
        spec=out/'quota-provider.json';retained.publish(spec,dict(schema_version=1,providers={'admin-fixture':dict(command=str(helper),args=['--model','fixture-admin-model'],model_policy=policy)}));native.cli(binary,root,['providers','config','init','--input',str(spec)])
        spec=out/'quota-create.json';retained.publish(spec,dict(schema_version=1,event_id='quota-create',expected_revision=0,title='Quota planning fixture',objective='Configure without execution',scope='Owned quota fixture',criteria=['Preserved usage']))
        quota_work=native.cli(binary,root,['work','create','--input',str(spec)])['work']['id']
        spec=out/'quota-enable.json';retained.publish(spec,dict(schema_version=1,event_id='quota-enable',expected_revision=1,provider='admin-fixture',level='standard',max_activations=3,max_tasks=10,max_decisions=10,max_review_calls=2,checks={'req-1':[dict(id='quota-inert-check',command=['python3',str(helper)],criteria=[1],justification='Declared fixture control only, never executed')]}));native.cli(binary,root,['planning','enable',quota_work,'--input',str(spec)])
    server=native.Server(binary,root,out,'cockpit');primary=None;collector_errors=[]
    def drain():
        """Drain only owned protocol frames through actual stopped event."""
        try:
            while server.record()['event']!='web-stopped':pass
        except BaseException as error:collector_errors.append(error)
    t=threading.Thread(target=drain);t.start()
    try:
        if a.existing:
            # Real successful observation precedes public refusal cases; no provider.
            path='/api/v1/cockpit-read?work='+works[0]
            observed=server.request(path,observe=False)
            assert observed['work_id']==works[0] and observed['budget']['limit_usd']==37.25 and observed['agents']==[]
            assert server.request('/api/v1/cockpit-read?work='+works[1],observe=False)['budget'] is None
            for query in ['', '?work=../escape', '?work='+works[0]+'&work='+works[1], '?work='+works[0]+'&unknown=1']:
                refused=server.request('/api/v1/cockpit-read'+query,status=400,observe=False)
                assert refused['failure']['code']=='cockpit-query-invalid'
            server.request(path,authorized=False,status=403,observe=False)
            server.request(path,body={},status=403,observe=False)
            assert server.request(path,observe=False)==observed
            csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
            import rust_resume_http_contract
            rust_resume_http_contract.verify(server,binary,root,works[0],csrf)
            import rust_pricing_http_contract
            rust_pricing_http_contract.verify(server,binary,root,works[0],csrf)
            base='/api/v1/work-budget?work='+works[0];before=server.request(base,observe=False)
            assert before['budget']['limit_usd']==37.25 and before['work_id']==works[0]
            proposal=dict(schema_version=1,event_id='budget-http-proof',expected_revision=before['revision'],budget=dict(limit_usd=37.25,reserve_per_launch_usd=1.5,estimate_source='Explicit inert UX fixture',reference_date='2026-10-24',actor='supplied-not-authority',updated='supplied-not-time'))
            preview=server.request('/api/v1/work-budget/preview?work='+works[0],proposal,csrf,observe=False)
            assert preview['proposed']['actor']=='' and preview['proposed']['updated']==''
            assert server.request(base,observe=False)==before
            bad={**proposal,'event_id':'budget-invalid-date','budget':{**proposal['budget'],'reference_date':'2026-02-30'}}
            server.request('/api/v1/work-budget/preview?work='+works[0],bad,csrf,status=400,observe=False)
            server.request('/api/v1/work-budget/preview?work='+works[0],{**proposal,'expected_revision':before['revision']-1},csrf,status=409,observe=False)
            assert server.request(base,observe=False)==before
            applied=server.request('/api/v1/work-budget/apply?work='+works[0],proposal,csrf,observe=False)
            assert applied['revision']==before['revision']+1
            again=server.request('/api/v1/work-budget/apply?work='+works[0],proposal,csrf,observe=False)
            after=server.request(base,observe=False);assert again['revision']==applied['revision']==after['revision']
            assert after['budget']['actor']!='supplied-not-authority' and after['budget']['updated']!='supplied-not-time'
            assert after['state']==before['state']
            server.request('/api/v1/work-budget/apply?work='+works[0],{**proposal,'event_id':'budget-stale-write'},csrf,status=409,observe=False)
            server.request('/api/v1/work-budget/apply?work='+works[0],{**proposal,'event_id':'budget-no-csrf'},status=403,observe=False)
            assert server.request(base,observe=False)==after
            assert server.request('/api/v1/work-budget?work='+works[1],observe=False)['budget'] is None
            retained.publish(out/'budget-route.json',dict(status='PASS',checks=['actual-policy','preview-no-write','preview-sanitizes-metadata','invalid-date-refused','stale-preview-refused','CAS-apply','same-event-replay-no-new-revision','engine-owned-metadata','financial-observation-preserved','stale-apply-refused','CSRF-refused','post-refusal-unchanged','missing-policy-null'],requests=server.requests))
            retained.publish(out/'observation-route.json',dict(status='PASS',checks=['actual-budget','missing-policy-null','missing-work-refused','unsafe-work-refused','duplicate-work-refused','unknown-query-refused','unauthorized-refused','uncredentialed-post-refused','recovery-unchanged'],requests=server.requests))
        browser=out/'browser';browser.mkdir();cfg=dict(address=server.address,key=server.key,root=str(root),output=str(browser),works=works,title=title,quota_work=quota_work)
        with (out/'browser.stdout').open('xb') as stdout,(out/'browser.stderr').open('xb') as stderr:
            run=subprocess.run(['node','tests/rust_existing_cockpit_ui.cjs' if a.existing else 'tests/rust_native_cockpit_ui.cjs'],input=json.dumps(cfg).encode(),stdout=stdout,stderr=stderr)
        assert run.returncode==0,'Cockpit browser failed; inspect retained stderr'
        result=json.loads((browser/'result.json').read_text());assert result['status']=='PASS' and result['checks'];assert not (root/'unexpected-provider-execution').exists()
        server.process.send_signal(signal.SIGINT);server.stop_sent=True;t.join();assert not collector_errors
        inputs=[binary,Path('tests/rust_native_cockpit_browser.py'),Path('tests/rust_native_cockpit_ui.cjs'),Path('web/native-cockpit.js'),Path('web/native-quotas.js'),Path('web/pricing.js'),Path('web/runtime-health.js'),Path('web/cockpit-native-instructions.js'),Path('web/cockpit-native-resume.js'),Path('tests/rust_existing_resume_flow.cjs'),Path('tests/rust_resume_http_contract.py'),Path('web/cockpit-native-version.js'),Path('tests/rust_existing_version_flow.cjs'),Path('web/cockpit-native-pricing.js'),Path('tests/rust_existing_pricing_flow.cjs'),Path('tests/rust_pricing_http_contract.py'),Path('web/cockpit-native-quotas.js'),Path('web/native-runtime.js'),Path('web/native-runtime.css'),Path('web/native-runtime.html'),Path('web/i18n-en.js'),Path('web/index.html'),Path('web/cockpit-native.js'),Path('web/cockpit-native-read.js'),Path('web/cockpit-native-panels.js'),Path('web/cockpit-native-budget.js'),Path('web/cockpit-native-graph.js'),Path('web/graph.js'),Path('web/lib/dagre.min.js'),Path('tests/rust_existing_cockpit_ui.cjs')]
        retained.publish(out/'result.json',dict(status='PASS',checks=result['checks'],inputs={str(n):hashlib.sha256(n.read_bytes()).hexdigest() for n in inputs},root=str(root),global_watchdog=None,provider_started=False,global_acceptance=False))
        print(('NATIVE_EXISTING_COCKPIT_PASS ' if a.existing else 'NATIVE_COCKPIT_PASS ')+str(len(result['checks']))+' named assertions',flush=True)
    except BaseException as error:primary=error
    finally:
        if server.process.poll() is None and not server.stop_sent:server.process.send_signal(signal.SIGINT);server.stop_sent=True
        t.join()
    native.finish(server,primary)


if __name__=='__main__':main()
