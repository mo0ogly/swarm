#!/usr/bin/env python3
"""Own a normal native HTTP process and Chrome proof with no provider/global watchdog.

Credentials travel to Chrome through stdin only. Stream outcomes concurrently so
browser resource loads cannot fill the child stdout pipe. Preserve failed Roots,
primary/cleanup causes, source/binary hashes and safe records. UI proof is separate
from global migration, installation and durable supervisor receipt.
"""
import argparse,json,signal,subprocess,threading,traceback,os
from pathlib import Path
import rust_native_http_cli as native
import rust_agent_model_route as retained



def restarted_cookie(binary,root,output,expected):
    """Restart only this owned fixture and observe the actual saved cookie policy.

    Login form response is consumed before reading its outcome. Cookie bearer
    remains memory-only; record public attributes/revision, not Set-Cookie bytes.
    None must omit Max-Age, and no live Go process is selected or stopped.
    """
    import http.client,urllib.parse,http.cookies
    server=native.Server(binary,root,output,'admin-restarted');primary=None;result=None
    try:
        connection=http.client.HTTPConnection(server.host,server.port,timeout=None)
        try:
            connection.request('POST','/login',body=urllib.parse.urlencode(dict(token=server.key,next='/')).encode(),headers={'Origin':'http://'+server.address,'Content-Type':'application/x-www-form-urlencoded','Accept':'application/json'})
            response=connection.getresponse();raw=response.read();assert response.status==200 and json.loads(raw)['redirect']=='/'
            cookies=http.cookies.SimpleCookie();cookies.load(response.getheader('Set-Cookie'))
        finally:connection.close()
        assert len(cookies)==1;morsel=next(iter(cookies.values()));assert morsel.value==server.key,'Returned scoped credential mismatch'
        assert morsel['httponly'] and morsel['samesite']=='Strict' and morsel['path']=='/'
        assert morsel['max-age']==('' if expected is None else expected)
        assert server.record()['event']=='web-outcome'
        shown=server.request('/api/v1/native-settings');assert shown['values']['web_cookie_max_age_seconds']==expected
        summary=server.stop();assert summary['configuration_outcomes']==0
        result=dict(status='PASS',max_age=expected,http_only=True,same_site='Strict',path='/',saved_revision=shown['revision'],scope='Actual Cookie header after next native bind, not server-side key expiry')
    except BaseException as error:primary=error
    native.finish(server,primary)
    for file in output.rglob('*'):
        if file.is_file():assert server.key.encode() not in file.read_bytes(), 'Restarted credential leaked into retained artifact'
    return result


def main():
    """Run the selected login/Admin scenario on an owned server, including saved
    status windows and v40 report clock tolerance; retain inputs/Root after checked cleanup. No provider or
    global watchdog is added; browser success does not qualify embedded assets."""
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',required=True,type=Path);parser.add_argument('--output',required=True,type=Path);parser.add_argument('--root-workers',type=int);parser.add_argument('--scenario',choices=['login','admin','admin-budget','quotas','review-response','telemetry-settings','status-windows','report-clock'],default='login');parser.add_argument('--npm-source-baseline',type=Path,help='Explicit PASS npm source manifest; revalidated without rearchiving');a=parser.parse_args();binary=a.binary.resolve();output=a.output.resolve();output.mkdir(parents=True,exist_ok=False)
    sources=['tools/verification/verify_npm_archive.py','tests/rust_native_login_browser.py','tests/rust_native_login_ui.cjs','tests/rust_native_http_cli.py','tests/rust_agent_model_route.py','tests/helpers/browser_verification_runtime.cjs','tests/helpers/verification_cleanup.cjs','web/access.html','web/access.css','web/access.js','web/wattson_themes.css','web/swarm-design.css','web/swarm-logo.png','package-lock.json',str(binary)]
    ui_script={'admin':'tests/rust_native_admin_ui.cjs','admin-budget':'tests/admin_budget_http_ui.cjs','login':'tests/rust_native_login_ui.cjs','quotas':'tests/rust_native_quotas_ui.cjs','review-response':'tests/rust_review_response_ui.cjs','telemetry-settings':'tests/rust_telemetry_settings_ui.cjs','status-windows':'tests/rust_status_windows_ui.cjs','report-clock':'tests/rust_report_clock_ui.cjs'}[a.scenario]
    if a.scenario in ['admin','admin-budget','quotas','review-response','telemetry-settings','status-windows','report-clock']:sources.extend(['tests/rust_native_admin_ui.cjs','tests/helpers/engine_settings_admin_state.cjs','web/native-runtime.html','web/native-runtime.css','web/native-runtime.js','web/i18n.js','web/i18n-en.js','web/processes.css','frontend/engine-settings/admin.css','frontend/engine-settings/admin.js','locales/en.json'])
    if a.scenario=='admin-budget':sources.append('tests/admin_budget_http_ui.cjs')
    if a.scenario=='quotas':sources.extend(['tests/rust_native_quotas_ui.cjs','web/native-quotas.js'])
    if a.scenario=='review-response':sources.append('tests/rust_review_response_ui.cjs')
    if a.scenario=='telemetry-settings':sources.append('tests/rust_telemetry_settings_ui.cjs')
    if a.scenario=='status-windows':sources.append('tests/rust_status_windows_ui.cjs')
    if a.scenario=='report-clock':sources.append('tests/rust_report_clock_ui.cjs')
    inputs={n:retained.digest(Path(n).read_bytes()) for n in sources};retained.publish(output/'inputs.json',inputs)
    for n in sources:
        if not n==str(binary):retained.publish(output/('source-'+n.replace('/','_')),Path(n).read_bytes())
    root=native.owned_root(binary)
    if a.root_workers is not None:
        document=native.cli(binary,root,['engine-settings','show']);values=document['values'];values['web_root_worker_threads']=str(a.root_workers)
        change=dict(schema_version=document['schema_version'],event_id='browser-root-workers',expected_revision=document['revision'],values=values,reason='Explicit browser fixture admission capacity',preview_token='')
        preview_file=root/'browser-preview.json';retained.publish(preview_file,change)
        preview=native.cli(binary,root,['engine-settings','preview','--input',str(preview_file)]);change['preview_token']=preview['preview_token']
        apply_file=root/'browser-apply.json';retained.publish(apply_file,change);native.cli(binary,root,['engine-settings','apply','--input',str(apply_file)])
        assert native.cli(binary,root,['engine-settings','show'])['values']['web_root_worker_threads']==str(a.root_workers)
        retained.publish(output/'fixture-policy.json',dict(root_workers=a.root_workers,source='Actual CLI preview/apply/readback before native bind',scope='Private test input, not a production default or raised capacity'))
    if a.scenario in ['admin','quotas']:
        try:
            # Purely inert configuration source: no model cache or provider account.
            helper=root/'skynet_harness'
            retained.publish(helper,b"#!/usr/bin/python3\nfrom pathlib import Path\nPath(__file__).with_name('unexpected-provider-execution').write_text('must not run')\nraise SystemExit(89)\n")
            os.chmod(helper,0o700)
            policy=dict(version=1,billing='external',work_level='standard',page_level='simple',levels={k:dict(model='fixture-admin-model',effort='') for k in ['simple','standard','exigeant']})
            configuration=root/'admin-provider-input.json';retained.publish(configuration,dict(schema_version=1,providers={'admin-fixture':dict(command=str(helper),args=['--model','fixture-admin-model'],model_policy=policy)}))
            native.cli(binary,root,['providers','config','init','--input',str(configuration)])
        except BaseException:
            retained.publish(output/'owned-setup-failure.json',dict(root=str(root),binary_sha256=retained.digest(binary.read_bytes()),provider_process_started=False,http_process_started=False,scope='Owned fixture setup failed before server; preserved'))
            raise
    server=native.Server(binary,root,output,'browser-native');primary=None;errors=[];summaries=[];success=False
    def collect():
        """Continuously drain owner records; retain exactly one final stopped summary."""
        try:
            while True:
                record=server.record()
                if record['event']=='web-stopped':summaries.append(record);return
                assert record['event']=='web-outcome'
        except BaseException as error:errors.append(error)
    collector=threading.Thread(target=collect);collector.start()
    try:
        payload=json.dumps(dict(address=server.address,key=server.key,root=str(root),binary=str(binary),output=str(output/'browser'))).encode()
        with (output/'browser.stdout').open('xb') as out,(output/'browser.stderr').open('xb') as err:
            run=subprocess.run(['node',ui_script],input=payload,stdout=out,stderr=err,env={**os.environ,**({'SWARM_VERIFICATION_NPM_BASELINE':str(a.npm_source_baseline.resolve())} if a.npm_source_baseline else {})})
        assert run.returncode==0,('Browser verification failed; safe stderr retained',run.returncode)
        browser_result=json.loads((output/'browser/result.json').read_bytes());assert browser_result['status']=='PASS'
        server.process.send_signal(signal.SIGINT);server.stop_sent=True;collector.join();assert not errors and len(summaries)==1
        server.stderr=server.process.stderr.read();assert server.process.wait()==0 and not server.stderr
        assert summaries[0]['rejected_requests']==browser_result.get('expected_rejected_requests',4) and summaries[0]['configuration_outcomes']==browser_result.get('expected_configuration_outcomes',0)
        if a.scenario=='admin-budget':
            shown=native.cli(binary,root,['admin-budget','show']);assert shown['configured'] and shown['reserved_estimate_micro_usd']=='0' and shown['actual_billed_cost_known'] is False
        if a.scenario=='admin':
            shown=native.cli(binary,root,['engine-settings','show']);assert all(shown['values'][name]==value for name,value in browser_result['expected_mission_policy'].items());assert shown['revision']==browser_result['expected_revision'] and shown['values']['review_delivery_max_bytes']==browser_result['expected_review_cap'] and shown['values']['web_cookie_max_age_seconds']==browser_result['expected_cookie_max_age']
        assert summaries[0]['completed_jobs']>0 and summaries[0]['observed_outcomes']==summaries[0]['completed_jobs']
        assert all(retained.digest(Path(n).read_bytes())==h for n,h in inputs.items())
        # npm inputs are immutable canonical dependencies; the private Chrome
        # profile stays outside evidence. Check every emitted raw artifact too.
        for file in output.rglob('*'):
            if file.is_file():assert server.key.encode() not in file.read_bytes(), 'Credential leaked into retained artifact'
        success=True
    except BaseException as error:primary=error
    finally:
        if server.process.poll() is None and not server.stop_sent:server.process.send_signal(signal.SIGINT);server.stop_sent=True
        collector.join()
        try:native.finish(server,primary)
        except BaseException as error:
            retained.publish(output/'failure.json',dict(status='FAIL',type=type(error).__name__,message=str(error),traceback=traceback.format_exc(),collector_errors=[type(e).__name__ for e in errors],retained_root=str(root)));raise
    if errors:raise BaseExceptionGroup('Outcome collection failed',errors)
    assert success
    cookie_after_restart=None
    if a.scenario=='admin':
        try:cookie_after_restart=restarted_cookie(binary,root,output,browser_result['expected_cookie_max_age'])
        except BaseException as error:
            retained.publish(output/'failure.json',dict(status='FAIL',phase='cookie-restart',type=type(error).__name__,message=str(error),traceback=traceback.format_exc(),retained_root=str(root)));raise
    retained.publish(output/'result.json',dict(cookie_after_restart=cookie_after_restart,status='PASS',scope='Actual normal native CLI/TCP/browser, no Go restart or full cockpit proof',scenario=a.scenario,checks=4,source_stable=True,global_watchdog=None,owner_summary=summaries[0],credential_not_in_evidence=True,retained_root=str(root)))
    print('PASS native-'+a.scenario+'-browser FR/EN etat/sombre actual TCP; fixture Root retained',flush=True)


if __name__=='__main__':main()
