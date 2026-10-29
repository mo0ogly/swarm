#!/usr/bin/env python3
"""Exercise saved cockpit defaults through real CLI and protected HTTP.

Owned inert Root only. No provider, conductor, live mission, SQL fixture write,
installation or global watchdog. Exact binary and named results are retained;
failures and owned-server cleanup causes remain visible together.
"""
import argparse
import hashlib
import json
import subprocess
import traceback
from pathlib import Path
import rust_native_http_cli as native
import rust_agent_model_route as retained


def historical(binary, legacy, out):
    """Read an actual v47 initializer's Root, then explicitly adopt only v48.

    Both binaries act through public CLI/API on an owned fixture. A read must
    preserve exact settings bytes and absent controls/consent. Preview remains
    read-only; apply preserves history and Work state. No SQL injection or model
    process is used. Original errors and owned cleanup failures propagate.
    """
    out.mkdir(mode=0o700)
    root = native.owned_root(legacy)
    old = native.cli(legacy, root, ['engine-settings', 'show'])
    assert old['schema_version'] == 47 and 'cockpit_default_autonomy' not in old['values']
    spec = out / 'create.json'
    retained.publish(spec, dict(schema_version=1, event_id='historical-autonomy-create', expected_revision=0,
        title='Historical cockpit autonomy', objective='No implicit adoption', scope='Owned fixture', criteria=['Preserve absence']))
    work = native.cli(legacy, root, ['work', 'create', '--input', str(spec)])['work']
    before = (root / '.swarm/engine-settings.json').read_bytes()
    assert native.cli(binary, root, ['engine-settings', 'show']) == old
    server = native.Server(binary, root, out, 'historical')
    primary = None
    try:
        server.capture_async(out / 'server.jsonl')
        path = '/api/v1/cockpit-read?work=' + work['id']
        observed = server.request(path, observe=False)['autonomy_observation']
        assert observed['autonomy'] is None and observed['source'] == 'not_configured'
        assert observed['consent_enabled'] is False and observed['launch_authorized'] is False
        assert (root / '.swarm/engine-settings.json').read_bytes() == before
        session = server.request('/api/v1/native-session', observe=False)
        change = dict(schema_version=48, event_id='historical-explicit-adoption', expected_revision=old['revision'],
            values={**old['values'], 'cockpit_default_autonomy': '2'}, reason='Explicit fixture adoption', preview_token='')
        preview = server.request('/api/v1/native-settings/preview', change, session['csrf_token'], observe=False)
        assert (root / '.swarm/engine-settings.json').read_bytes() == before
        change['preview_token'] = preview['preview_token']
        server.request('/api/v1/native-settings/apply', change, session['csrf_token'], observe=False)
        current = native.cli(binary, root, ['engine-settings', 'show'])
        assert current['schema_version'] == 48 and current['history'][:-1] == old['history']
        assert current['values'] == change['values']
        observed = server.request(path, observe=False)['autonomy_observation']
        assert observed['autonomy'] == 'autonome' and observed['source'] == 'installation'
        assert observed['consent_enabled'] is False and observed['launch_authorized'] is False
        assert native.cli(binary, root, ['work', 'show', work['id']])['work'] == work
        controls = native.cli(binary, root, ['mission', 'show', work['id']])
        assert controls['autonomy'] is None and controls['consent_enabled'] is None
        retained.publish(out / 'result.json', dict(status='PASS', root=str(root), work=work['id'],
            legacy_sha256=hashlib.sha256(legacy.read_bytes()).hexdigest(), current_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
            old_schema=47, adopted_schema=48, read_no_adoption=True, history_preserved=True,
            no_inferred_consent=True, provider_started=False, installed=False))
        print('COCKPIT_AUTONOMY_HISTORICAL_VERIFIED', flush=True)
    except BaseException as error:
        primary = error
        retained.publish(out / 'failure.json', dict(status='FAIL', cause=traceback.format_exc()))
    finally:
        try:
            server.cleanup()
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup('Historical flow and cleanup', [primary, cleanup])
            raise
    if primary is not None:
        raise primary


def run(binary, out, browser=False, profile_inputs=None):
    """Check default, explicit saves, refusal without mutation and mission priority.

    CLI creates the fixture; HTTP uses its genuine admission cookie and CSRF.
    Settings preview is asserted read-only, save is reread through CLI and HTTP,
    and pause creates real manual controls which override later installation
    defaults. No observation is treated as launch authority or consent.
    """
    catalog = native.cli(binary, out, ['engine-settings', 'describe'])
    assert catalog['schema_version'] == 6, 'scalar-choice-catalog-not-present'
    field = next(f for f in catalog['fields'] if f['name'] == 'cockpit_default_autonomy')
    assert field['unit'] == 'choice' and field['introduced_schema_version'] == 48
    assert [c['value'] for c in field['choices']] == ['0', '1', '2']
    assets = {}
    if browser:
        assert profile_inputs is not None, 'explicit-production-profile-inputs-required'
        profile = json.loads((profile_inputs.parent / 'result.json').read_bytes())
        assert profile['status'] == 'VERIFIED_DEFAULT_NATIVE_PROFILE'
        assert profile['sha256'] == hashlib.sha256(binary.read_bytes()).hexdigest()
        inputs = json.loads(profile_inputs.read_bytes())
        assets = {route: inputs[source] for route, source in {
            '/native-runtime/settings.js': 'frontend/engine-settings/admin.js',
            '/native-runtime/admin.css': 'frontend/engine-settings/admin.css',
            '/native-runtime/english.js': 'web/i18n-en.js',
            '/cockpit-native-panels.js': 'web/cockpit-native-panels.js',
        }.items()}
    root = native.owned_root(binary)
    spec = out / 'create.json'
    retained.publish(spec, dict(schema_version=1, event_id='autonomy-create', expected_revision=0,
        title='Cockpit autonomy fixture', objective='Read-only default is not launch authority',
        scope='Owned fixture', criteria=['Explicit consent remains required']))
    work = native.cli(binary, root, ['work', 'create', '--input', str(spec)])['work']['id']
    server = native.Server(binary, root, out, 'autonomy')
    primary = None
    checks = []
    def check(name, value):
        """Record an observable assertion only after it passes; fail on false."""
        assert value, name
        checks.append(name)
        print('PASS ' + name, flush=True)
    try:
        server.capture_async(out / 'server.jsonl')
        path = '/api/v1/cockpit-read?work=' + work
        observe = lambda: server.request(path, observe=False)['autonomy_observation']
        initial = observe()
        check('automatic-initial-without-consent', initial['autonomy'] == 'autonome'
            and initial['source'] == 'installation' and initial['consent_enabled'] is False
            and initial['launch_authorized'] is False)
        csrf = server.request('/api/v1/native-session', observe=False)['csrf_token']
        def save(mode, event):
            """Apply one explicit operator choice after checking preview nonmutation."""
            doc = server.request('/api/v1/native-settings', observe=False)
            before = (root / '.swarm/engine-settings.json').read_bytes()
            request = dict(schema_version=48, event_id=event, expected_revision=doc['revision'],
                reason='Explicit inert fixture mode choice',
                values={**doc['values'], 'cockpit_default_autonomy': mode}, preview_token='')
            preview = server.request('/api/v1/native-settings/preview', request, csrf, observe=False)
            check(event + '-preview-read-only', (root / '.swarm/engine-settings.json').read_bytes() == before)
            request['preview_token'] = preview['preview_token']
            server.request('/api/v1/native-settings/apply', request, csrf, observe=False)
            reread = native.cli(binary, root, ['engine-settings', 'show'])
            check(event + '-cli-reread', reread['values']['cockpit_default_autonomy'] == mode)
            return reread
        for mode, wire in [('0', 'manuel'), ('1', 'assiste'), ('2', 'autonome')]:
            save(mode, 'mode-' + mode)
            value = observe()
            check('actual-consumer-' + mode, value['autonomy'] == wire
                and value['source'] == 'installation' and not value['consent_enabled']
                and value['launch_authorized'] is False)
        doc = server.request('/api/v1/native-settings', observe=False)
        before = (root / '.swarm/engine-settings.json').read_bytes()
        for i, bad in enumerate(['3', '-1', '02', None]):
            request = dict(schema_version=48, event_id='invalid-' + str(i), expected_revision=doc['revision'],
                reason='Invalid closed choice', values={**doc['values'], 'cockpit_default_autonomy': bad}, preview_token='')
            failure = server.request('/api/v1/native-settings/preview', request, csrf, status=400, observe=False)
            check('invalid-choice-' + str(i), bool(failure.get('failure'))
                and (root / '.swarm/engine-settings.json').read_bytes() == before)
        if browser:
            # Credentials stay on stdin; only the owned public browser flow sees
            # them. Its artifacts contain no session or CSRF credentials.
            payload = json.dumps(dict(address=server.address, key=server.key,
                work=work, assets=assets, output=str(out / 'browser'))).encode()
            with (out / 'browser.stdout').open('xb') as stdout, (out / 'browser.stderr').open('xb') as stderr:
                result = subprocess.run(['node', 'tests/rust_cockpit_autonomy_ui.cjs'],
                    input=payload, stdout=stdout, stderr=stderr)
            check('actual-browser-flow', result.returncode == 0)
            browser_result = json.loads((out / 'browser/result.json').read_bytes())
            check('actual-browser-named-assertions', browser_result['status'] == 'PASS'
                and len(browser_result['checks']) == 78)
            for file in (out / 'browser').rglob('*'):
                if file.is_file():
                    assert server.key.encode() not in file.read_bytes(), 'fixture-key-leaked'
        current = native.cli(binary, root, ['work', 'show', work])['work']
        pause = out / 'pause.json'
        retained.publish(pause, dict(schema_version=1, event_id='explicit-pause',
            expected_revision=current['revision'], reason='Create explicit safe manual controls'))
        native.cli(binary, root, ['mission', 'pause', work, '--input', str(pause)])
        save('2', 'default-after-mission-mode')
        final = observe()
        check('saved-mission-mode-wins', final['autonomy'] == 'manuel' and final['source'] == 'mission'
            and final['consent_enabled'] is False and final['launch_authorized'] is False)
        mission = native.cli(binary, root, ['mission', 'show', work])
        check('no-inferred-consent-or-start', mission['paused'] is True and mission['consent_enabled'] is None
            and server.request(path, observe=False)['agents'] == [])
        retained.publish(out / 'result.json', dict(status='PASS', root=str(root), work=work,
            binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), checks=checks,
            provider_started=False, installed=False, global_watchdog=None))
        print('COCKPIT_AUTONOMY_PUBLIC_VERIFIED ' + str(len(checks)), flush=True)
    except BaseException as error:
        primary = error
        retained.publish(out / 'failure.json', dict(status='FAIL', root=str(root), cause=traceback.format_exc()))
    finally:
        try:
            server.cleanup()
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup('Autonomy flow and owned server cleanup', [primary, cleanup])
            raise
    if primary is not None:
        raise primary


def main():
    """Require explicit binary and new evidence directory before touching a fixture."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--browser', action='store_true')
    parser.add_argument('--profile-inputs', type=Path)
    parser.add_argument('--legacy-binary', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False, mode=0o700)
    try:
        if args.legacy_binary:
            historical(args.binary.resolve(strict=True), args.legacy_binary.resolve(strict=True), out / 'historical')
        run(args.binary.resolve(strict=True), out, args.browser,
            args.profile_inputs.resolve(strict=True) if args.profile_inputs else None)
    except BaseException:
        if not (out / 'failure.json').exists():
            retained.publish(out / 'failure.json', dict(status='FAIL', cause=traceback.format_exc()))
        raise


if __name__ == '__main__':
    main()
