#!/usr/bin/env python3
"""Exercise the installed-profile review route on an isolated Root through CLI and browser."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.parse import urlencode

import rust_native_http_cli as native


def verify(binary, out):
    """Create a submitted human task, prove refusals, then click Approve with real gate evidence.

    All setup uses public commands. The browser receives its fixture cookie on
    stdin. Only the owned server is stopped, and failures retain the fixture.
    """
    out.mkdir(mode=0o700)
    root = out / 'root'
    root.mkdir(mode=0o700)

    def cli(args, intent=None):
        """Use the existing CLI fixture adapter with an attributable command document."""
        if intent is not None:
            path = out / (intent['event_id'] + '.json')
            path.write_text(json.dumps(intent))
            args = [*args, '--input', str(path)]
        return native.cli(binary, root, args)

    cli(['init'])
    work = cli(['work', 'create'], dict(schema_version=1, event_id='create', expected_revision=0,
        title='Review fixture', objective='Review one result', scope='Isolated review',
        criteria=['Read a submitted task']))['work']
    wid = work['id']
    task = 'review-task'

    def update(action, event, **fields):
        """Advance the fixture revision by one supported operator task operation."""
        return cli(['task', action, wid], dict(id=task, event_id=event, schema_version=1,
            expected_revision=work['revision'], **fields))['work']

    work = update('add', 'add', title='Human review task', deliverable='Owned result',
                  criteria=['Readable proof'])
    work = update('update', 'policy', validation_policy={'mode': 'human'})
    work = update('update', 'start', status='running')
    work = update('update', 'submit', status='submitted', outcome='completed')
    server = native.Server(binary, root, out, 'review')
    primary = None
    try:
        server.capture_async(out / 'web.stdout')

        def api(path, **kwargs):
            """Call the real protected native HTTP service while its event stream drains."""
            return server.request(path, observe=False, **kwargs)

        def preview(revision):
            return '/api/v1/action?' + urlencode(dict(work=wid, task=task, revision=revision))

        def browser(lang, eligible):
            """Render with the fixture session and the effective temp directory.

            Resolve any worktree alias before Chrome binds its Unix socket.
            The browser uses the same temporary directory as the launcher.
            If it exits unsuccessfully, retain its files, publish the fixture
            diagnostics through the recipe logs and propagate the original failure.
            """
            config = dict(address=server.address, cookie=server.cookie, work=wid, task=task,
                          lang=lang, eligible=eligible, screenshot=str(out / (lang + '-detail.png')),
                          helpScreenshot=str(out / (lang + '-help.png')))
            environment = os.environ.copy()
            if environment.get('TMPDIR'):
                environment['TMPDIR'] = str(Path(environment['TMPDIR']).resolve(strict=True))
            stdout_path = out / (lang + '.stdout')
            stderr_path = out / (lang + '.stderr')
            try:
                with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
                    subprocess.run(['node', 'tests/rust_task_detail_review.cjs'],
                        input=json.dumps(config).encode(), stdout=stdout, stderr=stderr, env=environment, check=True)
            except subprocess.CalledProcessError as error:
                try:
                    print(f'{lang} browser failure progress:', file=sys.stderr, flush=True)
                    for line in stdout_path.read_text(errors='replace').splitlines():
                        if line.startswith('PASS '):
                            print(line, file=sys.stderr, flush=True)
                    print(stderr_path.read_text(errors='replace'), file=sys.stderr, flush=True)
                except OSError as diagnostic_error:
                    error.add_note(f'Browser diagnostics unavailable: {type(diagnostic_error).__name__}')
                raise
            for line in stdout_path.read_text().splitlines():
                if line.startswith('PASS '):
                    print(line, flush=True)

        current = '/api/v1/planning?' + urlencode(dict(work=wid))
        files = '/api/v1/agent-logs/task-review-files?' + urlencode(
            dict(work=wid, task=task, revision=work['revision']))
        api(files, authorized=False, status=403)
        invalid = api(files + '&unexpected=1', status=400)
        assert invalid['failure']['code'] == 'task-review-files-query-invalid'
        for invalid_query in ('include_report=0', 'include_report=1&file=report',
                              'include_report=1&include_report=1'):
            invalid = api(files + '&' + invalid_query, status=400)
            assert invalid['failure']['code'] == 'task-review-files-query-invalid'
        api(files + '&include_report=1', authorized=False, status=403)
        assert api(current)['revision'] == work['revision']
        print('PASS originals route rejects anonymous and malformed reads', flush=True)
        missing = api(preview(work['revision']), status=409)
        assert missing['failure']['reason'] == 'gate-required', missing
        browser('en', False)
        assert api(current)['revision'] == work['revision']
        print('PASS blocked dialog preserves submitted revision', flush=True)

        proof = b'Owned fixture result\n'
        (root / 'handoff.md').write_bytes(proof)
        checks = [dict(id=name, gate=phase, domain='quality', mandatory=True,
                       severity='minor', penalty=1, max_penalty=100)
                  for name, phase in work['tasks'][0]['plan_checks'].items()]
        checks.append(dict(id='fixture-delivery', gate='delivery', domain='quality',
                           mandatory=True, severity='minor', penalty=1, max_penalty=100))
        document = dict(method_version='3', scope_id=task, domains={'quality': 1}, checks=checks,
                        artifacts={'handoff.md': hashlib.sha256(proof).hexdigest()},
                        results=[dict(id=check['id'], status='PASS', count=0, evidence=['handoff.md'])
                                 for check in checks])
        work = cli(['gate', wid], dict(schema_version=1, event_id='gate', expected_revision=work['revision'],
            task_id=task, phase='delivery', name='Owned review proof', document=document))['work']
        revision = work['revision']
        assert api(preview(revision)) == dict(schema_version=1, work=wid, task=task, revision=revision, eligible=True)
        assert api(preview(revision-1), status=409)['failure']['code'] == 'task-review-revision-changed'
        api(preview(revision), authorized=False, status=403)
        api('/api/v1/action', body=dict(kind='task', work=wid, task=task, event_id='no-csrf',
            expected_revision=revision, request={'status': 'accepted'}), status=403)
        assert api(current)['revision'] == revision
        print('PASS protected read-only precheck, stale revision and CSRF refusal', flush=True)
        browser('fr', True)
        observed = api(current)
        assert observed['revision'] == revision + 1 and observed['tasks'][0]['status'] == 'accepted'
        event = cli(['work', 'show', wid])['events'][-1]
        assert event['kind'] == 'task.update'
        print('PASS click recorded one task.update and public readback confirmed acceptance', flush=True)
    except BaseException as error:
        primary = error
    finally:
        native.finish(server, primary)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    verify(args.binary.resolve(strict=True), args.output.resolve())
