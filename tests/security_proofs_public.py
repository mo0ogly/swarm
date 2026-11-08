#!/usr/bin/env python3
"""Exercise security metadata through actual CLI/HTTP/browser on an owned Root.

No providers, live state, proof import or qualification grant. The caller supplies
the exact normal binary. Browser credentials are synthetic, private and excluded
from the public result. The runner has no global watchdog; preserves failures.
"""
import argparse, hashlib, http.client, json, os, subprocess
from pathlib import Path
from rust_agent_model_route import Fixture, publish
from rust_native_http_cli import Server


def main():
    """Run normal/refusal/recovery cases, byte checks and actual browser gestures.

    All mutations initialize only this owned fixture via public operations. Hash
    retained Trust bytes before/after each observation; no SQL mutation or paid IO.
    Failures and cleanup errors retain their cause. Browser credential fields are
    masked; the browser profile/config remain private staging artifacts.
    """
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--binary',type=Path,required=True);ap.add_argument('--output',type=Path,required=True)
    a=ap.parse_args();binary=a.binary.resolve(strict=True);out=a.output.resolve();out.mkdir(mode=0o700,exist_ok=False)
    f=Fixture(binary,out,'security-proofs');server=None;checks=[];primary=None
    def check(name,value):
        """Retain each named assertion before any failure."""
        checks.append(dict(name=name,passed=bool(value)));publish(out/'checks.json',checks);assert value,name
    def trust(op,value,token='fixture-admin-token',success=True):
        """Use actual CLI with a synthetic credential outside argv/proof output."""
        f.environment['SWARM_TRUST_TOKEN']=token
        code,result=f.call(['trust',op],value);assert (code==0)==success,(op,code,result)
        return result
    try:
        trust('config-apply',dict(schema_version=1,event_id='init',expected_revision=0,reason='Owned security fixture',policy=dict(mandate_ttl_seconds=3600,unknown_reconcile_seconds=0,evidence_retention_days=365,require_separation=True),identities=[dict(subject='admin',integrity=3,roles=['admin'],token='fixture-admin-token'),dict(subject='worker',integrity=1,roles=['requester'],token='fixture-worker-token'),dict(subject='approver',integrity=2,roles=['approver'],token='fixture-approver-token')]),token='')
        state=f.root/'.swarm/trust-actions.json';before=state.read_bytes()
        cli=trust('security-show',dict(schema_version=1));check('cli-no-mutation',before==state.read_bytes())
        check('cli-no-policy',cli['domains'][0]['configuration']['configured'] is False)
        check('cli-no-success',cli['domains'][0]['verification']==dict(result='NOT_TESTED',freshness='unknown',evidence_loaded=False))
        for token in ['', 'fixture-worker-token','invalid-fixture-token']:
            result=trust('security-show',dict(schema_version=1),token=token,success=False)
            check('cli-refusal-'+(token or 'session'),result['failure']['code']=='authorization_required')
        server=Server(binary,f.root,out,'security-http',environment=f.environment);server.capture_async(out/'http.raw')
        csrf=server.request('/api/v1/native-session',observe=False)['csrf_token']
        def request(body,token='fixture-admin-token',status=200):
            """Read the actual protected API, preserving status and closing IO."""
            connection=http.client.HTTPConnection(server.host,server.port)
            try:
                headers={'Cookie':server.cookie,'Origin':'http://'+server.address,'X-Swarm-CSRF':csrf,'Content-Type':'application/json'}
                if token:headers['Authorization']='Bearer '+token
                connection.request('POST','/api/v1/trust-actions?action=security-show',body=body,headers=headers)
                response=connection.getresponse();raw=response.read();assert response.status==status,(response.status,raw)
                return json.loads(raw)
            finally:connection.close()
        observed=request('{"schema_version":1}')
        check('cli-http-exact-parity',observed==cli)
        for token in ['', 'fixture-worker-token','invalid-fixture-token']:
            check('http-refusal-'+(token or 'session'),request('{"schema_version":1}',token,status=403)['failure']['code']=='authorization_required')
        for invalid in ['{}','{"schema_version":2}','{"schema_version":1,"accepted":true}','{"schema_version":1,"schema_version":1}']:
            check('http-invalid-'+str(len(checks)),request(invalid,status=400)['failure']['code']=='format')
        check('http-no-mutation',before==state.read_bytes())
        trust('blp-policy-apply',dict(schema_version=1,event_id='policy',expected_revision=0,expected_trust_revision=1,reason='Explicit owned fixture policy',policy=dict(schema_version=1,levels=[dict(id='public',name_fr='Public',name_en='Public')],compartments=[],clearances={})))
        configured=trust('security-show',dict(schema_version=1))
        check('http-configured-exact-cli-parity',request('{"schema_version":1}')==configured)
        check('configured-stays-unqualified',configured['domains'][0]['configuration']['configured'] is True and configured['domains'][0]['qualification']['state']=='not_qualified')
        before=state.read_bytes()
        # Exact embedded bytes, not merely existence or content type.
        for filename in ['security-proofs.html','security-proofs.js','security-proofs.css']:
            connection=http.client.HTTPConnection(server.host,server.port)
            try:
                connection.request('GET','/'+filename,headers={'Cookie':server.cookie});response=connection.getresponse();raw=response.read()
                check('embedded-'+filename,response.status==200 and raw==(Path('web')/filename).read_bytes())
            finally:connection.close()
        private=out/'evidence';private.mkdir(mode=0o700)
        browser_out=out/'browser';browser_out.mkdir(mode=0o700)
        config=dict(url='http://'+server.address+'/',cookie_name=server.cookie.split('=',1)[0],cookie_value=server.key,profile=str(private/'profile'),output=str(browser_out))
        publish(private/'browser-config.json',config);os.chmod(private/'browser-config.json',0o600)
        run=subprocess.run(['node','tests/security_proofs_ui.cjs'],env=dict(os.environ,SWARM_SECURITY_UI_CONFIG=str(private/'browser-config.json')),capture_output=True)
        publish(out/'browser.stdout',run.stdout);publish(out/'browser.stderr',run.stderr)
        check('browser-public-journey',run.returncode==0 and b'SECURITY_PROOFS_BROWSER_PASS ' in run.stdout)
        check('browser-no-mutation',before==state.read_bytes())
        publish(out/'result.json',dict(status='PASS',checks=checks,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),scope='actual CLI/HTTP/browser on owned fixture; real Authority session and independent acceptance not established'))
        print('SECURITY_PROOFS_PUBLIC_PASS '+str(len(checks)),flush=True)
    except BaseException as error:primary=error;raise
    finally:
        if server is not None:
            try:server.cleanup()
            except BaseException as secondary:
                if primary is not None:raise BaseExceptionGroup('Journey and cleanup failed',[primary,secondary])
                raise


if __name__=='__main__':main()
