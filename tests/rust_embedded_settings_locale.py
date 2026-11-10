#!/usr/bin/env python3
"""Check the actual normal HTTP executable's embedded Admin English catalogue.

Uses an owned ephemeral server/Root and its scoped session; credentials remain
private. Valid protected capability response precedes static-asset assertions.
No installed service, provider process or watchdog. Preserve primary/cleanup
errors independently and retain every failed Root and original response bytes.
"""
import argparse,hashlib,http.client,json,traceback
from pathlib import Path
import rust_agent_model_route as common
from rust_native_http_cli import Server,finish


def verify(binary, profile, baseline, output, field_names=None):
    """Bind the served resource to the qualified effective-input hash and labels.

    Actual engine catalogue supplies field/help keys; English must exist in the
    embedded script, never just in a current workspace file. This does not verify
    all translations, complete product UI, provider identity or installation.
    Optional exact field names select their label/help contracts; omitted names
    retain the historical cost-policy check, never claim whole-catalog coverage.
    For this additive change retain all baseline keys so unchanged old consumers
    cannot lose their translations while new settings gain theirs.
    """
    out=output.resolve();out.mkdir(parents=True,exist_ok=False)
    common.publish(out/'method.py',Path(__file__).read_bytes())
    f=common.Fixture(binary,out,'embedded-locale')
    server=None;primary=None
    try:
        descriptor=f.ok(['engine-settings','describe'])
        names=field_names or ['mission_dispatch_cost_reserve_percent']
        fields=[next(x for x in descriptor['fields'] if x['name']==name) for name in names]
        server=Server(binary,f.root,out,'server')
        witness=server.request('/api/v1/native-capabilities')
        assert witness['profile']=='rust-native-http1-observed'
        common.publish(out/'valid-witness.json',dict(profile=witness['profile'],catalog_schema=descriptor['document_schema_version']))
        connection=http.client.HTTPConnection(server.host,server.port,timeout=None)
        try:
            connection.request('GET','/native-runtime/english.js',headers={'Host':server.address,'Origin':'http://'+server.address,'Cookie':server.cookie})
            response=connection.getresponse();raw=response.read();status=response.status
        finally:
            connection.close()
        common.publish(out/'english.js',raw)
        event=server.record()
        assert event['event']=='web-outcome' and event['outcome']['status']==status
        assert status==200
        expected=json.loads((profile.parent/'after.json').read_bytes())['web/i18n-en.js']
        assert hashlib.sha256(raw).hexdigest()==expected,'served asset differs from qualified effective input'
        text=raw.decode('utf-8')
        english=json.loads(text.split('globalThis.SwarmEnglish = ',1)[1].rsplit(';',1)[0])
        previous=json.loads(baseline.read_text().split('globalThis.SwarmEnglish = ',1)[1].rsplit(';',1)[0])
        missing=previous.keys()-english.keys()
        assert not missing,('embedded locale dropped existing consumer keys',sorted(missing))
        common.publish(out/'baseline.json',dict(path=str(baseline),sha256=common.digest(baseline.read_bytes()),previous_keys=len(previous)))
        for field in fields:
            for key in [field['label'],field['help']]:
                assert key in english,('missing embedded English setting translation',key)
                assert english[key] and english[key]!=key
        common.publish(out/'result.json',dict(status='PASS',normal_sha256=common.digest(binary.read_bytes()),asset_sha256=expected,fields=names,installed=False))
        print('PASS actual embedded Admin locale matches qualified binary and setting labels',flush=True)
    except BaseException as e:
        primary=e
        common.publish(out/'failure.json',dict(status='FAIL',message=str(e),traceback=traceback.format_exc(),root=str(f.root)))
    if server is not None:finish(server,primary)
    elif primary is not None:raise primary


def main():
    """Require fresh evidence and a qualified normal profile matching the binary."""
    p=argparse.ArgumentParser(description=__doc__)
    for key in ['binary','profile','baseline','output']:p.add_argument('--'+key,type=Path,required=True)
    p.add_argument('--field',action='append',help='Exact catalogue field to verify; repeatable, legacy cost policy by default')
    a=p.parse_args();binary=a.binary.resolve();profile=a.profile.resolve()
    data=json.loads(profile.read_bytes())
    assert data['no_fixture_feature'] and data['sha256']==common.digest(binary.read_bytes())
    verify(binary,profile,a.baseline.resolve(),a.output,a.field)


if __name__=='__main__':
    main()
