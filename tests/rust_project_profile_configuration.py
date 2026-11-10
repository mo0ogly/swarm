#!/usr/bin/env python3
"""Exercise explicit project profile selection through a normal Rust CLI/HTTP.

Only an attributed inert Root is mutated. Instruction files/native settings are
synthetic; no provider, live mission or global watchdog. Refusal, publication,
readback, SQL preservation and actual observable content are named assertions.
"""
import argparse
import hashlib
import json
import os
import sqlite3
import subprocess
from pathlib import Path

import rust_agent_model_route as fixture
import rust_native_http_cli as native


def main():
    """Verify catalogue, readonly preview, CAS, validation and public recovery."""
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--expect-unported', action='store_true')
    parser.add_argument('--browser', action='store_true')
    args = parser.parse_args()
    out = Path(args.output).resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=False)
    f = fixture.Fixture(Path(args.binary).resolve(), out, 'profile-configuration')
    checks = []
    server = None
    primary = None
    profile_file = f.root / 'swarm.project.json'

    def check(name, condition):
        """Preserve each named semantic result immediately, never a green exit alone."""
        assert condition, name
        checks.append(name)
        fixture.publish(out / 'checks.json', checks)
        print('PASS ' + name, flush=True)

    def sql():
        """Observe every row in the owned database without opening a live Root."""
        with sqlite3.connect('file:' + str(f.root / '.swarm/state.db') + '?mode=ro', uri=True) as database:
            return hashlib.sha256('\n'.join(database.iterdump()).encode()).hexdigest()

    try:
        before = sql()
        if args.expect_unported:
            rc, _ = f.call(['project-profile', 'list'])
            check('profile-configuration-baseline-unported', rc != 0 and not profile_file.exists() and before == sql())
            print('PROJECT_PROFILE_CONFIGURATION_BASELINE 1', flush=True)
            return
        fixture.publish(f.root / 'AGENTS.md', 'Shared ordinary fixture instructions.\n')
        private = f.root / '.claude'
        private.mkdir()
        fixture.publish(private / 'CLAUDE.md', 'Worker-only ordinary fixture instructions.\n')
        fixture.publish(private / 'settings.json', dict(permissions=dict(allow=['NEVER_IMPORT_NATIVE_PERMISSION'])))
        catalogue = f.ok(['project-profile', 'list'])
        ids = ['claude-project', 'claude-root', 'agents-root', 'agents-directory', 'gemini-root', 'gemini-directory']
        check('profile-catalogue-real-ordered-choices', [c['id'] for c in catalogue['choices']] == ids and catalogue['active_sha256'] == '')
        choice = next(c for c in catalogue['choices'] if c['id'] == 'claude-project')
        check('profile-catalogue-explicit-role-assignment', choice['available'] and choice['profile']['instructions'] == [dict(path='AGENTS.md', roles=['preparation', 'planner', 'subplanner', 'worker', 'reviewer']), dict(path='.claude/CLAUDE.md', roles=['worker'])])
        check('profile-catalogue-missing-choice-refuses', not next(c for c in catalogue['choices'] if c['id'] == 'gemini-root')['available'])
        check('profile-catalogue-no-mutation', not profile_file.exists() and before == sql())
        request = dict(profile={**choice['profile'], 'name': 'Équipe <pilote&>'}, expected_sha256='')
        preview = f.ok(['project-profile', 'preview'], request)
        check('profile-preview-five-real-role-contexts', len(preview['roles']) == 5 and all(r['name'] == request['profile']['name'] for r in preview['roles']) and len(next(r for r in preview['roles'] if r['role'] == 'worker')['sources']) == 2)
        check('profile-preview-no-source-or-native-text', 'NEVER_IMPORT_NATIVE_PERMISSION' not in json.dumps(preview) and 'Shared ordinary' not in json.dumps(preview) and all('prompt' not in r for r in preview['roles']))
        check('profile-preview-no-file-or-sql-change', not profile_file.exists() and before == sql())
        # Exact raw envelope keeps Go struct merge/null behavior before the
        # typed producer. A malformed first duplicate must not be erased.
        raw_file = out / 'profile-null-envelope.json'
        fixture.publish(raw_file, ('{"profile":' + json.dumps(request['profile']) + ',"profile":null,"expected_sha256":""}').encode())
        check('profile-ordered-null-preserves-definition', f.ok(['project-profile', 'preview', '--input', str(raw_file)]) == preview)
        bad_file = out / 'profile-type-error-envelope.json'
        fixture.publish(bad_file, ('{"profile":{"version":"PRIVATE_BAD_INPUT"},"profile":' + json.dumps(request['profile']) + ',"expected_sha256":""}').encode())
        rc, error = f.call(['project-profile', 'preview', '--input', str(bad_file)])
        check('profile-duplicate-cannot-erase-type-error', rc != 0 and 'PRIVATE_BAD_INPUT' not in json.dumps(error) and not profile_file.exists())
        server = native.Server(f.binary, f.root, out, 'profile-config-http', environment=f.environment)
        server.capture_async(out / 'profile-config-http.raw')
        csrf = server.request('/api/v1/native-session', observe=False)['csrf_token']
        check('profile-catalogue-cli-http-equal', server.request('/api/v1/project-profile/catalog', observe=False) == catalogue)
        server.request('/api/v1/project-profile/apply', request, status=403, observe=False)
        check('profile-csrf-before-publication', not profile_file.exists() and before == sql())
        check('profile-preview-cli-http-equal', server.request('/api/v1/project-profile/preview', request, csrf=csrf, observe=False) == preview)
        applied = f.ok(['project-profile', 'apply'], request)
        canonical = (json.dumps(request['profile'], ensure_ascii=False, indent=2).replace('<', '\\u003c').replace('>', '\\u003e').replace('&', '\\u0026').replace('\u2028', '\\u2028').replace('\u2029', '\\u2029') + '\n').encode()
        check('profile-canonical-file-and-real-readback', profile_file.read_bytes() == canonical and applied == f.ok(['project-profile', 'show']) and all(r['profile_sha256'] == hashlib.sha256(canonical).hexdigest() for r in applied['roles']))
        check('profile-native-settings-metadata-only', applied['claude_settings_detected'] == ['.claude/settings.json'] and 'NEVER_IMPORT_NATIVE_PERMISSION' not in json.dumps(applied))
        stale = server.request('/api/v1/project-profile/apply', request, csrf=csrf, status=409, observe=False)
        check('profile-stale-cas-no-rewrite', stale['failure']['code'] == 'project-profile-conflict' and profile_file.read_bytes() == canonical)
        current_sha = hashlib.sha256(canonical).hexdigest()
        invalid = {**request, 'expected_sha256': current_sha, 'profile': {**request['profile'], 'instructions': [dict(path='AGENTS.md', roles=['worker'])]}}
        denied = server.request('/api/v1/project-profile/preview', invalid, csrf=csrf, status=400, observe=False)
        check('profile-missing-role-refuses-before-write', denied['failure']['reason'] == 'project-role-has-no-source' and profile_file.read_bytes() == canonical)
        settings = f.ok(['engine-settings', 'show'])['values']
        f.settings_change(project_context_max_bytes='0')
        rc, _ = f.call(['project-profile', 'preview'], {**request, 'expected_sha256': current_sha})
        check('profile-saved-zero-cap-consumed', rc != 0 and profile_file.read_bytes() == canonical)
        f.settings_change(project_context_max_bytes=settings['project_context_max_bytes'])
        check('profile-saved-cap-public-recovery', f.ok(['project-profile', 'preview'], {**request, 'expected_sha256': current_sha})['profile'] == request['profile'])
        selected = server.request('/api/v1/project-profile/select', dict(id='agents-root', expected_sha256=current_sha), csrf=csrf, observe=False)
        check('profile-select-real-shared-source', f.ok(['project-profile', 'list'])['active_source'] == 'agents-root' and all([s['path'] for s in r['sources']] == ['AGENTS.md'] for r in selected['roles']))
        # Actual output failure after a successful configuration must carry the
        # common known-state marker instead of implying that no file was changed.
        request = dict(profile={**request['profile'], 'name': 'After output failure'}, expected_sha256=hashlib.sha256(profile_file.read_bytes()).hexdigest())
        input_file = out / 'output-failure-input.json'
        fixture.publish(input_file, request)
        argv = [str(f.binary), 'project-profile', 'apply', '--root', str(f.root), '--input', str(input_file), '--json']
        with open('/dev/full', 'wb') as sink:
            process = subprocess.run(argv, env=f.environment, stdout=sink, stderr=subprocess.PIPE)
        fixture.publish(out / 'output-failure.stderr', process.stderr)
        fixture.publish(out / 'output-failure-command.json', dict(argv=argv, exit=process.returncode))
        error = json.loads(process.stderr)
        check('profile-output-failure-preserves-recorded-state', process.returncode != 0 and error.get('configuration_may_be_recorded') is True and f.ok(['project-profile', 'show'])['name'] == request['profile']['name'])
        # Invalid active JSON is diagnostic data with a repair digest, never a
        # fabricated successful active profile. Explicit valid apply repairs it.
        valid = profile_file.read_bytes()
        fixture.publish(profile_file, b'{invalid active profile')
        diagnostic = f.ok(['project-profile', 'list'])
        check('profile-invalid-active-visible-with-cas-digest', diagnostic['active_error'] == 'project-profile-active-json-invalid' and diagnostic['active_name'] == '' and diagnostic['active_sha256'] == hashlib.sha256(profile_file.read_bytes()).hexdigest())
        request['expected_sha256'] = diagnostic['active_sha256']
        f.ok(['project-profile', 'apply'], request)
        check('profile-invalid-active-public-repair', profile_file.read_bytes() == valid)
        check('profile-no-work-or-agent-mutation', sql() == before and not (f.root / 'model-received.json').exists())
        if args.browser:
            before_settings = f.ok(['engine-settings', 'show'])
            browser_out = out / 'browser'
            browser_out.mkdir()
            cfg = dict(base='http://' + server.address, cookie=server.cookie, output=str(browser_out))
            with (out / 'browser.stdout').open('wb') as stdout, (out / 'browser.stderr').open('wb') as stderr:
                result = subprocess.run(['node', 'tests/rust_project_profile_configuration_ui.cjs'], input=json.dumps(cfg).encode(), stdout=stdout, stderr=stderr)
            print((out / 'browser.stdout').read_text(), end='', flush=True)
            check('profile-browser-public-journey', result.returncode == 0 and sql() == before and f.ok(['engine-settings', 'show']) == before_settings)
        fixture.publish(out / 'result.json', dict(passed=True, assertions=checks, binary_sha256=fixture.digest(f.binary.read_bytes()), root=str(f.root), limits='Owned inert Root; no real provider, living mission, identity or OS-confinement qualification.'))
        print('PROJECT_PROFILE_CONFIGURATION_PASS ' + str(len(checks)), flush=True)
    except BaseException as error:
        primary = error
    finally:
        if server is not None:
            native.finish(server, primary)
        elif primary is not None:
            raise primary


if __name__ == '__main__':
    main()
