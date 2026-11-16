#!/usr/bin/env python3
"""Verify native identity presentation transport on an owned inactive Root.

Public CLI prepares a worker/mapping; no provider runs. The real owner-key HTTP
server supplies assets compared with the qualified image's archived inputs,
never current workspace substitutes. Optional browser hold publishes only the
fixture address and private credential file path, not its contents. A separate
CUA observer performs real browser interaction; HTTP PASS does not prove rendering.
No live Root, global watchdog, automatic retry, compilation or installation.
"""
import argparse
import hashlib
from http.client import HTTPConnection
import json
import os
from pathlib import Path
import sys
import tarfile

from native_task_mandate import Run, require
from rust_agent_model_route import publish
from rust_native_http_cli import Server


def qualified_inputs(binary, record_path):
    """Validate the immutable default-profile image and retained source archive.

    Return the record, complete input hashes and archive path. Malformed/missing
    qualification, image mismatch or changed archived bytes refuse before creating
    a Root. No claim is made about an unrelated active service or current sources.
    """
    record = json.loads(record_path.read_bytes())
    image_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    require(record['binary_sha256'] == image_hash, 'qualified-image-mismatch')
    profile = Path(record['evidence']) / 'build/profile'
    result = json.loads((profile / 'result.json').read_bytes())
    require(result['status'] == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
            and result['sha256'] == image_hash
            and result['no_fixture_feature'] is True
            and result['all_core_features_empty'] is True
            and result['all_core_test_false'] is True, 'default-profile-required')
    inputs = json.loads((profile / 'after.json').read_bytes())
    archive = profile / 'effective-sources.tar.gz'
    expected = result['effective_sources_archived']['sha256']
    require(hashlib.sha256(archive.read_bytes()).hexdigest() == expected,
            'qualified-source-archive-mismatch')
    return record, inputs, archive


def response(server, path, *, authenticated=True, host=None):
    """Read actual HTTP bytes/status with optional bad-auth/Host test inputs.

    The caller owns this server; credentials stay in memory and are never returned
    or logged. Connections close on every outcome, preserving primary and cleanup
    causes together. No global network deadline substitutes for progress diagnosis.
    """
    headers = {'Cookie': server.cookie} if authenticated else {}
    if host is not None:
        headers['Host'] = host
    connection = HTTPConnection(server.host, server.port, timeout=None)
    try:
        connection.request('GET', path, headers=headers)
        incoming = connection.getresponse()
        result = incoming.status, incoming.read()
    except BaseException as primary:
        try:
            connection.close()
        except BaseException as secondary:
            raise BaseExceptionGroup('HTTP read and close failed', [primary, secondary])
        raise
    connection.close()
    require(server.key.encode() not in result[1], 'session-credential-in-response')
    number = len(server.requests)
    body_path = 'http-' + str(number).zfill(3) + '.body'
    publish(server.output / body_path, result[1])
    server.requests.append(dict(path=path, status=result[0], body=body_path,
                               authenticated=authenticated, host_override=host))
    return result


def main():
    """Run named HTTP controls, optionally hold for CUA, and drain the owned server.

    A hold blocks on an owned FIFO until the controller sends exactly 'finish';
    there is no timeout or automatic retry. Failure retains the original Root and
    evidence. Browser observation is separate; no empty browser report is promoted
    to PASS. The final Trust journal must remain byte-identical after read-only UI.
    """
    if sys.flags.optimize:
        raise RuntimeError('optimized Python forbidden by imported fixture contract')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--build-record', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--hold-browser', action='store_true')
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    record, inputs, archive = qualified_inputs(binary, args.build_record)
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    assertions = []

    def check(name, condition):
        """Retain each unique successful observation; false is never a green exit."""
        require(condition and name not in assertions, name)
        assertions.append(name)
        publish(output / 'assertions.json', assertions)
        print('PASS ' + name, flush=True)

    run = Run(binary, output / 'prepared')
    server = None
    original = (run.f.root / '.swarm/trust-actions.json').read_bytes()
    try:
        server = Server(binary, run.f.root, output, 'native-identity-web',
                        environment=run.f.environment)
        server.capture_async(output / 'server-stream.jsonl')
        status, raw = response(server, '/api/v1/native-version')
        version = json.loads(raw)
        check('served-version-matches-qualified-image', status == 200
              and version['binary']['version'] == record['version'])
        status, raw = response(server, '/api/v1/trust-actions')
        state = json.loads(raw)
        natives = [identity for identity in state['identities'] if identity.get('native_scope')]
        check('native-mapping-readback-exact', status == 200 and len(natives) == 1
              and natives[0]['subject'] == 'native-worker'
              and natives[0]['native_scope'] == run.scope)
        check('public-native-identity-has-no-bearer', 'token' not in natives[0]
              and 'token_hash' not in natives[0])
        check('presentation-creates-no-effect', state['ledger'] == [] and state['actions'] == {})
        status, raw = response(server, '/api/v1/trust-actions', authenticated=False)
        check('bare-session-refused', status == 403 and raw == b'web-access-refused')
        status, raw = response(server, '/api/v1/trust-actions', host='untrusted.invalid')
        check('wrong-host-refused', status == 403 and raw == b'web-access-refused')
        with tarfile.open(archive) as source:
            assets = {}
            for route, path in [('/trust-actions.html', 'web/trust-native.html'),
                                ('/trust-native.js', 'web/trust-native.js'),
                                ('/trust-native.css', 'web/trust-native.css'),
                                ('/i18n-en.js', 'web/i18n-en.js')]:
                stream = source.extractfile(path)
                require(stream is not None, 'missing-archived-asset-' + path)
                with stream:
                    expected = stream.read()
                status, actual = response(server, route)
                digest = hashlib.sha256(actual).hexdigest()
                check('served-qualified-asset-' + route, status == 200
                      and actual == expected and digest == inputs[path])
                assets[route] = dict(source=path, sha256=digest, bytes=len(actual))
            publish(output / 'served-assets.json', assets)
        check('faults-and-reads-preserve-trust-journal',
              original == (run.f.root / '.swarm/trust-actions.json').read_bytes())
        if args.hold_browser:
            fifo = output / 'browser-release.fifo'
            os.mkfifo(fifo, 0o600)
            session_file = run.f.root / '.swarm' / (
                'web-session-' + hashlib.sha256(server.address.encode()).hexdigest()[:16])
            publish(output / 'browser-ready.json', dict(address=server.address,
                    url='http://' + server.address + '/trust-actions.html?view=security&lang=fr&theme=etat',
                    session_file=str(session_file), root=str(run.f.root), pid=server.process.pid,
                    release_fifo=str(fifo), binary_sha256=record['binary_sha256'],
                    scope='isolated fixture; not the installed authority service'))
            print('BROWSER_READY ' + server.address, flush=True)
            with fifo.open() as release:
                require(release.read() == 'finish\n', 'unexpected-browser-release')
            check('browser-read-only-preserves-trust-journal',
                  original == (run.f.root / '.swarm/trust-actions.json').read_bytes())
    except BaseException as primary:
        if server is not None:
            try:
                server.cleanup()
            except BaseException as secondary:
                raise BaseExceptionGroup('Verification and cleanup failed', [primary, secondary])
        raise
    else:
        server.cleanup()
        check('owned-server-drained-without-error', server.process.returncode == 0)
    check('qualified-image-unchanged',
          hashlib.sha256(binary.read_bytes()).hexdigest() == record['binary_sha256'])
    publish(output / 'summary.json', dict(status='PASS', assertions=assertions,
            binary_sha256=record['binary_sha256'], version=record['version'],
            browser='separate CUA record required' if args.hold_browser else 'NOT_EXERCISED',
            scope='HTTP native identity projection and compiled assets; no provider/effect'))
    print('NATIVE_TASK_MANDATE_WEB_VERIFIED assertions=' + str(len(assertions)), flush=True)


if __name__ == '__main__':
    main()
